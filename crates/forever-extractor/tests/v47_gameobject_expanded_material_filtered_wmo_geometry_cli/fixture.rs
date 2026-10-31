// SPDX-License-Identifier: GPL-3.0-or-later
//! Minimal synthetic build-70450 CASC client with a valid GODI-only collision model.
use forever_casc::package::PartialEncryptedSection;
use forever_db2::{
    availability::{self, Source, Unavailable},
    catalog::{self, FieldType},
};
use std::{
    fs,
    os::unix::fs::{DirBuilderExt, PermissionsExt},
    path::Path,
};

#[path = "../../../forever-casc/src/local_index/jenkins.rs"]
mod jenkins;
#[path = "../../../forever-casc/src/blte/md5.rs"]
mod md5;

pub const MODEL: u32 = 300;
pub const WMO: u32 = 301;
pub const GODI_ONLY_MODEL: u32 = 302;
pub const TEXTURE: u32 = 303;
pub const NO_COLLISION_MODEL: u32 = 304;
pub const DOODAD_ONLY_WMO: u32 = 305;
pub const DOODAD_ONLY_CHILD: u32 = 306;
pub const ZERO_DISPLAY: u32 = 1;
pub const DISPLAY: u32 = 7;
pub const DUPLICATE_DISPLAY: u32 = 15;
pub const WMO_DISPLAY: u32 = 8;
pub const GODI_ONLY_DISPLAY: u32 = 9;
pub const TEXTURE_DISPLAY: u32 = 10;
pub const NO_COLLISION_DISPLAY: u32 = 11;
pub const DOODAD_ONLY_DISPLAY: u32 = 12;
pub const COPY_SOURCE_DISPLAY: u32 = 13;
pub const COPY_DESTINATION_DISPLAY: u32 = 14;
pub const MISSING_ROOT_DISPLAY: u32 = 16;
pub const ROOT_PRESENT_DISPLAY: u32 = 10_000;
const MAP_WDT: u32 = 100;
const MAP_ADT: u32 = 101;
fn put16(bytes: &mut [u8], at: usize, value: u16) {
    bytes[at..at + 2].copy_from_slice(&value.to_le_bytes());
}

fn put32(bytes: &mut [u8], at: usize, value: u32) {
    bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

fn field_width(kind: FieldType) -> usize {
    match kind {
        FieldType::String | FieldType::NonLocalizedString | FieldType::Int | FieldType::Float => 32,
        FieldType::Byte => 8,
        FieldType::Short => 16,
        FieldType::Long => 64,
    }
}

fn table(name: &str) -> &'static catalog::TableMetadata {
    catalog::resolve_for(catalog::WOWDBDEFS_70450_CATALOG_ID, name)
        .unwrap()
        .unwrap()
        .metadata()
}

fn complete_db2(name: &str, rows: &[(u32, u32)], copies: &[(u32, u32)]) -> Vec<u8> {
    let metadata = table(name);
    let fields = &metadata.fields[..metadata.file_field_count];
    let mut offsets = Vec::with_capacity(fields.len());
    let mut bit_offset = 0usize;
    for field in fields {
        offsets.push(bit_offset);
        bit_offset += field_width(field.kind) * usize::from(field.array_size);
    }
    let record_size = bit_offset.div_ceil(8).max(8);
    let mut records = Vec::new();
    for (row_index, (row_id, value)) in rows.iter().enumerate() {
        let mut record = vec![0; record_size];
        for (field_index, field) in fields.iter().enumerate() {
            if matches!(
                field.kind,
                FieldType::String | FieldType::NonLocalizedString
            ) {
                for array_index in 0..usize::from(field.array_size) {
                    let at = offsets[field_index] / 8 + array_index * 4;
                    let relative = (rows.len() - row_index) * record_size - at;
                    put32(&mut record, at, relative as u32);
                }
            }
        }
        if name == "GameObjectDisplayInfo" {
            let at = offsets[0] / 8 + 6 * 4;
            put32(&mut record, at, *value);
        }
        if name == "LiquidType" {
            let at = offsets[14] / 8;
            put32(&mut record, at, *value);
        }
        if name == "Map" {
            let wdt_field = 21;
            let at = offsets[wdt_field] / 8;
            put32(&mut record, at, *value);
            put16(&mut record, offsets[13] / 8, u16::MAX);
            put16(&mut record, offsets[14] / 8, u16::MAX);
        }
        if name == "LiquidObject" {
            put16(&mut record, offsets[2] / 8, *value as u16);
        }
        if metadata.index >= 0 {
            put32(
                &mut record,
                offsets[usize::try_from(metadata.index).unwrap()] / 8,
                *row_id,
            );
        }
        records.extend_from_slice(&record);
    }

    const HEADER: usize = 204;
    const SECTION: usize = 40;
    let column_meta_at = HEADER + SECTION + fields.len() * 4;
    let record_at = column_meta_at + fields.len() * 24;
    let strings = b"synthetic\0";
    let id_bytes = if metadata.index < 0 {
        rows.len() * 4
    } else {
        0
    };
    let copy_count = copies.len();
    let mut bytes = vec![0; record_at];
    bytes[..4].copy_from_slice(b"WDC5");
    for (at, value) in [
        (4, 5),
        (136, rows.len() as u32),
        (140, fields.len() as u32),
        (144, record_size as u32),
        (148, strings.len() as u32),
        (152, metadata.table_hash.unwrap_or(0)),
        (156, metadata.layout_hash),
        (160, 0),
        (164, u32::MAX),
        (176, metadata.field_count as u32),
        (188, (fields.len() * 24) as u32),
        (200, 1),
    ] {
        put32(&mut bytes, at, value);
    }
    bytes[174..176].copy_from_slice(&metadata.index.to_le_bytes());
    put32(&mut bytes, HEADER + 8, record_at as u32);
    put32(&mut bytes, HEADER + 12, rows.len() as u32);
    put32(&mut bytes, HEADER + 16, strings.len() as u32);
    put32(&mut bytes, HEADER + 24, id_bytes as u32);
    put32(&mut bytes, HEADER + 36, copy_count as u32);
    for (field_index, field) in fields.iter().enumerate() {
        let at = column_meta_at + field_index * 24;
        put16(&mut bytes, at, offsets[field_index] as u16);
        put32(&mut bytes, at + 16, field_width(field.kind) as u32);
        put32(&mut bytes, at + 20, u32::from(field.array_size));
    }
    bytes.extend_from_slice(&records);
    bytes.extend_from_slice(strings);
    if metadata.index < 0 {
        for (row_id, _) in rows {
            bytes.extend_from_slice(&row_id.to_le_bytes());
        }
    }
    for (destination, source) in copies {
        bytes.extend_from_slice(&destination.to_le_bytes());
        bytes.extend_from_slice(&source.to_le_bytes());
    }
    bytes
}

