// SPDX-License-Identifier: GPL-3.0-or-later
//! Synthetic plan/CLI admission checks only; no client, keys or network.
#![cfg(target_os = "linux")]

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT: AtomicU64 = AtomicU64::new(0);
const MAP_PLAN: &str =
    include_str!("../../../tests/contracts/fixtures/forever-product-v20/map2.tsv");
const ALL_PLAN: &str =
    include_str!("../../../tests/contracts/fixtures/forever-product-v20/available.tsv");

struct Lab(PathBuf);
impl Lab {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "rce-product20-cli-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn plan(&self, name: &str, text: &str) -> PathBuf {
        let path = self.0.join(name);
        fs::write(&path, text).unwrap();
        path
    }
}
impl Drop for Lab {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn command(verb: &str, plan: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_forever-extractor"));
    command.args([verb, "--plan"]).arg(plan);
    command
}
fn stderr(output: Output) -> String {
    String::from_utf8(output.stderr).unwrap()
}

#[test]
fn v20_plan_only_inspection_reports_70450_partial_coverage_boundary() {
    for text in [MAP_PLAN, ALL_PLAN] {
        let lab = Lab::new();
        let plan = lab.plan("plan.tsv", text);
        let result = command("inspect", &plan).output().unwrap();
        assert!(result.status.success());
        let report = String::from_utf8(result.stdout).unwrap();
        assert!(report.contains("\"contract\":\"forever-product-v20\""));
        assert!(report.contains("\"build\":70450"));
        assert!(report.contains("\"components\":8"));
        assert!(report.contains("\"schema_catalog\":\"wowdbdefs-70450-v1\""));
        assert!(report.contains("\"decryption\":\"disabled\""));
        assert!(report.contains(
            "\"terrain_source_profile\":\"linux70450-compatible-full-maps-available-map-v1\""
        ));
        assert!(report.contains("\"coverage_authentication\":\"producer_reported\""));
        assert!(report.contains("\"whole_client_map_coverage\":false"));
        assert!(report.contains("\"full_extractor_complete\":false"));
        assert!(!report.contains("format_parity"));
    }
}

#[test]
fn disabled_v20_refuses_keys_before_client_or_output_path_access() {
    for verb in ["inspect", "extract", "resume"] {
        let lab = Lab::new();
        let plan = lab.plan("plan.tsv", MAP_PLAN);
        let client = lab.0.join("synthetic-client-not-opened");
        let keys = lab.0.join("synthetic-key-list-not-opened");
        let output = lab.0.join("output-must-not-exist");
        let mut cmd = command(verb, &plan);
        cmd.args(["--client"])
            .arg(&client)
            .args(["--keys"])
            .arg(&keys);
        if verb != "inspect" {
            cmd.args(["--output"]).arg(&output);
        }
        let result = cmd.output().unwrap();
        assert_eq!(result.status.code(), Some(2), "{verb}");
        let error = stderr(result);
        assert!(error.contains("product_keys_forbidden"), "{verb}: {error}");
        assert!(!error.contains(client.to_string_lossy().as_ref()));
        assert!(!error.contains(keys.to_string_lossy().as_ref()));
        assert!(!output.exists());
    }
}

#[test]
fn resident_v20_forbids_scratch_before_client_or_output_access() {
    for verb in ["extract", "resume"] {
        let lab = Lab::new();
        let plan = lab.plan("plan.tsv", MAP_PLAN);
        let output = lab.0.join("output-must-not-exist");
        let result = command(verb, &plan)
            .args(["--client"])
            .arg(lab.0.join("synthetic-client-not-opened"))
            .args(["--output"])
            .arg(&output)
            .args(["--scratch"])
            .arg(lab.0.join("scratch-not-opened"))
            .output()
            .unwrap();
        assert_eq!(result.status.code(), Some(2), "{verb}");
        assert!(stderr(result).contains("product_scratch_forbidden"));
        assert!(!output.exists());
    }
}

#[test]
fn v20_cli_refuses_every_out_of_scope_plan_before_client_io() {
    let lab = Lab::new();
    let cases = [
        (
            MAP_PLAN.replace("build\t70450", "build\t70425"),
            "foreign build",
        ),
        (
            MAP_PLAN.replace("wowdbdefs-70450-v1", "wowdbdefs-70425-v1"),
            "foreign catalogue",
        ),
        (
            MAP_PLAN.replace("components\tmaps", "components\tmaps,db2"),
            "DB2",
        ),
        (
            MAP_PLAN.replace("components\tmaps", "components\tmaps,gt"),
            "game tables",
        ),
        (
            MAP_PLAN.replace("components\tmaps", "components\tmaps,cameras"),
            "cameras",
        ),
        (
            MAP_PLAN.replace("components\tmaps", "components\tmaps,vmaps"),
            "collision",
        ),
        (
            MAP_PLAN.replace("components\tmaps", "components\tmaps,mmaps"),
            "navigation component",
        ),
        (
            MAP_PLAN.replace("terrain\tmap:2", "terrain\ttile:2:3:4"),
            "tile selection",
        ),
        (
            MAP_PLAN.replace("navigation\tnone", "navigation\tmap:2"),
            "navigation selection",
        ),
        (
            format!("{MAP_PLAN}db2\tMap\t1349477\n"),
            "nonempty db2 selection",
        ),
        (format!("{MAP_PLAN}map-domain\t2\n"), "map-domain selection"),
        (
            format!("{MAP_PLAN}offmesh\tinline-v1\n"),
            "offmesh selection",
        ),
    ];
    for (index, (text, label)) in cases.iter().enumerate() {
        let plan = lab.plan(&format!("invalid-{index}.tsv"), text);
        let result = command("inspect", &plan).output().unwrap();
        assert!(!result.status.success(), "{label}");
    }
}
