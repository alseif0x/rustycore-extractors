// SPDX-License-Identifier: GPL-3.0-or-later
// Adapted 2026-10-12: authored synthetic actual disk-inventory runner acceptance.
use super::*;
use crate::{
    collision::{
        self,
        spool::{Limits as SpoolLimits, ProtectedInputs, Spool},
    },
    local_store::tests::{CollisionFile, CollisionFixture},
};
use forever_db2::terrain_sources::{DirectRecord, VMapFields};
use forever_vmap::placement::{self, DirBin};
type Outputs = Vec<(Context, Vec<u8>)>;
use std::{
    fs,
    io::Write,
    os::unix::fs::DirBuilderExt,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
fn put(bytes: &mut [u8], at: usize, value: u32) {
    bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
}
fn chunk(tag: &[u8; 4], body: &[u8]) -> Vec<u8> {
    let mut bytes: Vec<_> = tag.iter().rev().copied().collect();
    bytes.extend_from_slice(&(body.len() as u32).to_le_bytes());
    bytes.extend_from_slice(body);
    bytes
}
fn row(id: u32, wdt: i32, parent: i16) -> DirectRecord<VMapFields> {
    DirectRecord {
        id,
        fields: VMapFields {
            wdt_file_data_id: wdt,
            parent_map_id: parent,
            name: "Synthetic map".into(),
            directory: "Synthetic".into(),
        },
    }
}
fn maps(rows: &[DirectRecord<VMapFields>]) -> plan::Maps<'_> {
    plan::Maps::from_materialized(rows, rows.len(), plan::Limits::default()).unwrap()
}
fn wdt(tiles: &[(usize, usize, u32)], fdid: bool, global: Option<(u32, u32)>) -> Vec<u8> {
    let mut mphd = [0; 32];
    put(&mut mphd, 0, if fdid { 0x200 } else { 0 });
    let mut bytes = chunk(b"MPHD", &mphd);
    let mut main = vec![0; 32768];
    let mut maid = vec![0; 131072];
    for &(x, y, id) in tiles {
        put(&mut main, (y * 64 + x) * 8, 1);
        put(&mut maid, (y * 64 + x) * 32, 9999); // Root ADT is not acquired.
        put(&mut maid, (y * 64 + x) * 32 + 4, id);
        put(&mut maid, (y * 64 + x) * 32 + 8, 9998); // Neither is Obj1.
    }
    bytes.extend(chunk(b"MAIN", &main));
    if fdid {
        bytes.extend(chunk(b"MAID", &maid));
    }
    if let Some((id, unique)) = global {
        let mut source = [0; 64];
        put(&mut source, 0, id);
        put(&mut source, 4, unique);
        source[56..58].copy_from_slice(&8u16.to_le_bytes());
        bytes.extend(chunk(b"MODF", &source));
    }
    bytes
}
fn adt(id: u32, unique: u32) -> Vec<u8> {
    let mut record = [0; 36];
    put(&mut record, 0, id);
    put(&mut record, 4, unique);
    record[32..34].copy_from_slice(&1024u16.to_le_bytes());
    record[34..36].copy_from_slice(&0x40u16.to_le_bytes());
    chunk(b"MDDF", &record)
}
fn empty_adt() -> Vec<u8> {
    chunk(b"MVER", &18u32.to_le_bytes())
}
fn m2(collision: bool) -> Vec<u8> {
    let mut bytes = vec![0; 304];
    bytes[..4].copy_from_slice(b"MD20");
    if collision {
        put(&mut bytes, 216, 3);
        put(&mut bytes, 220, 304);
        put(&mut bytes, 224, 3);
        put(&mut bytes, 228, 310);
        bytes.extend_from_slice(&[0, 0, 1, 0, 2, 0]);
        for word in [0u32, 0, 0, 1f32.to_bits(), 0, 0, 0, 1f32.to_bits(), 0] {
            bytes.extend_from_slice(&word.to_le_bytes());
        }
    }
    bytes
}
fn wmo_files() -> Vec<CollisionFile<'static>> {
    let mut header = [0; 64];
    put(&mut header, 4, 1);
    put(&mut header, 32, 777);
    header[60] = 4;
    let mut root = chunk(b"MVER", &17u32.to_le_bytes());
    root.extend(chunk(b"MOHD", &header));
    root.extend(chunk(b"GFID", &30u32.to_le_bytes()));
    root.extend(chunk(b"MODI", &20u32.to_le_bytes()));
    let mut spawn = [0; 40];
    put(&mut spawn, 28, 1f32.to_bits());
    put(&mut spawn, 32, 1f32.to_bits());
    root.extend(chunk(b"MODD", &spawn));
    let mut set = [0; 32];
    put(&mut set, 24, 1);
    root.extend(chunk(b"MODS", &set));
    let mut group = vec![0; 68];
    put(&mut group, 56, 99);
    group.extend(chunk(b"MODR", &[0, 0]));
    group.extend(chunk(b"MOPY", &[8, 0]));
    group.extend(chunk(b"MOVI", &[0, 0, 1, 0, 2, 0]));
    let mut vertices = Vec::new();
    for word in [0u32, 0, 0, 1f32.to_bits(), 0, 0, 0, 1f32.to_bits(), 0] {
        vertices.extend_from_slice(&word.to_le_bytes());
    }
    group.extend(chunk(b"MOVT", &vertices));
    vec![
        (10, None, root),
        (20, None, m2(true)),
        (30, None, chunk(b"MOGP", &group)),
    ]
}
fn words(values: &[u32]) -> Vec<u8> {
    values.iter().flat_map(|word| word.to_le_bytes()).collect()
}
// Literal numeric WDC5 fixture, independent of generated schema objects.
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
fn fixture(rows: &[(u32, u32)], mut files: Vec<CollisionFile<'_>>) -> CollisionFixture {
    files.push((gameobjects::TABLE_FILE_DATA_ID, None, db2(rows, &[])));
    CollisionFixture::new(&files)
}
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Private {
    base: PathBuf,
}
impl Private {
    fn new() -> Self {
        let base = std::env::temp_dir().join(format!(
            "rce-disk-inventory-run-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::DirBuilder::new().mode(0o700).create(&base).unwrap();
        Self { base }
    }
    fn spool(&self, f: &CollisionFixture) -> Spool {
        Spool::create(
            &self.base.join("spool"),
            ProtectedInputs::capture(f.path(), &[]).unwrap(),
            SpoolLimits::default(),
        )
        .unwrap()
    }
    fn sink(&self) -> FileSink {
        FileSink {
            file: fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(self.base.join("placements"))
                .unwrap(),
            calls: 0,
            fail_at: None,
        }
    }
    fn read(&self) -> Outputs {
        let bytes = fs::read(self.base.join("placements")).unwrap();
        let mut rest = &bytes[..];
        let mut out = Vec::new();
        while !rest.is_empty() {
            assert!(rest.len() >= 20);
            let h = &rest[..20];
            let length = u64::from_le_bytes(h[12..20].try_into().unwrap()) as usize;
            let context = Context {
                map_id: u32::from_le_bytes(h[..4].try_into().unwrap()),
                original_map_id: u32::from_le_bytes(h[4..8].try_into().unwrap()),
                kind: if h[8] == 0 {
                    Kind::Global
                } else {
                    Kind::Tile { x: h[9], y: h[10] }
                },
                replayed: h[11] != 0,
            };
            out.push((context, rest[20..20 + length].to_vec()));
            rest = &rest[20 + length..];
        }
        out
    }
}
impl Drop for Private {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.base).unwrap();
    }
}
struct FileSink {
    file: fs::File,
    calls: usize,
    fail_at: Option<usize>,
}
impl Sink for FileSink {
    type Error = u8;
    fn placements(&mut self, c: Context, bytes: &[u8]) -> Result<(), u8> {
        if self.fail_at == Some(self.calls) {
            return Err(77);
        }
        self.calls += 1;
        let mut header = [0; 20];
        header[..4].copy_from_slice(&c.map_id.to_le_bytes());
        header[4..8].copy_from_slice(&c.original_map_id.to_le_bytes());
        if let Kind::Tile { x, y } = c.kind {
            header[8] = 1;
            header[9] = x;
            header[10] = y;
        }
        header[11] = u8::from(c.replayed);
        header[12..20].copy_from_slice(&(bytes.len() as u64).to_le_bytes());
        self.file.write_all(&header).map_err(|_| 88)?;
        self.file.write_all(bytes).map_err(|_| 88)
    }
}
#[derive(Default)]
struct Resident {
    placements: Outputs,
}
impl super::super::Sink for Resident {
    fn model(&mut self, _: &collision::Model) -> Result<(), super::super::SinkError> {
        Ok(())
    }
    fn placements(&mut self, c: Context, b: &[u8]) -> Result<(), super::super::SinkError> {
        self.placements.push((c, b.to_vec()));
        Ok(())
    }
}
fn limits() -> SharedLimits {
    SharedLimits {
        placements: placements::Limits {
            ids: ids::Limits {
                max_memory_bytes: 1024 * 1024,
                ..ids::Limits::default()
            },
            ..placements::Limits::default()
        },
        ..SharedLimits::default()
    }
}

