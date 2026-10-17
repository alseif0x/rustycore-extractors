// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-17: synthetic same-owner preparation/lookup/Drop controls.
use super::*;
use crate::collision::{
    InputRef, ModelRequest, gameobjects,
    resources::Profile,
    spool::{self, ProtectedInputs, Spool},
    world::tests as fixture,
};
use crate::local_store::{
    LocalStore,
    tests::{CollisionFile, CollisionFixture},
};
use crate::{Locale, local_index, local_store, root};
use forever_db2::catalog;
use std::{
    cell::Cell,
    fs,
    os::unix::fs::{DirBuilderExt, PermissionsExt},
    path::PathBuf,
    rc::Rc,
    sync::atomic::{AtomicU64, Ordering},
};

fn put(b: &mut [u8], at: usize, value: u32) {
    b[at..at + 4].copy_from_slice(&value.to_le_bytes());
}
fn files(build: u32, maps: &[(u32, u32, i16)]) -> Vec<CollisionFile<'static>> {
    let id = if build == 70425 {
        catalog::WOWDBDEFS_70425_CATALOG_ID
    } else {
        catalog::WOWDBDEFS_70374_CATALOG_ID
    };
    let mut rows = fixture::files(maps);
    for (fdid, _, bytes) in &mut rows {
        let name = match *fdid {
            gameobjects::TABLE_FILE_DATA_ID => "GameObjectDisplayInfo",
            strict_world::MAP_FILE_DATA_ID => "Map",
            strict_world::MATERIAL_FILE_DATA_ID => "LiquidMaterial",
            strict_world::TYPE_FILE_DATA_ID => "LiquidType",
            _ => unreachable!(),
        };
        let table = catalog::resolve_for(id, name).unwrap().unwrap();
        // Reuse wire builders only under the exact fixed target layout.
        assert_eq!(
            u32::from_le_bytes(bytes[156..160].try_into().unwrap()),
            table.metadata().layout_hash
        );
        put(bytes, 152, table.metadata().table_hash.unwrap());
    }
    rows
}
fn godi(rows: &mut [CollisionFile<'_>], build: u32, requests: &[(u32, u32)]) {
    let bytes = &mut rows
        .iter_mut()
        .find(|f| f.0 == gameobjects::TABLE_FILE_DATA_ID)
        .unwrap()
        .2;
    *bytes = fixture::gameobject_db2(requests, &[]);
    let cat = if build == 70425 {
        catalog::WOWDBDEFS_70425_CATALOG_ID
    } else {
        catalog::WOWDBDEFS_70374_CATALOG_ID
    };
    put(
        bytes,
        152,
        catalog::resolve_for(cat, "GameObjectDisplayInfo")
            .unwrap()
            .unwrap()
            .metadata()
            .table_hash
            .unwrap(),
    );
}
fn open(f: &CollisionFixture, build: u32) -> LocalStore {
    let options = local_store::OpenOptions {
        locale: Locale::EnUs,
        index_layouts: [local_index::Layout::Contiguous; 16],
        root_layout: root::Layout::Header50893V2,
        limits: local_store::Limits::default(),
    };
    match build {
        70374 => LocalStore::open_db2_70374(f.path(), options),
        70425 => LocalStore::open_db2_70425(f.path(), options),
        _ => unreachable!(),
    }
    .unwrap()
}
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Private(PathBuf);
impl Private {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "rce-compatible-world-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::DirBuilder::new().mode(0o700).create(&p).unwrap();
        Self(p)
    }
    fn spool(&self, f: &CollisionFixture, name: &str, profile: Profile) -> Spool {
        Spool::create_with_policy(
            &self.0.join(name),
            ProtectedInputs::capture(f.path(), &[]).unwrap(),
            profile.spool_limits(),
            spool::Profile::SameMountV1,
            profile,
        )
        .unwrap()
    }
}
impl Drop for Private {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
struct Sink {
    drops: Rc<Cell<usize>>,
    mutate: Option<PathBuf>,
    chmod: Option<PathBuf>,
}
impl Sink {
    fn new(drops: &Rc<Cell<usize>>) -> Self {
        Self {
            drops: drops.clone(),
            mutate: None,
            chmod: None,
        }
    }
}
impl placements::Sink for Sink {
    type Error = u8;
    fn stage(&mut self, _: placements::Staged<'_>) -> Result<(), u8> {
        Ok(())
    }
}
impl Drop for Sink {
    fn drop(&mut self) {
        self.drops.set(self.drops.get() + 1);
        if let Some(p) = &self.mutate {
            fs::write(p, b"synthetic-source-changed-on-drop").unwrap();
        }
        if let Some(p) = &self.chmod {
            fs::set_permissions(p, fs::Permissions::from_mode(0o755)).unwrap();
        }
    }
}
fn start<'s>(
    store: &'s mut LocalStore,
    build: u32,
    p: Spool,
    native: bool,
) -> startup::Startup<'s> {
    let l = p.resources().startup_limits();
    match (build, native) {
        (70374, false) => startup::extract_startup_fdid_70374(store, l, p),
        (70425, false) => startup::extract_startup_fdid_70425(store, l, p),
        (70374, true) => startup::extract_startup_fdid_native_gnu15_70374(store, l, p),
        (70425, true) => startup::extract_startup_fdid_native_gnu15_70425(store, l, p),
        _ => unreachable!(),
    }
    .unwrap()
}
fn handle(value: super::super::Outcome) -> crate::collision::ModelHandle {
    match value {
        super::super::Outcome::Present(h) => h,
        _ => panic!("expected synthetic committed model"),
    }
}
fn chunk(tag: &[u8; 4], body: &[u8]) -> Vec<u8> {
    let mut out: Vec<_> = tag.iter().rev().copied().collect();
    out.extend((body.len() as u32).to_le_bytes());
    out.extend(body);
    out
}
fn two_children(group: u32) -> Vec<u8> {
    let mut header = [0; 64];
    put(&mut header, 4, 1);
    header[60] = 4;
    let mut out = chunk(b"MVER", &17u32.to_le_bytes());
    out.extend(chunk(b"MOHD", &header));
    out.extend(chunk(b"GFID", &group.to_le_bytes()));
    out.extend(chunk(
        b"MODI",
        &[92u32.to_le_bytes(), 93u32.to_le_bytes()].concat(),
    ));
    let mut spawns = [0; 80];
    for i in [0, 40] {
        put(&mut spawns, i, (i / 40) as u32);
        put(&mut spawns, i + 28, 1f32.to_bits());
        put(&mut spawns, i + 32, 1f32.to_bits());
    }
    out.extend(chunk(b"MODD", &spawns));
    let mut set = [0; 32];
    put(&mut set, 24, 2);
    out.extend(chunk(b"MODS", &set));
    out
}
fn two_references() -> Vec<u8> {
    let mut b = fixture::wmo_group(true, true, false);
    let n = b.len();
    put(&mut b, n - 6, 4);
    b.extend(1u16.to_le_bytes());
    let n = b.len();
    put(&mut b, 4, (n - 8) as u32);
    b
}

