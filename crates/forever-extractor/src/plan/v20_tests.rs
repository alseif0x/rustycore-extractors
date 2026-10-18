// SPDX-License-Identifier: GPL-3.0-or-later

use super::*;

const V20_MAP: &str =
    include_str!("../../../../tests/contracts/fixtures/forever-product-v20/map2.tsv");
const V20_ALL: &str =
    include_str!("../../../../tests/contracts/fixtures/forever-product-v20/available.tsv");

#[test]
fn v20_is_the_canonical_70450_compatible_available_terrain_profile() {
    let map = Plan::parse(V20_MAP).unwrap();
    let all = Plan::parse(V20_ALL).unwrap();
    assert_eq!(map.version(), 20);
    assert_eq!(map.contract(), "forever-product-v20");
    assert_eq!(
        map.publication_profile,
        PublicationProfile::Build70450CompatibleTerrainV1
    );
    assert_eq!(map.build, 70450);
    assert_eq!(map.schema_catalog, SchemaCatalog::WowDbDefs70450V1);
    assert_eq!(map.components, Component::Terrain.bit());
    assert!(map.db2.is_empty());
    assert_eq!(map.terrain, Selection::Map(2));
    assert_eq!(all.terrain, Selection::All);
    assert_eq!(map.render(), V20_MAP);
    assert_eq!(all.render(), V20_ALL);
    assert!(map.is_v20() && map.is_compatible_terrain() && map.is_available_terrain());
    assert!(!map.requires_keys());
}

#[test]
fn v20_rejects_non_target_component_masks_and_geometry_components() {
    let base = Plan::parse(V20_MAP).unwrap();
    for components in 0..=u8::MAX {
        if components != Component::Terrain.bit() {
            assert!(
                Plan {
                    components,
                    ..base.clone()
                }
                .validate()
                .is_err(),
                "mask {components}"
            );
        }
    }
    for components in [
        Component::Terrain.bit() | Component::GameTables.bit(),
        Component::Terrain.bit() | Component::Cameras.bit(),
        Component::Terrain.bit() | Component::Collision.bit(),
        Component::Terrain.bit() | Component::Navigation.bit(),
        Component::Terrain.bit() | Component::Db2.bit(),
    ] {
        assert!(
            Plan {
                components,
                ..base.clone()
            }
            .validate()
            .is_err()
        );
    }
}

#[test]
fn v20_rejects_db2_keys_selection_domain_and_offmesh_capabilities() {
    let base = Plan::parse(V20_MAP).unwrap();
    let mut with_db2 = base.clone();
    with_db2.db2.push(Entry {
        name: "Map".into(),
        id: 1_349_477,
    });
    assert_eq!(with_db2.validate(), Err("product_plan_dependencies"));
    assert_eq!(
        Plan {
            terrain: Selection::Tile { map: 2, x: 3, y: 4 },
            ..base.clone()
        }
        .validate(),
        Err("product_terrain_selection")
    );
    assert!(
        Plan {
            terrain: Selection::None,
            ..base.clone()
        }
        .validate()
        .is_err()
    );
    assert!(
        Plan {
            navigation: Selection::Map(2),
            ..base.clone()
        }
        .validate()
        .is_err()
    );
    assert!(
        Plan {
            map_domain: Some(2),
            ..base.clone()
        }
        .validate()
        .is_err()
    );
    assert!(
        Plan {
            offmesh: Some(vec![]),
            ..base.clone()
        }
        .validate()
        .is_err()
    );
    assert!(
        Plan {
            decryption: Decryption::ProvidedSalsa20V1,
            ..base.clone()
        }
        .validate()
        .is_err()
    );
    let leased = DiskBudget {
        raw_bytes: 32,
        wdt_bytes: 32,
        cache_bytes: 32,
        placement_bytes: 32,
        vmo_bytes: 32,
        map_bytes: 32,
        output_bytes: 1,
    };
    assert!(
        Plan {
            collision_storage: CollisionStorage::LeasedDiskV1(leased),
            ..base.clone()
        }
        .validate()
        .is_err()
    );
}

#[test]
fn v20_rejects_foreign_build_catalogue_profile_and_noncanonical_selection() {
    let base = Plan::parse(V20_MAP).unwrap();
    for build in [0, 70_374, 70_425, 70_451, u32::MAX] {
        assert!(
            Plan {
                build,
                ..base.clone()
            }
            .validate()
            .is_err()
        );
    }
    assert!(
        Plan {
            schema_catalog: SchemaCatalog::WowDbDefs70425V1,
            ..base.clone()
        }
        .validate()
        .is_err()
    );
    assert!(
        Plan {
            publication_profile: PublicationProfile::Legacy,
            ..base.clone()
        }
        .validate()
        .is_err()
    );
    for invalid in [
        "all",
        "none",
        "tile:2:3:4",
        "map:02",
        "all-available-maps\textra",
    ] {
        let text = V20_MAP.replace("terrain\tmap:2", &format!("terrain\t{invalid}"));
        assert!(Plan::parse(&text).is_err(), "{invalid}");
    }
    assert!(Plan::parse(&V20_MAP.replace("wowdbdefs-70450-v1", "wowdbdefs-70425-v1")).is_err());
}

#[test]
fn v20_rejects_supplied_keys_before_any_client_path_access() {
    #[cfg(target_os = "linux")]
    {
        let plan = Plan::parse(V20_MAP).unwrap();
        let keyring = forever_casc::keys::Keyring::parse(
            b"forever-tact-keys\t1\n0000000000000001\t00000000000000000000000000000000\n",
        )
        .unwrap();
        let error = crate::driver::open_with_keys(
            &plan,
            std::path::Path::new("/authored/absent/client-v20"),
            keyring,
        )
        .err()
        .unwrap();
        assert_eq!(error.code, "product_keys_forbidden");
    }
    #[cfg(not(target_os = "linux"))]
    assert_eq!(
        crate::driver::admit(&Plan::parse(V20_MAP).unwrap())
            .err()
            .unwrap()
            .code,
        "product_platform"
    );
}
