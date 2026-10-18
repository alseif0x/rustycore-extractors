// SPDX-License-Identifier: GPL-3.0-or-later
//! Original source-to-container fixtures; no client files or external I/O.
use super::*;

struct Lookup {
    present: bool,
    visual: bool,
}
impl liquid::LiquidLookup for Lookup {
    fn liquid_type(&self, id: u32) -> Option<liquid::LiquidType> {
        (self.present && id == 7).then_some(liquid::LiquidType {
            sound_bank: 0,
            material_id: 3,
        })
    }
    fn material(&self, id: u32) -> Option<liquid::LiquidMaterial> {
        (id == 3).then_some(liquid::LiquidMaterial {
            flags: if self.visual { 2 } else { 0 },
            vertex_format: 0,
        })
    }
}
const LOOKUP: Lookup = Lookup {
    present: true,
    visual: false,
};
fn context() -> Context {
    Context {
        map_id: 2,
        tile_y: 3,
        tile_x: 7,
        build: 70205,
        pack: false,
    }
}
fn word(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap())
}
fn put(bytes: &mut [u8], at: usize, value: u32) {
    bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
}
fn frame(tag: &[u8; 4], payload: &[u8]) -> Vec<u8> {
    let mut out = tag.iter().rev().copied().collect::<Vec<_>>();
    out.extend((payload.len() as u32).to_le_bytes());
    out.extend(payload);
    out
}
fn adt(chunks: &[Vec<u8>]) -> Vec<u8> {
    let mut bytes = frame(b"MVER", &18u32.to_le_bytes());
    for chunk in chunks {
        bytes.extend(chunk);
    }
    bytes
}
fn prefix(x: u32, y: u32, base: f32, area: u32) -> Vec<u8> {
    let mut raw = vec![0; 128];
    put(&mut raw, 4, x);
    put(&mut raw, 8, y);
    put(&mut raw, 52, area);
    put(&mut raw, 112, base.to_bits());
    raw
}
fn cell(prefix: &[u8], children: &[Vec<u8>]) -> Vec<u8> {
    let mut raw = prefix.to_vec();
    for child in children {
        raw.extend(child);
    }
    frame(b"MCNK", &raw)
}
fn mcvt(values: &[u32]) -> Vec<u8> {
    frame(
        b"MCVT",
        &values
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect::<Vec<_>>(),
    )
}
fn old_liquid(tile_flags: u8, value: f32) -> Vec<u8> {
    let mut raw = vec![0; 804];
    for index in 0..81 {
        put(&mut raw, 12 + index * 8, value.to_bits());
    }
    raw[656..720].fill(tile_flags);
    raw
}
fn old_cell(base: f32, raw_liquid: &[u8]) -> Vec<u8> {
    let mut raw = prefix(0, 0, base, 0);
    put(&mut raw, 0, 4); // source MCNK water flag
    put(&mut raw, 100, 812);
    cell(&raw, &[frame(b"MCLQ", raw_liquid)])
}
fn modern_liquid() -> Vec<u8> {
    let mut raw = vec![0; 3072 + 24];
    put(&mut raw, 0, 3072);
    put(&mut raw, 4, 1);
    raw[3072..3074].copy_from_slice(&7u16.to_le_bytes());
    raw[3086] = 1;
    raw[3087] = 1;
    put(&mut raw, 3092, 3096);
    for value in [10f32, 11.0, 12.0, 13.0] {
        raw.extend(value.to_le_bytes());
    }
    raw
}
fn converted(bytes: &[u8]) -> ConvertedTile {
    convert_adt(bytes, &LOOKUP, context(), Limits::default()).unwrap()
}
fn view(tile: &ConvertedTile) -> maps::MapView<'_> {
    maps::verify(tile.bytes(), 70205, maps::MAX_MAP_BYTES).unwrap()
}

#[test]
fn defaults_and_errors_are_bounded_static_and_distinguishable() {
    assert_eq!(
        Limits::default(),
        Limits {
            max_input_bytes: 16 * 1024 * 1024,
            max_chunks: 4096,
            max_output_bytes: 202112
        }
    );
    for (error, code) in [
        (Error::InvalidCoordinates, "conversion_invalid_coordinates"),
        (Error::InvalidLimits, "conversion_invalid_limits"),
        (
            Error::Adt(crate::Error::MissingChunk),
            "terrain_missing_chunk",
        ),
        (
            Error::Height(height::Error::InvalidFlightBounds),
            "height_invalid_flight_bounds",
        ),
        (Error::Liquid(liquid::Error::Bounds), "liquid_bounds"),
        (
            Error::LiquidSection(liquid_section::Error::Limit),
            "liquid_section_policy_limit",
        ),
        (
            Error::Maps(maps::Error::UnsupportedBuild),
            "maps_unsupported_build",
        ),
    ] {
        assert_eq!(error.code(), code);
        assert_eq!(error.to_string(), code);
    }
}

