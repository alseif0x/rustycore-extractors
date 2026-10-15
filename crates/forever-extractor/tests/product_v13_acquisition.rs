// SPDX-License-Identifier: GPL-3.0-or-later
//! Actual product subprocesses over authored toy CASC. No real client/native API.
//! Recovery cuts are deterministic Run states, not killed-process/fsync evidence.
#![cfg(target_os = "linux")]
use forever_casc::package::{Component, Run};
use forever_extractor::{plan::Plan, verify};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Lab(PathBuf);
impl Lab {
    fn new(mode: &str, version: u8, all: bool) -> Self {
        let root = std::env::temp_dir().join(format!(
            "rce-v13-acquire-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/forever/product_terrain_70374_fixture.py");
        let output = Command::new("python3")
            .args(["-S", "-B"])
            .arg(fixture)
            .arg(&root)
            .arg(mode)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let text =
            include_str!("../../../tests/contracts/fixtures/forever-product-v13/available.tsv")
                .replace(
                    "forever-product\t13",
                    &format!("forever-product\t{version}"),
                );
        fs::write(
            root.join("plan.tsv"),
            if all {
                text
            } else {
                text.replace("all-available-maps", "map:0")
            },
        )
        .unwrap();
        Self(root)
    }
    fn plan(&self) -> Plan {
        Plan::parse(&fs::read_to_string(self.0.join("plan.tsv")).unwrap()).unwrap()
    }
    fn output(&self) -> PathBuf {
        self.0.join("output")
    }
    fn command(&self, verb: &str, output: &Path) -> Output {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_forever-extractor"));
        cmd.arg(verb)
            .arg("--plan")
            .arg(self.0.join("plan.tsv"))
            .arg("--output")
            .arg(output);
        if matches!(verb, "extract" | "resume") {
            cmd.arg("--client").arg(self.0.join("client"));
        }
        cmd.output().unwrap()
    }
    fn extract(&self) {
        let out = self.command("extract", &self.output());
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    fn check(&self, output: &Path) {
        for verb in ["verify", "verify-links"] {
            let r = self.command(verb, output);
            assert!(r.status.success(), "{}", String::from_utf8_lossy(&r.stderr));
            let text = String::from_utf8(r.stdout).unwrap();
            assert!(
                text.contains("declared_available_map_omission_coverage_links")
                    && text.contains("\"full_extractor_complete\":false")
            );
        }
    }
    fn coverage(&self) -> String {
        fs::read_to_string(
            self.output()
                .join("maps/compatible-terrain-coverage-v2.tsv"),
        )
        .unwrap()
    }
}
impl Drop for Lab {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
fn files(output: &Path) -> Vec<(String, Vec<u8>)> {
    let mut f: Vec<_> = fs::read_dir(output.join("maps"))
        .unwrap()
        .map(|p| {
            let p = p.unwrap().path();
            (
                format!("maps/{}", p.file_name().unwrap().to_str().unwrap()),
                fs::read(p).unwrap(),
            )
        })
        .collect();
    f.sort_by(|a, b| a.0.cmp(&b.0));
    f
}
#[test]
fn v13_actual_toy_casc_clean_selected_and_all_acquire() {
    for all in [false, true] {
        let l = Lab::new("complete", 13, all);
        l.extract();
        l.check(&l.output());
        let c = l.coverage();
        assert!(c.contains("map-auth\tfalse\ttrue") && c.contains("map\t0\t0\t0\tloaded\t2\t2\t0"));
        let summary = verify::verify(&l.plan(), &l.output(), None).unwrap();
        assert_eq!(summary.files, if all { 5 } else { 4 });
    }
}
#[test]
fn v13_actual_missing_wdt_emits_unknown_zero_list_and_continues() {
    let l = Lab::new("missing_wdt", 13, true);
    l.extract();
    l.check(&l.output());
    let c = l.coverage();
    assert!(
        c.contains("map\t0\t0\t0\tunavailable\tunknown\t0\t0")
            && c.contains("map\t1\t1\t9\tloaded\t0\t0\t0")
    );
    assert!(c.contains("finish\t2\t8192\t1\t1\t0\t4096\t4096\t0\t0\t2"));
    assert_eq!(
        verify::verify(&l.plan(), &l.output(), None).unwrap().files,
        3
    );
}
#[test]
fn v13_actual_missing_present_adt_clears_bit_and_continues() {
    let l = Lab::new("missing_adt", 13, true);
    l.extract();
    l.check(&l.output());
    assert!(l.coverage().contains("map\t0\t0\t0\tloaded\t2\t1\t1"));
    assert!(l.output().join("maps/0000_02_03.map").exists());
    assert!(!l.output().join("maps/0000_04_01.map").exists());
    assert_eq!(
        verify::verify(&l.plan(), &l.output(), None).unwrap().files,
        4
    );
}
#[test]
fn v13_malformed_adt_remains_terminal_and_both_verifiers_reject() {
    let l = Lab::new("malformed_adt", 13, true);
    let r = l.command("extract", &l.output());
    assert_eq!(r.status.code(), Some(9));
    let error = String::from_utf8(r.stderr).unwrap();
    assert!(
        error.contains("terrain_invalid_chunk_size") && error.contains("\"component\":\"maps\""),
        "{error}"
    );
    assert!(!l.output().join("forever-package.json").exists());
    for verb in ["verify", "verify-links"] {
        assert!(!l.command(verb, &l.output()).status.success());
    }
}
#[test]
fn v12_same_missing_fixtures_still_fail_strictly() {
    for mode in ["missing_adt", "missing_wdt"] {
        let l = Lab::new(mode, 12, true);
        let r = l.command("extract", &l.output());
        assert_eq!(r.status.code(), Some(9));
        assert!(
            String::from_utf8(r.stderr)
                .unwrap()
                .contains("file_unavailable")
        );
        assert!(!l.output().join("forever-package.json").exists());
        for verb in ["verify", "verify-links"] {
            assert!(!l.command(verb, &l.output()).status.success());
        }
    }
}
#[test]
fn v13_actual_resume_acquires_after_empty_partial_and_sealed_authored_states() {
    let l = Lab::new("missing_adt", 13, true);
    l.extract();
    let p = l.plan();
    let context = verify::verify(&p, &l.output(), None).unwrap().context;
    let expected = files(&l.output());
    for cut in [0, 1, 2] {
        let output = l.0.join(format!("recovered-{cut}"));
        let mut r = Run::create(&output, context, &[], Default::default()).unwrap();
        if cut == 1 {
            let (path, bytes) = expected.iter().find(|p| p.0.ends_with(".map")).unwrap();
            r.put(Component::Terrain, path, bytes).unwrap();
        }
        if cut == 2 {
            for (path, bytes) in &expected {
                r.put(Component::Terrain, path, bytes).unwrap();
            }
            r.finish_component(Component::Terrain).unwrap();
        }
        drop(r);
        assert!(verify::verify(&p, &output, None).is_err());
        let result = l.command("resume", &output);
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        l.check(&output);
        assert_eq!(files(&output), expected);
        assert_eq!(
            fs::read(output.join("forever-package.json")).unwrap(),
            fs::read(l.output().join("forever-package.json")).unwrap()
        );
    }
}
#[test]
fn v13_already_complete_resume_rejects_unchanged_then_verifies() {
    let l = Lab::new("missing_wdt", 13, true);
    l.extract();
    let before = files(&l.output());
    let manifest = fs::read(l.output().join("forever-package.json")).unwrap();
    assert!(!l.command("resume", &l.output()).status.success());
    assert_eq!(files(&l.output()), before);
    assert_eq!(
        fs::read(l.output().join("forever-package.json")).unwrap(),
        manifest
    );
    l.check(&l.output());
}
