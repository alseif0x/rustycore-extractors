// SPDX-License-Identifier: GPL-3.0-or-later
//! Synthetic build-70450 partial-catalog DB2 and authenticated terrain plan admission.
use super::*;

const PARTIAL: &str =
    include_str!("../../../../tests/contracts/fixtures/forever-product-v27/partial.tsv");
const ALL: &str = include_str!("../../../../tests/contracts/fixtures/forever-product-v27/all.tsv");
const V25: &str = include_str!("../../../../tests/contracts/fixtures/forever-product-v25/map2.tsv");
const V26: &str =
    include_str!("../../../../tests/contracts/fixtures/forever-product-v26/single.tsv");

fn plan() -> Plan {
    Plan::parse(PARTIAL).unwrap()
}

#[test]
fn v27_admits_exact_profile_and_canonical_roundtrips() {
    for (source, terrain) in [(PARTIAL, Selection::Map(48)), (ALL, Selection::All)] {
        let product = Plan::parse(source).unwrap();
        assert_eq!(product.version(), 27);
        assert_eq!(product.contract(), "forever-product-v27");
        assert_eq!(
            product.publication_profile,
            PublicationProfile::Build70450PartialCatalogTerrainV1
        );
        assert!(product.is_v27());
        assert!(!product.is_v26());
        assert!(!product.is_v25());
        assert!(product.is_available_terrain());
        assert_eq!(product.build, 70450);
        assert_eq!(product.schema_catalog, SchemaCatalog::WowDbDefs70450V1);
        assert_eq!(
            product.components,
            Component::Db2.bit() | Component::Terrain.bit()
        );
        assert_eq!(product.components, 9);
        assert_eq!(product.collision_storage, CollisionStorage::Resident);
        assert_eq!(product.decryption, Decryption::Disabled);
        assert_eq!(product.terrain, terrain);
        assert_eq!(product.navigation, Selection::None);
        assert_eq!(product.map_domain, None);
        assert_eq!(product.offmesh, None);
        assert!(product.pack);
        assert!(!product.big_base_unit);
        assert!(!product.skip_liquids);
        assert_eq!((product.maximum_slope, product.non_steep_slope), (55, 55));
        assert_eq!(
            product.db2,
            [
                Entry {
                    name: "Map".into(),
                    id: 1_349_477,
                },
                Entry {
                    name: "SpellEffect".into(),
                    id: 1_140_088,
                },
            ]
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
fn v27_requires_the_exact_partial_encrypted_sections_plan_gate() {
    assert_eq!(
        Plan::parse(&PARTIAL.replace("partial_encrypted_sections\tdeclared-skip\n", "")),
        Err("product_plan_partial_encrypted_sections")
    );
    assert_eq!(
        Plan::parse(&PARTIAL.replace(
            "partial_encrypted_sections\tdeclared-skip",
            "partial_encrypted_sections\tallow",
        )),
        Err("product_plan_partial_encrypted_sections")
    );
}

#[test]
fn v27_rejects_every_other_component_mask_and_provided_decryption() {
    let base = plan();
    assert_eq!(
        Plan {
            decryption: Decryption::ProvidedSalsa20V1,
            ..base.clone()
        }
        .validate(),
        Err("product_plan_component_scope")
    );
    for mask in 0..=u8::MAX {
        if mask != 9 {
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
fn v27_requires_exactly_one_canonical_map_and_admits_other_catalog_tables() {
    let base = plan();
    assert_eq!(
        base.db2.iter().filter(|entry| entry.name == "Map").count(),
        1
    );
    assert!(base.db2.iter().any(|entry| entry.name == "SpellEffect"));

    for changed in [
        Plan {
            db2: Vec::new(),
            ..base.clone()
        },
        Plan {
            db2: base.db2[1..].to_vec(),
            ..base.clone()
        },
        Plan {
            db2: {
                let mut entries = base.db2.clone();
                entries.insert(1, entries[0].clone());
                entries
            },
            ..base.clone()
        },
    ] {
        assert_eq!(changed.validate(), Err("product_plan_dependencies"));
    }

    let mut wrong_map = base;
    wrong_map.db2[0].id += 1;
    assert_eq!(wrong_map.validate(), Err("product_plan_schema"));
}

#[test]
fn v27_admits_only_all_available_or_single_map_terrain() {
    let base = plan();
    for terrain in [
        Selection::None,
        Selection::Tile {
            map: 48,
            x: 3,
            y: 4,
        },
    ] {
        assert_eq!(
            Plan {
                terrain,
                ..base.clone()
            }
            .validate(),
            Err("product_terrain_selection")
        );
    }
}

#[test]
fn v27_admission_binds_build_catalog_resident_storage_and_unchanged_options() {
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
            map_domain: Some(48),
            ..base.clone()
        },
        Plan {
            navigation: Selection::Map(48),
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
fn v27_preserves_the_1161_catalog_limit_and_rejects_1162() {
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
    assert_eq!(
        product
            .db2
            .iter()
            .filter(|entry| entry.name == "Map")
            .count(),
        1
    );
    product.validate().unwrap();
    assert_eq!(Plan::parse(&product.render()), Ok(product.clone()));

    product.db2.push(Entry {
        name: "SyntheticOverflow".into(),
        id: u32::MAX,
    });
    assert_eq!(product.db2.len(), 1162);
    assert!(product.validate().is_err());
}

#[test]
fn v27_does_not_relabel_existing_v25_or_v26_plans() {
    for (source, version, contract, profile) in [
        (
            V25,
            25,
            "forever-product-v25",
            PublicationProfile::Build70450AuthenticatedTerrainPartialMapV1,
        ),
        (
            V26,
            26,
            "forever-product-v26",
            PublicationProfile::Build70450Db2PartialV1,
        ),
    ] {
        let product = Plan::parse(source).unwrap();
        assert_eq!(product.version(), version);
        assert_eq!(product.contract(), contract);
        assert_eq!(product.publication_profile, profile);
        assert!(!product.is_v27());
        assert_eq!(product.render(), source);
    }
}
