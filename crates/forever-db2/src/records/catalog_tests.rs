// SPDX-License-Identifier: GPL-3.0-or-later
//! Synthetic catalog coverage, independent of generated decoder Kind slices.
//! Fixtures use raw source field declarations and exercise the existing decoder.
//! These tests are not client-file or runtime acceptance evidence.
use super::*;
use crate::{CharacterTable, DecodedTable, RecordError, Value};
use std::collections::BTreeSet;

fn put32(data: &mut [u8], offset: usize, value: u32) {
    data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn put16(data: &mut [u8], offset: usize, value: u16) {
    data[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

fn width(field: FieldMetadata) -> usize {
    match field.kind {
        FieldType::Byte => 1,
        FieldType::Short => 2,
        FieldType::Long => 8,
        _ => 4,
    }
}

fn header(table: CharacterTable, fields: usize) -> Vec<u8> {
    let mut data = vec![0; 204];
    data[..4].copy_from_slice(b"WDC5");
    put32(&mut data, 4, 5);
    put32(&mut data, 140, fields as u32);
    put32(&mut data, 152, table.schema_identity().0);
    put32(&mut data, 156, table.schema_identity().1);
    put32(&mut data, 176, fields as u32);
    data
}

/// One direct row with extremes, raw float payloads and UTF-8 strings; optional
/// parent lookup and chained copies. It never consults generated decode_fields.
fn fixture(metadata: &TableMetadata, lookup: bool) -> (Vec<u8>, Vec<Vec<Value>>, bool) {
    let table = CharacterTable::from_catalog(metadata.name)
        .unwrap()
        .unwrap();
    fixture_for(metadata, table, lookup)
}

fn fixture_for(
    metadata: &TableMetadata,
    table: CharacterTable,
    lookup: bool,
) -> (Vec<u8>, Vec<Vec<Value>>, bool) {
    fixture_for_fields(metadata, metadata.fields, table, lookup)
}

fn fixture_for_fields(
    metadata: &TableMetadata,
    wire_fields: &[FieldMetadata],
    table: CharacterTable,
    lookup: bool,
) -> (Vec<u8>, Vec<Vec<Value>>, bool) {
    let fields = &wire_fields[..metadata.file_field_count];
    let record_size: usize = fields
        .iter()
        .map(|field| width(*field) * usize::from(field.array_size))
        .sum();
    let mut record = vec![0; record_size];
    let mut strings = Vec::new();
    let mut expected = Vec::new();
    let mut offsets = Vec::new();
    let mut offset = 0;
    for (index, field) in fields.iter().enumerate() {
        offsets.push(offset);
        let mut values = Vec::new();
        for element in 0..field.array_size {
            let size = width(*field);
            let (raw, value) = if metadata.index == index as i16 {
                (7, Value::Unsigned(7))
            } else {
                match field.kind {
                    FieldType::String | FieldType::NonLocalizedString => {
                        let text = format!("á\"\\\n界{element}");
                        let relative = record_size + strings.len() - offset;
                        strings.extend_from_slice(text.as_bytes());
                        strings.push(0);
                        (relative as u64, Value::Text(text.to_owned()))
                    }
                    FieldType::Float => {
                        let bits =
                            [0x80000000, 0x7FC12345, 0xFF800000, 1][usize::from(element) % 4];
                        (u64::from(bits), Value::FloatBits(bits))
                    }
                    FieldType::Long if field.signed => (u64::MAX, Value::Signed64(-1)),
                    FieldType::Long => (u64::MAX, Value::Unsigned64(u64::MAX)),
                    _ if field.signed => (u64::MAX, Value::Signed(-1)),
                    _ => {
                        let raw = (1u64 << (size * 8)) - 1;
                        (raw, Value::Unsigned(raw as u32))
                    }
                }
            };
            record[offset..offset + size].copy_from_slice(&raw.to_le_bytes()[..size]);
            values.push(value);
            offset += size;
        }
        expected.push(values);
    }
    let appended = metadata.appended_parent_storage_bits().is_some();
    if appended {
        expected.push(vec![if metadata.fields.last().unwrap().signed {
            Value::Signed(0)
        } else {
            Value::Unsigned(0)
        }]);
    }
    let lookup = lookup && metadata.parent_index >= 0;
    let parent = if appended { 0x80000000 } else { 127 };
    if lookup {
        let field = metadata.fields[metadata.parent_index as usize];
        expected[metadata.parent_index as usize][0] = if field.signed {
            Value::Signed(parent as i32)
        } else {
            Value::Unsigned(parent)
        };
    }
    let copies = metadata.index < 0 || width(fields[metadata.index as usize]) == 4;
    let copy_count = if copies { 3 } else { 0 };
    let id_size = if metadata.index < 0 { 4 } else { 0 };
    let mut data = header(table, fields.len());
    put32(&mut data, 136, 1);
    put32(&mut data, 144, record_size as u32);
    put32(&mut data, 148, strings.len() as u32);
    put32(&mut data, 160, 7);
    put32(&mut data, 164, 80000);
    put32(&mut data, 184, u32::from(lookup));
    put32(&mut data, 188, (fields.len() * 24) as u32);
    put32(&mut data, 200, 1);
    data.resize(244 + 4 * fields.len(), 0);
    for (index, field) in fields.iter().enumerate() {
        let column = data.len();
        data.resize(column + 24, 0);
        data[column..column + 2].copy_from_slice(&((offsets[index] * 8) as u16).to_le_bytes());
        put32(&mut data, column + 16, (width(*field) * 8) as u32);
        put32(&mut data, column + 20, u32::from(field.array_size));
    }
    let file_offset = data.len() as u32;
    put32(&mut data, 212, file_offset);
    put32(&mut data, 216, 1);
    put32(&mut data, 220, strings.len() as u32);
    put32(&mut data, 228, id_size);
    put32(&mut data, 232, if lookup { 20 } else { 0 });
    put32(&mut data, 240, copy_count);
    data.extend(record);
    data.extend(strings);
    if id_size != 0 {
        data.extend(7u32.to_le_bytes());
    }
    if copies {
        for (new, source) in [(70000u32, 7u32), (70001, 70000), (80000, 0)] {
            data.extend(new.to_le_bytes());
            data.extend(source.to_le_bytes());
        }
    }
    if lookup {
        for word in [1u32, parent, parent, parent, 0] {
            data.extend(word.to_le_bytes());
        }
    }
    (data, expected, copies)
}

fn empty_sparse_parent_fixture(name: &str) -> Vec<u8> {
    let table = CharacterTable::from_catalog_id(WOWDBDEFS_70338_CATALOG_ID, name)
        .unwrap()
        .unwrap();
    let mut data = header(table, table.schema().fields.len());
    put16(&mut data, 172, 1);
    for kind in table.schema().fields {
        let unused: i16 = match kind {
            Kind::Unsigned(8, _) | Kind::Signed(8, _) => 24,
            Kind::Unsigned(16, _) | Kind::Signed(16, _) => 16,
            _ => 0,
        };
        data.extend_from_slice(&unused.to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes());
    }
    data
}

fn vendor_sparse_parent_fixture() -> Vec<u8> {
    let table = CharacterTable::from_catalog_id(
        WOWDBDEFS_70338_CATALOG_ID,
        "CollectableSourceVendorSparse",
    )
    .unwrap()
    .unwrap();
    let fields = table.schema().fields;
    let row_values = [
        0xFFFF_FFFFu32,
        0x8000_0000,
        0x3F80_0000,
        0x7FC1_2345,
        1,
        2,
        3,
        4,
    ];
    let record_offset = 412u32;
    let record_size = (row_values.len() * 4) as u32;
    let catalog_offset = record_offset + record_size;
    let end_of_ids = catalog_offset + 4 + 6 + 4;
    let parent_count = 1u32;

    let mut data = vec![0; 244];
    data[..4].copy_from_slice(b"WDC5");
    put32(&mut data, 4, 5);
    put32(&mut data, 136, 1);
    put32(&mut data, 140, fields.len() as u32);
    put32(&mut data, 144, record_size);
    put32(&mut data, 152, table.schema_identity().0);
    put32(&mut data, 156, table.schema_identity().1);
    put32(&mut data, 160, 7);
    put32(&mut data, 164, 7);
    put16(&mut data, 172, 1);
    put16(&mut data, 174, u16::MAX);
    put32(&mut data, 176, fields.len() as u32);
    put32(&mut data, 184, 1);
    put32(&mut data, 188, (fields.len() * 24) as u32);
    put32(&mut data, 200, 1);

    // One WDC5 section with one direct sparse record and a one-entry parent block.
    put32(&mut data, 212, record_offset);
    put32(&mut data, 216, 1);
    put32(&mut data, 224, catalog_offset);
    put32(&mut data, 228, 4);
    put32(&mut data, 232, 20);
    put32(&mut data, 236, 1);

    for _ in fields {
        data.extend_from_slice(&0i16.to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes());
    }
    data.resize(data.len() + fields.len() * 24, 0);
    assert_eq!(data.len(), record_offset as usize);
    for value in row_values {
        data.extend_from_slice(&value.to_le_bytes());
    }
    data.extend(7u32.to_le_bytes());
    data.extend(record_offset.to_le_bytes());
    data.extend((record_size as u16).to_le_bytes());
    data.extend(u32::MAX.to_le_bytes());
    assert_eq!(data.len(), end_of_ids as usize);
    data.extend(parent_count.to_le_bytes());
    data.extend(100u32.to_le_bytes());
    data.extend(100u32.to_le_bytes());
    data.extend(100u32.to_le_bytes());
    data.extend(0u32.to_le_bytes());
    data
}

#[test]
fn catalog_sparse_parent_tables_have_raw_only_bounded_validation() {
    for name in [
        "CollectableSourceEncounterSparse",
        "CollectableSourceQuestSparse",
    ] {
        let data = empty_sparse_parent_fixture(name);
        let table = CharacterTable::from_catalog_id(WOWDBDEFS_70338_CATALOG_ID, name)
            .unwrap()
            .unwrap();
        assert_eq!(
            DecodedTable::read(&data, table).err(),
            Some(RecordError::Unsupported)
        );
        assert!(
            DecodedTable::validate_catalog_raw_source_bytes(
                &data,
                WOWDBDEFS_70338_CATALOG_ID,
                name,
            )
            .is_ok()
        );
        assert_eq!(
            DecodedTable::validate_catalog_raw_source_bytes(
                &data[..data.len() - 1],
                WOWDBDEFS_70338_CATALOG_ID,
                name,
            ),
            Err(RecordError::Bounds)
        );
    }

    let data = vendor_sparse_parent_fixture();
    let table = CharacterTable::from_catalog_id(
        WOWDBDEFS_70338_CATALOG_ID,
        "CollectableSourceVendorSparse",
    )
    .unwrap()
    .unwrap();
    assert_eq!(
        DecodedTable::read(&data, table).err(),
        Some(RecordError::Unsupported)
    );
    assert!(
        DecodedTable::validate_catalog_raw_source_bytes(
            &data,
            WOWDBDEFS_70338_CATALOG_ID,
            "CollectableSourceVendorSparse",
        )
        .is_ok()
    );
    let mut bad_parent_block = data.clone();
    put32(&mut bad_parent_block, 232, 19);
    assert_eq!(
        DecodedTable::validate_catalog_raw_source_bytes(
            &bad_parent_block,
            WOWDBDEFS_70338_CATALOG_ID,
            "CollectableSourceVendorSparse",
        ),
        Err(RecordError::Bounds)
    );
}

#[test]
fn catalog_inventory_and_explicit_blockers() {
    assert_eq!(tables().len(), 1126);
    assert_eq!(
        tables()
            .iter()
            .filter(|table| table.shape_compatible())
            .count(),
        1126
    );
    assert_eq!(
        tables()
            .iter()
            .map(|table| table.file_data_id)
            .collect::<BTreeSet<_>>()
            .len(),
        1126
    );
    assert!(tables().windows(2).all(|pair| pair[0].name < pair[1].name));
    let blocked: Vec<_> = tables()
        .iter()
        .filter(|table| !table.shape_compatible())
        .map(|table| (table.name, table.blockers))
        .collect();
    assert!(blocked.is_empty());
    assert_eq!(
        CharacterTable::from_catalog("UnknownSyntheticTable"),
        Ok(None)
    );
    assert_eq!(CharacterTable::named("ItemSparse"), None);
    assert!(
        CharacterTable::from_catalog("ItemSparse")
            .unwrap()
            .is_some()
    );
}

#[test]
fn catalog_matches_every_physical_field_of_all_56_reviewed_schemas() {
    let mut reviewed_count = 0;
    for metadata in tables() {
        let Some(reviewed) = CharacterTable::named(metadata.name) else {
            continue;
        };
        reviewed_count += 1;
        assert_eq!(
            CharacterTable::from_catalog(metadata.name),
            Ok(Some(reviewed))
        );
        assert_eq!(reviewed.file_id(), metadata.file_data_id);
        let old = reviewed.schema();
        assert_eq!(old.layout, metadata.layout_hash, "{}", metadata.name);
        assert_eq!(old.index, metadata.index, "{}", metadata.name);
        assert_eq!(old.parent, metadata.parent_index, "{}", metadata.name);
        assert_eq!(
            old.fields,
            metadata.decode_fields.unwrap(),
            "{}",
            metadata.name
        );
        assert_eq!(
            metadata.field_count,
            old.fields.len() + usize::from(metadata.appended_parent_storage_bits().is_some())
        );
        let indirect = CharacterTable::Catalog(resolve(metadata.name).unwrap().unwrap());
        assert_eq!(indirect.schema_identity(), reviewed.schema_identity());
        let mut input = header(reviewed, metadata.file_field_count);
        input.resize(input.len() + 4 * metadata.file_field_count, 0);
        assert!(DecodedTable::read(&input, indirect).is_ok());
        if let Some(hash) = old.hash {
            put32(&mut input, 152, hash ^ 1);
            assert_eq!(
                DecodedTable::read(&input, indirect).err(),
                Some(RecordError::Schema)
            );
        }
    }
    assert_eq!(reviewed_count, 56);
}

#[test]
fn catalog_decodes_all_1126_shapes_and_exact_empty_variants() {
    for metadata in tables().iter().filter(|table| table.shape_compatible()) {
        let table = CharacterTable::from_catalog(metadata.name)
            .unwrap()
            .unwrap();
        let (data, expected, copies) = fixture(metadata, false);
        let decoded = DecodedTable::read(&data, table)
            .unwrap_or_else(|error| panic!("{}: {:?}", metadata.name, error));
        assert_eq!(decoded.rows()[&7], expected, "{}", metadata.name);
        assert_eq!(
            decoded.rows().len(),
            if copies { 3 } else { 1 },
            "{}",
            metadata.name
        );
        if copies {
            for id in [70000, 70001] {
                let mut copied = expected.clone();
                if metadata.index >= 0 {
                    copied[metadata.index as usize][0] = Value::Unsigned(id);
                }
                assert_eq!(decoded.rows()[&id], copied, "{}", metadata.name);
            }
        }
        let mut empty = header(table, metadata.file_field_count);
        empty.resize(empty.len() + 4 * metadata.file_field_count, 0);
        assert!(DecodedTable::read(&empty, table).unwrap().rows().is_empty());
        empty.pop();
        assert_eq!(
            DecodedTable::read(&empty, table).err(),
            Some(RecordError::Bounds),
            "{}",
            metadata.name
        );
    }
}

#[test]
fn empty_catalog_appended_parent_header_counts_are_admitted_without_rows() {
    let table =
        CharacterTable::from_catalog_id(WOWDBDEFS_70338_CATALOG_ID, "ExteriorComponentGroup")
            .unwrap()
            .unwrap();
    let metadata = resolve_for(WOWDBDEFS_70338_CATALOG_ID, "ExteriorComponentGroup")
        .unwrap()
        .unwrap()
        .metadata();
    assert_eq!(metadata.file_field_count, 2);
    assert_eq!(metadata.field_count, 3);
    assert_eq!(metadata.parent_index, 2);

    let mut empty = header(table, metadata.field_count);
    empty.resize(empty.len() + 4 * metadata.field_count, 0);
    assert!(DecodedTable::read(&empty, table).unwrap().rows().is_empty());

    let mut with_rows = empty.clone();
    put32(&mut with_rows, 136, 1);
    assert_eq!(
        DecodedTable::read(&with_rows, table).err(),
        Some(RecordError::Schema)
    );

    let mut wrong_count = empty;
    put32(&mut wrong_count, 140, 4);
    assert_eq!(
        DecodedTable::read(&wrong_count, table).err(),
        Some(RecordError::Schema)
    );
}

#[test]
fn raw_catalog_source_validation_preserves_non_utf8_text_without_projecting_it() {
    let catalog_id = WOWDBDEFS_70338_CATALOG_ID;
    let metadata = tables_for(catalog_id)
        .unwrap()
        .iter()
        .find(|metadata| metadata.name == "SpellMissileMotion")
        .unwrap();
    let table = CharacterTable::from_catalog_id(catalog_id, metadata.name)
        .unwrap()
        .unwrap();
    let (mut data, _, _) = fixture_for(metadata, table, false);
    let invalid_utf8 = data
        .windows(2)
        .position(|pair| pair == [0xC3, 0xA1])
        .unwrap();
    data[invalid_utf8] = 0x97;
    assert_eq!(
        DecodedTable::read(&data, table).err(),
        Some(RecordError::String)
    );
    assert_eq!(
        DecodedTable::validate_catalog_raw_source_bytes(&data, catalog_id, metadata.name),
        Ok(())
    );

    let mut truncated = data;
    truncated.pop();
    assert!(
        DecodedTable::validate_catalog_raw_source_bytes(&truncated, catalog_id, metadata.name)
            .is_err()
    );
}

#[test]
fn catalog_parent_lookups_and_normalized_appended_parents() {
    let mut parents = 0;
    let mut normalized_bytes = 0;
    for metadata in tables()
        .iter()
        .filter(|table| table.shape_compatible() && table.parent_index >= 0)
    {
        parents += 1;
        let table = CharacterTable::from_catalog(metadata.name)
            .unwrap()
            .unwrap();
        let (data, expected, copies) = fixture(metadata, true);
        let decoded = DecodedTable::read(&data, table)
            .unwrap_or_else(|error| panic!("{}: {:?}", metadata.name, error));
        assert_eq!(decoded.rows()[&7], expected, "{}", metadata.name);
        if copies {
            assert_eq!(
                decoded.rows()[&70000][metadata.parent_index as usize],
                expected[metadata.parent_index as usize],
                "{}",
                metadata.name
            );
        }
        if metadata.appended_parent_storage_bits().is_some()
            && metadata.fields.last().unwrap().kind == FieldType::Byte
            && metadata.fields.last().unwrap().signed
        {
            normalized_bytes += 1;
            assert_eq!(expected.last().unwrap(), &[Value::Signed(i32::MIN)]);
        }
    }
    assert_eq!(parents, 437);
    assert_eq!(normalized_bytes, 5);
}

#[test]
fn catalog_does_not_relax_record_variant_and_identity_gates() {
    let metadata = find("ItemSparse").unwrap();
    let table = CharacterTable::from_catalog(metadata.name)
        .unwrap()
        .unwrap();
    let (data, _, _) = fixture(metadata, false);
    for (offset, value, error) in [
        (156, metadata.layout_hash ^ 1, RecordError::Schema),
        (
            140,
            metadata.file_field_count as u32 + 1,
            RecordError::Schema,
        ),
        (172, 1, RecordError::Unsupported),
        (204, 1, RecordError::Unsupported),
        (200, 2, RecordError::Unsupported),
    ] {
        let mut invalid = data.clone();
        put32(&mut invalid, offset, value);
        assert_eq!(
            DecodedTable::read(&invalid, table).err(),
            Some(error),
            "offset {offset}"
        );
    }
    assert_eq!(
        DecodedTable::read(&data[..data.len() - 1], table).err(),
        Some(RecordError::Bounds)
    );
}

fn word(data: &[u8], offset: usize) -> usize {
    u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()) as usize
}

#[test]
fn catalog_signed_64_extremes_copies_and_compressed_rejections() {
    // Independently checked first FT_LONG column of InitiativeReward (9 fields,
    // inline ID3, 0x51D55D40) and TraitCondAccountElement (6, ID1, 0x70915E24).
    for (name, layout, fields, index) in [
        ("InitiativeReward", 0x51D55D40, 9, 3),
        ("TraitCondAccountElement", 0x70915E24, 6, 1),
    ] {
        let metadata = find(name).unwrap();
        assert_eq!(
            (
                metadata.layout_hash,
                metadata.file_field_count,
                metadata.index
            ),
            (layout, fields, index)
        );
        let table = CharacterTable::from_catalog(name).unwrap().unwrap();
        let (base, _, _) = fixture(metadata, false);
        let record = word(&base, 212);
        for value in [i64::MIN, i64::MAX, -1, 0, 0x1_0000_0001] {
            let mut data = base.clone();
            data[record..record + 8].copy_from_slice(&value.to_le_bytes());
            let decoded = DecodedTable::read(&data, table).unwrap();
            for id in [7, 70000, 70001] {
                assert_eq!(decoded.rows()[&id][0], [Value::Signed64(value)]);
            }
        }
        for mode in 1..=5 {
            let mut data = base.clone();
            let column = 244 + fields * 4;
            put32(&mut data, column + 8, mode);
            put32(&mut data, column + 16, 8);
            assert_eq!(
                DecodedTable::read(&data, table).err(),
                Some(RecordError::Column),
                "{name} mode {mode}"
            );
        }
    }
}

#[test]
fn catalog_string_arrays_use_each_pointer_and_reject_bad_elements() {
    // Six nonlocalized strings start at record offset0 in BattlePetEffectProperties.
    let metadata = find("BattlePetEffectProperties").unwrap();
    assert_eq!(
        (metadata.layout_hash, metadata.file_field_count),
        (0xED4FBC95, 3)
    );
    let table = CharacterTable::from_catalog(metadata.name)
        .unwrap()
        .unwrap();
    let (base, _, _) = fixture(metadata, false);
    let record = word(&base, 212);
    let decoded = DecodedTable::read(&base, table).unwrap();
    assert_eq!(
        decoded.rows()[&7][0],
        (0..6)
            .map(|element| Value::Text(format!("á\"\\\n界{element}")))
            .collect::<Vec<_>>()
    );
    let mut null = base.clone();
    put32(&mut null, record + 4, 0);
    assert_eq!(
        DecodedTable::read(&null, table).unwrap().rows()[&7][0][1],
        Value::Text(String::new())
    );
    for element in 0..6 {
        let mut invalid = base.clone();
        put32(&mut invalid, record + 4 * element, u32::MAX);
        assert_eq!(
            DecodedTable::read(&invalid, table).err(),
            Some(RecordError::String)
        );
    }
    let strings = record + word(&base, 144);
    let mut invalid_utf8 = base.clone();
    invalid_utf8[strings] = 0xFF;
    assert_eq!(
        DecodedTable::read(&invalid_utf8, table).err(),
        Some(RecordError::String)
    );
    let mut unterminated = base.clone();
    unterminated[strings + word(&base, 148) - 1] = b'x';
    assert_eq!(
        DecodedTable::read(&unterminated, table).err(),
        Some(RecordError::String)
    );
    let mut compressed = base;
    put32(&mut compressed, 244 + 3 * 4 + 8, 1);
    assert_eq!(
        DecodedTable::read(&compressed, table).err(),
        Some(RecordError::Column)
    );
    // 80k * (six strings + short + six bytes) exceeds the scalar budget,
    // while incorrectly charging only three logical fields would not.
    put32(&mut compressed, 136, 80000);
    assert_eq!(
        DecodedTable::read(&compressed, table).err(),
        Some(RecordError::Limit)
    );
}

#[test]
fn catalog_unsigned_appended_byte_uses_unsigned_32_bit_storage() {
    let metadata = find("UiWidgetVisTypeDataReq").unwrap();
    assert_eq!(
        (
            metadata.layout_hash,
            metadata.file_field_count,
            metadata.field_count,
            metadata.parent_index
        ),
        (0x9A097D11, 5, 6, 5)
    );
    assert_eq!(
        metadata.fields[5],
        FieldMetadata {
            kind: FieldType::Byte,
            array_size: 1,
            signed: false
        }
    );
    let table = CharacterTable::from_catalog(metadata.name)
        .unwrap()
        .unwrap();
    let (without_lookup, _, _) = fixture(metadata, false);
    assert_eq!(
        DecodedTable::read(&without_lookup, table).unwrap().rows()[&7][5],
        [Value::Unsigned(0)]
    );
    let (base, _, _) = fixture(metadata, true);
    for parent in [255u32, 256, 0x80000000, u32::MAX] {
        let mut data = base.clone();
        let value_offset = data.len() - 8;
        put32(&mut data, value_offset, parent);
        let decoded = DecodedTable::read(&data, table).unwrap();
        for id in [7, 70000, 70001] {
            assert_eq!(decoded.rows()[&id][5], [Value::Unsigned(parent)]);
        }
    }
    let mut invalid = base;
    let ordinal_offset = invalid.len() - 4;
    put32(&mut invalid, ordinal_offset, 1);
    assert_eq!(
        DecodedTable::read(&invalid, table).err(),
        Some(RecordError::Id)
    );
}

#[test]
fn build_70338_catalog_is_separately_pinned_and_schema_equivalent() {
    let old = tables_for(WOWDBDEFS_70205_CATALOG_ID).unwrap();
    let new = tables_for(WOWDBDEFS_70338_CATALOG_ID).unwrap();
    assert_eq!(old.len(), 1161);
    assert_eq!(new.len(), 1161);
    for (old, new) in old.iter().zip(new) {
        assert_eq!(old.name, new.name);
        assert_eq!(old.file_data_id, new.file_data_id);
        assert_eq!(old.table_hash, new.table_hash);
        assert_eq!(old.other_layout_hashes, new.other_layout_hashes);
        assert_eq!(old.layout_hash, new.layout_hash);
        assert_eq!(old.index, new.index);
        assert_eq!(old.parent_index, new.parent_index);
        assert_eq!(old.field_count, new.field_count);
        assert_eq!(old.file_field_count, new.file_field_count);
        assert_eq!(old.fields, new.fields);
        assert_eq!(old.blockers, new.blockers);
        assert_eq!(old.decode_fields, new.decode_fields);
    }

    for (name, id, fields, table_hash, layout_hash) in [
        ("TactKey", 1_302_850, 1, 0xDF2F53CF, 0xCBA490FC),
        ("TactKeyLookup", 1_302_851, 1, 0xAFC190D1, 0x4983962C),
    ] {
        let table = resolve_for(WOWDBDEFS_70338_CATALOG_ID, name)
            .unwrap()
            .unwrap();
        assert_eq!(table.catalog_id(), WOWDBDEFS_70338_CATALOG_ID);
        assert_eq!(table.metadata().file_data_id, id);
        assert_eq!(table.metadata().field_count, fields);
        assert_eq!(table.metadata().table_hash, Some(table_hash));
        assert_eq!(table.metadata().layout_hash, layout_hash);
    }

    let source = provenance(WOWDBDEFS_70338_CATALOG_ID).unwrap();
    assert_eq!(source.source_commit, WOWDBDEFS_70338_SOURCE_COMMIT);
    assert_eq!(source.facts_sha256, WOWDBDEFS_70338_FACTS_SHA256);
}

// The WoWDBDefs physical type keeps the DBD column category (Int), while
// decode_kinds retains <8>/<16>/<64>. Author wire widths from the independently
// pinned prior target, after the complete shape-equality test above. This is
// supplied synthetic decoder evidence, not a native or client-file oracle.
fn fixture_70374(
    metadata: &TableMetadata,
    table: CharacterTable,
    lookup: bool,
) -> (Vec<u8>, Vec<Vec<Value>>, bool) {
    let prior = resolve_for(WOWDBDEFS_70338_CATALOG_ID, metadata.name)
        .unwrap()
        .unwrap()
        .metadata();
    let mut fields = prior.fields.to_vec();
    for (field, kind) in fields.iter_mut().zip(prior.decode_fields.unwrap()) {
        field.kind = match kind {
            Kind::Unsigned(8, _) | Kind::Signed(8, _) => FieldType::Byte,
            Kind::Unsigned(16, _) | Kind::Signed(16, _) => FieldType::Short,
            Kind::Unsigned(64, _) | Kind::Signed(64, _) => FieldType::Long,
            _ => field.kind,
        };
    }
    fixture_for_fields(metadata, &fields, table, lookup)
}

// Added 2026-10-14: new target evidence is synthetic, not inherited client acceptance.
#[test]
fn build_70374_catalog_has_independent_provenance_and_all_1161_shapes() {
    let old = tables_for(WOWDBDEFS_70338_CATALOG_ID).unwrap();
    let new = tables_for(WOWDBDEFS_70374_CATALOG_ID).unwrap();
    assert_eq!(old.len(), 1161);
    assert_eq!(new.len(), 1161);
    assert!(new.windows(2).all(|pair| pair[0].name < pair[1].name));
    let mut ids = BTreeSet::new();
    for (old, new) in old.iter().zip(new) {
        assert!(ids.insert(new.file_data_id));
        assert_eq!(old.name, new.name);
        assert_eq!(old.file_data_id, new.file_data_id);
        assert_eq!(old.table_hash, new.table_hash);
        assert_eq!(old.other_layout_hashes, new.other_layout_hashes);
        assert_eq!(old.layout_hash, new.layout_hash);
        assert_eq!(old.index, new.index);
        assert_eq!(old.parent_index, new.parent_index);
        assert_eq!(old.field_count, new.field_count);
        assert_eq!(old.file_field_count, new.file_field_count);
        assert_eq!(old.fields, new.fields);
        assert_eq!(old.blockers, new.blockers);
        assert_eq!(old.decode_fields, new.decode_fields);
        assert!(new.shape_compatible());
        let previous = resolve_for(WOWDBDEFS_70338_CATALOG_ID, new.name)
            .unwrap()
            .unwrap();
        let current = resolve_for(WOWDBDEFS_70374_CATALOG_ID, new.name)
            .unwrap()
            .unwrap();
        assert_ne!(previous, current);
        assert_eq!(current.catalog_id(), WOWDBDEFS_70374_CATALOG_ID);
    }
    let source = provenance(WOWDBDEFS_70374_CATALOG_ID).unwrap();
    assert_eq!(source.id, "wowdbdefs-70374-v1");
    assert_eq!(
        source.source_commit,
        "464a25b3a7c02b0c74726be7a53a0b0ccf1a2d10"
    );
    assert_eq!(
        source.source_blob,
        "8a5d8122f312ae6a9564e8286fdc491831fecc22"
    );
    assert_eq!(
        source.source_sha256,
        "4314266fe055e30207e1c39498a7a31058a1aa2ab6cd674726491b313d135265"
    );
    assert_eq!(
        source.facts_sha256,
        "cf9a5a549136090c3aba1dddccc76c4aec45eb8a30162f053860f2bed44d2faf"
    );
    assert_ne!(source, provenance(WOWDBDEFS_70338_CATALOG_ID).unwrap());
    assert!(supports(WOWDBDEFS_70374_CATALOG_ID));
    assert_eq!(
        resolve_for(WOWDBDEFS_70374_CATALOG_ID, "UnknownSyntheticTable"),
        Ok(None)
    );
    assert!(!supports("wowdbdefs-70374-v2"));
    assert_eq!(tables().len(), 1126);
}

#[test]
fn build_70374_all_shapes_decode_direct_copies_and_exact_empty_inputs() {
    for metadata in tables_for(WOWDBDEFS_70374_CATALOG_ID).unwrap() {
        let table = CharacterTable::from_catalog_id(WOWDBDEFS_70374_CATALOG_ID, metadata.name)
            .unwrap()
            .unwrap();
        for lookup in [false, true] {
            let (data, expected, copies) = fixture_70374(metadata, table, lookup);
            let decoded = DecodedTable::read(&data, table)
                .unwrap_or_else(|e| panic!("{} lookup={lookup}: {e:?}", metadata.name));
            assert_eq!(decoded.rows()[&7], expected, "{}", metadata.name);
            assert_eq!(
                decoded.rows().len(),
                if copies { 3 } else { 1 },
                "{}",
                metadata.name
            );
            if copies {
                for id in [70000, 70001] {
                    let mut expected = expected.clone();
                    if metadata.index >= 0 {
                        expected[metadata.index as usize][0] = Value::Unsigned(id);
                    }
                    assert_eq!(decoded.rows()[&id], expected, "{}", metadata.name);
                }
            }
        }
        let mut empty = header(table, metadata.file_field_count);
        empty.resize(empty.len() + 4 * metadata.file_field_count, 0);
        assert!(DecodedTable::read(&empty, table).unwrap().rows().is_empty());
        empty.pop();
        assert_eq!(
            DecodedTable::read(&empty, table).err(),
            Some(RecordError::Bounds),
            "{}",
            metadata.name
        );
    }
}

#[test]
fn build_70374_target_table_layout_and_field_gates_remain_strict() {
    for name in ["Map", "GameObjectDisplayInfo", "TactKey", "TactKeyLookup"] {
        let table = CharacterTable::from_catalog_id(WOWDBDEFS_70374_CATALOG_ID, name)
            .unwrap()
            .unwrap();
        let metadata = resolve_for(WOWDBDEFS_70374_CATALOG_ID, name)
            .unwrap()
            .unwrap()
            .metadata();
        let (data, _, _) = fixture_70374(metadata, table, false);
        for offset in [140, 152, 156, 176] {
            let mut bad = data.clone();
            let old = u32::from_le_bytes(bad[offset..offset + 4].try_into().unwrap());
            put32(&mut bad, offset, old ^ 1);
            assert_eq!(
                DecodedTable::read(&bad, table).err(),
                Some(RecordError::Schema),
                "{name}@{offset}"
            );
        }
        let mut bad = data.clone();
        bad.push(0);
        assert_eq!(
            DecodedTable::read(&bad, table).err(),
            Some(RecordError::Bounds)
        );
        let mut encrypted = data.clone();
        put32(&mut encrypted, 204, 1);
        assert_eq!(
            DecodedTable::read(&encrypted, table).err(),
            Some(RecordError::Unsupported)
        );
    }
}

#[test]
fn build_70374_raw_text_keeps_opaque_bytes_but_checks_identity_and_eof() {
    let name = "SpellMissileMotion";
    let table = CharacterTable::from_catalog_id(WOWDBDEFS_70374_CATALOG_ID, name)
        .unwrap()
        .unwrap();
    let metadata = resolve_for(WOWDBDEFS_70374_CATALOG_ID, name)
        .unwrap()
        .unwrap()
        .metadata();
    let (mut data, _, _) = fixture_70374(metadata, table, false);
    let offset = data.windows(2).position(|b| b == [0xC3, 0xA1]).unwrap();
    data[offset] = 0x97;
    assert_eq!(
        DecodedTable::read(&data, table).err(),
        Some(RecordError::String)
    );
    assert_eq!(
        DecodedTable::validate_catalog_raw_source_bytes(&data, WOWDBDEFS_70374_CATALOG_ID, name),
        Ok(())
    );
    let mut bad = data.clone();
    put32(&mut bad, 152, 0);
    assert_eq!(
        DecodedTable::validate_catalog_raw_source_bytes(&bad, WOWDBDEFS_70374_CATALOG_ID, name),
        Err(RecordError::Schema)
    );
    data.pop();
    assert!(
        DecodedTable::validate_catalog_raw_source_bytes(&data, WOWDBDEFS_70374_CATALOG_ID, name)
            .is_err()
    );
}

#[test]
fn build_70374_exact_sparse_parent_allowlist_is_raw_only() {
    for name in [
        "CollectableSourceEncounterSparse",
        "CollectableSourceQuestSparse",
        "CollectableSourceVendorSparse",
    ] {
        let data = if name == "CollectableSourceVendorSparse" {
            vendor_sparse_parent_fixture()
        } else {
            empty_sparse_parent_fixture(name)
        };
        let table = CharacterTable::from_catalog_id(WOWDBDEFS_70374_CATALOG_ID, name)
            .unwrap()
            .unwrap();
        assert_eq!(
            DecodedTable::read(&data, table).err(),
            Some(RecordError::Unsupported)
        );
        assert_eq!(
            DecodedTable::validate_catalog_raw_source_bytes(
                &data,
                WOWDBDEFS_70374_CATALOG_ID,
                name
            ),
            Ok(())
        );
        assert!(
            DecodedTable::validate_catalog_raw_source_bytes(
                &data[..data.len() - 1],
                WOWDBDEFS_70374_CATALOG_ID,
                name
            )
            .is_err()
        );
        assert_eq!(
            DecodedTable::validate_catalog_raw_source_bytes(
                &data,
                WOWDBDEFS_70374_CATALOG_ID,
                "UnknownSyntheticTable"
            ),
            Err(RecordError::Schema)
        );
    }
}

#[test]
fn build_70374_empty_appended_parent_retains_count_gates() {
    let name = "ExteriorComponentGroup";
    let table = CharacterTable::from_catalog_id(WOWDBDEFS_70374_CATALOG_ID, name)
        .unwrap()
        .unwrap();
    let metadata = resolve_for(WOWDBDEFS_70374_CATALOG_ID, name)
        .unwrap()
        .unwrap()
        .metadata();
    assert_eq!(
        (
            metadata.file_field_count,
            metadata.field_count,
            metadata.parent_index
        ),
        (2, 3, 2)
    );
    let mut data = header(table, metadata.field_count);
    data.resize(data.len() + 4 * metadata.field_count, 0);
    assert!(DecodedTable::read(&data, table).unwrap().rows().is_empty());
    put32(&mut data, 136, 1);
    assert_eq!(
        DecodedTable::read(&data, table).err(),
        Some(RecordError::Schema)
    );
}

fn target_high_volume_numeric_fixture(count: u32) -> Vec<u8> {
    let name = "WMOMinimapTexture";
    let table = CharacterTable::from_catalog_id(WOWDBDEFS_70374_CATALOG_ID, name)
        .unwrap()
        .unwrap();
    let fields = 4;
    let record_size = 16;
    let mut data = header(table, fields);
    put32(&mut data, 136, count);
    put32(&mut data, 144, record_size as u32);
    put32(&mut data, 160, 1);
    put32(&mut data, 164, count);
    put32(&mut data, 188, (fields * 24) as u32);
    put32(&mut data, 200, 1);
    data.resize(244 + 4 * fields + 24 * fields, 0);
    for index in 0..fields {
        let at = 244 + 4 * fields + 24 * index;
        put16(&mut data, at, (index * 32) as u16);
        put32(&mut data, at + 16, 32);
        put32(&mut data, at + 20, 1);
    }
    let offset = data.len() as u32;
    put32(&mut data, 212, offset);
    put32(&mut data, 216, count);
    put32(&mut data, 228, count * 4);
    data.resize(data.len() + record_size * count as usize, 0);
    for id in 1..=count {
        data.extend(id.to_le_bytes());
    }
    data
}

#[test]
fn build_70374_high_volume_exact_limit_remains_explicit_and_bounded() {
    let name = "WMOMinimapTexture";
    let table = CharacterTable::from_catalog_id(WOWDBDEFS_70374_CATALOG_ID, name)
        .unwrap()
        .unwrap();
    let data = target_high_volume_numeric_fixture(160_000);
    assert_eq!(
        DecodedTable::read(&data, table).err(),
        Some(RecordError::Limit)
    );
    let decoded =
        DecodedTable::read_target_high_volume_from_catalog(&data, WOWDBDEFS_70374_CATALOG_ID, name)
            .unwrap()
            .unwrap();
    assert_eq!(decoded.rows().len(), 160_000);
    assert_eq!(decoded.rows()[&160_000].len(), 5);
    drop(decoded);
    assert_eq!(
        DecodedTable::validate_catalog_raw_source_bytes(&data, WOWDBDEFS_70374_CATALOG_ID, name),
        Ok(())
    );
    let over = target_high_volume_numeric_fixture(160_001);
    assert_eq!(
        DecodedTable::read_target_high_volume_from_catalog(&over, WOWDBDEFS_70374_CATALOG_ID, name)
            .err(),
        Some(RecordError::Limit)
    );
    assert_eq!(
        DecodedTable::validate_catalog_raw_source_bytes(&over, WOWDBDEFS_70374_CATALOG_ID, name),
        Err(RecordError::Limit)
    );
}

#[test]
fn build_70374_high_volume_profile_selection_does_not_widen_other_tables() {
    assert_eq!(
        DecodedTable::TARGET_HIGH_VOLUME_70374_PROFILE,
        "wowdbdefs-70374-high-volume-regular-v1"
    );
    for name in [
        "ManifestInterfaceData",
        "ModelFileData",
        "WMOMinimapTexture",
    ] {
        let table = CharacterTable::from_catalog_id(WOWDBDEFS_70374_CATALOG_ID, name)
            .unwrap()
            .unwrap();
        let metadata = resolve_for(WOWDBDEFS_70374_CATALOG_ID, name)
            .unwrap()
            .unwrap()
            .metadata();
        let (data, expected, _) = fixture_70374(metadata, table, false);
        let decoded = DecodedTable::read_target_high_volume_from_catalog(
            &data,
            WOWDBDEFS_70374_CATALOG_ID,
            name,
        )
        .unwrap()
        .unwrap();
        assert_eq!(decoded.rows()[&7], expected);
        let mut bad = data.clone();
        put32(&mut bad, 200, 2);
        assert_eq!(
            DecodedTable::read_target_high_volume_from_catalog(
                &bad,
                WOWDBDEFS_70374_CATALOG_ID,
                name
            )
            .err(),
            Some(RecordError::Unsupported)
        );
        put32(&mut bad, 200, 1);
        put16(&mut bad, 172, 1);
        assert_eq!(
            DecodedTable::read_target_high_volume_from_catalog(
                &bad,
                WOWDBDEFS_70374_CATALOG_ID,
                name
            )
            .err(),
            Some(RecordError::Unsupported)
        );
    }
    assert!(
        DecodedTable::read_target_high_volume_from_catalog(&[], WOWDBDEFS_70374_CATALOG_ID, "Map")
            .unwrap()
            .is_none()
    );
    assert!(
        DecodedTable::read_target_high_volume_from_catalog(
            &[],
            "wowdbdefs-70374-v2",
            "WMOMinimapTexture"
        )
        .unwrap()
        .is_none()
    );
}

// Authored build-70425 catalog-registration cases. Not executed at proposal time;
// synthetic schema checks do not establish client, component or product admission.
#[test]
fn build_70425_catalog_has_distinct_provenance_and_explicit_handles() {
    let source = provenance(WOWDBDEFS_70425_CATALOG_ID).unwrap();
    assert_eq!(source.id, "wowdbdefs-70425-v1");
    assert_eq!(
        source.source_commit,
        "996e53f029c0213b223aecb94d754f6b4b9efb49"
    );
    assert_eq!(
        source.source_blob,
        "f95528066d44e1b70ea6cd124a35a5a63685d384"
    );
    assert_eq!(
        source.source_sha256,
        "f7b752a991d6a0321d07ecb7f06e62cd94694c1594d2fe5ce0cb7b243c71559a"
    );
    assert_eq!(
        source.facts_sha256,
        "2150cbfc6a431931d410c3368e5e7b8de2c66b6b6a798d0052886f948624488a"
    );
    assert_ne!(source, provenance(WOWDBDEFS_70374_CATALOG_ID).unwrap());
    let current = tables_for(WOWDBDEFS_70425_CATALOG_ID).unwrap();
    let previous = tables_for(WOWDBDEFS_70374_CATALOG_ID).unwrap();
    assert_eq!(current.len(), 1161);
    assert_eq!(previous.len(), current.len());
    assert!(current.windows(2).all(|pair| pair[0].name < pair[1].name));
    let mut ids = BTreeSet::new();
    // These separately pinned targets currently have equal schema facts. This
    // assertion neither aliases their handles nor transfers acquisition evidence.
    for (old, new) in previous.iter().zip(current) {
        assert!(ids.insert(new.file_data_id));
        assert_eq!(old.name, new.name);
        assert_eq!(old.file_data_id, new.file_data_id);
        assert_eq!(old.table_hash, new.table_hash);
        assert_eq!(old.layout_hash, new.layout_hash);
        assert_eq!(old.other_layout_hashes, new.other_layout_hashes);
        assert_eq!(old.index, new.index);
        assert_eq!(old.parent_index, new.parent_index);
        assert_eq!(old.field_count, new.field_count);
        assert_eq!(old.file_field_count, new.file_field_count);
        assert_eq!(old.fields, new.fields);
        assert_eq!(old.blockers, new.blockers);
        assert_eq!(old.decode_fields, new.decode_fields);
        assert!(new.shape_compatible());
        let old_handle = resolve_for(WOWDBDEFS_70374_CATALOG_ID, new.name)
            .unwrap()
            .unwrap();
        let new_handle = resolve_for(WOWDBDEFS_70425_CATALOG_ID, new.name)
            .unwrap()
            .unwrap();
        assert_ne!(old_handle, new_handle);
        assert_eq!(new_handle.catalog_id(), WOWDBDEFS_70425_CATALOG_ID);
        assert!(std::ptr::eq(new_handle.metadata(), new));
        assert_eq!(new_handle.schema().hash, new.table_hash);
        assert_eq!(new_handle.schema().layout, new.layout_hash);
    }
}

#[test]
fn build_70425_explicit_registration_preserves_default_reviewed_lookup() {
    assert_eq!(tables().len(), 1126);
    assert!(std::ptr::eq(tables(), tables_for(CATALOG_ID).unwrap()));
    assert_eq!(CharacterTable::named("TactKey"), None);
    assert_eq!(CharacterTable::named("TactKeyLookup"), None);
    for name in ["Map", "TactKey", "TactKeyLookup"] {
        let default = CharacterTable::named(name)
            .unwrap_or_else(|| CharacterTable::Catalog(resolve(name).unwrap().unwrap()));
        assert_eq!(CharacterTable::from_catalog(name).unwrap(), Some(default));
        assert_eq!(
            CharacterTable::from_catalog_id(CATALOG_ID, name).unwrap(),
            Some(default)
        );
        let explicit = CharacterTable::from_catalog_id(WOWDBDEFS_70425_CATALOG_ID, name)
            .unwrap()
            .unwrap();
        assert_ne!(explicit, default);
    }
    assert!(supports(WOWDBDEFS_70425_CATALOG_ID));
    assert_eq!(
        resolve_for(WOWDBDEFS_70425_CATALOG_ID, "UnknownSyntheticTable"),
        Ok(None)
    );
    assert!(!supports("wowdbdefs-70425-v2"));
    assert_eq!(provenance("wowdbdefs-70425-v2"), None);
    assert_eq!(
        CharacterTable::from_catalog_id("wowdbdefs-70425-v2", "Map")
            .unwrap_err()
            .name,
        "unknown_catalog"
    );
}

fn fixture_70425(
    metadata: &TableMetadata,
    table: CharacterTable,
) -> (Vec<u8>, Vec<Vec<Value>>, bool) {
    // Use this target's declarations and normalized primitive widths. No older
    // target lookup supplies the physical fixture or its schema identity.
    let mut fields = metadata.fields.to_vec();
    for (field, kind) in fields.iter_mut().zip(metadata.decode_fields.unwrap()) {
        field.kind = match kind {
            Kind::Unsigned(8, _) | Kind::Signed(8, _) => FieldType::Byte,
            Kind::Unsigned(16, _) | Kind::Signed(16, _) => FieldType::Short,
            Kind::Unsigned(64, _) | Kind::Signed(64, _) => FieldType::Long,
            _ => field.kind,
        };
    }
    fixture_for_fields(metadata, &fields, table, false)
}

#[test]
fn build_70425_selected_schema_identity_field_and_eof_gates_remain_strict() {
    for name in ["Map", "GameObjectDisplayInfo", "TactKey", "TactKeyLookup"] {
        let table = CharacterTable::from_catalog_id(WOWDBDEFS_70425_CATALOG_ID, name)
            .unwrap()
            .unwrap();
        let metadata = resolve_for(WOWDBDEFS_70425_CATALOG_ID, name)
            .unwrap()
            .unwrap()
            .metadata();
        let (data, expected, _) = fixture_70425(metadata, table);
        assert_eq!(
            DecodedTable::read(&data, table).unwrap().rows()[&7],
            expected
        );
        for offset in [140, 152, 156, 176] {
            let mut bad = data.clone();
            let old = u32::from_le_bytes(bad[offset..offset + 4].try_into().unwrap());
            put32(&mut bad, offset, old ^ 1);
            assert_eq!(
                DecodedTable::read(&bad, table).err(),
                Some(RecordError::Schema),
                "{name}@{offset}"
            );
        }
        let mut trailing = data.clone();
        trailing.push(0);
        assert_eq!(
            DecodedTable::read(&trailing, table).err(),
            Some(RecordError::Bounds)
        );
        let mut encrypted = data;
        put32(&mut encrypted, 204, 1);
        assert_eq!(
            DecodedTable::read(&encrypted, table).err(),
            Some(RecordError::Unsupported)
        );
    }
}

#[test]
fn build_70425_registration_does_not_inherit_sparse_parent_raw_allowlist() {
    for name in [
        "CollectableSourceEncounterSparse",
        "CollectableSourceQuestSparse",
        "CollectableSourceVendorSparse",
    ] {
        // These authored inputs have the same separately pinned schema facts;
        // their equality is covered by the independent registration case.
        let data = if name == "CollectableSourceVendorSparse" {
            vendor_sparse_parent_fixture()
        } else {
            empty_sparse_parent_fixture(name)
        };
        let table = CharacterTable::from_catalog_id(WOWDBDEFS_70425_CATALOG_ID, name)
            .unwrap()
            .unwrap();
        assert_eq!(
            DecodedTable::read(&data, table).err(),
            Some(RecordError::Unsupported)
        );
        assert_eq!(
            DecodedTable::validate_catalog_raw_source_bytes(
                &data,
                WOWDBDEFS_70425_CATALOG_ID,
                name
            ),
            Err(RecordError::Unsupported)
        );
    }
}

#[test]
fn build_70425_registration_keeps_ordinary_raw_limits_and_closed_high_volume_profile() {
    for name in [
        "ManifestInterfaceData",
        "ModelFileData",
        "WMOMinimapTexture",
    ] {
        assert!(
            DecodedTable::read_target_high_volume_from_catalog(
                &[],
                WOWDBDEFS_70425_CATALOG_ID,
                name
            )
            .unwrap()
            .is_none()
        );
    }
    // Exact same-schema authored numeric input crosses only the ordinary 100k
    // direct-row ceiling. Catalog registration must not grant the 160k profile.
    let data = target_high_volume_numeric_fixture(100_001);
    let name = "WMOMinimapTexture";
    let table = CharacterTable::from_catalog_id(WOWDBDEFS_70425_CATALOG_ID, name)
        .unwrap()
        .unwrap();
    assert_eq!(
        DecodedTable::read(&data, table).err(),
        Some(RecordError::Limit)
    );
    assert_eq!(
        DecodedTable::validate_catalog_raw_source_bytes(&data, WOWDBDEFS_70425_CATALOG_ID, name),
        Err(RecordError::Limit)
    );
}
