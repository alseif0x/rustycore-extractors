// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original synthetic materialized-row and WDT fixtures.
// Map fixtures are the public closed reader's materialized output contract;
// copy replay itself remains covered by forever-db2's source-consumer tests.
use super::*;

fn row(id: u32, wdt: i32, directory: &str) -> DirectRecord<MapFields> {
    DirectRecord {
        id,
        fields: MapFields {
            wdt_file_data_id: wdt,
            name: "synthetic display name".to_owned(),
            directory: directory.to_owned(),
        },
    }
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
fn fixture(mode: Option<u32>, maid: bool, cells: &[(u8, u8, u32, u32, u32)]) -> Vec<u8> {
    let mut out = frame(b"MVER", &18u32.to_le_bytes());
    if let Some(flags) = mode {
        let mut bytes = [0; 32];
        put(&mut bytes, 0, flags);
        out.extend(frame(b"MPHD", &bytes));
    }
    let mut main = vec![0; 4096 * 8];
    let mut ids = vec![0; 4096 * 32];
    for &(x, y, flags, data1, id) in cells {
        let ordinal = usize::from(y) * 64 + usize::from(x);
        put(&mut main, ordinal * 8, flags);
        put(&mut main, ordinal * 8 + 4, data1);
        put(&mut ids, ordinal * 32, id);
        // Ancillary IDs must never replace rootADT, including rootADT==0.
        for word in 1..8 {
            put(&mut ids, ordinal * 32 + word * 4, 900 + word as u32);
        }
    }
    out.extend(frame(b"MAIN", &main));
    if maid {
        out.extend(frame(b"MAID", &ids));
    }
    out
}
fn parse(bytes: &[u8]) -> Wdt<'_> {
    Wdt::parse(bytes, forever_terrain::Limits::default()).unwrap()
}
fn modern(cells: &[(u8, u8, u32, u32, u32)]) -> Vec<u8> {
    fixture(Some(0x200), true, cells)
}
fn plan<'a>(rows: &'a [DirectRecord<MapFields>], selection: Selection<'_>) -> WdtPlan<'a> {
    WdtPlan::from_materialized(rows, selection, Limits::default()).unwrap()
}
fn request(map_id: u32, x: u8, y: u8) -> TileRequest {
    TileRequest { map_id, x, y }
}

#[test]
fn limits_and_error_codes_are_explicit_static_contracts() {
    let limits = Limits::default();
    assert_eq!(limits.max_source_rows, 200_000);
    assert_eq!(limits.max_maps, 200_000);
    assert_eq!(limits.max_explicit_tiles, 1_000_000);
    assert_eq!(limits.max_total_coordinates, 819_200_000);
    assert_eq!(limits.max_logical_name_bytes, 1024);
    for (error, code) in [
        (Error::InvalidLimits, "terrain_plan_invalid_limits"),
        (Error::Limit, "terrain_plan_policy_limit"),
        (Error::EmptySelection, "terrain_plan_empty_selection"),
        (Error::DuplicateRequest, "terrain_plan_duplicate_request"),
        (
            Error::InvalidCoordinates,
            "terrain_plan_invalid_coordinates",
        ),
        (
            Error::InvalidMaterializedRows,
            "terrain_plan_invalid_materialized_rows",
        ),
        (Error::MissingMap, "terrain_plan_missing_map"),
        (Error::AmbiguousMap, "terrain_plan_ambiguous_map"),
        (Error::WrongWdt, "terrain_plan_wrong_wdt"),
        (Error::UnexpectedWdt, "terrain_plan_unexpected_wdt"),
        (Error::MissingFileIds, "terrain_plan_missing_file_ids"),
        (
            Error::InvalidLogicalName,
            "terrain_plan_invalid_logical_name",
        ),
        (Error::FailedExpansion, "terrain_plan_failed_expansion"),
        (
            Error::IncompleteExpansion,
            "terrain_plan_incomplete_expansion",
        ),
    ] {
        assert_eq!(error.code(), code);
        assert_eq!(error.to_string(), code);
    }
}

