// SPDX-License-Identifier: GPL-3.0-or-later
//! Synthetic build-70450 DB2-only declared-partial-section plan admission.
use super::*;

const SINGLE: &str =
    include_str!("../../../../tests/contracts/fixtures/forever-product-v26/single.tsv");
const MULTIPLE: &str =
    include_str!("../../../../tests/contracts/fixtures/forever-product-v26/multiple.tsv");
const V16: &str =
    include_str!("../../../../tests/contracts/fixtures/forever-product-v16/disabled.tsv");
const V25: &str = include_str!("../../../../tests/contracts/fixtures/forever-product-v25/map2.tsv");

fn plan() -> Plan {
    Plan::parse(SINGLE).unwrap()
}

#[test]
fn v26_admits_exact_db2_only_profile_and_canonical_roundtrips() {
    for (source, entries) in [
        (SINGLE, vec![("AccountStoreCategory", 6_220_124_u32)]),
        (
            MULTIPLE,
            vec![
                ("AccountStoreCategory", 6_220_124_u32),
                ("AnimationData", 1_375_431_u32),
            ],
        ),
    ] {
        let product = Plan::parse(source).unwrap();
        assert_eq!(product.version(), 26);
        assert_eq!(product.contract(), "forever-product-v26");
        assert_eq!(
            product.publication_profile,
            PublicationProfile::Build70450Db2PartialV1
        );
        assert!(product.is_v26());
        assert!(!product.is_v16());
        assert!(!product.is_v25());
        assert_eq!(product.build, 70450);
        assert_eq!(product.schema_catalog, SchemaCatalog::WowDbDefs70450V1);
        assert_eq!(product.components, Component::Db2.bit());
        assert_eq!(product.collision_storage, CollisionStorage::Resident);
        assert_eq!(product.decryption, Decryption::Disabled);
        assert_eq!(product.terrain, Selection::None);
        assert_eq!(product.navigation, Selection::None);
        assert_eq!(product.map_domain, None);
        assert_eq!(product.offmesh, None);
        assert!(product.pack);
        assert!(!product.big_base_unit);
        assert!(!product.skip_liquids);
        assert_eq!((product.maximum_slope, product.non_steep_slope), (55, 55));
        assert_eq!(
            product
                .db2
                .iter()
                .map(|entry| (entry.name.as_str(), entry.id))
                .collect::<Vec<_>>(),
            entries
        );
        assert!(
            product
                .render()
                .contains("partial_encrypted_sections\tdeclared-skip\n")
        );
        assert_eq!(product.render(), source);
        assert_eq!(Plan::parse(&product.render()), Ok(product));
    }
}

#[test]
fn v26_requires_the_exact_partial_encrypted_sections_declaration() {
    assert_eq!(
        Plan::parse(&SINGLE.replace("partial_encrypted_sections\tdeclared-skip\n", "",)),
        Err("product_plan_partial_encrypted_sections")
    );
    assert_eq!(
        Plan::parse(&SINGLE.replace(
            "partial_encrypted_sections\tdeclared-skip",
            "partial_encrypted_sections\tallow",
        )),
        Err("product_plan_partial_encrypted_sections")
    );
}

#[test]
fn v26_rejects_provided_decryption_and_non_db2_component_sets() {
    assert_eq!(
        Plan::parse(&SINGLE.replace("decryption\tdisabled", "decryption\tprovided-salsa20-v1",)),
        Err("product_plan_component_scope")
    );
    assert_eq!(
        Plan::parse(&SINGLE.replace("components\tdb2", "components\tdb2,maps")),
        Err("product_plan_component_scope")
    );
    let base = plan();
    for mask in 0..=u8::MAX {
        if mask != Component::Db2.bit() {
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
}

#[test]
fn v26_admission_binds_build_catalog_and_all_db2_only_options() {
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
    for changed in [
        Plan {
            collision_storage: CollisionStorage::LeasedDiskV1(DiskBudget {
                raw_bytes: 32,
                wdt_bytes: 32,
                cache_bytes: 32,
                placement_bytes: 32,
                vmo_bytes: 32,
                map_bytes: 32,
                output_bytes: 1,
            }),
            ..base.clone()
        },
        Plan {
            map_domain: Some(2),
            ..base.clone()
        },
        Plan {
            terrain: Selection::All,
            ..base.clone()
        },
        Plan {
            navigation: Selection::Map(2),
            ..base.clone()
        },
        Plan {
            offmesh: Some(Vec::new()),
            ..base.clone()
        },
        Plan {
            pack: false,
            ..base.clone()
        },
        Plan {
            big_base_unit: true,
            ..base.clone()
        },
        Plan {
            skip_liquids: true,
            ..base.clone()
        },
        Plan {
            maximum_slope: 54,
            ..base.clone()
        },
        Plan {
            non_steep_slope: 54,
            ..base.clone()
        },
    ] {
        assert_eq!(changed.validate(), Err("product_plan_component_scope"));
    }
}

#[test]
fn v26_preserves_the_v16_catalog_limit_of_1161_entries() {
    let mut product = plan();
    product.db2 = catalog::tables_for(catalog::WOWDBDEFS_70450_CATALOG_ID)
        .unwrap()
        .iter()
        .map(|metadata| Entry {
            name: metadata.name.into(),
            id: metadata.file_data_id,
        })
        .collect();
    assert_eq!(product.db2.len(), 1161);
    product.validate().unwrap();
    assert_eq!(Plan::parse(&product.render()), Ok(product.clone()));

    product.db2.push(Entry {
        name: "SyntheticOverflow".into(),
        id: u32::MAX,
    });
    assert_eq!(product.validate(), Err("product_plan_size"));
}

#[test]
fn v26_does_not_relabel_existing_v16_or_v25_plans() {
    let v16 = Plan::parse(V16).unwrap();
    assert_eq!(v16.version(), 16);
    assert_eq!(v16.contract(), "forever-product-v16");
    assert_eq!(
        v16.publication_profile,
        PublicationProfile::Build70450Db2OnlyV1
    );
    assert_eq!(v16.render(), V16);

    let v25 = Plan::parse(V25).unwrap();
    assert_eq!(v25.version(), 25);
    assert_eq!(v25.contract(), "forever-product-v25");
    assert_eq!(
        v25.publication_profile,
        PublicationProfile::Build70450AuthenticatedTerrainPartialMapV1
    );
    assert_eq!(v25.render(), V25);
}
