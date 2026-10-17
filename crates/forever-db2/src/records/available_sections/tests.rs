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

fn vmap_short(spec: &mut Spec, row: usize, field: usize, value: i16) {
    let at = field_offset(spec, field) as usize / 8;
    spec.records[row][at..at + 2].copy_from_slice(&value.to_le_bytes());
}
fn vmap_text(spec: &mut Spec, row: usize, field: usize, value: &str) {
    let at = field_offset(spec, field);
    let relative =
        (spec.records.len() - row) * record_size(&spec.cols) + spec.strings.len() - at as usize / 8;
    write_word(&mut spec.records[row], at, relative as u32);
    spec.strings.extend_from_slice(value.as_bytes());
    spec.strings.push(0);
}

#[test]
fn available_vmap_complete_matches_existing_strict_projection() {
    let mut spec = source_spec("Map", &[7, 8], 0);
    vmap_short(&mut spec, 0, 13, -1);
    vmap_short(&mut spec, 0, 14, -2);
    vmap_short(&mut spec, 1, 13, 7);
    vmap_text(&mut spec, 0, 0, "Dír\\logical");
    vmap_text(&mut spec, 0, 1, "Nombre\té\n");
    spec.copies = vec![(9, 7), (10, 9), (11, 99)];
    let input = fixture("Map", vec![spec], &[false]);
    let strict = crate::terrain_sources::read_vmap_maps(&input.data).unwrap();
    let view = available::read_vmap_maps_70374(&input).unwrap();
    assert!(strict.direct_records() == view.direct_records());
    assert_eq!(strict.copies(), view.copies());
    assert_eq!(strict.decoded_utf8_bytes(), view.decoded_utf8_bytes());
    let rows = view.materialize().unwrap();
    assert!(strict.materialize().unwrap() == rows.rows());
    assert_eq!(rows.direct_records().len(), 2);
    assert_eq!(rows.rows().len(), 3);
    assert_eq!(rows.rows()[0].fields.directory, "Dír\\logical");
    assert_eq!(rows.rows()[0].fields.name, "Nombre\té\n");
    assert!(rows.all_sections_admitted());
}

#[test]
fn available_vmap_signed_effective_parent_extremes_survive() {
    let cases = [
        (-1, -1, -1),
        (-1, 0, 0),
        (-1, 7, 7),
        (0, 7, 0),
        (7, 9, 7),
        (i16::MAX, i16::MIN, i16::MAX),
        (i16::MIN, i16::MAX, i16::MAX),
        (-2, i16::MIN, i16::MIN),
    ];
    let ids: Vec<_> = (1..=cases.len() as u32).collect();
    let mut spec = source_spec("Map", &ids, 0);
    for (row, (parent, cosmetic, _)) in cases.iter().copied().enumerate() {
        vmap_short(&mut spec, row, 13, parent);
        vmap_short(&mut spec, row, 14, cosmetic);
    }
    let input = fixture("Map", vec![spec], &[false]);
    let rows = available::read_vmap_maps_70374(&input)
        .unwrap()
        .materialize()
        .unwrap();
    assert_eq!(rows.direct_records().len(), cases.len());
    for (row, (_, _, expected)) in rows.rows().iter().zip(cases) {
        assert_eq!(row.fields.parent_map_id, expected);
        assert_eq!(row.fields.wdt_file_data_id, 0);
    }
}

#[test]
fn available_vmap_copy_prefix_keeps_wdt_zero_and_last_direct_source() {
    let mut spec = source_spec("Map", &[0, 7, 9, 7, 11], 0);
    let wdt_at = field_offset(&spec, 21);
    for (row, (wdt, parent)) in [(10, -1), (20, 3), (30, 4), (40, 5), (0, 6)]
        .into_iter()
        .enumerate()
    {
        write_word(&mut spec.records[row], wdt_at, wdt);
        vmap_short(&mut spec, row, 13, parent);
    }
    spec.copies = vec![(100, 7), (101, 100), (7, 7), (0, 11), (90, 99), (55, 0)];
    let input = fixture("Map", vec![spec], &[false]);
    let view = available::read_vmap_maps_70374(&input).unwrap();
    assert_eq!(view.copies().len(), 6);
    let rows = view.materialize().unwrap();
    assert_eq!(
        rows.rows().iter().map(|r| r.id).collect::<Vec<_>>(),
        [0, 7, 9, 7, 11, 100, 7, 0, 55]
    );
    assert_eq!(rows.direct_records().len(), 5);
    assert_eq!(rows.rows()[5].fields.wdt_file_data_id, 40);
    assert_eq!(rows.rows()[5].fields.parent_map_id, 5);
    assert_eq!(rows.rows()[7].fields.wdt_file_data_id, 0);
    assert_eq!(rows.rows()[7].fields.parent_map_id, 6);
}

