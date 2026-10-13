// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-12: canonical v3 collision storage selection and limits.
use super::*;

const V1: &str = concat!(
    "forever-product\t1\nbuild\t70205\nlocale\tesES\nindex\tcontiguous\n",
    "root\theader-50893-v2\ncomponents\tvmaps\nterrain\tnone\npack\ttrue\n",
    "navigation\tnone\nbig-base-unit\tfalse\nskip-liquids\tfalse\n",
    "maximum-slope\t55\nnon-steep-slope\t55\n",
);
const FIELDS: [&str; 7] = [
    "collision-raw-bytes",
    "collision-wdt-bytes",
    "collision-cache-bytes",
    "collision-placement-bytes",
    "collision-vmo-bytes",
    "collision-map-bytes",
    "collision-output-bytes",
];
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
fn disk() -> Plan {
    Plan {
        collision_storage: CollisionStorage::LeasedDiskV1(budget()),
        ..Plan::parse(V1).unwrap()
    }
}
fn set_budget(budget: &mut DiskBudget, field: usize, value: u64) {
    *match field {
        0 => &mut budget.raw_bytes,
        1 => &mut budget.wdt_bytes,
        2 => &mut budget.cache_bytes,
        3 => &mut budget.placement_bytes,
        4 => &mut budget.vmo_bytes,
        5 => &mut budget.map_bytes,
        6 => &mut budget.output_bytes,
        _ => unreachable!(),
    } = value;
}
fn change_field(source: &str, name: &str, value: &str) -> String {
    source
        .lines()
        .map(|line| {
            if line.split_once('\t').is_some_and(|(key, _)| key == name) {
                format!("{name}\t{value}\n")
            } else {
                format!("{line}\n")
            }
        })
        .collect()
}

