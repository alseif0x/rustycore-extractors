// SPDX-License-Identifier: GPL-3.0-or-later
//! Synthetic inspect-db2 CLI refusals; no client, keys or network are read.
#![cfg(target_os = "linux")]

use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};

const V16: &str =
    include_str!("../../../tests/contracts/fixtures/forever-product-v16/disabled.tsv");
const V10: &str =
    include_str!("../../../tests/contracts/fixtures/forever-product-v10/disabled.tsv");
static NEXT: AtomicU64 = AtomicU64::new(0);

struct Lab(PathBuf);
impl Lab {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "rce-inspect-db2-cli-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
    fn plan(&self, name: &str, contents: &str) -> PathBuf {
        let path = self.0.join(name);
        fs::write(&path, contents).unwrap();
        path
    }
}
impl Drop for Lab {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn inspect(plan: &PathBuf, client: &PathBuf, output: &PathBuf, extra: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_forever-extractor"))
        .args(["inspect-db2", "--plan"])
        .arg(plan)
        .args(["--client"])
        .arg(client)
        .args(["--output"])
        .arg(output)
        .args(extra)
        .output()
        .unwrap()
}

fn error_code(output: &Output, code: &str, exit: i32) {
    assert_eq!(output.status.code(), Some(exit));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(code),
        "expected {code}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn inspect_db2_help_documents_keyless_report_only_scope() {
    let result = Command::new(env!("CARGO_BIN_EXE_forever-extractor"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(result.status.success());
    let help = String::from_utf8(result.stdout).unwrap();
    assert!(help.contains("forever-extractor inspect-db2 --plan PLAN"));
    assert!(help.contains("extracts nothing; --keys is forbidden"));
}

#[test]
fn inspect_db2_refuses_keys_and_non_70450_plans_before_client_access() {
    let lab = Lab::new();
    let v16 = lab.plan("v16.tsv", V16);
    let v10 = lab.plan("v10.tsv", V10);
    let missing_client = lab.0.join("client-not-opened");
    let output = lab.0.join("report-not-created.json");
    let key_path = lab.0.join("keys-not-opened");

    let keys = inspect(
        &v16,
        &missing_client,
        &output,
        &[
            "--file-data-id",
            "5152908",
            "--keys",
            key_path.to_str().unwrap(),
        ],
    );
    error_code(&keys, "product_keys_forbidden", 2);
    assert!(!key_path.exists());
    assert!(!output.exists());

    let foreign = inspect(
        &v10,
        &missing_client,
        &output,
        &["--file-data-id", "5152908"],
    );
    error_code(&foreign, "product_db2_inspection_profile", 2);
    assert!(!output.exists());
}

#[test]
fn inspect_db2_refuses_client_and_git_checkout_output_paths() {
    let lab = Lab::new();
    let plan = lab.plan("v16.tsv", V16);
    let client = lab.0.join("client");
    fs::create_dir_all(client.join("reports")).unwrap();
    let in_client = client.join("reports/inspection.json");
    let client_result = inspect(&plan, &client, &in_client, &["--file-data-id", "5152908"]);
    error_code(&client_result, "product_diagnostic_path", 9);
    assert!(!in_client.exists());

    let checkout = lab.0.join("checkout");
    fs::create_dir_all(checkout.join(".git")).unwrap();
    fs::create_dir_all(checkout.join("reports")).unwrap();
    let in_checkout = checkout.join("reports/inspection.json");
    let checkout_result = inspect(&plan, &client, &in_checkout, &["--file-data-id", "5152908"]);
    error_code(&checkout_result, "product_diagnostic_path", 9);
    assert!(!in_checkout.exists());
}
