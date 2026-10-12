// SPDX-License-Identifier: GPL-3.0-or-later
// Modified 2026-10-11: correct copy-table index semantics, reject unsafe
// reference edge cases, bound retained scalars and cumulative UTF-8 text,
// admit optional parent lookups,
// and require complete primitive metadata for the minimal zero-section file.
//! Independent checked implementation of the bounded regular WDC5 operation.
//! Evidence: TrinityCore 02245dcd DB2FileLoader.cpp::LoadHeaders/Load,
//! DB2FileLoaderRegularImpl::{RecordGetVarInt,RecordGetString,AutoProduceRecordCopies}.
//! A separate branch admits bounded scalar numeric sparse files. Encrypted and
//! multi-section variants fail explicitly in production `read`; a separate
//! opt-in numeric/external-ID API admits bounded regular sections. At most one
//! parent-lookup block is admitted and only for a table whose reviewed metadata
//! declares a parent index field.
pub mod catalog;
mod columns;
mod regular_sections;
mod schema;
mod sparse;
pub mod terrain_sources;
#[cfg(test)]
mod tests;
use crate::Metadata;
use columns::columns;
pub use schema::CharacterTable;
use schema::Kind;
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Cursor,
};

/// Native header flag word. Reference `DB2FileLoader.cpp:1852` is the only
/// discrimination performed: `if (!(_header.Flags & 0x1))` selects the regular
/// implementation, else the sparse one. Bit 0 selects our separately bounded
/// sparse branch. Other bits are left alone because the pinned reference does
/// not use them to select either implementation. This is not a format claim.
const SPARSE_FLAG: u16 = 0x1;

/// Cumulative UTF-8 payload allocated or cloned during one decode. This is a
/// resource policy, not a WDC5 format limit. Overwrites do not refund bytes.
const TEXT_POLICY_LIMIT: usize = 64 * 1024 * 1024;

#[derive(Debug)]
struct TextBudget {
    used: usize,
    limit: usize,
}

impl TextBudget {
    fn new(limit: usize) -> Self {
        Self { used: 0, limit }
    }

    fn charge(&mut self, bytes: usize) -> Result<(), RecordError> {
        self.used = self
            .used
            .checked_add(bytes)
            .filter(|used| *used <= self.limit)
            .ok_or(RecordError::Limit)?;
        Ok(())
    }
}

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
    /// Signed FT_LONG preserves all 64 bits; compressed 64-bit storage remains
    /// unsupported. CommonData leaves upper bytes uninitialized upstream.
    Signed64(i64),
    FloatBits(u32),
    Text(String),
}
pub struct DecodedTable {
    rows: BTreeMap<u32, Vec<Vec<Value>>>,
}
struct ScanResult {
    decoded: DecodedTable,
    direct_values: BTreeSet<u32>,
    terrain: Option<terrain_sources::Collected>,
}
impl DecodedTable {
    /// Separate opt-in numeric multi-section profile; production `read` keeps
    /// its existing single-section policy. This is not client acceptance.
    pub const REGULAR_SECTIONS_PROFILE: &'static str = "forever-db2-regular-sections-numeric-v1";
    /// Policy ceiling, not a WDC5 format limit.
    pub const MAX_REGULAR_SECTIONS: u32 = 16;

    /// Decode 2..=16 regular, unencrypted sections with reviewed numeric
    /// (non-catalog) schemas and external IDs. Strings, catalog and parent data
    /// are excluded; contiguous layout and exact EOF are required. Inline-ID
    /// schemas are outside this profile.
    /// `max_sections` must be 2..=16 and can only reduce the fixed ceiling.
    /// Original 16 MiB / 100k direct / 100k copy / 1M retained scalar limits
    /// apply cumulatively to the complete file. IDs and copies are processed
    /// globally; no partial table is returned on any error. No I/O is performed.
    pub fn read_regular_sections(
        data: &[u8],
        table: CharacterTable,
        max_sections: u32,
    ) -> Result<Self, RecordError> {
        regular_sections::read(data, table, max_sections)
    }

