// SPDX-License-Identifier: GPL-3.0-or-later
//! Synthetic build-70450 GameObject missing-ROOT plan admission.
use super::*;

const ALL: &str = include_str!("../../../../tests/contracts/fixtures/forever-product-v48/all.tsv");
const V47: &str = include_str!("../../../../tests/contracts/fixtures/forever-product-v47/all.tsv");
const V46: &str = include_str!("../../../../tests/contracts/fixtures/forever-product-v46/all.tsv");

#[test]
fn v48_admits_exact_missing_root_profile_and_canonical_roundtrip() {
    let product = Plan::parse(ALL).unwrap();
    assert_eq!(product.version(), 48);
    assert_eq!(product.contract(), "forever-product-v48");
    assert_eq!(
        product.publication_profile,
        PublicationProfile::Build70450GameObjectMissingRootMaterialFilteredWmoGeometryV1
    );
    assert!(product.is_v48());
    assert!(!product.is_v47());
    assert!(!product.is_v46());
    assert_eq!(product.build, 70450);
    assert_eq!(product.schema_catalog, SchemaCatalog::WowDbDefs70450V1);
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
        ]
    );
    assert_eq!(product.render(), ALL);
    assert_eq!(Plan::parse(&product.render()), Ok(product));
}

#[test]
fn v48_uses_exact_grammar_and_rejects_v47_identity_aliases() {
    let lines: Vec<_> = ALL.lines().collect();
    assert_eq!(lines.len(), 31);
    assert_eq!(lines[0], "forever-product\t48");
    assert_eq!(
        lines[16],
        "collision-profile\tlinux70450-gameobject-missing-root-material-filtered-wmo-vmap-product-v1"
    );
    assert_eq!(
        lines[18],
        "collision-coverage\tlinux70450-gameobject-missing-root-material-filtered-wmo-coverage-v21"
    );
    assert_eq!(
        lines[19],
        "gameobject-scope\tavailable-fdid-direct-missing-root-v1"
    );
    assert_eq!(
        lines[20],
        "model-scope\tplacement-and-gameobject-with-missing-root-v1"
    );
    assert!(ALL.ends_with('\n'));
    assert!(Plan::parse(&ALL.replace("forever-product\t48", "forever-product\t47")).is_err());
    assert!(Plan::parse(&V47.replace("forever-product\t47", "forever-product\t48")).is_err());
    assert!(Plan::parse(&V46.replace("forever-product\t46", "forever-product\t48")).is_err());
    assert!(
        Plan::parse(&ALL.replace(
            "available-fdid-direct-missing-root-v1",
            "available-fdid-direct-v1"
        ))
        .is_err()
    );
    assert!(Plan::parse(&ALL.replace("coverage-v21", "coverage-v20")).is_err());
    assert!(Plan::parse(&ALL.replace("forever-product\t48", "forever-product\t048")).is_err());
}

#[test]
fn v48_binds_its_resource_ceilings_without_changing_v47() {
    let base = Plan::parse(ALL).unwrap();
    let CollisionStorage::LeasedDiskV1(budget) = base.collision_storage else {
        panic!("v48 requires leased disk storage");
    };
    assert_eq!(budget.raw_bytes, 2 * 1024 * 1024 * 1024);
    assert_eq!(budget.output_bytes, 2 * 1024 * 1024 * 1024);
    assert_eq!(budget.wdt_bytes, 256 * 1024 * 1024);
    assert_eq!(budget.cache_bytes, 256 * 1024 * 1024);
    assert_eq!(budget.placement_bytes, 256 * 1024 * 1024);
    assert_eq!(budget.vmo_bytes, 512 * 1024 * 1024);
    assert_eq!(budget.map_bytes, 256 * 1024 * 1024);
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
            collision_storage: CollisionStorage::LeasedDiskV1(DiskBudget {
                raw_bytes: budget.raw_bytes + 1,
                ..budget
            }),
            ..base.clone()
        }
        .validate(),
        Err("product_plan_disk_budget")
    );
    let v47 = Plan::parse(V47).unwrap();
    assert_eq!(v47.version(), 47);
    assert!(v47.is_v47());
    assert!(!v47.is_v48());
    assert_eq!(v47.render(), V47);
}

#[cfg(test)]
mod fixture_tests {
    #[test]
    fn v48_fixture_is_a_separate_canonical_file_without_a_godi_row_count() {
        let fixture =
            include_str!("../../../../tests/contracts/fixtures/forever-product-v48/all.tsv");
        assert_eq!(fixture.lines().count(), 31);
        assert!(fixture.starts_with("forever-product\t48\n"));
        assert!(fixture.contains("collision-raw-bytes\t2147483648"));
        assert!(!fixture.contains("gameobject-rows"));
        assert!(fixture.ends_with("db2\tMap\t1349477\n"));
    }
}
