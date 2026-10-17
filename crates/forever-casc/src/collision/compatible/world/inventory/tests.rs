// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-17: synthetic compatible serial world/owned-cache controls.
use super::*;
use crate::collision::{
    InputRef, ModelRequest,
    compatible::Outcome,
    gameobjects,
    resources::Profile,
    spool::{self, BackendProfile, ProtectedInputs, Spool},
    world::{self, tests as fixture},
};
use crate::{
    Locale, local_index,
    local_store::{
        self, LocalStore,
        tests::{CollisionFile, CollisionFixture},
    },
    root,
};
use forever_db2::catalog;
use forever_vmap::placement::{self, DirBin};
use std::{
    cell::{Cell, RefCell},
    fs,
    os::unix::fs::{DirBuilderExt, PermissionsExt},
    path::PathBuf,
    rc::Rc,
    sync::atomic::{AtomicU64, Ordering},
};

fn put(b: &mut [u8], at: usize, n: u32) {
    b[at..at + 4].copy_from_slice(&n.to_le_bytes());
}
fn chunk(tag: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut out: Vec<_> = tag.iter().rev().copied().collect();
    out.extend((data.len() as u32).to_le_bytes());
    out.extend(data);
    out
}
fn files(build: u32, maps: &[(u32, u32, i16)]) -> Vec<CollisionFile<'static>> {
    let mut rows = fixture::files(maps);
    for (id, _, b) in &mut rows {
        stamp(build, *id, b);
    }
    rows
}
fn stamp(build: u32, id: u32, b: &mut [u8]) {
    let name = match id {
        gameobjects::TABLE_FILE_DATA_ID => "GameObjectDisplayInfo",
        world::MAP_FILE_DATA_ID => "Map",
        world::MATERIAL_FILE_DATA_ID => "LiquidMaterial",
        world::TYPE_FILE_DATA_ID => "LiquidType",
        _ => unreachable!(),
    };
    let catalog = if build == 70425 {
        catalog::WOWDBDEFS_70425_CATALOG_ID
    } else {
        catalog::WOWDBDEFS_70374_CATALOG_ID
    };
    let meta = catalog::resolve_for(catalog, name)
        .unwrap()
        .unwrap()
        .metadata();
    assert_eq!(
        u32::from_le_bytes(b[156..160].try_into().unwrap()),
        meta.layout_hash
    );
    put(b, 152, meta.table_hash.unwrap());
}
fn godi(rows: &mut [CollisionFile<'_>], build: u32, requests: &[(u32, u32)]) {
    let b = &mut rows
        .iter_mut()
        .find(|v| v.0 == gameobjects::TABLE_FILE_DATA_ID)
        .unwrap()
        .2;
    *b = fixture::gameobject_db2(requests, &[]);
    stamp(build, gameobjects::TABLE_FILE_DATA_ID, b);
}
fn wdt(tile: Option<u32>) -> Vec<u8> {
    fixture::wdt(None, tile)
}
fn m2_adt(id: u32, unique: u32) -> Vec<u8> {
    let mut b = [0; 36];
    put(&mut b, 0, id);
    put(&mut b, 4, unique);
    b[32..34].copy_from_slice(&1024u16.to_le_bytes());
    b[34..36].copy_from_slice(&0x40u16.to_le_bytes());
    chunk(b"MDDF", &b)
}
fn empty_adt() -> Vec<u8> {
    chunk(b"MVER", &18u32.to_le_bytes())
}
fn open(f: &CollisionFixture, build: u32) -> LocalStore {
    let o = local_store::OpenOptions {
        locale: Locale::EnUs,
        index_layouts: [local_index::Layout::Contiguous; 16],
        root_layout: root::Layout::Header50893V2,
        limits: local_store::Limits::default(),
    };
    match build {
        70374 => LocalStore::open_db2_70374(f.path(), o),
        70425 => LocalStore::open_db2_70425(f.path(), o),
        _ => unreachable!(),
    }
    .unwrap()
}
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Private(PathBuf);
impl Private {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "rce-compatible-inventory-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::DirBuilder::new().mode(0o700).create(&p).unwrap();
        Self(p)
    }
    fn spool(
        &self,
        f: &CollisionFixture,
        name: &str,
        profile: Profile,
        backend: BackendProfile,
    ) -> Spool {
        Spool::create_with_backend(
            &self.0.join(name),
            ProtectedInputs::capture(f.path(), &[]).unwrap(),
            profile.spool_limits(),
            spool::Profile::SameMountV1,
            profile,
            backend,
        )
        .unwrap()
    }
    fn inventories(
        &self,
        f: &CollisionFixture,
        profile: Profile,
        backend: BackendProfile,
        l: Limits,
    ) -> (Inventory, Inventory) {
        let one = |name| {
            Inventory::new(
                self.spool(f, name, profile, backend),
                inventory::Limits {
                    max_heap_bytes: 256 * 1024 * 1024,
                    wdt: l.world.run.wdt,
                    cache: l.world.run.cache,
                    ..profile.inventory_limits()
                },
            )
            .unwrap()
        };
        (one("wdts"), one("cache"))
    }
}
impl Drop for Private {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
type Outputs = Rc<RefCell<Vec<(crate::collision::run::Context, Vec<u8>)>>>;
struct Sink {
    out: Outputs,
    drops: Rc<Cell<usize>>,
    fail: bool,
    mutate_drop: Option<PathBuf>,
    chmod_drop: Option<PathBuf>,
    chmod_stage: Option<PathBuf>,
}
impl Sink {
    fn new() -> (Self, Outputs, Rc<Cell<usize>>) {
        let out = Rc::new(RefCell::new(Vec::with_capacity(32)));
        let drops = Rc::new(Cell::new(0));
        (
            Self {
                out: out.clone(),
                drops: drops.clone(),
                fail: false,
                mutate_drop: None,
                chmod_drop: None,
                chmod_stage: None,
            },
            out,
            drops,
        )
    }
}
impl run::Sink for Sink {
    type Error = u8;
    fn placements(&mut self, c: crate::collision::run::Context, b: &[u8]) -> Result<(), u8> {
        if let Some(p) = &self.chmod_stage {
            fs::set_permissions(p, fs::Permissions::from_mode(0o755)).unwrap();
        }
        if self.fail {
            return Err(7);
        }
        let mut out = self.out.borrow_mut();
        // Entire retained observation is inside the declared 64 KiB allowance;
        // each bounded fixture emits <32 callbacks, without vector growth.
        assert!(out.len() < out.capacity());
        assert!(b.len() < 4096);
        let bytes = b.to_vec();
        assert!(bytes.capacity() < 4096);
        assert!(
            out.iter().map(|v| v.1.capacity()).sum::<usize>()
                + bytes.capacity()
                + out.capacity() * size_of::<(crate::collision::run::Context, Vec<u8>)>()
                < 65536
        );
        out.push((c, bytes));
        Ok(())
    }
}
impl Drop for Sink {
    fn drop(&mut self) {
        self.drops.set(self.drops.get() + 1);
        if let Some(p) = &self.mutate_drop {
            fs::write(p, b"synthetic-world-sink-drop").unwrap();
        }
        if let Some(p) = &self.chmod_drop {
            fs::set_permissions(p, fs::Permissions::from_mode(0o755)).unwrap();
        }
    }
}
fn start<'s>(
    s: &'s mut LocalStore,
    f: &CollisionFixture,
    p: &Private,
    build: u32,
    profile: Profile,
    native: bool,
) -> startup::Startup<'s> {
    let spool = p.spool(f, "payload", profile, BackendProfile::LeasedAppendV1);
    match (build, native) {
        (70374, true) => {
            startup::extract_startup_fdid_native_gnu15_70374(s, profile.startup_limits(), spool)
        }
        (70425, true) => {
            startup::extract_startup_fdid_native_gnu15_70425(s, profile.startup_limits(), spool)
        }
        (70374, false) => startup::extract_startup_fdid_70374(s, profile.startup_limits(), spool),
        (70425, false) => startup::extract_startup_fdid_70425(s, profile.startup_limits(), spool),
        _ => unreachable!(),
    }
    .unwrap()
}
fn run_case<'s>(
    c: startup::Startup<'s>,
    f: &CollisionFixture,
    p: &Private,
    sink: Sink,
    profile: Profile,
    l: Limits,
) -> Result<Completed, Error<u8>> {
    let (w, k) = p.inventories(f, profile, BackendProfile::LeasedAppendV1, l);
    extract(c, w, k, sink, 65536, l)
}
fn record_ids(b: &[u8]) -> Vec<u32> {
    DirBin::parse(b, placement::Limits::default())
        .unwrap()
        .records()
        .map(|v| v.id())
        .collect()
}
fn tile_outputs(o: &Outputs) -> Vec<(crate::collision::run::Context, Vec<u8>)> {
    o.borrow()
        .iter()
        .filter(|v| matches!(v.0.kind, crate::collision::run::Kind::Tile { .. }))
        .cloned()
        .collect()
}

