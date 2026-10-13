// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-12: closed final package over complete authored toy CASC.
use super::*;
use crate::collision::assemble::staged::{self as placements, models as selected};
use crate::collision::world::{MAP_FILE_DATA_ID, MATERIAL_FILE_DATA_ID, TYPE_FILE_DATA_ID};
use crate::collision::{
    gameobjects, inventory_spool as inventory,
    spool::{Limits as SpoolLimits, ProtectedInputs, Spool},
};
use crate::local_store::tests::{CollisionFile, CollisionFixture};
use std::{
    fs,
    os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt},
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
    profile: spool::Profile,
    backend: spool::BackendProfile,
    resources: crate::collision::resources::Profile,
    _lease: std::sync::MutexGuard<'static, ()>,
}
impl Private {
    fn new() -> Self {
        let lease = PRIVATE_FIXTURE.lock().unwrap();
        let base = std::env::temp_dir().join(format!(
            "rce-closed-map-package-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::DirBuilder::new().mode(0o700).create(&base).unwrap();
        Self {
            base,
            profile: spool::Profile::SameMountV1,
            backend: spool::BackendProfile::FileSetV1,
            resources: crate::collision::resources::Profile::BoundedV1,
            _lease: lease,
        }
    }
    fn captured_volumes(parent: &Path, f: &CollisionFixture) -> Self {
        let lease = PRIVATE_FIXTURE.lock().unwrap();
        assert!(parent.is_absolute());
        assert_ne!(
            fs::metadata(parent).unwrap().dev(),
            fs::metadata(f.path()).unwrap().dev()
        );
        let base = parent.join(format!(
            "closed-cross-volume-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::DirBuilder::new().mode(0o700).create(&base).unwrap();
        // The default remains strict and rejects before namespace creation.
        let rejected = base.join("v1-must-reject");
        assert_eq!(
            Spool::create(
                &rejected,
                ProtectedInputs::capture(f.path(), &[]).unwrap(),
                SpoolLimits::default()
            )
            .err(),
            Some(payload::Error::Mount)
        );
        assert!(!rejected.exists());
        Self {
            base,
            profile: spool::Profile::CapturedVolumesV2,
            backend: spool::BackendProfile::FileSetV1,
            resources: crate::collision::resources::Profile::BoundedV1,
            _lease: lease,
        }
    }
    fn spool(&self, f: &CollisionFixture, name: &str, l: SpoolLimits) -> Spool {
        let spool = Spool::create_with_backend(
            &self.base.join(name),
            ProtectedInputs::capture(f.path(), &[]).unwrap(),
            l,
            self.profile,
            self.resources,
            self.backend,
        )
        .unwrap();
        assert_eq!(spool.profile(), self.profile);
        assert_eq!(spool.backend(), self.backend);
        spool
    }
}
impl Drop for Private {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.base).unwrap();
    }
}
fn prepared(f: &CollisionFixture, p: &Private) -> Prepared {
    prepared_selected(f, p, crate::collision::plan::MapSelection::All)
}
fn prepared_selected(
    f: &CollisionFixture,
    p: &Private,
    selection: crate::collision::plan::MapSelection,
) -> Prepared {
    let mut store = f.open(false);
    let startup = gameobjects::spooled::extract_startup_fdid(
        &mut store,
        p.resources.startup_limits(),
        p.spool(f, "models", p.resources.spool_limits()),
    )
    .unwrap();
    let inv = |name| {
        inventory::Inventory::new(
            p.spool(f, name, p.resources.spool_limits()),
            inventory::Limits {
                max_heap_bytes: 256 * 1024 * 1024,
                ..p.resources.inventory_limits()
            },
        )
        .unwrap()
    };
    let mut limits = p.resources.stage_limits();
    limits.world.world.run.placements.ids.max_memory_bytes = 1024 * 1024;
    let staged = placements::extract_selected(
        startup,
        inv("wdts"),
        inv("caches"),
        p.spool(f, "placements", p.resources.spool_limits()),
        selection,
        limits,
    )
    .unwrap();
    selected::prepare(
        staged,
        p.spool(f, "vmos", p.resources.spool_limits()),
        p.resources.model_limits(),
    )
    .unwrap()
}
fn perform(f: &CollisionFixture, limits: Limits) -> Result<(Completed, Private), Error> {
    let p = Private::new();
    let c = assemble(
        prepared(f, &p),
        p.spool(f, "maps", SpoolLimits::default()),
        limits,
    )?;
    Ok((c, p))
}
fn old(f: &CollisionFixture) -> Result<resident::Completed, resident::Error> {
    let mut store = f.open(false);
    let startup =
        gameobjects::extract_startup_fdid(&mut store, gameobjects::Limits::default()).unwrap();
    let mut l = resident::Limits::default();
    l.world.run.placements.ids.max_memory_bytes = 1024 * 1024;
    resident::extract(startup, l)
}
fn compare(f: &CollisionFixture) -> (Completed, Private, resident::Completed) {
    let expected = old(f).unwrap();
    let (c, p) = perform(f, Limits::default()).unwrap();
    compare_completed(c, p, expected)
}
fn compare_completed(
    mut c: Completed,
    p: Private,
    expected: resident::Completed,
) -> (Completed, Private, resident::Completed) {
    let want: Vec<_> = expected
        .artifacts()
        .map(|a| (a.name().to_string(), a.bytes().to_vec()))
        .collect();
    let meta: Vec<_> = c
        .artifacts()
        .unwrap()
        .map(|(n, len)| (n.to_string(), len))
        .collect();
    assert_eq!(
        meta,
        want.iter()
            .map(|(n, b)| (n.clone(), b.len()))
            .collect::<Vec<_>>()
    );
    let mut got = Vec::new();
    c.visit(64 * 1024 * 1024, |a| {
        got.push((a.name().to_string(), a.bytes().to_vec()));
        Ok::<_, u8>(())
    })
    .unwrap();
    assert_eq!(got, want);
    let a = c.stats().unwrap();
    let b = expected.stats();
    assert_eq!(
        (
            a.maps,
            a.selected_models,
            a.input_records,
            a.vertices,
            a.triangles,
            a.associations,
            a.files,
            a.output_bytes,
            a.proof_bytes
        ),
        (
            b.maps,
            b.selected_models,
            b.input_records,
            b.vertices,
            b.triangles,
            b.associations,
            b.files,
            b.output_bytes,
            b.proof_bytes
        )
    );
    assert_eq!(
        (
            a.raw_visit_bytes,
            a.bih_work,
            a.ancestor_steps,
            a.association_validation_bytes
        ),
        (
            b.raw_visit_bytes,
            b.bih_work,
            b.ancestor_steps,
            b.validation_bytes
        )
    );
    assert_eq!(
        a.unique_spawns,
        expected
            .maps()
            .iter()
            .map(|m| m.stats().unique_spawns)
            .sum::<usize>()
    );
    let actual_facts = c.prepared().stage().world().startup().extraction().facts();
    let expected_facts = expected.world().startup().extraction().facts();
    if matches!(
        c.prepared().stage().world().topology().selection(),
        crate::collision::plan::MapSelection::All
    ) {
        assert_eq!(actual_facts, expected_facts);
    } else {
        // The selected owner authenticated the complete original Map; the
        // independent reference authenticates only the closure Map. All other
        // complete reads, raw bytes, models and acquisition work agree.
        assert_eq!(
            (
                actual_facts.build,
                actual_facts.reads,
                actual_facts.dependency_requests,
                actual_facts.raw_bytes,
                actual_facts.models
            ),
            (
                expected_facts.build,
                expected_facts.reads,
                expected_facts.dependency_requests,
                expected_facts.raw_bytes,
                expected_facts.models
            )
        );
        assert_eq!(
            actual_facts.source_bytes
                - c.prepared().stage().world().tables()[0]
                    .source()
                    .byte_count(),
            expected_facts.source_bytes - expected.world().tables()[0].source().byte_count()
        );
    }
    assert!(a.peak_admitted_bytes <= MAX_MEMORY_BYTES);
    c.verify_unchanged().unwrap();
    (c, p, expected)
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
fn mutate(path: &Path) {
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
    fs::write(path, b"changed").unwrap();
}
struct MutateOnDrop(PathBuf);
impl Drop for MutateOnDrop {
    fn drop(&mut self) {
        mutate(&self.0);
    }
}
fn wdt_cells(values: &[(u8, u8, u32)]) -> Vec<u8> {
    let mut mphd = [0; 32];
    put(&mut mphd, 0, 0x200);
    let mut bytes = chunk(b"MPHD", &mphd);
    let mut main = vec![0; 32768];
    let mut maid = vec![0; 131072];
    for &(x, y, id) in values {
        let i = y as usize * 64 + x as usize;
        put(&mut main, i * 8, 1);
        put(&mut maid, i * 32 + 4, id);
    }
    bytes.extend(chunk(b"MAIN", &main));
    bytes.extend(chunk(b"MAID", &maid));
    bytes
}
#[test]
fn empty_world_exposes_only_complete_dtree_inventory() {
    let f = CollisionFixture::new(&files(&[]));
    let (c, _p, _) = compare(&f);
    assert_eq!(c.stats().unwrap().files, 1);
    assert!(c.maps.is_empty());
}
#[test]
fn empty_active_maps_keep_numeric_order_and_exact_nested_names() {
    let mut input = files(&[(70001, 101, -1), (9, 102, -1)]);
    input.extend([(101, None, wdt(None, None)), (102, None, wdt(None, None))]);
    let f = CollisionFixture::new(&input);
    let (c, _p, _) = compare(&f);
    let names: Vec<_> = c.artifacts().unwrap().map(|(n, _)| n.to_string()).collect();
    assert_eq!(names[1..], ["0009/0009.vmtree", "70001/70001.vmtree"]);
}
#[test]
fn complete_m2_map_model_tree_tile_and_index_bytes_match() {
    let f = one_m2();
    let (c, _p, _) = compare(&f);
    assert_eq!(c.stats().unwrap().selected_models, 1);
    assert_eq!(c.stats().unwrap().maps, 1);
    assert!(c.stats().unwrap().files >= 4);
}
#[test]
fn duplicate_ids_keep_every_model_name_before_first_id_wins() {
    let mut input = files(&[(1, 101, -1)]);
    let rows = [m2_adt(201, 42)[8..].to_vec(), m2_adt(202, 42)[8..].to_vec()].concat();
    input.extend([
        (101, None, wdt(None, Some(111))),
        (111, None, chunk(b"MDDF", &rows)),
        (201, None, m2(true)),
        (202, None, m2(true)),
    ]);
    let f = CollisionFixture::new(&input);
    let (c, _p, _) = compare(&f);
    assert_eq!(c.stats().unwrap().selected_models, 2);
    assert_eq!(c.stats().unwrap().input_records, 2);
    assert_eq!(c.stats().unwrap().unique_spawns, 1);
}
#[test]
fn startup_only_duplicates_omissions_and_no_collision_preserve_lists() {
    let mut input = files(&[]);
    input[0].2 = gameobject_db2(&[(1, 201), (2, 201), (3, 0), (4, 202), (5, 203)], &[]);
    input.extend([
        (201, None, m2(true)),
        (202, None, m2(false)),
        (203, None, b"BLP2texture".to_vec()),
    ]);
    let f = CollisionFixture::new(&input);
    let (c, _p, _) = compare(&f);
    assert_eq!(c.stats().unwrap().selected_models, 1);
    assert_eq!(c.stats().unwrap().files, 2);
}
#[test]
fn unused_acquired_wmo_doodad_dependency_is_excluded() {
    let mut input = files(&[]);
    input[0].2 = gameobject_db2(&[(1, 301)], &[]);
    input.extend([
        (301, None, wmo_root(302, Some(201))),
        (302, None, wmo_group(true, true, true)),
        (201, None, m2(true)),
    ]);
    let f = CollisionFixture::new(&input);
    let (c, _p, _) = compare(&f);
    assert_eq!(c.stats().unwrap().selected_models, 1);
    assert_eq!(
        c.prepared()
            .stage()
            .world()
            .startup()
            .extraction()
            .phase_compact()
            .unwrap()
            .model_count(),
        2
    );
}
#[test]
fn cached_startup_wmo_and_later_liquid_wmo_keep_exact_bytes() {
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
    let (c, _p, _) = compare(&f);
    assert!(c.stats().unwrap().selected_models >= 3);
}
#[test]
fn authenticated_runner_parent_only_tile_preserves_child_index() {
    let mut input = files(&[(2, 102, 1), (1, 101, -1)]);
    input.extend([
        (101, None, wdt(None, Some(111))),
        (102, None, wdt(None, None)),
        (111, None, m2_adt(201, 42)),
        (201, None, m2(true)),
    ]);
    let f = CollisionFixture::new(&input);
    let (mut c, _p, expected) = compare(&f);
    assert!(c.stats().unwrap().parent_only_tiles > 0);
    assert_eq!(
        c.stats().unwrap().ancestor_steps,
        expected.stats().ancestor_steps
    );
    c.with_map(2, 0, |v| {
        assert!(
            v.artifacts()
                .all(|a| !matches!(a.name(), resident::ArtifactName::Tile { index: false, .. }))
        );
        Ok::<_, u8>(())
    })
    .unwrap();
}
#[test]
fn grandchild_empty_own_cache_barrier_matches_closed_world() {
    let mut input = files(&[(3, 103, 2), (2, 102, 1), (1, 101, -1)]);
    input.extend([
        (101, None, wdt(None, Some(111))),
        (102, None, wdt(None, None)),
        (103, None, wdt(None, None)),
        (111, None, m2_adt(201, 42)),
        (201, None, m2(true)),
    ]);
    let f = CollisionFixture::new(&input);
    let (mut c, _p, _) = compare(&f);
    c.with_map(3, 0, |v| {
        assert_eq!(v.artifacts().count(), 1);
        Ok::<_, u8>(())
    })
    .unwrap();
}
#[test]
fn authenticated_mixed_nearer_payload_mismatch_is_terminal() {
    let mut input = files(&[(1, 101, -1), (2, 102, 1), (3, 103, 2)]);
    input.extend([
        (101, None, wdt_cells(&[(0, 0, 111)])),
        (102, None, wdt_cells(&[(1, 0, 112)])),
        (103, None, wdt(None, None)),
        (111, None, m2_adt(201, 42)),
        (112, None, m2_adt(202, 43)),
        (201, None, m2(true)),
        (202, None, m2(true)),
    ]);
    let f = CollisionFixture::new(&input);
    assert!(matches!(
        old(&f),
        Err(resident::Error::Assembly(assembly::Error::Association))
    ));
    let e = perform(&f, Limits::default()).err().unwrap();
    assert_eq!(
        e.cause,
        Cause::Kind(Kind::Assembly(assembly::Error::Association))
    );
}
#[test]
fn whole_geometry_model_map_record_and_work_caps_reject_before_bundle_write() {
    for axis in 0..7 {
        let f = one_m2();
        let p = Private::new();
        let mut l = Limits::default();
        match axis {
            0 => l.max_models = 0,
            1 => l.max_maps = 0,
            2 => l.max_records = 0,
            3 => l.max_vertices = 0,
            4 => l.max_triangles = 0,
            5 => l.max_raw_visit_bytes = 0,
            _ => l.max_bih_work = 0,
        };
        let e = assemble(
            prepared(&f, &p),
            p.spool(&f, "maps", SpoolLimits::default()),
            l,
        )
        .err()
        .unwrap();
        assert_eq!(e.cause, Cause::Kind(Kind::Limit));
        assert_eq!(fs::read_dir(p.base.join("maps")).unwrap().count(), 0);
    }
}
#[test]
fn exact_file_and_output_byte_boundaries_accept_then_lower_rejects() {
    let f = one_m2();
    let (c, p, old) = compare(&f);
    let stats = c.stats().unwrap();
    drop(c);
    drop(p);
    drop(old);
    let (c, p) = perform(
        &f,
        Limits {
            max_files: stats.files,
            max_output_bytes: stats.output_bytes,
            ..Default::default()
        },
    )
    .unwrap();
    drop(c);
    drop(p);
    for l in [
        Limits {
            max_files: stats.files - 1,
            ..Default::default()
        },
        Limits {
            max_output_bytes: stats.output_bytes - 1,
            ..Default::default()
        },
    ] {
        assert!(perform(&f, l).is_err());
    }
}
#[test]
fn assembly_association_and_component_work_caps_are_not_per_map() {
    for axis in 0..4 {
        let f = one_m2();
        let mut l = Limits::default();
        match axis {
            0 => l.max_associations = 0,
            1 => l.assembly.max_spawns = 0,
            2 => l.assembly.max_tiles = 0,
            _ => l.assembly.bih.max_work = 0,
        };
        assert!(perform(&f, l).is_err());
    }
}
#[test]
fn upward_phase_and_component_limits_always_reject() {
    for axis in 0..5 {
        let f = CollisionFixture::new(&files(&[]));
        let mut l = Limits::default();
        match axis {
            0 => l.max_memory_bytes += 1,
            1 => l.max_validation_bytes += 1,
            2 => l.assembly.max_tiles += 1,
            3 => l.assembly.max_name_bytes += 1,
            _ => l.assembly.bih.max_work += 1,
        };
        assert_eq!(
            perform(&f, l).err().unwrap().cause,
            Cause::Kind(Kind::InvalidLimits)
        );
    }
}
#[test]
fn nonempty_bundle_repository_is_rejected_without_reuse() {
    let f = one_m2();
    let p = Private::new();
    let mut repo = p.spool(&f, "maps", SpoolLimits::default());
    repo.write(0, |_| Ok(())).unwrap();
    assert_eq!(
        assemble(prepared(&f, &p), repo, Limits::default())
            .err()
            .unwrap()
            .cause,
        Cause::Kind(Kind::State)
    );
}
#[test]
fn private_bundle_record_disk_and_active_caps_fail_whole_selection() {
    for axis in 0..3 {
        let f = one_m2();
        let p = Private::new();
        let mut l = SpoolLimits::default();
        match axis {
            0 => l.max_records = 0,
            1 => l.max_disk_bytes = 0,
            _ => l.max_active_bytes = 0,
        };
        assert!(assemble(prepared(&f, &p), p.spool(&f, "maps", l), Limits::default()).is_err());
    }
}
#[test]
fn typed_map_callback_error_keeps_independent_archive_change() {
    let f = one_m2();
    let (mut c, p, _) = compare(&f);
    let action = MutateOnDrop(p.base.join("maps/00000000.payload"));
    let e = c
        .with_map(1, 4096, move |_| {
            drop(action);
            Err::<(), _>(73u8)
        })
        .unwrap_err();
    assert_eq!(e.cause, Cause::Callback(73));
    assert_eq!(e.guards.maps, Some(payload::Error::Changed));
}
#[test]
fn missing_map_drops_uninvoked_capture_before_placement_guard() {
    let f = one_m2();
    let (mut c, p, _) = compare(&f);
    let action = MutateOnDrop(p.base.join("placements/00000000.payload"));
    let e = c
        .with_map::<(), u8>(99, 0, move |_| {
            drop(action);
            panic!("missing callback must not execute")
        })
        .err()
        .unwrap();
    assert_eq!(e.cause, Cause::Kind(Kind::Missing));
    assert_eq!(
        e.guards.models.placements,
        Some(placements::StageError::Spool(payload::Error::Changed))
    );
}
#[test]
fn rejected_map_success_value_drops_before_vmo_and_bundle_guards() {
    let f = one_m2();
    let (mut c, p, _) = compare(&f);
    let bundle = p.base.join("maps/00000000.payload");
    let value = MutateOnDrop(p.base.join("vmos/00000000.payload"));
    let e = c
        .with_map(1, 4096, move |_| {
            mutate(&bundle);
            Ok::<_, u8>(value)
        })
        .err()
        .unwrap();
    assert_eq!(e.guards.maps, Some(payload::Error::Changed));
    assert_eq!(e.guards.models.archive, Some(payload::Error::Changed));
}
#[test]
fn typed_model_callback_preserves_bundle_guard() {
    let f = one_m2();
    let (mut c, p, _) = compare(&f);
    let name = c.models.models().unwrap().next().unwrap().0.to_vec();
    let path = p.base.join("maps/00000000.payload");
    let e = c
        .with_model(&name, 4096, move |_| {
            mutate(&path);
            Err::<(), _>(91u8)
        })
        .unwrap_err();
    assert_eq!(e.cause, Cause::Callback(91));
    assert_eq!(e.guards.maps, Some(payload::Error::Changed));
}
#[test]
fn smaller_package_memory_cap_also_restricts_vmo_reader() {
    let mut input = files(&[]);
    input[0].2 = gameobject_db2(&[(1, 201)], &[]);
    input.push((201, None, m2(true)));
    let f = CollisionFixture::new(&input);
    let p = Private::new();
    let models = prepared(&f, &p);
    let name = models.models().unwrap().next().unwrap().0.to_vec();
    let entry = models.index.iter().find(|e| e.selected).unwrap();
    let extent = selected::HEADER + name.len() + entry.bytes;
    let repo = p.spool(&f, "maps", SpoolLimits::default());
    let max =
        models.live().unwrap() + 2 * repo.limits().max_descriptor_bytes + GUARD_BYTES + extent;
    let mut c = assemble(
        models,
        repo,
        Limits {
            max_memory_bytes: max,
            ..Default::default()
        },
    )
    .unwrap();
    let mut invoked = false;
    let e = c
        .with_model(&name, 0, |_| {
            invoked = true;
            Ok::<_, u8>(())
        })
        .unwrap_err();
    assert!(!invoked);
    assert_eq!(e.code(), forever_vmap::vmo::Error::Limit.code());
    assert!(c.stats().is_err());
}
#[test]
fn refused_map_memory_drops_capture_without_reload_delta() {
    let f = one_m2();
    let (mut c, p, _) = compare(&f);
    let before = c.repository.stats().unwrap().reloads;
    let action = MutateOnDrop(p.base.join("maps/00000000.payload"));
    let e = c
        .with_map(1, usize::MAX, move |_| {
            drop(action);
            Ok::<_, u8>(())
        })
        .unwrap_err();
    assert_eq!(e.cause, Cause::Kind(Kind::Limit));
    assert_eq!(e.guards.maps, Some(payload::Error::Changed));
    assert_eq!(c.repository.stats().unwrap().reloads, before);
}
#[test]
fn refused_model_memory_drops_capture_without_vmo_reload_delta() {
    let f = one_m2();
    let (mut c, p, _) = compare(&f);
    let name = c.models.models().unwrap().next().unwrap().0.to_vec();
    let before = c.models.repository.stats().unwrap().reloads;
    let action = MutateOnDrop(p.base.join("maps/00000000.payload"));
    let e = c
        .with_model(&name, usize::MAX, move |_| {
            drop(action);
            Ok::<_, u8>(())
        })
        .unwrap_err();
    assert_eq!(e.cause, Cause::Kind(Kind::Limit));
    assert_eq!(e.guards.maps, Some(payload::Error::Changed));
    assert_eq!(c.models.repository.stats().unwrap().reloads, before);
}
#[test]
fn visitor_failure_drops_its_owned_capture_before_final_guards() {
    let f = one_m2();
    let (mut c, p, _) = compare(&f);
    let action = Some(MutateOnDrop(p.base.join("maps/00000000.payload")));
    let e = c
        .visit(4096, move |_| {
            let _ = &action;
            Err(42u8)
        })
        .unwrap_err();
    assert_eq!(e.cause, Cause::Callback(42));
    assert_eq!(e.guards.maps, Some(payload::Error::Changed));
}
#[test]
fn visitor_late_typed_failure_poison_retains_caller_progress() {
    let f = one_m2();
    let (mut c, _p, _) = compare(&f);
    let total = c.stats().unwrap().files;
    let mut seen = 0;
    let e = c
        .visit(0, |_| {
            seen += 1;
            if seen == total { Err(55u8) } else { Ok(()) }
        })
        .unwrap_err();
    assert_eq!(seen, total);
    assert_eq!(e.cause, Cause::Callback(55));
    assert!(c.artifacts().is_err());
}
#[test]
fn successful_visitor_capture_drops_before_final_bundle_guard() {
    let f = one_m2();
    let (mut c, p, _) = compare(&f);
    let action = Some(MutateOnDrop(p.base.join("maps/00000000.payload")));
    let e = c
        .visit(4096, move |_| {
            let _ = &action;
            Ok::<_, u8>(())
        })
        .unwrap_err();
    assert_eq!(e.guards.maps, Some(payload::Error::Changed));
    assert!(c.verify_unchanged().is_err());
}
#[test]
fn each_retained_repository_change_blocks_any_artifact_body() {
    for dir in ["models", "placements", "vmos", "maps"] {
        let f = one_m2();
        let (mut c, p, _) = compare(&f);
        mutate(&p.base.join(dir).join("00000000.payload"));
        let mut invoked = false;
        let e = c
            .with_map(1, 0, |_| {
                invoked = true;
                Ok::<_, u8>(())
            })
            .unwrap_err();
        assert!(!invoked);
        assert_eq!(e.code(), payload::Error::Changed.code());
        assert!(c.stats().is_err());
    }
}
#[test]
fn metadata_inventory_is_io_free_and_visitation_reads_each_body_once() {
    let f = one_m2();
    let (mut c, _p, _) = compare(&f);
    let map_before = c.repository.stats().unwrap().reloads;
    let model_before = c.models.repository.stats().unwrap().reloads;
    let count = c.artifacts().unwrap().count();
    assert_eq!(count, c.stats().unwrap().files);
    assert_eq!(c.repository.stats().unwrap().reloads, map_before);
    assert_eq!(c.models.repository.stats().unwrap().reloads, model_before);
    c.visit(0, |_| Ok::<_, u8>(())).unwrap();
    assert_eq!(
        c.repository.stats().unwrap().reloads - map_before,
        c.stats().unwrap().maps
    );
    assert_eq!(
        c.models.repository.stats().unwrap().reloads - model_before,
        c.stats().unwrap().selected_models
    );
}
#[test]
fn completed_private_package_does_not_reopen_live_client_content() {
    let f = one_m2();
    let (mut c, _p, _) = compare(&f);
    f.change_metadata();
    c.visit(0, |_| Ok::<_, u8>(())).unwrap();
    c.verify_unchanged().unwrap();
}
#[test]
fn bundle_identity_reserved_bytes_and_native_extents_are_bound() {
    let f = one_m2();
    let (c, p, _) = compare(&f);
    let map = &c.maps[0];
    let bytes = fs::read(p.base.join("maps/00000000.payload")).unwrap();
    assert_eq!(&bytes[..HEADER], &header(map.bound, map.tiles.len()));
    assert!(MapView::check(map, &bytes, assembly::Limits::default()).is_ok());
    for at in [0, 8, 12, 13, 16, 20, 24, 32, 40, 48, 56, HEADER + 40] {
        let mut changed = bytes.clone();
        changed[at] ^= 1;
        assert!(MapView::check(map, &changed, assembly::Limits::default()).is_err());
    }
}
#[test]
fn bundle_and_model_validation_budget_is_cumulative() {
    let f = one_m2();
    let (mut c, _p, _) = compare(&f);
    let name = c.models.models().unwrap().next().unwrap().0.to_vec();
    let before = c.stats().unwrap().validation_bytes;
    let model_bytes = c.models.index.iter().find(|e| e.selected).unwrap().bytes;
    c.limits.max_validation_bytes = before + model_bytes as u64;
    c.with_model(&name, 0, |_| Ok::<_, u8>(())).unwrap();
    assert_eq!(
        c.stats().unwrap().validation_bytes,
        before + model_bytes as u64
    );
    assert!(c.with_map(1, 0, |_| Ok::<_, u8>(())).is_err());
}
#[test]
fn typed_package_error_stays_bounded_without_heap_boxing() {
    assert!(std::hint::black_box(size_of::<Error<u8>>()) <= 128);
}
// The following supplied-proof cases test only the private ancestor checker.
// Public assemble cannot accept these authored placements or replace its stage.
fn proof_rows(name: &[u8], id: u32, parent: bool) -> Vec<u8> {
    let mut bytes = vec![0; 62];
    bytes[0] = if parent { 3 } else { 1 };
    put(&mut bytes, 2, id);
    put(&mut bytes, 30, 1f32.to_bits());
    for (i, n) in [0f32, 0., 0., 1., 1., 1.].into_iter().enumerate() {
        put(&mut bytes, 34 + i * 4, n.to_bits());
    }
    put(&mut bytes, 58, name.len() as u32);
    bytes.extend(name);
    bytes
}
fn supplied_proof_case(
    values: impl FnOnce(&[u8]) -> Vec<Vec<u8>>,
) -> (Completed, Private, CollisionFixture) {
    let mut input = files(&[(1, 101, -1), (2, 102, 1), (3, 103, 2)]);
    input[0].2 = gameobject_db2(&[(1, 201)], &[]);
    input.extend([
        (101, None, wdt(None, None)),
        (102, None, wdt(None, None)),
        (103, None, wdt(None, None)),
        (201, None, m2(true)),
    ]);
    let f = CollisionFixture::new(&input);
    let (mut c, p) = perform(&f, Limits::default()).unwrap();
    let name = c.models.models().unwrap().next().unwrap().0.to_vec();
    let rows = values(&name);
    c.repository = p.spool(&f, "proof-cases", SpoolLimits::default());
    c.maps.clear();
    c.stats.maps = 0;
    c.stats.ancestor_steps = 0;
    c.stats.validation_bytes = 0;
    c.stats.bundle_validation_bytes = 0;
    c.stats.association_validation_bytes = 0;
    for (i, row) in rows.iter().enumerate() {
        let view = placement::DirBin::parse(row, placement::Limits::default()).unwrap();
        let records: Vec<_> = view.records().collect();
        let built = assembly::build(i as u32 + 1, &records, assembly::Limits::default()).unwrap();
        c.write_map(&built, 0).unwrap();
    }
    (c, p, f)
}
#[test]
fn private_proof_first_present_ancestor_preserves_grandchild_index() {
    let (mut c, _p, _f) = supplied_proof_case(|n| {
        vec![
            proof_rows(n, 7, false),
            proof_rows(n, 7, true),
            proof_rows(n, 7, true),
        ]
    });
    c.verify_parents().unwrap();
    assert_eq!(c.stats.ancestor_steps, 3);
    assert_eq!(c.maps[2].tiles[0].payload.len, 0);
}
#[test]
fn private_proof_nearer_mixed_payload_never_falls_back_farther() {
    let (mut c, _p, _f) = supplied_proof_case(|n| {
        let mut mixed = proof_rows(n, 8, false);
        mixed.extend(proof_rows(n, 7, true));
        vec![proof_rows(n, 7, false), mixed, proof_rows(n, 7, true)]
    });
    assert_eq!(
        c.verify_parents().unwrap_err().cause,
        Cause::Kind(Kind::Assembly(assembly::Error::Association))
    );
}
#[test]
fn private_proof_missing_ancestor_is_terminal() {
    let (mut c, _p, _f) = supplied_proof_case(|n| vec![vec![], vec![], proof_rows(n, 7, true)]);
    assert_eq!(
        c.verify_parents().unwrap_err().cause,
        Cause::Kind(Kind::MissingAncestor)
    );
}
#[test]
fn private_proof_parent_work_and_validation_caps_reject() {
    for axis in 0..2 {
        let (mut c, _p, _f) =
            supplied_proof_case(|n| vec![proof_rows(n, 7, false), proof_rows(n, 7, true), vec![]]);
        if axis == 0 {
            c.limits.max_ancestor_steps = 0;
        } else {
            c.limits.max_validation_bytes = 0;
        }
        assert_eq!(
            c.verify_parents().unwrap_err().cause,
            Cause::Kind(Kind::Limit)
        );
    }
}
#[test]
fn private_proof_twenty_five_complete_identity_differences_reject() {
    for axis in 0..25 {
        let (mut c, _p, _f) = supplied_proof_case(|name| {
            let mut child = proof_rows(name, 7, true);
            match axis {
                0 => put(&mut child, 2, 8),
                1..=5 => child[1] = axis as u8,
                6..=8 => put(&mut child, 6 + (axis - 6) * 4, 1f32.to_bits()),
                9..=11 => put(&mut child, 18 + (axis - 9) * 4, 15f32.to_bits()),
                12 => put(&mut child, 30, 2f32.to_bits()),
                13..=15 => put(&mut child, 34 + (axis - 13) * 4, 0.1f32.to_bits()),
                16..=18 => put(&mut child, 46 + (axis - 16) * 4, 1.5f32.to_bits()),
                19..=21 => put(&mut child, 34 + (axis - 19) * 4, 0x8000_0000),
                22 => child[0] |= 4,
                23 => child[62] = b'Z',
                _ => {
                    child.pop();
                    put(&mut child, 58, (name.len() - 1) as u32);
                }
            }
            vec![proof_rows(name, 7, false), child, vec![]]
        });
        assert_eq!(
            c.verify_parents().unwrap_err().cause,
            Cause::Kind(Kind::Assembly(assembly::Error::Association)),
            "case {axis}"
        );
    }
}

#[test]
fn source_change_guard_precedes_policy_code_without_losing_typed_cause() {
    let source = payload::CollectorError::Collision(crate::collision::Error::Store(
        crate::local_store::Error::InputChanged,
    ));
    let e = Error {
        cause: Cause::Callback(77u8),
        guards: Guards {
            models: selected::Guards {
                collector: Some(source),
                placements: None,
                archive: None,
            },
            maps: Some(payload::Error::Changed),
        },
    };
    assert_eq!(e.cause, Cause::Callback(77));
    assert_eq!(e.code(), source.code());
    assert_eq!(e.guards.maps, Some(payload::Error::Changed));
}

#[test]
fn sixteen_maps_stream_more_artifact_bytes_than_the_active_headroom() {
    let maps: Vec<_> = (1..=16).map(|id| (id, 100 + id, -1)).collect();
    let mut input = files(&maps);
    for &(_, fdid, _) in &maps {
        input.push((fdid, None, wdt(None, Some(111))));
    }
    // Distinct WDT FDIDs must not alias the one shared Obj0ADT/model FDID.
    let obj = 400;
    for row in input.iter_mut().filter(|r| (101..=116).contains(&r.0)) {
        row.2 = wdt(None, Some(obj));
    }
    let mut model = m2(true);
    for (i, n) in [0f32, 0., 0., 16533., 16533., 0.].into_iter().enumerate() {
        put(&mut model, 188 + i * 4, n.to_bits());
    }
    input.extend([(obj, None, m2_adt(500, 42)), (500, None, model)]);
    let f = CollisionFixture::new(&input);
    let p = Private::new();
    let models = prepared(&f, &p);
    let repo = p.spool(&f, "maps", SpoolLimits::default());
    let base = models.live().unwrap() + 2 * repo.limits().max_descriptor_bytes + GUARD_BYTES;
    let headroom = 1024 * 1024;
    let cap = base + headroom;
    let mut c = assemble(
        models,
        repo,
        Limits {
            max_memory_bytes: cap,
            ..Default::default()
        },
    )
    .unwrap();
    let stats = c.stats().unwrap();
    assert_eq!(stats.maps, 16);
    assert!(stats.output_bytes > headroom);
    assert!(stats.reserved_bytes <= cap);
    assert!(stats.peak_admitted_bytes <= cap);
    let mut files = 0;
    let mut bytes = 0;
    c.visit(0, |a| {
        files += 1;
        bytes += a.bytes().len();
        Ok::<_, u8>(())
    })
    .unwrap();
    assert_eq!((files, bytes), (stats.files, stats.output_bytes));
    assert!(c.stats().unwrap().peak_admitted_bytes <= cap);
    assert_eq!(c.repository.stats().unwrap().records, 16);
    c.verify_unchanged().unwrap();
}

#[test]
#[ignore = "requires explicit RCE_SPOOL_CROSS_VOLUME_PARENT on a second mounted filesystem"]
fn explicit_cross_volume_closed_package_matches_resident_rust() {
    let parent = PathBuf::from(
        std::env::var_os("RCE_SPOOL_CROSS_VOLUME_PARENT")
            .expect("explicit second-volume private parent required"),
    );
    let mut total_files = 0usize;
    let mut total_bytes = 0usize;
    for case in 0..4 {
        let mut input = match case {
            0 => files(&[]),
            1 => files(&[(1, 101, -1)]),
            2 => files(&[(2, 102, -1), (1, 101, -1)]),
            _ => files(&[(3, 103, 2), (2, 102, 1), (1, 101, -1)]),
        };
        match case {
            0 => {}
            1 => {
                input[0].2 = gameobject_db2(&[(1, 201), (2, 201), (3, 0), (4, 203)], &[]);
                let rows = [m2_adt(201, 42)[8..].to_vec(), m2_adt(202, 42)[8..].to_vec()].concat();
                input.extend([
                    (101, None, wdt(None, Some(111))),
                    (111, None, chunk(b"MDDF", &rows)),
                    (201, None, m2(true)),
                    (202, None, m2(true)),
                    (203, None, b"BLP2texture".to_vec()),
                ]);
            }
            2 => {
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
            }
            _ => input.extend([
                (101, None, wdt(None, Some(111))),
                (102, None, wdt(None, None)),
                (103, None, wdt(None, None)),
                (111, None, m2_adt(201, 42)),
                (201, None, m2(true)),
            ]),
        }
        let f = CollisionFixture::new(&input);
        let expected = old(&f).unwrap();
        let p = Private::captured_volumes(&parent, &f);
        let c = assemble(
            prepared(&f, &p),
            p.spool(&f, "maps", SpoolLimits::default()),
            Limits::default(),
        )
        .unwrap();
        let (mut c, p, _) = compare_completed(c, p, expected);
        let stats = c.stats().unwrap();
        total_files += stats.files;
        total_bytes += stats.output_bytes;
        assert_eq!(fs::read_dir(&p.base).unwrap().count(), 6);
        assert_eq!(c.repository.profile(), spool::Profile::CapturedVolumesV2);
        if case == 1 {
            assert_eq!((stats.selected_models, stats.unique_spawns), (2, 1));
        }
        if case == 2 {
            assert!(stats.selected_models >= 3);
        }
        if case == 3 {
            assert!(stats.parent_only_tiles > 0);
        }
        // Complete recorded repository identity remains enforced across volumes.
        let map_file = p.base.join("maps/00000000.payload");
        if map_file.exists() {
            mutate(&map_file);
            assert_eq!(
                c.verify_unchanged().unwrap_err().guards.maps,
                Some(payload::Error::Changed)
            );
        }
    }
    // No usable package can escape a fatal nearer-parent association mismatch.
    let mut input = files(&[(1, 101, -1), (2, 102, 1), (3, 103, 2)]);
    input.extend([
        (101, None, wdt_cells(&[(0, 0, 111)])),
        (102, None, wdt_cells(&[(1, 0, 112)])),
        (103, None, wdt(None, None)),
        (111, None, m2_adt(201, 42)),
        (112, None, m2_adt(202, 43)),
        (201, None, m2(true)),
        (202, None, m2(true)),
    ]);
    let f = CollisionFixture::new(&input);
    assert!(matches!(
        old(&f),
        Err(resident::Error::Assembly(assembly::Error::Association))
    ));
    let p = Private::captured_volumes(&parent, &f);
    let e = assemble(
        prepared(&f, &p),
        p.spool(&f, "maps", SpoolLimits::default()),
        Limits::default(),
    )
    .err()
    .unwrap();
    assert_eq!(
        e.cause,
        Cause::Kind(Kind::Assembly(assembly::Error::Association))
    );
    assert_eq!(fs::read_dir(&p.base).unwrap().count(), 6);
    assert!(total_files > 10 && total_bytes > 1000);
    println!(
        "captured-volume corpus: 4 exact complete packages, 1 exact terminal Association; {total_files} files, {total_bytes} bytes"
    );
}

fn resource_private() -> Private {
    let mut p = Private::new();
    p.resources = crate::collision::resources::Profile::DiskWorldV1;
    p
}
#[test]
fn explicit_disk_resources_complete_small_package_matches_bounded_bytes() {
    let f = one_m2();
    let expected = old(&f).unwrap();
    let p = resource_private();
    let c = assemble(
        prepared(&f, &p),
        p.spool(&f, "maps", p.resources.spool_limits()),
        p.resources.package_limits(),
    )
    .unwrap();
    let (c, _p, _expected) = compare_completed(c, p, expected);
    assert_eq!(
        c.prepared().repository.resources(),
        crate::collision::resources::Profile::DiskWorldV1
    );
}
#[test]
fn resource_policy_mixing_rejects_world_before_map_acquisition() {
    let f = one_m2();
    let p = resource_private();
    let mut store = f.open(false);
    let startup = gameobjects::spooled::extract_startup_fdid(
        &mut store,
        p.resources.startup_limits(),
        p.spool(&f, "models", p.resources.spool_limits()),
    )
    .unwrap();
    let inv = |name| {
        inventory::Inventory::new(
            p.spool(&f, name, p.resources.spool_limits()),
            p.resources.inventory_limits(),
        )
        .unwrap()
    };
    let mismatched = Spool::create(
        &p.base.join("placements"),
        ProtectedInputs::capture(f.path(), &[]).unwrap(),
        SpoolLimits::default(),
    )
    .unwrap();
    let e = placements::extract(
        startup,
        inv("wdts"),
        inv("caches"),
        mismatched,
        p.resources.stage_limits(),
    )
    .err()
    .unwrap();
    assert_eq!(
        e.cause,
        placements::Cause::Stage(placements::StageError::InvalidLimits)
    );
    assert_eq!(fs::read_dir(p.base.join("placements")).unwrap().count(), 0);
}
#[test]
fn resource_policy_mixing_rejects_selected_models_and_final_map_archive() {
    for phase in 0..2 {
        let f = one_m2();
        let p = resource_private();
        let ready = prepared(&f, &p);
        let wrong = Spool::create(
            &p.base.join("wrong-archive"),
            ProtectedInputs::capture(f.path(), &[]).unwrap(),
            SpoolLimits::default(),
        )
        .unwrap();
        if phase == 0 {
            let Prepared { stage, .. } = ready;
            assert_eq!(
                selected::prepare(stage, wrong, p.resources.model_limits())
                    .err()
                    .unwrap()
                    .cause,
                selected::Cause::Kind(selected::Kind::InvalidLimits)
            );
        } else {
            assert_eq!(
                assemble(ready, wrong, p.resources.package_limits())
                    .err()
                    .unwrap()
                    .cause,
                Cause::Kind(Kind::InvalidLimits)
            );
        }
        assert_eq!(
            fs::read_dir(p.base.join("wrong-archive")).unwrap().count(),
            0
        );
    }
}

#[test]
fn expanded_spool_descriptor_admission_crosses_old_record_ceiling_without_files() {
    let f = one_m2();
    let p = resource_private();
    let mut repo = p.spool(&f, "admission", p.resources.spool_limits());
    repo.prepare_records(4097, 16 * 1024 * 1024).unwrap();
    assert_eq!(repo.stats().unwrap().records, 0);
    assert_eq!(fs::read_dir(p.base.join("admission")).unwrap().count(), 0);
    assert!(
        repo.stats().unwrap().descriptor_bytes < p.resources.spool_limits().max_descriptor_bytes
    );
}
#[test]
fn expanded_spool_resource_fields_reject_upward_before_creation() {
    let f = one_m2();
    let p = resource_private();
    for axis in 0..6 {
        let mut l = p.resources.spool_limits();
        match axis {
            0 => l.max_records += 1,
            1 => l.max_disk_bytes += 1,
            2 => l.max_descriptor_bytes += 1,
            3 => l.max_reloads += 1,
            4 => l.max_reload_bytes += 1,
            _ => l.max_active_bytes += 1,
        }
        let path = p.base.join(format!("upward-{axis}"));
        assert_eq!(
            Spool::create_with_policy(
                &path,
                ProtectedInputs::capture(f.path(), &[]).unwrap(),
                l,
                spool::Profile::SameMountV1,
                p.resources
            )
            .err(),
            Some(payload::Error::InvalidLimits)
        );
        assert!(!path.exists());
    }
}
#[test]
fn explicit_collector_reads_and_requests_cross_old_4096_without_reset() {
    let f = CollisionFixture::new(&[(201, None, b"BLP2synthetic".to_vec())]);
    let p = resource_private();
    let mut store = f.open(false);
    let mut l = p.resources.collector_limits();
    l.max_reads = 4100;
    l.max_requests = 4100;
    let mut col = payload::SpoolingExtractor::new(
        &mut store,
        &gameobjects::EmptyLookup,
        l,
        p.spool(&f, "models", p.resources.spool_limits()),
    )
    .unwrap();
    for _ in 0..4100 {
        assert_eq!(
            col.extract_gameobject_fdid(201).unwrap(),
            crate::collision::GameObjectOutcome::Texture
        );
    }
    assert_eq!(col.facts().unwrap().reads, 4100);
    assert_eq!(
        col.extract_gameobject_fdid(201).err(),
        Some(payload::CollectorError::Collision(
            crate::collision::Error::Limit
        ))
    );
    assert_eq!(
        col.facts().err(),
        Some(payload::CollectorError::Collision(
            crate::collision::Error::Limit
        ))
    );
}
#[test]
fn disk_model_and_package_aggregate_ceilings_only_reduce() {
    let r = crate::collision::resources::Profile::DiskWorldV1;
    r.model_limits().validate_resources(r).unwrap();
    r.package_limits().validate_resources(r).unwrap();
    assert_eq!(
        r.model_limits().validate(),
        Err(selected::Kind::InvalidLimits)
    );
    assert_eq!(r.package_limits().validate(), Err(Kind::InvalidLimits));
    for axis in 0..11 {
        let mut l = r.package_limits();
        match axis {
            0 => l.max_models += 1,
            1 => l.max_output_bytes += 1,
            2 => l.max_vertices += 1,
            3 => l.max_triangles += 1,
            4 => l.max_raw_visit_bytes += 1,
            5 => l.max_bih_work += 1,
            6 => l.max_validation_bytes += 1,
            7 => l.max_files += 1,
            8 => l.max_records += 1,
            9 => l.max_associations += 1,
            _ => l.max_ancestor_steps += 1,
        }
        assert_eq!(l.validate_resources(r), Err(Kind::InvalidLimits));
    }
    for axis in 0..3 {
        let mut l = r.package_limits();
        match axis {
            0 => l.max_memory_bytes += 1,
            1 => l.assembly.max_spawns += 1,
            _ => l.assembly.bih.max_depth += 1,
        }
        assert_eq!(l.validate_resources(r), Err(Kind::InvalidLimits));
    }
}

#[test]
#[ignore = "explicit cumulative 4100-row and repeated-geometry corpus"]
fn explicit_disk_resources_startup_crosses_candidate_raw_and_vertex_work_limits() {
    let rows: Vec<_> = (1..=4100).map(|id| (id, 201)).collect();
    let vertices = 32769usize;
    let mut model = m2(true);
    model.resize(310 + vertices * 12, 0);
    put(&mut model, 224, vertices as u32);
    let f = CollisionFixture::new(&[
        (
            gameobjects::TABLE_FILE_DATA_ID,
            None,
            gameobject_db2(&rows, &[]),
        ),
        (201, None, model),
    ]);
    {
        let p = Private::new();
        let mut store = f.open(false);
        assert!(
            gameobjects::spooled::extract_startup_fdid(
                &mut store,
                gameobjects::Limits::default(),
                p.spool(&f, "models", SpoolLimits::default())
            )
            .is_err()
        );
    }
    let p = resource_private();
    let mut store = f.open(false);
    let mut startup = gameobjects::spooled::extract_startup_fdid(
        &mut store,
        p.resources.startup_limits(),
        p.spool(&f, "models", p.resources.spool_limits()),
    )
    .unwrap();
    assert_eq!(startup.collector_mut().resources(), p.resources);
    let data = startup.data().unwrap();
    let lists = data.lists();
    let s = lists.stats();
    assert_eq!(s.rows, 4100);
    assert_eq!(s.vertex_visits, 4100 * vertices as u64);
    assert!(s.raw_visit_bytes > forever_vmap::gameobject_models::MAX_RAW_VISIT_BYTES);
    assert!(s.vertex_visits > forever_vmap::gameobject_models::MAX_VERTEX_VISITS);
    let temp =
        forever_vmap::gameobject_models::TempList::parse(lists.temp_bytes(), Default::default())
            .unwrap();
    let dtree =
        forever_vmap::gameobject_models::Dtree::parse(lists.dtree_bytes(), Default::default())
            .unwrap();
    assert_eq!((temp.row_count(), dtree.row_count()), (4100, 4100));
    for (at, row) in dtree.rows().enumerate() {
        assert_eq!(row.display_id(), at as u32 + 1);
        assert_eq!(row.name(), b"FILE000000C9.xxx");
        assert_eq!(
            row.bounds_bits(),
            [0, 0, 0, 1f32.to_bits(), 1f32.to_bits(), 0]
        );
    }
    println!(
        "disk-resource corpus: 4100 exact list rows; {} RAW bytes; {} vertex visits",
        s.raw_visit_bytes, s.vertex_visits
    );
    startup.collector_mut().verify_unchanged().unwrap();
}

#[test]
fn explicit_leased_append_all_six_owners_match_resident_complete_package() {
    let f = one_m2();
    let expected = old(&f).unwrap();
    let mut p = resource_private();
    p.backend = spool::BackendProfile::LeasedAppendV1;
    let c = assemble(
        prepared(&f, &p),
        p.spool(&f, "maps", p.resources.spool_limits()),
        p.resources.package_limits(),
    )
    .unwrap();
    let (c, p, _expected) = compare_completed(c, p, expected);
    assert_eq!(
        c.prepared().repository.backend(),
        spool::BackendProfile::LeasedAppendV1
    );
    for name in ["models", "wdts", "caches", "placements", "vmos", "maps"] {
        let files: Vec<_> = fs::read_dir(p.base.join(name))
            .unwrap()
            .map(|f| f.unwrap().file_name())
            .collect();
        assert_eq!(files, [std::ffi::OsString::from("records.archive")]);
    }
}

fn leased_append_package_matrix(parent: Option<&Path>) {
    let mut total_files = 0usize;
    let mut total_bytes = 0usize;
    for case in 0..4 {
        let mut input = match case {
            0 => files(&[]),
            1 => files(&[(1, 101, -1)]),
            2 => files(&[(2, 102, -1), (1, 101, -1)]),
            _ => files(&[(3, 103, 2), (2, 102, 1), (1, 101, -1)]),
        };
        match case {
            0 => {}
            1 => {
                input[0].2 = gameobject_db2(&[(1, 201), (2, 201), (3, 0), (4, 203)], &[]);
                let rows = [m2_adt(201, 42)[8..].to_vec(), m2_adt(202, 42)[8..].to_vec()].concat();
                input.extend([
                    (101, None, wdt(None, Some(111))),
                    (111, None, chunk(b"MDDF", &rows)),
                    (201, None, m2(true)),
                    (202, None, m2(true)),
                    (203, None, b"BLP2texture".to_vec()),
                ]);
            }
            2 => {
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
            }
            _ => input.extend([
                (101, None, wdt(None, Some(111))),
                (102, None, wdt(None, None)),
                (103, None, wdt(None, None)),
                (111, None, m2_adt(201, 42)),
                (201, None, m2(true)),
            ]),
        }
        let f = CollisionFixture::new(&input);
        let expected = old(&f).unwrap();
        let mut p = if let Some(parent) = parent {
            Private::captured_volumes(parent, &f)
        } else {
            Private::new()
        };
        p.resources = crate::collision::resources::Profile::DiskWorldV1;
        p.backend = spool::BackendProfile::LeasedAppendV1;
        let c = assemble(
            prepared(&f, &p),
            p.spool(&f, "maps", SpoolLimits::default()),
            Limits::default(),
        )
        .unwrap();
        let (mut c, p, _) = compare_completed(c, p, expected);
        let stats = c.stats().unwrap();
        total_files += stats.files;
        total_bytes += stats.output_bytes;
        assert_eq!(fs::read_dir(&p.base).unwrap().count(), 6);
        for name in ["models", "wdts", "caches", "placements", "vmos", "maps"] {
            let names: Vec<_> = fs::read_dir(p.base.join(name))
                .unwrap()
                .map(|e| e.unwrap().file_name())
                .collect();
            assert_eq!(names, [std::ffi::OsString::from("records.archive")]);
        }
        assert_eq!(c.repository.profile(), p.profile);
        assert_eq!(
            c.repository.backend(),
            spool::BackendProfile::LeasedAppendV1
        );
        if case == 1 {
            assert_eq!((stats.selected_models, stats.unique_spawns), (2, 1));
        }
        if case == 2 {
            assert!(stats.selected_models >= 3);
        }
        if case == 3 {
            assert!(stats.parent_only_tiles > 0);
        }
        // Complete recorded repository identity remains enforced across volumes.
        let map_file = p.base.join("maps/records.archive");
        if map_file.exists() {
            fs::set_permissions(&map_file, fs::Permissions::from_mode(0o400)).unwrap();
            assert_eq!(
                c.verify_unchanged().unwrap_err().guards.maps,
                Some(payload::Error::Changed)
            );
        }
    }
    // No usable package can escape a fatal nearer-parent association mismatch.
    let mut input = files(&[(1, 101, -1), (2, 102, 1), (3, 103, 2)]);
    input.extend([
        (101, None, wdt_cells(&[(0, 0, 111)])),
        (102, None, wdt_cells(&[(1, 0, 112)])),
        (103, None, wdt(None, None)),
        (111, None, m2_adt(201, 42)),
        (112, None, m2_adt(202, 43)),
        (201, None, m2(true)),
        (202, None, m2(true)),
    ]);
    let f = CollisionFixture::new(&input);
    assert!(matches!(
        old(&f),
        Err(resident::Error::Assembly(assembly::Error::Association))
    ));
    let mut p = if let Some(parent) = parent {
        Private::captured_volumes(parent, &f)
    } else {
        Private::new()
    };
    p.resources = crate::collision::resources::Profile::DiskWorldV1;
    p.backend = spool::BackendProfile::LeasedAppendV1;
    let e = assemble(
        prepared(&f, &p),
        p.spool(&f, "maps", SpoolLimits::default()),
        Limits::default(),
    )
    .err()
    .unwrap();
    assert_eq!(
        e.cause,
        Cause::Kind(Kind::Assembly(assembly::Error::Association))
    );
    assert_eq!(fs::read_dir(&p.base).unwrap().count(), 6);
    assert!(total_files > 10 && total_bytes > 1000);
    println!(
        "leased-append corpus: 4 exact complete packages, 1 exact terminal Association; {total_files} files, {total_bytes} bytes"
    );
}

#[test]
fn leased_append_m2_wmo_parent_packages_and_terminal_association_match() {
    leased_append_package_matrix(None);
}
#[test]
#[ignore = "explicit leased-archive package matrix on a second mounted filesystem"]
fn explicit_leased_append_cross_volume_complete_package_matrix() {
    let parent = PathBuf::from(
        std::env::var_os("RCE_SPOOL_CROSS_VOLUME_PARENT")
            .expect("explicit second-volume private parent required"),
    );
    leased_append_package_matrix(Some(&parent));
}
#[test]
fn leased_append_backend_mixing_rejects_wdt_cache_or_placement_before_world() {
    for wrong_phase in 0..3 {
        let f = one_m2();
        let mut p = resource_private();
        p.backend = spool::BackendProfile::LeasedAppendV1;
        let mut store = f.open(false);
        let startup = gameobjects::spooled::extract_startup_fdid(
            &mut store,
            p.resources.startup_limits(),
            p.spool(&f, "models", p.resources.spool_limits()),
        )
        .unwrap();
        let make = |name, wrong| {
            let backend = if wrong {
                spool::BackendProfile::FileSetV1
            } else {
                p.backend
            };
            Spool::create_with_backend(
                &p.base.join(name),
                ProtectedInputs::capture(f.path(), &[]).unwrap(),
                p.resources.spool_limits(),
                p.profile,
                p.resources,
                backend,
            )
            .unwrap()
        };
        let wdts = inventory::Inventory::new(
            make("wdts", wrong_phase == 0),
            p.resources.inventory_limits(),
        )
        .unwrap();
        let caches = inventory::Inventory::new(
            make("caches", wrong_phase == 1),
            p.resources.inventory_limits(),
        )
        .unwrap();
        let repository = make("placements", wrong_phase == 2);
        let e = placements::extract(
            startup,
            wdts,
            caches,
            repository,
            p.resources.stage_limits(),
        )
        .err()
        .unwrap();
        assert_eq!(
            e.cause,
            placements::Cause::Stage(placements::StageError::InvalidLimits)
        );
        for name in ["models", "wdts", "caches", "placements"] {
            let count = fs::read_dir(p.base.join(name)).unwrap().count();
            let expected = match (wrong_phase, name) {
                (0, "wdts") | (1, "caches") | (2, "placements") => 0,
                _ => 1,
            };
            assert_eq!(count, expected);
        }
    }
}
#[test]
fn leased_append_backend_mixing_rejects_selected_models_and_final_map_archive() {
    for phase in 0..2 {
        let f = one_m2();
        let mut p = resource_private();
        p.backend = spool::BackendProfile::LeasedAppendV1;
        let ready = prepared(&f, &p);
        let wrong = Spool::create_with_backend(
            &p.base.join("wrong-archive"),
            ProtectedInputs::capture(f.path(), &[]).unwrap(),
            p.resources.spool_limits(),
            p.profile,
            p.resources,
            spool::BackendProfile::FileSetV1,
        )
        .unwrap();
        if phase == 0 {
            let Prepared { stage, .. } = ready;
            assert_eq!(
                selected::prepare(stage, wrong, p.resources.model_limits())
                    .err()
                    .unwrap()
                    .cause,
                selected::Cause::Kind(selected::Kind::InvalidLimits)
            );
        } else {
            assert_eq!(
                assemble(ready, wrong, p.resources.package_limits())
                    .err()
                    .unwrap()
                    .cause,
                Cause::Kind(Kind::InvalidLimits)
            );
        }
        assert_eq!(
            fs::read_dir(p.base.join("wrong-archive")).unwrap().count(),
            0
        );
    }
}

#[test]
fn selected_domain_complete_disk_package_matches_independent_closure_world() {
    use crate::collision::plan::MapSelection;
    let mut original = files(&[
        (90, 190, -1),
        (3, 103, 2),
        (1, 101, -1),
        (2, 102, 1),
        (80, 180, -1),
    ]);
    original[0].2 = gameobject_db2(&[(4, 204)], &[]);
    original.extend([
        (180, None, b"unrelated malformed WDT".to_vec()),
        (101, None, wdt(None, Some(111))),
        (102, None, wdt(None, None)),
        (103, None, wdt(None, None)),
        (111, None, m2_adt(201, 42)),
        (201, None, m2(true)),
        (204, None, m2(true)),
    ]);
    // An independently complete closure-only input preserves relative source
    // order and proves the selected serial ID domain, not full-world IDs.
    let mut closure = original.clone();
    closure[1].2 = map_db2(&[(3, 103, 2), (1, 101, -1), (2, 102, 1)], &[]);
    let reference = CollisionFixture::new(&closure);
    let expected = old(&reference).unwrap();
    let f = CollisionFixture::new(&original);
    let p = Private::new();
    let selected = prepared_selected(&f, &p, MapSelection::MapAndAncestors(3));
    assert_eq!(selected.stage().world().topology().all().len(), 5);
    assert_eq!(
        selected
            .stage()
            .world()
            .topology()
            .selected_active()
            .map(|m| m.id())
            .collect::<Vec<_>>(),
        [3, 1, 2]
    );
    let c = assemble(
        selected,
        p.spool(&f, "maps", p.resources.spool_limits()),
        p.resources.package_limits(),
    )
    .unwrap();
    let (c, _p, _) = compare_completed(c, p, expected);
    assert_eq!(c.stats().unwrap().maps, 3);
    assert_eq!(c.stats().unwrap().selected_models, 2);
    assert_eq!(c.stats().unwrap().parent_only_tiles, 1);
}
#[test]
fn selected_domain_ids_are_deterministic_and_can_differ_from_full_world_slice() {
    use crate::collision::plan::MapSelection;
    let mut input = files(&[(90, 190, -1), (1, 101, -1)]);
    input.extend([
        (190, None, wdt(None, Some(191))),
        (191, None, m2_adt(201, 99)),
        (101, None, wdt(None, Some(111))),
        (111, None, m2_adt(201, 42)),
        (201, None, m2(true)),
    ]);
    let f = CollisionFixture::new(&input);
    let all = old(&f).unwrap();
    let all_bytes = all
        .maps()
        .iter()
        .find(|m| m.map_id() == 1)
        .unwrap()
        .ordered_dir_bin()
        .to_vec();
    let mut results = Vec::new();
    for _ in 0..2 {
        let p = Private::new();
        let mut selected = prepared_selected(&f, &p, MapSelection::MapAndAncestors(1));
        let selected_dir = selected
            .stage
            .with_map(1, 64 * 1024 * 1024, |m| Ok::<_, u8>(m.bytes().to_vec()))
            .unwrap();
        assert_ne!(selected_dir, all_bytes);
        let mut c = assemble(
            selected,
            p.spool(&f, "maps", p.resources.spool_limits()),
            p.resources.package_limits(),
        )
        .unwrap();
        let bytes = c
            .with_map(1, 0, |m| {
                Ok::<_, u8>(
                    m.artifacts()
                        .map(|a| (a.name().to_string(), a.bytes().to_vec()))
                        .collect::<Vec<_>>(),
                )
            })
            .unwrap();
        results.push(bytes);
    }
    assert_eq!(results[0], results[1]);
    assert!(!all_bytes.is_empty());
}
