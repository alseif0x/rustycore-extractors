// SPDX-License-Identifier: GPL-3.0-or-later
//! Synthetic build-70450 authenticated all-active-map geometry plan admission.
use super::*;

const ALL: &str = include_str!("../../../../tests/contracts/fixtures/forever-product-v29/all.tsv");
const V22: &str = include_str!("../../../../tests/contracts/fixtures/forever-product-v22/map2.tsv");
const V28: &str = include_str!("../../../../tests/contracts/fixtures/forever-product-v28/map2.tsv");

fn plan() -> Plan {
    Plan::parse(ALL).unwrap()
}

#[test]
fn v29_admits_exact_all_active_profile_and_canonical_roundtrip() {
    let product = plan();
    assert_eq!(product.version(), 29);
    assert_eq!(product.contract(), "forever-product-v29");
    assert_eq!(
        product.publication_profile,
        PublicationProfile::Build70450AuthenticatedAvailableGeometryV1
    );
    assert!(product.is_v29());
    assert!(!product.has_selected_geometry());
    assert!(!product.is_v22());
    assert!(!product.is_v28());
    assert_eq!(product.build, 70450);
    assert_eq!(product.locale.name(), "esES");
    assert_eq!(product.schema_catalog, SchemaCatalog::WowDbDefs70450V1);
    assert_eq!(
        product.components,
        Component::Db2.bit() | Component::Collision.bit()
    );
    assert_eq!(product.components, 17);
    assert_eq!(
        product.db2,
        [Entry {
            name: "Map".into(),
            id: 1_349_477,
        }]
    );
    assert_eq!(product.decryption, Decryption::Disabled);
    assert!(!product.requires_keys());
    assert_eq!(product.map_domain, None);
    assert_eq!(product.terrain, Selection::None);
    assert_eq!(product.navigation, Selection::None);
    assert_eq!(product.offmesh, None);
    assert!(product.pack);
    assert!(!product.big_base_unit);
    assert!(!product.skip_liquids);
    assert_eq!((product.maximum_slope, product.non_steep_slope), (55, 55));
    assert_eq!(product.render(), ALL);
    assert_eq!(Plan::parse(&product.render()), Ok(product));
}

#[test]
fn v29_requires_the_partial_gate_between_decryption_and_collision_storage() {
    assert!(ALL.contains(
        "decryption\tdisabled\npartial_encrypted_sections\tdeclared-skip\ncollision-storage\tlinux-leased-append-disk-world-v1\n"
    ));
    assert_eq!(
        Plan::parse(&ALL.replace("partial_encrypted_sections\tdeclared-skip\n", "")),
        Err("product_plan_partial_encrypted_sections")
    );
    assert_eq!(
        Plan::parse(&ALL.replace(
            "partial_encrypted_sections\tdeclared-skip",
            "partial_encrypted_sections\tallow",
        )),
        Err("product_plan_partial_encrypted_sections")
    );
    let moved = ALL
        .replace("partial_encrypted_sections\tdeclared-skip\n", "")
        .replace(
            "collision-profile\t",
            "partial_encrypted_sections\tdeclared-skip\ncollision-profile\t",
        );
    assert!(Plan::parse(&moved).is_err());
}

#[test]
fn v29_rejects_every_other_component_mask_and_provided_decryption() {
    let base = plan();
    assert_eq!(
        Plan {
            decryption: Decryption::ProvidedSalsa20V1,
            ..base.clone()
        }
        .validate(),
        Err("product_plan_component_scope")
    );
    for mask in 0..=u8::MAX {
        if mask != 17 {
            assert_eq!(
                Plan {
                    components: mask,
                    ..base.clone()
                }
                .validate(),
                Err("product_plan_component_scope"),
                "mask {mask}"
            );
        }
    }
}

#[test]
fn v29_requires_exactly_one_canonical_map_db2_entry() {
    let base = plan();
    for changed in [
        Plan {
            db2: Vec::new(),
            ..base.clone()
        },
        Plan {
            db2: vec![base.db2[0].clone(), base.db2[0].clone()],
            ..base.clone()
        },
        Plan {
            db2: {
                let mut entries = base.db2.clone();
                entries.push(Entry {
                    name: "SpellEffect".into(),
                    id: 1_140_088,
                });
                entries
            },
            ..base.clone()
        },
    ] {
        assert_eq!(changed.validate(), Err("product_plan_dependencies"));
    }

    let mut wrong_map = base;
    wrong_map.db2[0].id += 1;
    assert_eq!(wrong_map.validate(), Err("product_plan_schema"));
}

#[test]
fn v29_requires_all_available_maps_without_a_requested_map_or_sentinel() {
    assert_eq!(plan().map_domain, None);
    for domain in [
        "map:0",
        "map:2",
        "map:48",
        "all",
        "map:02",
        "map:4294967296",
    ] {
        let text = ALL.replace(
            "collision-domain\tall-available-maps",
            &format!("collision-domain\t{domain}"),
        );
        assert!(Plan::parse(&text).is_err(), "domain {domain}");
    }

    let base = plan();
    for requested in [0, 2, 48, u32::MAX] {
        assert_eq!(
            Plan {
                map_domain: Some(requested),
                ..base.clone()
            }
            .validate(),
            Err("product_plan_component_scope"),
            "requested map {requested}"
        );
    }
}

