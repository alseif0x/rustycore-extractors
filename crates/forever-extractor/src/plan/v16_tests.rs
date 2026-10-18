// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

const V16: &str =
    include_str!("../../../../tests/contracts/fixtures/forever-product-v16/disabled.tsv");

fn plan() -> Plan {
    Plan::parse(V16).unwrap()
}

#[test]
fn v16_exact_explicit_profile_and_canonical_plan() {
    let p = plan();
    assert_eq!(p.version(), 16);
    assert_eq!(p.contract(), "forever-product-v16");
    assert_eq!(
        p.publication_profile,
        PublicationProfile::Build70450Db2OnlyV1
    );
    assert_eq!(p.build, 70450);
    assert_eq!(p.schema_catalog.id(), "wowdbdefs-70450-v1");
    assert_eq!(p.components, Component::Db2.bit());
    assert_eq!(p.collision_storage, CollisionStorage::Resident);
    assert_eq!(p.decryption, Decryption::Disabled);
    assert_eq!(p.map_domain, None);
    assert_eq!(p.terrain, Selection::None);
    assert_eq!(p.navigation, Selection::None);
    assert_eq!(p.offmesh, None);
    assert_eq!(p.render(), V16);
}

#[test]
fn v16_locales_and_layouts_roundtrip_with_disabled_decryption() {
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
            decryption: Decryption::ProvidedSalsa20V1,
            ..plan()
        }
        .validate(),
        Err("product_plan_component_scope")
    );
}

#[test]
fn v16_requires_exact_db2_component_mask_and_rejects_collision_or_terrain() {
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
    assert_eq!(
        Plan {
            components: Component::Collision.bit(),
            ..p.clone()
        }
        .validate(),
        Err("product_plan_component_scope")
    );
    assert_eq!(
        Plan {
            components: Component::Terrain.bit(),
            terrain: Selection::All,
            ..p.clone()
        }
        .validate(),
        Err("product_plan_component_scope")
    );
    assert_eq!(
        Plan {
            map_domain: Some(0),
            ..p.clone()
        }
        .validate(),
        Err("product_plan_component_scope")
    );
    assert_eq!(
        Plan {
            offmesh: Some(vec![]),
            ..p.clone()
        }
        .validate(),
        Err("product_plan_component_scope")
    );
    assert_eq!(
        Plan {
            terrain: Selection::All,
            ..p.clone()
        }
        .validate(),
        Err("product_plan_dependencies")
    );
}

#[test]
fn v16_requires_70450_and_its_exact_catalog() {
    let p = plan();
    for build in [0, 70338, 70374, 70425, u32::MAX] {
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
}

#[test]
fn v16_requires_a_nonempty_exact_catalog_db2_list() {
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
fn v16_catalog_supports_all_1161_exact_names() {
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
fn v16_catalog_does_not_implicitly_select_the_profile_and_v15_stays_collision_only() {
    let p = Plan {
        publication_profile: PublicationProfile::Legacy,
        ..plan()
    };
    assert_eq!(p.validate(), Err("product_plan_build"));
    assert_ne!(p.version(), 16);

    let v15 = Plan::parse(super::v15_tests::V15).unwrap();
    assert_eq!(v15.version(), 15);
    assert_eq!(v15.contract(), "forever-product-v15");
    assert_eq!(v15.validate(), Ok(()));
    assert_eq!(
        Plan {
            components: Component::Db2.bit(),
            ..v15
        }
        .validate(),
        Err("product_plan_component_scope")
    );
}

#[test]
fn v16_rejects_aliases_unknown_fields_and_reserved_versions() {
    for text in [
        V16.replace("forever-product\t16", "forever-product\t016"),
        V16.replace("forever-product\t16", "forever-product\t7"),
        V16.replace("forever-product\t16", "forever-product\t17"),
        V16.replace("components\tdb2", "components\tdb2,gt"),
        V16.replace("components\tdb2", "components\tvmaps"),
        V16.trim_end().into(),
    ] {
        assert!(Plan::parse(&text).is_err());
    }
}
