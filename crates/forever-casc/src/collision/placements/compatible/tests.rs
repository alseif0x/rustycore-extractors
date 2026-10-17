// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-17: manufactured toy LocalStore/compatible placement cases.
// Shared fixture recipes follow the existing authored strict spool tests.
use super::*;
use crate::collision::Limits as CollisionLimits;
use crate::collision::spool::{Limits as SpoolLimits, ProtectedInputs, Spool};
use crate::local_store::tests::{CollisionFile, CollisionFixture};
use crate::{
    Locale, local_index,
    local_store::{self, LocalStore},
};
use forever_vmap::{
    objects::{self, Objects},
    placement::DirBin,
};
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
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Private {
    base: PathBuf,
    path: PathBuf,
}
impl Private {
    fn new() -> Self {
        let base = std::env::temp_dir().join(format!(
            "rce-compatible-placement-{}-{}",
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
#[derive(Default)]
struct TestSink {
    rows: Vec<(u32, u32, usize, Vec<u8>)>,
    fail_at: Option<usize>,
}
const SINK_ALLOWANCE: usize = 64 * 1024;
impl Sink for TestSink {
    type Error = u8;
    fn stage(&mut self, batch: Staged<'_>) -> Result<(), u8> {
        if self.fail_at == Some(self.rows.len()) {
            return Err(7);
        }
        let retained = self.rows.capacity() * size_of::<(u32, u32, usize, Vec<u8>)>()
            + self.rows.iter().map(|r| r.3.capacity()).sum::<usize>();
        let next = self.rows.len() + 1;
        let table = if next > self.rows.capacity() {
            next * size_of::<(u32, u32, usize, Vec<u8>)>()
        } else {
            0
        };
        if retained + table + batch.bytes.len() > SINK_ALLOWANCE {
            return Err(8);
        }
        self.rows.try_reserve_exact(1).map_err(|_| 9)?;
        let mut bytes = Vec::new();
        bytes.try_reserve_exact(batch.bytes.len()).map_err(|_| 9)?;
        if retained
            + self.rows.capacity() * size_of::<(u32, u32, usize, Vec<u8>)>()
            + bytes.capacity()
            > SINK_ALLOWANCE
        {
            return Err(8);
        }
        bytes.extend_from_slice(batch.bytes);
        self.rows
            .push((batch.map_id, batch.original_map_id, batch.records, bytes));
        Ok(())
    }
}
fn limits() -> Limits {
    Limits {
        ids: ids::Limits {
            max_memory_bytes: 1024 * 1024,
            ..ids::Limits::default()
        },
        ..Limits::default()
    }
}
fn mddf(id: u32, unique: u32) -> [u8; 36] {
    let mut b = [0; 36];
    put(&mut b, 0, id);
    put(&mut b, 4, unique);
    b[32..34].copy_from_slice(&1024u16.to_le_bytes());
    b[34..36].copy_from_slice(&0x40u16.to_le_bytes());
    b
}
fn modf(id: u32, unique: u32, set: u16) -> [u8; 64] {
    let mut b = [0; 64];
    put(&mut b, 0, id);
    put(&mut b, 4, unique);
    b[56..58].copy_from_slice(&8u16.to_le_bytes());
    b[58..60].copy_from_slice(&set.to_le_bytes());
    b
}
fn parse(b: &[u8]) -> Objects<'_> {
    Objects::parse_adt(b, objects::Limits::default()).unwrap()
}
fn batch<'a, 'b>(objects: &'a Objects<'b>, map: u32) -> Batch<'a, 'b> {
    Batch {
        objects,
        map_id: map,
        original_map_id: 9,
    }
}
fn overlap_root(group: u32, child: u32) -> Vec<u8> {
    let mut h = [0; 64];
    put(&mut h, 4, 1);
    h[60] = 4;
    let mut b = chunk(b"MOHD", &h);
    b.extend(chunk(b"GFID", &words(&[group])));
    b.extend(chunk(b"MODI", &words(&[child])));
    let mut spawns = [0; 80];
    for at in [0, 40] {
        put(&mut spawns, at + 28, 1f32.to_bits());
        put(&mut spawns, at + 32, 1f32.to_bits());
    }
    b.extend(chunk(b"MODD", &spawns));
    let mut sets = [0; 64];
    put(&mut sets, 24, 2);
    put(&mut sets, 52, 1);
    put(&mut sets, 56, 1);
    b.extend(chunk(b"MODS", &sets));
    b
}
fn mixed_files() -> Vec<CollisionFile<'static>> {
    vec![
        (10, None, m2(true)),
        (20, None, m2(false)),
        (30, None, overlap_root(31, 10)),
        (31, None, group(true, false, &[1, 0])),
        (40, None, overlap_root(41, 10)),
        (41, None, group(false, false, &[1, 0])),
    ]
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
fn collector<'s>(
    store: &'s mut LocalStore,
    p: &Private,
    f: &CollisionFixture,
    build: u32,
) -> Collector<'s, 'static, Lookup> {
    let spool = p.spool(f, SpoolLimits::default());
    match build {
        70374 => Collector::new_70374(store, &Lookup, CollisionLimits::default(), spool),
        70425 => Collector::new_70425(store, &Lookup, CollisionLimits::default(), spool),
        _ => unreachable!(),
    }
    .unwrap()
}
fn ids(bytes: &[u8]) -> Vec<u32> {
    DirBin::parse(bytes, placement::Limits::default())
        .unwrap()
        .records()
        .map(|p| p.id())
        .collect()
}
#[test]
fn compatible_physical_omissions_first_middle_last_repeated_and_following_success() {
    for build in [70374, 70425] {
        let f = CollisionFixture::for_build(&[(10, None, m2(true)), (20, None, m2(false))], build);
        let p = Private::new();
        let mut store = open(&f, build);
        let mut e = collector(&mut store, &p, &f, build);
        let mut bytes = Vec::new();
        for id in [9, 10, 19, 20, 29, 9, 10] {
            bytes.extend(chunk(b"MDDF", &mddf(id, 100 + id)))
        }
        let o = parse(&bytes);
        let mut s = Session::new(&mut e, TestSink::default(), SINK_ALLOWANCE, limits()).unwrap();
        assert_eq!(
            s.profile(),
            if build == 70374 {
                PROFILE_70374
            } else {
                PROFILE_70425
            }
        );
        s.project_to_sink(&[batch(&o, 9)]).unwrap();
        let stats = s.stats();
        assert_eq!(
            (
                stats.source_requests,
                stats.omitted_requests,
                stats.no_collision_requests,
                stats.records
            ),
            (7, 4, 1, 2)
        );
        let out = ids(&s.sink().rows[0].3);
        assert_eq!(out, vec![u32::MAX - 1, u32::MAX - 1]);
        assert_eq!((stats.ids.calls, stats.ids.m2_counter_consumed), (2, 2));
        s.finish().unwrap();
        assert_eq!(e.facts().unwrap().reads, 5);
        e.finish().unwrap();
    }
}
#[test]
fn compatible_empty_omitted_batch_is_provisional_barrier() {
    let f = CollisionFixture::for_build(&[], 70374);
    let p = Private::new();
    let mut store = open(&f, 70374);
    let mut e = collector(&mut store, &p, &f, 70374);
    let b = chunk(b"MDDF", &mddf(9, 7));
    let o = parse(&b);
    let mut s = Session::new(&mut e, TestSink::default(), SINK_ALLOWANCE, limits()).unwrap();
    s.project_to_sink(&[batch(&o, 9)]).unwrap();
    assert_eq!(s.sink().rows[0].2, 0);
    assert!(s.sink().rows[0].3.is_empty());
    assert_eq!(s.stats().ids.calls, 0);
    s.finish().unwrap();
    e.finish().unwrap();
}
#[test]
fn compatible_parent_collision_doodadonly_and_overlapping_sets_keep_serial_ids() {
    for build in [70374, 70425] {
        let f = CollisionFixture::for_build(&mixed_files(), build);
        let p = Private::new();
        let mut store = open(&f, build);
        let mut e = collector(&mut store, &p, &f, build);
        let mut b = chunk(b"MODF", &modf(30, 200, 1));
        b.extend(chunk(b"MODF", &modf(40, 300, 0)));
        let o = parse(&b);
        let mut s = Session::new(&mut e, TestSink::default(), SINK_ALLOWANCE, limits()).unwrap();
        s.project_to_sink(&[batch(&o, 9)]).unwrap();
        let out = ids(&s.sink().rows[0].3);
        assert_eq!(
            out,
            vec![
                200,
                u32::MAX - 1,
                u32::MAX - 2,
                u32::MAX - 3,
                u32::MAX - 4,
                u32::MAX - 5
            ]
        );
        assert_eq!(
            (s.stats().ids.calls, s.stats().ids.m2_counter_consumed),
            (6, 5)
        );
        s.finish().unwrap();
        e.finish().unwrap();
    }
}
#[test]
fn compatible_selected_missing_children_skip_successful_ordinal() {
    let mut root = overlap_root(31, 10);
    let at = root.windows(4).position(|v| v == b"IDOM").unwrap() + 8;
    put(&mut root, at, 9);
    root.splice(at + 4..at + 4, 10u32.to_le_bytes());
    put(&mut root, at - 4, 8); // two MODI names
    // Spawn0 is missing name0, spawn1 is present name1.
    let at = root.windows(4).position(|v| v == b"DDOM").unwrap() + 8;
    put(&mut root, at + 40, 1);
    let f = CollisionFixture::for_build(
        &[
            (10, None, m2(true)),
            (30, None, root),
            (31, None, group(true, false, &[0, 1])),
        ],
        70374,
    );
    let p = Private::new();
    let mut store = open(&f, 70374);
    let mut e = collector(&mut store, &p, &f, 70374);
    let b = chunk(b"MODF", &modf(30, 200, 1));
    let o = parse(&b);
    let mut s = Session::new(&mut e, TestSink::default(), SINK_ALLOWANCE, limits()).unwrap();
    s.project_to_sink(&[batch(&o, 9)]).unwrap();
    assert_eq!(
        ids(&s.sink().rows[0].3),
        vec![200, u32::MAX - 1, u32::MAX - 2]
    );
    assert_eq!(s.stats().ids.m2_counter_consumed, 2);
    s.finish().unwrap();
    e.finish().unwrap();
}
#[test]
fn compatible_no_collision_child_and_missing_wmo_consume_no_ids() {
    let f = CollisionFixture::for_build(
        &[
            (20, None, m2(false)),
            (30, None, overlap_root(31, 20)),
            (31, None, group(true, false, &[0, 1])),
        ],
        70374,
    );
    let p = Private::new();
    let mut store = open(&f, 70374);
    let mut e = collector(&mut store, &p, &f, 70374);
    let mut b = chunk(b"MODF", &modf(9, 100, 0));
    b.extend(chunk(b"MODF", &modf(30, 200, 1)));
    let o = parse(&b);
    let mut s = Session::new(&mut e, TestSink::default(), SINK_ALLOWANCE, limits()).unwrap();
    s.project_to_sink(&[batch(&o, 9)]).unwrap();
    assert_eq!(ids(&s.sink().rows[0].3), vec![200]);
    assert_eq!(s.stats().ids.m2_counter_consumed, 0);
    s.finish().unwrap();
    e.finish().unwrap();
}
#[test]
fn compatible_missing_required_group_retains_orphan_without_placement() {
    let f = CollisionFixture::for_build(
        &[(10, None, m2(true)), (30, None, overlap_root(31, 10))],
        70425,
    );
    let p = Private::new();
    let mut store = open(&f, 70425);
    let mut e = collector(&mut store, &p, &f, 70425);
    let b = chunk(b"MODF", &modf(30, 200, 1));
    let o = parse(&b);
    let mut s = Session::new(&mut e, TestSink::default(), SINK_ALLOWANCE, limits()).unwrap();
    s.project_to_sink(&[batch(&o, 9)]).unwrap();
    assert_eq!(s.stats().omitted_requests, 1);
    assert_eq!(s.stats().ids.calls, 0);
    s.finish().unwrap();
    let h = e.model_handle(0).unwrap();
    assert_eq!(e.descriptor(h).unwrap().class(), Class::M2);
    e.with_payload(h, |p| {
        assert!(p.raw().is_some());
        Ok(())
    })
    .unwrap();
    e.finish().unwrap();
}
#[test]
fn compatible_multiple_calls_reuse_ids_but_consume_successful_m2_counter() {
    let f = CollisionFixture::for_build(&[(10, None, m2(true))], 70374);
    let p = Private::new();
    let mut store = open(&f, 70374);
    let mut e = collector(&mut store, &p, &f, 70374);
    let b = chunk(b"MDDF", &mddf(10, 7));
    let o = parse(&b);
    let mut s = Session::new(&mut e, TestSink::default(), SINK_ALLOWANCE, limits()).unwrap();
    for map in [9, 10] {
        s.project_to_sink(&[batch(&o, map)]).unwrap();
    }
    assert_eq!(ids(&s.sink().rows[0].3), ids(&s.sink().rows[1].3));
    assert_eq!(
        (s.stats().ids.entries, s.stats().ids.m2_counter_consumed),
        (1, 2)
    );
    s.finish().unwrap();
    assert_eq!(e.facts().unwrap().reads, 1);
    e.finish().unwrap();
}
#[test]
fn compatible_sink_failure_is_sticky_and_no_later_reads() {
    let f = CollisionFixture::for_build(&[(10, None, m2(true))], 70374);
    let p = Private::new();
    let mut store = open(&f, 70374);
    let mut e = collector(&mut store, &p, &f, 70374);
    let b = chunk(b"MDDF", &mddf(10, 7));
    let o = parse(&b);
    let sink = TestSink {
        fail_at: Some(0),
        ..TestSink::default()
    };
    let mut s = Session::new(&mut e, sink, SINK_ALLOWANCE, limits()).unwrap();
    let err = s.project_to_sink(&[batch(&o, 9)]).unwrap_err();
    assert_eq!(err.cause(), Cause::Sink(7));
    let before = s.stats();
    assert_eq!(s.project_to_sink(&[batch(&o, 9)]).unwrap_err(), err);
    assert_eq!(s.stats(), before);
    drop(s);
    assert!(e.finish().is_err());
}
#[test]
fn compatible_early_drop_poison_even_without_work() {
    let f = CollisionFixture::for_build(&[], 70374);
    let p = Private::new();
    let mut store = open(&f, 70374);
    let mut e = collector(&mut store, &p, &f, 70374);
    drop(Session::new(&mut e, TestSink::default(), SINK_ALLOWANCE, limits()).unwrap());
    assert!(e.finish().is_err());
}
#[test]
fn compatible_malformed_model_fails_before_any_sink_or_id() {
    let f = CollisionFixture::for_build(&[(10, None, b"broken".to_vec())], 70374);
    let p = Private::new();
    let mut store = open(&f, 70374);
    let mut e = collector(&mut store, &p, &f, 70374);
    let b = chunk(b"MDDF", &mddf(10, 7));
    let o = parse(&b);
    let mut s = Session::new(&mut e, TestSink::default(), SINK_ALLOWANCE, limits()).unwrap();
    assert!(s.project_to_sink(&[batch(&o, 9)]).is_err());
    assert_eq!(s.stats().ids.calls, 0);
    assert!(s.sink().rows.is_empty());
    drop(s);
    assert!(e.finish().is_err());
}
#[test]
fn compatible_physical_limit_precedes_acquisition() {
    let f = CollisionFixture::for_build(&[], 70374);
    let p = Private::new();
    let mut store = open(&f, 70374);
    let mut e = collector(&mut store, &p, &f, 70374);
    let b = chunk(b"MDDF", &mddf(9, 7));
    let o = parse(&b);
    let mut l = limits();
    l.max_source_requests = 0;
    let mut s = Session::new(&mut e, TestSink::default(), SINK_ALLOWANCE, l).unwrap();
    assert!(s.project_to_sink(&[batch(&o, 9)]).is_err());
    assert_eq!(s.stats().ids.calls, 0);
    drop(s);
    assert_eq!(e.request_calls(), 0);
    assert!(e.finish().is_err());
}
#[test]
fn compatible_ledger_reservation_boundary_rejects_before_reads() {
    let f = CollisionFixture::for_build(&[], 70374);
    let p = Private::new();
    let mut store = open(&f, 70374);
    let mut e = collector(&mut store, &p, &f, 70374);
    let b = chunk(b"MDDF", &mddf(9, 7));
    let o = parse(&b);
    let mut l = limits();
    l.max_memory_bytes = l.ids.max_memory_bytes + SINK_ALLOWANCE + SCRATCH;
    let mut s = Session::new(&mut e, TestSink::default(), SINK_ALLOWANCE, l).unwrap();
    assert!(s.project_to_sink(&[batch(&o, 9)]).is_err());
    drop(s);
    assert_eq!(e.request_calls(), 0);
    assert!(e.finish().is_err());
}
#[test]
fn compatible_output_limit_failure_preserves_sticky_state() {
    let f = CollisionFixture::for_build(&[(10, None, m2(true))], 70374);
    let p = Private::new();
    let mut store = open(&f, 70374);
    let mut e = collector(&mut store, &p, &f, 70374);
    let b = chunk(b"MDDF", &mddf(10, 7));
    let o = parse(&b);
    let mut l = limits();
    l.max_output_bytes = 0;
    let mut s = Session::new(&mut e, TestSink::default(), SINK_ALLOWANCE, l).unwrap();
    assert!(s.project_to_sink(&[batch(&o, 9)]).is_err());
    assert!(s.sink().rows.is_empty());
    drop(s);
    assert!(e.finish().is_err());
}
#[test]
fn compatible_source_mutation_from_sink_keeps_typed_cause_and_guard_priority() {
    struct Changing<'a>(&'a CollisionFixture);
    impl Sink for Changing<'_> {
        type Error = u8;
        fn stage(&mut self, _: Staged<'_>) -> Result<(), u8> {
            fs::write(self.0.path().join(".build.info"), b"changed").unwrap();
            Err(7)
        }
    }
    let f = CollisionFixture::for_build(&[(10, None, m2(true))], 70374);
    let p = Private::new();
    let mut store = open(&f, 70374);
    let mut e = collector(&mut store, &p, &f, 70374);
    let b = chunk(b"MDDF", &mddf(10, 7));
    let o = parse(&b);
    let mut s = Session::new(&mut e, Changing(&f), 0, limits()).unwrap();
    let err = s.project_to_sink(&[batch(&o, 9)]).unwrap_err();
    assert_eq!(err.cause(), Cause::Sink(7));
    assert_eq!(err.code(), "local_store_input_changed");
    drop(s);
}
#[test]
fn compatible_spool_mutation_from_sink_is_guarded() {
    struct Changing<'a>(&'a PathBuf);
    impl Sink for Changing<'_> {
        type Error = u8;
        fn stage(&mut self, _: Staged<'_>) -> Result<(), u8> {
            fs::set_permissions(self.0, fs::Permissions::from_mode(0o755)).unwrap();
            Ok(())
        }
    }
    let f = CollisionFixture::for_build(&[(10, None, m2(true))], 70374);
    let p = Private::new();
    let mut store = open(&f, 70374);
    let mut e = collector(&mut store, &p, &f, 70374);
    let b = chunk(b"MDDF", &mddf(10, 7));
    let o = parse(&b);
    let mut s = Session::new(&mut e, Changing(&p.path), 0, limits()).unwrap();
    assert!(s.project_to_sink(&[batch(&o, 9)]).is_err());
    drop(s);
    assert!(e.finish().is_err());
}
#[test]
fn compatible_finish_drops_sink_before_final_guards() {
    struct Changing<'a>(&'a CollisionFixture);
    impl Sink for Changing<'_> {
        type Error = u8;
        fn stage(&mut self, _: Staged<'_>) -> Result<(), u8> {
            Ok(())
        }
    }
    impl Drop for Changing<'_> {
        fn drop(&mut self) {
            fs::write(self.0.path().join(".build.info"), b"changed").unwrap();
        }
    }
    let f = CollisionFixture::for_build(&[], 70374);
    let p = Private::new();
    let mut store = open(&f, 70374);
    let mut e = collector(&mut store, &p, &f, 70374);
    let s = Session::new(&mut e, Changing(&f), 0, limits()).unwrap();
    assert_eq!(s.finish().unwrap_err().code(), "local_store_input_changed");
}

