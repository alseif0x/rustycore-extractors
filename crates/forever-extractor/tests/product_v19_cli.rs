// SPDX-License-Identifier: GPL-3.0-or-later
//! Synthetic plans and authored missing paths only; no client, keys, or network.
#![cfg(target_os = "linux")]
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Lab(PathBuf);

impl Lab {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "rce-product19-cli-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        Self(root)
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

const V19: &str =
    include_str!("../../../tests/contracts/fixtures/forever-product-v19/disabled.tsv");
const V14: &str =
    include_str!("../../../tests/contracts/fixtures/forever-product-v14/all-available-maps.tsv");
const V15: &str =
    include_str!("../../../tests/contracts/fixtures/forever-product-v15/all-available-maps.tsv");
const V16: &str =
    include_str!("../../../tests/contracts/fixtures/forever-product-v16/disabled.tsv");
const V17: &str =
    include_str!("../../../tests/contracts/fixtures/forever-product-v17/provided.tsv");
const V18: &str =
    include_str!("../../../tests/contracts/fixtures/forever-product-v18/all-available-maps.tsv");

fn command(verb: &str, plan: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_forever-extractor"));
    command.args([verb, "--plan"]).arg(plan);
    command
}

fn stderr(output: Output) -> String {
    String::from_utf8(output.stderr).unwrap()
}

#[test]
fn v19_plan_only_inspection_reports_exact_triple_and_camera_catalogue() {
    let lab = Lab::new();
    let plan = lab.plan("v19.tsv", V19);
    let result = command("inspect", &plan).output().unwrap();
    assert!(result.status.success());
    let text = String::from_utf8(result.stdout).unwrap();
    assert!(text.contains("\"contract\":\"forever-product-v19\""));
    assert!(text.contains("\"build\":70450"));
    assert!(text.contains("\"components\":7"));
    assert!(text.contains("\"schema_catalog\":\"wowdbdefs-70450-v1\""));
    assert!(text.contains("\"decryption\":\"disabled\""));
    assert!(text.contains("\"full_extractor_complete\":false"));
    assert!(!text.contains("CinematicCamera"));
}

#[test]
fn disabled_v19_refuses_keys_before_client_key_or_output_path_access() {
    for verb in ["inspect", "extract", "resume"] {
        let lab = Lab::new();
        let plan = lab.plan("v19.tsv", V19);
        let client = lab.0.join("authored-client-70450");
        let key_path = lab.0.join("authored-key-list-deadbeef");
        let output = lab.0.join("must-not-be-created");
        let diagnostic = lab.0.join("must-not-be-created.jsonl");
        let mut command = command(verb, &plan);
        command
            .args(["--client"])
            .arg(&client)
            .args(["--keys"])
            .arg(&key_path)
            .env("FOREVER_PRODUCT_DIAGNOSTIC", &diagnostic);
        if verb != "inspect" {
            command.args(["--output"]).arg(&output);
        }
        let result = command.output().unwrap();
        assert_eq!(result.status.code(), Some(2), "{verb}");
        let error = stderr(result);
        assert!(error.contains("product_keys_forbidden"), "{verb}: {error}");
        assert!(error.contains("forever-product-v19"), "{verb}: {error}");
        assert!(!error.contains(key_path.to_string_lossy().as_ref()));
        assert!(!error.contains(client.to_string_lossy().as_ref()));
        assert!(!output.exists());
        assert!(!diagnostic.exists());
    }
}

#[test]
fn resident_v19_rejects_scratch_before_any_client_or_output_access() {
    for verb in ["extract", "resume"] {
        let lab = Lab::new();
        let plan = lab.plan("v19.tsv", V19);
        let output = lab.0.join("scratch-forbidden-output");
        let result = command(verb, &plan)
            .args(["--client"])
            .arg(lab.0.join("authored-client-not-opened"))
            .args(["--output"])
            .arg(&output)
            .args(["--scratch"])
            .arg(lab.0.join("authored-scratch-not-opened"))
            .output()
            .unwrap();
        assert_eq!(result.status.code(), Some(2), "{verb}");
        assert!(stderr(result).contains("product_scratch_forbidden"));
        assert!(!output.exists());
    }
}

#[test]
fn disabled_v19_rejects_each_geometry_or_map_domain_component_mask() {
    let lab = Lab::new();
    for components in [
        "db2",
        "gt",
        "cameras",
        "maps",
        "vmaps",
        "mmaps",
        "db2,gt",
        "db2,cameras",
        "gt,cameras",
        "db2,gt,cameras,vmaps",
    ] {
        let plan_text = V19.replace(
            "components\tdb2,gt,cameras",
            &format!("components\t{components}"),
        );
        let plan = lab.plan("wrong-mask.tsv", &plan_text);
        let result = command("inspect", &plan).output().unwrap();
        assert_eq!(result.status.code(), Some(2), "{components}");
        let error = stderr(result);
        assert!(
            error.contains("product_plan_component_scope"),
            "{components}: {error}"
        );
        assert!(
            error.contains("forever-product-v1"),
            "{components}: {error}"
        );
    }
    for (bad_field, replacement, code) in [
        ("build\t70450", "build\t70425", "product_plan_build"),
        (
            "schema-catalog\twowdbdefs-70450-v1",
            "schema-catalog\twowdbdefs-70425-v1",
            "product_plan_schema_catalog",
        ),
        (
            "decryption\tdisabled",
            "decryption\tprovided-salsa20-v1",
            "product_plan_component_scope",
        ),
        ("terrain\tnone", "terrain\tall", "product_plan_dependencies"),
        (
            "navigation\tnone",
            "navigation\tmap:0",
            "product_plan_dependencies",
        ),
    ] {
        let plan_text = V19.replace(bad_field, replacement);
        let plan = lab.plan("wrong-profile.tsv", &plan_text);
        let result = command("inspect", &plan).output().unwrap();
        assert_eq!(result.status.code(), Some(2), "{replacement}");
        assert!(stderr(result).contains(code), "{replacement}");
    }
}

#[test]
fn revisions_14_through_18_preserve_their_key_policies() {
    for (revision, fixture, scratch_required) in
        [(14, V14, true), (15, V15, true), (16, V16, false)]
    {
        let lab = Lab::new();
        let plan = lab.plan(&format!("v{revision}.tsv"), fixture);
        let output = lab.0.join("disabled-key-output");
        let mut command = command("extract", &plan);
        command
            .args(["--client"])
            .arg(lab.0.join("authored-client-not-opened"))
            .args(["--output"])
            .arg(&output);
        if scratch_required {
            command
                .args(["--scratch"])
                .arg(lab.0.join("authored-scratch-not-opened"));
        }
        let result = command
            .args(["--keys"])
            .arg(lab.0.join("authored-key-list-not-opened"))
            .output()
            .unwrap();
        assert_eq!(result.status.code(), Some(2), "v{revision}");
        assert!(
            stderr(result).contains("product_keys_forbidden"),
            "v{revision}"
        );
        assert!(!output.exists());
    }

    let lab = Lab::new();
    let plan = lab.plan("v17.tsv", V17);
    let result = command("inspect", &plan)
        .args(["--client"])
        .arg(lab.0.join("authored-client-not-opened"))
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(2));
    assert!(stderr(result).contains("product_keys_required"));