struct FixtureBytes<'a>(&'a [u8]);

impl Source for FixtureBytes<'_> {
    fn declared_bytes(&self) -> u64 {
        self.0.len() as u64
    }

    fn range(&self, offset: u64, size: usize) -> Result<&[u8], Unavailable> {
        let start = usize::try_from(offset).map_err(|_| Unavailable)?;
        self.0
            .get(start..start.checked_add(size).ok_or(Unavailable)?)
            .ok_or(Unavailable)
    }
}

struct PartialDb2 {
    bytes: Vec<u8>,
    hidden_start: usize,
    hidden_size: usize,
}

fn partial_db2(name: &str, complete: &[u8]) -> PartialDb2 {
    const HEADER: usize = 204;
    const SECTION: usize = 40;
    let metadata = table(name);
    let rows = u32::from_le_bytes(complete[136..140].try_into().unwrap()) as usize;
    let record_size = u32::from_le_bytes(complete[144..148].try_into().unwrap()) as usize;
    let fields = &metadata.fields[..metadata.file_field_count];
    let old_prefix = HEADER + SECTION + fields.len() * 4 + fields.len() * 24;
    let prefix = HEADER + 2 * SECTION + fields.len() * 4 + fields.len() * 24;
    let strings_size = u32::from_le_bytes(complete[148..152].try_into().unwrap()) as usize;
    let copy_count =
        u32::from_le_bytes(complete[HEADER + 36..HEADER + 40].try_into().unwrap()) as usize;
    let old_payload_end =
        old_prefix + rows * record_size + strings_size + rows * 4 + copy_count * 8;
    let tact_ids = 8;
    let payload_start = prefix + tact_ids;
    let first_payload_size = rows * record_size + strings_size + rows * 4 + copy_count * 8;
    let hidden_start = payload_start + first_payload_size;
    let hidden_size = record_size + 1 + 4;
    let mut bytes = vec![0; hidden_start + hidden_size];
    bytes[..HEADER].copy_from_slice(&complete[..HEADER]);
    bytes[HEADER + 2 * SECTION..prefix].copy_from_slice(&complete[HEADER + SECTION..old_prefix]);
    let old_record_start = old_prefix;
    let old_string_start = old_record_start + rows * record_size;
    let old_id_start = old_string_start + strings_size;
    let old_copy_start = old_id_start + rows * 4;
    bytes[payload_start..payload_start + rows * record_size]
        .copy_from_slice(&complete[old_record_start..old_string_start]);
    bytes[payload_start + rows * record_size..payload_start + rows * record_size + strings_size]
        .copy_from_slice(&complete[old_string_start..old_id_start]);
    bytes[payload_start + rows * record_size + strings_size
        ..payload_start + rows * record_size + strings_size + rows * 4]
        .copy_from_slice(&complete[old_id_start..old_copy_start]);
    bytes[payload_start + rows * record_size + strings_size + rows * 4
        ..payload_start + first_payload_size]
        .copy_from_slice(&complete[old_copy_start..old_payload_end]);
    bytes[136..140].copy_from_slice(&((rows + 1) as u32).to_le_bytes());
    bytes[148..152].copy_from_slice(&((strings_size + 1) as u32).to_le_bytes());
    bytes[200..204].copy_from_slice(&2_u32.to_le_bytes());
    bytes[HEADER + 24..HEADER + 28].copy_from_slice(&8_u32.to_le_bytes());
    let first_section = HEADER;
    let second_section = HEADER + SECTION;
    bytes[second_section..second_section + 8]
        .copy_from_slice(&0x1122_3344_5566_7788_u64.to_le_bytes());
    bytes[prefix..prefix + 4].copy_from_slice(&1_u32.to_le_bytes());
    bytes[prefix + 4..payload_start].copy_from_slice(&[2, 0, 0, 0]);
    bytes[first_section + 8..first_section + 12]
        .copy_from_slice(&(payload_start as u32).to_le_bytes());
    bytes[first_section + 12..first_section + 16].copy_from_slice(&(rows as u32).to_le_bytes());
    bytes[first_section + 16..first_section + 20]
        .copy_from_slice(&(strings_size as u32).to_le_bytes());
    bytes[first_section + 24..first_section + 28]
        .copy_from_slice(&((rows * 4) as u32).to_le_bytes());
    bytes[first_section + 36..first_section + 40]
        .copy_from_slice(&(copy_count as u32).to_le_bytes());
    bytes[second_section + 8..second_section + 12]
        .copy_from_slice(&(hidden_start as u32).to_le_bytes());
    bytes[second_section + 12..second_section + 16].copy_from_slice(&0_u32.to_le_bytes());
    bytes[second_section + 16..second_section + 20].copy_from_slice(&0_u32.to_le_bytes());
    bytes[second_section + 24..second_section + 28].copy_from_slice(&0_u32.to_le_bytes());
    bytes[second_section + 12..second_section + 16].copy_from_slice(&1_u32.to_le_bytes());
    bytes[second_section + 16..second_section + 20].copy_from_slice(&1_u32.to_le_bytes());
    bytes[second_section + 24..second_section + 28].copy_from_slice(&4_u32.to_le_bytes());
    assert_eq!(
        u32::from_le_bytes(bytes[prefix..prefix + 4].try_into().unwrap()),
        1
    );
    assert_eq!(
        u32::from_le_bytes(bytes[prefix + 4..payload_start].try_into().unwrap()),
        2
    );
    assert!(bytes[hidden_start..].iter().all(|byte| *byte == 0));
    let facts = availability::read_structure_70450_for_table(&FixtureBytes(&bytes), name).unwrap();
    assert_eq!(facts.tact_payload_offsets, [hidden_start as u64]);
    assert_eq!(facts.tact_payload_sizes, [hidden_size as u64]);
    assert_eq!(facts.sections.len(), 2);
    assert_eq!(
        facts.sections[0].state,
        availability::SectionState::Admitted
    );
    assert_eq!(
        facts.sections[1].state,
        availability::SectionState::UnknownTact
    );
    PartialDb2 {
        bytes,
        hidden_start,
        hidden_size,
    }
}

