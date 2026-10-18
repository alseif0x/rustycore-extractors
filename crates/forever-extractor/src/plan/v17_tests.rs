// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

const V17: &str =
    include_str!("../../../../tests/contracts/fixtures/forever-product-v17/provided.tsv");

fn plan() -> Plan {
    Plan::parse(V17).unwrap()
}

#[test]
fn v17_exact_identity_is_the_70450_provided_key_db2_profile() {
    let p = plan();
    assert_eq!(p.version(), 17);
    assert_eq!(p.contract(), "forever-product-v17");
    assert_eq!(
        p.publication_profile,
        PublicationProfile::Build70450Db2ProvidedKeysV1
    );
    assert_eq!(p.build, 70450);
    assert_eq!(p.schema_catalog.id(), "wowdbdefs-70450-v1");
    assert_eq!(p.components, Component::Db2.bit());
    assert_eq!(p.collision_storage, CollisionStorage::Resident);
    assert_eq!(p.decryption, Decryption::ProvidedSalsa20V1);
    assert!(p.requires_keys());
    assert_eq!(p.map_domain, None);
    assert_eq!(p.terrain, Selection::None);
    assert_eq!(p.navigation, Selection::None);
    assert_eq!(p.offmesh, None);
    assert_eq!(p.render(), V17);
}

#[test]
fn v17_locales_and_layouts_roundtrip_with_provided_decryption() {
    for locale in [Locale::EnUs, Locale::EsEs] {
        for index_layout in [local_index::Layout::Contiguous, local_index::Layout::Paged] {
            for root_layout in [
                root::Layout::Headerless18125,
                root::Layout::Header30080,
                root::Layout::Header50893V1,
                root::Layout::Header50893V2,
            ] {
                let p = Plan {
                    locale,
                    index_layout,
                    root_layout,
                    ..plan()
                };
                assert_eq!(Plan::parse(&p.render()), Ok(p));
            }
        }
    }
    assert_eq!(
        Plan {
            decryption: Decryption::Disabled,
            ..plan()
        }
        .validate(),
        Err("product_plan_component_scope")
    );
}

#[test]
fn v17_rejects_non_db2_components_and_all_world_scope() {
    let p = plan();
    for components in 0..=u8::MAX {
        if components != Component::Db2.bit() {
            assert_eq!(
                Plan {
                    components,
                    ..p.clone()
                }
                .validate(),
                Err("product_plan_component_scope")
            );
        }
    }
    for q in [
        Plan {
            components: Component::Collision.bit(),
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
            map_domain: Some(0),
            ..p.clone()
        },
        Plan {
            offmesh: Some(vec![]),
            ..p.clone()
        },
    ] {
        assert!(q.validate().is_err());
    }
}

#[test]
fn v17_requires_build_70450_and_its_exact_catalogue() {
    let p = plan();
    for build in [0, 70170, 70338, 70374, 70425, u32::MAX] {
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
    let legacy = Plan {
        publication_profile: PublicationProfile::Legacy,
        ..p
    };
    assert_eq!(legacy.validate(), Err("product_plan_build"));
    assert_ne!(legacy.version(), 17);
}

#[test]
fn v17_requires_a_nonempty_exact_catalogue_db2_list() {
    let p = plan();
    assert_eq!(
        Plan {
            db2: Vec::new(),
            ..p.clone()
        }
        .validate(),
        Err("product_plan_dependencies")
    );
    assert_eq!(
        Plan {
            db2: vec![Entry {
                name: "ForeignTable".into(),
                id: 1,
            }],
            ..p.clone()
        }
        .validate(),
        Err("product_plan_schema")
    );
    let mut wrong_id = p;
    wrong_id.db2[0].id += 1;
    assert_eq!(wrong_id.validate(), Err("product_plan_schema"));
}

#[test]
fn v17_catalogue_supports_all_1161_exact_names() {
    let mut p = plan();
    p.db2 = catalog::tables_for(catalog::WOWDBDEFS_70450_CATALOG_ID)
        .unwrap()
        .iter()
        .map(|metadata| Entry {
            name: metadata.name.into(),
            id: metadata.file_data_id,
        })
        .collect();
    assert_eq!(p.db2.len(), 1161);
    p.validate().unwrap();
    assert_eq!(Plan::parse(&p.render()), Ok(p));
}

#[test]
fn v14_v15_and_v16_identities_remain_disabled_key_profiles() {
    let v14 = Plan::parse(super::v14_tests::V14).unwrap();
    let v15 = Plan::parse(super::v15_tests::V15).unwrap();
    let v16 = Plan::parse(include_str!(
        "../../../../tests/contracts/fixtures/forever-product-v16/disabled.tsv"
    ))
    .unwrap();
    for (revision, profile) in [(v14, 14), (v15, 15), (v16, 16)] {
        assert_eq!(revision.version(), profile);
        assert!(!revision.requires_keys());
        assert_eq!(revision.validate(), Ok(()));
    }
}
