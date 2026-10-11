// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: complete synthetic stores, persistent startup/world cache.
use super::*;
use crate::{
    collision,
    local_store::tests::{CollisionFile, CollisionFixture},
};
use forever_db2::terrain_sources::{DirectRecord, VMapFields};
use forever_vmap::{placement::DirBin, wmo::raw};

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
    let mut root = chunk(b"MVER", &17u32.to_le_bytes());
    root.extend(chunk(b"MOHD", &header));
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

fn words(values: &[u32]) -> Vec<u8> {
    values.iter().flat_map(|word| word.to_le_bytes()).collect()
}
// Literal numeric WDC5 fixture, independent of generated schema objects.
fn db2(rows: &[(u32, u32)], copies: &[(u32, u32)]) -> Vec<u8> {
    let mut bytes = vec![0; 496];
    bytes[..4].copy_from_slice(b"WDC5");
    for (at, value) in [
        (4, 5),
        (136, rows.len() as u32),
        (140, 9),
        (144, 52),
        (156, 0x16D1_DD61),
        (164, u32::MAX),
        (176, 9),
        (180, 52),
        (188, 216),
        (200, 1),
        (212, 496),
        (216, rows.len() as u32),
        (228, rows.len() as u32 * 4),
        (240, copies.len() as u32),
    ] {
        put(&mut bytes, at, value);
    }
    bytes[172..174].copy_from_slice(&4u16.to_le_bytes());
    bytes[174..176].copy_from_slice(&u16::MAX.to_le_bytes());
    for (field, (offset, bits, count)) in [
        (0u16, 32u16, 6u16),
        (24, 32, 1),
        (28, 16, 1),
        (30, 32, 1),
        (34, 32, 1),
        (38, 32, 1),
        (42, 32, 1),
        (46, 32, 1),
        (50, 16, 1),
    ]
    .into_iter()
    .enumerate()
    {
        let primitive = 244 + field * 4;
        bytes[primitive..primitive + 2].copy_from_slice(&(32 - bits).to_le_bytes());
        bytes[primitive + 2..primitive + 4].copy_from_slice(&offset.to_le_bytes());
        let col = 280 + field * 24;
        bytes[col..col + 2].copy_from_slice(&(offset * 8).to_le_bytes());
        bytes[col + 2..col + 4].copy_from_slice(&(bits * count).to_le_bytes());
    }
    for (_, fdid) in rows {
        let mut record = [0; 52];
        put(&mut record, 24, *fdid);
        bytes.extend_from_slice(&record);
    }
    for (id, _) in rows {
        bytes.extend_from_slice(&id.to_le_bytes());
    }
    for (new, old) in copies {
        bytes.extend(words(&[*new, *old]));
    }
    bytes
}
fn fixture(rows: &[(u32, u32)], mut files: Vec<CollisionFile<'_>>) -> CollisionFixture {
    files.push((gameobjects::TABLE_FILE_DATA_ID, None, db2(rows, &[])));
    CollisionFixture::new(&files)
}
fn startup(store: &mut LocalStore) -> gameobjects::Startup<'_> {
    gameobjects::extract_startup_fdid(store, gameobjects::Limits::default()).unwrap()
}
fn assert_aborted(context: gameobjects::Startup<'_>) {
    assert_eq!(
        context.finish().err(),
        Some(gameobjects::Error::Collision(collision::Error::Aborted))
    );
}

