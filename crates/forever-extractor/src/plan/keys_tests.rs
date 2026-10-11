// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: versioned supplied-key capability plan grammar.
use super::*;

const V1: &str = concat!(
    "forever-product\t1\nbuild\t70205\nlocale\tesES\nindex\tcontiguous\n",
    "root\theader-50893-v2\ncomponents\tgt\nterrain\tnone\npack\ttrue\n",
    "navigation\tnone\nbig-base-unit\tfalse\nskip-liquids\tfalse\n",
    "maximum-slope\t55\nnon-steep-slope\t55\n",
);
const CAPABILITY: &str = "decryption\tprovided-salsa20-v1\n";

fn supplied() -> Plan {
    Plan {
        decryption: Decryption::ProvidedSalsa20V1,
        ..Plan::parse(V1).unwrap()
    }
}

#[test]
fn v1_bytes_and_implicit_disabled_semantics_preserved() {
    let plan = Plan::parse(V1).unwrap();
    assert_eq!(plan.decryption, Decryption::Disabled);
    assert_eq!(plan.version(), 1);
    assert_eq!(plan.contract(), "forever-product-v1");
    assert!(!plan.requires_keys());
    assert_eq!(plan.render(), V1);
}

#[test]
fn v2_roundtrip_has_exact_capability_position() {
    let plan = supplied();
    let expected = V1
        .replace("forever-product\t1\n", "forever-product\t2\n")
        .replace("components\tgt\n", &format!("{CAPABILITY}components\tgt\n"));
    assert_eq!(plan.validate(), Ok(()));
    assert_eq!(plan.version(), 2);
    assert_eq!(plan.contract(), "forever-product-v2");
    assert!(plan.requires_keys());
    assert_eq!(plan.render(), expected);
    assert_eq!(Plan::parse(&expected), Ok(plan));
}

#[test]
fn changing_capability_changes_only_header_and_explicit_line() {
    let mut plan = supplied();
    let v2 = plan.render();
    plan.decryption = Decryption::Disabled;
    assert_eq!(plan.render(), V1);
    assert_eq!(
        v2.replace("forever-product\t2\n", "forever-product\t1\n")
            .replace(CAPABILITY, ""),
        V1
    );
    assert_ne!(
        forever_casc::integrity::digest(v2.as_bytes()),
        forever_casc::integrity::digest(V1.as_bytes())
    );
}

#[test]
fn rejects_v2_without_capability_and_v1_with_capability() {
    assert!(Plan::parse(&V1.replace("forever-product\t1", "forever-product\t2")).is_err());
    assert!(
        Plan::parse(
            &supplied()
                .render()
                .replace("forever-product\t2", "forever-product\t1")
        )
        .is_err()
    );
}

#[test]
fn rejects_capability_aliases_and_unknown_versions() {
    let source = supplied().render();
    for value in [
        "none",
        "disabled",
        "salsa20",
        "Provided-salsa20-v1",
        "provided-salsa20-v2",
        "",
    ] {
        assert!(
            Plan::parse(&source.replace("provided-salsa20-v1", value)).is_err(),
            "{value}"
        );
    }
    for version in ["0", "3", "02", "+2", "2 "] {
        assert!(
            Plan::parse(
                &source.replace("forever-product\t2", &format!("forever-product\t{version}"))
            )
            .is_err()
        );
    }
}

#[test]
fn rejects_duplicate_reordered_and_misspelled_capability() {
    let source = supplied().render();
    for changed in [
        source.replace(CAPABILITY, &format!("{CAPABILITY}{CAPABILITY}")),
        source.replace(CAPABILITY, "").replace(
            "root\theader-50893-v2\n",
            &format!("{CAPABILITY}root\theader-50893-v2\n"),
        ),
        source
            .replace(CAPABILITY, "")
            .replace("components\tgt\n", &format!("components\tgt\n{CAPABILITY}")),
        source.replace("decryption\t", "Decryption\t"),
        source.replace(CAPABILITY, "decryption\tprovided-salsa20-v1 \n"),
        source.replace(CAPABILITY, "decryption\tprovided-salsa20-v1\t\n"),
    ] {
        assert!(Plan::parse(&changed).is_err(), "{changed}");
    }
}

#[test]
fn rejects_key_material_paths_and_identity_fields_in_plan() {
    let source = supplied().render();
    for field in [
        "key-path\t/private/keys",
        "key\t0000000000000000",
        "key-count\t1",
        "key-sha256\t1234",
        "key-id\t1",
    ] {
        assert!(Plan::parse(&format!("{source}{field}\n")).is_err());
        assert!(
            Plan::parse(&source.replace(CAPABILITY, &format!("{CAPABILITY}{field}\n"))).is_err()
        );
    }
}

#[test]
fn v2_keeps_component_dependencies_and_selection_limits() {
    let source = supplied().render();
    for changed in [
        source.replace("components\tgt", "components\tcameras"),
        source.replace("components\tgt", "components\tmmaps"),
        source.replace("terrain\tnone", "terrain\tall"),
        source.replace("navigation\tnone", "navigation\tmap:0"),
        source.replace("maximum-slope\t55", "maximum-slope\t91"),
        source.replace("skip-liquids\tfalse", "skip-liquids\ttrue"),
        source
            .replace("components\tgt", "components\tmaps")
            .replace("terrain\tnone", "terrain\ttile:0:64:0"),
    ] {
        assert!(Plan::parse(&changed).is_err(), "{changed}");
    }
}

#[test]
fn v2_all_components_and_layouts_roundtrip() {
    for build in [70170, 70205] {
        for root_layout in [
            root::Layout::Headerless18125,
            root::Layout::Header30080,
            root::Layout::Header50893V1,
            root::Layout::Header50893V2,
        ] {
            for index_layout in [local_index::Layout::Contiguous, local_index::Layout::Paged] {
                let plan = Plan {
                    build,
                    root_layout,
                    index_layout,
                    components: 63,
                    db2: ["CinematicCamera", "LiquidType", "Map"]
                        .into_iter()
                        .map(|name| Entry {
                            name: name.into(),
                            id: catalog::find(name).unwrap().file_data_id,
                        })
                        .collect(),
                    terrain: Selection::All,
                    navigation: Selection::Tile {
                        map: 0,
                        x: 63,
                        y: 0,
                    },
                    ..supplied()
                };
                assert_eq!(Plan::parse(&plan.render()), Ok(plan));
            }
        }
    }
}

#[test]
fn v2_retains_size_and_ascii_gates() {
    assert_eq!(
        Plan::parse(&"x".repeat(MAX_PLAN_BYTES + 1)),
        Err("product_plan_size")
    );
    let source = supplied().render();
    for changed in [
        source.replace('\n', "\r\n"),
        source.replace("salsa20", "salsá20"),
        source.trim_end().into(),
    ] {
        assert!(Plan::parse(&changed).is_err());
    }
}