#[test]
fn available_vmap_keeps_zero_wdt_ancestors_without_changing_terrain() {
    let mut spec = source_spec("Map", &[7], 0);
    spec.copies = vec![(8, 7)];
    let input = fixture("Map", vec![spec], &[false]);
    let vmap = available::read_vmap_maps_70374(&input)
        .unwrap()
        .materialize()
        .unwrap();
    assert_eq!(vmap.rows().len(), 2);
    assert_eq!(vmap.direct_records().len(), 1);
    assert!(
        available::read_map_70374(&input)
            .unwrap()
            .materialize()
            .unwrap()
            .rows()
            .is_empty()
    );
}

#[test]
fn available_vmap_unknown_section_and_copies_remain_attached_after_replay() {
    let mut omitted = source_spec("Map", &[7], 0);
    omitted.copies = vec![(20, 8)];
    let mut admitted = source_spec("Map", &[8], 2);
    vmap_short(&mut admitted, 0, 13, 7);
    admitted.copies = vec![(21, 7), (22, 8)];
    let mut input = fixture("Map", vec![omitted, admitted], &[true, false]);
    let hole = payload(&input, 0);
    input.holes.push(hole.clone());
    let view = available::read_vmap_maps_70374(&input).unwrap();
    assert_eq!(view.copies().len(), 2);
    let coverage = view.sections().to_vec();
    let rows = view.materialize().unwrap();
    assert_eq!(rows.sections(), coverage);
    assert_eq!(rows.sections()[0].declared_copies, 1);
    assert_eq!(rows.sections()[1].declared_copies, 2);
    assert_eq!(rows.sections()[0].state, SectionState::UnknownTact);
    assert!(!rows.all_sections_admitted());
    assert_eq!(rows.direct_records().len(), 1);
    assert_eq!(
        rows.rows().iter().map(|r| r.id).collect::<Vec<_>>(),
        [8, 22]
    );
    assert_eq!(rows.rows()[0].fields.parent_map_id, 7);
    assert!(
        input
            .reads
            .borrow()
            .iter()
            .all(|r| r.end <= hole.start || r.start >= hole.end)
    );
}

#[test]
fn available_vmap_unknown_readable_payload_never_becomes_source() {
    let input = fixture("Map", vec![source_spec("Map", &[7], 0)], &[true]);
    let rows = available::read_vmap_maps_70374(&input)
        .unwrap()
        .materialize()
        .unwrap();
    assert!(rows.rows().is_empty());
    assert!(rows.direct_records().is_empty());
    assert!(!rows.all_sections_admitted());
    let omitted = payload(&input, 0);
    assert!(input.reads.borrow().iter().all(|r| r.end <= omitted.start));
}

#[test]
fn available_vmap_preserves_signed_wdt_bits_and_duplicate_direct_order() {
    let mut spec = source_spec("Map", &[7, 7, 9, 10], 0);
    let at = field_offset(&spec, 21);
    for (row, bits) in [0, i32::MAX, i32::MIN, -1].into_iter().enumerate() {
        write_word(&mut spec.records[row], at, bits as u32);
    }
    let input = fixture("Map", vec![spec], &[false]);
    let rows = available::read_vmap_maps_70374(&input)
        .unwrap()
        .materialize()
        .unwrap();
    assert_eq!(
        rows.rows()
            .iter()
            .map(|r| (r.id, r.fields.wdt_file_data_id))
            .collect::<Vec<_>>(),
        [(7, 0), (7, i32::MAX), (9, i32::MIN), (10, -1)]
    );
}

#[test]
fn available_vmap_repaired_ids_keep_omitted_global_ordinal() {
    let input = fixture(
        "Map",
        vec![source_spec("Map", &[5, 6], 0), source_spec("Map", &[0], 0)],
        &[true, false],
    );
    let rows = available::read_vmap_maps_70374(&input)
        .unwrap()
        .materialize()
        .unwrap();
    assert_eq!(rows.direct_records()[0].id, 2);
}

