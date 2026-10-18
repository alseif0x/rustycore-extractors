// SPDX-License-Identifier: GPL-3.0-or-later
//! Synthetic plans and supplied-key text only; no client or network access.
#![cfg(target_os = "linux")]
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT: AtomicU64 = AtomicU64::new(0);
const TEST_KEY_NAME: u64 = 0x0102_0304_0506_0708;
const TEST_KEY: [u8; 16] = [0x42; 16];

struct Lab(PathBuf);

impl Lab {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "rce-product18-cli-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn plan(&self, name: &str, contents: &str) -> PathBuf {
        let path = self.0.join(name);
        fs::write(&path, contents).unwrap();
        path
    }

    fn keys(&self) -> PathBuf {
        let key_hex: String = TEST_KEY.iter().map(|byte| format!("{byte:02x}")).collect();
        let path = self.0.join("synthetic-keys.tsv");
        fs::write(
            &path,
            format!("forever-tact-keys\t1\n{TEST_KEY_NAME:016x}\t{key_hex}\n"),
        )
        .unwrap();
        path
    }

    fn scratch(&self) -> PathBuf {
        let path = self.0.join("private-scratch");
        fs::create_dir(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        path
    }
}

impl Drop for Lab {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

const V18: &str =
    include_str!("../../../tests/contracts/fixtures/forever-product-v18/all-available-maps.tsv");
const V14: &str =
    include_str!("../../../tests/contracts/fixtures/forever-product-v14/all-available-maps.tsv");
const V15: &str =
    include_str!("../../../tests/contracts/fixtures/forever-product-v15/all-available-maps.tsv");
const V16: &str =
    include_str!("../../../tests/contracts/fixtures/forever-product-v16/disabled.tsv");
const V17: &str =
    include_str!("../../../tests/contracts/fixtures/forever-product-v17/provided.tsv");

fn command(verb: &str, plan: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_forever-extractor"));
    command.args([verb, "--plan"]).arg(plan);
    command
}

fn output_text(output: Output) -> (String, String) {
    (
        String::from_utf8(output.stdout).unwrap(),
        String::from_utf8(output.stderr).unwrap(),
    )
}

fn assert_no_key_material(text: &str, key_path: &Path) {
    let key_hex: String = TEST_KEY.iter().map(|byte| format!("{byte:02x}")).collect();
    assert!(!text.contains(&key_hex));
    assert!(!text.contains(&format!("{TEST_KEY_NAME:016x}")));
    assert!(!text.contains(key_path.to_string_lossy().as_ref()));
}

#[test]
fn plan_only_inspection_reports_v18_collision_identity_without_key_input() {
    let lab = Lab::new();
    let plan = lab.plan("v18.tsv", V18);
    let result = command("inspect", &plan).output().unwrap();
    assert!(result.status.success());
    let (stdout, _) = output_text(result);
    assert!(stdout.contains("\"contract\":\"forever-product-v18\""));
    assert!(stdout.contains("\"build\":70450"));
    assert!(stdout.contains("\"components\":16"));
    assert!(stdout.contains("\"decryption\":\"provided-salsa20-v1\""));
    assert!(stdout.contains("\"collision_storage\":\"linux-leased-append-disk-world-v1\""));
    assert!(stdout.contains("\"collision_source_profile\":\"linux70450-compatible-gnu15-v2\""));
    assert!(stdout.contains("\"collision_domain\":\"all-available-maps\""));
    assert!(stdout.contains("\"full_extractor_complete\":false"));
}

#[test]
fn v18_client_inspect_extract_and_resume_require_keys_before_client_access() {
    for verb in ["inspect", "extract", "resume"] {
        let lab = Lab::new();
        let plan = lab.plan("v18.tsv", V18);
        let output_path = lab.0.join("must-not-be-created");
        let mut command = command(verb, &plan);
        command
            .args(["--client"])
            .arg(lab.0.join("synthetic-client-not-created"));
        if verb != "inspect" {
            command
                .args(["--output"])
                .arg(&output_path)
                .args(["--scratch"])
                .arg(lab.0.join("scratch-not-created"));
        }
        let result = command.output().unwrap();
        assert_eq!(result.status.code(), Some(2), "{verb}");
        let (_, stderr) = output_text(result);
        assert!(stderr.contains("product_keys_required"), "{verb}: {stderr}");
        assert!(stderr.contains("forever-product-v18"), "{verb}: {stderr}");
        assert!(!output_path.exists());
    }
}

#[test]
fn v18_extract_and_resume_require_scratch() {
    for verb in ["extract", "resume"] {
        let lab = Lab::new();
        let plan = lab.plan("v18.tsv", V18);
        let output_path = lab.0.join("scratch-required-output");
        let result = command(verb, &plan)
            .args(["--client"])
            .arg(lab.0.join("synthetic-client-not-created"))
            .args(["--output"])
            .arg(&output_path)
            .args(["--keys"])
            .arg(lab.0.join("synthetic-key-file-not-read"))
            .output()
            .unwrap();
        assert_eq!(result.status.code(), Some(2), "{verb}");
        let (_, stderr) = output_text(result);
        assert!(
            stderr.contains("product_scratch_required"),
            "{verb}: {stderr}"
        );
        assert!(!output_path.exists());
    }
}

#[test]
fn revisions_14_to_16_forbid_keys_while_v17_still_requires_them() {
    for (revision, contents, scratch_required) in
        [(14, V14, true), (15, V15, true), (16, V16, false)]
    {
        let lab = Lab::new();
        let plan = lab.plan(&format!("v{revision}.tsv"), contents);
        let output_path = lab.0.join("disabled-key-output");
        let mut command = command("extract", &plan);
        command
            .args(["--client"])
            .arg(lab.0.join("synthetic-client-not-created"))
            .args(["--output"])
            .arg(&output_path);
        if scratch_required {
            command
                .args(["--scratch"])
                .arg(lab.0.join("scratch-not-created"));
        }
        let result = command
            .args(["--keys"])
            .arg(lab.0.join("synthetic-keys-not-created"))
            .output()
            .unwrap();
        assert_eq!(result.status.code(), Some(2), "v{revision}");
        let (_, stderr) = output_text(result);
        assert!(
            stderr.contains("product_keys_forbidden"),
            "v{revision}: {stderr}"
        );
        assert!(stderr.contains(&format!("forever-product-v{revision}")));
        assert!(!output_path.exists());
    }

    let lab = Lab::new();
    let plan = lab.plan("v17.tsv", V17);
    let result = command("inspect", &plan)
        .args(["--client"])
        .arg(lab.0.join("synthetic-client-not-created"))
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(2));
    let (_, stderr) = output_text(result);
    assert!(stderr.contains("product_keys_required"), "{stderr}");
    assert!(stderr.contains("forever-product-v17"), "{stderr}");
}