fn partial_blte(bytes: &[u8], hidden_start: usize, hidden_size: usize) -> ([u8; 16], Vec<u8>) {
    let mut normal = vec![b'N'];
    normal.extend_from_slice(&bytes[..hidden_start]);
    let mut encrypted = vec![b'E', 8];
    encrypted.extend_from_slice(&0x1122_3344_5566_7788_u64.to_le_bytes());
    encrypted.push(4);
    encrypted.extend_from_slice(&[0x55; 4]);
    encrypted.push(b'S');
    encrypted.extend_from_slice(&[0xa5; 16]);
    let header_size = 12 + 2 * 24;
    let mut encoded = b"BLTE".to_vec();
    encoded.extend_from_slice(&(header_size as u32).to_be_bytes());
    encoded.extend_from_slice(&[15, 0, 0, 2]);
    for (frame, decoded_size) in [
        (&normal[..], hidden_start as u32),
        (&encrypted[..], hidden_size as u32),
    ] {
        encoded.extend_from_slice(&(frame.len() as u32).to_be_bytes());
        encoded.extend_from_slice(&decoded_size.to_be_bytes());
        encoded.extend_from_slice(&md5::digest(frame));
    }
    let ekey = md5::digest(&encoded);
    encoded.extend_from_slice(&normal);
    encoded.extend_from_slice(&encrypted);
    (ekey, encoded)
}

pub fn partial_godi_declaration() -> PartialEncryptedSection {
    PartialEncryptedSection {
        lookup: format!("{:016x}", 0x1122_3344_5566_7788_u64),
        frame_ordinal: 1,
        decoded_size: 61,
    }
}

fn chunk(tag: &[u8; 4], payload: &[u8]) -> Vec<u8> {
    let mut bytes: Vec<_> = tag.iter().rev().copied().collect();
    bytes.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    bytes.extend_from_slice(payload);
    bytes
}

fn wdt() -> Vec<u8> {
    let mut mphd = vec![0; 32];
    put32(&mut mphd, 0, 0x200);
    let mut main = vec![0; 64 * 64 * 8];
    let mut maid = vec![0; 64 * 64 * 32];
    put32(&mut main, 0, 1);
    put32(&mut maid, 4, MAP_ADT);
    let mut bytes = chunk(b"MPHD", &mphd);
    bytes.extend_from_slice(&chunk(b"MAIN", &main));
    bytes.extend_from_slice(&chunk(b"MAID", &maid));
    bytes
}

fn adt(include_wmo: bool) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut placement = [0; 36];
    put32(&mut placement, 0, MODEL);
    placement[32..34].copy_from_slice(&1024_u16.to_le_bytes());
    placement[34..36].copy_from_slice(&0x40_u16.to_le_bytes());
    bytes.extend_from_slice(&chunk(b"MDDF", &placement));

    if include_wmo {
        let mut placement = [0; 64];
        put32(&mut placement, 0, WMO);
        for (at, value) in [
            (32, 0.0_f32),
            (36, 0.0),
            (40, 0.0),
            (44, 1.0),
            (48, 1.0),
            (52, 1.0),
        ] {
            put32(&mut placement, at, value.to_bits());
        }
        placement[56..58].copy_from_slice(&8_u16.to_le_bytes());
        bytes.extend_from_slice(&chunk(b"MODF", &placement));
    }
    bytes
}

fn no_collision_m2() -> Vec<u8> {
    let mut bytes = vec![0; 304];
    bytes[..4].copy_from_slice(b"MD20");
    bytes
}

fn collision_m2() -> Vec<u8> {
    let mut bytes = vec![0; 304];
    bytes[..4].copy_from_slice(b"MD20");
    for (at, value) in [(216, 3), (220, 304), (224, 3), (228, 310)] {
        put32(&mut bytes, at, value);
    }
    bytes.extend_from_slice(&[0, 0, 1, 0, 2, 0]);
    for value in [0.0_f32, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0] {
        bytes.extend_from_slice(&value.to_bits().to_le_bytes());
    }
    bytes
}

