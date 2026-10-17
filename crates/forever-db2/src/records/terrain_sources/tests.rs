// SPDX-License-Identifier: GPL-3.0-or-later
// Original synthetic source-order and consumer-copy fixtures, 2026-10-11.
use super::*;
use crate::terrain_sources::{self as terrain, Collected, RecordCopy, Role};

#[path = "target_70425_tests.rs"]
mod target_70425_tests;

fn table(name: &str) -> CharacterTable {
    CharacterTable::from_catalog(name).unwrap().unwrap()
}
fn source_spec(name: &str, ids: &[u32]) -> Spec {
    let mut s = idtable_spec(table(name), ids.len());
    s.min_id = 0;
    s.ids = ids.to_vec();
    if name == "Map" {
        for row in 0..ids.len() {
            // Non-null pointers to NUL: empty std::string is well-defined.
            set_text(&mut s, row, 0, "");
            set_text(&mut s, row, 1, "");
        }
    }
    s
}
fn set_word(s: &mut Spec, row: usize, field: usize, value: u32) {
    let offset = field_offset(s, field);
    write_word(&mut s.records[row], offset, value);
}
fn set_byte(s: &mut Spec, row: usize, field: usize, value: u8) {
    let offset = field_offset(s, field);
    write_byte(&mut s.records[row], offset, value);
}
fn set_short(s: &mut Spec, row: usize, field: usize, value: i16) {
    let offset = usize::from(field_offset(s, field)) / 8;
    s.records[row][offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}
fn set_text(s: &mut Spec, row: usize, field: usize, text: &str) {
    let offset = field_offset(s, field);
    let relative =
        (s.records.len() - row) * record_size(&s.cols) + s.strings.len() - offset as usize / 8;
    write_word(&mut s.records[row], offset, relative as u32);
    s.strings.extend_from_slice(text.as_bytes());
    s.strings.push(0);
}

#[test]
fn closed_identities_and_physical_bindings_match_independent_facts() {
    for (name, id, layout, fields) in [
        ("Map", 1349477, 0xD43AFAC3, 26),
        ("LiquidType", 1371380, 0xD1ECEEC9, 21),
        ("LiquidMaterial", 1132538, 0x98E5D7AA, 2),
        ("LiquidObject", 1308058, 0xCB0D39E8, 5),
    ] {
        let t = table(name);
        assert_eq!(t.file_id(), id);
        assert_eq!(
            (
                t.schema().layout,
                t.schema().index,
                t.schema().parent,
                t.schema().fields.len()
            ),
            (layout, -1, -1, fields)
        );
    }
    assert_eq!(table("Map").schema().fields[21], Kind::Signed(32, 1));
    let s = table("LiquidType").schema();
    assert_eq!(s.fields[1], Kind::StringArray(6));
    assert_eq!(s.fields[3], Kind::Unsigned(8, 1));
    assert_eq!(s.fields[14], Kind::Unsigned(8, 1));
    assert_eq!(
        table("LiquidMaterial").schema().fields,
        &[Kind::Signed(32, 1), Kind::Signed(8, 1)]
    );
}

#[test]
fn map_keeps_direct_order_duplicates_and_source_only_copy_index() {
    let mut s = source_spec("Map", &[0, 7, 9, 7, 11]);
    for (row, value) in [10, 20, 30, 40, 0].into_iter().enumerate() {
        set_word(&mut s, row, 21, value);
    }
    set_text(&mut s, 3, 0, "dir\té");
    set_text(&mut s, 3, 1, "name\\\n");
    s.max_id = 1; // GetRecordCopy consumers do not use AutoProduceData's extent.
    s.copies = vec![
        (9, 7),
        (100, 9),
        (101, 100),
        (0, 0),
        (7, 7),
        (7, 0),
        (u32::MAX, 7),
        (12, 11),
    ];
    let bytes = assemble(table("Map"), &s);
    let source = terrain::read_map(&bytes).unwrap();
    assert_eq!(
        source
            .direct_records()
            .iter()
            .map(|r| r.id)
            .collect::<Vec<_>>(),
        vec![0, 7, 9, 7, 11]
    );
    assert_eq!(source.copies().len(), 8);
    let out = source.materialize().unwrap();
    assert_eq!(
        out.iter()
            .map(|r| (r.id, r.fields.wdt_file_data_id))
            .collect::<Vec<_>>(),
        vec![
            (0, 10),
            (7, 20),
            (9, 30),
            (7, 40),
            (9, 40),
            (100, 30),
            (0, 10),
            (7, 40),
            (7, 10),
            (u32::MAX, 40)
        ]
    );
    assert_eq!(out.last().unwrap().fields.directory, "dir\té");
    assert!(matches!(
        DecodedTable::read(&bytes, table("Map")),
        Err(RecordError::Id)
    ));
}

#[test]
fn map_negative_wdt_and_source_zero_survive_but_zero_wdt_filters_after_copies() {
    let mut s = source_spec("Map", &[0, 4]);
    set_word(&mut s, 0, 21, u32::MAX);
    s.copies = vec![(5, 4), (6, 0), (4, 0)];
    let out = terrain::read_map(&assemble(table("Map"), &s))
        .unwrap()
        .materialize()
        .unwrap();
    assert_eq!(
        out.iter()
            .map(|r| (r.id, r.fields.wdt_file_data_id))
            .collect::<Vec<_>>(),
        vec![(0, -1), (6, -1), (4, -1)]
    );
}

#[test]
fn map_selected_null_strings_fail_even_before_zero_wdt_filter() {
    for field in [0, 1] {
        for wdt in [0, 1] {
            let mut s = source_spec("Map", &[7, 8]);
            set_word(&mut s, 1, 21, wdt);
            set_word(&mut s, 1, field, 0);
            let bytes = assemble(table("Map"), &s);
            assert!(matches!(
                terrain::read_map(&bytes),
                Err(RecordError::String)
            ));
            // The generic decoder retains its historical null-to-empty policy.
            assert!(DecodedTable::read(&bytes, table("Map")).is_ok());
        }
    }
}

#[test]
fn map_pointed_empty_strings_are_valid_and_unselected_null_stays_valid() {
    let mut s = source_spec("Map", &[7]);
    set_word(&mut s, 0, 21, 1);
    let source = terrain::read_map(&assemble(table("Map"), &s)).unwrap();
    assert_eq!(source.decoded_utf8_bytes(), 0);
    let rows = source.materialize().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].fields.name, "");
    assert_eq!(rows[0].fields.directory, "");
}