#[test]
fn both_targets_consume_authentic_preparation_into_completed_world() {
    for build in [70374, 70425] {
        let mut rows = files(build, &[(1, 100, -1)]);
        rows.extend([
            (100, None, wdt(Some(101))),
            (101, None, m2_adt(90, 17)),
            (90, None, fixture::m2(true)),
        ]);
        let f = CollisionFixture::for_build(&rows, build);
        let p = Private::new();
        let mut s = open(&f, build);
        let (sink, out, drops) = Sink::new();
        let c = start(&mut s, &f, &p, build, Profile::BoundedV1, true);
        let prepared = super::super::prepare(c, sink, 65536, world::Limits::default()).unwrap();
        let before = prepared.map_stats().unwrap();
        let l = Limits::default();
        let (w, k) = p.inventories(&f, Profile::BoundedV1, BackendProfile::LeasedAppendV1, l);
        let mut done = extract_prepared(prepared, w, k, l.max_combined_memory_bytes).unwrap();
        assert_eq!(done.map_stats(), before);
        assert_eq!(done.map().file_data_id(), world::MAP_FILE_DATA_ID);
        assert!(done.map().all_sections_admitted());
        assert_eq!(done.liquids()[0].source().id, world::MATERIAL_FILE_DATA_ID);
        assert_eq!(done.liquids()[1].source().id, world::TYPE_FILE_DATA_ID);
        assert_eq!(done.report().collector_before.reads, 4);
        assert_eq!(done.report().collector_after.reads, 7);
        assert_eq!(done.report().object_adts, 1);
        assert_eq!(done.report().own_tiles, 1);
        assert_eq!(done.report().placements.source_requests, 1);
        assert_eq!(done.report().ids.calls, 1);
        assert_eq!(done.report().model_reads, 1);
        assert_eq!(
            done.report().request_calls_after - done.report().request_calls_before,
            1
        );
        assert_eq!(tile_outputs(&out).len(), 1);
        assert_eq!(drops.get(), 1);
        assert!(done.resident_bytes() > 0);
        assert!(done.profile().contains("gnu15"));
        assert_eq!(done.extraction_mut().facts().models, 1);
        done.verify_unchanged().unwrap();
    }
}
#[test]
fn typed_missing_wdt_skips_map_without_globals_tiles_or_ids() {
    for build in [70374, 70425] {
        let f = CollisionFixture::for_build(&files(build, &[(1, 100, -1)]), build);
        let p = Private::new();
        let mut s = open(&f, build);
        let (sink, out, drops) = Sink::new();
        let c = start(&mut s, &f, &p, build, Profile::BoundedV1, true);
        let done = run_case(c, &f, &p, sink, Profile::BoundedV1, Limits::default()).unwrap();
        assert_eq!(
            (
                done.report().maps,
                done.report().wdt_files,
                done.report().missing_wdts
            ),
            (1, 0, 1)
        );
        assert_eq!(done.report().collector_after.reads, 5);
        assert_eq!(done.report().ids.calls, 0);
        assert!(out.borrow().is_empty());
        assert_eq!(drops.get(), 1);
    }
}
#[test]
fn missing_own_adt_replays_immediate_parent_without_read_or_id_on_replay() {
    for build in [70374, 70425] {
        let mut rows = files(build, &[(1, 100, -1), (2, 200, 1)]);
        rows.extend([
            (100, None, wdt(Some(101))),
            (101, None, m2_adt(90, 17)),
            (200, None, wdt(Some(201))),
            (90, None, fixture::m2(true)),
        ]);
        let f = CollisionFixture::for_build(&rows, build);
        let p = Private::new();
        let mut s = open(&f, build);
        let (sink, out, _) = Sink::new();
        let c = start(&mut s, &f, &p, build, Profile::BoundedV1, true);
        let done = run_case(c, &f, &p, sink, Profile::BoundedV1, Limits::default()).unwrap();
        let outputs = tile_outputs(&out);
        assert_eq!(
            (
                done.report().own_tiles,
                done.report().missing_object_adts,
                done.report().replayed_tiles
            ),
            (1, 1, 1)
        );
        assert_eq!(done.report().ids.calls, 1);
        assert_eq!(done.report().model_reads, 1);
        assert_eq!(outputs.len(), 2);
        assert_eq!(record_ids(&outputs[0].1), record_ids(&outputs[1].1));
        assert_eq!(outputs[1].0.original_map_id, 1);
        assert!(outputs[1].0.replayed);
        assert_eq!(done.report().placements.source_requests, 1);
    }
}
#[test]
fn successful_empty_own_tile_blocks_grandparent_fallback() {
    let mut rows = files(70374, &[(1, 100, -1), (2, 200, 1), (3, 300, 2)]);
    rows.extend([
        (100, None, wdt(Some(101))),
        (101, None, m2_adt(90, 17)),
        (200, None, wdt(Some(201))),
        (201, None, empty_adt()),
        (300, None, wdt(Some(301))),
        (90, None, fixture::m2(true)),
    ]);
    let f = CollisionFixture::for_build(&rows, 70374);
    let p = Private::new();
    let mut s = open(&f, 70374);
    let (sink, out, _) = Sink::new();
    let c = start(&mut s, &f, &p, 70374, Profile::BoundedV1, true);
    let done = run_case(c, &f, &p, sink, Profile::BoundedV1, Limits::default()).unwrap();
    let outputs = tile_outputs(&out);
    assert_eq!(
        (
            done.report().cached_tiles,
            done.report().own_tiles,
            done.report().empty_replays
        ),
        (2, 2, 1)
    );
    assert_eq!(outputs.len(), 3);
    assert!(outputs[1].1.is_empty());
    assert!(outputs[2].1.is_empty());
    assert_eq!(outputs[2].0.original_map_id, 2);
    assert_eq!(done.report().ids.calls, 1);
}
#[test]
fn inherited_tile_never_becomes_own_cache_for_grandchild() {
    let mut rows = files(70374, &[(1, 100, -1), (2, 200, 1), (3, 300, 2)]);
    rows.extend([
        (100, None, wdt(Some(101))),
        (101, None, m2_adt(90, 17)),
        (200, None, wdt(Some(201))),
        (300, None, wdt(Some(301))),
        (90, None, fixture::m2(true)),
    ]);
    let f = CollisionFixture::for_build(&rows, 70374);
    let p = Private::new();
    let mut s = open(&f, 70374);
    let (sink, out, _) = Sink::new();
    let c = start(&mut s, &f, &p, 70374, Profile::BoundedV1, true);
    let done = run_case(c, &f, &p, sink, Profile::BoundedV1, Limits::default()).unwrap();
    assert_eq!(
        (
            done.report().cached_tiles,
            done.report().missing_object_adts,
            done.report().replayed_tiles
        ),
        (1, 2, 1)
    );
    assert!(tile_outputs(&out).iter().all(|v| v.0.map_id != 3));
    assert_eq!(done.report().ids.calls, 1);
}
#[test]
fn missing_and_no_collision_models_make_present_empty_own_barriers() {
    for collision_input in [None, Some(fixture::m2(false))] {
        let mut rows = files(70425, &[(1, 100, -1), (2, 200, 1)]);
        rows.extend([
            (100, None, wdt(Some(101))),
            (101, None, m2_adt(90, 17)),
            (200, None, wdt(Some(201))),
        ]);
        if let Some(b) = collision_input {
            rows.push((90, None, b));
        }
        let f = CollisionFixture::for_build(&rows, 70425);
        let p = Private::new();
        let mut s = open(&f, 70425);
        let (sink, out, _) = Sink::new();
        let c = start(&mut s, &f, &p, 70425, Profile::BoundedV1, true);
        let done = run_case(c, &f, &p, sink, Profile::BoundedV1, Limits::default()).unwrap();
        assert_eq!(
            (
                done.report().cached_tiles,
                done.report().empty_replays,
                done.report().ids.calls
            ),
            (1, 1, 0)
        );
        assert!(tile_outputs(&out).iter().all(|v| v.1.is_empty()));
        assert_eq!(
            done.report().placements.omitted_requests
                + done.report().placements.no_collision_requests,
            1
        );
    }
}
#[test]
fn repeated_missing_model_uses_same_negative_cache_and_no_ids() {
    let mut rows = files(70374, &[(1, 100, -1)]);
    let mut b = m2_adt(99, 17);
    b.extend(m2_adt(99, 18));
    rows.extend([(100, None, wdt(Some(101))), (101, None, b)]);
    let f = CollisionFixture::for_build(&rows, 70374);
    let p = Private::new();
    let mut s = open(&f, 70374);
    let (sink, _, _) = Sink::new();
    let c = start(&mut s, &f, &p, 70374, Profile::BoundedV1, true);
    let mut done = run_case(c, &f, &p, sink, Profile::BoundedV1, Limits::default()).unwrap();
    assert_eq!(done.report().placements.omitted_requests, 2);
    assert_eq!(done.report().model_reads, 1);
    assert_eq!(done.report().ids.calls, 0);
    assert_eq!(done.extraction_mut().cached_identity_count(), 1);
    assert_eq!(done.extraction_mut().facts().models, 0);
}
#[test]
fn required_group_omits_parent_but_keeps_committed_orphan_raw() {
    for build in [70374, 70425] {
        let mut rows = files(build, &[(1, 100, -1)]);
        rows.extend([
            (100, None, wdt(Some(101))),
            (101, None, fixture::wmo_adt(90, 17)),
            (90, None, fixture::wmo_root(91, Some(92))),
            (92, None, fixture::m2(true)),
        ]);
        let f = CollisionFixture::for_build(&rows, build);
        let p = Private::new();
        let mut s = open(&f, build);
        let (sink, out, _) = Sink::new();
        let c = start(&mut s, &f, &p, build, Profile::BoundedV1, true);
        let mut done = run_case(c, &f, &p, sink, Profile::BoundedV1, Limits::default()).unwrap();
        assert_eq!(done.report().placements.omitted_requests, 1);
        assert_eq!(done.report().ids.calls, 0);
        assert!(tile_outputs(&out)[0].1.is_empty());
        assert_eq!(done.extraction_mut().facts().models, 1);
        let h = done.extraction_mut().model_handle(0).unwrap();
        assert!(matches!(
            done.extraction_mut().descriptor(h).unwrap().origin(),
            InputRef::FileDataId(92)
        ));
        done.extraction_mut()
            .with_payload(h, |v| {
                assert!(v.raw().is_some());
                Ok(())
            })
            .unwrap();
    }
}
#[test]
fn cached_startup_wmo_is_not_rebuilt_or_reacquired_during_world() {
    let mut rows = files(70425, &[(1, 100, -1)]);
    godi(&mut rows, 70425, &[(1, 90)]);
    rows.extend([
        (100, None, wdt(Some(101))),
        (101, None, fixture::wmo_adt(90, 17)),
        (90, None, fixture::wmo_root(91, Some(92))),
        (91, None, fixture::wmo_group(true, true, false)),
        (92, None, fixture::m2(true)),
    ]);
    let f = CollisionFixture::for_build(&rows, 70425);
    let p = Private::new();
    let mut s = open(&f, 70425);
    let (sink, _, _) = Sink::new();
    let mut c = start(&mut s, &f, &p, 70425, Profile::BoundedV1, true);
    let h = match c
        .collector_mut()
        .extract(ModelRequest::Wmo(InputRef::FileDataId(90)))
        .unwrap()
    {
        Outcome::Present(h) => h,
        _ => panic!("present startup"),
    };
    let reads = c.collector_mut().facts().unwrap().reads;
    let mut done = run_case(c, &f, &p, sink, Profile::BoundedV1, Limits::default()).unwrap();
    assert_eq!(done.report().model_reads, 0);
    assert_eq!(done.report().collector_after.reads, reads + 5);
    assert_eq!(done.extraction_mut().model_handle(1).unwrap(), h);
    assert_eq!(done.report().ids.calls, 2);
    assert!(done.profile().contains("gnu15"));
    done.extraction_mut()
        .with_payload(h, |v| {
            assert_eq!(v.profile(), payload::COMPATIBLE_GNU15_PROFILE);
            assert_eq!(v.wmo().unwrap().references().collect::<Vec<_>>(), [0]);
            Ok(())
        })
        .unwrap();
}
#[test]
fn malformed_wdt_is_fatal_not_a_missing_skip() {
    let mut rows = files(70374, &[(1, 100, -1)]);
    rows.push((100, None, b"invalid-wdt".to_vec()));
    let f = CollisionFixture::for_build(&rows, 70374);
    let p = Private::new();
    let mut s = open(&f, 70374);
    let (sink, out, drops) = Sink::new();
    let c = start(&mut s, &f, &p, 70374, Profile::BoundedV1, true);
    let e = run_case(c, &f, &p, sink, Profile::BoundedV1, Limits::default())
        .err()
        .unwrap();
    assert!(!matches!(
        e.cause(),
        Cause::Collector(CollectorError::Collision(crate::collision::Error::Store(
            local_store::Error::MissingFile
        )))
    ));
    assert!(out.borrow().is_empty());
    assert_eq!(drops.get(), 1);
}
#[test]
fn malformed_own_adt_never_uses_parent_fallback() {
    let mut rows = files(70425, &[(1, 100, -1), (2, 200, 1)]);
    rows.extend([
        (100, None, wdt(Some(101))),
        (101, None, m2_adt(90, 17)),
        (200, None, wdt(Some(201))),
        (201, None, b"invalid-adt".to_vec()),
        (90, None, fixture::m2(true)),
    ]);
    let f = CollisionFixture::for_build(&rows, 70425);
    let p = Private::new();
    let mut s = open(&f, 70425);
    let (sink, out, drops) = Sink::new();
    let c = start(&mut s, &f, &p, 70425, Profile::BoundedV1, true);
    let e = run_case(c, &f, &p, sink, Profile::BoundedV1, Limits::default())
        .err()
        .unwrap();
    let Cause::Run(inner) = e.cause() else {
        panic!("own ADT retains its traversal cause");
    };
    assert!(matches!(
        inner.cause(),
        run::Cause::Run(crate::collision::run::Error::Objects(_))
    ));
    assert!(tile_outputs(&out).iter().all(|v| !v.0.replayed));
    assert_eq!(drops.get(), 1);
}
#[test]
fn sink_drop_source_change_overrides_callback_error() {
    let mut rows = files(70374, &[(1, 100, -1)]);
    rows.push((100, None, wdt(None)));
    let f = CollisionFixture::for_build(&rows, 70374);
    let p = Private::new();
    let mut s = open(&f, 70374);
    let (mut sink, _, drops) = Sink::new();
    sink.fail = true;
    sink.mutate_drop = Some(f.path().join(".build.info"));
    let c = start(&mut s, &f, &p, 70374, Profile::BoundedV1, true);
    let e = run_case(c, &f, &p, sink, Profile::BoundedV1, Limits::default())
        .err()
        .unwrap();
    assert_eq!(e.code(), local_store::Error::InputChanged.code());
    assert_eq!(drops.get(), 1);
}
#[test]
fn sink_drop_payload_change_overrides_callback_error() {
    let mut rows = files(70374, &[(1, 100, -1)]);
    rows.push((100, None, wdt(None)));
    let f = CollisionFixture::for_build(&rows, 70374);
    let p = Private::new();
    let mut s = open(&f, 70374);
    let (mut sink, _, drops) = Sink::new();
    sink.fail = true;
    sink.chmod_drop = Some(p.0.join("payload"));
    let c = start(&mut s, &f, &p, 70374, Profile::BoundedV1, true);
    let e = run_case(c, &f, &p, sink, Profile::BoundedV1, Limits::default())
        .err()
        .unwrap();
    assert_eq!(e.code(), payload::Error::Changed.code());
    assert_eq!(drops.get(), 1);
}
#[test]
fn sink_mutating_inventory_is_rejected_after_owned_sink_drop() {
    let mut rows = files(70374, &[(1, 100, -1)]);
    rows.push((100, None, wdt(None)));
    let f = CollisionFixture::for_build(&rows, 70374);
    let p = Private::new();
    let mut s = open(&f, 70374);
    let (mut sink, _, drops) = Sink::new();
    sink.chmod_stage = Some(p.0.join("wdts"));
    let c = start(&mut s, &f, &p, 70374, Profile::BoundedV1, true);
    let e = run_case(c, &f, &p, sink, Profile::BoundedV1, Limits::default())
        .err()
        .unwrap();
    assert_eq!(e.code(), payload::Error::Changed.code());
    assert_eq!(drops.get(), 1);
}
#[test]
fn output_limit_rejects_new_model_before_any_placement_callback() {
    let mut rows = files(70425, &[(1, 100, -1)]);
    rows.extend([
        (100, None, wdt(Some(101))),
        (101, None, m2_adt(90, 17)),
        (90, None, fixture::m2(true)),
    ]);
    let f = CollisionFixture::for_build(&rows, 70425);
    let p = Private::new();
    let mut s = open(&f, 70425);
    let (sink, out, drops) = Sink::new();
    let c = start(&mut s, &f, &p, 70425, Profile::BoundedV1, true);
    let mut l = Limits::default();
    l.world.run.max_output_bytes = 1;
    run_case(c, &f, &p, sink, Profile::BoundedV1, l)
        .err()
        .unwrap();
    assert!(tile_outputs(&out).is_empty());
    assert_eq!(drops.get(), 1);
}
#[test]
fn lowered_combined_limit_fails_before_world_source_io_and_drops_sink() {
    let f = CollisionFixture::for_build(&files(70374, &[(1, 100, -1)]), 70374);
    let p = Private::new();
    let mut s = open(&f, 70374);
    let (sink, out, drops) = Sink::new();
    let c = start(&mut s, &f, &p, 70374, Profile::BoundedV1, true);
    let l = Limits {
        max_combined_memory_bytes: 0,
        ..Limits::default()
    };
    let e = run_case(c, &f, &p, sink, Profile::BoundedV1, l)
        .err()
        .unwrap();
    assert_eq!(e.code(), crate::collision::run::Error::Limit.code());
    assert!(out.borrow().is_empty());
    assert_eq!(drops.get(), 1);
}
#[test]
fn mixed_inventory_backend_is_rejected_without_traversal() {
    let f = CollisionFixture::for_build(&files(70425, &[]), 70425);
    let p = Private::new();
    let mut s = open(&f, 70425);
    let (sink, out, drops) = Sink::new();
    let c = start(&mut s, &f, &p, 70425, Profile::BoundedV1, true);
    let l = Limits::default();
    let (w, k) = p.inventories(&f, Profile::BoundedV1, BackendProfile::FileSetV1, l);
    let e = extract(c, w, k, sink, 65536, l).err().unwrap();
    assert_eq!(e.code(), crate::collision::run::Error::InvalidLimits.code());
    assert!(out.borrow().is_empty());
    assert_eq!(drops.get(), 1);
}
#[test]
fn explicit_disk_world_policy_is_retained_by_all_three_current_owners() {
    let f = CollisionFixture::for_build(&files(70425, &[]), 70425);
    let p = Private::new();
    let mut s = open(&f, 70425);
    let (sink, _, drops) = Sink::new();
    let c = start(&mut s, &f, &p, 70425, Profile::DiskWorldV1, true);
    let mut l = Limits::default();
    l.world.run.cache = Profile::DiskWorldV1.inventory_limits().cache;
    l.world.run.placements.max_models = Profile::DiskWorldV1.collector_limits().max_models;
    let mut done = run_case(c, &f, &p, sink, Profile::DiskWorldV1, l).unwrap();
    assert_eq!(done.extraction_mut().resources(), Profile::DiskWorldV1);
    assert_eq!(
        done.extraction_mut().backend(),
        BackendProfile::LeasedAppendV1
    );
    assert!(done.report().combined_reserved_bytes <= run::MAX_COMBINED_MEMORY_BYTES);
    assert_eq!(drops.get(), 1);
}

