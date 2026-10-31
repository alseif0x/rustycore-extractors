// SPDX-License-Identifier: GPL-3.0-or-later
//! Synthetic build-70450 DB2, terrain, and VMap plan admission.
use super::*;

const ALL: &str = include_str!("../../../../tests/contracts/fixtures/forever-product-v51/all.tsv");
const V50: &str = include_str!("../../../../tests/contracts/fixtures/forever-product-v50/all.tsv");
const V25: &str = include_str!("../../../../tests/contracts/fixtures/forever-product-v25/all.tsv");

#[test]
fn v51_admits_exact_db2_terrain_vmap_profile_and_canonical_roundtrip() {
    let product = Plan::parse(ALL).unwrap();
    assert_eq!(product.version(), 51);
    assert_eq!(product.contract(), "forever-product-v51");
    assert_eq!(
        product.publication_profile,
        PublicationProfile::Build70450LiquidTerrainVmapCombinedV1
    );
    assert!(product.is_v51());
    assert!(!product.is_v50());
    assert_eq!(product.build, 70450);
    assert_eq!(product.schema_catalog, SchemaCatalog::WowDbDefs70450V1);
    assert_eq!(product.components, 25);
    assert_eq!(product.terrain, Selection::All);
    assert!(product.contains(Component::Db2));
    assert!(product.contains(Component::Terrain));
    assert!(product.contains(Component::Collision));
    assert!(!product.contains(Component::Navigation));
    assert_eq!(
        product.db2,
        [
            Entry {
                name: "GameObjectDisplayInfo".into(),
                id: 1_266_277
            },
            Entry {
                name: "Map".into(),
                id: 1_349_477
            },
            Entry {
                name: "LiquidObject".into(),
                id: 1_308_058
            },
            Entry {
                name: "LiquidMaterial".into(),
                id: 1_132_538
            },
            Entry {
                name: "LiquidType".into(),
                id: 1_371_380
            },
        ]
    );
    assert_eq!(product.render(), ALL);
    assert_eq!(Plan::parse(&product.render()), Ok(product));
}

#[test]
fn v51_has_distinct_publication_and_artifact_identity() {
    let lines: Vec<_> = ALL.lines().collect();
    assert_eq!(lines.len(), 34);
    assert_eq!(lines[0], "forever-product\t51");
    assert_eq!(
        lines[8],
        "collision-storage\tlinux-leased-append-disk-world-v1"
    );
    assert_eq!(
        lines[16],
        "collision-profile\tlinux70450-db2-terrain-liquid-vmap-product-v1"
    );
    assert_eq!(
        lines[18],
        "collision-coverage\tlinux70450-db2-terrain-liquid-vmap-coverage-v1"
    );
    assert_eq!(lines[21], "components\tdb2,maps,vmaps");
    assert_eq!(lines[22], "terrain\tall-available-maps");
    assert_eq!(
        &lines[29..],
        [
            "db2\tGameObjectDisplayInfo\t1266277",
            "db2\tMap\t1349477",
            "db2\tLiquidObject\t1308058",
            "db2\tLiquidMaterial\t1132538",
            "db2\tLiquidType\t1371380",
        ]
    );
    assert!(ALL.ends_with('\n'));
    assert!(Plan::parse(&ALL.replace("forever-product\t51", "forever-product\t50")).is_err());
    assert!(Plan::parse(&V50.replace("forever-product\t50", "forever-product\t51")).is_err());
    assert!(Plan::parse(&V25.replace("forever-product\t25", "forever-product\t51")).is_err());
    assert!(Plan::parse(&ALL.replace("1132538", "1132539")).is_err());
    assert!(Plan::parse(&ALL.replace("1371380", "1371381")).is_err());
    let reversed = ALL.replace(
        "db2\tLiquidObject\t1308058\ndb2\tLiquidMaterial\t1132538",
        "db2\tLiquidMaterial\t1132538\ndb2\tLiquidObject\t1308058",
    );
    assert!(Plan::parse(&reversed).is_err());
    assert!(Plan::parse(&ALL.replace("all-available-maps", "none")).is_err());
    assert!(Plan::parse(&ALL.replace("db2,maps,vmaps", "db2,mmaps,vmaps")).is_err());
    assert!(Plan::parse(&ALL[..ALL.len() - 1]).is_err());
}