fn collision_wmo() -> (Vec<u8>, Vec<u8>) {
    let mut root_header = [0; 64];
    put32(&mut root_header, 4, 1);
    put32(&mut root_header, 20, 1);
    put32(&mut root_header, 24, 1);
    put32(&mut root_header, 32, 0x1234);
    root_header[60] = 4;
    let mut root = chunk(b"MVER", &17_u32.to_le_bytes());
    root.extend_from_slice(&chunk(b"MOHD", &root_header));
    root.extend_from_slice(&chunk(b"GFID", &400_u32.to_le_bytes()));
    root.extend_from_slice(&chunk(b"MODI", &MODEL.to_le_bytes()));
    let mut spawn = [0; 40];
    put32(&mut spawn, 32, 1.0_f32.to_bits());
    root.extend_from_slice(&chunk(b"MODD", &spawn));
    let mut set = [0; 32];
    put32(&mut set, 20, 0);
    put32(&mut set, 24, 1);
    root.extend_from_slice(&chunk(b"MODS", &set));

    let mut group_header = vec![0; 68];
    put32(&mut group_header, 56, 99);
    group_header.extend_from_slice(&chunk(b"MPY2", &[8, 0, 8, 0]));
    group_header.extend_from_slice(&chunk(b"MOVI", &[0, 0, 1, 0, 2, 0]));
    let mut vertices = Vec::new();
    for value in [0.0_f32, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0] {
        vertices.extend_from_slice(&value.to_bits().to_le_bytes());
    }
    group_header.extend_from_slice(&chunk(b"MOVT", &vertices));
    let mut batch = [0; 24];
    batch[16..18].copy_from_slice(&513_u16.to_le_bytes());
    group_header.extend_from_slice(&chunk(b"MOBA", &batch));
    (root, chunk(b"MOGP", &group_header))
}

fn doodad_only_wmo() -> (Vec<u8>, Vec<u8>) {
    let mut root_header = [0; 64];
    put32(&mut root_header, 4, 1);
    put32(&mut root_header, 20, 1);
    put32(&mut root_header, 24, 1);
    put32(&mut root_header, 32, 0x5678);
    root_header[60] = 4;
    let mut root = chunk(b"MVER", &17_u32.to_le_bytes());
    root.extend_from_slice(&chunk(b"MOHD", &root_header));
    root.extend_from_slice(&chunk(b"GFID", &402_u32.to_le_bytes()));
    root.extend_from_slice(&chunk(b"MODI", &DOODAD_ONLY_CHILD.to_le_bytes()));
    let mut spawn = [0; 40];
    put32(&mut spawn, 32, 1.0_f32.to_bits());
    root.extend_from_slice(&chunk(b"MODD", &spawn));
    let mut set = [0; 32];
    put32(&mut set, 24, 1);
    root.extend_from_slice(&chunk(b"MODS", &set));

    let mut group_header = vec![0; 68];
    put32(&mut group_header, 56, 99);
    group_header.extend_from_slice(&chunk(b"MODR", &[0, 0]));
    (root, chunk(b"MOGP", &group_header))
}

fn normal_record(bytes: &[u8]) -> Vec<u8> {
    let mut record = b"BLTE\0\0\0\0N".to_vec();
    record.extend_from_slice(bytes);
    record
}

fn selected_ekey(record: &[u8]) -> [u8; 16] {
    let header_size = u32::from_be_bytes(record[4..8].try_into().unwrap()) as usize;
    let authenticated = if header_size == 0 {
        record
    } else {
        &record[..header_size]
    };
    md5::digest(authenticated)
}

fn encrypted_record(lookup: u64) -> Vec<u8> {
    let mut frame = vec![b'E', 8];
    frame.extend_from_slice(&lookup.to_le_bytes());
    frame.extend_from_slice(&[4, 0x55, 0x55, 0x55, 0x55, b'S']);
    frame.extend_from_slice(&[0xa5; 16]);
    let mut encoded = b"BLTE\0\0\0\0".to_vec();
    encoded.extend_from_slice(&frame);
    encoded
}

fn corrupt_record(lookup: u64) -> Vec<u8> {
    let mut record = encrypted_record(lookup);
    record[23] = b'A';
    record
}

fn multiframe_corrupt_record(lookup: u64) -> Vec<u8> {
    let normal = b"Nabc";
    let encrypted = encrypted_record(lookup)[8..].to_vec();
    let header_size = 12 + 2 * 24;
    let mut encoded = b"BLTE".to_vec();
    encoded.extend_from_slice(&(header_size as u32).to_be_bytes());
    encoded.extend_from_slice(&[15, 0, 0, 2]);
    for (index, (frame, size)) in [(&encrypted[..], 1_u32), (normal.as_slice(), 3)]
        .iter()
        .enumerate()
    {
        encoded.extend_from_slice(&(frame.len() as u32).to_be_bytes());
        encoded.extend_from_slice(&size.to_be_bytes());
        let mut digest = md5::digest(frame);
        if index == 1 {
            digest[0] ^= 1;
        }
        encoded.extend_from_slice(&digest);
    }
    encoded.extend_from_slice(&encrypted);
    encoded.extend_from_slice(normal);
    encoded
}

