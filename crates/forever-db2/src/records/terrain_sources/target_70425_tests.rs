// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-17: complete synthetic70425 liquid sources, without client IO.
use super::*;

fn table70425(name: &str) -> CharacterTable {
    CharacterTable::from_catalog_id(crate::catalog::WOWDBDEFS_70425_CATALOG_ID, name)
        .unwrap()
        .unwrap()
}

fn spec70425(name: &str, ids: &[u32]) -> Spec {
    // Reuse the authored legacy wire builder only after checking every physical
    // decode kind and the authoritative index/parent against the fixed target.
    let legacy = table(name).schema();
    let target = table70425(name).schema();
    assert_eq!(legacy.fields, target.fields);
    assert_eq!(legacy.index, target.index);
    assert_eq!(legacy.parent, target.parent);
    source_spec(name, ids)
}

fn read70425(name: &str, bytes: &[u8], text_limit: usize) -> Result<Collected, RecordError> {
    let role = match name {
        "LiquidMaterial" => Role::LiquidMaterial,
        "LiquidType" => Role::LiquidType,
        _ => panic!("closed liquid fixture name"),
    };
    terrain::read_liquids_70425(bytes, role, text_limit)
}

#[test]
fn liquid70425_catalog_identity_and_physical_bindings_are_explicit() {
    for (name, id, hash, layout, count) in [
        ("LiquidMaterial", 1132538, 0x8C379329, 0x98E5D7AA, 2),
        ("LiquidType", 1371380, 0x6613BED3, 0xD1ECEEC9, 21),
    ] {
        let target = table70425(name);
        let CharacterTable::Catalog(catalog) = target else {
            panic!("expected fixed70425 catalog");
        };
        assert_eq!(
            catalog.catalog_id(),
            crate::catalog::WOWDBDEFS_70425_CATALOG_ID
        );
        assert_ne!(target, table(name));
        assert_eq!(target.file_id(), id);
        let schema = target.schema();
        assert_eq!(schema.hash, Some(hash));
        assert_eq!(
            (schema.layout, schema.index, schema.parent),
            (layout, -1, -1)
        );
        assert_eq!(schema.fields.len(), count);
        let spec = spec70425(name, &[7]);
        let bytes = assemble(target, &spec);
        let header = crate::Header::parse(bytes[..HEADER].try_into().unwrap()).unwrap();
        assert!(target.matches(&header));
        assert_eq!(schema.fields, table(name).schema().fields);
    }
}

#[test]
fn liquid70425_types_preserve_physical_rows_copies_and_numeric_extremes() {
    let mut spec = spec70425("LiquidType", &[0, 7, 7]);
    for (row, (sound, material)) in [(255, 255), (2, 0), (3, 128)].into_iter().enumerate() {
        set_byte(&mut spec, row, 3, sound);
        set_byte(&mut spec, row, 14, material);
    }
    spec.max_id = 0;
    spec.copies = vec![(9, 7), (7, 0), (10, 9), (11, 99), (0, 0), (88, 88)];
    let bytes = assemble(table70425("LiquidType"), &spec);
    let source = terrain::read_liquid_types_70425(&bytes).unwrap();
    assert_eq!(
        source
            .direct_records()
            .iter()
            .map(|r| r.id)
            .collect::<Vec<_>>(),
        vec![0, 7, 7]
    );
    assert_eq!(
        source
            .direct_records()
            .iter()
            .map(|r| (r.fields.sound_bank, r.fields.material_id))
            .collect::<Vec<_>>(),
        vec![(255, 255), (2, 0), (3, 128)]
    );
    assert_eq!(
        source.copies(),
        spec.copies
            .iter()
            .map(|&(new_id, source_id)| RecordCopy { new_id, source_id })
            .collect::<Vec<_>>()
    );
    let rows = source.materialize().unwrap();
    assert_eq!(rows[&7].sound_bank, 255);
    assert_eq!((rows[&9].sound_bank, rows[&10].material_id), (3, 128));
    for id in [11, 99, 88] {
        assert_eq!(rows[&id], terrain::LiquidTypeFields::default());
    }
}