#[test]
fn mver_only_has_literal_empty_map_bytes_and_no_fabricated_coverage() {
    let input = adt(&[]);
    for build in [70170u32, 70205] {
        let tile = convert_adt(
            &input,
            &LOOKUP,
            Context { build, ..context() },
            Limits::default(),
        )
        .unwrap();
        let mut expected = b"MAPS".to_vec();
        for value in [10, build, 44, 8, 52, 16, 0, 0, 0, 0] {
            expected.extend(value.to_le_bytes());
        }
        expected.extend(b"AREA\x01\0\0\0MHGT\x01\0\0\0\0\0\0\0\0\0\0\0");
        assert_eq!(tile.bytes(), expected);
        assert_eq!(tile.stats().visited_cells, 0);
        assert_eq!(tile.stats().unique_cells, 0);
        assert_eq!(tile.stats().missing_mcvt, 0);
        assert_eq!(tile.stats().coverage, [false; 256]);
        assert!(
            !tile.stats().has_liquid && !tile.stats().has_holes && !tile.stats().has_flight_bounds
        );
        assert_eq!(tile.into_bytes(), expected);
    }
}

#[test]
fn context_rejects_out_of_grid_coordinates_and_build_before_parsing() {
    for (x, y) in [(64, 0), (0, 64), (255, 255)] {
        assert_eq!(
            convert_adt(
                &[],
                &LOOKUP,
                Context {
                    tile_x: x,
                    tile_y: y,
                    ..context()
                },
                Limits::default()
            )
            .err(),
            Some(Error::InvalidCoordinates)
        );
    }
    for build in [0, 51943, 70169, u32::MAX] {
        assert_eq!(
            convert_adt(
                &[],
                &LOOKUP,
                Context { build, ..context() },
                Limits::default()
            )
            .err(),
            Some(Error::Maps(maps::Error::UnsupportedBuild))
        );
    }
    assert!(
        convert_adt(
            &adt(&[]),
            &LOOKUP,
            Context {
                map_id: u32::MAX,
                tile_x: 63,
                tile_y: 63,
                ..context()
            },
            Limits::default()
        )
        .is_ok()
    );
}

#[test]
fn build70450_conversion_uses_explicit_format_assumption_and_exact_build_gate() {
    assert_eq!(
        BUILD_70450_FORMAT_ASSUMPTION,
        "build 70450 ADT/MAPS conversion assumes build 70374 terrain wire formats"
    );
    let input = adt(&[]);
    let context_70450 = Context {
        build: 70450,
        ..context()
    };
    let converted = convert_adt_70450(&input, &LOOKUP, context_70450, Limits::default()).unwrap();
    assert_eq!(
        maps::verify_70450(converted.bytes(), 70450, maps::MAX_MAP_BYTES)
            .unwrap()
            .build(),
        70450
    );
    let old = convert_adt_70374(
        &input,
        &LOOKUP,
        Context {
            build: 70374,
            ..context()
        },
        Limits::default(),
    )
    .unwrap();
    let mut expected = old.bytes().to_vec();
    expected[8..12].copy_from_slice(&70450u32.to_le_bytes());
    assert_eq!(converted.bytes(), expected);

    for (target, context_build) in [(70450, 70374), (70374, 70450)] {
        let context_for_target = Context {
            build: context_build,
            ..context()
        };
        let result = if target == 70450 {
            convert_adt_70450(&[], &LOOKUP, context_for_target, Limits::default())
        } else {
            convert_adt_70374(&[], &LOOKUP, context_for_target, Limits::default())
        };
        assert_eq!(
            result.err(),
            Some(Error::Maps(maps::Error::UnsupportedBuild))
        );
    }

    let mut unsupported_mver = input;
    put(&mut unsupported_mver, 8, crate::wdt::FILE_VERSION + 1);
    assert_eq!(
        convert_adt_70450(&unsupported_mver, &LOOKUP, context_70450, Limits::default()).err(),
        Some(Error::Adt(crate::Error::UnsupportedVersion))
    );
}

