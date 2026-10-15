// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original synthetic spooling/resident equivalence tests.
use super::super::{Extractor, InputRef};
use super::*;
use crate::collision::spool::{Limits as SpoolLimits, ProtectedInputs, Spool};
use crate::local_store::tests::CollisionFixture;
use std::{
    fs,
    os::unix::fs::{DirBuilderExt, PermissionsExt},
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
struct Lookup;
impl LiquidLookup for Lookup {
    fn material_id(&self, _: u32) -> Option<u8> {
        None
    }
    fn material_flags(&self, _: u8) -> Option<u32> {
        None
    }
}
fn put(bytes: &mut [u8], at: usize, value: u32) {
    bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
}
fn words(values: &[u32]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}
fn chunk(tag: &[u8; 4], body: &[u8]) -> Vec<u8> {
    let mut out: Vec<_> = tag.iter().rev().copied().collect();
    out.extend_from_slice(&(body.len() as u32).to_le_bytes());
    out.extend_from_slice(body);
    out
}
fn m2(faces: bool) -> Vec<u8> {
    let mut b = vec![0; 304];
    b[..4].copy_from_slice(b"MD20");
    if faces {
        put(&mut b, 216, 3);
        put(&mut b, 220, 304);
        put(&mut b, 224, 3);
        put(&mut b, 228, 310);
        for v in [0u16, 1, 2] {
            b.extend_from_slice(&v.to_le_bytes());
        }
        b.extend(words(&[
            0x8000_0000,
            0,
            0,
            0x3f80_0000,
            0,
            0,
            0,
            0x3f80_0000,
            0,
        ]));
    }
    b
}
fn root(ids: &[u32], doodads: &[u32], sets: bool) -> Vec<u8> {
    let mut h = [0; 64];
    put(&mut h, 4, ids.len() as u32);
    put(&mut h, 32, 777);
    h[60] = 4;
    let mut b = chunk(b"MOHD", &h);
    b.extend(chunk(b"GFID", &words(ids)));
    b.extend(chunk(b"MODI", &words(doodads)));
    let mut spawn = [0; 40];
    put(&mut spawn, 32, 1f32.to_bits());
    b.extend(chunk(b"MODD", &spawn));
    if sets {
        let mut set = [0; 32];
        put(&mut set, 24, 1);
        b.extend(chunk(b"MODS", &set));
    }
    b
}
fn group(faces: bool, skip: bool, refs: &[u16]) -> Vec<u8> {
    let mut h = vec![0; 68];
    put(&mut h, 56, 99);
    put(&mut h, 8, if skip { 0x80 } else { 0 });
    if faces {
        h.extend(chunk(b"MOPY", &[8, 0]));
        h.extend(chunk(b"MOVI", &[0, 0, 1, 0, 2, 0]));
        h.extend(chunk(
            b"MOVT",
            &words(&[0, 0, 0, 0x3f80_0000, 0, 0, 0, 0x3f80_0000, 0]),
        ));
    }
    h.extend(chunk(
        b"MODR",
        &refs
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect::<Vec<_>>(),
    ));
    chunk(b"MOGP", &h)
}
fn request(id: u32) -> ModelRequest<'static> {
    ModelRequest::M2(InputRef::FileDataId(id))
}
fn wmo_request(id: u32) -> ModelRequest<'static> {
    ModelRequest::Wmo(InputRef::FileDataId(id))
}

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Private {
    base: PathBuf,
    path: PathBuf,
}
impl Private {
    fn new() -> Self {
        let base = std::env::temp_dir().join(format!(
            "rce-payload-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::DirBuilder::new().mode(0o700).create(&base).unwrap();
        let path = base.join("spool");
        Self { base, path }
    }
    fn spool(&self, fixture: &CollisionFixture, limits: SpoolLimits) -> Spool {
        Spool::create(
            &self.path,
            ProtectedInputs::capture(fixture.path(), &[]).unwrap(),
            limits,
        )
        .unwrap()
    }
}
impl Drop for Private {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.base).unwrap();
    }
}
fn large_m2() -> Vec<u8> {
    let mut b = m2(true);
    b.resize(310 + 120 * 12, 0);
    put(&mut b, 224, 120);
    put(&mut b, 228, 310);
    b
}
#[test]
fn m2_raw_facts_class_handles_and_cache_equal_owned() {
    let files = [(10, None, m2(true)), (20, None, m2(false))];
    let f = CollisionFixture::new(&files);
    let p = Private::new();
    let mut store = f.open(false);
    let mut old = Extractor::new(&mut store, &Lookup, CollisionLimits::default()).unwrap();
    let a = old.extract(request(10)).unwrap();
    old.extract(request(10)).unwrap();
    let b = old.extract(request(20)).unwrap();
    let old = old.finish().unwrap();
    let mut store = f.open(false);
    let mut e = SpoolingExtractor::new(
        &mut store,
        &Lookup,
        CollisionLimits::default(),
        p.spool(&f, SpoolLimits::default()),
    )
    .unwrap();
    assert_eq!(e.extract(request(10)).unwrap(), a);
    assert_eq!(e.extract(request(10)).unwrap(), a);
    assert_eq!(e.extract(request(20)).unwrap(), b);
    assert_eq!(e.facts().unwrap(), old.facts());
    assert_eq!(e.request_calls(), 3);
    let mut new = e.finish().unwrap();
    assert_eq!(new.requests().unwrap(), old.requests());
    for h in [a, b] {
        let d = new.descriptor(h).unwrap();
        let m = old.model(h).unwrap();
        assert_eq!(d.class(), m.class());
        assert_eq!(d.status(), m.status());
        assert_eq!(d.output_basename(), m.output_basename());
        assert_eq!(
            d.source().unwrap().content_key(),
            m.source().unwrap().content_key()
        );
        assert_eq!(
            new.with_payload(h, |p| Ok(p.raw().map(<[u8]>::to_vec)))
                .unwrap(),
            m.raw().map(<[u8]>::to_vec)
        );
    }
    assert!(
        new.models
            .iter()
            .all(|m| m.raw.is_none() && m.wmo.is_none())
    );
}
#[test]
fn many_models_exceed_active_budget_without_retained_payloads() {
    let files: Vec<_> = (10..30).map(|id| (id, None, large_m2())).collect();
    let f = CollisionFixture::new(&files);
    let p = Private::new();
    let mut store = f.open(false);
    let mut e = SpoolingExtractor::new(
        &mut store,
        &Lookup,
        CollisionLimits::default(),
        p.spool(
            &f,
            SpoolLimits {
                max_active_bytes: 4096,
                ..SpoolLimits::default()
            },
        ),
    )
    .unwrap();
    let mut handles = Vec::new();
    for id in 10..30 {
        handles.push(e.extract(request(id)).unwrap());
        assert!(
            e.inner
                .models
                .iter()
                .all(|m| m.raw.is_none() && m.wmo.is_none())
        );
    }
    assert!(e.facts().unwrap().raw_bytes > 4096);
    let stats = e.stats().unwrap();
    assert_eq!(stats.active_ceiling, 4096);
    assert!(stats.observed_payload_bytes <= 4096);
    let mut result = e.finish().unwrap();
    let expected = forever_vmap::M2Collision::parse(&large_m2(), forever_vmap::Limits::default())
        .unwrap()
        .unwrap()
        .write_raw(4096)
        .unwrap();
    for h in handles {
        assert_eq!(
            result
                .with_payload(h, |p| Ok(p.raw().unwrap().to_vec()))
                .unwrap(),
            expected
        )
    }
    assert!(result.stats().unwrap().spool.peak_reload_bytes <= 4096);
    assert!(result.stats().unwrap().spool.disk_bytes > 4096);
    // Same source and old resident collector keep every RAW; its retained
    // payload sum exceeds the new route's whole active payload allowance.
    let mut store = f.open(false);
    let mut old = Extractor::new(&mut store, &Lookup, CollisionLimits::default()).unwrap();
    for id in 10..30 {
        old.extract(request(id)).unwrap();
    }
    let old = old.finish().unwrap();
    assert!(
        old.models()
            .iter()
            .map(|m| m.raw().unwrap().len())
            .sum::<usize>()
            > 4096
    );
    assert_eq!(result.facts(), old.facts());
    // Repeat both routes with the SAME global ceiling, including repository
    // descriptors. The resident route cannot retain this collection under it.
    let stats = result.stats().unwrap();
    let ceiling = old.store_resident_bytes()
        + stats.compact_bytes
        + stats.repository_bytes
        + 4096
        + std::mem::size_of_val(old.models()); // full compact-vector overlap

    let limits = CollisionLimits {
        max_memory_bytes: ceiling,
        ..CollisionLimits::default()
    };
    let mut store = f.open(false);
    let mut resident = Extractor::new(&mut store, &Lookup, limits).unwrap();
    let mut stopped = false;
    for id in 10..30 {
        if resident.extract(request(id)).is_err() {
            stopped = true;
            break;
        }
    }
    assert!(
        stopped,
        "owned collector unexpectedly fit all retained payloads"
    );
    drop(resident);
    let p2 = Private::new();
    let mut store = f.open(false);
    let mut bounded = SpoolingExtractor::new(
        &mut store,
        &Lookup,
        limits,
        p2.spool(
            &f,
            SpoolLimits {
                max_active_bytes: 4096,
                ..SpoolLimits::default()
            },
        ),
    )
    .unwrap();
    for id in 10..30 {
        bounded.extract(request(id)).unwrap();
    }
    assert_eq!(bounded.finish().unwrap().facts(), old.facts());
}
#[test]
fn recursive_wmo_doodads_all_metadata_and_raw_equal_owned() {
    let f = CollisionFixture::new(&[
        (10, None, root(&[30], &[20], true)),
        (20, None, m2(true)),
        (30, None, group(true, false, &[0])),
    ]);
    let p = Private::new();
    let mut store = f.open(false);
    let mut old = Extractor::new(&mut store, &Lookup, CollisionLimits::default()).unwrap();
    let h = old.extract(wmo_request(10)).unwrap();
    let old = old.finish().unwrap();
    let mut store = f.open(false);
    let mut e = SpoolingExtractor::new(
        &mut store,
        &Lookup,
        CollisionLimits::default(),
        p.spool(&f, SpoolLimits::default()),
    )
    .unwrap();
    assert_eq!(e.extract(wmo_request(10)).unwrap(), h);
    assert_eq!(e.facts().unwrap(), old.facts());
    assert!(
        e.inner
            .models
            .iter()
            .all(|m| m.raw.is_none() && m.wmo.is_none())
    );
    let mut result = e.finish().unwrap();
    let m = old.model(h).unwrap();
    let w = m.wmo_metadata().unwrap();
    result
        .with_payload(h, |p| {
            assert_eq!(p.raw(), m.raw());
            let n = p.wmo().unwrap();
            assert_eq!(n.root().bytes(), w.root().unwrap().bytes());
            assert_eq!(n.bindings().collect::<Vec<_>>(), w.bindings());
            assert_eq!(
                n.references().collect::<Vec<_>>(),
                w.selection().references()
            );
            assert_eq!(n.set_count(), w.selection().set_count());
            for set in [0, 1, u16::MAX] {
                assert_eq!(
                    n.set_references(set).collect::<Vec<_>>(),
                    w.selection().set_references(set).collect::<Vec<_>>()
                )
            }
            let groups = n.group_sources().collect::<Vec<_>>();
            assert_eq!(groups.len(), w.group_sources().len());
            for (a, b) in groups.iter().zip(w.group_sources()) {
                assert_eq!(a.file_data_id(), b.file_data_id());
                assert_eq!(a.content_key(), b.content_key());
                assert_eq!(a.byte_count(), b.byte_count());
                assert_eq!(a.first_magic(), b.first_magic());
            }
            Ok(())
        })
        .unwrap();
    assert_eq!(
        result
            .with_payload(ModelHandle(1), |p| Ok(p.raw().unwrap().to_vec()))
            .unwrap(),
        old.model(ModelHandle(1)).unwrap().raw().unwrap()
    );
}
#[test]
fn doodad_only_and_texture_classification_survive_lookup_transition() {
    let mut classified_root = chunk(b"MVER", &17u32.to_le_bytes());
    classified_root.extend(root(&[30], &[20], true));
    let f = CollisionFixture::new(&[
        (10, None, classified_root),
        (20, None, m2(true)),
        (30, None, group(false, false, &[0])),
        (40, None, b"BLP2synthetic".to_vec()),
    ]);
    let p = Private::new();
    let mut store = f.open(false);
    let mut e = SpoolingExtractor::new(
        &mut store,
        &Lookup,
        CollisionLimits::default(),
        p.spool(&f, SpoolLimits::default()),
    )
    .unwrap();
    let h = e.extract(wmo_request(10)).unwrap();
    assert_eq!(e.descriptor(h).unwrap().status(), Status::DoodadOnly);
    let before = e.facts().unwrap();
    let mut e = e.with_lookup(&Lookup).unwrap();
    assert_eq!(
        e.extract_gameobject_fdid(10).unwrap(),
        GameObjectOutcome::Model(h)
    );
    assert_eq!(e.facts().unwrap(), before);
    assert_eq!(
        e.extract_gameobject_fdid(40).unwrap(),
        GameObjectOutcome::Texture
    );
    assert_eq!(e.request_calls(), 3);
    e.with_payload(h, |p| {
        assert!(p.raw().is_none());
        assert_eq!(p.wmo().unwrap().references().collect::<Vec<_>>(), [0]);
        Ok(())
    })
    .unwrap();
    e.finish().unwrap();
}
#[test]
fn large_wmo_metadata_is_on_disk_and_reloaded_without_global_copy() {
    let mut r = root(&[30], &[20], true);
    r.extend(chunk(b"ZZZZ", &vec![0; 8192]));
    let f = CollisionFixture::new(&[
        (10, None, r.clone()),
        (20, None, m2(true)),
        (30, None, group(true, false, &[0])),
    ]);
    let p = Private::new();
    let mut store = f.open(false);
    let mut e = SpoolingExtractor::new(
        &mut store,
        &Lookup,
        CollisionLimits::default(),
        p.spool(
            &f,
            SpoolLimits {
                max_active_bytes: 32768,
                ..SpoolLimits::default()
            },
        ),
    )
    .unwrap();
    let h = e.extract(wmo_request(10)).unwrap();
    assert!(e.inner.models[h.index()].wmo.is_none());
    assert!(e.stats().unwrap().compact_bytes < 8192);
    e.with_payload(h, |p| {
        assert_eq!(p.wmo().unwrap().root().bytes(), r);
        Ok(())
    })
    .unwrap();
}
#[test]
fn class_alias_poison_is_sticky_no_extra_acquisition() {
    let f = CollisionFixture::new(&[(10, None, m2(true))]);
    let p = Private::new();
    let mut store = f.open(false);
    let mut e = SpoolingExtractor::new(
        &mut store,
        &Lookup,
        CollisionLimits::default(),
        p.spool(&f, SpoolLimits::default()),
    )
    .unwrap();
    e.extract(request(10)).unwrap();
    assert_eq!(
        e.extract(wmo_request(10)),
        Err(CollectorError::Collision(CollisionError::Alias))
    );
    assert_eq!(e.inner.reads, 1);
    assert_eq!(
        e.extract(request(10)),
        Err(CollectorError::Collision(CollisionError::Alias))
    );
    assert_eq!(e.inner.reads, 1);
    assert!(e.finish().is_err());
}
#[test]
fn acquisition_failure_no_prefix_exposed_or_fallback() {
    let f = CollisionFixture::new(&[(10, None, m2(true))]);
    let p = Private::new();
    let mut store = f.open(false);
    let mut e = SpoolingExtractor::new(
        &mut store,
        &Lookup,
        CollisionLimits::default(),
        p.spool(&f, SpoolLimits::default()),
    )
    .unwrap();
    let h = e.extract(request(10)).unwrap();
    let err = e.extract(request(20)).unwrap_err();
    assert!(e.descriptor(h).is_err());
    assert_eq!(e.with_payload(h, |_| Ok(())).err(), Some(err));
    assert_eq!(e.extract(request(10)), Err(err));
    assert!(e.finish().is_err());
}
#[test]
fn source_mutation_on_callback_failure_invalidates_whole_collector() {
    let f = CollisionFixture::new(&[(10, None, m2(true))]);
    let p = Private::new();
    let mut store = f.open(false);
    let mut e = SpoolingExtractor::new(
        &mut store,
        &Lookup,
        CollisionLimits::default(),
        p.spool(&f, SpoolLimits::default()),
    )
    .unwrap();
    let h = e.extract(request(10)).unwrap();
    let err = e
        .with_payload(h, |_| {
            f.change_metadata();
            Err::<(), _>(Error::Callback)
        })
        .unwrap_err();
    assert_eq!(err.code(), "local_store_input_changed");
    assert!(e.finish().is_err());
}
#[test]
fn output_mutation_on_callback_failure_is_terminal() {
    let f = CollisionFixture::new(&[(10, None, m2(true))]);
    let p = Private::new();
    let mut store = f.open(false);
    let mut e = SpoolingExtractor::new(
        &mut store,
        &Lookup,
        CollisionLimits::default(),
        p.spool(&f, SpoolLimits::default()),
    )
    .unwrap();
    let h = e.extract(request(10)).unwrap();
    let file = p.path.join("00000000.payload");
    let err = e
        .with_payload(h, |_| {
            fs::set_permissions(&file, fs::Permissions::from_mode(0o600)).unwrap();
            fs::write(&file, b"changed").unwrap();
            Err::<(), _>(Error::Callback)
        })
        .unwrap_err();
    assert_eq!(err, CollectorError::Payload(Error::Changed));
    assert!(e.finish().is_err());
}
#[test]
fn request_and_raw_cumulative_limits_not_refunded_when_evicted() {
    let f = CollisionFixture::new(&[(10, None, m2(true)), (20, None, m2(true))]);
    let p = Private::new();
    let mut store = f.open(false);
    let one = forever_vmap::M2Collision::parse(&m2(true), forever_vmap::Limits::default())
        .unwrap()
        .unwrap()
        .write_raw(4096)
        .unwrap()
        .len();
    let mut e = SpoolingExtractor::new(
        &mut store,
        &Lookup,
        CollisionLimits {
            max_raw_bytes: one,
            ..CollisionLimits::default()
        },
        p.spool(&f, SpoolLimits::default()),
    )
    .unwrap();
    e.extract(request(10)).unwrap();
    assert_eq!(e.inner.raw_bytes, one);
    assert_eq!(
        e.extract(request(20)).err().map(|e| e.code()),
        Some("vmap_policy_limit")
    );
    assert_eq!(e.inner.raw_bytes, one);
}
#[test]
fn reload_quota_and_descriptor_limits_poison_finished_result() {
    let f = CollisionFixture::new(&[(10, None, m2(true))]);
    let p = Private::new();
    let mut store = f.open(false);
    let mut e = SpoolingExtractor::new(
        &mut store,
        &Lookup,
        CollisionLimits::default(),
        p.spool(
            &f,
            SpoolLimits {
                max_reloads: 1,
                ..SpoolLimits::default()
            },
        ),
    )
    .unwrap();
    let h = e.extract(request(10)).unwrap();
    let mut done = e.finish().unwrap();
    done.with_payload(h, |_| Ok(())).unwrap();
    assert_eq!(
        done.with_payload(h, |_| Ok(())).err(),
        Some(CollectorError::Payload(Error::Limit))
    );
    assert!(done.descriptor(h).is_err());
}
#[test]
fn malformed_payload_extents_and_flags_reject_without_panics() {
    for len in 0..HEADER {
        assert!(Payload::parse(&vec![0; len], wmo::Limits::default()).is_err())
    }
    let mut b = vec![0; HEADER];
    b[..8].copy_from_slice(MAGIC);
    assert!(Payload::parse(&b, wmo::Limits::default()).is_ok());
    for bad in [4, u32::MAX] {
        b[8..12].copy_from_slice(&bad.to_le_bytes());
        assert!(Payload::parse(&b, wmo::Limits::default()).is_err())
    }
    b[8..12].fill(0);
    for index in 0..5 {
        let mut c = b.clone();
        c[12 + index * 8..20 + index * 8].copy_from_slice(&u64::MAX.to_le_bytes());
        assert!(Payload::parse(&c, wmo::Limits::default()).is_err())
    }
    b.push(0);
    assert!(Payload::parse(&b, wmo::Limits::default()).is_err());
}

