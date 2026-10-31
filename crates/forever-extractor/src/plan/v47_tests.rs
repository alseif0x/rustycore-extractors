// SPDX-License-Identifier: GPL-3.0-or-later
//! Synthetic build-70450 GameObjectExpanded material-filtered-WMO plan admission.
use super::*;

const ALL: &str = include_str!("../../../../tests/contracts/fixtures/forever-product-v47/all.tsv");
const V46: &str = include_str!("../../../../tests/contracts/fixtures/forever-product-v46/all.tsv");
const V45: &str = include_str!("../../../../tests/contracts/fixtures/forever-product-v45/all.tsv");

fn plan() -> Plan {
    Plan::parse(ALL).unwrap()
}

#[test]
fn v47_admits_exact_gameobject_expanded_profile_and_canonical_roundtrip() {
    let product = plan();
    assert_eq!(product.version(), 47);
    assert_eq!(product.contract(), "forever-product-v47");
    assert_eq!(
        product.publication_profile,
        PublicationProfile::Build70450GameObjectExpandedMaterialFilteredWmoGeometryV1
    );
    assert!(product.is_v47());
    assert!(!product.is_v46());
    assert!(!product.is_v45());
    assert_eq!(product.build, 70450);
    assert_eq!(product.schema_catalog, SchemaCatalog::WowDbDefs70450V1);
    assert_eq!(
        product.components,
        Component::Db2.bit() | Component::Collision.bit()
    );
    assert_eq!(
        product.db2,
        [
            Entry {
                name: "GameObjectDisplayInfo".into(),
                id: 1_266_277,
            },
            Entry {
                name: "Map".into(),
                id: 1_349_477,
            },
        ]
    );
    assert_eq!(product.decryption, Decryption::Disabled);
    assert_eq!(product.map_domain, None);
    assert_eq!(product.render(), ALL);
    assert_eq!(Plan::parse(&product.render()), Ok(product));
    assert_eq!(
        GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_COLLISION_PROFILE,
        "linux70450-gameobject-expanded-material-filtered-wmo-vmap-product-v1"
    );
    assert_eq!(
        GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_COVERAGE_PROFILE,
        "linux70450-gameobject-expanded-material-filtered-wmo-coverage-v20"
    );
    assert_eq!(
        GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_VERIFY_PROFILE,
        "linux70450-gameobject-expanded-material-filtered-wmo-verification-v1"
    );
    assert_eq!(
        GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_VERIFY_SCOPE,
        "artifact_integrity_gameobject_expanded_material_filtered_wmo_and_independently_derived_map_godi_vmap_domain_coverage"
    );
    assert_eq!(
        GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_GAMEOBJECT_SCOPE,
        "available-fdid-direct-v1"
    );
    assert_eq!(
        GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_MODEL_SCOPE,
        "placement-and-gameobject-v1"
    );
}

#[test]
fn v47_requires_exact_31_line_grammar_two_db2_entries_and_v20_identity() {
    let lines: Vec<_> = ALL.lines().collect();
    assert_eq!(lines.len(), 31);
    assert_eq!(lines[0], "forever-product\t47");
    assert_eq!(lines[7], "partial_encrypted_sections\tdeclared-skip");
    assert_eq!(
        lines[16],
        "collision-profile\tlinux70450-gameobject-expanded-material-filtered-wmo-vmap-product-v1"
    );
    assert_eq!(
        lines[18],
        "collision-coverage\tlinux70450-gameobject-expanded-material-filtered-wmo-coverage-v20"
    );
    assert_eq!(lines[19], "gameobject-scope\tavailable-fdid-direct-v1");
    assert_eq!(lines[20], "model-scope\tplacement-and-gameobject-v1");
    assert_eq!(lines[21], "components\tdb2,vmaps");
    assert_eq!(lines[29], "db2\tGameObjectDisplayInfo\t1266277");
    assert_eq!(lines[30], "db2\tMap\t1349477");
    assert!(ALL.ends_with('\n'));
    assert_eq!(
        Plan::parse(&ALL.replace("partial_encrypted_sections\tdeclared-skip\n", "")),
        Err("product_plan_partial_encrypted_sections")
    );
    for (from, to) in [
        (
            "linux70450-gameobject-expanded-material-filtered-wmo-vmap-product-v1",
            "linux70450-assembly-expanded-material-filtered-wmo-vmap-product-v1",
        ),
        (
            "linux70450-gameobject-expanded-material-filtered-wmo-coverage-v20",
            "linux70450-assembly-expanded-material-filtered-wmo-coverage-v19",
        ),
        ("available-fdid-direct-v1", "excluded-by-profile-v1"),
        ("placement-and-gameobject-v1", "placement-only-v1"),
        ("1266277", "1266278"),
        ("1349477", "1349478"),
    ] {
        assert!(Plan::parse(&ALL.replace(from, to)).is_err(), "{to}");
    }
    assert!(Plan::parse(&ALL.replace("forever-product\t47", "forever-product\t46")).is_err());
    assert!(Plan::parse(&V46.replace("forever-product\t46", "forever-product\t47")).is_err());
    assert!(Plan::parse(&ALL.replace("forever-product\t47", "forever-product\t45")).is_err());
    assert!(Plan::parse(&V45.replace("forever-product\t45", "forever-product\t47")).is_err());
    let reversed = ALL.replace(
        "db2\tGameObjectDisplayInfo\t1266277\ndb2\tMap\t1349477",
        "db2\tMap\t1349477\ndb2\tGameObjectDisplayInfo\t1266277",
    );
    assert_ne!(reversed, ALL);
    assert_eq!(Plan::parse(&reversed), Err("product_plan_dependencies"));
}

#[test]
fn v47_binds_component_scope_and_keeps_v46_fixture_unchanged() {
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
    for mask in 0..=u8::MAX {
        if mask != Component::Db2.bit() | Component::Collision.bit() {
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
    for changed in [
        Plan {
            db2: vec![base.db2[1].clone()],
            ..base.clone()
        },
        Plan {
            db2: vec![base.db2[0].clone()],
            ..base.clone()
        },
        Plan {
            db2: vec![base.db2[1].clone(), base.db2[0].clone()],
            ..base.clone()
        },
        Plan {
            decryption: Decryption::ProvidedSalsa20V1,
            ..base.clone()
        },
        Plan {
            collision_storage: CollisionStorage::Resident,
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
        assert!(changed.validate().is_err());
    }

    let v46 = Plan::parse(V46).unwrap();
    assert_eq!(v46.version(), 46);
    assert!(v46.is_v46());
    assert!(!v46.is_v47());
    assert_eq!(v46.render(), V46);
}

#[cfg(test)]
mod fixture_tests {
    #[test]
    fn v47_fixture_is_a_separate_canonical_file() {
        let fixture =
            include_str!("../../../../tests/contracts/fixtures/forever-product-v47/all.tsv");
        assert_eq!(fixture.lines().count(), 31);
        assert!(fixture.starts_with("forever-product\t47\n"));
        assert!(
            fixture.contains("linux70450-gameobject-expanded-material-filtered-wmo-coverage-v20")
        );
        assert!(fixture.ends_with("db2\tMap\t1349477\n"));
    }
}