#[test]
fn limits_cannot_expand_even_for_tiny_or_invalid_inputs() {
    let default = Limits::default();
    for limits in [
        Limits {
            max_input_bytes: default.max_input_bytes + 1,
            ..default
        },
        Limits {
            max_chunks: default.max_chunks + 1,
            ..default
        },
        Limits {
            max_output_bytes: default.max_output_bytes + 1,
            ..default
        },
        Limits {
            max_input_bytes: usize::MAX,
            max_chunks: usize::MAX,
            max_output_bytes: usize::MAX,
        },
    ] {
        assert_eq!(
            convert_adt(&[], &LOOKUP, context(), limits).err(),
            Some(Error::InvalidLimits)
        );
    }
}

#[test]
fn exact_input_aggregate_frame_and_total_output_limits_apply() {
    let input = adt(&[cell(&prefix(0, 0, 0.0, 0), &[mcvt(&[0; 145])])]);
    let exact = Limits {
        max_input_bytes: input.len(),
        max_chunks: 3,
        max_output_bytes: 68,
    };
    assert_eq!(
        convert_adt(&input, &LOOKUP, context(), exact)
            .unwrap()
            .bytes()
            .len(),
        68
    );
    for limits in [
        Limits {
            max_input_bytes: input.len() - 1,
            ..exact
        },
        Limits {
            max_chunks: 2,
            ..exact
        },
        Limits {
            max_chunks: 0,
            ..exact
        },
        Limits {
            max_input_bytes: 0,
            ..exact
        },
    ] {
        assert_eq!(
            convert_adt(&input, &LOOKUP, context(), limits).err(),
            Some(Error::Adt(crate::Error::Limit))
        );
    }
    assert_eq!(
        convert_adt(
            &input,
            &LOOKUP,
            context(),
            Limits {
                max_output_bytes: 67,
                ..exact
            }
        )
        .err(),
        Some(Error::Maps(maps::Error::Limit))
    );
    assert_eq!(
        convert_adt(
            &input,
            &LOOKUP,
            context(),
            Limits {
                max_output_bytes: 0,
                ..exact
            }
        )
        .err(),
        Some(Error::Height(height::Error::Limit))
    );
}

#[test]
fn missing_mcvt_and_staggered_samples_reach_serialized_height_in_y_x_order() {
    let values = (0..145).map(|i| (i as f32).to_bits()).collect::<Vec<_>>();
    let input = adt(&[
        cell(&prefix(2, 7, 10.0, 0x1234abcd), &[mcvt(&values)]),
        cell(&prefix(4, 1, 42.0, 7), &[]),
    ]);
    let tile = converted(&input);
    let map = view(&tile);
    assert_eq!(map.area().id(7 * 16 + 2), Some(0xabcd));
    assert_eq!(map.area().id(20), Some(7));
    assert_eq!(map.height().encoding(), height::Encoding::Float32);
    assert_eq!(
        word(map.height().v9_samples(), (56 * 129 + 16) * 4),
        10f32.to_bits()
    );
    assert_eq!(
        word(map.height().v9_samples(), (64 * 129 + 24) * 4),
        154f32.to_bits()
    );
    assert_eq!(
        word(map.height().v8_samples(), (56 * 128 + 16) * 4),
        19f32.to_bits()
    );
    assert_eq!(
        word(map.height().v8_samples(), (8 * 128 + 32) * 4),
        42f32.to_bits()
    );
    assert_eq!(tile.stats().visited_cells, 2);
    assert_eq!(tile.stats().unique_cells, 2);
    assert_eq!(tile.stats().missing_mcvt, 1);
    assert!(tile.stats().coverage[7 * 16 + 2]);
    assert!(!tile.stats().coverage[2 * 16 + 7]);
}

#[test]
fn repeated_coordinates_and_shared_edges_preserve_physical_order() {
    let input = adt(&[
        cell(&prefix(0, 0, 1.0, 1), &[]),
        cell(&prefix(1, 0, 2.0, 2), &[]),
        cell(&prefix(0, 0, 3.0, 3), &[]),
    ]);
    let tile = converted(&input);
    let map = view(&tile);
    assert_eq!(
        (
            tile.stats().visited_cells,
            tile.stats().unique_cells,
            tile.stats().missing_mcvt
        ),
        (3, 2, 3)
    );
    assert_eq!(map.area().id(0), Some(3));
    assert_eq!(word(map.height().v9_samples(), 8 * 4), 3f32.to_bits());
    assert_eq!(word(map.height().v9_samples(), 9 * 4), 2f32.to_bits());
}