#[test]
fn liquid70425_materials_preserve_signed_bits_and_copy_overwrites() {
    let mut spec = spec70425("LiquidMaterial", &[0, 7, 7]);
    for (row, (flags, lvf)) in [(0x80000002, 0x80), (0, 0), (u32::MAX, 0xff)]
        .into_iter()
        .enumerate()
    {
        set_word(&mut spec, row, 0, flags);
        set_byte(&mut spec, row, 1, lvf);
    }
    spec.copies = vec![(9, 7), (7, 0), (10, 9), (11, 99), (0, 0), (88, 88)];
    let bytes = assemble(table70425("LiquidMaterial"), &spec);
    let source = terrain::read_liquid_materials_70425(&bytes).unwrap();
    assert_eq!(
        source
            .direct_records()
            .iter()
            .map(|r| r.id)
            .collect::<Vec<_>>(),
        vec![0, 7, 7]
    );
    assert_eq!(source.direct_records()[0].fields.lvf, -128);
    assert_eq!(source.direct_records()[2].fields.lvf, -1);
    let rows = source.materialize().unwrap();
    assert_eq!((rows[&7].flags_bits, rows[&7].lvf), (0x80000002, -128));
    assert_eq!((rows[&9].flags_bits, rows[&10].lvf), (u32::MAX, -1));
    for id in [11, 99, 88] {
        assert_eq!(rows[&id], terrain::LiquidMaterialFields::default());
    }
}

#[test]
fn liquid70425_repaired_ids_keep_checked_ordinal_arithmetic() {
    let mut spec = spec70425("LiquidMaterial", &[0, 50, 49, 55, u32::MAX]);
    spec.min_id = 50;
    spec.max_id = 0;
    let bytes = assemble(table70425("LiquidMaterial"), &spec);
    assert_eq!(
        terrain::read_liquid_materials_70425(&bytes)
            .unwrap()
            .direct_records()
            .iter()
            .map(|r| r.id)
            .collect::<Vec<_>>(),
        vec![50, 51, 52, 55, u32::MAX]
    );
    spec.min_id = u32::MAX;
    assert_eq!(
        terrain::read_liquid_materials_70425(&assemble(table70425("LiquidMaterial"), &spec)).err(),
        Some(RecordError::Id)
    );
}

#[test]
fn liquid70425_complete_empty_sources_keep_primitive_and_column_profiles() {
    for name in ["LiquidMaterial", "LiquidType"] {
        let target = table70425(name);
        let spec = spec70425(name, &[]);
        let regular = assemble(target, &spec);
        let empty = build_zero_section_header(target, 0);
        let mut columns = empty.clone();
        let count = target.schema().fields.len();
        put32(&mut columns, 188, (count * 24) as u32);
        let start = HEADER + SECTION + count * 4;
        columns.extend_from_slice(&regular[start..start + count * 24]);
        for bytes in [regular, empty, columns] {
            match read70425(name, &bytes, 0).unwrap() {
                Collected::LiquidMaterial(rows) => {
                    assert!(rows.direct_records().is_empty());
                    assert!(rows.copies().is_empty());
                    assert!(rows.materialize().unwrap().is_empty());
                }
                Collected::LiquidType(rows) => {
                    assert!(rows.direct_records().is_empty());
                    assert!(rows.copies().is_empty());
                    assert!(rows.materialize().unwrap().is_empty());
                }
                _ => panic!("closed liquid projection"),
            }
        }
    }
}

#[test]
fn liquid70425_hash_layout_and_field_counts_are_required() {
    for name in ["LiquidMaterial", "LiquidType"] {
        let valid = assemble(table70425(name), &spec70425(name, &[7]));
        for at in [152, 156, 140, 176] {
            let mut bytes = valid.clone();
            put32(&mut bytes, at, 0);
            assert_eq!(read70425(name, &bytes, 64).err(), Some(RecordError::Schema));
        }
    }
}

