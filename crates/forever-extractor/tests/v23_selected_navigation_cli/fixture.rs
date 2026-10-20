// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-20: bounded synthetic CASC source for v23 CLI integration.
//! Every source byte is generated in memory. This fixture exists only to drive
//! the existing v20 and v22 producers from one deterministic local source.

use forever_db2::catalog::{self, FieldType};
use std::{fs, path::Path};

#[path = "../../../forever-casc/src/local_index/jenkins.rs"]
mod jenkins;
#[path = "../../../forever-casc/src/blte/md5.rs"]
mod md5;

fn put16(bytes: &mut [u8], at: usize, value: u16) {
    bytes[at..at + 2].copy_from_slice(&value.to_le_bytes());
}

fn put32(bytes: &mut [u8], at: usize, value: u32) {
    bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

fn write_value(bytes: &mut [u8], at: usize, size: usize, value: i64) {
    bytes[at..at + size].copy_from_slice(&value.to_le_bytes()[..size]);
}

fn hex16(value: [u8; 16]) -> String {
    value.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn field_width(kind: FieldType) -> usize {
    match kind {
        FieldType::String | FieldType::NonLocalizedString | FieldType::Int | FieldType::Float => 32,
        FieldType::Byte => 8,
        FieldType::Short => 16,
        FieldType::Long => 64,
    }
}

fn synthetic_db2(name: &str, rows: &[(u32, u32)]) -> Vec<u8> {
    let metadata = catalog::resolve_for(catalog::WOWDBDEFS_70450_CATALOG_ID, name)
        .unwrap()
        .unwrap()
        .metadata();
    assert_eq!(metadata.field_count, metadata.file_field_count);
    let fields = &metadata.fields[..metadata.file_field_count];
    let mut offsets = Vec::new();
    let mut bit_offset = 0usize;
    for field in fields {
        offsets.push(bit_offset);
        bit_offset += field_width(field.kind) * usize::from(field.array_size);
    }
    let record_size = bit_offset.div_ceil(8).max(8);
    let mut records = Vec::new();
    for (row_index, (row_id, wdt_id)) in rows.iter().enumerate() {
        let mut record = vec![0; record_size];
        for (field_index, field) in fields.iter().enumerate() {
            if matches!(
                field.kind,
                FieldType::String | FieldType::NonLocalizedString
            ) {
                for array_index in 0..usize::from(field.array_size) {
                    let at = offsets[field_index] / 8 + array_index * 4;
                    let relative = (rows.len() - row_index) * record_size - at;
                    put32(&mut record, at, u32::try_from(relative).unwrap());
                }
            }
        }
        match name {
            "Map" => {
                for (field, value) in [(13, -1), (14, -1), (21, i64::from(*wdt_id))] {
                    let at = offsets[field] / 8;
                    let size = field_width(fields[field].kind) / 8;
                    write_value(&mut record, at, size, value);
                }
            }
            // Match the positive v20 synthetic terrain fixture: LiquidType 22
            // references material 7, and the LiquidObject's signed type field
            // contains the established sentinel value.
            "LiquidType" => {
                let field = 14;
                let at = offsets[field] / 8;
                let size = field_width(fields[field].kind) / 8;
                write_value(&mut record, at, size, i64::from(*wdt_id));
            }
            "LiquidObject" => {
                let field = 2;
                let at = offsets[field] / 8;
                let size = field_width(fields[field].kind) / 8;
                write_value(&mut record, at, size, -1);
            }
            _ => {}
        }
        if metadata.index >= 0 {
            let index = usize::try_from(metadata.index).unwrap();
            put32(&mut record, offsets[index] / 8, *row_id);
        }
        records.extend_from_slice(&record);
    }

    const HEADER: usize = 204;
    const SECTION: usize = 40;
    let section_at = HEADER;
    let column_meta_at = HEADER + SECTION + fields.len() * 4;
    let record_at = column_meta_at + fields.len() * 24;
    let strings = b"synthetic\0";
    let ids_size = if metadata.index < 0 {
        rows.len() * 4
    } else {
        0
    };
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
        (164, 1000),
        (176, metadata.field_count as u32),
        (188, (fields.len() * 24) as u32),
        (200, 1),
    ] {
        put32(&mut bytes, at, value);
    }
    bytes[174..176].copy_from_slice(&metadata.index.to_le_bytes());
    put32(&mut bytes, section_at + 8, record_at as u32);
    put32(&mut bytes, section_at + 12, rows.len() as u32);
    put32(&mut bytes, section_at + 16, strings.len() as u32);
    put32(&mut bytes, section_at + 24, ids_size as u32);
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
    bytes
}

fn root_file(rows: &[(u32, [u8; 16])]) -> Vec<u8> {
    let mut bytes = b"TSFM".to_vec();
    for value in [20_u32, 2, rows.len() as u32, 0] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    for (file_data_id, ckey) in rows {
        for value in [1_u32, 2, 0x1000_0000, 0] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes.push(0);
        bytes.extend_from_slice(&file_data_id.to_le_bytes());
        bytes.extend_from_slice(ckey);
    }
    bytes
}

fn encoding_file(rows: &[([u8; 16], u32, [u8; 16])]) -> Vec<u8> {
    let mut page = Vec::new();
    for (ckey, size, ekey) in rows {
        page.extend_from_slice(&1_u16.to_le_bytes());
        page.extend_from_slice(&size.to_be_bytes());
        page.extend_from_slice(ckey);
        page.extend_from_slice(ekey);
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

fn chunk(tag: &[u8; 4], payload: &[u8]) -> Vec<u8> {
    let mut bytes: Vec<_> = tag.iter().rev().copied().collect();
    bytes.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    bytes.extend_from_slice(payload);
    bytes
}

fn combined_wdt() -> Vec<u8> {
    let mut mphd = vec![0; 32];
    put32(&mut mphd, 0, 0x200);
    let mut main = vec![0; 64 * 64 * 8];
    put32(&mut main, 0, 1);
    let mut maid = vec![0; 64 * 64 * 32];
    // The same synthetic ADT is the selected terrain tile and collision ADT.
    put32(&mut maid, 0, 200);
    put32(&mut maid, 4, 200);
    let mut bytes = chunk(b"MVER", &18_u32.to_le_bytes());
    bytes.extend_from_slice(&chunk(b"MPHD", &mphd));
    bytes.extend_from_slice(&chunk(b"MAIN", &main));
    bytes.extend_from_slice(&chunk(b"MAID", &maid));
    bytes
}

fn combined_adt() -> Vec<u8> {
    let mut header = vec![0; 128];
    put32(&mut header, 52, 42);
    put32(&mut header, 112, 2.0_f32.to_bits());
    let mut heights = vec![0; 145 * 4];
    for index in 0..145 {
        put32(&mut heights, index * 4, (index as f32 * 0.125).to_bits());
    }
    let mut cell = header;
    cell.extend_from_slice(&chunk(b"MCVT", &heights));
    let mut bytes = chunk(b"MVER", &18_u32.to_le_bytes());
    bytes.extend_from_slice(&chunk(b"MCNK", &cell));

    // One bounded synthetic M2 placement for the v22 selected-map reader.
    let mut row = [0; 36];
    put32(&mut row, 0, 300);
    put32(&mut row, 4, 42);
    row[32..34].copy_from_slice(&1024_u16.to_le_bytes());
    row[34..36].copy_from_slice(&0x40_u16.to_le_bytes());
    bytes.extend_from_slice(&chunk(b"MDDF", &row));
    bytes
}

fn synthetic_m2() -> Vec<u8> {
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

fn normal_record(bytes: &[u8]) -> Vec<u8> {
    let mut record = b"BLTE\0\0\0\0N".to_vec();
    record.extend_from_slice(bytes);
    record
}

/// Write a tiny local source that contains one selected map with one tile and
/// one unrelated WDT-zero row. All catalog tables and archive records are
/// synthetic, keyless, and generated in this function.
pub fn write_client(client: &Path) {
    let map_id = catalog::resolve_for(catalog::WOWDBDEFS_70450_CATALOG_ID, "Map")
        .unwrap()
        .unwrap()
        .metadata()
        .file_data_id;
    let liquid_type_id = catalog::resolve_for(catalog::WOWDBDEFS_70450_CATALOG_ID, "LiquidType")
        .unwrap()
        .unwrap()
        .metadata()
        .file_data_id;
    let liquid_material_id =
        catalog::resolve_for(catalog::WOWDBDEFS_70450_CATALOG_ID, "LiquidMaterial")
            .unwrap()
            .unwrap()
            .metadata()
            .file_data_id;
    let godi_id =
        catalog::resolve_for(catalog::WOWDBDEFS_70450_CATALOG_ID, "GameObjectDisplayInfo")
            .unwrap()
            .unwrap()
            .metadata()
            .file_data_id;
    let liquid_object_id =
        catalog::resolve_for(catalog::WOWDBDEFS_70450_CATALOG_ID, "LiquidObject")
            .unwrap()
            .unwrap()
            .metadata()
            .file_data_id;

    let inputs = vec![
        (map_id, synthetic_db2("Map", &[(48, 100), (49, 0)])),
        (liquid_type_id, synthetic_db2("LiquidType", &[(22, 7)])),
        (
            liquid_material_id,
            synthetic_db2("LiquidMaterial", &[(7, 0)]),
        ),
        (100, combined_wdt()),
        (200, combined_adt()),
        (300, synthetic_m2()),
        (
            godi_id,
            b"poison synthetic GameObjectDisplayInfo; must not be read".to_vec(),
        ),
        (liquid_object_id, synthetic_db2("LiquidObject", &[(9, 0)])),
    ];
    let mut source_routes = Vec::new();
    let mut records = Vec::<([u8; 16], Vec<u8>)>::new();
    let mut root_rows = Vec::new();
    for (ordinal, (file_data_id, data)) in inputs.into_iter().enumerate() {
        let ckey = md5::digest(&data);
        let ekey = [0x80_u8.wrapping_add(ordinal as u8); 16];
        source_routes.push((ckey, data.len() as u32, ekey));
        records.push((ekey, normal_record(&data)));
        root_rows.push((file_data_id, ckey));
    }
    let root = root_file(&root_rows);
    let root_ckey = md5::digest(&root);
    let root_ekey = [0x33; 16];
    let mut routes = vec![(root_ckey, root.len() as u32, root_ekey)];
    routes.extend(source_routes);
    let encoding = encoding_file(&routes);
    let encoding_ckey = md5::digest(&encoding);
    let encoding_ekey = [0x44; 16];
    records.push((root_ekey, normal_record(&root)));
    records.push((encoding_ekey, normal_record(&encoding)));

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
    fs::create_dir_all(&data).unwrap();
    fs::create_dir_all(&config_dir).unwrap();
    fs::write(config_dir.join(&config_name), &config).unwrap();
    fs::write(
        client.join(".build.info"),
        format!(
            "Product!STRING:0|Active!DEC:1|Version!STRING:0|Build Key!HEX:16|CDN Key!HEX:16|Tags!STRING:0\nwow_classic_beta|1|1.60.1.70450|{}|{}|enUS\n",
            hex16(config_ckey),
            hex16([0x21; 16])
        ),
    )
    .unwrap();

    let mut archive = Vec::new();
    let mut index_rows = Vec::new();
    for (ekey, record) in records {
        let offset = archive.len() as u64;
        archive.extend_from_slice(&[0xa5; 30]);
        archive.extend_from_slice(&record);
        index_rows.push(index_entry(
            ekey,
            offset,
            u32::try_from(30 + record.len()).unwrap(),
        ));
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