#[test]
fn liquid_types_insert_missing_source_zero_self_chains_and_overwrites() {
    let mut s = source_spec("LiquidType", &[0, 7, 7]);
    for (i, sound) in [255, 2, 3].into_iter().enumerate() {
        set_byte(&mut s, i, 3, sound);
        set_byte(&mut s, i, 14, 255);
    }
    s.max_id = 0;
    s.copies = vec![
        (9, 7),
        (7, 0),
        (10, 9),
        (11, 99),
        (100, 99),
        (0, 0),
        (88, 88),
        (u32::MAX, 10),
    ];
    let out = terrain::read_liquid_types(&assemble(table("LiquidType"), &s))
        .unwrap()
        .materialize()
        .unwrap();
    assert_eq!(out[&7].sound_bank, 255);
    for id in [9, 10, u32::MAX] {
        assert_eq!(out[&id].sound_bank, 3);
    }
    for id in [11, 99, 100, 88] {
        assert_eq!(out[&id], terrain::LiquidTypeFields::default());
    }
    assert_eq!(out.len(), 9);
}

#[test]
fn material_flags_bits_and_signed_lvf_are_reinterpreted_exactly() {
    let mut s = source_spec("LiquidMaterial", &[1, 2]);
    set_word(&mut s, 0, 0, 0x80000002);
    set_byte(&mut s, 0, 1, 0x80);
    set_word(&mut s, 1, 0, u32::MAX);
    set_byte(&mut s, 1, 1, 0xff);
    s.copies = vec![(3, 1), (4, 2), (5, 100)];
    let out = terrain::read_liquid_materials(&assemble(table("LiquidMaterial"), &s))
        .unwrap()
        .materialize()
        .unwrap();
    assert_eq!((out[&3].flags_bits, out[&3].lvf), (0x80000002, -128));
    assert_eq!((out[&4].flags_bits, out[&4].lvf), (u32::MAX, -1));
    assert_eq!(out[&100], terrain::LiquidMaterialFields::default());
}

#[test]
fn repair_overflow_and_generic_self_copy_policies_remain_separate() {
    let mut s = source_spec("LiquidMaterial", &[0, 0]);
    s.min_id = 50;
    s.max_id = 0;
    let bytes = assemble(table("LiquidMaterial"), &s);
    assert_eq!(
        terrain::read_liquid_materials(&bytes)
            .unwrap()
            .direct_records()
            .iter()
            .map(|r| r.id)
            .collect::<Vec<_>>(),
        vec![50, 51]
    );
    s.min_id = u32::MAX;
    assert!(matches!(
        terrain::read_liquid_materials(&assemble(table("LiquidMaterial"), &s)),
        Err(RecordError::Id)
    ));
    let mut s = source_spec("LiquidMaterial", &[7]);
    s.copies = vec![(7, 7)];
    let bytes = assemble(table("LiquidMaterial"), &s);
    assert!(
        terrain::read_liquid_materials(&bytes)
            .unwrap()
            .materialize()
            .is_ok()
    );
    assert!(matches!(
        DecodedTable::read(&bytes, table("LiquidMaterial")),
        Err(RecordError::Unsupported)
    ));
}

#[test]
fn complete_identity_variant_and_tail_checks_apply_before_snapshot() {
    for name in ["Map", "LiquidType", "LiquidMaterial", "LiquidObject"] {
        let role = match name {
            "Map" => Role::Map,
            "LiquidType" => Role::LiquidType,
            "LiquidObject" => Role::LiquidObject,
            _ => Role::LiquidMaterial,
        };
        let s = source_spec(name, &[7]);
        let valid = assemble(table(name), &s);
        for (at, value, error) in [
            (156, 0, RecordError::Schema),
            (140, 1, RecordError::Schema),
            (176, 1, RecordError::Schema),
            (184, 1, RecordError::Schema),
            (172, 1, RecordError::Unsupported),
            (204, 1, RecordError::Unsupported),
            (200, 2, RecordError::Unsupported),
            (240, 100001, RecordError::Limit),
        ] {
            let mut bytes = valid.clone();
            put32(&mut bytes, at, value);
            assert!(
                matches!(terrain::read(&bytes,role,64),Err(e) if e==error),
                "{name} offset {at}"
            );
        }
        for bytes in [
            &valid[..valid.len() - 1],
            &[valid.as_slice(), &[0]].concat(),
        ] {
            assert!(matches!(
                terrain::read(bytes, role, 64),
                Err(RecordError::Bounds)
            ));
        }
        let empty = build_zero_section_header(table(name), 0);
        assert!(terrain::read(&empty, role, 64).is_ok());
    }
}

#[test]
fn liquid_object_preserves_signed_source_bits_and_native_copy_replay() {
    let mut s = source_spec("LiquidObject", &[1, 2, 1]);
    set_short(&mut s, 0, 2, -1);
    set_short(&mut s, 1, 2, i16::MIN);
    set_short(&mut s, 2, 2, -2);
    // Ignored float fields retain arbitrary bits, including NaN and infinity.
    set_word(&mut s, 0, 0, 0x7fc0_1234);
    set_word(&mut s, 0, 1, 0x7f80_0000);
    s.copies = vec![(3, 1), (4, 3), (1, 2), (5, 99), (0, 0), (7, 7)];
    let rows = terrain::read_liquid_objects(&assemble(table("LiquidObject"), &s)).unwrap();
    assert_eq!(rows.direct_records()[0].fields.liquid_type_id, 65535);
    assert_eq!(rows.direct_records()[1].fields.liquid_type_id, 32768);
    assert_eq!(rows.copies().len(), 6);
    let materialized = rows.materialize().unwrap();
    for (id, value) in [
        (1, 32768),
        (2, 32768),
        (3, 65534),
        (4, 65534),
        (5, 0),
        (99, 0),
        (0, 0),
        (7, 0),
    ] {
        assert_eq!(materialized[&id].liquid_type_id, value);
    }
    assert_eq!(materialized.len(), 8);
}

#[test]
fn liquid_object_empty_and_complete_tail_policy() {
    let t = table("LiquidObject");
    assert_eq!(t.schema().fields[2], Kind::Signed(16, 1));
    let empty = assemble(t, &source_spec("LiquidObject", &[]));
    assert!(
        terrain::read_liquid_objects(&empty)
            .unwrap()
            .materialize()
            .unwrap()
            .is_empty()
    );
    for length in [0, 4, empty.len() - 1] {
        assert!(terrain::read_liquid_objects(&empty[..length]).is_err());
    }
}