#[test]
fn available_vmap_logical_text_after_omitted_pool_is_not_shifted() {
    let mut omitted = source_spec("Map", &[7], 0);
    vmap_text(&mut omitted, 0, 0, "omitted-directory");
    vmap_text(&mut omitted, 0, 1, "omitted-name");
    let mut admitted = source_spec("Map", &[8], 0);
    vmap_text(&mut admitted, 0, 0, "Dír");
    vmap_text(&mut admitted, 0, 1, "Nombre");
    let mut input = fixture("Map", vec![omitted, admitted], &[true, false]);
    input.holes.push(payload(&input, 0));
    let view = available::read_vmap_maps_70374(&input).unwrap();
    assert_eq!(view.direct_records()[0].fields.directory, "Dír");
    assert_eq!(view.direct_records()[0].fields.name, "Nombre");
    assert_eq!(view.decoded_utf8_bytes(), "DírNombre".len());
}

#[test]
fn available_vmap_selected_or_ignored_text_into_omitted_pool_is_terminal() {
    for field in [0, 2] {
        let spec = source_spec("Map", &[7], 0);
        let field_byte = field_offset(&spec, field) as usize / 8;
        let mut input = fixture(
            "Map",
            vec![spec, source_spec("Map", &[8], 0)],
            &[false, true],
        );
        let at = payload(&input, 0).start + field_byte;
        let size = u32::from_le_bytes(input.data[144..148].try_into().unwrap()) as usize;
        put32(&mut input.data, at, (2 * size + 2 - field_byte) as u32);
        let hole = payload(&input, 1);
        input.holes.push(hole.clone());
        assert_eq!(
            available::read_vmap_maps_70374(&input).err(),
            Some(RecordError::Bounds)
        );
        assert!(input.reads.borrow().iter().all(|r| r.end <= hole.start));
    }
}

#[test]
fn available_vmap_header_columns_and_encrypted_ids_are_required_metadata() {
    for at in [0, HEADER, HEADER + SECTION + 4 * 26] {
        let mut input = one("Map");
        input.holes.push(at..at + 1);
        assert_eq!(
            available::read_vmap_maps_70374(&input).err(),
            Some(RecordError::Bounds)
        );
    }
    let mut input = fixture("Map", vec![source_spec("Map", &[7], 0)], &[true]);
    let at = payload(&input, 0).start - 4;
    input.holes.push(at..at + 4);
    assert_eq!(
        available::read_vmap_maps_70374(&input).err(),
        Some(RecordError::Bounds)
    );
}

#[test]
fn available_vmap_missing_known_records_strings_ids_or_copies_is_terminal() {
    let mut spec = source_spec("Map", &[7], 0);
    spec.copies = vec![(8, 7)];
    let input = fixture("Map", vec![spec], &[false]);
    let start = payload(&input, 0).start;
    let size = u32::from_le_bytes(input.data[144..148].try_into().unwrap()) as usize;
    for at in [start, start + size, start + size + 2, input.data.len() - 8] {
        let mut input = Input {
            data: input.data.clone(),
            holes: std::iter::once(at..at + 1).collect(),
            reads: RefCell::new(Vec::new()),
        };
        assert_eq!(
            available::read_vmap_maps_70374(&input).err(),
            Some(RecordError::Bounds)
        );
        input.holes.clear();
    }
}

#[test]
fn available_vmap_omitted_rows_copies_and_scalar_limits_still_charge() {
    for (at, value) in [
        (136, 100_001),
        (136, 50_000),
        (HEADER + 36, 100_001),
        (HEADER + 12, 100_001),
    ] {
        let mut input = fixture("Map", vec![source_spec("Map", &[7], 0)], &[true]);
        put32(&mut input.data, at, value);
        assert_eq!(
            available::read_vmap_maps_70374(&input).err(),
            Some(RecordError::Limit)
        );
    }
}

#[test]
fn available_vmap_sparse_and_parent_layouts_remain_unsupported() {
    for (at, value) in [
        (184, 1),
        (HEADER + 20, 1),
        (HEADER + 28, 1),
        (HEADER + 32, 1),
    ] {
        let mut input = one("Map");
        put32(&mut input.data, at, value);
        assert_eq!(
            available::read_vmap_maps_70374(&input).err(),
            Some(RecordError::Unsupported)
        );
    }
    let mut input = one("Map");
    put16(&mut input.data, 172, 1);
    assert_eq!(
        available::read_vmap_maps_70374(&input).err(),
        Some(RecordError::Unsupported)
    );
}