#[test]
fn whole_maps_keep_materialized_order_full_ids_and_signed_wdt_bits() {
    let rows = [
        row(42, 10, "a"),
        row(0, -1, "b"),
        row(u32::MAX, -2, "c"),
        row(3, 11, "d"),
    ];
    let planned = plan(&rows, Selection::MapIds(&[3, u32::MAX, 42]));
    let requests = planned.maps().collect::<Vec<_>>();
    assert_eq!(
        requests.iter().map(|r| r.map_id).collect::<Vec<_>>(),
        [42, u32::MAX, 3]
    );
    assert_eq!(
        requests
            .iter()
            .map(|r| r.materialized_ordinal)
            .collect::<Vec<_>>(),
        [0, 2, 3]
    );
    assert_eq!(
        requests.iter().map(|r| r.plan_index).collect::<Vec<_>>(),
        [0, 1, 2]
    );
    assert_eq!(requests[1].file_data_id, u32::MAX - 1);
    assert!(requests.iter().all(|r| r.whole_map));
    assert_eq!(planned.selected_coordinates(), 3 * 4096);
    let all = plan(&rows, Selection::AllMaps);
    assert_eq!(all.maps().nth(1).unwrap().file_data_id, u32::MAX);
    assert_eq!(all.scope(), Scope::AllMaps);
}

#[test]
fn duplicate_materialized_map_ids_reject_only_when_selected() {
    // A direct duplicate, destination overwrite or self-copy in the consumer
    // may produce this vector. The planner must not silently choose a row.
    let rows = [row(8, 10, "a"), row(7, 11, "b"), row(8, 12, "c")];
    for selection in [
        Selection::AllMaps,
        Selection::MapIds(&[8]),
        Selection::Tiles(&[request(8, 0, 0)]),
    ] {
        assert_eq!(
            WdtPlan::from_materialized(&rows, selection, Limits::default()).err(),
            Some(Error::AmbiguousMap)
        );
    }
    let planned = plan(&rows, Selection::MapIds(&[7]));
    assert_eq!(planned.maps().next().unwrap().materialized_ordinal, 1);
}

#[test]
fn missing_map_empty_explicit_lists_duplicates_and_coordinates_reject() {
    let rows = [row(0, 1, "a")];
    for selection in [Selection::MapIds(&[]), Selection::Tiles(&[])] {
        assert_eq!(
            WdtPlan::from_materialized(&rows, selection, Limits::default()).err(),
            Some(Error::EmptySelection)
        );
    }
    for selection in [
        Selection::MapIds(&[1]),
        Selection::Tiles(&[request(1, 0, 0)]),
    ] {
        assert_eq!(
            WdtPlan::from_materialized(&rows, selection, Limits::default()).err(),
            Some(Error::MissingMap)
        );
    }
    for selection in [
        Selection::MapIds(&[0, 0]),
        Selection::Tiles(&[request(0, 1, 2), request(0, 1, 2)]),
    ] {
        assert_eq!(
            WdtPlan::from_materialized(&rows, selection, Limits::default()).err(),
            Some(Error::DuplicateRequest)
        );
    }
    for (x, y) in [(64, 0), (0, 64), (255, 255)] {
        assert_eq!(
            WdtPlan::from_materialized(
                &rows,
                Selection::Tiles(&[request(0, x, y)]),
                Limits::default()
            )
            .err(),
            Some(Error::InvalidCoordinates)
        );
    }
}

#[test]
fn unmaterialized_zero_wdt_is_not_silently_filtered() {
    let rows = [row(1, 0, "a"), row(2, 3, "b")];
    assert_eq!(
        WdtPlan::from_materialized(&rows, Selection::MapIds(&[2]), Limits::default()).err(),
        Some(Error::InvalidMaterializedRows)
    );
}

#[test]
fn empty_all_maps_is_complete_without_any_wdt() {
    let planned = plan(&[], Selection::AllMaps);
    assert_eq!(planned.maps().len(), 0);
    assert_eq!(
        planned.expansion().finish().unwrap(),
        Completion {
            scope: Scope::AllMaps,
            maps: 0,
            selected_coordinates: 0,
            present_tiles: 0,
            absent_tiles: 0
        }
    );
}