#[test]
fn multiple_large_wmo_roots_and_selection_do_not_accumulate_in_ram() {
    let mut source = root(&[100], &[101], true);
    source.extend(chunk(b"ZZZZ", &vec![0; 16384]));
    let mut files: Vec<_> = (10..20).map(|id| (id, None, source.clone())).collect();
    files.push((100, None, group(true, false, &[0])));
    files.push((101, None, m2(true)));
    let f = CollisionFixture::new(&files);
    let p = Private::new();
    let mut store = f.open(false);
    let mut e = SpoolingExtractor::new(
        &mut store,
        &Lookup,
        CollisionLimits::default(),
        p.spool(
            &f,
            SpoolLimits {
                max_active_bytes: 65536,
                ..SpoolLimits::default()
            },
        ),
    )
    .unwrap();
    let mut handles = Vec::new();
    for id in 10..20 {
        handles.push(e.extract(wmo_request(id)).unwrap());
        assert!(
            e.inner
                .models
                .iter()
                .all(|m| m.raw.is_none() && m.wmo.is_none())
        );
    }
    let stats = e.stats().unwrap();
    assert!(stats.spool.disk_bytes > 65536);
    assert!(stats.observed_payload_bytes <= 65536);
    assert!(stats.compact_bytes < 16384);
    assert_eq!(e.facts().unwrap().models, 11); // ten WMO + one shared doodad
    assert_eq!(e.facts().unwrap().dependency_requests, 10);
    assert_eq!(e.facts().unwrap().reads, 21); // each WMO/group + one shared doodad
    for handle in handles {
        e.with_payload(handle, |p| {
            assert_eq!(p.wmo().unwrap().root().bytes(), source);
            assert_eq!(p.wmo().unwrap().bindings().next().unwrap().model.index(), 1);
            Ok(())
        })
        .unwrap();
    }
    e.finish().unwrap();
}
#[test]
fn max_records_stops_before_second_source_and_preserves_first_spool() {
    let f = CollisionFixture::new(&[(10, None, m2(true)), (20, None, m2(true))]);
    let p = Private::new();
    let mut store = f.open(false);
    let mut e = SpoolingExtractor::new(
        &mut store,
        &Lookup,
        CollisionLimits::default(),
        p.spool(
            &f,
            SpoolLimits {
                max_records: 1,
                ..SpoolLimits::default()
            },
        ),
    )
    .unwrap();
    e.extract(request(10)).unwrap();
    assert_eq!(
        e.extract(request(20)),
        Err(CollectorError::Payload(Error::Limit))
    );
    assert_eq!(e.inner.reads, 1);
    assert_eq!(fs::read_dir(&p.path).unwrap().count(), 1);
    assert!(e.finish().is_err());
}
#[test]
fn descriptor_quota_is_separate_from_active_payload_admission() {
    let f = CollisionFixture::new(&[(10, None, m2(true))]);
    let p = Private::new();
    let baseline = p.spool(&f, SpoolLimits::default());
    let base = baseline.stats().unwrap().descriptor_bytes;
    drop(baseline);
    let path = p.base.join("other");
    let spool = Spool::create(
        &path,
        ProtectedInputs::capture(f.path(), &[]).unwrap(),
        SpoolLimits {
            max_descriptor_bytes: base + 100,
            ..SpoolLimits::default()
        },
    )
    .unwrap();
    let mut store = f.open(false);
    let mut e =
        SpoolingExtractor::new(&mut store, &Lookup, CollisionLimits::default(), spool).unwrap();
    assert_eq!(
        e.extract(request(10)),
        Err(CollectorError::Collision(CollisionError::Limit))
    );
    assert_eq!(e.inner.reads, 0);
}
#[test]
fn request_calls_and_source_bytes_remain_cumulative_on_new_route() {
    let f = CollisionFixture::new(&[(10, None, m2(true)), (20, None, m2(true))]);
    let p = Private::new();
    let mut store = f.open(false);
    let mut e = SpoolingExtractor::new(
        &mut store,
        &Lookup,
        CollisionLimits {
            max_requests: 1,
            ..CollisionLimits::default()
        },
        p.spool(&f, SpoolLimits::default()),
    )
    .unwrap();
    let h = e.extract(request(10)).unwrap();
    assert_eq!(
        e.extract(request(10)),
        Err(CollectorError::Collision(CollisionError::Limit))
    );
    assert_eq!(e.inner.reads, 1);
    assert_eq!(e.request_calls(), 1);
    assert!(e.descriptor(h).is_err());
    drop(e);
    let p = Private::new();
    let mut store = f.open(false);
    let mut e = SpoolingExtractor::new(
        &mut store,
        &Lookup,
        CollisionLimits {
            max_source_bytes: m2(true).len(),
            ..CollisionLimits::default()
        },
        p.spool(&f, SpoolLimits::default()),
    )
    .unwrap();
    e.extract(request(10)).unwrap();
    assert_eq!(
        e.extract(request(20)),
        Err(CollectorError::Collision(CollisionError::Limit))
    );
    assert_eq!(e.inner.source_bytes, m2(true).len());
    assert_eq!(e.inner.reads, 1);
}
#[test]
fn stored_record_replacement_before_reload_is_detected_and_no_prefix_returned() {
    let f = CollisionFixture::new(&[(10, None, m2(true))]);
    let p = Private::new();
    let mut store = f.open(false);
    let mut e = SpoolingExtractor::new(
        &mut store,
        &Lookup,
        CollisionLimits::default(),
        p.spool(&f, SpoolLimits::default()),
    )
    .unwrap();
    let h = e.extract(request(10)).unwrap();
    let mut result = e.finish().unwrap();
    let file = p.path.join("00000000.payload");
    let bytes = fs::read(&file).unwrap();
    fs::rename(&file, p.base.join("retained")).unwrap();
    fs::write(&file, bytes).unwrap();
    assert_eq!(
        result.with_payload(h, |_| Ok(())).err(),
        Some(CollectorError::Payload(Error::Changed))
    );
    assert!(result.descriptor(h).is_err());
}

