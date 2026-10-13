// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: synthetic graph/WDT planning fixtures, no client inputs.
use super::*;
use forever_vmap::objects::{Kind, Target};

fn row(id: u32, wdt: i32, parent: i16, directory: &str) -> DirectRecord<VMapFields> {
    DirectRecord {
        id,
        fields: VMapFields {
            wdt_file_data_id: wdt,
            parent_map_id: parent,
            name: "Synthetic map".into(),
            directory: directory.into(),
        },
    }
}
fn chunk(tag: &[u8; 4], body: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend(tag.iter().rev());
    bytes.extend_from_slice(&(body.len() as u32).to_le_bytes());
    bytes.extend_from_slice(body);
    bytes
}
fn put(bytes: &mut [u8], at: usize, value: u32) {
    bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
}
fn mphd(flags: u32) -> Vec<u8> {
    let mut body = [0; 32];
    put(&mut body, 0, flags);
    chunk(b"MPHD", &body)
}
fn main(cells: &[(usize, usize, u32)]) -> Vec<u8> {
    let mut body = vec![0; 32768];
    for &(x, y, flags) in cells {
        put(&mut body, (y * 64 + x) * 8, flags);
        put(&mut body, (y * 64 + x) * 8 + 4, u32::MAX);
    }
    chunk(b"MAIN", &body)
}
fn maid(cells: &[(usize, usize, u32)]) -> Vec<u8> {
    let mut body = vec![0; 131072];
    for &(x, y, id) in cells {
        let at = (y * 64 + x) * 32;
        put(&mut body, at, 700); // RootADT must not be selected.
        put(&mut body, at + 4, id);
        put(&mut body, at + 8, 701); // Obj1ADT must not be selected.
    }
    chunk(b"MAID", &body)
}
fn modf(id: u32, fdid: bool) -> Vec<u8> {
    let mut body = [0; 64];
    put(&mut body, 0, id);
    if fdid {
        body[56..58].copy_from_slice(&8u16.to_le_bytes());
    }
    chunk(b"MODF", &body)
}

#[test]
fn empty_graph_has_no_allocations_or_requests() {
    let maps = Maps::from_materialized(
        &[],
        0,
        Limits {
            max_memory_bytes: 0,
            ..Limits::default()
        },
    )
    .unwrap();
    assert_eq!(
        maps.stats(),
        Stats {
            rows: 0,
            direct_rows: 0,
            active_maps: 0,
            max_depth: 0,
            resident_bytes: 0,
            peak_bytes: 0
        }
    );
    assert!(maps.all().next().is_none());
    assert!(maps.preload().next().is_none());
    assert!(maps.processing().next().is_none());
    assert_eq!(maps.fallback_parent(0).err(), Some(Error::UnknownMap));
}

#[test]
fn graph_depth_and_parent_flags_precede_wdt_zero_filtering() {
    let rows = [
        row(8, 0, 4, "inactive child"),
        row(4, 44, 2, "middle"),
        row(2, 0, -1, "root"),
    ];
    let maps = Maps::from_materialized(&rows, 3, Limits::default()).unwrap();
    assert_eq!(
        maps.all()
            .map(|map| (map.id(), map.depth(), map.is_parent(), map.active()))
            .collect::<Vec<_>>(),
        [
            (8, 2, false, false),
            (4, 1, true, true),
            (2, 0, true, false)
        ]
    );
    assert_eq!(maps.preload().map(MapEntry::id).collect::<Vec<_>>(), [4]);
    assert_eq!(maps.processing().map(MapEntry::id).collect::<Vec<_>>(), [4]);
    assert_eq!(maps.stats().max_depth, 2);
    assert_eq!(
        maps.fallback_parent(4).err(),
        Some(Error::ParentUnavailable)
    );
    assert_eq!(maps.fallback_parent(8).unwrap().unwrap().id(), 4);
}

