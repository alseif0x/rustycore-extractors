// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

pub(crate) const V18: &str =
    include_str!("../../../../tests/contracts/fixtures/forever-product-v18/all-available-maps.tsv");

fn plan() -> Plan {
    Plan::parse(V18).unwrap()
}

#[test]
fn v18_is_the_exact_70450_keyed_compatible_collision_profile() {
    let p = plan();
    assert_eq!(p.version(), 18);
    assert_eq!(p.contract(), "forever-product-v18");
    assert_eq!(
        p.publication_profile,
        PublicationProfile::Build70450CollisionProvidedKeysV1
    );
    assert_eq!(p.build, 70450);
    assert_eq!(p.schema_catalog.id(), "wowdbdefs-70450-v1");
    assert_eq!(p.components, Component::Collision.bit());
    assert_eq!(
        p.collision_storage,
        CollisionStorage::LeasedDiskV1(DiskBudget {
            raw_bytes: 1_048_576,
            wdt_bytes: 1_048_576,
            cache_bytes: 1_048_576,
            placement_bytes: 1_048_576,
            vmo_bytes: 1_048_576,
            map_bytes: 1_048_576,
            output_bytes: 1_048_576,
        })
    );
    assert_eq!(p.decryption, Decryption::ProvidedSalsa20V1);
    assert!(p.requires_keys());
    assert!(p.db2.is_empty());
    assert_eq!(p.terrain, Selection::None);
    assert_eq!(p.navigation, Selection::None);
    assert_eq!(p.offmesh, None);
    assert!(p.pack);
    assert!(!p.big_base_unit);
    assert!(!p.skip_liquids);
    assert_eq!(p.maximum_slope, 55);
    assert_eq!(p.non_steep_slope, 55);
    assert_eq!(p.map_domain, None);
    assert_eq!(p.render(), V18);
}

#[test]
fn v18_both_locales_layouts_and_v15_collision_domains_roundtrip() {
    for locale in [Locale::EnUs, Locale::EsEs] {
        for index_layout in [local_index::Layout::Contiguous, local_index::Layout::Paged] {
            for root_layout in [
                root::Layout::Headerless18125,
                root::Layout::Header30080,
                root::Layout::Header50893V1,
                root::Layout::Header50893V2,
            ] {
                for map_domain in [None, Some(0), Some(u32::MAX)] {
                    let p = Plan {
                        locale,
                        index_layout,
                        root_layout,
                        map_domain,
                        ..plan()
                    };
                    assert_eq!(Plan::parse(&p.render()), Ok(p));
                }
            }
        }
    }
}

#[test]
fn v18_rejects_every_component_mask_except_collision() {
    let p = plan();
    for components in 0..=u8::MAX {
        if components != Component::Collision.bit() {
            assert_eq!(
                Plan {
                    components,
                    ..p.clone()
                }
                .validate(),
                Err("product_plan_component_scope"),
                "mask {components}"
            );
        }
    }
    for components in [
        Component::Db2.bit(),
        Component::Terrain.bit(),
        Component::Navigation.bit(),
        Component::Cameras.bit(),
        Component::GameTables.bit(),
        Component::Collision.bit() | Component::Db2.bit(),
    ] {
        assert_eq!(
            Plan {
                components,
                map_domain: Some(0),
                ..p.clone()
            }
            .validate(),
            Err("product_plan_component_scope")
        );
    }
}

#[test]
fn v18_rejects_any_non_collision_capability_or_unused_option() {
    let p = plan();
    for q in [
        Plan {
            collision_storage: CollisionStorage::Resident,
            ..p.clone()
        },
        Plan {
            decryption: Decryption::Disabled,
            ..p.clone()
        },
        Plan {
            db2: vec![Entry {
                name: "AccountStoreCategory".into(),
                id: 6_220_124,
            }],
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
            big_base_unit: true,
            ..p.clone()
        },
        Plan {
            skip_liquids: true,
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
fn v18_requires_its_exact_build_catalogue_and_profile() {
    let p = plan();
    for build in [0, 70170, 70205, 70338, 70374, 70425, u32::MAX] {
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
    for (publication_profile, expected) in [
        (PublicationProfile::Legacy, "product_plan_build"),
        (
            PublicationProfile::Build70450CompatibleCollisionV1,
            "product_plan_component_scope",
        ),
        (
            PublicationProfile::Build70450Db2OnlyV1,
            "product_plan_component_scope",
        ),
        (
            PublicationProfile::Build70450Db2ProvidedKeysV1,
            "product_plan_component_scope",
        ),
    ] {
        assert_eq!(
            Plan {
                publication_profile,
                ..p.clone()
            }
            .validate(),
            Err(expected)
        );
    }
}

#[test]
fn revisions_14_through_17_keep_their_key_admission_policies() {
    let v14 = Plan::parse(crate::plan::v14_tests::V14).unwrap();
    let v15 = Plan::parse(crate::plan::v15_tests::V15).unwrap();
    let v16 = Plan::parse(include_str!(
        "../../../../tests/contracts/fixtures/forever-product-v16/disabled.tsv"
    ))
    .unwrap();
    let v17 = Plan::parse(include_str!(
        "../../../../tests/contracts/fixtures/forever-product-v17/provided.tsv"
    ))
    .unwrap();
    for (revision, version, requires_keys) in [
        (v14, 14, false),
        (v15, 15, false),
        (v16, 16, false),
        (v17, 17, true),
    ] {
        assert_eq!(revision.version(), version);
        assert_eq!(revision.requires_keys(), requires_keys);
        revision.validate().unwrap();
    }
}
