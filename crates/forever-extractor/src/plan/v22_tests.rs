// SPDX-License-Identifier: GPL-3.0-or-later
//! Synthetic build-70450 selected-map collision-geometry plan admission.
use super::*;

const V22: &str = include_str!("../../../../tests/contracts/fixtures/forever-product-v22/map2.tsv");

fn plan() -> Plan {
    Plan::parse(V22).unwrap()
}

#[test]
fn v22_has_its_own_build_bound_collision_identity_and_coverage_contract() {
    let plan = plan();
    assert_eq!(plan.version(), 22);
    assert_eq!(plan.contract(), "forever-product-v22");
    assert_eq!(
        plan.publication_profile,
        PublicationProfile::Build70450SelectedGeometryV1
    );
    assert!(plan.is_v22());
    assert!(!plan.is_v15());
    assert!(!plan.is_v18());
    assert!(!plan.is_v21());
    assert_eq!(plan.build, 70450);
    assert_eq!(plan.schema_catalog, SchemaCatalog::WowDbDefs70450V1);
    assert_eq!(plan.components, Component::Collision.bit());
    assert!(plan.db2.is_empty());
    assert_eq!(plan.decryption, Decryption::Disabled);
    assert!(!plan.requires_keys());
    assert!(matches!(
        plan.collision_storage,
        CollisionStorage::LeasedDiskV1(_)
    ));
    assert_eq!(plan.map_domain, Some(2));
    assert_eq!(plan.terrain, Selection::None);
    assert_eq!(plan.navigation, Selection::None);
    assert_eq!(plan.offmesh, None);
    assert_eq!(plan.contract(), "forever-product-v22");
    assert_eq!(Plan::parse(&plan.render()), Ok(plan));
}

#[test]
fn v22_refuses_masks_keys_and_non_map_domains_before_io() {
    let base = plan();
    for mask in 0..=u8::MAX {
        if mask != Component::Collision.bit() {
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
    for domain in ["all-available-maps", "all", "map:02", "map:4294967296"] {
        let text = V22.replace(
            "collision-domain\tmap:2",
            &format!("collision-domain\t{domain}"),
        );
        assert!(Plan::parse(&text).is_err(), "domain {domain}");
    }
    let missing_domain = Plan {
        map_domain: None,
        ..base.clone()
    };
    assert_eq!(missing_domain.validate(), Err("product_plan_map_domain"));
    assert_eq!(missing_domain.try_render(), Err("product_plan_map_domain"));
    assert!(matches!(
        std::panic::catch_unwind(|| missing_domain.render()),
        Ok(rendered) if rendered.is_empty()
    ));
    assert_eq!(
        Plan {
            build: 70425,
            ..base.clone()
        }
        .validate(),
        Err("product_plan_build")
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

#[test]
fn v22_has_distinct_collision_and_policy_scope_literals() {
    for (from, to, error) in [
        (
            SELECTED_GEOMETRY_COLLISION_PROFILE,
            COMPATIBLE_COLLISION_VERIFY_PROFILE_70450,
            "product_plan_collision_profile",
        ),
        (
            SELECTED_GEOMETRY_COVERAGE_PROFILE,
            "compatible-collision-coverage-v1",
            "product_plan_collision_coverage",
        ),
        (
            SELECTED_GEOMETRY_GAMEOBJECT_SCOPE,
            "global-startup-v1",
            "product_plan_gameobject_scope",
        ),
        (
            SELECTED_GEOMETRY_MODEL_SCOPE,
            "global",
            "product_plan_model_scope",
        ),
    ] {
        assert_eq!(Plan::parse(&V22.replace(from, to)), Err(error), "{from}");
    }
}