#[test]
fn startup_cache_and_current_request_window_are_shared_across_two_tiles() {
    let fixture = fixture(
        &[(1, 10)],
        vec![
            (10, None, m2(true)),
            (101, None, wdt(&[(0, 0, 201), (0, 1, 202)], true, None)),
            (201, None, adt(10, 7)),
            (202, None, adt(10, 8)),
        ],
    );
    let rows = [row(1, 101, -1)];
    let mut store = fixture.open(false);
    let mut context = startup(&mut store);
    let initial = context.collector_mut().models()[0].raw().unwrap().to_vec();
    let before_external = context.collector_mut().external;
    let mut stage = Stage::default();
    let report = extract_shared(
        &mut context,
        &maps(&rows),
        &mut stage,
        SharedLimits::default(),
    )
    .unwrap();
    assert_eq!(report.collector_before.reads, 2);
    assert_eq!(report.collector_after.reads, 5);
    assert_eq!(report.run.model_reads, 0);
    assert_eq!(report.run.model_source_bytes, 0);
    assert_eq!(
        (report.request_calls_before, report.request_calls_after),
        (1, 3)
    );
    assert_eq!(
        (
            report.run.model_callbacks,
            report.run.own_tiles,
            report.run.ids.calls
        ),
        (1, 2, 2)
    );
    assert_eq!(stage.models[0].1.as_deref(), Some(initial.as_slice()));
    let tiles = tile_outputs(&stage);
    assert_eq!(ids(&tiles[0].1), [u32::MAX - 1]);
    assert_eq!(ids(&tiles[1].1), [u32::MAX - 2]);
    assert_eq!(context.collector_mut().external, before_external);
    assert_eq!(context.collector_mut().external_sources, 0);
    let completed = context.finish().unwrap();
    assert_eq!(completed.extraction().requests().len(), 3);
    assert_eq!(completed.extraction().models().len(), 1);
}

#[test]
fn dependency_handles_remain_valid_when_only_wmo_request_window_is_projected() {
    let mut files = wmo_files();
    files.push((101, None, wdt(&[], true, Some((10, 8)))));
    let fixture = fixture(&[(1, 10)], files);
    let rows = [row(1, 101, -1)];
    let mut store = fixture.open(false);
    let mut context = startup(&mut store);
    let mut stage = Stage::default();
    let report = extract_shared(
        &mut context,
        &maps(&rows),
        &mut stage,
        SharedLimits::default(),
    )
    .unwrap();
    assert_eq!(report.run.model_reads, 0);
    assert_eq!(report.run.model_callbacks, 2);
    assert_eq!(report.run.placement_records, 2);
    assert_eq!(report.run.ids.calls, 2);
    assert_eq!(ids(&stage.placements[0].1), [8, u32::MAX - 1]);
    assert_eq!(context.finish().unwrap().extraction().models().len(), 2);
}

struct Visual;
impl LiquidLookup for Visual {
    fn material_id(&self, _: u32) -> Option<u8> {
        Some(0)
    }
    fn material_flags(&self, _: u8) -> Option<u32> {
        Some(2)
    }
}
#[test]
fn lookup_transition_preserves_startup_raw_and_only_new_models_use_loaded_tables() {
    let mut files = wmo_files();
    // Header-only liquid 22 in both groups. Their RAW differs only because the
    // first root is cached during empty-lookup startup, before the transition.
    put(&mut files[2].2, 8 + 52, 22);
    let mut second_root = files[0].2.clone();
    // GFID is the single word after MVER12 + MOHD72 + GFID8.
    put(&mut second_root, 92, 31);
    let second_group = files[2].2.clone();
    files.extend([
        (11, None, second_root),
        (31, None, second_group),
        (101, None, wdt(&[], true, Some((10, 3)))),
        (102, None, wdt(&[], true, Some((11, 4)))),
    ]);
    let fixture = fixture(&[(1, 10)], files);
    let rows = [row(1, 101, -1), row(2, 102, -1)];
    let mut store = fixture.open(false);
    let initial = startup(&mut store);
    let mut context = initial.with_lookup(&Visual).unwrap();
    let before = context.collector_mut().models()[0].raw().unwrap().to_vec();
    let mut stage = Stage::default();
    let report = extract_shared(
        &mut context,
        &maps(&rows),
        &mut stage,
        SharedLimits::default(),
    )
    .unwrap();
    assert_eq!(report.run.model_reads, 2);
    assert_eq!(stage.models.len(), 3); // cached root+dependency; new root.
    assert_eq!(stage.models[0].1.as_deref(), Some(before.as_slice()));
    let raw_a = raw::RawWmo::parse(
        stage.models[0].1.as_ref().unwrap(),
        raw::ConversionLimits::default(),
    )
    .unwrap();
    let raw_b = raw::RawWmo::parse(
        stage.models[2].1.as_ref().unwrap(),
        raw::ConversionLimits::default(),
    )
    .unwrap();
    assert_eq!(raw_a.groups().next().unwrap().liquid_flags(), 2);
    assert_eq!(raw_b.groups().next().unwrap().liquid_flags(), 0);
    context.finish().unwrap();
}