    pub fn rows(&self) -> &BTreeMap<u32, Vec<Vec<Value>>> {
        &self.rows
    }
    /// Validate and decode every logical field, not a projection of selected columns.
    /// Policy: <=16 MiB, 100k direct records, 100k copies, <=1M values, one
    /// unencrypted regular or supported numeric sparse section, and <=64 MiB
    /// cumulative UTF-8 payload.
    /// Every direct string and applicable copy consumes text budget, including
    /// overwritten rows; skipped copies do not. No bytes are refunded. Outputs
    /// under this budget are unchanged. All sizes are checked before allocation
    /// or multiplication so 32-bit targets cannot overflow or index out of bounds.
    pub fn read(data: &[u8], table: CharacterTable) -> Result<Self, RecordError> {
        Self::read_impl(data, table, None, TEXT_POLICY_LIMIT).map(|(decoded, _)| decoded)
    }

    /// Decode the entire table and collect original values of one physical
    /// scalar unsigned 32-bit field from the direct records.
    ///
    /// `physical_field` indexes file fields, not flattened array elements or an
    /// appended parent. Other kinds, widths and array counts return `Schema`.
    /// The ordered set retains zero and deduplicates values before any parent
    /// lookup, record copy or inline-ID rewrite. `rows()` still contains the
    /// usual final logical rows, including all applicable copies.
    ///
    /// Both results are returned only after the complete file passes the same
    /// validation and limits as `read`; no partial collection escapes on error.
    /// The set has at most one entry per direct record (at most 100,000).
    /// This supports the direct-record dependency scan used by pinned
    /// `map_extractor/System.cpp:394-414` for CinematicCamera FileDataID, whose
    /// original value could otherwise be lost when a copy replaces its row.
    pub fn read_with_direct_u32_values(
        data: &[u8],
        table: CharacterTable,
        physical_field: usize,
    ) -> Result<(Self, BTreeSet<u32>), RecordError> {
        Self::read_impl(data, table, Some(physical_field), TEXT_POLICY_LIMIT)
    }

    fn read_impl(
        data: &[u8],
        table: CharacterTable,
        direct_field: Option<usize>,
        text_limit: usize,
    ) -> Result<(Self, BTreeSet<u32>), RecordError> {
        let result = Self::scan_regular_or_logical(data, table, direct_field, text_limit, None)?;
        Ok((result.decoded, result.direct_values))
    }