    let lab = Lab::new();
    let plan = lab.plan("v18.tsv", V18);
    let result = command("inspect", &plan)
        .args(["--client"])
        .arg(lab.0.join("authored-client-not-opened"))
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(2));
    assert!(stderr(result).contains("product_keys_required"));
}

#[test]
fn v19_readers_remain_synthetic_and_cli_has_no_key_or_client_payload_output() {
    // `driver::v19_reader_tests` exercises the shared GT/camera readers on
    // synthetic rows/WDC5; the CLI path here is admitted only up to input gating.
    let lab = Lab::new();
    let plan = lab.plan("v19.tsv", V19);
    let key_text = "synthetic-TACT-name-and-key-values-are-not-cli-output";
    let client = lab.0.join("authored-client-path-70450");
    let output = lab.0.join("uncreated-output-path");
    let result = command("inspect", &plan)
        .args(["--client"])
        .arg(&client)
        .args(["--keys"])
        .arg(lab.0.join(key_text))
        .output()
        .unwrap();
    assert!(!result.status.success());
    let stdout = String::from_utf8(result.stdout).unwrap();
    let error = String::from_utf8(result.stderr).unwrap();
    for diagnostic in [&stdout, &error] {
        assert!(!diagnostic.contains(key_text));
        assert!(!diagnostic.contains(client.to_string_lossy().as_ref()));
    }
    assert!(!output.exists());
}