#[test]
fn all_four_height_encodings_are_selected_from_actual_cell_bytes() {
    for (base, encoding) in [
        (0.0, height::Encoding::Flat),
        (1.0, height::Encoding::Uint8),
        (8.0, height::Encoding::Uint16),
        (2048.0, height::Encoding::Float32),
    ] {
        let input = adt(&[cell(&prefix(0, 0, base, 0), &[])]);
        let tile = convert_adt(
            &input,
            &LOOKUP,
            Context {
                pack: true,
                ..context()
            },
            Limits::default(),
        )
        .unwrap();
        assert_eq!(view(&tile).height().encoding(), encoding);
    }
}

#[test]
fn full_cell_coverage_is_reported_without_requiring_it_for_conversion() {
    let mut cells = Vec::new();
    for y in 0..16 {
        for x in 0..16 {
            cells.push(cell(&prefix(x, y, 7.0, 9), &[]));
        }
    }
    let tile = converted(&adt(&cells));
    assert_eq!(tile.stats().coverage, [true; 256]);
    assert_eq!(
        (
            tile.stats().visited_cells,
            tile.stats().unique_cells,
            tile.stats().missing_mcvt
        ),
        (256, 256, 256)
    );
    assert_eq!(view(&tile).height().encoding(), height::Encoding::Flat);
    assert_eq!(view(&tile).area().grid_area(), 9);
}

#[test]
fn low_holes_then_high_clear_keep_the_sticky_serialized_presence() {
    let mut low = prefix(2, 7, 0.0, 0);
    put(&mut low, 60, 1);
    let first = converted(&adt(&[cell(&low, &[])]));
    let holes = view(&first).holes().unwrap();
    assert_eq!(
        &holes[(7 * 16 + 2) * 8..(7 * 16 + 2) * 8 + 8],
        &[3, 3, 0, 0, 0, 0, 0, 0]
    );
    let mut high = prefix(2, 7, 0.0, 0);
    put(&mut high, 0, 0x10000);
    let tile = converted(&adt(&[cell(&low, &[]), cell(&high, &[])]));
    assert!(tile.stats().has_holes);
    assert_eq!(view(&tile).holes(), Some(&[0; 2048]));
    assert_eq!(tile.bytes().len(), 68 + 2048);
}

#[test]
fn mfbo_tag_alone_selects_exact_signed_flight_tail_even_for_flat_height() {
    let values: [i16; 18] = [
        1,
        -2,
        3,
        -4,
        5,
        -6,
        7,
        -8,
        9,
        i16::MIN,
        i16::MAX,
        -1,
        0,
        1,
        2,
        3,
        4,
        5,
    ];
    let payload = values
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .collect::<Vec<_>>();
    let plain = converted(&adt(&[frame(b"MFBO", &payload)]));
    let with_header = converted(&adt(&[
        frame(b"MHDR", &[0xff; 64]),
        frame(b"MFBO", &payload),
    ]));
    assert_eq!(plain.bytes(), with_header.bytes());
    assert!(plain.stats().has_flight_bounds);
    assert_eq!(view(&plain).height().flags(), 9);
    assert_eq!(
        view(&plain).height().flight_bounds().unwrap().as_slice(),
        payload
    );
    let missing = converted(&adt(&[frame(b"MHDR", &[0xff; 64])]));
    assert!(!missing.stats().has_flight_bounds);
    assert_eq!(view(&missing).height().flags(), 1);
}

#[test]
fn mfbo_duplicates_and_wrong_lengths_reject_before_any_result() {
    for size in [0, 35, 37] {
        assert_eq!(
            convert_adt(
                &adt(&[frame(b"MFBO", &vec![0; size])]),
                &LOOKUP,
                context(),
                Limits::default()
            )
            .err(),
            Some(Error::Height(height::Error::InvalidFlightBounds))
        );
    }
    assert_eq!(
        convert_adt(
            &adt(&[frame(b"MFBO", &[0; 36]), frame(b"MFBO", &[0; 36])]),
            &LOOKUP,
            context(),
            Limits::default()
        )
        .err(),
        Some(Error::Adt(crate::Error::DuplicateChunk))
    );
}