#[test]
fn preload_is_physical_and_processing_is_depth_then_physical() {
    let rows = [
        row(9, 90, 5, "c"),
        row(7, 70, -1, "b"),
        row(5, 50, -1, "a"),
        row(8, 80, 5, "d"),
    ];
    let maps = Maps::from_materialized(&rows, 4, Limits::default()).unwrap();
    assert_eq!(
        maps.preload().map(MapEntry::id).collect::<Vec<_>>(),
        [9, 7, 5, 8]
    );
    assert_eq!(
        maps.processing().map(MapEntry::id).collect::<Vec<_>>(),
        [7, 5, 9, 8]
    );
    let entry = maps.get(9).unwrap();
    assert_eq!(entry.physical_ordinal(), 0);
    assert_eq!(entry.parent_map_id(), Some(5));
    assert_eq!(entry.name(), "Synthetic map");
    assert_eq!(entry.directory(), "c");
}

#[test]
fn parent_only_in_copied_suffix_is_rejected() {
    let rows = [row(1, 11, 2, "child"), row(2, 22, -1, "copied parent")];
    assert_eq!(
        Maps::from_materialized(&rows, 1, Limits::default()).err(),
        Some(Error::CopiedParent)
    );
    assert!(Maps::from_materialized(&rows, 2, Limits::default()).is_ok());
}

#[test]
fn copied_child_with_direct_parent_is_admitted() {
    let rows = [
        row(1, 11, -1, "direct parent"),
        row(9, 99, 1, "copied child"),
    ];
    let maps = Maps::from_materialized(&rows, 1, Limits::default()).unwrap();
    assert_eq!(maps.stats().direct_rows, 1);
    assert_eq!(maps.get(9).unwrap().depth(), 1);
    assert_eq!(maps.fallback_parent(9).unwrap().unwrap().id(), 1);
    assert!(maps.get(1).unwrap().is_parent());
}

#[test]
fn duplicate_ids_are_rejected_across_direct_copy_and_inactive_rows() {
    for rows in [
        vec![row(4, 1, -1, "a"), row(4, 2, -1, "b")],
        vec![row(4, 0, -1, "a"), row(4, 0, -1, "b")],
    ] {
        for direct in [1, 2] {
            assert_eq!(
                Maps::from_materialized(&rows, direct, Limits::default()).err(),
                Some(Error::DuplicateId)
            );
        }
    }
}

#[test]
fn missing_parent_rejects_even_inactive_child() {
    for wdt in [0, 99] {
        let rows = [row(1, wdt, 8, "child")];
        assert_eq!(
            Maps::from_materialized(&rows, 1, Limits::default()).err(),
            Some(Error::MissingParent)
        );
    }
}

#[test]
fn direct_count_is_checked_without_inventing_provenance() {
    let rows = [row(1, 11, -1, "root")];
    assert_eq!(
        Maps::from_materialized(&rows, 2, Limits::default()).err(),
        Some(Error::DirectCount)
    );
    assert!(Maps::from_materialized(&rows, 0, Limits::default()).is_ok());
}

#[test]
fn self_and_multi_node_cycles_are_rejected() {
    for rows in [
        vec![row(1, 0, 1, "self")],
        vec![row(1, 1, 2, "a"), row(2, 0, 1, "b")],
    ] {
        assert_eq!(
            Maps::from_materialized(&rows, rows.len(), Limits::default()).err(),
            Some(Error::Cycle)
        );
    }
}

#[test]
fn depth_cap_is_inclusive_and_applies_to_filtered_rows() {
    let rows = [row(3, 0, 2, "c"), row(2, 0, 1, "b"), row(1, 0, -1, "a")];
    let limits = Limits {
        max_depth: 2,
        ..Limits::default()
    };
    assert_eq!(
        Maps::from_materialized(&rows, 3, limits)
            .unwrap()
            .stats()
            .max_depth,
        2
    );
    assert_eq!(
        Maps::from_materialized(
            &rows,
            3,
            Limits {
                max_depth: 1,
                ..limits
            }
        )
        .err(),
        Some(Error::Depth)
    );
    assert!(
        Maps::from_materialized(
            &rows[2..],
            1,
            Limits {
                max_depth: 0,
                ..limits
            }
        )
        .is_ok()
    );
}

