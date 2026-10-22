// SPDX-License-Identifier: GPL-3.0-or-later
//! Synthetic build-70450 sealed-Map authenticated terrain plan admission.
use super::*;

const MAP: &str = include_str!("../../../../tests/contracts/fixtures/forever-product-v24/map2.tsv");
const ALL: &str = include_str!("../../../../tests/contracts/fixtures/forever-product-v24/all.tsv");

#[test]
fn v24_admits_only_db2_map_and_v20_terrain_in_component_mask_nine() {
    for (source, terrain) in [(MAP, Selection::Map(2)), (ALL, Selection::All)] {
        let plan = Plan::parse(source).unwrap();
        assert_eq!(plan.version(), 24);
        assert_eq!(plan.contract(), "forever-product-v24");
        assert_eq!(
            plan.publication_profile,
            PublicationProfile::Build70450AuthenticatedTerrainV1
        );
        assert!(plan.is_v24());
        assert_eq!(plan.build, 70450);
        assert_eq!(plan.schema_catalog, SchemaCatalog::WowDbDefs70450V1);
        assert_eq!(plan.components, 9, "Db2=1 | Terrain=8");
        assert!(plan.contains(Component::Db2));
        assert!(plan.contains(Component::Terrain));
        assert_eq!(
            plan.db2,
            [Entry {
                name: "Map".into(),
                id: 1_349_477
            }]
        );
        assert_eq!(plan.decryption, Decryption::Disabled);
        assert_eq!(plan.collision_storage, CollisionStorage::Resident);
        assert_eq!(plan.map_domain, None);
        assert_eq!(plan.terrain, terrain);
        assert_eq!(plan.navigation, Selection::None);
        assert_eq!(plan.offmesh, None);
        assert_eq!(Plan::parse(&plan.render()), Ok(plan));
    }
}

#[test]
fn v24_refuses_every_other_component_mask_and_forbidden_scope() {
    let base = Plan::parse(MAP).unwrap();
    for mask in 0..=u8::MAX {
        if mask != 9 {
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
    for plan in [
        Plan {
            build: 70425,
            ..base.clone()
        },
        Plan {
            schema_catalog: SchemaCatalog::WowDbDefs70425V1,
            ..base.clone()
        },
        Plan {
            decryption: Decryption::ProvidedSalsa20V1,
            ..base.clone()
        },
        Plan {
            collision_storage: CollisionStorage::LeasedDiskV1(DiskBudget {
                raw_bytes: 1,
                wdt_bytes: 1,
                cache_bytes: 1,
                placement_bytes: 1,
                vmo_bytes: 1,
                map_bytes: 1,
                output_bytes: 1,
            }),
            ..base.clone()
        },
        Plan {
            db2: Vec::new(),
            ..base.clone()
        },
        Plan {
            db2: vec![Entry {
                name: "LiquidType".into(),
                id: catalog::resolve_for(SchemaCatalog::WowDbDefs70450V1.id(), "LiquidType")
                    .unwrap()
                    .unwrap()
                    .metadata()
                    .file_data_id,
            }],
            ..base.clone()
        },
        Plan {
            terrain: Selection::None,
            ..base.clone()
        },
        Plan {
            terrain: Selection::Tile { map: 2, x: 3, y: 4 },
            ..base.clone()
        },
        Plan {
            navigation: Selection::Map(2),
            ..base.clone()
        },
        Plan {
            map_domain: Some(2),
            ..base.clone()
        },
        Plan {
            offmesh: Some(Vec::new()),
            ..base.clone()
        },
    ] {
        assert!(plan.validate().is_err());
    }
    assert!(Plan::parse(&MAP.replace("forever-product\t24", "forever-product\t23")).is_err());
}