#[test]
fn unselected_text_arrays_and_late_strings_still_validate() {
    let mut s = source_spec("LiquidType", &[7, 8]);
    // Last Texture element is unselected but must validate before any return.
    let offset = field_offset(&s, 1) + 5 * 32;
    write_word(&mut s.records[1], offset, u32::MAX);
    assert!(matches!(
        terrain::read_liquid_types(&assemble(table("LiquidType"), &s)),
        Err(RecordError::String)
    ));
    let mut s = source_spec("Map", &[7]);
    set_text(&mut s, 0, 2, "x"); // unselected description
    let bad_text = s.strings.len() - 2;
    s.strings[bad_text] = 0xff;
    assert!(matches!(
        terrain::read_map(&assemble(table("Map"), &s)),
        Err(RecordError::String)
    ));
}

#[test]
fn map_text_budget_charges_unused_utf8_and_copy_overwrites_without_refunds() {
    let mut s = source_spec("Map", &[7]);
    set_word(&mut s, 0, 21, 0);
    set_text(&mut s, 0, 0, "é");
    set_text(&mut s, 0, 1, "a");
    set_text(&mut s, 0, 2, "ignored");
    s.copies = vec![(7, 7), (8, 7)];
    let bytes = assemble(table("Map"), &s);
    for (limit, ok) in [(16, true), (15, false)] {
        let Collected::Map(rows) = terrain::read(&bytes, Role::Map, limit).unwrap() else {
            panic!()
        };
        assert_eq!(rows.decoded_utf8_bytes(), 10);
        assert_eq!(rows.materialize().is_ok(), ok);
    }
    assert!(matches!(
        terrain::read(&bytes, Role::Map, 9),
        Err(RecordError::Limit)
    ));
}

#[test]
fn small_replay_caps_charge_appends_and_both_missing_liquid_keys() {
    let mut s = source_spec("Map", &[7]);
    set_word(&mut s, 0, 21, 1);
    s.copies = vec![(7, 7)];
    assert!(matches!(
        terrain::read_map(&assemble(table("Map"), &s))
            .unwrap()
            .materialize_with_limit(1),
        Err(RecordError::Limit)
    ));
    let mut s = source_spec("LiquidMaterial", &[]);
    s.copies = vec![(7, 8)];
    let bytes = assemble(table("LiquidMaterial"), &s);
    assert!(matches!(
        terrain::materialize_liquid(terrain::read_liquid_materials(&bytes).unwrap(), 1),
        Err(RecordError::Limit)
    ));
    assert_eq!(
        terrain::materialize_liquid(terrain::read_liquid_materials(&bytes).unwrap(), 2)
            .unwrap()
            .len(),
        2
    );
}

// Authored 2026-10-11: independent vmapexport.cpp ReadMapTable projection
// fixtures. No parent graph is traversed here; WDT-zero ancestors stay present.
#[test]
fn vmap_parent_physical_bindings_and_signed_fallback_boundaries() {
    let schema = table("Map").schema();
    assert_eq!(schema.fields[13], Kind::Signed(16, 1));
    assert_eq!(schema.fields[14], Kind::Signed(16, 1));
    assert_eq!(schema.fields[21], Kind::Signed(32, 1));
    let cases = [
        (-1, -1, -1),
        (-1, 0, 0),
        (-1, 7, 7),
        (0, 7, 0),
        (7, 9, 7),
        (i16::MAX, i16::MIN, i16::MAX),
        (i16::MIN, i16::MAX, i16::MAX),
        (-2, i16::MIN, i16::MIN),
    ];
    let ids: Vec<_> = (1..=cases.len() as u32).collect();
    let mut s = source_spec("Map", &ids);
    for (row, (parent, cosmetic, _)) in cases.into_iter().enumerate() {
        set_short(&mut s, row, 13, parent);
        set_short(&mut s, row, 14, cosmetic);
    }
    let rows = terrain::read_vmap_maps(&assemble(table("Map"), &s))
        .unwrap()
        .materialize()
        .unwrap();
    assert_eq!(rows.len(), cases.len());
    for (row, (_, _, expected)) in rows.iter().zip(cases) {
        assert_eq!(row.fields.parent_map_id, expected);
        assert_eq!(row.fields.wdt_file_data_id, 0);
    }
}

#[test]
fn vmap_preserves_physical_order_text_and_all_wdt_bits() {
    let mut s = source_spec("Map", &[10, 11, 12, 13]);
    for (row, value) in [0, i32::MAX, i32::MIN, -1].into_iter().enumerate() {
        set_word(&mut s, row, 21, value as u32);
        set_short(&mut s, row, 13, -1);
        set_short(&mut s, row, 14, -1);
        set_text(&mut s, row, 0, "Dír\\logical");
        set_text(&mut s, row, 1, "Nombre\té\n");
    }
    let rows = terrain::read_vmap_maps(&assemble(table("Map"), &s))
        .unwrap()
        .materialize()
        .unwrap();
    assert_eq!(
        rows.iter()
            .map(|row| (row.id, row.fields.wdt_file_data_id))
            .collect::<Vec<_>>(),
        vec![(10, 0), (11, i32::MAX), (12, i32::MIN), (13, -1)]
    );
    assert!(
        rows.iter()
            .all(|row| row.fields.directory == "Dír\\logical" && row.fields.name == "Nombre\té\n")
    );
}

#[test]
fn vmap_ordered_copies_use_only_last_direct_source_and_never_replace_rows() {
    let mut s = source_spec("Map", &[0, 7, 9, 7, 11]);
    for (row, (wdt, parent)) in [(10, -1), (20, 3), (30, 4), (40, 5), (0, 6)]
        .into_iter()
        .enumerate()
    {
        set_word(&mut s, row, 21, wdt);
        set_short(&mut s, row, 13, parent);
        set_short(&mut s, row, 14, parent);
    }
    set_text(&mut s, 3, 0, "last direct directory");
    set_text(&mut s, 3, 1, "last direct name");
    s.max_id = 1;
    s.copies = vec![
        (9, 7),
        (100, 9),
        (101, 100), // A preceding copy is not a source.
        (0, 0),
        (7, 7),
        (7, 0),
        (u32::MAX, 7),
        (12, 11),
        (99, 123), // Missing direct source is skipped.
    ];
    let bytes = assemble(table("Map"), &s);
    let source = terrain::read_vmap_maps(&bytes).unwrap();
    assert_eq!(source.direct_records().len(), 5);
    assert_eq!(source.copies().len(), 9);
    let rows = source.materialize().unwrap();
    assert_eq!(
        rows.iter()
            .map(|row| (
                row.id,
                row.fields.wdt_file_data_id,
                row.fields.parent_map_id
            ))
            .collect::<Vec<_>>(),
        vec![
            (0, 10, -1),
            (7, 20, 3),
            (9, 30, 4),
            (7, 40, 5),
            (11, 0, 6),
            (9, 40, 5),
            (100, 30, 4),
            (0, 10, -1),
            (7, 40, 5),
            (7, 10, -1),
            (u32::MAX, 40, 5),
            (12, 0, 6),
        ]
    );
    assert_eq!(rows[10].fields.directory, "last direct directory");
    assert_eq!(rows[10].fields.name, "last direct name");
    assert!(matches!(
        DecodedTable::read(&bytes, table("Map")),
        Err(RecordError::Id)
    ));
}

