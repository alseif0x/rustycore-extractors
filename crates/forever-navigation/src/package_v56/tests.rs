// SPDX-License-Identifier: GPL-3.0-or-later
// Added 2026-10-31: synthetic v56 package consumer tests.

use super::*;
use crate::{mmap, tilebuilder::Coordinate};
use forever_casc::{
    Locale, integrity,
    package::{self, Component, Context, Run, Summary},
};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_PACKAGE: AtomicU64 = AtomicU64::new(0);

struct Fixture {
    coverage: Coverage56,
    globals: Vec<(u32, Vec<u8>)>,
    tiles: Vec<(u32, Coordinate, Vec<u8>)>,
}

struct PackageLab {
    root: PathBuf,
    output: PathBuf,
}

impl Drop for PackageLab {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
    }
}

impl PackageLab {
    fn publish(fixture: &Fixture) -> Self {
        Self::publish_filtered(fixture, None, None)
    }

    fn publish_filtered(
        fixture: &Fixture,
        skip: Option<&str>,
        extra: Option<(&str, &[u8])>,
    ) -> Self {
        let root = std::env::temp_dir().join(format!(
            "rce-navigation-v56-consumer-{}-{}",
            std::process::id(),
            NEXT_PACKAGE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        let output = root.join("package");
        let mut run = Run::create(
            &output,
            fixture.coverage.context,
            &[],
            package::Limits::default(),
        )
        .unwrap();
        for (path, bytes) in fixture.artifacts() {
            if skip != Some(path.as_str()) {
                run.put(Component::Navigation, &path, &bytes).unwrap();
            }
        }
        if let Some((path, bytes)) = extra {
            run.put(Component::Navigation, path, bytes).unwrap();
        }
        run.finish_component(Component::Navigation).unwrap();
        run.finish().unwrap();
        Self { root, output }
    }
}

impl Fixture {
    fn artifacts(&self) -> Vec<(String, Vec<u8>)> {
        let mut artifacts = vec![(COVERAGE_PATH.to_owned(), self.coverage.render().unwrap())];
        for (map, bytes) in &self.globals {
            artifacts.push((mmap::global_path(*map), bytes.clone()));
        }
        for (map, tile, bytes) in &self.tiles {
            artifacts.push((
                mmap::tile_path(*map, tile.row, tile.column).unwrap(),
                bytes.clone(),
            ));
        }
        artifacts
    }
}

fn expected(coverage: &Coverage56, selected_map: u32) -> Expected {
    Expected {
        selected_map,
        locale: coverage.context.locale,
        plan_sha256: coverage.context.plan_sha256,
        context: Some(coverage.context),
    }
}

fn global(map: u32) -> Vec<u8> {
    mmap::encode_global(
        map,
        mmap::Params {
            origin: [0.0; 3],
            tile_width: 1.0,
            tile_height: 1.0,
            max_tiles: 4096,
            max_polys: i32::MIN,
        },
        &[],
        mmap::Limits::default(),
    )
    .unwrap()
}

fn tile() -> Vec<u8> {
    let mut payload = vec![0; 144];
    for (offset, value) in [(0, 0x444e_4156_u32), (4, 7), (24, 1), (28, 1)] {
        payload[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }
    let mut tile = vec![0; mmap::TILE_HEADER_BYTES + payload.len()];
    for (offset, value) in [
        (0, mmap::MAGIC),
        (4, 7),
        (8, mmap::VERSION),
        (12, payload.len() as u32),
    ] {
        tile[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }
    tile[16] = 1;
    tile[mmap::TILE_HEADER_BYTES..].copy_from_slice(&payload);
    mmap::TileView::parse(&tile, mmap::Limits::default()).unwrap();
    tile
}

fn output(map: u32, global: &[u8], tiles: Vec<TileOutput>) -> Output {
    Output {
        map,
        global: Some(TileOutput {
            tile: Coordinate { row: 0, column: 0 },
            written: true,
            bytes: global.len() as u64,
            sha256: integrity::digest(global),
        }),
        tiles,
        partial_triangulations: 0,
        detail_center_fallbacks: 0,
        unset_height_samples: 0,
    }
}

fn written(tile: Coordinate, bytes: &[u8]) -> TileOutput {
    TileOutput {
        tile,
        written: true,
        bytes: bytes.len() as u64,
        sha256: integrity::digest(bytes),
    }
}

fn empty(tile: Coordinate) -> TileOutput {
    TileOutput {
        tile,
        written: false,
        bytes: 0,
        sha256: [0; 32],
    }
}

fn sample() -> Fixture {
    let source_context = Context {
        build: 70450,
        locale: Locale::EsEs,
        source_sha256: [0x10; 32],
        plan_sha256: [0x51; 32],
        producer_sha256: [0x52; 32],
        components: 25,
    };
    let source = Summary {
        context: source_context,
        files: 10,
        bytes: 1000,
        inventory_sha256: [0x53; 32],
    };
    let context = Context {
        build: 70450,
        locale: Locale::EsEs,
        source_sha256: integrity::digest(
            format!("rustycore-navigation-v56-sealed-v51\n{}\n", source.json()).as_bytes(),
        ),
        plan_sha256: [0x34; 32],
        producer_sha256: [0x35; 32],
        components: Component::Navigation.bit(),
    };
    let global1 = global(1);
    let global2 = global(2);
    let tile1 = tile();
    let coordinate1 = Coordinate { row: 0, column: 0 };
    let coordinate2 = Coordinate { row: 1, column: 0 };
    let empty2 = Coordinate { row: 1, column: 1 };
    let maps = vec![
        MapFact {
            id: 1,
            root: 1,
            state: "planned",
            tiles: vec![coordinate1],
        },
        MapFact {
            id: 2,
            root: 2,
            state: "planned",
            tiles: vec![coordinate2, empty2],
        },
        MapFact {
            id: 3,
            root: 3,
            state: "empty",
            tiles: Vec::new(),
        },
        MapFact {
            id: 4,
            root: 4,
            state: "root-absent",
            tiles: Vec::new(),
        },
    ];
    let outputs = vec![
        output(1, &global1, vec![written(coordinate1, &tile1)]),
        output(
            2,
            &global2,
            vec![written(coordinate2, &tile1), empty(empty2)],
        ),
        Output {
            map: 3,
            global: None,
            tiles: Vec::new(),
            partial_triangulations: 0,
            detail_center_fallbacks: 0,
            unset_height_samples: 0,
        },
        Output {
            map: 4,
            global: None,
            tiles: Vec::new(),
            partial_triangulations: 0,
            detail_center_fallbacks: 0,
            unset_height_samples: 0,
        },
    ];
    Fixture {
        coverage: Coverage56 {
            context,
            source,
            maps,
            outputs,
            builder: 0,
            save: 0,
        },
        globals: vec![(1, global1), (2, global2)],
        tiles: vec![(1, coordinate1, tile1.clone()), (2, coordinate2, tile1)],
    }
}

#[test]
fn verified_read_selects_one_map_and_preserves_written_empty_results() {
    let fixture = sample();
    let lab = PackageLab::publish(&fixture);
    let verified = verify(&lab.output, expected(&fixture.coverage, 2)).unwrap();
    assert_eq!(verified.summary().files, 5);
    assert_eq!(verified.map().id, 2);
    assert_eq!(verified.output().tiles.len(), 2);

    let requested = [
        Coordinate { row: 1, column: 1 },
        Coordinate { row: 1, column: 0 },
    ];
    let batch = read_batch(
        &lab.output,
        expected(&fixture.coverage, 2),
        &requested,
        ReadLimits::default(),
    )
    .unwrap();
    assert_eq!(batch.map().id, 2);
    assert_eq!(batch.global_bytes(), fixture.globals[1].1.as_slice());
    assert_eq!(batch.tiles().len(), 2);
    assert!(batch.tiles()[0].is_empty());
    assert_eq!(batch.tiles()[0].bytes(), None);
    assert!(!batch.tiles()[1].is_empty());
    assert_eq!(
        batch.tiles()[1].bytes(),
        Some(fixture.tiles[1].2.as_slice())
    );

    let first = read_batch(
        &lab.output,
        expected(&fixture.coverage, 1),
        &[],
        ReadLimits::default(),
    )
    .unwrap();
    assert_eq!(first.map().id, 1);
    assert_eq!(first.global_bytes(), fixture.globals[0].1.as_slice());
}

#[test]
fn verifier_requires_exact_multi_map_inventory() {
    let fixture = sample();
    let tile_path = mmap::tile_path(2, 1, 0).unwrap();
    let missing = PackageLab::publish_filtered(&fixture, Some(&tile_path), None);
    assert_eq!(
        verify(&missing.output, expected(&fixture.coverage, 2))
            .unwrap_err()
            .code(),
        "product_navigation_output_inventory"
    );

    let extra = PackageLab::publish_filtered(
        &fixture,
        None,
        Some(("mmaps/0009_00_00.mmtile", &fixture.tiles[0].2)),
    );
    assert_eq!(
        verify(&extra.output, expected(&fixture.coverage, 1))
            .unwrap_err()
            .code(),
        "product_navigation_output_inventory"
    );
}

#[test]
fn read_rejects_unplanned_maps_bad_requests_and_reduced_limits() {
    let fixture = sample();
    let lab = PackageLab::publish(&fixture);
    assert_eq!(
        verify(&lab.output, expected(&fixture.coverage, 3))
            .unwrap_err()
            .code(),
        "product_navigation_coverage"
    );
    assert_eq!(
        read_batch(
            &lab.output,
            expected(&fixture.coverage, 2),
            &[Coordinate { row: 9, column: 9 }],
            ReadLimits::default(),
        )
        .unwrap_err()
        .code(),
        "product_navigation_read_unavailable"
    );
    assert_eq!(
        read_batch(
            &lab.output,
            expected(&fixture.coverage, 2),
            &[
                Coordinate { row: 1, column: 0 },
                Coordinate { row: 1, column: 0 },
            ],
            ReadLimits::default(),
        )
        .unwrap_err()
        .code(),
        "product_navigation_read_request"
    );
    let limits = ReadLimits {
        max_file_bytes: fixture.tiles[1].2.len() - 1,
        ..ReadLimits::default()
    };
    assert_eq!(
        read_batch(
            &lab.output,
            expected(&fixture.coverage, 2),
            &[Coordinate { row: 1, column: 0 }],
            limits,
        )
        .unwrap_err()
        .code(),
        "product_navigation_read_limit"
    );
}

#[test]
fn verifier_binds_expected_identity_and_detects_payload_changes() {
    let fixture = sample();
    let lab = PackageLab::publish(&fixture);
    let mut wrong_locale = expected(&fixture.coverage, 1);
    wrong_locale.locale = Locale::EnUs;
    assert_eq!(
        verify(&lab.output, wrong_locale).unwrap_err().code(),
        "product_navigation_identity"
    );
    let mut wrong_plan = expected(&fixture.coverage, 1);
    wrong_plan.plan_sha256 = [0x99; 32];
    assert_eq!(
        verify(&lab.output, wrong_plan).unwrap_err().code(),
        "product_navigation_identity"
    );

    let path = lab.output.join(mmap::tile_path(1, 0, 0).unwrap());
    let mut changed = fixture.tiles[0].2.clone();
    changed[20] ^= 1;
    fs::write(path, changed).unwrap();
    assert_eq!(
        verify(&lab.output, expected(&fixture.coverage, 1))
            .unwrap_err()
            .code(),
        "package_digest"
    );
}
