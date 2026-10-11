// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original synthetic ADT metadata/height/holes fixtures.
use super::*;

fn put(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}
fn frame(tag: [u8; 4], payload: &[u8]) -> Vec<u8> {
    let mut bytes = tag.into_iter().rev().collect::<Vec<_>>();
    bytes.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    bytes.extend_from_slice(payload);
    bytes
}
fn header(x: u32, y: u32, base: u32) -> Vec<u8> {
    let mut bytes = vec![0; MCNK_PREFIX_BYTES];
    put(&mut bytes, 4, x);
    put(&mut bytes, 8, y);
    put(&mut bytes, 112, base);
    bytes
}
fn cell(prefix: &[u8], children: &[Vec<u8>]) -> Vec<u8> {
    let mut payload = prefix.to_vec();
    for child in children {
        payload.extend(child);
    }
    frame(*b"MCNK", &payload)
}
fn mcvt(values: &[u32]) -> Vec<u8> {
    frame(
        *b"MCVT",
        &values
            .iter()
            .flat_map(|bits| bits.to_le_bytes())
            .collect::<Vec<_>>(),
    )
}
fn constant(bits: u32) -> Vec<u8> {
    mcvt(&[bits; MCVT_VALUES])
}
fn file(cells: &[Vec<u8>]) -> Vec<u8> {
    let mut bytes = frame(*b"MVER", &18u32.to_le_bytes());
    for cell in cells {
        bytes.extend(cell);
    }
    bytes
}
fn parse(bytes: &[u8]) -> Result<Adt<'_>, Error> {
    Adt::parse(bytes, Limits::default())
}
fn low(mask: u32) -> Vec<u8> {
    let mut h = header(2, 7, 0);
    put(&mut h, 60, mask);
    cell(&h, &[])
}
fn high(rows: [u8; 8]) -> Vec<u8> {
    let mut h = header(2, 7, 0);
    put(&mut h, 0, HIGH_RESOLUTION_HOLES);
    h[20..28].copy_from_slice(&rows);
    cell(&h, &[])
}

#[test]
fn mcnk_exact_offsets_preserve_all_prefix_words_and_float_bits() {
    let mut h = header(2, 7, 42f32.to_bits());
    for offset in (0..128).step_by(4) {
        put(&mut h, offset, offset as u32);
    }
    for (offset, value) in [
        (0, 0x80000000),
        (4, 2),
        (8, 7),
        (12, 19),
        (16, 23),
        (20, 0xfffffffe),
        (24, 0xffffffff),
        (52, 70000),
        (60, 0xabcd1234),
        (96, 800),
        (100, 900),
        (104, 0x7fc12345),
        (108, 0xff800000),
        (112, 42f32.to_bits()),
    ] {
        put(&mut h, offset, value);
    }
    let bytes = file(&[cell(&h, &[])]);
    let adt = parse(&bytes).unwrap();
    let cell = &adt.cells()[0];
    let view = cell.header();
    assert_eq!(view.raw(), h);
    assert_eq!((view.flags(), view.ix(), view.iy()), (0x80000000, 2, 7));
    assert_eq!(
        (view.n_layers(), view.n_doodad_refs(), view.area_id()),
        (19, 23, 70000)
    );
    assert_eq!(
        view.low_resolution_offsets(),
        Some([0xfffffffe, 0xffffffff])
    );
    assert!(view.high_resolution_holes().is_none());
    assert_eq!(view.raw_low_resolution_holes(), 0xabcd1234);
    assert_eq!(view.low_resolution_holes(), 0x1234);
    assert_eq!(
        view.position_bits(),
        [0x7fc12345, 0xff800000, 42f32.to_bits()]
    );
    assert_eq!((view.mclq_offset(), view.mclq_size()), (800, 900));
    assert_eq!(cell.offset(), 12);
    assert!(cell.children().all().is_empty());
    assert!(cell.mcvt().is_none());
    assert_eq!(view.raw().as_ptr(), bytes[20..].as_ptr());
    // Non-finite unused x/z positions are metadata, not height inputs.
    assert!(adt.project_heights().is_ok());
}

