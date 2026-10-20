// SPDX-License-Identifier: GPL-3.0-or-later
// Added 2026-10-20: synthetic public consumer tests for v23 package admission.
//! Synthetic-only coverage and package fixtures; no client assets or generated
//! client-derived package bytes are used.

use super::super::*;
use crate::{mmap, tilebuilder::Coordinate};
use forever_casc::{
    Locale, integrity,
    package::{self, Component, Context, Run},
};
use forever_db2::catalog;
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

fn expected(coverage: &Coverage) -> Expected {
    Expected {
        selected_map: coverage.identity.selected_map,
        locale: Locale::EnUs,
        plan_sha256: coverage.identity.plan_sha256,
        context: Some(coverage.context),
    }
}

fn file_data_id(name: &str) -> u32 {
    catalog::resolve_for(catalog::WOWDBDEFS_70450_CATALOG_ID, name)
        .unwrap()
        .unwrap()
        .metadata()
        .file_data_id
}

fn sample() -> Coverage {
    let plan_sha256 = [0x34; 32];
    let producer_sha256 = [0x35; 32];
    let current_source_sha256 = [0x46; 32];
    // Use the profile's admitted locale in this synthetic package fixture.
    let locale = Locale::EnUs;
    let package = |component, package_plan, inventory| PackageIdentity {
        context: Context {
            build: 70450,
            locale,
            source_sha256: current_source_sha256,
            plan_sha256: package_plan,
            producer_sha256,
            components: component,
        },
        plan_sha256: package_plan,
        files: 1,
        bytes: 128,
        inventory_sha256: [inventory; 32],
        coverage_sha256: [inventory + 1; 32],
    };
    let identity = SourceIdentity {
        selected_map: 48,
        build: 70450,
        locale: locale.name().to_owned(),
        plan_sha256,
        producer_sha256,
        current_source_sha256,
        map: MapSnapshot {
            file_data_id: file_data_id("Map"),
            declared_bytes: 1000,
            available_bytes: 800,
            full_ckey_verified: false,
            encoded_key_authenticated: true,
            range_count: 2,
            section_count: 1,
            declared_direct_rows: 1,
            declared_copy_pairs: 1,
            direct_rows: 1,
            copy_pairs: 0,
            terrain_materialized_rows: 1,
            vmap_materialized_rows: 1,
            decoded_sha256: [0x51; 32],
            snapshot_sha256: [0x52; 32],
        },
        closure: vec![
            ClosureMap {
                id: 1,
                root: 1,
                parent: None,
                cosmetic_parent: None,
                wdt_file_data_id: 1199,
                output: false,
                is_parent: true,
                tiles: 0,
            },
            ClosureMap {
                id: 48,
                root: 1,
                parent: Some(1),
                cosmetic_parent: None,
                wdt_file_data_id: 1200,
                output: true,
                is_parent: false,
                tiles: 2,
            },
        ],
        liquid: LiquidSnapshot {
            file_data_id: file_data_id("LiquidType"),
            bytes: 256,
            direct_rows: 3,
            copy_pairs: 1,
            sha256: [0x53; 32],
        },
        terrain: package(Component::Terrain.bit(), [0x61; 32], 0x71),
        terrain_maps: vec![
            TerrainMap {
                id: 1,
                loaded: true,
                known_present: 0,
                converted: 0,
                missing_root_adts: 0,
            },
            TerrainMap {
                id: 48,
                loaded: true,
                known_present: 0,
                converted: 0,
                missing_root_adts: 0,
            },
        ],
        vmap: package(Component::Collision.bit(), [0x62; 32], 0x72),
    };
    let context = Context {
        build: 70450,
        locale,
        source_sha256: [0x11; 32],
        plan_sha256,
        producer_sha256,
        components: Component::Navigation.bit(),
    };
    let mut coverage = Coverage {
        context,
        identity,
        global_bytes: 40,
        global_sha256: [0x81; 32],
        tiles: vec![
            TileOutput {
                tile: Coordinate { row: 0, column: 0 },
                written: true,
                bytes: 12,
                sha256: [0x82; 32],
            },
            TileOutput {
                tile: Coordinate { row: 0, column: 1 },
                written: false,
                bytes: 0,
                sha256: [0; 32],
            },
        ],
        maps: 1,
        written_tiles: 1,
        empty_tiles: 1,
        output_bytes: 52,
        builder_work: 7,
        save_work: 4,
        metadata_calls: 9,
    };
    coverage.context.source_sha256 =
        integrity::digest(coverage.identity.render().unwrap().as_bytes());
    coverage
}

fn assert_rejected(mut coverage: Coverage) {
    refresh_identity_binding(&mut coverage);
    let result = std::panic::catch_unwind(|| coverage.validate());
    assert!(
        result.is_ok(),
        "validation panicked on a synthetic invalid DTO"
    );
    assert!(result.unwrap().is_err());
}