#[test]
fn both_targets_prepare_authentic_map_complete_liquids_and_topology() {
    for build in [70374, 70425] {
        let f = CollisionFixture::for_build(&files(build, &[(1, 100, -1), (2, 101, 1)]), build);
        let p = Private::new();
        let mut store = open(&f, build);
        let drops = Rc::new(Cell::new(0));
        let c = start(
            &mut store,
            build,
            p.spool(&f, "payload", Profile::BoundedV1),
            true,
        );
        let mut world = prepare(c, Sink::new(&drops), 2048, Limits::default()).unwrap();
        assert_eq!(
            world.profile(),
            if build == 70374 {
                PROFILE_GNU15_70374
            } else {
                PROFILE_GNU15_70425
            }
        );
        assert_eq!(
            world.map().unwrap().file_data_id(),
            strict_world::MAP_FILE_DATA_ID
        );
        assert_eq!(
            world.liquids().unwrap()[0].source().file_data_id(),
            strict_world::MATERIAL_FILE_DATA_ID
        );
        assert_eq!(
            world.liquids().unwrap()[1].source().file_data_id(),
            strict_world::TYPE_FILE_DATA_ID
        );
        assert_eq!(
            world.topology().unwrap().get(2).unwrap().parent_map_id(),
            Some(1)
        );
        assert_eq!(world.facts().unwrap().reads, 4);
        assert_eq!(world.facts().unwrap().build, build);
        world
            .with_context(|context, maps, topology, _| {
                assert_eq!(maps.stats().rows, 2);
                assert_eq!(topology.all().len(), 2);
                assert_eq!(
                    context.collector_mut().inner.inner.lookup.material_id(22),
                    Some(7)
                );
                assert_eq!(
                    context.collector_mut().inner.inner.lookup.material_flags(7),
                    Some(2)
                );
                Ok(())
            })
            .unwrap();
        let (mut done, topology) = world.finish_preparation().unwrap();
        assert_eq!(drops.get(), 1);
        assert_eq!(topology.all().len(), 2);
        assert_eq!(
            done.data().unwrap().table().file_data_id(),
            gameobjects::TABLE_FILE_DATA_ID
        );
        done.extraction_mut().verify_unchanged().unwrap();
    }
}
#[test]
fn old_startup_and_world_profiles_stay_v1() {
    for build in [70374, 70425] {
        let f = CollisionFixture::for_build(&files(build, &[]), build);
        let p = Private::new();
        let mut store = open(&f, build);
        let drops = Rc::new(Cell::new(0));
        let c = start(
            &mut store,
            build,
            p.spool(&f, "payload", Profile::BoundedV1),
            false,
        );
        assert_eq!(
            c.profile(),
            if build == 70374 {
                startup::PROFILE_70374
            } else {
                startup::PROFILE_70425
            }
        );
        let world = prepare(c, Sink::new(&drops), 0, Limits::default()).unwrap();
        assert_eq!(
            world.profile(),
            if build == 70374 {
                PROFILE_70374
            } else {
                PROFILE_70425
            }
        );
        world.finish_preparation().unwrap();
    }
}
#[test]
fn cached_startup_wmo_preserves_native_order_and_handles_through_lookup() {
    for build in [70374, 70425] {
        let mut fs = files(build, &[]);
        godi(&mut fs, build, &[(1, 90)]);
        fs.extend([
            (90, None, two_children(91)),
            (91, None, two_references()),
            (92, None, fixture::m2(true)),
            (93, None, fixture::m2(true)),
            (94, None, two_children(95)),
            (95, None, two_references()),
        ]);
        let f = CollisionFixture::for_build(&fs, build);
        let p = Private::new();
        let mut store = open(&f, build);
        let drops = Rc::new(Cell::new(0));
        let mut c = start(
            &mut store,
            build,
            p.spool(&f, "payload", Profile::BoundedV1),
            true,
        );
        let h = handle(
            c.collector_mut()
                .extract(ModelRequest::Wmo(InputRef::FileDataId(90)))
                .unwrap(),
        );
        let before = c
            .collector_mut()
            .with_payload(h, |v| Ok(v.raw().unwrap().to_vec()))
            .unwrap();
        let mut w = prepare(c, Sink::new(&drops), 4096, Limits::default()).unwrap();
        let reads = w.facts().unwrap().reads;
        w.with_context(|c, _, _, _| {
            assert_eq!(
                handle(
                    c.collector_mut()
                        .extract(ModelRequest::Wmo(InputRef::FileDataId(90)))?
                ),
                h
            );
            assert_eq!(c.collector_mut().facts()?.reads, reads);
            c.collector_mut().with_payload(h, |v| {
                assert_eq!(v.profile(), payload::COMPATIBLE_GNU15_PROFILE);
                assert_eq!(v.raw().unwrap(), before);
                assert_eq!(v.wmo().unwrap().references().collect::<Vec<_>>(), [1, 0]);
                Ok(())
            })?;
            let later = handle(
                c.collector_mut()
                    .extract(ModelRequest::Wmo(InputRef::FileDataId(94)))?,
            );
            assert_ne!(later, h);
            Ok(())
        })
        .unwrap();
        assert_eq!(w.facts().unwrap().reads, reads + 2);
        w.with_context(|c, _, _, _| {
            assert_eq!(
                handle(
                    c.collector_mut()
                        .extract(ModelRequest::Wmo(InputRef::FileDataId(90)))?
                ),
                h
            );
            Ok(())
        })
        .unwrap();
        let (mut done, _) = w.finish_preparation().unwrap();
        done.extraction_mut()
            .with_payload(h, |v| {
                assert_eq!(v.wmo().unwrap().references().collect::<Vec<_>>(), [1, 0]);
                Ok(())
            })
            .unwrap();
    }
}
#[test]
fn committed_orphan_remains_enumerable_after_lookup_and_finish() {
    for build in [70374, 70425] {
        let mut fs = files(build, &[]);
        godi(&mut fs, build, &[(1, 90)]);
        fs.extend([
            (90, None, fixture::wmo_root(91, Some(92))),
            (92, None, fixture::m2(true)),
        ]);
        let f = CollisionFixture::for_build(&fs, build);
        let p = Private::new();
        let mut store = open(&f, build);
        let drops = Rc::new(Cell::new(0));
        let c = start(
            &mut store,
            build,
            p.spool(&f, "payload", Profile::BoundedV1),
            true,
        );
        assert!(matches!(
            c.data().unwrap().rows()[0].disposition,
            startup::Disposition::Omitted(_)
        ));
        let mut w = prepare(c, Sink::new(&drops), 0, Limits::default()).unwrap();
        let before = w.facts().unwrap();
        let h = w
            .with_context(|c, _, _, _| {
                let h = c.collector_mut().model_handle(0)?;
                assert!(matches!(
                    c.collector_mut().descriptor(h)?.origin(),
                    InputRef::FileDataId(92)
                ));
                Ok(h)
            })
            .unwrap();
        assert_eq!(w.facts().unwrap(), before);
        let (mut done, _) = w.finish_preparation().unwrap();
        assert_eq!(done.extraction_mut().model_handle(0).unwrap(), h);
    }
}
#[test]
fn explicit70425_liquid_hash_failure_stays_fatal() {
    let mut fs = files(70425, &[]);
    put(
        &mut fs
            .iter_mut()
            .find(|f| f.0 == strict_world::MATERIAL_FILE_DATA_ID)
            .unwrap()
            .2,
        152,
        0,
    );
    let f = CollisionFixture::for_build(&fs, 70425);
    let p = Private::new();
    let mut store = open(&f, 70425);
    let drops = Rc::new(Cell::new(0));
    let c = start(
        &mut store,
        70425,
        p.spool(&f, "payload", Profile::BoundedV1),
        true,
    );
    assert!(prepare(c, Sink::new(&drops), 0, Limits::default()).is_err());
    assert_eq!(drops.get(), 1);
}
#[test]
fn missing_complete_liquid_is_not_a_model_omission() {
    let fs: Vec<_> = files(70374, &[])
        .into_iter()
        .filter(|f| f.0 != strict_world::TYPE_FILE_DATA_ID)
        .collect();
    let f = CollisionFixture::for_build(&fs, 70374);
    let p = Private::new();
    let mut store = open(&f, 70374);
    let drops = Rc::new(Cell::new(0));
    let c = start(
        &mut store,
        70374,
        p.spool(&f, "payload", Profile::BoundedV1),
        true,
    );
    assert!(prepare(c, Sink::new(&drops), 0, Limits::default()).is_err());
    assert_eq!(drops.get(), 1);
}
#[test]
fn complete_graph_cycle_rejects_before_world_callback() {
    let f = CollisionFixture::for_build(&files(70374, &[(1, 100, 2), (2, 101, 1)]), 70374);
    let p = Private::new();
    let mut store = open(&f, 70374);
    let drops = Rc::new(Cell::new(0));
    let c = start(
        &mut store,
        70374,
        p.spool(&f, "payload", Profile::BoundedV1),
        true,
    );
    assert!(prepare(c, Sink::new(&drops), 0, Limits::default()).is_err());
    assert_eq!(drops.get(), 1);
}
#[test]
fn callbacks_retain_reservations_and_do_not_reset_source_counters() {
    let f = CollisionFixture::for_build(&files(70374, &[(1, 100, -1)]), 70374);
    let p = Private::new();
    let mut store = open(&f, 70374);
    let drops = Rc::new(Cell::new(0));
    let c = start(
        &mut store,
        70374,
        p.spool(&f, "payload", Profile::BoundedV1),
        true,
    );
    let mut w = prepare(c, Sink::new(&drops), 8192, Limits::default()).unwrap();
    let before = w.retained_bytes().unwrap();
    let facts = w.facts().unwrap();
    w.with_context(|c, maps, _, _| {
        assert!(c.collector_mut().retained_bytes()? >= before + maps.stats().resident_bytes);
        assert_eq!(c.collector_mut().facts()?, facts);
        Ok(())
    })
    .unwrap();
    assert_eq!(w.retained_bytes().unwrap(), before);
    assert_eq!(w.facts().unwrap(), facts);
    assert_eq!(drops.get(), 0);
    w.finish_preparation().unwrap();
    assert_eq!(drops.get(), 1);
}
#[test]
fn downward_table_and_sink_budgets_reject_without_completion() {
    for sink_bytes in [0, usize::MAX] {
        let f = CollisionFixture::for_build(&files(70374, &[(1, 100, -1)]), 70374);
        let p = Private::new();
        let mut store = open(&f, 70374);
        let drops = Rc::new(Cell::new(0));
        let c = start(
            &mut store,
            70374,
            p.spool(&f, "payload", Profile::BoundedV1),
            true,
        );
        let l = Limits {
            max_table_memory_bytes: 0,
            ..Limits::default()
        };
        assert!(prepare(c, Sink::new(&drops), sink_bytes, l).is_err());
        assert_eq!(drops.get(), 1);
    }
}
#[test]
fn callback_error_drops_sink_then_source_changed_takes_priority() {
    let f = CollisionFixture::for_build(&files(70374, &[]), 70374);
    let p = Private::new();
    let mut store = open(&f, 70374);
    let drops = Rc::new(Cell::new(0));
    let mut sink = Sink::new(&drops);
    sink.mutate = Some(f.path().join(".build.info"));
    let c = start(
        &mut store,
        70374,
        p.spool(&f, "payload", Profile::BoundedV1),
        true,
    );
    let mut w = prepare(c, sink, 0, Limits::default()).unwrap();
    let e = w
        .with_context::<()>(|_, _, _, _| Err(available::Error::State.into()))
        .unwrap_err();
    assert_eq!(drops.get(), 1);
    assert_eq!(e.code(), local_store::Error::InputChanged.code());
    assert_eq!(w.verify_unchanged().unwrap_err().code(), e.code());
}
#[test]
fn callback_error_drops_sink_then_spool_changed_takes_priority() {
    let f = CollisionFixture::for_build(&files(70374, &[]), 70374);
    let p = Private::new();
    let mut store = open(&f, 70374);
    let drops = Rc::new(Cell::new(0));
    let mut sink = Sink::new(&drops);
    sink.chmod = Some(p.0.join("payload"));
    let c = start(
        &mut store,
        70374,
        p.spool(&f, "payload", Profile::BoundedV1),
        true,
    );
    let mut w = prepare(c, sink, 0, Limits::default()).unwrap();
    let e = w
        .with_context::<()>(|_, _, _, _| Err(available::Error::State.into()))
        .unwrap_err();
    assert_eq!(e.code(), payload::Error::Changed.code());
    assert_eq!(drops.get(), 1);
}
#[test]
fn callback_guard_rejects_mutated_spool_without_retry() {
    let f = CollisionFixture::for_build(&files(70374, &[]), 70374);
    let p = Private::new();
    let mut store = open(&f, 70374);
    let drops = Rc::new(Cell::new(0));
    let c = start(
        &mut store,
        70374,
        p.spool(&f, "payload", Profile::BoundedV1),
        true,
    );
    let mut w = prepare(c, Sink::new(&drops), 0, Limits::default()).unwrap();
    let e = w
        .with_context(|_, _, _, _| {
            fs::set_permissions(p.0.join("payload"), fs::Permissions::from_mode(0o755)).unwrap();
            Ok(())
        })
        .unwrap_err();
    assert_eq!(e.code(), payload::Error::Changed.code());
    assert!(w.with_context(|_, _, _, _| Ok(())).is_err());
    assert_eq!(drops.get(), 1);
}
#[test]
fn sink_drop_during_successful_finish_is_guarded_before_completion() {
    let f = CollisionFixture::for_build(&files(70374, &[]), 70374);
    let p = Private::new();
    let mut store = open(&f, 70374);
    let drops = Rc::new(Cell::new(0));
    let mut sink = Sink::new(&drops);
    sink.mutate = Some(f.path().join(".build.info"));
    let c = start(
        &mut store,
        70374,
        p.spool(&f, "payload", Profile::BoundedV1),
        true,
    );
    let w = prepare(c, sink, 0, Limits::default()).unwrap();
    let e = match w.finish_preparation() {
        Err(e) => e,
        Ok(_) => panic!("changed source cannot complete"),
    };
    assert_eq!(e.code(), local_store::Error::InputChanged.code());
    assert_eq!(drops.get(), 1);
}
#[test]
fn early_drop_destroys_owned_sink_once_and_preserves_source_change() {
    let f = CollisionFixture::for_build(&files(70374, &[]), 70374);
    let p = Private::new();
    let mut store = open(&f, 70374);
    let drops = Rc::new(Cell::new(0));
    let mut sink = Sink::new(&drops);
    sink.mutate = Some(f.path().join(".build.info"));
    let c = start(
        &mut store,
        70374,
        p.spool(&f, "payload", Profile::BoundedV1),
        true,
    );
    let w = prepare(c, sink, 0, Limits::default()).unwrap();
    drop(w);
    assert_eq!(drops.get(), 1);
    assert_eq!(
        store.verify_unchanged().unwrap_err(),
        local_store::Error::InputChanged
    );
}
#[test]
fn failed_lookup_transition_returns_the_original_compatible_owner() {
    let f = CollisionFixture::for_build(&files(70374, &[]), 70374);
    let p = Private::new();
    let mut store = open(&f, 70374);
    let mut c = start(
        &mut store,
        70374,
        p.spool(&f, "payload", Profile::BoundedV1),
        true,
    );
    let count = c.collector_mut().request_calls();
    fs::set_permissions(p.0.join("payload"), fs::Permissions::from_mode(0o755)).unwrap();
    match c.try_with_lookup(&startup::EMPTY) {
        payload::LookupTransition::Ready(_) => panic!("changed spool cannot transition"),
        payload::LookupTransition::Failed(mut original, e) => {
            assert_eq!(e.code(), payload::Error::Changed.code());
            assert_eq!(original.collector_mut().request_calls(), count);
            assert_eq!(original.profile(), startup::PROFILE_GNU15_70374);
        }
    }
}
#[test]
fn wrong_target_startup_rejects_without_relabelling_the_store() {
    let f = CollisionFixture::for_build(&files(70374, &[]), 70374);
    let p = Private::new();
    let mut store = open(&f, 70374);
    assert!(
        startup::extract_startup_fdid_native_gnu15_70425(
            &mut store,
            gameobjects::Limits::default(),
            p.spool(&f, "payload", Profile::BoundedV1)
        )
        .is_err()
    );
    store.verify_unchanged().unwrap();
}
#[test]
fn disk_world_profile_is_retained_without_changing_any_ceiling() {
    let f = CollisionFixture::for_build(&files(70425, &[]), 70425);
    let p = Private::new();
    let mut store = open(&f, 70425);
    let drops = Rc::new(Cell::new(0));
    let c = start(
        &mut store,
        70425,
        p.spool(&f, "payload", Profile::DiskWorldV1),
        true,
    );
    let mut w = prepare(c, Sink::new(&drops), 1024, Limits::default()).unwrap();
    assert_eq!(
        w.memory_ceiling().unwrap(),
        Profile::DiskWorldV1.collector_limits().max_memory_bytes
    );
    let (mut done, _) = w.finish_preparation().unwrap();
    assert_eq!(done.extraction_mut().resources(), Profile::DiskWorldV1);
}

