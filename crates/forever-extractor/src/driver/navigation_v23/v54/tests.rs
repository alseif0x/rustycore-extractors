// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
const PLAN: &str =
    include_str!("../../../../../../tests/contracts/fixtures/forever-product-v54/all.tsv");
#[test]
fn v54_plan_identity_and_rejected_aliases() {
    let plan = Plan::parse(PLAN).unwrap();
    assert!(plan.is_v54());
    assert_eq!(plan.version(), 54);
    assert_eq!(plan.components, 32);
    assert_eq!(plan.render(), PLAN);
    for (old, new) in [
        ("all-available-maps", "all"),
        ("all-available-maps", "map:1"),
        ("components\tmmaps", "components\tdb2,mmaps"),
        ("decryption\tdisabled", "decryption\tprovided-salsa20-v1"),
        ("pack\ttrue", "pack\tfalse"),
        ("big-base-unit\tfalse", "big-base-unit\ttrue"),
    ] {
        assert!(Plan::parse(&PLAN.replace(old, new)).is_err(), "{new}");
    }
    assert_eq!(
        driver::open(&plan, Path::new("/nonexistent-client"))
            .err()
            .map(|e| e.code),
        Some("product_navigation_inputs_required")
    );
    assert_eq!(
        driver::extract(
            &plan,
            Path::new("/nonexistent-client"),
            Path::new("/nonexistent-output"),
            [0; 32],
            false,
        )
        .err()
        .map(|e| e.code),
        Some("product_navigation_inputs_required")
    );
}
fn coverage() -> Coverage54 {
    let source = Summary {
        context: Context {
            build: 70450,
            locale: forever_casc::Locale::EnUs,
            components: 25,
            source_sha256: [1; 32],
            plan_sha256: [2; 32],
            producer_sha256: [3; 32],
        },
        files: 5,
        bytes: 123,
        inventory_sha256: [4; 32],
    };
    let context = Context {
        build: 70450,
        locale: forever_casc::Locale::EnUs,
        components: 32,
        source_sha256: identity(source),
        plan_sha256: [5; 32],
        producer_sha256: [6; 32],
    };
    let tile = Coordinate { row: 1, column: 2 };
    Coverage54 {
        context,
        source,
        maps: vec![
            MapFact {
                id: 1,
                root: 1,
                state: "planned",
                tiles: vec![tile],
            },
            MapFact {
                id: 2,
                root: 2,
                state: "empty",
                tiles: vec![],
            },
            MapFact {
                id: 3,
                root: 3,
                state: "root-absent",
                tiles: vec![],
            },
        ],
        outputs: vec![
            Output {
                map: 1,
                global: Some(TileOutput {
                    tile: Coordinate { row: 0, column: 0 },
                    written: true,
                    bytes: 32,
                    sha256: [7; 32],
                }),
                tiles: vec![TileOutput {
                    tile,
                    written: false,
                    bytes: 0,
                    sha256: [0; 32],
                }],
                partial_triangulations: 1,
            },
            Output {
                map: 2,
                global: None,
                tiles: vec![],
                partial_triangulations: 0,
            },
            Output {
                map: 3,
                global: None,
                tiles: vec![],
                partial_triangulations: 0,
            },
        ],
        builder: 0,
        save: 0,
    }
}
#[test]
fn v54_coverage_empty_outcomes_roundtrip_and_invalid_inventory() {
    let value = coverage();
    let bytes = value.render().unwrap();
    assert_eq!(Coverage54::parse(&bytes).unwrap().render().unwrap(), bytes);
    let text = String::from_utf8(bytes).unwrap();
    for forged in [
        text.replace("map\t2\t2\tempty\t0", "map\t1\t2\tempty\t0"),
        text.replace("tile\t1\t2\tfalse\t0", "tile\t1\t2\tfalse\t1"),
        text.replace("map\t2\t2\tempty\t0", "map\t2\t2\tplanned\t0"),
        text.replace(
            "map\t2\t2\tempty\t0\npartials\t0",
            "map\t2\t2\tempty\t0\npartials\t1",
        ),
        text.replace("map\t1\t1\tplanned\t1", "map\t1\t1\tplanned\t4097"),
        text.replace("work\t0\t0", "work\t18446744073709551615\t0"),
    ] {
        assert!(Coverage54::parse(forged.as_bytes()).is_err());
    }
    assert!(sha(&"é".repeat(32)).is_err());
}
#[test]
fn v54_limits_scale_vertex_heap_without_changing_owner_or_v23() {
    use navigation_input::run::{AvailableLimits, AvailableProfile};
    let old = AvailableLimits::default();
    let world = AvailableLimits::all_available_v54(71, 3322).unwrap();
    assert_eq!(old.profile, AvailableProfile::SelectedV23);
    assert_eq!(old.run.maps, 1);
    assert_eq!(old.run.tiles, 4096);
    assert_eq!(world.profile, AvailableProfile::AllAvailableV54);
    assert_eq!(world.heap_bytes, old.heap_bytes);
    assert_eq!(world.run.pipeline.models.mesh.vertices, 1_000_000);
    assert_eq!(world.run.pipeline.compact.vertices, 1_000_000);
    assert_eq!(world.run.pipeline.models.mesh.heap_bytes, 48 * 1024 * 1024);
    assert_eq!(world.run.pipeline.compact.heap_bytes, 48 * 1024 * 1024);
    assert_eq!(world.run.pipeline.models.mesh.triangles, 500_000);
    assert_eq!(world.run.pipeline.compact.triangles, 500_000);
    assert!(world.run.pipeline.builder.allow_partial_triangulation);
    assert_eq!(old.run.pipeline.models.mesh.vertices, 250_000);
    assert_eq!(old.run.pipeline.compact.vertices, 250_000);
    world
        .run
        .pipeline
        .models
        .mesh
        .validate_available_v54()
        .unwrap();
    world.run.pipeline.compact.validate_available_v54().unwrap();
    let excessive = forever_navigation::Limits {
        vertices: 1_000_001,
        ..Default::default()
    };
    assert!(excessive.validate_available_v54().is_err());
    let excessive_heap = forever_navigation::Limits {
        vertices: 1_000_000,
        heap_bytes: 48 * 1024 * 1024 + 1,
        ..Default::default()
    };
    assert!(excessive_heap.validate_available_v54().is_err());
    assert_eq!(world.builder_work, 3322 * old.run.pipeline.builder.work);
    assert_eq!(world.save_work, 3322 * old.run.pipeline.save.work);
    assert!(AvailableLimits::all_available_v54(4096, 4096 * 4096).is_ok());
    for (maps, tiles) in [(4097, 1), (4096, 4096 * 4096 + 1), (1, 4097), (0, 1)] {
        assert!(AvailableLimits::all_available_v54(maps, tiles).is_err());
    }
}
