// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: complete synthetic LocalStore execution fixtures.
use super::*;
use crate::local_store::tests::{CollisionFile, CollisionFixture};
use forever_db2::terrain_sources::{DirectRecord, VMapFields};
use forever_vmap::placement::DirBin;

struct Lookup;
impl LiquidLookup for Lookup {
    fn material_id(&self, _: u32) -> Option<u8> {
        None
    }
    fn material_flags(&self, _: u8) -> Option<u32> {
        None
    }
}
#[derive(Default)]
struct Stage {
    models: Vec<(Status, Option<Vec<u8>>)>,
    placements: Vec<(Context, Vec<u8>)>,
}
impl Sink for Stage {
    fn model(&mut self, model: &Model) -> Result<(), SinkError> {
        self.models
            .push((model.status(), model.raw().map(<[u8]>::to_vec)));
        Ok(())
    }
    fn placements(&mut self, context: Context, bytes: &[u8]) -> Result<(), SinkError> {
        self.placements.push((context, bytes.to_vec()));
        Ok(())
    }
}
fn put(bytes: &mut [u8], at: usize, value: u32) {
    bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
}
fn chunk(tag: &[u8; 4], body: &[u8]) -> Vec<u8> {
    let mut bytes: Vec<_> = tag.iter().rev().copied().collect();
    bytes.extend_from_slice(&(body.len() as u32).to_le_bytes());
    bytes.extend_from_slice(body);
    bytes
}
fn row(id: u32, wdt: i32, parent: i16) -> DirectRecord<VMapFields> {
    DirectRecord {
        id,
        fields: VMapFields {
            wdt_file_data_id: wdt,
            parent_map_id: parent,
            name: "Synthetic map".into(),
            directory: "Synthetic".into(),
        },
    }
}
fn maps(rows: &[DirectRecord<VMapFields>]) -> plan::Maps<'_> {
    plan::Maps::from_materialized(rows, rows.len(), plan::Limits::default()).unwrap()
}
fn wdt(tiles: &[(usize, usize, u32)], fdid: bool, global: Option<(u32, u32)>) -> Vec<u8> {
    let mut mphd = [0; 32];
    put(&mut mphd, 0, if fdid { 0x200 } else { 0 });
    let mut bytes = chunk(b"MPHD", &mphd);
    let mut main = vec![0; 32768];
    let mut maid = vec![0; 131072];
    for &(x, y, id) in tiles {
        put(&mut main, (y * 64 + x) * 8, 1);
        put(&mut maid, (y * 64 + x) * 32, 9999); // Root ADT is not acquired.
        put(&mut maid, (y * 64 + x) * 32 + 4, id);
        put(&mut maid, (y * 64 + x) * 32 + 8, 9998); // Neither is Obj1.
    }
    bytes.extend(chunk(b"MAIN", &main));
    if fdid {
        bytes.extend(chunk(b"MAID", &maid));
    }
    if let Some((id, unique)) = global {
        let mut source = [0; 64];
        put(&mut source, 0, id);
        put(&mut source, 4, unique);
        source[56..58].copy_from_slice(&8u16.to_le_bytes());
        bytes.extend(chunk(b"MODF", &source));
    }
    bytes
}
fn adt(id: u32, unique: u32) -> Vec<u8> {
    let mut record = [0; 36];
    put(&mut record, 0, id);
    put(&mut record, 4, unique);
    record[32..34].copy_from_slice(&1024u16.to_le_bytes());
    record[34..36].copy_from_slice(&0x40u16.to_le_bytes());
    chunk(b"MDDF", &record)
}
fn empty_adt() -> Vec<u8> {
    chunk(b"MVER", &18u32.to_le_bytes())
}
fn m2(collision: bool) -> Vec<u8> {
    let mut bytes = vec![0; 304];
    bytes[..4].copy_from_slice(b"MD20");
    if collision {
        put(&mut bytes, 216, 3);
        put(&mut bytes, 220, 304);
        put(&mut bytes, 224, 3);
        put(&mut bytes, 228, 310);
        bytes.extend_from_slice(&[0, 0, 1, 0, 2, 0]);
        for word in [0u32, 0, 0, 1f32.to_bits(), 0, 0, 0, 1f32.to_bits(), 0] {
            bytes.extend_from_slice(&word.to_le_bytes());
        }
    }
    bytes
}
fn wmo_files() -> Vec<CollisionFile<'static>> {
    let mut header = [0; 64];
    put(&mut header, 4, 1);
    put(&mut header, 32, 777);
    header[60] = 4;
    let mut root = chunk(b"MOHD", &header);
    root.extend(chunk(b"GFID", &30u32.to_le_bytes()));
    root.extend(chunk(b"MODI", &20u32.to_le_bytes()));
    let mut spawn = [0; 40];
    put(&mut spawn, 28, 1f32.to_bits());
    put(&mut spawn, 32, 1f32.to_bits());
    root.extend(chunk(b"MODD", &spawn));
    let mut set = [0; 32];
    put(&mut set, 24, 1);
    root.extend(chunk(b"MODS", &set));
    let mut group = vec![0; 68];
    put(&mut group, 56, 99);
    group.extend(chunk(b"MODR", &[0, 0]));
    group.extend(chunk(b"MOPY", &[8, 0]));
    group.extend(chunk(b"MOVI", &[0, 0, 1, 0, 2, 0]));
    let mut vertices = Vec::new();
    for word in [0u32, 0, 0, 1f32.to_bits(), 0, 0, 0, 1f32.to_bits(), 0] {
        vertices.extend_from_slice(&word.to_le_bytes());
    }
    group.extend(chunk(b"MOVT", &vertices));
    vec![
        (10, None, root),
        (20, None, m2(true)),
        (30, None, chunk(b"MOGP", &group)),
    ]
}
fn ids(bytes: &[u8]) -> Vec<u32> {
    DirBin::parse(bytes, placement::Limits::default())
        .unwrap()
        .records()
        .map(|value| value.id())
        .collect()
}
fn tile_outputs(stage: &Stage) -> Vec<&(Context, Vec<u8>)> {
    stage
        .placements
        .iter()
        .filter(|(context, _)| matches!(context.kind, Kind::Tile { .. }))
        .collect()
}