#[test]
fn repeated_wmo_bindings_are_externalized_but_requests_never_refunded() {
    let refs = vec![20; 128];
    let f = CollisionFixture::new(&[
        (10, None, root(&[30], &refs, true)),
        (20, None, m2(true)),
        (30, None, group(true, false, &[0])),
    ]);
    let p = Private::new();
    let mut store = f.open(false);
    let mut e = SpoolingExtractor::new(
        &mut store,
        &Lookup,
        CollisionLimits::default(),
        p.spool(&f, SpoolLimits::default()),
    )
    .unwrap();
    let h = e.extract(wmo_request(10)).unwrap();
    assert!(e.inner.models[h.index()].wmo.is_none());
    assert_eq!(e.facts().unwrap().dependency_requests, 128);
    assert_eq!(e.facts().unwrap().reads, 3);
    e.with_payload(h, |p| {
        let w = p.wmo().unwrap();
        assert_eq!(w.bindings().len(), 128);
        assert!(w.bindings().all(|b| b.model.index() == 1));
        assert_eq!(w.references().collect::<Vec<_>>(), [0]);
        Ok(())
    })
    .unwrap();
}

#[test]
fn legacy_error_remains_exhaustively_matchable_without_new_variant() {
    fn exhaustive(e: CollisionError) -> &'static str {
        match e {
            CollisionError::InvalidLimits => "collision_invalid_limits",
            CollisionError::Limit => "collision_policy_limit",
            CollisionError::Allocation => "collision_allocation",
            CollisionError::Name => "collision_invalid_name",
            CollisionError::GroupName => "collision_group_name",
            CollisionError::Alias => "collision_output_alias",
            CollisionError::Cycle => "collision_cycle",
            CollisionError::NoCollision => "collision_no_valid_collision",
            CollisionError::GameObjectMagic => "collision_gameobject_magic",
            CollisionError::State => "collision_invalid_state",
            CollisionError::Aborted => "collision_external_run_failure",
            CollisionError::Store(e) => e.code(),
            #[cfg(target_os = "linux")]
            CollisionError::Available(e) => e.code(),
            CollisionError::M2(e) => e.code(),
            CollisionError::Wmo(e) => e.code(),
        }
    }
    assert_eq!(
        exhaustive(CollisionError::Alias),
        CollisionError::Alias.code()
    );
}
#[test]
fn mapping_reallocation_admits_full_new_allocation_while_old_is_live() {
    let f = CollisionFixture::new(&[(10, None, m2(true))]);
    let p = Private::new();
    let mut store = f.open(false);
    let mut e = SpoolingExtractor::new(
        &mut store,
        &Lookup,
        CollisionLimits::default(),
        p.spool(&f, SpoolLimits::default()),
    )
    .unwrap();
    e.extract(request(10)).unwrap();
    let r = e.inner.payloads.as_mut().unwrap();
    let old = r.mapping.capacity();
    assert_eq!(old, 1);
    assert_eq!(r.prepare(2, 0, size_of::<Mapping>()), Err(Error::Limit));
    assert_eq!(r.mapping.capacity(), old);
}
#[test]
fn new_collector_vector_reallocation_admits_full_new_capacity() {
    let f = CollisionFixture::new(&[(10, None, m2(true)), (20, None, m2(true))]);
    let p = Private::new();
    let mut store = f.open(false);
    let mut e = SpoolingExtractor::new(
        &mut store,
        &Lookup,
        CollisionLimits::default(),
        p.spool(&f, SpoolLimits::default()),
    )
    .unwrap();
    e.extract(request(10)).unwrap();
    let old = e.inner.models.capacity();
    assert_eq!(old, 1);
    let resident = e.inner.store.stats().resident_bytes
        + e.inner.payloads.as_ref().unwrap().resident_bytes()
        + e.inner.live;
    e.inner.limits.max_memory_bytes = resident + size_of::<Model>() + 16;
    assert!(e.extract(request(20)).is_err());
    assert_eq!(e.inner.models.capacity(), old);
    assert_eq!(e.inner.reads, 1);
}
#[test]
fn failing_source_guard_does_not_short_circuit_spool_guard() {
    let f = CollisionFixture::new(&[(10, None, m2(true))]);
    let p = Private::new();
    let mut store = f.open(false);
    let mut e = SpoolingExtractor::new(
        &mut store,
        &Lookup,
        CollisionLimits::default(),
        p.spool(&f, SpoolLimits::default()),
    )
    .unwrap();
    e.extract(request(10)).unwrap();
    f.change_metadata();
    fs::set_permissions(&p.path, fs::Permissions::from_mode(0o755)).unwrap();
    let err = e.verify_unchanged().unwrap_err();
    assert_eq!(err.code(), "local_store_input_changed");
    assert_eq!(
        e.inner.payloads.as_ref().unwrap().spool.failure(),
        Some(Error::Changed)
    );
    assert!(e.finish().is_err());
}
#[test]
fn gameobject_slot_is_admitted_before_unknown_class_source_read() {
    let f = CollisionFixture::new(&[(10, None, m2(true))]);
    let p = Private::new();
    let mut store = f.open(false);
    let mut e = SpoolingExtractor::new(
        &mut store,
        &Lookup,
        CollisionLimits::default(),
        p.spool(
            &f,
            SpoolLimits {
                max_records: 0,
                ..SpoolLimits::default()
            },
        ),
    )
    .unwrap();
    assert_eq!(
        e.extract_gameobject_fdid(10),
        Err(CollectorError::Payload(Error::Limit))
    );
    assert_eq!(e.inner.reads, 0);
    assert_eq!(e.inner.source_bytes, 0);
    assert_eq!(e.request_calls(), 1);
}
#[test]
fn texture_slot_reservation_is_retained_but_no_model_or_work_refund_occurs() {
    let f = CollisionFixture::new(&[(10, None, b"BLP2synthetic".to_vec()), (20, None, m2(true))]);
    let p = Private::new();
    let mut store = f.open(false);
    let mut e = SpoolingExtractor::new(
        &mut store,
        &Lookup,
        CollisionLimits::default(),
        p.spool(
            &f,
            SpoolLimits {
                max_records: 1,
                ..SpoolLimits::default()
            },
        ),
    )
    .unwrap();
    assert_eq!(
        e.extract_gameobject_fdid(10).unwrap(),
        GameObjectOutcome::Texture
    );
    assert_eq!(e.facts().unwrap().models, 0);
    assert_eq!(e.inner.models.capacity(), 1);
    assert_eq!(e.inner.payloads.as_ref().unwrap().mapping.capacity(), 1);
    assert_eq!(e.facts().unwrap().reads, 1);
    assert_eq!(e.facts().unwrap().source_bytes, 13);
    assert_eq!(
        e.extract_gameobject_fdid(20).unwrap(),
        GameObjectOutcome::Model(ModelHandle(0))
    );
    assert_eq!(e.request_calls(), 2);
    assert_eq!(e.facts().unwrap().reads, 2);
    assert_eq!(e.finish().unwrap().facts().models, 1);
}