#[test]
fn v29_requires_leased_disk_with_all_seven_budget_fields() {
    let product = plan();
    assert_eq!(
        product.collision_storage,
        CollisionStorage::LeasedDiskV1(DiskBudget {
            raw_bytes: 1_048_576,
            wdt_bytes: 1_048_576,
            cache_bytes: 1_048_576,
            placement_bytes: 1_048_576,
            vmo_bytes: 1_048_576,
            map_bytes: 1_048_576,
            output_bytes: 1_048_576,
        })
    );
    for field in [
        "collision-raw-bytes\t1048576\n",
        "collision-wdt-bytes\t1048576\n",
        "collision-cache-bytes\t1048576\n",
        "collision-placement-bytes\t1048576\n",
        "collision-vmo-bytes\t1048576\n",
        "collision-map-bytes\t1048576\n",
        "collision-output-bytes\t1048576\n",
    ] {
        assert_eq!(product.render().matches(field).count(), 1, "{field}");
    }
    assert_eq!(
        Plan {
            collision_storage: CollisionStorage::Resident,
            ..product
        }
        .validate(),
        Err("product_plan_component_scope")
    );
}

#[test]
fn v29_admission_binds_build_catalog_and_unchanged_options() {
    let base = plan();
    assert_eq!(
        Plan {
            build: 70425,
            ..base.clone()
        }
        .validate(),
        Err("product_plan_build")
    );
    assert_eq!(
        Plan {
            schema_catalog: SchemaCatalog::WowDbDefs70425V1,
            ..base.clone()
        }
        .validate(),
        Err("product_plan_schema_catalog")
    );
    for changed in [
        Plan {
            terrain: Selection::All,
            ..base.clone()
        },
        Plan {
            terrain: Selection::Map(2),
            ..base.clone()
        },
        Plan {
            navigation: Selection::Map(2),
            ..base.clone()
        },
        Plan {
            offmesh: Some(Vec::new()),
            ..base.clone()
        },
        Plan {
            pack: false,
            ..base.clone()
        },
        Plan {
            big_base_unit: true,
            ..base.clone()
        },
        Plan {
            skip_liquids: true,
            ..base.clone()
        },
        Plan {
            maximum_slope: 54,
            ..base.clone()
        },
        Plan {
            non_steep_slope: 54,
            ..base.clone()
        },
    ] {
        assert_eq!(changed.validate(), Err("product_plan_component_scope"));
    }
}

#[test]
fn v29_preserves_the_available_geometry_policy_literals() {
    for (from, to, error) in [
        (
            AVAILABLE_GEOMETRY_COLLISION_PROFILE,
            SELECTED_GEOMETRY_COLLISION_PROFILE,
            "product_plan_collision_profile",
        ),
        (
            AVAILABLE_GEOMETRY_COVERAGE_PROFILE,
            SELECTED_GEOMETRY_COVERAGE_PROFILE,
            "product_plan_collision_coverage",
        ),
        (
            SELECTED_GEOMETRY_GAMEOBJECT_SCOPE,
            "global-startup-v1",
            "product_plan_gameobject_scope",
        ),
        (
            SELECTED_GEOMETRY_MODEL_SCOPE,
            "global",
            "product_plan_model_scope",
        ),
    ] {
        assert_eq!(Plan::parse(&ALL.replace(from, to)), Err(error), "{from}");
    }
}

#[test]
fn v29_mask_17_does_not_permit_v28_profile_or_domain_crossing() {
    let selected = Plan::parse(V28).unwrap();
    assert_eq!(selected.components, plan().components);
    assert_eq!(
        Plan {
            publication_profile: PublicationProfile::Build70450AuthenticatedAvailableGeometryV1,
            ..selected
        }
        .validate(),
        Err("product_plan_component_scope")
    );
    assert_eq!(
        Plan {
            publication_profile: PublicationProfile::Build70450AuthenticatedSelectedGeometryV1,
            ..plan()
        }
        .validate(),
        Err("product_plan_component_scope")
    );
}

#[test]
fn v29_does_not_relabel_v22_or_v28_and_preserves_selected_geometry_dispatch() {
    let v22 = Plan::parse(V22).unwrap();
    assert_eq!(v22.version(), 22);
    assert_eq!(v22.contract(), "forever-product-v22");
    assert_eq!(
        v22.publication_profile,
        PublicationProfile::Build70450SelectedGeometryV1
    );
    assert_eq!(v22.components, Component::Collision.bit());
    assert_eq!(v22.map_domain, Some(2));
    assert!(v22.has_selected_geometry());
    assert!(!v22.is_v29());
    assert_eq!(v22.render(), V22);
    assert!(!v22.render().contains("partial_encrypted_sections"));

    let v28 = Plan::parse(V28).unwrap();
    assert_eq!(v28.version(), 28);
    assert_eq!(v28.contract(), "forever-product-v28");
    assert_eq!(
        v28.publication_profile,
        PublicationProfile::Build70450AuthenticatedSelectedGeometryV1
    );
    assert_eq!(v28.components, 17);
    assert_eq!(v28.map_domain, Some(2));
    assert!(v28.has_selected_geometry());
    assert!(!v28.is_v29());
    assert_eq!(v28.render(), V28);

    assert!(Plan::parse(&V22.replace("forever-product\t22", "forever-product\t29")).is_err());
    assert!(Plan::parse(&V28.replace("forever-product\t28", "forever-product\t29")).is_err());
    assert!(Plan::parse(&ALL.replace("forever-product\t29", "forever-product\t28")).is_err());
}
