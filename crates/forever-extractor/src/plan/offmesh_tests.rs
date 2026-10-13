// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

pub(crate) fn connection() -> OffMeshConnection {
    OffMeshConnection {
        map_id: 42,
        row: 3,
        column: 7,
        from_bits: [0x8000_0000, 1.25f32.to_bits(), (-2.5f32).to_bits()],
        to_bits: [10.0f32.to_bits(), 20.0f32.to_bits(), 30.0f32.to_bits()],
        radius_bits: 0.5f32.to_bits(),
        direction: 0,
        area: 63,
        flags: u16::MAX,
    }
}

pub(crate) fn plan() -> Plan {
    Plan {
        build: 70205,
        schema_catalog: SchemaCatalog::PinnedMetadataV1,
        locale: Locale::EnUs,
        index_layout: local_index::Layout::Contiguous,
        root_layout: root::Layout::Header50893V2,
        decryption: Decryption::Disabled,
        collision_storage: CollisionStorage::Resident,
        components: Component::Db2.bit()
            | Component::Terrain.bit()
            | Component::Collision.bit()
            | Component::Navigation.bit(),
        db2: ["LiquidType", "Map"]
            .map(|name| Entry {
                name: name.into(),
                id: catalog::find(name).unwrap().file_data_id,
            })
            .to_vec(),
        terrain: Selection::All,
        navigation: Selection::All,
        pack: true,
        big_base_unit: false,
        skip_liquids: false,
        maximum_slope: 55,
        non_steep_slope: 55,
        offmesh: Some(vec![connection()]),
    }
}

#[test]
fn v6_canonical_bits_order_duplicates_and_signed_zero_roundtrip() {
    let mut plan = plan();
    let first = connection();
    let second = OffMeshConnection {
        map_id: 7,
        direction: 1,
        ..first
    };
    plan.offmesh = Some(vec![first, second, first]);
    let text = plan.render();
    assert!(text.starts_with("forever-product\t6\n"));
    assert!(text.contains("root\theader-50893-v2\ndecryption\tdisabled\ncomponents\t"));
    assert!(text.contains("non-steep-slope\t55\noffmesh\tinline-f32-bits-v1\noffmesh-connection\t42\t3\t7\t80000000\t3fa00000\tc0200000\t41200000\t41a00000\t41f00000\t3f000000\t0\t63\t65535\n"));
    assert!(text.find("offmesh-connection").unwrap() < text.find("db2\t").unwrap());
    assert_eq!(Plan::parse(&text), Ok(plan.clone()));
    assert_eq!(plan.version(), 6);
    assert_eq!(plan.contract(), "forever-product-v6");
    assert_eq!(OffMeshConnection::from_source(first.source()), Ok(first));
    assert_eq!(first.source().from[0].to_bits(), 0x8000_0000);
}

#[test]
fn v6_explicit_empty_differs_from_legacy_empty() {
    let mut plan = plan();
    plan.offmesh = Some(vec![]);
    assert_eq!(Plan::parse(&plan.render()), Ok(plan.clone()));
    assert_eq!(plan.version(), 6);
    let hash = forever_casc::integrity::digest(plan.render().as_bytes());
    plan.offmesh = None;
    assert_eq!(plan.version(), 1);
    assert!(!plan.render().contains("offmesh"));
    assert_eq!(Plan::parse(&plan.render()), Ok(plan.clone()));
    assert_ne!(
        hash,
        forever_casc::integrity::digest(plan.render().as_bytes())
    );
}

#[test]
fn v6_both_builds_decryption_modes_and_storage_modes_roundtrip() {
    for build in [70170, 70205] {
        for decryption in [Decryption::Disabled, Decryption::ProvidedSalsa20V1] {
            for disk in [false, true] {
                let mut plan = plan();
                plan.build = build;
                plan.decryption = decryption;
                if disk {
                    plan.collision_storage = CollisionStorage::LeasedDiskV1(DiskBudget {
                        raw_bytes: 1024,
                        wdt_bytes: 1024,
                        cache_bytes: 1024,
                        placement_bytes: 1024,
                        vmo_bytes: 1024,
                        map_bytes: 1024,
                        output_bytes: 1024,
                    });
                }
                assert_eq!(plan.version(), 6);
                assert_eq!(plan.contract(), "forever-product-v6");
                assert_eq!(
                    plan.requires_keys(),
                    decryption == Decryption::ProvidedSalsa20V1
                );
                assert_eq!(Plan::parse(&plan.render()), Ok(plan));
            }
        }
    }
}