#[test]
fn sparse_ids_and_signed_wdt_bits_are_preserved_without_index_arrays() {
    let rows = [
        row(u32::MAX, -1, -1, "high"),
        row(0, i32::MIN, -2, "zero map ID"),
    ];
    let maps = Maps::from_materialized(&rows, 2, Limits::default()).unwrap();
    assert_eq!(maps.get(u32::MAX).unwrap().wdt_file_data_id(), u32::MAX);
    assert_eq!(maps.get(0).unwrap().wdt_file_data_id(), 0x80000000);
    assert_eq!(maps.get(0).unwrap().parent_map_id(), None);
    assert_eq!(maps.preload().len(), 2);
    assert!(maps.stats().peak_bytes < 1024);
}

#[test]
fn fallback_exposes_only_immediate_parent_not_grandparent() {
    let rows = [
        row(1, 11, -1, "root"),
        row(2, 22, 1, "middle"),
        row(3, 33, 2, "child"),
    ];
    let maps = Maps::from_materialized(&rows, 3, Limits::default()).unwrap();
    assert_eq!(maps.fallback_parent(3).unwrap().unwrap().id(), 2);
    assert_eq!(maps.fallback_parent(2).unwrap().unwrap().id(), 1);
    assert!(maps.fallback_parent(1).unwrap().is_none());
    assert_eq!(maps.fallback_parent(4).err(), Some(Error::UnknownMap));
}

#[test]
fn limits_only_reduce_and_are_validated_before_inputs() {
    let d = Limits::default();
    for limits in [
        Limits {
            max_rows: d.max_rows + 1,
            ..d
        },
        Limits {
            max_maps: d.max_maps + 1,
            ..d
        },
        Limits {
            max_depth: d.max_depth + 1,
            ..d
        },
        Limits {
            max_name_bytes: d.max_name_bytes + 1,
            ..d
        },
        Limits {
            max_memory_bytes: d.max_memory_bytes + 1,
            ..d
        },
        Limits {
            max_wdt_bytes: d.max_wdt_bytes + 1,
            ..d
        },
        Limits {
            max_wdt_chunks: d.max_wdt_chunks + 1,
            ..d
        },
    ] {
        assert_eq!(
            Maps::from_materialized(&[], 1, limits).err(),
            Some(Error::InvalidLimits)
        );
        assert_eq!(Wdt::parse(&[], limits).err(), Some(Error::InvalidLimits));
    }
}

#[test]
fn row_map_and_memory_caps_reject_instead_of_truncating() {
    let rows = [row(1, 1, -1, "a"), row(2, 0, -1, "b")];
    let d = Limits::default();
    assert_eq!(
        Maps::from_materialized(&rows, 2, Limits { max_rows: 1, ..d }).err(),
        Some(Error::Limit)
    );
    assert_eq!(
        Maps::from_materialized(&rows, 2, Limits { max_maps: 0, ..d }).err(),
        Some(Error::Limit)
    );
    assert_eq!(
        Maps::from_materialized(
            &rows,
            2,
            Limits {
                max_memory_bytes: 0,
                ..d
            }
        )
        .err(),
        Some(Error::Limit)
    );
    assert!(Maps::from_materialized(&rows, 2, Limits { max_maps: 1, ..d }).is_ok());
}

#[test]
fn graph_memory_charges_capacities_and_scratch_but_borrows_strings() {
    let rows = [row(2, 0, 1, &"x".repeat(2048)), row(1, 0, -1, "parent")];
    let maps = Maps::from_materialized(&rows, 2, Limits::default()).unwrap();
    let stats = maps.stats();
    assert_eq!(
        stats.resident_bytes,
        capacity(&maps.nodes).unwrap()
            + capacity(&maps.by_id).unwrap()
            + capacity(&maps.preload).unwrap()
            + capacity(&maps.processing).unwrap()
    );
    assert!(stats.peak_bytes > stats.resident_bytes);
    assert!(stats.peak_bytes < 2048);
    assert!(
        Maps::from_materialized(
            &rows,
            2,
            Limits {
                max_memory_bytes: stats.peak_bytes,
                ..Limits::default()
            }
        )
        .is_ok()
    );
    assert_eq!(
        Maps::from_materialized(
            &rows,
            2,
            Limits {
                max_memory_bytes: stats.peak_bytes - 1,
                ..Limits::default()
            }
        )
        .err(),
        Some(Error::Limit)
    );
}

