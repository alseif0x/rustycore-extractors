// SPDX-License-Identifier: GPL-3.0-or-later
//! Synthetic build-70450 model-output-expanded material-filtered-WMO admission.
use super::*;

const ALL: &str = include_str!("../../../../tests/contracts/fixtures/forever-product-v42/all.tsv");
const V41: &str = include_str!("../../../../tests/contracts/fixtures/forever-product-v41/all.tsv");
const V40: &str = include_str!("../../../../tests/contracts/fixtures/forever-product-v40/all.tsv");

fn plan() -> Plan {
    Plan::parse(ALL).unwrap()
}

#[test]
fn v42_admits_exact_model_output_expanded_profile_and_canonical_roundtrip() {
    let product = plan();
    assert_eq!(product.version(), 42);
    assert_eq!(product.contract(), "forever-product-v42");
    assert_eq!(
        product.publication_profile,
        PublicationProfile::Build70450ModelOutputExpandedMaterialFilteredWmoGeometryV1
    );
    assert!(product.is_v42());
    assert!(!product.is_v41());
    assert!(!product.is_v40());
    assert_eq!(product.build, 70450);
    assert_eq!(product.schema_catalog, SchemaCatalog::WowDbDefs70450V1);
    assert_eq!(
        product.components,
        Component::Db2.bit() | Component::Collision.bit()
    );
    assert_eq!(
        product.db2,
        [Entry {
            name: "Map".into(),
            id: 1_349_477
        }]
    );
    assert_eq!(product.decryption, Decryption::Disabled);
    assert_eq!(product.map_domain, None);
    assert_eq!(product.render(), ALL);
    assert_eq!(Plan::parse(&product.render()), Ok(product));
    assert_eq!(
        MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_COLLISION_PROFILE,
        "linux70450-model-output-expanded-material-filtered-wmo-vmap-product-v1"
    );
    assert_eq!(
        MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE,
        "linux70450-model-output-expanded-material-filtered-wmo-coverage-v15"
    );
    assert_eq!(
        MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_VERIFY_PROFILE,
        "linux70450-model-output-expanded-material-filtered-wmo-verification-v1"
    );
    assert_eq!(
        MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_VERIFY_SCOPE,
        "artifact_integrity_model_output_expanded_material_filtered_wmo_and_independently_derived_map_vmap_domain_coverage"
    );
}

#[test]
fn v42_requires_exact_30_line_grammar_and_declared_skip_gate() {
    let lines: Vec<_> = ALL.lines().collect();
    assert_eq!(lines.len(), 30);
    assert_eq!(lines[0], "forever-product\t42");
    assert_eq!(lines[7], "partial_encrypted_sections\tdeclared-skip");
    assert_eq!(
        lines[16],
        "collision-profile\tlinux70450-model-output-expanded-material-filtered-wmo-vmap-product-v1"
    );
    assert_eq!(
        lines[18],
        "collision-coverage\tlinux70450-model-output-expanded-material-filtered-wmo-coverage-v15"
    );
    assert_eq!(lines[29], "db2\tMap\t1349477");
    assert!(ALL.ends_with('\n'));
    assert_eq!(
        Plan::parse(&ALL.replace("partial_encrypted_sections\tdeclared-skip\n", "")),
        Err("product_plan_partial_encrypted_sections")
    );
    assert!(Plan::parse(&ALL.replace("declared-skip", "allow")).is_err());
    assert!(
        Plan::parse(&ALL.replace(
            "collision-domain\tall-available-maps",
            "collision-domain\tmap:2"
        ))
        .is_err()
    );
}

#[test]
fn v42_binds_build_catalog_components_map_and_profile_options() {
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
        if mask != Component::Db2.bit() | Component::Collision.bit() {
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
        Plan {
            decryption: Decryption::ProvidedSalsa20V1,
            ..base.clone()
        },
        Plan {
            collision_storage: CollisionStorage::Resident,
            ..base.clone()
        },
        Plan {
            terrain: Selection::All,
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
        assert!(changed.validate().is_err());
    }
}

#[test]
fn v42_identity_rejects_cross_version_profiles_and_keeps_prior_roundtrips() {
    let product = plan();
    let v41 = Plan::parse(V41).unwrap();
    let v40 = Plan::parse(V40).unwrap();
    assert!(Plan::parse(&ALL.replace("forever-product\t42", "forever-product\t41")).is_err());
    assert!(Plan::parse(&V41.replace("forever-product\t41", "forever-product\t42")).is_err());
    assert!(Plan::parse(&V40.replace("forever-product\t40", "forever-product\t42")).is_err());
    for (from, to) in [
        (
            "linux70450-model-output-expanded-material-filtered-wmo-vmap-product-v1",
            "linux70450-source-expanded-material-filtered-wmo-vmap-product-v1",
        ),
        (
            "linux70450-model-output-expanded-material-filtered-wmo-coverage-v15",
            "linux70450-source-expanded-material-filtered-wmo-coverage-v14",
        ),
        (
            "linux70450-model-output-expanded-material-filtered-wmo-vmap-product-v1",
            "linux70450-model-heavy-material-filtered-wmo-vmap-product-v1",
        ),
    ] {
        assert!(Plan::parse(&ALL.replace(from, to)).is_err(), "{from}");
    }
    assert_eq!(v41.version(), 41);
    assert_eq!(v41.render(), V41);
    assert_eq!(v40.version(), 40);
    assert_eq!(v40.render(), V40);
    assert!(product.is_v42());
    assert!(!v41.is_v42());
    assert!(!v40.is_v42());
}

#[cfg(target_os = "linux")]
#[test]
fn v42_model_output_expanded_capacities_are_profile_local() {
    use forever_casc::collision::resources::Profile;

    let profile = Profile::ModelOutputExpandedV1;
    let source_expanded = Profile::SourceExpandedV1;
    assert_eq!(
        profile.id(),
        "linux-collision-model-output-expanded-resources-v1"
    );
    assert_eq!(profile.collector_limits().max_source_bytes, 1_073_741_824);
    assert_eq!(profile.collector_limits().max_raw_bytes, 268_435_456);
    assert_eq!(profile.collector_limits().max_reads, 1_000_000);
    assert_eq!(profile.collector_limits().max_models, 65_536);
    assert_eq!(
        source_expanded.collector_limits(),
        profile.collector_limits()
    );
}

#[cfg(test)]
mod fixture_tests {
    #[test]
    fn v42_fixture_is_a_separate_canonical_file() {
        let fixture =
            include_str!("../../../../tests/contracts/fixtures/forever-product-v42/all.tsv");
        assert_eq!(fixture.lines().count(), 30);
        assert!(fixture.starts_with("forever-product\t42\n"));
        assert!(
            fixture.contains("linux70450-model-output-expanded-material-filtered-wmo-coverage-v15")
        );
    }
}
