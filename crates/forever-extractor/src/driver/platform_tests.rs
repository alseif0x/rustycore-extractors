// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-13: closed platform/plan admission without filesystem I/O.
use super::*;
use crate::plan::{Decryption, Entry, SchemaCatalog};
use forever_casc::Locale;

fn resident() -> Plan {
    Plan {
        publication_profile: crate::plan::PublicationProfile::Legacy,
        build: 70205,
        locale: Locale::EnUs,
        index_layout: forever_casc::local_index::Layout::Contiguous,
        root_layout: root::Layout::Header50893V2,
        schema_catalog: SchemaCatalog::PinnedMetadataV1,
        decryption: Decryption::Disabled,
        collision_storage: CollisionStorage::Resident,
        components: Component::ALL.iter().map(|component| component.bit()).sum(),
        db2: ["CinematicCamera", "LiquidType", "Map"]
            .map(|name| Entry {
                name: name.into(),
                id: forever_db2::catalog::find(name).unwrap().file_data_id,
            })
            .to_vec(),
        terrain: Selection::All,
        navigation: Selection::All,
        pack: true,
        big_base_unit: false,
        skip_liquids: false,
        maximum_slope: 55,
        non_steep_slope: 55,
        offmesh: None,
        map_domain: None,
    }
}

#[test]
fn all_six_resident_producers_admit_both_legacy_builds_and_locales() {
    for build in [70170, 70205] {
        for locale in [Locale::EnUs, Locale::EsEs] {
            let plan = Plan {
                build,
                locale,
                ..resident()
            };
            assert_eq!(admit(&plan), Ok(()));
            assert_eq!(Plan::parse(&plan.render()), Ok(plan));
        }
    }
}

#[test]
fn provided_keys_select_the_same_complete_resident_producer_set() {
    let plan = Plan {
        decryption: Decryption::ProvidedSalsa20V1,
        ..resident()
    };
    assert_eq!(plan.version(), 2);
    assert_eq!(admit(&plan), Ok(()));
    assert_eq!(validate_keys(&plan, true), Ok(()));
    assert_eq!(
        validate_keys(&plan, false).unwrap_err().code,
        "product_keys_required"
    );
    assert!(
        Component::ALL
            .into_iter()
            .all(|component| plan.contains(component))
    );
    assert_eq!(Plan::parse(&plan.render()), Ok(plan));
}

#[test]
fn v4_keeps_all_six_resident_components_with_exact_target_catalog() {
    let plan = Plan {
        schema_catalog: SchemaCatalog::WowDbDefs70205V1,
        ..resident()
    };
    assert_eq!(plan.version(), 4);
    assert_eq!(admit(&plan), Ok(()));
    assert_eq!(Plan::parse(&plan.render()), Ok(plan));
}

#[test]
fn v6_keeps_all_six_resident_components_and_explicit_empty_offmesh() {
    for decryption in [Decryption::Disabled, Decryption::ProvidedSalsa20V1] {
        let plan = Plan {
            decryption,
            offmesh: Some(vec![]),
            ..resident()
        };
        assert_eq!(plan.version(), 6);
        assert_eq!(admit(&plan), Ok(()));
        assert_eq!(Plan::parse(&plan.render()), Ok(plan));
    }
}

#[test]
fn build_70338_does_not_gain_geometry_through_platform_dispatch() {
    let invalid = Plan {
        build: 70338,
        schema_catalog: SchemaCatalog::WowDbDefs70338V1,
        ..resident()
    };
    assert_eq!(admit(&invalid), invalid.validate().map_err(Error::from));
    assert!(admit(&invalid).is_err());
    let plan = Plan {
        components: Component::Db2.bit(),
        terrain: Selection::None,
        navigation: Selection::None,
        db2: vec![Entry {
            name: "Map".into(),
            id: forever_db2::catalog::find("Map").unwrap().file_data_id,
        }],
        ..invalid
    };
    assert_eq!(admit(&plan), Ok(()));
    assert_eq!(Plan::parse(&plan.render()), Ok(plan));
}

#[test]
fn malformed_plan_rejects_before_platform_or_input_admission() {
    let plan = Plan {
        build: 0,
        ..resident()
    };
    let path = Path::new("nonexistent-authored-product-input");
    let expected = Error::from(plan.validate().unwrap_err());
    assert_eq!(admit(&plan), Err(expected));
    assert_eq!(inspect(&plan, path, [0; 32], None), Err(expected));
    assert_eq!(extract(&plan, path, path, [0; 32], false), Err(expected));
    assert_eq!(crate::verify::verify(&plan, path, None), Err(expected));
}

#[cfg(windows)]
#[test]
fn windows_disk_plan_rejects_all_execution_entries_before_io() {
    let mut plan = resident();
    plan.collision_storage = CollisionStorage::LeasedDiskV1(crate::plan::DiskBudget {
        raw_bytes: 1024,
        wdt_bytes: 1024,
        cache_bytes: 1024,
        placement_bytes: 1024,
        vmo_bytes: 1024,
        map_bytes: 1024,
        output_bytes: 1024,
    });
    let path = Path::new("nonexistent-authored-product-disk-input");
    assert_eq!(admit(&plan).unwrap_err().code, "product_disk_unsupported");
    assert_eq!(
        open(&plan, path).err().unwrap().code,
        "product_disk_unsupported"
    );
    assert_eq!(
        inspect(&plan, path, [0; 32], None).unwrap_err().code,
        "product_disk_unsupported"
    );
    assert_eq!(
        extract(&plan, path, path, [0; 32], false).unwrap_err().code,
        "product_disk_unsupported"
    );
    assert_eq!(
        extract_disk(&plan, path, path, [0; 32], true, None, path)
            .unwrap_err()
            .code,
        "product_disk_unsupported"
    );
    assert_eq!(
        crate::verify::verify(&plan, path, None).unwrap_err().code,
        "product_disk_unsupported"
    );
}