#[test]
fn explicit_tiles_follow_source_map_then_row_column_order() {
    let rows = [row(9, 10, "a"), row(2, 20, "b")];
    let selection = [
        request(2, 8, 1),
        request(9, 7, 2),
        request(9, 1, 2),
        request(9, 63, 0),
    ];
    let planned = plan(&rows, Selection::Tiles(&selection));
    assert_eq!(planned.scope(), Scope::Tiles);
    assert_eq!(planned.selected_coordinates(), 4);
    assert_eq!(planned.maps().map(|r| r.map_id).collect::<Vec<_>>(), [9, 2]);
    let bytes = modern(&[
        (7, 2, 1, 777, 40),
        (1, 2, 0x101, 123, 41),
        (8, 1, 1, 456, 42),
    ]);
    let checked = parse(&bytes);
    let mut expansion = planned.expansion();
    let first = expansion.expand_next(10, &checked).unwrap();
    assert_eq!(
        first
            .tiles()
            .iter()
            .map(|t| (t.x(), t.y()))
            .collect::<Vec<_>>(),
        [(63, 0), (1, 2), (7, 2)]
    );
    assert!(!first.covers_whole_map());
    assert!(!first.tiles()[0].present());
    assert!(matches!(first.tiles()[0].source(), TileSource::Absent));
    assert_eq!(first.present_tiles(), 2);
    assert_eq!(first.absent_tiles(), 1);
    assert_eq!(first.tiles()[1].flags(), 0x101);
    assert_eq!(first.tiles()[1].data1(), 123);
    let second = expansion.expand_next(20, &checked).unwrap();
    assert_eq!((second.tiles()[0].x(), second.tiles()[0].y()), (8, 1));
    assert_eq!(
        expansion.finish().unwrap(),
        Completion {
            scope: Scope::Tiles,
            maps: 2,
            selected_coordinates: 4,
            present_tiles: 3,
            absent_tiles: 1
        }
    );
}

#[test]
fn full_map_expands_all_4096_coordinates_without_a_64_tile_cutoff() {
    let rows = [row(5, 9, "unused")];
    let planned = plan(&rows, Selection::AllMaps);
    let cells = (0..64)
        .flat_map(|y| (0..64).map(move |x| (x, y, 1, 0, 123)))
        .collect::<Vec<_>>();
    let bytes = modern(&cells);
    let mut expansion = planned.expansion();
    let map = expansion.expand_next(9, &parse(&bytes)).unwrap();
    assert_eq!(map.tiles().len(), 4096);
    assert_eq!(map.present_tiles(), 4096);
    assert_eq!(map.absent_tiles(), 0);
    assert!(map.covers_whole_map());
    assert_eq!((map.tiles()[4095].x(), map.tiles()[4095].y()), (63, 63));
    assert!(
        map.tiles()
            .iter()
            .all(|tile| matches!(tile.source(), TileSource::FileDataId(123)))
    );
    assert_eq!(expansion.finish().unwrap().present_tiles, 4096);
}

#[test]
fn whole_map_retains_absent_cells_and_opaque_main_words() {
    let rows = [row(5, 9, "unused")];
    let planned = plan(&rows, Selection::MapIds(&[5]));
    let bytes = modern(&[(7, 2, 2, u32::MAX, 987), (1, 3, 3, 42, 99)]);
    let mut expansion = planned.expansion();
    let map = expansion.expand_next(9, &parse(&bytes)).unwrap();
    let absent = &map.tiles()[2 * 64 + 7];
    assert_eq!(absent.flags(), 2);
    assert_eq!(absent.data1(), u32::MAX);
    assert!(matches!(absent.source(), TileSource::Absent));
    assert_eq!(map.present_tiles(), 1);
    assert_eq!(map.absent_tiles(), 4095);
    assert_eq!(expansion.finish().unwrap().absent_tiles, 4095);
}