#[test]
fn own_parent_cache_replay_keeps_ids_and_does_not_fill_grandchild() {
    let rows = [row(3, 103, 2), row(2, 102, 1), row(1, 101, -1)];
    let fixture = fixture(
        &[],
        vec![
            (101, None, wdt(&[(2, 1, 201), (3, 1, 202)], true, None)),
            (102, None, wdt(&[], true, None)),
            (103, None, wdt(&[], true, None)),
            (201, None, adt(10, 7)),
            (202, None, empty_adt()),
            (10, None, m2(true)),
        ],
    );
    let mut store = fixture.open(false);
    let mut context = startup(&mut store);
    let mut stage = Stage::default();
    let report = extract_shared(
        &mut context,
        &maps(&rows),
        &mut stage,
        SharedLimits::default(),
    )
    .unwrap();
    assert_eq!(
        (
            report.run.own_tiles,
            report.run.replayed_tiles,
            report.run.empty_replays,
            report.run.cached_tiles
        ),
        (2, 2, 1, 2)
    );
    assert_eq!(report.run.ids.calls, 1);
    let tiles = tile_outputs(&stage);
    assert_eq!(
        tiles.iter().map(|(c, _)| c.map_id).collect::<Vec<_>>(),
        [1, 1, 2, 2]
    );
    assert_eq!(ids(&tiles[0].1), ids(&tiles[2].1));
    assert_eq!(tiles[2].1[0], 2);
    assert!(tiles[3].1.is_empty());
    assert_eq!(context.collector_mut().external_sources, 0);
    context.finish().unwrap();
}

#[test]
fn own_empty_tile_blocks_parent_and_is_replayed_by_its_child() {
    let rows = [row(1, 101, -1), row(2, 102, 1), row(3, 103, 2)];
    let fixture = fixture(
        &[],
        vec![
            (101, None, wdt(&[(0, 0, 201)], true, None)),
            (102, None, wdt(&[(0, 0, 202)], true, None)),
            (103, None, wdt(&[], true, None)),
            (201, None, adt(10, 7)),
            (202, None, empty_adt()),
            (10, None, m2(true)),
        ],
    );
    let mut store = fixture.open(false);
    let mut context = startup(&mut store);
    let mut stage = Stage::default();
    let report = extract_shared(
        &mut context,
        &maps(&rows),
        &mut stage,
        SharedLimits::default(),
    )
    .unwrap();
    let tiles = tile_outputs(&stage);
    assert_eq!(tiles.len(), 3);
    assert!(!tiles[1].0.replayed);
    assert!(tiles[1].1.is_empty());
    assert!(tiles[2].0.replayed);
    assert_eq!(tiles[2].0.original_map_id, 2);
    assert!(tiles[2].1.is_empty());
    assert_eq!((report.run.cached_tiles, report.run.ids.calls), (2, 1));
    context.finish().unwrap();
}

