// SPDX-License-Identifier: GPL-3.0-or-later
//! Authored 2026-10-18: synthetic70450 sections; no client inputs.
use super::*;

fn table70450(name: &str) -> CharacterTable {
    let table = CharacterTable::from_catalog_id(crate::catalog::WOWDBDEFS_70450_CATALOG_ID, name)
        .unwrap()
        .unwrap();
    match table {
        CharacterTable::Catalog(catalog) => assert_eq!(
            catalog.catalog_id(),
            crate::catalog::WOWDBDEFS_70450_CATALOG_ID
        ),
        _ => panic!("expected explicit70450 catalog identity"),
    }
    table
}

fn spec70450(name: &str, ids: &[u32], value: u32) -> Spec {
    let prior = table(name).schema();
    let target = table70450(name).schema();
    assert_eq!(prior.fields, target.fields);
    assert_eq!(prior.index, target.index);
    assert_eq!(prior.parent, target.parent);
    assert_eq!(prior.layout, target.layout);
    assert_eq!(prior.hash, target.hash);
    source_spec(name, ids, value)
}

fn fixture70450(name: &str, specs: Vec<Spec>, tact: &[bool]) -> Input {
    let target = table70450(name).schema();
    let mut input = fixture(name, specs, tact);
    put32(&mut input.data, 152, target.hash.unwrap_or(0));
    put32(&mut input.data, 156, target.layout);
    assert!(
        table70450(name)
            .matches(&crate::Header::parse(input.data[..HEADER].try_into().unwrap()).unwrap())
    );
    input
}

fn one70450(name: &str) -> Input {
    fixture70450(name, vec![spec70450(name, &[7], 1)], &[false])
}

#[test]
fn available70450_identity_and_dispatch_are_independent_of_70425() {
    assert_eq!(
        available::PROFILE_70450,
        "wowdbdefs-70450-available-sections-disabled-v1"
    );
    for (name, file_id, layout) in [
        ("Map", 1_349_477, 0xD43AFAC3),
        ("GameObjectDisplayInfo", 1_266_277, 0x16D1DD61),
    ] {
        let current = table70450(name);
        let previous =
            CharacterTable::from_catalog_id(crate::catalog::WOWDBDEFS_70425_CATALOG_ID, name)
                .unwrap()
                .unwrap();
        assert_ne!(current, previous);
        assert_eq!(current.schema().layout, layout);
        assert_eq!(current.file_id(), file_id);
    }

    let map = one70450("Map");
    let terrain = available::read_map_70450(&map).unwrap();
    let vmap = available::read_vmap_maps_70450(&map).unwrap();
    assert_eq!(terrain.direct_records()[0].fields.wdt_file_data_id, 1);
    assert_eq!(vmap.direct_records()[0].fields.wdt_file_data_id, 1);
    assert!(terrain.all_sections_admitted());
    assert!(vmap.all_sections_admitted());

    let godi = one70450("GameObjectDisplayInfo");
    assert_eq!(
        available::read_gameobject_display_info_70450(&godi)
            .unwrap()
            .direct_records()[0]
            .fields
            .file_data_id,
        1
    );
}

#[test]
fn available70450_preserves_direct_godi_order_and_vmap_copy_policy() {
    let mut godi = spec70450("GameObjectDisplayInfo", &[7, 7, 0], u32::MAX);
    godi.copies = vec![(9, 7), (10, 9)];
    let godi = fixture70450("GameObjectDisplayInfo", vec![godi], &[false]);
    let rows = available::read_gameobject_display_info_70450(&godi).unwrap();
    assert_eq!(
        rows.direct_records()
            .iter()
            .map(|row| (row.id, row.fields.file_data_id))
            .collect::<Vec<_>>(),
        [(7, u32::MAX), (7, u32::MAX), (2, u32::MAX)]
    );
    assert_eq!(rows.copies().len(), 2);

    let mut map = spec70450("Map", &[7], 0);
    map.copies = vec![(8, 7), (9, 8)];
    let map = fixture70450("Map", vec![map], &[false]);
    assert!(
        available::read_map_70450(&map)
            .unwrap()
            .materialize()
            .unwrap()
            .rows()
            .is_empty()
    );
    let vmap = available::read_vmap_maps_70450(&map)
        .unwrap()
        .materialize()
        .unwrap();
    assert_eq!(
        vmap.rows().iter().map(|row| row.id).collect::<Vec<_>>(),
        [7, 8]
    );
    assert_eq!(vmap.direct_records().len(), 1);
}

#[test]
fn available70450_keeps_hole_and_schema_gates_closed() {
    let mut omitted = spec70450("Map", &[5, 6], 0);
    omitted.copies = vec![(20, 5)];
    let admitted = spec70450("Map", &[0], 1);
    let mut input = fixture70450("Map", vec![omitted, admitted], &[true, false]);
    let hole = payload(&input, 0);
    input.holes.push(hole.clone());
    let rows = available::read_vmap_maps_70450(&input).unwrap();
    assert_eq!(rows.sections()[0].state, SectionState::UnknownTact);
    assert_eq!(rows.direct_records()[0].id, 2);
    assert!(
        input
            .reads
            .borrow()
            .iter()
            .all(|range| range.end <= hole.start || range.start >= hole.end)
    );

    for name in ["Map", "GameObjectDisplayInfo"] {
        for (offset, value) in [(152, 0), (156, 0), (140, 1), (176, 1)] {
            let mut invalid = one70450(name);
            put32(&mut invalid.data, offset, value);
            let error = if name == "Map" {
                available::read_map_70450(&invalid).err()
            } else {
                available::read_gameobject_display_info_70450(&invalid).err()
            };
            assert_eq!(error, Some(RecordError::Schema), "{name}@{offset}");
        }
    }
    let mut missing = one70450("Map");
    missing.holes.push(payload(&missing, 0));
    assert_eq!(
        available::read_map_70450(&missing).err(),
        Some(RecordError::Bounds)
    );
}
