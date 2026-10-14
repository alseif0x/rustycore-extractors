// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: synthetic product plan grammar and dependency checks.
use super::*;

fn entry(name: &str) -> Entry {
    Entry {
        name: name.into(),
        id: catalog::find(name).unwrap().file_data_id,
    }
}
fn build_70338_entry(name: &str) -> Entry {
    Entry {
        name: name.into(),
        id: catalog::resolve_for(catalog::WOWDBDEFS_70338_CATALOG_ID, name)
            .unwrap()
            .unwrap()
            .metadata()
            .file_data_id,
    }
}
fn subset() -> Plan {
    Plan {
        publication_profile: crate::plan::PublicationProfile::Legacy,
        build: 70205,
        schema_catalog: crate::plan::SchemaCatalog::PinnedMetadataV1,
        locale: Locale::EsEs,
        index_layout: local_index::Layout::Contiguous,
        root_layout: root::Layout::Header50893V2,
        decryption: Decryption::Disabled,
        collision_storage: CollisionStorage::Resident,
        map_domain: None,
        components: Component::Db2.bit(),
        db2: vec![entry("ChrClasses")],
        terrain: Selection::None,
        navigation: Selection::None,
        pack: true,
        big_base_unit: false,
        skip_liquids: false,
        maximum_slope: 55,
        non_steep_slope: 55,
        offmesh: None,
    }
}
fn complete() -> Plan {
    Plan {
        components: 63,
        db2: vec![entry("CinematicCamera"), entry("LiquidType"), entry("Map")],
        terrain: Selection::All,
        navigation: Selection::All,
        maximum_slope: 70,
        non_steep_slope: 50,
        ..subset()
    }
}

#[test]
fn canonical_subset_roundtrip() {
    let plan = subset();
    assert_eq!(plan.validate(), Ok(()));
    assert_eq!(Plan::parse(&plan.render()), Ok(plan.clone()));
    assert!(plan.contains(Component::Db2));
    assert!(!plan.contains(Component::Navigation));
    let expected = concat!(
        "forever-product\t1\nbuild\t70205\nlocale\tesES\nindex\tcontiguous\n",
        "root\theader-50893-v2\ncomponents\tdb2\nterrain\tnone\npack\ttrue\n",
        "navigation\tnone\nbig-base-unit\tfalse\nskip-liquids\tfalse\n",
        "maximum-slope\t55\nnon-steep-slope\t55\n",
    );
    assert!(plan.render().starts_with(expected));
}
#[test]
fn canonical_all_components_roundtrip() {
    let plan = complete();
    assert_eq!(Plan::parse(&plan.render()), Ok(plan.clone()));
    assert!(
        plan.render()
            .contains("components\tdb2,gt,cameras,maps,vmaps,mmaps\n")
    );
    assert!(
        Component::ALL
            .into_iter()
            .all(|component| plan.contains(component))
    );
}

#[test]
fn build_70338_catalog_has_a_separate_canonical_v5_plan() {
    let plan = Plan {
        build: 70338,
        schema_catalog: SchemaCatalog::WowDbDefs70338V1,
        db2: vec![
            build_70338_entry("TactKey"),
            build_70338_entry("TactKeyLookup"),
        ],
        ..subset()
    };
    assert_eq!(plan.validate(), Ok(()));
    assert_eq!(plan.version(), 5);
    assert_eq!(plan.contract(), "forever-product-v5");
    assert!(!plan.requires_keys());
    assert_eq!(Plan::parse(&plan.render()), Ok(plan.clone()));
    assert!(
        plan.render()
            .contains("schema-catalog\twowdbdefs-70338-v1\n")
    );
    assert!(
        plan.render()
            .starts_with("forever-product\t5\nbuild\t70338\n")
    );
}

#[test]
fn build_70338_cannot_use_the_older_build_metadata_catalog() {
    let plan = Plan {
        build: 70338,
        ..subset()
    };
    assert_eq!(plan.validate(), Err("product_plan_schema_catalog"));

    let plan = Plan {
        build: 70205,
        schema_catalog: SchemaCatalog::WowDbDefs70338V1,
        ..subset()
    };
    assert_eq!(plan.validate(), Err("product_plan_schema_catalog"));
}

