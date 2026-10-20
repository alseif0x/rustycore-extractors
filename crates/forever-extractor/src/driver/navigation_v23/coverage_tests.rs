// SPDX-License-Identifier: GPL-3.0-or-later
// Modified by the RustyCore Extractors project, 2026-10-20.
//! Synthetic producer input and map projection checks for selected navigation.
//! No client assets or generated packages are used.

use super::*;
use forever_casc::Locale;
use std::{
    process::{Command, Stdio},
    sync::atomic::{AtomicU64, Ordering},
    thread,
    time::{Duration, Instant},
};

static NEXT_PACKAGE: AtomicU64 = AtomicU64::new(0);

fn file_data_id(name: &str) -> u32 {
    catalog::resolve_for(SchemaCatalog::WowDbDefs70450V1.id(), name)
        .unwrap()
        .unwrap()
        .metadata()
        .file_data_id
}

fn sample() -> Coverage {
    let plan = Plan::parse(include_str!(
        "../../../../../tests/contracts/fixtures/forever-product-v23/map48.tsv"
    ))
    .unwrap();
    let plan_sha256 = integrity::digest(plan.render().as_bytes());
    let producer_sha256 = [0x35; 32];
    let current_source_sha256 = [0x46; 32];
    // Bind the synthetic output package to the exact locale of the pinned plan fixture.
    let locale = plan.locale;
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

#[test]
fn producer_coverage_binds_the_canonical_product_plan_digest() {
    let coverage = sample();
    let plan = Plan::parse(include_str!(
        "../../../../../tests/contracts/fixtures/forever-product-v23/map48.tsv"
    ))
    .unwrap();
    assert_eq!(
        coverage.identity.plan_sha256,
        integrity::digest(plan.render().as_bytes())
    );
}

struct LockedInputLab {
    root: PathBuf,
    output: PathBuf,
    summary: Summary,
    artifact: Artifact,
}

impl LockedInputLab {
    fn publish(component: Component, path: &str, bytes: &[u8]) -> Self {
        let root = std::env::temp_dir().join(format!(
            "rce-navigation-v23-locked-input-{}-{}",
            std::process::id(),
            NEXT_PACKAGE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        let output = root.join("package");
        let context = Context {
            build: 70450,
            locale: Locale::EnUs,
            source_sha256: [0x21; 32],
            plan_sha256: [0x22; 32],
            producer_sha256: [0x23; 32],
            components: component.bit(),
        };
        let mut run = Run::create(&output, context, &[], package::Limits::default()).unwrap();
        run.put(component, path, bytes).unwrap();
        let artifact = Artifact {
            component,
            path: path.to_owned(),
            bytes: bytes.len() as u64,
            sha256: integrity::digest(bytes),
        };
        run.finish_component(component).unwrap();
        let summary = run.finish().unwrap();
        Self {
            root,
            output,
            summary,
            artifact,
        }
    }
}

impl Drop for LockedInputLab {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
    }
}

struct SeparationLab {
    root: PathBuf,
}

impl SeparationLab {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "rce-navigation-v23-separation-{}-{}",
            std::process::id(),
            NEXT_PACKAGE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        Self { root }
    }
}

impl Drop for SeparationLab {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
    }
}

fn verify_locked_input(
    lab: &LockedInputLab,
    expected_artifacts: &[Artifact],
    coverage_sha256: [u8; 32],
    callback: impl FnOnce() -> Result<(), Error>,
) -> Result<Summary, Error> {
    verify_locked_package(
        LockedPackage {
            root: &lab.output,
            expected_summary: lab.summary,
            expected_artifacts,
            coverage_path: &lab.artifact.path,
            coverage_sha256,
            max_coverage: 1024,
            limits: package::Limits::default(),
        },
        callback,
    )
}