#[test]
fn base_only_cell_sets_exact_v9_v8_regions_without_mhdr_requirement() {
    let bytes = file(&[cell(&header(2, 7, 42f32.to_bits()), &[])]);
    let adt = parse(&bytes).unwrap();
    let p = adt.project_heights().unwrap();
    assert_eq!(p.v9_bits().len(), 16641);
    assert_eq!(p.v8_bits().len(), 16384);
    assert_eq!(p.hole_bytes().len(), 2048);
    for (x, y) in [(16, 56), (24, 56), (16, 64), (24, 64)] {
        assert_eq!(p.v9_at(x, y), Some(42f32.to_bits()));
    }
    assert_eq!(p.v8_at(16, 56), Some(42f32.to_bits()));
    assert_eq!(p.v8_at(23, 63), Some(42f32.to_bits()));
    for (x, y) in [(15, 56), (25, 64), (24, 55)] {
        assert_eq!(p.v9_at(x, y), Some(0));
    }
    assert_eq!(p.v8_at(24, 63), Some(0));
    assert_eq!(p.v8_at(16, 64), Some(0));
    assert_eq!((p.visited_cells(), p.unique_cells()), (1, 1));
    assert!(p.coverage()[7 * 16 + 2]);
}

#[test]
fn mcvt_staggered_sample_mapping_and_raw_values_are_exact() {
    let values = (0..145).map(|i| (i as f32).to_bits()).collect::<Vec<_>>();
    let bytes = file(&[cell(&header(2, 7, 100f32.to_bits()), &[mcvt(&values)])]);
    let adt = parse(&bytes).unwrap();
    let samples = adt.cells()[0].mcvt().unwrap();
    assert_eq!(samples.raw().len(), 580);
    assert_eq!(samples.bits().collect::<Vec<_>>(), values);
    assert_eq!(samples.value_bits(144), Some(144f32.to_bits()));
    assert!(samples.value_bits(145).is_none());
    assert!(samples.value_bits(usize::MAX).is_none());
    let p = adt.project_heights().unwrap();
    for (x, y, sample) in [(0, 0, 0), (8, 0, 8), (0, 1, 17), (8, 8, 144)] {
        assert_eq!(
            p.v9_at(16 + x, 56 + y),
            Some((100f32 + sample as f32).to_bits())
        );
    }
    for (x, y, sample) in [(0, 0, 9), (7, 0, 16), (0, 1, 26), (7, 7, 135)] {
        assert_eq!(
            p.v8_at(16 + x, 56 + y),
            Some((100f32 + sample as f32).to_bits())
        );
    }
}

#[test]
fn shared_v9_edges_follow_physical_order_in_both_axes() {
    for vertical in [false, true] {
        let first = cell(&header(0, 0, 10f32.to_bits()), &[]);
        let second = cell(
            &header(
                if vertical { 0 } else { 1 },
                if vertical { 1 } else { 0 },
                20f32.to_bits(),
            ),
            &[],
        );
        for reverse in [false, true] {
            let cells = if reverse {
                vec![second.clone(), first.clone()]
            } else {
                vec![first.clone(), second.clone()]
            };
            let bytes = file(&cells);
            let p = parse(&bytes).unwrap().project_heights().unwrap();
            let (x, y) = if vertical { (3, 8) } else { (8, 3) };
            assert_eq!(
                p.v9_at(x, y),
                Some(if reverse { 10f32 } else { 20f32 }.to_bits())
            );
            assert_eq!(p.v8_at(3, 3), Some(10f32.to_bits()));
            assert_eq!(
                p.v8_at(if vertical { 3 } else { 9 }, if vertical { 9 } else { 3 }),
                Some(20f32.to_bits())
            );
        }
    }
}

#[test]
fn duplicate_coordinates_overwrite_heights_and_truncate_area_ids_in_order() {
    let mut first = header(2, 7, 10f32.to_bits());
    put(&mut first, 52, 70000);
    let mut second = header(2, 7, 25f32.to_bits());
    put(&mut second, 52, u32::MAX);
    let cells = [
        cell(&first, &[constant(2f32.to_bits())]),
        cell(&second, &[]),
    ];
    for reverse in [false, true] {
        let bytes = file(&if reverse {
            [cells[1].clone(), cells[0].clone()]
        } else {
            cells.clone()
        });
        let adt = parse(&bytes).unwrap();
        let p = adt.project_heights().unwrap();
        assert_eq!(adt.cells().len(), 2);
        assert_eq!((p.visited_cells(), p.unique_cells()), (2, 1));
        assert_eq!(
            p.v9_at(16, 56),
            Some(if reverse { 12f32 } else { 25f32 }.to_bits())
        );
        assert_eq!(
            p.v8_at(23, 63),
            Some(if reverse { 12f32 } else { 25f32 }.to_bits())
        );
        assert_eq!(
            p.area_ids()[7 * 16 + 2],
            if reverse { 70000u32 as u16 } else { u16::MAX }
        );
    }
}

