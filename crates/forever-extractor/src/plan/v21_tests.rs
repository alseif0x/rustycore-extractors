// SPDX-License-Identifier: GPL-3.0-or-later
//! Synthetic build-70450 map-domain navigation-plan admission.
use super::*;

const V21: &str =
    include_str!("../../../../tests/contracts/fixtures/forever-product-v21/navigation-map2.tsv");
const V21_OFFMESH: &str = include_str!(
    "../../../../tests/contracts/fixtures/forever-product-v21/navigation-map2-offmesh.tsv"
);

fn plan() -> Plan {
    Plan::parse(V21).unwrap()
}

#[test]
fn v21_identity_and_scope_are_canonical_and_build_bound() {
    let plan = plan();
    assert_eq!(plan.version(), 21);
    assert_eq!(plan.contract(), "forever-product-v21");
    assert_eq!(
        plan.publication_profile,
        PublicationProfile::Build70450NavigationV1
    );
    assert_eq!(plan.build, 70450);
    assert_eq!(plan.schema_catalog, SchemaCatalog::WowDbDefs70450V1);
    assert_eq!(
        plan.components,
        Component::Db2.bit()
            | Component::Terrain.bit()
            | Component::Collision.bit()
            | Component::Navigation.bit()
    );
    assert!(matches!(
        plan.collision_storage,
        CollisionStorage::LeasedDiskV1(_)
    ));
    assert_eq!(plan.decryption, Decryption::Disabled);
    assert_eq!(plan.map_domain, Some(2));
    assert_eq!(plan.terrain, Selection::Map(2));
    assert_eq!(plan.navigation, Selection::Map(2));
    assert_eq!(
        plan.db2
            .iter()
            .map(|entry| entry.name.as_str())
            .collect::<Vec<_>>(),
        ["LiquidType", "Map"]
    );
    assert_eq!(Plan::parse(&plan.render()), Ok(plan));
}

#[test]
fn v21_offmesh_records_are_canonical_and_bound_to_the_selected_map() {
    let plan = Plan::parse(V21_OFFMESH).unwrap();
    let records = plan.offmesh.as_ref().unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].map_id, 2);
    assert_eq!(Plan::parse(&plan.render()), Ok(plan));
    let missing_profile = V21_OFFMESH.replace("offmesh\tinline-f32-bits-v1\n", "");
    assert!(Plan::parse(&missing_profile).is_err());
}

#[test]
fn v21_fails_closed_on_foreign_build_catalog_and_incomplete_navigation_inputs() {
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
    for missing in ["Map", "LiquidType"] {
        let mut variant = base.clone();
        variant.db2.retain(|entry| entry.name != missing);
        assert!(variant.validate().is_err(), "missing {missing}");
    }
    assert_eq!(
        Plan {
            terrain: Selection::Map(3),
            ..base.clone()
        }
        .validate(),
        Err("product_plan_map_domain_selection")
    );
    assert_eq!(
        Plan {
            navigation: Selection::Map(3),
            ..base.clone()
        }
        .validate(),
        Err("product_plan_map_domain_selection")
    );
    assert!(
        Plan {
            navigation: Selection::None,
            offmesh: None,
            ..base.clone()
        }
        .validate()
        .is_err()
    );
    let mut other_map_connection = offmesh_tests::connection();
    other_map_connection.map_id = 3;
    assert_eq!(
        Plan {
            offmesh: Some(vec![other_map_connection]),
            ..base.clone()
        }
        .validate(),
        Err("product_plan_map_domain_offmesh")
    );
    assert!(
        Plan {
            collision_storage: CollisionStorage::Resident,
            ..base
        }
        .validate()
        .is_err()
    );
}