#[test]
fn input_output_separation_rejects_aliases_and_overlaps() {
    let lab = SeparationLab::new();
    let terrain = lab.root.join("terrain");
    let vmap = lab.root.join("vmap");
    let terrain_maps = terrain.join("maps");
    let output_parent = lab.root.join("output-parent");
    fs::create_dir(&terrain).unwrap();
    fs::create_dir(&terrain_maps).unwrap();
    fs::create_dir(&vmap).unwrap();
    fs::create_dir(&output_parent).unwrap();
    let terrain_plan = lab.root.join("terrain-plan.tsv");
    let vmap_plan = lab.root.join("vmap-plan.tsv");
    fs::write(&terrain_plan, b"synthetic terrain plan\n").unwrap();
    fs::write(&vmap_plan, b"synthetic vmap plan\n").unwrap();
    let output = output_parent.join("package");

    let (admitted_output, admitted_roots) =
        validate_input_output_separation(&terrain, &terrain_plan, &vmap, &vmap_plan, &output)
            .unwrap();
    assert_eq!(admitted_output, output);
    assert_eq!(admitted_roots, [terrain.clone(), vmap.clone()]);

    let error =
        validate_input_output_separation(&terrain, &terrain_plan, &terrain, &vmap_plan, &output)
            .unwrap_err();
    assert_eq!(error.code, "product_navigation_path_overlap");

    let error = validate_input_output_separation(
        &terrain,
        &terrain_plan,
        &terrain_maps,
        &vmap_plan,
        &output,
    )
    .unwrap_err();
    assert_eq!(error.code, "product_navigation_path_overlap");

    let error = validate_input_output_separation(
        &terrain,
        &terrain_plan,
        &vmap,
        &vmap_plan,
        &terrain.join("new-output"),
    )
    .unwrap_err();
    assert_eq!(error.code, "product_navigation_path_overlap");

    let error =
        validate_input_output_separation(&terrain, &terrain_plan, &vmap, &vmap_plan, &lab.root)
            .unwrap_err();
    assert_eq!(error.code, "product_navigation_path_overlap");

    let nested_plan = terrain.join("nested-plan.tsv");
    fs::write(&nested_plan, b"synthetic nested plan\n").unwrap();
    let error =
        validate_input_output_separation(&terrain, &nested_plan, &vmap, &vmap_plan, &output)
            .unwrap_err();
    assert_eq!(error.code, "product_navigation_path_overlap");

    let shared_plan =
        validate_input_output_separation(&terrain, &terrain_plan, &vmap, &terrain_plan, &output)
            .unwrap_err();
    assert_eq!(shared_plan.code, "product_navigation_path_overlap");

    let terrain_link = lab.root.join("terrain-link");
    std::os::unix::fs::symlink(&terrain, &terrain_link).unwrap();
    let error =
        validate_input_output_separation(&terrain_link, &terrain_plan, &vmap, &vmap_plan, &output)
            .unwrap_err();
    assert_eq!(error.code, "product_navigation_path");
}

#[test]
fn locked_input_callback_is_skipped_for_inventory_or_coverage_mismatch() {
    let lab = LockedInputLab::publish(
        Component::Terrain,
        INPUT_COVERAGE_TERRAIN,
        b"synthetic locked terrain coverage\n",
    );

    let mut wrong_inventory = lab.artifact.clone();
    wrong_inventory.sha256[0] ^= 1;
    let mut inventory_callback_calls = 0;
    let error = verify_locked_input(&lab, &[wrong_inventory], lab.artifact.sha256, || {
        inventory_callback_calls += 1;
        Ok(())
    })
    .unwrap_err();
    assert_eq!(error.code, "package_inventory");
    assert_eq!(inventory_callback_calls, 0);

    let mut coverage_callback_calls = 0;
    let error = verify_locked_input(
        &lab,
        std::slice::from_ref(&lab.artifact),
        [0x99; 32],
        || {
            coverage_callback_calls += 1;
            Ok(())
        },
    )
    .unwrap_err();
    assert_eq!(error.code, "product_navigation_coverage");
    assert_eq!(coverage_callback_calls, 0);
}