#[test]
fn immediate_parent_replays_own_cache_but_never_fills_child_for_grandchild() {
    let rows = [row(3, 103, 2), row(2, 102, 1), row(1, 101, -1)];
    let maps = maps(&rows);
    let fixture = CollisionFixture::new(&[
        (101, None, wdt(&[(2, 1, 201), (3, 1, 202)], true, None)),
        (102, None, wdt(&[], true, None)),
        (103, None, wdt(&[], true, None)),
        (201, None, adt(10, 7)),
        (202, None, empty_adt()),
        (10, None, m2(true)),
    ]);
    let mut stage = Stage::default();
    let report = extract(
        &mut fixture.open(false),
        &maps,
        &Lookup,
        &mut stage,
        Limits::default(),
    )
    .unwrap();
    assert_eq!(
        (
            report.own_tiles,
            report.replayed_tiles,
            report.empty_replays
        ),
        (2, 2, 1)
    );
    assert_eq!(report.cached_tiles, 2);
    assert_eq!(report.ids.calls, 1);
    assert_eq!(report.tile_positions, 3 * 4096);
    let tiles = tile_outputs(&stage);
    assert_eq!(
        tiles
            .iter()
            .map(|(context, _)| context.map_id)
            .collect::<Vec<_>>(),
        [1, 1, 2, 2]
    );
    assert_eq!(ids(&tiles[0].1), ids(&tiles[2].1));
    assert_eq!(tiles[0].1[0], 0);
    assert_eq!(tiles[2].1[0], 2);
    assert_eq!(tiles[2].0.original_map_id, 1);
    assert!(tiles[3].1.is_empty());
    assert_eq!(
        stage.placements[..3]
            .iter()
            .map(|(context, _)| context.map_id)
            .collect::<Vec<_>>(),
        [3, 2, 1]
    );
}

#[test]
fn empty_own_adt_blocks_parent_and_is_cached_for_its_child() {
    let rows = [row(1, 101, -1), row(2, 102, 1), row(3, 103, 2)];
    let fixture = CollisionFixture::new(&[
        (101, None, wdt(&[(0, 0, 201)], true, None)),
        (102, None, wdt(&[(0, 0, 202)], true, None)),
        (103, None, wdt(&[], true, None)),
        (201, None, adt(10, 7)),
        (202, None, empty_adt()),
        (10, None, m2(true)),
    ]);
    let mut stage = Stage::default();
    let report = extract(
        &mut fixture.open(false),
        &maps(&rows),
        &Lookup,
        &mut stage,
        Limits::default(),
    )
    .unwrap();
    let tiles = tile_outputs(&stage);
    assert_eq!(tiles.len(), 3);
    assert!(!tiles[1].0.replayed);
    assert!(tiles[1].1.is_empty());
    assert!(tiles[2].0.replayed);
    assert_eq!(tiles[2].0.original_map_id, 2);
    assert!(tiles[2].1.is_empty());
    assert_eq!(
        (
            report.cached_tiles,
            report.ids.calls,
            report.model_callbacks
        ),
        (2, 1, 1)
    );
}