#[test]
fn supplied_synthetic_key_is_accepted_but_not_echoed_in_output_record_or_error() {
    let lab = Lab::new();
    let plan = lab.plan("v18.tsv", V18);
    let key_path = lab.keys();
    let scratch = lab.scratch();
    let output_path = lab.0.join("failed-before-client-output");
    let diagnostic = lab.0.join("diagnostic.jsonl");

    let inspect = command("inspect", &plan)
        .args(["--client"])
        .arg(lab.0.join("synthetic-client-not-created"))
        .args(["--keys"])
        .arg(&key_path)
        .output()
        .unwrap();
    assert!(!inspect.status.success());
    let (stdout, stderr) = output_text(inspect);
    assert!(!stderr.contains("product_keys_forbidden"), "{stderr}");
    assert_no_key_material(&stdout, &key_path);
    assert_no_key_material(&stderr, &key_path);

    for verb in ["extract", "resume"] {
        let result = command(verb, &plan)
            .args(["--client"])
            .arg(lab.0.join("synthetic-client-not-created"))
            .args(["--output"])
            .arg(&output_path)
            .args(["--scratch"])
            .arg(&scratch)
            .args(["--keys"])
            .arg(&key_path)
            .env("FOREVER_PRODUCT_DIAGNOSTIC", &diagnostic)
            .output()
            .unwrap();
        assert!(!result.status.success(), "{verb}");
        let (stdout, stderr) = output_text(result);
        assert!(
            !stderr.contains("product_keys_forbidden"),
            "{verb}: {stderr}"
        );
        assert_no_key_material(&stdout, &key_path);
        assert_no_key_material(&stderr, &key_path);
        assert!(!output_path.exists());
        assert!(!diagnostic.exists());
    }
}

#[test]
fn verify_commands_reject_the_keys_option() {
    for verb in ["verify", "verify-links"] {
        let lab = Lab::new();
        let plan = lab.plan("v18.tsv", V18);
        let result = command(verb, &plan)
            .args(["--output"])
            .arg(lab.0.join("output-not-created"))
            .args(["--keys"])
            .arg(lab.0.join("synthetic-keys-not-created"))
            .output()
            .unwrap();
        assert_eq!(result.status.code(), Some(2), "{verb}");
        let (_, stderr) = output_text(result);
        assert!(stderr.contains("product_usage"), "{verb}: {stderr}");
    }
}
