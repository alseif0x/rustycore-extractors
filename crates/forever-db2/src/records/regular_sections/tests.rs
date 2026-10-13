// SPDX-License-Identifier: GPL-3.0-or-later
//! Authored in-memory fixtures only. These tests are not native or real-client
//! acceptance and never read files, execute programs or access the network.
use super::*;

#[derive(Default)]
struct Part {
    rows: Vec<(u32, u32)>, // stored ID and repeated numeric payload
    copies: Vec<(u32, u32)>,
}

#[derive(Default)]
struct TextPart {
    rows: Vec<(u32, Option<usize>)>, // stored ID and section-local string offset
    strings: Vec<u8>,
    copies: Vec<(u32, u32)>,
}

fn put32(data: &mut [u8], at: usize, value: u32) {
    data[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

fn fixture(table: CharacterTable, parts: &[Part]) -> Vec<u8> {
    let schema = table.schema();
    let fields = schema.fields.len();
    let record_size: usize = schema
        .fields
        .iter()
        .map(|kind| kind.bits() as usize / 8 * kind.count())
        .sum();
    let mut data = vec![0; HEADER_SIZE + parts.len() * SECTION_SIZE + fields * 28];
    data[..4].copy_from_slice(b"WDC5");
    put32(&mut data, 4, 5);
    put32(
        &mut data,
        136,
        parts.iter().map(|part| part.rows.len() as u32).sum(),
    );
    put32(&mut data, 140, fields as u32);
    put32(&mut data, 144, record_size as u32);
    put32(&mut data, 152, schema.hash.unwrap_or(0));
    put32(&mut data, 156, schema.layout);
    put32(&mut data, 160, 10);
    put32(&mut data, 164, 100);
    data[174..176].copy_from_slice(&schema.index.to_le_bytes());
    put32(&mut data, 176, fields as u32);
    put32(&mut data, 188, fields as u32 * 24);
    put32(&mut data, 200, parts.len() as u32);
    let mut bit = 0u16;
    for (field, kind) in schema.fields.iter().enumerate() {
        let at = column_at(parts.len(), fields, field);
        data[at..at + 2].copy_from_slice(&bit.to_le_bytes());
        bit += kind.bits() as u16 * kind.count() as u16;
    }
    for (index, part) in parts.iter().enumerate() {
        let at = HEADER_SIZE + index * SECTION_SIZE;
        let file_offset = data.len() as u32;
        put32(&mut data, at + 8, file_offset);
        put32(&mut data, at + 12, part.rows.len() as u32);
        put32(&mut data, at + 24, part.rows.len() as u32 * 4);
        put32(&mut data, at + 36, part.copies.len() as u32);
        for &(_, value) in &part.rows {
            for kind in schema.fields {
                for _ in 0..kind.count() {
                    let mut word = [0u8; 8];
                    word[..4].copy_from_slice(&value.to_le_bytes());
                    data.extend_from_slice(&word[..kind.bits() as usize / 8]);
                }
            }
        }
        for &(id, _) in &part.rows {
            data.extend_from_slice(&id.to_le_bytes());
        }
        for &(new, source) in &part.copies {
            data.extend_from_slice(&new.to_le_bytes());
            data.extend_from_slice(&source.to_le_bytes());
        }
    }
    data
}

fn text_fixture(parts: &[TextPart]) -> Vec<u8> {
    text_fixture_for(CharacterTable::AnimKitBoneSet, parts)
}

fn text_fixture_for(table: CharacterTable, parts: &[TextPart]) -> Vec<u8> {
    let schema = table.schema();
    assert_eq!(schema.fields[0], Kind::String);
    let fields = schema.fields.len();
    let record_size: usize = schema
        .fields
        .iter()
        .map(|kind| kind.bits() as usize / 8 * kind.count())
        .sum();
    let record_count: usize = parts.iter().map(|part| part.rows.len()).sum();
    let string_size: usize = parts.iter().map(|part| part.strings.len()).sum();
    let mut data = vec![0; HEADER_SIZE + parts.len() * SECTION_SIZE + fields * 28];
    data[..4].copy_from_slice(b"WDC5");
    put32(&mut data, 4, 5);
    put32(&mut data, 136, record_count as u32);
    put32(&mut data, 140, fields as u32);
    put32(&mut data, 144, record_size as u32);
    put32(&mut data, 148, string_size as u32);
    put32(&mut data, 152, schema.hash.unwrap_or(0));
    put32(&mut data, 156, schema.layout);
    put32(&mut data, 160, 10);
    put32(&mut data, 164, 100);
    data[174..176].copy_from_slice(&schema.index.to_le_bytes());
    put32(&mut data, 176, fields as u32);
    put32(&mut data, 188, fields as u32 * 24);
    put32(&mut data, 200, parts.len() as u32);
    let mut bit = 0u16;
    for (field, kind) in schema.fields.iter().enumerate() {
        let at = column_at(parts.len(), fields, field);
        data[at..at + 2].copy_from_slice(&bit.to_le_bytes());
        bit += kind.bits() as u16 * kind.count() as u16;
    }

    let mut record_ordinal = 0usize;
    let mut prior_strings = 0usize;
    for (index, part) in parts.iter().enumerate() {
        let section_at = HEADER_SIZE + index * SECTION_SIZE;
        let records_at = data.len();
        put32(&mut data, section_at + 8, records_at as u32);
        put32(&mut data, section_at + 12, part.rows.len() as u32);
        put32(&mut data, section_at + 16, part.strings.len() as u32);
        put32(&mut data, section_at + 24, part.rows.len() as u32 * 4);
        put32(&mut data, section_at + 36, part.copies.len() as u32);

        for &(_, local_string) in &part.rows {
            let mut raw = vec![0; record_size];
            if let Some(local_string) = local_string {
                let logical_target = record_count * record_size + prior_strings + local_string;
                let logical_record = record_ordinal * record_size;
                let displacement = logical_target - logical_record;
                raw[..4].copy_from_slice(&(displacement as u32).to_le_bytes());
            }
            data.extend_from_slice(&raw);
            record_ordinal += 1;
        }
        data.extend_from_slice(&part.strings);
        for &(id, _) in &part.rows {
            data.extend_from_slice(&id.to_le_bytes());
        }
        for &(new, source) in &part.copies {
            data.extend_from_slice(&new.to_le_bytes());
            data.extend_from_slice(&source.to_le_bytes());
        }
        prior_strings += part.strings.len();
    }
    data
}

fn column_at(sections: usize, fields: usize, field: usize) -> usize {
    HEADER_SIZE + sections * SECTION_SIZE + fields * 4 + field * 24
}

fn parts() -> [Part; 2] {
    [
        Part {
            rows: vec![(20, 7)],
            copies: vec![],
        },
        Part {
            rows: vec![(30, 9)],
            copies: vec![],
        },
    ]
}

fn decode(data: &[u8]) -> Result<DecodedTable, RecordError> {
    DecodedTable::read_regular_sections(data, CharacterTable::SpellDuration, 16)
}

fn decode_text(data: &[u8]) -> Result<DecodedTable, RecordError> {
    DecodedTable::read_regular_sections_with_text(data, CharacterTable::AnimKitBoneSet, 16)
}

fn error(data: &[u8]) -> RecordError {
    match decode(data) {
        Ok(_) => panic!("fixture must reject"),
        Err(error) => error,
    }
}

#[test]
fn explicit_profile_and_section_ceiling() {
    assert_eq!(
        DecodedTable::REGULAR_SECTIONS_PROFILE,
        "forever-db2-regular-sections-numeric-v1"
    );
    assert_eq!(DecodedTable::MAX_REGULAR_SECTIONS, 16);
    let data = fixture(CharacterTable::SpellDuration, &parts());
    for limit in [0, 1, 17, u32::MAX] {
        assert!(matches!(
            DecodedTable::read_regular_sections(&data, CharacterTable::SpellDuration, limit),
            Err(RecordError::Limit)
        ));
    }
    assert!(DecodedTable::read_regular_sections(&data, CharacterTable::SpellDuration, 2).is_ok());
}

#[test]
fn text_profile_is_separate_and_resolves_section_ordered_strings_and_copies() {
    assert_eq!(
        DecodedTable::REGULAR_SECTIONS_TEXT_PROFILE,
        "forever-db2-regular-sections-text-v1"
    );
    let data = text_fixture(&[
        TextPart {
            rows: vec![(20, Some(0))],
            strings: b"first\0".to_vec(),
            copies: vec![(40, 30)],
        },
        TextPart {
            rows: vec![(30, Some(0))],
            strings: b"second\0".to_vec(),
            copies: vec![(50, 40)],
        },
    ]);
    assert!(matches!(
        DecodedTable::read_regular_sections(&data, CharacterTable::AnimKitBoneSet, 16),
        Err(RecordError::Unsupported)
    ));
    let decoded = decode_text(&data).unwrap();
    for id in [30, 40, 50] {
        assert_eq!(decoded.rows()[&id][0][0], Value::Text("second".into()));
    }
    assert_eq!(decoded.rows()[&20][0][0], Value::Text("first".into()));
}

#[test]
fn text_profile_accepts_null_strings_and_charges_utf8_and_copy_bytes() {
    let data = text_fixture(&[
        TextPart {
            rows: vec![(20, None), (21, Some(0))],
            strings: "é\0".as_bytes().to_vec(),
            copies: vec![(40, 21)],
        },
        TextPart::default(),
    ]);
    let table = CharacterTable::AnimKitBoneSet;
    assert_eq!(
        DecodedTable::read_regular_sections_with_text(&data, table, 2)
            .unwrap()
            .rows()[&20][0][0],
        Value::Text(String::new())
    );
    assert!(read_with_text_limits(&data, table, 2, 100, 100, 100, 4).is_ok());
    assert!(matches!(
        read_with_text_limits(&data, table, 2, 100, 100, 100, 3),
        Err(RecordError::Limit)
    ));
}

#[test]
fn text_profile_rejects_bad_string_offsets_terminators_and_aggregate_sizes() {
    let fixture = || {
        text_fixture(&[
            TextPart {
                rows: vec![(20, Some(0))],
                strings: b"alpha\0".to_vec(),
                copies: vec![],
            },
            TextPart {
                rows: vec![(30, Some(0))],
                strings: b"beta\0".to_vec(),
                copies: vec![],
            },
        ])
    };
    let mut bad_pointer = fixture();
    let first_records = u32::from_le_bytes(
        bad_pointer[HEADER_SIZE + 8..HEADER_SIZE + 12]
            .try_into()
            .unwrap(),
    ) as usize;
    put32(&mut bad_pointer, first_records, u32::MAX);
    assert!(matches!(
        decode_text(&bad_pointer),
        Err(RecordError::String)
    ));

    let mut missing_terminator = fixture();
    let second_section = HEADER_SIZE + SECTION_SIZE;
    let second_records = u32::from_le_bytes(
        missing_terminator[second_section + 8..second_section + 12]
            .try_into()
            .unwrap(),
    ) as usize;
    let record_size: usize = CharacterTable::AnimKitBoneSet
        .schema()
        .fields
        .iter()
        .map(|kind| kind.bits() as usize / 8 * kind.count())
        .sum();
    let second_string_size = u32::from_le_bytes(
        missing_terminator[second_section + 16..second_section + 20]
            .try_into()
            .unwrap(),
    ) as usize;
    let second_strings = second_records + record_size;
    let last_second_string = second_strings + second_string_size - 1;
    missing_terminator[last_second_string] = b'x';
    assert!(matches!(
        decode_text(&missing_terminator),
        Err(RecordError::String)
    ));

    let mut wrong_aggregate = fixture();
    let total = u32::from_le_bytes(wrong_aggregate[148..152].try_into().unwrap());
    put32(&mut wrong_aggregate, 148, total - 1);
    assert!(matches!(
        decode_text(&wrong_aggregate),
        Err(RecordError::Bounds)
    ));
}

#[test]
fn text_profile_validates_string_compression_even_without_records() {
    let data = text_fixture(&[TextPart::default(), TextPart::default()]);
    assert!(decode_text(&data).unwrap().rows().is_empty());
    let field = column_at(2, CharacterTable::AnimKitBoneSet.schema().fields.len(), 0);
    for mode in 1..=5 {
        let mut compressed = data.clone();
        put32(&mut compressed, field + 8, mode);
        assert!(matches!(decode_text(&compressed), Err(RecordError::Column)));
    }
}

fn catalog_ids() -> [&'static str; 3] {
    [
        crate::catalog::CATALOG_ID,
        "wowdbdefs-70205-v1",
        "wowdbdefs-70338-v1",
    ]
}

#[test]
fn catalog_profile_decodes_numeric_rows_and_global_copies_for_each_catalog() {
    assert_eq!(
        DecodedTable::REGULAR_SECTIONS_METADATA_PROFILE,
        "forever-db2-regular-sections-metadata-v1"
    );
    for catalog in catalog_ids() {
        let name = "GameObjectDisplayInfo";
        let table = CharacterTable::from_catalog_id(catalog, name)
            .unwrap()
            .unwrap();
        assert!(matches!(table, CharacterTable::Catalog(_)));
        let mut sections = parts();
        sections[0].copies = vec![(40, 30)];
        sections[1].copies = vec![(50, 40)];
        let data = fixture(table, &sections);
        let decoded =
            DecodedTable::read_regular_sections_from_catalog(&data, catalog, name, 2).unwrap();
        assert_eq!(
            decoded.rows().keys().copied().collect::<Vec<_>>(),
            [20, 30, 40, 50]
        );
        assert_eq!(decoded.rows()[&20][0], vec![Value::FloatBits(7); 6]);
        for id in [30, 40, 50] {
            assert_eq!(decoded.rows()[&id][0], vec![Value::FloatBits(9); 6]);
            assert_eq!(decoded.rows()[&id][1], [Value::Signed(9)]);
        }
        // The separately versioned numeric v1 never gains catalog admission.
        assert!(matches!(
            DecodedTable::read_regular_sections(&data, table, 2),
            Err(RecordError::Unsupported)
        ));
    }
}

#[test]
fn catalog_profile_decodes_text_rows_and_global_copies_for_each_catalog() {
    for catalog in catalog_ids() {
        let name = "AnimKitBoneSet";
        let table = CharacterTable::from_catalog_id(catalog, name)
            .unwrap()
            .unwrap();
        let data = text_fixture_for(
            table,
            &[
                TextPart {
                    rows: vec![(0, Some(0))],
                    strings: b"first\0".to_vec(),
                    copies: vec![(40, 11)],
                },
                TextPart {
                    rows: vec![(0, Some(0))],
                    strings: "segundo é\0".as_bytes().to_vec(),
                    copies: vec![(50, 40)],
                },
            ],
        );
        let decoded =
            DecodedTable::read_regular_sections_from_catalog(&data, catalog, name, 2).unwrap();
        assert_eq!(decoded.rows()[&10][0], [Value::Text("first".into())]);
        for id in [11, 40, 50] {
            assert_eq!(decoded.rows()[&id][0], [Value::Text("segundo é".into())]);
        }
        assert!(matches!(
            DecodedTable::read(&data, table),
            Err(RecordError::Unsupported)
        ));
    }
}

#[test]
fn catalog_profile_rejects_compressed_text_in_empty_tables_for_each_catalog() {
    for catalog in catalog_ids() {
        let name = "AnimKitBoneSet";
        let table = CharacterTable::from_catalog_id(catalog, name)
            .unwrap()
            .unwrap();
        let data = text_fixture_for(table, &[TextPart::default(), TextPart::default()]);
        assert!(
            DecodedTable::read_regular_sections_from_catalog(&data, catalog, name, 2)
                .unwrap()
                .rows()
                .is_empty()
        );
        let column = column_at(2, table.schema().fields.len(), 0);
        for mode in 1..=5 {
            let mut compressed = data.clone();
            put32(&mut compressed, column + 8, mode);
            assert!(matches!(
                DecodedTable::read_regular_sections_from_catalog(&compressed, catalog, name, 2),
                Err(RecordError::Column)
            ));
        }
    }
}

#[test]
fn catalog_profile_preserves_target_table_hash_layout_and_field_count_gates() {
    for catalog in ["wowdbdefs-70205-v1", "wowdbdefs-70338-v1"] {
        let name = "GameObjectDisplayInfo";
        let table = CharacterTable::from_catalog_id(catalog, name)
            .unwrap()
            .unwrap();
        assert!(table.schema().hash.is_some());
        let data = fixture(table, &parts());
        for offset in [140, 152, 156, 176] {
            let mut wrong_identity = data.clone();
            let value = u32_at(&data, offset).unwrap();
            put32(&mut wrong_identity, offset, value ^ 1);
            assert!(matches!(
                DecodedTable::read_regular_sections_from_catalog(&wrong_identity, catalog, name, 2),
                Err(RecordError::Schema)
            ));
        }
        assert!(matches!(
            DecodedTable::read_regular_sections_from_catalog(&data, catalog, "SpellDuration", 2),
            Err(RecordError::Schema)
        ));
    }
}

#[test]
fn catalog_profile_rejects_unknown_selection_inline_ids_and_parents() {
    let data = fixture(CharacterTable::SpellDuration, &parts());
    for (catalog, name) in [
        ("unknown-catalog", "SpellDuration"),
        ("wowdbdefs-70338-v1", "UnknownTable"),
    ] {
        assert!(matches!(
            DecodedTable::read_regular_sections_from_catalog(&data, catalog, name, 2),
            Err(RecordError::Schema)
        ));
    }
    for catalog in catalog_ids() {
        for name in ["Curve", "ChrClassesXPowerTypes"] {
            let table = CharacterTable::from_catalog_id(catalog, name)
                .unwrap()
                .unwrap();
            let data = fixture(table, &[Part::default(), Part::default()]);
            assert!(matches!(
                DecodedTable::read_regular_sections_from_catalog(&data, catalog, name, 2),
                Err(RecordError::Unsupported)
            ));
        }
    }
}

#[test]
fn two_numeric_sections_preserve_all_values() {
    let decoded = decode(&fixture(CharacterTable::SpellDuration, &parts())).unwrap();
    assert_eq!(decoded.rows().len(), 2);
    assert_eq!(decoded.rows()[&20], vec![vec![Value::Signed(7)]; 3]);
    assert_eq!(decoded.rows()[&30], vec![vec![Value::Signed(9)]; 3]);
}

#[test]
fn float_and_signed_schemas_are_explicitly_admitted() {
    for table in [CharacterTable::SpellRadius, CharacterTable::SpellCastTimes] {
        let data = fixture(table, &parts());
        let decoded = DecodedTable::read_regular_sections(&data, table, 2).unwrap();
        let expected = if table == CharacterTable::SpellRadius {
            Value::FloatBits(7)
        } else {
            Value::Signed(7)
        };
        assert_eq!(decoded.rows()[&20][0][0], expected);
    }
}

#[test]
fn empty_middle_section_keeps_global_id_repair_ordinal() {
    let sections = [
        Part {
            rows: vec![(0, 7)],
            copies: vec![],
        },
        Part::default(),
        Part {
            rows: vec![(10, 9)],
            copies: vec![],
        },
    ];
    let decoded = decode(&fixture(CharacterTable::SpellDuration, &sections)).unwrap();
    assert_eq!(decoded.rows().keys().copied().collect::<Vec<_>>(), [10, 11]);
    assert_eq!(decoded.rows()[&11][0][0], Value::Signed(9));
}

#[test]
fn wholly_empty_multiple_sections_are_complete() {
    let data = fixture(
        CharacterTable::SpellDuration,
        &[Part::default(), Part::default()],
    );
    assert!(decode(&data).unwrap().rows().is_empty());
}

#[test]
fn copies_see_later_directs_then_global_chains_and_overwrites() {
    let sections = [
        Part {
            rows: vec![(20, 7)],
            copies: vec![(40, 30), (20, 40)],
        },
        Part {
            rows: vec![(30, 9)],
            copies: vec![(50, 20)],
        },
    ];
    let decoded = decode(&fixture(CharacterTable::SpellDuration, &sections)).unwrap();
    for id in [20, 30, 40, 50] {
        assert_eq!(decoded.rows()[&id][0][0], Value::Signed(9));
    }
}

#[test]
fn skipped_sources_are_not_retried_and_do_not_consume_values() {
    let sections = [
        Part {
            rows: vec![(20, 7)],
            copies: vec![(40, 50), (60, 0), (70, 99)],
        },
        Part {
            rows: vec![(30, 9)],
            copies: vec![(50, 30), (80, 101)],
        },
    ];
    let data = fixture(CharacterTable::SpellDuration, &sections);
    let decoded = read_with_limits(&data, CharacterTable::SpellDuration, 16, 2, 5, 9).unwrap();
    assert_eq!(
        decoded.rows().keys().copied().collect::<Vec<_>>(),
        [20, 30, 50]
    );
}

#[test]
fn self_copy_rejects_the_whole_result() {
    let mut sections = parts();
    sections[1].copies = vec![(40, 20), (30, 30)];
    assert_eq!(
        error(&fixture(CharacterTable::SpellDuration, &sections)),
        RecordError::Unsupported
    );
}

#[test]
fn duplicate_direct_ids_across_sections_reject() {
    let mut sections = parts();
    sections[1].rows[0].0 = 20;
    assert_eq!(
        error(&fixture(CharacterTable::SpellDuration, &sections)),
        RecordError::Id
    );
}

#[test]
fn global_repair_collision_rejects() {
    let sections = [
        Part {
            rows: vec![(11, 7)],
            copies: vec![],
        },
        Part {
            rows: vec![(0, 9)],
            copies: vec![],
        },
    ];
    assert_eq!(
        error(&fixture(CharacterTable::SpellDuration, &sections)),
        RecordError::Id
    );
}

#[test]
fn direct_and_even_skipped_copy_destinations_respect_max_id() {
    let mut sections = parts();
    sections[1].rows[0].0 = 101;
    assert_eq!(
        error(&fixture(CharacterTable::SpellDuration, &sections)),
        RecordError::Id
    );
    sections = parts();
    sections[0].copies = vec![(101, 0)];
    assert_eq!(
        error(&fixture(CharacterTable::SpellDuration, &sections)),
        RecordError::Id
    );
}

#[test]
fn extent_overflow_and_global_repair_overflow_reject() {
    let mut sections = parts();
    sections[1].copies = vec![(u32::MAX, 0)];
    let mut data = fixture(CharacterTable::SpellDuration, &sections);
    put32(&mut data, 164, u32::MAX);
    assert_eq!(error(&data), RecordError::Id);
    sections = parts();
    sections[0].rows[0].0 = 0;
    sections[1].rows[0].0 = 0;
    let mut data = fixture(CharacterTable::SpellDuration, &sections);
    put32(&mut data, 160, u32::MAX);
    put32(&mut data, 164, u32::MAX);
    assert_eq!(error(&data), RecordError::Id);
}

#[test]
fn cumulative_direct_copy_and_scalar_limits_are_not_per_section() {
    let mut sections = parts();
    sections[0].copies = vec![(40, 20)];
    sections[1].copies = vec![(50, 30)];
    let data = fixture(CharacterTable::SpellDuration, &sections);
    for (records, copies, values) in [(1, 2, 12), (2, 1, 12), (2, 2, 11), (2, 2, 5)] {
        assert!(matches!(
            read_with_limits(
                &data,
                CharacterTable::SpellDuration,
                16,
                records,
                copies,
                values
            ),
            Err(RecordError::Limit)
        ));
    }
    assert!(read_with_limits(&data, CharacterTable::SpellDuration, 16, 2, 2, 12).is_ok());
}

#[test]
fn overwrite_copies_do_not_add_retained_scalar_slots() {
    let mut sections = parts();
    sections[0].copies = vec![(30, 20)];
    sections[1].copies = vec![(20, 30)];
    let data = fixture(CharacterTable::SpellDuration, &sections);
    assert!(read_with_limits(&data, CharacterTable::SpellDuration, 16, 2, 2, 6).is_ok());
}

#[test]
fn legacy_entry_points_still_reject_multi_section_files() {
    let data = fixture(CharacterTable::SpellDuration, &parts());
    assert!(matches!(
        DecodedTable::read(&data, CharacterTable::SpellDuration),
        Err(RecordError::Unsupported)
    ));
    assert!(matches!(
        DecodedTable::read_with_direct_u32_values(&data, CharacterTable::SpellDuration, 0),
        Err(RecordError::Schema)
    ));
    let data = fixture(CharacterTable::ClassUIDisplay, &parts());
    assert!(matches!(
        DecodedTable::read_with_direct_u32_values(&data, CharacterTable::ClassUIDisplay, 1),
        Err(RecordError::Unsupported)
    ));
}

#[test]
fn single_and_zero_sections_are_outside_explicit_profile() {
    for count in [0, 1] {
        let sections: Vec<_> = (0..count).map(|_| Part::default()).collect();
        assert_eq!(
            error(&fixture(CharacterTable::SpellDuration, &sections)),
            RecordError::Unsupported
        );
    }
}

#[test]
fn section_count_limit_can_only_decrease() {
    let sections: Vec<_> = (0..16).map(|_| Part::default()).collect();
    let data = fixture(CharacterTable::SpellDuration, &sections);
    assert!(decode(&data).is_ok());
    assert!(matches!(
        DecodedTable::read_regular_sections(&data, CharacterTable::SpellDuration, 15),
        Err(RecordError::Limit)
    ));
    let sections: Vec<_> = (0..17).map(|_| Part::default()).collect();
    assert_eq!(
        error(&fixture(CharacterTable::SpellDuration, &sections)),
        RecordError::Limit
    );
}

#[test]
fn schema_layout_field_counts_and_known_hash_gates_remain() {
    for at in [140, 156, 176] {
        let mut data = fixture(CharacterTable::SpellDuration, &parts());
        put32(&mut data, at, 0);
        assert_eq!(error(&data), RecordError::Schema);
    }
    // A reviewed known-hash table still fails its identity gate before the
    // numeric-only restriction. The explicit path does not weaken matches().
    let mut data = fixture(CharacterTable::Classes, &parts());
    put32(&mut data, 152, 0);
    assert!(matches!(
        DecodedTable::read_regular_sections(&data, CharacterTable::Classes, 16),
        Err(RecordError::Schema)
    ));
}

#[test]
fn inline_text_and_parent_schemas_reject_even_when_empty() {
    for table in [
        CharacterTable::Curve,
        CharacterTable::Classes,
        CharacterTable::Races,
        CharacterTable::ClassXPowerTypes,
    ] {
        let data = fixture(table, &[Part::default(), Part::default()]);
        assert!(matches!(
            DecodedTable::read_regular_sections(&data, table, 16),
            Err(RecordError::Unsupported)
        ));
    }
}

#[test]
fn catalog_schemas_are_not_silently_admitted() {
    let table = CharacterTable::from_catalog("GameObjectDisplayInfo")
        .unwrap()
        .unwrap();
    assert!(matches!(table, CharacterTable::Catalog(_)));
    let data = fixture(table, &[Part::default(), Part::default()]);
    assert!(matches!(
        DecodedTable::read_regular_sections(&data, table, 16),
        Err(RecordError::Unsupported)
    ));
}

#[test]
fn sparse_parent_and_text_declarations_reject() {
    for at in [148, 184] {
        let mut data = fixture(CharacterTable::SpellDuration, &parts());
        put32(&mut data, at, 1);
        assert_eq!(error(&data), RecordError::Unsupported);
    }
    let mut data = fixture(CharacterTable::SpellDuration, &parts());
    data[172] = 1;
    assert_eq!(error(&data), RecordError::Unsupported);
}

#[test]
fn encrypted_catalog_text_and_parent_sections_reject() {
    for section in 0..2 {
        for field in [0, 16, 20, 28, 32] {
            let mut data = fixture(CharacterTable::SpellDuration, &parts());
            put32(&mut data, HEADER_SIZE + section * SECTION_SIZE + field, 1);
            assert_eq!(error(&data), RecordError::Unsupported);
        }
    }
}

#[test]
fn section_offsets_must_be_exact_with_no_overlap_or_gap() {
    for section in 0..2 {
        for delta in [-1i64, 1] {
            let mut data = fixture(CharacterTable::SpellDuration, &parts());
            let at = HEADER_SIZE + section * SECTION_SIZE + 8;
            let offset = u32_at(&data, at).unwrap();
            put32(&mut data, at, (i64::from(offset) + delta) as u32);
            assert_eq!(error(&data), RecordError::Bounds);
        }
    }
}

#[test]
fn aggregate_header_count_and_section_id_lengths_must_match() {
    let mut data = fixture(CharacterTable::SpellDuration, &parts());
    put32(&mut data, 136, 3);
    assert_eq!(error(&data), RecordError::Bounds);
    for section in 0..2 {
        let mut data = fixture(CharacterTable::SpellDuration, &parts());
        put32(&mut data, HEADER_SIZE + section * SECTION_SIZE + 24, 0);
        assert_eq!(error(&data), RecordError::Id);
    }
}

#[test]
fn all_truncations_and_trailing_bytes_reject() {
    let data = fixture(CharacterTable::SpellDuration, &parts());
    for end in 0..data.len() {
        assert!(decode(&data[..end]).is_err());
    }
    let mut tail = data;
    tail.push(0);
    assert_eq!(error(&tail), RecordError::Bounds);
}

#[test]
fn oversized_file_and_huge_declarations_reject_before_body_use() {
    assert_eq!(error(&vec![0; MAX_BYTES + 1]), RecordError::Limit);
    for (at, expected) in [
        (136, RecordError::Limit),
        (144, RecordError::Bounds),
        (HEADER_SIZE + 12, RecordError::Limit),
        (HEADER_SIZE + 36, RecordError::Limit),
    ] {
        let mut data = fixture(CharacterTable::SpellDuration, &parts());
        put32(&mut data, at, u32::MAX);
        assert_eq!(error(&data), expected);
    }
}

#[test]
fn metadata_size_record_size_and_id_range_must_be_valid() {
    for (at, value, expected) in [
        (188, 0, RecordError::Column),
        (144, 0, RecordError::Bounds),
        (160, 101, RecordError::Bounds),
    ] {
        let mut data = fixture(CharacterTable::SpellDuration, &parts());
        put32(&mut data, at, value);
        assert_eq!(error(&data), expected);
    }
}

#[test]
fn shared_compression_modes_are_used_across_sections() {
    // Existing immediate, common and signed-immediate column decoding remains
    // shared. Common overrides are absent, so the declared default is used.
    for mode in [1, 2, 5] {
        let mut data = fixture(CharacterTable::SpellDuration, &parts());
        let at = column_at(2, 3, 0);
        put32(&mut data, at + 8, mode);
        put32(&mut data, at + 12, if mode == 2 { 42 } else { 0 });
        put32(&mut data, at + 16, 8);
        let decoded = decode(&data).unwrap();
        assert_eq!(
            decoded.rows()[&20][0][0],
            Value::Signed(if mode == 2 { 42 } else { 7 })
        );
        assert_eq!(
            decoded.rows()[&30][0][0],
            Value::Signed(if mode == 2 { 42 } else { 9 })
        );
    }
}

#[test]
fn unsupported_and_out_of_record_columns_reject() {
    for (field, value, expected) in [
        (8, 6, RecordError::Column),
        (4, 1, RecordError::Column),
        (0, 65528, RecordError::Bounds),
    ] {
        let mut data = fixture(CharacterTable::SpellDuration, &parts());
        let at = column_at(2, 3, 0);
        if field == 0 {
            data[at..at + 2].copy_from_slice(&(value as u16).to_le_bytes());
        } else {
            put32(&mut data, at + field, value);
        }
        assert_eq!(error(&data), expected);
    }
}

#[test]
fn numeric_arrays_and_wide_values_keep_their_shape() {
    let data = fixture(CharacterTable::AnimationData, &parts());
    let decoded =
        DecodedTable::read_regular_sections(&data, CharacterTable::AnimationData, 2).unwrap();
    assert_eq!(decoded.rows()[&20][3], [Value::Signed(7), Value::Signed(7)]);
    let data = fixture(CharacterTable::BoneWindModifiers, &parts());
    let decoded =
        DecodedTable::read_regular_sections(&data, CharacterTable::BoneWindModifiers, 2).unwrap();
    assert_eq!(decoded.rows()[&30][0], vec![Value::FloatBits(9); 3]);
    let mut data = fixture(CharacterTable::BankTab, &parts());
    let record = u32_at(&data, HEADER_SIZE + 8).unwrap() as usize;
    data[record..record + 8].copy_from_slice(&0xfedc_ba98_7654_3210u64.to_le_bytes());
    let decoded = DecodedTable::read_regular_sections(&data, CharacterTable::BankTab, 2).unwrap();
    assert_eq!(
        decoded.rows()[&20][0][0],
        Value::Unsigned64(0xfedc_ba98_7654_3210)
    );
}

#[test]
fn compressed_wide_and_non_pallet_array_columns_remain_unsupported() {
    for table in [CharacterTable::BankTab, CharacterTable::BoneWindModifiers] {
        let mut data = fixture(table, &parts());
        let at = column_at(2, table.schema().fields.len(), 0);
        put32(&mut data, at + 8, 2);
        assert!(matches!(
            DecodedTable::read_regular_sections(&data, table, 2),
            Err(RecordError::Column)
        ));
    }
}

fn add_pallet(
    data: &mut Vec<u8>,
    table: CharacterTable,
    field: usize,
    count: usize,
    values: &[u32],
) {
    let fields = table.schema().fields.len();
    let at = column_at(2, fields, field);
    put32(data, at + 4, values.len() as u32 * 4);
    put32(data, at + 8, if count == 1 { 3 } else { 4 });
    put32(data, at + 12, 1); // bit 1 differs between 7 and 9
    put32(data, at + 16, 1);
    put32(data, at + 20, count as u32);
    put32(data, 196, values.len() as u32 * 4);
    for section in 0..2 {
        let at = HEADER_SIZE + section * SECTION_SIZE + 8;
        let old = u32_at(data, at).unwrap();
        put32(data, at, old + values.len() as u32 * 4);
    }
    let insert = HEADER_SIZE + 2 * SECTION_SIZE + fields * 28;
    data.splice(
        insert..insert,
        values.iter().flat_map(|value| value.to_le_bytes()),
    );
}

#[test]
fn shared_scalar_and_array_pallets_resolve_globally() {
    let mut data = fixture(CharacterTable::SpellDuration, &parts());
    add_pallet(&mut data, CharacterTable::SpellDuration, 0, 1, &[21, 42]);
    let decoded = decode(&data).unwrap();
    assert_eq!(decoded.rows()[&20][0][0], Value::Signed(42));
    assert_eq!(decoded.rows()[&30][0][0], Value::Signed(21));
    let table = CharacterTable::BoneWindModifiers;
    let mut data = fixture(table, &parts());
    add_pallet(&mut data, table, 0, 3, &[1, 2, 3, 4, 5, 6]);
    let decoded = DecodedTable::read_regular_sections(&data, table, 2).unwrap();
    assert_eq!(
        decoded.rows()[&20][0],
        [
            Value::FloatBits(4),
            Value::FloatBits(5),
            Value::FloatBits(6)
        ]
    );
    assert_eq!(
        decoded.rows()[&30][0],
        [
            Value::FloatBits(1),
            Value::FloatBits(2),
            Value::FloatBits(3)
        ]
    );
}