#[test]
fn low_resolution_holes_expand_two_by_two_and_accumulate_with_or() {
    let bytes = file(&[low(1), low(1 << 15), low(0)]);
    let p = parse(&bytes).unwrap().project_heights().unwrap();
    assert_eq!(p.holes_at(2, 7), Some([3, 3, 0, 0, 0, 0, 0xc0, 0xc0]));
    assert!(p.has_holes());
    let bytes = file(&[low(0xffff0000)]);
    let p = parse(&bytes).unwrap().project_heights().unwrap();
    assert!(!p.has_holes());
    assert_eq!(p.holes_at(2, 7), Some([0; 8]));
}

#[test]
fn high_resolution_holes_use_raw_union_rows_and_ignore_low_mask() {
    let rows = [1, 2, 4, 8, 16, 32, 64, 128];
    let mut h = header(2, 7, 0);
    put(&mut h, 0, HIGH_RESOLUTION_HOLES | 0x80000000);
    h[20..28].copy_from_slice(&rows);
    put(&mut h, 60, u32::MAX);
    let bytes = file(&[cell(&h, &[])]);
    let adt = parse(&bytes).unwrap();
    let header = adt.cells()[0].header();
    assert!(header.low_resolution_offsets().is_none());
    assert_eq!(header.high_resolution_holes(), Some(rows));
    assert_eq!(adt.project_heights().unwrap().holes_at(2, 7), Some(rows));
}

#[test]
fn mixed_hole_duplicates_replace_or_accumulate_and_keep_sticky_flag() {
    for (cells, expected) in [
        (vec![low(1), high([0; 8])], [0; 8]),
        (
            vec![high([0x80; 8]), low(1)],
            [0x83, 0x83, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80],
        ),
        (vec![high([0xff; 8]), high([1; 8])], [1; 8]),
        (vec![high([0xff; 8]), high([0; 8]), low(0)], [0; 8]),
    ] {
        let bytes = file(&cells);
        let p = parse(&bytes).unwrap().project_heights().unwrap();
        assert_eq!(p.holes_at(2, 7), Some(expected));
        assert!(p.has_holes());
    }
    let bytes = file(&[high([0; 8])]);
    assert!(
        !parse(&bytes)
            .unwrap()
            .project_heights()
            .unwrap()
            .has_holes()
    );
}

#[test]
fn all_256_cells_are_bounded_and_use_row_major_orientation() {
    let mut cells = Vec::new();
    for y in 0..16 {
        for x in 0..16 {
            cells.push(cell(
                &header(x, y, ((y * 16 + x + 1) as f32).to_bits()),
                &[],
            ));
        }
    }
    let bytes = file(&cells);
    let adt = parse(&bytes).unwrap();
    let p = adt.project_heights().unwrap();
    assert_eq!(adt.total_chunks(), 257);
    assert_eq!((p.unique_cells(), p.visited_cells()), (256, 256));
    assert!(p.coverage().iter().all(|&present| present));
    assert_eq!(p.v8_at(120, 0), Some(16f32.to_bits()));
    assert_eq!(p.v8_at(0, 120), Some(241f32.to_bits()));
    assert_eq!(p.v9_at(128, 128), Some(256f32.to_bits()));
}

#[test]
fn raw_nonfinite_metadata_is_preserved_but_projection_rejects_used_values() {
    for bits in [0x7f800000, 0xff800000, 0x7fc12345, 0x7f812345] {
        let bytes = file(&[cell(&header(0, 0, bits), &[])]);
        let adt = parse(&bytes).unwrap();
        assert_eq!(adt.cells()[0].header().height_base_bits(), bits);
        assert_eq!(adt.project_heights().err(), Some(Error::NonFiniteHeight));
        let mut values = [0; 145];
        values[144] = bits;
        let bytes = file(&[cell(&header(0, 0, 0), &[mcvt(&values)])]);
        let adt = parse(&bytes).unwrap();
        assert_eq!(adt.cells()[0].mcvt().unwrap().value_bits(144), Some(bits));
        assert_eq!(adt.project_heights().err(), Some(Error::NonFiniteHeight));
    }
}