#[test]
fn vmap_keeps_zero_wdt_ancestors_without_changing_terrain_materialization() {
    let mut s = source_spec("Map", &[7, 8]);
    set_short(&mut s, 0, 13, -1);
    set_short(&mut s, 0, 14, -1);
    set_short(&mut s, 1, 13, 7);
    set_word(&mut s, 1, 21, 42);
    s.copies = vec![(9, 7)];
    let bytes = assemble(table("Map"), &s);
    let vmap = terrain::read_vmap_maps(&bytes)
        .unwrap()
        .materialize()
        .unwrap();
    assert_eq!(
        vmap.iter().map(|row| row.id).collect::<Vec<_>>(),
        vec![7, 8, 9]
    );
    assert_eq!(vmap[1].fields.parent_map_id, 7);
    let terrain = terrain::read_map(&bytes).unwrap().materialize().unwrap();
    assert_eq!(terrain.len(), 1);
    assert_eq!(terrain[0].id, 8);
}

#[test]
fn vmap_snapshot_does_not_traverse_or_repair_parent_graphs() {
    let mut s = source_spec("Map", &[7, 8, 9]);
    for (row, parent) in [8, 7, 32767].into_iter().enumerate() {
        set_short(&mut s, row, 13, parent);
    }
    let rows = terrain::read_vmap_maps(&assemble(table("Map"), &s))
        .unwrap()
        .materialize()
        .unwrap();
    assert_eq!(
        rows.iter()
            .map(|row| row.fields.parent_map_id)
            .collect::<Vec<_>>(),
        vec![8, 7, 32767]
    );
}

#[test]
fn vmap_selected_null_string_rejects_late_rows_even_with_zero_wdt() {
    for field in [0, 1] {
        for wdt in [0, 1] {
            let mut s = source_spec("Map", &[7, 8]);
            set_word(&mut s, 1, 21, wdt);
            set_word(&mut s, 1, field, 0);
            let bytes = assemble(table("Map"), &s);
            assert!(matches!(
                terrain::read_vmap_maps(&bytes),
                Err(RecordError::String)
            ));
            assert!(DecodedTable::read(&bytes, table("Map")).is_ok());
        }
    }
}

#[test]
fn vmap_pointed_empty_strings_and_unselected_null_strings_remain_valid() {
    let s = source_spec("Map", &[7]);
    let source = terrain::read_vmap_maps(&assemble(table("Map"), &s)).unwrap();
    assert_eq!(source.decoded_utf8_bytes(), 0);
    let rows = source.materialize().unwrap();
    assert_eq!(rows.len(), 1);
    assert!(rows[0].fields.name.is_empty());
    assert!(rows[0].fields.directory.is_empty());
}

#[test]
fn vmap_validates_unselected_late_text_before_returning_any_snapshot() {
    let mut s = source_spec("Map", &[7, 8]);
    set_text(&mut s, 1, 2, "unselected");
    let at = s.strings.len() - 2;
    s.strings[at] = 0xff;
    assert!(matches!(
        terrain::read_vmap_maps(&assemble(table("Map"), &s)),
        Err(RecordError::String)
    ));
}

#[test]
fn vmap_text_budget_retains_unused_text_and_charges_each_source_copy() {
    let mut s = source_spec("Map", &[7]);
    set_text(&mut s, 0, 0, "é");
    set_text(&mut s, 0, 1, "a");
    set_text(&mut s, 0, 2, "ignored");
    s.copies = vec![(7, 7), (8, 7), (9, 8), (10, 123)];
    let bytes = assemble(table("Map"), &s);
    for (limit, succeeds) in [(16, true), (15, false)] {
        let Collected::VMap(rows) = terrain::read(&bytes, Role::VMap, limit).unwrap() else {
            panic!()
        };
        assert_eq!(rows.decoded_utf8_bytes(), 10);
        assert_eq!(rows.materialize().is_ok(), succeeds);
    }
    assert!(matches!(
        terrain::read(&bytes, Role::VMap, 9),
        Err(RecordError::Limit)
    ));
}

#[test]
fn vmap_replay_cap_counts_zero_wdt_direct_rows_and_copy_appends() {
    let mut s = source_spec("Map", &[7, 8]);
    s.copies = vec![(7, 7), (9, 7), (10, 9), (11, 123)];
    let bytes = assemble(table("Map"), &s);
    for cap in [0, 1, 2, 3] {
        assert!(matches!(
            terrain::read_vmap_maps(&bytes)
                .unwrap()
                .materialize_with_limit(cap),
            Err(RecordError::Limit)
        ));
    }
    assert_eq!(
        terrain::read_vmap_maps(&bytes)
            .unwrap()
            .materialize_with_limit(4)
            .unwrap()
            .len(),
        4
    );
}

#[test]
fn vmap_repaired_direct_ids_are_the_copy_source_keys() {
    let mut s = source_spec("Map", &[0, 0]);
    s.min_id = 50;
    s.copies = vec![(60, 50), (61, 51), (62, 0)];
    let rows = terrain::read_vmap_maps(&assemble(table("Map"), &s))
        .unwrap()
        .materialize()
        .unwrap();
    assert_eq!(
        rows.iter().map(|row| row.id).collect::<Vec<_>>(),
        vec![50, 51, 60, 61]
    );
    s.min_id = u32::MAX;
    assert!(matches!(
        terrain::read_vmap_maps(&assemble(table("Map"), &s)),
        Err(RecordError::Id)
    ));
}

