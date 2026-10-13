// SPDX-License-Identifier: GPL-3.0-or-later
// Modified 2026-10-12: retained authored toy-CASC helpers and spooled-world comparisons.
use super::*;
use crate::collision::spool::{Limits as SpoolLimits, ProtectedInputs, Spool};
use crate::collision::world::{MAP_FILE_DATA_ID, MATERIAL_FILE_DATA_ID, TYPE_FILE_DATA_ID};
use crate::local_store::tests::{CollisionFile, CollisionFixture};
use forever_vmap::wmo::raw;
use std::{
    fs,
    io::Write,
    os::unix::fs::{DirBuilderExt, PermissionsExt},
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

fn put(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}
fn words(values: &[u32]) -> Vec<u8> {
    values
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect()
}
fn chunk(tag: &[u8; 4], body: &[u8]) -> Vec<u8> {
    let mut out: Vec<_> = tag.iter().rev().copied().collect();
    out.extend_from_slice(&(body.len() as u32).to_le_bytes());
    out.extend_from_slice(body);
    out
}
// Literal numeric WDC5 fixture, independent of generated schema objects.
fn gameobject_db2(rows: &[(u32, u32)], copies: &[(u32, u32)]) -> Vec<u8> {
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
fn m2(faces: bool) -> Vec<u8> {
    let mut bytes = vec![0; 304];
    bytes[..4].copy_from_slice(b"MD20");
    if faces {
        for (at, value) in [(216, 3), (220, 304), (224, 3), (228, 310)] {
            put(&mut bytes, at, value);
        }
        bytes.extend_from_slice(&[0, 0, 1, 0, 2, 0]);
        bytes.extend(words(&[
            0x8000_0000,
            0,
            0,
            1f32.to_bits(),
            0,
            0,
            0,
            1f32.to_bits(),
            0,
        ]));
    }
    bytes
}
fn wmo_root(group: u32, child: Option<u32>) -> Vec<u8> {
    let mut header = [0; 64];
    put(&mut header, 4, 1);
    put(&mut header, 32, 777);
    header[60] = 4;
    let mut bytes = chunk(b"MVER", &17u32.to_le_bytes());
    bytes.extend(chunk(b"MOHD", &header));
    bytes.extend(chunk(b"GFID", &group.to_le_bytes()));
    if let Some(child) = child {
        bytes.extend(chunk(b"MODI", &child.to_le_bytes()));
        let mut spawn = [0; 40];
        put(&mut spawn, 28, 1f32.to_bits());
        put(&mut spawn, 32, 1f32.to_bits());
        bytes.extend(chunk(b"MODD", &spawn));
        let mut set = [0; 32];
        put(&mut set, 24, 1);
        bytes.extend(chunk(b"MODS", &set));
    }
    bytes
}
fn wmo_group(faces: bool, child: bool, liquid: bool) -> Vec<u8> {
    let mut header = vec![0; 68];
    put(&mut header, 56, 99);
    if liquid {
        put(&mut header, 52, 22);
    }
    if faces {
        header.extend(chunk(b"MOPY", &[8, 0]));
        header.extend(chunk(b"MOVI", &[0, 0, 1, 0, 2, 0]));
        header.extend(chunk(
            b"MOVT",
            &words(&[0, 0, 0, 1f32.to_bits(), 0, 0, 0, 1f32.to_bits(), 0]),
        ));
    }
    if child {
        header.extend(chunk(b"MODR", &[0, 0]));
    }
    chunk(b"MOGP", &header)
}

// Independent literal schema widths/array counts. No production metadata or
// decoder internals are used to construct these complete regular WDC5 files.
const MAP: &[(u16, u16)] = &[
    (32, 1),
    (32, 1),
    (32, 1),
    (32, 1),
    (32, 1),
    (32, 1),
    (32, 2),
    (8, 1),
    (8, 1),
    (8, 1),
    (16, 1),
    (16, 1),
    (16, 1),
    (16, 1),
    (16, 1),
    (8, 1),
    (32, 1),
    (16, 1),
    (8, 1),
    (16, 1),
    (32, 1),
    (32, 1),
    (32, 1),
    (32, 1),
    (32, 1),
    (32, 3),
];
const TYPE: &[(u16, u16)] = &[
    (32, 1),
    (32, 6),
    (32, 1),
    (8, 1),
    (32, 1),
    (32, 1),
    (32, 1),
    (32, 1),
    (32, 1),
    (32, 1),
    (16, 1),
    (32, 1),
    (8, 1),
    (8, 1),
    (8, 1),
    (32, 1),
    (8, 6),
    (32, 3),
    (32, 38),
    (32, 4),
    (32, 4),
];
fn offset(fields: &[(u16, u16)], index: usize) -> usize {
    fields[..index]
        .iter()
        .map(|(b, n)| usize::from(b * n / 8))
        .sum()
}
fn size(fields: &[(u16, u16)]) -> usize {
    offset(fields, fields.len())
}
fn table(
    layout: u32,
    fields: &[(u16, u16)],
    rows: &[(u32, Vec<u8>)],
    strings: &[u8],
    copies: &[(u32, u32)],
) -> Vec<u8> {
    let start = 244 + fields.len() * 28;
    let mut bytes = vec![0; start];
    bytes[..4].copy_from_slice(b"WDC5");
    for (at, value) in [
        (4, 5),
        (136, rows.len() as u32),
        (140, fields.len() as u32),
        (144, size(fields) as u32),
        (148, strings.len() as u32),
        (156, layout),
        (164, u32::MAX),
        (176, fields.len() as u32),
        (180, size(fields) as u32),
        (188, (fields.len() * 24) as u32),
        (200, 1),
        (212, start as u32),
        (216, rows.len() as u32),
        (220, strings.len() as u32),
        (228, (rows.len() * 4) as u32),
        (240, copies.len() as u32),
    ] {
        put(&mut bytes, at, value);
    }
    bytes[172..174].copy_from_slice(&4u16.to_le_bytes());
    bytes[174..176].copy_from_slice(&u16::MAX.to_le_bytes());
    for (i, &(width, count)) in fields.iter().enumerate() {
        let primitive = 244 + i * 4;
        bytes[primitive..primitive + 2].copy_from_slice(&(32 - width).to_le_bytes());
        bytes[primitive + 2..primitive + 4]
            .copy_from_slice(&(offset(fields, i) as u16).to_le_bytes());
        let col = 244 + fields.len() * 4 + i * 24;
        bytes[col..col + 2].copy_from_slice(&((offset(fields, i) * 8) as u16).to_le_bytes());
        bytes[col + 2..col + 4].copy_from_slice(&(width * count).to_le_bytes());
    }
    for (_, row) in rows {
        assert_eq!(row.len(), size(fields));
        bytes.extend(row);
    }
    bytes.extend(strings);
    for (id, _) in rows {
        bytes.extend(id.to_le_bytes());
    }
    for (new, old) in copies {
        bytes.extend(words(&[*new, *old]));
    }
    bytes
}
fn map_db2(values: &[(u32, u32, i16)], copies: &[(u32, u32)]) -> Vec<u8> {
    let mut rows = Vec::new();
    let mut strings = Vec::new();
    for (ordinal, &(id, wdt, parent)) in values.iter().enumerate() {
        let mut row = vec![0; size(MAP)];
        for (field, text) in [(0, "Synthetic"), (1, "Name")] {
            let relative =
                (values.len() - ordinal) * size(MAP) + strings.len() - offset(MAP, field);
            put(&mut row, offset(MAP, field), relative as u32);
            strings.extend(text.as_bytes());
            strings.push(0);
        }
        put(&mut row, offset(MAP, 21), wdt);
        for field in [13, 14] {
            let at = offset(MAP, field);
            row[at..at + 2].copy_from_slice(&parent.to_le_bytes());
        }
        rows.push((id, row));
    }
    table(0xD43A_FAC3, MAP, &rows, &strings, copies)
}
fn material_db2(values: &[(u32, u32)], copies: &[(u32, u32)]) -> Vec<u8> {
    let rows: Vec<_> = values
        .iter()
        .map(|(id, flags)| {
            let mut row = vec![0; 5];
            put(&mut row, 0, *flags);
            (*id, row)
        })
        .collect();
    table(0x98E5_D7AA, &[(32, 1), (8, 1)], &rows, &[], copies)
}
fn type_db2(values: &[(u32, u8)], copies: &[(u32, u32)]) -> Vec<u8> {
    let rows: Vec<_> = values
        .iter()
        .map(|(id, material)| {
            let mut row = vec![0; size(TYPE)];
            row[offset(TYPE, 14)] = *material;
            (*id, row)
        })
        .collect();
    table(0xD1EC_EEC9, TYPE, &rows, &[], copies)
}
fn files(maps: &[(u32, u32, i16)]) -> Vec<CollisionFile<'static>> {
    vec![
        (
            gameobjects::TABLE_FILE_DATA_ID,
            None,
            gameobject_db2(&[], &[]),
        ),
        (MAP_FILE_DATA_ID, None, map_db2(maps, &[])),
        (MATERIAL_FILE_DATA_ID, None, material_db2(&[(7, 2)], &[])),
        (TYPE_FILE_DATA_ID, None, type_db2(&[(22, 7)], &[])),
    ]
}
fn wdt(global: Option<u32>, tile: Option<u32>) -> Vec<u8> {
    let mut bytes = Vec::new();
    if let Some(id) = global {
        bytes.extend(wmo_adt(id, 7));
    }
    if let Some(id) = tile {
        let mut mphd = [0; 32];
        put(&mut mphd, 0, 0x200);
        bytes.extend(chunk(b"MPHD", &mphd));
        let mut main = vec![0; 32768];
        put(&mut main, 0, 1);
        bytes.extend(chunk(b"MAIN", &main));
        let mut maid = vec![0; 131072];
        put(&mut maid, 4, id);
        bytes.extend(chunk(b"MAID", &maid));
    }
    if bytes.is_empty() {
        bytes.extend(chunk(b"MVER", &99u32.to_le_bytes()));
    }
    bytes
}
fn wmo_adt(id: u32, unique: u32) -> Vec<u8> {
    let mut row = [0; 64];
    put(&mut row, 0, id);
    put(&mut row, 4, unique);
    row[56..58].copy_from_slice(&8u16.to_le_bytes());
    chunk(b"MODF", &row)
}
fn m2_adt(id: u32, unique: u32) -> Vec<u8> {
    let mut row = [0; 36];
    put(&mut row, 0, id);
    put(&mut row, 4, unique);
    row[32..34].copy_from_slice(&1024u16.to_le_bytes());
    row[34..36].copy_from_slice(&0x40u16.to_le_bytes());
    chunk(b"MDDF", &row)
}
#[derive(Default)]
struct Stage {
    placements: Vec<(run::Context, Vec<u8>)>,
    fail: bool,
}
impl run::Sink for Stage {
    fn model(&mut self, _: &crate::collision::Model) -> Result<(), run::SinkError> {
        if self.fail {
            return Err(run::SinkError::Rejected);
        }
        Ok(())
    }
    fn placements(&mut self, context: run::Context, bytes: &[u8]) -> Result<(), run::SinkError> {
        if self.fail {
            return Err(run::SinkError::Rejected);
        }
        self.placements.push((context, bytes.to_vec()));
        Ok(())
    }
}
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Private {
    base: PathBuf,
}
impl Private {
    fn new() -> Self {
        let base = std::env::temp_dir().join(format!(
            "rce-spooled-world-{}-{}",
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
            fail: false,
            actions: Vec::new(),
            on_drop: false,
        }
    }
    fn read(&self) -> Vec<(run::Context, Vec<u8>)> {
        let b = fs::read(self.base.join("placements")).unwrap();
        let mut at = 0;
        let mut out = Vec::new();
        while at < b.len() {
            let h = &b[at..at + 20];
            let n = u64::from_le_bytes(h[12..20].try_into().unwrap()) as usize;
            out.push((
                run::Context {
                    map_id: u32::from_le_bytes(h[..4].try_into().unwrap()),
                    original_map_id: u32::from_le_bytes(h[4..8].try_into().unwrap()),
                    kind: if h[8] == 0 {
                        run::Kind::Global
                    } else {
                        run::Kind::Tile { x: h[9], y: h[10] }
                    },
                    replayed: h[11] != 0,
                },
                b[at + 20..at + 20 + n].to_vec(),
            ));
            at += 20 + n;
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
    fail: bool,
    actions: Vec<PathBuf>,
    on_drop: bool,
}
impl FileSink {
    fn mutate(&self) {
        for p in &self.actions {
            fs::set_permissions(p, fs::Permissions::from_mode(0o600)).unwrap();
            fs::write(p, b"changed").unwrap();
        }
    }
}
impl Drop for FileSink {
    fn drop(&mut self) {
        if self.on_drop {
            self.mutate();
        }
    }
}
impl run::spooled::Sink for FileSink {
    type Error = u8;
    fn placements(&mut self, c: run::Context, b: &[u8]) -> Result<(), u8> {
        if !self.on_drop {
            self.mutate();
        }
        if self.fail {
            return Err(71);
        }
        let mut h = [0; 20];
        h[..4].copy_from_slice(&c.map_id.to_le_bytes());
        h[4..8].copy_from_slice(&c.original_map_id.to_le_bytes());
        if let run::Kind::Tile { x, y } = c.kind {
            h[8] = 1;
            h[9] = x;
            h[10] = y;
        }
        h[11] = u8::from(c.replayed);
        h[12..].copy_from_slice(&(b.len() as u64).to_le_bytes());
        self.file.write_all(&h).map_err(|_| 72)?;
        self.file.write_all(b).map_err(|_| 72)
    }
}
fn limits() -> Limits {
    Limits {
        run: run::SharedLimits {
            placements: crate::collision::placements::Limits {
                ids: forever_vmap::placement::ids::Limits {
                    max_memory_bytes: 1024 * 1024,
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        },
        ..Default::default()
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
fn perform(f: &CollisionFixture, limits: Limits) -> Result<(Completed, Private), Error<u8>> {
    let p = Private::new();
    let mut store = f.open(false);
    let c = startup(f, &p, &mut store);
    let out = extract(c, p.sink(), 2 * 1024 * 1024, 0, limits)?;
    Ok((out, p))
}
fn compare(f: &CollisionFixture) -> (Completed, Private) {
    let mut store = f.open(false);
    let c = gameobjects::extract_startup_fdid(&mut store, gameobjects::Limits::default()).unwrap();
    let mut stage = Stage::default();
    let old = crate::collision::world::extract(c, &mut stage, limits()).unwrap();
    let (mut new, p) = perform(f, limits()).unwrap();
    assert_eq!(p.read(), stage.placements);
    assert_eq!(
        new.startup().extraction().facts(),
        old.startup().extraction().facts()
    );
    assert_eq!(new.report().ids, old.report().run.ids);
    assert_eq!(new.report().collector_before, old.report().collector_before);
    assert_eq!(new.report().collector_after, old.report().collector_after);
    assert_eq!(new.lookup_stats(), old.lookup_stats());
    for (a, b) in new.tables().iter().zip(old.tables()) {
        assert_eq!(
            (
                a.direct_rows,
                a.copy_records,
                a.materialized_rows,
                a.retained_bytes,
                a.decode_allowance_bytes
            ),
            (
                b.direct_rows,
                b.copy_records,
                b.materialized_rows,
                b.retained_bytes,
                b.decode_allowance_bytes
            )
        );
        assert_eq!(a.source().file_data_id(), b.source().file_data_id());
        assert_eq!(a.source().content_key(), b.source().content_key());
        assert_eq!(a.source().byte_count(), b.source().byte_count());
    }
    assert_eq!(new.map_stats(), old.map_stats());
    assert_eq!(new.topology().direct_count(), old.topology().direct_count());
    for (a, b) in new.topology().all().iter().zip(old.topology().all()) {
        assert_eq!(
            (
                a.id(),
                a.parent_map_id(),
                a.wdt_file_data_id(),
                a.physical_ordinal(),
                a.depth(),
                a.is_parent()
            ),
            (
                b.id(),
                b.parent_map_id(),
                b.wdt_file_data_id(),
                b.physical_ordinal(),
                b.depth(),
                b.is_parent()
            )
        );
    }
    assert_eq!(new.topology().all().len(), old.topology().all().len());
    assert_eq!(
        new.startup().data().unwrap().lists().temp_bytes(),
        old.startup().data().lists().temp_bytes()
    );
    assert_eq!(
        new.startup().data().unwrap().lists().dtree_bytes(),
        old.startup().data().lists().dtree_bytes()
    );
    assert_eq!(
        new.startup().extraction().requests().unwrap().len(),
        old.startup().extraction().requests().len()
    );
    for (h, m) in new
        .startup()
        .extraction()
        .requests()
        .unwrap()
        .to_vec()
        .iter()
        .zip(old.startup().extraction().requests())
    {
        let descriptor = new.startup().extraction().descriptor(*h).unwrap();
        let oldmodel = &old.startup().extraction().models()[m.index()];
        assert_eq!(descriptor.class(), oldmodel.class());
        assert_eq!(descriptor.output_basename(), oldmodel.output_basename());
        let raw = new
            .extraction_mut()
            .with_payload(*h, |p| Ok(p.raw().map(<[u8]>::to_vec)))
            .unwrap();
        assert_eq!(raw.as_deref(), oldmodel.raw());
    }
    let stats = new.startup().extraction().stats().unwrap();
    assert_eq!(
        new.resident_bytes(),
        stats.compact_bytes
            + stats.repository_bytes
            + new.startup().data().unwrap().stats().resident_bytes
            + new.topology().resident_bytes()
    );
    (new, p)
}
#[test]
fn complete_empty_world_matches_native_owner() {
    let f = CollisionFixture::new(&files(&[]));
    let (out, _p) = compare(&f);
    assert_eq!(out.startup().extraction().facts().reads, 4);
    assert_eq!(
        out.tables().each_ref().map(|t| t.source().file_data_id()),
        [MAP_FILE_DATA_ID, MATERIAL_FILE_DATA_ID, TYPE_FILE_DATA_ID]
    );
    assert_eq!(out.map_stats().active_maps, 0);
}
#[test]
fn cached_startup_wmo_and_new_wmo_preserve_distinct_liquid_flags() {
    let mut fs = files(&[(1, 101, -1)]);
    fs[0].2 = gameobject_db2(&[(1, 10)], &[]);
    fs.extend([
        (10, None, wmo_root(11, None)),
        (11, None, wmo_group(true, false, true)),
        (20, None, wmo_root(21, None)),
        (21, None, wmo_group(true, false, true)),
        (101, None, wdt(Some(10), Some(201))),
        (201, None, wmo_adt(20, 8)),
    ]);
    let f = CollisionFixture::new(&fs);
    let (mut out, _p) = compare(&f);
    let hs = out.startup().extraction().requests().unwrap().to_vec();
    let flags: Vec<_> = hs
        .iter()
        .map(|h| {
            out.extraction_mut()
                .with_payload(*h, |p| {
                    Ok(
                        raw::RawWmo::parse(p.raw().unwrap(), raw::ConversionLimits::default())
                            .unwrap()
                            .groups()
                            .next()
                            .unwrap()
                            .liquid_flags(),
                    )
                })
                .unwrap()
        })
        .collect();
    assert_eq!(flags, [2, 2, 0]);
    assert_eq!(out.startup().extraction().facts().models, 2);
}
#[test]
fn cached_m2_and_immediate_parent_replay_match_bytes_and_ids() {
    let mut fs = files(&[(1, 101, -1), (2, 102, 1)]);
    fs[0].2 = gameobject_db2(&[(1, 10), (2, 10)], &[]);
    fs.extend([
        (10, None, m2(true)),
        (101, None, wdt(None, Some(201))),
        (102, None, wdt(None, None)),
        (201, None, m2_adt(10, 9)),
    ]);
    let f = CollisionFixture::new(&fs);
    let (out, _p) = compare(&f);
    assert_eq!(out.report().replayed_tiles, 1);
    assert_eq!(out.startup().extraction().facts().models, 1);
}
#[test]
fn direct_prefix_copies_and_inactive_topology_match() {
    let mut fs = files(&[(1, 0, -1)]);
    fs[1].2 = map_db2(&[(1, 0, -1)], &[(2, 1)]);
    let f = CollisionFixture::new(&fs);
    let (out, _p) = compare(&f);
    assert_eq!(out.topology().all().len(), 2);
    assert_eq!(out.topology().direct_count(), 1);
    assert!(!out.topology().get(2).unwrap().active());
}
#[test]
fn liquid_replay_matches_materializer_including_zero_self_chains_and_overwrites() {
    let mut fs = files(&[]);
    let copies = [
        (3, 1),
        (4, 3),
        (9, 99),
        (0, 0),
        (8, 0),
        (1, 4),
        (99, 2),
        (7, 7),
    ];
    fs[2].2 = material_db2(&[(1, 1), (1, 2), (2, u32::MAX)], &copies);
    fs[3].2 = type_db2(&[(22, 7), (22, 1)], &[(23, 22), (24, 23), (0, 0)]);
    let f = CollisionFixture::new(&fs);
    let p = Private::new();
    let mut store = f.open(false);
    let mut c = startup(&f, &p, &mut store);
    let prep = input::prepare(c.collector_mut(), limits()).unwrap();
    let expected = crate::collision::world::source::read_liquid_materials(&fs[2].2)
        .unwrap()
        .materialize()
        .unwrap();
    assert_eq!(
        prep.lookup
            .materials
            .iter()
            .copied()
            .collect::<std::collections::BTreeMap<_, _>>(),
        expected
    );
    assert_eq!(prep.lookup.material_id(24), Some(1));
    let retained = prep.retained;
    drop(prep);
    c.collector_mut().release_external(retained).unwrap();
    c.finish().unwrap();
}
#[test]
fn missing_each_required_table_never_returns_completed() {
    for id in [MAP_FILE_DATA_ID, MATERIAL_FILE_DATA_ID, TYPE_FILE_DATA_ID] {
        let fs: Vec<_> = files(&[]).into_iter().filter(|r| r.0 != id).collect();
        let f = CollisionFixture::new(&fs);
        assert!(perform(&f, limits()).is_err());
    }
}
#[test]
fn malformed_type_and_map_null_string_are_terminal() {
    for at in [1, 3] {
        let mut fs = files(&[(1, 0, -1)]);
        if at == 1 {
            put(&mut fs[1].2, 244 + MAP.len() * 28, 0);
        } else {
            fs[3].2.push(0);
        }
        let f = CollisionFixture::new(&fs);
        assert!(matches!(
            perform(&f, limits()).err().unwrap().cause(),
            Cause::World(super::super::Error::Db2(_))
        ));
    }
}
#[test]
fn duplicate_and_copy_only_parent_graphs_reject() {
    for (rows, copies) in [
        (vec![(1, 0, -1), (1, 0, -1)], vec![]),
        (vec![(2, 101, 1), (3, 0, -1)], vec![(1, 3)]),
        (vec![(1, 101, 2), (2, 102, 1)], vec![]),
    ] {
        let mut fs = files(&[]);
        fs[1].2 = map_db2(&rows, &copies);
        let f = CollisionFixture::new(&fs);
        assert!(matches!(
            perform(&f, limits()).err().unwrap().cause(),
            Cause::World(super::super::Error::Plan(_))
        ));
    }
}
#[test]
fn missing_wdt_rejects_after_complete_tables() {
    let f = CollisionFixture::new(&files(&[(1, 101, -1)]));
    assert!(matches!(
        perform(&f, limits()).err().unwrap().cause(),
        Cause::Run(_)
    ));
}
#[test]
fn upward_and_reduced_limits_are_terminal() {
    let f = CollisionFixture::new(&files(&[]));
    for l in [
        Limits {
            max_db2_bytes: super::super::MAX_DB2_BYTES + 1,
            ..limits()
        },
        Limits {
            max_lookup_rows: super::super::MAX_LOOKUP_ROWS + 1,
            ..limits()
        },
        Limits {
            max_decode_allowance_bytes: 1,
            ..limits()
        },
        Limits {
            max_db2_bytes: 1,
            ..limits()
        },
        Limits {
            max_table_memory_bytes: 0,
            ..limits()
        },
        Limits {
            max_lookup_rows: 0,
            ..limits()
        },
    ] {
        assert!(perform(&f, l).is_err());
    }
}
#[test]
fn typed_sink_failure_remains_nested_run_cause() {
    let mut fs = files(&[(1, 101, -1)]);
    fs.push((101, None, wdt(None, None)));
    let f = CollisionFixture::new(&fs);
    let p = Private::new();
    let mut store = f.open(false);
    let c = startup(&f, &p, &mut store);
    let mut s = p.sink();
    s.fail = true;
    let e = extract(c, s, 2 * 1024 * 1024, 0, limits()).err().unwrap();
    let Cause::Run(e) = e.cause() else {
        panic!("not run error")
    };
    let run::spooled::Cause::Bridge(e) = e.cause() else {
        panic!("not bridge error")
    };
    assert_eq!(
        e.cause(),
        crate::collision::placements::spooled::Cause::Sink(run::spooled::StageError::Sink(71))
    );
}
#[test]
fn sink_heap_charge_spans_decode_admission() {
    let f = CollisionFixture::new(&files(&[]));
    let p = Private::new();
    let mut store = f.open(false);
    let c = gameobjects::spooled::extract_startup_fdid(
        &mut store,
        gameobjects::Limits {
            collision: crate::collision::Limits {
                max_memory_bytes: 128 * 1024 * 1024,
                ..Default::default()
            },
            ..Default::default()
        },
        p.spool(&f),
    )
    .unwrap();
    assert_eq!(
        extract(c, p.sink(), 64 * 1024, 80 * 1024 * 1024, limits())
            .err()
            .unwrap()
            .code(),
        crate::collision::Error::Limit.code()
    );
}
#[test]
fn precharged_sink_is_transferred_without_double_charge() {
    let f = CollisionFixture::new(&files(&[]));
    let p = Private::new();
    let mut store = f.open(false);
    let c = gameobjects::spooled::extract_startup_fdid(
        &mut store,
        gameobjects::Limits {
            collision: crate::collision::Limits {
                max_memory_bytes: 200 * 1024 * 1024,
                ..Default::default()
            },
            ..Default::default()
        },
        p.spool(&f),
    )
    .unwrap();
    let out = extract(c, p.sink(), 64 * 1024, 112 * 1024 * 1024, limits()).unwrap();
    assert!(out.resident_bytes() < 1024 * 1024);
}
struct ChangedSink<'a> {
    inner: FileSink,
    fixture: Option<&'a CollisionFixture>,
}
impl Drop for ChangedSink<'_> {
    fn drop(&mut self) {
        if let Some(f) = self.fixture {
            f.change_metadata();
        }
    }
}
impl run::spooled::Sink for ChangedSink<'_> {
    type Error = u8;
    fn placements(&mut self, c: run::Context, b: &[u8]) -> Result<(), u8> {
        self.inner.placements(c, b)
    }
}
#[test]
fn early_pure_failure_drops_sink_before_final_source_guard() {
    let f = CollisionFixture::new(&files(&[]));
    let p = Private::new();
    let mut store = f.open(false);
    let c = startup(&f, &p, &mut store);
    let sink = ChangedSink {
        inner: p.sink(),
        fixture: Some(&f),
    };
    let e = extract(
        c,
        sink,
        64 * 1024,
        0,
        Limits {
            max_db2_bytes: super::super::MAX_DB2_BYTES + 1,
            ..limits()
        },
    )
    .err()
    .unwrap();
    assert_eq!(e.cause(), Cause::World(super::super::Error::InvalidLimits));
    assert_eq!(e.code(), "local_store_input_changed");
}
#[test]
fn pure_decoder_failure_drops_sink_before_final_spool_guard() {
    let mut fs = files(&[]);
    fs[0].2 = gameobject_db2(&[(1, 10)], &[]);
    fs.push((10, None, m2(true)));
    fs[1].2.truncate(244);
    let f = CollisionFixture::new(&fs);
    let p = Private::new();
    let mut store = f.open(false);
    let c = startup(&f, &p, &mut store);
    let mut sink = p.sink();
    sink.actions.push(p.base.join("spool/00000000.payload"));
    sink.on_drop = true;
    let e = extract(c, sink, 64 * 1024, 4096, limits()).err().unwrap();
    assert!(matches!(
        e.cause(),
        Cause::World(super::super::Error::Db2(_))
    ));
    assert_eq!(e.code(), payload::Error::Changed.code());
}
#[test]
fn lookup_transition_retains_owner_for_post_sink_drop_guard() {
    let mut fs = files(&[]);
    fs[0].2 = gameobject_db2(&[(1, 10)], &[]);
    fs.push((10, None, m2(true)));
    let f = CollisionFixture::new(&fs);
    let p = Private::new();
    let mut store = f.open(false);
    let c = startup(&f, &p, &mut store);
    let file = p.base.join("spool/00000000.payload");
    fs::set_permissions(&file, fs::Permissions::from_mode(0o600)).unwrap();
    fs::write(file, b"changed").unwrap();
    let mut sink = Some(ChangedSink {
        inner: p.sink(),
        fixture: Some(&f),
    });
    let e = transition(c, &super::super::EMPTY, &mut sink)
        .err()
        .unwrap();
    assert!(sink.is_none());
    assert_eq!(
        e.cause(),
        Cause::Collector(CollectorError::Payload(payload::Error::Changed))
    );
    assert_eq!(e.code(), "local_store_input_changed");
}
#[test]
fn typed_sink_cause_survives_both_payload_and_source_changes() {
    let mut fs = files(&[(1, 101, -1)]);
    fs[0].2 = gameobject_db2(&[(1, 10)], &[]);
    fs.extend([(10, None, m2(true)), (101, None, wdt(None, None))]);
    let f = CollisionFixture::new(&fs);
    let p = Private::new();
    let mut store = f.open(false);
    let c = startup(&f, &p, &mut store);
    let mut inner = p.sink();
    inner.fail = true;
    inner.actions.push(p.base.join("spool/00000000.payload"));
    let sink = ChangedSink {
        inner,
        fixture: Some(&f),
    };
    let e = extract(c, sink, 64 * 1024, 4096, limits()).err().unwrap();
    assert_eq!(e.code(), "local_store_input_changed");
    let Cause::Run(e) = e.cause() else {
        panic!("not run")
    };
    let run::spooled::Cause::Bridge(e) = e.cause() else {
        panic!("not bridge")
    };
    assert_eq!(
        e.cause(),
        crate::collision::placements::spooled::Cause::Sink(run::spooled::StageError::Sink(71))
    );
}
#[test]
fn completed_retains_exact_data_and_topology_charge_across_reloads() {
    let mut fs = files(&[(1, 101, -1)]);
    fs[0].2 = gameobject_db2(&[(1, 10)], &[]);
    fs.extend([
        (10, None, m2(true)),
        (101, None, wdt(None, Some(201))),
        (201, None, m2_adt(10, 9)),
    ]);
    let f = CollisionFixture::new(&fs);
    let (mut out, _p) = compare(&f);
    let expected =
        out.startup().data().unwrap().stats().resident_bytes + out.topology().resident_bytes();
    assert_eq!(
        out.startup().extraction().retained_external_for_test(),
        expected
    );
    let h = out.startup().extraction().requests().unwrap()[0];
    out.extraction_mut()
        .with_payload(h, |p| {
            assert!(p.raw().is_some());
            Ok(())
        })
        .unwrap();
    assert_eq!(
        out.startup().extraction().retained_external_for_test(),
        expected
    );
}
#[test]
fn completed_payload_change_invalidates_startup_access() {
    let mut fs = files(&[]);
    fs[0].2 = gameobject_db2(&[(1, 10)], &[]);
    fs.push((10, None, m2(true)));
    let f = CollisionFixture::new(&fs);
    let (mut out, p) = perform(&f, limits()).unwrap();
    let h = out.startup().extraction().requests().unwrap()[0];
    let file = p.base.join("spool/00000000.payload");
    fs::set_permissions(&file, fs::Permissions::from_mode(0o600)).unwrap();
    fs::write(file, b"changed").unwrap();
    assert_eq!(
        out.extraction_mut()
            .with_payload(h, |_| Ok(()))
            .unwrap_err(),
        CollectorError::Payload(payload::Error::Changed)
    );
    assert!(out.startup().data().is_err());
}