#[test]
fn resident_is_default_and_preserves_v1_bytes() {
    assert_eq!(CollisionStorage::default(), CollisionStorage::Resident);
    let plan = Plan::parse(V1).unwrap();
    assert_eq!(plan.collision_storage, CollisionStorage::Resident);
    assert_eq!(plan.render(), V1);
    assert_eq!(plan.version(), 1);
}
#[test]
fn v3_disabled_roundtrip_has_exact_storage_position() {
    let plan = disk();
    let expected = concat!(
        "forever-product\t3\nbuild\t70205\nlocale\tesES\nindex\tcontiguous\n",
        "root\theader-50893-v2\ndecryption\tdisabled\n",
        "collision-storage\tlinux-leased-append-disk-world-v1\n",
        "collision-raw-bytes\t101\ncollision-wdt-bytes\t102\n",
        "collision-cache-bytes\t103\ncollision-placement-bytes\t104\n",
        "collision-vmo-bytes\t105\ncollision-map-bytes\t106\n",
        "collision-output-bytes\t107\ncomponents\tvmaps\nterrain\tnone\npack\ttrue\n",
        "navigation\tnone\nbig-base-unit\tfalse\nskip-liquids\tfalse\n",
        "maximum-slope\t55\nnon-steep-slope\t55\n",
    );
    assert_eq!(plan.validate(), Ok(()));
    assert_eq!(plan.version(), 3);
    assert_eq!(plan.contract(), "forever-product-v3");
    assert!(!plan.requires_keys());
    assert_eq!(plan.render(), expected);
    assert_eq!(Plan::parse(expected), Ok(plan));
}
#[test]
fn v3_supplied_keys_are_independent_of_storage() {
    let mut plan = disk();
    plan.decryption = Decryption::ProvidedSalsa20V1;
    assert!(plan.requires_keys());
    assert_eq!(plan.version(), 3);
    assert_eq!(Plan::parse(&plan.render()), Ok(plan.clone()));
    assert!(
        plan.render()
            .contains("decryption\tprovided-salsa20-v1\ncollision-storage\t")
    );
    plan.collision_storage = CollisionStorage::Resident;
    assert_eq!(plan.version(), 2);
    assert_eq!(plan.contract(), "forever-product-v2");
    assert!(!plan.render().contains("collision-"));
    assert_eq!(Plan::parse(&plan.render()), Ok(plan));
}
#[test]
fn switching_storage_restores_exact_resident_bytes() {
    let mut plan = disk();
    plan.collision_storage = CollisionStorage::Resident;
    assert_eq!(plan.render(), V1);
}
#[test]
fn all_seven_limits_admit_exact_bounds() {
    for field in 0..7 {
        let (min, max) = if field == 6 {
            (1, DiskBudget::MAX_OUTPUT_BYTES)
        } else {
            (DiskBudget::MIN_RECORD_BYTES, DiskBudget::MAX_RECORD_BYTES)
        };
        for value in [min, max] {
            let mut b = budget();
            set_budget(&mut b, field, value);
            let plan = Plan {
                collision_storage: CollisionStorage::LeasedDiskV1(b),
                ..disk()
            };
            assert_eq!(plan.validate(), Ok(()));
            assert_eq!(Plan::parse(&plan.render()), Ok(plan));
        }
    }
}
#[test]
fn all_seven_limits_reject_outside_bounds_before_io() {
    for field in 0..7 {
        let (min, max) = if field == 6 {
            (1, DiskBudget::MAX_OUTPUT_BYTES)
        } else {
            (DiskBudget::MIN_RECORD_BYTES, DiskBudget::MAX_RECORD_BYTES)
        };
        for value in [0, min - 1, max + 1, u64::MAX] {
            let mut b = budget();
            set_budget(&mut b, field, value);
            assert_eq!(b.validate(), Err("product_plan_disk_budget"));
            let plan = Plan {
                collision_storage: CollisionStorage::LeasedDiskV1(b),
                ..disk()
            };
            assert_eq!(plan.validate(), Err("product_plan_disk_budget"));
            assert!(Plan::parse(&plan.render()).is_err());
        }
    }
}
#[test]
fn budgets_are_u64_canonical_decimals_not_u32() {
    let mut b = budget();
    for field in 0..7 {
        set_budget(&mut b, field, u64::from(u32::MAX) + 1);
    }
    let plan = Plan {
        collision_storage: CollisionStorage::LeasedDiskV1(b),
        ..disk()
    };
    assert_eq!(Plan::parse(&plan.render()), Ok(plan));
}
#[test]
fn rejects_noncanonical_budget_decimals_and_overflow() {
    let source = disk().render();
    for field in FIELDS {
        for value in [
            "",
            "0101",
            "+101",
            "-101",
            "1.0",
            "1e2",
            "101 ",
            " 101",
            "18446744073709551616",
            "999999999999999999999999999999999999",
        ] {
            assert!(
                Plan::parse(&change_field(&source, field, value)).is_err(),
                "{field}: {value}"
            );
        }
    }
}
#[test]
fn disk_requires_collision_component_in_parsed_and_manual_plans() {
    let mut plan = disk();
    plan.components = Component::GameTables.bit();
    assert_eq!(plan.validate(), Err("product_plan_dependencies"));
    assert_eq!(
        Plan::parse(&plan.render()),
        Err("product_plan_dependencies")
    );
}
#[test]
fn v3_rejects_missing_reordered_and_duplicate_budget_lines() {
    let source = disk().render();
    for (i, field) in FIELDS.iter().enumerate() {
        let line = format!("{field}\t{}\n", 101 + i);
        for changed in [
            source.replace(&line, ""),
            source.replace(&line, &format!("{line}{line}")),
            source
                .replace(&line, "")
                .replace("terrain\t", &format!("{line}terrain\t")),
        ] {
            assert!(Plan::parse(&changed).is_err(), "{field}");
        }
    }
    assert!(
        Plan::parse(&source.replace(
            "collision-raw-bytes\t101\ncollision-wdt-bytes\t102",
            "collision-wdt-bytes\t102\ncollision-raw-bytes\t101"
        ))
        .is_err()
    );
}
#[test]
fn v3_rejects_storage_aliases_and_resident_spelling() {
    let source = disk().render();
    for value in [
        "resident",
        "file-set-v1",
        "linux-private-collision-leased-append-v1",
        "linux-leased-append-disk-world-v2",
        "Linux-leased-append-disk-world-v1",
        "",
    ] {
        assert!(Plan::parse(&change_field(&source, "collision-storage", value)).is_err());
    }
}
#[test]
fn v3_requires_explicit_decryption_and_storage() {
    let source = disk().render();
    for changed in [
        source.replace("decryption\tdisabled\n", ""),
        source.replace("collision-storage\tlinux-leased-append-disk-world-v1\n", ""),
        source.replace(
            "decryption\tdisabled\n",
            "decryption\tdisabled\ndecryption\tdisabled\n",
        ),
        source
            .replace("decryption\tdisabled\n", "")
            .replace("components\t", "decryption\tdisabled\ncomponents\t"),
    ] {
        assert!(Plan::parse(&changed).is_err());
    }
    for value in ["none", "salsa20", "Disabled", "provided-salsa20-v2"] {
        assert!(Plan::parse(&change_field(&source, "decryption", value)).is_err());
    }
}
#[test]
fn v1_and_v2_reject_disk_fields_and_v3_cannot_be_implicit() {
    let source = disk().render();
    for version in ["1", "2"] {
        assert!(Plan::parse(&change_field(&source, "forever-product", version)).is_err());
    }
    assert!(Plan::parse(&change_field(V1, "forever-product", "3")).is_err());
    for key in FIELDS {
        assert!(Plan::parse(&format!("{V1}{key}\t101\n")).is_err());
    }
}
#[test]
fn v3_retains_strict_ascii_whitespace_and_eof() {
    let source = disk().render();
    for changed in [
        source.trim_end().into(),
        format!("{source}\n"),
        source.replace('\n', "\r\n"),
        source.replace("disabled", "disá bled"),
        source.replace("collision-output-bytes", "collision-output-bytes\0"),
        source.replace("collision-map-bytes\t106", "collision-map-bytes\t106\t"),
        format!("{source}unknown\t1\n"),
    ] {
        assert!(Plan::parse(&changed).is_err());
    }
}
#[test]
fn v3_rejects_paths_key_material_and_identity_fields() {
    let source = disk().render();
    for line in [
        "collision-path\t/private/stage",
        "collision-raw-path\t/private/raw",
        "key-path\t/private/key",
        "key-id\t1",
        "key-sha256\tabcd",
    ] {
        assert!(Plan::parse(&format!("{source}{line}\n")).is_err());
        assert!(
            Plan::parse(&source.replace("components\t", &format!("{line}\ncomponents\t"))).is_err()
        );
    }
}
#[test]
fn v3_all_components_layouts_and_key_capabilities_roundtrip() {
    for build in [70170, 70205] {
        for locale in [Locale::EnUs, Locale::EsEs] {
            for index_layout in [local_index::Layout::Contiguous, local_index::Layout::Paged] {
                for root_layout in [
                    root::Layout::Headerless18125,
                    root::Layout::Header30080,
                    root::Layout::Header50893V1,
                    root::Layout::Header50893V2,
                ] {
                    for decryption in [Decryption::Disabled, Decryption::ProvidedSalsa20V1] {
                        let plan = Plan {
                            build,
                            schema_catalog: SchemaCatalog::PinnedMetadataV1,
                            locale,
                            index_layout,
                            root_layout,
                            decryption,
                            components: 63,
                            db2: ["CinematicCamera", "LiquidType", "Map"]
                                .into_iter()
                                .map(|name| Entry {
                                    name: name.into(),
                                    id: catalog::find(name).unwrap().file_data_id,
                                })
                                .collect(),
                            terrain: Selection::All,
                            navigation: Selection::All,
                            ..disk()
                        };
                        assert_eq!(Plan::parse(&plan.render()), Ok(plan));
                    }
                }
            }
        }
    }
}
#[test]
fn v3_retains_selection_dependency_and_size_limits() {
    let source = disk().render();
    for changed in [
        source.replace("components\tvmaps", "components\tvmaps,mmaps"),
        source.replace("terrain\tnone", "terrain\tall"),
        source.replace("navigation\tnone", "navigation\tmap:0"),
        source.replace("maximum-slope\t55", "maximum-slope\t91"),
    ] {
        assert!(Plan::parse(&changed).is_err());
    }
    assert_eq!(
        Plan::parse(&"x".repeat(MAX_PLAN_BYTES + 1)),
        Err("product_plan_size")
    );
}