#[test]
fn vmap_identity_regular_profile_and_complete_tail_checks_apply() {
    let valid = assemble(table("Map"), &source_spec("Map", &[7]));
    for (at, value, error) in [
        (156, 0, RecordError::Schema),
        (140, 1, RecordError::Schema),
        (176, 1, RecordError::Schema),
        (184, 1, RecordError::Schema),
        (172, 1, RecordError::Unsupported),
        (204, 1, RecordError::Unsupported),
        (200, 2, RecordError::Unsupported),
        (240, 100001, RecordError::Limit),
    ] {
        let mut bytes = valid.clone();
        put32(&mut bytes, at, value);
        assert!(matches!(terrain::read_vmap_maps(&bytes), Err(e) if e == error));
    }
    for bytes in [
        &valid[..valid.len() - 1],
        &[valid.as_slice(), &[0]].concat(),
    ] {
        assert!(matches!(
            terrain::read_vmap_maps(bytes),
            Err(RecordError::Bounds)
        ));
    }
}

#[test]
fn vmap_minimal_empty_requires_primitive_metadata_and_exact_eof() {
    let empty = build_zero_section_header(table("Map"), 0);
    let rows = terrain::read_vmap_maps(&empty)
        .unwrap()
        .materialize_with_limit(0)
        .unwrap();
    assert!(rows.is_empty());
    for bytes in [
        &empty[..204],
        &empty[..empty.len() - 1],
        &[empty.as_slice(), &[0]].concat(),
    ] {
        assert!(matches!(
            terrain::read_vmap_maps(bytes),
            Err(RecordError::Bounds)
        ));
    }
}

#[test]
fn vmap_missing_copy_sources_do_not_consume_replay_capacity_or_insert_defaults() {
    let mut s = source_spec("Map", &[]);
    s.copies = vec![(0, 0), (7, 7), (u32::MAX, 99)];
    let rows = terrain::read_vmap_maps(&assemble(table("Map"), &s))
        .unwrap()
        .materialize_with_limit(0)
        .unwrap();
    assert!(rows.is_empty());
}

// Authored 2026-10-11: GameObjectDisplayInfo direct-record consumer fixtures.
// The native producer's model dispatch and candidate filtering are not executed.
fn gameobject_spec(ids: &[u32]) -> Spec {
    source_spec("GameObjectDisplayInfo", ids)
}
fn gameobject_bytes(s: &Spec) -> Vec<u8> {
    assemble(table("GameObjectDisplayInfo"), s)
}
fn gameobject_pairs(data: &[u8]) -> Result<Vec<(u32, u32)>, RecordError> {
    Ok(terrain::read_gameobject_display_info(data)?
        .direct_records()
        .iter()
        .map(|row| (row.id, row.fields.file_data_id))
        .collect())
}

#[test]
fn gameobject_closed_schema_has_nine_physical_and_fourteen_scalar_values() {
    let t = table("GameObjectDisplayInfo");
    let schema = t.schema();
    assert_eq!(t.file_id(), 1266277);
    assert_eq!(
        (schema.layout, schema.index, schema.parent),
        (0x16D1DD61, -1, -1)
    );
    assert_eq!(
        schema.fields,
        &[
            Kind::Float(6),
            Kind::Signed(32, 1),
            Kind::Signed(16, 1),
            Kind::Float(1),
            Kind::Float(1),
            Kind::Signed(32, 1),
            Kind::Signed(32, 1),
            Kind::Signed(32, 1),
            Kind::Unsigned(16, 1),
        ]
    );
    assert_eq!(
        schema.fields.iter().map(|kind| kind.count()).sum::<usize>(),
        14
    );
    let bytes = gameobject_bytes(&gameobject_spec(&[7]));
    assert_eq!(u32::from_le_bytes(bytes[188..192].try_into().unwrap()), 216);
}

#[test]
fn gameobject_direct_signed_extrema_zero_and_duplicate_fdids_keep_physical_order() {
    let ids = [7, 2, 7, 9, 10, 11, 12];
    let values = [0, 1, i32::MAX as u32, 0x80000000, u32::MAX, 1, 0];
    let mut s = gameobject_spec(&ids);
    s.max_id = 0; // Direct snapshots do not impose AutoProduceData's index extent.
    for (row, value) in values.into_iter().enumerate() {
        set_word(&mut s, row, 1, value);
    }
    let bytes = gameobject_bytes(&s);
    assert_eq!(
        gameobject_pairs(&bytes).unwrap(),
        ids.into_iter().zip(values).collect::<Vec<_>>()
    );
    assert!(matches!(
        DecodedTable::read(&bytes, table("GameObjectDisplayInfo")),
        Err(RecordError::Id)
    ));
    assert!(matches!(
        DecodedTable::read_with_direct_u32_values(&bytes, table("GameObjectDisplayInfo"), 1),
        Err(RecordError::Schema)
    ));
}

#[test]
fn gameobject_id_table_repair_preserves_bits_and_checks_overflow() {
    let mut s = gameobject_spec(&[0, 50, 49, 55, u32::MAX]);
    s.min_id = 50;
    s.max_id = 0;
    for row in 0..5 {
        set_word(&mut s, row, 1, row as u32 + 100);
    }
    assert_eq!(
        gameobject_pairs(&gameobject_bytes(&s)).unwrap(),
        [(50, 100), (51, 101), (52, 102), (55, 103), (u32::MAX, 104)]
    );
    s.min_id = u32::MAX;
    assert_eq!(
        gameobject_pairs(&gameobject_bytes(&s)).err(),
        Some(RecordError::Id)
    );
}

#[test]
fn gameobject_copy_operations_are_retained_without_changing_direct_rows() {
    let mut s = gameobject_spec(&[0, 7, 7, 9]);
    for (row, value) in [0, 10, 20, u32::MAX].into_iter().enumerate() {
        set_word(&mut s, row, 1, value);
    }
    s.max_id = 0;
    s.copies = vec![
        (7, 9),
        (10, 7),
        (11, 10),
        (0, 0),
        (9, 9),
        (7, 0),
        (12, 777),
        (u32::MAX, u32::MAX),
    ];
    let source = terrain::read_gameobject_display_info(&gameobject_bytes(&s)).unwrap();
    assert_eq!(
        source
            .direct_records()
            .iter()
            .map(|r| (r.id, r.fields.file_data_id))
            .collect::<Vec<_>>(),
        [(0, 0), (7, 10), (7, 20), (9, u32::MAX)]
    );
    assert_eq!(
        source
            .copies()
            .iter()
            .map(|r| (r.new_id, r.source_id))
            .collect::<Vec<_>>(),
        s.copies
    );
    assert_eq!(source.decoded_utf8_bytes(), 0);
    // No materialize method exists for GameObjectFields; consumers iterate this
    // direct snapshot and decide separately whether a model can be extracted.
}

