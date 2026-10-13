// SPDX-License-Identifier: GPL-3.0-or-later
//! Explicit supplied-byte regular multi-section profiles. The ordinary
//! decoder, sparse path and closed physical-row adapters never call this
//! module. Global ordinal repair and deferred ordered copies follow pinned
//! DB2FileLoader.cpp::Load/AutoProduceRecordCopies (02245dcd); framing is a
//! deliberately narrower Rust admission policy, not native rejection parity.
use super::{
    CharacterTable, DecodedTable, Kind, RecordError, SPARSE_FLAG, TEXT_POLICY_LIMIT, TextBudget,
    Value, u32_at,
};
use crate::{HEADER_SIZE, Metadata, SECTION_SIZE};
use std::{collections::BTreeMap, io::Cursor};

const MAX_BYTES: usize = 16 * 1024 * 1024;
const MAX_RECORDS: usize = 100_000;
const MAX_COPIES: usize = 100_000;
const MAX_VALUES: usize = 1_000_000;

struct Range {
    records: usize,
    strings: usize,
    strings_end: usize,
    ids: usize,
    copies: usize,
}

/// Private decode policy. Public entry points choose fixed limits and distinct
/// text/catalog admission; synthetic tests may only supply bounded reductions.
struct Policy {
    record_limit: usize,
    copy_limit: usize,
    value_limit: usize,
    allow_text: bool,
    allow_catalog: bool,
    text_limit: usize,
}

pub(super) fn read(
    data: &[u8],
    table: CharacterTable,
    max_sections: u32,
) -> Result<DecodedTable, RecordError> {
    read_with_limits(
        data,
        table,
        max_sections,
        MAX_RECORDS,
        MAX_COPIES,
        MAX_VALUES,
    )
}

pub(super) fn read_with_text(
    data: &[u8],
    table: CharacterTable,
    max_sections: u32,
) -> Result<DecodedTable, RecordError> {
    read_with_text_limits(
        data,
        table,
        max_sections,
        MAX_RECORDS,
        MAX_COPIES,
        MAX_VALUES,
        TEXT_POLICY_LIMIT,
    )
}

pub(super) fn read_from_catalog(
    data: &[u8],
    table: CharacterTable,
    max_sections: u32,
) -> Result<DecodedTable, RecordError> {
    let schema = table.schema();
    let allow_text = schema
        .fields
        .iter()
        .any(|kind| matches!(kind, Kind::String | Kind::StringArray(_)));
    read_impl(
        data,
        table,
        max_sections,
        Policy {
            record_limit: MAX_RECORDS,
            copy_limit: MAX_COPIES,
            value_limit: MAX_VALUES,
            allow_text,
            allow_catalog: true,
            text_limit: TEXT_POLICY_LIMIT,
        },
    )
}

// Private reductions make exact cumulative boundaries reviewable with small
// authored fixtures. Public callers can only reduce the section ceiling.
fn read_with_limits(
    data: &[u8],
    table: CharacterTable,
    max_sections: u32,
    record_limit: usize,
    copy_limit: usize,
    value_limit: usize,
) -> Result<DecodedTable, RecordError> {
    read_impl(
        data,
        table,
        max_sections,
        Policy {
            record_limit,
            copy_limit,
            value_limit,
            allow_text: false,
            allow_catalog: false,
            text_limit: 0,
        },
    )
}

fn read_with_text_limits(
    data: &[u8],
    table: CharacterTable,
    max_sections: u32,
    record_limit: usize,
    copy_limit: usize,
    value_limit: usize,
    text_limit: usize,
) -> Result<DecodedTable, RecordError> {
    read_impl(
        data,
        table,
        max_sections,
        Policy {
            record_limit,
            copy_limit,
            value_limit,
            allow_text: true,
            allow_catalog: false,
            text_limit,
        },
    )
}

