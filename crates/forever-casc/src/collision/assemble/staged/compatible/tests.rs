// SPDX-License-Identifier: GPL-3.0-or-later
// Modified by the RustyCore Extractors project, 2026-10-17.
// Authored 2026-10-17: synthetic genuine compatible Stage controls; UNRUN.
use super::*;
use crate::collision::{
    InputRef, gameobjects,
    resources::Profile,
    spool::{BackendProfile, ProtectedInputs, Spool},
    world::{self as strict_world, tests as fixture},
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
use std::{
    cell::Cell,
    fs,
    os::unix::fs::{DirBuilderExt, PermissionsExt},
    path::PathBuf,
    rc::Rc,
    sync::atomic::{AtomicU64, Ordering},
};
fn put(b: &mut [u8], at: usize, n: u32) {
    b[at..at + 4].copy_from_slice(&n.to_le_bytes());
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
        strict_world::MAP_FILE_DATA_ID => "Map",
        strict_world::MATERIAL_FILE_DATA_ID => "LiquidMaterial",
        strict_world::TYPE_FILE_DATA_ID => "LiquidType",
        _ => unreachable!(),
    };
    let c = match build {
        70374 => catalog::WOWDBDEFS_70374_CATALOG_ID,
        70425 => catalog::WOWDBDEFS_70425_CATALOG_ID,
        _ => unreachable!(),
    };
    let m = catalog::resolve_for(c, name).unwrap().unwrap().metadata();
    assert_eq!(
        u32::from_le_bytes(b[156..160].try_into().unwrap()),
        m.layout_hash
    );
    put(b, 152, m.table_hash.unwrap());
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
            "rce-compatible-authentic-stage-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::DirBuilder::new().mode(0o700).create(&p).unwrap();
        Self(p)
    }
    fn spool(&self, f: &CollisionFixture, name: &str) -> Spool {
        let profile = Profile::BoundedV1;
        Spool::create_with_backend(
            &self.0.join(name),
            ProtectedInputs::capture(f.path(), &[]).unwrap(),
            profile.spool_limits(),
            spool::Profile::SameMountV1,
            profile,
            BackendProfile::LeasedAppendV1,
        )
        .unwrap()
    }
    fn inventories(&self, f: &CollisionFixture, l: Limits) -> (Inventory, Inventory) {
        let one = |name| {
            Inventory::new(
                self.spool(f, name),
                inventory::Limits {
                    max_heap_bytes: 256 * 1024 * 1024,
                    wdt: l.world.world.run.wdt,
                    cache: l.world.world.run.cache,
                    ..Profile::BoundedV1.inventory_limits()
                },
            )
            .unwrap()
        };
        (one("wdts"), one("cache"))
    }
    fn stage(&self, f: &CollisionFixture, l: Limits) -> Stage {
        Stage::new(self.spool(f, "placements"), l.stage()).unwrap()
    }
}
impl Drop for Private {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
fn limits() -> Limits {
    let mut l = Limits::default();
    l.world.world.run.placements.ids.max_memory_bytes = 1024 * 1024;
    l
}
fn start<'s>(
    s: &'s mut LocalStore,
    f: &CollisionFixture,
    p: &Private,
    build: u32,
    native: bool,
) -> startup::Startup<'s> {
    let spool = p.spool(f, "models");
    let l = Profile::BoundedV1.startup_limits();
    match (build, native) {
        (70374, false) => startup::extract_startup_fdid_70374(s, l, spool),
        (70425, false) => startup::extract_startup_fdid_70425(s, l, spool),
        (70374, true) => startup::extract_startup_fdid_native_gnu15_70374(s, l, spool),
        (70425, true) => startup::extract_startup_fdid_native_gnu15_70425(s, l, spool),
        _ => unreachable!(),
    }
    .unwrap()
}
fn acquire(
    f: &CollisionFixture,
    p: &Private,
    build: u32,
    native: bool,
    l: Limits,
    selection: plan::MapSelection,
) -> Result<Staged, Error> {
    let mut s = open(f, build);
    let startup = start(&mut s, f, p, build, native);
    let (wdts, cache) = p.inventories(f, l);
    extract_selected(startup, wdts, cache, p.spool(f, "placements"), selection, l)
}
fn one_map(build: u32) -> CollisionFixture {
    let mut rows = files(build, &[(1, 100, -1)]);
    rows.push((100, None, fixture::wdt(None, None)));
    CollisionFixture::for_build(&rows, build)
}
fn global(map: u32) -> run::Context {
    run::Context {
        map_id: map,
        original_map_id: map,
        kind: run::Kind::Global,
        replayed: false,
    }
}
fn tile(map: u32, parent: Option<u32>, x: u8, y: u8) -> run::Context {
    run::Context {
        map_id: map,
        original_map_id: parent.unwrap_or(map),
        kind: run::Kind::Tile { x, y },
        replayed: parent.is_some(),
    }
}
fn row(flags: u8, id: u32, name: &[u8]) -> Vec<u8> {
    let mut b = vec![flags, 7];
    b.extend(id.to_le_bytes());
    for x in [1f32, 2., 3., 0., 0., 0., 1.] {
        b.extend(x.to_bits().to_le_bytes());
    }
    if flags & 1 != 0 {
        for x in [0f32, 0., 0., 1., 1., 1.] {
            b.extend(x.to_bits().to_le_bytes());
        }
    }
    b.extend((name.len() as u32).to_le_bytes());
    b.extend(name);
    b
}
fn ids(s: &mut Staged, map: u32) -> Vec<u32> {
    s.with_map(map, 4096, |v| {
        Ok::<_, u8>(v.records().map(|r| r.id()).collect::<Vec<_>>())
    })
    .unwrap()
}
struct OnDrop {
    path: PathBuf,
    drops: Rc<Cell<usize>>,
    chmod: bool,
}
impl Drop for OnDrop {
    fn drop(&mut self) {
        self.drops.set(self.drops.get() + 1);
        if self.chmod {
            fs::set_permissions(&self.path, fs::Permissions::from_mode(0o755)).unwrap();
        } else {
            fs::write(&self.path, b"synthetic-stage-captured-drop").unwrap();
        }
    }
}

