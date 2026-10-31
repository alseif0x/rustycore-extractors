// SPDX-License-Identifier: GPL-3.0-or-later
//! Synthetic build-70450 model-heavy material-filtered-WMO plan admission.
use super::*;

const ALL: &str = include_str!("../../../../tests/contracts/fixtures/forever-product-v40/all.tsv");
const V39: &str = include_str!("../../../../tests/contracts/fixtures/forever-product-v39/all.tsv");
const V38: &str = include_str!("../../../../tests/contracts/fixtures/forever-product-v38/all.tsv");

fn plan() -> Plan {
    Plan::parse(ALL).unwrap()
}

#[test]
fn v40_admits_exact_model_heavy_profile_and_canonical_roundtrip() {
    let product = plan();
    assert_eq!(product.version(), 40);
    assert_eq!(product.contract(), "forever-product-v40");
    assert_eq!(
        product.publication_profile,
        PublicationProfile::Build70450ModelHeavyMaterialFilteredWmoGeometryV1
    );
    assert!(product.is_v40());
    assert!(!product.is_v39());
    assert!(!product.is_v38());
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
fn v40_requires_exact_plan_grammar_and_declared_skip_gate() {
    let lines: Vec<_> = ALL.lines().collect();
    assert_eq!(lines.len(), 30);
    assert_eq!(lines[0], "forever-product\t40");
    assert_eq!(lines[6], "decryption\tdisabled");
    assert_eq!(lines[7], "partial_encrypted_sections\tdeclared-skip");
    assert_eq!(
        lines[8],
        "collision-storage\tlinux-leased-append-disk-world-v1"
    );
    assert_eq!(
        lines[16],
        "collision-profile\tlinux70450-model-heavy-material-filtered-wmo-vmap-product-v1"
    );
    assert_eq!(
        lines[18],
        "collision-coverage\tlinux70450-model-heavy-material-filtered-wmo-coverage-v13"
    );
    assert_eq!(lines[29], "db2\tMap\t1349477");
    assert!(ALL.ends_with('\n'));
    assert!(!ALL.contains("requested_map"));
    for field in [
        "collision-source-bytes",
        "collision-raw-bytes-source-limit",
        "collision-read-limit",
    ] {
        assert!(!ALL.contains(field));
    }
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
fn v40_binds_build_catalog_mask_decryption_and_single_map() {
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
    let mut wrong_map = base.clone();
    wrong_map.db2[0].id += 1;
    assert_eq!(wrong_map.validate(), Err("product_plan_schema"));
}

#[test]
fn v40_requires_all_maps_leased_disk_and_unchanged_options() {
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
    assert_eq!(
        base.collision_storage,
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
}

#[test]
fn v40_identity_does_not_cross_v39_v38_or_prior_resource_profiles() {
    let product = plan();
    let v39 = Plan::parse(V39).unwrap();
    let v38 = Plan::parse(V38).unwrap();
    assert!(v39.is_v39());
    assert!(v38.is_v38());
    assert!(Plan::parse(&ALL.replace("forever-product\t40", "forever-product\t39")).is_err());
    assert!(Plan::parse(&V39.replace("forever-product\t39", "forever-product\t40")).is_err());
    assert!(Plan::parse(&V38.replace("forever-product\t38", "forever-product\t40")).is_err());
    for (from, to) in [
        (
            "linux70450-model-heavy-material-filtered-wmo-vmap-product-v1",
            "linux70450-read-heavy-material-filtered-wmo-vmap-product-v1",
        ),
        (
            "linux70450-model-heavy-material-filtered-wmo-coverage-v13",
            "linux70450-read-heavy-material-filtered-wmo-coverage-v12",
        ),
        (
            "linux70450-model-heavy-material-filtered-wmo-vmap-product-v1",
            "linux70450-source-heavy-material-filtered-wmo-vmap-product-v1",
        ),
        (
            "linux70450-model-heavy-material-filtered-wmo-coverage-v13",
            "linux70450-source-heavy-material-filtered-wmo-coverage-v11",
        ),
    ] {
        assert!(Plan::parse(&ALL.replace(from, to)).is_err(), "{from}");
    }
    assert_eq!(v39.version(), 39);
    assert_eq!(v39.contract(), "forever-product-v39");
    assert_eq!(v39.render(), V39);
    assert_eq!(v38.version(), 38);
    assert_eq!(v38.contract(), "forever-product-v38");
    assert_eq!(v38.render(), V38);
    assert!(product.is_v40());
    assert!(!v39.is_v40());
    assert!(!v38.is_v40());
}

#[cfg(target_os = "linux")]
#[test]
fn v40_model_heavy_capacity_is_profile_local_and_preserves_read_limit() {
    use forever_casc::collision::resources::Profile;

    assert_eq!(
        MODEL_HEAVY_MATERIAL_FILTERED_WMO_COLLISION_PROFILE,
        "linux70450-model-heavy-material-filtered-wmo-vmap-product-v1"
    );
    assert_eq!(
        MODEL_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE,
        "linux70450-model-heavy-material-filtered-wmo-coverage-v13"
    );
    assert_eq!(
        MODEL_HEAVY_MATERIAL_FILTERED_WMO_VERIFY_PROFILE,
        "linux70450-model-heavy-material-filtered-wmo-verification-v1"
    );
    assert_eq!(
        MODEL_HEAVY_MATERIAL_FILTERED_WMO_VERIFY_SCOPE,
        "artifact_integrity_model_heavy_material_filtered_wmo_and_independently_derived_map_vmap_domain_coverage"
    );

    let model = Profile::ModelHeavyV1;
    let read = Profile::ReadHeavyV1;
    assert_eq!(model.id(), "linux-collision-model-heavy-resources-v1");
    assert_eq!(model.collector_limits().max_models, 65_536);
    assert_eq!(model.collector_limits().max_reads, 1_000_000);
    assert_eq!(model.startup_limits().collision.max_models, 65_536);
    assert_eq!(model.startup_limits().collision.max_reads, 1_000_000);
    assert_eq!(model.spool_limits().max_records, 65_536);
    assert_eq!(model.inventory_limits(), read.inventory_limits());
    assert_eq!(
        model.stage_limits().max_callbacks,
        read.stage_limits().max_callbacks
    );
    assert_eq!(
        model.stage_limits().world.world.run.placements.max_models,
        65_536
    );
    assert_eq!(model.model_limits().max_models, 65_536);
    assert_eq!(model.package_limits().max_models, 65_536);

    let mut expected_model = read.collector_limits();
    expected_model.max_models = 65_536;
    assert_eq!(model.collector_limits(), expected_model);
    let mut expected_startup = read.startup_limits();
    expected_startup.collision.max_models = 65_536;
    assert_eq!(model.startup_limits(), expected_startup);
    let mut expected_spool = read.spool_limits();
    expected_spool.max_records = 65_536;
    assert_eq!(model.spool_limits(), expected_spool);
    assert_eq!(model.list_profile(), read.list_profile());
    assert_eq!(model.inventory_limits(), read.inventory_limits());
    let mut expected_stage = read.stage_limits();
    expected_stage.world.world.run.placements.max_models = 65_536;
    assert_eq!(model.stage_limits(), expected_stage);
    let mut expected_model_limits = read.model_limits();
    expected_model_limits.max_models = 65_536;
    assert_eq!(model.model_limits(), expected_model_limits);
    let mut expected_package_limits = read.package_limits();
    expected_package_limits.max_models = 65_536;
    assert_eq!(model.package_limits(), expected_package_limits);

    assert_eq!(read.collector_limits().max_models, 4_096);
    assert_eq!(read.collector_limits().max_reads, 1_000_000);
    assert_eq!(read.startup_limits().collision.max_models, 4_096);
    assert_eq!(read.startup_limits().collision.max_reads, 1_000_000);
}

#[cfg(target_os = "linux")]
#[test]
fn v39_and_v38_retain_their_model_caps_and_coverage_versions() {
    use forever_casc::collision::resources::Profile;

    let read = Profile::ReadHeavyV1;
    let source = Profile::SourceHeavyV1;
    assert_eq!(read.collector_limits().max_models, 4_096);
    assert_eq!(read.collector_limits().max_reads, 1_000_000);
    assert_eq!(source.collector_limits().max_models, 4_096);
    assert_eq!(source.collector_limits().max_reads, 4_096);
    assert_eq!(
        READ_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE,
        "linux70450-read-heavy-material-filtered-wmo-coverage-v12"
    );
    assert_eq!(
        SOURCE_HEAVY_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE,
        "linux70450-source-heavy-material-filtered-wmo-coverage-v11"
    );
}

#[test]
fn v40_does_not_inherit_read_heavy_limits_from_its_predecessor() {
    let source = ALL.replace("forever-product\t40", "forever-product\t39");
    assert!(Plan::parse(&source).is_err());
    let parsed_v39 = Plan::parse(V39).unwrap();
    assert_eq!(parsed_v39.version(), 39);
    assert_eq!(parsed_v39.render(), V39);
}

#[cfg(test)]
mod contract_tests {
    #[test]
    fn structural_contract_names_the_canonical_v40_fixture_without_embedding_it() {
        let contract = include_str!("../../../../contracts/forever-product-contract-v40.json");
        assert!(contract.contains("tests/contracts/fixtures/forever-product-v40/all.tsv"));
        assert!(contract.contains("\"fixture_status\": \"canonical synthetic plan fixture supplied; contains no client data or real extraction evidence\""));
        let real_acceptance = contract
            .split_once("\"real_acceptance\": {")
            .unwrap()
            .1
            .split_once("\n  },")
            .unwrap()
            .0;
        assert!(real_acceptance.contains("\"run_id\": \"v40-model-heavy-wmo-20261027T000044Z\""));
        assert!(real_acceptance.contains("\"performed\": true"));
        assert!(real_acceptance.contains("\"status\": \"failed\""));
        assert!(real_acceptance.contains("\"accepted\": false"));
        assert!(real_acceptance.contains("\"accepted_prefix\": false"));
    }
}
