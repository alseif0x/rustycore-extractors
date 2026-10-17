// SPDX-License-Identifier: GPL-3.0-or-later
// Modified by the RustyCore Extractors project, 2026-10-17.
// Authored 2026-10-17: genuine compatible final package controls. All UNRUN.
use super::super::Owner as _;
use super::*;
use crate::collision::{
    assemble::staged::compatible as placement,
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
            "rce-compatible-package-phase-{}-{}",
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
    stage_inner(f, p, build, native, false)
}
fn stage_inner(
    f: &CollisionFixture,
    p: &Private,
    build: u32,
    native: bool,
    capture: bool,
) -> placement::Staged {
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
    let extract = if capture {
        placement::extract_selected_with_coverage
    } else {
        placement::extract_selected
    };
    extract(
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
fn prepared(f: &CollisionFixture, p: &Private, build: u32, native: bool) -> models::Prepared {
    models::prepare(
        stage(f, p, build, native),
        p.spool(f, "vmos"),
        models::Limits::default(),
    )
    .unwrap()
}
fn ready(f: &CollisionFixture, p: &Private, build: u32, native: bool) -> Completed {
    assemble(
        prepared(f, p, build, native),
        p.spool(f, "maps"),
        Limits::default(),
    )
    .unwrap()
}
fn name(c: &Completed) -> Vec<u8> {
    c.prepared().models().unwrap().next().unwrap().0.to_vec()
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

fn failed(result: Result<Completed, Error>) -> Error {
    match result {
        Ok(_) => panic!("unexpected synthetic package prefix"),
        Err(e) => e,
    }
}
fn word(b: &[u8], at: usize) -> usize {
    u32::from_le_bytes(b[at..at + 4].try_into().unwrap()) as usize
}
// Whole-CKey verified synthetic bytes still declare an unknown TACT section.
// The available DB2 reader must omit it; no encrypted real source is acquired.
fn unknown(mut b: Vec<u8>, text: bool) -> Vec<u8> {
    let start = word(&b, 212);
    let rows = word(&b, 136);
    let size = word(&b, 144);
    if text {
        for i in 0..rows {
            for field in [0, 4] {
                let at = start + i * size + field;
                let n = word(&b, at);
                put(&mut b, at, (n + size) as u32);
            }
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
    out.extend(vec![0xa5; size]);
    out.extend(99u32.to_le_bytes());
    out
}

#[test]
fn genuine_package_both_builds_and_both_profiles_matches_inventory_visit() {
    for build in [70374, 70425] {
        for native in [false, true] {
            let f = one_m2(build);
            let p = Private::new();
            let mut c = ready(&f, &p, build, native);
            assert_eq!(c.profile(), PROFILE);
            assert_eq!(
                c.prepared().stage().world().profile().contains("gnu15"),
                native
            );
            let expected: Vec<_> = c
                .artifacts()
                .unwrap()
                .map(|(n, b)| (n.to_string(), b))
                .collect();
            let mut actual = Vec::new();
            c.visit(4 * 1024 * 1024, |a| {
                actual.push((a.name().to_string(), a.bytes().to_vec()));
                Ok::<_, u8>(())
            })
            .unwrap();
            assert_eq!(
                expected,
                actual
                    .iter()
                    .map(|(n, b)| (n.clone(), b.len()))
                    .collect::<Vec<_>>()
            );
            assert_eq!(c.stats().unwrap().files, expected.len());
            assert_eq!(
                c.stats().unwrap().output_bytes,
                expected.iter().map(|(_, b)| b).sum::<usize>()
            );
            c.with_model(&name(&c), 0, |b| {
                forever_vmap::vmo::View::parse(b, Default::default()).unwrap();
                Ok::<_, u8>(())
            })
            .unwrap();
            c.with_map(1, 0, |v| {
                assert!(v.artifacts().count() >= 1);
                Ok::<_, u8>(())
            })
            .unwrap();
            c.verify_unchanged().unwrap();
        }
    }
}
#[test]
fn absent_wdt_does_not_become_an_empty_final_tree() {
    for build in [70374, 70425] {
        let f = CollisionFixture::for_build(&files(build, &[(1, 100, -1)]), build);
        let p = Private::new();
        let c = ready(&f, &p, build, true);
        assert_eq!(c.prepared().stage().world().report().missing_wdts, 1);
        assert_eq!(c.stats().unwrap().maps, 0);
        assert_eq!(
            c.artifacts()
                .unwrap()
                .map(|(n, _)| n.to_string())
                .collect::<Vec<_>>(),
            vec!["GameObjectModels.dtree"]
        );
    }
}
#[test]
fn present_empty_wdt_produces_one_genuine_empty_tree() {
    for build in [70374, 70425] {
        let mut rows = files(build, &[(1, 100, -1)]);
        rows.push((100, None, fixture::wdt(None, None)));
        let f = CollisionFixture::for_build(&rows, build);
        let p = Private::new();
        let mut c = ready(&f, &p, build, true);
        assert_eq!(c.stats().unwrap().maps, 1);
        assert_eq!(c.stats().unwrap().selected_models, 0);
        assert_eq!(
            c.artifacts()
                .unwrap()
                .map(|(n, _)| n.to_string())
                .collect::<Vec<_>>(),
            vec!["GameObjectModels.dtree", "0001/0001.vmtree"]
        );
        c.with_map(1, 0, |v| {
            assert_eq!(v.artifacts().count(), 1);
            Ok::<_, u8>(())
        })
        .unwrap();
    }
}
#[test]
fn partial_map_and_godi_facts_stay_available_in_the_retained_owner() {
    for build in [70374, 70425] {
        let mut rows = files(build, &[(1, 100, -1)]);
        godi(&mut rows, build, &[(1, 90)]);
        for id in [
            gameobjects::TABLE_FILE_DATA_ID,
            strict_world::MAP_FILE_DATA_ID,
        ] {
            let r = rows.iter_mut().find(|r| r.0 == id).unwrap();
            r.2 = unknown(r.2.clone(), id == strict_world::MAP_FILE_DATA_ID);
        }
        rows.extend([
            (100, None, fixture::wdt(None, None)),
            (90, None, fixture::m2(true)),
        ]);
        let f = CollisionFixture::for_build(&rows, build);
        let p = Private::new();
        let c = ready(&f, &p, build, true);
        let world = c.prepared().stage().world();
        assert!(!world.map().all_sections_admitted());
        assert_eq!(world.map().direct_rows, 1);
        assert_eq!(world.map().declared_direct_rows, 2);
        assert_eq!(world.topology().direct_count(), 1);
        assert!(
            !world
                .startup_view()
                .data_view()
                .unwrap()
                .table()
                .all_sections_admitted()
        );
        assert_eq!(c.stats().unwrap().maps, 1);
        assert_eq!(c.stats().unwrap().selected_models, 1);
    }
}
#[test]
fn inherited_child_keeps_its_index_and_first_real_ancestor_payload() {
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
    let mut c = ready(&f, &p, build, true);
    assert_eq!(c.prepared().stage().world().report().ids.calls, 1);
    assert_eq!(c.stats().unwrap().parent_only_tiles, 1);
    assert_eq!(c.stats().unwrap().payload_tiles, 1);
    assert!(c.stats().unwrap().ancestor_steps >= 1);
    c.with_map(2, 0, |v| {
        let names: Vec<_> = v.artifacts().map(|a| a.name().to_string()).collect();
        assert!(names.iter().any(|n| n.ends_with(".vmtileidx")));
        assert!(!names.iter().any(|n| n.ends_with(".vmtile")));
        Ok::<_, u8>(())
    })
    .unwrap();
}
#[test]
fn empty_owned_cache_barrier_does_not_synthesize_grandparent_placements() {
    let build = 70374;
    let mut rows = files(build, &[(1, 100, -1), (2, 200, 1), (3, 300, 2)]);
    rows.extend([
        (100, None, fixture::wdt(None, Some(101))),
        (101, None, fixture::m2_adt(90, 17)),
        (200, None, fixture::wdt(None, Some(201))),
        (201, None, chunk(b"MVER", &18u32.to_le_bytes())),
        (300, None, fixture::wdt(None, Some(301))),
        (90, None, fixture::m2(true)),
    ]);
    let f = CollisionFixture::for_build(&rows, build);
    let p = Private::new();
    let c = ready(&f, &p, build, true);
    assert_eq!(c.prepared().stage().world().report().ids.calls, 1);
    assert_eq!(c.stats().unwrap().unique_spawns, 1);
    assert_eq!(c.stats().unwrap().maps, 3);
    assert!(
        c.state
            .maps
            .iter()
            .filter(|m| m.bound.id != 1)
            .all(|m| m.tiles.is_empty())
    );
}
#[test]
fn orphan_child_raw_is_owned_but_not_a_final_model_artifact() {
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
        let c = ready(&f, &p, build, true);
        assert_eq!(
            c.prepared()
                .stage()
                .world()
                .report()
                .placements
                .omitted_requests,
            1
        );
        assert_eq!(c.prepared().stage().world().report().ids.calls, 0);
        assert_eq!(c.state.models.view().index.len(), 1);
        assert_eq!(c.stats().unwrap().selected_models, 0);
        assert!(
            !c.artifacts()
                .unwrap()
                .any(|(n, _)| matches!(n, resident::ArtifactName::Model(_)))
        );
    }
}
#[test]
fn doodad_only_parent_emits_only_the_genuine_child_vmo() {
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
        let c = ready(&f, &p, build, true);
        assert_eq!(c.stats().unwrap().selected_models, 1);
        assert!(
            c.prepared()
                .models()
                .unwrap()
                .all(|(_, class)| class == crate::collision::Class::M2)
        );
    }
}
#[test]
fn n05_complete_empty_wmo_preserves_the_following_m2_id_and_package() {
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
        let c = ready(&f, &p, build, true);
        assert_eq!(c.prepared().stage().world().report().ids.calls, 1);
        assert_eq!(c.stats().unwrap().unique_spawns, 1);
        assert_eq!(c.stats().unwrap().selected_models, 1);
        assert_eq!(c.state.models.view().index.len(), 1);
    }
}
#[test]
fn all_physical_names_stay_selected_before_first_id_wins() {
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
    let c = ready(&f, &p, build, true);
    assert_eq!(c.stats().unwrap().selected_models, 2);
    assert_eq!(c.stats().unwrap().input_records, 2);
    assert_eq!(c.stats().unwrap().unique_spawns, 1);
    assert_eq!(c.prepared().models().unwrap().count(), 2);
}
#[test]
fn nonfresh_map_repository_is_rejected_without_prefix() {
    let build = 70374;
    let f = one_m2(build);
    let p = Private::new();
    let m = prepared(&f, &p, build, true);
    let mut maps = p.spool(&f, "maps");
    maps.write(1, |w| w.bytes(b"x")).unwrap();
    assert_eq!(
        failed(assemble(m, maps, Limits::default())).cause,
        Cause::Kind(Kind::State)
    );
}
#[test]
fn map_backend_mismatch_is_rejected_even_for_empty_map_domain() {
    let build = 70425;
    let f = CollisionFixture::for_build(&files(build, &[]), build);
    let p = Private::new();
    let m = prepared(&f, &p, build, true);
    let maps = Spool::create_with_backend(
        &p.0.join("maps"),
        ProtectedInputs::capture(f.path(), &[]).unwrap(),
        Profile::BoundedV1.spool_limits(),
        spool::Profile::SameMountV1,
        Profile::BoundedV1,
        BackendProfile::FileSetV1,
    )
    .unwrap();
    assert_eq!(
        failed(assemble(m, maps, Limits::default())).cause,
        Cause::Kind(Kind::InvalidLimits)
    );
}
#[test]
fn package_budget_includes_models_two_descriptors_and_all_actual_capacities() {
    let build = 70374;
    let f = one_m2(build);
    let p = Private::new();
    let m = prepared(&f, &p, build, true);
    let maps = p.spool(&f, "maps");
    let base = m.view().live().unwrap()
        + 2 * maps.limits().max_descriptor_bytes
        + super::super::GUARD_BYTES;
    let limits = Limits {
        max_memory_bytes: base - 1,
        ..Limits::default()
    };
    assert_eq!(
        failed(assemble(m, maps, limits)).cause,
        Cause::Kind(Kind::Limit)
    );
}
#[test]
fn upward_package_limits_are_rejected_without_changing_the_owner_profile() {
    let build = 70425;
    let f = one_m2(build);
    let p = Private::new();
    let m = prepared(&f, &p, build, true);
    let mut limits = Limits::default();
    limits.max_maps += 1;
    assert_eq!(
        failed(assemble(m, p.spool(&f, "maps"), limits)).cause,
        Cause::Kind(Kind::InvalidLimits)
    );
}
#[test]
fn refused_callback_capture_drops_before_all_final_guards() {
    let build = 70374;
    let f = one_m2(build);
    let p = Private::new();
    let mut c = ready(&f, &p, build, true);
    let count = Rc::new(Cell::new(0));
    let action = OnDrop {
        path: p.0.join("maps"),
        count: count.clone(),
        mode: true,
    };
    let e = c
        .with_map(1, usize::MAX, move |_| {
            let _ = &action;
            Ok::<_, u8>(())
        })
        .unwrap_err();
    assert_eq!(count.get(), 1);
    assert_eq!(e.cause, Cause::Kind(Kind::Limit));
    assert_eq!(e.guards.maps, Some(payload::Error::Changed));
}
#[test]
fn typed_callback_error_survives_changed_map_repository() {
    let build = 70425;
    let f = one_m2(build);
    let p = Private::new();
    let mut c = ready(&f, &p, build, true);
    let count = Rc::new(Cell::new(0));
    let action = OnDrop {
        path: p.0.join("maps"),
        count: count.clone(),
        mode: true,
    };
    let e = c
        .with_map(1, 0, move |_| {
            drop(action);
            Err::<(), u8>(7)
        })
        .unwrap_err();
    assert_eq!(count.get(), 1);
    assert_eq!(e.cause, Cause::Callback(7));
    assert_eq!(e.code(), payload::Error::Changed.code());
}
#[test]
fn rejected_success_value_drops_then_protected_root_and_map_guards_recheck() {
    let build = 70374;
    let f = one_m2(build);
    let p = Private::new();
    let mut c = ready(&f, &p, build, true);
    let _restore = RestoreRoot::new(f.path());
    let count = Rc::new(Cell::new(0));
    let value = RootDrop {
        path: f.path().to_path_buf(),
        count: count.clone(),
    };
    let maps = p.0.join("maps");
    let e = match c.with_map(1, 0, move |_| {
        fs::set_permissions(maps, fs::Permissions::from_mode(0o755)).unwrap();
        Ok::<_, u8>(value)
    }) {
        Ok(_) => panic!("returned value escaped changed repository"),
        Err(e) => e,
    };
    assert_eq!(count.get(), 1);
    assert!(e.guards.models.collector.is_some());
    assert_eq!(e.guards.maps, Some(payload::Error::Changed));
    assert_eq!(e.code(), payload::Error::Changed.code());
}
#[test]
fn visit_heap_refusal_drops_owned_capture_before_final_archive_checks() {
    let build = 70425;
    let f = one_m2(build);
    let p = Private::new();
    let mut c = ready(&f, &p, build, true);
    let count = Rc::new(Cell::new(0));
    let action = OnDrop {
        path: p.0.join("vmos"),
        count: count.clone(),
        mode: true,
    };
    let e = c
        .visit(usize::MAX, move |_| {
            let _ = &action;
            Ok::<_, u8>(())
        })
        .unwrap_err();
    assert_eq!(count.get(), 1);
    assert_eq!(e.cause, Cause::Kind(Kind::Limit));
    assert_eq!(e.guards.models.archive, Some(payload::Error::Changed));
}
#[test]
fn poisoned_package_never_exposes_a_later_artifact_prefix() {
    let build = 70374;
    let f = one_m2(build);
    let p = Private::new();
    let mut c = ready(&f, &p, build, true);
    assert!(c.with_map(u32::MAX, 0, |_| Ok::<_, u8>(())).is_err());
    assert!(c.stats().is_err());
    assert!(c.artifacts().is_err());
    assert!(c.verify_unchanged().is_err());
}
#[test]
fn typed_nested_input_change_wins_over_a_simultaneous_outer_map_change() {
    let e = Error::<u8> {
        cause: Cause::Models(models::Cause::Collector(CollectorError::Collision(
            crate::collision::Error::Store(local_store::Error::InputChanged),
        ))),
        guards: Guards {
            models: models::Guards::default(),
            maps: Some(payload::Error::Changed),
        },
    };
    assert_eq!(e.code(), local_store::Error::InputChanged.code());
    let observed = CollectorError::Collision(crate::collision::Error::Store(
        local_store::Error::InputChanged,
    ));
    let startup: startup::Error = observed.into();
    let e = Error::<u8> {
        cause: Cause::Models(models::Cause::Phase(placement::Cause::Startup(startup))),
        guards: Guards {
            models: models::Guards::default(),
            maps: Some(payload::Error::Changed),
        },
    };
    assert_eq!(e.code(), local_store::Error::InputChanged.code());
    let e = Error::<u8> {
        cause: Cause::Callback(9),
        guards: Guards {
            models: models::Guards {
                collector: Some(CollectorError::Collision(crate::collision::Error::Store(
                    local_store::Error::InputChanged,
                ))),
                ..Default::default()
            },
            maps: Some(payload::Error::Changed),
        },
    };
    assert_eq!(e.code(), local_store::Error::InputChanged.code());
}
#[test]
fn private_bundle_layout_and_borrowed_workspace_stay_original_and_bounded() {
    assert_eq!(super::super::MAGIC, b"RCEPKG1\0");
    assert_eq!(super::super::HEADER, 64);
    assert_eq!(super::super::TILE_HEADER, 48);
    assert!(
        std::hint::black_box(std::mem::size_of::<
            super::super::PackageWork<'_, models::Prepared>,
        >()) <= super::super::GUARD_BYTES
    );
}

#[test]
fn resource_profile_mismatch_is_rejected_without_promoting_bounded_owners() {
    let build = 70374;
    let f = one_m2(build);
    let p = Private::new();
    let m = prepared(&f, &p, build, true);
    let profile = Profile::DiskWorldV1;
    let maps = Spool::create_with_backend(
        &p.0.join("maps"),
        ProtectedInputs::capture(f.path(), &[]).unwrap(),
        profile.spool_limits(),
        spool::Profile::SameMountV1,
        profile,
        BackendProfile::LeasedAppendV1,
    )
    .unwrap();
    assert_eq!(
        failed(assemble(m, maps, Limits::default())).cause,
        Cause::Kind(Kind::InvalidLimits)
    );
}
#[test]
fn smaller_package_budget_retains_both_model_and_package_ledgers() {
    let build = 70425;
    let f = one_m2(build);
    let p = Private::new();
    let m = prepared(&f, &p, build, true);
    let maps = p.spool(&f, "maps");
    let model_cap = m.view().limits.max_memory_bytes;
    let base = m.view().live().unwrap()
        + 2 * maps.limits().max_descriptor_bytes
        + super::super::GUARD_BYTES;
    let limits = Limits {
        max_memory_bytes: base + 64 * 1024 * 1024,
        ..Limits::default()
    };
    assert!(limits.max_memory_bytes < model_cap);
    let mut c = assemble(m, maps, limits).unwrap();
    c.with_model(&name(&c), 1024 * 1024, |b| {
        assert!(!b.is_empty());
        Ok::<_, u8>(())
    })
    .unwrap();
    assert!(c.stats().unwrap().peak_admitted_bytes <= limits.max_memory_bytes);
    assert!(c.stats().unwrap().reserved_bytes >= base);
    assert_eq!(c.prepared().stage().world().report().ids.calls, 1);
}

#[test]
fn raw_repository_only_change_is_not_hidden_by_an_earlier_callback_refusal() {
    let build = 70425;
    let f = one_m2(build);
    let p = Private::new();
    let mut c = ready(&f, &p, build, true);
    let count = Rc::new(Cell::new(0));
    let action = OnDrop {
        path: p.0.join("models"),
        count: count.clone(),
        mode: true,
    };
    let e = c
        .with_map(u32::MAX, 0, move |_| {
            let _ = &action;
            Ok::<_, u8>(())
        })
        .unwrap_err();
    assert_eq!(count.get(), 1);
    assert_eq!(e.cause, Cause::Kind(Kind::Missing));
    assert_eq!(
        e.guards.models.collector,
        Some(CollectorError::Payload(payload::Error::Changed))
    );
    assert_eq!(e.guards.models.placements, None);
    assert_eq!(e.guards.models.archive, None);
    assert_eq!(e.guards.maps, None);
    assert_eq!(e.code(), payload::Error::Changed.code());
}

fn ready_with_coverage(f: &CollisionFixture, p: &Private, build: u32, native: bool) -> Completed {
    let stage = stage_inner(f, p, build, native, true);
    let models = models::prepare(stage, p.spool(f, "vmos"), models::Limits::default()).unwrap();
    assemble(models, p.spool(f, "maps"), Limits::default()).unwrap()
}
// Authored 2026-10-17: authentic traversal coverage reaches the final owner.
#[test]
fn package_coverage_both_builds_keeps_same_world_and_final_artifacts() {
    use crate::collision::run::compatible_inventory::coverage::{Own, Replay, Wdt};
    for build in [70374, 70425] {
        let f = one_m2(build);
        let p = Private::new();
        let mut c = ready_with_coverage(&f, &p, build, true);
        let before = c.stats().unwrap();
        c.with_coverage(0, |v| {
            assert_eq!(v.maps().len(), 1);
            let map = &v.maps()[0];
            assert_eq!(map.id(), 1);
            assert_eq!(map.wdt(), Wdt::Present);
            assert_eq!(map.tile(0, 0).unwrap().own(), Own::Output);
            assert_eq!(map.tile(0, 0).unwrap().replay(), Replay::None);
            assert_eq!(
                v.startup().table().file_data_id(),
                gameobjects::TABLE_FILE_DATA_ID
            );
            assert_eq!(v.report().object_adts, 1);
            assert!(v.profile().contains("gnu15"));
            Ok::<_, u8>(())
        })
        .unwrap();
        c.with_map(1, 0, |v| {
            let proof = forever_vmap::placement::DirBin::parse(
                v.ordered_dir_bin(),
                forever_vmap::placement::Limits::default(),
            )
            .unwrap();
            assert_eq!(proof.record_count(), 1);
            Ok::<_, u8>(())
        })
        .unwrap();
        assert_eq!(c.stats().unwrap().maps, before.maps);
        c.verify_unchanged().unwrap();
    }
}
#[test]
fn coverage_budget_refusal_drops_uninvoked_capture_then_checks_map_archive() {
    let build = 70425;
    let f = one_m2(build);
    let p = Private::new();
    let mut c = ready_with_coverage(&f, &p, build, true);
    let count = Rc::new(Cell::new(0));
    let calls = Rc::new(Cell::new(0));
    let seen = calls.clone();
    let action = OnDrop {
        path: p.0.join("maps"),
        count: count.clone(),
        mode: true,
    };
    let e = c
        .with_coverage(usize::MAX, move |_| {
            seen.set(seen.get() + 1);
            drop(action);
            Ok::<_, u8>(())
        })
        .unwrap_err();
    assert_eq!(calls.get(), 0);
    assert_eq!(count.get(), 1);
    assert_eq!(e.guards.maps, Some(payload::Error::Changed));
    assert_eq!(e.code(), payload::Error::Changed.code());
}
#[test]
fn coverage_callback_error_retains_typed_cause_and_vmo_guard() {
    let build = 70374;
    let f = one_m2(build);
    let p = Private::new();
    let mut c = ready_with_coverage(&f, &p, build, true);
    let count = Rc::new(Cell::new(0));
    let action = OnDrop {
        path: p.0.join("vmos"),
        count: count.clone(),
        mode: true,
    };
    let e = c
        .with_coverage(4096, move |_| {
            drop(action);
            Err::<(), u8>(23)
        })
        .unwrap_err();
    assert_eq!(count.get(), 1);
    assert_eq!(e.cause, Cause::Callback(23));
    assert_eq!(e.guards.models.archive, Some(payload::Error::Changed));
    assert_eq!(e.code(), payload::Error::Changed.code());
}
#[test]
fn coverage_rejected_value_drops_before_final_protected_and_map_guards() {
    let build = 70425;
    let f = one_m2(build);
    let p = Private::new();
    let mut c = ready_with_coverage(&f, &p, build, true);
    let _restore = RestoreRoot::new(f.path());
    let count = Rc::new(Cell::new(0));
    let value = RootDrop {
        path: f.path().to_path_buf(),
        count: count.clone(),
    };
    let maps = p.0.join("maps");
    let e = match c.with_coverage(4096, move |_| {
        fs::set_permissions(maps, fs::Permissions::from_mode(0o755)).unwrap();
        Ok::<_, u8>(value)
    }) {
        Ok(_) => panic!("coverage value escaped changed map archive"),
        Err(e) => e,
    };
    assert_eq!(count.get(), 1);
    assert!(e.guards.models.collector.is_some());
    assert_eq!(e.guards.maps, Some(payload::Error::Changed));
    assert_eq!(e.code(), payload::Error::Changed.code());
}
#[test]
fn coverage_error_poisons_later_metadata_without_prefix_access() {
    let build = 70425;
    let f = one_m2(build);
    let p = Private::new();
    let mut c = ready_with_coverage(&f, &p, build, true);
    let e = c.with_coverage(0, |_| Err::<(), u8>(9)).unwrap_err();
    assert_eq!(e.cause, Cause::Callback(9));
    let calls = Cell::new(0);
    assert!(
        c.with_coverage(0, |_| {
            calls.set(1);
            Ok::<_, u8>(())
        })
        .is_err()
    );
    assert_eq!(calls.get(), 0);
    assert!(c.stats().is_err());
    assert!(c.artifacts().is_err());
}
