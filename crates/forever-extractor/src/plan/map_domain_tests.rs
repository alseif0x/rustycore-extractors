// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-13: bounded selected-domain grammar evidence only.
use super::*;

const V8: &str = concat!(
    "forever-product\t8\nbuild\t70205\nlocale\tenUS\nindex\tcontiguous\n",
    "root\theader-50893-v2\ndecryption\tdisabled\n",
    "collision-storage\tlinux-leased-append-disk-world-v1\n",
    "collision-raw-bytes\t1024\ncollision-wdt-bytes\t1024\n",
    "collision-cache-bytes\t1024\ncollision-placement-bytes\t1024\n",
    "collision-vmo-bytes\t1024\ncollision-map-bytes\t1024\ncollision-output-bytes\t1024\n",
    "map-domain\tmap-and-ancestors-v1\nmap-id\t42\n",
    "gameobject-scope\tglobal-startup-v1\nplacement-ids\tselected-closure-serial-ids-v1\n",
    "components\tdb2,maps,vmaps\nterrain\tmap:42\npack\ttrue\nnavigation\tnone\n",
    "big-base-unit\tfalse\nskip-liquids\tfalse\nmaximum-slope\t55\nnon-steep-slope\t55\n",
    "db2\tMap\t1349477\n",
);
fn plan() -> Plan {
    Plan::parse(V8).unwrap()
}
fn navigation() -> Plan {
    let mut plan = plan();
    plan.components |= Component::Navigation.bit();
    plan.navigation = Selection::Map(42);
    plan.db2.insert(
        0,
        Entry {
            name: "LiquidType".into(),
            id: catalog::find("LiquidType").unwrap().file_data_id,
        },
    );
    plan.offmesh = Some(vec![]);
    plan
}

