// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: complete synthetic CASC to assembled package tests.
use super::super::world::{MAP_FILE_DATA_ID, MATERIAL_FILE_DATA_ID, TYPE_FILE_DATA_ID};
use super::*;
use crate::local_store::tests::{CollisionFile, CollisionFixture};

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
fn perform(f: &CollisionFixture, limits: Limits) -> Result<Completed, Error> {
    let mut store = f.open(false);
    let startup =
        gameobjects::extract_startup_fdid(&mut store, gameobjects::Limits::default()).unwrap();
    extract(startup, limits)
}
fn staged(f: &CollisionFixture) -> (world::Completed, Stage) {
    let mut store = f.open(false);
    let startup =
        gameobjects::extract_startup_fdid(&mut store, gameobjects::Limits::default()).unwrap();
    let mut stage = Stage::new(Limits::default());
    let world = world::extract(startup, &mut stage, world::Limits::default()).unwrap();
    (world, stage)
}
fn names(out: &Completed) -> Vec<String> {
    out.artifacts().map(|a| a.name().to_string()).collect()
}
fn simple() -> Vec<CollisionFile<'static>> {
    let mut fs = files(&[(1, 101, -1)]);
    fs.extend([
        (10, None, m2(true)),
        (101, None, wdt(None, Some(201))),
        (201, None, m2_adt(10, 77)),
    ]);
    fs
}
fn record(id: u32, parent: bool, name: &[u8]) -> Vec<u8> {
    let mut bytes = vec![0; 38];
    bytes[0] = if parent { 2 } else { 0 };
    put(&mut bytes, 2, id);
    put(&mut bytes, 30, 1f32.to_bits());
    put(&mut bytes, 34, name.len() as u32);
    bytes.extend_from_slice(name);
    bytes
}
fn replace(stage: &mut Stage, id: u32, bytes: Vec<u8>) {
    let at = stage.maps.binary_search_by_key(&id, |m| m.id).unwrap();
    let old = std::mem::take(&mut stage.maps[at].bytes);
    let old_count = placement::DirBin::parse(&old, placement::Limits::default())
        .unwrap()
        .record_count();
    let old_capacity = old.capacity();
    drop(old);
    stage.memory.release(old_capacity);
    stage.records -= old_count;
    stage.append(id, &bytes).unwrap();
}
fn inherited_fixture() -> CollisionFixture {
    let mut fs = files(&[(1, 101, -1), (2, 102, 1), (3, 103, 2)]);
    fs[0].2 = gameobject_db2(&[(1, 10)], &[]);
    fs.extend([
        (10, None, m2(true)),
        (101, None, wdt(None, None)),
        (102, None, wdt(None, None)),
        (103, None, wdt(None, None)),
    ]);
    CollisionFixture::new(&fs)
}
#[test]
fn empty_authenticated_world_has_only_immutable_dtree() {
    let out = perform(&CollisionFixture::new(&files(&[])), Limits::default()).unwrap();
    assert_eq!(names(&out), ["GameObjectModels.dtree"]);
    assert_eq!(out.artifacts().next().unwrap().bytes(), b"VMAP_4.E");
    assert_eq!(out.world().startup().extraction().facts().reads, 4);
    assert_eq!(out.stats().output_bytes, 8);
    assert_eq!(out.stats().files, 1);
}
#[test]
fn empty_active_maps_keep_exact_inventory_numeric_order_and_nested_names() {
    let mut fs = files(&[(9, 109, -1), (2, 102, -1)]);
    fs.extend([(109, None, wdt(None, None)), (102, None, wdt(None, None))]);
    let out = perform(&CollisionFixture::new(&fs), Limits::default()).unwrap();
    assert_eq!(
        names(&out),
        [
            "GameObjectModels.dtree",
            "0002/0002.vmtree",
            "0009/0009.vmtree"
        ]
    );
    for map in out.maps() {
        assert_eq!(map.tree_bytes().len(), 56);
        assert!(map.tiles().is_empty());
    }
    assert_eq!(out.stats().files, 3);
}
#[test]
fn startup_duplicate_rows_and_world_repeated_requests_keep_one_vmo() {
    let mut fs = simple();
    fs[0].2 = gameobject_db2(&[(2, 10), (2, 10)], &[]);
    fs.last_mut().unwrap().2.extend(m2_adt(10, 77));
    let out = perform(&CollisionFixture::new(&fs), Limits::default()).unwrap();
    assert_eq!(out.stats().selected_models, 1);
    assert_eq!(out.maps()[0].stats().input_records, 2);
    assert_eq!(out.maps()[0].stats().unique_spawns, 1);
    assert_eq!(out.world().startup().data().lists().stats().rows, 2);
    assert_eq!(
        names(&out),
        [
            "GameObjectModels.dtree",
            "FILE0000000A.xxx.vmo",
            "0001/0001.vmtree",
            "0001/0001_00_00.vmtile",
            "0001/0001_00_00.vmtileidx"
        ]
    );
}
#[test]
fn duplicate_id_different_name_is_in_complete_model_union() {
    let mut fs = simple();
    fs.last_mut().unwrap().2.extend(m2_adt(11, 77));
    fs.push((11, None, m2(true)));
    let out = perform(&CollisionFixture::new(&fs), Limits::default()).unwrap();
    assert_eq!(out.maps()[0].stats().unique_spawns, 1);
    assert_eq!(out.stats().selected_models, 2);
    assert_eq!(out.stats().vertices, 6);
    assert_eq!(
        out.models().map(|(n, _)| n).collect::<Vec<_>>(),
        [
            b"FILE0000000A.xxx".as_slice(),
            b"FILE0000000B.xxx".as_slice()
        ]
    );
}
#[test]
fn unused_acquired_doodad_is_not_converted_even_with_selected_limit_one() {
    let mut fs = files(&[(1, 101, -1)]);
    fs.extend([
        (10, None, wmo_root(11, Some(12))),
        (11, None, wmo_group(true, false, false)),
        (12, None, m2(true)),
        (101, None, wdt(Some(10), None)),
    ]);
    let out = perform(
        &CollisionFixture::new(&fs),
        Limits {
            max_models: 1,
            ..Limits::default()
        },
    )
    .unwrap();
    assert_eq!(out.world().startup().extraction().models().len(), 2);
    assert_eq!(out.stats().selected_models, 1);
    assert_eq!(out.models().next().unwrap().0, b"FILE0000000A.xxx");
}
#[test]
fn startup_only_model_is_selected_without_any_map_placement() {
    let mut fs = files(&[]);
    fs[0].2 = gameobject_db2(&[(1, 10)], &[]);
    fs.push((10, None, m2(true)));
    let out = perform(&CollisionFixture::new(&fs), Limits::default()).unwrap();
    assert_eq!(out.stats().maps, 0);
    assert_eq!(out.stats().selected_models, 1);
    assert_eq!(out.stats().files, 2);
}
#[test]
fn inherited_payload_uses_real_chain_and_preserves_child_index() {
    let (world, mut stage) = staged(&inherited_fixture());
    replace(&mut stage, 1, record(7, false, b"FILE0000000A.xxx"));
    replace(&mut stage, 2, record(7, true, b"FILE0000000A.xxx"));
    replace(&mut stage, 3, record(7, true, b"FILE0000000A.xxx"));
    let out = build(world, stage, Limits::default()).unwrap();
    assert!(out.maps()[0].tiles()[0].payload().is_some());
    assert!(out.maps()[1].tiles()[0].payload().is_none());
    assert!(out.maps()[2].tiles()[0].payload().is_none());
    assert_eq!(out.stats().ancestor_steps, 3);
    assert!(out.stats().validation_bytes > 0);
    assert_eq!(
        out.maps()[0].tiles()[0].index_bytes(),
        out.maps()[2].tiles()[0].index_bytes()
    );
}
#[test]
fn nearer_existing_mismatching_payload_is_terminal_not_repaired() {
    let (world, mut stage) = staged(&inherited_fixture());
    replace(&mut stage, 1, record(7, false, b"FILE0000000A.xxx"));
    let mut mixed = record(8, false, b"FILE0000000A.xxx");
    mixed.extend(record(7, true, b"FILE0000000A.xxx"));
    replace(&mut stage, 2, mixed);
    replace(&mut stage, 3, record(7, true, b"FILE0000000A.xxx"));
    assert_eq!(
        build(world, stage, Limits::default()).err(),
        Some(Error::Assembly(assembly::Error::Association))
    );
}
#[test]
fn missing_ancestor_payload_is_terminal() {
    let (world, mut stage) = staged(&inherited_fixture());
    replace(&mut stage, 3, record(7, true, b"FILE0000000A.xxx"));
    assert_eq!(
        build(world, stage, Limits::default()).err(),
        Some(Error::MissingAncestor)
    );
}
#[test]
fn inherited_work_caps_are_aggregate_and_enforced_before_checks() {
    for limits in [
        Limits {
            max_ancestor_steps: 0,
            ..Limits::default()
        },
        Limits {
            max_validation_bytes: 0,
            ..Limits::default()
        },
    ] {
        let (world, mut stage) = staged(&inherited_fixture());
        replace(&mut stage, 1, record(7, false, b"FILE0000000A.xxx"));
        replace(&mut stage, 2, record(7, true, b"FILE0000000A.xxx"));
        assert_eq!(build(world, stage, limits).err(), Some(Error::Limit));
    }
}
#[test]
fn public_runner_parent_cache_replay_closes_against_original_payload() {
    let mut fs = files(&[(1, 101, -1), (2, 102, 1)]);
    fs.extend([
        (101, None, wdt(None, Some(201))),
        (102, None, wdt(None, None)),
        (201, None, m2_adt(10, 77)),
        (10, None, m2(true)),
    ]);
    let out = perform(&CollisionFixture::new(&fs), Limits::default()).unwrap();
    assert_eq!(out.stats().selected_models, 1);
    assert_eq!(out.maps().len(), 2);
    assert_eq!(out.stats().ancestor_steps, 1);
}
#[test]
fn wrong_or_missing_stage_inventory_fails_closed() {
    for extra in [false, true] {
        let (world, mut stage) = staged(&inherited_fixture());
        if extra {
            stage.append(42, &[]).unwrap();
        } else {
            stage.maps.pop();
        }
        assert_eq!(
            build(world, stage, Limits::default()).err(),
            Some(Error::Inventory)
        );
    }
}
#[test]
fn discarded_duplicate_with_missing_model_still_rejects() {
    let (world, mut stage) = staged(&inherited_fixture());
    let mut bytes = record(7, false, b"FILE0000000A.xxx");
    bytes.extend(record(7, false, b"absent.m2"));
    replace(&mut stage, 1, bytes);
    assert_eq!(
        build(world, stage, Limits::default()).err(),
        Some(Error::Binding)
    );
}
#[test]
fn unbounded_wmo_is_not_reinterpreted_as_m2() {
    let mut fs = files(&[(1, 101, -1)]);
    fs[0].2 = gameobject_db2(&[(1, 10)], &[]);
    fs.extend([
        (10, None, wmo_root(11, None)),
        (11, None, wmo_group(true, false, false)),
        (101, None, wdt(None, None)),
    ]);
    let (world, mut stage) = staged(&CollisionFixture::new(&fs));
    replace(&mut stage, 1, record(1, false, b"FILE0000000A.xxx"));
    assert_eq!(
        build(world, stage, Limits::default()).err(),
        Some(Error::Class)
    );
}
#[test]
fn cached_startup_liquids_remain_immutable_through_world_and_vmo() {
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
    let out = perform(&CollisionFixture::new(&fs), Limits::default()).unwrap();
    let liquids = out
        .models()
        .map(|(_, m)| m.stats().liquid_groups)
        .collect::<Vec<_>>();
    assert_eq!(liquids, [1, 0]);
}
#[test]
fn aggregate_primitive_caps_reject_before_any_vmo_is_returned() {
    let f = CollisionFixture::new(&simple());
    for limits in [
        Limits {
            max_vertices: 2,
            ..Limits::default()
        },
        Limits {
            max_triangles: 0,
            ..Limits::default()
        },
        Limits {
            max_raw_visit_bytes: 0,
            ..Limits::default()
        },
    ] {
        assert_eq!(perform(&f, limits).err(), Some(Error::Limit));
    }
}
#[test]
fn aggregate_record_model_map_and_association_caps_are_not_per_call() {
    let f = CollisionFixture::new(&simple());
    for limits in [
        Limits {
            max_records: 0,
            ..Limits::default()
        },
        Limits {
            max_models: 0,
            ..Limits::default()
        },
        Limits {
            max_maps: 0,
            ..Limits::default()
        },
        Limits {
            max_associations: 0,
            ..Limits::default()
        },
        Limits {
            max_bih_work: 0,
            ..Limits::default()
        },
    ] {
        assert!(perform(&f, limits).is_err());
    }
}
#[test]
fn artifact_count_and_bytes_caps_have_exact_positive_boundary() {
    let f = CollisionFixture::new(&simple());
    let first = perform(&f, Limits::default()).unwrap();
    let s = first.stats();
    drop(first);
    assert!(
        perform(
            &f,
            Limits {
                max_files: s.files,
                max_output_bytes: s.output_bytes,
                ..Limits::default()
            }
        )
        .is_ok()
    );
    for limits in [
        Limits {
            max_files: s.files - 1,
            ..Limits::default()
        },
        Limits {
            max_output_bytes: s.output_bytes - 1,
            ..Limits::default()
        },
    ] {
        assert!(perform(&f, limits).is_err());
    }
}
#[test]
fn stage_capacity_and_growth_overlap_are_admitted_before_reserve() {
    let bytes = record(1, false, b"x");
    let mut stage = Stage::new(Limits::default());
    stage.append(1, &bytes).unwrap();
    let resident = stage.memory.live;
    stage.memory.cap = resident + bytes.len();
    assert_eq!(stage.append(1, &bytes).err(), Some(Error::Limit));
    assert_eq!(stage.maps[0].bytes, bytes);
    assert_eq!(stage.records, 1);
}
#[test]
fn empty_map_staging_still_charges_metadata_and_invalid_record_is_not_appended() {
    let mut stage = Stage::new(Limits {
        max_stage_bytes: 0,
        ..Limits::default()
    });
    assert_eq!(stage.append(1, &[]).err(), Some(Error::Limit));
    let mut stage = Stage::new(Limits::default());
    assert!(stage.append(1, &[0]).is_err());
    assert!(stage.maps.is_empty());
}
#[test]
fn limits_only_reduce_and_stage_partition_cannot_underflow() {
    let f = CollisionFixture::new(&files(&[]));
    for limits in [
        Limits {
            max_memory_bytes: MAX_MEMORY_BYTES + 1,
            ..Limits::default()
        },
        Limits {
            max_models: MAX_MODELS + 1,
            ..Limits::default()
        },
        Limits {
            max_validation_bytes: MAX_VALIDATION_BYTES + 1,
            ..Limits::default()
        },
    ] {
        assert_eq!(perform(&f, limits).err(), Some(Error::InvalidLimits));
    }
    assert_eq!(
        perform(
            &f,
            Limits {
                max_memory_bytes: 1,
                ..Limits::default()
            }
        )
        .err(),
        Some(Error::Limit)
    );
}
#[test]
fn completed_residency_includes_world_store_and_all_artifact_buffers() {
    let out = perform(&CollisionFixture::new(&simple()), Limits::default()).unwrap();
    let s = out.stats();
    let sum = out.artifacts().map(|a| a.bytes().len()).sum::<usize>();
    assert_eq!(sum, s.output_bytes);
    assert_eq!(out.artifacts().count(), s.files);
    let minimum = out.world().resident_bytes()
        + out.world().store_resident_bytes()
        + out
            .maps()
            .iter()
            .map(|m| m.stats().resident_bytes)
            .sum::<usize>()
        + out
            .models()
            .map(|(_, m)| m.stats().resident_bytes)
            .sum::<usize>();
    assert!(s.resident_bytes >= minimum);
    assert!(s.peak_bytes >= s.resident_bytes);
    assert!(s.proof_bytes > 0);
}
#[test]
fn late_acquisition_error_or_changed_input_never_returns_completed_package() {
    let mut fs = simple();
    fs.last_mut().unwrap().2.push(0);
    let f = CollisionFixture::new(&fs);
    assert!(perform(&f, Limits::default()).is_err());
    let f = CollisionFixture::new(&simple());
    let mut store = f.open(false);
    let startup =
        gameobjects::extract_startup_fdid(&mut store, gameobjects::Limits::default()).unwrap();
    f.change_metadata();
    assert!(extract(startup, Limits::default()).is_err());
}
#[test]
fn canonical_artifact_coordinates_use_x_then_y_and_nested_map_directory() {
    assert_eq!(
        ArtifactName::Tile {
            map: 7,
            x: 123,
            y: 5,
            index: false
        }
        .to_string(),
        "0007/0007_123_05.vmtile"
    );
    assert_eq!(
        ArtifactName::Tile {
            map: u32::MAX,
            x: 32767,
            y: 255,
            index: true
        }
        .to_string(),
        "4294967295/4294967295_32767_255.vmtileidx"
    );
}

#[test]
fn wmo_raw_validation_visits_count_all_five_passes_with_exact_cap() {
    let mut fs = files(&[]);
    fs[0].2 = gameobject_db2(&[(1, 10)], &[]);
    fs.extend([
        (10, None, wmo_root(11, None)),
        (11, None, wmo_group(true, false, true)),
    ]);
    let fixture = CollisionFixture::new(&fs);
    let out = perform(&fixture, Limits::default()).unwrap();
    let raw_len = out.world().startup().extraction().models()[0]
        .raw()
        .unwrap()
        .len() as u64;
    let expected = 5 * raw_len;
    assert_eq!(out.stats().raw_visit_bytes, expected);
    drop(out);
    let out = perform(
        &fixture,
        Limits {
            max_raw_visit_bytes: expected,
            ..Limits::default()
        },
    )
    .unwrap();
    assert_eq!(out.stats().raw_visit_bytes, expected);
    drop(out);
    assert_eq!(
        perform(
            &fixture,
            Limits {
                max_raw_visit_bytes: expected - 1,
                ..Limits::default()
            }
        )
        .err(),
        Some(Error::Limit)
    );
}