#[test]
fn absent_metadata_defaults_to_zero_and_mver_is_opaque() {
    for bytes in [
        chunk(b"JUNK", &[9, 7]),
        chunk(b"MVER", &[0xff]),
        mphd(0x200),
    ] {
        let wdt = Wdt::parse(&bytes, Limits::default()).unwrap();
        assert_eq!(wdt.stats().present_tiles, 0);
        assert!(wdt.globals().requests().is_empty());
        let rows = [row(1, 1, -1, "map")];
        let maps = Maps::from_materialized(&rows, 1, Limits::default()).unwrap();
        let mut tiles = wdt.tiles(maps.get(1).unwrap()).unwrap();
        assert!(tiles.next(&mut [0; MAX_NAME_BYTES]).is_none());
    }
}

#[test]
fn only_main_low_bit_selects_tiles_async_and_other_flags_do_not() {
    let bytes = main(&[(1, 2, 2), (3, 4, 0x80000000), (5, 6, 3)]);
    let wdt = Wdt::parse(&bytes, Limits::default()).unwrap();
    assert_eq!(wdt.flags(), 0);
    assert_eq!(wdt.stats().present_tiles, 1);
    let rows = [row(1, 1, -1, "Map")];
    let maps = Maps::from_materialized(&rows, 1, Limits::default()).unwrap();
    let mut tiles = wdt.tiles(maps.get(1).unwrap()).unwrap();
    let mut buffer = [0; MAX_NAME_BYTES];
    let request = tiles.next(&mut buffer).unwrap();
    assert_eq!((request.x, request.y), (5, 6));
    let TileTarget::Name(name) = request.target else {
        panic!("expected name");
    };
    assert_eq!(name, br"World\Maps\Map\Map_5_6_obj0.adt");
    assert!(tiles.next(&mut buffer).is_none());
    assert!(tiles.next(&mut buffer).is_none());
}

#[test]
fn tile_order_is_x_outer_y_inner_with_row_major_metadata() {
    let bytes = main(&[(63, 63, 1), (1, 0, 1), (0, 63, 1), (0, 1, 1), (0, 0, 1)]);
    let wdt = Wdt::parse(&bytes, Limits::default()).unwrap();
    let rows = [row(1, 1, -1, "Map")];
    let maps = Maps::from_materialized(&rows, 1, Limits::default()).unwrap();
    let mut tiles = wdt.tiles(maps.get(1).unwrap()).unwrap();
    let mut buffer = [0; MAX_NAME_BYTES];
    let mut coordinates = Vec::new();
    while let Some(request) = tiles.next(&mut buffer) {
        coordinates.push((request.x, request.y));
    }
    assert_eq!(coordinates, [(0, 0), (0, 1), (0, 63), (1, 0), (63, 63)]);
}

#[test]
fn maid_selects_only_obj0_and_retains_zero_max_and_asymmetric_coordinates() {
    let bytes = [
        mphd(0x200),
        main(&[(1, 2, 1), (2, 1, 1), (63, 63, 1)]),
        maid(&[(1, 2, 0), (2, 1, u32::MAX), (63, 63, 42)]),
    ]
    .concat();
    let wdt = Wdt::parse(&bytes, Limits::default()).unwrap();
    let rows = [row(1, 1, -1, "unused")];
    let maps = Maps::from_materialized(&rows, 1, Limits::default()).unwrap();
    let mut tiles = wdt.tiles(maps.get(1).unwrap()).unwrap();
    let mut buffer = [0; MAX_NAME_BYTES];
    let mut requests = Vec::new();
    while let Some(request) = tiles.next(&mut buffer) {
        let TileTarget::FileDataId(id) = request.target else {
            panic!("expected ID");
        };
        requests.push((request.x, request.y, id));
    }
    assert_eq!(requests, [(1, 2, 0), (2, 1, u32::MAX), (63, 63, 42)]);
}

