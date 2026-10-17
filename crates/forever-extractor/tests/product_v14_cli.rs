// SPDX-License-Identifier: GPL-3.0-or-later
//! Synthetic v14 plan/package CLI checks; no client or native process interface.
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
const TSV: &str = "forever-product\t14\nbuild\t70425\nlocale\tesES\nindex\tcontiguous\nroot\theader-50893-v2\nschema-catalog\twowdbdefs-70425-v1\ndecryption\tdisabled\ncollision-storage\tlinux-leased-append-disk-world-v1\ncollision-raw-bytes\t1048576\ncollision-wdt-bytes\t1048576\ncollision-cache-bytes\t1048576\ncollision-placement-bytes\t1048576\ncollision-vmo-bytes\t1048576\ncollision-map-bytes\t1048576\ncollision-output-bytes\t1048576\ncollision-profile\tlinux70425-compatible-gnu15-vmap-product-v1\ncollision-domain\tall-available-maps\ncollision-coverage\tcompatible-collision-coverage-v1\ncomponents\tvmaps\nterrain\tnone\npack\ttrue\nnavigation\tnone\nbig-base-unit\tfalse\nskip-liquids\tfalse\nmaximum-slope\t55\nnon-steep-slope\t55\n";
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Lab(PathBuf);
impl Lab {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "rce-v14-cli-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        fs::write(p.join("plan.tsv"), TSV).unwrap();
        Self(p)
    }
    fn command(&self, verb: &str, extra: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_forever-extractor"))
            .arg(verb)
            .arg("--plan")
            .arg(self.0.join("plan.tsv"))
            .args(extra)
            .output()
            .unwrap()
    }
    fn publish(&self, coverage: bool) {
        let plan = Plan::parse(TSV).unwrap();
        let context = Context {
            build: 70425,
            locale: plan.locale,
            components: Component::Collision.bit(),
            source_sha256: [1; 32],
            plan_sha256: integrity::digest(Plan::parse(TSV).unwrap().render().as_bytes()),
            producer_sha256: [2; 32],
        };
        let output = self.0.join("output");
        let mut run = Run::create(&output, context, &[], Default::default()).unwrap();
        let map_id = 2;
        let tree = forever_vmap::assembly::build(map_id, &[], Default::default()).unwrap();
        run.put(
            Component::Collision,
            &format!("vmaps/{map_id:04}/{map_id:04}.vmtree"),
            tree.tree_bytes(),
        )
        .unwrap();
        if coverage {
            run.put(
                Component::Collision,
                "vmaps/compatible-collision-coverage-v1.bin",
                b"RCVMCV1\0",
            )
            .unwrap();
        }
        run.finish_component(Component::Collision).unwrap();
        run.finish().unwrap();
    }
}
impl Drop for Lab {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
#[test]
fn v14_cli_inspect_discloses_available_map_scope() {
    let l = Lab::new();
    let r = l.command("inspect", &[]);
    assert!(r.status.success());
    let s = String::from_utf8(r.stdout).unwrap();
    assert!(s.contains("forever-product-v14"));
    assert!(s.contains("\"collision_source_profile\":\"linux70425-compatible-gnu15-v2\""));
    assert!(s.contains("\"coverage_authentication\":\"producer_reported\""));
    assert!(s.contains("\"whole_client_map_coverage\":false"));
    assert!(s.contains("\"full_extractor_complete\":false"));
    assert!(s.contains("\"collision_domain\":\"all-available-maps\""));
}
#[test]
fn v14_cli_both_verifiers_require_coverage_without_semantic_fallback() {
    for (has_coverage, expected_code) in [
        (false, "package_inventory"),
        (true, "product_collision_coverage_schema"),
    ] {
        let l = Lab::new();
        l.publish(has_coverage);
        let out = l.0.join("output");
        let out = out.to_str().unwrap();
        for verb in ["verify", "verify-links"] {
            let r = l.command(verb, &["--output", out]);
            assert!(!r.status.success(), "{verb} accepted incomplete coverage");
            let stderr = String::from_utf8(r.stderr).unwrap();
            assert!(
                stderr.contains(&format!("\"code\":\"{expected_code}\"")),
                "{verb} failed for a reason other than the missing or malformed coverage: {stderr}"
            );
        }
    }
}
#[test]
fn v14_cli_keys_scratch_and_native_options_reject_without_reading_paths() {
    let l = Lab::new();
    for (extra, expected_code) in [
        (
            vec![
                "--client",
                "/authored/absent/client",
                "--output",
                "/authored/absent/output",
                "--keys",
                "/authored/absent/keys",
                "--scratch",
                "/authored/absent/scratch",
            ],
            "product_keys_forbidden",
        ),
        (
            vec![
                "--client",
                "/authored/absent/client",
                "--output",
                "/authored/absent/output",
            ],
            "product_scratch_required",
        ),
        (
            vec![
                "--client",
                "/authored/absent/client",
                "--output",
                "/authored/absent/output",
                "--backend",
                "native",
            ],
            "product_usage",
        ),
    ] {
        let r = l.command("extract", &extra);
        assert!(!r.status.success(), "extract unexpectedly succeeded");
        let stderr = String::from_utf8(r.stderr).unwrap();
        assert!(
            stderr.contains(&format!("\"code\":\"{expected_code}\"")),
            "extract failed for a reason other than {expected_code}: {stderr}"
        );
    }
}
#[test]
fn v14_cli_legacy_domain_cross_target_and_wrong_component_are_rejected() {
    for (plan, expected_code) in [
        (
            TSV.replace(
                "collision-domain\tall-available-maps",
                "collision-domain\tall",
            ),
            "product_plan_collision_domain",
        ),
        (
            TSV.replace("build\t70425", "build\t70374"),
            "product_plan_build",
        ),
        (
            TSV.replace("components\tvmaps", "components\tmaps"),
            "product_plan_component_scope",
        ),
    ] {
        let l = Lab::new();
        fs::write(l.0.join("plan.tsv"), plan).unwrap();
        let r = l.command("inspect", &[]);
        assert!(!r.status.success(), "inspect unexpectedly succeeded");
        let stderr = String::from_utf8(r.stderr).unwrap();
        assert!(
            stderr.contains(&format!("\"code\":\"{expected_code}\"")),
            "inspect failed for a reason other than {expected_code}: {stderr}"
        );
    }
}