fn strict_reference(
    f: &CollisionFixture,
    build: u32,
    batches: &[Batch<'_, '_>],
    named: bool,
) -> super::super::Output {
    let mut store = open(f, build);
    if named {
        store
            .enable_names(crate::root::name::Limits::default())
            .unwrap();
    }
    let mut e =
        crate::collision::Extractor::new(&mut store, &Lookup, CollisionLimits::default()).unwrap();
    for batch in batches {
        for r in batch.objects.requests() {
            let input = super::super::input(r.target());
            e.extract(match r.source() {
                Source::M2(_) => ModelRequest::M2(input),
                Source::Wmo(_) => ModelRequest::Wmo(input),
            })
            .unwrap();
        }
    }
    super::super::build(&e.finish().unwrap(), batches, limits()).unwrap()
}
#[test]
fn compatible_complete_serial_bytes_match_unchanged_strict_projection() {
    for build in [70374, 70425] {
        let f = CollisionFixture::for_build(&mixed_files(), build);
        let p = Private::new();
        let mut b = chunk(b"MDDF", &mddf(10, 100));
        b.extend(chunk(b"MDDF", &mddf(20, 101)));
        b.extend(chunk(b"MODF", &modf(30, 200, 1)));
        b.extend(chunk(b"MODF", &modf(40, 300, 0)));
        let o = parse(&b);
        let batches = [batch(&o, 9)];
        let expected = strict_reference(&f, build, &batches, false);
        let mut store = open(&f, build);
        let mut e = collector(&mut store, &p, &f, build);
        let mut s = Session::new(&mut e, TestSink::default(), SINK_ALLOWANCE, limits()).unwrap();
        s.project_to_sink(&batches).unwrap();
        assert_eq!(s.sink().rows[0].3, expected.batches()[0].bytes());
        assert_eq!(s.stats().ids, expected.stats().ids);
        s.finish().unwrap();
        e.finish().unwrap();
    }
}
#[test]
fn compatible_global_wdt_and_cross_class_id_cache_match_strict() {
    let f = CollisionFixture::for_build(&mixed_files(), 70374);
    let p = Private::new();
    let a = chunk(b"MDDF", &mddf(10, 7));
    let oa = parse(&a);
    let mut w = modf(30, 7, 1);
    w[60..62].copy_from_slice(&0x12ffu16.to_le_bytes());
    put(&mut w, 8, (-0.0f32).to_bits());
    put(&mut w, 12, 3f32.to_bits());
    let b = chunk(b"MODF", &w);
    let ob = Objects::parse_wdt(&b, objects::Limits::default()).unwrap();
    let batches = [batch(&oa, 9), batch(&ob, 10)];
    let expected = strict_reference(&f, 70374, &batches, false);
    let mut store = open(&f, 70374);
    let mut e = collector(&mut store, &p, &f, 70374);
    let mut s = Session::new(&mut e, TestSink::default(), SINK_ALLOWANCE, limits()).unwrap();
    s.project_to_sink(&batches).unwrap();
    for (a, b) in s.sink().rows.iter().zip(expected.batches()) {
        assert_eq!(a.3, b.bytes());
    }
    assert_eq!(s.stats().ids, expected.stats().ids);
    s.finish().unwrap();
    e.finish().unwrap();
}
#[test]
fn compatible_named_routes_match_strict_without_cloning_inventory() {
    let files = [
        (10, Some(&b"Models\\Thing.m2"[..]), m2(true)),
        (30, Some(&b"World\\Thing.wmo"[..]), overlap_root(31, 10)),
        (31, None, group(true, false, &[0, 1])),
    ];
    let f = CollisionFixture::for_build(&files, 70425);
    let p = Private::new();
    let mut b = chunk(b"MMDX", b"Models\\Thing.m2\0");
    let mut m = mddf(0, 7);
    m[34..36].fill(0);
    b.extend(chunk(b"MDDF", &m));
    b.extend(chunk(b"MWMO", b"World\\Thing.wmo\0"));
    let mut w = modf(0, 8, 1);
    w[56..58].fill(0);
    b.extend(chunk(b"MODF", &w));
    let o = parse(&b);
    let batches = [batch(&o, 9)];
    let expected = strict_reference(&f, 70425, &batches, true);
    let mut store = open(&f, 70425);
    store
        .enable_names(crate::root::name::Limits::default())
        .unwrap();
    let mut e = collector(&mut store, &p, &f, 70425);
    let mut s = Session::new(&mut e, TestSink::default(), SINK_ALLOWANCE, limits()).unwrap();
    s.project_to_sink(&batches).unwrap();
    assert_eq!(s.sink().rows[0].3, expected.batches()[0].bytes());
    assert_eq!(s.stats().ids, expected.stats().ids);
    s.finish().unwrap();
    e.finish().unwrap();
}
#[test]
fn compatible_class_alias_after_omission_fails_before_projection() {
    let f = CollisionFixture::for_build(&[], 70374);
    let p = Private::new();
    let mut store = open(&f, 70374);
    let mut e = collector(&mut store, &p, &f, 70374);
    let mut b = chunk(b"MDDF", &mddf(9, 7));
    b.extend(chunk(b"MODF", &modf(9, 8, 0)));
    let o = parse(&b);
    let mut s = Session::new(&mut e, TestSink::default(), SINK_ALLOWANCE, limits()).unwrap();
    let err = s.project_to_sink(&[batch(&o, 9)]).unwrap_err();
    assert_eq!(err.code(), "collision_output_alias");
    assert_eq!(s.stats().ids.calls, 0);
    assert!(s.sink().rows.is_empty());
    drop(s);
    assert!(e.finish().is_err());
}
#[test]
fn compatible_negative_named_alias_is_fatal_before_sink() {
    let f = CollisionFixture::for_build(&[], 70374);
    let p = Private::new();
    let mut store = open(&f, 70374);
    store
        .enable_names(crate::root::name::Limits::default())
        .unwrap();
    let mut e = collector(&mut store, &p, &f, 70374);
    let mut b = chunk(b"MMDX", b"a\\item.m2\0b\\item.m2\0");
    let mut m = mddf(0, 7);
    m[34..36].fill(0);
    let mut records = m.to_vec();
    put(&mut m, 0, 1);
    records.extend_from_slice(&m);
    b.extend(chunk(b"MDDF", &records));
    let o = parse(&b);
    let mut s = Session::new(&mut e, TestSink::default(), SINK_ALLOWANCE, limits()).unwrap();
    assert!(s.project_to_sink(&[batch(&o, 9)]).is_err());
    assert_eq!(s.stats().ids.calls, 0);
    assert!(s.sink().rows.is_empty());
    drop(s);
    assert!(e.finish().is_err());
}
#[test]
fn compatible_batch_and_map_limits_precede_all_physical_reads() {
    for maps in [false, true] {
        let f = CollisionFixture::for_build(&[], 70374);
        let p = Private::new();
        let mut store = open(&f, 70374);
        let mut e = collector(&mut store, &p, &f, 70374);
        let b = chunk(b"MDDF", &[]);
        let o = parse(&b);
        let mut l = limits();
        if maps {
            l.max_maps = 0
        } else {
            l.max_batches = 0
        };
        let mut s = Session::new(&mut e, TestSink::default(), SINK_ALLOWANCE, l).unwrap();
        assert!(s.project_to_sink(&[batch(&o, 9)]).is_err());
        assert!(s.sink().rows.is_empty());
        drop(s);
        assert_eq!(e.request_calls(), 0);
        assert!(e.finish().is_err());
    }
}
#[test]
fn compatible_zero_payload_reload_allowance_accepts_only_m2() {
    let f = CollisionFixture::for_build(&mixed_files(), 70374);
    let p = Private::new();
    let mut store = open(&f, 70374);
    let mut e = Collector::new_70374(
        &mut store,
        &Lookup,
        CollisionLimits::default(),
        p.spool(
            &f,
            SpoolLimits {
                max_reloads: 0,
                ..SpoolLimits::default()
            },
        ),
    )
    .unwrap();
    let b = chunk(b"MDDF", &mddf(10, 7));
    let o = parse(&b);
    let mut s = Session::new(&mut e, TestSink::default(), SINK_ALLOWANCE, limits()).unwrap();
    s.project_to_sink(&[batch(&o, 9)]).unwrap();
    s.finish().unwrap();
    assert_eq!(e.inner.stats().unwrap().spool.reloads, 0);
    e.finish().unwrap();
}
#[test]
fn compatible_reload_limit_rejects_before_parent_id_or_sink() {
    let f = CollisionFixture::for_build(&mixed_files(), 70374);
    let p = Private::new();
    let mut store = open(&f, 70374);
    let mut e = Collector::new_70374(
        &mut store,
        &Lookup,
        CollisionLimits::default(),
        p.spool(
            &f,
            SpoolLimits {
                max_reloads: 0,
                ..SpoolLimits::default()
            },
        ),
    )
    .unwrap();
    let b = chunk(b"MODF", &modf(30, 7, 1));
    let o = parse(&b);
    let mut s = Session::new(&mut e, TestSink::default(), SINK_ALLOWANCE, limits()).unwrap();
    assert!(s.project_to_sink(&[batch(&o, 9)]).is_err());
    assert_eq!(s.stats().ids.calls, 0);
    assert!(s.sink().rows.is_empty());
    drop(s);
    assert!(e.finish().is_err());
}
#[test]
fn compatible_output_bucket_charges_old_plus_full_new_capacity() {
    let source_bytes = mddf(10, 7);
    let source = placement::Mddf::parse(&source_bytes, placement::Limits::default()).unwrap();
    let value = placement::project_m2(
        &source,
        1,
        b"a",
        placement::Context {
            map_id: 9,
            original_map_id: 9,
            global_wmo: false,
        },
        placement::Limits::default(),
    )
    .unwrap();
    let mut bytes = Vec::new();
    let mut counters = Counters::default();
    emit(value, &mut bytes, &mut counters, limits(), 39).unwrap();
    assert_eq!(bytes.capacity(), 39);
    assert_eq!(
        emit(value, &mut bytes, &mut counters, limits(), 116),
        Err(ProjectionError::Limit)
    );
    let mut bytes = Vec::new();
    let mut counters = Counters::default();
    emit(value, &mut bytes, &mut counters, limits(), 117).unwrap();
    emit(value, &mut bytes, &mut counters, limits(), 117).unwrap();
    assert_eq!(bytes.len(), 78);
}

