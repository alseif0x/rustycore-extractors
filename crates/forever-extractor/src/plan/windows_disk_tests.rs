// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-13: explicit v7 Windows disk plan admission and hash binding.
use super::*;

fn budget() -> DiskBudget {
    DiskBudget {
        raw_bytes: 101,
        wdt_bytes: 102,
        cache_bytes: 103,
        placement_bytes: 104,
        vmo_bytes: 105,
        map_bytes: 106,
        output_bytes: 107,
    }
}
fn plan() -> Plan {
    Plan {
        build: 70205,
        schema_catalog: SchemaCatalog::PinnedMetadataV1,
        locale: Locale::EnUs,
        index_layout: local_index::Layout::Contiguous,
        root_layout: root::Layout::Header50893V2,
        decryption: Decryption::Disabled,
        collision_storage: CollisionStorage::WindowsDiskV1(budget()),
        components: 63,
        db2: ["CinematicCamera", "LiquidType", "Map"]
            .map(|name| Entry {
                name: name.into(),
                id: catalog::find(name).unwrap().file_data_id,
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
    }
}
fn field(source: &str, key: &str, value: &str) -> String {
    source
        .lines()
        .map(|line| match line.split_once('\t') {
            Some((name, _)) if name == key => format!("{key}\t{value}\n"),
            _ => format!("{line}\n"),
        })
        .collect()
}
fn set_budget(b: &mut DiskBudget, index: usize, value: u64) {
    *match index {
        0 => &mut b.raw_bytes,
        1 => &mut b.wdt_bytes,
        2 => &mut b.cache_bytes,
        3 => &mut b.placement_bytes,
        4 => &mut b.vmo_bytes,
        5 => &mut b.map_bytes,
        6 => &mut b.output_bytes,
        _ => unreachable!(),
    } = value;
}

#[test]
fn v7_explicit_profile_roundtrips_all_six_components_and_decryption_modes() {
    for build in [70170, 70205] {
        for locale in [Locale::EnUs, Locale::EsEs] {
            for decryption in [Decryption::Disabled, Decryption::ProvidedSalsa20V1] {
                let p = Plan {
                    build,
                    locale,
                    decryption,
                    ..plan()
                };
                assert_eq!(p.validate(), Ok(()));
                assert_eq!(p.version(), 7);
                assert_eq!(p.contract(), "forever-product-v7");
                assert!(
                    p.render()
                        .contains("collision-storage\twindows-exclusive-append-disk-world-v1\n")
                );
                assert_eq!(p.collision_storage.disk_budget(), Some(budget()));
                assert_eq!(Plan::parse(&p.render()), Ok(p));
            }
        }
    }
}

#[test]
fn v1_through_v6_never_accept_the_windows_storage_literal() {
    let source = plan().render();
    for version in 1..=6 {
        assert!(Plan::parse(&field(&source, "forever-product", &version.to_string())).is_err());
    }
    let linux = Plan {
        collision_storage: CollisionStorage::LeasedDiskV1(budget()),
        ..plan()
    };
    for version in [3, 4, 6] {
        let mut old = linux.clone();
        if version == 4 {
            old.schema_catalog = SchemaCatalog::WowDbDefs70205V1;
        } else if version == 6 {
            old.offmesh = Some(vec![]);
        }
        let source = old.render();
        assert_eq!(old.version(), version);
        assert_eq!(Plan::parse(&source), Ok(old));
        assert_eq!(
            Plan::parse(&source.replace(
                "linux-leased-append-disk-world-v1",
                "windows-exclusive-append-disk-world-v1"
            )),
            Err("product_plan_collision_storage")
        );
    }
}

#[test]
fn v7_requires_windows_storage_without_alias_or_resident_fallback() {
    let source = plan().render();
    for storage in [
        "linux-leased-append-disk-world-v1",
        "windows-exclusive-append-disk-world-v2",
        "windows-private-append-disk-world-v1",
        "resident",
    ] {
        assert_eq!(
            Plan::parse(&field(&source, "collision-storage", storage)),
            Err("product_plan_collision_storage")
        );
    }
    assert!(Plan::parse(&field(&source, "collision-storage", "")).is_err());
    let no_storage = source
        .lines()
        .filter(|line| !line.starts_with("collision-"))
        .map(|line| format!("{line}\n"))
        .collect::<String>();
    assert!(Plan::parse(&no_storage).is_err());
    assert!(Plan::parse(&source.replace("decryption\tdisabled\n", "")).is_err());
}

#[test]
fn v7_preserves_70205_catalog_identity_and_has_priority_over_v4() {
    let p = Plan {
        schema_catalog: SchemaCatalog::WowDbDefs70205V1,
        ..plan()
    };
    assert_eq!(p.version(), 7);
    assert_eq!(p.contract(), "forever-product-v7");
    assert_eq!(Plan::parse(&p.render()), Ok(p.clone()));
    let mut wrong_id = p.clone();
    wrong_id.db2[0].id ^= 1;
    assert_eq!(wrong_id.validate(), Err("product_plan_schema"));
    let wrong_build = Plan { build: 70170, ..p };
    assert_eq!(wrong_build.validate(), Err("product_plan_schema_catalog"));
}

#[test]
fn v7_optional_offmesh_retains_v6_scope_and_precedence() {
    let p = Plan {
        offmesh: Some(vec![]),
        ..plan()
    };
    assert_eq!(p.version(), 7);
    assert_eq!(p.contract(), "forever-product-v7");
    assert_eq!(Plan::parse(&p.render()), Ok(p.clone()));
    let catalog = Plan {
        schema_catalog: SchemaCatalog::WowDbDefs70205V1,
        ..p
    };
    assert_eq!(catalog.validate(), Err("product_plan_offmesh_scope"));
    let mut p = plan();
    p.offmesh = Some(vec![]);
    p.components &= !Component::Navigation.bit();
    p.navigation = Selection::None;
    assert_eq!(p.validate(), Err("product_plan_offmesh_scope"));
}

#[test]
fn v7_nonempty_offmesh_preserves_bits_duplicates_order_and_v6_limits() {
    let connection = super::offmesh_tests::connection();
    let second = OffMeshConnection {
        map_id: 7,
        direction: 1,
        ..connection
    };
    let mut p = Plan {
        offmesh: Some(vec![connection, second, connection]),
        ..plan()
    };
    assert_eq!(Plan::parse(&p.render()), Ok(p.clone()));
    assert_eq!(
        p.offmesh.as_ref().unwrap()[0].source().from[0].to_bits(),
        0x8000_0000
    );
    p.offmesh = Some(vec![connection; MAX_OFFMESH_CONNECTIONS]);
    assert_eq!(Plan::parse(&p.render()), Ok(p.clone()));
    p.offmesh.as_mut().unwrap().push(connection);
    assert_eq!(p.validate(), Err("product_plan_offmesh_limit"));
    for invalid in [
        OffMeshConnection {
            row: 64,
            ..connection
        },
        OffMeshConnection {
            radius_bits: 0,
            ..connection
        },
        OffMeshConnection {
            direction: 2,
            ..connection
        },
        OffMeshConnection {
            from_bits: [f32::INFINITY.to_bits(); 3],
            ..connection
        },
    ] {
        p.offmesh = Some(vec![invalid]);
        assert_eq!(p.validate(), Err("product_plan_offmesh"));
        assert!(Plan::parse(&p.render()).is_err());
    }
}

#[test]
fn v7_rejects_70338_and_requires_collision_in_manually_constructed_plans() {
    let p = Plan {
        build: 70338,
        schema_catalog: SchemaCatalog::WowDbDefs70338V1,
        ..plan()
    };
    assert_eq!(p.validate(), Err("product_plan_disk_scope"));
    assert!(Plan::parse(&p.render()).is_err());
    let p = Plan {
        components: Component::Db2.bit(),
        terrain: Selection::None,
        navigation: Selection::None,
        ..plan()
    };
    assert_eq!(p.validate(), Err("product_plan_dependencies"));
}

#[test]
fn v7_all_seven_downward_budgets_have_the_existing_exact_bounds() {
    for index in 0..7 {
        let (min, max) = if index == 6 {
            (1, DiskBudget::MAX_OUTPUT_BYTES)
        } else {
            (32, DiskBudget::MAX_RECORD_BYTES)
        };
        for value in [min, max] {
            let mut b = budget();
            set_budget(&mut b, index, value);
            let p = Plan {
                collision_storage: CollisionStorage::WindowsDiskV1(b),
                ..plan()
            };
            assert_eq!(Plan::parse(&p.render()), Ok(p));
        }
        for value in [0, min - 1, max + 1, u64::MAX] {
            let mut b = budget();
            set_budget(&mut b, index, value);
            let p = Plan {
                collision_storage: CollisionStorage::WindowsDiskV1(b),
                ..plan()
            };
            assert_eq!(p.validate(), Err("product_plan_disk_budget"));
            assert!(Plan::parse(&p.render()).is_err());
        }
    }
}

#[test]
fn v7_storage_and_each_budget_change_the_canonical_plan_hash() {
    let p = plan();
    let hash = forever_casc::integrity::digest(p.render().as_bytes());
    let linux = Plan {
        collision_storage: CollisionStorage::LeasedDiskV1(budget()),
        ..p.clone()
    };
    assert_ne!(
        forever_casc::integrity::digest(linux.render().as_bytes()),
        hash
    );
    for index in 0..7 {
        let mut b = budget();
        set_budget(&mut b, index, 256);
        let changed = Plan {
            collision_storage: CollisionStorage::WindowsDiskV1(b),
            ..p.clone()
        };
        assert_ne!(
            forever_casc::integrity::digest(changed.render().as_bytes()),
            hash
        );
    }
}

#[test]
fn v7_rejects_unknown_fields_key_material_noncanonical_whitespace_and_eof() {
    let source = plan().render();
    for changed in [
        source.trim_end().into(),
        format!("{source}\n"),
        source.replace('\n', "\r\n"),
        format!("{source}key-path\tC:/keys\n"),
        format!("{source}scratch\tC:/private\n"),
        field(&source, "collision-raw-bytes", "0101"),
    ] {
        assert!(Plan::parse(&changed).is_err());
    }
}

#[test]
fn switching_back_to_old_storage_restores_old_versions_without_windows_fields() {
    let mut p = plan();
    p.collision_storage = CollisionStorage::LeasedDiskV1(budget());
    assert_eq!(p.version(), 3);
    assert_eq!(Plan::parse(&p.render()), Ok(p.clone()));
    p.collision_storage = CollisionStorage::Resident;
    assert_eq!(p.version(), 1);
    assert_eq!(Plan::parse(&p.render()), Ok(p.clone()));
    p.offmesh = Some(vec![]);
    assert_eq!(p.version(), 6);
    assert_eq!(Plan::parse(&p.render()), Ok(p));
}