fn startup<'a>(
    f: &CollisionFixture,
    p: &Private,
    store: &'a mut crate::local_store::LocalStore,
) -> gameobjects::spooled::Startup<'a> {
    gameobjects::spooled::extract_startup_fdid(store, gameobjects::Limits::default(), p.spool(f))
        .unwrap()
}
fn ids(bytes: &[u8]) -> Vec<u32> {
    DirBin::parse(bytes, placement::Limits::default())
        .unwrap()
        .records()
        .map(|r| r.id())
        .collect()
}
fn compare(
    f: &CollisionFixture,
    rows: &[DirectRecord<VMapFields>],
    names: bool,
) -> (Report, Outputs) {
    let mut store = f.open(names);
    let mut old =
        gameobjects::extract_startup_fdid(&mut store, gameobjects::Limits::default()).unwrap();
    let mut reference = Resident::default();
    let expected =
        super::super::extract_shared(&mut old, &maps(rows), &mut reference, limits()).unwrap();
    old.finish().unwrap();
    let p = Private::new();
    let mut store = f.open(names);
    let mut new = startup(f, &p, &mut store);
    let data = new.data().unwrap().stats();
    let (w, k) = inventories(f, &p, limits());
    let report = extract(
        &mut new,
        &maps(rows),
        w,
        k,
        p.sink(),
        0,
        Limits {
            run: limits(),
            ..Limits::default()
        },
    )
    .unwrap();
    assert_eq!(new.data().unwrap().stats(), data);
    assert_eq!(report.ids, expected.run.ids);
    assert_eq!(
        (
            report.object_adts,
            report.own_tiles,
            report.replayed_tiles,
            report.empty_replays,
            report.cached_tiles,
            report.placement_callbacks,
            report.placement_records,
            report.placement_bytes
        ),
        (
            expected.run.object_adts,
            expected.run.own_tiles,
            expected.run.replayed_tiles,
            expected.run.empty_replays,
            expected.run.cached_tiles,
            expected.run.placement_callbacks,
            expected.run.placement_records,
            expected.run.placement_bytes
        )
    );
    assert_eq!(
        (
            report.model_reads,
            report.model_source_bytes,
            report.model_raw_bytes
        ),
        (
            expected.run.model_reads,
            expected.run.model_source_bytes,
            expected.run.model_raw_bytes as usize
        )
    );
    assert_eq!(report.collector_before, expected.collector_before);
    assert_eq!(report.collector_after, expected.collector_after);
    assert_eq!(
        (report.request_calls_before, report.request_calls_after),
        (expected.request_calls_before, expected.request_calls_after)
    );
    let actual = p.read();
    assert_eq!(actual, reference.placements);
    for (_, bytes) in &actual {
        DirBin::parse(bytes, placement::Limits::default()).unwrap();
    }
    new.finish().unwrap();
    (report, actual)
}
fn inventories(f: &CollisionFixture, p: &Private, l: SharedLimits) -> (Inventory, Inventory) {
    fn one(f: &CollisionFixture, p: &Private, name: &str, l: SharedLimits) -> Inventory {
        Inventory::new(
            Spool::create(
                &p.base.join(name),
                ProtectedInputs::capture(f.path(), &[]).unwrap(),
                SpoolLimits::default(),
            )
            .unwrap(),
            inventory::Limits {
                max_heap_bytes: 256 * 1024 * 1024,
                wdt: l.wdt,
                cache: l.cache,
                ..inventory::Limits::default()
            },
        )
        .unwrap()
    }
    (one(f, p, "wdts", l), one(f, p, "caches", l))
}
fn run<L: LiquidLookup + ?Sized, S: Sink>(
    f: &CollisionFixture,
    p: &Private,
    c: &mut gameobjects::spooled::Context<'_, '_, L>,
    rows: &[DirectRecord<VMapFields>],
    sink: S,
    l: SharedLimits,
) -> Result<Report, Error<S::Error>> {
    let (w, k) = inventories(f, p, l);
    extract(
        c,
        &maps(rows),
        w,
        k,
        sink,
        0,
        Limits {
            run: l,
            ..Limits::default()
        },
    )
}
#[test]
fn current_windows_and_cached_startup_models_match_resident_two_tiles() {
    let f = fixture(
        &[(1, 10)],
        vec![
            (10, None, m2(true)),
            (101, None, wdt(&[(0, 0, 201), (0, 1, 202)], true, None)),
            (201, None, adt(10, 7)),
            (202, None, adt(10, 8)),
        ],
    );
    let (r, out) = compare(&f, &[row(1, 101, -1)], false);
    assert_eq!(r.model_reads, 0);
    assert_eq!(r.ids.calls, 2);
    assert_eq!(ids(&out[1].1), [u32::MAX - 1]);
    assert_eq!(ids(&out[2].1), [u32::MAX - 2]);
}
#[test]
fn dependency_handles_and_wmo_globals_match_without_reloading_child() {
    let mut files = wmo_files();
    files.push((101, None, wdt(&[], true, Some((10, 8)))));
    let f = fixture(&[(1, 10)], files);
    let (r, out) = compare(&f, &[row(1, 101, -1)], false);
    assert_eq!(r.model_reads, 0);
    assert_eq!(r.placement_records, 2);
    assert_eq!(ids(&out[0].1), [8, u32::MAX - 1]);
}
#[test]
fn parent_own_empty_and_nonempty_replay_do_not_populate_grandchild() {
    let f = fixture(
        &[],
        vec![
            (101, None, wdt(&[(2, 1, 201), (3, 1, 202)], true, None)),
            (102, None, wdt(&[], true, None)),
            (103, None, wdt(&[], true, None)),
            (201, None, adt(10, 7)),
            (202, None, empty_adt()),
            (10, None, m2(true)),
        ],
    );
    let (r, out) = compare(
        &f,
        &[row(3, 103, 2), row(2, 102, 1), row(1, 101, -1)],
        false,
    );
    assert_eq!(
        (
            r.own_tiles,
            r.replayed_tiles,
            r.empty_replays,
            r.cached_tiles
        ),
        (2, 2, 1, 2)
    );
    assert_eq!(r.ids.calls, 1);
    let tile: Vec<_> = out
        .iter()
        .filter(|r| matches!(r.0.kind, Kind::Tile { .. }))
        .collect();
    assert_eq!(tile.len(), 4);
    assert_eq!(tile[2].1[0] & 2, 2);
    assert!(tile[3].1.is_empty());
}
#[test]
fn own_empty_blocks_fallback_and_is_cached_for_its_child() {
    let f = fixture(
        &[],
        vec![
            (101, None, wdt(&[(0, 0, 201)], true, None)),
            (102, None, wdt(&[(0, 0, 202)], true, None)),
            (103, None, wdt(&[], true, None)),
            (201, None, adt(10, 7)),
            (202, None, empty_adt()),
            (10, None, m2(true)),
        ],
    );
    let (r, out) = compare(
        &f,
        &[row(1, 101, -1), row(2, 102, 1), row(3, 103, 2)],
        false,
    );
    assert_eq!((r.cached_tiles, r.ids.calls), (2, 1));
    let tile: Vec<_> = out
        .iter()
        .filter(|r| matches!(r.0.kind, Kind::Tile { .. }))
        .collect();
    assert!(!tile[1].0.replayed);
    assert!(tile[2].0.replayed);
    assert_eq!(tile[2].0.original_map_id, 2);
    assert!(tile[2].1.is_empty());
}
#[test]
fn physical_globals_precede_depth_order_and_are_not_inherited() {
    let mut files = wmo_files();
    files.extend([
        (101, None, wdt(&[(0, 0, 201)], true, Some((10, 10)))),
        (102, None, wdt(&[], true, None)),
        (201, None, adt(20, 11)),
    ]);
    let f = fixture(&[], files);
    let (r, out) = compare(&f, &[row(2, 102, 1), row(1, 101, -1)], false);
    assert_eq!(out[0].0.map_id, 2);
    assert!(out[0].1.is_empty());
    assert_eq!(ids(&out[1].1), [10, u32::MAX - 1]);
    assert_eq!(r.ids.calls, 3);
}
#[test]
fn no_collision_startup_needs_no_id_and_matches_empty_placement() {
    let f = fixture(
        &[(1, 10)],
        vec![
            (10, None, m2(false)),
            (101, None, wdt(&[(0, 0, 201)], true, None)),
            (201, None, adt(10, 3)),
        ],
    );
    let (r, out) = compare(&f, &[row(1, 101, -1)], false);
    assert_eq!(r.ids.calls, 0);
    assert!(out[1].1.is_empty());
}
#[test]
fn ninth_parent_tile_is_disk_backed_and_replays_exactly() {
    let mut files = vec![
        (
            101,
            None,
            wdt(
                &(0..9).map(|x| (x, 0, 201 + x as u32)).collect::<Vec<_>>(),
                true,
                None,
            ),
        ),
        (102, None, wdt(&[], true, None)),
        (10, None, m2(true)),
    ];
    for i in 0..9 {
        files.push((201 + i, None, adt(10, 7 + i)));
    }
    let f = fixture(&[], files);
    let (r, _) = compare(&f, &[row(1, 101, -1), row(2, 102, 1)], false);
    assert_eq!((r.cached_tiles, r.replayed_tiles, r.ids.calls), (9, 9, 9));
    assert_eq!(r.cache.entries, 9);
    assert_eq!(r.cache.repository.records, 9);
    assert_eq!(r.cache.repository.reloads, 9);
}
#[test]
fn empty_world_releases_every_run_charge_but_preserves_data() {
    let f = fixture(&[], vec![]);
    let (r, out) = compare(&f, &[], false);
    assert!(out.is_empty());
    assert_eq!(r.ids.calls, 0);
}
#[test]
fn missing_requested_own_adt_never_falls_back_to_parent() {
    let f = fixture(
        &[],
        vec![
            (101, None, wdt(&[(0, 0, 201)], true, None)),
            (102, None, wdt(&[(0, 0, 999)], true, None)),
            (201, None, empty_adt()),
        ],
    );
    let p = Private::new();
    let mut store = f.open(false);
    let mut c = startup(&f, &p, &mut store);
    assert!(
        run(
            &f,
            &p,
            &mut c,
            &[row(1, 101, -1), row(2, 102, 1)],
            p.sink(),
            limits()
        )
        .is_err()
    );
    assert!(c.data().is_err());
    assert!(c.finish().is_err());
}
#[test]
fn malformed_late_adt_keeps_only_provisional_prefix() {
    let f = fixture(
        &[],
        vec![
            (101, None, wdt(&[(0, 0, 201), (0, 1, 202)], true, None)),
            (201, None, empty_adt()),
            (202, None, b"bad".to_vec()),
        ],
    );
    let p = Private::new();
    let mut store = f.open(false);
    let mut c = startup(&f, &p, &mut store);
    assert!(run(&f, &p, &mut c, &[row(1, 101, -1)], p.sink(), limits()).is_err());
    assert_eq!(p.read().len(), 2);
    assert!(c.finish().is_err());
}
#[test]
fn missing_late_wdt_fails_before_any_callback() {
    let f = fixture(&[], vec![(101, None, wdt(&[], true, None))]);
    let p = Private::new();
    let mut store = f.open(false);
    let mut c = startup(&f, &p, &mut store);
    assert!(
        run(
            &f,
            &p,
            &mut c,
            &[row(1, 101, -1), row(2, 999, -1)],
            p.sink(),
            limits()
        )
        .is_err()
    );
    assert!(p.read().is_empty());
    assert!(c.finish().is_err());
}
#[test]
fn sink_error_retains_typed_cause_and_poison() {
    let f = fixture(&[], vec![(101, None, wdt(&[], true, None))]);
    let p = Private::new();
    let mut store = f.open(false);
    let mut c = startup(&f, &p, &mut store);
    let mut sink = p.sink();
    sink.fail_at = Some(0);
    let err = run(&f, &p, &mut c, &[row(1, 101, -1)], sink, limits()).unwrap_err();
    assert_eq!(err.code(), "collision_disk_run_sink");
    assert!(
        matches!(err.cause(),Cause::Bridge(e)if matches!(e.cause(),bridge::Cause::Sink(StageError::Sink(77))))
    );
    assert!(c.finish().is_err());
}
#[test]
fn source_change_during_callback_and_owned_sink_drop_are_final_errors() {
    struct Changed<'a> {
        file: FileSink,
        fixture: &'a CollisionFixture,
        on_drop: bool,
    }
    impl Sink for Changed<'_> {
        type Error = u8;
        fn placements(&mut self, c: Context, b: &[u8]) -> Result<(), u8> {
            self.file.placements(c, b)?;
            if !self.on_drop {
                self.fixture.change_metadata();
            }
            Ok(())
        }
    }
    impl Drop for Changed<'_> {
        fn drop(&mut self) {
            if self.on_drop {
                self.fixture.change_metadata();
            }
        }
    }
    for on_drop in [false, true] {
        let f = fixture(&[], vec![(101, None, wdt(&[], true, None))]);
        let p = Private::new();
        let mut store = f.open(false);
        let mut c = startup(&f, &p, &mut store);
        let sink = Changed {
            file: p.sink(),
            fixture: &f,
            on_drop,
        };
        let err = run(&f, &p, &mut c, &[row(1, 101, -1)], sink, limits()).unwrap_err();
        assert_eq!(err.code(), "local_store_input_changed");
        assert!(c.finish().is_err());
    }
}