#[test]
fn finite_inputs_that_overflow_the_f32_sum_are_rejected() {
    let bytes = file(&[cell(
        &header(0, 0, f32::MAX.to_bits()),
        &[constant(f32::MAX.to_bits())],
    )]);
    let adt = parse(&bytes).unwrap();
    assert_eq!(adt.project_heights().err(), Some(Error::NonFiniteHeight));
}

#[test]
fn signed_zero_subnormals_and_round_to_even_match_single_f32_addition() {
    for (base, relative, expected) in [
        (0x80000000, None, 0x80000000),
        (0x80000000, Some(0), 0),
        (0x80000000, Some(0x80000000), 0x80000000),
        (0, Some(0x80000000), 0),
        (1, Some(1), 2),
        (0x3f800000, Some(0x33800000), 0x3f800000),
        (0x3f800000, Some(0x34400000), 0x3f800002),
    ] {
        let children = relative.map(constant).into_iter().collect::<Vec<_>>();
        let bytes = file(&[cell(&header(0, 0, base), &children)]);
        let p = parse(&bytes).unwrap().project_heights().unwrap();
        assert_eq!(p.v9_at(0, 0), Some(expected));
        assert_eq!(p.v8_at(7, 7), Some(expected));
    }
}

#[test]
fn missing_cells_stay_zero_with_explicit_empty_coverage() {
    let bytes = file(&[]);
    let adt = parse(&bytes).unwrap();
    let p = adt.project_heights().unwrap();
    assert!(adt.cells().is_empty());
    assert_eq!(adt.total_chunks(), 1);
    assert!(p.v9_bits().iter().chain(p.v8_bits()).all(|&v| v == 0));
    assert!(p.hole_bytes().iter().all(|&v| v == 0));
    assert!(p.area_ids().iter().all(|&v| v == 0));
    assert_eq!(p.unique_cells(), 0);
    assert!(!p.has_holes());
    assert!(p.v9_at(129, 0).is_none());
    assert!(p.v8_at(0, 128).is_none());
    assert!(p.holes_at(16, 0).is_none());
    assert!(p.v9_at(usize::MAX, 0).is_none());
}

#[test]
fn invalid_cell_coordinates_and_short_prefix_never_escape() {
    for (x, y) in [(16, 0), (0, 16), (u32::MAX, 1), (1, u32::MAX)] {
        let bytes = file(&[cell(&header(x, y, 0), &[])]);
        assert_eq!(parse(&bytes).err(), Some(Error::InvalidCellCoordinates));
    }
    for length in [0, 1, 127] {
        let bytes = file(&[cell(&vec![0; length], &[])]);
        assert_eq!(parse(&bytes).err(), Some(Error::InvalidChunkSize));
    }
}

#[test]
fn mcvt_exact_length_and_uniqueness_are_required() {
    for length in [0, 579, 581] {
        let bytes = file(&[cell(&header(0, 0, 0), &[frame(*b"MCVT", &vec![0; length])])]);
        assert_eq!(parse(&bytes).err(), Some(Error::InvalidChunkSize));
    }
    let bytes = file(&[cell(&header(0, 0, 0), &[constant(0), constant(0)])]);
    assert_eq!(parse(&bytes).err(), Some(Error::DuplicateChunk));
}

#[test]
fn nested_extent_cannot_escape_its_cell_even_if_later_file_bytes_exist() {
    let mut bad = frame(*b"UNKN", &[]);
    put(&mut bad, 4, 200);
    let bytes = file(&[
        cell(&header(0, 0, 0), &[bad]),
        cell(&header(1, 0, 0), &[constant(0)]),
    ]);
    assert_eq!(parse(&bytes).err(), Some(Error::UnsupportedNestedLayout));
    for length in 1..8 {
        let mut prefix = header(0, 0, 0);
        prefix.extend(vec![0; length]);
        let bytes = file(&[cell(&prefix, &[])]);
        assert_eq!(parse(&bytes).err(), Some(Error::UnsupportedNestedLayout));
    }
}

