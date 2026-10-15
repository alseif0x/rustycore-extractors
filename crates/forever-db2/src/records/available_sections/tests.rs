// SPDX-License-Identifier: GPL-3.0-or-later
//! Authored complete and hole-bearing WDC5 sources. No client or CASC access.
use super::*;
use crate::availability::{self as available, SectionState, Source, Unavailable};
use std::cell::RefCell;

struct Input {
    data: Vec<u8>,
    holes: Vec<std::ops::Range<usize>>,
    reads: RefCell<Vec<std::ops::Range<usize>>>,
}
impl Source for Input {
    fn declared_bytes(&self) -> u64 {
        self.data.len() as u64
    }
    fn range(&self, at: u64, len: usize) -> Result<&[u8], Unavailable> {
        if len == 0 {
            return Err(Unavailable);
        }
        let start = usize::try_from(at).map_err(|_| Unavailable)?;
        let end = start.checked_add(len).ok_or(Unavailable)?;
        if self.holes.iter().any(|h| start < h.end && h.start < end) {
            return Err(Unavailable);
        }
        self.reads.borrow_mut().push(start..end);
        self.data.get(start..end).ok_or(Unavailable)
    }
}
fn table(name: &str) -> CharacterTable {
    CharacterTable::from_catalog_id(crate::catalog::WOWDBDEFS_70374_CATALOG_ID, name)
        .unwrap()
        .unwrap()
}
fn source_spec(name: &str, ids: &[u32], value: u32) -> Spec {
    let mut spec = idtable_spec(table(name), ids.len());
    spec.min_id = 0;
    spec.ids = ids.to_vec();
    spec.max_id = 1; // Physical getter profile does not impose AutoProduceData extent.
    let field = if name == "Map" { 21 } else { 1 };
    let at = field_offset(&spec, field);
    for row in &mut spec.records {
        write_word(row, at, value);
    }
    if name == "Map" {
        for row in 0..ids.len() {
            for field in 0..2 {
                let offset = field_offset(&spec, field);
                let relative = (ids.len() - row) * record_size(&spec.cols) + spec.strings.len()
                    - offset as usize / 8;
                write_word(&mut spec.records[row], offset, relative as u32);
                spec.strings.push(0);
            }
        }
    }
    spec
}
/// All fixture strings use the logical all-records/all-pools address space,
/// including omitted sections. Payload offsets remain physical and contiguous.
fn fixture(name: &str, specs: Vec<Spec>, tact: &[bool]) -> Input {
    assert_eq!(specs.len(), tact.len());
    let singles: Vec<_> = specs.iter().map(|s| assemble(table(name), s)).collect();
    let prefix = HEADER + SECTION * specs.len();
    let old_start = u32::from_le_bytes(singles[0][212..216].try_into().unwrap()) as usize;
    let metadata = &singles[0][HEADER + SECTION..old_start];
    let mut data = singles[0][..HEADER].to_vec();
    data.resize(prefix, 0);
    data.extend_from_slice(metadata);
    for unknown in tact {
        if *unknown {
            data.extend_from_slice(&0u32.to_le_bytes());
        }
    }
    let record_bytes = specs
        .iter()
        .map(|s| s.records.len() * record_size(&s.cols))
        .sum::<usize>();
    let mut ordinal = 0usize;
    let mut string_start = 0usize;
    for (index, ((spec, single), unknown)) in specs.iter().zip(&singles).zip(tact).enumerate() {
        let at = HEADER + index * SECTION;
        data[at..at + SECTION].copy_from_slice(&single[HEADER..HEADER + SECTION]);
        if *unknown {
            data[at] = 1;
        }
        let start = data.len();
        put32(&mut data, at + 8, start as u32);
        let old = u32::from_le_bytes(single[212..216].try_into().unwrap()) as usize;
        let mut body = single[old..].to_vec();
        if name == "Map" {
            for row in 0..spec.records.len() {
                for field in 0..2 {
                    let field_byte = field_offset(spec, field) as usize / 8;
                    let pointer = row * record_size(&spec.cols) + field_byte;
                    let stored =
                        u32::from_le_bytes(body[pointer..pointer + 4].try_into().unwrap()) as usize;
                    let old_target =
                        pointer + stored - spec.records.len() * record_size(&spec.cols);
                    let new_target = record_bytes + string_start + old_target;
                    let logical_pointer = (ordinal + row) * record_size(&spec.cols) + field_byte;
                    put32(&mut body, pointer, (new_target - logical_pointer) as u32);
                }
            }
        }
        data.extend_from_slice(&body);
        ordinal += spec.records.len();
        string_start += spec.strings.len();
    }
    put32(&mut data, 136, ordinal as u32);
    put32(&mut data, 148, string_start as u32);
    put32(&mut data, 200, specs.len() as u32);
    Input {
        data,
        holes: Vec::new(),
        reads: RefCell::new(Vec::new()),
    }
}
fn one(name: &str) -> Input {
    fixture(name, vec![source_spec(name, &[7], 1)], &[false])
}
fn payload(input: &Input, section: usize) -> std::ops::Range<usize> {
    let start = u32::from_le_bytes(
        input.data[HEADER + SECTION * section + 8..HEADER + SECTION * section + 12]
            .try_into()
            .unwrap(),
    ) as usize;
    let end = if section + 1
        == u32::from_le_bytes(input.data[200..204].try_into().unwrap()) as usize
    {
        input.data.len()
    } else {
        u32::from_le_bytes(
            input.data[HEADER + SECTION * (section + 1) + 8..HEADER + SECTION * (section + 1) + 12]
                .try_into()
                .unwrap(),
        ) as usize
    };
    start..end
}
#[test]
fn available_full_map_equals_strict_projection_and_materialization() {
    let mut spec = source_spec("Map", &[7, 8], 1);
    spec.copies = vec![(9, 7), (10, 9), (11, 99)];
    let input = fixture("Map", vec![spec], &[false]);
    let strict = crate::terrain_sources::read_map(&input.data).unwrap();
    let view = available::read_map_70374(&input).unwrap();
    assert_eq!(strict.direct_records(), view.direct_records());
    assert_eq!(strict.copies(), view.copies());
    assert!(view.all_sections_admitted());
    assert_eq!(
        strict.materialize().unwrap(),
        view.materialize().unwrap().rows()
    );
}
#[test]
fn available_full_godi_keeps_direct_order_fdids_and_raw_copies() {
    let mut s = source_spec("GameObjectDisplayInfo", &[7, 7, 0], u32::MAX);
    s.copies = vec![(9, 7)];
    let input = fixture("GameObjectDisplayInfo", vec![s], &[false]);
    let out = available::read_gameobject_display_info_70374(&input).unwrap();
    assert_eq!(
        out.direct_records()
            .iter()
            .map(|r| (r.id, r.fields.file_data_id))
            .collect::<Vec<_>>(),
        [(7, u32::MAX), (7, u32::MAX), (2, u32::MAX)]
    );
    assert_eq!(out.copies().len(), 1);
}
#[test]
fn available_unknown_tact_is_skipped_even_when_payload_is_readable() {
    let input = fixture("Map", vec![source_spec("Map", &[7], 1)], &[true]);
    let out = available::read_map_70374(&input).unwrap();
    assert_eq!(out.sections()[0].state, SectionState::UnknownTact);
    assert!(out.direct_records().is_empty());
    assert!(!out.all_sections_admitted());
    let p = payload(&input, 0);
    assert!(
        input
            .reads
            .borrow()
            .iter()
            .all(|r| r.end <= p.start || r.start >= p.end)
    );
}
#[test]
fn available_unknown_payload_hole_is_never_requested() {
    let mut input = fixture(
        "Map",
        vec![source_spec("Map", &[7], 1), source_spec("Map", &[8], 2)],
        &[true, false],
    );
    let hole = payload(&input, 0);
    input.holes.push(hole.clone());
    let out = available::read_map_70374(&input).unwrap();
    assert_eq!(out.direct_records()[0].id, 8);
    assert!(
        input
            .reads
            .borrow()
            .iter()
            .all(|r| r.end <= hole.start || r.start >= hole.end)
    );
}
#[test]
fn available_skipped_slots_still_repair_global_id_ordinal() {
    let input = fixture(
        "Map",
        vec![source_spec("Map", &[5, 6], 1), source_spec("Map", &[0], 2)],
        &[true, false],
    );
    assert_eq!(
        available::read_map_70374(&input).unwrap().direct_records()[0].id,
        2
    );
}
#[test]
fn available_multiple_known_sections_preserve_physical_order_and_copies() {
    let mut a = source_spec("Map", &[9, 7], 1);
    a.copies = vec![(20, 8)];
    let mut b = source_spec("Map", &[8], 2);
    b.copies = vec![(21, 20), (22, 7)];
    let input = fixture("Map", vec![a, b], &[false, false]);
    let out = available::read_map_70374(&input).unwrap();
    assert_eq!(
        out.direct_records()
            .iter()
            .map(|r| r.id)
            .collect::<Vec<_>>(),
        [9, 7, 8]
    );
    assert_eq!(
        out.materialize()
            .unwrap()
            .rows()
            .iter()
            .map(|r| r.id)
            .collect::<Vec<_>>(),
        [9, 7, 8, 20, 22]
    );
}
#[test]
fn available_copies_from_unknown_sections_and_missing_sources_do_not_materialize() {
    let mut a = source_spec("Map", &[7], 1);
    a.copies = vec![(20, 7)];
    let mut b = source_spec("Map", &[8], 2);
    b.copies = vec![(21, 7), (22, 8)];
    let input = fixture("Map", vec![a, b], &[true, false]);
    let out = available::read_map_70374(&input).unwrap();
    assert_eq!(out.copies().len(), 2);
    assert_eq!(
        out.materialize()
            .unwrap()
            .rows()
            .iter()
            .map(|r| r.id)
            .collect::<Vec<_>>(),
        [8, 22]
    );
}
#[test]
fn available_header_hole_fails_before_payload() {
    let mut input = one("Map");
    input.holes.push(0..1);
    assert_eq!(
        available::read_map_70374(&input).err(),
        Some(RecordError::Bounds)
    );
    assert!(input.reads.borrow().is_empty());
}
#[test]
fn available_section_header_and_column_holes_are_fatal() {
    for at in [HEADER, HEADER + SECTION + 4 * 26] {
        let mut input = one("Map");
        input.holes.push(at..at + 1);
        assert_eq!(
            available::read_map_70374(&input).err(),
            Some(RecordError::Bounds)
        );
    }
}
#[test]
fn available_encrypted_id_declaration_and_ids_must_be_available() {
    let mut input = fixture("Map", vec![source_spec("Map", &[7], 1)], &[true]);
    let start = payload(&input, 0).start - 4;
    input.holes.push(start..start + 4);
    assert_eq!(
        available::read_map_70374(&input).err(),
        Some(RecordError::Bounds)
    );
}
#[test]
fn available_known_section_hole_is_fatal_before_any_row_decode() {
    for field in [0, 1] {
        let mut input = one("Map");
        let p = payload(&input, 0);
        let at = if field == 0 { p.start } else { p.end - 1 };
        input.holes.push(at..at + 1);
        assert_eq!(
            available::read_map_70374(&input).err(),
            Some(RecordError::Bounds)
        );
    }
}
#[test]
fn available_logical_string_into_omitted_pool_fails_without_touching_it() {
    let mut input = fixture(
        "Map",
        vec![source_spec("Map", &[7], 1), source_spec("Map", &[8], 2)],
        &[false, true],
    );
    let at = payload(&input, 0).start;
    let record_size = u32::from_le_bytes(input.data[144..148].try_into().unwrap()) as usize;
    put32(&mut input.data, at, (2 * record_size + 2) as u32);
    let hole = payload(&input, 1);
    input.holes.push(hole.clone());
    assert_eq!(
        available::read_map_70374(&input).err(),
        Some(RecordError::Bounds)
    );
    assert!(
        input
            .reads
            .borrow()
            .iter()
            .all(|r| r.end <= hole.start || r.start >= hole.end)
    );
}
#[test]
fn available_logical_string_can_target_later_admitted_pool() {
    let mut input = fixture(
        "Map",
        vec![source_spec("Map", &[7], 1), source_spec("Map", &[8], 2)],
        &[false, false],
    );
    let at = payload(&input, 0).start;
    let record_size = u32::from_le_bytes(input.data[144..148].try_into().unwrap()) as usize;
    put32(&mut input.data, at, (2 * record_size + 2) as u32);
    assert!(available::read_map_70374(&input).is_ok());
}
#[test]
fn available_unknown_tact_declared_records_and_copies_charge_caps() {
    for (at, value) in [
        (136, 100_001),
        (HEADER + 36, 100_001),
        (HEADER + 12, 100_001),
    ] {
        let mut input = fixture("Map", vec![source_spec("Map", &[7], 1)], &[true]);
        put32(&mut input.data, at, value);
        assert_eq!(
            available::read_map_70374(&input).err(),
            Some(RecordError::Limit)
        );
    }
}
#[test]
fn available_schema_hash_and_sparse_parent_gates_reject() {
    for (at, value, error) in [
        (152, 0, RecordError::Schema),
        (156, 0, RecordError::Schema),
        (184, 1, RecordError::Unsupported),
    ] {
        let mut input = one("Map");
        put32(&mut input.data, at, value);
        assert_eq!(available::read_map_70374(&input).err(), Some(error));
    }
    let mut input = one("Map");
    put16(&mut input.data, 172, 1);
    assert_eq!(
        available::read_map_70374(&input).err(),
        Some(RecordError::Unsupported)
    );
}
#[test]
fn available_framing_overlap_trailing_and_count_mismatches_reject() {
    for at in [HEADER + 8, HEADER + 20, HEADER + 24, 136, 148] {
        let mut input = one("Map");
        let value = u32::from_le_bytes(input.data[at..at + 4].try_into().unwrap());
        put32(&mut input.data, at, value + 1);
        assert!(available::read_map_70374(&input).is_err());
    }
    let mut input = one("Map");
    input.data.push(0);
    assert_eq!(
        available::read_map_70374(&input).err(),
        Some(RecordError::Bounds)
    );
}
#[test]
fn available_map_null_and_unterminated_strings_reject() {
    let mut input = one("Map");
    let at = payload(&input, 0).start;
    put32(&mut input.data, at, 0);
    assert_eq!(
        available::read_map_70374(&input).err(),
        Some(RecordError::String)
    );
    let mut input = one("Map");
    let size = u32::from_le_bytes(input.data[144..148].try_into().unwrap()) as usize;
    let at = payload(&input, 0).start + size;
    input.data[at] = b'x';
    input.data[at + 1] = b'x';
    assert_eq!(
        available::read_map_70374(&input).err(),
        Some(RecordError::String)
    );
}
#[test]
fn available_wrong_successful_slice_length_rejects() {
    struct Short(Vec<u8>);
    impl Source for Short {
        fn declared_bytes(&self) -> u64 {
            self.0.len() as u64
        }
        fn range(&self, _: u64, _: usize) -> Result<&[u8], Unavailable> {
            Ok(&self.0[..1])
        }
    }
    assert_eq!(
        available::read_map_70374(&Short(one("Map").data)).err(),
        Some(RecordError::Bounds)
    );
}
#[test]
fn available_declared_size_and_section_limits_reject_before_allocation() {
    struct Huge;
    impl Source for Huge {
        fn declared_bytes(&self) -> u64 {
            u64::MAX
        }
        fn range(&self, _: u64, _: usize) -> Result<&[u8], Unavailable> {
            panic!("must not read")
        }
    }
    assert_eq!(
        available::read_map_70374(&Huge).err(),
        Some(RecordError::Limit)
    );
    let mut input = one("Map");
    put32(&mut input.data, 200, 17);
    assert_eq!(
        available::read_map_70374(&input).err(),
        Some(RecordError::Limit)
    );
}
#[test]
fn available_default_strict_reader_still_rejects_encrypted_and_multisection() {
    let input = fixture("Map", vec![source_spec("Map", &[7], 1)], &[true]);
    assert_eq!(
        crate::terrain_sources::read_map(&input.data).err(),
        Some(RecordError::Unsupported)
    );
    let input = fixture(
        "Map",
        vec![source_spec("Map", &[7], 1), source_spec("Map", &[8], 2)],
        &[false, false],
    );
    assert_eq!(
        crate::terrain_sources::read_map(&input.data).err(),
        Some(RecordError::Unsupported)
    );
}
#[test]
fn available_encrypted_id_list_is_metadata_not_payload_and_holes_fail() {
    let mut input = fixture("Map", vec![source_spec("Map", &[7], 1)], &[true]);
    let p = payload(&input, 0).start;
    put32(&mut input.data, p - 4, 2);
    input
        .data
        .splice(p..p, [11u32.to_le_bytes(), 12u32.to_le_bytes()].concat());
    put32(&mut input.data, HEADER + 8, (p + 8) as u32);
    assert!(available::read_map_70374(&input).is_ok());
    input.holes.push(p..p + 1);
    assert_eq!(
        available::read_map_70374(&input).err(),
        Some(RecordError::Bounds)
    );
}
#[test]
fn available_nonzero_generated_marker_is_not_a_key_admission() {
    let mut input = fixture("Map", vec![source_spec("Map", &[7], 1)], &[true]);
    input.data[HEADER..HEADER + 8].fill(255);
    let out = available::read_map_70374(&input).unwrap();
    assert!(out.direct_records().is_empty());
    assert_eq!(out.sections()[0].state, SectionState::UnknownTact);
}
#[test]
fn available_known_copy_metadata_hole_is_fatal() {
    let mut s = source_spec("Map", &[7], 1);
    s.copies = vec![(8, 7)];
    let mut input = fixture("Map", vec![s], &[false]);
    let len = input.data.len();
    input.holes.push(len - 8..len);
    assert_eq!(
        available::read_map_70374(&input).err(),
        Some(RecordError::Bounds)
    );
}

#[test]
fn available_known_empty_section_accepts_source_that_rejects_empty_reads() {
    let input = fixture("Map", vec![source_spec("Map", &[], 1)], &[false]);
    let out = available::read_map_70374(&input).unwrap();
    assert!(out.all_sections_admitted());
    assert!(out.direct_records().is_empty());
    assert!(out.copies().is_empty());
    assert_eq!(out.sections()[0].declared_records, 0);
    assert!(input.range(input.data.len() as u64, 0).is_err());
}
