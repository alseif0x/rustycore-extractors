// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: complete synthetic on-disk CASC terrain acceptance.
use super::*;
use crate::local_store::tests::{CollisionFile, CollisionFixture};
use plan::{Selection, TileRequest};
pub(crate) mod fixtures;
mod native;
use fixtures::*;

#[derive(Default)]
struct Stage<'a> {
    sources: Vec<SourceFacts>,
    tiles: Vec<(conversion::Context, SourceFacts, Vec<u8>, conversion::Stats)>,
    lists: Vec<(u32, Vec<u8>)>,
    fail_source: Option<Step>,
    fail_tile: bool,
    fail_list: bool,
    change_on_list: Option<&'a CollisionFixture>,
}
impl Sink for Stage<'_> {
    fn source(&mut self, facts: SourceFacts) -> Result<(), SinkError> {
        self.sources.push(facts);
        if self.fail_source == Some(facts.step) {
            return Err(SinkError);
        }
        Ok(())
    }
    fn tile(
        &mut self,
        c: conversion::Context,
        f: SourceFacts,
        t: &conversion::ConvertedTile,
    ) -> Result<(), SinkError> {
        if self.fail_tile {
            return Err(SinkError);
        }
        self.tiles
            .push((c, f, t.bytes().to_vec(), t.stats().clone()));
        Ok(())
    }
    fn tile_list(&mut self, map: u32, bytes: &[u8; tilelist::BYTES]) -> Result<(), SinkError> {
        if self.fail_list {
            return Err(SinkError);
        }
        self.lists.push((map, bytes.to_vec()));
        if let Some(f) = self.change_on_list {
            f.change_metadata();
        }
        Ok(())
    }
}
fn execute(
    files: &[CollisionFile<'_>],
    selection: Selection<'_>,
    names: bool,
    limits: Limits,
) -> (Result<Completed, Failure>, Stage<'static>) {
    let fixture = CollisionFixture::new(files);
    let mut stage = Stage::default();
    let result = run(
        &mut fixture.open(names),
        selection,
        true,
        limits,
        &mut stage,
    );
    (result, stage)
}
fn failure(result: Result<Completed, Failure>) -> Failure {
    result.err().expect("must fail")
}

#[test]
fn startup_complete_root_routing_output_order_and_tilelist_are_connected() {
    let f = basic();
    let (result, stage) = execute(&f, Selection::AllMaps, false, Limits::default());
    let completed = result.unwrap();
    assert_eq!(
        stage
            .sources
            .iter()
            .map(|r| r.file_data_id)
            .collect::<Vec<_>>(),
        [MAP_ID, MATERIAL_ID, OBJECT_ID, TYPE_ID, 100, 201, 202]
    );
    assert_eq!(
        stage
            .tiles
            .iter()
            .map(|t| (t.0.map_id, t.0.tile_x, t.0.tile_y))
            .collect::<Vec<_>>(),
        [(7, 3, 2), (7, 1, 4)]
    );
    for (context, source, bytes, stats) in &stage.tiles {
        forever_terrain::maps::verify(bytes, 70205, 202112).unwrap();
        assert_eq!(
            source.bytes,
            f.iter()
                .find(|f| f.0 == source.file_data_id)
                .unwrap()
                .2
                .len()
        );
        assert_eq!(stats.unique_cells, 1);
        assert_eq!(context.build, 70205);
    }
    assert_eq!(stage.lists.len(), 1);
    for y in 0..64 {
        for x in 0..64 {
            assert_eq!(
                tilelist::present(&stage.lists[0].1, 70205, x, y).unwrap(),
                [(3, 2), (1, 4)].contains(&(x, y))
            );
        }
    }
    assert_eq!(completed.plan().selected_coordinates, 4096);
    assert_eq!(completed.plan().present_tiles, 2);
    assert_eq!(completed.plan().absent_tiles, 4094);
    assert_eq!(completed.stats().read_attempts, 7);
    assert_eq!(completed.stats().converted_tiles, 2);
    assert_eq!(completed.stats().tile_lists, 1);
    assert_eq!(
        completed.tables().map(|r| r.source.step),
        [Step::Map, Step::Material, Step::Object, Step::Type]
    );
}
#[test]
fn explicit_tiles_include_absence_and_never_claim_a_whole_map() {
    let requests = [
        TileRequest {
            map_id: 7,
            x: 1,
            y: 4,
        },
        TileRequest {
            map_id: 7,
            x: 0,
            y: 0,
        },
    ];
    let (r, s) = execute(
        &basic(),
        Selection::Tiles(&requests),
        false,
        Limits::default(),
    );
    let r = r.unwrap();
    assert_eq!(r.plan().selected_coordinates, 2);
    assert_eq!(r.plan().absent_tiles, 1);
    assert_eq!(s.tiles.len(), 1);
    assert_eq!((s.tiles[0].0.tile_x, s.tiles[0].0.tile_y), (1, 4));
    assert!(s.lists.is_empty());
    let all: Vec<_> = (0..64)
        .flat_map(|y| (0..64).map(move |x| TileRequest { map_id: 7, x, y }))
        .collect();
    let (r, s) = execute(&basic(), Selection::Tiles(&all), false, Limits::default());
    assert_eq!(r.unwrap().plan().scope, plan::Scope::Tiles);
    assert_eq!(s.tiles.len(), 2);
    assert!(s.lists.is_empty());
}
#[test]
fn both_admitted_builds_and_both_packing_modes_flow_from_store() {
    let mut packed = None;
    for build in [70170, 70205] {
        for pack in [false, true] {
            let f = CollisionFixture::for_build(&basic(), build);
            let mut s = Stage::default();
            let r = run(
                &mut f.open(false),
                Selection::AllMaps,
                pack,
                Limits::default(),
                &mut s,
            )
            .unwrap();
            assert_eq!(r.build(), build);
            assert_eq!(s.tiles[0].0.pack, pack);
            for (_, _, bytes, _) in &s.tiles {
                forever_terrain::maps::verify(bytes, build, 202112).unwrap();
            }
            tilelist::verify(&s.lists[0].1, build).unwrap();
            if build == 70205 {
                if pack {
                    assert_ne!(packed.as_ref().unwrap(), &s.tiles[0].2);
                } else {
                    packed = Some(s.tiles[0].2.clone());
                }
            }
        }
    }
}
#[test]
fn copied_maps_reuse_wdt_but_keep_output_identity_and_source_only_replay() {
    let mut f = basic();
    f[0].2 = map_db2(&[(9, 100, -1), (8, 0, -1)], &[(10, 9), (11, 10), (12, 8)]);
    let (r, s) = execute(&f, Selection::AllMaps, false, Limits::default());
    let r = r.unwrap();
    assert_eq!(r.tables()[0].materialized_rows, 2);
    assert_eq!(r.plan().maps, 2);
    assert_eq!(s.lists.iter().map(|r| r.0).collect::<Vec<_>>(), [9, 10]);
    assert_eq!(
        s.sources.iter().filter(|r| r.file_data_id == 100).count(),
        2
    );
    assert_eq!(
        s.tiles.iter().map(|t| t.0.map_id).collect::<Vec<_>>(),
        [9, 9, 10, 10]
    );
}
#[test]
fn empty_maps_still_require_all_startup_tables_and_emit_nothing() {
    for rows in [vec![], vec![(7, 0, -1)]] {
        let mut f = basic();
        f[0].2 = map_db2(&rows, &[]);
        let (r, s) = execute(&f, Selection::AllMaps, false, Limits::default());
        assert_eq!(r.unwrap().plan().maps, 0);
        assert_eq!(s.sources.len(), 4);
        assert!(s.tiles.is_empty());
        assert!(s.lists.is_empty());
    }
}
#[test]
fn an_empty_complete_wdt_emits_a_zero_whole_map_index() {
    let mut f = basic();
    f[4].2 = wdt(&[], true);
    let (r, s) = execute(&f, Selection::AllMaps, false, Limits::default());
    assert_eq!(r.unwrap().stats().converted_tiles, 0);
    assert_eq!(s.sources.len(), 5);
    assert!(s.lists[0].1[12..].iter().all(|b| *b == b'0'));
}
#[test]
fn logical_names_require_opt_in_and_bind_the_actual_name_selected_fdid() {
    let mut f = basic();
    f[4].2 = wdt(&[(3, 2, 999)], false);
    f[5].1 = Some(b"World/Maps/Synthetic/Synthetic_3_2.adt");
    let (r, s) = execute(&f, Selection::AllMaps, false, Limits::default());
    assert_eq!(
        failure(r).error,
        Error::Store(local_store::Error::NamesDisabled)
    );
    assert!(s.lists.is_empty());
    let (r, s) = execute(&f, Selection::AllMaps, true, Limits::default());
    r.unwrap();
    assert_eq!(s.tiles.len(), 1);
    assert_eq!(s.tiles[0].1.file_data_id, 201);
    assert_eq!(s.tiles[0].1.content_key, crate::blte::md5::digest(&f[5].2));
}
#[test]
fn missing_zero_or_max_root_id_never_selects_obj0_or_a_named_fallback() {
    for id in [0, u32::MAX, 999] {
        let mut f = basic();
        f[4].2 = wdt(&[(3, 2, id)], true);
        f[5].1 = Some(b"World/Maps/Synthetic/Synthetic_3_2.adt");
        let (r, s) = execute(&f, Selection::AllMaps, true, Limits::default());
        assert!(matches!(failure(r).error, Error::Store(_)));
        assert!(s.tiles.is_empty());
        assert!(s.lists.is_empty());
    }
}
#[test]
fn every_startup_table_is_required_complete_and_identity_checked() {
    for (index, step) in [
        (0, Step::Map),
        (1, Step::Material),
        (2, Step::Object),
        (3, Step::Type),
    ] {
        for corrupt in [false, true] {
            let mut f = basic();
            if corrupt {
                put(&mut f[index].2, 156, 0);
            } else {
                f.remove(index);
            }
            let (r, s) = execute(&f, Selection::AllMaps, false, Limits::default());
            assert_eq!(failure(r).step, step);
            assert!(s.tiles.is_empty());
            assert!(s.lists.is_empty());
        }
        let mut f = basic();
        f[index].2.pop();
        assert_eq!(
            failure(execute(&f, Selection::AllMaps, false, Limits::default()).0).step,
            step
        );
    }
}
#[test]
fn absent_wdt_and_malformed_wdt_cannot_become_empty_successful_maps() {
    for data in [
        None,
        Some(chunk(b"MVER", &18u32.to_le_bytes())),
        Some(wdt(&[(3, 2, 201)], true)[..50].to_vec()),
    ] {
        let mut f = basic();
        if let Some(b) = data {
            f[4].2 = b;
        } else {
            f.remove(4);
        }
        let (r, s) = execute(&f, Selection::AllMaps, false, Limits::default());
        assert_eq!(failure(r).step, Step::Wdt { map: 7 });
        assert!(s.tiles.is_empty());
        assert!(s.lists.is_empty());
    }
}
#[test]
fn late_tile_failure_leaves_only_provisional_prefix_without_completed_or_list() {
    let mut f = basic();
    f[6].2 = chunk(b"MVER", &17u32.to_le_bytes());
    let (r, s) = execute(&f, Selection::AllMaps, false, Limits::default());
    assert_eq!(failure(r).step, Step::Adt { map: 7, x: 1, y: 4 });
    assert_eq!(s.tiles.len(), 1);
    assert!(s.lists.is_empty());
}
#[test]
fn malformed_later_map_invalidates_the_entire_provisional_selection() {
    let mut f = basic();
    f[0].2 = map_db2(&[(7, 100, -1), (8, 101, -1)], &[]);
    let (r, s) = execute(&f, Selection::AllMaps, false, Limits::default());
    let e = failure(r);
    assert_eq!(e.step, Step::Wdt { map: 8 });
    assert_eq!(s.lists.len(), 1);
    assert_eq!(e.stats.converted_tiles, 2); // Neither this prefix nor its list is committed.
}
#[test]
fn source_tile_and_tilelist_sink_failures_abort_without_completion() {
    let f = CollisionFixture::new(&basic());
    for kind in 0..3 {
        let mut s = Stage {
            fail_source: if kind == 0 {
                Some(Step::Material)
            } else {
                None
            },
            fail_tile: kind == 1,
            fail_list: kind == 2,
            ..Default::default()
        };
        let e = failure(run(
            &mut f.open(false),
            Selection::AllMaps,
            true,
            Limits::default(),
            &mut s,
        ));
        assert_eq!(e.error, Error::Sink);
        assert!(s.lists.is_empty());
    }
}
#[test]
fn changed_metadata_after_last_sink_is_detected_at_finish() {
    let f = CollisionFixture::new(&basic());
    let mut store = f.open(false);
    let mut s = Stage {
        change_on_list: Some(&f),
        ..Default::default()
    };
    let e = failure(run(
        &mut store,
        Selection::AllMaps,
        true,
        Limits::default(),
        &mut s,
    ));
    assert_eq!(e.step, Step::Finish);
    assert_eq!(e.error, Error::Store(local_store::Error::InputChanged));
}
#[test]
fn archive_truncation_never_emits_any_tile_or_list() {
    let f = CollisionFixture::new(&basic());
    let mut store = f.open(false);
    f.truncate_archive();
    let mut s = Stage::default();
    assert!(
        run(
            &mut store,
            Selection::AllMaps,
            true,
            Limits::default(),
            &mut s
        )
        .is_err()
    );
    assert!(s.tiles.is_empty());
    assert!(s.lists.is_empty());
}
#[test]
fn output_and_source_byte_limits_are_cumulative_exact_and_reject_shortcuts() {
    let f = basic();
    let (r, _) = execute(&f, Selection::AllMaps, false, Limits::default());
    let stats = r.unwrap().stats();
    let limits = Limits {
        max_output_bytes: stats.output_bytes,
        max_source_bytes: stats.source_bytes,
        ..Default::default()
    };
    assert!(execute(&f, Selection::AllMaps, false, limits).0.is_ok());
    let e = failure(
        execute(
            &f,
            Selection::AllMaps,
            false,
            Limits {
                max_output_bytes: stats.output_bytes - 1,
                ..limits
            },
        )
        .0,
    );
    assert_eq!(e.error, Error::Limit);
    assert_eq!(e.step, Step::TileList { map: 7 });
    assert!(
        execute(
            &f,
            Selection::AllMaps,
            false,
            Limits {
                max_source_bytes: stats.source_bytes - 1,
                ..limits
            }
        )
        .0
        .is_err()
    );
}
#[test]
fn read_table_decode_memory_and_conversion_policies_fail_closed() {
    let limits = [
        Limits {
            max_reads: 5,
            ..Default::default()
        },
        Limits {
            max_direct_rows: 0,
            ..Default::default()
        },
        Limits {
            max_table_bytes: 0,
            ..Default::default()
        },
        Limits {
            max_decode_allowance: 0,
            ..Default::default()
        },
        Limits {
            max_memory_bytes: 0,
            ..Default::default()
        },
        Limits {
            max_wdt_bytes: 10,
            ..Default::default()
        },
        Limits {
            max_db2_bytes: 100,
            ..Default::default()
        },
        Limits {
            conversion: conversion::Limits {
                max_output_bytes: 0,
                ..Default::default()
            },
            ..Default::default()
        },
    ];
    for limit in limits {
        let (r, s) = execute(&basic(), Selection::AllMaps, false, limit);
        assert!(r.is_err());
        assert!(s.lists.is_empty());
    }
}
#[test]
fn raised_policies_reject_before_acquisition_or_sink_callbacks() {
    let d = Limits::default();
    let limits = [
        Limits {
            max_reads: d.max_reads + 1,
            ..d
        },
        Limits {
            max_memory_bytes: d.max_memory_bytes + 1,
            ..d
        },
        Limits {
            conversion: conversion::Limits {
                max_chunks: d.conversion.max_chunks + 1,
                ..d.conversion
            },
            ..d
        },
        Limits {
            plan: plan::Limits {
                max_maps: 4097,
                ..d.plan
            },
            ..d
        },
    ];
    for limit in limits {
        let (r, s) = execute(&basic(), Selection::AllMaps, false, limit);
        let e = failure(r);
        assert_eq!(e.error, Error::InvalidLimits);
        assert_eq!(e.stats.read_attempts, 0);
        assert!(s.sources.is_empty());
    }
}
#[test]
fn source_copy_and_lookup_caps_apply_before_wdt_execution() {
    let mut f = basic();
    f[1].2 = material_db2(&[(7, 0)], &[(8, 7), (9, 99)]);
    for l in [
        Limits {
            max_copy_records: 1,
            ..Default::default()
        },
        Limits {
            max_lookup_rows: 3,
            ..Default::default()
        },
    ] {
        let (r, s) = execute(&f, Selection::AllMaps, false, l);
        assert_eq!(failure(r).step, Step::Material);
        assert!(s.tiles.is_empty());
    }
}
#[test]
fn map_selection_errors_and_duplicates_remain_errors_after_startup() {
    for selection in [
        Selection::MapIds(&[]),
        Selection::MapIds(&[999]),
        Selection::MapIds(&[7, 7]),
    ] {
        let (r, s) = execute(&basic(), selection, false, Limits::default());
        assert!(matches!(failure(r).error, Error::Plan(_)));
        assert_eq!(s.sources.len(), 4);
    }
    let mut f = basic();
    f[0].2 = map_db2(&[(7, 100, -1)], &[(7, 7)]);
    assert_eq!(
        failure(execute(&f, Selection::AllMaps, false, Limits::default()).0).error,
        Error::Plan(plan::Error::AmbiguousMap)
    );
}
#[test]
fn liquid_copy_replay_is_used_by_actual_adt_conversion() {
    for visual in [false, true] {
        let mut f = basic();
        f[1].2 = material_db2(&[(5, if visual { 2 } else { 0 })], &[(7, 5)]);
        f[3].2 = type_db2(&[(21, 7)], &[(22, 21)]);
        f[5].2 = liquid_adt();
        let (r, s) = execute(&f, Selection::AllMaps, false, Limits::default());
        let r = r.unwrap();
        assert_eq!(r.tables()[1].materialized_rows, 2);
        assert_eq!(r.tables()[3].materialized_rows, 2);
        // The entry write precedes VisualOnly filtering in original System.cpp.
        // Nonuniform entries therefore retain MLIQ even for empty visibility.
        assert!(s.tiles[0].3.has_liquid);
        let view = forever_terrain::maps::verify(&s.tiles[0].2, 70205, 202112)
            .unwrap()
            .liquid()
            .unwrap();
        assert_eq!(&view.entry_bytes().unwrap()[..2], &22u16.to_le_bytes());
        assert_eq!(
            view.flag_bytes().unwrap()[0],
            if visual { 0 } else { liquid::WATER }
        );
        assert_eq!(
            (
                view.offset_x(),
                view.offset_y(),
                view.width(),
                view.height()
            ),
            if visual {
                (128, 128, 130, 130)
            } else {
                (0, 0, 9, 9)
            }
        );
    }
}