fn change_protected_root(path: &std::path::Path) {
    let mode = fs::metadata(path).unwrap().permissions().mode() & 0o7777;
    // Toggle a non-owner permission bit: guaranteed different independent of
    // test-process umask, while retaining owner traversal for final guards.
    fs::set_permissions(path, fs::Permissions::from_mode(mode ^ 0o010)).unwrap();
}
struct RootDrop {
    path: PathBuf,
    drops: Rc<Cell<usize>>,
}
impl Drop for RootDrop {
    fn drop(&mut self) {
        self.drops.set(self.drops.get() + 1);
        change_protected_root(&self.path);
    }
}
// Test-only restoration is after the last asserted guard, never an admission
// rebaseline. Node::check compares the protected directory mode/dev/ino/uid/gid.
struct RestoreRoot {
    path: PathBuf,
    mode: u32,
}
impl RestoreRoot {
    fn new(path: &std::path::Path) -> Self {
        Self {
            path: path.to_path_buf(),
            mode: fs::metadata(path).unwrap().permissions().mode() & 0o7777,
        }
    }
}
impl Drop for RestoreRoot {
    fn drop(&mut self) {
        fs::set_permissions(&self.path, fs::Permissions::from_mode(self.mode)).unwrap();
    }
}

#[test]
fn authentic_stage_both_builds_and_both_order_profiles_keep_same_world_owner() {
    for build in [70374, 70425] {
        for native in [false, true] {
            let mut rows = files(build, &[(1, 100, -1)]);
            rows.extend([
                (100, None, fixture::wdt(None, Some(101))),
                (101, None, fixture::m2_adt(90, 17)),
                (90, None, fixture::m2(true)),
            ]);
            let f = CollisionFixture::for_build(&rows, build);
            let p = Private::new();
            let mut s = acquire(&f, &p, build, native, limits(), plan::MapSelection::All).unwrap();
            assert_eq!(s.profile(), PROFILE);
            assert_eq!(s.world().profile().contains("gnu15"), native);
            assert_eq!(s.maps().unwrap().collect::<Vec<_>>(), [1]);
            assert_eq!(
                (
                    s.stats().unwrap().maps,
                    s.stats().unwrap().callbacks,
                    s.stats().unwrap().records
                ),
                (1, 2, 1)
            );
            assert_eq!(s.world().report().ids.calls, 1);
            assert_eq!(s.world().report().model_reads, 1);
            assert_eq!(s.world().report().collector_before.reads, 4);
            assert_eq!(s.world().report().collector_after.reads, 7);
            assert_eq!(
                s.world().report().request_calls_after - s.world().report().request_calls_before,
                1
            );
            assert_eq!(s.world.extraction_mut().facts().models, 1);
            assert_eq!(ids(&mut s, 1).len(), 1);
            assert_eq!(
                s.world().map().file_data_id(),
                strict_world::MAP_FILE_DATA_ID
            );
            assert!(s.world().map().all_sections_admitted());
            assert_eq!(
                s.world().liquids()[0].source().id,
                strict_world::MATERIAL_FILE_DATA_ID
            );
            s.verify_unchanged().unwrap();
        }
    }
}
#[test]
fn absent_wdt_is_not_a_fabricated_empty_map_stage() {
    for build in [70374, 70425] {
        let f = CollisionFixture::for_build(&files(build, &[(1, 100, -1)]), build);
        let p = Private::new();
        let s = acquire(&f, &p, build, true, limits(), plan::MapSelection::All).unwrap();
        assert_eq!(
            (
                s.world().report().maps,
                s.world().report().missing_wdts,
                s.stats().unwrap().maps
            ),
            (1, 1, 0)
        );
        assert_eq!(s.maps().unwrap().count(), 0);
        assert_eq!(s.world().topology().selected_active().count(), 1);
        assert_eq!(s.stats().unwrap().callbacks, 0);
        assert_eq!(s.world().report().ids.calls, 0);
    }
}
#[test]
fn present_empty_wdt_seals_global_callback_and_complete_empty_map() {
    let f = one_map(70425);
    let p = Private::new();
    let mut s = acquire(&f, &p, 70425, true, limits(), plan::MapSelection::All).unwrap();
    assert_eq!(s.maps().unwrap().collect::<Vec<_>>(), [1]);
    assert_eq!(s.stats().unwrap().callbacks, 1);
    assert_eq!(
        s.with_map(1, 0, |v| Ok::<_, u8>(v.bytes().len())).unwrap(),
        0
    );
}
#[test]
fn missing_own_tile_replays_immediate_parent_bytes_without_new_id() {
    let mut rows = files(70425, &[(1, 100, -1), (2, 200, 1)]);
    rows.extend([
        (100, None, fixture::wdt(None, Some(101))),
        (101, None, fixture::m2_adt(90, 17)),
        (200, None, fixture::wdt(None, Some(201))),
        (90, None, fixture::m2(true)),
    ]);
    let f = CollisionFixture::for_build(&rows, 70425);
    let p = Private::new();
    let mut s = acquire(&f, &p, 70425, true, limits(), plan::MapSelection::All).unwrap();
    assert_eq!(s.world().report().replayed_tiles, 1);
    assert_eq!(s.world().report().ids.calls, 1);
    assert_eq!(ids(&mut s, 1), ids(&mut s, 2));
    let e = s
        .stage
        .entries
        .iter()
        .find(|e| e.context.map_id == 2 && matches!(e.context.kind, run::Kind::Tile { .. }))
        .unwrap();
    assert!(e.context.replayed);
    assert_eq!(e.context.original_map_id, 1);
    s.with_map(2, 0, |v| {
        assert!(v.records().all(|r| r.flags() & 2 != 0));
        Ok::<_, u8>(())
    })
    .unwrap();
}
#[test]
fn successful_empty_own_tile_blocks_grandparent_placements() {
    let mut rows = files(70374, &[(1, 100, -1), (2, 200, 1), (3, 300, 2)]);
    rows.extend([
        (100, None, fixture::wdt(None, Some(101))),
        (101, None, fixture::m2_adt(90, 17)),
        (200, None, fixture::wdt(None, Some(201))),
        (201, None, b"REVM\x04\0\0\0\x12\0\0\0".to_vec()),
        (300, None, fixture::wdt(None, Some(301))),
        (90, None, fixture::m2(true)),
    ]);
    let f = CollisionFixture::for_build(&rows, 70374);
    let p = Private::new();
    let mut s = acquire(&f, &p, 70374, true, limits(), plan::MapSelection::All).unwrap();
    assert_eq!(s.world().report().empty_replays, 1);
    assert_eq!(s.world().report().cached_tiles, 2);
    assert!(!ids(&mut s, 1).is_empty());
    assert!(ids(&mut s, 2).is_empty());
    assert!(ids(&mut s, 3).is_empty());
}
#[test]
fn inherited_tile_does_not_become_own_cache_for_grandchild() {
    let mut rows = files(70374, &[(1, 100, -1), (2, 200, 1), (3, 300, 2)]);
    rows.extend([
        (100, None, fixture::wdt(None, Some(101))),
        (101, None, fixture::m2_adt(90, 17)),
        (200, None, fixture::wdt(None, Some(201))),
        (300, None, fixture::wdt(None, Some(301))),
        (90, None, fixture::m2(true)),
    ]);
    let f = CollisionFixture::for_build(&rows, 70374);
    let p = Private::new();
    let mut s = acquire(&f, &p, 70374, true, limits(), plan::MapSelection::All).unwrap();
    assert_eq!(s.world().report().cached_tiles, 1);
    assert_eq!(s.world().report().replayed_tiles, 1);
    assert!(ids(&mut s, 3).is_empty());
    assert_eq!(s.world().report().ids.calls, 1);
}
#[test]
fn selected_map_keeps_ancestors_and_excludes_unrelated_stage_domain() {
    let mut rows = files(70425, &[(1, 100, -1), (2, 200, 1), (3, 300, -1)]);
    rows.extend([
        (100, None, fixture::wdt(None, None)),
        (200, None, fixture::wdt(None, None)),
        (300, None, fixture::wdt(None, None)),
    ]);
    let f = CollisionFixture::for_build(&rows, 70425);
    let p = Private::new();
    let s = acquire(
        &f,
        &p,
        70425,
        true,
        limits(),
        plan::MapSelection::MapAndAncestors(2),
    )
    .unwrap();
    assert_eq!(s.maps().unwrap().collect::<Vec<_>>(), [1, 2]);
    assert_eq!(
        s.world().topology().selection(),
        plan::MapSelection::MapAndAncestors(2)
    );
}
#[test]
fn missing_required_group_preserves_orphan_raw_but_no_parent_placement_or_id() {
    let mut rows = files(70425, &[(1, 100, -1)]);
    rows.extend([
        (100, None, fixture::wdt(None, Some(101))),
        (101, None, fixture::wmo_adt(90, 17)),
        (90, None, fixture::wmo_root(91, Some(92))),
        (92, None, fixture::m2(true)),
    ]);
    let f = CollisionFixture::for_build(&rows, 70425);
    let p = Private::new();
    let mut s = acquire(&f, &p, 70425, true, limits(), plan::MapSelection::All).unwrap();
    assert_eq!(s.world().report().placements.omitted_requests, 1);
    assert_eq!(s.world().report().ids.calls, 0);
    assert!(ids(&mut s, 1).is_empty());
    assert_eq!(s.world.extraction_mut().facts().models, 1);
    let h = s.world.extraction_mut().model_handle(0).unwrap();
    assert!(matches!(
        s.world.extraction_mut().descriptor(h).unwrap().origin(),
        InputRef::FileDataId(92)
    ));
    s.world
        .extraction_mut()
        .with_payload(h, |v| {
            assert!(v.raw().is_some());
            Ok(())
        })
        .unwrap();
}
#[test]
fn duplicate_record_ids_and_empty_callbacks_are_preserved_by_shared_stage_kernel() {
    let f = one_map(70374);
    let p = Private::new();
    let mut s = p.stage(&f, limits());
    let a = row(0, 9, b"one");
    let b = row(0, 9, b"two");
    s.append(global(2), &[]).unwrap();
    s.append(global(1), &a).unwrap();
    s.append(tile(1, None, 0, 0), &b).unwrap();
    let mut sealed = false;
    seal(
        &mut Capture {
            stage: &mut s,
            sealed: &mut sealed,
        },
        &mut [2, 1],
    )
    .unwrap();
    assert!(sealed);
    assert_eq!(s.read_map(1, 0).unwrap(), [a, b].concat());
    assert_eq!(s.stats().unwrap().records, 2);
    assert_eq!(s.stats().unwrap().callbacks, 3);
}
#[test]
fn seal_uses_existing_active_capacity_without_touching_callback_sequences() {
    let f = one_map(70374);
    let p = Private::new();
    let mut s = p.stage(&f, limits());
    s.append(global(8), &[]).unwrap();
    s.append(global(1), &[]).unwrap();
    let sequences = s.entries.iter().map(|e| e.sequence).collect::<Vec<_>>();
    let mut active = vec![8, 1];
    let capacity = active.capacity();
    let ptr = active.as_ptr();
    let heap = s.heap().unwrap();
    let mut sealed = false;
    seal(
        &mut Capture {
            stage: &mut s,
            sealed: &mut sealed,
        },
        &mut active,
    )
    .unwrap();
    assert_eq!(active, [1, 8]);
    assert_eq!(active.capacity(), capacity);
    assert_eq!(active.as_ptr(), ptr);
    assert_eq!(s.heap().unwrap(), heap);
    assert_eq!(
        s.entries.iter().map(|e| e.sequence).collect::<Vec<_>>(),
        sequences
    );
}
#[test]
fn seal_rejects_duplicate_missing_and_extra_visited_domain_without_partial_success() {
    for mut domain in [vec![1, 1], vec![], vec![1, 2]] {
        let f = one_map(70374);
        let p = Private::new();
        let mut s = p.stage(&f, limits());
        s.append(global(1), &[]).unwrap();
        let mut sealed = false;
        assert_eq!(
            seal(
                &mut Capture {
                    stage: &mut s,
                    sealed: &mut sealed
                },
                &mut domain
            ),
            Err(StageError::Binding)
        );
        assert!(!sealed);
        assert!(s.check().is_err());
    }
}
#[test]
fn seal_is_single_use_even_for_empty_world() {
    let f = one_map(70374);
    let p = Private::new();
    let mut s = p.stage(&f, limits());
    let mut sealed = false;
    seal(
        &mut Capture {
            stage: &mut s,
            sealed: &mut sealed,
        },
        &mut [],
    )
    .unwrap();
    assert_eq!(
        seal(
            &mut Capture {
                stage: &mut s,
                sealed: &mut sealed
            },
            &mut []
        ),
        Err(StageError::State)
    );
}
#[test]
fn stage_callback_capacity_and_record_caps_fail_closed() {
    for records in [false, true] {
        let f = one_map(70374);
        let p = Private::new();
        let mut l = limits();
        if records {
            l.max_records = 0;
        } else {
            l.max_callbacks = 0;
        }
        let mut s = p.stage(&f, l);
        assert_eq!(
            s.append(global(1), &row(0, 1, b"x")),
            Err(StageError::Limit)
        );
        assert!(s.check().is_err());
    }
}
#[test]
fn combined_budget_underflow_rejects_before_map_and_liquid_acquisition() {
    let f = one_map(70425);
    let p = Private::new();
    let mut l = limits();
    l.world.max_combined_memory_bytes = l.max_heap_bytes + 512 * 1024 * 1024 - 1;
    assert_eq!(
        acquire(&f, &p, 70425, true, l, plan::MapSelection::All)
            .err()
            .unwrap()
            .code(),
        StageError::Limit.code()
    );
    assert!(!p.0.join("placements/00000000.payload").exists());
}
#[test]
fn stage_heap_cap_rejects_even_when_world_would_be_empty() {
    let f = one_map(70374);
    let p = Private::new();
    let mut l = limits();
    l.max_heap_bytes = 1;
    assert_eq!(
        acquire(&f, &p, 70374, true, l, plan::MapSelection::All)
            .err()
            .unwrap()
            .code(),
        StageError::Limit.code()
    );
}
#[test]
fn stage_cap_increase_is_rejected_without_world_callbacks() {
    let f = one_map(70374);
    let p = Private::new();
    let mut l = limits();
    l.max_heap_bytes = super::super::MAX_HEAP_BYTES + 1;
    assert_eq!(
        acquire(&f, &p, 70374, true, l, plan::MapSelection::All)
            .err()
            .unwrap()
            .code(),
        StageError::InvalidLimits.code()
    );
}
#[test]
fn nonfresh_stage_repository_rejects_without_exposing_its_record_as_placements() {
    let f = one_map(70374);
    let p = Private::new();
    let mut repository = p.spool(&f, "placements");
    repository.write(1, |out| out.bytes(b"x")).unwrap();
    assert!(matches!(
        Stage::new(repository, limits().stage()),
        Err(StageError::State)
    ));
}
#[test]
fn missing_map_access_poison_is_distinct_from_sealed_empty_map() {
    let f = one_map(70374);
    let p = Private::new();
    let mut s = acquire(&f, &p, 70374, true, limits(), plan::MapSelection::All).unwrap();
    assert_eq!(
        s.with_map(9, 0, |_| Ok::<_, u8>(())).unwrap_err().code(),
        StageError::Missing.code()
    );
    assert!(s.maps().is_err());
}
#[test]
fn callback_failure_poison_prevents_later_map_access() {
    let f = one_map(70425);
    let p = Private::new();
    let mut s = acquire(&f, &p, 70425, true, limits(), plan::MapSelection::All).unwrap();
    let e = s.with_map(1, 0, |_| Err::<(), u8>(73)).unwrap_err();
    assert_eq!(e.cause, AccessCause::Callback(73));
    assert!(s.with_map(1, 0, |_| Ok::<_, u8>(())).is_err());
}
#[test]
fn rejected_reload_drops_uninvoked_capture_before_final_stage_guard() {
    let f = one_map(70374);
    let p = Private::new();
    let mut s = acquire(&f, &p, 70374, true, limits(), plan::MapSelection::All).unwrap();
    let drops = Rc::new(Cell::new(0));
    let action = OnDrop {
        path: p.0.join("placements/records.archive"),
        drops: drops.clone(),
        chmod: true,
    };
    let invoked = Rc::new(Cell::new(false));
    let seen = invoked.clone();
    let e = s
        .with_map(1, usize::MAX, move |_| {
            seen.set(true);
            drop(action);
            Ok::<_, u8>(())
        })
        .unwrap_err();
    assert!(!invoked.get());
    assert_eq!(drops.get(), 1);
    assert_eq!(
        e.guards.stage,
        Some(StageError::Spool(payload::Error::Changed))
    );
    assert_eq!(e.code(), payload::Error::Changed.code());
}
#[test]
fn uninvoked_capture_protected_root_change_is_observed_over_poisoned_stage() {
    let f = one_map(70425);
    let p = Private::new();
    let mut s = acquire(&f, &p, 70425, true, limits(), plan::MapSelection::All).unwrap();
    let _restore = RestoreRoot::new(f.path());
    s.stage.failed = Some(StageError::Limit);
    let drops = Rc::new(Cell::new(0));
    let action = RootDrop {
        path: f.path().to_path_buf(),
        drops: drops.clone(),
    };
    let e = s
        .with_map(1, 0, move |_| {
            drop(action);
            Ok::<_, u8>(())
        })
        .unwrap_err();
    assert_eq!(drops.get(), 1);
    assert_eq!(e.code(), payload::Error::Changed.code());
}
#[test]
fn successful_callback_capture_destruction_is_guarded_before_value_return() {
    let f = one_map(70374);
    let p = Private::new();
    let mut s = acquire(&f, &p, 70374, true, limits(), plan::MapSelection::All).unwrap();
    let drops = Rc::new(Cell::new(0));
    let action = OnDrop {
        path: p.0.join("placements/records.archive"),
        drops: drops.clone(),
        chmod: true,
    };
    let e = s
        .with_map(1, 0, move |_| {
            drop(action);
            Ok::<_, u8>(())
        })
        .unwrap_err();
    assert_eq!(drops.get(), 1);
    assert_eq!(e.code(), payload::Error::Changed.code());
}
#[test]
fn returned_value_drop_rechecks_protected_root_after_collector_guard_failure() {
    let f = one_map(70425);
    let p = Private::new();
    let mut s = acquire(&f, &p, 70425, true, limits(), plan::MapSelection::All).unwrap();
    let _restore = RestoreRoot::new(f.path());
    let drops = Rc::new(Cell::new(0));
    let action = RootDrop {
        path: f.path().to_path_buf(),
        drops: drops.clone(),
    };
    let models = p.0.join("models");
    let e = match s.with_map(1, 0, move |_| {
        fs::set_permissions(models, fs::Permissions::from_mode(0o755)).unwrap();
        Ok::<_, u8>(action)
    }) {
        Err(e) => e,
        Ok(_) => panic!("changed collector admitted"),
    };
    assert_eq!(drops.get(), 1);
    assert_eq!(e.code(), payload::Error::Changed.code());
    assert!(e.guards.collector.is_some());
}
#[test]
fn protected_root_change_is_observed_with_simultaneous_stage_change_on_verify() {
    let f = one_map(70374);
    let p = Private::new();
    let mut s = acquire(&f, &p, 70374, true, limits(), plan::MapSelection::All).unwrap();
    let _restore = RestoreRoot::new(f.path());
    change_protected_root(f.path());
    fs::set_permissions(p.0.join("placements"), fs::Permissions::from_mode(0o755)).unwrap();
    let e = s.verify_unchanged().unwrap_err();
    assert!(e.guards.collector.is_some());
    assert_eq!(
        e.guards.stage,
        Some(StageError::Spool(payload::Error::Changed))
    );
    assert_eq!(e.code(), payload::Error::Changed.code());
}