#[test]
fn physical_globals_precede_depth_order_tiles_and_are_not_inherited() {
    let rows = [row(2, 102, 1), row(1, 101, -1)];
    let mut files = wmo_files();
    files.extend([
        (101, None, wdt(&[(0, 0, 201)], true, Some((10, 10)))),
        (102, None, wdt(&[], true, None)),
        (201, None, adt(20, 11)),
    ]);
    let fixture = fixture(&[], files);
    let mut store = fixture.open(false);
    let mut context = startup(&mut store);
    let mut stage = Stage::default();
    let report = extract_shared(
        &mut context,
        &maps(&rows),
        &mut stage,
        SharedLimits::default(),
    )
    .unwrap();
    assert_eq!(stage.placements[0].0.map_id, 2);
    assert!(stage.placements[0].1.is_empty());
    assert_eq!(ids(&stage.placements[1].1), [10, u32::MAX - 1]);
    let tiles = tile_outputs(&stage);
    assert_eq!(ids(&tiles[0].1), [u32::MAX - 2]);
    assert_eq!(ids(&tiles[1].1), [u32::MAX - 2]);
    assert_eq!(report.run.ids.calls, 3);
    assert_eq!(report.run.model_callbacks, 2);
    context.finish().unwrap();
}

#[test]
fn no_collision_startup_models_are_delivered_once_and_placements_consume_no_id() {
    let fixture = fixture(
        &[(1, 10)],
        vec![
            (10, None, m2(false)),
            (101, None, wdt(&[(0, 0, 201)], true, None)),
            (201, None, adt(10, 3)),
        ],
    );
    let rows = [row(1, 101, -1)];
    let mut store = fixture.open(false);
    let mut context = startup(&mut store);
    let mut stage = Stage::default();
    let report = extract_shared(
        &mut context,
        &maps(&rows),
        &mut stage,
        SharedLimits::default(),
    )
    .unwrap();
    assert_eq!(report.run.no_collision_models, 1);
    assert_eq!(report.run.model_callbacks, 1);
    assert_eq!(report.run.ids.calls, 0);
    assert_eq!(stage.models[0].0, Status::NoCollision);
    assert!(tile_outputs(&stage)[0].1.is_empty());
    context.finish().unwrap();
}

#[test]
fn second_run_is_rejected_even_after_lookup_transition() {
    let fixture = fixture(&[], vec![]);
    let mut store = fixture.open(false);
    let mut context = startup(&mut store);
    let mut stage = Stage::default();
    extract_shared(
        &mut context,
        &maps(&[]),
        &mut stage,
        SharedLimits::default(),
    )
    .unwrap();
    let mut context = context.with_lookup(&Visual).unwrap();
    assert_eq!(
        extract_shared(
            &mut context,
            &maps(&[]),
            &mut stage,
            SharedLimits::default()
        ),
        Err(Error::Collision(collision::Error::State))
    );
    assert_eq!(
        context.finish().err(),
        Some(gameobjects::Error::Collision(collision::Error::State))
    );
}