#[test]
fn v8_canonical_map_only_roundtrip_and_scope() {
    let plan = plan();
    assert_eq!(plan.render(), V8);
    assert_eq!(plan.version(), 8);
    assert_eq!(plan.contract(), "forever-product-v8");
    assert_eq!(plan.map_domain, Some(42));
    assert!(!plan.requires_keys());
    assert_eq!(MAP_DOMAIN_PROFILE, "map-and-ancestors-v1");
    assert_eq!(GAMEOBJECT_SCOPE_PROFILE, "global-startup-v1");
    assert_eq!(
        PLACEMENT_ID_DOMAIN_PROFILE,
        "selected-closure-serial-ids-v1"
    );
}
#[test]
fn v8_both_builds_locales_layouts_and_key_modes_roundtrip() {
    for build in [70170, 70205] {
        for locale in [Locale::EnUs, Locale::EsEs] {
            for decryption in [Decryption::Disabled, Decryption::ProvidedSalsa20V1] {
                for index_layout in [local_index::Layout::Contiguous, local_index::Layout::Paged] {
                    let plan = Plan {
                        build,
                        locale,
                        decryption,
                        index_layout,
                        ..plan()
                    };
                    assert_eq!(plan.validate(), Ok(()));
                    assert_eq!(Plan::parse(&plan.render()), Ok(plan));
                }
            }
        }
    }
}
#[test]
fn v8_requested_navigation_has_explicit_empty_offmesh() {
    let plan = navigation();
    assert_eq!(plan.validate(), Ok(()));
    assert_eq!(Plan::parse(&plan.render()), Ok(plan.clone()));
    assert!(
        plan.render()
            .contains("non-steep-slope\t55\noffmesh\tinline-f32-bits-v1\n")
    );
}
#[test]
fn v8_requested_connections_preserve_bits_order_and_duplicates() {
    let mut plan = navigation();
    let connection = offmesh_tests::connection();
    plan.offmesh = Some(vec![connection, connection]);
    assert_eq!(Plan::parse(&plan.render()), Ok(plan));
}
#[test]
fn v8_domain_fields_are_mandatory_canonical_and_ordered() {
    for line in [
        "map-domain\tmap-and-ancestors-v1\n",
        "map-id\t42\n",
        "gameobject-scope\tglobal-startup-v1\n",
        "placement-ids\tselected-closure-serial-ids-v1\n",
    ] {
        for changed in [
            V8.replace(line, ""),
            V8.replace(line, &format!("{line}{line}")),
        ] {
            assert!(Plan::parse(&changed).is_err());
        }
    }
    let swapped = V8.replace(
        "map-domain\tmap-and-ancestors-v1\nmap-id\t42\n",
        "map-id\t42\nmap-domain\tmap-and-ancestors-v1\n",
    );
    assert!(Plan::parse(&swapped).is_err());
}
#[test]
fn v8_unknown_profiles_and_noncanonical_map_ids_reject() {
    for (before, after, error) in [
        ("map-and-ancestors-v1", "unknown", "product_plan_map_domain"),
        (
            "global-startup-v1",
            "static-only",
            "product_plan_gameobject_scope",
        ),
        (
            "selected-closure-serial-ids-v1",
            "full-world-ids",
            "product_plan_placement_ids",
        ),
        ("map-id\t42", "map-id\t042", "product_plan_decimal"),
        ("map-id\t42", "map-id\t4294967296", "product_plan_decimal"),
    ] {
        assert_eq!(Plan::parse(&V8.replace(before, after)).unwrap_err(), error);
    }
}
#[test]
fn reserved_v7_and_relabeled_legacy_headers_reject() {
    for version in [1, 2, 3, 4, 5, 6, 7] {
        assert!(
            Plan::parse(&V8.replace("forever-product\t8", &format!("forever-product\t{version}")))
                .is_err()
        );
    }
    assert_eq!(
        Plan::parse(&V8.replace("forever-product\t8", "forever-product\t7")).unwrap_err(),
        "product_plan_version"
    );
}
#[test]
fn v8_requires_existing_disk_profile_and_pinned_authorized_build() {
    for plan in [
        Plan {
            collision_storage: CollisionStorage::Resident,
            ..plan()
        },
        Plan {
            build: 70338,
            ..plan()
        },
        Plan {
            schema_catalog: SchemaCatalog::WowDbDefs70205V1,
            ..plan()
        },
    ] {
        assert_eq!(plan.validate(), Err("product_plan_map_domain_scope"));
    }
}
#[test]
fn v8_requires_geometry_components_and_complete_map_declaration() {
    for component in [Component::Db2, Component::Terrain, Component::Collision] {
        let mut plan = plan();
        plan.components &= !component.bit();
        assert_eq!(plan.validate(), Err("product_plan_map_domain_dependencies"));
    }
    let mut plan = plan();
    plan.db2.clear();
    assert_eq!(plan.validate(), Err("product_plan_map_domain_dependencies"));
}
#[test]
fn v8_rejects_all_tile_or_divergent_terrain_and_navigation() {
    for terrain in [
        Selection::None,
        Selection::All,
        Selection::Map(7),
        Selection::Tile {
            map: 42,
            x: 7,
            y: 3,
        },
    ] {
        assert_eq!(
            Plan { terrain, ..plan() }.validate(),
            Err("product_plan_map_domain_selection")
        );
    }
    for navigation in [
        Selection::All,
        Selection::Map(7),
        Selection::Tile {
            map: 42,
            x: 7,
            y: 3,
        },
    ] {
        assert_eq!(
            Plan {
                navigation,
                ..plan()
            }
            .validate(),
            Err("product_plan_map_domain_selection")
        );
    }
}
#[test]
fn v8_navigation_requires_liquid_type_and_navigation_component() {
    let mut plan = navigation();
    plan.db2.retain(|entry| entry.name != "LiquidType");
    assert_eq!(plan.validate(), Err("product_plan_dependencies"));
    let mut plan = navigation();
    plan.components &= !Component::Navigation.bit();
    assert!(plan.validate().is_err());
}
#[test]
fn v8_offmesh_scope_is_fixed_by_navigation_and_requested_map() {
    let mut nav = navigation();
    nav.offmesh = None;
    assert_eq!(nav.validate(), Err("product_plan_map_domain_offmesh"));
    let mut plain = plan();
    plain.offmesh = Some(vec![]);
    assert_eq!(plain.validate(), Err("product_plan_map_domain_offmesh"));
    let mut nav = navigation();
    let mut other = offmesh_tests::connection();
    other.map_id = 7;
    nav.offmesh = Some(vec![other]);
    assert_eq!(nav.validate(), Err("product_plan_map_domain_offmesh"));
}
#[test]
fn v8_zero_and_full_u32_map_ids_are_explicit_plan_facts() {
    for id in [0, u32::MAX] {
        let plan = Plan {
            map_domain: Some(id),
            terrain: Selection::Map(id),
            ..plan()
        };
        assert_eq!(Plan::parse(&plan.render()), Ok(plan));
    }
}
#[test]
fn v8_plan_fingerprint_binds_requested_map_and_policies() {
    let first = plan();
    let second = Plan {
        map_domain: Some(7),
        terrain: Selection::Map(7),
        ..first.clone()
    };
    assert_ne!(
        forever_casc::integrity::digest(first.render().as_bytes()),
        forever_casc::integrity::digest(second.render().as_bytes())
    );
    let legacy = Plan {
        map_domain: None,
        terrain: Selection::All,
        ..first
    };
    assert_eq!(legacy.version(), 3);
    assert!(!legacy.render().contains("map-domain"));
    assert!(!legacy.render().contains("placement-ids"));
}
#[test]
fn v8_optional_gt_cameras_keep_original_dependencies() {
    let mut plan = plan();
    plan.components |= Component::GameTables.bit() | Component::Cameras.bit();
    plan.db2.insert(
        0,
        Entry {
            name: "CinematicCamera".into(),
            id: catalog::find("CinematicCamera").unwrap().file_data_id,
        },
    );
    assert_eq!(Plan::parse(&plan.render()), Ok(plan.clone()));
    plan.db2.remove(0);
    assert_eq!(plan.validate(), Err("product_plan_dependencies"));
}

