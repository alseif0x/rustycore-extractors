// SPDX-License-Identifier: GPL-3.0-or-later
//! Synthetic build-70450 authenticated selected-geometry plan admission.
use super::*;

const MAP2: &str =
    include_str!("../../../../tests/contracts/fixtures/forever-product-v28/map2.tsv");
const MAP48: &str =
    include_str!("../../../../tests/contracts/fixtures/forever-product-v28/map48.tsv");
const V22: &str = include_str!("../../../../tests/contracts/fixtures/forever-product-v22/map2.tsv");
const V27: &str =
    include_str!("../../../../tests/contracts/fixtures/forever-product-v27/partial.tsv");

fn plan() -> Plan {
    Plan::parse(MAP2).unwrap()
}

#[test]
fn v28_admits_exact_profile_and_canonical_roundtrips() {
    for (source, map) in [(MAP2, 2), (MAP48, 48)] {
        let product = Plan::parse(source).unwrap();
        assert_eq!(product.version(), 28);
        assert_eq!(product.contract(), "forever-product-v28");
        assert_eq!(
            product.publication_profile,
            PublicationProfile::Build70450AuthenticatedSelectedGeometryV1
        );
        assert!(product.is_v28());
        assert!(product.has_selected_geometry());
        assert!(!product.is_v22());
        assert!(!product.is_v27());
        assert_eq!(product.build, 70450);
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
        assert_eq!(product.map_domain, Some(map));
        assert_eq!(product.terrain, Selection::None);
        assert_eq!(product.navigation, Selection::None);
        assert_eq!(product.offmesh, None);
        assert!(product.pack);
        assert!(!product.big_base_unit);
        assert!(!product.skip_liquids);
        assert_eq!((product.maximum_slope, product.non_steep_slope), (55, 55));
        assert_eq!(product.render(), source);
        assert_eq!(Plan::parse(&product.render()), Ok(product));
    }
}

#[test]
fn v28_requires_the_partial_gate_between_decryption_and_collision_storage() {
    assert!(MAP2.contains(
        "decryption\tdisabled\npartial_encrypted_sections\tdeclared-skip\ncollision-storage\tlinux-leased-append-disk-world-v1\n"
    ));
    assert_eq!(
        Plan::parse(&MAP2.replace("partial_encrypted_sections\tdeclared-skip\n", "")),
        Err("product_plan_partial_encrypted_sections")
    );
    assert_eq!(
        Plan::parse(&MAP2.replace(
            "partial_encrypted_sections\tdeclared-skip",
            "partial_encrypted_sections\tallow",
        )),
        Err("product_plan_partial_encrypted_sections")
    );
    let moved = MAP2
        .replace("partial_encrypted_sections\tdeclared-skip\n", "")
        .replace(
            "collision-profile\t",
            "partial_encrypted_sections\tdeclared-skip\ncollision-profile\t",
        );
    assert!(Plan::parse(&moved).is_err());
}

#[test]
fn v28_rejects_every_other_component_mask_and_provided_decryption() {
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
fn v28_requires_exactly_one_canonical_map_db2_entry() {
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
fn v28_requires_leased_disk_with_all_seven_budget_fields() {
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
fn v28_requires_a_canonical_selected_map_domain() {
    let base = plan();
    for domain in ["all-available-maps", "all", "map:02", "map:4294967296"] {
        let text = MAP2.replace(
            "collision-domain\tmap:2",
            &format!("collision-domain\t{domain}"),
        );
        assert!(Plan::parse(&text).is_err(), "domain {domain}");
    }
    let missing = Plan {
        map_domain: None,
        ..base
    };
    assert_eq!(missing.validate(), Err("product_plan_component_scope"));
    assert_eq!(missing.try_render(), Err("product_plan_map_domain"));
}

#[test]
fn v28_admission_binds_build_catalog_and_unchanged_options() {
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
fn v28_preserves_the_selected_geometry_policy_literals() {
    for (from, to, error) in [
        (
            SELECTED_GEOMETRY_COLLISION_PROFILE,
            COMPATIBLE_COLLISION_VERIFY_PROFILE_70450,
            "product_plan_collision_profile",
        ),
        (
            SELECTED_GEOMETRY_COVERAGE_PROFILE,
            "compatible-collision-coverage-v1",
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
        assert_eq!(Plan::parse(&MAP2.replace(from, to)), Err(error), "{from}");
    }
}

#[test]
fn v28_does_not_relabel_v22_or_v27_and_preserves_selected_geometry_dispatch() {
    let v22 = Plan::parse(V22).unwrap();
    assert_eq!(v22.version(), 22);
    assert_eq!(v22.contract(), "forever-product-v22");
    assert_eq!(
        v22.publication_profile,
        PublicationProfile::Build70450SelectedGeometryV1
    );
    assert_eq!(v22.components, Component::Collision.bit());
    assert!(v22.has_selected_geometry());
    assert!(!v22.is_v28());
    assert_eq!(v22.render(), V22);
    assert!(!v22.render().contains("partial_encrypted_sections"));

    let v27 = Plan::parse(V27).unwrap();
    assert_eq!(v27.version(), 27);
    assert_eq!(v27.contract(), "forever-product-v27");
    assert_eq!(
        v27.publication_profile,
        PublicationProfile::Build70450PartialCatalogTerrainV1
    );
    assert!(!v27.has_selected_geometry());
    assert!(!v27.is_v28());
    assert_eq!(v27.render(), V27);

    assert!(Plan::parse(&V22.replace("forever-product\t22", "forever-product\t28")).is_err());
    assert!(Plan::parse(&V27.replace("forever-product\t27", "forever-product\t28")).is_err());
}
