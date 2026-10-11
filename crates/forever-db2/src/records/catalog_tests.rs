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
    let fields = &metadata.fields[..metadata.file_field_count];
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