#[test]
fn missing_maid_rejects_only_when_flag_and_present_tile_both_require_it() {
    let bytes = [mphd(0x200), main(&[(1, 1, 1)])].concat();
    assert_eq!(
        Wdt::parse(&bytes, Limits::default()).err(),
        Some(Error::MissingFileIds)
    );
    assert!(
        Wdt::parse(
            &[mphd(0x200), main(&[(1, 1, 2)])].concat(),
            Limits::default()
        )
        .is_ok()
    );
    assert!(Wdt::parse(&main(&[(1, 1, 1)]), Limits::default()).is_ok());
}

#[test]
fn maid_without_flag_does_not_override_logical_name_route() {
    let bytes = [main(&[(1, 2, 1)]), maid(&[(1, 2, 123)])].concat();
    let wdt = Wdt::parse(&bytes, Limits::default()).unwrap();
    let rows = [row(1, 1, -1, "World")];
    let maps = Maps::from_materialized(&rows, 1, Limits::default()).unwrap();
    let mut tiles = wdt.tiles(maps.get(1).unwrap()).unwrap();
    assert!(matches!(
        tiles.next(&mut [0; MAX_NAME_BYTES]).unwrap().target,
        TileTarget::Name(_)
    ));
}

#[test]
fn exact_metadata_extents_duplicate_chunks_and_tail_are_rejected() {
    for tag in [b"MPHD", b"MAIN", b"MAID"] {
        assert_eq!(
            Wdt::parse(&chunk(tag, &[0]), Limits::default()).err(),
            Some(Error::Objects(objects::Error::Framing))
        );
    }
    for bytes in [mphd(0), main(&[]), maid(&[])] {
        let duplicate = [&bytes[..], &bytes[..]].concat();
        assert_eq!(
            Wdt::parse(&duplicate, Limits::default()).err(),
            Some(Error::Objects(objects::Error::DuplicateMetadata))
        );
        let mut tail = bytes.clone();
        tail.push(0);
        assert_eq!(
            Wdt::parse(&tail, Limits::default()).err(),
            Some(Error::Objects(objects::Error::Framing))
        );
        assert_eq!(
            Wdt::parse(&bytes[..bytes.len() - 1], Limits::default()).err(),
            Some(Error::Objects(objects::Error::Framing))
        );
    }
    assert_eq!(
        Wdt::parse(&[], Limits::default()).err(),
        Some(Error::Objects(objects::Error::Framing))
    );
}

#[test]
fn every_truncated_header_and_unknown_chunk_body_is_rejected() {
    let bytes = chunk(b"JUNK", &[1, 2, 3]);
    for end in 0..bytes.len() {
        assert_eq!(
            Wdt::parse(&bytes[..end], Limits::default()).err(),
            Some(Error::Objects(objects::Error::Framing))
        );
    }
    assert!(Wdt::parse(&bytes, Limits::default()).is_ok());
}

#[test]
fn complete_global_objects_are_preserved_and_do_not_require_main_or_mphd() {
    let bytes = [
        chunk(b"MWMO", b"Root.wmo\0"),
        modf(0, false),
        modf(0, false),
        modf(u32::MAX, true),
    ]
    .concat();
    let wdt = Wdt::parse(&bytes, Limits::default()).unwrap();
    assert_eq!(wdt.stats().global_records, 3);
    assert_eq!(wdt.stats().present_tiles, 0);
    for request in wdt.globals().requests() {
        assert_eq!(request.kind(), Kind::Wmo);
        assert!(request.global_wmo());
    }
    let Target::Name(name) = wdt.globals().requests()[1].target() else {
        panic!("expected retained name");
    };
    assert_eq!(name, b"Root.wmo");
    assert!(matches!(
        wdt.globals().requests()[2].target(),
        Target::FileDataId(u32::MAX)
    ));
}

#[test]
fn invalid_late_global_record_prevents_any_wdt_result() {
    let bytes = [
        main(&[(0, 0, 1)]),
        chunk(b"MWMO", b"Root.wmo\0"),
        modf(0, false),
        modf(1, false),
    ]
    .concat();
    assert_eq!(
        Wdt::parse(&bytes, Limits::default()).err(),
        Some(Error::Objects(objects::Error::NameIndex))
    );
}

