// SPDX-License-Identifier: GPL-3.0-or-later
//! Synthetic build-70450 GameObject encrypted-unavailable plan admission.
use super::*;

const ALL: &str = include_str!("../../../../tests/contracts/fixtures/forever-product-v49/all.tsv");
const V48: &str = include_str!("../../../../tests/contracts/fixtures/forever-product-v48/all.tsv");
const V47: &str = include_str!("../../../../tests/contracts/fixtures/forever-product-v47/all.tsv");

#[test]
fn v49_admits_exact_root_encrypted_unavailable_profile_and_roundtrips() {
    let product = Plan::parse(ALL).unwrap();
    assert_eq!(product.version(), 49);
    assert_eq!(product.contract(), "forever-product-v49");
    assert!(product.is_v49());
    assert!(!product.is_v48());
    assert!(!product.is_v47());
    assert_eq!(product.build, 70450);
    assert_eq!(product.schema_catalog, SchemaCatalog::WowDbDefs70450V1);
    assert_eq!(
        product.components,
        Component::Db2.bit() | Component::Collision.bit()
    );
    assert_eq!(product.render(), ALL);
    assert_eq!(Plan::parse(&product.render()), Ok(product));
}

#[test]
fn v49_uses_distinct_coverage_and_gameobject_scopes() {
    let lines: Vec<_> = ALL.lines().collect();
    assert_eq!(lines.len(), 31);
    assert_eq!(lines[0], "forever-product\t49");
    assert_eq!(
        lines[16],
        "collision-profile\tlinux70450-gameobject-root-encrypted-unavailable-material-filtered-wmo-vmap-product-v1"
    );
    assert_eq!(
        lines[18],
        "collision-coverage\tlinux70450-gameobject-root-encrypted-unavailable-material-filtered-wmo-coverage-v22"
    );
    assert_eq!(
        lines[19],
        "gameobject-scope\tavailable-fdid-direct-root-absent-or-encrypted-unavailable-v1"
    );
    assert_eq!(
        lines[20],
        "model-scope\tplacement-and-gameobject-with-root-absent-or-encrypted-unavailable-v1"
    );
    assert!(ALL.ends_with('\n'));
    assert!(Plan::parse(&ALL.replace("forever-product\t49", "forever-product\t48")).is_err());
    assert!(Plan::parse(&V48.replace("forever-product\t48", "forever-product\t49")).is_err());
    assert!(Plan::parse(&V47.replace("forever-product\t47", "forever-product\t49")).is_err());
    assert!(Plan::parse(&ALL.replace("coverage-v22", "coverage-v21")).is_err());
    assert!(Plan::parse(&ALL.replace("encrypted-unavailable-v1", "missing-root-v1")).is_err());
}

#[cfg(target_os = "linux")]
#[test]
fn v49_keeps_the_existing_expanded_request_and_output_caps() {
    let product = Plan::parse(ALL).unwrap();
    assert_eq!(
        forever_casc::collision::resources::Profile::GameObjectRootAndEncryptedUnavailableExpandedV1
            .id(),
        "linux-collision-gameobject-root-encrypted-unavailable-expanded-resources-v1"
    );
    let CollisionStorage::LeasedDiskV1(budget) = product.collision_storage else {
        panic!("v49 requires leased disk storage");
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
            collision_storage: CollisionStorage::LeasedDiskV1(DiskBudget {
                output_bytes: budget.output_bytes + 1,
                ..budget
            }),
            ..product
        }
        .validate(),
        Err("product_plan_disk_budget")
    );
}

#[cfg(test)]
mod fixture_tests {
    #[test]
    fn v49_fixture_is_canonical_and_keeps_the_same_db2_inputs() {
        let fixture =
            include_str!("../../../../tests/contracts/fixtures/forever-product-v49/all.tsv");
        assert_eq!(fixture.lines().count(), 31);
        assert!(fixture.starts_with("forever-product\t49\n"));
        assert!(fixture.contains("collision-raw-bytes\t2147483648"));
        assert!(fixture.ends_with("db2\tMap\t1349477\n"));
    }
}

#[test]
fn v49_constants_are_bound_to_the_contract() {
    assert_eq!(
        GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_MATERIAL_FILTERED_WMO_COLLISION_PROFILE,
        "linux70450-gameobject-root-encrypted-unavailable-material-filtered-wmo-vmap-product-v1"
    );
    assert_eq!(
        GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE,
        "linux70450-gameobject-root-encrypted-unavailable-material-filtered-wmo-coverage-v22"
    );
    assert_eq!(
        GAMEOBJECT_ROOT_ENCRYPTED_UNAVAILABLE_MATERIAL_FILTERED_WMO_GAMEOBJECT_SCOPE,
        "available-fdid-direct-root-absent-or-encrypted-unavailable-v1"
    );
}
