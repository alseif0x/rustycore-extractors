// SPDX-License-Identifier: GPL-3.0-or-later
// Synthetic WDC5 available Map projection with one TACT-declared skipped section.

use forever_casc::blte::{self, available as blte_available};
use forever_db2::{
    availability::{self, SectionState, Source, Unavailable},
    catalog::{self, FieldType},
};

#[path = "../../forever-casc/src/blte/md5.rs"]
pub(crate) mod md5;

const LIMITS: blte::Limits = blte::Limits {
    max_encoded_bytes: 16 * 1024,
    max_decoded_bytes: 16 * 1024,
    max_frames: 8,
};

struct Bytes<'a>(&'a [u8]);
impl Source for Bytes<'_> {
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

fn map_record(
    fields: &[forever_db2::catalog::FieldMetadata],
    offsets: &[usize],
    record_size: usize,
) -> Vec<u8> {
    let mut record = vec![0; record_size];
    for (index, field) in fields.iter().enumerate() {
        if matches!(
            field.kind,
            FieldType::String | FieldType::NonLocalizedString
        ) {
            for array_index in 0..usize::from(field.array_size) {
                let at = offsets[index] / 8 + array_index * 4;
                put32(&mut record, at, (2 * record_size - at) as u32);
            }
        }
    }
    for (index, value) in [(13, -1_i32), (14, -1_i32), (21, 100_i32)] {
        let at = offsets[index] / 8;
        record[at..at + 4].copy_from_slice(&value.to_le_bytes());
    }
    record
}

pub fn partial_map() -> (Vec<u8>, usize, usize) {
    let metadata = catalog::resolve_for(catalog::WOWDBDEFS_70450_CATALOG_ID, "Map")
        .unwrap()
        .unwrap()
        .metadata();
    let fields = &metadata.fields[..metadata.file_field_count];
    let mut offsets = Vec::new();
    let mut bits = 0usize;
    for field in fields {
        offsets.push(bits);
        bits += field_width(field.kind) * usize::from(field.array_size);
    }
    let record_size = bits.div_ceil(8).max(8);
    let strings = b"synthetic\0";
    let encrypted_string_size = 138usize
        .checked_sub(record_size + 4)
        .expect("synthetic Map record permits a 138-byte hidden section");
    let header_size = 204;
    let section_bytes = 2 * 40;
    let field_structure_bytes = fields.len() * 4;
    let column_meta_bytes = fields.len() * 24;
    let column_meta_at = header_size + section_bytes + field_structure_bytes;
    let metadata_end = header_size + section_bytes + field_structure_bytes + column_meta_bytes;
    let encrypted_ids = [1_u32, 77_u32];
    let data_start = metadata_end + encrypted_ids.len() * 4;
    let visible_size = record_size + strings.len() + 4;
    let hidden_start = data_start + visible_size;
    let hidden_size = record_size + encrypted_string_size + 4;
    assert_eq!(hidden_size, 138);

    let mut bytes = vec![0; metadata_end];
    bytes[..4].copy_from_slice(b"WDC5");
    for (at, value) in [
        (4, 5),
        (136, 2),
        (140, fields.len() as u32),
        (144, record_size as u32),
        (148, (strings.len() + encrypted_string_size) as u32),
        (152, metadata.table_hash.unwrap_or(0)),
        (156, metadata.layout_hash),
        (160, 48),
        (164, 49),
        (176, metadata.field_count as u32),
        (188, column_meta_bytes as u32),
        (200, 2),
    ] {
        put32(&mut bytes, at, value);
    }
    bytes[174..176].copy_from_slice(&metadata.index.to_le_bytes());
    // First available section.
    put32(&mut bytes, header_size + 8, data_start as u32);
    put32(&mut bytes, header_size + 12, 1);
    put32(&mut bytes, header_size + 16, strings.len() as u32);
    put32(&mut bytes, header_size + 24, 4);
    // Second section is declared by TACT metadata and is represented by 138
    // zero-filled bytes from one E frame.
    let second = header_size + 40;
    bytes[second..second + 8].copy_from_slice(&0x1122_3344_5566_7788_u64.to_le_bytes());
    put32(&mut bytes, second + 8, hidden_start as u32);
    put32(&mut bytes, second + 12, 1);
    put32(&mut bytes, second + 16, encrypted_string_size as u32);
    put32(&mut bytes, second + 24, 4);
    for (field_index, field) in fields.iter().enumerate() {
        let at = column_meta_at + field_index * 24;
        put16(&mut bytes, at, offsets[field_index] as u16);
        put32(&mut bytes, at + 16, field_width(field.kind) as u32);
        put32(&mut bytes, at + 20, u32::from(field.array_size));
    }
    bytes.extend_from_slice(&encrypted_ids[0].to_le_bytes());
    bytes.extend_from_slice(&encrypted_ids[1].to_le_bytes());
    bytes.extend_from_slice(&map_record(fields, &offsets, record_size));
    bytes.extend_from_slice(strings);
    bytes.extend_from_slice(&48_u32.to_le_bytes());
    bytes.resize(bytes.len() + hidden_size, 0);
    (bytes, hidden_start, hidden_size)
}