#[test]
fn named_obj0_route_matches_resident_and_requires_explicit_activation() {
    let f = fixture(
        &[],
        vec![
            (101, None, wdt(&[(0, 0, 0)], false, None)),
            (
                201,
                Some(&b"World\\Maps\\Synthetic\\Synthetic_0_0_obj0.adt"[..]),
                empty_adt(),
            ),
        ],
    );
    let rows = [row(1, 101, -1)];
    let (r, _) = compare(&f, &rows, true);
    assert_eq!(r.object_adts, 1);
    let p = Private::new();
    let mut store = f.open(false);
    let mut c = startup(&f, &p, &mut store);
    assert!(run(&f, &p, &mut c, &rows, p.sink(), limits()).is_err());
    assert!(c.finish().is_err());
}

#[test]
fn combined_admission_rejects_before_any_source_or_callback() {
    for upward in [false, true] {
        let f = fixture(&[], vec![(101, None, wdt(&[], true, None))]);
        let p = Private::new();
        let mut store = f.open(false);
        let mut c = startup(&f, &p, &mut store);
        let before = c.collector_mut().facts().unwrap();
        let (w, k) = inventories(&f, &p, limits());
        let cap = if upward {
            MAX_COMBINED_MEMORY_BYTES + 1
        } else {
            1
        };
        let err = extract(
            &mut c,
            &maps(&[row(1, 101, -1)]),
            w,
            k,
            p.sink(),
            0,
            Limits {
                max_combined_memory_bytes: cap,
                run: limits(),
            },
        )
        .unwrap_err();
        assert!(matches!(
            err.cause(),
            Cause::Run(super::super::Error::Limit | super::super::Error::InvalidLimits)
        ));
        assert!(p.read().is_empty());
        assert_eq!(before.reads, 1);
        assert!(c.finish().is_err());
    }
}
#[test]
fn combined_ceiling_exact_boundary_and_one_byte_below() {
    for under in [false, true] {
        let f = fixture(&[], vec![]);
        let p = Private::new();
        let mut store = f.open(false);
        let mut c = startup(&f, &p, &mut store);
        let (w, k) = inventories(&f, &p, limits());
        let exact = c.collector_mut().memory_ceiling()
            + w.limits().max_heap_bytes
            + k.limits().max_heap_bytes;
        let result = extract(
            &mut c,
            &maps(&[]),
            w,
            k,
            p.sink(),
            0,
            Limits {
                run: limits(),
                max_combined_memory_bytes: exact - usize::from(under),
            },
        );
        if under {
            assert_eq!(
                result.unwrap_err().code(),
                super::super::Error::Limit.code()
            );
            assert!(c.finish().is_err());
        } else {
            assert_eq!(result.unwrap().combined_reserved_bytes, exact);
            c.finish().unwrap();
        }
    }
}
#[test]
fn fresh_inventory_is_required_and_existing_cache_is_not_reinterpreted() {
    let f = fixture(&[], vec![]);
    let p = Private::new();
    let mut store = f.open(false);
    let mut c = startup(&f, &p, &mut store);
    let (w, mut k) = inventories(&f, &p, limits());
    k.insert_own_tile(
        Tile {
            map_id: 1,
            x: 0,
            y: 0,
        },
        &[],
    )
    .unwrap();
    assert_eq!(
        extract(
            &mut c,
            &maps(&[]),
            w,
            k,
            p.sink(),
            0,
            Limits {
                run: limits(),
                ..Limits::default()
            }
        )
        .unwrap_err()
        .code(),
        super::super::Error::State.code()
    );
    assert!(p.read().is_empty());
    assert!(c.finish().is_err());
}
#[test]
fn complete_wdt_bodies_exceed_collector_ceiling_with_exact_resident_outputs() {
    let mut wire = wdt(&[], true, None);
    wire.extend(chunk(b"ZZZZ", &vec![0; 512 * 1024]));
    let f = fixture(&[], (101..121).map(|id| (id, None, wire.clone())).collect());
    let rows: Vec<_> = (1..21).map(|id| row(id, 100 + id as i32, -1)).collect();
    let mut store = f.open(false);
    let mut old =
        gameobjects::extract_startup_fdid(&mut store, gameobjects::Limits::default()).unwrap();
    let mut reference = Resident::default();
    let expected =
        super::super::extract_shared(&mut old, &maps(&rows), &mut reference, limits()).unwrap();
    old.finish().unwrap();
    let p = Private::new();
    let mut store = f.open(false);
    let mut c = gameobjects::spooled::extract_startup_fdid(
        &mut store,
        gameobjects::Limits {
            collision: collision::Limits {
                max_memory_bytes: 8 * 1024 * 1024,
                ..collision::Limits::default()
            },
            ..gameobjects::Limits::default()
        },
        p.spool(&f),
    )
    .unwrap();
    let result = run(&f, &p, &mut c, &rows, p.sink(), limits()).unwrap();
    assert_eq!(p.read(), reference.placements);
    assert_eq!(result.ids, expected.run.ids);
    assert_eq!(result.collector_after, expected.collector_after);
    assert_eq!(result.wdt.entries, 20);
    assert!(result.wdt.repository.disk_bytes > 8 * 1024 * 1024);
    assert_eq!(result.wdt.repository.reloads, 40);
    assert!(result.wdt.repository.peak_reload_bytes < 1024 * 1024);
    c.finish().unwrap();
}
#[test]
fn independent_wdt_cache_changes_on_owned_sink_drop_reject_success() {
    struct Mutating {
        sink: FileSink,
        path: PathBuf,
    }
    impl Sink for Mutating {
        type Error = u8;
        fn placements(&mut self, c: Context, b: &[u8]) -> Result<(), u8> {
            self.sink.placements(c, b)
        }
    }
    impl Drop for Mutating {
        fn drop(&mut self) {
            use std::os::unix::fs::PermissionsExt;
            let p = self.path.join("00000000.payload");
            fs::set_permissions(&p, fs::Permissions::from_mode(0o600)).unwrap();
            fs::write(p, b"changed").unwrap();
        }
    }
    for namespace in ["wdts", "caches"] {
        let f = fixture(
            &[],
            vec![
                (101, None, wdt(&[(0, 0, 201)], true, None)),
                (102, None, wdt(&[], true, None)),
                (201, None, empty_adt()),
            ],
        );
        let p = Private::new();
        let mut store = f.open(false);
        let mut c = startup(&f, &p, &mut store);
        let err = run(
            &f,
            &p,
            &mut c,
            &[row(1, 101, -1), row(2, 102, 1)],
            Mutating {
                sink: p.sink(),
                path: p.base.join(namespace),
            },
            limits(),
        )
        .unwrap_err();
        assert_eq!(err.code(), payload::Error::Changed.code());
        assert_eq!(
            if namespace == "wdts" {
                err.guards().wdt
            } else {
                err.guards().cache
            },
            Some(inventory::Error::Spool(payload::Error::Changed))
        );
        assert!(c.finish().is_err());
    }
}
#[test]
fn reduced_original_component_policies_are_enforced_before_acquisition() {
    for which in 0..5 {
        let f = fixture(&[], vec![(101, None, wdt(&[], true, None))]);
        let p = Private::new();
        let mut store = f.open(false);
        let mut c = startup(&f, &p, &mut store);
        let (w, k) = inventories(&f, &p, limits());
        let mut l = limits();
        match which {
            0 => l.max_wdt_memory_bytes = 1,
            1 => l.max_cache_memory_bytes = 1,
            2 => l.wdt.max_memory_bytes = 0,
            3 => l.cache.max_memory_bytes = 0,
            _ => l.max_tile_positions = 4095,
        };
        assert!(
            extract(
                &mut c,
                &maps(&[row(1, 101, -1)]),
                w,
                k,
                p.sink(),
                0,
                Limits {
                    run: l,
                    ..Limits::default()
                }
            )
            .is_err()
        );
        assert!(p.read().is_empty());
        assert!(c.finish().is_err());
    }
}

