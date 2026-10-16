// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-16. Synthetic cases; unrun in this source proposal.
use super::*;
use crate::collision::{compatible as c, payload, spool};
use crate::{Locale, local_index};
use std::{
    fs,
    os::unix::fs::{DirBuilderExt, PermissionsExt},
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Private {
    base: PathBuf,
    path: PathBuf,
}
impl Private {
    fn new() -> Self {
        let base = std::env::temp_dir().join(format!(
            "rce-compatible-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::DirBuilder::new().mode(0o700).create(&base).unwrap();
        Self {
            path: base.join("spool"),
            base,
        }
    }
    fn spool(&self, fixture: &CollisionFixture) -> spool::Spool {
        spool::Spool::create(
            &self.path,
            spool::ProtectedInputs::capture(fixture.path(), &[]).unwrap(),
            spool::Limits::default(),
        )
        .unwrap()
    }
}
impl Drop for Private {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.base).unwrap();
    }
}
fn open(f: &CollisionFixture, build: u32) -> LocalStore {
    let options = local_store::OpenOptions {
        locale: Locale::EnUs,
        index_layouts: [local_index::Layout::Contiguous; 16],
        root_layout: crate::root::Layout::Header50893V2,
        limits: local_store::Limits::default(),
    };
    match build {
        70374 => LocalStore::open_db2_70374(f.path(), options),
        70425 => LocalStore::open_db2_70425(f.path(), options),
        _ => unreachable!(),
    }
    .unwrap()
}
fn present(outcome: c::Outcome) -> ModelHandle {
    match outcome {
        c::Outcome::Present(h) => h,
        other => panic!("unexpected outcome {other:?}"),
    }
}
fn godi(build: u32, rows: &[(u32, u32)]) -> Vec<u8> {
    let mut bytes = world::tests::gameobject_db2(rows, &[(100, rows[0].0)]);
    let catalog = if build == 70374 {
        forever_db2::catalog::WOWDBDEFS_70374_CATALOG_ID
    } else {
        forever_db2::catalog::WOWDBDEFS_70425_CATALOG_ID
    };
    let hash = forever_db2::catalog::resolve_for(catalog, "GameObjectDisplayInfo")
        .unwrap()
        .unwrap()
        .metadata()
        .table_hash
        .unwrap();
    put(&mut bytes, 152, hash);
    bytes
}
#[test]
fn compatible_missing_first_middle_last_preserves_successful_handles() {
    let f = CollisionFixture::for_build(&[(10, None, m2(true)), (20, None, m2(true))], 70374);
    let p = Private::new();
    let mut store = open(&f, 70374);
    let mut e =
        c::Collector::new_70374(&mut store, &Lookup, Limits::default(), p.spool(&f)).unwrap();
    assert_eq!(
        e.extract(request(9)).unwrap(),
        c::Outcome::Omitted(c::Omission::MissingRoot)
    );
    let a = present(e.extract(request(10)).unwrap());
    assert_eq!(
        e.extract(request(19)).unwrap(),
        c::Outcome::Omitted(c::Omission::MissingRoot)
    );
    let b = present(e.extract(request(20)).unwrap());
    assert_eq!(
        e.extract(request(29)).unwrap(),
        c::Outcome::Omitted(c::Omission::MissingRoot)
    );
    assert_eq!((a.index(), b.index()), (0, 1));
    assert_eq!(e.facts().unwrap().reads, 5);
    assert_eq!(present(e.extract(request(10)).unwrap()), a);
    assert_eq!(e.finish().unwrap().cached_identity_count(), 5);
}
#[test]
fn compatible_negative_hits_charge_requests_without_refunding_read_work() {
    let f = CollisionFixture::for_build(&[(10, None, m2(true))], 70374);
    let p = Private::new();
    let mut store = open(&f, 70374);
    let mut e = c::Collector::new_70374(
        &mut store,
        &Lookup,
        Limits {
            max_requests: 2,
            ..Limits::default()
        },
        p.spool(&f),
    )
    .unwrap();
    for _ in 0..2 {
        assert_eq!(
            e.extract(request(99)).unwrap(),
            c::Outcome::Omitted(c::Omission::MissingRoot)
        );
    }
    assert_eq!(e.request_calls(), 2);
    assert_eq!(e.facts().unwrap().reads, 1);
    assert_eq!(
        e.extract(request(10)),
        Err(payload::CollectorError::Collision(Error::Limit))
    );
    assert!(e.finish().is_err());
}
#[test]
fn compatible_child_survives_parent_missing_group_and_later_wmo() {
    let files = [
        (10, None, root(&[30, 31], &[20], true)),
        (11, None, root(&[30], &[20], true)),
        (20, None, m2(true)),
        (30, None, group(true, false, &[0])),
    ];
    let f = CollisionFixture::for_build(&files, 70374);
    let p = Private::new();
    let mut store = open(&f, 70374);
    let mut e =
        c::Collector::new_70374(&mut store, &Lookup, Limits::default(), p.spool(&f)).unwrap();
    assert_eq!(
        e.extract(wmo_request(10)).unwrap(),
        c::Outcome::Omitted(c::Omission::MissingRequiredGroup)
    );
    let before = e.facts().unwrap();
    let child = present(e.extract(request(20)).unwrap());
    assert_eq!(child.index(), 0);
    assert_eq!(e.facts().unwrap(), before);
    e.with_payload(child, |p| {
        assert!(p.raw().is_some());
        Ok(())
    })
    .unwrap();
    let parent = present(e.extract(wmo_request(11)).unwrap());
    assert_eq!(parent.index(), 1);
    assert_eq!(e.facts().unwrap().dependency_requests, 2);
    assert_eq!(
        e.facts().unwrap().source_bytes,
        files
            .iter()
            .filter(|f| f.0 != 20)
            .map(|f| f.2.len())
            .sum::<usize>()
            + m2(true).len()
            + group(true, false, &[0]).len()
    );
    let mut done = e.finish().unwrap();
    assert_eq!(done.facts().models, 2);
    done.with_payload(child, |p| {
        assert!(p.raw().is_some());
        Ok(())
    })
    .unwrap();
}
#[test]
fn compatible_missing_doodad_retains_geometry_and_ordered_binding() {
    let f = CollisionFixture::for_build(
        &[
            (10, None, root(&[30], &[20, 21], true)),
            (21, None, m2(true)),
            (30, None, group(true, false, &[0])),
        ],
        70374,
    );
    let p = Private::new();
    let mut store = open(&f, 70374);
    let mut e =
        c::Collector::new_70374(&mut store, &Lookup, Limits::default(), p.spool(&f)).unwrap();
    let parent = present(e.extract(wmo_request(10)).unwrap());
    e.with_payload(parent, |p| {
        assert!(p.raw().is_some());
        let w = p.wmo().unwrap();
        let bindings: Vec<_> = w.bindings().collect();
        assert_eq!(
            bindings[0],
            c::Binding {
                name_index: 0,
                disposition: c::Disposition::Missing
            }
        );
        assert!(matches!(
            bindings[1].disposition,
            c::Disposition::Present(_)
        ));
        assert_eq!(w.references().len(), 0);
        Ok(())
    })
    .unwrap();
    assert!(e.finish().is_ok());
}
#[test]
fn compatible_zero_geometry_surviving_child_is_doodad_only() {
    let f = CollisionFixture::for_build(
        &[
            (10, None, root(&[30], &[20], true)),
            (20, None, m2(true)),
            (30, None, group(false, false, &[0])),
        ],
        70374,
    );
    let p = Private::new();
    let mut store = open(&f, 70374);
    let mut e =
        c::Collector::new_70374(&mut store, &Lookup, Limits::default(), p.spool(&f)).unwrap();
    let h = present(e.extract(wmo_request(10)).unwrap());
    assert_eq!(e.descriptor(h).unwrap().status(), Status::DoodadOnly);
    assert!(e.finish().is_ok());
}
#[test]
fn compatible_zero_geometry_lost_usable_child_has_distinct_omission() {
    let f = CollisionFixture::for_build(
        &[
            (10, None, root(&[30], &[20], true)),
            (30, None, group(false, false, &[0])),
        ],
        70374,
    );
    let p = Private::new();
    let mut store = open(&f, 70374);
    let mut e =
        c::Collector::new_70374(&mut store, &Lookup, Limits::default(), p.spool(&f)).unwrap();
    assert_eq!(
        e.extract(wmo_request(10)).unwrap(),
        c::Outcome::Omitted(c::Omission::NoUsableCollisionAfterOmission)
    );
    assert_eq!(e.facts().unwrap().models, 0);
    assert!(e.finish().is_ok());
}
#[test]
fn compatible_intrinsic_empty_wmo_with_unreferenced_missing_child_still_fails() {
    let f = CollisionFixture::for_build(
        &[
            (10, None, root(&[30], &[20], false)),
            (30, None, group(false, false, &[])),
        ],
        70374,
    );
    let p = Private::new();
    let mut store = open(&f, 70374);
    let mut e =
        c::Collector::new_70374(&mut store, &Lookup, Limits::default(), p.spool(&f)).unwrap();
    assert_eq!(
        e.extract(wmo_request(10)),
        Err(payload::CollectorError::Collision(Error::NoCollision))
    );
    assert!(e.finish().is_err());
}
#[test]
fn compatible_unclassified_missing_has_no_class_identity() {
    let f = CollisionFixture::for_build(&[(10, None, m2(true))], 70374);
    let p = Private::new();
    let mut store = open(&f, 70374);
    let mut e =
        c::Collector::new_70374(&mut store, &Lookup, Limits::default(), p.spool(&f)).unwrap();
    assert_eq!(
        e.extract_gameobject_fdid(99).unwrap(),
        c::GameObjectOutcome::Omitted(c::Omission::MissingRoot)
    );
    assert_eq!(
        e.extract(wmo_request(99)).unwrap(),
        c::Outcome::Omitted(c::Omission::MissingRoot)
    );
    assert_eq!(e.facts().unwrap().reads, 2);
    assert_eq!(e.finish().unwrap().cached_identity_count(), 1);
}
#[test]
fn compatible_negative_class_alias_is_fatal() {
    let f = CollisionFixture::for_build(&[(10, None, m2(true))], 70374);
    let p = Private::new();
    let mut store = open(&f, 70374);
    let mut e =
        c::Collector::new_70374(&mut store, &Lookup, Limits::default(), p.spool(&f)).unwrap();
    e.extract(request(99)).unwrap();
    assert_eq!(
        e.extract(wmo_request(99)),
        Err(payload::CollectorError::Collision(Error::Alias))
    );
    assert!(e.finish().is_err());
}
#[test]
fn compatible_building_cycle_is_fatal_without_public_pending_handle() {
    let f = CollisionFixture::for_build(
        &[
            (10, None, root(&[30], &[10], true)),
            (30, None, group(true, false, &[0])),
        ],
        70374,
    );
    let p = Private::new();
    let mut store = open(&f, 70374);
    let mut e =
        c::Collector::new_70374(&mut store, &Lookup, Limits::default(), p.spool(&f)).unwrap();
    assert_eq!(
        e.extract(wmo_request(10)),
        Err(payload::CollectorError::Collision(Error::Cycle))
    );
    assert!(e.finish().is_err());
}
#[test]
fn compatible_invalid_sentinel_rejects_before_any_missing_read() {
    for id in [0, u32::MAX] {
        let f = CollisionFixture::for_build(&[(10, None, m2(true))], 70374);
        let p = Private::new();
        let mut store = open(&f, 70374);
        let mut e =
            c::Collector::new_70374(&mut store, &Lookup, Limits::default(), p.spool(&f)).unwrap();
        assert_eq!(
            e.extract(request(id)),
            Err(payload::CollectorError::Collision(Error::Name))
        );
        assert_eq!(e.inner.inner.reads, 0);
    }
}
#[test]
fn compatible_names_disabled_and_malformed_present_are_not_omissions() {
    let f = CollisionFixture::for_build(&[(10, None, b"MD20".to_vec())], 70374);
    let p = Private::new();
    let mut store = open(&f, 70374);
    let mut e =
        c::Collector::new_70374(&mut store, &Lookup, Limits::default(), p.spool(&f)).unwrap();
    assert_eq!(
        e.extract(ModelRequest::M2(InputRef::LogicalName(b"absent.m2"))),
        Err(payload::CollectorError::Collision(Error::Store(
            local_store::Error::NamesDisabled
        )))
    );
    let p2 = Private::new();
    let mut store = open(&f, 70374);
    let mut e =
        c::Collector::new_70374(&mut store, &Lookup, Limits::default(), p2.spool(&f)).unwrap();
    assert!(matches!(
        e.extract(request(10)),
        Err(payload::CollectorError::Collision(Error::M2(_)))
    ));
    assert!(e.finish().is_err());
}
#[test]
fn compatible_negative_identity_ceiling_and_read_ceiling_are_cumulative() {
    for limits in [
        Limits {
            max_models: 1,
            ..Limits::default()
        },
        Limits {
            max_reads: 1,
            ..Limits::default()
        },
    ] {
        let f = CollisionFixture::for_build(&[(10, None, m2(true))], 70374);
        let p = Private::new();
        let mut store = open(&f, 70374);
        let mut e = c::Collector::new_70374(&mut store, &Lookup, limits, p.spool(&f)).unwrap();
        e.extract(request(99)).unwrap();
        assert_eq!(
            e.extract(request(10)),
            Err(payload::CollectorError::Collision(Error::Limit))
        );
        assert!(e.finish().is_err());
    }
}
#[test]
fn compatible_negative_cache_store_guard_dominates_omission() {
    let f = CollisionFixture::for_build(&[(10, None, m2(true))], 70374);
    let p = Private::new();
    let mut store = open(&f, 70374);
    let mut e =
        c::Collector::new_70374(&mut store, &Lookup, Limits::default(), p.spool(&f)).unwrap();
    e.extract(request(99)).unwrap();
    f.change_metadata();
    assert_eq!(
        e.extract(request(99)).unwrap_err().code(),
        "local_store_input_changed"
    );
    assert!(e.finish().is_err());
}
#[test]
fn compatible_negative_cache_spool_guard_dominates_omission() {
    let f = CollisionFixture::for_build(&[(10, None, m2(true))], 70374);
    let p = Private::new();
    let mut store = open(&f, 70374);
    let mut e =
        c::Collector::new_70374(&mut store, &Lookup, Limits::default(), p.spool(&f)).unwrap();
    let h = present(e.extract(request(10)).unwrap());
    e.extract(request(99)).unwrap();
    let path = p.path.join("00000000.payload");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    fs::write(path, b"changed").unwrap();
    assert_eq!(
        e.extract(request(99)).unwrap_err(),
        payload::CollectorError::Payload(payload::Error::Changed)
    );
    assert!(e.with_payload(h, |_| Ok(())).is_err());
    assert!(e.finish().is_err());
}
#[test]
fn compatible_callback_failure_preserves_changed_guard_priority() {
    let f = CollisionFixture::for_build(&[(10, None, m2(true))], 70374);
    let p = Private::new();
    let mut store = open(&f, 70374);
    let mut e =
        c::Collector::new_70374(&mut store, &Lookup, Limits::default(), p.spool(&f)).unwrap();
    let h = present(e.extract(request(10)).unwrap());
    let err = e
        .with_payload(h, |_| {
            f.change_metadata();
            Err::<(), _>(payload::Error::Callback)
        })
        .unwrap_err();
    assert_eq!(err.code(), "local_store_input_changed");
    assert!(e.finish().is_err());
}
#[test]
fn compatible_lookup_transition_preserves_child_handle_and_magic() {
    let f = CollisionFixture::for_build(
        &[(10, None, root(&[30], &[20], true)), (20, None, m2(true))],
        70374,
    );
    let p = Private::new();
    let mut store = open(&f, 70374);
    let mut e =
        c::Collector::new_70374(&mut store, &Lookup, Limits::default(), p.spool(&f)).unwrap();
    e.extract(wmo_request(10)).unwrap();
    let h = present(e.extract(request(20)).unwrap());
    let facts = e.facts().unwrap();
    let mut e = e.with_lookup(&Lookup).unwrap();
    assert_eq!(
        e.extract_gameobject_fdid(20).unwrap(),
        c::GameObjectOutcome::Model(h)
    );
    assert_eq!(
        e.descriptor(h).unwrap().source().unwrap().first_magic(),
        Some(*b"MD20")
    );
    assert_eq!(e.facts().unwrap(), facts);
    assert!(e.finish().is_ok());
}
#[test]
fn compatible_closed_target_rejects_cross_build_before_model_reads() {
    for build in [70374, 70425] {
        let f = CollisionFixture::for_build(&[(10, None, m2(true))], build);
        let p = Private::new();
        let mut store = open(&f, build);
        let wrong = if build == 70374 {
            c::Collector::new_70425(&mut store, &Lookup, Limits::default(), p.spool(&f))
        } else {
            c::Collector::new_70374(&mut store, &Lookup, Limits::default(), p.spool(&f))
        };
        assert_eq!(
            wrong.err().unwrap().code(),
            "available_source_build_not_admitted"
        );
    }
}
#[test]
fn compatible_startup_keeps_physical_rows_duplicates_texture_and_missing() {
    for build in [70374, 70425] {
        let rows = [(1, 99), (2, 0), (3, 10), (4, 11), (5, 10), (6, 12)];
        let f = CollisionFixture::for_build(
            &[
                (gameobjects::TABLE_FILE_DATA_ID, None, godi(build, &rows)),
                (10, None, m2(true)),
                (11, None, b"BLP2toy".to_vec()),
                (12, None, m2(false)),
            ],
            build,
        );
        let p = Private::new();
        let mut store = open(&f, build);
        let context = if build == 70374 {
            c::startup::extract_startup_fdid_70374(
                &mut store,
                gameobjects::Limits::default(),
                p.spool(&f),
            )
        } else {
            c::startup::extract_startup_fdid_70425(
                &mut store,
                gameobjects::Limits::default(),
                p.spool(&f),
            )
        }
        .unwrap();
        let data = context.data().unwrap();
        assert_eq!(data.rows().len(), rows.len());
        assert_eq!(data.stats().collision_rows, 2);
        assert_eq!(data.stats().copy_records, 1);
        assert_eq!(data.stats().direct_rows, rows.len());
        assert!(matches!(
            data.rows()[0].disposition,
            c::startup::Disposition::Omitted(c::Omission::MissingRoot)
        ));
        assert_eq!(data.rows()[2].disposition, data.rows()[4].disposition);
        assert!(matches!(
            data.rows()[3].disposition,
            c::startup::Disposition::Normal(gameobjects::Outcome::Texture)
        ));
        assert!(context.finish().is_ok());
    }
}
#[test]
fn compatible_required_godi_missing_is_fatal() {
    let f = CollisionFixture::for_build(&[(10, None, m2(true))], 70425);
    let p = Private::new();
    let mut store = open(&f, 70425);
    assert!(
        c::startup::extract_startup_fdid_70425(
            &mut store,
            gameobjects::Limits::default(),
            p.spool(&f)
        )
        .is_err()
    );
}
#[test]
fn compatible_strict_collector_still_poisoned_on_missing() {
    let f = CollisionFixture::for_build(&[(10, None, m2(true))], 70374);
    let p = Private::new();
    let mut store = open(&f, 70374);
    let mut e =
        payload::SpoolingExtractor::new(&mut store, &Lookup, Limits::default(), p.spool(&f))
            .unwrap();
    assert_eq!(
        e.extract(request(99)),
        Err(payload::CollectorError::Collision(Error::Store(
            local_store::Error::MissingFile
        )))
    );
    assert!(e.extract(request(10)).is_err());
    assert!(e.finish().is_err());
}
#[test]
fn compatible_negative_named_alias_and_complete_class_alias_remain_fatal() {
    let f = CollisionFixture::for_build(&[(10, Some(&b"a\\thing.m2"[..]), m2(true))], 70374);
    let p = Private::new();
    let mut store = open(&f, 70374);
    store
        .enable_names(crate::root::name::Limits::default())
        .unwrap();
    let mut e =
        c::Collector::new_70374(&mut store, &Lookup, Limits::default(), p.spool(&f)).unwrap();
    e.extract(ModelRequest::M2(InputRef::LogicalName(b"missing\\item.m2")))
        .unwrap();
    assert_eq!(
        e.extract(ModelRequest::M2(InputRef::LogicalName(b"other\\item.m2"))),
        Err(payload::CollectorError::Collision(Error::Alias))
    );
    let p2 = Private::new();
    let mut store = open(&f, 70374);
    let mut e =
        c::Collector::new_70374(&mut store, &Lookup, Limits::default(), p2.spool(&f)).unwrap();
    e.extract(request(10)).unwrap();
    assert_eq!(
        e.extract(wmo_request(10)),
        Err(payload::CollectorError::Collision(Error::Alias))
    );
}
#[test]
fn compatible_negative_dependency_cache_still_charges_each_parent_occurrence() {
    let f = CollisionFixture::for_build(
        &[
            (10, None, root(&[30], &[20], true)),
            (11, None, root(&[30], &[20], true)),
            (30, None, group(true, false, &[0])),
        ],
        70374,
    );
    let p = Private::new();
    let mut store = open(&f, 70374);
    let mut e = c::Collector::new_70374(
        &mut store,
        &Lookup,
        Limits {
            max_dependency_requests: 1,
            ..Limits::default()
        },
        p.spool(&f),
    )
    .unwrap();
    e.extract(wmo_request(10)).unwrap();
    assert_eq!(e.facts().unwrap().dependency_requests, 1);
    assert_eq!(
        e.extract(wmo_request(11)),
        Err(payload::CollectorError::Collision(Error::Limit))
    );
    assert!(e.finish().is_err());
}
#[test]
fn compatible_identity_memory_admission_precedes_missing_acquisition() {
    let f = CollisionFixture::for_build(&[(10, None, m2(true))], 70374);
    let p = Private::new();
    let mut store = open(&f, 70374);
    let spool = p.spool(&f);
    let limits = Limits {
        max_memory_bytes: store.stats().resident_bytes + spool.resident_bytes().unwrap() + 1,
        ..Limits::default()
    };
    let mut e = c::Collector::new_70374(&mut store, &Lookup, limits, spool).unwrap();
    assert_eq!(
        e.extract(request(99)),
        Err(payload::CollectorError::Collision(Error::Limit))
    );
    assert_eq!(e.inner.inner.reads, 0);
}
#[test]
fn compatible_authenticated_model_hole_is_fatal_not_missing() {
    let f = CollisionFixture::with_available_holes(&[(10, None, m2(true))], &[(10, 4..8)]);
    let path = f.path().join(".build.info");
    let text = fs::read_to_string(&path).unwrap();
    fs::write(path, text.replace("1.60.1.70205", "1.60.1.70374")).unwrap();
    let p = Private::new();
    let mut store = open(&f, 70374);
    let mut e =
        c::Collector::new_70374(&mut store, &Lookup, Limits::default(), p.spool(&f)).unwrap();
    assert!(e.extract(request(10)).is_err());
    assert!(e.finish().is_err());
}
fn unknown_section(b: Vec<u8>) -> (Vec<u8>, std::ops::Range<usize>) {
    let start = u32::from_le_bytes(b[212..216].try_into().unwrap()) as usize;
    let rows = u32::from_le_bytes(b[136..140].try_into().unwrap());
    let size = u32::from_le_bytes(b[144..148].try_into().unwrap()) as usize;
    let mut out = b[..244].to_vec();
    out.extend([0; 40]);
    out.extend(&b[244..start]);
    out.extend(0u32.to_le_bytes());
    out.extend(&b[start..]);
    put(&mut out, 136, rows + 1);
    put(&mut out, 200, 2);
    put(&mut out, 212, (start + 44) as u32);
    let next = out.len();
    out[244..252].copy_from_slice(&1u64.to_le_bytes());
    put(&mut out, 252, next as u32);
    put(&mut out, 256, 1);
    put(&mut out, 268, 4);
    out.extend(vec![0xa5; size]);
    out.extend(99u32.to_le_bytes());
    (out, next..next + size)
}
#[test]
fn compatible_available_godi_retains_unknown_section_facts_separately() {
    let (db2, hole) = unknown_section(godi(70425, &[(1, 10)]));
    let f = CollisionFixture::with_available_holes(
        &[
            (gameobjects::TABLE_FILE_DATA_ID, None, db2),
            (10, None, m2(true)),
        ],
        &[(gameobjects::TABLE_FILE_DATA_ID, hole)],
    );
    let path = f.path().join(".build.info");
    let text = fs::read_to_string(&path).unwrap();
    fs::write(path, text.replace("1.60.1.70205", "1.60.1.70425")).unwrap();
    let p = Private::new();
    let mut store = open(&f, 70425);
    let context = c::startup::extract_startup_fdid_70425(
        &mut store,
        gameobjects::Limits::default(),
        p.spool(&f),
    )
    .unwrap();
    let data = context.data().unwrap();
    assert!(!data.table().full_ckey_verified);
    assert!(!data.table().all_sections_admitted());
    assert_eq!(data.table().direct_rows, 1);
    assert_eq!(data.rows().len(), 1);
    assert!(context.finish().is_ok());
}

#[test]
fn compatible_indexed_handles_export_orphan_child_without_later_request() {
    let f = CollisionFixture::for_build(
        &[(10, None, root(&[30], &[20], true)), (20, None, m2(true))],
        70374,
    );
    let p = Private::new();
    let mut store = open(&f, 70374);
    let mut e =
        c::Collector::new_70374(&mut store, &Lookup, Limits::default(), p.spool(&f)).unwrap();
    assert_eq!(
        e.extract(wmo_request(10)).unwrap(),
        c::Outcome::Omitted(c::Omission::MissingRequiredGroup)
    );
    let before = e.facts().unwrap();
    let calls = e.request_calls();
    assert_eq!(before.models, 1);
    let child = e.model_handle(0).unwrap();
    assert_eq!(child.index(), 0);
    assert!(matches!(
        e.descriptor(child).unwrap().origin(),
        InputRef::FileDataId(20)
    ));
    assert_eq!(
        e.descriptor(child)
            .unwrap()
            .source()
            .unwrap()
            .file_data_id(),
        20
    );
    let bytes = e
        .with_payload(child, |p| Ok(p.raw().unwrap().to_vec()))
        .unwrap();
    for _ in 0..8 {
        assert_eq!(e.model_handle(0).unwrap(), child);
    }
    assert_eq!(e.facts().unwrap(), before);
    assert_eq!(e.request_calls(), calls);
    let e = e.with_lookup(&Lookup).unwrap();
    assert!(matches!(
        e.descriptor(child).unwrap().origin(),
        InputRef::FileDataId(20)
    ));
    assert_eq!(e.facts().unwrap(), before);
    let mut done = e.finish().unwrap();
    assert_eq!(done.model_handle(0).unwrap(), child);
    assert_eq!(done.descriptor(child).unwrap().class(), Class::M2);
    assert!(matches!(
        done.descriptor(child).unwrap().origin(),
        InputRef::FileDataId(20)
    ));
    done.with_payload(child, |p| {
        assert_eq!(p.raw().unwrap(), bytes);
        Ok(())
    })
    .unwrap();
    assert_eq!(done.facts(), before);
}
#[test]
fn compatible_indexed_handles_reject_negative_identity_and_out_of_range() {
    let f = CollisionFixture::for_build(&[(10, None, m2(true))], 70425);
    let p = Private::new();
    let mut store = open(&f, 70425);
    let mut e =
        c::Collector::new_70425(&mut store, &Lookup, Limits::default(), p.spool(&f)).unwrap();
    e.extract(request(99)).unwrap();
    for at in [0, 1, usize::MAX] {
        assert_eq!(
            e.model_handle(at),
            Err(payload::CollectorError::Collision(Error::State))
        );
    }
    let h = present(e.extract(request(10)).unwrap());
    assert_eq!(e.model_handle(0).unwrap(), h);
    for at in [1, usize::MAX] {
        assert_eq!(
            e.model_handle(at),
            Err(payload::CollectorError::Collision(Error::State))
        );
    }
    let done = e.finish().unwrap();
    assert_eq!(done.cached_identity_count(), 2);
    assert_eq!(done.facts().models, 1);
    for at in [1, usize::MAX] {
        assert_eq!(
            done.model_handle(at),
            Err(payload::CollectorError::Collision(Error::State))
        );
    }
}
#[test]
fn compatible_indexed_handles_preserve_failed_collector_readiness() {
    let f = CollisionFixture::for_build(&[(10, None, m2(true))], 70374);
    let p = Private::new();
    let mut store = open(&f, 70374);
    let mut e =
        c::Collector::new_70374(&mut store, &Lookup, Limits::default(), p.spool(&f)).unwrap();
    e.extract(request(10)).unwrap();
    let error = e.extract(request(0)).unwrap_err();
    assert_eq!(e.model_handle(0), Err(error));
    assert_eq!(e.model_handle(usize::MAX), Err(error));
    assert!(e.finish().is_err());
}