    // A closed terrain collector stages physical rows without invoking the
    // AutoProduceData materializer. Every field still passes the same decoder.
    // No caller callback or partial snapshot escapes the complete-file check.
    fn scan_regular_or_logical(
        data: &[u8],
        table: CharacterTable,
        direct_field: Option<usize>,
        text_limit: usize,
        mut terrain: Option<terrain_sources::Collector>,
    ) -> Result<ScanResult, RecordError> {
        if data.len() > 16 * 1024 * 1024 {
            return Err(RecordError::Limit);
        }
        let meta = Metadata::read(&mut Cursor::new(data)).map_err(|_| RecordError::Metadata)?;
        let h = meta.header;
        let schema = table.schema();
        if !table.matches(&h) {
            return Err(RecordError::Schema);
        }
        if let Some(field) = direct_field
            && schema.fields.get(field) != Some(&Kind::Unsigned(32, 1))
        {
            return Err(RecordError::Schema);
        }
        let mut direct_values = BTreeSet::new();
        // Public entry points always supply the fixed policy limit. A private
        // parameter permits small synthetic boundary cases without huge input.
        let mut text_budget = TextBudget::new(text_limit);
        if h.flags & SPARSE_FLAG != 0 {
            if terrain.is_some() {
                return Err(RecordError::Unsupported);
            }
            return sparse::read(data, &meta, &schema, direct_field).map(
                |(decoded, direct_values)| ScanResult {
                    decoded,
                    direct_values,
                    terrain: None,
                },
            );
        }
        // DB2FileLoader.cpp:1767 rejects `ParentLookupCount > 1`; at most one
        // parent-lookup table is admitted. DB2FileLoader.cpp:1775 rejects a
        // declared lookup only when the reviewed schema has no parent field.
        // Count zero is valid even with a parent: Load leaves parentIndexes
        // empty (1866-1874), so AutoProduceData keeps an in-record parent or
        // initializes an appended parent to zero (480-510), with no lookup.
        if h.parent_lookup_count > 1 {
            return Err(RecordError::Unsupported);
        }
        if h.parent_lookup_count == 1 && schema.parent < 0 {
            return Err(RecordError::Schema);
        }
        if meta.sections.is_empty() {
            // A minimal zero-section file still contains 4 * FieldCount bytes
            // of primitive field metadata: LoadHeaders reads them regardless
            // of section count (DB2FileLoader.cpp:1782-1784), and Load checks
            // the exact file size (1903-1912). Other empty layouts with storage
            // or string payload are not reviewed here and fail explicitly.
            if h.record_count != 0 {
                return Err(RecordError::Unsupported);
            }
            if h.string_table_size != 0
                || h.column_meta_size != 0
                || h.common_data_size != 0
                || h.pallet_data_size != 0
            {
                return Err(RecordError::Unsupported);
            }
            let expected_len = (h.field_count as usize)
                .checked_mul(4)
                .and_then(|bytes| crate::HEADER_SIZE.checked_add(bytes))
                .ok_or(RecordError::Bounds)?;
            if data.len() != expected_len {
                return Err(RecordError::Bounds);
            }
            return Ok(ScanResult {
                decoded: Self {
                    rows: BTreeMap::new(),
                },
                direct_values,
                terrain: terrain.map(|collector| collector.finish(Vec::new(), text_budget)),
            });
        }
        if meta.sections.len() > 1 {
            return Err(RecordError::Unsupported);
        }
        let s = meta.sections[0];
        if s.tact_declared || s.catalog_data_count != 0 {
            return Err(RecordError::Unsupported);
        }
        // Count scalar array elements and an appended parent, not just columns.
        // Copies are charged below when they add a row; overwrites and skipped
        // copies do not increase the number of retained values.
        let row_values = schema
            .fields
            .iter()
            .try_fold(0usize, |count, kind| count.checked_add(kind.count()))
            .and_then(|count| {
                count.checked_add(usize::from(
                    schema.parent >= 0 && schema.parent as usize >= schema.fields.len(),
                ))
            })
            .ok_or(RecordError::Limit)?;
        let mut value_count = (h.record_count as usize)
            .checked_mul(row_values)
            .ok_or(RecordError::Limit)?;
        if h.record_count > 100_000 || s.copy_table_count > 100_000 || value_count > 1_000_000 {
            return Err(RecordError::Limit);
        }
        if s.record_count != h.record_count
            || s.string_table_size != h.string_table_size
            || h.record_size == 0
            || (terrain.is_none() && h.min_id > h.max_id)
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
            if terrain.is_none() && (id > h.max_id || rows.contains_key(&id)) {
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
                        Kind::String | Kind::StringArray(_) => {
                            if cols[field].compression != 0 {
                                return Err(RecordError::Column);
                            }
                            if bits == 0 {
                                if terrain.as_ref().is_some_and(|collector| {
                                    collector.requires_non_null_string(field)
                                }) {
                                    return Err(RecordError::String);
                                }
                                Value::Text(String::new())
                            } else {
                                let at = offset
                                    .checked_add(cols[field].offset as usize / 8)
                                    // RecordGetString:800 includes each array
                                    // element's own 32-bit relative pointer.
                                    .and_then(|at| {
                                        array.checked_mul(4).and_then(|n| at.checked_add(n))
                                    })
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
                                // Charge before UTF-8 conversion/allocation,
                                // including repeated pointers and array slots.
                                text_budget.charge(len)?;
                                Value::Text(
                                    std::str::from_utf8(&tail[..len])
                                        .map_err(|_| RecordError::String)?
                                        .to_owned(),
                                )
                            }
                        }
                        Kind::Float(_) => Value::FloatBits(bits as u32),
                        Kind::Signed(64, _) => Value::Signed64(bits as i64),
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
            if let Some(field) = direct_field {
                let Value::Unsigned(value) = &fields[field][0] else {
                    return Err(RecordError::Schema);
                };
                direct_values.insert(*value);
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
                fields[schema.parent as usize] = vec![if schema.appended_parent_unsigned {
                    Value::Unsigned(0)
                } else {
                    Value::Signed(0)
                }];
            }
            if let Some(collector) = terrain.as_mut() {
                collector.push(id, fields)?;
            } else {
                ordinal_ids.push(id);
                rows.insert(id, fields);
            }
        }
        if let Some(collector) = terrain {
            // Load/GetRecordCopy (:703-709,1969-1974) returns raw ordered
            // pairs. The terrain consumers never call GetMaxId or allocate an
            // ID index. Zero, missing, self and full-u32 IDs remain operations.
            let mut copies = Vec::new();
            copies
                .try_reserve_exact(s.copy_table_count as usize)
                .map_err(|_| RecordError::Limit)?;
            for index in 0..s.copy_table_count as usize {
                let at = copy_start + index * 8; // whole table checked above
                copies.push(terrain_sources::RecordCopy {
                    new_id: u32_at(data, at)?,
                    source_id: u32_at(data, at + 4)?,
                });
            }
            return Ok(ScanResult {
                decoded: Self { rows },
                direct_values,
                terrain: Some(collector.finish(copies, text_budget)),
            });
        }
        // AutoProduceData allocates GetMaxId() + 1 index slots, not RecordCount
        // slots (reference DB2FileLoader.cpp:394-398). GetMaxId scans direct ids
        // and EVERY copy destination before any copy is attempted, including
        // entries whose source will be skipped (955-977). Validate that extent
        // against Header.MaxId, and fail closed instead of overflowing its u32
        // size. The BTreeMap deliberately avoids allocating by this extent.
        let mut max_id = rows.last_key_value().map_or(0, |(&id, _)| id);
        for index in 0..s.copy_table_count as usize {
            let base = index.checked_mul(8).ok_or(RecordError::Bounds)?;
            let entry = copy_start.checked_add(base).ok_or(RecordError::Bounds)?;
            max_id = max_id.max(u32_at(data, entry)?);
        }
        if max_id > h.max_id {
            return Err(RecordError::Id);
        }
        let index_extent = max_id.checked_add(1).ok_or(RecordError::Id)?;
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
                } else if schema.appended_parent_unsigned {
                    Value::Unsigned(parent_id)
                } else {
                    Value::Signed(parent_id as i32)
                };
                row[schema.parent as usize][0] = value;
            }
        }
        // AutoProduceRecordCopies (DB2FileLoader.cpp:613-634) receives the index
        // extent above. Process copies in order: earlier copies can be sources,
        // missing/zero sources are skipped, and existing destinations are replaced.
        for index in 0..s.copy_table_count as usize {
            let base = index.checked_mul(8).ok_or(RecordError::Bounds)?;
            let entry = copy_start.checked_add(base).ok_or(RecordError::Bounds)?;
            let new = u32_at(data, entry)?;
            let source = u32_at(data, entry.checked_add(4).ok_or(RecordError::Bounds)?)?;
            if source == 0 || source >= index_extent || new >= index_extent {
                continue;
            }
            let Some(source_fields) = rows.get(&source) else {
                continue;
            };
            // The reference assigns the destination pointer before reading the
            // source pointer, making a self-copy read uninitialized storage. It
            // also writes a uint32 id even when the inline field is narrower
            // (ChrClasses), overwriting neighboring fields. Neither operation
            // has a safe reviewed compatibility contract; fail explicitly.
            if source == new
                || (schema.index >= 0 && schema.fields[schema.index as usize].bits() != 32)
            {
                return Err(RecordError::Unsupported);
            }
            if !rows.contains_key(&new) {
                value_count = value_count
                    .checked_add(row_values)
                    .filter(|count| *count <= 1_000_000)
                    .ok_or(RecordError::Limit)?;
            }
            // A clone owns a fresh String for every source text, even when it
            // replaces an existing row. The cumulative charge bounds retained
            // text and repeated overwrite work; skipped sources never reach it.
            let copied_text_bytes = source_fields
                .iter()
                .flatten()
                .try_fold(0usize, |total, value| match value {
                    Value::Text(text) => total.checked_add(text.len()),
                    _ => Some(total),
                })
                .ok_or(RecordError::Limit)?;
            text_budget.charge(copied_text_bytes)?;
            let mut fields = source_fields.clone();
            // Only a table with an inline index field rewrites the copied id.
            if schema.index >= 0 {
                fields[schema.index as usize][0] = Value::Unsigned(new);
            }
            rows.insert(new, fields);
        }
        Ok(ScanResult {
            decoded: Self { rows },
            direct_values,
            terrain: None,
        })
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
        Kind::String | Kind::StringArray(_) | Kind::Float(_) => Err(RecordError::Schema),
    }
}
