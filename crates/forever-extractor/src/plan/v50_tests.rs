// SPDX-License-Identifier: GPL-3.0-or-later
//! Synthetic build-70450 complete LiquidObject plan admission.
use super::*;

const ALL: &str = include_str!("../../../../tests/contracts/fixtures/forever-product-v50/all.tsv");
const V49: &str = include_str!("../../../../tests/contracts/fixtures/forever-product-v49/all.tsv");

#[test]
fn v50_admits_exact_liquid_object_profile_and_canonical_roundtrip() {
    let product = Plan::parse(ALL).unwrap();
    assert_eq!(product.version(), 50);
    assert_eq!(product.contract(), "forever-product-v50");
    assert_eq!(
        product.publication_profile,
        PublicationProfile::Build70450LiquidObjectCompleteSourceV1
    );
    assert!(product.is_v50());
    assert!(!product.is_v49());
    assert_eq!(product.build, 70450);
    assert_eq!(product.schema_catalog, SchemaCatalog::WowDbDefs70450V1);
    assert_eq!(
        product.components,
        Component::Db2.bit() | Component::Collision.bit()
    );
    assert_eq!(
        product.db2,
        [
            Entry {
                name: "GameObjectDisplayInfo".into(),
                id: 1_266_277,
            },
            Entry {
                name: "Map".into(),
                id: 1_349_477,
            },
            Entry {
                name: "LiquidObject".into(),
                id: 1_308_058,
            },
        ]
    );
    assert_eq!(product.render(), ALL);
    assert_eq!(Plan::parse(&product.render()), Ok(product));
}

#[test]
fn v50_uses_its_separate_liquid_object_evidence_identity() {
    let lines: Vec<_> = ALL.lines().collect();
    assert_eq!(lines.len(), 32);
    assert_eq!(lines[0], "forever-product\t50");
    assert_eq!(
        lines[16],
        "collision-profile\tlinux70450-liquid-object-complete-source-vmap-product-v1"
    );
    assert_eq!(
        lines[18],
        "collision-coverage\tlinux70450-liquid-object-complete-source-coverage-v23"
    );
    assert_eq!(lines[19], "gameobject-scope\tsource-only-v1");
    assert_eq!(lines[20], "model-scope\tsource-only-v1");
    assert_eq!(
        &lines[29..],
        [
            "db2\tGameObjectDisplayInfo\t1266277",
            "db2\tMap\t1349477",
            "db2\tLiquidObject\t1308058",
        ]
    );
    assert!(ALL.ends_with('\n'));
    assert!(Plan::parse(&ALL.replace("forever-product\t50", "forever-product\t49")).is_err());
    assert!(Plan::parse(&V49.replace("forever-product\t49", "forever-product\t50")).is_err());
    assert!(Plan::parse(&ALL.replace("1308058", "1308059")).is_err());
    let reversed = ALL.replace(
        "db2\tMap\t1349477\ndb2\tLiquidObject\t1308058",
        "db2\tLiquidObject\t1308058\ndb2\tMap\t1349477",
    );
    assert!(Plan::parse(&reversed).is_err());
    assert!(Plan::parse(&ALL[..ALL.len() - 1]).is_err());
    assert!(Plan::parse(&ALL.replace("source-only-v1", "excluded-by-profile-v1")).is_err());
}

#[test]
fn v50_binds_complete_source_component_and_disk_budget() {
    let base = Plan::parse(ALL).unwrap();
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
    for components in 0..=u8::MAX {
        if components != Component::Db2.bit() | Component::Collision.bit() {
            assert_eq!(
                Plan {
                    components,
                    ..base.clone()
                }
                .validate(),
                Err("product_plan_component_scope")
            );
        }
    }
    let CollisionStorage::LeasedDiskV1(budget) = base.collision_storage else {
        panic!("v50 requires leased disk storage");
    };
    assert_eq!(budget.raw_bytes, 2 * 1024 * 1024 * 1024);
    assert_eq!(budget.output_bytes, 2 * 1024 * 1024 * 1024);
    assert_eq!(budget.map_bytes, 256 * 1024 * 1024);
    assert_eq!(
        Plan {
            collision_storage: CollisionStorage::LeasedDiskV1(DiskBudget {
                output_bytes: budget.output_bytes + 1,
                ..budget
            }),
            ..base
        }
        .validate(),
        Err("product_plan_disk_budget")
    );
}

#[test]
fn v50_constants_and_fixture_are_bound_to_contract_values() {
    assert_eq!(
        LIQUID_OBJECT_COMPLETE_SOURCE_PROFILE,
        "linux70450-liquid-object-complete-source-vmap-product-v1"
    );
    assert_eq!(
        LIQUID_OBJECT_COMPLETE_SOURCE_COVERAGE_PROFILE,
        "linux70450-liquid-object-complete-source-coverage-v23"
    );
    assert_eq!(
        LIQUID_OBJECT_COMPLETE_SOURCE_CODEC_PROFILE,
        "linux70450-liquid-object-complete-source-v1"
    );
    assert_eq!(
        LIQUID_OBJECT_COMPLETE_SOURCE_VERIFY_PROFILE,
        "linux70450-liquid-object-complete-source-verification-v1"
    );
    assert_eq!(ALL.lines().count(), 32);
    assert!(ALL.starts_with("forever-product\t50\n"));
    assert!(ALL.ends_with("db2\tLiquidObject\t1308058\n"));
}