#[test]
fn build_70338_metadata_plan_is_limited_to_db2_until_other_components_are_accepted() {
    let plan = Plan {
        build: 70338,
        schema_catalog: SchemaCatalog::WowDbDefs70338V1,
        db2: vec![build_70338_entry("TactKey")],
        components: Component::Db2.bit() | Component::Terrain.bit(),
        ..subset()
    };
    assert_eq!(plan.validate(), Err("product_plan_component_scope"));
}
#[test]
fn all_layouts_builds_and_locales_roundtrip() {
    for build in [70170, 70205] {
        for locale in [Locale::EnUs, Locale::EsEs] {
            for index_layout in [local_index::Layout::Contiguous, local_index::Layout::Paged] {
                for root_layout in [
                    root::Layout::Headerless18125,
                    root::Layout::Header30080,
                    root::Layout::Header50893V1,
                    root::Layout::Header50893V2,
                ] {
                    let plan = Plan {
                        build,
                        schema_catalog: SchemaCatalog::PinnedMetadataV1,
                        locale,
                        index_layout,
                        root_layout,
                        ..subset()
                    };
                    assert_eq!(Plan::parse(&plan.render()), Ok(plan));
                }
            }
        }
    }
}
#[test]
fn source_free_components_allow_no_db2() {
    for component in [Component::GameTables, Component::Collision] {
        let plan = Plan {
            components: component.bit(),
            db2: vec![],
            ..subset()
        };
        assert_eq!(Plan::parse(&plan.render()), Ok(plan));
    }
}
#[test]
fn terrain_selection_coordinates_and_zero_map() {
    for terrain in [
        Selection::All,
        Selection::Map(0),
        Selection::Map(u32::MAX),
        Selection::Tile {
            map: 42,
            x: 0,
            y: 63,
        },
    ] {
        let plan = Plan {
            components: Component::Terrain.bit(),
            db2: vec![],
            terrain,
            ..subset()
        };
        assert_eq!(Plan::parse(&plan.render()), Ok(plan));
    }
}
#[test]
fn navigation_selections_and_options() {
    for navigation in [
        Selection::All,
        Selection::Map(0),
        Selection::Tile {
            map: 42,
            x: 63,
            y: 0,
        },
    ] {
        let plan = Plan {
            navigation,
            big_base_unit: true,
            skip_liquids: true,
            ..complete()
        };
        assert_eq!(Plan::parse(&plan.render()), Ok(plan));
    }
}
#[test]
fn navigation_slope_boundaries() {
    for (maximum_slope, non_steep_slope) in [(0, 0), (90, 90), (90, 0), (55, 55)] {
        let plan = Plan {
            maximum_slope,
            non_steep_slope,
            ..complete()
        };
        assert_eq!(Plan::parse(&plan.render()), Ok(plan));
    }
}
#[test]
fn rejects_unknown_version_build_locale_and_layout() {
    let source = subset().render();
    for (from, to) in [
        ("forever-product\t1", "forever-product\t2"),
        ("build\t70205", "build\t51943"),
        ("locale\tesES", "locale\tenGB"),
        ("index\tcontiguous", "index\tauto"),
        ("root\theader-50893-v2", "root\tauto"),
    ] {
        assert!(Plan::parse(&source.replace(from, to)).is_err(), "{to}");
    }
}
#[test]
fn rejects_extra_missing_reordered_or_duplicate_lines() {
    let source = subset().render();
    for changed in [
        format!("{source}unknown\tvalue\n"),
        source.replace("pack\ttrue\n", ""),
        source.replace("pack\ttrue\n", "pack\ttrue\npack\ttrue\n"),
        source.replace("terrain\tnone\npack\ttrue", "pack\ttrue\nterrain\tnone"),
        source.replace("pack\ttrue", "Pack\ttrue"),
    ] {
        assert!(Plan::parse(&changed).is_err());
    }
}
#[test]
fn rejects_noncanonical_whitespace_and_controls() {
    let source = subset().render();
    for changed in [
        source.trim_end().to_owned(),
        format!("{source}\n"),
        format!(" {source}"),
        source.replace('\n', "\r\n"),
        source.replace("pack\ttrue", "pack\ttrue "),
        source.replace("pack\ttrue", "pack\ttrue\t"),
        format!("{source}# comment\n"),
        source.replace("esES", "esÉS"),
        source.replace("esES", "es\0ES"),
        source.replace("esES", "es\u{7f}ES"),
    ] {
        assert!(Plan::parse(&changed).is_err());
    }
}
#[test]
fn rejects_noncanonical_decimals_and_booleans() {
    let source = subset().render();
    for (from, to) in [
        ("70205", "070205"),
        ("70205", "+70205"),
        ("70205", "-70205"),
        ("70205", "4294967296"),
        ("pack\ttrue", "pack\t1"),
        ("pack\ttrue", "pack\tTrue"),
        ("maximum-slope\t55", "maximum-slope\t055"),
    ] {
        assert!(Plan::parse(&source.replace(from, to)).is_err(), "{to}");
    }
}
#[test]
fn rejects_component_aliases_duplicates_and_order() {
    let source = complete().render();
    for value in [
        "",
        "DB2,gt,cameras,maps,vmaps,mmaps",
        "all",
        "db2,db2",
        "gt,db2,cameras,maps,vmaps,mmaps",
        "db2, gt",
    ] {
        assert!(Plan::parse(&source.replace("db2,gt,cameras,maps,vmaps,mmaps", value)).is_err());
    }
    for components in [0, 64, 255] {
        assert!(
            Plan {
                components,
                ..subset()
            }
            .validate()
            .is_err()
        );
    }
}
#[test]
fn rejects_unknown_db2_name_wrong_identity_or_pathname() {
    let source = subset().render();
    for name in [
        "Unknown",
        "chrclasses",
        "../ChrClasses",
        "ChrClasses.db2",
        "Chr Classes",
    ] {
        assert!(Plan::parse(&source.replace("ChrClasses", name)).is_err());
    }
    let mut plan = subset();
    for id in [0, 1, u32::MAX] {
        plan.db2[0].id = id;
        assert!(plan.validate().is_err());
        assert!(Plan::parse(&plan.render()).is_err());
    }
}
#[test]
fn rejects_duplicate_unsorted_or_unselected_db2() {
    let mut plan = subset();
    plan.db2.push(plan.db2[0].clone());
    assert!(plan.validate().is_err());
    let mut plan = complete();
    plan.db2.reverse();
    assert!(Plan::parse(&plan.render()).is_err());
    let plan = Plan {
        db2: vec![],
        ..subset()
    };
    assert!(plan.validate().is_err());
    let plan = Plan {
        components: Component::GameTables.bit(),
        ..subset()
    };
    assert!(plan.validate().is_err());
}
#[test]
fn rejects_camera_without_explicit_source() {
    let plan = Plan {
        components: Component::Db2.bit() | Component::Cameras.bit(),
        ..subset()
    };
    assert!(plan.validate().is_err());
    let plan = Plan {
        db2: vec![entry("CinematicCamera")],
        ..plan
    };
    assert_eq!(Plan::parse(&plan.render()), Ok(plan));
}
#[test]
fn rejects_navigation_missing_components_or_sources() {
    for component in [Component::Db2, Component::Terrain, Component::Collision] {
        let mut plan = complete();
        plan.components &= !component.bit();
        if component == Component::Db2 {
            plan.db2.clear();
        }
        if component == Component::Terrain {
            plan.terrain = Selection::None;
        }
        assert!(plan.validate().is_err());
    }
    for name in ["Map", "LiquidType"] {
        let mut plan = complete();
        plan.db2.retain(|entry| entry.name != name);
        assert!(plan.validate().is_err());
    }
}
#[test]
fn rejects_navigation_without_complete_terrain_inventory() {
    for terrain in [
        Selection::None,
        Selection::Map(0),
        Selection::Tile { map: 0, x: 0, y: 0 },
    ] {
        assert!(
            Plan {
                terrain,
                ..complete()
            }
            .validate()
            .is_err()
        );
    }
}
#[test]
fn rejects_selected_component_without_selection_and_inverse() {
    assert!(
        Plan {
            terrain: Selection::All,
            ..subset()
        }
        .validate()
        .is_err()
    );
    assert!(
        Plan {
            navigation: Selection::All,
            ..subset()
        }
        .validate()
        .is_err()
    );
    let mut plan = complete();
    plan.navigation = Selection::None;
    assert!(plan.validate().is_err());
    plan.terrain = Selection::None;
    assert!(plan.validate().is_err());
}
#[test]
fn rejects_bad_selection_grammar_and_coordinates() {
    let source = complete().render();
    for value in [
        "tile:1:64:0",
        "tile:1:0:64",
        "tile:1:256:0",
        "tile:1:01:0",
        "map:-1",
        "map:01",
        "map:",
        "all:1",
        "tile:1:1",
        "tile:1:1:2:3",
    ] {
        assert!(
            Plan::parse(&source.replace("navigation\tall", &format!("navigation\t{value}")))
                .is_err()
        );
    }
    assert!(
        Plan {
            navigation: Selection::Tile {
                map: 0,
                x: 64,
                y: 0
            },
            ..complete()
        }
        .validate()
        .is_err()
    );
}
#[test]
fn rejects_invalid_or_inactive_navigation_options() {
    for (maximum_slope, non_steep_slope) in [(91, 55), (55, 56), (255, 255)] {
        assert!(
            Plan {
                maximum_slope,
                non_steep_slope,
                ..complete()
            }
            .validate()
            .is_err()
        );
    }
    for plan in [
        Plan {
            maximum_slope: 70,
            ..subset()
        },
        Plan {
            non_steep_slope: 50,
            ..subset()
        },
        Plan {
            big_base_unit: true,
            ..subset()
        },
        Plan {
            skip_liquids: true,
            ..subset()
        },
    ] {
        assert!(plan.validate().is_err());
    }
}
#[test]
fn rejects_oversized_plan_and_entry_count() {
    assert_eq!(
        Plan::parse(&"a".repeat(MAX_PLAN_BYTES + 1)),
        Err("product_plan_size")
    );
    let mut plan = subset();
    plan.db2 = vec![entry("ChrClasses"); MAX_DB2_ENTRIES + 1];
    assert_eq!(plan.validate(), Err("product_plan_size"));
    let mut source = subset().render();
    for _ in 1..=MAX_DB2_ENTRIES {
        source.push_str("db2\tMap\t1349477\n");
    }
    assert_eq!(Plan::parse(&source), Err("product_plan_size"));
}
#[test]
fn complete_catalog_is_an_explicit_valid_db2_plan() {
    let plan = Plan {
        db2: catalog::tables()
            .iter()
            .map(|metadata| Entry {
                name: metadata.name.into(),
                id: metadata.file_data_id,
            })
            .collect(),
        ..subset()
    };
    assert_eq!(plan.db2.len(), MAX_DB2_ENTRIES);
    assert_eq!(Plan::parse(&plan.render()), Ok(plan));
}
