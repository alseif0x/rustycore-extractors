// SPDX-License-Identifier: GPL-3.0-or-later
// Modified 2026-10-19: add explicit 70450 navigation projection.
//! Bounded regular available-section reader, independent of CASC. Physical
//! extents and global metadata admit before rows. Logical string coordinates
//! retain omitted sections exactly as DB2FileLoaderRegularImpl::LoadTableData;
//! a pointer into an omitted pool fails rather than reading synthesized bytes.
use super::{
    CharacterTable, Kind, RecordError, SPARSE_FLAG, TEXT_POLICY_LIMIT, TextBudget, Value, columns,
    terrain_sources::{self as terrain, Collected, Collector, RecordCopy},
};
use crate::{
    HEADER_SIZE, Header, Metadata, SECTION_SIZE,
    availability::{AvailableRecords, SectionCoverage, SectionState, Source},
};
use std::io::Cursor;

/// Closed crate-private decoder choice. Callers cannot supply an arbitrary
/// catalog string or inherit another target's component admission.
#[derive(Clone, Copy)]
pub(crate) enum Target {
    Build70374,
    Build70425,
    Build70450,
}
impl Target {
    const fn catalog_id(self) -> &'static str {
        match self {
            Self::Build70374 => super::catalog::WOWDBDEFS_70374_CATALOG_ID,
            Self::Build70425 => super::catalog::WOWDBDEFS_70425_CATALOG_ID,
            Self::Build70450 => super::catalog::WOWDBDEFS_70450_CATALOG_ID,
        }
    }
}

const MAX_BYTES: usize = 16 * 1024 * 1024;
const MAX_SECTIONS: usize = 16;
const MAX_ROWS: usize = 100_000;
const MAX_COPIES: usize = 100_000;
const MAX_VALUES: usize = 1_000_000;

struct SectionRange {
    records: usize,
    strings: usize,
    strings_end: usize,
    ids: usize,
    copies: usize,
    end: usize,
    ordinal: usize,
    logical_strings: usize,
}

pub(crate) fn read_map(
    input: &impl Source,
    target: Target,
) -> Result<AvailableRecords<terrain::MapFields>, RecordError> {
    let (rows, sections) = read(input, "Map", Collector::Map(Vec::new()), target)?;
    match rows {
        Collected::Map(records) => Ok(AvailableRecords { records, sections }),
        _ => Err(RecordError::Schema),
    }
}
pub(crate) fn read_vmap_maps(
    input: &impl Source,
    target: Target,
) -> Result<AvailableRecords<terrain::VMapFields>, RecordError> {
    let (rows, sections) = read(input, "Map", Collector::VMap(Vec::new()), target)?;
    match rows {
        Collected::VMap(records) => Ok(AvailableRecords { records, sections }),
        _ => Err(RecordError::Schema),
    }
}
pub(crate) fn read_gameobjects(
    input: &impl Source,
    target: Target,
) -> Result<AvailableRecords<terrain::GameObjectFields>, RecordError> {
    let (rows, sections) = read(
        input,
        "GameObjectDisplayInfo",
        Collector::GameObject(Vec::new()),
        target,
    )?;
    match rows {
        Collected::GameObject(records) => Ok(AvailableRecords { records, sections }),
        _ => Err(RecordError::Schema),
    }
}
pub(crate) fn read_navigation_maps_70450(
    input: &impl Source,
) -> Result<AvailableRecords<terrain::NavigationMapFields>, RecordError> {
    let (rows, sections) = read(
        input,
        "Map",
        Collector::Navigation(Vec::new()),
        Target::Build70450,
    )?;
    match rows {
        Collected::Navigation(records) => Ok(AvailableRecords { records, sections }),
        _ => Err(RecordError::Schema),
    }
}

fn bytes(input: &impl Source, at: usize, len: usize) -> Result<&[u8], RecordError> {
    let end = at.checked_add(len).ok_or(RecordError::Bounds)?;
    if end as u64 > input.declared_bytes() {
        return Err(RecordError::Bounds);
    }
    // Empty admitted sections need no capability read. Still validate the
    // original offset above; an empty out-of-bounds range is not valid.
    if len == 0 {
        return Ok(&[]);
    }
    input
        .range(at as u64, len)
        .map_err(|_| RecordError::Bounds)
        .and_then(|value| {
            if value.len() == len {
                Ok(value)
            } else {
                Err(RecordError::Bounds)
            }
        })
}
fn word(input: &impl Source, at: usize) -> Result<u32, RecordError> {
    Ok(u32::from_le_bytes(bytes(input, at, 4)?.try_into().unwrap()))
}
fn add(a: usize, b: usize) -> Result<usize, RecordError> {
    a.checked_add(b).ok_or(RecordError::Bounds)
}
fn mul(a: usize, b: usize) -> Result<usize, RecordError> {
    a.checked_mul(b).ok_or(RecordError::Bounds)
}