fn root_file(rows: &[(u32, [u8; 16])]) -> Vec<u8> {
    let mut bytes = b"TSFM".to_vec();
    for value in [20_u32, 2, rows.len() as u32, 0] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    for (file_data_id, ckey) in rows {
        for value in [1_u32, 0, 0x1000_0000, 0] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes.push(0);
        bytes.extend_from_slice(&file_data_id.to_le_bytes());
        bytes.extend_from_slice(ckey);
    }
    bytes
}

fn encoding_file(rows: &[([u8; 16], u32, [u8; 16])]) -> Vec<u8> {
    let mut pages = Vec::new();
    for entries in rows.chunks(26) {
        let mut page = Vec::new();
        for (ckey, size, ekey) in entries {
            page.extend_from_slice(&1_u16.to_le_bytes());
            page.extend_from_slice(&size.to_be_bytes());
            page.extend_from_slice(ckey);
            page.extend_from_slice(ekey);
        }
        assert!(page.len() <= 1024);
        page.resize(1024, 0);
        pages.push(page);
    }
    let mut bytes = b"EN\x01\x10\x10".to_vec();
    bytes.extend_from_slice(&1_u16.to_be_bytes());
    bytes.extend_from_slice(&1_u16.to_be_bytes());
    bytes.extend_from_slice(&(pages.len() as u32).to_be_bytes());
    bytes.extend_from_slice(&0_u32.to_be_bytes());
    bytes.push(0);
    bytes.extend_from_slice(&0_u32.to_be_bytes());
    for (page, entries) in pages.iter().zip(rows.chunks(26)) {
        bytes.extend_from_slice(&entries[0].0);
        bytes.extend_from_slice(&md5::digest(page));
    }
    for page in pages {
        bytes.extend_from_slice(&page);
    }
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
    let hash = jenkins::hashlittle(&bytes[8..24], 0);
    bytes[4..8].copy_from_slice(&hash.to_le_bytes());
    bytes.extend_from_slice(&u32::try_from(rows.len() * 18).unwrap().to_le_bytes());
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

fn private_dirs(path: &Path) {
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(path)
        .unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
}

#[derive(Clone, Copy)]
enum GodiSource {
    Complete,
    Partial,
    PartialEncryptedUnavailable,
    UnavailableEncoding,
    EncryptedUnavailable,
    CorruptCandidate,
    MultiframeCorruptCandidate,
}

pub fn write_client(client: &Path, include_godi_only_model: bool) {
    write_client_with_options(
        client,
        include_godi_only_model,
        true,
        &[],
        &[],
        false,
        GodiSource::Complete,
    );
}

pub fn write_v46_client(client: &Path) {
    write_client_with_options(client, true, false, &[], &[], false, GodiSource::Complete);
}

pub fn write_v48_client(client: &Path, missing_fdids: &[u32]) {
    write_v48_client_with_root_rows(client, missing_fdids, &[], false);
}

pub fn write_v49_client(client: &Path, missing_fdids: &[u32], encrypted_fdids: &[u32]) {
    write_v49_client_with_partial_godi(client, missing_fdids, encrypted_fdids, false);
}

pub fn write_v50_client(
    client: &Path,
    liquid_rows: &[(u32, i16)],
    liquid_copies: &[(u32, u32)],
    partial_liquid_object: bool,
    trailing_liquid_object: bool,
) {
    write_client_with_options_and_liquid_object(
        client,
        ClientOptions {
            include_godi_only_model: true,
            include_wmo_placement: true,
            missing_root_fdids: &[],
            root_present_missing_fdids: &[],
            minimal_godi: false,
            godi_source: GodiSource::Complete,
            liquid_object: Some(LiquidObjectFixture {
                rows: liquid_rows,
                copies: liquid_copies,
                partial: partial_liquid_object,
                trailing: trailing_liquid_object,
            }),
        },
    );
}

pub fn write_v50_client_without_liquid_object(client: &Path) {
    write_client_with_options_and_liquid_object(
        client,
        ClientOptions {
            include_godi_only_model: true,
            include_wmo_placement: true,
            missing_root_fdids: &[],
            root_present_missing_fdids: &[],
            minimal_godi: false,
            godi_source: GodiSource::Complete,
            liquid_object: None,
        },
    );
}

pub fn write_v49_client_with_repeated_missing_candidates(client: &Path, count: usize) {
    assert!(count <= 4097);
    let repeated = vec![60_000; count];
    write_client_with_options(
        client,
        true,
        true,
        &repeated,
        &[],
        false,
        GodiSource::Complete,
    );
}

pub fn write_v49_client_with_repeated_encrypted_candidates(client: &Path, count: usize) {
    assert!(count <= 4097);
    let repeated = vec![60_000; count];
    write_client_with_options(
        client,
        false,
        false,
        &[],
        &repeated,
        true,
        GodiSource::EncryptedUnavailable,
    );
}

pub fn write_v49_client_without_initial_godi_encoding(client: &Path) {
    write_client_with_options(
        client,
        true,
        true,
        &[],
        &[],
        false,
        GodiSource::UnavailableEncoding,
    );
}

pub fn write_v49_client_with_corrupt_candidate(client: &Path, fdid: u32, multiframe: bool) {
    write_client_with_options(
        client,
        true,
        true,
        &[],
        &[fdid],
        false,
        if multiframe {
            GodiSource::MultiframeCorruptCandidate
        } else {
            GodiSource::CorruptCandidate
        },
    );
}

pub fn write_v49_client_with_partial_godi(
    client: &Path,
    missing_fdids: &[u32],
    encrypted_fdids: &[u32],
    partial_godi: bool,
) {
    write_client_with_options(
        client,
        true,
        true,
        missing_fdids,
        encrypted_fdids,
        false,
        match (partial_godi, encrypted_fdids.is_empty()) {
            (true, true) => GodiSource::Partial,
            (true, false) => GodiSource::PartialEncryptedUnavailable,
            (false, true) => GodiSource::Complete,
            (false, false) => GodiSource::EncryptedUnavailable,
        },
    );
}

pub fn write_v48_client_without_initial_godi_encoding(client: &Path) {
    write_client_with_options(
        client,
        true,
        true,
        &[],
        &[],
        false,
        GodiSource::UnavailableEncoding,
    );
}

pub fn write_v48_client_with_root_rows(
    client: &Path,
    missing_fdids: &[u32],
    root_present_missing_fdids: &[u32],
    minimal_godi: bool,
) {
    write_v48_client_with_partial_godi(
        client,
        missing_fdids,
        root_present_missing_fdids,
        minimal_godi,
        false,
    );
}

pub fn write_v48_client_with_partial_godi(
    client: &Path,
    missing_fdids: &[u32],
    root_present_missing_fdids: &[u32],
    minimal_godi: bool,
    partial_godi: bool,
) {
    assert!(missing_fdids.len() + root_present_missing_fdids.len() <= 4096);
    write_client_with_options(
        client,
        !minimal_godi,
        !minimal_godi,
        missing_fdids,
        root_present_missing_fdids,
        minimal_godi,
        if partial_godi {
            GodiSource::Partial
        } else {
            GodiSource::Complete
        },
    );
}

fn write_client_with_options(
    client: &Path,
    include_godi_only_model: bool,
    include_wmo_placement: bool,
    missing_root_fdids: &[u32],
    root_present_missing_fdids: &[u32],
    minimal_godi: bool,
    godi_source: GodiSource,
) {
    write_client_with_options_and_liquid_object(
        client,
        ClientOptions {
            include_godi_only_model,
            include_wmo_placement,
            missing_root_fdids,
            root_present_missing_fdids,
            minimal_godi,
            godi_source,
            liquid_object: None,
        },
    );
}

struct LiquidObjectFixture<'a> {
    rows: &'a [(u32, i16)],
    copies: &'a [(u32, u32)],
    partial: bool,
    trailing: bool,
}