#[test]
fn ready_payload_shape_sizes_match_actual_complete_record() {
    let f = CollisionFixture::new(&[(10, None, m2(true))]);
    let p = Private::new();
    let mut store = f.open(false);
    let mut e = SpoolingExtractor::new(
        &mut store,
        &Lookup,
        CollisionLimits::default(),
        p.spool(&f, SpoolLimits::default()),
    )
    .unwrap();
    let h = e.extract(request(10)).unwrap();
    let shape = e.payload_shape(h).unwrap();
    assert_eq!(shape.class, Class::M2);
    assert_eq!(shape.raw_bytes, Some(148));
    assert_eq!(shape.record_bytes, HEADER + 148);
    e.with_payload(h, |p| {
        assert_eq!(p.raw().map(<[u8]>::len), shape.raw_bytes);
        Ok(())
    })
    .unwrap();
}
#[test]
fn actual_callback_rejects_altered_raw_shape_before_user_callback() {
    let f = CollisionFixture::new(&[(10, None, m2(true))]);
    let p = Private::new();
    let mut store = f.open(false);
    let mut e = SpoolingExtractor::new(
        &mut store,
        &Lookup,
        CollisionLimits::default(),
        p.spool(&f, SpoolLimits::default()),
    )
    .unwrap();
    let h = e.extract(request(10)).unwrap();
    e.inner.payloads.as_mut().unwrap().mapping[0]
        .shape
        .raw_bytes = Some(149);
    let mut called = false;
    assert_eq!(
        e.with_payload(h, |_| {
            called = true;
            Ok(())
        })
        .err(),
        Some(CollectorError::Payload(Error::Format))
    );
    assert!(!called);
    assert!(e.payload_shape(h).is_err());
}
#[test]
fn generic_callback_cause_survives_plain_failure_and_changed_guard() {
    for changed in [false, true] {
        let f = CollisionFixture::new(&[(10, None, m2(true))]);
        let p = Private::new();
        let mut store = f.open(false);
        let mut e = SpoolingExtractor::new(
            &mut store,
            &Lookup,
            CollisionLimits::default(),
            p.spool(&f, SpoolLimits::default()),
        )
        .unwrap();
        let h = e.extract(request(10)).unwrap();
        let result = e.with_payload_result(h, |_| {
            if changed {
                f.change_metadata()
            }
            Err::<(), _>(123u32)
        });
        match result {
            Err(CallbackFailure::Callback { cause, guard }) => {
                assert_eq!(cause, 123);
                assert_eq!(guard.is_some(), changed);
                if let Some(e) = guard {
                    assert_eq!(e.code(), "local_store_input_changed")
                }
            }
            _ => panic!("cause bridge"),
        };
        assert!(e.descriptor(h).is_err());
    }
}
#[test]
fn final_external_failure_retains_cause_and_poison_even_without_io_failure() {
    let f = CollisionFixture::new(&[(10, None, m2(true))]);
    let p = Private::new();
    let mut store = f.open(false);
    let mut e = SpoolingExtractor::new(
        &mut store,
        &Lookup,
        CollisionLimits::default(),
        p.spool(&f, SpoolLimits::default()),
    )
    .unwrap();
    let result = e.finish_external_operation(Err::<(), _>(77u32));
    assert!(matches!(
        result,
        Err(CallbackFailure::Callback {
            cause: 77,
            guard: None
        })
    ));
    assert!(e.facts().is_err());
    assert!(e.finish().is_err());
}
#[test]
fn retained_external_transfer_requires_exact_owned_charge() {
    for transfer in [0usize, 999, 1000] {
        let f = CollisionFixture::new(&[(10, None, m2(true))]);
        let p = Private::new();
        let mut store = f.open(false);
        let mut e = SpoolingExtractor::new(
            &mut store,
            &Lookup,
            CollisionLimits::default(),
            p.spool(&f, SpoolLimits::default()),
        )
        .unwrap();
        e.extract(request(10)).unwrap();
        e.reserve_external(1000).unwrap();
        let result = e.finish_with_retained_external(transfer);
        if transfer == 1000 {
            assert_eq!(result.unwrap().external_resident, 1000)
        } else {
            assert_eq!(
                result.err(),
                Some(CollectorError::Collision(CollisionError::State))
            )
        }
    }
}
#[test]
fn retained_data_capacity_remains_in_complete_global_reload_budget() {
    for deficit in [0usize, 1] {
        let f = CollisionFixture::new(&[(10, None, m2(true))]);
        let p = Private::new();
        let mut store = f.open(false);
        let mut e = SpoolingExtractor::new(
            &mut store,
            &Lookup,
            CollisionLimits::default(),
            p.spool(&f, SpoolLimits::default()),
        )
        .unwrap();
        let h = e.extract(request(10)).unwrap();
        let shape = e.payload_shape(h).unwrap();
        e.reserve_external(1000).unwrap();
        let mut done = e.finish_with_retained_external(1000).unwrap();
        let compact = compact_bytes(&done.models, &done.requests).unwrap();
        done.memory = done.store_resident
            + compact
            + done.repository.resident_bytes()
            + done.external_resident
            + shape.record_bytes
            - deficit;
        let mut called = false;
        let result = done.with_payload(h, |_| {
            called = true;
            Ok(())
        });
        assert_eq!(result.is_ok(), deficit == 0);
        assert_eq!(called, deficit == 0);
        assert_eq!(done.external_resident, 1000);
    }
}
#[test]
fn retained_data_capacity_also_remains_in_active_reload_budget() {
    let f = CollisionFixture::new(&[(10, None, m2(true))]);
    let p = Private::new();
    let mut store = f.open(false);
    let mut e = SpoolingExtractor::new(
        &mut store,
        &Lookup,
        CollisionLimits::default(),
        p.spool(
            &f,
            SpoolLimits {
                max_active_bytes: 4096,
                ..SpoolLimits::default()
            },
        ),
    )
    .unwrap();
    let h = e.extract(request(10)).unwrap();
    e.reserve_external(3900).unwrap();
    let mut done = e.finish_with_retained_external(3900).unwrap();
    assert!(done.with_payload(h, |_| Ok(())).is_err());
    assert_eq!(done.external_resident, 3900);
}
#[test]
fn preadmitted_reload_totals_are_consumed_only_by_actual_reads() {
    let f = CollisionFixture::new(&[(10, None, m2(true))]);
    let p = Private::new();
    let mut store = f.open(false);
    let mut e = SpoolingExtractor::new(
        &mut store,
        &Lookup,
        CollisionLimits::default(),
        p.spool(&f, SpoolLimits::default()),
    )
    .unwrap();
    let h = e.extract(request(10)).unwrap();
    let bytes = e.payload_shape(h).unwrap().record_bytes;
    e.admit_reloads(1, bytes).unwrap();
    assert_eq!(e.stats().unwrap().spool.reloads, 0);
    assert_eq!(e.stats().unwrap().spool.reload_bytes, 0);
    e.with_payload(h, |_| Ok(())).unwrap();
    assert_eq!(e.stats().unwrap().spool.reloads, 1);
    assert_eq!(e.stats().unwrap().spool.reload_bytes, bytes);
}
#[test]
fn guarded_source_and_external_lifecycle_prevents_unreleased_finish() {
    let f = CollisionFixture::new(&[(10, None, m2(true))]);
    let p = Private::new();
    let mut store = f.open(false);
    let mut e = SpoolingExtractor::new(
        &mut store,
        &Lookup,
        CollisionLimits::default(),
        p.spool(&f, SpoolLimits::default()),
    )
    .unwrap();
    let (bytes, _) = e.read_source(10, 4096).unwrap();
    let n = bytes.capacity();
    e.reserve_external(100).unwrap();
    e.release_external(100).unwrap();
    drop(bytes);
    e.release_source(n).unwrap();
    assert_eq!(e.facts().unwrap().reads, 1);
    e.finish().unwrap();
}