#[test]
fn failed_restore_returns_original_native_owner_with_negative_cache() {
    struct Lookup;
    impl LiquidLookup for Lookup {
        fn material_id(&self, _: u32) -> Option<u8> {
            Some(7)
        }
        fn material_flags(&self, _: u8) -> Option<u32> {
            Some(2)
        }
    }
    let mut rows = files(70374, &[]);
    godi(&mut rows, 70374, &[(1, 99)]);
    let f = CollisionFixture::for_build(&rows, 70374);
    let p = Private::new();
    let mut store = open(&f, 70374);
    let mut c = start(
        &mut store,
        70374,
        p.spool(&f, "payload", Profile::BoundedV1),
        true,
    );
    // An unclassified missing GODI row cannot create a typed model identity.
    assert_eq!(c.collector_mut().state.entries.len(), 0);
    let omitted = super::super::Outcome::Omitted(super::super::Omission::MissingRoot);
    assert_eq!(
        c.collector_mut()
            .extract(ModelRequest::M2(InputRef::FileDataId(99)))
            .unwrap(),
        omitted
    );
    let reads = c.collector_mut().facts().unwrap().reads;
    let requests_before_lookup = c.collector_mut().request_calls();
    let lookup = Lookup;
    let mut c = match c.try_with_lookup(&lookup) {
        payload::LookupTransition::Ready(c) => c,
        payload::LookupTransition::Failed(_, _) => panic!("unchanged lookup transition"),
    };
    assert_eq!(
        c.collector_mut()
            .extract(ModelRequest::M2(InputRef::FileDataId(99)))
            .unwrap(),
        omitted
    );
    assert_eq!(c.collector_mut().facts().unwrap().reads, reads);
    assert_eq!(
        c.collector_mut().request_calls(),
        requests_before_lookup + 1
    );
    let identities = c.collector_mut().state.entries.len();
    let requests = c.collector_mut().request_calls();
    assert_eq!(identities, 1);
    fs::write(
        f.path().join(".build.info"),
        b"synthetic-change-before-restore",
    )
    .unwrap();
    match c.try_with_lookup(&startup::EMPTY) {
        payload::LookupTransition::Ready(_) => panic!("changed source cannot restore"),
        payload::LookupTransition::Failed(mut original, e) => {
            assert_eq!(e.code(), local_store::Error::InputChanged.code());
            assert_eq!(original.collector_mut().state.entries.len(), identities);
            assert_eq!(original.collector_mut().request_calls(), requests);
            assert!(original.collector_mut().native_order());
            assert_eq!(original.collector_mut().target(), Target::Build70374);
        }
    }
}
#[test]
fn selected_context_keeps_full_authenticated_topology_and_ancestors() {
    let f = CollisionFixture::for_build(
        &files(70374, &[(1, 100, -1), (2, 101, 1), (3, 102, -1)]),
        70374,
    );
    let p = Private::new();
    let mut store = open(&f, 70374);
    let drops = Rc::new(Cell::new(0));
    let c = start(
        &mut store,
        70374,
        p.spool(&f, "payload", Profile::BoundedV1),
        true,
    );
    let mut w = prepare_selected(
        c,
        Sink::new(&drops),
        0,
        plan::MapSelection::MapAndAncestors(2),
        Limits::default(),
    )
    .unwrap();
    assert_eq!(w.topology().unwrap().all().len(), 3);
    assert!(w.topology().unwrap().contains_selected(1));
    assert!(w.topology().unwrap().contains_selected(2));
    assert!(!w.topology().unwrap().contains_selected(3));
    w.with_context(|c, maps, topology, _| {
        assert_eq!(maps.all().len(), 3);
        assert_eq!(topology.selected_active().count(), 2);
        assert_eq!(c.collector_mut().facts()?.reads, 4);
        Ok(())
    })
    .unwrap();
    let (_, topology) = w.finish_preparation().unwrap();
    assert_eq!(topology.selection(), plan::MapSelection::MapAndAncestors(2));
}

