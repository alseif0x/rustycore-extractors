// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;

pub(crate) const V19: &str =
    include_str!("../../../../tests/contracts/fixtures/forever-product-v19/disabled.tsv");

fn plan() -> Plan {
    Plan::parse(V19).unwrap()
}

#[test]
fn v19_is_the_exact_70450_catalog_db2_gt_camera_profile() {
    let p = plan();
    assert_eq!(p.version(), 19);
    assert_eq!(p.contract(), "forever-product-v19");
    assert_eq!(
        p.publication_profile,
        PublicationProfile::Build70450Db2GtCamerasV1
    );
    assert_eq!(p.build, 70450);
    assert_eq!(p.schema_catalog.id(), "wowdbdefs-70450-v1");
    assert_eq!(
        p.components,
        Component::Db2.bit() | Component::GameTables.bit() | Component::Cameras.bit()
    );
    assert_eq!(p.collision_storage, CollisionStorage::Resident);
    assert_eq!(p.decryption, Decryption::Disabled);
    assert!(!p.requires_keys());
    assert_eq!(p.db2.len(), 1);
    assert_eq!(p.db2[0].name, "CinematicCamera");
    assert_eq!(p.db2[0].id, 1_294_214);
    assert_eq!(p.terrain, Selection::None);
    assert_eq!(p.navigation, Selection::None);
    assert_eq!(p.map_domain, None);
    assert_eq!(p.offmesh, None);
    assert!(p.is_gt_cameras());
    assert_eq!(p.render(), V19);
}

#[test]
fn v19_requires_db2_camera_source_and_the_70450_cinematiccamera_fdid() {
    let p = plan();
    let camera_id = catalog::resolve_for(p.schema_catalog.id(), "CinematicCamera")
        .unwrap()
        .unwrap()
        .metadata()
        .file_data_id;
    assert_eq!(camera_id, 1_294_214);

    assert_eq!(
        Plan {
            components: Component::GameTables.bit() | Component::Cameras.bit(),
            db2: Vec::new(),
            ..p.clone()
        }
        .validate(),
        Err("product_plan_component_scope")
    );
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
                name: "Map".into(),
                id: catalog::resolve_for(p.schema_catalog.id(), "Map")
                    .unwrap()
                    .unwrap()
                    .metadata()
                    .file_data_id,
            }],
            ..p.clone()
        }
        .validate(),
        Err("product_plan_dependencies")
    );
    let mut wrong_camera_id = p;
    wrong_camera_id.db2[0].id += 1;
    assert_eq!(wrong_camera_id.validate(), Err("product_plan_schema"));
}

#[test]
fn v19_rejects_every_non_target_component_mask() {
    let p = plan();
    let expected = Component::Db2.bit() | Component::GameTables.bit() | Component::Cameras.bit();
    for components in 0..=u8::MAX {
        if components != expected {
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
}

#[test]
fn v19_rejects_geometry_capabilities_and_selection_fields() {
    let p = plan();
    for changed in [
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
            ..p.clone()
        },
        Plan {
            map_domain: Some(0),
            ..p.clone()
        },
        Plan {
            terrain: Selection::All,
            ..p.clone()
        },
        Plan {
            navigation: Selection::Map(0),
            ..p.clone()
        },
        Plan {
            offmesh: Some(vec![]),
            ..p.clone()
        },
    ] {
        assert!(changed.validate().is_err());
    }
}

#[test]
fn v19_requires_build_70450_and_the_exact_schema_catalogue() {
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
}

#[test]
fn v19_disabled_key_policy_and_versions_14_to_18_are_preserved() {
    let p = plan();
    assert_eq!(
        Plan {
            decryption: Decryption::ProvidedSalsa20V1,
            ..p.clone()
        }
        .validate(),
        Err("product_plan_component_scope")
    );

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
    let v18 = Plan::parse(include_str!(
        "../../../../tests/contracts/fixtures/forever-product-v18/all-available-maps.tsv"
    ))
    .unwrap();
    for (revision, version, requires_keys) in [
        (v14, 14, false),
        (v15, 15, false),
        (v16, 16, false),
        (v17, 17, true),
        (v18, 18, true),
    ] {
        assert_eq!(revision.version(), version);
        assert_eq!(revision.requires_keys(), requires_keys);
        revision.validate().unwrap();
    }
}

#[test]
fn v19_layout_and_locale_variants_retain_canonical_rendering() {
    let p = plan();
    for locale in [Locale::EnUs, Locale::EsEs] {
        for index_layout in [local_index::Layout::Contiguous, local_index::Layout::Paged] {
            for root_layout in [
                root::Layout::Headerless18125,
                root::Layout::Header30080,
                root::Layout::Header50893V1,
                root::Layout::Header50893V2,
            ] {
                let variant = Plan {
                    locale,
                    index_layout,
                    root_layout,
                    ..p.clone()
                };
                assert_eq!(Plan::parse(&variant.render()), Ok(variant));
            }
        }
    }
}
