// SPDX-License-Identifier: GPL-3.0-or-later
//! Synthetic build-70450 declared-partial-Map plan admission.
use super::*;

const MAP: &str = include_str!("../../../../tests/contracts/fixtures/forever-product-v25/map2.tsv");
const ALL: &str = include_str!("../../../../tests/contracts/fixtures/forever-product-v25/all.tsv");

#[test]
fn v25_admits_only_db2_map_and_v20_terrain_with_declared_skip() {
    for (source, terrain) in [(MAP, Selection::Map(2)), (ALL, Selection::All)] {
        let plan = Plan::parse(source).unwrap();
        assert_eq!(plan.version(), 25);
        assert_eq!(plan.contract(), "forever-product-v25");
        assert_eq!(
            plan.publication_profile,
            PublicationProfile::Build70450AuthenticatedTerrainPartialMapV1
        );
        assert!(plan.is_v25());
        assert!(plan.is_available_terrain());
        assert_eq!(plan.build, 70450);
        assert_eq!(plan.schema_catalog, SchemaCatalog::WowDbDefs70450V1);
        assert_eq!(plan.components, 9);
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
        assert!(
            plan.render()
                .contains("partial_encrypted_sections\tdeclared-skip\n")
        );
        assert_eq!(Plan::parse(&plan.render()), Ok(plan));
    }
}

#[test]
fn v25_rejects_undeclared_or_misdeclared_partial_sections_and_other_revisions() {
    assert_eq!(
        Plan::parse(&MAP.replace(
            "partial_encrypted_sections\tdeclared-skip\n",
            "partial_encrypted_sections\tallow\n"
        ))
        .unwrap_err(),
        "product_plan_partial_encrypted_sections"
    );
    assert!(Plan::parse(&MAP.replace("partial_encrypted_sections\tdeclared-skip\n", "")).is_err());
    let v24 = MAP
        .replace("forever-product\t25", "forever-product\t24")
        .replace("partial_encrypted_sections\tdeclared-skip\n", "");
    assert!(Plan::parse(&v24).is_ok());
    assert!(
        Plan::parse(&v24.replace(
            "decryption\tdisabled\n",
            "decryption\tdisabled\npartial_encrypted_sections\tdeclared-skip\n"
        ))
        .is_err()
    );
}

#[test]
fn v25_rejects_every_other_mask_and_forbidden_product_scope() {
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
}
