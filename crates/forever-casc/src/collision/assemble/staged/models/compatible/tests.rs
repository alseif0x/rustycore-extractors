// SPDX-License-Identifier: GPL-3.0-or-later
// Modified by the RustyCore Extractors project, 2026-10-17.
// Authored 2026-10-17: genuine compatible model-phase synthetic controls; UNRUN.
use super::super::Phase;
use super::*;
use crate::collision::{
    compatible::startup,
    gameobjects,
    inventory_spool::{self as inventory, Inventory},
    plan,
    resources::Profile,
    spool::{self, BackendProfile, ProtectedInputs, Spool},
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
fn chunk(tag: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut b: Vec<_> = tag.iter().rev().copied().collect();
    b.extend((data.len() as u32).to_le_bytes());
    b.extend(data);
    b
}
fn stamp(build: u32, id: u32, b: &mut [u8]) {
    let table = match id {
        gameobjects::TABLE_FILE_DATA_ID => "GameObjectDisplayInfo",
        strict_world::MAP_FILE_DATA_ID => "Map",
        strict_world::MATERIAL_FILE_DATA_ID => "LiquidMaterial",
        strict_world::TYPE_FILE_DATA_ID => "LiquidType",
        _ => unreachable!(),
    };
    let catalog = match build {
        70374 => catalog::WOWDBDEFS_70374_CATALOG_ID,
        70425 => catalog::WOWDBDEFS_70425_CATALOG_ID,
        _ => unreachable!(),
    };
    let m = catalog::resolve_for(catalog, table)
        .unwrap()
        .unwrap()
        .metadata();
    assert_eq!(
        u32::from_le_bytes(b[156..160].try_into().unwrap()),
        m.layout_hash
    );
    put(b, 152, m.table_hash.unwrap());
}
fn files(build: u32, maps: &[(u32, u32, i16)]) -> Vec<CollisionFile<'static>> {
    let mut rows = fixture::files(maps);
    for (id, _, b) in &mut rows {
        stamp(build, *id, b);
    }
    rows
}
fn godi(rows: &mut [CollisionFile<'static>], build: u32, values: &[(u32, u32)]) {
    let g = rows
        .iter_mut()
        .find(|r| r.0 == gameobjects::TABLE_FILE_DATA_ID)
        .unwrap();
    g.2 = fixture::gameobject_db2(values, &[]);
    stamp(build, g.0, &mut g.2);
}
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Private(PathBuf);
impl Private {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "rce-compatible-model-phase-{}-{}",
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
}
impl Drop for Private {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
fn stage(f: &CollisionFixture, p: &Private, build: u32, native: bool) -> placement::Staged {
    let o = local_store::OpenOptions {
        locale: Locale::EnUs,
        index_layouts: [local_index::Layout::Contiguous; 16],
        root_layout: root::Layout::Header50893V2,
        limits: local_store::Limits::default(),
    };
    let mut store = match build {
        70374 => LocalStore::open_db2_70374(f.path(), o),
        70425 => LocalStore::open_db2_70425(f.path(), o),
        _ => unreachable!(),
    }
    .unwrap();
    let l = Profile::BoundedV1.startup_limits();
    let raw = p.spool(f, "models");
    let start = match (build, native) {
        (70374, false) => startup::extract_startup_fdid_70374(&mut store, l, raw),
        (70425, false) => startup::extract_startup_fdid_70425(&mut store, l, raw),
        (70374, true) => startup::extract_startup_fdid_native_gnu15_70374(&mut store, l, raw),
        (70425, true) => startup::extract_startup_fdid_native_gnu15_70425(&mut store, l, raw),
        _ => unreachable!(),
    }
    .unwrap();
    let mut l = placement::Limits::default();
    l.world.world.run.placements.ids.max_memory_bytes = 1024 * 1024;
    let one = |name| {
        Inventory::new(
            p.spool(f, name),
            inventory::Limits {
                max_heap_bytes: 256 * 1024 * 1024,
                wdt: l.world.world.run.wdt,
                cache: l.world.world.run.cache,
                ..Profile::BoundedV1.inventory_limits()
            },
        )
        .unwrap()
    };
    placement::extract_selected(
        start,
        one("wdts"),
        one("cache"),
        p.spool(f, "placements"),
        plan::MapSelection::All,
        l,
    )
    .unwrap()
}
fn one_m2(build: u32) -> CollisionFixture {
    let mut rows = files(build, &[(1, 100, -1)]);
    rows.extend([
        (100, None, fixture::wdt(None, Some(101))),
        (101, None, fixture::m2_adt(90, 17)),
        (90, None, fixture::m2(true)),
    ]);
    CollisionFixture::for_build(&rows, build)
}
fn ready(f: &CollisionFixture, p: &Private, build: u32, native: bool) -> Prepared {
    prepare(
        stage(f, p, build, native),
        p.spool(f, "vmos"),
        Limits::default(),
    )
    .unwrap()
}
fn name(m: &Prepared) -> Vec<u8> {
    m.models().unwrap().next().unwrap().0.to_vec()
}
struct OnDrop {
    path: PathBuf,
    count: Rc<Cell<usize>>,
    mode: bool,
}
impl Drop for OnDrop {
    fn drop(&mut self) {
        self.count.set(self.count.get() + 1);
        if self.mode {
            fs::set_permissions(&self.path, fs::Permissions::from_mode(0o755)).unwrap();
        } else {
            fs::set_permissions(&self.path, fs::Permissions::from_mode(0o600)).unwrap();
            fs::write(&self.path, b"synthetic-model-phase-change").unwrap();
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
    count: Rc<Cell<usize>>,
}
impl Drop for RootDrop {
    fn drop(&mut self) {
        self.count.set(self.count.get() + 1);
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
fn genuine_models_both_builds_and_both_order_profiles_reload_vmo() {
    for build in [70374, 70425] {
        for native in [false, true] {
            let f = one_m2(build);
            let p = Private::new();
            let mut m = ready(&f, &p, build, native);
            assert_eq!(m.profile(), PROFILE);
            assert_eq!(m.stage().world().profile().contains("gnu15"), native);
            assert_eq!(m.models().unwrap().count(), 1);
            assert_eq!(m.stats().unwrap().selected_models, 1);
            m.with_model(&name(&m), 0, |v| {
                assert_eq!(v.flags(), 1);
                Ok::<_, u8>(())
            })
            .unwrap();
            m.verify_unchanged().unwrap();
        }
    }
}
#[test]
fn absent_wdt_stays_absent_in_model_domain_not_forged_empty_map() {
    let build = 70425;
    let f = CollisionFixture::for_build(&files(build, &[(1, 100, -1)]), build);
    let p = Private::new();
    let m = ready(&f, &p, build, true);
    assert_eq!(m.stage().world().report().missing_wdts, 1);
    assert_eq!(m.stage().maps().unwrap().count(), 0);
    assert_eq!(m.stats().unwrap().maps_scanned, 0);
    assert_eq!(m.models().unwrap().count(), 0);
}
#[test]
fn present_empty_wdt_is_scanned_once_without_vmo_records() {
    let build = 70374;
    let mut rows = files(build, &[(1, 100, -1)]);
    rows.push((100, None, fixture::wdt(None, None)));
    let f = CollisionFixture::for_build(&rows, build);
    let p = Private::new();
    let m = ready(&f, &p, build, true);
    assert_eq!(m.stats().unwrap().maps_scanned, 1);
    assert_eq!(m.state.repository.stats().unwrap().records, 0);
}
#[test]
fn orphan_child_raw_is_retained_but_not_selected_as_vmo() {
    for build in [70374, 70425] {
        let mut rows = files(build, &[(1, 100, -1)]);
        rows.extend([
            (100, None, fixture::wdt(None, Some(101))),
            (101, None, fixture::wmo_adt(90, 17)),
            (90, None, fixture::wmo_root(91, Some(92))),
            (92, None, fixture::m2(true)),
        ]);
        let f = CollisionFixture::for_build(&rows, build);
        let p = Private::new();
        let m = ready(&f, &p, build, true);
        assert_eq!(m.stage().world().report().placements.omitted_requests, 1);
        assert_eq!(m.stage().world().report().ids.calls, 0);
        assert_eq!(m.state.index.len(), 1);
        assert_eq!(m.models().unwrap().count(), 0);
        assert_eq!(m.state.repository.stats().unwrap().records, 0);
    }
}
#[test]
fn doodad_only_parent_keeps_child_vmo_without_parent_raw_or_id() {
    for build in [70374, 70425] {
        let mut rows = files(build, &[(1, 100, -1)]);
        rows.extend([
            (100, None, fixture::wdt(None, Some(101))),
            (101, None, fixture::wmo_adt(90, 17)),
            (90, None, fixture::wmo_root(91, Some(92))),
            (91, None, fixture::wmo_group(false, true, false)),
            (92, None, fixture::m2(true)),
        ]);
        let f = CollisionFixture::for_build(&rows, build);
        let p = Private::new();
        let m = ready(&f, &p, build, true);
        assert_eq!(m.state.index.len(), 2);
        assert_eq!(m.stats().unwrap().selected_models, 1);
        assert!(m.models().unwrap().all(|(_, class)| class == Class::M2));
    }
}
#[test]
fn n05_empty_complete_wmo_does_not_prevent_following_m2_vmo() {
    for build in [70374, 70425] {
        let mut rows = files(build, &[(1, 100, -1)]);
        let mut adt = fixture::wmo_adt(90, 17);
        adt.extend(fixture::m2_adt(93, 18));
        rows.extend([
            (100, None, fixture::wdt(None, Some(101))),
            (101, None, adt),
            (90, None, fixture::wmo_root(91, None)),
            (91, None, fixture::wmo_group(false, false, false)),
            (93, None, fixture::m2(true)),
        ]);
        let f = CollisionFixture::for_build(&rows, build);
        let p = Private::new();
        let m = ready(&f, &p, build, true);
        assert_eq!(m.stage().world().report().ids.calls, 1);
        assert_eq!(m.stats().unwrap().selected_models, 1);
        assert_eq!(m.state.index.len(), 1);
        assert!(m.models().unwrap().all(|(_, c)| c == Class::M2));
    }
}
#[test]
fn duplicate_placement_ids_select_all_names_before_id_deduplication() {
    let build = 70425;
    let mut rows = files(build, &[(1, 100, -1)]);
    let bodies = [
        fixture::m2_adt(90, 17)[8..].to_vec(),
        fixture::m2_adt(92, 17)[8..].to_vec(),
    ]
    .concat();
    rows.extend([
        (100, None, fixture::wdt(None, Some(101))),
        (101, None, chunk(b"MDDF", &bodies)),
        (90, None, fixture::m2(true)),
        (92, None, fixture::m2(true)),
    ]);
    let f = CollisionFixture::for_build(&rows, build);
    let p = Private::new();
    let m = ready(&f, &p, build, true);
    assert_eq!(m.stats().unwrap().selected_models, 2);
    assert_eq!(m.stats().unwrap().placement_records, 2);
}
#[test]
fn successful_startup_temp_rows_join_selection_without_world_placement() {
    let build = 70374;
    let mut rows = files(build, &[]);
    godi(&mut rows, build, &[(1, 90), (2, 90), (3, 0), (4, 91)]);
    rows.extend([
        (90, None, fixture::m2(true)),
        (91, None, fixture::m2(false)),
    ]);
    let f = CollisionFixture::for_build(&rows, build);
    let p = Private::new();
    let m = ready(&f, &p, build, true);
    assert_eq!(m.stats().unwrap().selected_models, 1);
    assert_eq!(m.stats().unwrap().list_rows, 2);
    assert_eq!(m.stats().unwrap().placement_records, 0);
}
#[test]
fn inherited_records_preserve_parent_cache_and_selected_model_union() {
    let build = 70425;
    let mut rows = files(build, &[(1, 100, -1), (2, 200, 1)]);
    rows.extend([
        (100, None, fixture::wdt(None, Some(101))),
        (101, None, fixture::m2_adt(90, 17)),
        (200, None, fixture::wdt(None, Some(201))),
        (90, None, fixture::m2(true)),
    ]);
    let f = CollisionFixture::for_build(&rows, build);
    let p = Private::new();
    let m = ready(&f, &p, build, true);
    assert_eq!(m.stage().world().report().replayed_tiles, 1);
    assert_eq!(m.stage().world().report().ids.calls, 1);
    assert_eq!(m.stats().unwrap().placement_records, 2);
    assert_eq!(m.stats().unwrap().selected_models, 1);
}
#[test]
fn model_reservation_retains_full_stage_and_completed_external_owners() {
    let build = 70374;
    let f = one_m2(build);
    let p = Private::new();
    let m = ready(&f, &p, build, true);
    let resident = m.state.stage.resident_bytes().unwrap();
    assert!(
        m.stats().unwrap().reserved_bytes
            >= resident + m.state.stage.archive().limits.max_heap_bytes
    );
    assert!(m.stats().unwrap().peak_admitted_bytes <= m.state.limits.max_memory_bytes);
}
#[test]
fn combined_model_budget_underflow_rejects_whole_preparation() {
    let build = 70374;
    let f = one_m2(build);
    let p = Private::new();
    let s = stage(&f, &p, build, true);
    let l = Limits {
        max_memory_bytes: 1,
        ..Limits::default()
    };
    assert_eq!(
        prepare(s, p.spool(&f, "vmos"), l).err().unwrap().code(),
        Kind::Limit.code()
    );
}
#[test]
fn aggregate_geometry_rejects_before_first_vmo_write() {
    for triangles in [false, true] {
        let build = 70425;
        let f = one_m2(build);
        let p = Private::new();
        let s = stage(&f, &p, build, true);
        let mut l = Limits::default();
        if triangles {
            l.max_triangles = 0;
        } else {
            l.max_vertices = 0;
        }
        assert_eq!(
            prepare(s, p.spool(&f, "vmos"), l).err().unwrap().code(),
            Kind::Limit.code()
        );
        assert!(!p.0.join("vmos/00000000.payload").exists());
    }
}
#[test]
fn fresh_archive_rejects_preexisting_record_without_relabeling_it_vmo() {
    let build = 70425;
    let f = one_m2(build);
    let p = Private::new();
    let s = stage(&f, &p, build, true);
    let mut archive = p.spool(&f, "vmos");
    archive.write(1, |out| out.bytes(b"x")).unwrap();
    assert_eq!(
        prepare(s, archive, Limits::default()).err().unwrap().code(),
        Kind::State.code()
    );
}
#[test]
fn upward_profile_caps_reject_even_for_empty_selection() {
    let build = 70374;
    let f = CollisionFixture::for_build(&files(build, &[]), build);
    let p = Private::new();
    let s = stage(&f, &p, build, true);
    let l = Limits {
        max_models: super::super::super::super::MAX_MODELS + 1,
        ..Limits::default()
    };
    assert_eq!(
        prepare(s, p.spool(&f, "vmos"), l).err().unwrap().code(),
        Kind::InvalidLimits.code()
    );
}
#[test]
fn unknown_model_drops_uninvoked_capture_then_guards_archive() {
    let build = 70425;
    let f = one_m2(build);
    let p = Private::new();
    let mut m = ready(&f, &p, build, true);
    let count = Rc::new(Cell::new(0));
    let action = OnDrop {
        path: p.0.join("vmos/records.archive"),
        count: count.clone(),
        mode: true,
    };
    let called = Rc::new(Cell::new(false));
    let saw = called.clone();
    let e = m
        .with_model(b"unknown", 0, move |_| {
            saw.set(true);
            drop(action);
            Ok::<_, u8>(())
        })
        .unwrap_err();
    assert!(!called.get());
    assert_eq!(count.get(), 1);
    assert_eq!(e.guards.archive, Some(payload::Error::Changed));
}
#[test]
fn callback_heap_refusal_drops_capture_before_protected_root_and_archive_guards() {
    let build = 70374;
    let f = one_m2(build);
    let p = Private::new();
    let mut m = ready(&f, &p, build, true);
    let n = name(&m);
    let _restore = RestoreRoot::new(f.path());
    let count = Rc::new(Cell::new(0));
    let action = RootDrop {
        path: f.path().to_path_buf(),
        count: count.clone(),
    };
    let e = m
        .with_model(&n, usize::MAX, move |_| {
            drop(action);
            Ok::<_, u8>(())
        })
        .unwrap_err();
    assert_eq!(count.get(), 1);
    assert_eq!(e.code(), payload::Error::Changed.code());
}
#[test]
fn typed_callback_failure_survives_mutating_archive_capture_drop() {
    let build = 70425;
    let f = one_m2(build);
    let p = Private::new();
    let mut m = ready(&f, &p, build, true);
    let n = name(&m);
    let count = Rc::new(Cell::new(0));
    let action = OnDrop {
        path: p.0.join("vmos/records.archive"),
        count: count.clone(),
        mode: true,
    };
    let e = m
        .with_model(&n, 0, move |_| {
            drop(action);
            Err::<(), u8>(73)
        })
        .unwrap_err();
    assert_eq!(e.cause, Cause::Callback(73));
    assert_eq!(e.guards.archive, Some(payload::Error::Changed));
    assert_eq!(count.get(), 1);
}
#[test]
fn returned_value_drop_rechecks_protected_root_after_outer_archive_change() {
    let build = 70425;
    let f = one_m2(build);
    let p = Private::new();
    let mut m = ready(&f, &p, build, true);
    let n = name(&m);
    let _restore = RestoreRoot::new(f.path());
    let count = Rc::new(Cell::new(0));
    let value = RootDrop {
        path: f.path().to_path_buf(),
        count: count.clone(),
    };
    let archive = p.0.join("vmos");
    let e = match m.with_model(&n, 0, move |_| {
        fs::set_permissions(archive, fs::Permissions::from_mode(0o755)).unwrap();
        Ok::<_, u8>(value)
    }) {
        Err(e) => e,
        Ok(_) => panic!("changed VMO archive admitted"),
    };
    assert_eq!(count.get(), 1);
    assert_eq!(e.code(), payload::Error::Changed.code());
}
#[test]
fn protected_root_change_is_observed_with_placement_and_vmo_change() {
    let build = 70374;
    let f = one_m2(build);
    let p = Private::new();
    let mut m = ready(&f, &p, build, true);
    let _restore = RestoreRoot::new(f.path());
    change_protected_root(f.path());
    for part in ["placements", "vmos"] {
        fs::set_permissions(p.0.join(part), fs::Permissions::from_mode(0o755)).unwrap();
    }
    assert_eq!(
        m.verify_unchanged().unwrap_err().code(),
        payload::Error::Changed.code()
    );
}
#[test]
fn poisoned_model_phase_never_returns_later_vmo_prefix() {
    let build = 70425;
    let f = one_m2(build);
    let p = Private::new();
    let mut m = ready(&f, &p, build, true);
    let n = name(&m);
    assert_eq!(
        m.with_model(&n, 0, |_| Err::<(), u8>(29))
            .unwrap_err()
            .cause,
        Cause::Callback(29)
    );
    assert!(m.models().is_err());
    assert!(m.with_model(&n, 0, |_| Ok::<_, u8>(())).is_err());
}
#[test]
fn compatible_raw_phase_refusal_drops_capture_before_final_protected_root_guard() {
    let build = 70374;
    let f = one_m2(build);
    let p = Private::new();
    let mut m = ready(&f, &p, build, true);
    let _restore = RestoreRoot::new(f.path());
    let h = m.state.index.iter().find(|e| e.selected).unwrap().handle;
    let count = Rc::new(Cell::new(0));
    let action = RootDrop {
        path: f.path().to_path_buf(),
        count: count.clone(),
    };
    let e = m.state.stage.with_raw_phase(h, 0, 0, move |_| {
        drop(action);
        Ok::<_, u8>(())
    });
    assert!(matches!(e, Err(payload::CallbackFailure::Collector(_))));
    assert_eq!(count.get(), 1);
    assert_eq!(
        m.verify_unchanged().unwrap_err().code(),
        payload::Error::Changed.code()
    );
}
#[test]
fn compatible_raw_phase_accepts_genuine_magic2_metadata_and_raw() {
    for build in [70374, 70425] {
        let f = one_m2(build);
        let p = Private::new();
        let mut m = ready(&f, &p, build, true);
        let h = m.state.index.iter().find(|e| e.selected).unwrap().handle;
        let shape = m.state.stage.shape(h).unwrap();
        assert!(matches!(
            m.state
                .stage
                .with_raw_phase(h, shape.record_bytes, 0, |raw| {
                    assert!(raw.is_some());
                    Ok::<_, u8>(())
                }),
            Ok(())
        ));
    }
}
#[test]
fn callback_source_change_is_not_misclassified_when_guard_is_spool_only() {
    let e = Error::<u8> {
        cause: Cause::Collector(CollectorError::Payload(payload::Error::Changed)),
        guards: Guards::default(),
    };
    assert_eq!(e.code(), payload::Error::Changed.code());
    let e = Error::<u8> {
        cause: Cause::Phase(placement::Cause::Collector(CollectorError::Collision(
            crate::collision::Error::Store(local_store::Error::InputChanged),
        ))),
        guards: Guards {
            archive: Some(payload::Error::Changed),
            ..Default::default()
        },
    };
    assert_eq!(e.code(), local_store::Error::InputChanged.code());
}
#[test]
fn compatible_error_and_closed_work_view_have_bounded_inline_storage() {
    assert!(std::hint::black_box(std::mem::size_of::<Error<u8>>()) <= super::super::GUARD_BYTES);
    assert!(
        std::hint::black_box(std::mem::size_of::<
            super::super::ModelWork<'_, placement::Staged>,
        >()) <= super::super::GUARD_BYTES
    );
}

// Isolated RAW repository guards: only the synthetic models spool changes.
// Both setup refusals drop the owned capture before all final guards. UNRUN.
#[test]
fn raw_spool_change_after_binding_refusal_preserves_cause_and_wins_code() {
    for build in [70374, 70425] {
        let f = one_m2(build);
        let p = Private::new();
        let mut m = ready(&f, &p, build, true);
        let count = Rc::new(Cell::new(0));
        let action = OnDrop {
            path: p.0.join("models"),
            count: count.clone(),
            mode: true,
        };
        let e = m
            .with_model(b"absent", 0, move |_| {
                let _ = &action;
                Ok::<_, u8>(())
            })
            .unwrap_err();
        assert_eq!(count.get(), 1);
        assert_eq!(e.cause, Cause::Kind(Kind::Binding));
        assert_eq!(
            e.guards.collector,
            Some(CollectorError::Payload(payload::Error::Changed))
        );
        assert_eq!(e.guards.placements, None);
        assert_eq!(e.guards.archive, None);
        assert_eq!(e.code(), payload::Error::Changed.code());
    }
}
#[test]
fn raw_spool_change_after_budget_refusal_preserves_cause_and_wins_code() {
    for build in [70374, 70425] {
        let f = one_m2(build);
        let p = Private::new();
        let mut m = ready(&f, &p, build, true);
        let selected = name(&m);
        let count = Rc::new(Cell::new(0));
        let action = OnDrop {
            path: p.0.join("models"),
            count: count.clone(),
            mode: true,
        };
        let e = m
            .with_model(&selected, usize::MAX, move |_| {
                let _ = &action;
                Ok::<_, u8>(())
            })
            .unwrap_err();
        assert_eq!(count.get(), 1);
        assert_eq!(e.cause, Cause::Kind(Kind::Limit));
        assert_eq!(
            e.guards.collector,
            Some(CollectorError::Payload(payload::Error::Changed))
        );
        assert_eq!(e.guards.placements, None);
        assert_eq!(e.guards.archive, None);
        assert_eq!(e.code(), payload::Error::Changed.code());
    }
}
