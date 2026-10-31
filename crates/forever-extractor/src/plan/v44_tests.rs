// SPDX-License-Identifier: GPL-3.0-or-later
//! Synthetic build-70450 SourceExpandedV2 material-filtered-WMO plan admission.
use super::*;

const ALL: &str = include_str!("../../../../tests/contracts/fixtures/forever-product-v44/all.tsv");
const V43: &str = include_str!("../../../../tests/contracts/fixtures/forever-product-v43/all.tsv");
const V42: &str = include_str!("../../../../tests/contracts/fixtures/forever-product-v42/all.tsv");

fn plan() -> Plan {
    Plan::parse(ALL).unwrap()
}

#[test]
fn v44_admits_exact_source_expanded_v2_profile_and_canonical_roundtrip() {
    let product = plan();
    assert_eq!(product.version(), 44);
    assert_eq!(product.contract(), "forever-product-v44");
    assert_eq!(
        product.publication_profile,
        PublicationProfile::Build70450SourceExpandedV2MaterialFilteredWmoGeometryV1
    );
    assert!(product.is_v44());
    assert!(!product.is_v43());
    assert!(!product.is_v42());
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
        SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_COLLISION_PROFILE,
        "linux70450-source-expanded-v2-material-filtered-wmo-vmap-product-v1"
    );
    assert_eq!(
        SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE,
        "linux70450-source-expanded-v2-material-filtered-wmo-coverage-v17"
    );
    assert_eq!(
        SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_VERIFY_PROFILE,
        "linux70450-source-expanded-v2-material-filtered-wmo-verification-v1"
    );
    assert_eq!(
        SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_VERIFY_SCOPE,
        "artifact_integrity_source_expanded_v2_material_filtered_wmo_and_independently_derived_map_vmap_domain_coverage"
    );
}

#[test]
fn v44_requires_exact_30_line_grammar_and_v17_identity() {
    let lines: Vec<_> = ALL.lines().collect();
    assert_eq!(lines.len(), 30);
    assert_eq!(lines[0], "forever-product\t44");
    assert_eq!(lines[7], "partial_encrypted_sections\tdeclared-skip");
    assert_eq!(
        lines[16],
        "collision-profile\tlinux70450-source-expanded-v2-material-filtered-wmo-vmap-product-v1"
    );
    assert_eq!(
        lines[18],
        "collision-coverage\tlinux70450-source-expanded-v2-material-filtered-wmo-coverage-v17"
    );
    assert!(ALL.ends_with('\n'));
    assert_eq!(
        Plan::parse(&ALL.replace("partial_encrypted_sections\tdeclared-skip\n", "")),
        Err("product_plan_partial_encrypted_sections")
    );
    for (from, to) in [
        (
            "linux70450-source-expanded-v2-material-filtered-wmo-vmap-product-v1",
            "linux70450-model-bytes-expanded-material-filtered-wmo-vmap-product-v1",
        ),
        (
            "linux70450-source-expanded-v2-material-filtered-wmo-coverage-v17",
            "linux70450-model-bytes-expanded-material-filtered-wmo-coverage-v16",
        ),
        (
            "linux70450-source-expanded-v2-material-filtered-wmo-vmap-product-v1",
            "linux70450-source-expanded-material-filtered-wmo-vmap-product-v1",
        ),
    ] {
        assert!(Plan::parse(&ALL.replace(from, to)).is_err(), "{to}");
    }
    assert!(Plan::parse(&ALL.replace("forever-product\t44", "forever-product\t43")).is_err());
    assert!(Plan::parse(&V43.replace("forever-product\t43", "forever-product\t44")).is_err());
    assert!(Plan::parse(&V42.replace("forever-product\t42", "forever-product\t44")).is_err());
}

#[test]
fn v44_binds_build_catalog_components_and_profile_options() {
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
    let v43 = Plan {
        publication_profile:
            PublicationProfile::Build70450ModelBytesExpandedMaterialFilteredWmoGeometryV1,
        ..base.clone()
    };
    assert!(v43.validate().is_ok());
    assert!(v43.is_v43());
    assert!(!v43.is_v44());
    assert_eq!(v43.contract(), "forever-product-v43");
    assert!(v43.render().starts_with("forever-product\t43\n"));
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
            map_domain: Some(2),
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

#[cfg(target_os = "linux")]
#[test]
fn v44_uses_source_expanded_v2_limits_without_changing_other_capacities() {
    use forever_casc::collision::resources::Profile;

    let profile = Profile::SourceExpandedV2;
    let v43 = Profile::ModelBytesExpandedV1;
    assert_eq!(profile.id(), "linux-collision-source-expanded-resources-v2");
    assert_eq!(profile.collector_limits().max_source_bytes, 2_147_483_648);
    assert_eq!(profile.collector_limits().max_raw_bytes, 268_435_456);
    assert_eq!(profile.collector_limits().max_models, 65_536);
    assert_eq!(profile.collector_limits().max_reads, 1_000_000);
    assert_eq!(profile.collector_limits().max_requests, 16_000_000);
    assert_eq!(
        profile.collector_limits().max_dependency_requests,
        16_000_000
    );
    let mut expected = v43.collector_limits();
    expected.max_source_bytes = profile.collector_limits().max_source_bytes;
    assert_eq!(profile.collector_limits(), expected);

    let mut startup = v43.startup_limits();
    let v2_startup = profile.startup_limits();
    startup.collision.max_source_bytes = v2_startup.collision.max_source_bytes;
    assert_eq!(v2_startup, startup);
    assert_eq!(
        profile.stage_limits().world.world.run.placements.max_models,
        65_536
    );
    assert_eq!(profile.model_limits().max_models, 65_536);
    assert_eq!(profile.package_limits().max_models, 65_536);

    let mut reduced = plan();
    let CollisionStorage::LeasedDiskV1(mut budget) = reduced.collision_storage else {
        panic!("v44 requires leased disk storage");
    };
    budget.raw_bytes = 2 * 1024 * 1024;
    budget.vmo_bytes = 512 * 1024;
    budget.output_bytes = 512 * 1024;
    reduced.collision_storage = CollisionStorage::LeasedDiskV1(budget);
    assert!(reduced.validate().is_ok());
    assert_eq!(Plan::parse(&reduced.render()), Ok(reduced));
}

#[cfg(test)]
mod fixture_tests {
    #[test]
    fn v44_fixture_is_a_separate_canonical_file() {
        let fixture =
            include_str!("../../../../tests/contracts/fixtures/forever-product-v44/all.tsv");
        assert_eq!(fixture.lines().count(), 30);
        assert!(fixture.starts_with("forever-product\t44\n"));
        assert!(
            fixture.contains("linux70450-source-expanded-v2-material-filtered-wmo-coverage-v17")
        );
    }
}
