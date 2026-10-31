// SPDX-License-Identifier: GPL-3.0-or-later
//! Synthetic build-70450 PlacementExpanded material-filtered-WMO plan admission.
use super::*;

const ALL: &str = include_str!("../../../../tests/contracts/fixtures/forever-product-v45/all.tsv");
const V44: &str = include_str!("../../../../tests/contracts/fixtures/forever-product-v44/all.tsv");
const V43: &str = include_str!("../../../../tests/contracts/fixtures/forever-product-v43/all.tsv");

fn plan() -> Plan {
    Plan::parse(ALL).unwrap()
}

#[test]
fn v45_admits_exact_placement_expanded_profile_and_canonical_roundtrip() {
    let product = plan();
    assert_eq!(product.version(), 45);
    assert_eq!(product.contract(), "forever-product-v45");
    assert_eq!(
        product.publication_profile,
        PublicationProfile::Build70450PlacementExpandedMaterialFilteredWmoGeometryV1
    );
    assert!(product.is_v45());
    assert!(!product.is_v44());
    assert!(!product.is_v43());
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
        PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_COLLISION_PROFILE,
        "linux70450-placement-expanded-material-filtered-wmo-vmap-product-v1"
    );
    assert_eq!(
        PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE,
        "linux70450-placement-expanded-material-filtered-wmo-coverage-v18"
    );
    assert_eq!(
        PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_VERIFY_PROFILE,
        "linux70450-placement-expanded-material-filtered-wmo-verification-v1"
    );
    assert_eq!(
        PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_VERIFY_SCOPE,
        "artifact_integrity_placement_expanded_material_filtered_wmo_and_independently_derived_map_vmap_domain_coverage"
    );
}

#[test]
fn v45_requires_exact_30_line_grammar_and_v18_identity() {
    let lines: Vec<_> = ALL.lines().collect();
    assert_eq!(lines.len(), 30);
    assert_eq!(lines[0], "forever-product\t45");
    assert_eq!(lines[7], "partial_encrypted_sections\tdeclared-skip");
    assert_eq!(
        lines[16],
        "collision-profile\tlinux70450-placement-expanded-material-filtered-wmo-vmap-product-v1"
    );
    assert_eq!(
        lines[18],
        "collision-coverage\tlinux70450-placement-expanded-material-filtered-wmo-coverage-v18"
    );
    assert!(ALL.ends_with('\n'));
    assert_eq!(
        Plan::parse(&ALL.replace("partial_encrypted_sections\tdeclared-skip\n", "")),
        Err("product_plan_partial_encrypted_sections")
    );
    for (from, to) in [
        (
            "linux70450-placement-expanded-material-filtered-wmo-vmap-product-v1",
            "linux70450-source-expanded-v2-material-filtered-wmo-vmap-product-v1",
        ),
        (
            "linux70450-placement-expanded-material-filtered-wmo-coverage-v18",
            "linux70450-source-expanded-v2-material-filtered-wmo-coverage-v17",
        ),
        (
            "linux70450-placement-expanded-material-filtered-wmo-vmap-product-v1",
            "linux70450-model-bytes-expanded-material-filtered-wmo-vmap-product-v1",
        ),
    ] {
        assert!(Plan::parse(&ALL.replace(from, to)).is_err(), "{to}");
    }
    assert!(Plan::parse(&ALL.replace("forever-product\t45", "forever-product\t44")).is_err());
    assert!(Plan::parse(&V44.replace("forever-product\t44", "forever-product\t45")).is_err());
    assert!(Plan::parse(&V43.replace("forever-product\t43", "forever-product\t45")).is_err());
}

#[cfg(target_os = "linux")]
#[test]
fn v45_uses_placement_expanded_resources_without_changing_other_capacities() {
    use forever_casc::collision::resources::Profile;

    let profile = Profile::PlacementExpandedV1;
    let source_expanded = Profile::SourceExpandedV2;
    assert_eq!(
        profile.id(),
        "linux-collision-placement-expanded-resources-v1"
    );
    assert_eq!(profile.collector_limits().max_source_bytes, 2_147_483_648);
    assert_eq!(profile.collector_limits().max_raw_bytes, 268_435_456);
    assert_eq!(profile.collector_limits().max_models, 65_536);
    assert_eq!(profile.collector_limits().max_reads, 1_000_000);
    assert_eq!(profile.collector_limits().max_requests, 16_000_000);
    assert_eq!(
        profile.collector_limits().max_dependency_requests,
        16_000_000
    );
    assert_eq!(
        profile.stage_limits().world.world.run.placements.max_models,
        65_536
    );
    assert_eq!(profile.model_limits().max_models, 65_536);
    assert_eq!(profile.package_limits().max_models, 65_536);
    let mut expected = source_expanded.collector_limits();
    expected.max_source_bytes = profile.collector_limits().max_source_bytes;
    assert_eq!(profile.collector_limits(), expected);
}

#[cfg(test)]
mod fixture_tests {
    #[test]
    fn v45_fixture_is_a_separate_canonical_file() {
        let fixture =
            include_str!("../../../../tests/contracts/fixtures/forever-product-v45/all.tsv");
        assert_eq!(fixture.lines().count(), 30);
        assert!(fixture.starts_with("forever-product\t45\n"));
        assert!(
            fixture.contains("linux70450-placement-expanded-material-filtered-wmo-coverage-v18")
        );
    }
}