fn read(
    input: &impl Source,
    name: &str,
    mut collector: Collector,
    target: Target,
) -> Result<(Collected, Vec<SectionCoverage>), RecordError> {
    let len = usize::try_from(input.declared_bytes()).map_err(|_| RecordError::Limit)?;
    if len > MAX_BYTES {
        return Err(RecordError::Limit);
    }
    let raw: &[u8; HEADER_SIZE] = bytes(input, 0, HEADER_SIZE)?.try_into().unwrap();
    let h = Header::parse(raw).map_err(|_| RecordError::Metadata)?;
    let table = CharacterTable::from_catalog_id(target.catalog_id(), name)
        .map_err(|_| RecordError::Schema)?
        .ok_or(RecordError::Schema)?;
    let schema = table.schema();
    if !table.matches(&h) {
        return Err(RecordError::Schema);
    }
    if h.flags & SPARSE_FLAG != 0
        || h.parent_lookup_count != 0
        || schema.parent >= 0
        || schema.index >= 0
    {
        return Err(RecordError::Unsupported);
    }
    if h.section_count as usize > MAX_SECTIONS || h.record_count as usize > MAX_ROWS {
        return Err(RecordError::Limit);
    }
    let scalar_count = schema
        .fields
        .iter()
        .try_fold(0usize, |n, k| n.checked_add(k.count()))
        .ok_or(RecordError::Limit)?;
    if mul(h.record_count as usize, scalar_count)? > MAX_VALUES {
        return Err(RecordError::Limit);
    }
    if h.record_size == 0 {
        return Err(RecordError::Bounds);
    }
    let prefix = add(HEADER_SIZE, mul(h.section_count as usize, SECTION_SIZE)?)?;
    let meta = Metadata::read(&mut Cursor::new(bytes(input, 0, prefix)?))
        .map_err(|_| RecordError::Metadata)?;
    let global_end = add(
        add(
            add(
                add(prefix, mul(h.field_count as usize, 4)?)?,
                h.column_meta_size as usize,
            )?,
            h.pallet_data_size as usize,
        )?,
        h.common_data_size as usize,
    )?;
    // One complete available prefix covers all column/pallet/common bytes.
    let global = bytes(input, 0, global_end)?;
    let (cols, mut end) = columns::columns_after_prefix(global, &h, prefix)?;
    if end != global_end {
        return Err(RecordError::Column);
    }
    for (field, kind) in schema.fields.iter().enumerate() {
        if matches!(kind, Kind::String | Kind::StringArray(_)) && cols[field].compression != 0 {
            return Err(RecordError::Column);
        }
    }
    // WDC5 encrypted-ID declarations precede ALL payloads. Even omitted
    // section counts/IDs must be available global metadata; no payload read.
    let mut encrypted_ids = 0usize;
    for section in &meta.sections {
        if section.tact_declared {
            let count = word(input, end)? as usize;
            encrypted_ids = add(encrypted_ids, count)?;
            if encrypted_ids > MAX_ROWS {
                return Err(RecordError::Limit);
            }
            let block = add(4, mul(count, 4)?)?;
            bytes(input, end, block)?;
            end = add(end, block)?;
        }
    }
    let mut ranges = Vec::new();
    let mut coverage = Vec::new();
    ranges
        .try_reserve_exact(meta.sections.len())
        .map_err(|_| RecordError::Limit)?;
    coverage
        .try_reserve_exact(meta.sections.len())
        .map_err(|_| RecordError::Limit)?;
    let (mut rows, mut copies, mut strings) = (0usize, 0usize, 0usize);
    for (ordinal, s) in meta.sections.iter().enumerate() {
        if s.catalog_data_count != 0 || s.catalog_data_offset != 0 || s.parent_lookup_data_size != 0
        {
            return Err(RecordError::Unsupported);
        }
        if s.file_offset as usize != end {
            return Err(RecordError::Bounds);
        }
        let record_ordinal = rows;
        rows = add(rows, s.record_count as usize)?;
        copies = add(copies, s.copy_table_count as usize)?;
        if rows > MAX_ROWS || copies > MAX_COPIES {
            return Err(RecordError::Limit);
        }
        let records = end;
        let string_at = add(
            records,
            mul(s.record_count as usize, h.record_size as usize)?,
        )?;
        let strings_end = add(string_at, s.string_table_size as usize)?;
        if s.id_table_size as usize != mul(s.record_count as usize, 4)? {
            return Err(RecordError::Id);
        }
        let copy_at = add(strings_end, s.id_table_size as usize)?;
        end = add(copy_at, mul(s.copy_table_count as usize, 8)?)?;
        if end > len {
            return Err(RecordError::Bounds);
        }
        ranges.push(SectionRange {
            records,
            strings: string_at,
            strings_end,
            ids: strings_end,
            copies: copy_at,
            end,
            ordinal: record_ordinal,
            logical_strings: strings,
        });
        strings = add(strings, s.string_table_size as usize)?;
        coverage.push(SectionCoverage {
            ordinal: ordinal as u32,
            declared_records: s.record_count,
            declared_copies: s.copy_table_count,
            state: if s.tact_declared {
                SectionState::UnknownTact
            } else {
                SectionState::Admitted
            },
        });
    }
    if end != len || rows != h.record_count as usize || strings != h.string_table_size as usize {
        return Err(RecordError::Bounds);
    }
    // All known section payloads admit before any row decode; an unavailable
    // known section is fatal. Unknown Tact sections are NEVER requested.
    for (fact, range) in coverage.iter().zip(&ranges) {
        if fact.state == SectionState::Admitted {
            bytes(input, range.records, range.end - range.records)?;
        }
    }
    let logical_strings_start = mul(rows, h.record_size as usize)?;
    let mut text = TextBudget::new(TEXT_POLICY_LIMIT);
    let mut raw_copies = Vec::new();
    raw_copies
        .try_reserve_exact(copies)
        .map_err(|_| RecordError::Limit)?;
    for (section_index, (section, range)) in meta.sections.iter().zip(&ranges).enumerate() {
        if coverage[section_index].state != SectionState::Admitted {
            continue;
        }
        for record in 0..section.record_count as usize {
            let ordinal = add(range.ordinal, record)?;
            let raw = bytes(
                input,
                add(range.records, mul(record, h.record_size as usize)?)?,
                h.record_size as usize,
            )?;
            let stored = word(input, add(range.ids, mul(record, 4)?)?)?;
            let id = if stored <= h.min_id {
                h.min_id
                    .checked_add(ordinal as u32)
                    .ok_or(RecordError::Id)?
            } else {
                stored
            };
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
                            if bits == 0 {
                                if collector.requires_non_null_string(field) {
                                    return Err(RecordError::String);
                                }
                                Value::Text(String::new())
                            } else {
                                let pointer = add(
                                    add(
                                        mul(ordinal, h.record_size as usize)?,
                                        cols[field].offset as usize / 8,
                                    )?,
                                    mul(array, 4)?,
                                )?;
                                let logical = add(
                                    pointer,
                                    usize::try_from(bits).map_err(|_| RecordError::String)?,
                                )?
                                .checked_sub(logical_strings_start)
                                .ok_or(RecordError::String)?;
                                Value::Text(read_string(
                                    input, logical, &ranges, &coverage, &mut text,
                                )?)
                            }
                        }
                        Kind::Float(_) => Value::FloatBits(bits as u32),
                        Kind::Signed(64, _) => Value::Signed64(bits as i64),
                        Kind::Signed(width, _) => Value::Signed(match width {
                            8 => bits as u8 as i8 as i32,
                            16 => bits as u16 as i16 as i32,
                            _ => bits as i32,
                        }),
                        Kind::Unsigned(64, _) => Value::Unsigned64(bits),
                        Kind::Unsigned(_, _) => Value::Unsigned(bits as u32),
                    });
                }
                fields.push(values);
            }
            collector.push(id, fields)?;
        }
        for index in 0..section.copy_table_count as usize {
            let at = add(range.copies, mul(index, 8)?)?;
            raw_copies.push(RecordCopy {
                new_id: word(input, at)?,
                source_id: word(input, add(at, 4)?)?,
            });
        }
    }
    Ok((collector.finish(raw_copies, text), coverage))
}

