// SPDX-License-Identifier: GPL-3.0-or-later
// Authored toy-CASC tests only; no execution claimed by the private candidate.
use super::*;
use crate::collision::{
    gameobjects,
    resources::Profile,
    spool::{self, ProtectedInputs, Spool},
    world::tests as fixture,
};
use crate::local_store::tests::{CollisionFile, CollisionFixture};
use crate::{Locale, local_index, local_store, root};
use forever_db2::catalog;
use std::{
    cell::RefCell,
    fs,
    os::unix::fs::{DirBuilderExt, PermissionsExt},
    path::PathBuf,
    rc::Rc,
    sync::atomic::{AtomicU64, Ordering},
};

fn put(b: &mut [u8], at: usize, n: u32) {
    b[at..at + 4].copy_from_slice(&n.to_le_bytes());
}
fn word(b: &[u8], at: usize) -> usize {
    u32::from_le_bytes(b[at..at + 4].try_into().unwrap()) as usize
}
fn hash(b: &mut [u8], name: &str) {
    put(
        b,
        152,
        catalog::resolve_for(catalog::WOWDBDEFS_70374_CATALOG_ID, name)
            .unwrap()
            .unwrap()
            .metadata()
            .table_hash
            .unwrap(),
    );
}
fn files(maps: &[(u32, u32, i16)]) -> Vec<CollisionFile<'static>> {
    let mut fs = fixture::files(maps);
    for (id, _, b) in &mut fs {
        let name = match *id {
            gameobjects::TABLE_FILE_DATA_ID => Some("GameObjectDisplayInfo"),
            strict_world::MAP_FILE_DATA_ID => Some("Map"),
            strict_world::MATERIAL_FILE_DATA_ID => Some("LiquidMaterial"),
            strict_world::TYPE_FILE_DATA_ID => Some("LiquidType"),
            _ => None,
        };
        if let Some(n) = name {
            hash(b, n);
        }
    }
    fs
}
fn godi(fs: &mut [CollisionFile<'_>], rows: &[(u32, u32)]) {
    let b = &mut fs
        .iter_mut()
        .find(|f| f.0 == gameobjects::TABLE_FILE_DATA_ID)
        .unwrap()
        .2;
    *b = fixture::gameobject_db2(rows, &[]);
    hash(b, "GameObjectDisplayInfo");
}
fn map_bytes<'a>(fs: &'a mut [CollisionFile<'_>]) -> &'a mut Vec<u8> {
    &mut fs
        .iter_mut()
        .find(|f| f.0 == strict_world::MAP_FILE_DATA_ID)
        .unwrap()
        .2
}
fn unknown(b: Vec<u8>, text: bool) -> Vec<u8> {
    let start = word(&b, 212);
    let rows = word(&b, 136);
    let size = word(&b, 144);
    let mut b = b;
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
fn open(f: &CollisionFixture) -> local_store::LocalStore {
    local_store::LocalStore::open_db2_70374(
        f.path(),
        local_store::OpenOptions {
            locale: Locale::EnUs,
            index_layouts: [local_index::Layout::Contiguous; 16],
            root_layout: root::Layout::Header50893V2,
            limits: local_store::Limits::default(),
        },
    )
    .unwrap()
}
fn holes(fs: &[CollisionFile<'_>], holes: &[(u32, std::ops::Range<usize>)]) -> CollisionFixture {
    let f = CollisionFixture::with_available_holes(fs, holes);
    let p = f.path().join(".build.info");
    let text = fs::read_to_string(&p).unwrap();
    fs::write(p, text.replace("1.60.1.70205", "1.60.1.70374")).unwrap();
    f
}
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Private {
    base: PathBuf,
}
impl Private {
    fn new() -> Self {
        let base = std::env::temp_dir().join(format!(
            "rce-available-disk-world-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::DirBuilder::new().mode(0o700).create(&base).unwrap();
        Self { base }
    }
    fn spool(&self, f: &CollisionFixture, name: &str, p: Profile) -> Spool {
        Spool::create_with_policy(
            &self.base.join(name),
            ProtectedInputs::capture(f.path(), &[]).unwrap(),
            p.spool_limits(),
            spool::Profile::SameMountV1,
            p,
        )
        .unwrap()
    }
    fn inventories(&self, f: &CollisionFixture, p: Profile, l: Limits) -> (Inventory, Inventory) {
        let mut limits = p.inventory_limits();
        limits.wdt = l.world.run.wdt;
        limits.cache = l.world.run.cache;
        let wdt_limits = inventory::Limits {
            max_heap_bytes: limits.max_heap_bytes.min(l.world.run.max_wdt_memory_bytes),
            ..limits
        };
        let cache_limits = inventory::Limits {
            max_heap_bytes: limits
                .max_heap_bytes
                .min(l.world.run.max_cache_memory_bytes),
            ..limits
        };
        (
            Inventory::new(self.spool(f, "wdts", p), wdt_limits).unwrap(),
            Inventory::new(self.spool(f, "caches", p), cache_limits).unwrap(),
        )
    }
}
impl Drop for Private {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.base).unwrap();
    }
}
type Records = Rc<RefCell<Vec<(run::Context, Vec<u8>)>>>;
struct Sink {
    records: Records,
    fail: bool,
    on_drop: Option<PathBuf>,
    on_drop_mode: Option<PathBuf>,
}
impl Sink {
    fn new() -> Self {
        Self {
            records: Rc::default(),
            fail: false,
            on_drop: None,
            on_drop_mode: None,
        }
    }
}
impl Drop for Sink {
    fn drop(&mut self) {
        if let Some(p) = &self.on_drop {
            fs::write(p, b"changed-authored-only").unwrap();
        }
        if let Some(p) = &self.on_drop_mode {
            fs::set_permissions(p, fs::Permissions::from_mode(0o755)).unwrap();
        }
    }
}
impl run::inventory::Sink for Sink {
    type Error = u8;
    fn placements(&mut self, c: run::Context, b: &[u8]) -> Result<(), u8> {
        if self.fail {
            return Err(71);
        }
        let mut out = self.records.borrow_mut();
        assert!(
            out.len() < 4096
                && out.iter().map(|r| r.1.len()).sum::<usize>() + b.len() < 8 * 1024 * 1024
        );
        out.push((c, b.to_vec()));
        Ok(())
    }
}
fn limits(p: Profile) -> Limits {
    let mut l = p.stage_limits().world;
    l.world.run.placements.ids.max_memory_bytes = 1024 * 1024;
    l
}
fn simple() -> Vec<CollisionFile<'static>> {
    let mut fs = files(&[(7, 100, -1)]);
    godi(&mut fs, &[(1, 50), (2, 50)]);
    fs.extend([
        (50, None, fixture::m2(true)),
        (100, None, fixture::wdt(None, Some(201))),
        (201, None, fixture::m2_adt(50, 7)),
    ]);
    fs
}
fn perform(
    f: &CollisionFixture,
    p: &Private,
    profile: Profile,
    sink: Sink,
) -> Result<Completed, Error<u8>> {
    let mut store = open(f);
    let start = super::super::extract_startup_fdid(
        &mut store,
        profile.startup_limits(),
        p.spool(f, "payload", profile),
    )
    .unwrap();
    let l = limits(profile);
    let (w, c) = p.inventories(f, profile, l);
    extract(start, w, c, sink, 8 * 1024 * 1024, l)
}

#[test]
fn empty_world_keeps_available_map_and_complete_liquid_facts() {
    let f = CollisionFixture::for_build(&files(&[(7, 0, -1)]), 70374);
    let p = Private::new();
    let out = perform(&f, &p, Profile::BoundedV1, Sink::new()).unwrap();
    assert_eq!(out.profile(), PROFILE);
    assert!(out.map().all_sections_admitted());
    assert!(out.map().full_ckey_verified);
    assert_eq!(out.topology().direct_count(), 1);
    assert!(!out.topology().get(7).unwrap().active());
    assert_eq!(out.startup().extraction().facts().reads, 4);
    assert_eq!(
        out.liquids()[0].source().file_data_id(),
        strict_world::MATERIAL_FILE_DATA_ID
    );
    assert_eq!(
        out.liquids()[1].source().file_data_id(),
        strict_world::TYPE_FILE_DATA_ID
    );
    let charge =
        out.startup().data().unwrap().stats().resident_bytes + out.topology().resident_bytes();
    assert_eq!(
        out.startup().extraction().retained_external_for_test(),
        charge
    );
}
#[test]
fn complete_available_world_equals_existing_strict_disk_world() {
    let f = CollisionFixture::for_build(&simple(), 70374);
    let p = Private::new();
    let sink = Sink::new();
    let new_records = sink.records.clone();
    let new = perform(&f, &p, Profile::BoundedV1, sink).unwrap();
    let p2 = Private::new();
    let mut store = open(&f);
    let start = gameobjects::spooled::extract_startup_fdid(
        &mut store,
        gameobjects::Limits::default(),
        p2.spool(&f, "payload", Profile::BoundedV1),
    )
    .unwrap();
    let l = limits(Profile::BoundedV1);
    let (w, c) = p2.inventories(&f, Profile::BoundedV1, l);
    let sink = Sink::new();
    let old_records = sink.records.clone();
    let old = strict_world::inventory::extract(start, w, c, sink, 8 * 1024 * 1024, l).unwrap();
    assert_eq!(*new_records.borrow(), *old_records.borrow());
    assert_eq!(
        new.startup().data().unwrap().lists().temp_bytes(),
        old.startup().data().unwrap().lists().temp_bytes()
    );
    assert_eq!(
        new.startup().data().unwrap().lists().dtree_bytes(),
        old.startup().data().unwrap().lists().dtree_bytes()
    );
    assert_eq!(new.report().ids, old.report().ids);
    assert_eq!(
        new.startup().extraction().facts(),
        old.startup().extraction().facts()
    );
    assert_eq!(new.map_stats(), old.map_stats());
    assert_eq!(new.lookup_stats(), old.lookup_stats());
}
#[test]
fn startup_cache_handles_and_cumulative_reads_survive_world() {
    let f = CollisionFixture::for_build(&simple(), 70374);
    let p = Private::new();
    let mut store = open(&f);
    store.enable_names(root::name::Limits::default()).unwrap();
    let start = super::super::extract_startup_fdid(
        &mut store,
        gameobjects::Limits::default(),
        p.spool(&f, "payload", Profile::BoundedV1),
    )
    .unwrap();
    let handle = start.data().unwrap().outcomes()[0].outcome();
    let l = limits(Profile::BoundedV1);
    let (w, c) = p.inventories(&f, Profile::BoundedV1, l);
    let out = extract(start, w, c, Sink::new(), 8 * 1024 * 1024, l).unwrap();
    assert_eq!(
        out.startup().data().unwrap().outcomes()[0].outcome(),
        handle
    );
    assert_eq!(out.report().collector_before.reads, 5);
    assert_eq!(out.startup().extraction().facts().reads, 7);
    assert_eq!(out.startup().extraction().facts().models, 1);
    assert!(store.names_enabled());
    store.verify_unchanged().unwrap();
}
#[test]
fn two_encrypted_unknown_tables_keep_distinct_available_coverage() {
    let mut fs = simple();
    let godi = fs
        .iter_mut()
        .find(|f| f.0 == gameobjects::TABLE_FILE_DATA_ID)
        .unwrap();
    godi.2 = unknown(godi.2.clone(), false);
    let a = word(&godi.2, 252)..godi.2.len();
    let map = map_bytes(&mut fs);
    *map = unknown(map.clone(), true);
    let b = word(map, 252)..map.len();
    let f = holes(
        &fs,
        &[
            (gameobjects::TABLE_FILE_DATA_ID, a),
            (strict_world::MAP_FILE_DATA_ID, b),
        ],
    );
    let p = Private::new();
    let out = perform(&f, &p, Profile::BoundedV1, Sink::new()).unwrap();
    for table in [out.startup().data().unwrap().table(), out.map()] {
        assert!(!table.full_ckey_verified);
        assert!(table.encoded_key_authenticated);
        assert!(!table.all_sections_admitted());
        assert!(table.available_bytes < table.declared_bytes);
    }
    assert_eq!(out.map().direct_rows, 1);
    assert_eq!(out.map().materialized_rows, 1);
    assert_eq!(out.topology().direct_count(), 1);
    assert_eq!(out.startup().extraction().facts().models, 1);
}
#[test]
fn map_copies_materialize_without_becoming_direct_requests() {
    let mut fs = files(&[(7, 0, -1)]);
    *map_bytes(&mut fs) = fixture::map_db2(&[(7, 0, -1)], &[(8, 7)]);
    hash(map_bytes(&mut fs), "Map");
    let f = CollisionFixture::for_build(&fs, 70374);
    let p = Private::new();
    let out = perform(&f, &p, Profile::BoundedV1, Sink::new()).unwrap();
    assert_eq!(
        (
            out.map().direct_rows,
            out.map().copy_records,
            out.map().materialized_rows
        ),
        (1, 1, 2)
    );
    assert_eq!(out.topology().direct_count(), 1);
    assert_eq!(out.topology().all().len(), 2);
}
#[test]
fn signed_negative_parent_and_inactive_rows_remain_in_topology() {
    let mut fs = files(&[(7, 0, -1), (8, 100, -3)]);
    fs.push((100, None, fixture::wdt(None, None)));
    let f = CollisionFixture::for_build(&fs, 70374);
    let p = Private::new();
    let out = perform(&f, &p, Profile::BoundedV1, Sink::new()).unwrap();
    assert!(!out.topology().get(7).unwrap().active());
    assert_eq!(out.topology().get(8).unwrap().parent_map_id(), None);
    assert_eq!(out.topology().get(8).unwrap().physical_ordinal(), 1);
}
#[test]
fn inactive_parent_policy_is_not_relaxed() {
    let mut fs = files(&[(7, 0, -1), (8, 100, 7)]);
    fs.push((100, None, fixture::wdt(None, None)));
    let f = CollisionFixture::for_build(&fs, 70374);
    let p = Private::new();
    let e = perform(&f, &p, Profile::BoundedV1, Sink::new())
        .err()
        .unwrap();
    assert_eq!(e.code(), plan::Error::ParentUnavailable.code());
}
#[test]
fn missing_known_wdt_is_terminal() {
    let f = CollisionFixture::for_build(&files(&[(7, 100, -1)]), 70374);
    let p = Private::new();
    assert_eq!(
        perform(&f, &p, Profile::BoundedV1, Sink::new())
            .err()
            .unwrap()
            .code(),
        "file_unavailable"
    );
}
#[test]
fn missing_known_adt_is_terminal() {
    let mut fs = simple();
    fs.retain(|f| f.0 != 201);
    let f = CollisionFixture::for_build(&fs, 70374);
    let p = Private::new();
    assert_eq!(
        perform(&f, &p, Profile::BoundedV1, Sink::new())
            .err()
            .unwrap()
            .code(),
        "file_unavailable"
    );
}
#[test]
fn missing_world_model_is_terminal_after_empty_startup() {
    let mut fs = simple();
    godi(&mut fs, &[]);
    fs.retain(|f| f.0 != 50);
    let f = CollisionFixture::for_build(&fs, 70374);
    let p = Private::new();
    assert_eq!(
        perform(&f, &p, Profile::BoundedV1, Sink::new())
            .err()
            .unwrap()
            .code(),
        "file_unavailable"
    );
}
#[test]
fn complete_liquids_never_use_available_omission_path() {
    for id in [
        strict_world::MATERIAL_FILE_DATA_ID,
        strict_world::TYPE_FILE_DATA_ID,
    ] {
        let fs = files(&[(7, 0, -1)]);
        let len = fs.iter().find(|f| f.0 == id).unwrap().2.len();
        let f = holes(&fs, &[(id, 0..len)]);
        let p = Private::new();
        assert!(perform(&f, &p, Profile::BoundedV1, Sink::new()).is_err());
    }
}
#[test]
fn failed_sink_drop_mutation_closes_source_after_owned_drop() {
    let f = CollisionFixture::for_build(&simple(), 70374);
    let p = Private::new();
    let mut sink = Sink::new();
    sink.fail = true;
    sink.on_drop = Some(f.path().join(".build.info"));
    let e = perform(&f, &p, Profile::BoundedV1, sink).err().unwrap();
    assert_eq!(e.code(), "local_store_input_changed");
    assert!(e.guard().is_some());
}
#[test]
fn inventory_namespace_mutation_on_sink_drop_rejects_completion() {
    let f = CollisionFixture::for_build(&simple(), 70374);
    let p = Private::new();
    let mut sink = Sink::new();
    // Guards bind directory object/mode and registered entries, not a closed
    // enumeration of unrelated sibling names. Mutate the captured cache root.
    sink.on_drop_mode = Some(p.base.join("caches"));
    let e = perform(&f, &p, Profile::BoundedV1, sink).err().unwrap();
    assert!(e.code().contains("changed"));
    assert!(e.inventory_guards().iter().any(Option::is_some));
}
#[test]
fn resource_profile_mismatch_rejects_before_map_preparation() {
    let f = CollisionFixture::for_build(&files(&[]), 70374);
    let p = Private::new();
    let mut store = open(&f);
    let start = super::super::extract_startup_fdid(
        &mut store,
        Profile::DiskWorldV1.startup_limits(),
        p.spool(&f, "payload", Profile::DiskWorldV1),
    )
    .unwrap();
    let l = limits(Profile::BoundedV1);
    let (w, c) = p.inventories(&f, Profile::BoundedV1, l);
    assert!(extract(start, w, c, Sink::new(), 0, l).is_err());
}
#[test]
fn combined_memory_admission_exact_and_one_below() {
    for below in [false, true] {
        let f = CollisionFixture::for_build(&files(&[]), 70374);
        let p = Private::new();
        let mut store = open(&f);
        let start = super::super::extract_startup_fdid(
            &mut store,
            gameobjects::Limits::default(),
            p.spool(&f, "payload", Profile::BoundedV1),
        )
        .unwrap();
        let mut l = limits(Profile::BoundedV1);
        let (w, c) = p.inventories(&f, Profile::BoundedV1, l);
        l.max_combined_memory_bytes = start.collector.memory_ceiling()
            + w.limits().max_heap_bytes
            + c.limits().max_heap_bytes
            - usize::from(below);
        let result = extract(start, w, c, Sink::new(), 0, l);
        assert_eq!(result.is_err(), below);
    }
}
#[test]
fn disk_world_keeps_4097_startup_candidates_and_same_single_model_cache() {
    let mut fs = files(&[(7, 0, -1)]);
    godi(&mut fs, &(0..4097).map(|n| (n, 50)).collect::<Vec<_>>());
    fs.push((50, None, fixture::m2(true)));
    let f = CollisionFixture::for_build(&fs, 70374);
    let p = Private::new();
    let out = perform(&f, &p, Profile::DiskWorldV1, Sink::new()).unwrap();
    assert_eq!(
        out.startup().data().unwrap().stats().candidate_requests,
        4097
    );
    assert_eq!(out.startup().extraction().facts().models, 1);
    assert_eq!(out.startup().extraction().facts().reads, 5);
    assert_eq!(out.startup().extraction().resources(), Profile::DiskWorldV1);
    assert_eq!(Profile::DiskWorldV1.collector_limits().max_models, 65536);
    assert_eq!(
        Profile::DiskWorldV1.startup_limits().max_candidate_requests,
        71428
    );
}
#[test]
fn cached_wmo_raw_survives_liquid_and_world_transitions() {
    let mut fs = files(&[(7, 100, -1)]);
    godi(&mut fs, &[(1, 70)]);
    fs.extend([
        (70, None, fixture::wmo_root(60, None)),
        (60, None, fixture::wmo_group(true, false, true)),
        (80, None, fixture::wmo_root(61, None)),
        (61, None, fixture::wmo_group(true, false, true)),
        (100, None, fixture::wdt(Some(70), Some(201))),
        (201, None, fixture::wmo_adt(80, 8)),
    ]);
    let f = CollisionFixture::for_build(&fs, 70374);
    let p = Private::new();
    let mut store = open(&f);
    let mut start = super::super::extract_startup_fdid(
        &mut store,
        gameobjects::Limits::default(),
        p.spool(&f, "payload", Profile::BoundedV1),
    )
    .unwrap();
    let handle = match start.data().unwrap().outcomes()[0].outcome() {
        gameobjects::Outcome::Collision(h) => h,
        _ => panic!("WMO"),
    };
    let cached = start
        .collector_mut()
        .with_payload(handle, |p| Ok(p.raw().unwrap().to_vec()))
        .unwrap();
    let l = limits(Profile::BoundedV1);
    let (w, c) = p.inventories(&f, Profile::BoundedV1, l);
    let mut out = extract(start, w, c, Sink::new(), 8 * 1024 * 1024, l).unwrap();
    let after = out
        .extraction_mut()
        .with_payload(handle, |p| Ok(p.raw().unwrap().to_vec()))
        .unwrap();
    assert_eq!(after, cached);
    assert_eq!(out.startup().extraction().facts().models, 2);
}
#[test]
fn invalid_map_precedes_missing_liquid_dependency() {
    let mut fs = files(&[]);
    map_bytes(&mut fs)[0] = b'X';
    fs.retain(|f| f.0 != strict_world::MATERIAL_FILE_DATA_ID);
    let f = CollisionFixture::for_build(&fs, 70374);
    let p = Private::new();
    let e = perform(&f, &p, Profile::BoundedV1, Sink::new())
        .err()
        .unwrap();
    assert!(matches!(
        e.cause(),
        Cause::Available(available::Error::Db2(_))
    ));
}
#[test]
fn error_copy_shape_does_not_expand_old_disk_world_error() {
    fn copy<T: Copy>() {}
    copy::<Error<u8>>();
    assert!(size_of::<Error<u8>>() <= size_of::<strict_world::inventory::Error<u8>>());
}