#[test]
fn gameobject_regular_no_direct_rows_can_retain_copies_without_replay() {
    let mut s = gameobject_spec(&[]);
    s.copies = vec![(0, 0), (7, 7), (u32::MAX, 55)];
    let source = terrain::read_gameobject_display_info(&gameobject_bytes(&s)).unwrap();
    assert!(source.direct_records().is_empty());
    assert_eq!(source.copies().len(), 3);
}

#[test]
fn gameobject_full_width_immediate_modes_preserve_fdids() {
    let words = [0, 1, 0x7fffffff, 0x80000000, u32::MAX];
    for mode in [0, 1, 5] {
        let mut s = gameobject_spec(&[7, 8, 9, 10, 11]);
        let bit = u32::from(field_offset(&s, 1));
        if mode != 0 {
            as_mode(&mut s, 1, mode, bit, 32, 0, 0);
        }
        for (row, word) in words.into_iter().enumerate() {
            set_word(&mut s, row, 1, word);
        }
        assert_eq!(
            gameobject_pairs(&gameobject_bytes(&s))
                .unwrap()
                .iter()
                .map(|r| r.1)
                .collect::<Vec<_>>(),
            words,
            "mode {mode}"
        );
    }
}

#[test]
fn gameobject_narrow_immediate_signed_extension_differs_from_unsigned_packing() {
    for (mode, expected) in [(1, [0x7f, 0x80, 0xff]), (5, [0x7f, 0xffffff80, u32::MAX])] {
        let mut s = gameobject_spec(&[7, 8, 9]);
        let bit = u32::from(field_offset(&s, 1));
        as_mode(&mut s, 1, mode, bit, 8, 0, 0);
        for (row, word) in [0x7f, 0x80, 0xff].into_iter().enumerate() {
            set_word(&mut s, row, 1, word);
        }
        assert_eq!(
            gameobject_pairs(&gameobject_bytes(&s))
                .unwrap()
                .iter()
                .map(|r| r.1)
                .collect::<Vec<_>>(),
            expected
        );
    }
}

#[test]
fn gameobject_common_defaults_overrides_and_repaired_ids_preserve_bits() {
    let mut s = gameobject_spec(&[0, 7, 7, 9]);
    s.min_id = 3;
    as_mode(&mut s, 1, 2, 0x80000000, 0, 0, 24);
    s.common = [3u32, 0, 7, 1, 7, u32::MAX]
        .into_iter()
        .flat_map(u32::to_le_bytes)
        .collect();
    assert_eq!(
        gameobject_pairs(&gameobject_bytes(&s)).unwrap(),
        [(3, 0), (7, u32::MAX), (7, u32::MAX), (9, 0x80000000)]
    );
    s.cols[1].extra = 23;
    s.common.pop();
    assert_eq!(
        gameobject_pairs(&gameobject_bytes(&s)).err(),
        Some(RecordError::Column)
    );
}

#[test]
fn gameobject_scalar_pallet_and_single_element_pallet_array_are_checked() {
    for mode in [3, 4] {
        let mut s = gameobject_spec(&[7, 8, 9]);
        let bit = u32::from(field_offset(&s, 1));
        as_mode(&mut s, 1, mode, bit, 2, if mode == 4 { 1 } else { 0 }, 12);
        s.pallet = [u32::MAX, 0, 0x80000000]
            .into_iter()
            .flat_map(u32::to_le_bytes)
            .collect();
        for row in 0..3 {
            set_word(&mut s, row, 1, row as u32);
        }
        assert_eq!(
            gameobject_pairs(&gameobject_bytes(&s)).unwrap(),
            [(7, u32::MAX), (8, 0), (9, 0x80000000)]
        );
        set_word(&mut s, 2, 1, 3);
        assert_eq!(
            gameobject_pairs(&gameobject_bytes(&s)).err(),
            Some(RecordError::Column)
        );
    }
}

#[test]
fn gameobject_ignored_geobox_nan_infinity_signed_zero_are_not_finiteness_gated() {
    let mut s = gameobject_spec(&[7]);
    set_word(&mut s, 0, 1, 0x80000001);
    for (index, value) in [
        0x7fc01234, 0xffc00001, 0x80000000, 0, 0x7f800000, 0xff800000,
    ]
    .into_iter()
    .enumerate()
    {
        write_word(&mut s.records[0], (index * 32) as u16, value);
    }
    set_word(&mut s, 0, 3, 0x7fc01234);
    set_word(&mut s, 0, 4, 0xff800000);
    let bytes = gameobject_bytes(&s);
    assert_eq!(gameobject_pairs(&bytes).unwrap(), [(7, 0x80000001)]);
    let generic = DecodedTable::read(&bytes, table("GameObjectDisplayInfo")).unwrap();
    assert_eq!(generic.rows()[&7][0][0], Value::FloatBits(0x7fc01234));
    assert_eq!(generic.rows()[&7][0][2], Value::FloatBits(0x80000000));
}

#[test]
fn gameobject_geobox_array_pallet_is_validated_even_though_unselected() {
    let mut s = gameobject_spec(&[7, 8]);
    as_mode(&mut s, 0, 4, 0, 1, 6, 48);
    s.pallet = [
        0, 0x80000000, 0x7fc00001, 0x7f800000, 0xff800000, 3, 1, 2, 3, 4, 5, 6,
    ]
    .into_iter()
    .flat_map(u32::to_le_bytes)
    .collect();
    s.records[1][0] = 1;
    set_word(&mut s, 0, 1, 123);
    set_word(&mut s, 1, 1, 456);
    assert_eq!(
        gameobject_pairs(&gameobject_bytes(&s)).unwrap(),
        [(7, 123), (8, 456)]
    );
    s.cols[0].c = 5;
    assert_eq!(
        gameobject_pairs(&gameobject_bytes(&s)).err(),
        Some(RecordError::Column)
    );
}

