// SPDX-License-Identifier: GPL-3.0-or-later
// Authored by RustyCore Extractors, 2026-10-17.
use super::*;
pub(crate) const V14: &str = "forever-product\t14\nbuild\t70425\nlocale\tesES\nindex\tcontiguous\nroot\theader-50893-v2\nschema-catalog\twowdbdefs-70425-v1\ndecryption\tdisabled\ncollision-storage\tlinux-leased-append-disk-world-v1\ncollision-raw-bytes\t1048576\ncollision-wdt-bytes\t1048576\ncollision-cache-bytes\t1048576\ncollision-placement-bytes\t1048576\ncollision-vmo-bytes\t1048576\ncollision-map-bytes\t1048576\ncollision-output-bytes\t1048576\ncollision-profile\tlinux70425-compatible-gnu15-vmap-product-v1\ncollision-domain\tall-available-maps\ncollision-coverage\tcompatible-collision-coverage-v1\ncomponents\tvmaps\nterrain\tnone\npack\ttrue\nnavigation\tnone\nbig-base-unit\tfalse\nskip-liquids\tfalse\nmaximum-slope\t55\nnon-steep-slope\t55\n";
#[test]
fn v14_canonical_collision_domain_and_profile() {
    for domain in ["all-available-maps", "map:0", "map:4294967295"] {
        let text = V14.replace("all-available-maps", domain);
        let plan = Plan::parse(&text).unwrap();
        assert!(plan.is_v14());
        assert_eq!(plan.version(), 14);
        assert_eq!(plan.contract(), "forever-product-v14");
        assert_eq!(plan.render(), text);
    }
}
#[test]
fn v14_exact_mask_build_catalog_and_disabled_keys() {
    let p = Plan::parse(V14).unwrap();
    for mask in 0..=255 {
        if mask != Component::Collision.bit() {
            assert!(
                Plan {
                    components: mask,
                    ..p.clone()
                }
                .validate()
                .is_err()
            );
        }
    }
    for build in [0, 70170, 70205, 70338, 70374] {
        assert!(Plan { build, ..p.clone() }.validate().is_err());
    }
    for schema_catalog in [
        SchemaCatalog::PinnedMetadataV1,
        SchemaCatalog::WowDbDefs70205V1,
        SchemaCatalog::WowDbDefs70338V1,
        SchemaCatalog::WowDbDefs70374V1,
    ] {
        assert!(
            Plan {
                schema_catalog,
                ..p.clone()
            }
            .validate()
            .is_err()
        );
    }
    assert!(
        Plan {
            decryption: Decryption::ProvidedSalsa20V1,
            ..p
        }
        .validate()
        .is_err()
    );
}
#[test]
fn v14_rejects_unsupported_storage_and_unused_policies() {
    let p = Plan::parse(V14).unwrap();
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
fn v14_closed_canonical_domain_and_metadata_grammar() {
    for value in [
        "all",
        "none",
        "tile:0:1:1",
        "map:00",
        "map:-1",
        "map:4294967296",
        "map:",
    ] {
        assert!(Plan::parse(&V14.replace("all-available-maps", value)).is_err());
    }
    for (a, b) in [
        (
            "compatible-collision-coverage-v1",
            "compatible-collision-coverage-v2",
        ),
        (COMPATIBLE_COLLISION_VERIFY_PROFILE, "legacy"),
        ("1048576", "0"),
    ] {
        assert!(Plan::parse(&V14.replace(a, b)).is_err());
    }
}
#[test]
fn v14_cannot_relabel_legacy_or_other_target_profiles() {
    let p = Plan::parse(V14).unwrap();
    for profile in [
        PublicationProfile::Legacy,
        PublicationProfile::Build70374Db2OnlyV1,
        PublicationProfile::Build70374CompatibleTerrainV1,
    ] {
        assert!(
            Plan {
                publication_profile: profile,
                ..p.clone()
            }
            .validate()
            .is_err()
        );
    }
    assert!(Plan::parse(&V14.replace("forever-product\t14", "forever-product\t13")).is_err());
}