#[test]
fn globals_are_physical_before_tiles_use_one_id_session_and_never_inherit() {
    let rows = [row(2, 102, 1), row(1, 101, -1)];
    let mut files = wmo_files();
    files.extend([
        (101, None, wdt(&[(0, 0, 201)], true, Some((10, 100)))),
        (102, None, wdt(&[], true, None)),
        (201, None, adt(20, 7)),
    ]);
    let fixture = CollisionFixture::new(&files);
    let mut stage = Stage::default();
    let report = extract(
        &mut fixture.open(false),
        &maps(&rows),
        &Lookup,
        &mut stage,
        Limits::default(),
    )
    .unwrap();
    assert_eq!(stage.placements.len(), 4);
    assert_eq!(stage.placements[0].0.map_id, 2);
    assert!(stage.placements[0].1.is_empty());
    assert_eq!(stage.placements[1].0.kind, Kind::Global);
    assert_eq!(ids(&stage.placements[1].1), [100, u32::MAX - 1]);
    assert_eq!(ids(&stage.placements[2].1), [u32::MAX - 2]);
    assert_eq!(ids(&stage.placements[3].1), [u32::MAX - 2]);
    assert_eq!((report.ids.calls, report.ids.m2_counter_consumed), (3, 2));
    assert_eq!(report.model_callbacks, 3); // Same M2 is explicitly acquired per batch.
}

#[test]
fn own_tiles_use_x_outer_y_inner_order_and_only_obj0_ids() {
    let rows = [row(1, 101, -1)];
    let fixture = CollisionFixture::new(&[
        (101, None, wdt(&[(1, 0, 201), (0, 2, 202)], true, None)),
        (201, None, adt(10, 11)),
        (202, None, adt(10, 12)),
        (10, None, m2(true)),
    ]);
    let mut stage = Stage::default();
    let report = extract(
        &mut fixture.open(false),
        &maps(&rows),
        &Lookup,
        &mut stage,
        Limits::default(),
    )
    .unwrap();
    let tiles = tile_outputs(&stage);
    assert_eq!(tiles[0].0.kind, Kind::Tile { x: 0, y: 2 });
    assert_eq!(tiles[1].0.kind, Kind::Tile { x: 1, y: 0 });
    assert_eq!(ids(&tiles[0].1), [u32::MAX - 1]);
    assert_eq!(ids(&tiles[1].1), [u32::MAX - 2]);
    assert_eq!(report.model_reads, 2);
}

#[test]
fn no_collision_models_are_reported_and_do_not_allocate_ids() {
    let rows = [row(1, 101, -1)];
    let fixture = CollisionFixture::new(&[
        (101, None, wdt(&[(0, 0, 201)], true, None)),
        (201, None, adt(10, 7)),
        (10, None, m2(false)),
    ]);
    let mut stage = Stage::default();
    let report = extract(
        &mut fixture.open(false),
        &maps(&rows),
        &Lookup,
        &mut stage,
        Limits::default(),
    )
    .unwrap();
    assert_eq!(stage.models.len(), 1);
    assert_eq!(stage.models[0].0, Status::NoCollision);
    assert!(stage.models[0].1.is_none());
    assert_eq!(
        (
            report.no_collision_models,
            report.model_raw_bytes,
            report.ids.calls
        ),
        (1, 0, 0)
    );
    assert!(tile_outputs(&stage)[0].1.is_empty());
}

#[test]
fn all_wdt_reads_must_complete_before_any_sink_callback() {
    let rows = [row(1, 101, -1), row(2, 102, -1)];
    let fixture = CollisionFixture::new(&[(101, None, wdt(&[], true, None))]);
    let mut stage = Stage::default();
    assert_eq!(
        extract(
            &mut fixture.open(false),
            &maps(&rows),
            &Lookup,
            &mut stage,
            Limits::default()
        )
        .err(),
        Some(Error::Store(local_store::Error::MissingFile))
    );
    assert!(stage.models.is_empty());
    assert!(stage.placements.is_empty());
}