fn read_string(
    input: &impl Source,
    logical: usize,
    ranges: &[SectionRange],
    coverage: &[SectionCoverage],
    budget: &mut TextBudget,
) -> Result<String, RecordError> {
    let mut next = logical;
    let mut value = Vec::new();
    // Strings may span contiguous logical pools, but never an omitted pool.
    loop {
        let (index, range) = ranges
            .iter()
            .enumerate()
            .find(|(_, r)| {
                next >= r.logical_strings && next - r.logical_strings < r.strings_end - r.strings
            })
            .ok_or(RecordError::String)?;
        if coverage[index].state != SectionState::Admitted {
            return Err(RecordError::Bounds);
        }
        let offset = next - range.logical_strings;
        let tail = bytes(
            input,
            add(range.strings, offset)?,
            range.strings_end - range.strings - offset,
        )?;
        let count = tail.iter().position(|b| *b == 0).unwrap_or(tail.len());
        budget.charge(count)?;
        value
            .try_reserve_exact(count)
            .map_err(|_| RecordError::Limit)?;
        value.extend_from_slice(&tail[..count]);
        if count < tail.len() {
            return String::from_utf8(value).map_err(|_| RecordError::String);
        }
        next = add(next, count)?;
    }
}

#[cfg(test)]
mod boundaries {
    use super::*;
    use crate::availability::Unavailable;
    struct Input(Vec<u8>);
    impl Source for Input {
        fn declared_bytes(&self) -> u64 {
            self.0.len() as u64
        }
        fn range(&self, offset: u64, len: usize) -> Result<&[u8], Unavailable> {
            assert_ne!(len, 0, "empty capability read must not occur");
            let start = offset as usize;
            self.0
                .get(start..start.checked_add(len).ok_or(Unavailable)?)
                .ok_or(Unavailable)
        }
    }
    fn region(start: usize, end: usize, logical: usize) -> SectionRange {
        SectionRange {
            records: 0,
            strings: start,
            strings_end: end,
            ids: 0,
            copies: 0,
            end,
            ordinal: 0,
            logical_strings: logical,
        }
    }
    fn fact(state: SectionState) -> SectionCoverage {
        SectionCoverage {
            ordinal: 0,
            declared_records: 0,
            declared_copies: 0,
            state,
        }
    }
    #[test]
    fn available_empty_ranges_validate_bounds_without_source_call() {
        let input = Input(vec![1, 2, 3]);
        assert_eq!(bytes(&input, 3, 0).unwrap(), &[]);
        assert_eq!(bytes(&input, 4, 0), Err(RecordError::Bounds));
        assert_eq!(bytes(&input, usize::MAX, 1), Err(RecordError::Bounds));
    }
    #[test]
    fn available_text_limit_charges_before_value_allocation() {
        let input = Input(b"ab\0".to_vec());
        assert_eq!(
            read_string(
                &input,
                0,
                &[region(0, 3, 0)],
                &[fact(SectionState::Admitted)],
                &mut TextBudget::new(1)
            ),
            Err(RecordError::Limit)
        );
    }
    #[test]
    fn available_text_crosses_only_admitted_logical_pools() {
        let input = Input(b"aXb\0".to_vec());
        let ranges = [region(0, 1, 0), region(2, 4, 1)];
        assert_eq!(
            read_string(
                &input,
                0,
                &ranges,
                &[fact(SectionState::Admitted), fact(SectionState::Admitted)],
                &mut TextBudget::new(2)
            )
            .unwrap(),
            "ab"
        );
        assert_eq!(
            read_string(
                &input,
                0,
                &ranges,
                &[
                    fact(SectionState::Admitted),
                    fact(SectionState::UnknownTact)
                ],
                &mut TextBudget::new(2)
            ),
            Err(RecordError::Bounds)
        );
    }
    #[test]
    fn available_text_utf8_stays_strict() {
        let input = Input(vec![255, 0]);
        assert_eq!(
            read_string(
                &input,
                0,
                &[region(0, 2, 0)],
                &[fact(SectionState::Admitted)],
                &mut TextBudget::new(2)
            ),
            Err(RecordError::String)
        );
    }
}