struct Reject {
    models: bool,
}
impl Sink for Reject {
    fn model(&mut self, _: &Model) -> Result<(), SinkError> {
        if self.models {
            Err(SinkError::Rejected)
        } else {
            Ok(())
        }
    }
    fn placements(&mut self, _: Context, _: &[u8]) -> Result<(), SinkError> {
        Err(SinkError::Rejected)
    }
}
#[test]
fn model_sink_failure_poison_prevents_finish_transition_and_new_reads() {
    let fixture = fixture(&[(1, 10)], vec![(10, None, m2(true))]);
    let mut store = fixture.open(false);
    let mut context = startup(&mut store);
    assert_eq!(
        extract_shared(
            &mut context,
            &maps(&[]),
            &mut Reject { models: true },
            SharedLimits::default()
        ),
        Err(Error::Sink)
    );
    let before = context.collector_mut().facts();
    assert_eq!(
        context
            .collector_mut()
            .extract(ModelRequest::M2(InputRef::FileDataId(10)))
            .err(),
        Some(collision::Error::Aborted)
    );
    assert_eq!(context.collector_mut().facts(), before);
    assert_eq!(
        context.with_lookup(&Visual).err(),
        Some(gameobjects::Error::Collision(collision::Error::Aborted))
    );
}
#[test]
fn placements_sink_failure_poison_survives_finish() {
    let fixture = fixture(&[], vec![(101, None, wdt(&[], true, None))]);
    let rows = [row(1, 101, -1)];
    let mut store = fixture.open(false);
    let mut context = startup(&mut store);
    assert_eq!(
        extract_shared(
            &mut context,
            &maps(&rows),
            &mut Reject { models: false },
            SharedLimits::default()
        ),
        Err(Error::Sink)
    );
    assert_aborted(context);
}
#[test]
fn missing_own_adt_is_terminal_instead_of_parent_fallback() {
    let fixture = fixture(
        &[],
        vec![
            (101, None, wdt(&[(0, 0, 201)], true, None)),
            (102, None, wdt(&[(0, 0, 999)], true, None)),
            (201, None, empty_adt()),
        ],
    );
    let rows = [row(1, 101, -1), row(2, 102, 1)];
    let mut store = fixture.open(false);
    let mut context = startup(&mut store);
    let error = extract_shared(
        &mut context,
        &maps(&rows),
        &mut Stage::default(),
        SharedLimits::default(),
    )
    .unwrap_err();
    let Error::Collision(cause) = error else {
        panic!("expected source error")
    };
    assert_eq!(
        context.finish().err(),
        Some(gameobjects::Error::Collision(cause))
    );
}
#[test]
fn malformed_late_adt_poison_cannot_publish_earlier_success() {
    let fixture = fixture(
        &[],
        vec![
            (101, None, wdt(&[(0, 0, 201), (0, 1, 202)], true, None)),
            (201, None, empty_adt()),
            (202, None, b"bad".to_vec()),
        ],
    );
    let rows = [row(1, 101, -1)];
    let mut store = fixture.open(false);
    let mut context = startup(&mut store);
    let mut stage = Stage::default();
    assert!(matches!(
        extract_shared(
            &mut context,
            &maps(&rows),
            &mut stage,
            SharedLimits::default()
        ),
        Err(Error::Objects(_))
    ));
    assert_eq!(tile_outputs(&stage).len(), 1);
    assert_aborted(context);
}
#[test]
fn named_obj0_requires_explicit_store_names_and_is_not_model_normalized() {
    let name = b"World\\Maps\\Synthetic\\Synthetic_0_0_obj0.adt";
    let fixture = fixture(
        &[],
        vec![
            (101, None, wdt(&[(0, 0, 0)], false, None)),
            (201, Some(name), empty_adt()),
        ],
    );
    let rows = [row(1, 101, -1)];
    let mut store = fixture.open(true);
    let mut context = startup(&mut store);
    let report = extract_shared(
        &mut context,
        &maps(&rows),
        &mut Stage::default(),
        SharedLimits::default(),
    )
    .unwrap();
    assert_eq!(report.run.object_adts, 1);
    context.finish().unwrap();
    let mut store = fixture.open(false);
    let mut context = startup(&mut store);
    assert!(matches!(
        extract_shared(
            &mut context,
            &maps(&rows),
            &mut Stage::default(),
            SharedLimits::default()
        ),
        Err(Error::Collision(_))
    ));
    assert!(context.finish().is_err());
}
#[test]
fn cumulative_read_and_request_caps_include_startup_and_prior_tiles() {
    let fixture = fixture(
        &[(1, 10)],
        vec![
            (10, None, m2(true)),
            (101, None, wdt(&[(0, 0, 201), (0, 1, 202)], true, None)),
            (201, None, adt(10, 1)),
            (202, None, adt(10, 2)),
        ],
    );
    let rows = [row(1, 101, -1)];
    for requests in [false, true] {
        let mut limits = gameobjects::Limits::default();
        if requests {
            limits.collision.max_requests = 2;
        } else {
            limits.collision.max_reads = 3;
        }
        let mut store = fixture.open(false);
        let mut context = gameobjects::extract_startup_fdid(&mut store, limits).unwrap();
        assert_eq!(
            extract_shared(
                &mut context,
                &maps(&rows),
                &mut Stage::default(),
                SharedLimits::default()
            )
            .err(),
            Some(Error::Collision(collision::Error::Limit))
        );
        assert_eq!(
            context.finish().err(),
            Some(gameobjects::Error::Collision(collision::Error::Limit))
        );
    }
}
#[test]
fn component_reductions_fail_whole_run_and_poison() {
    let fixture = fixture(
        &[(1, 10)],
        vec![
            (10, None, m2(true)),
            (101, None, wdt(&[(0, 0, 201)], true, None)),
            (102, None, wdt(&[], true, None)),
            (201, None, adt(10, 1)),
        ],
    );
    let rows = [row(1, 101, -1), row(2, 102, 1)];
    for variant in 0..8 {
        let mut limits = SharedLimits::default();
        match variant {
            0 => limits.max_maps = 1,
            1 => limits.max_tile_positions = 8191,
            2 => limits.max_file_bytes = 1,
            3 => limits.max_wdt_memory_bytes = 1,
            4 => limits.max_cache_memory_bytes = 0,
            5 => limits.max_output_bytes = 0,
            6 => limits.placements.max_memory_bytes = 0,
            _ => limits.objects.max_memory_bytes = 0,
        }
        let mut store = fixture.open(false);
        let mut context = startup(&mut store);
        assert!(
            extract_shared(&mut context, &maps(&rows), &mut Stage::default(), limits).is_err(),
            "variant {variant}"
        );
        assert!(context.finish().is_err());
    }
}
#[test]
fn external_lookup_reservations_survive_success_and_are_not_released_by_runner() {
    let fixture = fixture(&[], vec![]);
    let mut store = fixture.open(false);
    let mut context = startup(&mut store);
    context.collector_mut().reserve_external(12345).unwrap();
    let before = context.collector_mut().external;
    extract_shared(
        &mut context,
        &maps(&[]),
        &mut Stage::default(),
        SharedLimits::default(),
    )
    .unwrap();
    assert_eq!(context.collector_mut().external, before);
    context.collector_mut().release_external(12345).unwrap();
    context.finish().unwrap();
}
#[test]
fn final_store_guard_after_sink_mutation_poison_preserves_input_changed() {
    struct Change<'a>(&'a CollisionFixture);
    impl Sink for Change<'_> {
        fn model(&mut self, _: &Model) -> Result<(), SinkError> {
            self.0.change_metadata();
            Ok(())
        }
        fn placements(&mut self, _: Context, _: &[u8]) -> Result<(), SinkError> {
            Ok(())
        }
    }
    let fixture = fixture(&[(1, 10)], vec![(10, None, m2(true))]);
    let mut store = fixture.open(false);
    let mut context = startup(&mut store);
    assert_eq!(
        extract_shared(
            &mut context,
            &maps(&[]),
            &mut Change(&fixture),
            SharedLimits::default()
        )
        .err(),
        Some(Error::Collision(collision::Error::Store(
            local_store::Error::InputChanged
        )))
    );
    assert_eq!(
        context.finish().err(),
        Some(gameobjects::Error::Collision(collision::Error::Store(
            local_store::Error::InputChanged
        )))
    );
}
#[test]
fn invalid_upward_limits_are_terminal_before_any_world_read() {
    let fixture = fixture(&[], vec![]);
    let mut store = fixture.open(false);
    let mut context = startup(&mut store);
    let before = context.collector_mut().facts();
    let limits = SharedLimits {
        max_maps: MAX_MAPS + 1,
        ..SharedLimits::default()
    };
    assert_eq!(
        extract_shared(&mut context, &maps(&[]), &mut Stage::default(), limits),
        Err(Error::InvalidLimits)
    );
    assert_eq!(context.collector_mut().facts(), before);
    assert_aborted(context);
}