#[test]
fn locked_input_callback_error_is_preserved() {
    let lab = LockedInputLab::publish(
        Component::Terrain,
        INPUT_COVERAGE_TERRAIN,
        b"synthetic locked terrain coverage\n",
    );
    let mut calls = 0;
    let error = verify_locked_input(
        &lab,
        std::slice::from_ref(&lab.artifact),
        lab.artifact.sha256,
        || {
            calls += 1;
            Err(Error::from("product_navigation_synthetic_callback"))
        },
    )
    .unwrap_err();
    assert_eq!(calls, 1);
    assert_eq!(error.code, "product_navigation_synthetic_callback");
}

#[test]
fn nested_shared_package_locks_remain_held_during_callback() {
    const CHILD: &str = "RCE_NAV_V23_LOCK_CHILD";
    const EXPECT_HELD: &str = "RCE_NAV_V23_LOCK_EXPECT_HELD";
    const TERRAIN_LOCK: &str = "RCE_NAV_V23_TERRAIN_LOCK";
    const VMAP_LOCK: &str = "RCE_NAV_V23_VMAP_LOCK";
    if std::env::var_os(CHILD).is_some() {
        let expect_held = std::env::var_os(EXPECT_HELD).is_some();
        for variable in [TERRAIN_LOCK, VMAP_LOCK] {
            let path = std::env::var_os(variable).expect("child lock path is supplied");
            let lock = OpenOptions::new()
                .read(true)
                .write(true)
                .open(path)
                .unwrap();
            if expect_held {
                assert!(
                    matches!(lock.try_lock(), Err(std::fs::TryLockError::WouldBlock)),
                    "exclusive child lock was not blocked by the callback-held shared lock"
                );
            } else {
                lock.try_lock()
                    .expect("exclusive child lock should succeed after callback returns");
                lock.unlock().expect("child lock should release cleanly");
            }
        }
        return;
    }

    let terrain = LockedInputLab::publish(
        Component::Terrain,
        INPUT_COVERAGE_TERRAIN,
        b"synthetic locked terrain coverage\n",
    );
    let vmap = LockedInputLab::publish(
        Component::Collision,
        "vmaps/selected-input-coverage.bin",
        b"synthetic locked vmap coverage\n",
    );
    let mut callback_calls = 0;
    let assert_lock_state = |expect_held: bool| {
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .arg("nested_shared_package_locks_remain_held_during_callback")
            .arg("--nocapture")
            .current_dir(&terrain.root)
            .env(CHILD, "1")
            .env(TERRAIN_LOCK, terrain.output.join(".state/lock"))
            .env(VMAP_LOCK, vmap.output.join(".state/lock"))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if expect_held {
            command.env(EXPECT_HELD, "1");
        } else {
            command.env_remove(EXPECT_HELD);
        }
        let mut child = command.spawn().unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        let output = loop {
            if child.try_wait().unwrap().is_some() {
                break child.wait_with_output().unwrap();
            }
            if Instant::now() >= deadline {
                let _ = child.kill();
                let output = child.wait_with_output().unwrap();
                panic!(
                    "nested lock child exceeded its five-second bound: {}",
                    String::from_utf8_lossy(&output.stdout)
                );
            }
            thread::sleep(Duration::from_millis(10));
        };
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            output.status.success(),
            "nested lock child failed: {stdout}"
        );
        assert!(
            stdout.contains("1 passed"),
            "child test did not execute: {stdout}"
        );
    };
    let result = verify_locked_input(
        &terrain,
        std::slice::from_ref(&terrain.artifact),
        terrain.artifact.sha256,
        || {
            verify_locked_input(
                &vmap,
                std::slice::from_ref(&vmap.artifact),
                vmap.artifact.sha256,
                || {
                    assert_lock_state(true);
                    callback_calls += 1;
                    Ok(())
                },
            )
            .map(|_| ())
        },
    );
    assert!(result.is_ok(), "nested package checks failed: {result:?}");
    assert_eq!(callback_calls, 1);
    assert_lock_state(false);
}