#[test]
fn v51_preserves_v50_leased_disk_budget_without_expansion() {
    let product = Plan::parse(ALL).unwrap();
    let v50 = Plan::parse(V50).unwrap();
    let CollisionStorage::LeasedDiskV1(budget) = product.collision_storage else {
        panic!("v51 requires leased disk storage");
    };
    let CollisionStorage::LeasedDiskV1(v50_budget) = v50.collision_storage else {
        panic!("v50 requires leased disk storage");
    };
    assert_eq!(budget, v50_budget);
    assert_eq!(budget.raw_bytes, 2 * 1024 * 1024 * 1024);
    assert_eq!(budget.output_bytes, 2 * 1024 * 1024 * 1024);
    assert_eq!(budget.wdt_bytes, 256 * 1024 * 1024);
    assert_eq!(budget.cache_bytes, 256 * 1024 * 1024);
    assert_eq!(budget.placement_bytes, 256 * 1024 * 1024);
    assert_eq!(budget.map_bytes, 256 * 1024 * 1024);
    assert_eq!(budget.vmo_bytes, 512 * 1024 * 1024);
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

#[test]
fn v51_profile_constants_and_fixture_are_exact() {
    assert_eq!(
        DB2_TERRAIN_LIQUID_VMAP_PRODUCT_PROFILE,
        "linux70450-db2-terrain-liquid-vmap-product-v1"
    );
    assert_eq!(
        DB2_TERRAIN_LIQUID_VMAP_COMPLETION_PROFILE,
        "linux70450-db2-terrain-liquid-vmap-completion-v1"
    );
    assert_eq!(
        DB2_TERRAIN_LIQUID_VMAP_COVERAGE_PROFILE,
        "linux70450-db2-terrain-liquid-vmap-coverage-v1"
    );
    assert_eq!(
        DB2_TERRAIN_LIQUID_VMAP_VERIFY_PROFILE,
        "linux70450-db2-terrain-liquid-vmap-verification-v1"
    );
    assert_eq!(
        DB2_TERRAIN_LIQUID_VMAP_CODEC_PROFILE,
        "linux70450-db2-terrain-liquid-vmap-geometry-v1"
    );
    assert_eq!(
        DB2_TERRAIN_LIQUID_VMAP_DOMAIN_PROFILE,
        "db2-terrain-liquid-vmap-available-map-domain-v1"
    );
    assert_eq!(ALL.lines().count(), 34);
    assert!(ALL.starts_with("forever-product\t51\n"));
    assert!(ALL.ends_with("db2\tLiquidType\t1371380\n"));
}

#[test]
fn v51_context_uses_stable_component_bits_without_aliases() {
    let product = Plan::parse(ALL).unwrap();
    assert_eq!(product.validate(), Ok(()));
    assert!(product.has_authenticated_map_terrain());
    let context = forever_casc::package::Context {
        build: product.build,
        locale: product.locale,
        source_sha256: [1; 32],
        plan_sha256: [2; 32],
        producer_sha256: [3; 32],
        components: product.components,
    };
    assert_eq!(context.components, 25);
    assert_eq!(context.validate(), Ok(()));
    assert!(context.contains(Component::Db2));
    assert!(context.contains(Component::Terrain));
    assert!(context.contains(Component::Collision));
    assert!(!context.contains(Component::Navigation));
    assert!(
        forever_casc::package::Context {
            components: 21,
            ..context
        }
        .validate()
        .is_err()
    );
    assert!(
        Plan {
            components: 21,
            ..product
        }
        .validate()
        .is_err()
    );
}