fn encrypted_frame() -> Vec<u8> {
    let mut frame = vec![b'E', 8];
    frame.extend_from_slice(&0x1122_3344_5566_7788_u64.to_le_bytes());
    frame.push(4);
    frame.extend_from_slice(&[0x55; 4]);
    frame.push(b'S');
    frame.extend_from_slice(&[0xa5; 16]);
    frame
}

fn framed(prefix: &[u8], encrypted: &[u8], hidden_size: usize) -> (Vec<u8>, [u8; 16]) {
    let header_size = 12 + 2 * 24;
    let mut normal = vec![b'N'];
    normal.extend_from_slice(prefix);
    let mut encoded = b"BLTE".to_vec();
    encoded.extend_from_slice(&(header_size as u32).to_be_bytes());
    encoded.extend_from_slice(&[15, 0, 0, 2]);
    for (frame, decoded_size) in [
        (&normal[..], prefix.len() as u32),
        (encrypted, hidden_size as u32),
    ] {
        encoded.extend_from_slice(&(frame.len() as u32).to_be_bytes());
        encoded.extend_from_slice(&decoded_size.to_be_bytes());
        encoded.extend_from_slice(&md5::digest(frame));
    }
    let ekey = md5::digest(&encoded);
    encoded.extend_from_slice(&normal);
    encoded.extend_from_slice(encrypted);
    (encoded, ekey)
}

#[test]
fn one_readable_and_one_tact_declared_map_section_materialize_after_zero_fill() {
    let (map, hidden_start, hidden_size) = partial_map();
    let encrypted = encrypted_frame();
    let (encoded, ekey) = framed(&map[..hidden_start], &encrypted, hidden_size);
    let available =
        blte_available::decode(&encoded, map.len() as u64, ekey, [0; 16], LIMITS).unwrap();
    let (zero_filled, declarations) = available.into_zerofilled();
    assert_eq!(declarations.len(), 1);
    assert_eq!(declarations[0].frame_ordinal, 1);
    assert_eq!(declarations[0].decoded_size, 138);
    assert_eq!(declarations[0].lookup_hex, "1122334455667788");
    assert!(
        zero_filled[hidden_start..hidden_start + hidden_size]
            .iter()
            .all(|byte| *byte == 0)
    );
    let field_count = u32::from_le_bytes(zero_filled[140..144].try_into().unwrap()) as usize;
    let column_meta_size = u32::from_le_bytes(zero_filled[188..192].try_into().unwrap()) as usize;
    let column_meta_at = 204 + 2 * 40 + field_count * 4;
    assert_eq!(column_meta_size, field_count * 24);
    for field in 0..field_count {
        let at = column_meta_at + field * 24;
        assert_eq!(
            u32::from_le_bytes(zero_filled[at + 8..at + 12].try_into().unwrap()),
            0
        );
    }

    let parsed = availability::read_map_70450(&Bytes(&zero_filled)).unwrap();
    assert_eq!(
        parsed
            .sections()
            .iter()
            .filter(|section| section.state == SectionState::UnknownTact)
            .count(),
        1
    );
    let materialized = parsed.materialize().unwrap();
    assert_eq!(materialized.rows().len(), 1);
    assert_eq!(materialized.rows()[0].id, 48);
    assert_eq!(materialized.rows()[0].fields.wdt_file_data_id, 100);
}
