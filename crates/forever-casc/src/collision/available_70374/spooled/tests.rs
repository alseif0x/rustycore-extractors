// SPDX-License-Identifier: GPL-3.0-or-later
// Authored synthetic-only; no test/program execution claimed by this source cut.
use super::*;
use crate::collision::spool::{self, ProtectedInputs};
use crate::collision::{self, resources::Profile, world::tests as fixture};
use crate::local_store::tests::{CollisionFile, CollisionFixture};
use crate::{Locale, local_index, local_store, root};
use forever_db2::catalog;
use std::{
    fs,
    os::unix::fs::{DirBuilderExt, PermissionsExt},
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

fn put(b: &mut [u8], at: usize, n: u32) {
    b[at..at + 4].copy_from_slice(&n.to_le_bytes());
}
fn word(b: &[u8], at: usize) -> usize {
    u32::from_le_bytes(b[at..at + 4].try_into().unwrap()) as usize
}
fn godi(rows: &[(u32, u32)], copies: &[(u32, u32)]) -> Vec<u8> {
    let mut b = fixture::gameobject_db2(rows, copies);
    let hash = catalog::resolve_for(catalog::WOWDBDEFS_70374_CATALOG_ID, "GameObjectDisplayInfo")
        .unwrap()
        .unwrap()
        .metadata()
        .table_hash
        .unwrap();
    put(&mut b, 152, hash);
    b
}
// Same explicit two-section numeric fixture shape as the existing available
// tests. Unknown plaintext is only an authored declaration, never decrypted.
fn unknown(b: Vec<u8>, copies: usize) -> Vec<u8> {
    let start = word(&b, 212);
    let rows = word(&b, 136);
    let size = word(&b, 144);
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
    put(&mut out, 280, copies as u32);
    out.extend(vec![0xa5; size]);
    out.extend(99u32.to_le_bytes());
    out.extend(vec![0xa5; copies * 8]);
    out
}
fn files(rows: &[(u32, u32)], copies: &[(u32, u32)]) -> Vec<CollisionFile<'static>> {
    vec![
        (gameobjects::TABLE_FILE_DATA_ID, None, godi(rows, copies)),
        (50, None, fixture::m2(true)),
    ]
}
fn open(f: &CollisionFixture) -> LocalStore {
    LocalStore::open_db2_70374(
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
fn hole_fixture(
    fs: &[CollisionFile<'_>],
    id: u32,
    hole: std::ops::Range<usize>,
) -> CollisionFixture {
    let f = CollisionFixture::with_available_holes(fs, &[(id, hole)]);
    let path = f.path().join(".build.info");
    let text = fs::read_to_string(&path).unwrap();
    fs::write(path, text.replace("1.60.1.70205", "1.60.1.70374")).unwrap();
    f
}
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Private {
    base: PathBuf,
    path: PathBuf,
}
impl Private {
    fn new() -> Self {
        let base = std::env::temp_dir().join(format!(
            "rce-available-spooled-startup-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::DirBuilder::new().mode(0o700).create(&base).unwrap();
        let path = base.join("spool");
        Self { base, path }
    }
    fn spool(&self, f: &CollisionFixture, profile: Profile) -> Spool {
        Spool::create_with_policy(
            &self.path,
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
        fs::remove_dir_all(&self.base).unwrap();
    }
}
fn perform(f: &CollisionFixture, p: &Private, profile: Profile) -> CompletedStartup {
    let mut store = open(f);
    extract_startup_fdid(&mut store, profile.startup_limits(), p.spool(f, profile))
        .unwrap()
        .finish()
        .unwrap()
}

#[test]
fn empty_available_startup_retains_headers_coverage_and_data_charge() {
    let f = CollisionFixture::for_build(&files(&[], &[]), 70374);
    let p = Private::new();
    let out = perform(&f, &p, Profile::BoundedV1);
    assert_eq!(out.profile(), PROFILE);
    let d = out.data().unwrap();
    assert!(d.table().full_ckey_verified);
    assert!(d.table().all_sections_admitted());
    assert_eq!(d.table().direct_rows, 0);
    assert_eq!(d.lists().temp_bytes(), b"VMAP04E\0");
    assert_eq!(d.stats().resident_bytes, 16);
    assert_eq!(out.extraction().retained_external_for_test(), 16);
    assert_eq!(out.extraction().stats().unwrap().spool.reloads, 0);
}
#[test]
fn complete_source_matches_strict_spooled_outcomes_lists_and_facts() {
    let f = CollisionFixture::for_build(&files(&[(7, 50), (8, 0), (9, 50)], &[(70, 7)]), 70374);
    let oldp = Private::new();
    let mut store = open(&f);
    let old = gameobjects::spooled::extract_startup_fdid(
        &mut store,
        gameobjects::Limits::default(),
        oldp.spool(&f, Profile::BoundedV1),
    )
    .unwrap()
    .finish()
    .unwrap();
    drop(store);
    let p = Private::new();
    let new = perform(&f, &p, Profile::BoundedV1);
    let a = old.data().unwrap();
    let b = new.data().unwrap();
    assert_eq!(a.outcomes().len(), b.outcomes().len());
    for (left, right) in a.outcomes().iter().zip(b.outcomes()) {
        assert_eq!(left.ordinal(), right.ordinal());
        assert_eq!(left.display_id(), right.display_id());
        assert_eq!(left.file_data_id(), right.file_data_id());
        assert_eq!(left.outcome(), right.outcome());
    }
    assert_eq!(a.lists().temp_bytes(), b.lists().temp_bytes());
    assert_eq!(a.lists().dtree_bytes(), b.lists().dtree_bytes());
    assert_eq!(a.startup_facts(), b.startup_facts());
    assert_eq!(old.extraction().facts(), new.extraction().facts());
    assert_eq!(b.stats().candidate_requests, 2);
    assert_eq!(b.stats().copy_records, 1);
    assert_eq!(new.extraction().facts().reads, 2);
}
#[test]
fn physical_duplicates_zero_and_copies_share_one_model_descriptor() {
    let f = CollisionFixture::for_build(&files(&[(10, 50), (11, 0), (10, 50)], &[(20, 10)]), 70374);
    let p = Private::new();
    let out = perform(&f, &p, Profile::BoundedV1);
    let d = out.data().unwrap();
    assert_eq!(
        d.outcomes()
            .iter()
            .map(|r| (r.ordinal(), r.display_id()))
            .collect::<Vec<_>>(),
        [(0, 10), (1, 11), (2, 10)]
    );
    assert_eq!(d.stats().collision_rows, 2);
    assert_eq!(d.stats().zero_fdids, 1);
    assert_eq!(out.extraction().facts().models, 1);
    assert_eq!(out.extraction().stats().unwrap().spool.reloads, 2);
}
#[test]
fn complete_ckey_does_not_admit_unknown_db2_section() {
    let mut fs = files(&[(1, 50)], &[]);
    fs[0].2 = unknown(fs[0].2.clone(), 2);
    let f = CollisionFixture::for_build(&fs, 70374);
    let p = Private::new();
    let out = perform(&f, &p, Profile::BoundedV1);
    let t = out.data().unwrap().table();
    assert!(t.full_ckey_verified);
    assert!(!t.encoded_key_authenticated);
    assert!(!t.all_sections_admitted());
    assert_eq!(
        (
            t.declared_direct_rows,
            t.direct_rows,
            t.declared_copy_records,
            t.copy_records
        ),
        (2, 1, 2, 0)
    );
    assert_eq!(out.data().unwrap().stats().candidate_requests, 1);
}
#[test]
fn encrypted_unknown_hole_keeps_partial_ekey_facts_and_direct_lists() {
    let mut fs = files(&[(1, 50), (2, 50)], &[(20, 1)]);
    fs[0].2 = unknown(fs[0].2.clone(), 1);
    let start = word(&fs[0].2, 252);
    let end = fs[0].2.len();
    let f = hole_fixture(&fs, gameobjects::TABLE_FILE_DATA_ID, start..end);
    let p = Private::new();
    let out = perform(&f, &p, Profile::BoundedV1);
    let t = out.data().unwrap().table();
    assert!(!t.full_ckey_verified);
    assert!(t.encoded_key_authenticated);
    assert_eq!(t.available_bytes, start as u64);
    assert_eq!(t.declared_bytes, end as u64);
    assert_eq!(t.work.strict_candidate_attempts, 1);
    assert_eq!(t.work.partial_candidate_attempts, 1);
    assert_eq!((t.direct_rows, t.copy_records), (2, 1));
    assert_eq!(out.extraction().facts().models, 1);
    let mut store = open(&f);
    let resident = super::super::extract_startup_fdid(&mut store, gameobjects::Limits::default())
        .unwrap()
        .finish()
        .unwrap();
    assert_eq!(
        resident.data().lists().temp_bytes(),
        out.data().unwrap().lists().temp_bytes()
    );
    assert_eq!(
        resident.data().lists().dtree_bytes(),
        out.data().unwrap().lists().dtree_bytes()
    );
    assert_eq!(resident.extraction().facts(), out.extraction().facts());
}
#[test]
fn known_record_hole_is_terminal() {
    let fs = files(&[(1, 50)], &[]);
    let start = word(&fs[0].2, 212);
    let f = hole_fixture(&fs, gameobjects::TABLE_FILE_DATA_ID, start..start + 52);
    let p = Private::new();
    let mut store = open(&f);
    assert!(
        extract_startup_fdid(
            &mut store,
            gameobjects::Limits::default(),
            p.spool(&f, Profile::BoundedV1)
        )
        .is_err()
    );
    store.verify_unchanged().unwrap();
}
#[test]
fn known_model_missing_rejects_without_completed_prefix() {
    let f = CollisionFixture::for_build(
        &[(gameobjects::TABLE_FILE_DATA_ID, None, godi(&[(1, 50)], &[]))],
        70374,
    );
    let p = Private::new();
    let mut store = open(&f);
    let e = extract_startup_fdid(
        &mut store,
        gameobjects::Limits::default(),
        p.spool(&f, Profile::BoundedV1),
    )
    .err()
    .unwrap();
    assert_eq!(e.code(), "file_unavailable");
    store.verify_unchanged().unwrap();
}
#[test]
fn no_collision_and_texture_remain_normal_omissions() {
    let fs = vec![
        (
            gameobjects::TABLE_FILE_DATA_ID,
            None,
            godi(&[(1, 50), (2, 51)], &[]),
        ),
        (50, None, fixture::m2(false)),
        (51, None, b"BLP2authored".to_vec()),
    ];
    let f = CollisionFixture::for_build(&fs, 70374);
    let p = Private::new();
    let out = perform(&f, &p, Profile::BoundedV1);
    let d = out.data().unwrap();
    assert_eq!(d.stats().m2_no_collision, 1);
    assert_eq!(d.stats().textures, 1);
    assert_eq!(d.stats().collision_rows, 0);
    assert_eq!(out.extraction().stats().unwrap().spool.reloads, 0);
}
#[test]
fn names_capability_and_cached_handles_survive_lookup_transition() {
    let f = CollisionFixture::for_build(&files(&[(1, 50), (2, 50)], &[]), 70374);
    let p = Private::new();
    let mut store = open(&f);
    store.enable_names(root::name::Limits::default()).unwrap();
    let mut context = extract_startup_fdid(
        &mut store,
        gameobjects::Limits::default(),
        p.spool(&f, Profile::BoundedV1),
    )
    .unwrap();
    let before = context.collector_mut().facts().unwrap();
    let outcome = context.data().unwrap().outcomes()[0].outcome();
    let mut context = context.with_lookup(&super::super::EMPTY).unwrap();
    let got = context.collector_mut().extract_gameobject_fdid(50).unwrap();
    if let (gameobjects::Outcome::Collision(handle), collision::GameObjectOutcome::Model(cached)) =
        (outcome, got)
    {
        assert_eq!(handle, cached);
    } else {
        panic!("expected cached model");
    }
    assert_eq!(context.collector_mut().facts().unwrap().reads, before.reads);
    let completed = context.finish().unwrap();
    assert_eq!(completed.extraction().facts().models, 1);
    assert!(store.names_enabled());
    store.verify_unchanged().unwrap();
}
#[test]
fn disk_world_existing_profile_admits_4097_repeated_candidates() {
    let rows = (0..4097).map(|n| (n, 50)).collect::<Vec<_>>();
    let f = CollisionFixture::for_build(&files(&rows, &[]), 70374);
    let p = Private::new();
    let out = perform(&f, &p, Profile::DiskWorldV1);
    assert_eq!(out.data().unwrap().stats().candidate_requests, 4097);
    assert_eq!(out.extraction().facts().models, 1);
    assert_eq!(out.extraction().stats().unwrap().spool.reloads, 4097);
    assert_eq!(out.extraction().resources(), Profile::DiskWorldV1);
}
#[test]
fn disk_world_downward_candidate_cap_is_terminal() {
    let rows = (0..4097).map(|n| (n, 50)).collect::<Vec<_>>();
    let f = CollisionFixture::for_build(&files(&rows, &[]), 70374);
    let p = Private::new();
    let mut store = open(&f);
    let mut limits = Profile::DiskWorldV1.startup_limits();
    limits.max_candidate_requests = 4096;
    assert_eq!(
        extract_startup_fdid(&mut store, limits, p.spool(&f, Profile::DiskWorldV1))
            .err()
            .unwrap()
            .code(),
        "gameobject_startup_policy_limit"
    );
}
#[test]
fn disk_world_limits_with_bounded_owner_reject_before_source_read() {
    let f = CollisionFixture::for_build(&files(&[], &[]), 70374);
    let p = Private::new();
    let mut store = open(&f);
    assert!(
        extract_startup_fdid(
            &mut store,
            Profile::DiskWorldV1.startup_limits(),
            p.spool(&f, Profile::BoundedV1)
        )
        .is_err()
    );
}
#[test]
fn decoder_allowance_only_reduces() {
    let f = CollisionFixture::for_build(&files(&[], &[]), 70374);
    let p = Private::new();
    let mut store = open(&f);
    let limits = gameobjects::Limits {
        max_decode_allowance_bytes: 0,
        ..gameobjects::Limits::default()
    };
    assert!(extract_startup_fdid(&mut store, limits, p.spool(&f, Profile::BoundedV1)).is_err());
}
#[test]
fn splitting_data_preserves_external_charge_for_reload() {
    let f = CollisionFixture::for_build(&files(&[(1, 50)], &[]), 70374);
    let p = Private::new();
    let out = perform(&f, &p, Profile::BoundedV1);
    let charged = out.data().unwrap().stats().resident_bytes;
    let (data, mut extraction) = out.into_parts();
    let handle = match data.outcomes()[0].outcome() {
        gameobjects::Outcome::Collision(h) => h,
        _ => panic!("model"),
    };
    drop(data);
    assert_eq!(extraction.retained_external_for_test(), charged);
    assert!(
        extraction
            .with_payload(handle, |p| Ok(p.raw().is_some()))
            .unwrap()
    );
    assert_eq!(extraction.retained_external_for_test(), charged);
}
#[test]
fn changed_source_rejects_consuming_finish() {
    let f = CollisionFixture::for_build(&files(&[(1, 50)], &[]), 70374);
    let p = Private::new();
    let mut store = open(&f);
    let context = extract_startup_fdid(
        &mut store,
        gameobjects::Limits::default(),
        p.spool(&f, Profile::BoundedV1),
    )
    .unwrap();
    f.change_metadata();
    assert_eq!(
        context.finish().err().unwrap().code(),
        "local_store_input_changed"
    );
}
#[test]
fn namespace_mutation_rejects_finish_and_never_exposes_completion() {
    let f = CollisionFixture::for_build(&files(&[(1, 50)], &[]), 70374);
    let p = Private::new();
    let mut store = open(&f);
    let context = extract_startup_fdid(
        &mut store,
        gameobjects::Limits::default(),
        p.spool(&f, Profile::BoundedV1),
    )
    .unwrap();
    // Guards bind directory object/mode and registered entries, not a closed
    // enumeration of unrelated sibling names. Mutate the captured spool root.
    fs::set_permissions(&p.path, fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(
        context.finish().err().unwrap().code(),
        "collision_payload_changed"
    );
}

#[test]
fn new_error_remains_copy_and_does_not_nest_a_second_collector_guard() {
    fn copy<T: Copy>() {}
    copy::<Error>();
    assert!(size_of::<Error>() < 128);
    let old = gameobjects::spooled::Error::from(gameobjects::Error::Limit);
    let new = Error::from(old);
    assert_eq!(new.guard(), old.guard());
    assert_eq!(new.code(), old.code());
}
#[test]
fn cached_wmo_raw_is_not_reconverted_by_lookup_transition() {
    struct Lookup;
    impl LiquidLookup for Lookup {
        fn material_id(&self, _: u32) -> Option<u8> {
            Some(7)
        }
        fn material_flags(&self, _: u8) -> Option<u32> {
            Some(0)
        }
    }
    let fs = vec![
        (gameobjects::TABLE_FILE_DATA_ID, None, godi(&[(1, 70)], &[])),
        (70, None, fixture::wmo_root(60, None)),
        (60, None, fixture::wmo_group(true, false, true)),
    ];
    let f = CollisionFixture::for_build(&fs, 70374);
    let p = Private::new();
    let mut store = open(&f);
    let mut c = extract_startup_fdid(
        &mut store,
        gameobjects::Limits::default(),
        p.spool(&f, Profile::BoundedV1),
    )
    .unwrap();
    let handle = match c.data().unwrap().outcomes()[0].outcome() {
        gameobjects::Outcome::Collision(h) => h,
        _ => panic!("WMO"),
    };
    let raw = c
        .collector_mut()
        .with_payload(handle, |p| Ok(p.raw().unwrap().to_vec()))
        .unwrap();
    let before = c.collector_mut().facts().unwrap();
    let lookup = Lookup;
    let mut c = c.with_lookup(&lookup).unwrap();
    assert!(
        matches!(c.collector_mut().extract_gameobject_fdid(70).unwrap(),collision::GameObjectOutcome::Model(h)if h==handle)
    );
    let after = c
        .collector_mut()
        .with_payload(handle, |p| Ok(p.raw().unwrap().to_vec()))
        .unwrap();
    assert_eq!(raw, after);
    assert_eq!(c.collector_mut().facts().unwrap().reads, before.reads);
    c.finish().unwrap();
}
#[test]
fn wmo_doodad_only_keeps_no_raw_and_no_list_reload() {
    let fs = vec![
        (gameobjects::TABLE_FILE_DATA_ID, None, godi(&[(1, 70)], &[])),
        (70, None, fixture::wmo_root(60, Some(50))),
        (50, None, fixture::m2(true)),
        (60, None, fixture::wmo_group(false, true, false)),
    ];
    let f = CollisionFixture::for_build(&fs, 70374);
    let p = Private::new();
    let out = perform(&f, &p, Profile::BoundedV1);
    assert_eq!(out.data().unwrap().stats().wmo_doodad_only, 1);
    assert_eq!(out.data().unwrap().stats().collision_rows, 0);
    let gameobjects::Outcome::WmoDoodadOnly(handle) = out.data().unwrap().outcomes()[0].outcome()
    else {
        panic!("expected valid doodad-only WMO");
    };
    assert_eq!(
        out.extraction().phase_shape(handle).unwrap().raw_bytes,
        None
    );
    assert_eq!(out.extraction().facts().reads, 4);
    assert_eq!(out.extraction().facts().dependency_requests, 1);
    assert_eq!(out.extraction().facts().models, 2);
    // Only the admitted collision M2 child contributes RAW; lists omit the WMO.
    assert!(out.extraction().facts().raw_bytes > 0);
    assert_eq!(out.extraction().stats().unwrap().spool.records, 2);
    assert_eq!(out.extraction().stats().unwrap().spool.reloads, 0);
}