#[test]
fn compatible_failed_acquisition_counts_only_attempted_physical_rows() {
    let f = CollisionFixture::for_build(&[(10, None, b"broken".to_vec())], 70374);
    let p = Private::new();
    let mut store = open(&f, 70374);
    let mut e = collector(&mut store, &p, &f, 70374);
    let mut records = mddf(10, 7).to_vec();
    records.extend(mddf(20, 8));
    let b = chunk(b"MDDF", &records);
    let o = parse(&b);
    let mut s = Session::new(&mut e, TestSink::default(), SINK_ALLOWANCE, limits()).unwrap();
    assert!(s.project_to_sink(&[batch(&o, 9)]).is_err());
    assert_eq!(s.stats().source_requests, 1);
    drop(s);
    assert_eq!(e.request_calls(), 1);
    assert!(e.finish().is_err());
}
#[test]
fn compatible_cumulative_physical_budget_prechecks_next_call() {
    let f = CollisionFixture::for_build(&[], 70374);
    let p = Private::new();
    let mut store = open(&f, 70374);
    let mut e = collector(&mut store, &p, &f, 70374);
    let b = chunk(b"MDDF", &mddf(9, 7));
    let o = parse(&b);
    let mut l = limits();
    l.max_source_requests = 1;
    let mut s = Session::new(&mut e, TestSink::default(), SINK_ALLOWANCE, l).unwrap();
    s.project_to_sink(&[batch(&o, 9)]).unwrap();
    assert!(s.project_to_sink(&[batch(&o, 9)]).is_err());
    assert_eq!(s.stats().source_requests, 1);
    drop(s);
    assert_eq!(e.request_calls(), 1);
    assert!(e.finish().is_err());
}
#[test]
fn compatible_id_capacity_exhaustion_is_terminal_without_stage() {
    let f = CollisionFixture::for_build(&[(10, None, m2(true))], 70374);
    let p = Private::new();
    let mut store = open(&f, 70374);
    let mut e = collector(&mut store, &p, &f, 70374);
    let b = chunk(b"MDDF", &mddf(10, 7));
    let o = parse(&b);
    let mut l = limits();
    l.ids.max_entries = 0;
    let mut s = Session::new(&mut e, TestSink::default(), SINK_ALLOWANCE, l).unwrap();
    assert!(s.project_to_sink(&[batch(&o, 9)]).is_err());
    assert!(s.sink().rows.is_empty());
    drop(s);
    assert!(e.finish().is_err());
}