#[test]
fn deep_water_exception_uses_context_row_first_and_explicit_map_id() {
    let input = adt(&[old_cell(0.0, &old_liquid(0x80, 7.0))]);
    for (map_id, tile_y, tile_x, ignored) in [
        (0, 39, 24, true),
        (0, 40, 26, true),
        (0, 41, 18, true),
        (0, 46, 26, true),
        (0, 24, 39, false),
        (0, 38, 24, false),
        (0, 40, 23, false),
        (0, 47, 26, false),
        (1, 43, 39, true),
        (1, 43, 40, true),
        (1, 39, 43, false),
        (1, 43, 41, false),
        (2, 43, 39, false),
    ] {
        let tile = convert_adt(
            &input,
            &LOOKUP,
            Context {
                map_id,
                tile_y,
                tile_x,
                ..context()
            },
            Limits::default(),
        )
        .unwrap();
        assert_eq!(tile.stats().ignore_deep_water, ignored);
        let liquid = view(&tile).liquid().unwrap();
        assert_eq!(
            liquid.flag_bytes().unwrap()[0],
            1 | if ignored { 0 } else { 16 }
        );
    }
}

#[test]
fn liquid_raw_heights_do_not_receive_the_terrain_floor() {
    let tile = converted(&adt(&[old_cell(-3000.0, &old_liquid(0, -3000.0))]));
    let map = view(&tile);
    assert_eq!(word(map.height().v9_samples(), 0), (-2000f32).to_bits());
    assert_eq!(map.liquid().unwrap().level_bits(), (-3000f32).to_bits());
    assert!(tile.stats().has_liquid);
}

#[test]
fn one_pack_switch_controls_near_flat_height_and_liquid_sections() {
    let mut old = old_liquid(0, -2000.0);
    put(&mut old, 12, (-1999.9995f32).to_bits());
    let input = adt(&[old_cell(0.0005, &old)]);
    for pack in [false, true] {
        let tile = convert_adt(
            &input,
            &LOOKUP,
            Context { pack, ..context() },
            Limits::default(),
        )
        .unwrap();
        let map = view(&tile);
        assert_eq!(
            map.height().encoding(),
            if pack {
                height::Encoding::Flat
            } else {
                height::Encoding::Float32
            }
        );
        assert_eq!(map.liquid().unwrap().flags() & 2 != 0, pack);
    }
}

#[test]
fn modern_liquid_uses_borrowed_lookup_and_preserves_entry_before_filters() {
    let input = adt(&[frame(b"MH2O", &modern_liquid())]);
    for (present, visual, filtered) in [
        (true, false, false),
        (false, false, true),
        (true, true, true),
    ] {
        let tile = convert_adt(
            &input,
            &Lookup { present, visual },
            context(),
            Limits::default(),
        )
        .unwrap();
        let liquid = view(&tile).liquid().unwrap();
        assert_eq!(&liquid.entry_bytes().unwrap()[..2], &7u16.to_le_bytes());
        assert!(tile.stats().has_liquid);
        assert_eq!(liquid.flags() & 2 != 0, filtered);
        if filtered {
            assert_eq!(
                (
                    liquid.offset_x(),
                    liquid.offset_y(),
                    liquid.width(),
                    liquid.height()
                ),
                (128, 128, 130, 130)
            );
        } else {
            assert_eq!(
                (
                    liquid.offset_x(),
                    liquid.offset_y(),
                    liquid.width(),
                    liquid.height()
                ),
                (0, 0, 2, 2)
            );
            assert_eq!(word(liquid.height_bytes(), 0), 10f32.to_bits());
            // Only the first tile is visible; neighboring interior samples
            // are replaced by the MLIQ invisible floor when cropped.
            assert_eq!(word(liquid.height_bytes(), 4), (-2000f32).to_bits());
        }
    }
}

#[test]
fn malformed_framing_dimensions_and_nested_samples_do_not_convert() {
    for (input, error) in [
        (vec![], crate::Error::MissingChunk),
        (adt(&[vec![0]]), crate::Error::Truncated),
        (
            adt(&[cell(&prefix(16, 0, 0.0, 0), &[])]),
            crate::Error::InvalidCellCoordinates,
        ),
        (
            adt(&[cell(&prefix(0, 16, 0.0, 0), &[])]),
            crate::Error::InvalidCellCoordinates,
        ),
        (
            adt(&[frame(b"MCNK", &[0; 127])]),
            crate::Error::InvalidChunkSize,
        ),
        (
            adt(&[cell(&prefix(0, 0, 0.0, 0), &[mcvt(&[0; 144])])]),
            crate::Error::InvalidChunkSize,
        ),
        (
            adt(&[cell(
                &prefix(0, 0, 0.0, 0),
                &[mcvt(&[0; 145]), mcvt(&[0; 145])],
            )]),
            crate::Error::DuplicateChunk,
        ),
    ] {
        assert_eq!(
            convert_adt(&input, &LOOKUP, context(), Limits::default()).err(),
            Some(Error::Adt(error))
        );
    }
}

