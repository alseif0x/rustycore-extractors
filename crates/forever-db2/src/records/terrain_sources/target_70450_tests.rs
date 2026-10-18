// SPDX-License-Identifier: GPL-3.0-or-later
//! Authored 2026-10-18: complete synthetic70450 liquid sources, without client IO.
use super::*;

fn table70450(name: &str) -> CharacterTable {
    CharacterTable::from_catalog_id(crate::catalog::WOWDBDEFS_70450_CATALOG_ID, name)
        .unwrap()
        .unwrap()
}

fn spec70450(name: &str, ids: &[u32]) -> Spec {
    let prior = table(name).schema();
    let target = table70450(name).schema();
    assert_eq!(prior.fields, target.fields);
    assert_eq!(prior.index, target.index);
    assert_eq!(prior.parent, target.parent);
    assert_eq!(prior.layout, target.layout);
    source_spec(name, ids)
}

fn read70450(name: &str, bytes: &[u8], text_limit: usize) -> Result<Collected, RecordError> {
    let role = match name {
        "LiquidMaterial" => Role::LiquidMaterial,
        "LiquidType" => Role::LiquidType,
        "LiquidObject" => Role::LiquidObject,
        _ => panic!("closed liquid fixture name"),
    };
    terrain::read_liquids_70450(bytes, role, text_limit)
}

#[test]
fn liquid70450_catalog_bindings_are_explicit_and_distinct() {
    for (name, file_id, hash, layout, fields) in [
        ("LiquidMaterial", 1_132_538, 0x8C379329, 0x98E5D7AA, 2),
        ("LiquidType", 1_371_380, 0x6613BED3, 0xD1ECEEC9, 21),
    ] {
        let target = table70450(name);
        let previous =
            CharacterTable::from_catalog_id(crate::catalog::WOWDBDEFS_70425_CATALOG_ID, name)
                .unwrap()
                .unwrap();
        let CharacterTable::Catalog(catalog) = target else {
            panic!("expected fixed70450 catalog");
        };
        assert_eq!(
            catalog.catalog_id(),
            crate::catalog::WOWDBDEFS_70450_CATALOG_ID
        );
        assert_ne!(target, previous);
        assert_eq!(target.file_id(), file_id);
        assert_eq!(target.schema().hash, Some(hash));
        assert_eq!(
            (
                target.schema().layout,
                target.schema().index,
                target.schema().parent
            ),
            (layout, -1, -1)
        );
        assert_eq!(target.schema().fields.len(), fields);
        assert_eq!(target.schema().fields, table(name).schema().fields);
        assert!(
            target.matches(
                &crate::Header::parse(
                    assemble(target, &spec70450(name, &[7]))[..HEADER]
                        .try_into()
                        .unwrap()
                )
                .unwrap()
            )
        );
    }
}

#[test]
fn liquid70450_readers_decode_only_their_build_specific_binding() {
    let mut liquid_type = spec70450("LiquidType", &[0, 7, 7]);
    set_byte(&mut liquid_type, 0, 3, 255);
    set_byte(&mut liquid_type, 0, 14, 128);
    liquid_type.copies = vec![(9, 7), (10, 9)];
    let bytes = assemble(table70450("LiquidType"), &liquid_type);
    let source = terrain::read_liquid_types_70450(&bytes).unwrap();
    assert_eq!(source.direct_records()[0].fields.sound_bank, 255);
    assert_eq!(source.direct_records()[0].fields.material_id, 128);
    assert_eq!(source.materialize().unwrap()[&9].sound_bank, 0);

    let mut material = spec70450("LiquidMaterial", &[7]);
    set_word(&mut material, 0, 0, u32::MAX);
    set_byte(&mut material, 0, 1, 0x80);
    let bytes = assemble(table70450("LiquidMaterial"), &material);
    let source = terrain::read_liquid_materials_70450(&bytes).unwrap();
    assert_eq!(source.direct_records()[0].fields.flags_bits, u32::MAX);
    assert_eq!(source.direct_records()[0].fields.lvf, i8::MIN);

    for role in [Role::Map, Role::GameObject] {
        assert_eq!(
            terrain::read_liquids_70450(&bytes, role, 64).err(),
            Some(RecordError::Schema)
        );
    }
}

#[test]
fn liquid_object70450_reader_binds_catalog_and_validates_physical_structure() {
    let mut object = spec70450("LiquidObject", &[7, 9]);
    set_short(&mut object, 0, 2, -1);
    set_short(&mut object, 1, 2, 513);
    object.copies = vec![(11, 7)];
    let valid = assemble(table70450("LiquidObject"), &object);
    let rows = terrain::read_liquid_objects_70450(&valid).unwrap();
    assert_eq!(rows.direct_records()[0].fields.liquid_type_id, u16::MAX);
    assert_eq!(rows.direct_records()[1].fields.liquid_type_id, 513);
    assert_eq!(rows.materialize().unwrap()[&11].liquid_type_id, u16::MAX);

    for (offset, value) in [(152, 0), (156, 0), (140, 0), (176, 0)] {
        let mut invalid = valid.clone();
        put32(&mut invalid, offset, value);
        assert_eq!(
            terrain::read_liquid_objects_70450(&invalid).err(),
            Some(RecordError::Schema),
            "header offset {offset}"
        );
    }
    let mut trailing = valid.clone();
    trailing.push(0);
    assert_eq!(
        terrain::read_liquid_objects_70450(&trailing).err(),
        Some(RecordError::Bounds)
    );
    let mut sparse = valid.clone();
    put16(&mut sparse, 172, 1);
    assert_eq!(
        terrain::read_liquid_objects_70450(&sparse).err(),
        Some(RecordError::Unsupported)
    );
    assert_eq!(
        terrain::read_liquids_70450(&valid, Role::Map, 64).err(),
        Some(RecordError::Schema)
    );
}

#[test]
fn liquid70450_hash_layout_and_physical_shape_gates_remain_strict() {
    for name in ["LiquidMaterial", "LiquidType"] {
        let valid = assemble(table70450(name), &spec70450(name, &[7]));
        for (offset, value) in [(152, 0), (156, 0), (140, 0), (176, 0)] {
            let mut invalid = valid.clone();
            put32(&mut invalid, offset, value);
            assert_eq!(
                read70450(name, &invalid, 64).err(),
                Some(RecordError::Schema)
            );
        }
        let mut trailing = valid.clone();
        trailing.push(0);
        assert_eq!(
            read70450(name, &trailing, 64).err(),
            Some(RecordError::Bounds)
        );
        let mut sparse = valid;
        put16(&mut sparse, 172, 1);
        assert_eq!(
            read70450(name, &sparse, 64).err(),
            Some(RecordError::Unsupported)
        );
    }
}
