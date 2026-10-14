// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
const V11: &str =
    include_str!("../../../../tests/contracts/fixtures/forever-product-v11/disabled.tsv");
fn plan() -> Plan {
    Plan::parse(V11).unwrap()
}
#[test]
fn v11_exact_canonical_profile_and_fixed_components() {
    let p = plan();
    assert_eq!(p.version(), 11);
    assert_eq!(p.contract(), "forever-product-v11");
    assert_eq!(
        p.publication_profile,
        PublicationProfile::Build70374Db2GtCamerasV1
    );
    assert_eq!(p.components, 7);
    assert_eq!(p.render(), V11);
    assert!(!p.requires_keys());
}
#[test]
fn v11_both_policies_layouts_and_locales_roundtrip() {
    for locale in [Locale::EnUs, Locale::EsEs] {
        for decryption in [Decryption::Disabled, Decryption::ProvidedSalsa20V1] {
            for index_layout in [local_index::Layout::Contiguous, local_index::Layout::Paged] {
                for root_layout in [
                    root::Layout::Headerless18125,
                    root::Layout::Header30080,
                    root::Layout::Header50893V1,
                    root::Layout::Header50893V2,
                ] {
                    let p = Plan {
                        locale,
                        decryption,
                        index_layout,
                        root_layout,
                        ..plan()
                    };
                    assert_eq!(p.validate(), Ok(()));
                    assert_eq!(Plan::parse(&p.render()), Ok(p));
                }
            }
        }
    }
}
#[test]
fn v11_rejects_every_other_component_mask_before_paths() {
    for components in 0..=255 {
        if components == 7 {
            continue;
        }
        let p = Plan {
            components,
            ..plan()
        };
        assert_eq!(p.validate(), Err("product_plan_component_scope"));
    }
}
#[test]
fn v11_requires_exact_build_and_catalog() {
    for build in [70170, 70205, 70338, 0, u32::MAX] {
        assert_eq!(
            Plan { build, ..plan() }.validate(),
            Err("product_plan_build")
        );
    }
    for schema_catalog in [
        SchemaCatalog::PinnedMetadataV1,
        SchemaCatalog::WowDbDefs70205V1,
        SchemaCatalog::WowDbDefs70338V1,
    ] {
        assert_eq!(
            Plan {
                schema_catalog,
                ..plan()
            }
            .validate(),
            Err("product_plan_schema_catalog")
        );
    }
}
#[test]
fn v11_does_not_widen_v10_or_v9_or_legacy() {
    let mut p = plan();
    p.publication_profile = PublicationProfile::Build70374Db2OnlyV1;
    assert_eq!(p.validate(), Err("product_plan_component_scope"));
    p.components = Component::Db2.bit();
    p.validate().unwrap();
    assert_eq!(p.version(), 10);
    assert_eq!(Plan::parse(&p.render()), Ok(p));
    let mut p = plan();
    p.publication_profile = PublicationProfile::Legacy;
    assert_eq!(p.validate(), Err("product_plan_build"));
    let mut p = plan();
    p.publication_profile = PublicationProfile::Build70338Db2GtCamerasV1;
    assert_eq!(p.validate(), Err("product_plan_build"));
}
#[test]
fn v11_rejects_geometry_capabilities_and_selections() {
    assert_eq!(
        Plan {
            map_domain: Some(0),
            ..plan()
        }
        .validate(),
        Err("product_plan_component_scope")
    );
    assert_eq!(
        Plan {
            offmesh: Some(vec![]),
            ..plan()
        }
        .validate(),
        Err("product_plan_component_scope")
    );
    let disk = CollisionStorage::LeasedDiskV1(DiskBudget {
        raw_bytes: 1,
        wdt_bytes: 1,
        cache_bytes: 1,
        placement_bytes: 1,
        vmo_bytes: 1,
        map_bytes: 1,
        output_bytes: 1,
    });
    assert_eq!(
        Plan {
            collision_storage: disk,
            ..plan()
        }
        .validate(),
        Err("product_plan_component_scope")
    );
    assert_eq!(
        Plan {
            terrain: Selection::All,
            ..plan()
        }
        .validate(),
        Err("product_plan_dependencies")
    );
    assert_eq!(
        Plan {
            navigation: Selection::Map(0),
            ..plan()
        }
        .validate(),
        Err("product_plan_dependencies")
    );
}
#[test]
fn v11_requires_canonical_target_dependency_rows() {
    assert_eq!(
        Plan {
            db2: vec![],
            ..plan()
        }
        .validate(),
        Err("product_plan_dependencies")
    );
    let mut p = plan();
    p.db2[0].id += 1;
    assert_eq!(p.validate(), Err("product_plan_schema"));
    let mut p = plan();
    p.db2.push(p.db2[0].clone());
    assert_eq!(p.validate(), Err("product_plan_schema"));
    let mut p = plan();
    p.db2[0].name = "TactKey".into();
    p.db2[0].id = catalog::resolve_for(p.schema_catalog.id(), "TactKey")
        .unwrap()
        .unwrap()
        .metadata()
        .file_data_id;
    assert_eq!(p.validate(), Err("product_plan_dependencies"));
}
#[test]
fn v11_unknown_alias_extra_fields_and_reserved7_reject() {
    for changed in [
        V11.replace("forever-product\t11", "forever-product\t7"),
        V11.replace("forever-product\t11", "forever-product\t09"),
        V11.replace(
            "components\t",
            "gt-profile\tgt-catalog-70374-v1\ncomponents\t",
        ),
        V11.replace(
            "components\t",
            "collision-storage\tlinux-leased-append-disk-world-v1\ncomponents\t",
        ),
        V11.replace("db2,gt,cameras", "db2,cameras,gt"),
        V11.trim_end().into(),
    ] {
        assert!(Plan::parse(&changed).is_err());
    }
}
#[test]
fn v11_preserves_unselected_navigation_defaults() {
    for p in [
        Plan {
            big_base_unit: true,
            ..plan()
        },
        Plan {
            skip_liquids: true,
            ..plan()
        },
        Plan {
            maximum_slope: 70,
            ..plan()
        },
        Plan {
            non_steep_slope: 50,
            ..plan()
        },
    ] {
        assert_eq!(p.validate(), Err("product_plan_options"));
    }
}

#[test]
fn v11_all_target_catalog_entries_are_exact_and_include_camera_source() {
    let mut p = plan();
    p.db2 = catalog::tables_for(catalog::WOWDBDEFS_70374_CATALOG_ID)
        .unwrap()
        .iter()
        .map(|m| Entry {
            name: m.name.into(),
            id: m.file_data_id,
        })
        .collect();
    assert_eq!(p.db2.len(), MAX_70374_DB2_ENTRIES);
    p.validate().unwrap();
    assert_eq!(Plan::parse(&p.render()), Ok(p));
}
