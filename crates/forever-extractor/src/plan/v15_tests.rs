// SPDX-License-Identifier: GPL-3.0-or-later
// Authored by RustyCore Extractors, 2026-10-18.
use super::*;
pub(crate) const V15: &str = "forever-product\t15\nbuild\t70450\nlocale\tesES\nindex\tcontiguous\nroot\theader-50893-v2\nschema-catalog\twowdbdefs-70450-v1\ndecryption\tdisabled\ncollision-storage\tlinux-leased-append-disk-world-v1\ncollision-raw-bytes\t1048576\ncollision-wdt-bytes\t1048576\ncollision-cache-bytes\t1048576\ncollision-placement-bytes\t1048576\ncollision-vmo-bytes\t1048576\ncollision-map-bytes\t1048576\ncollision-output-bytes\t1048576\ncollision-profile\tlinux70450-compatible-gnu15-vmap-product-v1\ncollision-domain\tall-available-maps\ncollision-coverage\tcompatible-collision-coverage-v1\ncomponents\tvmaps\nterrain\tnone\npack\ttrue\nnavigation\tnone\nbig-base-unit\tfalse\nskip-liquids\tfalse\nmaximum-slope\t55\nnon-steep-slope\t55\n";

#[test]
fn v15_canonical_collision_domain_and_independent_profile() {
    assert_ne!(
        COMPATIBLE_COLLISION_VERIFY_PROFILE_70450,
        COMPATIBLE_COLLISION_VERIFY_PROFILE
    );
    for domain in ["all-available-maps", "map:0", "map:4294967295"] {
        let text = V15.replace("all-available-maps", domain);
        let plan = Plan::parse(&text).unwrap();
        assert!(plan.is_v15());
        assert!(!plan.is_v14());
        assert_eq!(plan.version(), 15);
        assert_eq!(plan.contract(), "forever-product-v15");
        assert_eq!(plan.schema_catalog.id(), "wowdbdefs-70450-v1");
        assert_eq!(plan.render(), text);
    }
}

#[test]
fn v15_exact_mask_build_catalog_and_disabled_keys() {
    let p = Plan::parse(V15).unwrap();
    for mask in 0..=255 {
        if mask != Component::Collision.bit() {
            assert_eq!(
                Plan {
                    components: mask,
                    ..p.clone()
                }
                .validate(),
                Err("product_plan_component_scope")
            );
        }
    }
    for build in [0, 70170, 70205, 70338, 70374, 70425] {
        assert_eq!(
            Plan { build, ..p.clone() }.validate(),
            Err("product_plan_build")
        );
    }
    for schema_catalog in [
        SchemaCatalog::PinnedMetadataV1,
        SchemaCatalog::WowDbDefs70205V1,
        SchemaCatalog::WowDbDefs70338V1,
        SchemaCatalog::WowDbDefs70374V1,
        SchemaCatalog::WowDbDefs70425V1,
    ] {
        assert_eq!(
            Plan {
                schema_catalog,
                ..p.clone()
            }
            .validate(),
            Err("product_plan_schema_catalog")
        );
    }
    assert_eq!(
        Plan {
            decryption: Decryption::ProvidedSalsa20V1,
            ..p
        }
        .validate(),
        Err("product_plan_component_scope")
    );
}

#[test]
fn v15_rejects_unsupported_storage_and_unused_policies() {
    let p = Plan::parse(V15).unwrap();
    for q in [
        Plan {
            collision_storage: CollisionStorage::Resident,
            ..p.clone()
        },
        Plan {
            terrain: Selection::All,
            ..p.clone()
        },
        Plan {
            navigation: Selection::All,
            ..p.clone()
        },
        Plan {
            offmesh: Some(vec![]),
            ..p.clone()
        },
        Plan {
            pack: false,
            ..p.clone()
        },
        Plan {
            skip_liquids: true,
            ..p.clone()
        },
        Plan {
            big_base_unit: true,
            ..p.clone()
        },
        Plan {
            maximum_slope: 54,
            ..p.clone()
        },
        Plan {
            non_steep_slope: 54,
            ..p
        },
    ] {
        assert!(q.validate().is_err());
    }
}

#[test]
fn v15_closed_canonical_domain_and_metadata_grammar() {
    for value in [
        "all",
        "none",
        "tile:0:1:1",
        "map:00",
        "map:-1",
        "map:4294967296",
        "map:",
    ] {
        assert!(Plan::parse(&V15.replace("all-available-maps", value)).is_err());
    }
    for (a, b) in [
        (
            "compatible-collision-coverage-v1",
            "compatible-collision-coverage-v2",
        ),
        (COMPATIBLE_COLLISION_VERIFY_PROFILE_70450, "legacy"),
        ("1048576", "0"),
    ] {
        assert!(Plan::parse(&V15.replace(a, b)).is_err());
    }
}

#[test]
fn v15_rejects_cross_build_catalogue_and_wrong_profiles() {
    let p = Plan::parse(V15).unwrap();
    assert_eq!(
        Plan {
            schema_catalog: SchemaCatalog::WowDbDefs70425V1,
            ..p.clone()
        }
        .validate(),
        Err("product_plan_schema_catalog")
    );
    assert_eq!(
        Plan {
            build: 70425,
            ..p.clone()
        }
        .validate(),
        Err("product_plan_build")
    );
    assert_eq!(
        Plan {
            build: 70170,
            ..p.clone()
        }
        .validate(),
        Err("product_plan_build")
    );
    for profile in [
        PublicationProfile::Legacy,
        PublicationProfile::Build70425CompatibleCollisionV1,
        PublicationProfile::Build70374Db2OnlyV1,
        PublicationProfile::Build70374CompatibleTerrainV1,
    ] {
        assert_eq!(
            Plan {
                publication_profile: profile,
                ..p.clone()
            }
            .validate(),
            Err("product_plan_build")
        );
    }
    assert_eq!(
        Plan::parse(&V15.replace(
            "linux70450-compatible-gnu15-vmap-product-v1",
            "linux70425-compatible-gnu15-vmap-product-v1"
        )),
        Err("product_plan_collision_profile")
    );
    assert!(Plan::parse(&V15.replace("forever-product\t15", "forever-product\t14")).is_err());
}
