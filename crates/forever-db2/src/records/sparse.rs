// SPDX-License-Identifier: GPL-3.0-or-later
// Independently authored 2026-10-11 from pinned TrinityCore 02245dcd facts.
//! Bounded sparse subset: external catalog IDs, no parent, fields composed of
//! U8/U16/U32/I8/I16/I32, F32 or inline UTF-8 strings, and zero or one
//! unencrypted section. Numeric and string arrays use their complete metadata
//! shape. This does not provide general sparse compatibility.
use super::{
    DecodedTable, RecordError, StringMode, TextBudget, Value,
    schema::{Kind, Schema},
    u32_at,
};
use crate::{HEADER_SIZE, Metadata, SECTION_SIZE};
use std::collections::{BTreeMap, BTreeSet};

fn add(at: usize, bytes: usize) -> Result<usize, RecordError> {
    at.checked_add(bytes).ok_or(RecordError::Bounds)
}
fn count_bytes(count: u32, width: usize) -> Result<usize, RecordError> {
    (count as usize)
        .checked_mul(width)
        .ok_or(RecordError::Bounds)
}
fn half(data: &[u8], at: usize) -> Result<u16, RecordError> {
    Ok(u16::from_le_bytes(
        data.get(at..add(at, 2)?)
            .ok_or(RecordError::Bounds)?
            .try_into()
            .unwrap(),
    ))
}

struct Policy {
    value_limit: usize,
    text_limit: usize,
    string_mode: StringMode,
    allow_appended_parent: bool,
}

pub(super) fn read(
    data: &[u8],
    meta: &Metadata,
    schema: &Schema,
    direct_field: Option<usize>,
    text_limit: usize,
) -> Result<(DecodedTable, BTreeSet<u32>), RecordError> {
    read_with_limits_and_text(
        data,
        meta,
        schema,
        direct_field,
        Policy {
            value_limit: 1_000_000,
            text_limit,
            string_mode: StringMode::StrictUtf8,
            allow_appended_parent: false,
        },
    )
}

/// Validate a fixed catalog sparse-parent input without exposing decoded rows
/// or parent associations. The native materializer rejects this variant.
pub(super) fn validate_catalog_parented_raw(
    data: &[u8],
    meta: &Metadata,
    schema: &Schema,
    text_limit: usize,
) -> Result<(), RecordError> {
    read_with_limits_and_text(
        data,
        meta,
        schema,
        None,
        Policy {
            value_limit: 1_000_000,
            text_limit,
            string_mode: StringMode::OpaqueBytes,
            allow_appended_parent: true,
        },
    )
    .map(|_| ())
}

#[cfg(test)]
fn read_with_limit(
    data: &[u8],
    meta: &Metadata,
    schema: &Schema,
    direct_field: Option<usize>,
    value_limit: usize,
) -> Result<(DecodedTable, BTreeSet<u32>), RecordError> {
    read_with_limits_and_text(
        data,
        meta,
        schema,
        direct_field,
        Policy {
            value_limit,
            text_limit: 0,
            string_mode: StringMode::StrictUtf8,
            allow_appended_parent: false,
        },
    )
}

