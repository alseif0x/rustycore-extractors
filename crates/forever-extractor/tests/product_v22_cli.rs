// SPDX-License-Identifier: GPL-3.0-or-later
//! Synthetic v22 plan, inspect, option admission, and verifier dispatch only.
#![cfg(target_os = "linux")]

use forever_casc::{
    integrity,
    package::{Component, Context, Run},
};
use forever_extractor::plan::Plan;
use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};

const V22: &str = include_str!("../../../tests/contracts/fixtures/forever-product-v22/map2.tsv");
static NEXT: AtomicU64 = AtomicU64::new(0);

struct Lab(PathBuf);
impl Lab {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "rce-v22-cli-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        fs::write(path.join("plan.tsv"), V22).unwrap();
        Self(path)
    }
    fn plan(&self) -> PathBuf {
        self.0.join("plan.tsv")
    }
    fn command(&self, verb: &str, extra: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_forever-extractor"))
            .arg(verb)
            .arg("--plan")
            .arg(self.plan())
            .args(extra)
            .output()
            .unwrap()
    }
    fn write_plan(&self, plan: &str) {
        fs::write(self.plan(), plan).unwrap();
    }
    fn publish_missing_coverage(&self) -> PathBuf {
        let plan = Plan::parse(V22).unwrap();
        let context = Context {
            build: 70450,
            locale: plan.locale,
            components: Component::Collision.bit(),
            source_sha256: [0x21; 32],
            plan_sha256: integrity::digest(plan.render().as_bytes()),
            producer_sha256: [0x22; 32],
        };
        let output = self.0.join("package");
        let mut run = Run::create(&output, context, &[], Default::default()).unwrap();
        let map = forever_vmap::assembly::build(2, &[], Default::default()).unwrap();
        run.put(
            Component::Collision,
            "vmaps/0002/0002.vmtree",
            map.tree_bytes(),
        )
        .unwrap();
        run.finish_component(Component::Collision).unwrap();
        run.finish().unwrap();
        output
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
fn v22_plan_only_inspect_reports_selected_geometry_without_client_io() {
    let lab = Lab::new();
    let output = lab.command("inspect", &[]);
    assert!(output.status.success(), "{}", stderr(output));
    let json = String::from_utf8(output.stdout).unwrap();
    for field in [
        "forever-product-v22",
        "\"build\":70450",
        "\"components\":16",
        "\"schema_catalog\":\"wowdbdefs-70450-v1\"",
        "linux70450-selected-map-geometry-v1",
        "\"selected-map-and-active-ancestors\"",
        "\"gameobject_scope\":\"excluded_by_profile\"",
        "\"model_scope\":\"placement_only\"",
        "\"complete_global_map_coverage_claim\":false",
        "\"full_extractor_complete\":false",
    ] {
        assert!(json.contains(field), "missing {field}: {json}");
    }
    for forbidden_claim in [
        "\"mmaps\"",
        "\"runtime_acceptance\":true",
        "\"full_world\":true",
    ] {
        assert!(
            !json.contains(forbidden_claim),
            "unexpected {forbidden_claim}: {json}"
        );
    }
}

#[test]
fn v22_cli_requires_scratch_and_refuses_keys_before_source_io() {
    let lab = Lab::new();
    let missing_scratch = lab.command(
        "extract",
        &[
            "--client",
            "synthetic-source-not-created",
            "--output",
            "synthetic-package-not-created",
        ],
    );
    assert!(!missing_scratch.status.success());
    assert!(stderr(missing_scratch).contains("\"code\":\"product_scratch_required\""));

    let keys = lab.command(
        "resume",
        &[
            "--client",
            "synthetic-source-not-created",
            "--output",
            "synthetic-package-not-created",
            "--scratch",
            "synthetic-scratch-not-created",
            "--keys",
            "synthetic-key-file-not-created",
        ],
    );
    assert!(!keys.status.success());
    assert!(stderr(keys).contains("\"code\":\"product_keys_forbidden\""));
}

#[test]
fn v22_cli_rejects_wrong_masks_and_domains_and_routes_verify_to_own_coverage() {
    let lab = Lab::new();
    for (text, code) in [
        (
            V22.replace("components\tvmaps", "components\tmaps"),
            "product_plan_component_scope",
        ),
        (
            V22.replace(
                "collision-domain\tmap:2",
                "collision-domain\tall-available-maps",
            ),
            "product_plan_collision_domain",
        ),
        (
            V22.replace("decryption\tdisabled", "decryption\tprovided-salsa20-v1"),
            "product_plan_component_scope",
        ),
    ] {
        lab.write_plan(&text);
        let output = lab.command("inspect", &[]);
        assert!(!output.status.success());
        assert!(stderr(output).contains(&format!("\"code\":\"{code}\"")));
    }

    lab.write_plan(V22);
    let package = lab.publish_missing_coverage();
    let output_path = package.to_str().unwrap();
    for command in ["verify", "verify-links"] {
        let output = lab.command(command, &["--output", output_path]);
        assert!(
            !output.status.success(),
            "{command} accepted no v22 coverage"
        );
        assert!(stderr(output).contains("\"code\":\"package_inventory\""));
    }
}