#[test]
fn requested_child_adt_failure_never_replays_available_parent_cache() {
    let rows = [row(1, 101, -1), row(2, 102, 1)];
    let fixture = CollisionFixture::new(&[
        (101, None, wdt(&[(0, 0, 201)], true, None)),
        (102, None, wdt(&[(0, 0, 299)], true, None)),
        (201, None, adt(10, 7)),
        (10, None, m2(true)),
    ]);
    let mut stage = Stage::default();
    assert_eq!(
        extract(
            &mut fixture.open(false),
            &maps(&rows),
            &Lookup,
            &mut stage,
            Limits::default()
        )
        .err(),
        Some(Error::Store(local_store::Error::MissingFile))
    );
    assert_eq!(tile_outputs(&stage).len(), 1);
    assert!(
        tile_outputs(&stage)
            .iter()
            .all(|(context, _)| !context.replayed)
    );
}

#[test]
fn zero_obj0_is_fatal_and_does_not_try_root_or_obj1() {
    let rows = [row(1, 101, -1)];
    let fixture = CollisionFixture::new(&[
        (101, None, wdt(&[(0, 0, 0)], true, None)),
        (9999, None, empty_adt()),
        (9998, None, empty_adt()),
    ]);
    let mut stage = Stage::default();
    assert_eq!(
        extract(
            &mut fixture.open(false),
            &maps(&rows),
            &Lookup,
            &mut stage,
            Limits::default()
        )
        .err(),
        Some(Error::Store(local_store::Error::MissingFile))
    );
    assert!(tile_outputs(&stage).is_empty());
}

#[test]
fn malformed_complete_wdt_and_adt_are_fatal_not_empty_outputs() {
    let rows = [row(1, 101, -1)];
    for bad_wdt in [true, false] {
        let fixture = CollisionFixture::new(&[
            (
                101,
                None,
                if bad_wdt {
                    vec![0]
                } else {
                    wdt(&[(0, 0, 201)], true, None)
                },
            ),
            (201, None, vec![0]),
        ]);
        let mut stage = Stage::default();
        assert!(
            extract(
                &mut fixture.open(false),
                &maps(&rows),
                &Lookup,
                &mut stage,
                Limits::default()
            )
            .is_err()
        );
        assert!(tile_outputs(&stage).is_empty());
        assert!(stage.models.is_empty());
    }
}

#[test]
fn named_obj0_requires_existing_explicit_name_capability() {
    let rows = [row(1, 101, -1)];
    let fixture = CollisionFixture::new(&[
        (101, None, wdt(&[(2, 3, 0)], false, None)),
        (
            201,
            Some(b"World\\Maps\\Synthetic\\Synthetic_2_3_obj0.adt"),
            empty_adt(),
        ),
    ]);
    let mut stage = Stage::default();
    let error = extract(
        &mut fixture.open(false),
        &maps(&rows),
        &Lookup,
        &mut stage,
        Limits::default(),
    )
    .err()
    .unwrap();
    assert_eq!(error.code(), "local_store_names_disabled");
    let mut enabled = Stage::default();
    assert!(
        extract(
            &mut fixture.open(true),
            &maps(&rows),
            &Lookup,
            &mut enabled,
            Limits::default()
        )
        .is_ok()
    );
    assert_eq!(tile_outputs(&enabled).len(), 1);
}

#[test]
fn sink_failure_stops_before_later_acquisition_and_does_not_return_report() {
    struct Reject;
    impl Sink for Reject {
        fn model(&mut self, _: &Model) -> Result<(), SinkError> {
            Err(SinkError::Rejected)
        }
        fn placements(&mut self, _: Context, _: &[u8]) -> Result<(), SinkError> {
            Ok(())
        }
    }
    let rows = [row(1, 101, -1)];
    let fixture = CollisionFixture::new(&[
        (101, None, wdt(&[(0, 0, 201), (0, 1, 299)], true, None)),
        (201, None, adt(10, 7)),
        (10, None, m2(true)),
    ]);
    assert_eq!(
        extract(
            &mut fixture.open(false),
            &maps(&rows),
            &Lookup,
            &mut Reject,
            Limits::default()
        )
        .err(),
        Some(Error::Sink)
    );
}

