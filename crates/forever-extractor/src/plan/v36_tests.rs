// SPDX-License-Identifier: GPL-3.0-or-later
//! Synthetic build-70450 material-filtered-WMO geometry plan admission.
use super::*;

const ALL: &str = include_str!("../../../../tests/contracts/fixtures/forever-product-v36/all.tsv");
const V35: &str = include_str!("../../../../tests/contracts/fixtures/forever-product-v35/all.tsv");
const V34: &str = include_str!("../../../../tests/contracts/fixtures/forever-product-v34/all.tsv");
const V22: &str = include_str!("../../../../tests/contracts/fixtures/forever-product-v22/map2.tsv");

fn plan() -> Plan {
    Plan::parse(ALL).unwrap()
}

#[test]
fn v36_admits_exact_material_filtered_wmo_profile_and_canonical_roundtrip() {
    let product = plan();
    assert_eq!(product.version(), 36);
    assert_eq!(product.contract(), "forever-product-v36");
    assert_eq!(
        product.publication_profile,
        PublicationProfile::Build70450MaterialFilteredWmoGeometryV1
    );
    assert!(product.is_v36());
    assert!(!product.is_v35());
    assert!(!product.is_v34());
    assert!(!product.has_selected_geometry());
    assert_eq!(product.build, 70450);
    assert_eq!(product.locale.name(), "esES");
    assert_eq!(product.schema_catalog, SchemaCatalog::WowDbDefs70450V1);
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
fn v36_requires_exact_plan_grammar_and_declared_skip_gate() {
    let lines: Vec<_> = ALL.lines().collect();
    assert_eq!(lines.len(), 30);
    assert_eq!(lines[0], "forever-product\t36");
    assert_eq!(lines[6], "decryption\tdisabled");
    assert_eq!(lines[7], "partial_encrypted_sections\tdeclared-skip");
    assert_eq!(
        lines[8],
        "collision-storage\tlinux-leased-append-disk-world-v1"
    );
    assert_eq!(lines[29], "db2\tMap\t1349477");
    assert!(ALL.ends_with('\n'));
    assert!(!ALL.contains("requested_map"));
    assert_eq!(
        Plan::parse(&ALL.replace("partial_encrypted_sections\tdeclared-skip\n", "")),
        Err("product_plan_partial_encrypted_sections")
    );
    assert!(Plan::parse(&ALL.replace("declared-skip", "allow")).is_err());
    let moved = ALL
        .replace("partial_encrypted_sections\tdeclared-skip\n", "")
        .replace(
            "collision-profile\t",
            "partial_encrypted_sections\tdeclared-skip\ncollision-profile\t",
        );
    assert!(Plan::parse(&moved).is_err());
}

#[test]
fn v36_binds_build_catalog_mask_decryption_and_single_map() {
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
    assert_eq!(
        Plan {
            decryption: Decryption::ProvidedSalsa20V1,
            ..base.clone()
        }
        .validate(),
        Err("product_plan_component_scope")
    );
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
            db2: vec![Entry {
                name: "SpellEffect".into(),
                id: 1_140_088,
            }],
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
fn v36_requires_all_available_maps_leased_disk_and_unchanged_options() {
    let base = plan();
    for domain in [
        "map:0",
        "map:2",
        "map:48",
        "all",
        "map:02",
        "map:4294967296",
    ] {
        assert!(
            Plan::parse(&ALL.replace(
                "collision-domain\tall-available-maps",
                &format!("collision-domain\t{domain}")
            ))
            .is_err()
        );
    }
    for requested in [0, 2, 48, u32::MAX] {
        assert_eq!(
            Plan {
                map_domain: Some(requested),
                ..base.clone()
            }
            .validate(),
            Err("product_plan_component_scope")
        );
    }
    assert_eq!(
        Plan {
            collision_storage: CollisionStorage::Resident,
            ..base.clone()
        }
        .validate(),
        Err("product_plan_component_scope")
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
fn v34_v35_and_v36_profiles_versions_and_coverage_cannot_cross() {
    let v34 = Plan::parse(V34).unwrap();
    let v35 = Plan::parse(V35).unwrap();
    let v22 = Plan::parse(V22).unwrap();
    assert!(v34.is_v34());
    assert!(v35.is_v35());
    assert!(v22.is_v22());
    assert!(plan().is_v36());
    assert!(Plan::parse(&ALL.replace("forever-product\t36", "forever-product\t35")).is_err());
    assert!(Plan::parse(&V35.replace("forever-product\t35", "forever-product\t36")).is_err());
    assert!(Plan::parse(&V34.replace("forever-product\t34", "forever-product\t36")).is_err());
    for (from, to) in [
        (
            "linux70450-material-filtered-wmo-vmap-product-v1",
            "linux70450-available-unselected-wmo-vmap-product-v1",
        ),
        (
            "linux70450-material-filtered-wmo-coverage-v9",
            "linux70450-available-unselected-wmo-coverage-v8",
        ),
        (
            "linux70450-material-filtered-wmo-vmap-product-v1",
            "linux70450-available-empty-wmo-vmap-product-v1",
        ),
        (
            "linux70450-material-filtered-wmo-coverage-v9",
            "linux70450-available-empty-wmo-coverage-v7",
        ),
    ] {
        assert!(Plan::parse(&ALL.replace(from, to)).is_err(), "{from}");
    }
}

#[test]
fn v36_identity_literals_are_distinct_and_exact() {
    assert_eq!(
        MATERIAL_FILTERED_WMO_COLLISION_PROFILE,
        "linux70450-material-filtered-wmo-vmap-product-v1"
    );
    assert_eq!(
        MATERIAL_FILTERED_WMO_COVERAGE_PROFILE,
        "linux70450-material-filtered-wmo-coverage-v9"
    );
    assert_eq!(
        MATERIAL_FILTERED_WMO_VERIFY_PROFILE,
        "linux70450-material-filtered-wmo-verification-v1"
    );
    assert_eq!(
        MATERIAL_FILTERED_WMO_VERIFY_SCOPE,
        "artifact_integrity_material_filtered_wmo_and_independently_derived_map_vmap_domain_coverage"
    );
}

#[test]
fn v36_profile_rejects_wrong_identity_and_scope_literals() {
    for (from, to, error) in [
        (
            MATERIAL_FILTERED_WMO_COLLISION_PROFILE,
            UNSELECTED_WMO_COLLISION_PROFILE,
            "product_plan_collision_profile",
        ),
        (
            MATERIAL_FILTERED_WMO_COVERAGE_PROFILE,
            UNSELECTED_WMO_COVERAGE_PROFILE,
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
fn v22_v34_and_v35_keep_their_original_versions_and_canonical_plans() {
    for (source, version, contract) in [
        (V22, 22, "forever-product-v22"),
        (V34, 34, "forever-product-v34"),
        (V35, 35, "forever-product-v35"),
    ] {
        let product = Plan::parse(source).unwrap();
        assert_eq!(product.version(), version);
        assert_eq!(product.contract(), contract);
        assert!(!product.is_v36());
        assert_eq!(product.render(), source);
    }
}