#[test]
fn zero_wdt_parser_memory_accepts_borrowed_tables_but_rejects_global_descriptors() {
    let rows = [row(1, 101, -1)];
    let no_globals = fixture(&[], vec![(101, None, wdt(&[], true, None))]);
    let mut store = no_globals.open(false);
    let mut context = startup(&mut store);
    let mut limits = SharedLimits::default();
    limits.wdt.max_memory_bytes = 0;
    extract_shared(&mut context, &maps(&rows), &mut Stage::default(), limits).unwrap();
    context.finish().unwrap();

    let mut files = wmo_files();
    files.push((101, None, wdt(&[], true, Some((10, 100)))));
    let with_globals = fixture(&[], files);
    let mut store = with_globals.open(false);
    let mut context = startup(&mut store);
    let control = extract_shared(
        &mut context,
        &maps(&rows),
        &mut Stage::default(),
        SharedLimits::default(),
    )
    .unwrap();
    assert_eq!(control.run.placement_records, 2);
    context.finish().unwrap();
    let mut store = with_globals.open(false);
    let mut context = startup(&mut store);
    assert!(matches!(
        extract_shared(&mut context, &maps(&rows), &mut Stage::default(), limits),
        Err(Error::Plan(_))
    ));
    assert_aborted(context);
}