#[test]
fn sorted_parent_key_lookup_covers_maps_empty_cells_and_last_cell() {
    let f = fixture(&[], vec![]);
    let p = Private::new();
    let (_, mut k) = inventories(&f, &p, limits());
    for tile in [
        Tile {
            map_id: 2,
            x: 63,
            y: 63,
        },
        Tile {
            map_id: 1,
            x: 7,
            y: 3,
        },
        Tile {
            map_id: 1,
            x: 0,
            y: 0,
        },
    ] {
        k.insert_own_tile(tile, &[]).unwrap();
    }
    assert_eq!(k.next_own_tile(0, 0).unwrap(), None);
    assert_eq!(
        k.next_own_tile(1, 0).unwrap(),
        Some(Tile {
            map_id: 1,
            x: 0,
            y: 0
        })
    );
    assert_eq!(
        k.next_own_tile(1, 1).unwrap(),
        Some(Tile {
            map_id: 1,
            x: 7,
            y: 3
        })
    );
    assert_eq!(k.next_own_tile(1, 452).unwrap(), None);
    assert_eq!(
        k.next_own_tile(2, 4095).unwrap(),
        Some(Tile {
            map_id: 2,
            x: 63,
            y: 63
        })
    );
    assert_eq!(k.next_own_tile(2, 4096).unwrap(), None);
    assert_eq!(k.stats().unwrap().repository.reloads, 0);
    assert_eq!(k.next_own_tile(2, 4097), Err(inventory::Error::Binding));
    assert!(k.failed());
}
#[test]
fn absent_ranges_preserve_interleaved_own_parent_and_boundary_order() {
    let f = fixture(
        &[],
        vec![
            (
                101,
                None,
                wdt(
                    &[(0, 0, 201), (2, 3, 202), (7, 7, 203), (63, 63, 204)],
                    true,
                    None,
                ),
            ),
            (
                102,
                None,
                wdt(&[(1, 1, 205), (2, 3, 206), (62, 63, 207)], true, None),
            ),
            (201, None, adt(10, 1)),
            (202, None, adt(10, 2)),
            (203, None, empty_adt()),
            (204, None, adt(10, 4)),
            (205, None, adt(10, 5)),
            (206, None, empty_adt()),
            (207, None, adt(10, 7)),
            (10, None, m2(true)),
        ],
    );
    let (r, out) = compare(&f, &[row(2, 102, 1), row(1, 101, -1)], false);
    assert_eq!(
        (r.cached_tiles, r.replayed_tiles, r.empty_replays),
        (4, 3, 1)
    );
    assert_eq!(r.cache.repository.reloads, 3);
    let child: Vec<_> = out
        .iter()
        .filter(|(c, _)| c.map_id == 2 && matches!(c.kind, Kind::Tile { .. }))
        .collect();
    assert_eq!(
        child.iter().map(|v| v.0.kind).collect::<Vec<_>>(),
        [
            Kind::Tile { x: 0, y: 0 },
            Kind::Tile { x: 1, y: 1 },
            Kind::Tile { x: 2, y: 3 },
            Kind::Tile { x: 7, y: 7 },
            Kind::Tile { x: 62, y: 63 },
            Kind::Tile { x: 63, y: 63 }
        ]
    );
    assert!(!child[2].0.replayed);
    assert!(child[2].1.is_empty());
}
#[test]
fn inactive_parent_error_still_precedes_empty_cache_lookup() {
    let f = fixture(
        &[],
        vec![
            (102, None, wdt(&[(0, 0, 201)], true, None)),
            (201, None, empty_adt()),
        ],
    );
    let rows = [row(1, 0, -1), row(2, 102, 1)];
    let mut store = f.open(false);
    let mut old =
        gameobjects::extract_startup_fdid(&mut store, gameobjects::Limits::default()).unwrap();
    let mut reference = Resident::default();
    let expected =
        super::super::extract_shared(&mut old, &maps(&rows), &mut reference, limits()).unwrap_err();
    assert_eq!(
        expected,
        super::super::Error::Plan(plan::Error::ParentUnavailable)
    );
    assert!(old.finish().is_err());
    let p = Private::new();
    let mut store = f.open(false);
    let mut c = startup(&f, &p, &mut store);
    assert_eq!(
        run(&f, &p, &mut c, &rows, p.sink(), limits())
            .unwrap_err()
            .code(),
        plan::Error::ParentUnavailable.code()
    );
    assert_eq!(p.read(), reference.placements);
    assert!(c.finish().is_err());
}
#[test]
fn sink_cause_survives_both_inventory_changes_after_drop() {
    struct Change {
        sink: FileSink,
        base: PathBuf,
    }
    impl Sink for Change {
        type Error = u8;
        fn placements(&mut self, c: Context, b: &[u8]) -> Result<(), u8> {
            self.sink.placements(c, b)?;
            if matches!(c.kind, Kind::Tile { .. }) && c.replayed {
                Err(77)
            } else {
                Ok(())
            }
        }
    }
    impl Drop for Change {
        fn drop(&mut self) {
            use std::os::unix::fs::PermissionsExt;
            for dir in ["wdts", "caches"] {
                let p = self.base.join(dir).join("00000000.payload");
                fs::set_permissions(&p, fs::Permissions::from_mode(0o600)).unwrap();
                fs::write(p, b"changed").unwrap();
            }
        }
    }
    let f = fixture(
        &[],
        vec![
            (101, None, wdt(&[(0, 0, 201)], true, None)),
            (102, None, wdt(&[], true, None)),
            (201, None, empty_adt()),
        ],
    );
    let p = Private::new();
    let mut store = f.open(false);
    let mut c = startup(&f, &p, &mut store);
    let err = run(
        &f,
        &p,
        &mut c,
        &[row(1, 101, -1), row(2, 102, 1)],
        Change {
            sink: p.sink(),
            base: p.base.clone(),
        },
        limits(),
    )
    .unwrap_err();
    assert_eq!(err.code(), payload::Error::Changed.code());
    assert!(
        matches!(err.cause(),Cause::Bridge(e) if matches!(e.cause(),bridge::Cause::Sink(StageError::Sink(77))))
    );
    assert_eq!(
        err.guards().wdt,
        Some(inventory::Error::Spool(payload::Error::Changed))
    );
    assert_eq!(
        err.guards().cache,
        Some(inventory::Error::Spool(payload::Error::Changed))
    );
    assert!(c.finish().is_err());
}
#[test]
fn refused_combined_budget_drops_sink_before_source_guard() {
    struct Change<'a> {
        f: &'a CollisionFixture,
    }
    impl Sink for Change<'_> {
        type Error = u8;
        fn placements(&mut self, _: Context, _: &[u8]) -> Result<(), u8> {
            panic!("not admitted")
        }
    }
    impl Drop for Change<'_> {
        fn drop(&mut self) {
            self.f.change_metadata();
        }
    }
    let f = fixture(&[], vec![]);
    let p = Private::new();
    let mut store = f.open(false);
    let mut c = startup(&f, &p, &mut store);
    let (w, k) = inventories(&f, &p, limits());
    let e = extract(
        &mut c,
        &maps(&[]),
        w,
        k,
        Change { f: &f },
        0,
        Limits {
            max_combined_memory_bytes: 0,
            run: limits(),
        },
    )
    .unwrap_err();
    assert!(matches!(e.cause(), Cause::Run(super::super::Error::Limit)));
    assert_eq!(e.code(), "local_store_input_changed");
    assert!(c.finish().is_err());
}
#[test]
fn independent_inventory_empty_parent_ranges_emit_no_replays() {
    let f = fixture(
        &[],
        vec![
            (101, None, wdt(&[], true, None)),
            (102, None, wdt(&[], true, None)),
        ],
    );
    let (r, out) = compare(&f, &[row(1, 101, -1), row(2, 102, 1)], false);
    assert_eq!(r.cached_tiles, 0);
    assert_eq!(r.replayed_tiles, 0);
    assert_eq!(out.len(), 2);
    assert_eq!(r.cache.repository.reloads, 0);
    assert_eq!(r.wdt.repository.reloads, 4);
}