#[test]
fn failed_post_callback_package_check_leaves_only_provisional_output() {
    let lab = LockedInputLab::publish(
        Component::Terrain,
        INPUT_COVERAGE_TERRAIN,
        b"synthetic locked terrain coverage\n",
    );
    let provisional = lab.root.join("provisional-navigation");
    let context = sample().context;
    let mut callback_calls = 0;
    let result = verify_locked_input(
        &lab,
        std::slice::from_ref(&lab.artifact),
        lab.artifact.sha256,
        || {
            callback_calls += 1;
            let mut run = Run::create(
                &provisional,
                context,
                &[lab.output.as_path()],
                package::Limits::default(),
            )
            .unwrap();
            run.put(
                Component::Navigation,
                COVERAGE_PATH,
                b"synthetic provisional output\n",
            )
            .unwrap();
            drop(run);
            fs::write(
                lab.output.join(&lab.artifact.path),
                b"mutated after callback\n",
            )
            .unwrap();
            Ok(())
        },
    );
    let error = result.unwrap_err();
    assert_eq!(callback_calls, 1);
    assert_eq!(error.code, "package_input_changed");
    assert!(provisional.join(package::INCOMPLETE).is_file());
    assert!(!provisional.join(package::MANIFEST).exists());
    assert!(provisional.join(COVERAGE_PATH).is_file());
    assert!(!provisional.join(".state/sealed/navigation").exists());
    assert!(!provisional.join(".state/sealed/mmaps").exists());
}

struct SyntheticMapSource(Vec<u8>);

impl forever_db2::availability::Source for SyntheticMapSource {
    fn declared_bytes(&self) -> u64 {
        self.0.len() as u64
    }

    fn range(
        &self,
        offset: u64,
        bytes: usize,
    ) -> Result<&[u8], forever_db2::availability::Unavailable> {
        let start = usize::try_from(offset).map_err(|_| forever_db2::availability::Unavailable)?;
        let end = start
            .checked_add(bytes)
            .ok_or(forever_db2::availability::Unavailable)?;
        self.0
            .get(start..end)
            .ok_or(forever_db2::availability::Unavailable)
    }
}