#[test]
fn logical_directory_is_preserved_including_case_spaces_separators_and_empty() {
    let bytes = main(&[(3, 14, 1)]);
    let wdt = Wdt::parse(&bytes, Limits::default()).unwrap();
    for directory in ["", "Ab C", "A/b\\C", ".."] {
        let rows = [row(1, 1, -1, directory)];
        let maps = Maps::from_materialized(&rows, 1, Limits::default()).unwrap();
        let mut tiles = wdt.tiles(maps.get(1).unwrap()).unwrap();
        let mut buffer = [0; MAX_NAME_BYTES];
        let TileTarget::Name(name) = tiles.next(&mut buffer).unwrap().target else {
            panic!("expected name");
        };
        assert_eq!(
            name,
            format!("World\\Maps\\{directory}\\{directory}_3_14_obj0.adt").as_bytes()
        );
    }
}

#[test]
fn non_ascii_and_nul_directories_reject_only_when_logical_tiles_use_them() {
    let named = main(&[(0, 0, 1)]);
    let id_bytes = [mphd(0x200), named.clone(), maid(&[(0, 0, 1)])].concat();
    let empty = mphd(0);
    for directory in ["ñ", "a\0b"] {
        let rows = [row(1, 1, -1, directory)];
        let maps = Maps::from_materialized(&rows, 1, Limits::default()).unwrap();
        assert_eq!(
            Wdt::parse(&named, Limits::default())
                .unwrap()
                .tiles(maps.get(1).unwrap())
                .err(),
            Some(Error::Name)
        );
        assert!(
            Wdt::parse(&id_bytes, Limits::default())
                .unwrap()
                .tiles(maps.get(1).unwrap())
                .is_ok()
        );
        assert!(
            Wdt::parse(&empty, Limits::default())
                .unwrap()
                .tiles(maps.get(1).unwrap())
                .is_ok()
        );
    }
}

#[test]
fn all_requested_name_lengths_are_validated_before_first_tile() {
    let bytes = main(&[(0, 0, 1), (63, 63, 1)]);
    let rows = [row(1, 1, -1, "")];
    let maps = Maps::from_materialized(&rows, 1, Limits::default()).unwrap();
    let limits = Limits {
        max_name_bytes: 26,
        ..Limits::default()
    };
    assert_eq!(
        Wdt::parse(&bytes, limits)
            .unwrap()
            .tiles(maps.get(1).unwrap())
            .err(),
        Some(Error::Limit)
    );
    let only_short = main(&[(0, 0, 1)]);
    assert!(
        Wdt::parse(
            &only_short,
            Limits {
                max_name_bytes: 25,
                ..limits
            }
        )
        .unwrap()
        .tiles(maps.get(1).unwrap())
        .is_ok()
    );
    assert_eq!(
        Wdt::parse(
            &only_short,
            Limits {
                max_name_bytes: 24,
                ..limits
            }
        )
        .unwrap()
        .tiles(maps.get(1).unwrap())
        .err(),
        Some(Error::Limit)
    );
}

#[test]
fn full_1024_byte_name_fits_and_1025_is_rejected_without_truncation() {
    let rows = [row(1, 1, -1, &"A".repeat(499))];
    let maps = Maps::from_materialized(&rows, 1, Limits::default()).unwrap();
    let bytes = main(&[(0, 63, 1)]); // 23 + 2*499 + 1 + 2 = 1024.
    let wdt = Wdt::parse(&bytes, Limits::default()).unwrap();
    let mut tiles = wdt.tiles(maps.get(1).unwrap()).unwrap();
    let mut buffer = [0; MAX_NAME_BYTES];
    let TileTarget::Name(name) = tiles.next(&mut buffer).unwrap().target else {
        panic!("expected name");
    };
    assert_eq!(name.len(), 1024);
    assert!(name.ends_with(b"_0_63_obj0.adt"));
    let too_long = main(&[(63, 63, 1)]);
    assert_eq!(
        Wdt::parse(&too_long, Limits::default())
            .unwrap()
            .tiles(maps.get(1).unwrap())
            .err(),
        Some(Error::Limit)
    );
}

