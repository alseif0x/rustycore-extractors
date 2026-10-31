// SPDX-License-Identifier: GPL-3.0-or-later
#![cfg(target_os = "linux")]
#[allow(dead_code)]
#[path = "v47_gameobject_expanded_material_filtered_wmo_geometry_cli/fixture.rs"]
mod fixture;
use forever_casc::package::{self, Artifact, Component, Context, Limits};
use std::{
    collections::BTreeMap,
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
const PLAN: &str = include_str!("../../../tests/contracts/fixtures/forever-product-v53/all.tsv");
const SOURCE: &str = include_str!("../../../tests/contracts/fixtures/forever-product-v51/all.tsv");
struct Lab(PathBuf);
impl Lab {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "rce-v53-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        fs::write(
            root.join("v53.tsv"),
            PLAN.replace("locale\tenUS", "locale\tesES"),
        )
        .unwrap();
        fs::write(
            root.join("v51.tsv"),
            SOURCE.replace("locale\tenUS", "locale\tesES"),
        )
        .unwrap();
        Self(root)
    }
    fn source(&self) -> PathBuf {
        let client = self.0.join("client");
        fs::create_dir(&client).unwrap();
        fixture::write_v52_source_client(&client, None, true);
        let scratch = self.0.join("scratch");
        fs::create_dir(&scratch).unwrap();
        fs::set_permissions(&scratch, fs::Permissions::from_mode(0o700)).unwrap();
        let source = self.0.join("source");
        ok(Command::new(env!("CARGO_BIN_EXE_forever-extractor"))
            .args(["extract", "--plan"])
            .arg(self.0.join("v51.tsv"))
            .arg("--client")
            .arg(&client)
            .arg("--scratch")
            .arg(scratch)
            .arg("--output")
            .arg(&source)
            .output()
            .unwrap());
        fs::remove_dir_all(client).unwrap();
        source
    }
    fn command(&self, verb: &str, output: &Path) -> Command {
        let mut c = Command::new(env!("CARGO_BIN_EXE_forever-extractor"));
        c.arg(verb).arg("--plan").arg(self.0.join("v53.tsv"));
        if verb != "inspect" {
            c.arg("--output").arg(output);
        }
        if ["extract", "resume", "verify-links"].contains(&verb) {
            c.arg("--terrain-package")
                .arg(self.0.join("source"))
                .arg("--terrain-plan")
                .arg(self.0.join("v51.tsv"));
        }
        c
    }
}
impl Drop for Lab {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn ok(output: Output) -> String {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}
fn artifacts(root: &Path) -> BTreeMap<String, (Component, Vec<u8>)> {
    let context =
        Context::parse(&fs::read_to_string(root.join(".state/context.json")).unwrap()).unwrap();
    fs::read_dir(root.join(".state/files"))
        .unwrap()
        .map(|entry| {
            let a = Artifact::parse(
                &fs::read_to_string(entry.unwrap().path()).unwrap(),
                context,
                Limits::default(),
            )
            .unwrap();
            let bytes = fs::read(root.join(&a.path)).unwrap();
            (a.path, (a.component, bytes))
        })
        .collect()
}
#[test]
fn v53_lifecycle_offline_multi_map_resume_determinism() {
    let lab = Lab::new();
    lab.source();
    let output = lab.0.join("output");
    assert!(ok(lab.command("inspect", &output).output().unwrap()).contains("forever-product-v53"));
    ok(lab.command("extract", &output).output().unwrap());
    for verb in ["verify", "verify-links"] {
        ok(lab.command(verb, &output).output().unwrap());
    }
    let files = artifacts(&output);
    assert!(files.values().all(|(c, _)| *c == Component::Navigation));
    let coverage =
        String::from_utf8(files["mmaps/available-navigation-v53.tsv"].1.clone()).unwrap();
    assert!(coverage.contains("map\t1\t1\tplanned\t1"), "{coverage}");
    assert!(coverage.contains("map\t2\t2\tplanned\t1"), "{coverage}");
    assert!(coverage.contains("map\t3\t3\tempty\t0"), "{coverage}");
    assert!(coverage.contains("map\t4\t4\troot-absent\t0"), "{coverage}");
    let failed = lab.0.join("failed");
    let context =
        Context::parse(&fs::read_to_string(output.join(".state/context.json")).unwrap()).unwrap();
    let mut run = package::Run::create(&failed, context, &[], Limits::default()).unwrap();
    let first = files
        .iter()
        .find(|(p, _)| p.ends_with("0001.mmap"))
        .unwrap();
    run.put(Component::Navigation, first.0, &first.1.1).unwrap();
    let second_global = files.keys().find(|p| p.ends_with("0002.mmap")).unwrap();
    run.put(Component::Navigation, second_global, b"wrong replay")
        .unwrap();
    drop(run);
    assert!(
        !lab.command("resume", &failed)
            .output()
            .unwrap()
            .status
            .success()
    );
    assert!(!failed.join("forever-package.json").exists());
    assert!(failed.join(".forever-package-incomplete").exists());
    let second = lab.0.join("second");
    ok(lab.command("extract", &second).output().unwrap());
    assert_eq!(files, artifacts(&second));
    fs::remove_file(second.join("forever-package.json")).unwrap();
    fs::remove_file(second.join(".state/sealed/mmaps")).unwrap();
    fs::write(second.join(".forever-package-incomplete"), b"incomplete\n").unwrap();
    ok(lab.command("resume", &second).output().unwrap());
    assert_eq!(files, artifacts(&second));
    for verb in ["extract", "resume", "verify-links"] {
        assert!(
            !lab.command(verb, &output)
                .arg("--client")
                .arg(lab.0.join("missing-client"))
                .output()
                .unwrap()
                .status
                .success()
        );
    }
}
#[test]
fn v53_rejects_source_tampering_and_cross_profiles() {
    let lab = Lab::new();
    let source = lab.source();
    let output = lab.0.join("output");
    ok(lab.command("extract", &output).output().unwrap());
    let path = source.join("dbc/esES/LiquidType.db2");
    let bytes = fs::read(&path).unwrap();
    fs::write(&path, b"bad").unwrap();
    for verb in ["extract", "verify-links"] {
        assert!(
            !lab.command(verb, &lab.0.join("rejected"))
                .output()
                .unwrap()
                .status
                .success()
        );
    }
    fs::write(path, bytes).unwrap();
    let original = fs::read(lab.0.join("v51.tsv")).unwrap();
    for plan in [
        include_str!("../../../tests/contracts/fixtures/forever-product-v25/all.tsv"),
        include_str!("../../../tests/contracts/fixtures/forever-product-v50/all.tsv"),
    ] {
        fs::write(
            lab.0.join("v51.tsv"),
            plan.replace("locale\tenUS", "locale\tesES"),
        )
        .unwrap();
        assert!(
            !lab.command("verify-links", &output)
                .output()
                .unwrap()
                .status
                .success()
        );
    }
    fs::write(lab.0.join("v51.tsv"), original).unwrap();
    fs::write(source.join(".forever-package-incomplete"), b"incomplete\n").unwrap();
    assert!(
        !lab.command("verify-links", &output)
            .output()
            .unwrap()
            .status
            .success()
    );
}
#[test]
fn v53_forged_inventory_is_rejected() {
    let lab = Lab::new();
    lab.source();
    let output = lab.0.join("output");
    ok(lab.command("extract", &output).output().unwrap());
    let context =
        Context::parse(&fs::read_to_string(output.join(".state/context.json")).unwrap()).unwrap();
    let files = artifacts(&output);
    let forged = lab.0.join("forged");
    let mut run = package::Run::create(&forged, context, &[], Limits::default()).unwrap();
    for (path, (component, bytes)) in files {
        if path.ends_with(".mmap") {
            continue;
        }
        run.put(component, &path, &bytes).unwrap();
    }
    run.finish_component(Component::Navigation).unwrap();
    run.finish().unwrap();
    assert!(
        !lab.command("verify", &forged)
            .output()
            .unwrap()
            .status
            .success()
    );
}