#[test]
fn duplicate_zero_and_maximum_root_adts_are_not_filtered_or_deduplicated() {
    let rows = [row(1, 9, "a"), row(2, 9, "b")];
    let planned = plan(&rows, Selection::AllMaps);
    assert_eq!(
        planned.maps().map(|r| r.file_data_id).collect::<Vec<_>>(),
        [9, 9]
    );
    let bytes = modern(&[
        (0, 0, 1, 0, 0),
        (1, 0, 1, 0, u32::MAX),
        (2, 0, 1, 0, 50),
        (3, 0, 1, 0, 50),
    ]);
    let mut expansion = planned.expansion();
    for map_id in [1, 2] {
        let map = expansion.expand_next(9, &parse(&bytes)).unwrap();
        assert_eq!(map.request().map_id, map_id);
        for (tile, expected) in map.tiles().iter().take(4).zip([0, u32::MAX, 50, 50]) {
            assert!(matches!(tile.source(), TileSource::FileDataId(id) if *id == expected));
        }
    }
    assert_eq!(expansion.finish().unwrap().present_tiles, 8);
}

#[test]
fn modern_empty_main_needs_no_maid_and_creates_no_adt_dependency() {
    let rows = [row(7, 8, "a")];
    let planned = plan(&rows, Selection::AllMaps);
    let bytes = fixture(Some(0x200), false, &[]);
    let mut expansion = planned.expansion();
    let map = expansion.expand_next(8, &parse(&bytes)).unwrap();
    assert_eq!(map.present_tiles(), 0);
    assert_eq!(map.absent_tiles(), 4096);
    assert!(
        map.tiles()
            .iter()
            .all(|tile| matches!(tile.source(), TileSource::Absent))
    );
    assert_eq!(expansion.finish().unwrap().present_tiles, 0);
}

#[test]
fn missing_mphd_or_clear_bit_selects_logical_names_even_when_maid_exists() {
    let rows = [row(1, 8, "dUrOtAr")];
    let requests = [request(1, 7, 2)];
    let planned = plan(&rows, Selection::Tiles(&requests));
    for mode in [None, Some(0), Some(0x100)] {
        let bytes = fixture(mode, true, &[(7, 2, 1, 0, 123)]);
        let mut expansion = planned.expansion();
        let map = expansion.expand_next(8, &parse(&bytes)).unwrap();
        assert!(
            matches!(map.tiles()[0].source(), TileSource::LogicalName(name) if name == "WORLD\\MAPS\\DUROTAR\\DUROTAR_7_2.ADT")
        );
        assert_eq!(expansion.finish().unwrap().present_tiles, 1);
    }
}

#[test]
fn logical_name_normalization_is_ascii_only_and_never_an_output_path() {
    for (directory, expected) in [
        ("éx/a", "WORLD\\MAPS\\éX\\A\\éX\\A_63_0.ADT"),
        ("", "WORLD\\MAPS\\\\_63_0.ADT"),
        ("../escape", "WORLD\\MAPS\\..\\ESCAPE\\..\\ESCAPE_63_0.ADT"),
    ] {
        let rows = [row(u32::MAX, 9, directory)];
        let requests = [request(u32::MAX, 63, 0)];
        let planned = plan(&rows, Selection::Tiles(&requests));
        let bytes = fixture(None, false, &[(63, 0, 1, 0, 0)]);
        let mut expansion = planned.expansion();
        let map = expansion.expand_next(9, &parse(&bytes)).unwrap();
        assert!(
            matches!(map.tiles()[0].source(), TileSource::LogicalName(name) if name == expected)
        );
        assert_eq!(
            map_relative_path(map.request().map_id, 63, 0).unwrap(),
            "maps/4294967295_00_63.map"
        );
    }
    assert_eq!(map_relative_path(0, 7, 2).unwrap(), "maps/0000_02_07.map");
    assert_eq!(
        map_relative_path(10000, 63, 63).unwrap(),
        "maps/10000_63_63.map"
    );
    assert_eq!(
        map_relative_path(0, 64, 0).err(),
        Some(Error::InvalidCoordinates)
    );
}