#[test]
fn unreleased_source_capacity_cannot_transfer_to_completed_data() {
    let f = CollisionFixture::new(&[(10, None, m2(true))]);
    let p = Private::new();
    let mut store = f.open(false);
    let mut e = SpoolingExtractor::new(
        &mut store,
        &Lookup,
        CollisionLimits::default(),
        p.spool(&f, SpoolLimits::default()),
    )
    .unwrap();
    let (bytes, _) = e.read_source(10, 4096).unwrap();
    drop(bytes);
    assert_eq!(
        e.finish_with_retained_external(0).err(),
        Some(CollectorError::Collision(CollisionError::State))
    );
}

#[test]
fn compact_descriptor_callback_exposes_no_payload_and_preserves_original_error() {
    let f = CollisionFixture::new(&[(10, None, m2(true))]);
    let p = Private::new();
    let mut store = f.open(false);
    let mut e = SpoolingExtractor::new(
        &mut store,
        &Lookup,
        CollisionLimits::default(),
        p.spool(&f, SpoolLimits::default()),
    )
    .unwrap();
    let handle = e.extract(request(10)).unwrap();
    let result: Result<(), CallbackFailure<u16>> = e.with_descriptors_result(|view| {
        assert_eq!(view.model_count(), 1);
        assert_eq!(view.request(0).unwrap(), handle);
        assert_eq!(view.descriptor(handle).unwrap().class(), Class::M2);
        view.check_identity(handle, Class::M2, InputRef::FileDataId(10))
            .unwrap();
        Err(99)
    });
    assert!(matches!(
        result,
        Err(CallbackFailure::Callback {
            cause: 99,
            guard: None
        })
    ));
    assert_eq!(e.stats().unwrap().spool.reloads, 0);
    assert!(e.finish().is_err());
}
#[test]
fn split_payload_callback_resolves_children_without_additional_reload() {
    let f = CollisionFixture::new(&[
        (10, None, root(&[30], &[20], true)),
        (20, None, m2(true)),
        (30, None, group(false, false, &[0])),
    ]);
    let p = Private::new();
    let mut store = f.open(false);
    let mut e = SpoolingExtractor::new(
        &mut store,
        &Lookup,
        CollisionLimits::default(),
        p.spool(&f, SpoolLimits::default()),
    )
    .unwrap();
    let handle = e.extract(wmo_request(10)).unwrap();
    let result: Result<(), CallbackFailure<u16>> =
        e.with_payload_descriptors_result(handle, |p, view| {
            let w = p.wmo().unwrap();
            let b = w.binding(0).unwrap();
            assert!(w.binding(1).is_none());
            assert_eq!(
                view.descriptor(b.model).unwrap().status(),
                Status::Collision
            );
            Ok(())
        });
    assert!(result.is_ok());
    assert_eq!(e.stats().unwrap().spool.reloads, 1);
    e.finish().unwrap();
}

#[test]
fn compact_callback_error_still_runs_both_changed_guards_independently() {
    let f = CollisionFixture::new(&[(10, None, m2(true))]);
    let p = Private::new();
    let mut store = f.open(false);
    let mut e = SpoolingExtractor::new(
        &mut store,
        &Lookup,
        CollisionLimits::default(),
        p.spool(&f, SpoolLimits::default()),
    )
    .unwrap();
    e.extract(request(10)).unwrap();
    let file = fs::read_dir(&p.path)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let result: Result<(), CallbackFailure<u16>> = e.with_descriptors_result(|_| {
        f.change_metadata();
        fs::set_permissions(&file, fs::Permissions::from_mode(0o600)).unwrap();
        fs::write(&file, b"changed").unwrap();
        Err(91)
    });
    assert!(matches!(
        result,
        Err(CallbackFailure::Callback {
            cause: 91,
            guard: Some(CollectorError::Collision(CollisionError::Store(
                crate::local_store::Error::InputChanged
            )))
        })
    ));
    assert_eq!(
        e.inner.payloads.as_ref().unwrap().spool.failure(),
        Some(Error::Changed)
    );
    assert!(e.finish().is_err());
}
