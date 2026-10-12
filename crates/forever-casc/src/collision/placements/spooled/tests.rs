// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-12: original complete synthetic placement/spool fixtures.
use super::*;
use crate::collision::spool::{Limits as SpoolLimits, ProtectedInputs, Spool};
use crate::collision::{Extractor, InputRef, Limits as CollisionLimits};
use crate::local_store::tests::{CollisionFile, CollisionFixture};
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
            "rce-spooled-placement-{}-{}",
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
fn db2(rows: &[(u32, u32)], copies: &[(u32, u32)]) -> Vec<u8> {
    let mut bytes = vec![0; 496];
    bytes[..4].copy_from_slice(b"WDC5");
    for (at, value) in [
        (4, 5),
        (136, rows.len() as u32),
        (140, 9),
        (144, 52),
        (156, 0x16D1_DD61),
        (164, u32::MAX),
        (176, 9),
        (180, 52),
        (188, 216),
        (200, 1),
        (212, 496),
        (216, rows.len() as u32),
        (228, rows.len() as u32 * 4),
        (240, copies.len() as u32),
    ] {
        put(&mut bytes, at, value);
    }
    bytes[172..174].copy_from_slice(&4u16.to_le_bytes());
    bytes[174..176].copy_from_slice(&u16::MAX.to_le_bytes());
    for (field, (offset, bits, count)) in [
        (0u16, 32u16, 6u16),
        (24, 32, 1),
        (28, 16, 1),
        (30, 32, 1),
        (34, 32, 1),
        (38, 32, 1),
        (42, 32, 1),
        (46, 32, 1),
        (50, 16, 1),
    ]
    .into_iter()
    .enumerate()
    {
        let primitive = 244 + field * 4;
        bytes[primitive..primitive + 2].copy_from_slice(&(32 - bits).to_le_bytes());
        bytes[primitive + 2..primitive + 4].copy_from_slice(&offset.to_le_bytes());
        let col = 280 + field * 24;
        bytes[col..col + 2].copy_from_slice(&(offset * 8).to_le_bytes());
        bytes[col + 2..col + 4].copy_from_slice(&(bits * count).to_le_bytes());
    }
    for (_, fdid) in rows {
        let mut record = [0; 52];
        put(&mut record, 24, *fdid);
        bytes.extend_from_slice(&record);
    }
    for (id, _) in rows {
        bytes.extend_from_slice(&id.to_le_bytes());
    }
    for (new, old) in copies {
        bytes.extend(words(&[*new, *old]));
    }
    bytes
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
fn acquire<L: LiquidLookup + ?Sized>(e: &mut SpoolingExtractor<'_, '_, L>, o: &Objects<'_>) {
    for r in o.requests() {
        let target = super::super::input(r.target());
        e.extract(match r.source() {
            Source::M2(_) => ModelRequest::M2(target),
            Source::Wmo(_) => ModelRequest::Wmo(target),
        })
        .unwrap();
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
#[test]
fn resident_and_spooled_multicall_bytes_and_serial_ids_match() {
    let f = CollisionFixture::new(&mixed_files());
    let p = Private::new();
    let mut a = chunk(b"MDDF", &mddf(10, 100));
    a.extend(chunk(b"MDDF", &mddf(20, 101)));
    a.extend(chunk(b"MODF", &modf(30, 200, 1)));
    a.extend(chunk(b"MODF", &modf(40, 300, 0)));
    let b = chunk(b"MODF", &modf(30, 200, 9));
    let oa = parse(&a);
    let ob = parse(&b);
    let mut store = f.open(false);
    let mut resident = Extractor::new(&mut store, &Lookup, CollisionLimits::default()).unwrap();
    for o in [&oa, &ob] {
        for r in o.requests() {
            let input = super::super::input(r.target());
            resident
                .extract(match r.source() {
                    Source::M2(_) => ModelRequest::M2(input),
                    Source::Wmo(_) => ModelRequest::Wmo(input),
                })
                .unwrap();
        }
    }
    let resident = resident.finish().unwrap();
    let mut reference = super::super::Session::new(limits()).unwrap();
    let mut view = resident.view();
    view.requests = &resident.requests()[..4];
    let ra = reference.project_view(view, &[batch(&oa, 9)]).unwrap();
    let mut view = resident.view();
    view.requests = &resident.requests()[4..];
    let rb = reference.project_view(view, &[batch(&ob, 10)]).unwrap();
    let mut store = f.open(false);
    let mut e = SpoolingExtractor::new(
        &mut store,
        &Lookup,
        CollisionLimits::default(),
        p.spool(&f, SpoolLimits::default()),
    )
    .unwrap();
    acquire(&mut e, &oa);
    let mut session = Session::new(&mut e, TestSink::default(), SINK_ALLOWANCE, limits()).unwrap();
    session.project_to_sink(0..4, &[batch(&oa, 9)]).unwrap();
    session.extract(wmo_request(30)).unwrap();
    session.project_to_sink(4..5, &[batch(&ob, 10)]).unwrap();
    for (actual, expected) in session
        .sink()
        .rows
        .iter()
        .zip([&ra.batches()[0], &rb.batches()[0]])
    {
        assert_eq!(
            (actual.0, actual.1, actual.2, actual.3.as_slice()),
            (
                expected.map_id(),
                expected.original_map_id(),
                expected.record_count(),
                expected.bytes()
            )
        );
        DirBin::parse(&actual.3, placement::Limits::default()).unwrap();
    }
    assert_eq!(session.stats().ids, reference.id_stats());
    assert_eq!(session.stats().source_requests, 5);
    session.finish().unwrap();
    assert_eq!(e.stats().unwrap().spool.reloads, 3);
    e.finish().unwrap();
}
#[test]
fn m2_only_has_zero_payload_reloads_and_repeated_counter_consumption() {
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
                max_reloads: 0,
                ..SpoolLimits::default()
            },
        ),
    )
    .unwrap();
    let mut b = chunk(b"MDDF", &mddf(10, 7));
    b.extend(chunk(b"MDDF", &mddf(10, 7)));
    let o = parse(&b);
    acquire(&mut e, &o);
    let mut s = Session::new(&mut e, TestSink::default(), SINK_ALLOWANCE, limits()).unwrap();
    s.project_to_sink(0..2, &[batch(&o, 9)]).unwrap();
    assert_eq!(s.stats().ids.calls, 2);
    assert_eq!(s.stats().ids.entries, 1);
    assert_eq!(s.stats().ids.m2_counter_consumed, 2);
    s.finish().unwrap();
    assert_eq!(e.stats().unwrap().spool.reloads, 0);
    e.finish().unwrap();
}
#[test]
fn preflight_rejects_late_identity_before_any_reload_id_or_sink() {
    let f = CollisionFixture::new(&mixed_files());
    let p = Private::new();
    let mut store = f.open(false);
    let mut e = SpoolingExtractor::new(
        &mut store,
        &Lookup,
        CollisionLimits::default(),
        p.spool(&f, SpoolLimits::default()),
    )
    .unwrap();
    e.extract(wmo_request(30)).unwrap();
    e.extract(request(10)).unwrap();
    let mut b = chunk(b"MODF", &modf(30, 9, 0));
    b.extend(chunk(b"MDDF", &mddf(20, 7)));
    let o = parse(&b);
    let mut s = Session::new(&mut e, TestSink::default(), SINK_ALLOWANCE, limits()).unwrap();
    assert_eq!(
        s.project_to_sink(0..2, &[batch(&o, 9)])
            .unwrap_err()
            .cause(),
        Cause::Projection(ProjectionError::Identity)
    );
    assert_eq!(s.stats().ids.calls, 0);
    assert!(s.sink().rows.is_empty());
    drop(s);
    assert_eq!(e.stats().unwrap().spool.reloads, 0);
    assert!(e.finish().is_err());
}
#[test]
fn empty_windows_still_stage_empty_batches_without_fake_reload() {
    let f = CollisionFixture::new(&[]);
    let p = Private::new();
    let mut store = f.open(false);
    let mut e = SpoolingExtractor::new(
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
    let empty = chunk(b"MDDF", &[]);
    let o = parse(&empty);
    let mut s = Session::new(&mut e, TestSink::default(), SINK_ALLOWANCE, limits()).unwrap();
    s.project_to_sink(0..0, &[batch(&o, 9)]).unwrap();
    assert_eq!(s.sink().rows[0].2, 0);
    assert!(s.sink().rows[0].3.is_empty());
    s.finish().unwrap();
    e.finish().unwrap();
}
#[test]
fn sink_failure_keeps_first_batch_provisional_and_is_sticky() {
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
    let b = chunk(b"MDDF", &mddf(10, 7));
    let o = parse(&b);
    acquire(&mut e, &o);
    acquire(&mut e, &o);
    let mut s = Session::new(
        &mut e,
        TestSink {
            fail_at: Some(1),
            ..TestSink::default()
        },
        SINK_ALLOWANCE,
        limits(),
    )
    .unwrap();
    let err = s
        .project_to_sink(0..2, &[batch(&o, 9), batch(&o, 10)])
        .unwrap_err();
    assert_eq!(err.cause(), Cause::Sink(7));
    assert_eq!(s.sink().rows.len(), 1);
    assert_eq!(s.stats().staged_batches, 1);
    assert_eq!(
        s.project_to_sink(0..0, &[]).unwrap_err().cause(),
        Cause::Sink(7)
    );
    assert!(s.finish().is_err());
    assert!(e.finish().is_err());
}
#[test]
fn success_drops_owned_reservations_before_releasing_while_early_drop_poisons() {
    for early in [false, true] {
        let f = CollisionFixture::new(&[]);
        let p = Private::new();
        let mut store = f.open(false);
        let mut e = SpoolingExtractor::new(
            &mut store,
            &Lookup,
            CollisionLimits::default(),
            p.spool(&f, SpoolLimits::default()),
        )
        .unwrap();
        let before = e.retained_bytes().unwrap();
        let s = Session::new(&mut e, TestSink::default(), SINK_ALLOWANCE, limits()).unwrap();
        if early {
            drop(s);
            assert!(e.finish().is_err());
        } else {
            s.finish().unwrap();
            assert_eq!(e.retained_bytes().unwrap(), before);
            e.finish().unwrap();
        }
    }
}
#[test]
fn request_ranges_counts_and_class_reject_before_work() {
    for mode in 0..5 {
        let f = CollisionFixture::new(&mixed_files());
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
        let b = chunk(
            if mode == 4 { b"MODF" } else { b"MDDF" },
            &if mode == 4 {
                modf(10, 7, 0).to_vec()
            } else {
                mddf(10, 7).to_vec()
            },
        );
        let o = parse(&b);
        let window = match mode {
            0 => Range { start: 1, end: 0 },
            1 => 0..2,
            2 => 0..0,
            _ => 0..1,
        };
        let l = if mode == 3 {
            Limits {
                max_batches: 0,
                ..limits()
            }
        } else {
            limits()
        };
        let mut s = Session::new(&mut e, TestSink::default(), SINK_ALLOWANCE, l).unwrap();
        assert!(s.project_to_sink(window, &[batch(&o, 9)]).is_err());
        assert_eq!(s.stats().ids.calls, 0);
        assert!(s.sink().rows.is_empty());
    }
}
#[test]
fn cumulative_source_record_and_byte_caps_are_not_refunded() {
    for mode in 0..3 {
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
        let b = chunk(b"MDDF", &mddf(10, 7));
        let o = parse(&b);
        acquire(&mut e, &o);
        acquire(&mut e, &o);
        let l = match mode {
            0 => Limits {
                max_source_requests: 1,
                ..limits()
            },
            1 => Limits {
                max_records: 1,
                ..limits()
            },
            _ => Limits {
                max_output_bytes: 54,
                ..limits()
            },
        };
        let mut s = Session::new(&mut e, TestSink::default(), SINK_ALLOWANCE, l).unwrap();
        s.project_to_sink(0..1, &[batch(&o, 9)]).unwrap();
        assert!(s.project_to_sink(1..2, &[batch(&o, 10)]).is_err());
        assert_eq!(s.sink().rows.len(), 1);
        assert!(s.finish().is_err());
    }
}
#[test]
fn whole_reload_sum_is_admitted_before_the_first_wmo() {
    let f = CollisionFixture::new(&mixed_files());
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
    let mut b = chunk(b"MODF", &modf(30, 9, 0));
    b.extend(chunk(b"MODF", &modf(40, 10, 0)));
    let o = parse(&b);
    acquire(&mut e, &o);
    let mut s = Session::new(&mut e, TestSink::default(), SINK_ALLOWANCE, limits()).unwrap();
    assert!(s.project_to_sink(0..2, &[batch(&o, 9)]).is_err());
    assert_eq!(s.stats().ids.calls, 0);
    drop(s);
    assert_eq!(e.stats().unwrap().spool.reloads, 0);
}
#[test]
fn many_wmo_roots_total_exceeds_active_one_current_record_only() {
    let mut files: Vec<_> = (100..112)
        .map(|id| {
            let mut b = overlap_root(1000, 1001);
            b.extend(chunk(b"ZZZZ", &vec![0; 320 * 1024]));
            (id, None, b)
        })
        .collect();
    files.extend([
        (1000, None, group(true, false, &[0, 1])),
        (1001, None, m2(true)),
    ]);
    let f = CollisionFixture::new(&files);
    let p = Private::new();
    let mut store = f.open(false);
    let mut e = SpoolingExtractor::new(
        &mut store,
        &Lookup,
        CollisionLimits {
            max_memory_bytes: 3 * 1024 * 1024,
            ..CollisionLimits::default()
        },
        p.spool(
            &f,
            SpoolLimits {
                max_active_bytes: 3 * 1024 * 1024,
                ..SpoolLimits::default()
            },
        ),
    )
    .unwrap();
    let mut b = Vec::new();
    for id in 100..112 {
        b.extend(chunk(b"MODF", &modf(id, id, 1)));
    }
    let o = parse(&b);
    let reference = resident_reference(&f, &[batch(&o, 9)], false);
    let mut limited_store = f.open(false);
    let mut limited = Extractor::new(
        &mut limited_store,
        &Lookup,
        CollisionLimits {
            max_memory_bytes: 3 * 1024 * 1024,
            ..CollisionLimits::default()
        },
    )
    .unwrap();
    assert!((100..112).any(|id| limited.extract(wmo_request(id)).is_err()));
    acquire(&mut e, &o);
    assert!(e.stats().unwrap().spool.disk_bytes > 3 * 1024 * 1024);
    let mut s = Session::new(&mut e, TestSink::default(), SINK_ALLOWANCE, limits()).unwrap();
    s.project_to_sink(0..12, &[batch(&o, 9)]).unwrap();
    assert_eq!(s.stats().records, 48);
    assert_eq!(s.sink().rows[0].3, reference.batches()[0].bytes());
    assert_eq!(s.stats().ids, reference.stats().ids);
    s.finish().unwrap();
    let stats = e.stats().unwrap().spool;
    assert_eq!(stats.reloads, 12);
    assert!(stats.peak_reload_bytes > 320 * 1024);
    assert!(stats.peak_reload_bytes < 321 * 1024);
    e.finish().unwrap();
}
#[test]
fn startup_data_charge_and_cached_handles_survive_placement_session() {
    use crate::collision::gameobjects::{self, spooled as startup};
    let f = CollisionFixture::new(&[
        (gameobjects::TABLE_FILE_DATA_ID, None, db2(&[(1, 10)], &[])),
        (10, None, m2(true)),
    ]);
    let p = Private::new();
    let mut store = f.open(false);
    let mut c = startup::extract_startup_fdid(
        &mut store,
        gameobjects::Limits::default(),
        p.spool(&f, SpoolLimits::default()),
    )
    .unwrap();
    let lists = c.data().unwrap().stats();
    let before = c.collector_mut().retained_bytes().unwrap();
    let b = chunk(b"MDDF", &mddf(10, 7));
    let o = parse(&b);
    let mut s = Session::new(
        c.collector_mut(),
        TestSink::default(),
        SINK_ALLOWANCE,
        limits(),
    )
    .unwrap();
    let start = s.request_count().unwrap();
    s.extract(request(10)).unwrap();
    s.project_to_sink(start..start + 1, &[batch(&o, 9)])
        .unwrap();
    let expected = s.collector.retained_bytes().unwrap() - s.reservation;
    assert!(expected >= before);
    s.finish().unwrap();
    assert_eq!(c.collector_mut().retained_bytes().unwrap(), expected);
    assert_eq!(c.data().unwrap().stats(), lists);
    let done = c.finish().unwrap();
    assert_eq!(done.extraction().stats().unwrap().spool.reloads, 1);
}
#[test]
fn vector_old_plus_full_new_overlap_and_exact_boundary() {
    let source_bytes = mddf(10, 7);
    let value = placement::project_m2(
        &placement::Mddf::parse(&source_bytes, placement::Limits::default()).unwrap(),
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
    let mut c = Counters::default();
    emit(value, &mut bytes, &mut c, limits(), 39).unwrap();
    assert_eq!(bytes.capacity(), 39);
    assert_eq!(
        emit(value, &mut bytes, &mut c, limits(), 116),
        Err(ProjectionError::Limit)
    );
    let mut bytes = Vec::new();
    let mut c = Counters::default();
    emit(value, &mut bytes, &mut c, limits(), 117).unwrap();
    emit(value, &mut bytes, &mut c, limits(), 117).unwrap();
    assert_eq!(bytes.len(), 78);
}
#[test]
fn invalid_limits_and_insufficient_id_sink_reservation_poison() {
    for mode in 0..3 {
        let f = CollisionFixture::new(&[]);
        let p = Private::new();
        let mut store = f.open(false);
        let mut e = SpoolingExtractor::new(
            &mut store,
            &Lookup,
            CollisionLimits::default(),
            p.spool(&f, SpoolLimits::default()),
        )
        .unwrap();
        let l = match mode {
            0 => Limits {
                max_models: super::super::MAX_MODELS + 1,
                ..limits()
            },
            1 => Limits {
                max_memory_bytes: 1,
                ..limits()
            },
            _ => Limits {
                ids: ids::Limits {
                    max_entries: ids::MAX_ENTRIES + 1,
                    ..limits().ids
                },
                ..limits()
            },
        };
        assert!(Session::new(&mut e, TestSink::default(), SINK_ALLOWANCE, l).is_err());
        assert!(e.finish().is_err());
    }
}
#[test]
fn source_change_combined_with_sink_failure_preserves_original_cause() {
    struct ChangingSink<'a>(&'a CollisionFixture);
    impl Sink for ChangingSink<'_> {
        type Error = u8;
        fn stage(&mut self, _: Staged<'_>) -> Result<(), u8> {
            self.0.change_metadata();
            Err(7)
        }
    }
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
    let b = chunk(b"MDDF", &mddf(10, 7));
    let o = parse(&b);
    acquire(&mut e, &o);
    let mut s = Session::new(&mut e, ChangingSink(&f), 0, limits()).unwrap();
    let err = s.project_to_sink(0..1, &[batch(&o, 9)]).unwrap_err();
    assert_eq!(err.cause(), Cause::Sink(7));
    assert!(err.guard().is_some_and(changed));
    assert_eq!(err.code(), "local_store_input_changed");
    assert!(s.finish().is_err());
}
#[test]
fn record_replacement_before_reload_returns_no_sink_output() {
    let f = CollisionFixture::new(&mixed_files());
    let p = Private::new();
    let mut store = f.open(false);
    let mut e = SpoolingExtractor::new(
        &mut store,
        &Lookup,
        CollisionLimits::default(),
        p.spool(&f, SpoolLimits::default()),
    )
    .unwrap();
    let b = chunk(b"MODF", &modf(30, 9, 0));
    let o = parse(&b);
    acquire(&mut e, &o);
    let file = fs::read_dir(&p.path)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    fs::set_permissions(&file, fs::Permissions::from_mode(0o600)).unwrap();
    fs::write(&file, b"changed").unwrap();
    let mut s = Session::new(&mut e, TestSink::default(), SINK_ALLOWANCE, limits());
    // Constructor itself checks every completed spool identity.
    if let Ok(ref mut session) = s {
        assert!(session.project_to_sink(0..1, &[batch(&o, 9)]).is_err());
        assert!(session.sink().rows.is_empty());
    } else {
        assert!(s.is_err());
    }
}