#[test]
fn exact_name_cap_and_expansion_storage_cap_are_enforced_without_partial_map() {
    let rows = [row(1, 9, "a")];
    let requests = [request(1, 7, 2)];
    let bytes = fixture(None, false, &[(7, 2, 1, 0, 0)]);
    let name_bytes = "WORLD\\MAPS\\A\\A_7_2.ADT".len();
    let exact_bytes = size_of::<Tile>() + name_bytes;
    for (name_cap, expansion_cap, okay) in [
        (name_bytes, exact_bytes, true),
        (name_bytes - 1, exact_bytes, false),
        (name_bytes, exact_bytes - 1, false),
    ] {
        let planned = WdtPlan::from_materialized(
            &rows,
            Selection::Tiles(&requests),
            Limits {
                max_logical_name_bytes: name_cap,
                max_expansion_bytes: expansion_cap,
                ..Limits::default()
            },
        )
        .unwrap();
        let mut expansion = planned.expansion();
        let result = expansion.expand_next(9, &parse(&bytes));
        assert_eq!(result.is_ok(), okay);
        if !okay {
            assert_eq!(result.err(), Some(Error::Limit));
            assert_eq!(expansion.finish().err(), Some(Error::FailedExpansion));
        }
    }
}

#[test]
fn logical_nul_is_rejected_only_when_a_name_is_needed() {
    let rows = [row(1, 9, "a\0b")];
    let requests = [request(1, 0, 0)];
    let planned = plan(&rows, Selection::Tiles(&requests));
    let named = fixture(None, false, &[(0, 0, 1, 0, 0)]);
    let mut expansion = planned.expansion();
    assert_eq!(
        expansion.expand_next(9, &parse(&named)).err(),
        Some(Error::InvalidLogicalName)
    );
    let numeric = modern(&[(0, 0, 1, 0, 3)]);
    assert!(planned.expansion().expand_next(9, &parse(&numeric)).is_ok());
    let absent = fixture(None, false, &[]);
    assert!(planned.expansion().expand_next(9, &parse(&absent)).is_ok());
}

#[test]
fn second_map_failure_poisoning_prevents_successful_selection_completion() {
    let rows = [row(1, 1, "a"), row(2, 2, "a long name")];
    let selected = [request(1, 0, 0), request(2, 0, 0)];
    let planned = WdtPlan::from_materialized(
        &rows,
        Selection::Tiles(&selected),
        Limits {
            max_logical_name_bytes: 22,
            ..Limits::default()
        },
    )
    .unwrap();
    let bytes = fixture(None, false, &[(0, 0, 1, 0, 0)]);
    let mut expansion = planned.expansion();
    assert!(expansion.expand_next(1, &parse(&bytes)).is_ok());
    assert_eq!(
        expansion.expand_next(2, &parse(&bytes)).err(),
        Some(Error::Limit)
    );
    assert_eq!(
        expansion.expand_next(2, &parse(&bytes)).err(),
        Some(Error::FailedExpansion)
    );
    assert_eq!(expansion.finish().err(), Some(Error::FailedExpansion));
}

#[test]
fn wrong_out_of_order_missing_and_extra_wdt_associations_are_explicit() {
    let rows = [row(1, 10, "a"), row(2, 20, "b")];
    let planned = plan(&rows, Selection::AllMaps);
    let bytes = modern(&[]);
    let checked = parse(&bytes);
    let mut expansion = planned.expansion();
    assert_eq!(
        expansion.expand_next(20, &checked).err(),
        Some(Error::WrongWdt)
    );
    assert_eq!(expansion.finish().err(), Some(Error::FailedExpansion));
    let mut expansion = planned.expansion();
    expansion.expand_next(10, &checked).unwrap();
    assert_eq!(expansion.finish().err(), Some(Error::IncompleteExpansion));
    let mut expansion = planned.expansion();
    expansion.expand_next(10, &checked).unwrap();
    expansion.expand_next(20, &checked).unwrap();
    assert_eq!(
        expansion.expand_next(20, &checked).err(),
        Some(Error::UnexpectedWdt)
    );
    assert_eq!(expansion.finish().err(), Some(Error::FailedExpansion));
}