#[test]
fn gameobject_late_unselected_fields_and_compression_parameters_still_validate() {
    let mut s = gameobject_spec(&[7, 8]);
    as_mode(&mut s, 8, 3, 0, 2, 0, 4);
    let size = record_size(&s.cols);
    for row in &mut s.records {
        row.resize(size, 0);
    }
    s.pallet = 0u32.to_le_bytes().to_vec();
    s.records[1][0] = 1;
    assert_eq!(
        gameobject_pairs(&gameobject_bytes(&s)).err(),
        Some(RecordError::Column)
    );
    let mut s = gameobject_spec(&[7]);
    s.cols[1].offset += 1;
    assert_eq!(
        gameobject_pairs(&gameobject_bytes(&s)).err(),
        Some(RecordError::Column)
    );
    for mode in [1, 5] {
        let mut s = gameobject_spec(&[7]);
        as_mode(&mut s, 1, mode, 0, 33, 0, 0);
        assert_eq!(
            gameobject_pairs(&gameobject_bytes(&s)).err(),
            Some(RecordError::Column)
        );
    }
    let mut s = gameobject_spec(&[7]);
    as_mode(&mut s, 0, 1, 0, 8, 0, 0);
    assert_eq!(
        gameobject_pairs(&gameobject_bytes(&s)).err(),
        Some(RecordError::Column)
    );
}

#[test]
fn gameobject_schema_identity_does_not_add_table_hash_or_header_index_gates() {
    let valid = gameobject_bytes(&gameobject_spec(&[7]));
    for (at, value) in [(140, 8), (156, 0), (176, 10)] {
        let mut bytes = valid.clone();
        put32(&mut bytes, at, value);
        assert_eq!(gameobject_pairs(&bytes).err(), Some(RecordError::Schema));
    }
    let mut bytes = valid;
    put32(&mut bytes, 152, 0x12345678);
    put16(&mut bytes, 174, 0);
    assert_eq!(gameobject_pairs(&bytes).unwrap(), [(7, 0)]);
}

#[test]
fn gameobject_sparse_tact_multi_parent_and_catalog_variants_fail_closed() {
    let valid = gameobject_bytes(&gameobject_spec(&[7]));
    for (at, value, error) in [
        (172, 1, RecordError::Unsupported),
        (204, 1, RecordError::Unsupported),
        (184, 1, RecordError::Schema),
        // Header::parse rejects counts above one before record-profile checks.
        (184, 2, RecordError::Metadata),
        (236, 1, RecordError::Unsupported),
        (232, 12, RecordError::Bounds),
    ] {
        let mut bytes = valid.clone();
        put32(&mut bytes, at, value);
        assert_eq!(
            gameobject_pairs(&bytes).err(),
            Some(error),
            "header offset {at}"
        );
    }
    let mut s = gameobject_spec(&[7]);
    s.extra_sections = 1;
    assert_eq!(
        gameobject_pairs(&gameobject_bytes(&s)).err(),
        Some(RecordError::Unsupported)
    );
}

#[test]
fn gameobject_complete_record_id_copy_and_string_extents_are_required() {
    let valid = gameobject_bytes(&gameobject_spec(&[7]));
    for cut in 0..valid.len() {
        assert!(gameobject_pairs(&valid[..cut]).is_err(), "cut {cut}");
    }
    assert_eq!(
        gameobject_pairs(&[valid.as_slice(), &[0]].concat()).err(),
        Some(RecordError::Bounds)
    );
    for (at, value, error) in [
        (216, 2, RecordError::Bounds),
        (220, 1, RecordError::Bounds),
        (228, 0, RecordError::Id),
        (240, 1, RecordError::Bounds),
        (188, 215, RecordError::Column),
    ] {
        let mut bytes = valid.clone();
        put32(&mut bytes, at, value);
        assert_eq!(
            gameobject_pairs(&bytes).err(),
            Some(error),
            "header offset {at}"
        );
    }
    let mut s = gameobject_spec(&[7]);
    s.copies = vec![(1, 7), (2, 1)];
    let mut bytes = gameobject_bytes(&s);
    bytes.pop();
    assert_eq!(gameobject_pairs(&bytes).err(), Some(RecordError::Bounds));
}

#[test]
fn gameobject_zero_section_minimum_requires_all_primitive_metadata() {
    let empty = build_zero_section_header(table("GameObjectDisplayInfo"), 0);
    assert_eq!(empty.len(), 240);
    let rows = terrain::read_gameobject_display_info(&empty).unwrap();
    assert!(rows.direct_records().is_empty());
    assert!(rows.copies().is_empty());
    for bytes in [
        &empty[..204],
        &empty[..239],
        &[empty.as_slice(), &[0]].concat(),
    ] {
        assert_eq!(gameobject_pairs(bytes).err(), Some(RecordError::Bounds));
    }
    for at in [136, 148, 188, 192, 196] {
        let mut bytes = empty.clone();
        put32(&mut bytes, at, 1);
        assert_eq!(
            gameobject_pairs(&bytes).err(),
            Some(RecordError::Unsupported)
        );
    }
}

#[test]
fn gameobject_numeric_source_needs_no_text_budget() {
    let bytes = gameobject_bytes(&gameobject_spec(&[7, 8]));
    let Collected::GameObject(rows) = terrain::read(&bytes, Role::GameObject, 0).unwrap() else {
        panic!()
    };
    assert_eq!(rows.direct_records().len(), 2);
    assert_eq!(rows.decoded_utf8_bytes(), 0);
}

#[test]
fn gameobject_file_direct_copy_and_scalar_policy_limits_apply() {
    let valid = gameobject_bytes(&gameobject_spec(&[7]));
    let mut oversized = valid.clone();
    oversized.resize(16 * 1024 * 1024 + 1, 0);
    assert_eq!(gameobject_pairs(&oversized).err(), Some(RecordError::Limit));
    for (at, value) in [(136, 100001), (240, 100001), (136, 71429)] {
        let mut bytes = valid.clone();
        put32(&mut bytes, at, value);
        assert_eq!(gameobject_pairs(&bytes).err(), Some(RecordError::Limit));
    }
    // Fourteen decoded scalars per row: 71,428 * 14 == 999,992; one more
    // would exceed the shared million-value budget despite only nine columns.
    let s = gameobject_spec(&vec![1; 71428]);
    let source = terrain::read_gameobject_display_info(&gameobject_bytes(&s)).unwrap();
    assert_eq!(source.direct_records().len(), 71428);
}

#[test]
fn gameobject_copy_limit_is_independent_of_projected_scalar_count() {
    let mut s = gameobject_spec(&[7]);
    set_word(&mut s, 0, 1, 123);
    s.copies = vec![(u32::MAX, 7); 100000];
    let source = terrain::read_gameobject_display_info(&gameobject_bytes(&s)).unwrap();
    assert_eq!(source.direct_records().len(), 1);
    assert_eq!(source.direct_records()[0].fields.file_data_id, 123);
    assert_eq!(source.copies().len(), 100000);
}