#[test]
fn available_vmap_current_catalog_hashes_are_mandatory() {
    for at in [152, 156] {
        let mut input = one("Map");
        put32(&mut input.data, at, 0);
        assert_eq!(
            available::read_vmap_maps_70374(&input).err(),
            Some(RecordError::Schema)
        );
    }
}

#[test]
fn available_vmap_null_selected_or_invalid_utf8_text_rejects() {
    let mut input = one("Map");
    let at = payload(&input, 0).start;
    put32(&mut input.data, at, 0);
    assert_eq!(
        available::read_vmap_maps_70374(&input).err(),
        Some(RecordError::String)
    );
    let mut input = one("Map");
    let size = u32::from_le_bytes(input.data[144..148].try_into().unwrap()) as usize;
    let at = payload(&input, 0).start + size;
    input.data[at] = 255;
    assert_eq!(
        available::read_vmap_maps_70374(&input).err(),
        Some(RecordError::String)
    );
}

#[test]
fn available_vmap_empty_known_section_keeps_coverage_and_zero_prefix() {
    let input = fixture("Map", vec![source_spec("Map", &[], 0)], &[false]);
    let rows = available::read_vmap_maps_70374(&input)
        .unwrap()
        .materialize()
        .unwrap();
    assert!(rows.rows().is_empty());
    assert!(rows.direct_records().is_empty());
    assert_eq!(rows.sections().len(), 1);
    assert!(rows.all_sections_admitted());
    assert!(rows.resident_bytes().unwrap() >= std::mem::size_of::<available::SectionCoverage>());
}

#[test]
fn available_vmap_resident_accounting_includes_copied_owned_strings() {
    let mut spec = source_spec("Map", &[7], 0);
    vmap_text(&mut spec, 0, 0, "directory");
    vmap_text(&mut spec, 0, 1, "name");
    spec.copies = vec![(8, 7), (9, 7)];
    let input = fixture("Map", vec![spec], &[false]);
    let rows = available::read_vmap_maps_70374(&input)
        .unwrap()
        .materialize()
        .unwrap();
    let strings: usize = rows
        .rows()
        .iter()
        .map(|r| r.fields.name.capacity() + r.fields.directory.capacity())
        .sum();
    let minimum =
        std::mem::size_of_val(rows.rows()) + strings + std::mem::size_of_val(rows.sections());
    assert!(rows.resident_bytes().unwrap() >= minimum);
    assert_eq!(rows.direct_records().len(), 1);
    assert_eq!(rows.rows().len(), 3);
}

#[test]
fn available_vmap_reuses_existing_materializer_downward_row_limit() {
    let mut spec = source_spec("Map", &[7], 0);
    spec.copies = vec![(8, 7)];
    let input = fixture("Map", vec![spec], &[false]);
    let view = available::read_vmap_maps_70374(&input).unwrap();
    assert_eq!(
        view.records.materialize_with_limit(1).err(),
        Some(RecordError::Limit)
    );
}

#[test]
fn available_vmap_complete_api_still_rejects_partial_or_multisection() {
    for tact in [[false, false], [false, true]] {
        let input = fixture(
            "Map",
            vec![source_spec("Map", &[7], 0), source_spec("Map", &[8], 0)],
            &tact,
        );
        assert_eq!(
            crate::terrain_sources::read_vmap_maps(&input.data).err(),
            Some(RecordError::Unsupported)
        );
        assert!(available::read_vmap_maps_70374(&input).is_ok());
    }
}

#[test]
fn available_vmap_section_size_and_successful_short_ranges_reject() {
    struct Huge;
    impl Source for Huge {
        fn declared_bytes(&self) -> u64 {
            u64::MAX
        }
        fn range(&self, _: u64, _: usize) -> Result<&[u8], Unavailable> {
            panic!("size rejects before read")
        }
    }
    assert_eq!(
        available::read_vmap_maps_70374(&Huge).err(),
        Some(RecordError::Limit)
    );
    let mut input = one("Map");
    put32(&mut input.data, 200, 17);
    assert_eq!(
        available::read_vmap_maps_70374(&input).err(),
        Some(RecordError::Limit)
    );
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
        available::read_vmap_maps_70374(&Short(one("Map").data)).err(),
        Some(RecordError::Bounds)
    );
}

#[path = "target_70425_tests.rs"]
mod target_70425_tests;
#[path = "target_70450_tests.rs"]
mod target_70450_tests;
