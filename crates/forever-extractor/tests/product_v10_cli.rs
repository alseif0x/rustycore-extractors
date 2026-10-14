// SPDX-License-Identifier: GPL-3.0-or-later
//! Authored build metadata traps only; no CASC/bootstrap input.
#![cfg(target_os = "linux")]
use std::{
    fs,
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Lab(PathBuf);
impl Lab {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "rce-product10-cli-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        Self(p)
    }
}
impl Drop for Lab {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
const PLAN: &str =
    include_str!("../../../tests/contracts/fixtures/forever-product-v10/disabled.tsv");
#[test]
fn wrong_authored_client_build_rejects_before_bootstrap_or_output() {
    for build in [70170, 70205, 70338] {
        let lab = Lab::new();
        let client = lab.0.join("client");
        fs::create_dir(&client).unwrap();
        fs::write(client.join(".build.info"),format!("Product!STRING:0|Active!DEC:1|Version!STRING:0|Build Key!HEX:16|CDN Key!HEX:16|Tags!STRING:0\nwow_classic_beta|1|1.60.1.{build}|11111111111111111111111111111111|22222222222222222222222222222222|esES\n")).unwrap();
        let plan = lab.0.join("plan.tsv");
        fs::write(&plan, PLAN).unwrap();
        let out = lab.0.join("output");
        let result = Command::new(env!("CARGO_BIN_EXE_forever-extractor"))
            .arg("extract")
            .arg("--plan")
            .arg(plan)
            .arg("--client")
            .arg(client)
            .arg("--output")
            .arg(&out)
            .output()
            .unwrap();
        let text = String::from_utf8(result.stderr).unwrap();
        assert_eq!(result.status.code(), Some(9));
        assert!(text.contains("unsupported_client"), "{text}");
        assert!(text.contains("forever-product-v10"));
        assert!(!out.exists());
    }
}
#[test]
fn v10_inspect_without_client_has_new_contract_and_target_catalog() {
    let help = Command::new(env!("CARGO_BIN_EXE_forever-extractor"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(help.status.success());
    let help = String::from_utf8(help.stdout).unwrap();
    assert!(help.contains("Linux v10 is explicit Rust-local build-70374 DB2-only"));
    assert!(help.contains("v2-v6/v8/v9 client commands require --keys"));
    assert!(help.contains("Windows v10 execution reject"));
    let lab = Lab::new();
    let plan = lab.0.join("plan.tsv");
    fs::write(&plan, PLAN).unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_forever-extractor"))
        .args(["inspect", "--plan"])
        .arg(plan)
        .output()
        .unwrap();
    assert!(result.status.success());
    let text = String::from_utf8(result.stdout).unwrap();
    assert!(
        text.contains("forever-product-v10")
            && text.contains("wowdbdefs-70374-v1")
            && text.contains("\"components\":1")
    );
}

#[test]
fn v10_wrong_mask_rejects_before_nonexistent_client_with_legacy_parse_envelope() {
    let lab = Lab::new();
    let p = lab.0.join("plan.tsv");
    fs::write(&p, PLAN.replace("components\tdb2", "components\tdb2,gt")).unwrap();
    let out = lab.0.join("output");
    let r = Command::new(env!("CARGO_BIN_EXE_forever-extractor"))
        .args(["extract", "--plan"])
        .arg(p)
        .args(["--client", "/nonexistent-authored-client", "--output"])
        .arg(&out)
        .output()
        .unwrap();
    let text = String::from_utf8(r.stderr).unwrap();
    assert_eq!(r.status.code(), Some(2));
    assert!(text.contains("product_plan_component_scope") && text.contains("forever-product-v1"));
    assert!(!out.exists());
}
#[test]
fn v5_db2_only_inspect_retains_old_contract() {
    let lab = Lab::new();
    let p = lab.0.join("plan.tsv");
    fs::write(
        &p,
        include_str!("../../../tests/contracts/fixtures/forever-product-v9/disabled.tsv")
            .replace("forever-product\t9", "forever-product\t5")
            .replace("db2,gt,cameras", "db2"),
    )
    .unwrap();
    let r = Command::new(env!("CARGO_BIN_EXE_forever-extractor"))
        .args(["inspect", "--plan"])
        .arg(p)
        .output()
        .unwrap();
    assert!(r.status.success());
    let t = String::from_utf8(r.stdout).unwrap();
    assert!(t.contains("forever-product-v5") && t.contains("\"components\":1"));
}
#[test]
fn v10_key_option_policies_precede_client_and_key_input() {
    for provided in [false, true] {
        let lab = Lab::new();
        let p = lab.0.join("plan.tsv");
        fs::write(
            &p,
            if provided {
                PLAN.replace("decryption\tdisabled", "decryption\tprovided-salsa20-v1")
            } else {
                PLAN.into()
            },
        )
        .unwrap();
        let out = lab.0.join("output");
        let mut c = Command::new(env!("CARGO_BIN_EXE_forever-extractor"));
        c.args(["extract", "--plan"])
            .arg(p)
            .args(["--client", "/nonexistent-authored-client", "--output"])
            .arg(&out);
        if !provided {
            c.args(["--keys", "/nonexistent-authored-keys"]);
        }
        let r = c.output().unwrap();
        let t = String::from_utf8(r.stderr).unwrap();
        assert_eq!(r.status.code(), Some(2));
        assert!(t.contains(if provided {
            "product_keys_required"
        } else {
            "product_keys_forbidden"
        }));
        assert!(t.contains("forever-product-v10"));
        assert!(!out.exists());
    }
}
