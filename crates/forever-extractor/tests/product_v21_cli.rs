// SPDX-License-Identifier: GPL-3.0-or-later
//! Synthetic v21 plan and CLI admission only; no client, keys or network.
#![cfg(target_os = "linux")]

use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};

const V21: &str =
    include_str!("../../../tests/contracts/fixtures/forever-product-v21/navigation-map2.tsv");
static NEXT: AtomicU64 = AtomicU64::new(0);

struct Lab(PathBuf);
impl Lab {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "rce-product21-cli-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
    fn plan(&self, name: &str, text: &str) -> PathBuf {
        let path = self.0.join(name);
        fs::write(&path, text).unwrap();
        path
    }
    fn command(&self, verb: &str, plan: &PathBuf, extra: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_forever-extractor"))
            .arg(verb)
            .arg("--plan")
            .arg(plan)
            .args(extra)
            .output()
            .unwrap()
    }
}
impl Drop for Lab {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn stderr(output: Output) -> String {
    String::from_utf8(output.stderr).unwrap()
}

#[test]
fn v21_plan_only_inspection_reports_the_navigation_profile_and_map_domain() {
    let lab = Lab::new();
    let plan = lab.plan("v21.tsv", V21);
    let result = lab.command("inspect", &plan, &[]);
    assert!(result.status.success());
    let stdout = String::from_utf8(result.stdout).unwrap();
    for fact in [
        "\"contract\":\"forever-product-v21\"",
        "\"build\":70450",
        "\"components\":57",
        "\"schema_catalog\":\"wowdbdefs-70450-v1\"",
        "\"decryption\":\"disabled\"",
        "\"collision_storage\":\"linux-leased-append-disk-world-v1\"",
        "\"map_domain\":\"map-and-ancestors-v1\"",
        "\"requested_map\":2",
        "\"gameobject_scope\":\"global-startup-v1\"",
        "\"placement_ids\":\"selected-closure-serial-ids-v1\"",
        "\"full_extractor_complete\":false",
    ] {
        assert!(stdout.contains(fact), "missing {fact}: {stdout}");
    }
}

#[test]
fn v21_cli_refuses_keys_before_opening_client_or_output_paths() {
    let lab = Lab::new();
    let plan = lab.plan("v21.tsv", V21);
    let client = lab.0.join("synthetic-client-not-opened");
    let output = lab.0.join("output-not-created");
    let key_path = lab.0.join("keys-not-opened");
    let scratch = lab.0.join("scratch-not-opened");
    for verb in ["inspect", "extract", "resume"] {
        let mut extra = vec![
            "--client",
            client.to_str().unwrap(),
            "--keys",
            key_path.to_str().unwrap(),
        ];
        if verb != "inspect" {
            extra.extend(["--output", output.to_str().unwrap()]);
            extra.extend(["--scratch", scratch.to_str().unwrap()]);
        }
        let result = lab.command(verb, &plan, &extra);
        assert_eq!(result.status.code(), Some(2), "{verb}");
        assert!(stderr(result).contains("product_keys_forbidden"), "{verb}");
        assert!(!key_path.exists());
        assert!(!output.exists());
    }
}
