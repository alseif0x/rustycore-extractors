// SPDX-License-Identifier: GPL-3.0-or-later
//! Authored 2026-10-16: synthetic70425 sections; no client inputs.
use super::*;

fn table70425(name: &str) -> CharacterTable {
    let table = CharacterTable::from_catalog_id(crate::catalog::WOWDBDEFS_70425_CATALOG_ID, name)
        .unwrap()
        .unwrap();
    match table {
        CharacterTable::Catalog(catalog) => assert_eq!(
            catalog.catalog_id(),
            crate::catalog::WOWDBDEFS_70425_CATALOG_ID
        ),
        _ => panic!("expected explicit70425 catalog identity"),
    }
    table
}
fn spec70425(name: &str, ids: &[u32], value: u32) -> Spec {
    // Reuse the old authored physical getter fixture ONLY after proving the
    // currently pinned targets have equal wire field/index/parent shapes.
    let old = table(name).schema();
    let target = table70425(name).schema();
    assert_eq!(old.fields, target.fields);
    assert_eq!(old.index, target.index);
    assert_eq!(old.parent, target.parent);
    let spec = source_spec(name, ids, value);
    assert!(
        table70425(name).matches(
            &crate::Header::parse(
                assemble(table70425(name), &spec)[..HEADER]
                    .try_into()
                    .unwrap()
            )
            .unwrap()
        )
    );
    spec
}
fn fixture70425(name: &str, specs: Vec<Spec>, tact: &[bool]) -> Input {
    let target = table70425(name).schema();
    // Parent fixture owns the multi-section/string-coordinate arithmetic.
    // Bind its resulting header to the explicit70425 schema, not to a
    // presumed old-catalog identity. Layout equality is separately asserted.
    assert_eq!(table(name).schema().fields, target.fields);
    let mut input = fixture(name, specs, tact);
    put32(&mut input.data, 152, target.hash.unwrap_or(0));
    put32(&mut input.data, 156, target.layout);
    assert!(
        table70425(name)
            .matches(&crate::Header::parse(input.data[..HEADER].try_into().unwrap()).unwrap())
    );
    input
}
fn one70425(name: &str) -> Input {
    fixture70425(name, vec![spec70425(name, &[7], 1)], &[false])
}