#[test]
fn count_and_planner_limits_reject_whole_selection_before_expansion() {
    let rows = [row(1, 1, "a"), row(2, 2, "b")];
    for limits in [
        Limits {
            max_source_rows: 1,
            ..Limits::default()
        },
        Limits {
            max_maps: 1,
            ..Limits::default()
        },
        Limits {
            max_total_coordinates: 8191,
            ..Limits::default()
        },
        Limits {
            max_planner_bytes: 0,
            ..Limits::default()
        },
    ] {
        assert_eq!(
            WdtPlan::from_materialized(&rows, Selection::AllMaps, limits).err(),
            Some(Error::Limit)
        );
    }
    let requests = [request(1, 0, 0), request(1, 1, 0)];
    for limits in [
        Limits {
            max_explicit_tiles: 1,
            ..Limits::default()
        },
        Limits {
            max_total_coordinates: 1,
            ..Limits::default()
        },
    ] {
        assert_eq!(
            WdtPlan::from_materialized(&rows, Selection::Tiles(&requests), limits).err(),
            Some(Error::Limit)
        );
    }
    assert!(
        WdtPlan::from_materialized(
            &rows,
            Selection::AllMaps,
            Limits {
                max_total_coordinates: 8192,
                ..Limits::default()
            }
        )
        .is_ok()
    );
}

#[test]
fn limit_ceiling_cannot_be_enlarged_and_zero_caps_are_meaningful() {
    let max = Limits::default();
    for limits in [
        Limits {
            max_source_rows: max.max_source_rows + 1,
            ..max
        },
        Limits {
            max_maps: max.max_maps + 1,
            ..max
        },
        Limits {
            max_explicit_tiles: max.max_explicit_tiles + 1,
            ..max
        },
        Limits {
            max_total_coordinates: max.max_total_coordinates + 1,
            ..max
        },
        Limits {
            max_planner_bytes: max.max_planner_bytes + 1,
            ..max
        },
        Limits {
            max_expansion_bytes: max.max_expansion_bytes + 1,
            ..max
        },
        Limits {
            max_logical_name_bytes: max.max_logical_name_bytes + 1,
            ..max
        },
    ] {
        assert_eq!(
            WdtPlan::from_materialized(&[], Selection::AllMaps, limits).err(),
            Some(Error::InvalidLimits)
        );
    }
    let zero = Limits {
        max_source_rows: 0,
        max_maps: 0,
        max_explicit_tiles: 0,
        max_total_coordinates: 0,
        max_planner_bytes: 0,
        max_expansion_bytes: 0,
        max_logical_name_bytes: 0,
    };
    assert!(
        WdtPlan::from_materialized(&[], Selection::AllMaps, zero)
            .unwrap()
            .expansion()
            .finish()
            .is_ok()
    );
}

#[test]
fn per_map_storage_is_bounded_even_when_all_tiles_are_absent() {
    let rows = [row(1, 1, "a")];
    let bytes = modern(&[]);
    let exact = 4096 * size_of::<Tile>();
    for (cap, okay) in [(exact, true), (exact - 1, false)] {
        let planned = WdtPlan::from_materialized(
            &rows,
            Selection::AllMaps,
            Limits {
                max_expansion_bytes: cap,
                ..Limits::default()
            },
        )
        .unwrap();
        assert_eq!(
            planned.expansion().expand_next(1, &parse(&bytes)).is_ok(),
            okay
        );
    }
}

#[test]
fn explicit_all_coordinates_remains_subset_scope_without_tilelist_authority() {
    let rows = [row(1, 1, "a")];
    let selected = (0..64)
        .flat_map(|y| (0..64).map(move |x| request(1, x, y)))
        .collect::<Vec<_>>();
    let planned = plan(&rows, Selection::Tiles(&selected));
    let bytes = modern(&[]);
    let mut expansion = planned.expansion();
    let map = expansion.expand_next(1, &parse(&bytes)).unwrap();
    assert_eq!(map.tiles().len(), 4096);
    assert!(!map.covers_whole_map());
    assert_eq!(expansion.finish().unwrap().scope, Scope::Tiles);
}