#[test]
fn maps_reduced_name_cap_survives_a_wdt_parsed_with_default_limits() {
    let rows = [row(1, 1, -1, "")];
    let maps = Maps::from_materialized(
        &rows,
        1,
        Limits {
            max_name_bytes: 24,
            ..Limits::default()
        },
    )
    .unwrap();
    let bytes = main(&[(0, 0, 1)]);
    let wdt = Wdt::parse(&bytes, Limits::default()).unwrap();
    assert_eq!(wdt.tiles(maps.get(1).unwrap()).err(), Some(Error::Limit));
}

#[test]
fn inactive_map_cannot_request_tiles_and_fdid_route_ignores_name_limit() {
    let rows = [row(1, 0, -1, "unused"), row(2, 2, -1, "unused")];
    let maps = Maps::from_materialized(&rows, 2, Limits::default()).unwrap();
    let bytes = [mphd(0x200), main(&[(0, 0, 1)]), maid(&[(0, 0, 9)])].concat();
    let wdt = Wdt::parse(
        &bytes,
        Limits {
            max_name_bytes: 0,
            ..Limits::default()
        },
    )
    .unwrap();
    assert_eq!(
        wdt.tiles(maps.get(1).unwrap()).err(),
        Some(Error::InactiveMap)
    );
    assert!(wdt.tiles(maps.get(2).unwrap()).is_ok());
}

#[test]
fn wdt_input_chunk_and_own_global_memory_caps_are_reducible() {
    let bytes = [mphd(0), main(&[])].concat();
    assert!(
        Wdt::parse(
            &bytes,
            Limits {
                max_wdt_bytes: bytes.len(),
                max_wdt_chunks: 2,
                max_memory_bytes: 0,
                ..Limits::default()
            }
        )
        .is_ok()
    );
    assert_eq!(
        Wdt::parse(
            &bytes,
            Limits {
                max_wdt_bytes: bytes.len() - 1,
                ..Limits::default()
            }
        )
        .err(),
        Some(Error::Objects(objects::Error::Limit))
    );
    assert_eq!(
        Wdt::parse(
            &bytes,
            Limits {
                max_wdt_chunks: 1,
                ..Limits::default()
            }
        )
        .err(),
        Some(Error::Objects(objects::Error::Limit))
    );
    let global = modf(1, true);
    assert_eq!(
        Wdt::parse(
            &global,
            Limits {
                max_memory_bytes: 0,
                ..Limits::default()
            }
        )
        .err(),
        Some(Error::Objects(objects::Error::Limit))
    );
    let wdt = Wdt::parse(&global, Limits::default()).unwrap();
    assert_eq!(
        wdt.stats().resident_bytes,
        wdt.globals().stats().resident_bytes
    );
    assert_eq!(wdt.stats().peak_bytes, wdt.globals().stats().peak_bytes);
}

#[test]
fn full_grid_has_exactly_4096_requests_and_no_counter_wrap() {
    let mut body = vec![0; 32768];
    for cell in 0..4096 {
        put(&mut body, cell * 8, 1);
    }
    let bytes = chunk(b"MAIN", &body);
    let wdt = Wdt::parse(&bytes, Limits::default()).unwrap();
    let rows = [row(1, 1, -1, "Grid")];
    let maps = Maps::from_materialized(&rows, 1, Limits::default()).unwrap();
    let mut tiles = wdt.tiles(maps.get(1).unwrap()).unwrap();
    let mut buffer = [0; MAX_NAME_BYTES];
    for ordinal in 0..4096 {
        let request = tiles.next(&mut buffer).unwrap();
        assert_eq!(
            (request.x as usize, request.y as usize),
            (ordinal / 64, ordinal % 64)
        );
    }
    assert_eq!(wdt.stats().present_tiles, 4096);
    assert!(tiles.next(&mut buffer).is_none());
}

#[test]
fn errors_format_only_static_codes() {
    for error in [
        Error::CopiedParent,
        Error::ParentUnavailable,
        Error::Name,
        Error::Objects(objects::Error::NameIndex),
    ] {
        assert_eq!(error.to_string(), error.code());
        assert!(error.code().is_ascii());
    }
}
