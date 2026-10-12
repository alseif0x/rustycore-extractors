// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-12: selected VMO disk phase with complete toy-CASC inputs.
use super::*;
use crate::collision::world::{MAP_FILE_DATA_ID, MATERIAL_FILE_DATA_ID, TYPE_FILE_DATA_ID};
use crate::collision::{
    gameobjects, inventory_spool as inventory,
    spool::{Limits as SpoolLimits, ProtectedInputs, Spool},
};
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
// Each case owns five repositories, each retaining complete source/destination
// directory chains. Bound test-only lifetime under the usual 1024-FD soft limit;
// production limits, parallel test selection and every assertion are unchanged.
static PRIVATE_FIXTURE: std::sync::Mutex<()> = std::sync::Mutex::new(());
struct Private {
    base: PathBuf,
    _lease: std::sync::MutexGuard<'static, ()>,
}
impl Private {
    fn new() -> Self {
        let lease = PRIVATE_FIXTURE.lock().unwrap();
        let base = std::env::temp_dir().join(format!(
            "rce-selected-vmo-stage-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::DirBuilder::new().mode(0o700).create(&base).unwrap();
        Self {
            base,
            _lease: lease,
        }
    }
    fn spool(&self, f: &CollisionFixture, name: &str, l: SpoolLimits) -> Spool {
        Spool::create(
            &self.base.join(name),
            ProtectedInputs::capture(f.path(), &[]).unwrap(),
            l,
        )
        .unwrap()
    }
}
impl Drop for Private {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.base).unwrap();
    }
}
fn stage(f: &CollisionFixture, p: &Private) -> Staged {
    let mut store = f.open(false);
    let startup = gameobjects::spooled::extract_startup_fdid(
        &mut store,
        gameobjects::Limits::default(),
        p.spool(f, "models", SpoolLimits::default()),
    )
    .unwrap();
    let inv = |n: &str| {
        inventory::Inventory::new(
            p.spool(f, n, SpoolLimits::default()),
            inventory::Limits {
                max_heap_bytes: 256 * 1024 * 1024,
                ..Default::default()
            },
        )
        .unwrap()
    };
    let mut l = super::super::Limits::default();
    l.world.world.run.placements.ids.max_memory_bytes = 1024 * 1024;
    super::super::extract(
        startup,
        inv("wdts"),
        inv("caches"),
        p.spool(f, "placements", SpoolLimits::default()),
        l,
    )
    .unwrap()
}
fn prepare_for(f: &CollisionFixture, l: super::Limits) -> Result<(Prepared, Private), Error> {
    let p = Private::new();
    let out = super::prepare(stage(f, &p), p.spool(f, "vmos", SpoolLimits::default()), l)?;
    Ok((out, p))
}
fn compare(f: &CollisionFixture) -> (Prepared, Private) {
    let mut store = f.open(false);
    let startup =
        gameobjects::extract_startup_fdid(&mut store, gameobjects::Limits::default()).unwrap();
    let mut l = crate::collision::assemble::Limits::default();
    l.world.run.placements.ids.max_memory_bytes = 1024 * 1024;
    let old = crate::collision::assemble::extract(startup, l).unwrap();
    let (mut new, p) = prepare_for(f, super::Limits::default()).unwrap();
    let expected: Vec<_> = old
        .models()
        .map(|(n, m)| (n.to_vec(), m.bytes().to_vec()))
        .collect();
    assert_eq!(
        new.models()
            .unwrap()
            .map(|(n, _)| n.to_vec())
            .collect::<Vec<_>>(),
        expected.iter().map(|(n, _)| n.clone()).collect::<Vec<_>>()
    );
    for (name, bytes) in expected {
        assert_eq!(
            new.with_model(&name, bytes.len(), |v| Ok::<_, u8>(v.bytes().to_vec()))
                .unwrap(),
            bytes
        );
    }
    let a = new.stats().unwrap();
    let b = old.stats();
    assert_eq!(
        (a.selected_models, a.vertices, a.triangles, a.output_bytes),
        (
            b.selected_models,
            b.vertices,
            b.triangles,
            b.output_bytes
                - old.world().startup().data().lists().dtree_bytes().len()
                - old
                    .maps()
                    .iter()
                    .map(|m| m.stats().output_bytes)
                    .sum::<usize>()
        )
    );
    assert_eq!(
        a.bih_work,
        old.models().map(|(_, m)| m.stats().build_work).sum::<u64>()
    );
    assert_eq!(a.raw_visit_bytes, b.raw_visit_bytes);
    assert_eq!(
        new.stage().world().startup().extraction().facts(),
        old.world().startup().extraction().facts()
    );
    assert!(a.peak_admitted_bytes <= MAX_MEMORY_BYTES);
    assert_eq!(new.repository.stats().unwrap().records, a.selected_models);
    (new, p)
}
fn one_m2() -> CollisionFixture {
    let mut input = files(&[(1, 101, -1)]);
    input.extend([
        (101, None, wdt(None, Some(111))),
        (111, None, m2_adt(201, 42)),
        (201, None, m2(true)),
    ]);
    CollisionFixture::new(&input)
}
fn mutate(p: &Path) {
    fs::set_permissions(p, fs::Permissions::from_mode(0o600)).unwrap();
    fs::write(p, b"changed").unwrap();
}
struct MutateOnDrop(PathBuf);
impl Drop for MutateOnDrop {
    fn drop(&mut self) {
        mutate(&self.0);
    }
}
#[test]
fn empty_world_has_no_vmo_or_forged_selection() {
    let f = CollisionFixture::new(&files(&[]));
    let (mut c, _p) = compare(&f);
    assert_eq!(c.models().unwrap().count(), 0);
    assert_eq!(c.stats().unwrap().selected_models, 0);
    c.verify_unchanged().unwrap();
}
#[test]
fn complete_m2_vmo_bounds_identity_and_bytes_match_resident_assembler() {
    let f = one_m2();
    let (c, p) = compare(&f);
    let e = *c.index.iter().find(|e| e.selected).unwrap();
    let identity = identity(c.stage.world.startup().extraction(), e).unwrap();
    let bytes = fs::read(p.base.join("vmos/00000000.payload")).unwrap();
    assert_eq!(&bytes[..HEADER], &identity.header);
    assert_eq!(
        &bytes[HEADER..HEADER + identity.len],
        &identity.name[..identity.len]
    );
    assert!(e.bounds.is_some());
    assert_eq!(c.stats().unwrap().vertices, 3);
    assert_eq!(c.stats().unwrap().triangles, 1);
}
#[test]
fn duplicate_placement_ids_select_all_names_before_first_id_wins() {
    let mut input = files(&[(1, 101, -1)]);
    let rows = [m2_adt(201, 42)[8..].to_vec(), m2_adt(202, 42)[8..].to_vec()].concat();
    input.extend([
        (101, None, wdt(None, Some(111))),
        (111, None, chunk(b"MDDF", &rows)),
        (201, None, m2(true)),
        (202, None, m2(true)),
    ]);
    let f = CollisionFixture::new(&input);
    let (c, _p) = compare(&f);
    assert_eq!(c.stats().unwrap().selected_models, 2);
    assert_eq!(c.stats().unwrap().placement_records, 2);
}
#[test]
fn startup_duplicates_omissions_and_no_collision_preserve_list_union() {
    let mut input = files(&[]);
    input[0].2 = gameobject_db2(&[(1, 201), (2, 201), (3, 0), (4, 202), (5, 203)], &[]);
    input.extend([
        (201, None, m2(true)),
        (202, None, m2(false)),
        (203, None, b"BLP2texture".to_vec()),
    ]);
    let f = CollisionFixture::new(&input);
    let (c, _p) = compare(&f);
    assert_eq!(c.stats().unwrap().selected_models, 1);
    assert_eq!(c.stats().unwrap().list_rows, 2);
    assert_eq!(c.stats().unwrap().placement_records, 0);
}
#[test]
fn startup_wmo_excludes_acquired_unused_doodad_model_dependency() {
    let mut input = files(&[]);
    input[0].2 = gameobject_db2(&[(1, 301)], &[]);
    input.extend([
        (301, None, wmo_root(302, Some(201))),
        (302, None, wmo_group(true, true, true)),
        (201, None, m2(true)),
    ]);
    let f = CollisionFixture::new(&input);
    let (c, _p) = compare(&f);
    assert_eq!(c.stats().unwrap().selected_models, 1);
    assert_eq!(
        c.stage
            .world
            .startup()
            .extraction()
            .phase_compact()
            .unwrap()
            .model_count(),
        2
    );
    assert_eq!(c.models().unwrap().next().unwrap().1, Class::Wmo);
}
#[test]
fn startup_cached_wmo_and_later_wmo_match_distinct_liquid_conversion() {
    let mut input = files(&[(2, 102, -1), (1, 101, -1)]);
    input[0].2 = gameobject_db2(&[(1, 301)], &[]);
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
    assert!(c.stats().unwrap().selected_models >= 3);
}
#[test]
fn inherited_records_and_parent_empty_barriers_preserve_model_union() {
    let mut input = files(&[(3, 103, 2), (2, 102, 1), (1, 101, -1)]);
    input.extend([
        (101, None, wdt(None, Some(111))),
        (102, None, wdt(None, None)),
        (103, None, wdt(None, None)),
        (111, None, m2_adt(201, 42)),
        (201, None, m2(true)),
    ]);
    let f = CollisionFixture::new(&input);
    let (c, _p) = compare(&f);
    assert_eq!(c.stats().unwrap().maps_scanned, 3);
    assert_eq!(c.stats().unwrap().selected_models, 1);
}
#[test]
fn aggregate_geometry_is_admitted_before_first_archive_write() {
    for (vertices, triangles) in [(2, 16), (16, 0)] {
        let f = one_m2();
        let p = Private::new();
        let l = super::Limits {
            max_vertices: vertices,
            max_triangles: triangles,
            ..Default::default()
        };
        let e = super::prepare(
            stage(&f, &p),
            p.spool(&f, "vmos", SpoolLimits::default()),
            l,
        )
        .err()
        .unwrap();
        assert_eq!(e.code(), Kind::Limit.code());
        assert_eq!(fs::read_dir(p.base.join("vmos")).unwrap().count(), 0);
    }
}
#[test]
fn model_output_work_memory_and_selection_caps_fail_whole_phase() {
    for axis in 0..5 {
        let f = one_m2();
        let p = Private::new();
        let mut l = super::Limits::default();
        match axis {
            0 => l.max_models = 0,
            1 => l.max_output_bytes = 0,
            2 => l.max_bih_work = 0,
            3 => l.max_raw_visit_bytes = 0,
            _ => l.max_memory_bytes = 0,
        };
        assert!(
            super::prepare(
                stage(&f, &p),
                p.spool(&f, "vmos", SpoolLimits::default()),
                l
            )
            .is_err()
        );
    }
}
#[test]
fn upward_component_and_phase_limits_reject_even_with_empty_selection() {
    for axis in 0..3 {
        let f = CollisionFixture::new(&files(&[]));
        let p = Private::new();
        let mut l = super::Limits::default();
        match axis {
            0 => l.max_memory_bytes += 1,
            1 => l.conversion.max_bytes += 1,
            _ => l.conversion.bih.max_work += 1,
        };
        assert_eq!(
            super::prepare(
                stage(&f, &p),
                p.spool(&f, "vmos", SpoolLimits::default()),
                l
            )
            .err()
            .unwrap()
            .code(),
            Kind::InvalidLimits.code()
        );
    }
}
#[test]
fn nonempty_archive_is_not_admitted_as_a_fresh_owner() {
    let f = one_m2();
    let p = Private::new();
    let mut repo = p.spool(&f, "vmos", SpoolLimits::default());
    repo.write(0, |_| Ok(())).unwrap();
    assert_eq!(
        super::prepare(stage(&f, &p), repo, super::Limits::default())
            .err()
            .unwrap()
            .code(),
        Kind::State.code()
    );
}
#[test]
fn changed_vmo_archive_or_raw_or_placement_prevents_complete_read() {
    for name in ["vmos", "models", "placements"] {
        let f = one_m2();
        let (mut c, p) = compare(&f);
        let model = c.models().unwrap().next().unwrap().0.to_vec();
        mutate(&p.base.join(name).join("00000000.payload"));
        let e = c.with_model(&model, 0, |_| Ok::<_, u8>(())).unwrap_err();
        assert_eq!(e.code(), payload::Error::Changed.code());
        assert!(c.verify_unchanged().is_err());
    }
}
#[test]
fn typed_callback_error_survives_mutating_capture_drop() {
    let f = one_m2();
    let (mut c, p) = compare(&f);
    let name = c.models().unwrap().next().unwrap().0.to_vec();
    let action = MutateOnDrop(p.base.join("vmos/00000000.payload"));
    let e = c
        .with_model(&name, 0, move |_| {
            drop(action);
            Err::<(), _>(73u8)
        })
        .unwrap_err();
    assert_eq!(e.cause, Cause::Callback(73));
    assert_eq!(e.guards.archive, Some(payload::Error::Changed));
    assert_eq!(e.code(), payload::Error::Changed.code());
}
#[test]
fn refused_reload_drops_capture_before_all_archive_guards() {
    let f = one_m2();
    let (mut c, p) = compare(&f);
    let name = c.models().unwrap().next().unwrap().0.to_vec();
    let action = MutateOnDrop(p.base.join("vmos/00000000.payload"));
    let e = c
        .with_model(&name, usize::MAX, move |_| {
            drop(action);
            Ok::<_, u8>(())
        })
        .unwrap_err();
    assert_eq!(e.cause, Cause::Kind(Kind::Limit));
    assert_eq!(e.guards.archive, Some(payload::Error::Changed));
}
#[test]
fn completed_payload_refusal_drops_capture_and_keeps_zero_reload_delta() {
    let f = one_m2();
    let (mut c, p) = compare(&f);
    let h = c.index.iter().find(|e| e.selected).unwrap().handle;
    let extractor = c.stage.world.extraction_mut();
    let before = extractor.stats().unwrap().spool.reloads;
    let action = MutateOnDrop(p.base.join("models/00000000.payload"));
    let e = extractor.with_payload_phase(h, 0, 0, move |_| {
        drop(action);
        Ok::<_, u8>(())
    });
    assert!(matches!(
        e,
        Err(CallbackFailure::Collector(CollectorError::Payload(
            payload::Error::Changed
        )))
    ));
    assert_eq!(extractor.stats().unwrap().spool.reloads, before);
}
#[test]
fn typed_completed_payload_failure_survives_source_guard() {
    let f = one_m2();
    let (mut c, p) = compare(&f);
    let h = c.index.iter().find(|e| e.selected).unwrap().handle;
    let path = p.base.join("models/00000000.payload");
    let shape = c.stage.world.startup().extraction().phase_shape(h).unwrap();
    let e =
        c.stage
            .world
            .extraction_mut()
            .with_payload_phase(h, shape.record_bytes, 0, move |_| {
                mutate(&path);
                Err::<(), _>(91u8)
            });
    assert!(matches!(
        e,
        Err(CallbackFailure::Callback {
            cause: 91,
            guard: Some(CollectorError::Payload(payload::Error::Changed))
        })
    ));
}
#[test]
fn phase_residency_retains_exact_data_topology_and_recorded_store() {
    let f = one_m2();
    let (c, _p) = compare(&f);
    let ex = c.stage.world.startup().extraction();
    let s = ex.stats().unwrap();
    let retained = c
        .stage
        .world
        .startup()
        .data()
        .unwrap()
        .stats()
        .resident_bytes
        + c.stage.world.topology().resident_bytes();
    assert_eq!(ex.retained_external_for_test(), retained);
    assert!(ex.phase_resident_bytes().unwrap() >= s.compact_bytes + s.repository_bytes + retained);
    assert!(
        c.stats().unwrap().reserved_bytes
            >= ex.phase_resident_bytes().unwrap() + c.stage.stage.limits.max_heap_bytes
    );
}
#[test]
fn typed_phase_error_has_bounded_inline_size() {
    let n = std::hint::black_box(size_of::<Error<u8>>());
    assert!(n <= 128);
}