#[test]
fn id_capacity_and_numeric_collisions_are_terminal() {
    for collision in [false, true] {
        let f = CollisionFixture::new(&mixed_files());
        let p = Private::new();
        let mut store = f.open(false);
        let mut e = SpoolingExtractor::new(
            &mut store,
            &Lookup,
            CollisionLimits::default(),
            p.spool(&f, SpoolLimits::default()),
        )
        .unwrap();
        let b = if collision {
            chunk(b"MODF", &modf(30, u32::MAX - 1, 0))
        } else {
            chunk(b"MDDF", &mddf(10, 7))
        };
        let o = parse(&b);
        acquire(&mut e, &o);
        let l = if collision {
            limits()
        } else {
            Limits {
                ids: ids::Limits {
                    max_entries: 0,
                    ..limits().ids
                },
                ..limits()
            }
        };
        let mut s = Session::new(&mut e, TestSink::default(), SINK_ALLOWANCE, l).unwrap();
        let err = s.project_to_sink(0..1, &[batch(&o, 9)]).unwrap_err();
        assert_eq!(
            err.cause(),
            Cause::Projection(ProjectionError::Ids(if collision {
                ids::Error::AssignedIdCollision
            } else {
                ids::Error::Limit
            }))
        );
        assert!(s.sink().rows.is_empty());
        assert!(s.finish().is_err());
    }
}
#[test]
fn metadata_ceiling_and_whole_record_tail_reject_before_sink() {
    for mode in 0..2 {
        let f = CollisionFixture::new(&mixed_files());
        let p = Private::new();
        let mut store = f.open(false);
        let mut e = SpoolingExtractor::new(
            &mut store,
            &Lookup,
            CollisionLimits::default(),
            p.spool(&f, SpoolLimits::default()),
        )
        .unwrap();
        let b = chunk(b"MODF", &modf(30, 7, 0));
        let o = parse(&b);
        acquire(&mut e, &o);
        let handle = e.request_handle(0).unwrap();
        let r = e.payload_shape(handle).unwrap().record_bytes;
        let mut s = Session::new(&mut e, TestSink::default(), SINK_ALLOWANCE, limits()).unwrap();
        if mode == 0 {
            s.counters.metadata = MAX_SOURCE_REQUESTS;
        } else {
            let free = s.collector.free_bytes().unwrap();
            s.collector.reserve_external(free - r + 1).unwrap();
        }
        assert!(s.project_to_sink(0..1, &[batch(&o, 9)]).is_err());
        assert_eq!(s.stats().ids.calls, 0);
        assert!(s.sink().rows.is_empty());
        drop(s);
        assert_eq!(e.stats().unwrap().spool.reloads, usize::from(mode == 0));
    }
}
#[test]
fn source_mutation_during_owned_sink_drop_is_checked_before_finish_success() {
    struct OnDrop<'a>(&'a CollisionFixture);
    impl Drop for OnDrop<'_> {
        fn drop(&mut self) {
            self.0.change_metadata();
        }
    }
    impl Sink for OnDrop<'_> {
        type Error = u8;
        fn stage(&mut self, _: Staged<'_>) -> Result<(), u8> {
            Ok(())
        }
    }
    let f = CollisionFixture::new(&[]);
    let p = Private::new();
    let mut store = f.open(false);
    let mut e = SpoolingExtractor::new(
        &mut store,
        &Lookup,
        CollisionLimits::default(),
        p.spool(&f, SpoolLimits::default()),
    )
    .unwrap();
    let s = Session::new(&mut e, OnDrop(&f), 0, limits()).unwrap();
    assert_eq!(s.finish().unwrap_err().code(), "local_store_input_changed");
    assert!(e.finish().is_err());
}