#[test]
fn available70425_explicit_catalog_identity_is_distinct_despite_equal_wire_layouts() {
    assert_eq!(
        available::PROFILE,
        "wowdbdefs-70374-available-sections-disabled-v1"
    );
    assert_eq!(
        available::PROFILE_70425,
        "wowdbdefs-70425-available-sections-disabled-v1"
    );
    for (name, id, layout) in [
        ("Map", 1349477, 0xD43AFAC3),
        ("GameObjectDisplayInfo", 1266277, 0x16D1DD61),
    ] {
        let target = table70425(name);
        assert_ne!(target, table(name));
        assert_eq!(target.schema().layout, layout);
        match target {
            CharacterTable::Catalog(catalog) => assert_eq!(catalog.metadata().file_data_id, id),
            _ => unreachable!(),
        }
    }
}
#[test]
fn available70425_all_three_entrypoints_decode_explicit_authored_sources() {
    let input = one70425("Map");
    let terrain = available::read_map_70425(&input).unwrap();
    let vmap = available::read_vmap_maps_70425(&input).unwrap();
    assert_eq!(terrain.direct_records()[0].fields.wdt_file_data_id, 1);
    assert_eq!(vmap.direct_records()[0].fields.wdt_file_data_id, 1);
    assert_eq!(
        terrain.direct_records()[0].fields.name,
        vmap.direct_records()[0].fields.name
    );
    assert!(terrain.all_sections_admitted());
    assert!(vmap.all_sections_admitted());
    let input = one70425("GameObjectDisplayInfo");
    assert_eq!(
        available::read_gameobject_display_info_70425(&input)
            .unwrap()
            .direct_records()[0]
            .fields
            .file_data_id,
        1
    );
}
#[test]
fn available70425_godi_direct_order_signed_fdid_bits_and_unreplayed_copies() {
    let mut spec = spec70425("GameObjectDisplayInfo", &[7, 7, 0], u32::MAX);
    spec.copies = vec![(9, 7), (10, 9)];
    let input = fixture70425("GameObjectDisplayInfo", vec![spec], &[false]);
    let view = available::read_gameobject_display_info_70425(&input).unwrap();
    assert_eq!(
        view.direct_records()
            .iter()
            .map(|r| (r.id, r.fields.file_data_id))
            .collect::<Vec<_>>(),
        [(7, u32::MAX), (7, u32::MAX), (2, u32::MAX)]
    );
    assert_eq!(view.copies().len(), 2);
    assert_eq!(view.sections()[0].declared_copies, 2);
}
#[test]
fn available70425_terrain_and_vmap_keep_distinct_wdt_zero_materialization() {
    let mut spec = spec70425("Map", &[7], 0);
    spec.copies = vec![(8, 7), (9, 8)];
    let input = fixture70425("Map", vec![spec], &[false]);
    assert!(
        available::read_map_70425(&input)
            .unwrap()
            .materialize()
            .unwrap()
            .rows()
            .is_empty()
    );
    let vmap = available::read_vmap_maps_70425(&input)
        .unwrap()
        .materialize()
        .unwrap();
    assert_eq!(vmap.rows().iter().map(|r| r.id).collect::<Vec<_>>(), [7, 8]);
    assert_eq!(vmap.direct_records().len(), 1);
    assert_eq!(vmap.rows()[1].fields.wdt_file_data_id, 0);
}
#[test]
fn available70425_vmap_signed_effective_parents_and_last_direct_copy_source() {
    let mut spec = spec70425("Map", &[7, 7, 8], 1);
    for (row, (parent, cosmetic)) in [(-1, 9), (i16::MIN, i16::MAX), (0, -1)]
        .into_iter()
        .enumerate()
    {
        vmap_short(&mut spec, row, 13, parent);
        vmap_short(&mut spec, row, 14, cosmetic);
    }
    spec.copies = vec![(10, 7), (11, 10), (12, 99)];
    let input = fixture70425("Map", vec![spec], &[false]);
    let view = available::read_vmap_maps_70425(&input)
        .unwrap()
        .materialize()
        .unwrap();
    assert_eq!(
        view.rows()
            .iter()
            .map(|r| (r.id, r.fields.parent_map_id))
            .collect::<Vec<_>>(),
        [(7, 9), (7, i16::MAX), (8, 0), (10, i16::MAX)]
    );
    assert_eq!(view.direct_records().len(), 3);
    assert!(view.resident_bytes().unwrap() > 0);
}
#[test]
fn available70425_unknown_tact_holes_never_read_and_global_id_ordinal_survives() {
    let mut omitted = spec70425("Map", &[5, 6], 0);
    omitted.copies = vec![(20, 5)];
    let mut admitted = spec70425("Map", &[0], 1);
    admitted.copies = vec![(21, 5), (22, 2)];
    let mut input = fixture70425("Map", vec![omitted, admitted], &[true, false]);
    let hole = payload(&input, 0);
    input.holes.push(hole.clone());
    let view = available::read_vmap_maps_70425(&input).unwrap();
    assert_eq!(view.sections()[0].state, SectionState::UnknownTact);
    assert_eq!(view.sections()[0].declared_records, 2);
    assert_eq!(view.sections()[0].declared_copies, 1);
    assert_eq!(view.direct_records()[0].id, 2);
    assert_eq!(view.copies().len(), 2);
    let coverage = view.sections().to_vec();
    let rows = view.materialize().unwrap();
    assert_eq!(rows.sections(), coverage);
    assert!(!rows.all_sections_admitted());
    assert_eq!(
        rows.rows().iter().map(|r| r.id).collect::<Vec<_>>(),
        [2, 22]
    );
    assert!(
        input
            .reads
            .borrow()
            .iter()
            .all(|r| r.end <= hole.start || r.start >= hole.end)
    );
}
#[test]
fn available70425_nonzero_tact_is_omitted_even_with_readable_payload() {
    for name in ["Map", "GameObjectDisplayInfo"] {
        let mut input = fixture70425(name, vec![spec70425(name, &[7], 1)], &[true]);
        input.data[HEADER..HEADER + 8].copy_from_slice(&u64::MAX.to_le_bytes());
        let hole = payload(&input, 0);
        if name == "Map" {
            assert!(
                available::read_map_70425(&input)
                    .unwrap()
                    .direct_records()
                    .is_empty()
            );
            assert!(
                available::read_vmap_maps_70425(&input)
                    .unwrap()
                    .direct_records()
                    .is_empty()
            );
        } else {
            assert!(
                available::read_gameobject_display_info_70425(&input)
                    .unwrap()
                    .direct_records()
                    .is_empty()
            );
        }
        assert!(
            input
                .reads
                .borrow()
                .iter()
                .all(|r| r.end <= hole.start || r.start >= hole.end)
        );
    }
}
#[test]
fn available70425_known_section_missing_bytes_are_fatal_for_each_projection() {
    let mut map = one70425("Map");
    map.holes.push(payload(&map, 0));
    assert_eq!(
        available::read_map_70425(&map).err(),
        Some(RecordError::Bounds)
    );
    assert_eq!(
        available::read_vmap_maps_70425(&map).err(),
        Some(RecordError::Bounds)
    );
    let mut godi = one70425("GameObjectDisplayInfo");
    godi.holes.push(payload(&godi, 0));
    assert_eq!(
        available::read_gameobject_display_info_70425(&godi).err(),
        Some(RecordError::Bounds)
    );
}
#[test]
fn available70425_header_column_and_encrypted_id_metadata_remain_required() {
    for at in [0, HEADER, HEADER + SECTION + 4 * 26] {
        let mut input = one70425("Map");
        input.holes.push(at..at + 1);
        assert_eq!(
            available::read_map_70425(&input).err(),
            Some(RecordError::Bounds)
        );
        assert_eq!(
            available::read_vmap_maps_70425(&input).err(),
            Some(RecordError::Bounds)
        );
    }
    let mut input = fixture70425("Map", vec![spec70425("Map", &[7], 0)], &[true]);
    let at = payload(&input, 0).start - 4;
    input.holes.push(at..at + 4);
    assert_eq!(
        available::read_vmap_maps_70425(&input).err(),
        Some(RecordError::Bounds)
    );
}
#[test]
fn available70425_identity_hash_layout_field_gates_and_header_index_semantics_stay_closed() {
    for name in ["Map", "GameObjectDisplayInfo"] {
        for (at, value) in [(152, 0), (156, 0), (140, 1), (176, 1)] {
            let mut input = one70425(name);
            put32(&mut input.data, at, value);
            if name == "Map" {
                assert_eq!(
                    available::read_map_70425(&input).err(),
                    Some(RecordError::Schema)
                );
                assert_eq!(
                    available::read_vmap_maps_70425(&input).err(),
                    Some(RecordError::Schema)
                );
            } else {
                assert_eq!(
                    available::read_gameobject_display_info_70425(&input).err(),
                    Some(RecordError::Schema)
                );
            }
        }
        let mut input = one70425(name);
        put16(&mut input.data, 174, 0);
        // The authoritative schema index is pinned at -1. The header index
        // word is intentionally not an identity gate in the existing reader.
        if name == "Map" {
            assert!(available::read_map_70425(&input).is_ok());
            assert!(available::read_vmap_maps_70425(&input).is_ok());
        } else {
            assert!(available::read_gameobject_display_info_70425(&input).is_ok());
        }
    }
}
#[test]
fn available70425_omitted_section_rows_copies_scalars_and_sections_still_count() {
    for (at, value) in [
        (136, 100_001),
        (136, 50_000),
        (HEADER + 36, 100_001),
        (HEADER + 12, 100_001),
        (200, 17),
    ] {
        let mut input = fixture70425("Map", vec![spec70425("Map", &[7], 0)], &[true]);
        put32(&mut input.data, at, value);
        assert_eq!(
            available::read_map_70425(&input).err(),
            Some(RecordError::Limit)
        );
        assert_eq!(
            available::read_vmap_maps_70425(&input).err(),
            Some(RecordError::Limit)
        );
    }
}
#[test]
fn available70425_sparse_parent_and_nonregular_section_forms_stay_unsupported() {
    for (at, value) in [
        (184, 1),
        (HEADER + 20, 1),
        (HEADER + 28, 1),
        (HEADER + 32, 1),
    ] {
        let mut input = one70425("Map");
        put32(&mut input.data, at, value);
        assert_eq!(
            available::read_vmap_maps_70425(&input).err(),
            Some(RecordError::Unsupported)
        );
    }
    let mut input = one70425("Map");
    put16(&mut input.data, 172, 1);
    assert_eq!(
        available::read_map_70425(&input).err(),
        Some(RecordError::Unsupported)
    );
}
#[test]
fn available70425_logical_utf8_strings_after_omitted_pool_keep_coordinates() {
    let mut omitted = spec70425("Map", &[7], 0);
    vmap_text(&mut omitted, 0, 0, "omitted-directory");
    vmap_text(&mut omitted, 0, 1, "omitted-name");
    let mut admitted = spec70425("Map", &[8], 1);
    vmap_text(&mut admitted, 0, 0, "Dír");
    vmap_text(&mut admitted, 0, 1, "Nombre");
    let mut input = fixture70425("Map", vec![omitted, admitted], &[true, false]);
    input.holes.push(payload(&input, 0));
    let view = available::read_vmap_maps_70425(&input).unwrap();
    assert_eq!(view.direct_records()[0].fields.directory, "Dír");
    assert_eq!(view.direct_records()[0].fields.name, "Nombre");
    assert_eq!(view.decoded_utf8_bytes(), "DírNombre".len());
    let map = available::read_map_70425(&input).unwrap();
    assert_eq!(map.direct_records()[0].fields.directory, "Dír");
    assert_eq!(map.direct_records()[0].fields.name, "Nombre");
}
#[test]
fn available70425_selected_and_ignored_text_dependencies_into_holes_remain_fatal() {
    for field in [0, 2] {
        let spec = spec70425("Map", &[7], 0);
        let field_byte = field_offset(&spec, field) as usize / 8;
        let mut input = fixture70425("Map", vec![spec, spec70425("Map", &[8], 0)], &[false, true]);
        let at = payload(&input, 0).start + field_byte;
        let size = u32::from_le_bytes(input.data[144..148].try_into().unwrap()) as usize;
        put32(&mut input.data, at, (2 * size + 2 - field_byte) as u32);
        input.holes.push(payload(&input, 1));
        assert_eq!(
            available::read_map_70425(&input).err(),
            Some(RecordError::Bounds)
        );
        assert_eq!(
            available::read_vmap_maps_70425(&input).err(),
            Some(RecordError::Bounds)
        );
    }
}
#[test]
fn available70425_null_unterminated_and_invalid_utf8_strings_reject() {
    let mut input = one70425("Map");
    let at = payload(&input, 0).start;
    put32(&mut input.data, at, 0);
    assert_eq!(
        available::read_map_70425(&input).err(),
        Some(RecordError::String)
    );
    let mut input = one70425("Map");
    let size = u32::from_le_bytes(input.data[144..148].try_into().unwrap()) as usize;
    let at = payload(&input, 0).start + size;
    input.data[at] = 0xff;
    assert_eq!(
        available::read_vmap_maps_70425(&input).err(),
        Some(RecordError::String)
    );
    let mut input = one70425("Map");
    let at = payload(&input, 0).start + size;
    input.data[at..at + 2].copy_from_slice(b"xx");
    assert_eq!(
        available::read_map_70425(&input).err(),
        Some(RecordError::String)
    );
}
#[test]
fn available70425_framing_trailing_bytes_and_declared_length_cap_are_strict() {
    let mut input = one70425("Map");
    input.data.push(0);
    assert_eq!(
        available::read_map_70425(&input).err(),
        Some(RecordError::Bounds)
    );
    struct Huge;
    impl Source for Huge {
        fn declared_bytes(&self) -> u64 {
            16 * 1024 * 1024 + 1
        }
        fn range(&self, _: u64, _: usize) -> Result<&[u8], Unavailable> {
            panic!("cap before source read")
        }
    }
    assert_eq!(
        available::read_map_70425(&Huge).err(),
        Some(RecordError::Limit)
    );
    assert_eq!(
        available::read_vmap_maps_70425(&Huge).err(),
        Some(RecordError::Limit)
    );
    assert_eq!(
        available::read_gameobject_display_info_70425(&Huge).err(),
        Some(RecordError::Limit)
    );
}
