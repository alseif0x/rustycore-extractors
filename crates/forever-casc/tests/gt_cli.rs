// SPDX-License-Identifier: GPL-3.0-or-later
//! Metadata-only CLI traps: no CASC bootstrap files or real inputs.
use std::{
    fs,
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Lab(PathBuf);
impl Lab {
    fn new(build: u32) -> Self {
        let root = std::env::temp_dir().join(format!(
            "rce-gt-cli-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::write(root.join(".build.info"),format!("Product!STRING:0|Active!DEC:1|Version!STRING:0|Build Key!HEX:16|CDN Key!HEX:16|Tags!STRING:0\nwow_classic_beta|1|1.60.1.{build}|11111111111111111111111111111111|22222222222222222222222222222222|enUS\n")).unwrap();
        Self(root)
    }
    fn check(&self, tail: &[&str], code: &str, exit: i32) -> String {
        let out = self.0.join("output");
        let result = Command::new(env!("CARGO_BIN_EXE_forever-gt-extract"))
            .args(["--client"])
            .arg(&self.0)
            .args(["--output"])
            .arg(&out)
            .args(["--locale", "enUS"])
            .args(tail)
            .output()
            .unwrap();
        assert_eq!(
            result.status.code(),
            Some(exit),
            "{}",
            String::from_utf8_lossy(&result.stdout)
        );
        let text = String::from_utf8(result.stdout).unwrap();
        assert!(text.contains(&format!("\"code\":\"{code}\"")), "{text}");
        assert!(result.stderr.is_empty());
        assert!(!out.exists());
        text
    }
}
impl Drop for Lab {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
const RUST: &[&str] = &[
    "--backend",
    "rust-local",
    "--index-layout",
    "contiguous",
    "--root-layout",
    "header-50893-v2",
];
#[test]
fn legacy_70338_rejects_before_missing_casc_bootstrap_for_every_group() {
    let lab = Lab::new(70338);
    for group in ["all", "initial", "spell-values", "catalog"] {
        let mut args = RUST.to_vec();
        args.extend(["--group", group]);
        let t = lab.check(&args, "unsupported_client", 3);
        assert!(t.contains("\"report_version\":3"));
    }
}
#[test]
fn new_profile_wrong_build_rejects_before_missing_casc_bootstrap() {
    for build in [70170, 70205] {
        let lab = Lab::new(build);
        let mut args = RUST.to_vec();
        args.extend(["--group", "catalog", "--profile", "gt-catalog-70338-v1"]);
        let t = lab.check(&args, "unsupported_client", 3);
        assert!(t.contains("\"report_version\":4,\"component_profile\":\"gt-catalog-70338-v1\""));
    }
}
#[test]
fn profile_combinations_reject_with_nonexistent_client() {
    let lab = Lab::new(70338);
    fs::remove_file(lab.0.join(".build.info")).unwrap();
    for tail in [
        &["--profile", "gt-catalog-70338-v1"][..],
        &["--profile", "auto"],
        &[
            "--group",
            "catalog",
            "--profile",
            "gt-catalog-70338-v1",
            "--backend",
            "native",
        ],
        &[
            "--profile",
            "gt-catalog-70338-v1",
            "--profile",
            "gt-catalog-70338-v1",
        ],
    ] {
        lab.check(tail, "invalid_invocation", 2);
    }
}
#[cfg(not(feature = "native"))]
#[test]
fn unavailable_default_backend_precedes_missing_build_metadata() {
    let lab = Lab::new(70338);
    fs::remove_file(lab.0.join(".build.info")).unwrap();
    lab.check(&[], "backend_unavailable", 2);
}