#[test]
fn rejected_success_value_drops_before_independent_raw_and_archive_guards() {
    let f = one_m2();
    let (mut c, p) = compare(&f);
    let name = c.models().unwrap().next().unwrap().0.to_vec();
    let archive = p.base.join("vmos/00000000.payload");
    let value = MutateOnDrop(p.base.join("models/00000000.payload"));
    let e = c
        .with_model(&name, 4096, move |_| {
            mutate(&archive);
            Ok::<_, u8>(value)
        })
        .err()
        .unwrap();
    assert_eq!(e.guards.archive, Some(payload::Error::Changed));
    assert_eq!(
        e.guards.collector,
        Some(CollectorError::Payload(payload::Error::Changed))
    );
    assert_eq!(e.code(), payload::Error::Changed.code());
    assert!(c.stats().is_err());
}
#[test]
fn refused_model_binding_drops_uninvoked_capture_before_placement_guard() {
    let f = one_m2();
    let (mut c, p) = compare(&f);
    let action = MutateOnDrop(p.base.join("placements/00000000.payload"));
    let e = c
        .with_model::<(), u8>(b"not-selected.vmo", 0, move |_| {
            drop(action);
            panic!("a refused model binding must not invoke its callback");
        })
        .err()
        .unwrap();
    assert_eq!(e.cause, Cause::Kind(Kind::Binding));
    assert_eq!(
        e.guards.placements,
        Some(StageError::Spool(payload::Error::Changed))
    );
    assert!(c.verify_unchanged().is_err());
}
#[test]
fn completed_payload_callback_heap_refuses_before_source_reload() {
    let f = one_m2();
    let (mut c, _p) = compare(&f);
    let handle = c.index.iter().find(|e| e.selected).unwrap().handle;
    let extraction = c.stage.world.extraction_mut();
    let shape = extraction.phase_shape(handle).unwrap();
    let before = extraction.stats().unwrap().spool.reloads;
    let mut invoked = false;
    let e = extraction
        .with_payload_phase(handle, shape.record_bytes, usize::MAX, |_| {
            invoked = true;
            Ok::<_, u8>(())
        })
        .err()
        .unwrap();
    assert!(!invoked);
    assert!(matches!(
        e,
        CallbackFailure::Collector(CollectorError::Collision(crate::collision::Error::Limit))
    ));
    assert_eq!(extraction.stats().unwrap().spool.reloads, before);
    assert!(extraction.phase_compact().is_err());
}