#[test]
fn legacy_v1_to_v6_exact_render_bytes_and_fingerprints_are_preserved() {
    for (version, text, digest) in [
        (
            1,
            "forever-product\t1\nbuild\t70205\nlocale\tenUS\nindex\tcontiguous\nroot\theader-50893-v2\ncomponents\tdb2\nterrain\tnone\npack\ttrue\nnavigation\tnone\nbig-base-unit\tfalse\nskip-liquids\tfalse\nmaximum-slope\t55\nnon-steep-slope\t55\ndb2\tMap\t1349477\n",
            "05c6b1190c5128d0e738a679bc724527978ccaad97437665492a10f229f93402",
        ),
        (
            2,
            "forever-product\t2\nbuild\t70205\nlocale\tenUS\nindex\tcontiguous\nroot\theader-50893-v2\ndecryption\tprovided-salsa20-v1\ncomponents\tdb2\nterrain\tnone\npack\ttrue\nnavigation\tnone\nbig-base-unit\tfalse\nskip-liquids\tfalse\nmaximum-slope\t55\nnon-steep-slope\t55\ndb2\tMap\t1349477\n",
            "a40430ebf216d879a059ebd9ac88116c1e7b232aa1060b05283411c3d8282c2b",
        ),
        (
            3,
            "forever-product\t3\nbuild\t70205\nlocale\tenUS\nindex\tcontiguous\nroot\theader-50893-v2\ndecryption\tdisabled\ncollision-storage\tlinux-leased-append-disk-world-v1\ncollision-raw-bytes\t1024\ncollision-wdt-bytes\t1024\ncollision-cache-bytes\t1024\ncollision-placement-bytes\t1024\ncollision-vmo-bytes\t1024\ncollision-map-bytes\t1024\ncollision-output-bytes\t1024\ncomponents\tdb2,vmaps\nterrain\tnone\npack\ttrue\nnavigation\tnone\nbig-base-unit\tfalse\nskip-liquids\tfalse\nmaximum-slope\t55\nnon-steep-slope\t55\ndb2\tMap\t1349477\n",
            "cf31ebff658c8ca69436ff317cdc61d6a52d5236a279f898abd0bda01f3923bd",
        ),
        (
            4,
            "forever-product\t4\nbuild\t70205\nlocale\tenUS\nindex\tcontiguous\nroot\theader-50893-v2\nschema-catalog\twowdbdefs-70205-v1\ndecryption\tdisabled\ncomponents\tdb2\nterrain\tnone\npack\ttrue\nnavigation\tnone\nbig-base-unit\tfalse\nskip-liquids\tfalse\nmaximum-slope\t55\nnon-steep-slope\t55\ndb2\tMap\t1349477\n",
            "a976067adfc7b261b746d229fed0d300a159c915155db150288776cade6aefc4",
        ),
        (
            5,
            "forever-product\t5\nbuild\t70338\nlocale\tenUS\nindex\tcontiguous\nroot\theader-50893-v2\nschema-catalog\twowdbdefs-70338-v1\ndecryption\tdisabled\ncomponents\tdb2\nterrain\tnone\npack\ttrue\nnavigation\tnone\nbig-base-unit\tfalse\nskip-liquids\tfalse\nmaximum-slope\t55\nnon-steep-slope\t55\ndb2\tMap\t1349477\n",
            "30abb8c4bcd41f493b186ac7592af1a95d0b97ec5b33f6661dcf22df14439033",
        ),
        (
            6,
            "forever-product\t6\nbuild\t70205\nlocale\tenUS\nindex\tcontiguous\nroot\theader-50893-v2\ndecryption\tdisabled\ncomponents\tdb2,maps,vmaps,mmaps\nterrain\tall\npack\ttrue\nnavigation\tall\nbig-base-unit\tfalse\nskip-liquids\tfalse\nmaximum-slope\t55\nnon-steep-slope\t55\noffmesh\tinline-f32-bits-v1\ndb2\tLiquidType\t1371380\ndb2\tMap\t1349477\n",
            "ff597322872e7bdb3a57450b9f5812fb1f97d498b6cc2a97ab756b159a742c95",
        ),
    ] {
        let plan = Plan::parse(text).unwrap();
        assert_eq!(plan.map_domain, None);
        assert_eq!(plan.version(), version);
        assert_eq!(plan.render(), text);
        assert_eq!(
            forever_casc::integrity::hex(&forever_casc::integrity::digest(
                plan.render().as_bytes()
            )),
            digest
        );
    }
}