#[test]
fn ninth_parent_tile_grows_cache_descriptors_with_overlap_then_replays_all_ids() {
    let rows = [row(1, 101, -1), row(2, 102, 1)];
    let tiles: Vec<_> = (0..9).map(|y| (0, y, 201 + y as u32)).collect();
    let mut files = vec![
        (101, None, wdt(&tiles, true, None)),
        (102, None, wdt(&[], true, None)),
        (10, None, m2(true)),
    ];
    files.extend((0..9).map(|n| (201 + n, None, adt(10, 100 + n))));
    let fixture = fixture(&[], files);
    let mut store = fixture.open(false);
    let mut context = startup(&mut store);
    let before_external = context.collector_mut().external;
    let mut stage = Stage::default();
    let report = extract_shared(
        &mut context,
        &maps(&rows),
        &mut stage,
        SharedLimits::default(),
    )
    .unwrap();
    assert_eq!((report.run.cached_tiles, report.run.replayed_tiles), (9, 9));
    assert_eq!(report.run.ids.calls, 9);
    assert_eq!(report.run.model_reads, 1);
    assert_eq!(report.run.model_callbacks, 1);
    assert_eq!(report.collector_after.reads, 13);
    let outputs = tile_outputs(&stage);
    assert_eq!(outputs.len(), 18);
    for (own, inherited) in outputs[..9].iter().zip(&outputs[9..]) {
        assert_eq!(ids(&own.1), ids(&inherited.1));
        assert_eq!(own.0.map_id, 1);
        assert_eq!(inherited.0.map_id, 2);
        assert_eq!(own.1[0], 0);
        assert_eq!(inherited.1[0], 2);
        assert_eq!(&own.1[1..], &inherited.1[1..]);
    }
    // The ninth insertion allocates sixteen descriptors while the old eight
    // are still live. Cache values also remain live during that growth.
    assert_eq!(
        report.run.cache_peak_bytes - report.run.cache_resident_bytes,
        8 * size_of::<TileCache>()
    );
    assert_eq!(context.collector_mut().external, before_external);
    assert_eq!(context.collector_mut().external_sources, 0);
    context.finish().unwrap();

    // The exact measured overlap ceiling succeeds. One byte less fails even
    // though the eventual resident cache would fit: no earlier tile is success.
    let ceiling = report.run.cache_peak_bytes;
    assert!(ceiling > report.run.cache_resident_bytes);
    for cap in [ceiling, ceiling - 1] {
        let mut store = fixture.open(false);
        let mut context = startup(&mut store);
        let mut stage = Stage::default();
        let result = extract_shared(
            &mut context,
            &maps(&rows),
            &mut stage,
            SharedLimits {
                max_cache_memory_bytes: cap,
                ..SharedLimits::default()
            },
        );
        if cap == ceiling {
            let report = result.unwrap();
            assert_eq!(report.run.cache_peak_bytes, ceiling);
            assert_eq!(report.run.replayed_tiles, 9);
            context.finish().unwrap();
        } else {
            assert_eq!(result, Err(Error::Limit));
            assert_eq!(tile_outputs(&stage).len(), 9);
            assert_aborted(context);
        }
    }
}