// Literal second nonzero-TACT section: only the first complete section is known.
// Reused public synthetic framing, not a SourceFacts constructor/oracle.
fn unknown_map(mut b: Vec<u8>) -> Vec<u8> {
    let word = |b: &[u8], at| u32::from_le_bytes(b[at..at + 4].try_into().unwrap()) as usize;
    let start = word(&b, 212);
    let rows = word(&b, 136);
    let size = word(&b, 144);
    for i in 0..rows {
        for field in [0, 4] {
            let at = start + i * size + field;
            let n = word(&b, at);
            put(&mut b, at, (n + size) as u32);
        }
    }
    let mut out = b[..244].to_vec();
    out.extend([0; 40]);
    out.extend(&b[244..start]);
    out.extend(0u32.to_le_bytes());
    out.extend(&b[start..]);
    put(&mut out, 136, (rows + 1) as u32);
    put(&mut out, 200, 2);
    put(&mut out, 212, (start + 44) as u32);
    let next = out.len() as u32;
    out[244..252].copy_from_slice(&1u64.to_le_bytes());
    put(&mut out, 252, next);
    put(&mut out, 256, 1);
    put(&mut out, 268, 4);
    put(&mut out, 280, 1);
    out.extend(vec![0xa5; size]);
    out.extend(99u32.to_le_bytes());
    out.extend([0xa5; 8]);
    out
}
#[test]
fn partial_map_coverage_stays_available_and_never_creates_unknown_stage_maps() {
    for build in [70374, 70425] {
        let mut rows = files(build, &[(1, 100, -1)]);
        let map = rows
            .iter_mut()
            .find(|v| v.0 == strict_world::MAP_FILE_DATA_ID)
            .unwrap();
        map.2 = unknown_map(std::mem::take(&mut map.2));
        rows.push((100, None, fixture::wdt(None, None)));
        let f = CollisionFixture::for_build(&rows, build);
        let p = Private::new();
        let s = acquire(&f, &p, build, true, limits(), plan::MapSelection::All).unwrap();
        assert!(!s.world().map().all_sections_admitted());
        assert_eq!(s.world().map().declared_direct_rows, 2);
        assert_eq!(s.world().map().direct_rows, 1);
        assert_eq!(s.world().topology().all().len(), 1);
        assert_eq!(s.maps().unwrap().collect::<Vec<_>>(), [1]);
        assert_eq!(s.world().map().copy_records, 0);
        assert_eq!(s.world().liquids().len(), 2);
    }
}
#[test]
fn startup_model_handle_cache_and_cumulative_facts_survive_authentic_staging() {
    let build = 70425;
    let mut rows = files(build, &[(1, 100, -1)]);
    let godi = rows
        .iter_mut()
        .find(|v| v.0 == gameobjects::TABLE_FILE_DATA_ID)
        .unwrap();
    godi.2 = fixture::gameobject_db2(&[(1, 90)], &[]);
    stamp(build, gameobjects::TABLE_FILE_DATA_ID, &mut godi.2);
    rows.extend([
        (100, None, fixture::wdt(None, Some(101))),
        (101, None, fixture::wmo_adt(90, 17)),
        (90, None, fixture::wmo_root(91, Some(92))),
        (91, None, fixture::wmo_group(true, true, false)),
        (92, None, fixture::m2(true)),
    ]);
    let f = CollisionFixture::for_build(&rows, build);
    let p = Private::new();
    let mut store = open(&f, build);
    let mut startup = start(&mut store, &f, &p, build, true);
    let before = startup.collector_mut().facts().unwrap();
    let handle = match startup
        .collector_mut()
        .extract(crate::collision::ModelRequest::Wmo(InputRef::FileDataId(
            90,
        )))
        .unwrap()
    {
        crate::collision::compatible::Outcome::Present(h) => h,
        _ => panic!("complete startup WMO omitted"),
    };
    let l = limits();
    let (w, k) = p.inventories(&f, l);
    let mut s = extract(startup, w, k, p.spool(&f, "placements"), l).unwrap();
    assert_eq!(s.world().report().model_reads, 0);
    assert_eq!(s.world().report().collector_before.reads, before.reads + 3);
    assert_eq!(s.world().report().collector_before.models, before.models);
    assert_eq!(s.world().report().ids.calls, 2);
    assert_eq!(s.world.extraction_mut().model_handle(1).unwrap(), handle);
    assert_eq!(ids(&mut s, 1).len(), 2);
    s.verify_unchanged().unwrap();
}

#[test]
fn already_observed_typed_input_changed_keeps_priority_over_outer_stage_change() {
    let e = Error {
        cause: Cause::Collector(CollectorError::Collision(crate::collision::Error::Store(
            local_store::Error::InputChanged,
        ))),
        guards: Guards {
            stage: Some(StageError::Spool(payload::Error::Changed)),
            ..Default::default()
        },
    };
    assert_eq!(e.code(), local_store::Error::InputChanged.code());
}
