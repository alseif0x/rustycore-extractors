// SPDX-License-Identifier: GPL-3.0-or-later
//! Independent checked implementation of the bounded regular WDC5 operation.
//! Evidence: TrinityCore 02245dcd DB2FileLoader.cpp::LoadHeaders/Load,
//! DB2FileLoaderRegularImpl::{RecordGetVarInt,RecordGetString,AutoProduceRecordCopies}.
//! Sparse, encrypted and multi-section variants fail explicitly; at most one
//! parent-lookup block is admitted and only for a table whose reviewed metadata
//! declares a parent index field.
mod columns;
mod schema;
#[cfg(test)]
mod tests;
use crate::Metadata;
use columns::columns;
pub use schema::CharacterTable;
use schema::Kind;
use std::{collections::BTreeMap, io::Cursor};

/// Native header flag word. Reference `DB2FileLoader.cpp:1852` is the only
/// discrimination performed: `if (!(_header.Flags & 0x1))` selects the regular
/// implementation, else the sparse one. No other flag bit is validated or
/// assigned a benign meaning. Bit 0 (sparse) is therefore rejected as
/// unsupported; other bits are left alone because the pinned reference ignores
/// them for these tables. This is not a format claim.
const SPARSE_FLAG: u16 = 0x1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordError {
    Metadata,
    Schema,
    Unsupported,
    Bounds,
    Column,
    Id,
    String,
    Limit,
}
impl RecordError {
    pub fn code(self) -> &'static str {
        match self {
            Self::Metadata => "invalid_record_metadata",
            Self::Schema => "unsupported_table_schema",
            Self::Unsupported => "unsupported_record_variant",
            Self::Bounds => "record_data_bounds",
            Self::Column => "invalid_column_data",
            Self::Id => "invalid_record_ids",
            Self::String => "invalid_record_string",
            Self::Limit => "record_policy_limit",
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Unsigned(u32),
    /// An unsigned 64-bit (`FT_LONG`) value, read as 8 little-endian bytes in
    /// compression 0. A separate variant from `Unsigned` so a >32-bit value is
    /// never silently truncated.
    Unsigned64(u64),
    Signed(i32),
    FloatBits(u32),
    Text(String),
}
pub struct DecodedTable {
    rows: BTreeMap<u32, Vec<Vec<Value>>>,
}
impl DecodedTable {
    pub fn rows(&self) -> &BTreeMap<u32, Vec<Vec<Value>>> {
        &self.rows
    }
    /// Validate and decode every logical field, not a projection of selected columns.
    /// Policy: <=16 MiB, 100k direct records, 100k copies, <=1M values, one
    /// unencrypted regular section. All sizes are checked before allocation or
    /// multiplication so 32-bit targets cannot overflow or index out of bounds.
    pub fn read(data: &[u8], table: CharacterTable) -> Result<Self, RecordError> {
        if data.len() > 16 * 1024 * 1024 {
            return Err(RecordError::Limit);
        }
        let meta = Metadata::read(&mut Cursor::new(data)).map_err(|_| RecordError::Metadata)?;
        let h = meta.header;
        let schema = table.schema();
        if !table.matches(&h) {
            return Err(RecordError::Schema);
        }
        if h.flags & SPARSE_FLAG != 0 {
            return Err(RecordError::Unsupported);
        }
        // DB2FileLoader.cpp:1767 rejects `ParentLookupCount > 1`; at most one
        // parent-lookup table is admitted. DB2FileLoader.cpp:1773 then requires
        // the reviewed metadata to agree: a parent-bearing table has a parent
        // index field, so a section that declares a parent block but a schema
        // without one (or vice versa) is a schema mismatch, not a data error.
        if h.parent_lookup_count > 1 {
            return Err(RecordError::Unsupported);
        }
        if (h.parent_lookup_count == 1) != (schema.parent >= 0) {
            return Err(RecordError::Schema);
        }
        if meta.sections.is_empty() {
            // DB2FileLoader.cpp:1778-1779 admits a zero-section header and
            // DB2FileLoader.cpp:1877 then assigns an empty record map: with no
            // section there are no records. The only consistent body is a
            // header declaring zero records; anything else has no offsets to
            // read and is rejected rather than silently dropping data.
            if h.record_count != 0 {
                return Err(RecordError::Unsupported);
            }
            return Ok(Self {
                rows: BTreeMap::new(),
            });
        }
        if meta.sections.len() > 1 {
            return Err(RecordError::Unsupported);
        }
        let s = meta.sections[0];
        if s.tact_declared || s.catalog_data_count != 0 {
            return Err(RecordError::Unsupported);
        }
        // Bound every raw count before it is multiplied: record_count, value
        // count and copy_table_count. Checked product keeps 32-bit usize safe.
        let value_count = (h.record_count as usize)
            .checked_mul(schema.fields.len())
            .ok_or(RecordError::Limit)?;
        if h.record_count > 100_000 || s.copy_table_count > 100_000 || value_count > 1_000_000 {
            return Err(RecordError::Limit);
        }
        if s.record_count != h.record_count
            || s.string_table_size != h.string_table_size
            || h.record_size == 0
            || h.min_id > h.max_id
        {
            return Err(RecordError::Bounds);
        }
        let (cols, metadata_end) = columns(data, &h)?;
        let start = s.file_offset as usize;
        if start != metadata_end {
            return Err(RecordError::Bounds);
        }
        let record_size = h.record_size as usize;
        let records_len = (h.record_count as usize)
            .checked_mul(record_size)
            .ok_or(RecordError::Bounds)?;
        let strings_start = start.checked_add(records_len).ok_or(RecordError::Bounds)?;
        let strings_end = strings_start
            .checked_add(s.string_table_size as usize)
            .ok_or(RecordError::Bounds)?;
        let id_bytes = if schema.index < 0 {
            (h.record_count as usize)
                .checked_mul(4)
                .ok_or(RecordError::Bounds)?
        } else {
            0
        };
        if s.id_table_size as usize != id_bytes {
            return Err(RecordError::Id);
        }
        let copy_start = strings_end
            .checked_add(id_bytes)
            .ok_or(RecordError::Bounds)?;
        let copy_bytes = (s.copy_table_count as usize)
            .checked_mul(8)
            .ok_or(RecordError::Bounds)?;
        let end = copy_start
            .checked_add(copy_bytes)
            .ok_or(RecordError::Bounds)?;
        // Parent lookup block: reference DB2FileLoader.cpp:1767-1776 admits at
        // most one parent-lookup table and DB2FileLoader.cpp:1877 fills the
        // appended parent field from a header of 12 bytes (`NumEntries`,
        // `MinId`, `MaxId`) followed by `NumEntries` 8-byte `{ParentId,
        // RecordIndex}` entries. Only NumEntries is needed here; it bounds the
        // block size, so the remaining header words are only availability
        // checked by the size equality below.
        let mut parent_start = 0usize;
        let mut parent_entries = 0u32;
        if h.parent_lookup_count == 1 {
            let declared = u32_at(data, end)?;
            if declared > 100_000 {
                return Err(RecordError::Limit);
            }
            let parent_bytes = (declared as usize)
                .checked_mul(8)
                .and_then(|bytes| bytes.checked_add(12))
                .ok_or(RecordError::Bounds)?;
            if s.parent_lookup_data_size as usize != parent_bytes {
                return Err(RecordError::Bounds);
            }
            let parent_end = end.checked_add(parent_bytes).ok_or(RecordError::Bounds)?;
            if parent_end != data.len() {
                return Err(RecordError::Bounds);
            }
            parent_start = end;
            parent_entries = declared;
        } else if s.parent_lookup_data_size != 0 || end != data.len() {
            return Err(RecordError::Bounds);
        }
        if schema.index >= 0 && !matches!(cols[schema.index as usize].compression, 0 | 1 | 5) {
            return Err(RecordError::Column);
        }
        let mut rows = BTreeMap::new();
        // Record ordinal -> row id, so the parent-lookup block, whose entries
        // address rows by ordinal, can find the id key the row was inserted
        // under (inline or repaired id-table value).
        let mut ordinal_ids = Vec::with_capacity(h.record_count as usize);
        for record in 0..h.record_count as usize {
            let record_at = record.checked_mul(record_size).ok_or(RecordError::Bounds)?;
            let offset = start.checked_add(record_at).ok_or(RecordError::Bounds)?;
            let record_end = offset.checked_add(record_size).ok_or(RecordError::Bounds)?;
            let raw = data.get(offset..record_end).ok_or(RecordError::Bounds)?;
            let id = if schema.index >= 0 {
                // Inline index field: DB2FileLoader.cpp:747-760 `RecordGetId`
                // returns the stored value unchanged. The `<= MinId` repair at
                // DB2FileLoader.cpp:1958-1967 is applied to the separate id table
                // only, so it is deliberately not used on this path.
                let field = schema.index as usize;
                let bits = cols[field].bits(
                    raw,
                    h.packed_data_offset,
                    0,
                    0,
                    schema.fields[field].bits(),
                    1,
                )?;
                // An inline index field must fit the u32 row key; the reviewed
                // inline index fields are all 32-bit or narrower.
                u32::try_from(bits).map_err(|_| RecordError::Id)?
            } else {
                // Separate id table: DB2FileLoader.cpp:1958-1967 repairs every
                // entry with `id <= MinId` to `MinId + i`. This decoder permits a
                // single section, so the id-table index `i` is the record ordinal.
                let id_at = record
                    .checked_mul(4)
                    .and_then(|at| strings_end.checked_add(at))
                    .ok_or(RecordError::Bounds)?;
                let id = u32_at(data, id_at)?;
                if id <= h.min_id {
                    h.min_id.checked_add(record as u32).ok_or(RecordError::Id)?
                } else {
                    id
                }
            };
            if id > h.max_id || rows.contains_key(&id) {
                return Err(RecordError::Id);
            }
            let mut fields =
                Vec::with_capacity(schema.fields.len() + usize::from(schema.parent >= 0));
            for (field, kind) in schema.fields.iter().enumerate() {
                let count = kind.count();
                let mut values = Vec::with_capacity(count);
                for array in 0..count {
                    let bits = cols[field].bits(
                        raw,
                        h.packed_data_offset,
                        id,
                        array,
                        kind.bits(),
                        count,
                    )?;
                    let value = match *kind {
                        Kind::String => {
                            if cols[field].compression != 0 {
                                return Err(RecordError::Column);
                            }
                            if bits == 0 {
                                Value::Text(String::new())
                            } else {
                                let at = offset
                                    .checked_add(cols[field].offset as usize / 8)
                                    .and_then(|at| at.checked_add(bits as usize))
                                    .ok_or(RecordError::String)?;
                                if !(strings_start..strings_end).contains(&at) {
                                    return Err(RecordError::String);
                                }
                                let tail = data.get(at..strings_end).ok_or(RecordError::String)?;
                                let len = tail
                                    .iter()
                                    .position(|byte| *byte == 0)
                                    .ok_or(RecordError::String)?;
                                Value::Text(
                                    std::str::from_utf8(&tail[..len])
                                        .map_err(|_| RecordError::String)?
                                        .to_owned(),
                                )
                            }
                        }
                        Kind::Float(_) => Value::FloatBits(bits as u32),
                        Kind::Signed(width, _) => Value::Signed(match width {
                            8 => bits as u8 as i8 as i32,
                            16 => bits as u16 as i16 as i32,
                            _ => bits as i32,
                        }),
                        // 64-bit unsigned fields carry the whole value; every
                        // narrower unsigned field stays in `Value::Unsigned`.
                        Kind::Unsigned(64, _) => Value::Unsigned64(bits),
                        Kind::Unsigned(_, _) => Value::Unsigned(bits as u32),
                    };
                    values.push(value);
                }
                fields.push(values);
            }
            // Extra field at the end: a parent-bearing table's `ParentIndexField`
            // is an index into the TOTAL field array. When it is
            // `>= fields.len()` here it sits exactly one past the decoded
            // columns, so grow the row to `schema.parent` and default the append
            // slot to a signed zero. It is later filled from the parent-lookup
            // block at the same ordinal index.
            //
            // When instead `0 <= schema.parent < fields.len()` the parent is an
            // EXISTING decoded column ("in data block"); its already-decoded
            // value is kept until the parent-lookup block overwrites it below.
            if schema.parent >= 0 && (schema.parent as usize) >= schema.fields.len() {
                fields.resize(schema.parent as usize + 1, Vec::new());
                fields[schema.parent as usize] = vec![Value::Signed(0)];
            }
            ordinal_ids.push(id);
            rows.insert(id, fields);
        }
        // Apply the parent lookup after every row exists and before the copy
        // table, so a copied row carries its source's parent value. Entries are
        // `{ParentId, RecordIndex}` (8 bytes, LE); a later duplicate for the
        // same RecordIndex is applied last and wins.
        if h.parent_lookup_count == 1 {
            let entries_at = parent_start.checked_add(12).ok_or(RecordError::Bounds)?;
            for entry in 0..parent_entries as usize {
                let base = entry.checked_mul(8).ok_or(RecordError::Bounds)?;
                let at = entries_at.checked_add(base).ok_or(RecordError::Bounds)?;
                let parent_id = u32_at(data, at)?;
                let record_index = u32_at(data, at.checked_add(4).ok_or(RecordError::Bounds)?)?;
                let rid = *ordinal_ids
                    .get(record_index as usize)
                    .ok_or(RecordError::Id)?;
                let row = rows.get_mut(&rid).ok_or(RecordError::Id)?;
                // Reference DB2FileLoader.cpp:650-712 `FillParentLookup`: for a
                // parent index INSIDE the file field count it writes with the
                // field's own width (FT_BYTE -> u8, FT_SHORT -> u16, FT_INT ->
                // u32) and asserts the value fits; a too-narrow field is
                // `invalid_record_ids`. For the extra-field-at-the-end variant
                // (`parent >= fields.len()`) the value is always a u32 written
                // into the appended slot.
                let value = if (schema.parent as usize) < schema.fields.len() {
                    parent_in_data_value(schema.fields[schema.parent as usize], parent_id)?
                } else {
                    Value::Signed(parent_id as i32)
                };
                row[schema.parent as usize][0] = value;
            }
        }
        // Source AutoProduceRecordCopies (DB2FileLoader.cpp:613-634): a copy is
        // attempted only when the source is a present nonzero record and both the
        // source and new row ids are below the record count. Any other entry is
        // skipped without error, overwriting an existing destination row.
        let record_count = h.record_count as usize;
        for index in 0..s.copy_table_count as usize {
            let base = index.checked_mul(8).ok_or(RecordError::Bounds)?;
            let entry = copy_start.checked_add(base).ok_or(RecordError::Bounds)?;
            let new = u32_at(data, entry)?;
            let source = u32_at(data, entry.checked_add(4).ok_or(RecordError::Bounds)?)?;
            if source == 0 || source as usize >= record_count || new as usize >= record_count {
                continue;
            }
            let Some(mut fields) = rows.get(&source).cloned() else {
                continue;
            };
            // Only a table with an inline index field rewrites the copied id.
            if schema.index >= 0 {
                fields[schema.index as usize][0] = Value::Unsigned(new);
            }
            rows.insert(new, fields);
        }
        Ok(Self { rows })
    }
}
fn u32_at(data: &[u8], at: usize) -> Result<u32, RecordError> {
    Ok(u32::from_le_bytes(
        data.get(at..at.checked_add(4).ok_or(RecordError::Bounds)?)
            .ok_or(RecordError::Bounds)?
            .try_into()
            .unwrap(),
    ))
}
/// Value to write into a parent index field that lives inside the data block,
/// mirroring reference `DB2FileLoader.cpp:650-712`: the write uses the field's
/// own kind/width, and a ParentId that does not fit is `invalid_record_ids`
/// (the reference `ASSERT`). FT_INT and wider fields carry the u32 unchanged.
fn parent_in_data_value(kind: Kind, parent_id: u32) -> Result<Value, RecordError> {
    match kind {
        Kind::Signed(8, _) => u8::try_from(parent_id)
            .map(|value| Value::Signed(value as i8 as i32))
            .map_err(|_| RecordError::Id),
        Kind::Unsigned(8, _) => u8::try_from(parent_id)
            .map(|value| Value::Unsigned(u32::from(value)))
            .map_err(|_| RecordError::Id),
        Kind::Signed(16, _) => u16::try_from(parent_id)
            .map(|value| Value::Signed(value as i16 as i32))
            .map_err(|_| RecordError::Id),
        Kind::Unsigned(16, _) => u16::try_from(parent_id)
            .map(|value| Value::Unsigned(u32::from(value)))
            .map_err(|_| RecordError::Id),
        Kind::Signed(_, _) => Ok(Value::Signed(parent_id as i32)),
        Kind::Unsigned(_, _) => Ok(Value::Unsigned(parent_id)),
        // A string/float parent field is not a reviewed shape; reject rather
        // than guess a write.
        Kind::String | Kind::Float(_) => Err(RecordError::Schema),
    }
}