#[test]
fn gameobject_capacity_accounting_and_transfer_preserve_vectors() {
    let mut spec = gameobject_spec(&[9, 7, 9]);
    for (ordinal, value) in [0, u32::MAX, 17].into_iter().enumerate() {
        set_word(&mut spec, ordinal, 1, value);
    }
    spec.copies = vec![(7, 9), (19, 7)];
    let source = terrain::read_gameobject_display_info(&gameobject_bytes(&spec)).unwrap();
    let bytes = source.resident_bytes().unwrap();
    let direct_ptr = source.direct_records().as_ptr();
    let copies_ptr = source.copies().as_ptr();
    let (direct, copies) = source.into_parts();
    assert_eq!(direct.as_ptr(), direct_ptr);
    assert_eq!(copies.as_ptr(), copies_ptr);
    assert_eq!(
        bytes,
        direct.capacity() * size_of::<terrain::DirectRecord<terrain::GameObjectFields>>()
            + copies.capacity() * size_of::<terrain::RecordCopy>()
    );
    assert_eq!(
        direct.iter().map(|row| row.id).collect::<Vec<_>>(),
        [9, 7, 9]
    );
    assert_eq!(
        direct
            .iter()
            .map(|row| row.fields.file_data_id)
            .collect::<Vec<_>>(),
        [0, u32::MAX, 17]
    );
    assert_eq!(
        copies
            .iter()
            .map(|copy| (copy.new_id, copy.source_id))
            .collect::<Vec<_>>(),
        [(7, 9), (19, 7)]
    );
}

#[test]
fn navigation_projection_keeps_numeric_fields_raw_parents_and_direct_copies() {
    let mut s = source_spec("Map", &[0, 7, 7, 9]);
    s.copies = vec![(8, 7), (7, 9), (10, 8)];
    for row in 0..4 {
        set_byte(&mut s, row, 7, [0, 3, 255, 1][row]);
        set_byte(&mut s, row, 8, [3, 255, 128, 0][row]);
        set_short(&mut s, row, 13, [-1, 0, -32768, 7][row]);
        set_short(&mut s, row, 14, [32767, 9, 0, -2][row]);
        set_word(&mut s, row, 25, [0, 2, 0x80000000, u32::MAX][row]);
    }
    let b = assemble(table("Map"), &s);
    let source = terrain::read_navigation_maps(&b).unwrap();
    assert_eq!(
        source
            .direct_records()
            .iter()
            .map(|r| r.id)
            .collect::<Vec<_>>(),
        [0, 7, 7, 9]
    );
    assert_eq!(
        source.copies(),
        [
            RecordCopy {
                new_id: 8,
                source_id: 7
            },
            RecordCopy {
                new_id: 7,
                source_id: 9
            },
            RecordCopy {
                new_id: 10,
                source_id: 8
            }
        ]
    );
    let f = source.direct_records()[2].fields;
    assert_eq!(
        (
            f.map_type,
            f.instance_type,
            f.parent_map_id,
            f.cosmetic_parent_map_id,
            f.flags,
            f.effective_parent()
        ),
        (255, -128, -32768, 0, i32::MIN, 0)
    );
    assert_eq!(source.direct_records()[1].fields.effective_parent(), 0);
}
#[test]
fn navigation_numeric_consumer_accepts_null_unused_strings() {
    let mut s = source_spec("Map", &[7]);
    set_word(&mut s, 0, 0, 0);
    set_word(&mut s, 0, 1, 0);
    let b = assemble(table("Map"), &s);
    assert!(terrain::read_navigation_maps(&b).is_ok());
    assert!(terrain::read_map(&b).is_err());
    assert!(terrain::read_vmap_maps(&b).is_err());
}
#[test]
fn navigation_projection_requires_complete_closed_map() {
    let s = source_spec("Map", &[7]);
    let b = assemble(table("Map"), &s);
    assert!(terrain::read_navigation_maps(&b[..b.len() - 1]).is_err());
    let b = assemble(table("LiquidObject"), &source_spec("LiquidObject", &[7]));
    assert!(terrain::read_navigation_maps(&b).is_err());
}

#[test]
#[ignore = "requires explicit external synthetic navigation requests/output"]
fn write_navigation_map_corpus() {
    use std::{env, fs, path::PathBuf};
    let input = PathBuf::from(env::var_os("RCE_NAVIGATION_MAP_REQUESTS").expect("input"));
    let output = PathBuf::from(env::var_os("RCE_NAVIGATION_MAP_OUTPUT").expect("output"));
    fs::create_dir(&output).unwrap();
    for entry in fs::read_dir(input).unwrap() {
        let path = entry.unwrap().path();
        let bytes = fs::read(&path).unwrap();
        let mut at = 0;
        let mut next = || {
            let n = u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
            at += 4;
            n
        };
        let n = next() as usize;
        let copies = next() as usize;
        let mut rows = Vec::new();
        for _ in 0..n {
            rows.push(std::array::from_fn::<u32, 6, _>(|_| next()));
        }
        let pairs = (0..copies).map(|_| (next(), next())).collect::<Vec<_>>();
        assert_eq!(at, bytes.len());
        let mut s = source_spec("Map", &rows.iter().map(|r| r[0]).collect::<Vec<_>>());
        s.copies = pairs;
        for (i, r) in rows.iter().enumerate() {
            set_byte(&mut s, i, 7, r[1] as u8);
            set_byte(&mut s, i, 8, r[2] as u8);
            set_short(&mut s, i, 13, r[3] as i16);
            set_short(&mut s, i, 14, r[4] as i16);
            set_word(&mut s, i, 25, r[5]);
            // Numeric-only navigation does not invoke string getters. Retain
            // null source strings, WDT-zero rows and unrelated flags elements.
            set_word(&mut s, i, 0, 0);
            set_word(&mut s, i, 1, 0);
            let offset = field_offset(&s, 25) / 8;
            put32(&mut s.records[i], offset as usize + 4, 0x80000000);
            put32(&mut s.records[i], offset as usize + 8, u32::MAX);
        }
        let data = assemble(table("Map"), &s);
        terrain::read_navigation_maps(&data).unwrap();
        let dir = output.join(path.file_stem().unwrap());
        fs::create_dir(&dir).unwrap();
        fs::write(dir.join("Map.db2"), data).unwrap();
    }
}