#[test]
fn borrowed_callback_cannot_be_relabelled_as_one_session_consuming_world() {
    let f = CollisionFixture::for_build(&files(70374, &[]), 70374);
    let p = Private::new();
    let mut s = open(&f, 70374);
    let (sink, out, drops) = Sink::new();
    let c = start(&mut s, &f, &p, 70374, Profile::BoundedV1, true);
    let mut prepared = super::super::prepare(c, sink, 65536, world::Limits::default()).unwrap();
    prepared.with_context(|_, _, _, _| Ok(())).unwrap();
    let l = Limits::default();
    let (w, k) = p.inventories(&f, Profile::BoundedV1, BackendProfile::LeasedAppendV1, l);
    let e = extract_prepared(prepared, w, k, l.max_combined_memory_bytes)
        .err()
        .unwrap();
    assert_eq!(e.code(), world::Error::State.code());
    assert!(out.borrow().is_empty());
    assert_eq!(drops.get(), 1);
}

fn two_children(group: u32) -> Vec<u8> {
    let mut header = [0; 64];
    put(&mut header, 4, 1);
    header[60] = 4;
    let mut b = chunk(b"MVER", &17u32.to_le_bytes());
    b.extend(chunk(b"MOHD", &header));
    b.extend(chunk(b"GFID", &group.to_le_bytes()));
    b.extend(chunk(
        b"MODI",
        &[92u32.to_le_bytes(), 93u32.to_le_bytes()].concat(),
    ));
    let mut spawn = [0; 80];
    for i in [0, 40] {
        put(&mut spawn, i, (i / 40) as u32);
        put(&mut spawn, i + 28, 1f32.to_bits());
        put(&mut spawn, i + 32, 1f32.to_bits());
    }
    b.extend(chunk(b"MODD", &spawn));
    let mut set = [0; 32];
    put(&mut set, 24, 2);
    b.extend(chunk(b"MODS", &set));
    b
}
fn two_references_without_parent_collision() -> Vec<u8> {
    let mut b = fixture::wmo_group(false, true, false);
    let n = b.len();
    put(&mut b, n - 6, 4);
    b.extend(1u16.to_le_bytes());
    let n = b.len();
    put(&mut b, 4, (n - 8) as u32);
    b
}
#[test]
fn gnu15_doodad_only_world_keeps_child_name_id_order_and_skips_parent_id() {
    for build in [70374, 70425] {
        let mut rows = files(build, &[(1, 100, -1)]);
        rows.extend([
            (100, None, wdt(Some(101))),
            (101, None, fixture::wmo_adt(90, 17)),
            (90, None, two_children(91)),
            (91, None, two_references_without_parent_collision()),
            (92, None, fixture::m2(true)),
            (93, None, fixture::m2(true)),
        ]);
        let f = CollisionFixture::for_build(&rows, build);
        let p = Private::new();
        let mut s = open(&f, build);
        let (sink, out, _) = Sink::new();
        let c = start(&mut s, &f, &p, build, Profile::BoundedV1, true);
        let mut done = run_case(c, &f, &p, sink, Profile::BoundedV1, Limits::default()).unwrap();
        let rows = tile_outputs(&out);
        let records = DirBin::parse(&rows[0].1, placement::Limits::default())
            .unwrap()
            .records()
            .collect::<Vec<_>>();
        assert_eq!(records.len(), 2);
        assert_eq!(done.report().ids.calls, 2);
        assert_eq!(done.report().ids.m2_counter_consumed, 2);
        assert!(done.report().placements_profile.contains("gnu15"));
        let h93 = done.extraction_mut().model_handle(1).unwrap();
        let h92 = done.extraction_mut().model_handle(0).unwrap();
        assert_eq!(
            records[0].name(),
            done.extraction_mut()
                .descriptor(h93)
                .unwrap()
                .output_basename()
        );
        assert_eq!(
            records[1].name(),
            done.extraction_mut()
                .descriptor(h92)
                .unwrap()
                .output_basename()
        );
        let parent = done.extraction_mut().model_handle(2).unwrap();
        assert_eq!(
            done.extraction_mut().descriptor(parent).unwrap().status(),
            crate::collision::Status::DoodadOnly
        );
        done.extraction_mut()
            .with_payload(parent, |v| {
                assert!(v.raw().is_none());
                assert_eq!(v.wmo().unwrap().references().collect::<Vec<_>>(), [1, 0]);
                Ok(())
            })
            .unwrap();
    }
}
#[test]
fn callback_used_rejection_still_drops_sink_before_last_source_guard() {
    let f = CollisionFixture::for_build(&files(70425, &[(1, 100, -1)]), 70425);
    let p = Private::new();
    let mut s = open(&f, 70425);
    let (mut sink, out, drops) = Sink::new();
    sink.mutate_drop = Some(f.path().join(".build.info"));
    let c = start(&mut s, &f, &p, 70425, Profile::BoundedV1, true);
    let mut prepared = super::super::prepare(c, sink, 65536, world::Limits::default()).unwrap();
    prepared
        .with_context(|c, _, _, _| {
            assert_eq!(c.collector_mut().facts().unwrap().reads, 4);
            Ok(())
        })
        .unwrap();
    let l = Limits::default();
    let (w, k) = p.inventories(&f, Profile::BoundedV1, BackendProfile::LeasedAppendV1, l);
    let e = extract_prepared(prepared, w, k, l.max_combined_memory_bytes)
        .err()
        .unwrap();
    assert_eq!(e.code(), local_store::Error::InputChanged.code());
    assert!(out.borrow().is_empty());
    assert_eq!(drops.get(), 1);
}
#[test]
fn per_file_limit_rejects_present_wdt_instead_of_skipping() {
    let mut rows = files(70374, &[(1, 100, -1)]);
    rows.push((100, None, wdt(None)));
    let f = CollisionFixture::for_build(&rows, 70374);
    let p = Private::new();
    let mut s = open(&f, 70374);
    let (sink, out, drops) = Sink::new();
    let c = start(&mut s, &f, &p, 70374, Profile::BoundedV1, true);
    let mut l = Limits::default();
    l.world.run.max_file_bytes = 1;
    run_case(c, &f, &p, sink, Profile::BoundedV1, l)
        .err()
        .unwrap();
    assert!(out.borrow().is_empty());
    assert_eq!(drops.get(), 1);
}
#[test]
fn mixed_resource_profiles_are_rejected_before_traversal() {
    let f = CollisionFixture::for_build(&files(70425, &[]), 70425);
    let p = Private::new();
    let mut s = open(&f, 70425);
    let (sink, out, drops) = Sink::new();
    let c = start(&mut s, &f, &p, 70425, Profile::DiskWorldV1, true);
    let l = Limits::default();
    let (w, k) = p.inventories(&f, Profile::BoundedV1, BackendProfile::LeasedAppendV1, l);
    let e = extract(c, w, k, sink, 65536, l).err().unwrap();
    assert_eq!(e.code(), crate::collision::run::Error::InvalidLimits.code());
    assert!(out.borrow().is_empty());
    assert_eq!(drops.get(), 1);
}