#[test]
fn final_guard_detects_metadata_change_in_last_sink_callback() {
    struct Mutate<'a>(&'a CollisionFixture);
    impl Sink for Mutate<'_> {
        fn model(&mut self, _: &Model) -> Result<(), SinkError> {
            Ok(())
        }
        fn placements(&mut self, _: Context, _: &[u8]) -> Result<(), SinkError> {
            self.0.change_metadata();
            Ok(())
        }
    }
    let rows = [row(1, 101, -1)];
    let fixture = CollisionFixture::new(&[(101, None, wdt(&[], true, None))]);
    let mut store = fixture.open(false);
    assert_eq!(
        extract(
            &mut store,
            &maps(&rows),
            &Lookup,
            &mut Mutate(&fixture),
            Limits::default()
        )
        .err(),
        Some(Error::Store(local_store::Error::InputChanged))
    );
}

#[test]
fn truncated_archive_after_preload_is_fatal_without_any_tile_output() {
    struct Truncate<'a> {
        fixture: &'a CollisionFixture,
        globals: usize,
        tiles: usize,
    }
    impl Sink for Truncate<'_> {
        fn model(&mut self, _: &Model) -> Result<(), SinkError> {
            panic!("no model may be acquired from the truncated archive")
        }
        fn placements(&mut self, context: Context, _: &[u8]) -> Result<(), SinkError> {
            match context.kind {
                Kind::Global => {
                    self.globals += 1;
                    self.fixture.truncate_archive();
                }
                Kind::Tile { .. } => self.tiles += 1,
            }
            Ok(())
        }
    }
    let rows = [row(1, 101, -1)];
    let fixture = CollisionFixture::new(&[
        (101, None, wdt(&[(0, 0, 201)], true, None)),
        (201, None, empty_adt()),
    ]);
    let mut store = fixture.open(false);
    let mut sink = Truncate {
        fixture: &fixture,
        globals: 0,
        tiles: 0,
    };
    assert!(matches!(
        extract(
            &mut store,
            &maps(&rows),
            &Lookup,
            &mut sink,
            Limits::default()
        ),
        Err(Error::Store(_))
    ));
    assert_eq!((sink.globals, sink.tiles), (1, 0));
}

#[test]
fn absent_tile_with_inactive_immediate_parent_rejects_instead_of_climbing() {
    let rows = [row(1, 0, -1), row(2, 102, 1)];
    let fixture = CollisionFixture::new(&[(102, None, wdt(&[], true, None))]);
    let mut stage = Stage::default();
    assert_eq!(
        extract(
            &mut fixture.open(false),
            &maps(&rows),
            &Lookup,
            &mut stage,
            Limits::default()
        )
        .err(),
        Some(Error::Plan(plan::Error::ParentUnavailable))
    );
}

#[test]
fn complete_plan_caps_reject_before_any_sink_work() {
    let rows = [row(1, 101, -1)];
    let fixture = CollisionFixture::new(&[(101, None, wdt(&[], true, None))]);
    for limits in [
        Limits {
            max_maps: 0,
            ..Limits::default()
        },
        Limits {
            max_tile_positions: 4095,
            ..Limits::default()
        },
        Limits {
            max_wdt_memory_bytes: 0,
            ..Limits::default()
        },
        Limits {
            max_file_bytes: 1,
            ..Limits::default()
        },
    ] {
        let mut stage = Stage::default();
        assert!(
            extract(
                &mut fixture.open(false),
                &maps(&rows),
                &Lookup,
                &mut stage,
                limits
            )
            .is_err()
        );
        assert!(stage.models.is_empty());
        assert!(stage.placements.is_empty());
    }
}

#[test]
fn retained_wdt_and_parent_cache_capacity_limits_are_exact_and_reducible() {
    let rows = [row(1, 101, -1), row(2, 102, 1)];
    let fixture = CollisionFixture::new(&[
        (101, None, wdt(&[(0, 0, 201)], true, None)),
        (102, None, wdt(&[], true, None)),
        (201, None, empty_adt()),
    ]);
    let baseline = extract(
        &mut fixture.open(false),
        &maps(&rows),
        &Lookup,
        &mut Stage::default(),
        Limits::default(),
    )
    .unwrap();
    let limits = Limits {
        max_wdt_memory_bytes: baseline.wdt_resident_bytes,
        max_cache_memory_bytes: baseline.cache_peak_bytes,
        ..Limits::default()
    };
    assert!(
        extract(
            &mut fixture.open(false),
            &maps(&rows),
            &Lookup,
            &mut Stage::default(),
            limits
        )
        .is_ok()
    );
    for reduced in [
        Limits {
            max_wdt_memory_bytes: limits.max_wdt_memory_bytes - 1,
            ..limits
        },
        Limits {
            max_cache_memory_bytes: limits.max_cache_memory_bytes - 1,
            ..limits
        },
    ] {
        assert!(
            extract(
                &mut fixture.open(false),
                &maps(&rows),
                &Lookup,
                &mut Stage::default(),
                reduced
            )
            .is_err()
        );
    }
}