#[test]
fn liquid70425_metadata_index_remains_distinct_from_header_index() {
    for name in ["LiquidMaterial", "LiquidType"] {
        let mut bytes = assemble(table70425(name), &spec70425(name, &[7]));
        put16(&mut bytes, 174, 123);
        assert!(read70425(name, &bytes, 0).is_ok());
        // The catalog still declares external IDs regardless of the header word.
        put32(&mut bytes, 228, 0);
        assert_eq!(read70425(name, &bytes, 0).err(), Some(RecordError::Id));
    }
}

#[test]
fn liquid70425_unsupported_sparse_encrypted_multiple_and_parent_shapes_reject() {
    for name in ["LiquidMaterial", "LiquidType"] {
        let valid = assemble(table70425(name), &spec70425(name, &[7]));
        for at in [172, 204] {
            let mut bytes = valid.clone();
            put32(&mut bytes, at, 1);
            assert_eq!(
                read70425(name, &bytes, 64).err(),
                Some(RecordError::Unsupported)
            );
        }
        let mut multi = spec70425(name, &[7]);
        multi.extra_sections = 1;
        assert_eq!(
            read70425(name, &assemble(table70425(name), &multi), 64).err(),
            Some(RecordError::Unsupported)
        );
        let mut bytes = valid.clone();
        put32(&mut bytes, 184, 1);
        assert_eq!(read70425(name, &bytes, 64).err(), Some(RecordError::Schema));
        // Header::parse rejects counts above one before record-profile checks,
        // exactly as in the existing legacy GameObject source test.
        put32(&mut bytes, 184, 2);
        assert_eq!(
            read70425(name, &bytes, 64).err(),
            Some(RecordError::Metadata)
        );
    }
}

#[test]
fn liquid70425_complete_record_id_copy_and_tail_extents_are_required() {
    for name in ["LiquidMaterial", "LiquidType"] {
        let mut spec = spec70425(name, &[7, 9]);
        spec.copies = vec![(11, 7), (12, 11)];
        let valid = assemble(table70425(name), &spec);
        for length in 0..valid.len() {
            assert!(read70425(name, &valid[..length], 64).is_err());
        }
        let mut extra = valid.clone();
        extra.push(0);
        assert_eq!(read70425(name, &extra, 64).err(), Some(RecordError::Bounds));
        for at in [212, 216, 220, 232] {
            let mut bytes = valid.clone();
            put32(&mut bytes, at, 1);
            assert!(read70425(name, &bytes, 64).is_err(), "{name} offset {at}");
        }
    }
}

#[test]
fn liquid70425_unselected_late_texture_string_must_validate() {
    let mut spec = spec70425("LiquidType", &[7, 8]);
    let offset = field_offset(&spec, 1) + 5 * 32;
    write_word(&mut spec.records[1], offset, u32::MAX);
    let bytes = assemble(table70425("LiquidType"), &spec);
    assert_eq!(
        terrain::read_liquid_types_70425(&bytes).err(),
        Some(RecordError::String)
    );
}

#[test]
fn liquid70425_unselected_text_utf8_and_cumulative_budget_are_checked() {
    let mut spec = spec70425("LiquidType", &[7, 7]);
    set_text(&mut spec, 0, 0, "é");
    set_text(&mut spec, 1, 0, "abcd");
    let bytes = assemble(table70425("LiquidType"), &spec);
    let Collected::LiquidType(rows) = read70425("LiquidType", &bytes, 6).unwrap() else {
        panic!("liquid type projection");
    };
    assert_eq!(rows.decoded_utf8_bytes(), 6);
    assert_eq!(rows.materialize().unwrap().len(), 1);
    assert_eq!(
        read70425("LiquidType", &bytes, 5).err(),
        Some(RecordError::Limit)
    );
    let at = spec.strings.len() - 2;
    spec.strings[at] = 0xff;
    let bytes = assemble(table70425("LiquidType"), &spec);
    assert_eq!(
        terrain::read_liquid_types_70425(&bytes).err(),
        Some(RecordError::String)
    );
}