#[test]
fn v6_header_and_connection_order_are_mandatory() {
    let text = plan().render();
    let header = "offmesh\tinline-f32-bits-v1\n";
    let record = text
        .lines()
        .find(|line| line.starts_with("offmesh-connection\t"))
        .unwrap();
    for changed in [
        text.replace(header, ""),
        text.replace(header, &format!("{header}{header}")),
        text.replace(header, "offmesh\tunknown\n"),
        format!("{}{record}\n", text.replace(&format!("{record}\n"), "")),
        text.replace("decryption\tdisabled\n", ""),
        text.trim_end_matches('\n').into(),
    ] {
        assert!(Plan::parse(&changed).is_err(), "{changed}");
    }
}

#[test]
fn legacy_versions_do_not_admit_inline_header_or_records() {
    for decryption in [Decryption::Disabled, Decryption::ProvidedSalsa20V1] {
        let mut legacy = plan();
        legacy.offmesh = None;
        legacy.decryption = decryption;
        let text = legacy.render();
        assert!(
            Plan::parse(&text.replace(
                "db2\tLiquidType",
                "offmesh\tinline-f32-bits-v1\ndb2\tLiquidType"
            ))
            .is_err()
        );
        let mut record = String::new();
        connection().render(&mut record);
        assert!(
            Plan::parse(&text.replace("db2\tLiquidType", &format!("{record}db2\tLiquidType")))
                .is_err()
        );
    }
    for version in [3, 4, 5] {
        let changed = plan().render().replacen(
            "forever-product\t6",
            &format!("forever-product\t{version}"),
            1,
        );
        assert!(Plan::parse(&changed).is_err());
    }
}

#[test]
fn v6_exact_connection_ceiling_and_plan_byte_ceiling() {
    let mut plan = plan();
    plan.offmesh = Some(vec![connection(); MAX_OFFMESH_CONNECTIONS]);
    assert!(plan.render().len() < MAX_PLAN_BYTES);
    assert_eq!(Plan::parse(&plan.render()), Ok(plan.clone()));
    plan.offmesh.as_mut().unwrap().push(connection());
    assert_eq!(plan.validate(), Err("product_plan_offmesh_limit"));
    assert_eq!(
        Plan::parse(&plan.render()),
        Err("product_plan_offmesh_limit")
    );
    assert_eq!(
        Plan::parse(&"x".repeat(MAX_PLAN_BYTES + 1)),
        Err("product_plan_size")
    );
}

#[test]
fn v6_hex_spelling_and_decimal_spelling_are_closed() {
    let text = plan().render();
    for value in [
        "3FA00000",
        "0x3fa00000",
        "3fa0000",
        "03fa00000",
        "+3fa0000",
        "3ga00000",
    ] {
        assert_eq!(
            Plan::parse(&text.replace("3fa00000", value)),
            Err("product_plan_offmesh_bits")
        );
    }
    for (before, after) in [
        ("\t42\t3\t7\t", "\t042\t3\t7\t"),
        ("\t0\t63\t65535\n", "\t+0\t63\t65535\n"),
        ("\t0\t63\t65535\n", "\t0\t63\t065535\n"),
    ] {
        assert_eq!(
            Plan::parse(&text.replace(before, after)),
            Err("product_plan_decimal")
        );
    }
}

