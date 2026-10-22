// SPDX-License-Identifier: GPL-3.0-or-later
//! Synthetic plan-only v25 admission; no client, keys or network are used.
#![cfg(target_os = "linux")]

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};

const PLAN: &str = include_str!("../../../tests/contracts/fixtures/forever-product-v25/map2.tsv");
static NEXT: AtomicU64 = AtomicU64::new(0);

struct Lab(PathBuf);
impl Lab {
    fn new() -> Self {
        let leaf = format!(
            "rce-v25-plan-cli-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        );
        let root = std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join(".local/share")
            .join(leaf);
        fs::create_dir_all(root.parent().unwrap()).unwrap();
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        Self(root)
    }
    fn plan(&self, name: &str, contents: &str) -> PathBuf {
        let path = self.0.join(name);
        fs::write(&path, contents).unwrap();
        path
    }
    fn command(&self, verb: &str, plan: &Path) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_forever-extractor"));
        command
            .current_dir(&self.0)
            .args([verb, "--plan"])
            .arg(plan);
        command
    }
}
impl Drop for Lab {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn assert_code(output: Output, code: &str) {
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains(&format!("\"code\":\"{code}\"")), "{stderr}");
}

#[test]
fn v25_plan_inspection_reports_partial_scope_without_qualified_claims() {
    let lab = Lab::new();
    let plan = lab.plan("plan.tsv", PLAN);
    let output = lab.command("inspect", &plan).output().unwrap();
    assert!(output.status.success());
    let report = String::from_utf8(output.stdout).unwrap();
    for expected in [
        "\"contract\":\"forever-product-v25\"",
        "\"status\":\"planned\"",
        "\"components\":9",
        "\"partial_encrypted_sections\":\"declared-skip\"",
        "\"verification_coverage_authentication\":\"independently_derived\"",
        "\"whole_client_map_coverage\":false",
        "\"full_extractor_complete\":false",
    ] {
        assert!(report.contains(expected), "missing {expected}: {report}");
    }
    assert!(!report.contains("complete_source_decode_verified"));
}

#[test]
fn v25_rejects_keys_and_wrong_or_cross_revision_partial_declarations_before_io() {
    let lab = Lab::new();
    let plan = lab.plan("plan.tsv", PLAN);
    let output = lab.0.join("no-output");
    assert_code(
        lab.command("extract", &plan)
            .args(["--client", "missing-client", "--output"])
            .arg(&output)
            .args(["--keys", "missing-keys"])
            .output()
            .unwrap(),
        "product_keys_forbidden",
    );
    assert!(!output.exists());

    let wrong = lab.plan(
        "wrong.tsv",
        &PLAN.replace(
            "partial_encrypted_sections\tdeclared-skip",
            "partial_encrypted_sections\tallow",
        ),
    );
    assert_code(
        lab.command("inspect", &wrong).output().unwrap(),
        "product_plan_partial_encrypted_sections",
    );

    let v24_with_field = lab.plan(
        "v24-with-field.tsv",
        &PLAN
            .replace("forever-product\t25", "forever-product\t24")
            .replace("partial_encrypted_sections\tdeclared-skip\n", ""),
    );
    let with_field = lab.plan(
        "v24-extra-field.tsv",
        &fs::read_to_string(&v24_with_field).unwrap().replace(
            "decryption\tdisabled\n",
            "decryption\tdisabled\npartial_encrypted_sections\tdeclared-skip\n",
        ),
    );
    assert_code(
        lab.command("inspect", &with_field).output().unwrap(),
        "product_plan_format",
    );
}