#[test]
fn compatible_failed_finish_checks_source_after_owned_sink_drop() {
    struct OnDrop<'a>(&'a CollisionFixture);
    impl Sink for OnDrop<'_> {
        type Error = u8;
        fn stage(&mut self, _: Staged<'_>) -> Result<(), u8> {
            Err(7)
        }
    }
    impl Drop for OnDrop<'_> {
        fn drop(&mut self) {
            self.0.change_metadata();
        }
    }
    let f = CollisionFixture::for_build(&[(10, None, m2(true))], 70374);
    let p = Private::new();
    let mut store = open(&f, 70374);
    let mut e = collector(&mut store, &p, &f, 70374);
    let b = chunk(b"MDDF", &mddf(10, 7));
    let o = parse(&b);
    let mut s = Session::new(&mut e, OnDrop(&f), 0, limits()).unwrap();
    assert_eq!(
        s.project_to_sink(&[batch(&o, 9)]).unwrap_err().cause(),
        Cause::Sink(7)
    );
    let err = s.finish().unwrap_err();
    assert_eq!(err.cause(), Cause::Sink(7));
    assert_eq!(err.code(), "local_store_input_changed");
    assert_eq!(
        e.finish().err().unwrap().code(),
        "local_store_input_changed"
    );
}
#[test]
fn compatible_successful_finish_releases_owned_allowances_after_drop() {
    let f = CollisionFixture::for_build(&[], 70374);
    let p = Private::new();
    let mut store = open(&f, 70374);
    let mut e = collector(&mut store, &p, &f, 70374);
    let before = e.free_bytes().unwrap();
    let b = chunk(b"MDDF", &[]);
    let o = parse(&b);
    let mut s = Session::new(&mut e, TestSink::default(), SINK_ALLOWANCE, limits()).unwrap();
    s.project_to_sink(&[batch(&o, 9)]).unwrap();
    s.finish().unwrap();
    assert_eq!(e.free_bytes().unwrap(), before);
    e.finish().unwrap();
}
#[test]
fn compatible_rejected_constructor_drops_sink_before_guard_return() {
    struct OnDrop<'a>(&'a CollisionFixture);
    impl Sink for OnDrop<'_> {
        type Error = u8;
        fn stage(&mut self, _: Staged<'_>) -> Result<(), u8> {
            Ok(())
        }
    }
    impl Drop for OnDrop<'_> {
        fn drop(&mut self) {
            self.0.change_metadata();
        }
    }
    let f = CollisionFixture::for_build(&[], 70374);
    let p = Private::new();
    let mut store = open(&f, 70374);
    let mut e = collector(&mut store, &p, &f, 70374);
    let mut l = limits();
    l.max_batches = super::super::MAX_BATCHES + 1;
    let err = match Session::new(&mut e, OnDrop(&f), 0, l) {
        Ok(_) => panic!("invalid limits accepted"),
        Err(e) => e,
    };
    assert_eq!(
        err.cause(),
        Cause::Projection(ProjectionError::InvalidLimits)
    );
    assert_eq!(err.code(), "local_store_input_changed");
    assert_eq!(
        e.finish().err().unwrap().code(),
        "local_store_input_changed"
    );
}