#[test]
fn cumulative_output_cap_counts_model_raw_and_replayed_placement_bytes() {
    let rows = [row(1, 101, -1), row(2, 102, 1)];
    let fixture = CollisionFixture::new(&[
        (101, None, wdt(&[(0, 0, 201)], true, None)),
        (102, None, wdt(&[], true, None)),
        (201, None, adt(10, 7)),
        (10, None, m2(true)),
    ]);
    let baseline = extract(
        &mut fixture.open(false),
        &maps(&rows),
        &Lookup,
        &mut Stage::default(),
        Limits::default(),
    )
    .unwrap();
    assert_eq!(
        baseline.output_bytes,
        baseline.model_raw_bytes + baseline.placement_bytes
    );
    assert_eq!(baseline.placement_bytes, 108);
    let limits = Limits {
        max_output_bytes: baseline.output_bytes,
        ..Limits::default()
    };
    assert!(
        extract(
            &mut fixture.open(false),
            &maps(&rows),
            &Lookup,
            &mut Stage::default(),
            limits
        )
        .is_ok()
    );
    assert_eq!(
        extract(
            &mut fixture.open(false),
            &maps(&rows),
            &Lookup,
            &mut Stage::default(),
            Limits {
                max_output_bytes: limits.max_output_bytes - 1,
                ..limits
            }
        )
        .err(),
        Some(Error::Limit)
    );
}

#[test]
fn raised_component_limits_reject_even_empty_map_plan() {
    let fixture = CollisionFixture::new(&[]);
    let rows = [];
    for limits in [
        Limits {
            max_maps: MAX_MAPS + 1,
            ..Limits::default()
        },
        Limits {
            max_tile_positions: MAX_TILE_POSITIONS + 1,
            ..Limits::default()
        },
        Limits {
            max_file_bytes: MAX_FILE_BYTES + 1,
            ..Limits::default()
        },
        Limits {
            max_wdt_memory_bytes: MAX_WDT_MEMORY_BYTES + 1,
            ..Limits::default()
        },
        Limits {
            max_cache_memory_bytes: MAX_CACHE_MEMORY_BYTES + 1,
            ..Limits::default()
        },
        Limits {
            objects: objects::Limits {
                max_chunks: objects::MAX_CHUNKS + 1,
                ..objects::Limits::default()
            },
            ..Limits::default()
        },
        Limits {
            placements: placements::Limits {
                max_maps: placements::MAX_MAPS + 1,
                ..placements::Limits::default()
            },
            ..Limits::default()
        },
    ] {
        let mut stage = Stage::default();
        assert!(
            extract(
                &mut fixture.open(false),
                &maps(&rows),
                &Lookup,
                &mut stage,
                limits
            )
            .is_err()
        );
        assert!(stage.placements.is_empty());
    }
}

#[test]
fn empty_plan_returns_guarded_empty_report() {
    let fixture = CollisionFixture::new(&[]);
    let rows = [];
    let mut stage = Stage::default();
    let report = extract(
        &mut fixture.open(false),
        &maps(&rows),
        &Lookup,
        &mut stage,
        Limits {
            max_maps: 0,
            max_tile_positions: 0,
            max_wdt_memory_bytes: 0,
            max_cache_memory_bytes: 0,
            max_output_bytes: 0,
            ..Limits::default()
        },
    )
    .unwrap();
    assert_eq!(
        (
            report.maps,
            report.wdt_files,
            report.ids.calls,
            report.output_bytes
        ),
        (0, 0, 0, 0)
    );
    assert!(stage.models.is_empty());
    assert!(stage.placements.is_empty());
}

#[test]
fn checked_total_counters_never_wrap() {
    let mut value = u64::MAX;
    assert_eq!(bump(&mut value, 1), Err(Error::Limit));
    assert_eq!(value, u64::MAX);
}
