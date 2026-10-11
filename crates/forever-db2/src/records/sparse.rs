// SPDX-License-Identifier: GPL-3.0-or-later
// Independently authored 2026-10-11 from pinned TrinityCore 02245dcd facts.
//! First sparse subset: external catalog IDs, no parent, scalar U8/U16/U32/I32,
//! no text/64-bit/array values, and zero or one unencrypted section. This does
//! not relax the regular decoder or provide general sparse compatibility.
use super::{
    DecodedTable, RecordError, Value,
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

pub(super) fn read(
    data: &[u8],
    meta: &Metadata,
    schema: &Schema,
    direct_field: Option<usize>,
) -> Result<(DecodedTable, BTreeSet<u32>), RecordError> {
    read_with_limit(data, meta, schema, direct_field, 1_000_000)
}

fn read_with_limit(
    data: &[u8],
    meta: &Metadata,
    schema: &Schema,
    direct_field: Option<usize>,
    value_limit: usize,
) -> Result<(DecodedTable, BTreeSet<u32>), RecordError> {
    let h = meta.header;
    // Sparse AutoProduceData:1055 rejects appended parents even for no rows.
    // The first subset excludes in-record parents and inline IDs as well.
    if schema.parent >= 0
        || schema.index >= 0
        || schema
            .fields
            .iter()
            .any(|kind| !matches!(kind, Kind::Unsigned(8 | 16 | 32, 1) | Kind::Signed(32, 1)))
    {
        return Err(RecordError::Unsupported);
    }
    if h.parent_lookup_count > 1 {
        return Err(RecordError::Unsupported);
    }
    if h.parent_lookup_count != 0 {
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
    let row_values = schema.fields.len();
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
    let mut widths = Vec::with_capacity(row_values);
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
        if matches!(kind, Kind::Unsigned(8, 1)) && unused != 24
            || matches!(kind, Kind::Unsigned(16, 1)) && unused != 16
        {
            return Err(RecordError::Column);
        }
        widths.push((width, 32 - unused as u32));
    }
    let mut rows = BTreeMap::new();
    let mut direct_values = BTreeSet::new();
    if meta.sections.is_empty() {
        if h.record_count != 0 || h.column_meta_size != 0 {
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
        || section.parent_lookup_data_size != 0
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
    // Sparse LoadColumnData:1003 discards columns. The first subset admits
    // absent storage metadata or complete None columns without extra payload.
    if h.column_meta_size != 0 && h.column_meta_size as usize != expected_columns {
        return Err(RecordError::Column);
    }
    let metadata_end = add(primitive_end, h.column_meta_size as usize)?;
    if metadata_end > data.len() {
        return Err(RecordError::Bounds);
    }
    if h.column_meta_size != 0 {
        for field in 0..row_values {
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
    if add(trailing_ids_at, id_bytes)? != data.len() {
        return Err(RecordError::Bounds);
    }
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
        let mut fields = Vec::with_capacity(row_values);
        for (kind, &(width, significant)) in schema.fields.iter().zip(&widths) {
            let end = add(at, width)?;
            let encoded = raw.get(at..end).ok_or(RecordError::Bounds)?;
            let mut word = [0u8; 4];
            word[..width].copy_from_slice(encoded);
            let bits = u32::from_le_bytes(word);
            let value = match kind {
                Kind::Unsigned(32, 1) => Value::Unsigned(bits & (u32::MAX >> (32 - significant))),
                Kind::Signed(32, 1) => {
                    Value::Signed(((bits << (32 - significant)) as i32) >> (32 - significant))
                }
                Kind::Unsigned(_, 1) => Value::Unsigned(bits),
                _ => return Err(RecordError::Unsupported),
            };
            fields.push(vec![value]);
            at = end;
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
        if !rows.contains_key(&new) {
            value_count = value_count
                .checked_add(row_values)
                .filter(|n| *n <= value_limit)
                .ok_or(RecordError::Limit)?;
        }
        let fields = fields.clone();
        rows.insert(new, fields);
    }
    Ok((DecodedTable { rows }, direct_values))
}

#[cfg(test)]
mod tests;
