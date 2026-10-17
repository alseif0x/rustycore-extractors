// SPDX-License-Identifier: GPL-3.0-or-later
// Authored by RustyCore Extractors, 2026-10-17.
//! Synthetic fixture assembly only. The numeric WDC5 and CASC builders are
//! retained authored test kernels from forever-casc; no client assets or IO at
//! source-authoring time. Integrity helpers reuse their original source bytes.
use forever_casc::collision::{gameobjects, world as strict_world};
use forever_db2::catalog;
use std::{
    fs,
    os::unix::fs::DirBuilderExt,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};
use strict_world::{MAP_FILE_DATA_ID, MATERIAL_FILE_DATA_ID, TYPE_FILE_DATA_ID};
#[path = "../../../../../forever-casc/src/local_index/jenkins.rs"]
mod jenkins;
#[path = "../../../../../forever-casc/src/blte/md5.rs"]
mod md5;
const ROOT_EKEY: [u8; 16] = [0x33; 16];
const ENCODING_EKEY: [u8; 16] = [0x44; 16];
type Route = ([u8; 16], u32, Vec<[u8; 16]>);
type CollisionFile<'a> = (u32, Option<&'a [u8]>, Vec<u8>);
fn hex(key: [u8; 16]) -> String {
    key.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn build_info(key: [u8; 16], version: &str) -> String {
    format!(
        "Product!STRING:0|Active!DEC:1|Version!STRING:0|Build Key!HEX:16|CDN Key!HEX:16|Tags!STRING:0\nwow_classic_beta|1|{version}|{}|{}|enUS\n",
        hex(key),
        hex([0x55; 16])
    )
}

fn encoding_file(rows: &[Route]) -> Vec<u8> {
    let mut page = Vec::new();
    for (key, size, ekeys) in rows {
        page.extend_from_slice(&(ekeys.len() as u16).to_le_bytes());
        page.extend_from_slice(&size.to_be_bytes());
        page.extend_from_slice(key);
        for ekey in ekeys {
            page.extend_from_slice(ekey);
        }
    }
    assert!(page.len() < 1024);
    page.resize(1024, 0);
    let mut bytes = b"EN\x01\x10\x10".to_vec();
    bytes.extend_from_slice(&1_u16.to_be_bytes());
    bytes.extend_from_slice(&1_u16.to_be_bytes());
    bytes.extend_from_slice(&1_u32.to_be_bytes());
    bytes.extend_from_slice(&0_u32.to_be_bytes());
    bytes.push(0);
    bytes.extend_from_slice(&0_u32.to_be_bytes());
    bytes.extend_from_slice(&rows[0].0);
    bytes.extend_from_slice(&md5::digest(&page));
    bytes.extend_from_slice(&page);
    bytes
}

fn index_entry(key: [u8; 16], offset: u64, size: u32) -> [u8; 18] {
    let mut row = [0; 18];
    row[..9].copy_from_slice(&key[..9]);
    row[9..14].copy_from_slice(&offset.to_be_bytes()[3..]);
    row[14..].copy_from_slice(&size.to_le_bytes());
    row
}

fn index_file(bucket: u8, rows: &[[u8; 18]]) -> Vec<u8> {
    let mut bytes = vec![0; 32];
    bytes[..4].copy_from_slice(&16_u32.to_le_bytes());
    bytes[8..10].copy_from_slice(&7_u16.to_le_bytes());
    bytes[10] = bucket;
    bytes[12..16].copy_from_slice(&[4, 5, 9, 30]);
    let header_hash = jenkins::hashlittle(&bytes[8..24], 0);
    bytes[4..8].copy_from_slice(&header_hash.to_le_bytes());
    bytes.extend_from_slice(&(rows.len() as u32 * 18).to_le_bytes());
    let mut checksum = (0, 0);
    for row in rows {
        checksum = jenkins::hashlittle2(row, checksum.0, checksum.1);
    }
    bytes.extend_from_slice(&checksum.0.to_le_bytes());
    for row in rows {
        bytes.extend_from_slice(row);
    }
    bytes
}
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
fn stamp(build: u32, id: u32, b: &mut [u8]) {
    let table = match id {
        gameobjects::TABLE_FILE_DATA_ID => "GameObjectDisplayInfo",
        strict_world::MAP_FILE_DATA_ID => "Map",
        strict_world::MATERIAL_FILE_DATA_ID => "LiquidMaterial",
        strict_world::TYPE_FILE_DATA_ID => "LiquidType",
        _ => unreachable!(),
    };
    let catalog = match build {
        70374 => catalog::WOWDBDEFS_70374_CATALOG_ID,
        70425 => catalog::WOWDBDEFS_70425_CATALOG_ID,
        _ => unreachable!(),
    };
    let m = catalog::resolve_for(catalog, table)
        .unwrap()
        .unwrap()
        .metadata();
    assert_eq!(
        u32::from_le_bytes(b[156..160].try_into().unwrap()),
        m.layout_hash
    );
    put(b, 152, m.table_hash.unwrap());
}

static NEXT: AtomicUsize = AtomicUsize::new(0);
pub(super) struct Fixture {
    pub root: PathBuf,
    pub client: PathBuf,
    pub scratch: PathBuf,
    pub output: PathBuf,
}
impl Fixture {
    pub fn new(maps: &[(u32, u32, i16)], extra: &[CollisionFile<'_>]) -> Self {
        let root = std::env::temp_dir().join(format!(
            "rce-v14-genuine-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::DirBuilder::new().mode(0o700).create(&root).unwrap();
        let client = root.join("client");
        fs::create_dir(&client).unwrap();
        let data = client.join("Data/data");
        fs::create_dir_all(&data).unwrap();
        let scratch = root.join("scratch");
        fs::DirBuilder::new().mode(0o700).create(&scratch).unwrap();
        let mut files = files(maps);
        for (id, _, b) in &mut files {
            stamp(70425, *id, b);
        }
        files.extend_from_slice(extra);
        let mut root_bytes = b"TSFM".to_vec();
        for n in [
            20,
            2,
            files.len() as u32,
            files.iter().filter(|f| f.1.is_some()).count() as u32,
        ] {
            root_bytes.extend(n.to_le_bytes());
        }
        let mut routes = Vec::new();
        let mut records = Vec::new();
        for (ordinal, (id, name, b)) in files.iter().enumerate() {
            let key = md5::digest(b);
            let ekey = [0x60 + ordinal as u8; 16];
            routes.push((key, b.len() as u32, vec![ekey]));
            records.push((ekey, b.clone()));
            for n in [1u32, 2, if name.is_some() { 0 } else { 0x10000000 }, 0] {
                root_bytes.extend(n.to_le_bytes());
            }
            root_bytes.push(0);
            root_bytes.extend(id.to_le_bytes());
            root_bytes.extend(key);
            if let Some(name) = name {
                root_bytes.extend(
                    forever_casc::root::name::hash_name(name, 1024)
                        .unwrap()
                        .to_le_bytes(),
                );
            }
        }
        let root_key = md5::digest(&root_bytes);
        routes.insert(0, (root_key, root_bytes.len() as u32, vec![ROOT_EKEY]));
        let encoding = encoding_file(&routes);
        let enc_key = md5::digest(&encoding);
        let config=format!("root = {}\nencoding = {} {}\nencoding-size = {} {}\nbuild-uid = wow_classic_beta\nbuild-name = synthetic-1.60.1.70425\n",hex(root_key),hex(enc_key),hex(ENCODING_EKEY),encoding.len(),encoding.len()+39).into_bytes();
        let config_key = md5::digest(&config);
        let key = hex(config_key);
        let cp = client.join("Data/config").join(&key[..2]).join(&key[2..4]);
        fs::create_dir_all(&cp).unwrap();
        fs::write(cp.join(key), config).unwrap();
        fs::write(
            client.join(".build.info"),
            build_info(config_key, "1.60.1.70425"),
        )
        .unwrap();
        records.push((ROOT_EKEY, root_bytes));
        records.push((ENCODING_EKEY, encoding));
        let mut archive = Vec::new();
        let mut rows = Vec::new();
        for (key, b) in records {
            let at = archive.len() as u64;
            archive.extend([0xa5; 30]);
            archive.extend(b"BLTE\0\0\0\0");
            archive.push(b'N');
            archive.extend(&b);
            rows.push(index_entry(key, at, (archive.len() as u64 - at) as u32));
        }
        fs::write(data.join("data.000"), archive).unwrap();
        for bucket in 0..16u8 {
            let entries = if bucket == 0 {
                rows.clone()
            } else {
                vec![index_entry([0xe0 + bucket; 16], 0, 39)]
            };
            fs::write(
                data.join(format!("{bucket:02x}00000001.idx")),
                index_file(bucket, &entries),
            )
            .unwrap();
        }
        Self {
            output: root.join("output"),
            root,
            client,
            scratch,
        }
    }
    pub fn extra_m2(id: u32, unique: u32) -> Vec<CollisionFile<'static>> {
        vec![
            (100, None, wdt(None, Some(101))),
            (101, None, m2_adt(id, unique)),
            (id, None, m2(true)),
        ]
    }
    pub fn inherited() -> Vec<CollisionFile<'static>> {
        vec![
            (100, None, wdt(None, Some(101))),
            (101, None, m2_adt(90, 17)),
            (200, None, wdt(None, None)),
            (90, None, m2(true)),
        ]
    }
    pub fn empty_barrier() -> Vec<CollisionFile<'static>> {
        let mut files = Self::inherited();
        files[2] = (200, None, wdt(None, Some(201)));
        files.push((201, None, chunk(b"MVER", &18u32.to_le_bytes())));
        files.push((300, None, wdt(None, None)));
        files
    }
    pub fn no_collision_then_m2() -> Vec<CollisionFile<'static>> {
        let mut adt = wmo_adt(91, 8);
        adt.extend(m2_adt(90, 17));
        vec![
            (100, None, wdt(None, Some(101))),
            (101, None, adt),
            (90, None, m2(true)),
            (91, None, wmo_root(92, None)),
            (92, None, wmo_group(false, false, false)),
        ]
    }
    pub fn missing_wdt() -> Self {
        Self::new(&[(0, 100, -1)], &[])
    }
    pub fn plan(&self) -> crate::plan::Plan {
        let mut p = crate::plan::Plan::parse(crate::plan::v14_tests::V14).unwrap();
        p.locale = forever_casc::Locale::EnUs;
        let crate::plan::CollisionStorage::LeasedDiskV1(mut b) = p.collision_storage else {
            unreachable!()
        };
        b.output_bytes = 16 * 1024 * 1024;
        p.collision_storage = crate::plan::CollisionStorage::LeasedDiskV1(b);
        p
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
    }
}
