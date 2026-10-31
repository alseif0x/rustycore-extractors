// SPDX-License-Identifier: GPL-3.0-or-later
//! Synthetic v27 partial-catalog and authenticated-terrain CLI lifecycle only.
#![cfg(target_os = "linux")]

#[path = "v25_partial_map_cli/fixture.rs"]
#[allow(dead_code)]
mod synthetic_client;
mod db2_fixture {
    include!("v25_partial_map_db2.rs");
}

use forever_casc::package::{Artifact, Component, Context, Limits, PartialEncryptedSection, Run};
use forever_db2::catalog;
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};

const V25_PLAN: &str =
    include_str!("../../../tests/contracts/fixtures/forever-product-v25/map2.tsv");
const MAP_PATH: &str = "dbc/enUS/Map.db2";
const LIQUID_TYPE_PATH: &str = "dbc/enUS/LiquidType.db2";
static NEXT: AtomicU64 = AtomicU64::new(0);

struct Lab(PathBuf);
impl Lab {
    fn new() -> Self {
        let root = std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join(format!(
                ".local/share/rce-v27-partial-catalog-terrain-cli-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
        fs::create_dir_all(root.parent().unwrap()).unwrap();
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        Self(root)
    }

    fn plan(&self) -> PathBuf {
        let liquid_type = catalog::resolve_for(catalog::WOWDBDEFS_70450_CATALOG_ID, "LiquidType")
            .unwrap()
            .unwrap()
            .metadata()
            .file_data_id;
        let map = "db2\tMap\t1349477\n";
        let selected = format!("db2\tLiquidType\t{liquid_type}\n{map}");
        let text = V25_PLAN
            .replace("forever-product\t25", "forever-product\t27")
            .replace("terrain\tmap:2", "terrain\tmap:48")
            .replace(map, &selected);
        let path = self.0.join("plan.tsv");
        fs::write(&path, text).unwrap();
        path
    }

    fn command(&self, verb: &str, plan: &Path) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_forever-extractor"));
        command
            .current_dir(&self.0)
            .args([verb, "--plan"])
            .arg(plan);
        command
    }
}
impl Drop for Lab {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn assert_success(output: Output, label: &str) -> String {
    assert!(
        output.status.success(),
        "{label} exited {:?}: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

fn assert_failure(output: Output, code: &str) {
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains(&format!("\"code\":\"{code}\"")), "{stderr}");
}

fn verify_package(lab: &Lab, plan: &Path, package: &Path) {
    for verb in ["verify", "verify-links"] {
        let report = assert_success(
            lab.command(verb, plan)
                .args(["--output"])
                .arg(package)
                .output()
                .unwrap(),
            verb,
        );
        for (field, value) in [
            (
                "verification_profile",
                forever_extractor::plan::PARTIAL_CATALOG_TERRAIN_VERIFY_PROFILE,
            ),
            (
                "scope",
                forever_extractor::plan::PARTIAL_CATALOG_TERRAIN_VERIFY_SCOPE,
            ),
        ] {
            let identity = format!("\"{field}\":\"{value}\"");
            assert!(
                report.contains(&identity),
                "{verb} omitted {identity}: {report}"
            );
        }
    }
}

fn client_context(package: &Path) -> Context {
    Context::parse(&fs::read_to_string(package.join(".state/context.json")).unwrap()).unwrap()
}

fn package_artifacts(package: &Path, context: Context) -> Vec<Artifact> {
    fs::read_dir(package.join(".state/files"))
        .unwrap()
        .map(|entry| {
            let text = fs::read_to_string(entry.unwrap().path()).unwrap();
            Artifact::parse(&text, context, Limits::default()).unwrap()
        })
        .collect()
}

fn artifact_inventory(package: &Path) -> Vec<String> {
    let context = client_context(package);
    let mut artifacts: Vec<_> = package_artifacts(package, context)
        .into_iter()
        .map(|artifact| artifact.json())
        .collect();
    artifacts.sort_unstable();
    artifacts
}

fn rebuilt_package(
    base: &Path,
    output: &Path,
    global_sections: Option<Vec<PartialEncryptedSection>>,
    map_sections: Option<Vec<PartialEncryptedSection>>,
    mutate_map: impl FnOnce(&mut Vec<u8>),
) {
    let context = client_context(base);
    let mut run = Run::create(output, context, &[base], Limits::default()).unwrap();
    if let Some(sections) = global_sections {
        run.set_partial_encrypted_sections(&sections).unwrap();
    }
    if let Some(sections) = map_sections {
        run.set_partial_encrypted_table(MAP_PATH, &sections)
            .unwrap();
    }
    let mut mutate_map = Some(mutate_map);
    for artifact in package_artifacts(base, context) {
        let mut bytes = fs::read(base.join(&artifact.path)).unwrap();
        if artifact.path == MAP_PATH {
            mutate_map.take().unwrap()(&mut bytes);
        }
        run.put(artifact.component, &artifact.path, &bytes).unwrap();
    }
    run.finish_component(Component::Db2).unwrap();
    run.finish_component(Component::Terrain).unwrap();
    run.finish().unwrap();
}

fn declaration() -> PartialEncryptedSection {
    PartialEncryptedSection {
        lookup: "1122334455667788".into(),
        frame_ordinal: 1,
        decoded_size: 138,
    }
}

fn assert_canonical_partial_manifest(manifest: &str) {
    let map_table = r#""partial_encrypted_tables":[{"path":"dbc/enUS/Map.db2","sections":[{"lookup":"1122334455667788","frame_ordinal":1,"decoded_size":138}]}]"#;
    assert!(manifest.contains(map_table), "{manifest}");
    assert!(!manifest.contains(r#""partial_encrypted_sections":"#));
    assert!(!manifest.contains(r#""path":"dbc/enUS/LiquidType.db2","sections":"#));
    assert_eq!(
        manifest
            .matches(r#""path":"dbc/enUS/Map.db2","sections":"#)
            .count(),
        1
    );
    assert_eq!(
        manifest.matches(r#""lookup":"1122334455667788""#).count(),
        1
    );
}

#[test]
fn v27_partial_catalog_terrain_extract_resume_verify_and_verify_links() {
    let lab = Lab::new();
    let plan = lab.plan();
    let client = lab.0.join("client");
    fs::create_dir(&client).unwrap();
    synthetic_client::write_partial_client(&client, b'S');
    let package = lab.0.join("package");
    let inspection = assert_success(
        lab.command("inspect", &plan).output().unwrap(),
        "v27 plan-only inspect",
    );
    for expected in [
        "\"contract\":\"forever-product-v27\"",
        "\"db2_files\":2",
        "\"terrain_source_profile\":\"linux70450-partial-catalog-authenticated-map-terrain-v1\"",
        "\"partial_db2_declarations\":\"per-table-including-Map\"",
        "\"full_extractor_complete\":false",
    ] {
        assert!(inspection.contains(expected), "{inspection}");
    }
    assert_success(
        lab.command("extract", &plan)
            .args(["--client"])
            .arg(&client)
            .args(["--output"])
            .arg(&package)
            .output()
            .unwrap(),
        "v27 partial extract",
    );
    assert!(package.join(MAP_PATH).is_file());
    assert!(package.join(LIQUID_TYPE_PATH).is_file());
    let original_manifest = fs::read_to_string(package.join("forever-package.json")).unwrap();
    assert_canonical_partial_manifest(&original_manifest);
    let original_inventory = artifact_inventory(&package);
    verify_package(&lab, &plan, &package);

    let resume_client = lab.0.join("resume-client");
    fs::create_dir(&resume_client).unwrap();
    let resume_wdt = synthetic_client::write_partial_client(&resume_client, b'S');
    let archive_path = resume_client.join("Data/data/data.000");
    let mut archive = fs::read(&archive_path).unwrap();
    let corrupt_at = resume_wdt as usize + 30 + 8 + 1 + 8;
    archive[corrupt_at] ^= 0xff;
    fs::write(&archive_path, archive).unwrap();
    let resumed_package = lab.0.join("resume-package");
    let interrupted = lab
        .command("extract", &plan)
        .args(["--client"])
        .arg(&resume_client)
        .args(["--output"])
        .arg(&resumed_package)
        .output()
        .unwrap();
    assert!(!interrupted.status.success());
    assert!(resumed_package.join(".state/sealed/db2").is_file());
    assert!(!resumed_package.join(".state/sealed/maps").exists());
    assert!(
        resumed_package
            .join(".forever-package-incomplete")
            .is_file()
    );
    assert!(!resumed_package.join("forever-package.json").exists());

    synthetic_client::write_partial_client(&resume_client, b'S');
    assert_success(
        lab.command("resume", &plan)
            .args(["--client"])
            .arg(&resume_client)
            .args(["--output"])
            .arg(&resumed_package)
            .output()
            .unwrap(),
        "v27 partial resume",
    );
    assert!(!resumed_package.join(".forever-package-incomplete").exists());
    verify_package(&lab, &plan, &resumed_package);
    let resumed_manifest =
        fs::read_to_string(resumed_package.join("forever-package.json")).unwrap();
    assert_eq!(resumed_manifest, original_manifest);
    assert_eq!(artifact_inventory(&resumed_package), original_inventory);
    for path in [MAP_PATH, LIQUID_TYPE_PATH] {
        let artifact = format!("\"component\":\"db2\",\"path\":\"{path}\"");
        assert_eq!(resumed_manifest.matches(&artifact).count(), 1);
    }
    assert_canonical_partial_manifest(&resumed_manifest);
}

#[test]
fn v27_rejects_global_undeclared_wrong_size_and_corrupt_zero_fill() {
    let lab = Lab::new();
    let plan = lab.plan();
    let client = lab.0.join("client");
    fs::create_dir(&client).unwrap();
    synthetic_client::write_partial_client(&client, b'S');
    let package = lab.0.join("package");
    assert_success(
        lab.command("extract", &plan)
            .args(["--client"])
            .arg(&client)
            .args(["--output"])
            .arg(&package)
            .output()
            .unwrap(),
        "v27 base partial extract",
    );
    let declaration = declaration();

    let global = lab.0.join("global-v25-declaration");
    rebuilt_package(
        &package,
        &global,
        Some(vec![declaration.clone()]),
        Some(vec![declaration.clone()]),
        |_| {},
    );
    assert_failure(
        lab.command("verify", &plan)
            .args(["--output"])
            .arg(&global)
            .output()
            .unwrap(),
        "product_partial_manifest_scope",
    );

    let undeclared = lab.0.join("undeclared-map");
    rebuilt_package(&package, &undeclared, None, None, |_| {});
    assert_failure(
        lab.command("verify", &plan)
            .args(["--output"])
            .arg(&undeclared)
            .output()
            .unwrap(),
        "product_partial_undeclared",
    );

    let mut wrong_size = declaration.clone();
    wrong_size.decoded_size = 137;
    let inconsistent = lab.0.join("wrong-decoded-size");
    rebuilt_package(
        &package,
        &inconsistent,
        None,
        Some(vec![wrong_size]),
        |_| {},
    );
    assert_failure(
        lab.command("verify", &plan)
            .args(["--output"])
            .arg(&inconsistent)
            .output()
            .unwrap(),
        "product_partial_section_count",
    );

    let (_, hidden_start, _) = db2_fixture::partial_map();
    let corrupted = lab.0.join("corrupted-zero-fill");
    rebuilt_package(
        &package,
        &corrupted,
        None,
        Some(vec![declaration]),
        |bytes| {
            assert_eq!(bytes[hidden_start], 0);
            bytes[hidden_start] = 1;
        },
    );
    assert_failure(
        lab.command("verify", &plan)
            .args(["--output"])
            .arg(&corrupted)
            .output()
            .unwrap(),
        "product_partial_zerofill",
    );
}

#[test]
fn v27_rejects_scratch_before_client_or_output_io() {
    let lab = Lab::new();
    let plan = lab.plan();
    let client = lab.0.join("absent-client");
    let scratch = lab.0.join("absent-scratch");
    let output = lab.0.join("output");
    assert_failure(
        lab.command("extract", &plan)
            .args(["--client"])
            .arg(&client)
            .args(["--output"])
            .arg(&output)
            .args(["--scratch"])
            .arg(&scratch)
            .output()
            .unwrap(),
        "product_scratch_forbidden",
    );
    assert!(!client.exists());
    assert!(!scratch.exists());
    assert!(!output.exists());
}
