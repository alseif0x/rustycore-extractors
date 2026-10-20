// SPDX-License-Identifier: GPL-3.0-or-later
//! Synthetic v23 command grammar and dispatch admission only; no client data.
#![cfg(target_os = "linux")]

use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};

const V23: &str = include_str!("../../../tests/contracts/fixtures/forever-product-v23/map48.tsv");
const V22: &str = include_str!("../../../tests/contracts/fixtures/forever-product-v22/map2.tsv");
const V21: &str =
    include_str!("../../../tests/contracts/fixtures/forever-product-v21/navigation-map2.tsv");
static NEXT: AtomicU64 = AtomicU64::new(0);

struct Lab(PathBuf);

impl Lab {
    fn new(plan: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "rce-v23-cli-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        fs::write(path.join("plan.tsv"), plan).unwrap();
        Self(path)
    }

    fn plan(&self) -> PathBuf {
        self.0.join("plan.tsv")
    }

    fn write_plan(&self, plan: &str) {
        fs::write(self.plan(), plan).unwrap();
    }

    fn command(&self, verb: &str, extra: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_forever-extractor"))
            .current_dir(&self.0)
            .arg(verb)
            .arg("--plan")
            .arg(self.plan())
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

fn assert_code(output: Output, code: &str) {
    assert_eq!(
        output.status.code(),
        Some(2),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains(&format!("\"code\":\"{code}\"")), "{stderr}");
}

fn assert_output_absent(lab: &Lab, name: &str) {
    assert!(
        !lab.0.join(name).exists(),
        "unexpected output created: {name}"
    );
}

fn all_input_pairs() -> [&'static str; 8] {
    [
        "--terrain-package",
        "missing-terrain-package",
        "--terrain-plan",
        "missing-terrain-plan",
        "--vmap-package",
        "missing-vmap-package",
        "--vmap-plan",
        "missing-vmap-plan",
    ]
}

#[test]
fn v23_plan_only_inspect_is_supported_but_client_inspect_fails_before_client_io() {
    let lab = Lab::new(V23);
    let planned = lab.command("inspect", &[]);
    assert!(
        planned.status.success(),
        "{}",
        String::from_utf8_lossy(&planned.stderr)
    );
    let stdout = String::from_utf8(planned.stdout).unwrap();
    for field in [
        "\"contract\":\"forever-product-v23\"",
        "\"status\":\"planned\"",
        "\"navigation_source_profile\":\"linux70450-selected-navigation-v1\"",
        "\"input_coverage\":\"producer_reported\"",
        "\"complete_navigation_input_claim\":false",
        "\"full_extractor_complete\":false",
    ] {
        assert!(stdout.contains(field), "missing {field}: {stdout}");
    }

    let client_inspect = lab.command("inspect", &["--client", "no-such-client"]);
    assert_code(client_inspect, "product_v23_client_inspect_unsupported");
    assert_output_absent(&lab, "no-such-client");
}

#[test]
fn v23_extract_resume_and_verify_links_require_both_complete_input_pairs() {
    let lab = Lab::new(V23);
    for (verb, args) in [
        ("extract", &[][..]),
        (
            "resume",
            &["--client", "no-client", "--output", "no-output"][..],
        ),
        ("verify-links", &["--output", "no-output"][..]),
    ] {
        assert_code(
            lab.command(verb, args),
            "product_navigation_inputs_required",
        );
        assert_output_absent(&lab, "no-output");
    }

    assert_code(
        lab.command(
            "extract",
            &[
                "--client",
                "no-client",
                "--output",
                "no-output",
                "--terrain-package",
                "no-terrain-package",
            ],
        ),
        "product_navigation_inputs_required",
    );
    assert_output_absent(&lab, "no-output");
    assert_code(
        lab.command(
            "resume",
            &[
                "--client",
                "no-client",
                "--output",
                "no-output",
                "--terrain-package",
                "no-terrain-package",
                "--terrain-plan",
                "no-terrain-plan",
            ],
        ),
        "product_navigation_inputs_required",
    );
    assert_output_absent(&lab, "no-output");
    assert_code(
        lab.command(
            "verify-links",
            &[
                "--output",
                "no-output",
                "--terrain-package",
                "no-terrain-package",
                "--terrain-plan",
                "no-terrain-plan",
                "--vmap-package",
                "no-vmap-package",
            ],
        ),
        "product_navigation_inputs_required",
    );
    assert_output_absent(&lab, "no-output");

    // Complete pairs pass CLI admission and reach the offline verifier. The
    // deliberately absent package/output paths keep this a read-only failure.
    let pairs = all_input_pairs();
    let complete = lab.command(
        "verify-links",
        &[
            "--output",
            "no-output",
            pairs[0],
            pairs[1],
            pairs[2],
            pairs[3],
            pairs[4],
            pairs[5],
            pairs[6],
            pairs[7],
        ],
    );
    assert!(!complete.status.success());
    let stderr = String::from_utf8(complete.stderr).unwrap();
    assert!(
        !stderr.contains("\"code\":\"product_navigation_inputs_required\""),
        "{stderr}"
    );
    assert!(
        !stderr.contains("\"code\":\"product_navigation_inputs_forbidden\""),
        "{stderr}"
    );
    assert!(!stderr.contains("\"code\":\"product_usage\""), "{stderr}");
    assert_output_absent(&lab, "no-output");

    let output_path = lab.0.join("extract-output");
    let output_text = output_path.to_str().unwrap();
    let mut extract_args = vec!["--client", "no-client", "--output", output_text];
    extract_args.extend(pairs);
    let complete_extract = lab.command("extract", &extract_args);
    assert!(!complete_extract.status.success());
    let stderr = String::from_utf8(complete_extract.stderr).unwrap();
    assert!(
        !stderr.contains("\"code\":\"product_navigation_inputs_required\""),
        "{stderr}"
    );
    assert!(
        !stderr.contains("\"code\":\"product_navigation_inputs_forbidden\""),
        "{stderr}"
    );
    assert!(!stderr.contains("\"code\":\"product_usage\""), "{stderr}");
    assert!(!output_path.exists());
}

#[test]
fn navigation_pairs_reject_empty_duplicate_unknown_and_wrong_command_options() {
    let lab = Lab::new(V23);
    assert_code(
        lab.command(
            "extract",
            &[
                "--client",
                "no-client",
                "--output",
                "no-output",
                "--terrain-package",
                "",
                "--terrain-plan",
                "no-terrain-plan",
                "--vmap-package",
                "no-vmap-package",
                "--vmap-plan",
                "no-vmap-plan",
            ],
        ),
        "product_usage",
    );
    assert_output_absent(&lab, "no-output");
    assert_code(
        lab.command(
            "extract",
            &[
                "--client",
                "no-client",
                "--output",
                "no-output",
                "--terrain-package",
                "a",
                "--terrain-package",
                "b",
                "--terrain-plan",
                "p",
                "--vmap-package",
                "v",
                "--vmap-plan",
                "vp",
            ],
        ),
        "product_usage",
    );
    assert_output_absent(&lab, "no-output");
    assert_code(
        lab.command(
            "verify-links",
            &["--output", "no-output", "--unexpected", "x"],
        ),
        "product_usage",
    );
    assert_output_absent(&lab, "no-output");

    // Basic verify and inspect never accept linked-input flags, including for
    // v23. Only verify-links carries those pairs.
    let pairs = all_input_pairs();
    let mut verify_args = vec!["--output", "no-output"];
    verify_args.extend(pairs);
    assert_code(lab.command("verify", &verify_args), "product_usage");
    assert_output_absent(&lab, "no-output");
    assert_code(
        lab.command("inspect", &["--terrain-package", "no-terrain-package"]),
        "product_usage",
    );

    let basic_verify = lab.command("verify", &["--output", "no-output"]);
    assert!(!basic_verify.status.success());
    let stderr = String::from_utf8(basic_verify.stderr).unwrap();
    assert!(
        !stderr.contains("\"code\":\"product_navigation_inputs_required\""),
        "{stderr}"
    );
    assert!(
        !stderr.contains("\"code\":\"product_navigation_inputs_forbidden\""),
        "{stderr}"
    );
    assert!(!stderr.contains("\"code\":\"product_usage\""), "{stderr}");
    assert_output_absent(&lab, "no-output");
}

#[test]
fn other_profiles_forbid_navigation_pairs_and_keep_existing_cli_behavior() {
    let lab = Lab::new(V22);
    for (verb, base) in [
        (
            "extract",
            &["--client", "no-client", "--output", "no-output"][..],
        ),
        (
            "resume",
            &["--client", "no-client", "--output", "no-output"][..],
        ),
        ("verify-links", &["--output", "no-output"][..]),
    ] {
        let mut args = base.to_vec();
        args.extend(all_input_pairs());
        assert_code(
            lab.command(verb, &args),
            "product_navigation_inputs_forbidden",
        );
        assert_output_absent(&lab, "no-output");
    }

    let planned_v22 = lab.command("inspect", &[]);
    assert!(planned_v22.status.success());
    assert!(
        String::from_utf8(planned_v22.stdout)
            .unwrap()
            .contains("\"contract\":\"forever-product-v22\"")
    );

    let old_extract_order = lab.command(
        "extract",
        &["--client", "no-client", "--output", "no-output"],
    );
    assert_code(old_extract_order, "product_scratch_required");
    assert_output_absent(&lab, "no-output");

    lab.write_plan(V21);
    let planned_v21 = lab.command("inspect", &[]);
    assert!(planned_v21.status.success());
    let stdout = String::from_utf8(planned_v21.stdout).unwrap();
    assert!(stdout.contains("\"contract\":\"forever-product-v21\""));
    assert!(!stdout.contains("linux70450-selected-navigation-v1"));
}