struct ClientOptions<'a> {
    include_godi_only_model: bool,
    include_wmo_placement: bool,
    missing_root_fdids: &'a [u32],
    root_present_missing_fdids: &'a [u32],
    minimal_godi: bool,
    godi_source: GodiSource,
    liquid_object: Option<LiquidObjectFixture<'a>>,
}

pub fn write_v51_client(client: &Path, partial_table: Option<&str>) {
    write_v52_source_client(client, partial_table, false);
}

pub fn write_v52_source_client(client: &Path, partial_table: Option<&str>, multiple_maps: bool) {
    write_client_with_terrain(
        client,
        ClientOptions {
            include_godi_only_model: true,
            include_wmo_placement: true,
            missing_root_fdids: &[],
            root_present_missing_fdids: &[],
            minimal_godi: false,
            godi_source: GodiSource::Complete,
            liquid_object: Some(LiquidObjectFixture {
                rows: &[(7, 22)],
                copies: &[],
                partial: false,
                trailing: false,
            }),
        },
        true,
        partial_table,
        multiple_maps,
    );
}

fn write_client_with_options_and_liquid_object(client: &Path, options: ClientOptions<'_>) {
    write_client_with_terrain(client, options, false, None, false);
}

fn write_client_with_terrain(
    client: &Path,
    options: ClientOptions<'_>,
    terrain: bool,
    partial_table: Option<&str>,
    multiple_maps: bool,
) {
    let ClientOptions {
        include_godi_only_model,
        include_wmo_placement,
        missing_root_fdids,
        root_present_missing_fdids,
        minimal_godi,
        godi_source,
        liquid_object,
    } = options;
    let encrypted_candidate_source = matches!(
        godi_source,
        GodiSource::EncryptedUnavailable
            | GodiSource::CorruptCandidate
            | GodiSource::MultiframeCorruptCandidate
    );
    let repeated_encrypted_source = matches!(
        godi_source,
        GodiSource::EncryptedUnavailable
            | GodiSource::CorruptCandidate
            | GodiSource::MultiframeCorruptCandidate
    );
    let candidate_limit = if encrypted_candidate_source {
        4097
    } else {
        4096
    };
    assert!(missing_root_fdids.len() + root_present_missing_fdids.len() <= candidate_limit);
    assert!(missing_root_fdids.windows(2).all(|pair| pair[0] <= pair[1]));
    assert!(
        root_present_missing_fdids
            .windows(2)
            .all(|pair| { pair[0] < pair[1] || (repeated_encrypted_source && pair[0] == pair[1]) })
    );
    let partial_godi = matches!(
        godi_source,
        GodiSource::Partial | GodiSource::PartialEncryptedUnavailable
    );
    let mut godi_rows = if partial_godi {
        vec![
            (ZERO_DISPLAY, 0),
            (DISPLAY, MODEL),
            (DUPLICATE_DISPLAY, MODEL),
            (WMO_DISPLAY, WMO),
            (GODI_ONLY_DISPLAY, GODI_ONLY_MODEL),
            (TEXTURE_DISPLAY, TEXTURE),
            (NO_COLLISION_DISPLAY, NO_COLLISION_MODEL),
            (DOODAD_ONLY_DISPLAY, DOODAD_ONLY_WMO),
            (COPY_SOURCE_DISPLAY, 0),
        ]
    } else if minimal_godi {
        vec![(ZERO_DISPLAY, 0)]
    } else {
        vec![
            (ZERO_DISPLAY, 0),
            (DISPLAY, MODEL),
            (DUPLICATE_DISPLAY, MODEL),
            (WMO_DISPLAY, WMO),
            (GODI_ONLY_DISPLAY, GODI_ONLY_MODEL),
            (TEXTURE_DISPLAY, TEXTURE),
            (NO_COLLISION_DISPLAY, NO_COLLISION_MODEL),
            (DOODAD_ONLY_DISPLAY, DOODAD_ONLY_WMO),
            (COPY_SOURCE_DISPLAY, 0),
        ]
    };
    for (index, &id) in missing_root_fdids.iter().enumerate() {
        godi_rows.push((MISSING_ROOT_DISPLAY + index as u32, id));
    }
    for (index, &id) in root_present_missing_fdids.iter().enumerate() {
        godi_rows.push((ROOT_PRESENT_DISPLAY + index as u32, id));
    }
    let copies = if minimal_godi && !partial_godi {
        Vec::new()
    } else {
        vec![(COPY_DESTINATION_DISPLAY, COPY_SOURCE_DISPLAY)]
    };
    let godi = complete_db2("GameObjectDisplayInfo", &godi_rows, &copies);
    let (godi, mut partial_godi_encoding) = if partial_godi {
        let partial = partial_db2("GameObjectDisplayInfo", &godi);
        let encoding = partial_blte(&partial.bytes, partial.hidden_start, partial.hidden_size);
        let ckey = md5::digest(&partial.bytes);
        (partial.bytes, Some((ckey, encoding.0, encoding.1)))
    } else {
        (godi, None)
    };
    let map_rows = if multiple_maps {
        vec![(1, MAP_WDT), (2, MAP_WDT), (3, 110), (4, 111)]
    } else {
        vec![(1, MAP_WDT)]
    };
    let map = complete_db2("Map", &map_rows, &[]);
    let liquid_object = liquid_object.map(|fixture| {
        let unsigned_rows: Vec<_> = fixture
            .rows
            .iter()
            .map(|(id, value)| (*id, *value as u16 as u32))
            .collect();
        let mut bytes = complete_db2("LiquidObject", &unsigned_rows, fixture.copies);
        if fixture.partial {
            bytes[200..204].copy_from_slice(&2_u32.to_le_bytes());
        }
        if fixture.trailing {
            bytes.push(0);
        }
        bytes
    });
    let liquids = complete_db2("LiquidMaterial", &[(7, 0)], &[]);
    let liquid_types = complete_db2("LiquidType", &[(22, 7)], &[]);
    let (wmo_root, wmo_group) = collision_wmo();
    let (doodad_root, doodad_group) = doodad_only_wmo();
    let mut files = vec![
        (table("Map").file_data_id, map),
        (table("GameObjectDisplayInfo").file_data_id, godi),
        (table("LiquidMaterial").file_data_id, liquids),
        (table("LiquidType").file_data_id, liquid_types),
        (
            MAP_WDT,
            if terrain {
                let mut bytes = chunk(b"MVER", &18_u32.to_le_bytes());
                let mut data = wdt();
                let maid_start = 40 + 8 + 64 * 64 * 8 + 8;
                put32(&mut data, maid_start, MAP_ADT);
                bytes.extend_from_slice(&data);
                bytes
            } else {
                wdt()
            },
        ),
        (
            MAP_ADT,
            if terrain {
                let mut bytes = chunk(b"MVER", &18_u32.to_le_bytes());
                let mut cell = vec![0; 128];
                put32(&mut cell, 52, 42);
                put32(&mut cell, 112, 2.0_f32.to_bits());
                cell.extend_from_slice(&chunk(b"MCVT", &[0; 145 * 4]));
                bytes.extend_from_slice(&chunk(b"MCNK", &cell));
                bytes.extend_from_slice(&adt(include_wmo_placement));
                bytes
            } else {
                adt(include_wmo_placement)
            },
        ),
        (WMO, wmo_root),
        (400, wmo_group),
        (TEXTURE, b"BLP2synthetic".to_vec()),
        (DOODAD_ONLY_WMO, doodad_root),
        (402, doodad_group),
        (DOODAD_ONLY_CHILD, collision_m2()),
        (NO_COLLISION_MODEL, no_collision_m2()),
    ];
    if multiple_maps {
        let mut empty = chunk(b"MVER", &18_u32.to_le_bytes());
        let mut data = wdt();
        put32(&mut data, 48, 0);
        data[40 + 8 + 64 * 64 * 8 + 8..].fill(0);
        empty.extend_from_slice(&data);
        files.push((110, empty));
    }
    if include_godi_only_model {
        files.push((GODI_ONLY_MODEL, collision_m2()));
    }
    if let Some(bytes) = liquid_object {
        files.push((table("LiquidObject").file_data_id, bytes));
    }
    for id in missing_root_fdids {
        assert!(!files.iter().any(|(file_data_id, _)| file_data_id == id));
    }
    files.push((MODEL, collision_m2()));
    let mut root_rows = Vec::new();
    let mut source_routes = Vec::new();
    let mut records = Vec::new();
    for (file_data_id, bytes) in files {
        let partial = partial_table
            .filter(|name| table(name).file_data_id == file_data_id)
            .map(|name| partial_db2(name, &bytes));
        let bytes = partial
            .as_ref()
            .map_or(bytes, |source| source.bytes.clone());
        let ckey = md5::digest(&bytes);
        let (ckey, ekey, record) = if let Some(source) = partial {
            let (ekey, record) = partial_blte(&bytes, source.hidden_start, source.hidden_size);
            (ckey, ekey, record)
        } else if file_data_id == table("GameObjectDisplayInfo").file_data_id {
            if let Some((partial_ckey, ekey, encoded)) = partial_godi_encoding.take() {
                (partial_ckey, ekey, encoded)
            } else {
                let content = if matches!(godi_source, GodiSource::UnavailableEncoding) {
                    b"unrelated authenticated content".as_slice()
                } else {
                    bytes.as_slice()
                };
                let record = normal_record(content);
                let ekey = selected_ekey(&record);
                (ckey, ekey, record)
            }
        } else {
            let record = normal_record(&bytes);
            let ekey = selected_ekey(&record);
            (ckey, ekey, record)
        };
        source_routes.push((ckey, bytes.len() as u32, ekey));
        root_rows.push((file_data_id, ckey));
        records.push((Some(file_data_id), ekey, record));
    }
    let missing_body = collision_m2();
    if !matches!(
        godi_source,
        GodiSource::EncryptedUnavailable
            | GodiSource::CorruptCandidate
            | GodiSource::MultiframeCorruptCandidate
            | GodiSource::PartialEncryptedUnavailable
    ) {
        for &id in root_present_missing_fdids {
            assert!(
                !root_rows
                    .iter()
                    .any(|(file_data_id, _)| *file_data_id == id)
            );
            let ckey = md5::digest(&id.to_le_bytes());
            let encoded = normal_record(&missing_body);
            let ekey = selected_ekey(&encoded);
            source_routes.push((ckey, missing_body.len() as u32, ekey));
            root_rows.push((id, ckey));
        }
    }
    let mut encrypted_records = Vec::new();
    if matches!(
        godi_source,
        GodiSource::EncryptedUnavailable
            | GodiSource::CorruptCandidate
            | GodiSource::MultiframeCorruptCandidate
            | GodiSource::PartialEncryptedUnavailable
    ) {
        let mut seen_encrypted_fdids = std::collections::BTreeSet::new();
        for (index, &id) in root_present_missing_fdids.iter().enumerate() {
            if !seen_encrypted_fdids.insert(id) {
                continue;
            }
            let ckey = md5::digest(&id.to_le_bytes());
            let record = match godi_source {
                GodiSource::EncryptedUnavailable | GodiSource::PartialEncryptedUnavailable => {
                    encrypted_record(0x8877_6655_4433_2211 + index as u64)
                }
                GodiSource::CorruptCandidate => {
                    corrupt_record(0x8877_6655_4433_2211 + index as u64)
                }
                GodiSource::MultiframeCorruptCandidate => {
                    multiframe_corrupt_record(0x8877_6655_4433_2211 + index as u64)
                }
                _ => unreachable!(),
            };
            let ekey = selected_ekey(&record);
            source_routes.push((ckey, 4, ekey));
            root_rows.push((id, ckey));
            encrypted_records.push((Some(id), ekey, record));
        }
    }
    let root = root_file(&root_rows);
    let root_ckey = md5::digest(&root);
    let root_record = normal_record(&root);
    let root_ekey = selected_ekey(&root_record);
    let mut routes = vec![(root_ckey, root.len() as u32, root_ekey)];
    routes.extend(source_routes);
    let encoding = encoding_file(&routes);
    let encoding_ckey = md5::digest(&encoding);
    let encoding_record = normal_record(&encoding);
    let encoding_ekey = selected_ekey(&encoding_record);
    records.extend(encrypted_records);
    records.push((None, root_ekey, root_record));
    records.push((None, encoding_ekey, encoding_record));
    let config = format!(
        "root = {}\nencoding = {} {}\nencoding-size = {} {}\nbuild-uid = wow_classic_beta\nbuild-name = synthetic-1.60.1.70450\n",
        hex16(root_ckey),
        hex16(encoding_ckey),
        hex16(encoding_ekey),
        encoding.len(),
        encoding.len() + 39
    )
    .into_bytes();
    let config_ckey = md5::digest(&config);
    let config_name = hex16(config_ckey);
    let data = client.join("Data/data");
    let config_dir = client
        .join("Data/config")
        .join(&config_name[..2])
        .join(&config_name[2..4]);
    private_dirs(&data);
    private_dirs(&config_dir);
    fs::write(config_dir.join(&config_name), &config).unwrap();
    fs::write(
        client.join(".build.info"),
        format!(
            "Product!STRING:0|Active!DEC:1|Version!STRING:0|Build Key!HEX:16|CDN Key!HEX:16|Tags!STRING:0\nwow_classic_beta|1|1.60.1.70450|{}|{}|esES\n",
            hex16(config_ckey),
            hex16([0x21; 16])
        ),
    )
    .unwrap();
    let mut archive = Vec::new();
    let mut index_rows = Vec::new();
    for (_file_data_id, ekey, record) in records {
        let offset = archive.len() as u64;
        archive.extend_from_slice(&[0xa5; 30]);
        archive.extend_from_slice(&record);
        index_rows.push(index_entry(ekey, offset, (30 + record.len()) as u32));
    }
    index_rows.sort_unstable_by_key(|row| row[..9].to_vec());
    fs::write(data.join("data.000"), archive).unwrap();
    for bucket in 0..16_u8 {
        let rows = if bucket == 0 {
            index_rows.clone()
        } else {
            vec![index_entry([0xe0 + bucket; 16], 0, 39)]
        };
        fs::write(
            data.join(format!("{bucket:02x}00000001.idx")),
            index_file(bucket, &rows),
        )
        .unwrap();
    }
}

fn hex16(value: [u8; 16]) -> String {
    value.iter().map(|byte| format!("{byte:02x}")).collect()
}