#[test]
fn prepare_rejection_preserves_source_changed_over_simultaneous_inventory_changed() {
    for build in [70374, 70425] {
        let mut rows = files(build, &[(1, 100, -1)]);
        rows.push((100, None, wdt(Some(101))));
        rows.push((101, None, m2_adt(90, 17)));
        rows.push((90, None, fixture::m2(true)));
        let f = CollisionFixture::for_build(&rows, build);
        let p = Private::new();
        let mut s = open(&f, build);
        let (mut sink, out, drops) = Sink::new();
        sink.mutate_drop = Some(f.path().join(".build.info"));
        sink.chmod_drop = Some(p.0.join("wdts"));
        let c = start(&mut s, &f, &p, build, Profile::BoundedV1, true);
        let mut l = Limits::default();
        l.world.max_table_memory_bytes = 0;
        let e = run_case(c, &f, &p, sink, Profile::BoundedV1, l)
            .err()
            .unwrap();
        let source_changed = CollectorError::Collision(crate::collision::Error::Store(
            local_store::Error::InputChanged,
        ));
        assert_eq!(e.code(), local_store::Error::InputChanged.code());
        assert_eq!(e.guard(), Some(source_changed));
        assert_eq!(
            e.inventory_guards()[0],
            Some(inventory::Error::Spool(payload::Error::Changed))
        );
        let Cause::Startup(inner) = e.cause() else {
            panic!("preparation cause must survive outer closure");
        };
        assert_eq!(inner.guard(), Some(source_changed));
        // Typed preparation Limit remains the cause: the Err arm never invokes
        // traversal/WDT/ADT acquisition. This is flow evidence, not an IO trace.
        assert!(matches!(
            inner.cause(),
            available::spooled::Cause::Available(available::Error::Limit)
        ));
        assert!(out.borrow().is_empty());
        assert_eq!(drops.get(), 1);
    }
}