fn resident_reference(
    f: &CollisionFixture,
    batches: &[Batch<'_, '_>],
    names: bool,
) -> super::super::Output {
    let mut store = f.open(names);
    let mut e = Extractor::new(&mut store, &Lookup, CollisionLimits::default()).unwrap();
    for batch in batches {
        for request in batch.objects.requests() {
            let target = super::super::input(request.target());
            e.extract(match request.source() {
                Source::M2(_) => ModelRequest::M2(target),
                Source::Wmo(_) => ModelRequest::Wmo(target),
            })
            .unwrap();
        }
    }
    super::super::build(&e.finish().unwrap(), batches, limits()).unwrap()
}
#[test]
fn global_wdt_and_same_client_cross_class_reuse_match_resident_bits() {
    let f = CollisionFixture::new(&mixed_files());
    let p = Private::new();
    let mut store = f.open(false);
    let mut e = SpoolingExtractor::new(
        &mut store,
        &Lookup,
        CollisionLimits::default(),
        p.spool(&f, SpoolLimits::default()),
    )
    .unwrap();
    let a = chunk(b"MDDF", &mddf(10, 7));
    let oa = parse(&a);
    let mut placement = modf(30, 7, 1);
    placement[60..62].copy_from_slice(&0x12ffu16.to_le_bytes());
    put(&mut placement, 8, (-0.0f32).to_bits());
    put(&mut placement, 12, 3f32.to_bits());
    put(&mut placement, 16, (-7f32).to_bits());
    let b = chunk(b"MODF", &placement);
    let ob = Objects::parse_wdt(&b, objects::Limits::default()).unwrap();
    let batches = [batch(&oa, 9), batch(&ob, 10)];
    let reference = resident_reference(&f, &batches, false);
    acquire(&mut e, &oa);
    acquire(&mut e, &ob);
    let mut s = Session::new(&mut e, TestSink::default(), SINK_ALLOWANCE, limits()).unwrap();
    s.project_to_sink(0..2, &batches).unwrap();
    for (actual, expected) in s.sink().rows.iter().zip(reference.batches()) {
        assert_eq!(actual.3, expected.bytes());
    }
    assert_eq!(s.stats().ids, reference.stats().ids);
    let records = DirBin::parse(&s.sink().rows[1].3, placement::Limits::default()).unwrap();
    let parent = records.records().next().unwrap();
    assert_eq!(parent.name_set(), 0xff);
    assert_eq!(parent.flags() & 2, 2);
    assert_ne!(parent.position_bits(), [0, 0, 0]);
    s.finish().unwrap();
    assert_eq!(e.stats().unwrap().spool.reloads, 1);
    e.finish().unwrap();
}
#[test]
fn explicit_name_routes_match_resident_without_cloning_name_inventory() {
    let files = [
        (10, Some(&b"Models\\Thing.m2"[..]), m2(true)),
        (30, Some(&b"World\\Thing.wmo"[..]), overlap_root(31, 10)),
        (31, None, group(true, false, &[0, 1])),
    ];
    let f = CollisionFixture::new(&files);
    let p = Private::new();
    let mut store = f.open(true);
    let mut e = SpoolingExtractor::new(
        &mut store,
        &Lookup,
        CollisionLimits::default(),
        p.spool(&f, SpoolLimits::default()),
    )
    .unwrap();
    let mut a = chunk(b"MMDX", b"Models\\Thing.m2\0");
    let mut m = mddf(0, 7);
    m[34..36].fill(0);
    a.extend(chunk(b"MDDF", &m));
    a.extend(chunk(b"MWMO", b"World\\Thing.wmo\0"));
    let mut w = modf(0, 8, 1);
    w[56..58].fill(0);
    a.extend(chunk(b"MODF", &w));
    let o = parse(&a);
    let batches = [batch(&o, 9)];
    let reference = resident_reference(&f, &batches, true);
    acquire(&mut e, &o);
    let mut s = Session::new(&mut e, TestSink::default(), SINK_ALLOWANCE, limits()).unwrap();
    s.project_to_sink(0..2, &batches).unwrap();
    assert_eq!(s.sink().rows[0].3, reference.batches()[0].bytes());
    assert_eq!(s.stats().ids, reference.stats().ids);
    s.finish().unwrap();
    assert_eq!(e.stats().unwrap().spool.reloads, 1);
    e.finish().unwrap();
}