#[test]
fn v6_coordinates_direction_area_flags_and_record_shape_validate() {
    let text = plan().render();
    for (before, after) in [
        ("\t42\t3\t7\t", "\t42\t64\t7\t"),
        ("\t42\t3\t7\t", "\t42\t3\t64\t"),
        ("\t42\t3\t7\t", "\t42\t256\t7\t"),
        ("\t0\t63\t65535\n", "\t2\t63\t65535\n"),
        ("\t0\t63\t65535\n", "\t0\t64\t65535\n"),
        ("\t0\t63\t65535\n", "\t0\t63\t65536\n"),
    ] {
        assert_eq!(
            Plan::parse(&text.replace(before, after)),
            Err("product_plan_offmesh")
        );
    }
    for replacement in ["\t0\t63\n", "\t0\t63\t65535\textra\n"] {
        assert_eq!(
            Plan::parse(&text.replace("\t0\t63\t65535\n", replacement)),
            Err("product_plan_format")
        );
    }
}

#[test]
fn v6_all_float_components_reject_nonfinite_and_radius_must_be_positive() {
    let original = connection();
    for bits in [
        f32::INFINITY.to_bits(),
        f32::NEG_INFINITY.to_bits(),
        0x7fc0_0001,
    ] {
        for field in 0..7 {
            let mut invalid = original;
            match field {
                0..=2 => invalid.from_bits[field] = bits,
                3..=5 => invalid.to_bits[field - 3] = bits,
                _ => invalid.radius_bits = bits,
            }
            assert_eq!(invalid.validate(), Err("product_plan_offmesh"));
            let mut plan = plan();
            plan.offmesh = Some(vec![invalid]);
            assert_eq!(plan.validate(), Err("product_plan_offmesh"));
            assert_eq!(Plan::parse(&plan.render()), Err("product_plan_offmesh"));
        }
    }
    for radius in [0.0f32, -0.0, -1.0] {
        assert_eq!(
            OffMeshConnection {
                radius_bits: radius.to_bits(),
                ..original
            }
            .validate(),
            Err("product_plan_offmesh")
        );
    }
    assert_eq!(
        OffMeshConnection {
            radius_bits: 1,
            ..original
        }
        .validate(),
        Ok(())
    );
}

#[test]
fn v6_scope_requires_pinned_metadata_navigation_and_older_builds() {
    let mut plan = plan();
    for catalog in [
        SchemaCatalog::WowDbDefs70205V1,
        SchemaCatalog::WowDbDefs70338V1,
    ] {
        plan.schema_catalog = catalog;
        assert_eq!(plan.validate(), Err("product_plan_offmesh_scope"));
    }
    plan.schema_catalog = SchemaCatalog::PinnedMetadataV1;
    plan.build = 70338;
    assert_eq!(plan.validate(), Err("product_plan_offmesh_scope"));
    plan.build = 70205;
    plan.components &= !Component::Navigation.bit();
    plan.navigation = Selection::None;
    assert_eq!(plan.validate(), Err("product_plan_offmesh_scope"));
}

#[test]
fn v6_preserves_navigation_dependencies() {
    let original = plan();
    for component in [Component::Db2, Component::Terrain, Component::Collision] {
        let mut plan = original.clone();
        plan.components &= !component.bit();
        assert_eq!(plan.validate(), Err("product_plan_dependencies"));
    }
    let mut plan = original;
    plan.terrain = Selection::Map(42);
    assert_eq!(plan.validate(), Err("product_plan_dependencies"));
}

#[test]
fn v6_every_connection_bit_order_and_duplicate_changes_plan_identity() {
    let plan = plan();
    let original = forever_casc::integrity::digest(plan.render().as_bytes());
    let mut changed = plan.clone();
    changed.offmesh.as_mut().unwrap()[0].from_bits[0] = 0;
    assert_ne!(
        original,
        forever_casc::integrity::digest(changed.render().as_bytes())
    );
    changed = plan.clone();
    changed.offmesh.as_mut().unwrap().push(connection());
    assert_ne!(
        original,
        forever_casc::integrity::digest(changed.render().as_bytes())
    );
    let mut ordered = plan.clone();
    ordered.offmesh.as_mut().unwrap().push(OffMeshConnection {
        direction: 1,
        ..connection()
    });
    changed = ordered.clone();
    changed.offmesh.as_mut().unwrap().reverse();
    assert_ne!(
        forever_casc::integrity::digest(ordered.render().as_bytes()),
        forever_casc::integrity::digest(changed.render().as_bytes())
    );
}