fn run_case_with_coverage<'s>(
    c: startup::Startup<'s>,
    f: &CollisionFixture,
    p: &Private,
    sink: Sink,
    profile: Profile,
    l: Limits,
) -> Result<Completed, Error<u8>> {
    let (w, k) = p.inventories(f, profile, BackendProfile::LeasedAppendV1, l);
    extract_selected_with_coverage(c, w, k, sink, 65536, plan::MapSelection::All, l)
}
// Authored 2026-10-17: real coverage belongs to the same successful traversal.
#[test]
fn coverage_same_run_both_builds_distinguishes_missing_empty_output_and_replay() {
    use run::coverage::{Own, Replay, Wdt};
    for build in [70374, 70425] {
        let mut rows = files(
            build,
            &[(1, 100, -1), (2, 200, 1), (3, 300, 2), (4, 400, -1)],
        );
        rows.extend([
            (100, None, wdt(Some(101))),
            (101, None, m2_adt(90, 17)),
            (200, None, wdt(Some(201))),
            (201, None, empty_adt()),
            (300, None, wdt(Some(301))),
            (90, None, fixture::m2(true)),
        ]);
        let f = CollisionFixture::for_build(&rows, build);
        let p = Private::new();
        let mut store = open(&f, build);
        let (sink, _, drops) = Sink::new();
        let c = start(&mut store, &f, &p, build, Profile::BoundedV1, true);
        let mut done =
            run_case_with_coverage(c, &f, &p, sink, Profile::BoundedV1, Limits::default()).unwrap();
        let before = *done.report();
        done.with_coverage(0, |v| {
            assert_eq!(v.report(), &before);
            assert_eq!(v.maps().len(), 4);
            assert_eq!(v.map_table().file_data_id(), world::MAP_FILE_DATA_ID);
            assert_eq!(
                v.startup().table().file_data_id(),
                gameobjects::TABLE_FILE_DATA_ID
            );
            assert_eq!(v.liquids()[0].source().id, world::MATERIAL_FILE_DATA_ID);
            let m = &v.maps()[3];
            assert_eq!(m.id(), 4);
            assert_eq!(m.wdt_file_data_id(), 400);
            assert_eq!(m.wdt(), Wdt::Missing);
            assert_eq!(m.tile(0, 0), None);
            // Fixture WDT advertises x=0,y=0. Parent OWN empty is a barrier:
            // grandchild's missing own request replays ONLY that empty parent.
            assert_eq!(v.maps()[0].tile(0, 0).unwrap().own(), Own::Output);
            assert_eq!(v.maps()[1].tile(0, 0).unwrap().own(), Own::Empty);
            let t = v.maps()[2].tile(0, 0).unwrap();
            assert_eq!(t.own(), Own::Missing);
            assert_eq!(t.replay(), Replay::Empty);
            assert_eq!(t.replay_parent(), Some(2));
            Ok::<_, u8>(())
        })
        .unwrap();
        assert_eq!(*done.report(), before);
        assert_eq!(drops.get(), 1);
        done.verify_unchanged().unwrap();
    }
}
#[test]
fn coverage_repeated_view_has_no_new_reads_ids_or_retained_growth() {
    let build = 70425;
    let mut rows = files(build, &[(1, 100, -1)]);
    rows.push((100, None, wdt(None)));
    let f = CollisionFixture::for_build(&rows, build);
    let p = Private::new();
    let mut store = open(&f, build);
    let (sink, _, _) = Sink::new();
    let c = start(&mut store, &f, &p, build, Profile::BoundedV1, true);
    let mut done =
        run_case_with_coverage(c, &f, &p, sink, Profile::BoundedV1, Limits::default()).unwrap();
    let before = *done.report();
    let resident = done.resident_bytes();
    for _ in 0..3 {
        done.with_coverage(0, |v| {
            assert_eq!(v.maps()[0].global_records(), Some(0));
            Ok::<_, u8>(())
        })
        .unwrap();
    }
    assert_eq!(*done.report(), before);
    assert_eq!(done.resident_bytes(), resident);
}
#[test]
fn coverage_refused_heap_drops_capture_before_last_payload_guard() {
    struct Change(PathBuf, Rc<Cell<usize>>);
    impl Drop for Change {
        fn drop(&mut self) {
            self.1.set(self.1.get() + 1);
            fs::set_permissions(&self.0, fs::Permissions::from_mode(0o755)).unwrap();
        }
    }
    let build = 70425;
    let mut rows = files(build, &[(1, 100, -1)]);
    rows.push((100, None, wdt(None)));
    let f = CollisionFixture::for_build(&rows, build);
    let p = Private::new();
    let mut store = open(&f, build);
    let (sink, _, _) = Sink::new();
    let c = start(&mut store, &f, &p, build, Profile::BoundedV1, true);
    let mut done =
        run_case_with_coverage(c, &f, &p, sink, Profile::BoundedV1, Limits::default()).unwrap();
    let count = Rc::new(Cell::new(0));
    let invoked = Rc::new(Cell::new(0));
    let called = invoked.clone();
    let action = Change(p.0.join("payload"), count.clone());
    let error = done
        .with_coverage(usize::MAX, move |_| {
            called.set(called.get() + 1);
            drop(action);
            Ok::<_, u8>(())
        })
        .unwrap_err();
    assert_eq!(invoked.get(), 0);
    assert_eq!(count.get(), 1);
    assert_eq!(error.code(), payload::Error::Changed.code());
}
#[test]
fn original_world_selector_retains_no_coverage_allocation_or_profile_change() {
    let build = 70425;
    let mut rows = files(build, &[(1, 100, -1)]);
    rows.push((100, None, wdt(None)));
    let f = CollisionFixture::for_build(&rows, build);
    let p = Private::new();
    let mut store = open(&f, build);
    let (sink, _, _) = Sink::new();
    let c = start(&mut store, &f, &p, build, Profile::BoundedV1, true);
    let mut done = run_case(c, &f, &p, sink, Profile::BoundedV1, Limits::default()).unwrap();
    assert!(!done.has_coverage());
    assert_eq!(done.profile(), PROFILE_GNU15_70425);
    assert!(done.coverage.is_none());
    let invoked = Cell::new(0);
    let e = done
        .with_coverage(0, |_| {
            invoked.set(1);
            Ok::<_, u8>(())
        })
        .unwrap_err();
    assert_eq!(invoked.get(), 0);
    assert_eq!(e.code(), crate::collision::Error::State.code());
    done.verify_unchanged().unwrap();
}
#[test]
fn coverage_actual_capacity_remains_charged_through_completed_owner() {
    for build in [70374, 70425] {
        let mut rows = files(build, &[(1, 100, -1)]);
        rows.push((100, None, wdt(None)));
        let f = CollisionFixture::for_build(&rows, build);
        let p = Private::new();
        let mut store = open(&f, build);
        let (sink, _, _) = Sink::new();
        let c = start(&mut store, &f, &p, build, Profile::BoundedV1, true);
        let done =
            run_case_with_coverage(c, &f, &p, sink, Profile::BoundedV1, Limits::default()).unwrap();
        let coverage = done.coverage.as_ref().unwrap().resident_bytes().unwrap();
        assert!(coverage >= 4096);
        let startup = done.startup.data_view().unwrap().stats().resident_bytes;
        let model = done.startup.model_view().unwrap();
        assert_eq!(
            model.extraction.retained_external_for_test(),
            model.state.resident + startup + done.topology().resident_bytes() + coverage
        );
        assert!(done.has_coverage());
        assert!(done.profile().contains("coverage"));
    }
}