fn synthetic_map_with_zero_wdt_and_one_direct_copy() -> SyntheticMapSource {
    use forever_db2::catalog::FieldType;

    const HEADER: usize = 204;
    const SECTION: usize = 40;
    fn put_u16(bytes: &mut [u8], at: usize, value: u16) {
        bytes[at..at + 2].copy_from_slice(&value.to_le_bytes());
    }
    fn put_u32(bytes: &mut [u8], at: usize, value: u32) {
        bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
    }
    fn width(kind: FieldType) -> u32 {
        match kind {
            FieldType::Byte => 8,
            FieldType::Short => 16,
            FieldType::String
            | FieldType::NonLocalizedString
            | FieldType::Int
            | FieldType::Float => 32,
            FieldType::Long => 64,
        }
    }

    let table = catalog::resolve_for(SchemaCatalog::WowDbDefs70450V1.id(), "Map")
        .unwrap()
        .unwrap();
    let metadata = table.metadata();
    assert_eq!(metadata.name, "Map");
    assert_eq!(metadata.field_count, metadata.file_field_count);
    assert_eq!(metadata.index, -1);
    assert!(matches!(
        metadata.fields[0].kind,
        FieldType::String | FieldType::NonLocalizedString
    ));
    assert!(matches!(
        metadata.fields[1].kind,
        FieldType::String | FieldType::NonLocalizedString
    ));

    let field_count = metadata.file_field_count;
    let mut offsets = Vec::with_capacity(field_count);
    let mut bit_offset = 0_u32;
    for field in metadata.fields.iter().take(field_count) {
        offsets.push(bit_offset);
        bit_offset = bit_offset
            .checked_add(width(field.kind) * u32::from(field.array_size))
            .unwrap();
    }
    assert_eq!(bit_offset % 8, 0);
    let record_size = usize::try_from(bit_offset / 8).unwrap();
    let columns_start = HEADER + SECTION + field_count * 4;
    let data_start = columns_start + field_count * 24;
    let mut bytes = vec![0_u8; data_start];

    bytes[..4].copy_from_slice(b"WDC5");
    put_u32(&mut bytes, 4, 5);
    put_u32(&mut bytes, 136, 1);
    put_u32(&mut bytes, 140, field_count as u32);
    put_u32(&mut bytes, 144, record_size as u32);
    put_u32(&mut bytes, 148, 2);
    put_u32(&mut bytes, 152, metadata.table_hash.unwrap_or(0));
    put_u32(&mut bytes, 156, metadata.layout_hash);
    put_u32(&mut bytes, 160, 0);
    put_u32(&mut bytes, 164, 48);
    put_u16(&mut bytes, 172, 0);
    bytes[174..176].copy_from_slice(&metadata.index.to_le_bytes());
    put_u32(&mut bytes, 176, field_count as u32);
    put_u32(&mut bytes, 180, 0);
    put_u32(&mut bytes, 184, 0);
    put_u32(&mut bytes, 188, (field_count * 24) as u32);
    put_u32(&mut bytes, 192, 0);
    put_u32(&mut bytes, 196, 0);
    put_u32(&mut bytes, 200, 1);

    let section_at = HEADER;
    put_u32(&mut bytes, section_at + 8, data_start as u32);
    put_u32(&mut bytes, section_at + 12, 1);
    put_u32(&mut bytes, section_at + 16, 2);
    put_u32(&mut bytes, section_at + 20, 0);
    put_u32(&mut bytes, section_at + 24, 4);
    put_u32(&mut bytes, section_at + 28, 0);
    put_u32(&mut bytes, section_at + 32, 0);
    put_u32(&mut bytes, section_at + 36, 1);

    for (index, field) in metadata.fields.iter().take(field_count).enumerate() {
        let at = columns_start + index * 24;
        put_u16(&mut bytes, at, u16::try_from(offsets[index]).unwrap());
        put_u32(&mut bytes, at + 8, 0);
        put_u32(&mut bytes, at + 16, width(field.kind));
        put_u32(&mut bytes, at + 20, u32::from(field.array_size));
    }

    let mut record = vec![0_u8; record_size];
    for (index, target) in [(0, record_size), (1, record_size + 1)] {
        let offset = usize::try_from(offsets[index] / 8).unwrap();
        let relative = u32::try_from(target - offset).unwrap();
        put_u32(&mut record, offset, relative);
    }
    for field in [13, 14] {
        let offset = usize::try_from(offsets[field] / 8).unwrap();
        put_u16(&mut record, offset, u16::MAX);
    }
    bytes.extend_from_slice(&record);
    bytes.extend_from_slice(&[0, 0]);
    bytes.extend_from_slice(&48_u32.to_le_bytes());
    bytes.extend_from_slice(&49_u32.to_le_bytes());
    bytes.extend_from_slice(&48_u32.to_le_bytes());
    SyntheticMapSource(bytes)
}

