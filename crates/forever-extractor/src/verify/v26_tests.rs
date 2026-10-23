// SPDX-License-Identifier: GPL-3.0-or-later
//! Synthetic build-70450 per-table DB2 declaration verification only.
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
            "rce-v26-verify-synthetic-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        Self(root)
    }

    fn output(&self) -> PathBuf {
        self.0.join("package")
    }

    fn publish(
        &self,
        plan: &Plan,
        files: &[(String, Vec<u8>)],
        tables: &[package::PartialEncryptedTable],
    ) {
        let mut run =
            package::Run::create(&self.output(), context(plan), &[], Default::default()).unwrap();
        for table in tables {
            run.set_partial_encrypted_table(&table.path, &table.sections)
                .unwrap();
        }
        for (path, bytes) in files {
            run.put(Component::Db2, path, bytes).unwrap();
        }
        run.finish_component(Component::Db2).unwrap();
        run.finish().unwrap();
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

fn put32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

/// A bounded one-section WDC5 structure, either TACT zero-filled or complete.
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

fn files(plan: &Plan, first: Vec<u8>, second: Vec<u8>) -> Vec<(String, Vec<u8>)> {
    vec![
        (
            format!("dbc/{}/{}.db2", plan.locale.name(), plan.db2[0].name),
            first,
        ),
        (
            format!("dbc/{}/{}.db2", plan.locale.name(), plan.db2[1].name),
            second,
        ),
    ]
}

fn section() -> package::PartialEncryptedSection {
    package::PartialEncryptedSection {
        lookup: "0123456789abcdef".into(),
        frame_ordinal: 3,
        decoded_size: 5,
    }
}

fn partial_table(path: String) -> package::PartialEncryptedTable {
    package::PartialEncryptedTable {
        path,
        sections: vec![section()],
    }
}

fn valid_tables(plan: &Plan) -> Vec<package::PartialEncryptedTable> {
    vec![partial_table(format!(
        "dbc/{}/{}.db2",
        plan.locale.name(),
        plan.db2[0].name
    ))]
}

#[test]
fn v26_verification_accepts_table_scoped_partial_and_complete_db2() {
    let lab = Lab::new();
    let plan = plan();
    let files = files(&plan, db2(true), db2(false));
    lab.publish(&plan, &files, &valid_tables(&plan));

    let summary = verify(&plan, &lab.output(), Some(context(&plan))).unwrap();
    assert_eq!(summary.files, 2);
}

#[test]
fn v26_verification_rejects_nonzero_tact_payload() {
    let lab = Lab::new();
    let plan = plan();
    let mut partial = db2(true);
    partial[248] = 1;
    lab.publish(
        &plan,
        &files(&plan, partial, db2(false)),
        &valid_tables(&plan),
    );

    assert_eq!(
        verify(&plan, &lab.output(), None).unwrap_err().code,
        "product_partial_zerofill"
    );
}

#[test]
fn v26_verification_rejects_missing_table_declaration() {
    let lab = Lab::new();
    let plan = plan();
    lab.publish(&plan, &files(&plan, db2(true), db2(false)), &[]);

    assert_eq!(
        verify(&plan, &lab.output(), None).unwrap_err().code,
        "product_partial_undeclared"
    );
}

#[test]
fn v26_verification_rejects_declaration_for_complete_table() {
    let lab = Lab::new();
    let plan = plan();
    let mut tables = valid_tables(&plan);
    tables.push(partial_table(format!(
        "dbc/{}/{}.db2",
        plan.locale.name(),
        plan.db2[1].name
    )));
    lab.publish(&plan, &files(&plan, db2(true), db2(false)), &tables);

    assert_eq!(
        verify(&plan, &lab.output(), None).unwrap_err().code,
        "product_partial_overdeclared"
    );
}

#[test]
fn v26_verification_rejects_manifest_declaration_without_its_db2_file() {
    let lab = Lab::new();
    let plan = plan();
    let mut tables = valid_tables(&plan);
    tables.push(partial_table(format!(
        "dbc/{}/NotSelected.db2",
        plan.locale.name()
    )));
    lab.publish(&plan, &files(&plan, db2(true), db2(false)), &tables);

    assert_eq!(
        verify(&plan, &lab.output(), None).unwrap_err().code,
        "product_partial_declared_file_missing"
    );
}

#[test]
fn v26_verification_rejects_missing_selected_db2_file() {
    let lab = Lab::new();
    let plan = plan();
    let only_first = vec![files(&plan, db2(true), db2(false)).remove(0)];
    lab.publish(&plan, &only_first, &valid_tables(&plan));

    assert_eq!(
        verify(&plan, &lab.output(), None).unwrap_err().code,
        "product_inventory"
    );
}