fn read_impl(
    data: &[u8],
    table: CharacterTable,
    max_sections: u32,
    policy: Policy,
) -> Result<DecodedTable, RecordError> {
    let Policy {
        record_limit,
        copy_limit,
        value_limit,
        allow_text,
        allow_catalog,
        text_limit,
    } = policy;
    if !(2..=DecodedTable::MAX_REGULAR_SECTIONS).contains(&max_sections) || data.len() > MAX_BYTES {
        return Err(RecordError::Limit);
    }
    let meta = Metadata::read(&mut Cursor::new(data)).map_err(|_| RecordError::Metadata)?;
    let h = meta.header;
    if h.section_count > max_sections {
        return Err(RecordError::Limit);
    }
    if h.section_count < 2 {
        return Err(RecordError::Unsupported);
    }
    let schema = table.schema();
    let schema_has_text = schema
        .fields
        .iter()
        .any(|kind| matches!(kind, Kind::String | Kind::StringArray(_)));
    if !table.matches(&h) {
        return Err(RecordError::Schema);
    }
    if schema_has_text != allow_text {
        return Err(RecordError::Unsupported);
    }
    if h.flags & SPARSE_FLAG != 0
        || (!allow_catalog && matches!(table, CharacterTable::Catalog(_)))
        || schema.index >= 0
        || h.parent_lookup_count != 0
        || schema.parent >= 0
        || (!allow_text && h.string_table_size != 0)
    {
        return Err(RecordError::Unsupported);
    }
    if h.record_size == 0 || h.min_id > h.max_id {
        return Err(RecordError::Bounds);
    }
    let row_values = schema
        .fields
        .iter()
        .try_fold(0usize, |sum, kind| sum.checked_add(kind.count()))
        .ok_or(RecordError::Limit)?;
    let mut value_count = (h.record_count as usize)
        .checked_mul(row_values)
        .filter(|count| *count <= value_limit)
        .ok_or(RecordError::Limit)?;
    if h.record_count as usize > record_limit {
        return Err(RecordError::Limit);
    }
    let prefix = meta
        .sections
        .len()
        .checked_mul(SECTION_SIZE)
        .and_then(|bytes| HEADER_SIZE.checked_add(bytes))
        .ok_or(RecordError::Bounds)?;
    let (cols, mut end) = super::columns::columns_after_prefix(data, &h, prefix)?;
    if allow_text {
        for (field, kind) in schema.fields.iter().enumerate() {
            if matches!(kind, Kind::String | Kind::StringArray(_))
                && cols.get(field).ok_or(RecordError::Column)?.compression != 0
            {
                return Err(RecordError::Column);
            }
        }
    }
    let mut ranges = Vec::with_capacity(meta.sections.len());
    let mut record_count = 0usize;
    let mut copy_count = 0usize;
    let mut string_bytes = 0usize;
    // Validate the entire physical inventory before decoding any row. Empty
    // intermediate sections occupy zero bytes and share the next start offset.
    for section in &meta.sections {
        // CatalogDataOffset is unused by regular C++; zero is an explicit
        // conservative restriction of this profile, not a native requirement.
        if section.tact_declared
            || section.catalog_data_count != 0
            || section.catalog_data_offset != 0
            || (!allow_text && section.string_table_size != 0)
            || section.parent_lookup_data_size != 0
        {
            return Err(RecordError::Unsupported);
        }
        record_count = record_count
            .checked_add(section.record_count as usize)
            .filter(|count| *count <= record_limit)
            .ok_or(RecordError::Limit)?;
        copy_count = copy_count
            .checked_add(section.copy_table_count as usize)
            .filter(|count| *count <= copy_limit)
            .ok_or(RecordError::Limit)?;
        if section.file_offset as usize != end {
            return Err(RecordError::Bounds);
        }
        let records = end;
        let strings = (section.record_count as usize)
            .checked_mul(h.record_size as usize)
            .and_then(|bytes| records.checked_add(bytes))
            .ok_or(RecordError::Bounds)?;
        let strings_end = strings
            .checked_add(section.string_table_size as usize)
            .filter(|end| *end <= data.len())
            .ok_or(RecordError::Bounds)?;
        string_bytes = string_bytes
            .checked_add(section.string_table_size as usize)
            .ok_or(RecordError::Bounds)?;
        let ids = strings_end;
        let id_bytes = (section.record_count as usize)
            .checked_mul(4)
            .ok_or(RecordError::Bounds)?;
        if section.id_table_size as usize != id_bytes {
            return Err(RecordError::Id);
        }
        let copies = ids.checked_add(id_bytes).ok_or(RecordError::Bounds)?;
        end = (section.copy_table_count as usize)
            .checked_mul(8)
            .and_then(|bytes| copies.checked_add(bytes))
            .filter(|end| *end <= data.len())
            .ok_or(RecordError::Bounds)?;
        ranges.push(Range {
            records,
            strings,
            strings_end,
            ids,
            copies,
        });
    }
    if record_count != h.record_count as usize
        || string_bytes != h.string_table_size as usize
        || end != data.len()
    {
        return Err(RecordError::Bounds);
    }
    let mut string_pool = Vec::new();
    if allow_text {
        string_pool
            .try_reserve_exact(string_bytes)
            .map_err(|_| RecordError::Limit)?;
        for range in &ranges {
            let bytes = data
                .get(range.strings..range.strings_end)
                .ok_or(RecordError::Bounds)?;
            string_pool.extend_from_slice(bytes);
        }
    }
    let logical_strings_start = (h.record_count as usize)
        .checked_mul(h.record_size as usize)
        .ok_or(RecordError::Bounds)?;
    let mut text_budget = TextBudget::new(text_limit);
    let mut rows = BTreeMap::new();
    let mut ordinal = 0u32;
    for (section, range) in meta.sections.iter().zip(&ranges) {
        for record in 0..section.record_count as usize {
            // Complete section bounds and the fixed 16 MiB admission above
            // prove these local products/additions fit usize on every target.
            let at = range.records + record * h.record_size as usize;
            let raw = &data[at..at + h.record_size as usize];
            let record_ordinal = ordinal;
            let stored = u32_at(data, range.ids + record * 4)?;
            let id = if stored <= h.min_id {
                h.min_id
                    .checked_add(record_ordinal)
                    .ok_or(RecordError::Id)?
            } else {
                stored
            };
            ordinal = ordinal.checked_add(1).ok_or(RecordError::Id)?;
            if id > h.max_id || rows.contains_key(&id) {
                return Err(RecordError::Id);
            }
            let mut fields = Vec::with_capacity(schema.fields.len());
            for (field, kind) in schema.fields.iter().enumerate() {
                let mut values = Vec::with_capacity(kind.count());
                for array in 0..kind.count() {
                    let bits = cols[field].bits(
                        raw,
                        h.packed_data_offset,
                        id,
                        array,
                        kind.bits(),
                        kind.count(),
                    )?;
                    values.push(match *kind {
                        Kind::String | Kind::StringArray(_) => {
                            if !allow_text || cols[field].compression != 0 {
                                return Err(RecordError::Column);
                            }
                            if bits == 0 {
                                Value::Text(String::new())
                            } else {
                                let logical_record_at = (record_ordinal as usize)
                                    .checked_mul(h.record_size as usize)
                                    .ok_or(RecordError::Bounds)?;
                                let field_offset = (cols[field].offset as usize / 8)
                                    .checked_add(array.checked_mul(4).ok_or(RecordError::Bounds)?)
                                    .ok_or(RecordError::Bounds)?;
                                let pointer_at = logical_record_at
                                    .checked_add(field_offset)
                                    .ok_or(RecordError::Bounds)?;
                                let string_at = pointer_at
                                    .checked_add(
                                        usize::try_from(bits).map_err(|_| RecordError::String)?,
                                    )
                                    .ok_or(RecordError::String)?;
                                let string_offset = string_at
                                    .checked_sub(logical_strings_start)
                                    .filter(|offset| *offset < string_pool.len())
                                    .ok_or(RecordError::String)?;
                                let tail = string_pool
                                    .get(string_offset..)
                                    .ok_or(RecordError::String)?;
                                let len = tail
                                    .iter()
                                    .position(|byte| *byte == 0)
                                    .ok_or(RecordError::String)?;
                                text_budget.charge(len)?;
                                Value::Text(
                                    std::str::from_utf8(&tail[..len])
                                        .map_err(|_| RecordError::String)?
                                        .to_owned(),
                                )
                            }
                        }
                        Kind::Unsigned(64, _) => Value::Unsigned64(bits),
                        Kind::Unsigned(_, _) => Value::Unsigned(bits as u32),
                        Kind::Signed(64, _) => Value::Signed64(bits as i64),
                        Kind::Signed(width, _) => Value::Signed(match width {
                            8 => bits as u8 as i8 as i32,
                            16 => bits as u16 as i16 as i32,
                            _ => bits as i32,
                        }),
                        Kind::Float(_) => Value::FloatBits(bits as u32),
                    });
                }
                fields.push(values);
            }
            rows.insert(id, fields);
        }
    }
    // Extent includes ALL destinations, even copies that will be skipped.
    let mut max_id = rows.last_key_value().map_or(0, |(&id, _)| id);
    for (section, range) in meta.sections.iter().zip(&ranges) {
        for index in 0..section.copy_table_count as usize {
            max_id = max_id.max(u32_at(data, range.copies + index * 8)?);
        }
    }
    if max_id > h.max_id {
        return Err(RecordError::Id);
    }
    let extent = max_id.checked_add(1).ok_or(RecordError::Id)?;
    // Every direct row exists before the first copy: sources may reside in
    // later physical sections, and earlier copies may feed later copies.
    for (section, range) in meta.sections.iter().zip(&ranges) {
        for index in 0..section.copy_table_count as usize {
            let at = range.copies + index * 8;
            let new = u32_at(data, at)?;
            let source = u32_at(data, at + 4)?;
            if source == 0 || source >= extent || new >= extent {
                continue;
            }
            let Some(source_fields) = rows.get(&source) else {
                continue;
            };
            if source == new {
                return Err(RecordError::Unsupported);
            }
            let copied_text_bytes = source_fields
                .iter()
                .flatten()
                .try_fold(0usize, |total, value| match value {
                    Value::Text(text) => total.checked_add(text.len()),
                    _ => Some(total),
                })
                .ok_or(RecordError::Limit)?;
            text_budget.charge(copied_text_bytes)?;
            if !rows.contains_key(&new) {
                value_count = value_count
                    .checked_add(row_values)
                    .filter(|count| *count <= value_limit)
                    .ok_or(RecordError::Limit)?;
            }
            let fields = source_fields.clone();
            rows.insert(new, fields);
        }
    }
    Ok(DecodedTable { rows })
}

#[cfg(test)]
mod tests;