fn read_with_limits_and_text(
    data: &[u8],
    meta: &Metadata,
    schema: &Schema,
    direct_field: Option<usize>,
    policy: Policy,
) -> Result<(DecodedTable, BTreeSet<u32>), RecordError> {
    let Policy {
        value_limit,
        text_limit,
        string_mode,
        allow_appended_parent,
    } = policy;
    let h = meta.header;
    // Sparse AutoProduceData:1055 rejects appended parents even for no rows.
    // Strict row projection still excludes parents and inline IDs. A separate
    // raw-only caller may admit an appended parent for its fixed catalog list.
    let appended_parent = allow_appended_parent
        && schema.parent >= 0
        && schema.parent as usize == schema.fields.len();
    if (schema.parent >= 0 && !appended_parent)
        || schema.index >= 0
        || schema.fields.iter().any(|kind| match kind {
            Kind::String => false,
            Kind::StringArray(count) => *count == 0,
            Kind::Unsigned(8 | 16 | 32, count)
            | Kind::Signed(8 | 16 | 32, count)
            | Kind::Float(count) => *count == 0,
            _ => true,
        })
    {
        return Err(RecordError::Unsupported);
    }
    if allow_appended_parent && !appended_parent {
        return Err(RecordError::Unsupported);
    }
    if h.parent_lookup_count > 1 {
        return Err(RecordError::Unsupported);
    }
    if h.parent_lookup_count != 0 && !appended_parent {
        return Err(RecordError::Schema);
    }
    if meta.sections.len() > 1
        || h.string_table_size != 0
        || h.common_data_size != 0
        || h.pallet_data_size != 0
    {
        return Err(RecordError::Unsupported);
    }
    if h.record_count > 100_000 {
        return Err(RecordError::Limit);
    }
    let row_values = schema.fields.iter().try_fold(0_usize, |total, kind| {
        total.checked_add(kind.count()).ok_or(RecordError::Limit)
    })?;
    let mut value_count = (h.record_count as usize)
        .checked_mul(row_values)
        .filter(|n| *n <= value_limit)
        .ok_or(RecordError::Limit)?;
    if h.min_id > h.max_id {
        return Err(RecordError::Bounds);
    }
    // Unlike regular GetMaxId(), sparse allocates Header.MaxId+1 (:1064).
    // Keep the same range semantics without allocating an array by that ID.
    let extent = h.max_id.checked_add(1).ok_or(RecordError::Id)?;
    let primitive_at = add(HEADER_SIZE, meta.sections.len() * SECTION_SIZE)?;
    let primitive_end = add(primitive_at, count_bytes(h.field_count, 4)?)?;
    if primitive_end > data.len() {
        return Err(RecordError::Bounds);
    }
    let mut widths = Vec::with_capacity(schema.fields.len());
    for (field, kind) in schema.fields.iter().enumerate() {
        let unused = half(data, add(primitive_at, field * 4)?)? as i16;
        // GetFieldSize:1509 is 4 - UnusedBits/8. The 32-bit getter also
        // masks/sign-extends the meaningful bits (:1489). BYTE/SHORT getters
        // read their native widths, so only their matching slot sizes belong
        // to this first subset. Primitive Offset is ignored by C++ (:1545).
        if !(0..=31).contains(&unused) {
            return Err(RecordError::Column);
        }
        let width = 4 - unused as usize / 8;
        match kind {
            Kind::Unsigned(8, _) | Kind::Signed(8, _) if unused != 24 => {
                return Err(RecordError::Column);
            }
            Kind::Unsigned(16, _) | Kind::Signed(16, _) if unused != 16 => {
                return Err(RecordError::Column);
            }
            Kind::Float(_) | Kind::String | Kind::StringArray(_) if unused != 0 => {
                return Err(RecordError::Column);
            }
            _ => {}
        }
        widths.push((width, 32 - unused as u32));
    }
    let mut text_budget = TextBudget::new(text_limit);
    let mut rows = BTreeMap::new();
    let mut direct_values = BTreeSet::new();
    if meta.sections.is_empty() {
        if h.record_count != 0 || h.column_meta_size != 0 || h.parent_lookup_count != 0 {
            return Err(RecordError::Unsupported);
        }
        if data.len() != primitive_end {
            return Err(RecordError::Bounds);
        }
        return Ok((DecodedTable { rows }, direct_values));
    }
    let section = meta.sections[0];
    if section.tact_declared
        || section.string_table_size != 0
        || (h.parent_lookup_count == 0 && section.parent_lookup_data_size != 0)
    {
        return Err(RecordError::Unsupported);
    }
    if section.copy_table_count > 100_000 {
        return Err(RecordError::Limit);
    }
    if section.record_count != h.record_count || section.catalog_data_count != h.record_count {
        return Err(RecordError::Bounds);
    }
    let expected_columns = count_bytes(h.total_field_count, 24)?;
    // Sparse LoadColumnData:1003 discards columns. Admit absent storage
    // metadata or complete None columns without extra payload.
    if h.column_meta_size != 0 && h.column_meta_size as usize != expected_columns {
        return Err(RecordError::Column);
    }
    let metadata_end = add(primitive_end, h.column_meta_size as usize)?;
    if metadata_end > data.len() {
        return Err(RecordError::Bounds);
    }
    if h.column_meta_size != 0 {
        for field in 0..schema.fields.len() {
            let at = add(primitive_end, field * 24)?;
            if u32_at(data, add(at, 4)?)? != 0 || u32_at(data, add(at, 8)?)? != 0 {
                return Err(RecordError::Unsupported);
            }
        }
    }
    let payload_start = section.file_offset as usize;
    let catalog_start = section.catalog_data_offset as usize;
    if payload_start != metadata_end || catalog_start < payload_start || catalog_start > data.len()
    {
        return Err(RecordError::Bounds);
    }
    // LoadCatalogData:1020-1040 reads IDs, then copies, then packed six-byte
    // {absolute file offset,u16 size} entries. Load:1949 then consumes a second
    // ID table; Sparse SetAdditionalData ignores that table and its ID repair.
    let id_bytes = count_bytes(section.catalog_data_count, 4)?;
    if section.id_table_size as usize != id_bytes {
        return Err(RecordError::Id);
    }
    let copies_at = add(catalog_start, id_bytes)?;
    let entries_at = add(copies_at, count_bytes(section.copy_table_count, 8)?)?;
    let trailing_ids_at = add(entries_at, count_bytes(section.catalog_data_count, 6)?)?;
    let end = add(trailing_ids_at, id_bytes)?;
    for ordinal in 0..section.catalog_data_count as usize {
        let id = u32_at(data, add(catalog_start, ordinal * 4)?)?;
        if id >= extent || rows.contains_key(&id) {
            return Err(RecordError::Id);
        }
        let entry = add(entries_at, ordinal * 6)?;
        let record_at = u32_at(data, entry)? as usize;
        let record_end = add(record_at, half(data, add(entry, 4)?)? as usize)?;
        if record_at < payload_start || record_end > catalog_start || record_at == record_end {
            return Err(RecordError::Bounds);
        }
        let raw = data.get(record_at..record_end).ok_or(RecordError::Bounds)?;
        let mut at = 0;
        let mut fields = Vec::with_capacity(schema.fields.len());
        for (field, kind) in schema.fields.iter().enumerate() {
            let (width, significant) = widths[field];
            let mut values = Vec::with_capacity(kind.count());
            for _ in 0..kind.count() {
                let value = match kind {
                    Kind::String | Kind::StringArray(_) => {
                        let tail = raw.get(at..).ok_or(RecordError::String)?;
                        let length = tail
                            .iter()
                            .position(|byte| *byte == 0)
                            .ok_or(RecordError::String)?;
                        text_budget.charge(length)?;
                        let text = match string_mode {
                            StringMode::StrictUtf8 => std::str::from_utf8(&tail[..length])
                                .map_err(|_| RecordError::String)?
                                .to_owned(),
                            StringMode::OpaqueBytes | StringMode::CatalogSparseParentRaw => {
                                String::new()
                            }
                        };
                        at = add(at, length + 1)?;
                        Value::Text(text)
                    }
                    Kind::Float(_) => {
                        let end = add(at, 4)?;
                        let bits = u32::from_le_bytes(
                            raw.get(at..end)
                                .ok_or(RecordError::Bounds)?
                                .try_into()
                                .unwrap(),
                        );
                        at = end;
                        Value::FloatBits(bits)
                    }
                    Kind::Unsigned(_, _) | Kind::Signed(_, _) => {
                        let end = add(at, width)?;
                        let encoded = raw.get(at..end).ok_or(RecordError::Bounds)?;
                        let mut word = [0u8; 4];
                        word[..width].copy_from_slice(encoded);
                        let bits = u32::from_le_bytes(word);
                        at = end;
                        match kind {
                            Kind::Unsigned(32, _) => {
                                Value::Unsigned(bits & (u32::MAX >> (32 - significant)))
                            }
                            Kind::Signed(_, _) => Value::Signed(
                                ((bits << (32 - significant)) as i32) >> (32 - significant),
                            ),
                            Kind::Unsigned(_, _) => Value::Unsigned(bits),
                            _ => return Err(RecordError::Unsupported),
                        }
                    }
                };
                values.push(value);
            }
            fields.push(values);
        }
        // Native sparse getters ignore trailing bytes/padding in catalog rows.
        // Header.RecordSize and primitive Offset do not determine these ranges.
        if let Some(field) = direct_field {
            let Value::Unsigned(value) = &fields[field][0] else {
                return Err(RecordError::Schema);
            };
            direct_values.insert(*value);
        }
        rows.insert(id, fields);
    }
    // Sparse AutoProduceRecordCopies:1297-1320 uses Header.MaxId+1. Missing,
    // zero and out-of-range sources/destinations skip; forward copies are not
    // retried. Self-copies still have the reference's undefined read and fail.
    for index in 0..section.copy_table_count as usize {
        let at = add(copies_at, index * 8)?;
        let new = u32_at(data, at)?;
        let source = u32_at(data, add(at, 4)?)?;
        if source == 0 || source >= extent || new >= extent {
            continue;
        }
        let Some(fields) = rows.get(&source) else {
            continue;
        };
        if source == new {
            return Err(RecordError::Unsupported);
        }
        let cloned_text = fields.iter().flatten().try_fold(0_usize, |total, value| {
            let bytes = match value {
                Value::Text(text) => text.len(),
                _ => 0,
            };
            total.checked_add(bytes).ok_or(RecordError::Limit)
        })?;
        text_budget.charge(cloned_text)?;
        if !rows.contains_key(&new) {
            value_count = value_count
                .checked_add(row_values)
                .filter(|n| *n <= value_limit)
                .ok_or(RecordError::Limit)?;
        }
        let fields = fields.clone();
        rows.insert(new, fields);
    }
    let end = if h.parent_lookup_count == 1 {
        if !appended_parent || section.parent_lookup_data_size < 12 {
            return Err(RecordError::Bounds);
        }
        let parent_count = u32_at(data, end)?;
        if parent_count > 100_000 {
            return Err(RecordError::Limit);
        }
        let parent_bytes = count_bytes(parent_count, 8)?
            .checked_add(12)
            .ok_or(RecordError::Bounds)?;
        if section.parent_lookup_data_size as usize != parent_bytes {
            return Err(RecordError::Bounds);
        }
        add(end, parent_bytes)?
    } else {
        if section.parent_lookup_data_size != 0 {
            return Err(RecordError::Bounds);
        }
        end
    };
    if end != data.len() {
        return Err(RecordError::Bounds);
    }
    Ok((DecodedTable { rows }, direct_values))
}

#[cfg(test)]
mod tests;