#[test]
fn expansion_owns_no_wdt_bytes_and_preserves_input_rows() {
    let rows = [row(1, 2, "a")];
    let before = rows.clone();
    let planned = plan(&rows, Selection::MapIds(&[1]));
    let mut expansion = planned.expansion();
    let map = {
        let bytes = modern(&[(7, 2, 1, 0, 99)]);
        let saved = bytes.clone();
        let checked = parse(&bytes);
        let result = expansion.expand_next(2, &checked).unwrap();
        assert_eq!(bytes, saved);
        result
    };
    assert!(matches!(
        map.tiles()[2 * 64 + 7].source(),
        TileSource::FileDataId(99)
    ));
    assert_eq!(rows, before);
    assert_eq!(expansion.finish().unwrap().maps, 1);
}

#[test]
fn compatible_planner_unknown_coordinates_and_loaded_main_remain_separate() {
    let rows = [row(9, 100, "Nine"), row(7, 100, "Seven")];
    let p = plan(&rows, Selection::AllMaps);
    let mut e = p.compatible_expansion();
    let requests: Vec<_> = p.maps().collect();
    e.unavailable_next(requests[0]).unwrap();
    let bytes = modern(&[(3, 2, 1, 0, 201)]);
    e.expand_next(100, &parse(&bytes)).unwrap();
    let done = e.finish_compatible().unwrap();
    assert_eq!(
        done,
        CompatibleCompletion {
            scope: Scope::AllMaps,
            maps: 2,
            selected_coordinates: 8192,
            known_present_tiles: 1,
            known_absent_tiles: 4095,
            unknown_coordinates: 4096,
            unavailable_wdts: 1,
        }
    );
}

#[test]
fn compatible_unavailable_transition_requires_exact_next_owned_request() {
    for field in 0..5 {
        let rows = [row(9, 100, "Nine"), row(7, 100, "Seven")];
        let p = plan(&rows, Selection::AllMaps);
        let requests: Vec<_> = p.maps().collect();
        let mut wrong = requests[0];
        match field {
            0 => wrong.plan_index = requests[1].plan_index,
            1 => wrong.materialized_ordinal = requests[1].materialized_ordinal,
            2 => wrong.map_id = requests[1].map_id,
            3 => wrong.file_data_id += 1,
            _ => wrong.whole_map = false,
        }
        let mut e = p.compatible_expansion();
        assert_eq!(e.unavailable_next(wrong), Err(Error::WrongWdt));
        assert_eq!(e.unavailable_next(requests[0]), Err(Error::FailedExpansion));
        assert_eq!(e.finish_compatible(), Err(Error::FailedExpansion));
    }
}

#[test]
fn compatible_planner_incomplete_extra_failed_and_tiles_lifecycles_reject() {
    let rows = [row(9, 100, "Nine")];
    let p = plan(&rows, Selection::AllMaps);
    assert_eq!(
        p.compatible_expansion().finish_compatible(),
        Err(Error::IncompleteExpansion)
    );
    let request = p.maps().next().unwrap();
    let mut e = p.compatible_expansion();
    e.unavailable_next(request).unwrap();
    assert_eq!(e.unavailable_next(request), Err(Error::UnexpectedWdt));
    assert_eq!(e.finish_compatible(), Err(Error::FailedExpansion));
    let bytes = modern(&[]);
    let mut e = p.compatible_expansion();
    assert!(e.expand_next(101, &parse(&bytes)).is_err());
    assert_eq!(e.unavailable_next(request), Err(Error::FailedExpansion));
    let tiles = [TileRequest {
        map_id: 9,
        x: 0,
        y: 0,
    }];
    let p = plan(&rows, Selection::Tiles(&tiles));
    let mut e = p.compatible_expansion();
    assert_eq!(
        e.unavailable_next(p.maps().next().unwrap()),
        Err(Error::UnexpectedWdt)
    );
    assert_eq!(e.finish_compatible(), Err(Error::FailedExpansion));
    assert_eq!(
        p.compatible_expansion().finish_compatible(),
        Err(Error::UnexpectedWdt)
    );
    // The existing strict transition remains unable to finish without a WDT.
    assert_eq!(
        plan(&rows, Selection::AllMaps).expansion().finish(),
        Err(Error::IncompleteExpansion)
    );
}
