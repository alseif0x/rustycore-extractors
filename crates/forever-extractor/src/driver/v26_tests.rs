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

/// Small schema-independent WDC5: a declared TACT table has a zero-filled
/// record+ID extent; a complete table has one ordinary record+ID extent.
fn db2(tact: bool) -> Vec<u8> {
    let file_offset = 244 + usize::from(tact) * 4;
    let mut bytes = vec![0; 244];
    bytes[..4].copy_from_slice(b"WDC5");
    for (offset, value) in [
        (4, 5),
        (136, 1),
        (140, 0),
        (144, 1),
        (148, 0),
        (176, 0),
        (200, 1),
        (204 + 8, file_offset as u32),
        (204 + 12, 1),
        (204 + 24, 4),
    ] {
        put32(&mut bytes, offset, value);
    }
    if tact {
        bytes[204] = 1;
        bytes.extend_from_slice(&0u32.to_le_bytes()); // zero encrypted-ID count
    }
    bytes.push(if tact { 0 } else { 0xA5 });
    bytes.extend_from_slice(&[0; 4]); // section ID table
    bytes
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
    let complete = db2(false);
    let frame = forever_casc::blte::available::PartialSection {
        lookup_hex: "0123456789abcdef".into(),
        frame_ordinal: 3,
        decoded_size: 5,
    };

    publish_v26_db2_table(
        &plan,
        &plan.db2[0],
        &partial,
        vec![frame],
        &mut run,
        &mut section_total,
    )
    .unwrap();
    publish_v26_db2_table(
        &plan,
        &plan.db2[1],
        &complete,
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
