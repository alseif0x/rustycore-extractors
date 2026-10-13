// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-12: private placement disk phase and closed toy-CASC acceptance.
use super::*;
use crate::collision::spool::{Limits as SpoolLimits, ProtectedInputs, Spool};
use crate::collision::world::{MAP_FILE_DATA_ID, MATERIAL_FILE_DATA_ID, TYPE_FILE_DATA_ID};
use crate::local_store::tests::{CollisionFile, CollisionFixture};
use std::{
    fs,
    os::unix::fs::{DirBuilderExt, PermissionsExt},
    path::{Path, PathBuf},
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

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Private {
    base: PathBuf,
    source: PathBuf,
}
impl Private {
    fn new() -> Self {
        let base = std::env::temp_dir().join(format!(
            "rce-closed-placement-stage-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::DirBuilder::new().mode(0o700).create(&base).unwrap();
        let source = base.join("source");
        fs::DirBuilder::new().mode(0o700).create(&source).unwrap();
        Self { base, source }
    }
    fn spool(&self, source: &Path, name: &str, limits: SpoolLimits) -> Spool {
        Spool::create(
            &self.base.join(name),
            ProtectedInputs::capture(source, &[]).unwrap(),
            limits,
        )
        .unwrap()
    }
    fn stage(&self, limits: super::Limits) -> super::Stage {
        super::Stage::new(
            self.spool(&self.source, "placements", SpoolLimits::default()),
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
fn global(map: u32) -> run::Context {
    run::Context {
        map_id: map,
        original_map_id: map,
        kind: run::Kind::Global,
        replayed: false,
    }
}
fn tile(map: u32, parent: Option<u32>, x: u8, y: u8) -> run::Context {
    run::Context {
        map_id: map,
        original_map_id: parent.unwrap_or(map),
        kind: run::Kind::Tile { x, y },
        replayed: parent.is_some(),
    }
}
fn row(flags: u8, id: u32, name: &[u8]) -> Vec<u8> {
    let mut b = vec![flags, 7];
    b.extend(id.to_le_bytes());
    for x in [1f32, 2., 3., 0., 0., 0., 1.] {
        b.extend(x.to_bits().to_le_bytes());
    }
    if flags & 1 != 0 {
        for x in [0f32, 0., 0., 1., 1., 1.] {
            b.extend(x.to_bits().to_le_bytes());
        }
    }
    b.extend((name.len() as u32).to_le_bytes());
    b.extend(name);
    b
}
fn reduced_world() -> world::inventory::Limits {
    let mut l = world::inventory::Limits::default();
    l.world.run.placements.ids.max_memory_bytes = 1024 * 1024;
    l
}
fn acquire(f: &CollisionFixture, p: &Private, limits: super::Limits) -> Result<Staged, Error> {
    acquire_selected(f, p, plan::MapSelection::All, limits)
}
fn acquire_selected(
    f: &CollisionFixture,
    p: &Private,
    selection: plan::MapSelection,
    limits: super::Limits,
) -> Result<Staged, Error> {
    let mut store = f.open(false);
    let startup = gameobjects::spooled::extract_startup_fdid(
        &mut store,
        gameobjects::Limits::default(),
        p.spool(f.path(), "models", SpoolLimits::default()),
    )
    .unwrap();
    let inv = |name: &str| {
        inventory::Inventory::new(
            p.spool(f.path(), name, SpoolLimits::default()),
            inventory::Limits {
                max_heap_bytes: 256 * 1024 * 1024,
                wdt: limits.world.world.run.wdt,
                cache: limits.world.world.run.cache,
                ..Default::default()
            },
        )
        .unwrap()
    };
    super::extract_selected(
        startup,
        inv("wdts"),
        inv("caches"),
        p.spool(f.path(), "placements", SpoolLimits::default()),
        selection,
        limits,
    )
}
#[derive(Default)]
struct Resident {
    placements: Vec<(run::Context, Vec<u8>)>,
}
impl run::Sink for Resident {
    fn model(&mut self, _: &crate::collision::Model) -> Result<(), run::SinkError> {
        Ok(())
    }
    fn placements(&mut self, c: run::Context, b: &[u8]) -> Result<(), run::SinkError> {
        self.placements.push((c, b.to_vec()));
        Ok(())
    }
}
fn compare(f: &CollisionFixture) -> (Staged, Private) {
    let limits = super::Limits {
        world: reduced_world(),
        ..Default::default()
    };
    let mut store = f.open(false);
    let startup =
        gameobjects::extract_startup_fdid(&mut store, gameobjects::Limits::default()).unwrap();
    let mut resident = Resident::default();
    let old = world::extract(startup, &mut resident, limits.world.world).unwrap();
    let p = Private::new();
    let mut new = acquire(f, &p, limits).unwrap();
    let mut expected_maps: Vec<_> = old
        .topology()
        .all()
        .iter()
        .filter(|m| m.active())
        .map(|m| m.id())
        .collect();
    expected_maps.sort_unstable();
    assert_eq!(new.maps().unwrap().collect::<Vec<_>>(), expected_maps);
    for map in expected_maps {
        let bytes: Vec<_> = resident
            .placements
            .iter()
            .filter(|(c, _)| c.map_id == map)
            .flat_map(|(_, b)| b.iter().copied())
            .collect();
        let actual = new
            .with_map(map, bytes.len(), |v| Ok::<_, ()>(v.bytes().to_vec()))
            .unwrap();
        assert_eq!(actual, bytes);
    }
    assert_eq!(
        new.world().startup().extraction().facts(),
        old.startup().extraction().facts()
    );
    assert_eq!(new.world().report().ids, old.report().run.ids);
    for (a, b) in new.world().tables().iter().zip(old.tables()) {
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
        assert_eq!(
            (
                a.source().file_data_id(),
                a.source().content_key(),
                a.source().byte_count()
            ),
            (
                b.source().file_data_id(),
                b.source().content_key(),
                b.source().byte_count()
            )
        );
    }
    assert_eq!(
        new.world().topology().all().len(),
        old.topology().all().len()
    );
    for (a, b) in new
        .world()
        .topology()
        .all()
        .iter()
        .zip(old.topology().all())
    {
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
    assert_eq!(
        new.world().startup().data().unwrap().lists().temp_bytes(),
        old.startup().data().lists().temp_bytes()
    );
    assert_eq!(
        new.world().startup().data().unwrap().lists().dtree_bytes(),
        old.startup().data().lists().dtree_bytes()
    );
    assert_eq!(
        new.stats().unwrap().callbacks as u64,
        old.report().run.placement_callbacks
    );
    assert_eq!(
        new.stats().unwrap().records as u64,
        old.report().run.placement_records
    );
    assert_eq!(
        new.stats().unwrap().placement_bytes as u64,
        old.report().run.placement_bytes
    );
    assert!(new.stats().unwrap().peak_admitted_heap_bytes <= MAX_HEAP_BYTES);
    (new, p)
}
#[test]
fn typed_envelopes_preserve_duplicate_ids_names_flags_and_callback_sequence() {
    let p = Private::new();
    let mut s = p.stage(super::Limits::default());
    let a = row(1, 9, b"a.wmo");
    let b = row(0, 9, b"different.m2x");
    let c = row(2, 10, b"parent.m2x");
    s.append(global(9), &a).unwrap();
    s.append(global(2), &[]).unwrap();
    s.append(tile(9, None, 0, 63), &b).unwrap();
    s.append(tile(9, Some(2), 1, 0), &c).unwrap();
    assert_eq!(s.read_map(9, 0).unwrap(), [a.clone(), b, c].concat());
    assert_eq!(
        s.entries.iter().map(|e| e.sequence).collect::<Vec<_>>(),
        [1, 0, 2, 3]
    );
    let mut h = b"RCEPLC1\0".to_vec();
    h.extend(9u32.to_le_bytes());
    h.extend(9u32.to_le_bytes());
    h.extend([0; 4]);
    h.extend(0u64.to_le_bytes());
    h.extend(1u64.to_le_bytes());
    h.extend((a.len() as u64).to_le_bytes());
    h.extend([0; 4]);
    assert_eq!(
        fs::read(p.base.join("placements/00000000.payload")).unwrap(),
        [h, a].concat()
    );
    assert_eq!(s.stats().unwrap().maps, 2);
    assert_eq!(s.stats().unwrap().records, 3);
}
#[test]
fn map_grouping_sorts_globals_then_tiles_without_global_id_deduplication() {
    let p = Private::new();
    let mut s = p.stage(super::Limits::default());
    for (c, b) in [
        (global(8), row(0, 7, b"global")),
        (global(1), vec![]),
        (tile(1, None, 63, 63), vec![]),
        (tile(8, None, 9, 0), row(0, 7, b"nine")),
        (tile(8, None, 1, 0), row(0, 7, b"one")),
    ] {
        s.append(c, &b).unwrap();
    }
    assert_eq!(s.read_map(1, 0).unwrap(), []);
    assert_eq!(
        DirBin::parse(&s.read_map(8, 0).unwrap(), placement::Limits::default())
            .unwrap()
            .records()
            .map(|r| r.name().to_vec())
            .collect::<Vec<_>>(),
        [b"global".to_vec(), b"one".to_vec(), b"nine".to_vec()]
    );
    assert_eq!(s.stats().unwrap().callbacks, 5);
}
#[test]
fn duplicate_callback_is_terminal_even_when_empty() {
    let p = Private::new();
    let mut s = p.stage(super::Limits::default());
    s.append(global(1), &[]).unwrap();
    assert_eq!(s.append(global(1), &[]), Err(StageError::Duplicate));
    assert_eq!(s.read_map(1, 0), Err(StageError::Duplicate));
}
#[test]
fn invalid_declared_contexts_and_parent_bits_reject_without_writes() {
    let cases = [
        (tile(1, None, 64, 0), vec![]),
        (tile(1, Some(1), 0, 0), vec![]),
        (
            run::Context {
                original_map_id: 2,
                ..global(1)
            },
            vec![],
        ),
        (
            run::Context {
                original_map_id: 2,
                replayed: true,
                ..global(1)
            },
            vec![],
        ),
        (tile(1, Some(2), 0, 0), row(0, 1, b"x")),
        (tile(1, None, 0, 0), row(2, 1, b"x")),
    ];
    for (c, b) in cases {
        let p = Private::new();
        let mut s = p.stage(super::Limits::default());
        assert_eq!(s.append(c, &b), Err(StageError::Binding));
        assert_eq!(s.repository.stats().unwrap().records, 0);
    }
}
#[test]
fn incomplete_or_trailing_record_is_fatal_after_prior_success() {
    for b in [vec![0], [row(0, 1, b"x"), vec![0]].concat()] {
        let p = Private::new();
        let mut s = p.stage(super::Limits::default());
        s.append(global(1), &[]).unwrap();
        assert!(matches!(
            s.append(tile(1, None, 0, 0), &b),
            Err(StageError::Placement(_))
        ));
        assert!(s.check().is_err());
        assert_eq!(s.repository.stats().unwrap().records, 1);
    }
}
#[test]
fn empty_callbacks_count_against_capacity_and_cumulative_record_cap_is_separate() {
    let p = Private::new();
    let mut s = p.stage(super::Limits {
        max_callbacks: 1,
        ..Default::default()
    });
    s.append(global(1), &[]).unwrap();
    assert_eq!(s.append(global(2), &[]), Err(StageError::Limit));
    let p = Private::new();
    let mut s = p.stage(super::Limits {
        max_records: 1,
        ..Default::default()
    });
    s.append(global(1), &row(0, 1, b"x")).unwrap();
    assert_eq!(
        s.append(global(2), &row(0, 2, b"x")),
        Err(StageError::Limit)
    );
}
#[test]
fn complete_map_extent_and_record_caps_apply_across_all_callbacks() {
    for records in [false, true] {
        let p = Private::new();
        let mut l = super::Limits::default();
        if records {
            l.records.max_records = 1;
        } else {
            l.records.max_bytes = row(0, 1, b"x").len();
        }
        let mut s = p.stage(l);
        s.append(global(1), &row(0, 1, b"x")).unwrap();
        s.append(tile(1, None, 0, 0), &row(0, 2, b"x")).unwrap();
        assert_eq!(s.read_map(1, 0), Err(StageError::Limit));
    }
}
#[test]
fn heap_reloads_include_complete_output_envelope_and_callback_allowance() {
    let p = Private::new();
    let mut s = p.stage(super::Limits::default());
    let b = row(0, 1, b"x");
    s.append(global(1), &b).unwrap();
    let cap = s.heap().unwrap() + b.len() * 2 + HEADER + 17;
    s.limits.max_heap_bytes = cap;
    assert_eq!(s.read_map(1, 17).unwrap(), b);
    assert_eq!(s.peak, cap);
    s.limits.max_heap_bytes = cap - 1;
    assert_eq!(s.read_map(1, 17), Err(StageError::Limit));
}
#[test]
fn descriptor_growth_old_and_new_capacities_are_both_admitted() {
    let p = Private::new();
    let mut s = p.stage(super::Limits::default());
    s.append(global(1), &[]).unwrap();
    s.limits.max_heap_bytes = s.heap().unwrap() + 2 * size_of::<Entry>() - 1;
    assert_eq!(s.append(global(2), &[]), Err(StageError::Limit));
    assert_eq!(s.entries.len(), 1);
    assert_eq!(s.repository.stats().unwrap().records, 1);
}
#[test]
fn private_repository_disk_active_and_reload_caps_remain_independent() {
    for axis in 0..3 {
        let p = Private::new();
        let mut l = SpoolLimits::default();
        if axis == 0 {
            l.max_disk_bytes = HEADER - 1;
        } else if axis == 1 {
            l.max_active_bytes = HEADER - 1;
        } else {
            l.max_reloads = 0;
        }
        let mut s = super::Stage::new(
            p.spool(&p.source, "placements", l),
            super::Limits::default(),
        )
        .unwrap();
        if axis < 2 {
            assert!(s.append(global(1), &[]).is_err());
        } else {
            s.append(global(1), &[]).unwrap();
            assert!(s.read_map(1, 0).is_err());
        }
    }
}
#[test]
fn initial_nonempty_repository_and_upward_stage_limits_reject() {
    let p = Private::new();
    let mut repo = p.spool(&p.source, "placements", SpoolLimits::default());
    repo.write(0, |_| Ok(())).unwrap();
    assert!(matches!(
        super::Stage::new(repo, super::Limits::default()),
        Err(StageError::State)
    ));
    for axis in 0..3 {
        let p = Private::new();
        let mut l = super::Limits::default();
        match axis {
            0 => l.max_heap_bytes += 1,
            1 => l.max_callbacks += 1,
            _ => l.max_records += 1,
        };
        assert!(matches!(
            super::Stage::new(p.spool(&p.source, "placements", SpoolLimits::default()), l),
            Err(StageError::InvalidLimits)
        ));
    }
}
fn mutate(path: &Path) {
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
    fs::write(path, b"changed").unwrap();
}
#[test]
fn payload_mutation_hardlink_and_directory_rebinding_are_sticky() {
    for axis in 0..3 {
        let p = Private::new();
        let mut s = p.stage(super::Limits::default());
        s.append(global(1), &[]).unwrap();
        let root = p.base.join("placements");
        let payload = root.join("00000000.payload");
        match axis {
            0 => mutate(&payload),
            1 => fs::hard_link(&payload, p.base.join("alias")).unwrap(),
            _ => {
                fs::rename(&root, p.base.join("old")).unwrap();
                fs::DirBuilder::new().mode(0o700).create(&root).unwrap();
            }
        }
        assert_eq!(
            s.read_map(1, 0),
            Err(StageError::Spool(payload::Error::Changed))
        );
        assert_eq!(
            s.append(global(2), &[]),
            Err(StageError::Spool(payload::Error::Changed))
        );
    }
}
#[test]
fn stored_envelope_binds_every_context_sequence_count_extent_and_reserved_byte() {
    let p = Private::new();
    let mut s = p.stage(super::Limits::default());
    let b = row(0, 3, b"x");
    s.append(global(9), &b).unwrap();
    let e = s.entries[0];
    let valid = [e.header().to_vec(), b].concat();
    assert!(e.body(&valid, s.limits.records).is_ok());
    for at in [0, 8, 12, 16, 17, 18, 19, 20, 28, 36, 44, 47] {
        let mut bad = valid.clone();
        bad[at] ^= 1;
        assert_eq!(e.body(&bad, s.limits.records), Err(StageError::Binding));
    }
    assert_eq!(
        e.body(&valid[..valid.len() - 1], s.limits.records),
        Err(StageError::Binding)
    );
}
#[test]
fn complete_empty_authenticated_world_has_closed_empty_stage() {
    let f = CollisionFixture::new(&files(&[]));
    let (mut c, _p) = compare(&f);
    assert_eq!(c.maps().unwrap().count(), 0);
    assert_eq!(c.stats().unwrap().callbacks, 0);
    assert!(c.verify_unchanged().is_ok());
}
#[test]
fn authenticated_parent_empty_own_and_global_order_match_resident_world() {
    let mut input = files(&[(3, 103, 2), (2, 102, 1), (1, 101, -1)]);
    input.extend([
        (101, None, wdt(None, Some(111))),
        (102, None, wdt(None, None)),
        (103, None, wdt(None, None)),
        (111, None, m2_adt(201, 91)),
        (201, None, m2(true)),
    ]);
    let f = CollisionFixture::new(&input);
    let (mut c, _p) = compare(&f);
    assert_eq!(c.stats().unwrap().maps, 3);
    assert_eq!(c.world().report().replayed_tiles, 1);
    assert_eq!(
        c.with_map(3, 0, |v| Ok::<_, ()>(v.record_count())).unwrap(),
        0
    );
}
#[test]
fn startup_and_later_wmo_use_same_model_cache_across_liquid_lookup_transition() {
    let mut input = files(&[(2, 102, -1), (1, 101, -1)]);
    input[0].2 = gameobject_db2(&[(7, 301)], &[]);
    input.extend([
        (101, None, wdt(Some(301), Some(111))),
        (102, None, wdt(Some(303), None)),
        (111, None, wmo_adt(303, 42)),
        (301, None, wmo_root(302, Some(201))),
        (302, None, wmo_group(true, true, true)),
        (303, None, wmo_root(304, None)),
        (304, None, wmo_group(true, false, true)),
        (201, None, m2(true)),
    ]);
    let f = CollisionFixture::new(&input);
    let (c, _p) = compare(&f);
    assert!(c.stats().unwrap().records >= 3);
    assert_eq!(c.world().report().maps, 2);
}
#[test]
fn direct_prefix_inactive_topology_and_copies_remain_closed() {
    let mut input = files(&[(1, 101, -1), (4, 0, -1)]);
    input[1].2 = map_db2(&[(1, 101, -1), (4, 0, -1)], &[(2, 1)]);
    input.push((101, None, wdt(None, None)));
    let f = CollisionFixture::new(&input);
    let (c, _p) = compare(&f);
    assert_eq!(c.maps().unwrap().collect::<Vec<_>>(), [1, 2]);
    assert_eq!(c.world().topology().all().len(), 3);
}
#[test]
fn missing_required_table_and_late_wdt_never_return_a_staged_owner() {
    for axis in 0..4 {
        let p = Private::new();
        let mut input = files(&[(1, 101, -1), (2, 102, -1)]);
        input.push((101, None, wdt(None, None)));
        if axis < 3 {
            input.remove(axis + 1);
        }
        let f = CollisionFixture::new(&input);
        let result = acquire(
            &f,
            &p,
            super::Limits {
                world: reduced_world(),
                ..Default::default()
            },
        );
        assert!(result.is_err());
        assert!(!p.base.join("vmaps").exists()); // only private repositories can exist
    }
}
#[test]
fn combined_ceiling_includes_stage_before_acquisition_and_can_only_reduce_collector() {
    let f = CollisionFixture::new(&files(&[]));
    for delta in [0, 1] {
        let p = Private::new();
        let mut l = super::Limits {
            world: reduced_world(),
            ..Default::default()
        };
        l.world.max_combined_memory_bytes =
            l.max_heap_bytes + 512 * 1024 * 1024 + 1024 * 1024 * 1024 - delta;
        let combined = l.world.max_combined_memory_bytes;
        let stage_heap = l.max_heap_bytes;
        let result = acquire(&f, &p, l).unwrap();
        assert_eq!(
            result.world().report().combined_reserved_bytes,
            combined - stage_heap
        );
        assert_eq!(
            result.world().report().combined_reserved_bytes - 512 * 1024 * 1024,
            1024 * 1024 * 1024 - delta
        );
    }
    let p = Private::new();
    let mut l = super::Limits {
        world: reduced_world(),
        ..Default::default()
    };
    l.world.max_combined_memory_bytes = l.max_heap_bytes + 512 * 1024 * 1024 - 1;
    assert_eq!(
        acquire(&f, &p, l).err().unwrap().code(),
        StageError::Limit.code()
    );
}
struct MutateOnDrop(PathBuf);
impl Drop for MutateOnDrop {
    fn drop(&mut self) {
        mutate(&self.0);
    }
}
fn one_map() -> CollisionFixture {
    let mut f = files(&[(1, 101, -1)]);
    f.push((101, None, wdt(None, None)));
    CollisionFixture::new(&f)
}
#[test]
fn callback_capture_drop_runs_before_final_stage_guards_and_typed_cause_survives() {
    let f = one_map();
    let (mut c, p) = compare(&f);
    let action = MutateOnDrop(p.base.join("placements/00000000.payload"));
    let err = c
        .with_map(1, 0, move |_| {
            drop(action);
            Err::<(), _>(73u8)
        })
        .unwrap_err();
    assert_eq!(err.cause, AccessCause::Callback(73));
    assert_eq!(
        err.guards.stage,
        Some(StageError::Spool(payload::Error::Changed))
    );
    assert_eq!(err.code(), payload::Error::Changed.code());
    assert!(c.stats().is_err());
}
#[test]
fn refused_reload_drops_callback_capture_before_guarding() {
    let f = one_map();
    let (mut c, p) = compare(&f);
    let action = MutateOnDrop(p.base.join("placements/00000000.payload"));
    let err = c
        .with_map(1, usize::MAX, move |_| {
            drop(action);
            Ok::<_, u8>(())
        })
        .unwrap_err();
    assert!(matches!(
        err.cause,
        AccessCause::Access(Cause::Stage(StageError::Limit))
    ));
    assert_eq!(
        err.guards.stage,
        Some(StageError::Spool(payload::Error::Changed))
    );
}
#[test]
fn returned_owned_value_is_dropped_before_rechecking_model_guard_failure() {
    let f = one_map();
    let (mut c, p) = compare(&f);
    let stage_path = p.base.join("placements/00000000.payload");
    // Empty startup/world has no model payload, so mutate its private root.
    let model_root = p.base.join("models");
    let err = c.with_map(1, 0, move |_| {
        fs::rename(&model_root, model_root.with_extension("old")).unwrap();
        fs::DirBuilder::new()
            .mode(0o700)
            .create(&model_root)
            .unwrap();
        Ok::<_, u8>(MutateOnDrop(stage_path))
    });
    let err = match err {
        Err(e) => e,
        Ok(_) => panic!("changed model root admitted"),
    };
    assert_eq!(
        err.guards.stage,
        Some(StageError::Spool(payload::Error::Changed))
    );
    assert!(err.guards.collector.is_some());
    assert!(c.verify_unchanged().is_err());
}
#[test]
fn absent_map_is_terminal_and_does_not_masquerade_as_empty_success() {
    let f = one_map();
    let (mut c, _p) = compare(&f);
    assert_eq!(
        c.with_map(2, 0, |_| Ok::<_, u8>(())).unwrap_err().code(),
        StageError::Missing.code()
    );
    assert!(c.maps().is_err());
}
#[test]
fn many_map_bodies_exceed_stage_heap_while_each_complete_map_is_bounded() {
    let p = Private::new();
    let repository = p.spool(
        &p.source,
        "placements",
        SpoolLimits {
            max_descriptor_bytes: 512 * 1024,
            ..Default::default()
        },
    );
    let mut s = super::Stage::new(
        repository,
        super::Limits {
            max_heap_bytes: 3 * 1024 * 1024,
            ..Default::default()
        },
    )
    .unwrap();
    let bytes = row(0, 99, b"same.m2x").repeat(8192);
    for map in 0..24 {
        s.append(global(map), &bytes).unwrap();
    }
    assert!(s.stats().unwrap().placement_bytes > 2 * s.limits.max_heap_bytes);
    for map in 0..24 {
        assert_eq!(s.read_map(map, 0).unwrap(), bytes);
    }
    assert!(s.stats().unwrap().peak_admitted_heap_bytes <= s.limits.max_heap_bytes);
    assert_eq!(s.repository.stats().unwrap().records, 24);
}
#[test]
fn typed_callback_failure_is_sticky_without_any_filesystem_change() {
    let f = one_map();
    let (mut c, _p) = compare(&f);
    let err = c.with_map(1, 0, |_| Err::<(), _>(91u8)).unwrap_err();
    assert_eq!(err.cause, AccessCause::Callback(91));
    assert_eq!(err.code(), StageError::Callback.code());
    assert!(c.maps().is_err());
    assert!(c.with_map(1, 0, |_| Ok::<_, u8>(())).is_err());
}
#[test]
fn private_stage_failure_keeps_world_typed_sink_cause_and_no_final_package() {
    let mut input = files(&[(1, 101, -1), (2, 102, -1)]);
    input.extend([(101, None, wdt(None, None)), (102, None, wdt(None, None))]);
    let f = CollisionFixture::new(&input);
    let p = Private::new();
    let e = acquire(
        &f,
        &p,
        super::Limits {
            max_callbacks: 1,
            world: reduced_world(),
            ..Default::default()
        },
    )
    .err()
    .unwrap();
    assert_eq!(e.guards.stage, Some(StageError::Limit));
    let Cause::World(world_error) = e.cause else {
        panic!("world cause lost")
    };
    let world::inventory::Cause::Run(run_error) = world_error else {
        panic!("run cause lost")
    };
    let run::inventory::Cause::Bridge(bridge_error) = run_error.cause() else {
        panic!("bridge cause lost")
    };
    assert_eq!(
        bridge_error.cause(),
        crate::collision::placements::spooled::Cause::Sink(run::inventory::StageError::Sink(
            StageError::Limit
        ))
    );
    assert!(!p.base.join("vmaps").exists());
}
#[test]
fn invalid_admission_still_checks_actual_source_and_each_private_repository() {
    let f = one_map();
    let p = Private::new();
    let mut store = f.open(false);
    let startup = gameobjects::spooled::extract_startup_fdid(
        &mut store,
        gameobjects::Limits::default(),
        p.spool(f.path(), "models", SpoolLimits::default()),
    )
    .unwrap();
    let inv = |name: &str| {
        inventory::Inventory::new(
            p.spool(f.path(), name, SpoolLimits::default()),
            inventory::Limits {
                max_heap_bytes: 256 * 1024 * 1024,
                ..Default::default()
            },
        )
        .unwrap()
    };
    let wdts = inv("wdts");
    let caches = inv("caches");
    let repo = p.spool(f.path(), "placements", SpoolLimits::default());
    for name in ["models", "wdts", "caches", "placements"] {
        let root = p.base.join(name);
        fs::rename(&root, root.with_extension("old")).unwrap();
        fs::DirBuilder::new().mode(0o700).create(&root).unwrap();
    }
    f.change_metadata();
    let e = super::extract(
        startup,
        wdts,
        caches,
        repo,
        super::Limits {
            max_callbacks: MAX_CALLBACKS + 1,
            ..Default::default()
        },
    )
    .err()
    .unwrap();
    assert_eq!(e.cause, Cause::Stage(StageError::InvalidLimits));
    assert!(e.guards.collector.is_some_and(collector_changed));
    assert_eq!(
        e.guards.wdt,
        Some(inventory::Error::Spool(payload::Error::Changed))
    );
    assert_eq!(
        e.guards.cache,
        Some(inventory::Error::Spool(payload::Error::Changed))
    );
    assert_eq!(
        e.guards.stage,
        Some(StageError::Spool(payload::Error::Changed))
    );
    assert_eq!(e.code(), "local_store_input_changed");
}

#[test]
fn compact_typed_error_guards_have_no_duplicate_storage_or_heap_indirection() {
    let [error_bytes, access_bytes] =
        std::hint::black_box([size_of::<Error>(), size_of::<AccessError<u8>>()]);
    assert!(error_bytes <= 128);
    assert!(access_bytes <= 128);
    let guards = Guards {
        collector: Some(payload::CollectorError::Payload(payload::Error::Changed)),
        wdt: Some(inventory::Error::Limit),
        cache: Some(inventory::Error::Missing),
        stage: Some(StageError::Callback),
    };
    let e = Error {
        cause: Cause::Stage(StageError::Limit),
        guards,
    };
    let access = AccessError::<u8> {
        cause: AccessCause::Access(e.cause),
        guards: e.guards,
    };
    assert_eq!(access.code(), e.code());
    assert_eq!(access.guards, e.guards);
    assert_eq!(
        access.cause,
        AccessCause::Access(Cause::Stage(StageError::Limit))
    );
}

#[test]
fn selected_domain_projects_all_globals_before_any_depth_ordered_adt() {
    let mut input = files(&[(90, 190, -1), (3, 103, 2), (1, 101, -1), (2, 102, 1)]);
    input.extend([
        (103, None, wdt(Some(301), Some(113))),
        (101, None, wdt(None, Some(111))),
        (102, None, wdt(None, Some(112))),
        (111, None, m2_adt(201, 7)),
        (112, None, m2_adt(201, 7)),
        (113, None, chunk(b"MVER", &18u32.to_le_bytes())),
        (201, None, m2(true)),
        (301, None, wmo_root(302, None)),
        (302, None, wmo_group(true, false, false)),
    ]);
    let f = CollisionFixture::new(&input);
    let p = Private::new();
    let mut out = acquire_selected(
        &f,
        &p,
        plan::MapSelection::MapAndAncestors(3),
        super::Limits {
            world: reduced_world(),
            ..Default::default()
        },
    )
    .unwrap();
    let mut chronological = out.stage.entries.clone();
    chronological.sort_unstable_by_key(|e| e.sequence);
    assert_eq!(
        chronological
            .iter()
            .map(|e| (e.context.map_id, e.context.kind))
            .collect::<Vec<_>>(),
        [
            (3, run::Kind::Global),
            (1, run::Kind::Global),
            (2, run::Kind::Global),
            (1, run::Kind::Tile { x: 0, y: 0 }),
            (2, run::Kind::Tile { x: 0, y: 0 }),
            (3, run::Kind::Tile { x: 0, y: 0 })
        ]
    );
    // The child's global WMO client ID7 is inserted before the root M2 with
    // that same serial allocator key. One domain Session must retain ID7.
    out.with_map(1, 0, |view| {
        assert_eq!(view.records().next().unwrap().id(), 7);
        Ok::<_, ()>(())
    })
    .unwrap();
    assert_eq!(out.world().report().ids.calls, 3);
    assert_eq!(out.world().report().ids.entries, 1);
    assert_eq!(out.world().report().own_tiles, 3);
    assert_eq!(out.world().report().replayed_tiles, 0);
}
#[test]
fn selected_stage_rejects_an_extra_out_of_domain_global_and_poisons() {
    let mut input = files(&[(1, 101, -1), (2, 102, -1)]);
    input.push((101, None, wdt(None, None)));
    let f = CollisionFixture::new(&input);
    let p = Private::new();
    let mut out = acquire_selected(
        &f,
        &p,
        plan::MapSelection::MapAndAncestors(1),
        super::Limits {
            world: reduced_world(),
            ..Default::default()
        },
    )
    .unwrap();
    out.stage.append(global(2), &[]).unwrap();
    assert_eq!(
        out.stage.validate_topology(out.world.topology()),
        Err(StageError::Binding)
    );
    assert!(out.stage.check().is_err());
}
