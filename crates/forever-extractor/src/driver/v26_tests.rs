// SPDX-License-Identifier: GPL-3.0-or-later
//! Synthetic driver/package coverage for build-70450 DB2 partial declarations.
//! No client acquisition, comparison, full-extraction or runtime claim.
use super::*;
use crate::plan::Plan;
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Lab(PathBuf);
impl Lab {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "rce-v26-driver-synthetic-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        Self(root)
    }

    fn output(&self) -> PathBuf {
        self.0.join("package")
    }
}
impl Drop for Lab {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn plan() -> Plan {
    Plan::parse(include_str!(
        "../../../../tests/contracts/fixtures/forever-product-v26/multiple.tsv"
    ))
    .unwrap()
}

fn put32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}
fn put16(bytes: &mut [u8], offset: usize, value: u16) {
    bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

/// Small schema-independent sparse WDC5 with a raw catalog record and parent
/// lookup. A TACT record extent is zero-filled; a complete one stays ordinary.
fn db2(tact: bool) -> Vec<u8> {
    let file_offset = 244 + usize::from(tact) * 4;
    let catalog_offset = file_offset + 1;
    let mut bytes = vec![0; 244];
    bytes[..4].copy_from_slice(b"WDC5");
    for (offset, value) in [
        (4, 5),
        (136, 1),
        (140, 0),
        (144, 1),
        (148, 0),
        (176, 0),
        (184, 1),
        (200, 1),
        (204 + 8, file_offset as u32),
        (204 + 12, 1),
        (204 + 20, catalog_offset as u32),
        (204 + 24, 4),
        (204 + 28, 20),
        (204 + 32, 1),
    ] {
        put32(&mut bytes, offset, value);
    }
    put16(&mut bytes, 172, 1); // sparse catalog framing
    if tact {
        bytes[204] = 1;
        bytes.extend_from_slice(&0u32.to_le_bytes()); // zero encrypted-ID count
    }
    bytes.push(if tact { 0 } else { 0xA5 });
    if tact {
        bytes.resize(bytes.len() + 10 + 4 + 20, 0);
    } else {
        bytes.extend_from_slice(&7u32.to_le_bytes()); // catalog ID list
        bytes.extend_from_slice(&(file_offset as u32).to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes()); // raw catalog record size
        bytes.extend_from_slice(&7u32.to_le_bytes()); // section ID table
        bytes.extend_from_slice(&1u32.to_le_bytes()); // parent lookup entry count
        bytes.extend_from_slice(&1u32.to_le_bytes()); // parent minimum ID
        bytes.extend_from_slice(&2u32.to_le_bytes()); // parent maximum ID
        bytes.extend_from_slice(&2u32.to_le_bytes()); // parent ID
        bytes.extend_from_slice(&0u32.to_le_bytes()); // record ordinal
    }
    bytes
}

fn complete_db2_with_unadmitted_catalog_bounds() -> Vec<u8> {
    let mut bytes = db2(false);
    let catalog = u32::from_le_bytes(bytes[204 + 20..204 + 24].try_into().unwrap()) as usize;
    put32(&mut bytes, catalog + 4, catalog as u32);
    assert_eq!(
        forever_db2::availability::read_structure_70450(&CompleteDb2Source(&bytes)).err(),
        Some(RecordError::Bounds)
    );
    bytes
}

fn encrypted_range(offset: u64, bytes: u64) -> forever_casc::blte::available::CoverageRange {
    forever_casc::blte::available::CoverageRange {
        offset,
        bytes,
        state: forever_casc::blte::available::RangeState::EncryptedUnavailable,
    }
}

fn empty_available_range() -> forever_casc::blte::available::CoverageRange {
    forever_casc::blte::available::CoverageRange {
        offset: 0,
        bytes: 0,
        state: forever_casc::blte::available::RangeState::Available,
    }
}

fn frames_for(
    coverage: &[forever_casc::blte::available::CoverageRange],
) -> Vec<forever_casc::blte::available::PartialSection> {
    coverage
        .iter()
        .enumerate()
        .filter(|(_, range)| {
            range.state == forever_casc::blte::available::RangeState::EncryptedUnavailable
        })
        .map(
            |(ordinal, range)| forever_casc::blte::available::PartialSection {
                lookup_hex: "0123456789abcdef".into(),
                frame_ordinal: ordinal as u32,
                decoded_size: range.bytes,
            },
        )
        .collect()
}

fn structural(bytes: &[u8]) -> forever_db2::availability::StructuralFacts {
    forever_db2::availability::read_structure_70450_for_table(
        &CompleteDb2Source(bytes),
        "AccountStoreCategory",
    )
    .unwrap()
}

fn context(plan: &Plan) -> Context {
    Context {
        build: plan.build,
        locale: plan.locale,
        components: plan.components,
        source_sha256: [1; 32],
        plan_sha256: integrity::digest(plan.render().as_bytes()),
        producer_sha256: [2; 32],
    }
}

#[test]
fn v26_driver_writes_partial_and_complete_tables_with_per_table_manifest() {
    let lab = Lab::new();
    let plan = plan();
    plan.validate().unwrap();
    let package_context = context(&plan);
    let mut run = Run::create(&lab.output(), package_context, &[], Default::default()).unwrap();
    let mut section_total = 0;
    let partial = db2(true);
    let complete = complete_db2_with_unadmitted_catalog_bounds();
    let coverage = vec![
        empty_available_range(),
        empty_available_range(),
        empty_available_range(),
        encrypted_range(248, 35),
    ];
    let frames = frames_for(&coverage);

    publish_v26_db2_table(
        &plan,
        &plan.db2[0],
        &partial,
        &coverage,
        frames,
        &mut run,
        &mut section_total,
    )
    .unwrap();
    publish_v26_db2_table(
        &plan,
        &plan.db2[1],
        &complete,
        &[],
        Vec::new(),
        &mut run,
        &mut section_total,
    )
    .unwrap();
    run.finish_component(Component::Db2).unwrap();
    let summary = run.finish().unwrap();

    assert_eq!(section_total, 1);
    assert_eq!(summary.files, 2);
    assert_eq!(
        fs::read(lab.output().join("dbc/enUS/AccountStoreCategory.db2")).unwrap(),
        partial
    );
    assert_eq!(
        fs::read(lab.output().join("dbc/enUS/AnimationData.db2")).unwrap(),
        complete
    );
    let manifest = fs::read_to_string(lab.output().join(package::MANIFEST)).unwrap();
    let header = manifest.lines().next().unwrap();
    assert!(header.contains("\"partial_encrypted_tables\":["));
    assert!(header.contains("dbc/enUS/AccountStoreCategory.db2"));
    assert!(!header.contains("dbc/enUS/AnimationData.db2"));
    assert!(!header.contains("\"partial_encrypted_sections\":"));
    package::verify(&lab.output(), Some(package_context), Default::default()).unwrap();
}

#[test]
fn v26_partial_coverage_tiles_multiple_frames_and_rejects_gaps() {
    let partial = db2(true);
    let facts = structural(&partial);
    let split = vec![encrypted_range(248, 17), encrypted_range(265, 18)];
    assert!(validate_v26_partial_coverage(&facts, &split, &frames_for(&split)).is_ok());

    for coverage in [
        vec![encrypted_range(248, 17), encrypted_range(265, 19)],
        vec![encrypted_range(248, 17)],
        vec![
            encrypted_range(248, 17),
            encrypted_range(265, 18),
            encrypted_range(283, 1),
        ],
    ] {
        assert!(validate_v26_partial_coverage(&facts, &coverage, &frames_for(&coverage)).is_err());
    }

    let mut bad_metadata = frames_for(&split);
    bad_metadata[0].frame_ordinal = 1;
    assert!(validate_v26_partial_coverage(&facts, &split, &bad_metadata).is_err());
}

#[test]
fn v27_map_uses_only_multi_frame_per_table_declarations() {
    let mut product = Plan::parse(include_str!(
        "../../../../tests/contracts/fixtures/forever-product-v25/map2.tsv"
    ))
    .unwrap();
    product.publication_profile =
        crate::plan::PublicationProfile::Build70450PartialCatalogTerrainV1;
    product.validate().unwrap();
    assert!(product.is_v27());

    let lab = Lab::new();
    let mut run = Run::create(
        &lab.output(),
        context(&product),
        &[],
        terrain_v20::package_limits_v27(),
    )
    .unwrap();
    let partial = db2(true);
    let facts = forever_db2::availability::read_structure_70450_for_table(
        &CompleteDb2Source(&partial),
        "Map",
    )
    .unwrap();
    let coverage = vec![encrypted_range(248, 17), encrypted_range(265, 18)];
    assert!(validate_v26_partial_coverage(&facts, &coverage, &frames_for(&coverage)).is_ok());
    let mut bad_frames = frames_for(&coverage);
    bad_frames[0].decoded_size -= 1;
    assert!(validate_v26_partial_coverage(&facts, &coverage, &bad_frames).is_err());

    let mut section_total = 0;
    publish_v26_db2_table(
        &product,
        &product.db2[0],
        &partial,
        &coverage,
        frames_for(&coverage),
        &mut run,
        &mut section_total,
    )
    .unwrap();
    run.finish_component(Component::Db2).unwrap();
    run.finish_component(Component::Terrain).unwrap();
    let summary = run.finish().unwrap();

    assert_eq!(section_total, 2);
    assert_eq!(summary.files, 1);
    assert_eq!(
        fs::read(lab.output().join("dbc/enUS/Map.db2")).unwrap(),
        partial
    );
    let manifest = fs::read_to_string(lab.output().join(package::MANIFEST)).unwrap();
    assert!(manifest.contains("\"partial_encrypted_tables\":[{"));
    assert!(manifest.contains("dbc/enUS/Map.db2"));
    assert!(manifest.contains("\"decoded_size\":17"));
    assert!(manifest.contains("\"decoded_size\":18"));
    assert!(!manifest.contains("\"partial_encrypted_sections\":"));
}

#[test]
fn v27_preserves_db2_budgets_and_adds_them_to_the_terrain_package_budget() {
    let product = Plan::parse(include_str!(
        "../../../../tests/contracts/fixtures/forever-product-v27/partial.tsv"
    ))
    .unwrap();
    assert_eq!(
        db2_read_limit(&product, DB2_TOTAL_BYTES - 4096).unwrap(),
        4096
    );
    assert_eq!(
        terrain_v20::package_limits_v24().total_bytes,
        64 * 1024 * 1024 * 1024
    );
    assert_eq!(
        terrain_v20::package_limits_v27().total_bytes,
        80 * 1024 * 1024 * 1024
    );
}
