// SPDX-License-Identifier: GPL-3.0-or-later
//! Explicit, supplied-byte numeric regular multi-section profile. The ordinary
//! decoder, sparse path and closed physical-row adapters never call this module.
//! Global ordinal repair and deferred ordered copies follow pinned
//! DB2FileLoader.cpp::Load/AutoProduceRecordCopies (02245dcd); framing is a
//! deliberately narrower Rust admission policy, not native rejection parity.
use super::{CharacterTable, DecodedTable, Kind, RecordError, SPARSE_FLAG, Value, u32_at};
use crate::{HEADER_SIZE, Metadata, SECTION_SIZE};
use std::{collections::BTreeMap, io::Cursor};

const MAX_BYTES: usize = 16 * 1024 * 1024;
const MAX_RECORDS: usize = 100_000;
const MAX_COPIES: usize = 100_000;
const MAX_VALUES: usize = 1_000_000;

struct Range {
    records: usize,
    ids: usize,
    copies: usize,
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
    if !table.matches(&h) {
        return Err(RecordError::Schema);
    }
    if h.flags & SPARSE_FLAG != 0
        || matches!(table, CharacterTable::Catalog(_))
        || schema.index >= 0
        || h.parent_lookup_count != 0
        || schema.parent >= 0
        || h.string_table_size != 0
        || schema
            .fields
            .iter()
            .any(|kind| matches!(kind, Kind::String | Kind::StringArray(_)))
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
    let mut ranges = Vec::with_capacity(meta.sections.len());
    let mut record_count = 0usize;
    let mut copy_count = 0usize;
    // Validate the entire physical inventory before decoding any row. Empty
    // intermediate sections occupy zero bytes and share the next start offset.
    for section in &meta.sections {
        // CatalogDataOffset is unused by regular C++; zero is an explicit
        // conservative restriction of this profile, not a native requirement.
        if section.tact_declared
            || section.catalog_data_count != 0
            || section.catalog_data_offset != 0
            || section.string_table_size != 0
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
        let ids = (section.record_count as usize)
            .checked_mul(h.record_size as usize)
            .and_then(|bytes| records.checked_add(bytes))
            .ok_or(RecordError::Bounds)?;
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
            ids,
            copies,
        });
    }
    if record_count != h.record_count as usize || end != data.len() {
        return Err(RecordError::Bounds);
    }
    let mut rows = BTreeMap::new();
    let mut ordinal = 0u32;
    for (section, range) in meta.sections.iter().zip(&ranges) {
        for record in 0..section.record_count as usize {
            // Complete section bounds and the fixed 16 MiB admission above
            // prove these local products/additions fit usize on every target.
            let at = range.records + record * h.record_size as usize;
            let raw = &data[at..at + h.record_size as usize];
            let stored = u32_at(data, range.ids + record * 4)?;
            let id = if stored <= h.min_id {
                h.min_id.checked_add(ordinal).ok_or(RecordError::Id)?
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
                        Kind::Unsigned(64, _) => Value::Unsigned64(bits),
                        Kind::Unsigned(_, _) => Value::Unsigned(bits as u32),
                        Kind::Signed(64, _) => Value::Signed64(bits as i64),
                        Kind::Signed(width, _) => Value::Signed(match width {
                            8 => bits as u8 as i8 as i32,
                            16 => bits as u16 as i16 as i32,
                            _ => bits as i32,
                        }),
                        Kind::Float(_) => Value::FloatBits(bits as u32),
                        Kind::String | Kind::StringArray(_) => {
                            return Err(RecordError::Unsupported);
                        }
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
