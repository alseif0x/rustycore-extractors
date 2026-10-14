// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
const V10: &str =
    include_str!("../../../../tests/contracts/fixtures/forever-product-v10/disabled.tsv");
fn plan() -> Plan {
    Plan::parse(V10).unwrap()
}
#[test]
fn v10_exact_explicit_profile_and_canonical_plan() {
    let p = plan();
    assert_eq!(p.version(), 10);
    assert_eq!(p.contract(), "forever-product-v10");
    assert_eq!(
        p.publication_profile,
        PublicationProfile::Build70374Db2OnlyV1
    );
    assert_eq!(p.components, 1);
    assert_eq!(p.render(), V10);
}
#[test]
fn v10_both_keys_locales_and_layouts_roundtrip() {
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
                    assert_eq!(Plan::parse(&p.render()), Ok(p));
                }
            }
        }
    }
}
#[test]
fn v10_rejects_every_other_component_mask() {
    for components in 0..=255 {
        if components != 1 {
            assert_eq!(
                Plan {
                    components,
                    ..plan()
                }
                .validate(),
                Err("product_plan_component_scope")
            );
        }
    }
}
#[test]
fn v10_requires_its_own_build_and_catalog() {
    for build in [0, 70170, 70205, 70338, u32::MAX] {
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
fn v10_catalog_never_selects_new_publication_implicitly() {
    let p = Plan {
        publication_profile: PublicationProfile::Legacy,
        ..plan()
    };
    assert_eq!(p.validate(), Err("product_plan_build"));
    assert_ne!(p.version(), 10);
    for version in [1, 2, 3, 4, 5, 6, 8, 9] {
        assert!(
            Plan::parse(&V10.replace(
                "forever-product\t10",
                &format!("forever-product\t{version}")
            ))
            .is_err()
        );
    }
}
#[test]
fn v10_rejects_disk_domain_offmesh_and_geometry_options() {
    for p in [
        Plan {
            map_domain: Some(0),
            ..plan()
        },
        Plan {
            offmesh: Some(vec![]),
            ..plan()
        },
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
            ..plan()
        },
    ] {
        assert_eq!(p.validate(), Err("product_plan_component_scope"));
    }
    for p in [
        Plan {
            terrain: Selection::All,
            ..plan()
        },
        Plan {
            navigation: Selection::Map(0),
            ..plan()
        },
    ] {
        assert_eq!(p.validate(), Err("product_plan_dependencies"));
    }
    for p in [
        Plan {
            pack: false,
            big_base_unit: true,
            ..plan()
        },
        Plan {
            skip_liquids: true,
            ..plan()
        },
        Plan {
            maximum_slope: 60,
            ..plan()
        },
    ] {
        assert_eq!(p.validate(), Err("product_plan_options"));
    }
}
#[test]
fn v10_requires_complete_exact_selected_table_identity() {
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
    p.db2 = vec![p.db2[0].clone(); MAX_70374_DB2_ENTRIES + 1];
    assert_eq!(p.validate(), Err("product_plan_size"));
}
#[test]
fn v10_rejects_aliases_unknown_fields_and_reserved7() {
    for s in [
        V10.replace("forever-product\t10", "forever-product\t010"),
        V10.replace("forever-product\t10", "forever-product\t7"),
        V10.replace("components\t", "backend\trust-local\ncomponents\t"),
        V10.replace("components\tdb2", "components\tdb2,gt"),
        V10.trim_end().into(),
    ] {
        assert!(Plan::parse(&s).is_err());
    }
}
#[test]
fn v10_all1161_names_keep_exact_new_catalog_identity() {
    let mut p = plan();
    p.db2 = catalog::tables_for(catalog::WOWDBDEFS_70374_CATALOG_ID)
        .unwrap()
        .iter()
        .map(|m| Entry {
            name: m.name.into(),
            id: m.file_data_id,
        })
        .collect();
    assert_eq!(p.db2.len(), 1161);
    p.validate().unwrap();
    assert_eq!(Plan::parse(&p.render()), Ok(p));
}
#[test]
fn v10_does_not_change_old_unknown_build_error() {
    let p = Plan {
        publication_profile: PublicationProfile::Legacy,
        schema_catalog: SchemaCatalog::PinnedMetadataV1,
        ..plan()
    };
    assert_eq!(p.validate(), Err("product_plan_build"));
}