#[test]
fn map_reader_projections_bind_distinct_wdt_filtered_and_vmap_counts() {
    let source = synthetic_map_with_zero_wdt_and_one_direct_copy();
    let navigation = availability::read_navigation_maps_70450(&source).unwrap();
    let terrain = availability::read_map_70450(&source).unwrap();
    let vmap = availability::read_vmap_maps_70450(&source).unwrap();
    assert_eq!(navigation.direct_records().len(), 1);
    assert_eq!(navigation.copies().len(), 1);
    assert_eq!(terrain.direct_records()[0].fields.wdt_file_data_id, 0);
    validate_map_reader_parity(&navigation, &terrain, &vmap).unwrap();
    let projections = materialize_map_projections(terrain, vmap).unwrap();
    assert_eq!(projections.terrain_rows, 0);
    assert_eq!(projections.vmap_rows, 2);

    let decoded_sha = map_decoded_facts_sha256(&navigation, 0, 2).unwrap();
    assert_ne!(
        decoded_sha,
        map_decoded_facts_sha256(&navigation, 1, 2).unwrap()
    );
    assert_ne!(
        decoded_sha,
        map_decoded_facts_sha256(&navigation, 0, 1).unwrap()
    );

    let mut coverage = sample();
    coverage.identity.map.direct_rows = navigation.direct_records().len();
    coverage.identity.map.copy_pairs = navigation.copies().len();
    coverage.identity.map.terrain_materialized_rows = projections.terrain_rows;
    coverage.identity.map.vmap_materialized_rows = projections.vmap_rows;
    coverage.identity.map.decoded_sha256 = decoded_sha;
    assert!(terrain_materialized_rows_match(
        &coverage.identity.map,
        projections.terrain_rows
    ));
    assert!(!terrain_materialized_rows_match(
        &coverage.identity.map,
        projections.vmap_rows
    ));
    assert!(vmap_materialized_rows_match(
        &coverage.identity.map,
        projections.vmap_rows
    ));
    assert!(!vmap_materialized_rows_match(
        &coverage.identity.map,
        projections.terrain_rows
    ));
}

#[test]
fn map_snapshot_commits_ordered_zero_length_available_and_encrypted_ranges() {
    use forever_casc::blte::available::{CoverageRange, RangeState};

    let data = [0x12, 0x34, 0x56];
    let ranges = [
        CoverageRange {
            offset: 0,
            bytes: 0,
            state: RangeState::Available,
        },
        CoverageRange {
            offset: 0,
            bytes: 0,
            state: RangeState::EncryptedUnavailable,
        },
        CoverageRange {
            offset: 0,
            bytes: 3,
            state: RangeState::Available,
        },
        CoverageRange {
            offset: 3,
            bytes: 2,
            state: RangeState::EncryptedUnavailable,
        },
    ];
    let mut calls = Vec::new();
    let snapshot = append_map_range_snapshot(5, 3, false, true, &ranges, |offset, len| {
        calls.push((offset, len));
        let start = usize::try_from(offset).map_err(|_| Error::from("test_range"))?;
        let end = start
            .checked_add(len)
            .ok_or_else(|| Error::from("test_range"))?;
        data.get(start..end)
            .ok_or_else(|| Error::from("test_range"))
    })
    .unwrap();
    assert_eq!(calls, [(0, 3)]);

    let reordered = [ranges[1], ranges[0], ranges[2], ranges[3]];
    let reordered_snapshot =
        append_map_range_snapshot(5, 3, false, true, &reordered, |offset, len| {
            let start = usize::try_from(offset).map_err(|_| Error::from("test_range"))?;
            let end = start
                .checked_add(len)
                .ok_or_else(|| Error::from("test_range"))?;
            data.get(start..end)
                .ok_or_else(|| Error::from("test_range"))
        })
        .unwrap();
    assert_ne!(
        integrity::digest(&snapshot),
        integrity::digest(&reordered_snapshot)
    );

    let without_empty_frames =
        append_map_range_snapshot(5, 3, false, true, &[ranges[2], ranges[3]], |offset, len| {
            let start = usize::try_from(offset).map_err(|_| Error::from("test_range"))?;
            let end = start
                .checked_add(len)
                .ok_or_else(|| Error::from("test_range"))?;
            data.get(start..end)
                .ok_or_else(|| Error::from("test_range"))
        })
        .unwrap();
    assert_ne!(
        integrity::digest(&snapshot),
        integrity::digest(&without_empty_frames)
    );
}