fn refresh_identity_binding(coverage: &mut Coverage) {
    if let Ok(identity) = coverage.identity.render() {
        coverage.context.source_sha256 = integrity::digest(identity.as_bytes());
    }
}

fn rebind_identity_text(base: &Coverage, replacement: String) -> Vec<u8> {
    let mut output = String::from_utf8(base.render().unwrap()).unwrap();
    let old_identity = base.identity.render().unwrap();
    let old_digest = integrity::digest(old_identity.as_bytes());
    let new_digest = integrity::digest(replacement.as_bytes());
    let mut context = base.context;
    context.source_sha256 = new_digest;
    output = output.replace(
        &format!("context\t{}\n", base.context.json()),
        &format!("context\t{}\n", context.json()),
    );
    output = output.replace(
        &format!("identity-sha256\t{}\n", integrity::hex(&old_digest)),
        &format!("identity-sha256\t{}\n", integrity::hex(&new_digest)),
    );
    output = output.replace(
        &format!("identity-hex\t{}\n", hex_bytes(old_identity.as_bytes())),
        &format!("identity-hex\t{}\n", hex_bytes(replacement.as_bytes())),
    );
    output.into_bytes()
}

fn hex_bytes(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 15) as usize] as char);
    }
    output
}

fn parse_rejects_without_panicking(bytes: &[u8]) {
    let result = std::panic::catch_unwind(|| Coverage::parse(bytes));
    assert!(
        result.is_ok(),
        "parser panicked on adversarial synthetic bytes"
    );
    assert!(result.unwrap().is_err());
}

static NEXT_PACKAGE: AtomicU64 = AtomicU64::new(0);

struct PackageLab {
    root: PathBuf,
    output: PathBuf,
}

impl PackageLab {
    fn publish(coverage: &Coverage, global: &[u8], tile: &[u8]) -> Self {
        let coverage_bytes = coverage.render().unwrap();
        let tile_path = mmap::tile_path(
            coverage.identity.selected_map,
            coverage.tiles[0].tile.row,
            coverage.tiles[0].tile.column,
        )
        .unwrap();
        Self::publish_raw(
            coverage,
            &coverage_bytes,
            global,
            &[(tile_path, tile.to_vec())],
        )
    }

    fn publish_raw(
        coverage: &Coverage,
        coverage_bytes: &[u8],
        global: &[u8],
        tiles: &[(String, Vec<u8>)],
    ) -> Self {
        let root = std::env::temp_dir().join(format!(
            "rce-navigation-v23-coverage-{}-{}",
            std::process::id(),
            NEXT_PACKAGE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        let output = root.join("package");
        let mut run =
            package::Run::create(&output, coverage.context, &[], package::Limits::default())
                .unwrap();
        run.put(Component::Navigation, COVERAGE_PATH, coverage_bytes)
            .unwrap();
        run.put(
            Component::Navigation,
            &mmap::global_path(coverage.identity.selected_map),
            global,
        )
        .unwrap();
        for (path, bytes) in tiles {
            run.put(Component::Navigation, path, bytes).unwrap();
        }
        run.finish_component(Component::Navigation).unwrap();
        run.finish().unwrap();
        Self { root, output }
    }

    fn incomplete(context: Context) -> Self {
        let root = std::env::temp_dir().join(format!(
            "rce-navigation-v23-incomplete-{}-{}",
            std::process::id(),
            NEXT_PACKAGE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        let output = root.join("package");
        drop(Run::create(&output, context, &[], package::Limits::default()).unwrap());
        Self { root, output }
    }
}

impl Drop for PackageLab {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
    }
}

fn framed_fixture(coverage: &mut Coverage) -> (Vec<u8>, Vec<u8>) {
    let global = mmap::encode_global(
        coverage.identity.selected_map,
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
    .unwrap();

    // Strict framing fixture only: one declared polygon and vertex, without
    // claiming valid Detour topology or native-query safety.
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
    mmap::GlobalView::parse(
        &global,
        coverage.identity.selected_map,
        mmap::Limits::default(),
    )
    .unwrap();
    mmap::TileView::parse(&tile, mmap::Limits::default()).unwrap();

    coverage.global_bytes = global.len() as u64;
    coverage.global_sha256 = integrity::digest(&global);
    coverage.tiles[0].bytes = tile.len() as u64;
    coverage.tiles[0].sha256 = integrity::digest(&tile);
    coverage.output_bytes = coverage.global_bytes + coverage.tiles[0].bytes;
    (global, tile)
}

fn replace_line(bytes: &[u8], prefix: &str, replacement: &str) -> Vec<u8> {
    let mut text = String::from_utf8(bytes.to_vec()).unwrap();
    let start = text.find(prefix).expect("fixture line exists");
    let end = text[start..]
        .find('\n')
        .map(|n| start + n + 1)
        .unwrap_or(text.len());
    text.replace_range(start..end, replacement);
    text.into_bytes()
}

mod consumer;
mod schema;