#[test]
fn liquid70425_unselected_column_compression_is_not_ignored() {
    let mut spec = spec70425("LiquidType", &[7]);
    spec.cols[1].compression = 6;
    let bytes = assemble(table70425("LiquidType"), &spec);
    assert_eq!(
        terrain::read_liquid_types_70425(&bytes).err(),
        Some(RecordError::Column)
    );
}

#[test]
fn liquid70425_file_direct_copy_and_scalar_limits_are_unchanged() {
    for name in ["LiquidMaterial", "LiquidType"] {
        let valid = assemble(table70425(name), &spec70425(name, &[7]));
        let mut oversized = valid.clone();
        oversized.resize(16 * 1024 * 1024 + 1, 0);
        assert_eq!(
            read70425(name, &oversized, 64).err(),
            Some(RecordError::Limit)
        );
        for (at, value) in [(136, 100001), (240, 100001)] {
            let mut bytes = valid.clone();
            put32(&mut bytes, at, value);
            assert_eq!(read70425(name, &bytes, 64).err(), Some(RecordError::Limit));
        }
    }
    let target = table70425("LiquidType");
    let scalars = target
        .schema()
        .fields
        .iter()
        .map(|kind| kind.count())
        .sum::<usize>();
    assert_eq!(scalars, 76);
    let mut bytes = assemble(target, &spec70425("LiquidType", &[7]));
    put32(&mut bytes, 136, 13158);
    assert_eq!(
        terrain::read_liquid_types_70425(&bytes).err(),
        Some(RecordError::Limit)
    );
}

#[test]
fn liquid70425_copy_limit_is_independent_of_projected_fields() {
    let mut spec = spec70425("LiquidMaterial", &[7]);
    spec.copies = vec![(u32::MAX, 7); 100000];
    let bytes = assemble(table70425("LiquidMaterial"), &spec);
    let source = terrain::read_liquid_materials_70425(&bytes).unwrap();
    assert_eq!(source.direct_records().len(), 1);
    assert_eq!(source.copies().len(), 100000);
}

#[test]
fn liquid70425_replay_limit_charges_both_missing_source_and_destination() {
    let mut spec = spec70425("LiquidMaterial", &[]);
    spec.copies = vec![(7, 8)];
    let bytes = assemble(table70425("LiquidMaterial"), &spec);
    let source = terrain::read_liquid_materials_70425(&bytes).unwrap();
    assert_eq!(
        terrain::materialize_liquid(source, 1).err(),
        Some(RecordError::Limit)
    );
    let source = terrain::read_liquid_materials_70425(&bytes).unwrap();
    assert_eq!(terrain::materialize_liquid(source, 2).unwrap().len(), 2);
}

#[test]
fn liquid70425_hash_gate_does_not_strengthen_legacy_readers() {
    for name in ["LiquidMaterial", "LiquidType"] {
        let mut bytes = assemble(table70425(name), &spec70425(name, &[7]));
        put32(&mut bytes, 152, 0);
        assert_eq!(read70425(name, &bytes, 64).err(), Some(RecordError::Schema));
        let role = if name == "LiquidMaterial" {
            Role::LiquidMaterial
        } else {
            Role::LiquidType
        };
        assert!(terrain::read(&bytes, role, 64).is_ok());
    }
}

#[test]
fn liquid70425_closed_private_selector_rejects_other_component_roles() {
    let bytes = assemble(
        table70425("LiquidMaterial"),
        &spec70425("LiquidMaterial", &[7]),
    );
    for role in [
        Role::Map,
        Role::VMap,
        Role::Navigation,
        Role::GameObject,
        Role::LiquidObject,
    ] {
        assert_eq!(
            terrain::read_liquids_70425(&bytes, role, 64).err(),
            Some(RecordError::Schema)
        );
    }
}