#[test]
fn mcnr_opaque_frames_are_allowed_but_undeclared_13_byte_gaps_are_not() {
    for length in [435, 448] {
        let bytes = file(&[cell(
            &header(0, 0, 1f32.to_bits()),
            &[frame(*b"MCNR", &vec![0; length]), constant(2f32.to_bits())],
        )]);
        let adt = parse(&bytes).unwrap();
        assert_eq!(adt.cells()[0].children().all()[0].payload().len(), length);
        assert_eq!(
            adt.project_heights().unwrap().v9_at(0, 0),
            Some(3f32.to_bits())
        );
    }
    for with_mcvt in [false, true] {
        let mut gap = frame(*b"MCNR", &vec![0; 435]);
        gap.extend([0; 13]);
        if with_mcvt {
            gap.extend(constant(0));
        }
        let bytes = file(&[cell(&header(0, 0, 0), &[gap])]);
        assert_eq!(parse(&bytes).err(), Some(Error::UnsupportedNestedLayout));
    }
}

#[test]
fn opaque_nested_payloads_and_header_offsets_do_not_trigger_scanning() {
    let mut h = header(0, 0, 5f32.to_bits());
    put(&mut h, 20, u32::MAX);
    put(&mut h, 24, u32::MAX);
    let children = [
        frame(*b"UNKN", &constant(100f32.to_bits())),
        frame(*b"MCNK", &constant(200f32.to_bits())),
    ];
    let bytes = file(&[cell(&h, &children)]);
    let adt = parse(&bytes).unwrap();
    assert!(adt.cells()[0].mcvt().is_none());
    assert_eq!(
        adt.project_heights().unwrap().v9_at(0, 0),
        Some(5f32.to_bits())
    );
    let bytes = file(&[frame(*b"UNKN", &cell(&h, &[constant(0)]))]);
    assert!(parse(&bytes).unwrap().cells().is_empty());
}

#[test]
fn child_budget_is_aggregated_across_all_cells_and_top_level_chunks() {
    let child = frame(*b"UNKN", &[]);
    let bytes = file(&[
        cell(&header(0, 0, 0), &[child.clone(), child.clone()]),
        cell(&header(1, 0, 0), &[child.clone(), child]),
    ]);
    let exact = Limits {
        max_file_bytes: bytes.len(),
        max_chunks: 7,
    };
    let adt = Adt::parse(&bytes, exact).unwrap();
    assert_eq!(adt.total_chunks(), 7);
    assert_eq!(
        Adt::parse(
            &bytes,
            Limits {
                max_chunks: 6,
                ..exact
            }
        )
        .err(),
        Some(Error::Limit)
    );
    assert_eq!(
        Adt::parse(
            &bytes,
            Limits {
                max_file_bytes: bytes.len() - 1,
                ..exact
            }
        )
        .err(),
        Some(Error::Limit)
    );
}

#[test]
fn required_version_and_whole_top_level_tail_are_checked() {
    assert_eq!(parse(&[]).err(), Some(Error::MissingChunk));
    let mut bytes = file(&[]);
    bytes.extend(frame(*b"MVER", &18u32.to_le_bytes()));
    assert_eq!(parse(&bytes).err(), Some(Error::DuplicateChunk));
    let bytes = frame(*b"MVER", &[18, 0, 0]);
    assert_eq!(parse(&bytes).err(), Some(Error::InvalidChunkSize));
    let bytes = frame(*b"MVER", &19u32.to_le_bytes());
    assert_eq!(parse(&bytes).err(), Some(Error::UnsupportedVersion));
    let mut bytes = file(&[cell(&header(0, 0, 0), &[])]);
    bytes.push(0);
    assert_eq!(parse(&bytes).err(), Some(Error::Truncated));
}

#[test]
fn adt_errors_are_fixed_without_embedded_payload_or_coordinates() {
    for (error, code) in [
        (
            Error::InvalidCellCoordinates,
            "terrain_invalid_cell_coordinates",
        ),
        (
            Error::UnsupportedNestedLayout,
            "terrain_unsupported_nested_layout",
        ),
        (Error::NonFiniteHeight, "terrain_nonfinite_height"),
    ] {
        assert_eq!(error.code(), code);
        assert_eq!(error.to_string(), code);
    }
}