struct CapturedDrop {
    drops: Rc<Cell<usize>>,
    path: PathBuf,
    spool: bool,
}
impl Drop for CapturedDrop {
    fn drop(&mut self) {
        self.drops.set(self.drops.get() + 1);
        if self.spool {
            fs::set_permissions(&self.path, fs::Permissions::from_mode(0o755)).unwrap();
        } else {
            fs::write(&self.path, b"synthetic-uncalled-capture-drop-change").unwrap();
        }
    }
}
fn uncalled_plan_capture(spool: bool) {
    let f = CollisionFixture::for_build(&files(70374, &[(1, 100, -1)]), 70374);
    let p = Private::new();
    let mut store = open(&f, 70374);
    let sink_drops = Rc::new(Cell::new(0));
    let capture_drops = Rc::new(Cell::new(0));
    let invoked = Rc::new(Cell::new(0));
    let c = start(
        &mut store,
        70374,
        p.spool(&f, "payload", Profile::BoundedV1),
        true,
    );
    let mut w = prepare(c, Sink::new(&sink_drops), 0, Limits::default()).unwrap();
    // Test-only downward bound after preparation: force the replan path to fail
    // before the FnOnce is invoked, without a race or production test hook.
    w.limits.maps.max_memory_bytes = 0;
    let capture = CapturedDrop {
        drops: capture_drops.clone(),
        path: if spool {
            p.0.join("payload")
        } else {
            f.path().join(".build.info")
        },
        spool,
    };
    let called = invoked.clone();
    let error = w
        .with_context(move |_, _, _, _| {
            called.set(called.get() + 1);
            drop(capture);
            Ok(())
        })
        .unwrap_err();
    assert_eq!(invoked.get(), 0);
    assert_eq!(capture_drops.get(), 1);
    assert_eq!(sink_drops.get(), 1);
    assert_eq!(
        error.code(),
        if spool {
            payload::Error::Changed.code()
        } else {
            local_store::Error::InputChanged.code()
        }
    );
    assert!(w.finish_preparation().is_err());
}
#[test]
fn uncalled_plan_callback_drop_source_change_wins_over_plan_limit() {
    uncalled_plan_capture(false);
}
#[test]
fn uncalled_plan_callback_drop_spool_change_wins_over_plan_limit() {
    uncalled_plan_capture(true);
}
fn uncalled_lookup_capture(spool: bool) {
    let f = CollisionFixture::for_build(&files(70374, &[]), 70374);
    let p = Private::new();
    let mut store = open(&f, 70374);
    let sink_drops = Rc::new(Cell::new(0));
    let capture_drops = Rc::new(Cell::new(0));
    let invoked = Rc::new(Cell::new(0));
    let mut c = start(
        &mut store,
        70374,
        p.spool(&f, "payload", Profile::BoundedV1),
        true,
    );
    // Exercise the identical uninvoked-transition cleanup seam directly: a
    // sticky failed owner is recoverable, and capture Drop changes a guard.
    // This asserts no racy claim about hitting a between-guards mutation point.
    let _ = c
        .collector_mut()
        .finish_external_operation::<(), Error>(Err(available::Error::State.into()));
    let mut original = match c.try_with_lookup(&startup::EMPTY) {
        payload::LookupTransition::Ready(_) => panic!("poisoned owner cannot transition"),
        payload::LookupTransition::Failed(original, _) => original,
    };
    let capture = CapturedDrop {
        drops: capture_drops.clone(),
        path: if spool {
            p.0.join("payload")
        } else {
            f.path().join(".build.info")
        },
        spool,
    };
    let called = invoked.clone();
    let mut callback = Some(move || {
        called.set(called.get() + 1);
        drop(capture);
    });
    let mut sink = Some(Sink::new(&sink_drops));
    let error = drop_callback_and_guard::<(), _, _, _>(
        &mut original,
        &mut sink,
        &mut callback,
        available::Error::State.into(),
    )
    .unwrap_err();
    assert!(callback.is_none());
    assert!(sink.is_none());
    assert_eq!(invoked.get(), 0);
    assert_eq!(capture_drops.get(), 1);
    assert_eq!(sink_drops.get(), 1);
    assert_eq!(
        error.code(),
        if spool {
            payload::Error::Changed.code()
        } else {
            local_store::Error::InputChanged.code()
        }
    );
    assert!(original.finish().is_err());
}
#[test]
fn uncalled_failed_lookup_capture_drop_source_change_has_priority() {
    uncalled_lookup_capture(false);
}
#[test]
fn uncalled_failed_lookup_capture_drop_spool_change_has_priority() {
    uncalled_lookup_capture(true);
}
#[test]
fn uncalled_capture_drop_source_change_wins_after_prior_spool_guard_failure() {
    let f = CollisionFixture::for_build(&files(70374, &[(1, 100, -1)]), 70374);
    let p = Private::new();
    let mut store = open(&f, 70374);
    let sink_drops = Rc::new(Cell::new(0));
    let capture_drops = Rc::new(Cell::new(0));
    let invoked = Rc::new(Cell::new(0));
    let c = start(
        &mut store,
        70374,
        p.spool(&f, "payload", Profile::BoundedV1),
        true,
    );
    let mut w = prepare(c, Sink::new(&sink_drops), 0, Limits::default()).unwrap();
    fs::set_permissions(p.0.join("payload"), fs::Permissions::from_mode(0o755)).unwrap();
    let capture = CapturedDrop {
        drops: capture_drops.clone(),
        path: f.path().join(".build.info"),
        spool: false,
    };
    let called = invoked.clone();
    let error = w
        .with_context(move |_, _, _, _| {
            called.set(1);
            drop(capture);
            Ok(())
        })
        .unwrap_err();
    assert_eq!(error.code(), local_store::Error::InputChanged.code());
    assert_eq!(invoked.get(), 0);
    assert_eq!(capture_drops.get(), 1);
    assert_eq!(sink_drops.get(), 1);
}
