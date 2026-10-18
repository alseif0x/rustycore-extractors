// SPDX-License-Identifier: GPL-3.0-or-later
//! Synthetic plan and option admission only; no client or key input is read.
#![cfg(target_os = "linux")]
use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Lab(PathBuf);

impl Lab {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "rce-product16-cli-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn plan(&self) -> PathBuf {
        let path = self.0.join("plan.tsv");
        fs::write(&path, PLAN).unwrap();
        path
    }
}

impl Drop for Lab {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

const PLAN: &str =
    include_str!("../../../tests/contracts/fixtures/forever-product-v16/disabled.tsv");

fn cli(args: &[&str], plan: &PathBuf) -> Output {
    Command::new(env!("CARGO_BIN_EXE_forever-extractor"))
        .args(args)
        .arg(plan)
        .output()
        .unwrap()
}

#[test]
fn v16_help_and_plan_only_inspection_report_the_exact_db2_identity() {
    let help = Command::new(env!("CARGO_BIN_EXE_forever-extractor"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(help.status.success());
    assert!(
        String::from_utf8(help.stdout)
            .unwrap()
            .contains("Linux v16 separately selects disabled-key 70450 DB2-only")
    );

    let lab = Lab::new();
    let plan = lab.plan();
    let result = cli(&["inspect", "--plan"], &plan);
    assert!(result.status.success());
    let text = String::from_utf8(result.stdout).unwrap();
    assert!(text.contains("\"contract\":\"forever-product-v16\""));
    assert!(text.contains("\"build\":70450"));
    assert!(text.contains("\"components\":1"));
    assert!(text.contains("\"schema_catalog\":\"wowdbdefs-70450-v1\""));
    assert!(text.contains("\"decryption\":\"disabled\""));
    assert!(text.contains("\"full_extractor_complete\":false"));
}

#[test]
fn supplied_keys_for_v16_are_refused_before_client_or_key_input() {
    let lab = Lab::new();
    let plan = lab.plan();
    let output = lab.0.join("output");
    let result = Command::new(env!("CARGO_BIN_EXE_forever-extractor"))
        .args(["extract", "--plan"])
        .arg(&plan)
        .args(["--client", "/nonexistent-v16-client", "--output"])
        .arg(&output)
        .args(["--keys", "/nonexistent-v16-keys"])
        .output()
        .unwrap();
    let text = String::from_utf8(result.stderr).unwrap();
    assert_eq!(result.status.code(), Some(2));
    assert!(text.contains("product_keys_forbidden"), "{text}");
    assert!(text.contains("forever-product-v16"), "{text}");
    assert!(!output.exists());
}

#[test]
fn resident_v16_plan_refuses_scratch_before_client_or_output_io() {
    let lab = Lab::new();
    let plan = lab.plan();
    let output = lab.0.join("output");
    let result = Command::new(env!("CARGO_BIN_EXE_forever-extractor"))
        .args(["extract", "--plan"])
        .arg(&plan)
        .args(["--client", "/nonexistent-v16-client", "--output"])
        .arg(&output)
        .args(["--scratch", "/nonexistent-v16-scratch"])
        .output()
        .unwrap();
    let text = String::from_utf8(result.stderr).unwrap();
    assert_eq!(result.status.code(), Some(2));
    assert!(text.contains("product_scratch_forbidden"), "{text}");
    assert!(text.contains("forever-product-v16"), "{text}");
    assert!(!output.exists());
}

#[test]
fn verification_commands_do_not_accept_a_keys_option() {
    for command in ["verify", "verify-links"] {
        let lab = Lab::new();
        let plan = lab.plan();
        let output = lab.0.join("nonexistent-output");
        let result = Command::new(env!("CARGO_BIN_EXE_forever-extractor"))
            .args([command, "--plan"])
            .arg(&plan)
            .args(["--output"])
            .arg(&output)
            .args(["--keys", "/nonexistent-v16-keys"])
            .output()
            .unwrap();
        let text = String::from_utf8(result.stderr).unwrap();
        assert_eq!(result.status.code(), Some(2), "{command}: {text}");
        assert!(text.contains("product_usage"), "{command}: {text}");
        assert!(!output.exists());
    }
}