#[test]
fn used_nonfinite_and_late_liquid_bounds_errors_return_no_partial_tile() {
    let input = adt(&[cell(&prefix(0, 0, f32::NAN, 0), &[])]);
    assert_eq!(
        convert_adt(&input, &LOOKUP, context(), Limits::default()).err(),
        Some(Error::Adt(crate::Error::NonFiniteHeight))
    );
    let input = adt(&[old_cell(0.0, &old_liquid(0, f32::INFINITY))]);
    assert_eq!(
        convert_adt(&input, &LOOKUP, context(), Limits::default()).err(),
        Some(Error::Liquid(liquid::Error::NonFiniteHeight))
    );
    let mut raw = modern_liquid();
    raw.pop();
    let input = adt(&[cell(&prefix(0, 0, 1.0, 7), &[]), frame(b"MH2O", &raw)]);
    assert_eq!(
        convert_adt(&input, &LOOKUP, context(), Limits::default()).err(),
        Some(Error::Liquid(liquid::Error::Bounds))
    );
}

#[test]
fn selected_liquid_rectangle_format_and_prefix_gates_are_preserved() {
    for (offset, value, error) in [
        (3086, 9, liquid::Error::InvalidRectangle),
        (3074, 41, liquid::Error::UnsupportedVertexFormat),
    ] {
        let mut raw = modern_liquid();
        raw[offset] = value;
        assert_eq!(
            convert_adt(
                &adt(&[frame(b"MH2O", &raw)]),
                &LOOKUP,
                context(),
                Limits::default()
            )
            .err(),
            Some(Error::Liquid(error)),
        );
    }
    let input = adt(&[old_cell(0.0, &[0; 803])]);
    assert_eq!(
        convert_adt(&input, &LOOKUP, context(), Limits::default()).err(),
        Some(Error::Liquid(liquid::Error::Bounds))
    );
}

#[test]
fn exact_complex_output_budget_and_input_immutability_hold() {
    let input = adt(&[
        old_cell(42.0, &old_liquid(0x80, 7.0)),
        frame(b"MFBO", &[0; 36]),
    ]);
    let before = input.clone();
    let tile = converted(&input);
    for (cap, okay) in [(tile.bytes().len(), true), (tile.bytes().len() - 1, false)] {
        let result = convert_adt(
            &input,
            &LOOKUP,
            context(),
            Limits {
                max_output_bytes: cap,
                ..Limits::default()
            },
        );
        assert_eq!(result.is_ok(), okay);
        if let Ok(result) = result {
            assert_eq!(result.bytes(), tile.bytes());
        }
        assert_eq!(input, before);
    }
}

#[test]
fn explicit_70374_conversion_preserves_kernels_and_old_build_rejections() {
    let input = adt(&[
        cell(&prefix(0, 0, 12.5, 42), &[]),
        frame(b"MH2O", &modern_liquid()),
    ]);
    for pack in [false, true] {
        let mut c = context();
        c.pack = pack;
        let old = convert_adt(&input, &LOOKUP, c, Limits::default()).unwrap();
        c.build = 70374;
        assert!(matches!(
            convert_adt(&input, &LOOKUP, c, Limits::default()),
            Err(Error::Maps(maps::Error::UnsupportedBuild))
        ));
        let now = convert_adt_70374(&input, &LOOKUP, c, Limits::default()).unwrap();
        let mut expected = old.bytes().to_vec();
        put(&mut expected, 8, 70374);
        assert_eq!(now.bytes(), expected);
        assert_eq!(
            maps::verify_70374(now.bytes(), 70374, maps::MAX_MAP_BYTES)
                .unwrap()
                .build(),
            70374
        );
        c.build = 70205;
        assert!(matches!(
            convert_adt_70374(&input, &LOOKUP, c, Limits::default()),
            Err(Error::Maps(maps::Error::UnsupportedBuild))
        ));
    }
    let mut c = context();
    c.build = 70374;
    assert!(convert_adt_70374(&input[..11], &LOOKUP, c, Limits::default()).is_err());
    c.tile_x = 64;
    assert!(matches!(
        convert_adt_70374(&input, &LOOKUP, c, Limits::default()),
        Err(Error::InvalidCoordinates)
    ));
}
