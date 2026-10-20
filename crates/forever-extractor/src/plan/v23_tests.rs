// SPDX-License-Identifier: GPL-3.0-or-later
//! Synthetic build-70450 selected-navigation plan admission.
use super::*;

const V23: &str =
    include_str!("../../../../tests/contracts/fixtures/forever-product-v23/map48.tsv");

#[test]
fn v23_has_an_additive_single_map_navigation_identity() {
    let plan = Plan::parse(V23).unwrap();
    assert_eq!(plan.version(), 23);
    assert_eq!(plan.contract(), "forever-product-v23");
    assert_eq!(
        plan.publication_profile,
        PublicationProfile::Build70450SelectedNavigationV1
    );
    assert!(plan.is_v23());
    assert_eq!(plan.build, 70450);
    assert_eq!(plan.schema_catalog, SchemaCatalog::WowDbDefs70450V1);
    assert_eq!(plan.components, Component::Navigation.bit());
    assert!(plan.db2.is_empty());
    assert_eq!(plan.decryption, Decryption::Disabled);
    assert_eq!(plan.collision_storage, CollisionStorage::Resident);
    assert_eq!(plan.map_domain, None);
    assert_eq!(plan.terrain, Selection::None);
    assert_eq!(plan.navigation, Selection::Map(48));
    assert_eq!(plan.offmesh, None);
    assert_eq!(Plan::parse(&plan.render()), Ok(plan));
}

#[test]
fn v23_refuses_every_other_mask_and_forbidden_plan_scope() {
    let base = Plan::parse(V23).unwrap();
    for mask in 0..=u8::MAX {
        if mask != Component::Navigation.bit() {
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
    assert_eq!(
        Plan {
            db2: vec![Entry {
                name: "Map".into(),
                id: forever_db2::catalog::resolve_for(SchemaCatalog::WowDbDefs70450V1.id(), "Map",)
                    .unwrap()
                    .unwrap()
                    .metadata()
                    .file_data_id,
            }],
            ..base.clone()
        }
        .validate(),
        Err("product_plan_component_scope")
    );
    for plan in [
        Plan {
            build: 70425,
            ..base.clone()
        },
        Plan {
            navigation: Selection::All,
            ..base.clone()
        },
        Plan {
            navigation: Selection::None,
            ..base.clone()
        },
        Plan {
            pack: false,
            ..base.clone()
        },
        Plan {
            offmesh: Some(Vec::new()),
            ..base.clone()
        },
    ] {
        assert!(plan.validate().is_err());
    }
    assert!(Plan::parse(&V23.replace("forever-product\t23", "forever-product\t22")).is_err());
}
