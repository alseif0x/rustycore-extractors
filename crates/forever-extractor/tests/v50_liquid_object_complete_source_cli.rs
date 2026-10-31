// SPDX-License-Identifier: GPL-3.0-or-later
#![cfg(target_os = "linux")]

#[allow(dead_code)]
#[path = "v47_gameobject_expanded_material_filtered_wmo_geometry_cli/fixture.rs"]
mod fixture;
#[path = "v46_assembly_expanded_material_filtered_wmo_geometry_cli/json.rs"]
mod json;

use forever_casc::package::{Artifact, Component, Context, Limits};
use forever_db2::terrain_sources;
use json::JsonValue;
use std::{
    collections::BTreeMap,
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};

const V50_PLAN: &str =
    include_str!("../../../tests/contracts/fixtures/forever-product-v50/all.tsv");
const V49_PLAN: &str =
    include_str!("../../../tests/contracts/fixtures/forever-product-v49/all.tsv");
const LIQUID_OBJECT_PATH: &str = "dbc/esES/LiquidObject.db2";
const LIQUID_EVIDENCE_PATH: &str = "vmaps/liquid-object-complete-source-v1.bin";
static NEXT: AtomicU64 = AtomicU64::new(0);

const LIMITS: [(&str, u64); 10] = [
    ("source_bytes_max", 2_147_483_648),
    ("model_bytes_max", 536_870_912),
    ("placement_bytes_max", 268_435_456),
    ("placement_bytes_max_per_map", 67_108_864),
    ("models_max", 65_536),
    ("reads_max", 1_000_000),
    ("request_calls_max", 16_000_000),
    ("dependency_requests_max", 16_000_000),
    ("raw_bytes_max", 268_435_456),
    ("assembled_domain_bytes_max", 268_435_456),
];

struct Lab(PathBuf);

impl Lab {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "rce-v50-liquid-object-cli-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        fs::write(root.join("plan-v50.tsv"), V50_PLAN).unwrap();
        fs::write(root.join("plan-v49.tsv"), V49_PLAN).unwrap();
        Self(root)
    }

    fn private_dir(&self, name: &str) -> PathBuf {
        let path = self.0.join(name);
        fs::create_dir(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        path
    }

    fn command(&self, verb: &str, version: u8) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_forever-extractor"));
        command
            .current_dir(&self.0)
            .env_remove("FOREVER_PRODUCT_DIAGNOSTIC")
            .args([verb, "--plan"])
            .arg(self.0.join(format!("plan-v{version}.tsv")));
        command
    }

    fn run_product(&self, verb: &str, client: &Path, package: &Path) -> Output {
        let scratch = self.private_dir(&format!(
            "{verb}-scratch-{}",
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        self.command(verb, 50)
            .args(["--client"])
            .arg(client)
            .args(["--output"])
            .arg(package)
            .args(["--scratch"])
            .arg(scratch)
            .output()
            .unwrap()
    }

    fn verify(&self, verb: &str, version: u8, package: &Path) -> Output {
        self.command(verb, version)
            .args(["--output"])
            .arg(package)
            .output()
            .unwrap()
    }

    fn client(&self, name: &str, rows: &[(u32, i16)], copies: &[(u32, u32)]) -> PathBuf {
        let client = self.private_dir(&format!("{name}-client"));
        fixture::write_v50_client(&client, rows, copies, false, false);
        client
    }

    fn package(&self, name: &str, client: &Path) -> PathBuf {
        let package = self.0.join(format!("{name}-package"));
        let output = self.run_product("extract", client, &package);
        success_json(output, "extract", Some(&package));
        package
    }
}

impl Drop for Lab {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn success_json(output: Output, verb: &str, package: Option<&Path>) -> BTreeMap<String, JsonValue> {
    assert!(
        output.status.success(),
        "{verb} exited {:?}: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty(), "{verb} wrote stderr");
    let report = String::from_utf8(output.stdout).unwrap();
    assert_eq!(report.matches('\n').count(), 1, "{verb}: {report}");
    let JsonValue::Object(fields) = json::parse(&report).unwrap() else {
        panic!("{verb} JSON root is not an object: {report}");
    };
    assert_eq!(
        fields.get("contract"),
        Some(&JsonValue::String("forever-product-v50".into())),
        "{report}"
    );
    assert_eq!(
        fields.get("full_extractor_complete"),
        Some(&JsonValue::Scalar),
        "{report}"
    );
    assert!(
        report.contains("\"full_extractor_complete\":false"),
        "{report}"
    );
    assert_eq!(
        fields.get("collision_source_profile"),
        Some(&JsonValue::String(
            "linux70450-liquid-object-complete-source-vmap-product-v1".into()
        )),
        "{report}"
    );
    assert_eq!(
        fields.get("coverage_profile"),
        Some(&JsonValue::String(
            "linux70450-liquid-object-complete-source-coverage-v23".into()
        )),
        "{report}"
    );
    assert_eq!(
        fields.get("codec_profile"),
        Some(&JsonValue::String(
            "linux70450-liquid-object-complete-source-v1".into()
        )),
        "{report}"
    );
    assert_eq!(
        fields.get("domain_profile"),
        Some(&JsonValue::String(
            "liquid-object-complete-source-v1".into()
        )),
        "{report}"
    );
    assert_eq!(
        fields.get("verification_profile"),
        Some(&JsonValue::String(
            "linux70450-liquid-object-complete-source-verification-v1".into()
        )),
        "{report}"
    );
    assert_eq!(
        fields.get("verification_scope"),
        Some(&JsonValue::String(
            "sealed_liquid_object_source_facts_and_v49_available_map_geometry".into()
        )),
        "{report}"
    );
    for (key, value) in LIMITS {
        assert_eq!(
            fields.get(key),
            Some(&JsonValue::Number(value.to_string())),
            "{verb}: {key}: {report}"
        );
    }
    if let Some(package) = package {
        assert!(package.is_dir(), "{verb} did not create package");
        assert!(fields.contains_key("package"), "{verb}: {report}");
    }
    fields
}

fn package_artifacts(package: &Path) -> BTreeMap<String, (Component, Vec<u8>)> {
    let context =
        Context::parse(&fs::read_to_string(package.join(".state/context.json")).unwrap()).unwrap();
    fs::read_dir(package.join(".state/files"))
        .unwrap()
        .map(|entry| {
            let path = entry.unwrap().path();
            let artifact = Artifact::parse(
                &fs::read_to_string(path).unwrap(),
                context,
                Limits::default(),
            )
            .unwrap();
            let bytes = fs::read(package.join(&artifact.path)).unwrap();
            (artifact.path, (artifact.component, bytes))
        })
        .collect()
}

fn copy_tree(source: &Path, destination: &Path) {
    fs::create_dir(destination).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let from = entry.path();
        let to = destination.join(entry.file_name());
        if from.is_dir() {
            copy_tree(&from, &to);
        } else {
            fs::copy(from, to).unwrap();
        }
    }
}

fn remove_artifact(package: &Path, path: &str) {
    let context =
        Context::parse(&fs::read_to_string(package.join(".state/context.json")).unwrap()).unwrap();
    let index = package.join(".state/files");
    for entry in fs::read_dir(&index).unwrap() {
        let entry = entry.unwrap();
        let artifact = Artifact::parse(
            &fs::read_to_string(entry.path()).unwrap(),
            context,
            Limits::default(),
        )
        .unwrap();
        if artifact.path == path {
            fs::remove_file(package.join(path)).unwrap();
            fs::remove_file(entry.path()).unwrap();
        }
    }
}

#[test]
fn v50_cli_lifecycle_liquid_object_evidence_and_profile_isolation() {
    let lab = Lab::new();
    let inspect = lab.command("inspect", 50).output().unwrap();
    success_json(inspect, "inspect", None);

    let rows = [(7, -1), (9, i16::MIN), (7, 513)];
    let copies = [(11, 7), (12, 11), (7, 9), (20, 99), (30, 30)];
    let client = lab.client("main", &rows, &copies);
    let package = lab.package("main", &client);
    let artifacts = package_artifacts(&package);
    let liquid = &artifacts[LIQUID_OBJECT_PATH].1;
    let parsed = terrain_sources::read_liquid_objects_70450(liquid).unwrap();
    assert_eq!(parsed.direct_records().len(), 3);
    assert_eq!(parsed.direct_records()[0].fields.liquid_type_id, u16::MAX);
    assert_eq!(parsed.direct_records()[1].fields.liquid_type_id, 32768);
    assert_eq!(parsed.direct_records()[2].fields.liquid_type_id, 513);
    assert_eq!(parsed.copies().len(), copies.len());
    let materialized = parsed.materialize().unwrap();
    for (id, expected) in [
        (7, 32768),
        (9, 32768),
        (11, 513),
        (12, 513),
        (20, 0),
        (30, 0),
        (99, 0),
    ] {
        assert_eq!(materialized[&id].liquid_type_id, expected, "id={id}");
    }
    assert_eq!(artifacts[LIQUID_EVIDENCE_PATH].0, Component::Collision);
    assert_eq!(artifacts[LIQUID_EVIDENCE_PATH].1.len(), 112);

    for verb in ["verify", "verify-links"] {
        success_json(lab.verify(verb, 50, &package), verb, Some(&package));
        assert!(
            !lab.verify(verb, 49, &package).status.success(),
            "v49 accepted v50 package via {verb}"
        );
    }
    let v49_against_v50 = lab
        .command("extract", 49)
        .arg("--client")
        .arg(&client)
        .arg("--output")
        .arg(&package)
        .arg("--scratch")
        .arg(lab.private_dir("crossed-v49-scratch"))
        .output()
        .unwrap();
    assert!(
        !v49_against_v50.status.success(),
        "v49 resume/extract accepted existing v50 package"
    );

    let second = lab.package("second", &client);
    assert_eq!(package_artifacts(&package), package_artifacts(&second));

    let resumed = lab.0.join("resumed-package");
    copy_tree(&package, &resumed);
    fs::remove_file(resumed.join("forever-package.json")).unwrap();
    fs::remove_file(resumed.join(".state/sealed/vmaps")).unwrap();
    remove_artifact(
        &resumed,
        "vmaps/available-gameobject-root-encrypted-unavailable-material-filtered-wmo-geometry-coverage-v22.bin",
    );
    remove_artifact(&resumed, "vmaps/GameObjectModels.dtree");
    remove_artifact(&resumed, LIQUID_EVIDENCE_PATH);
    for path in artifacts
        .keys()
        .filter(|path| path.starts_with("vmaps/") && path.ends_with(".vmo"))
    {
        remove_artifact(&resumed, path);
    }
    fs::write(resumed.join(".forever-package-incomplete"), b"incomplete\n").unwrap();
    success_json(
        lab.run_product("resume", &client, &resumed),
        "resume",
        Some(&resumed),
    );
    assert_eq!(package_artifacts(&package), package_artifacts(&resumed));

    let v49_package = {
        let path = lab.0.join("v49-package");
        let scratch = lab.private_dir("v49-scratch");
        let output = lab
            .command("extract", 49)
            .arg("--client")
            .arg(&client)
            .arg("--output")
            .arg(&path)
            .arg("--scratch")
            .arg(scratch)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "v49 geometry baseline: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        path
    };
    let v49_files = package_artifacts(&v49_package);
    let v50_geometry: BTreeMap<_, _> = artifacts
        .iter()
        .filter(|(path, (component, _))| {
            *component == Component::Collision
                && !path.contains("coverage")
                && *path != LIQUID_EVIDENCE_PATH
        })
        .map(|(path, (_, bytes))| (path.as_str(), bytes.as_slice()))
        .collect();
    let v49_geometry: BTreeMap<_, _> = v49_files
        .iter()
        .filter(|(path, (component, _))| {
            *component == Component::Collision && !path.contains("coverage")
        })
        .map(|(path, (_, bytes))| (path.as_str(), bytes.as_slice()))
        .collect();
    assert_eq!(
        v50_geometry, v49_geometry,
        "shared v49 geometry changed under v50"
    );
    for verb in ["verify", "verify-links"] {
        assert!(
            !lab.verify(verb, 50, &v49_package).status.success(),
            "v50 accepted v49 package via {verb}"
        );
    }
}

#[test]
fn v50_rejects_missing_partial_and_malformed_liquid_object_sources_before_package_publish() {
    let lab = Lab::new();
    let cases = [
        ("partial", false, true, false),
        ("trailing", false, false, true),
        ("missing", true, false, false),
    ];
    for (name, missing, partial, trailing) in cases {
        let client = lab.private_dir(&format!("{name}-client"));
        if missing {
            fixture::write_v50_client_without_liquid_object(&client);
        } else {
            fixture::write_v50_client(&client, &[(7, 1)], &[], partial, trailing);
        }
        let package = lab.0.join(format!("{name}-package"));
        let result = lab.run_product("extract", &client, &package);
        assert!(
            !result.status.success(),
            "{name} LiquidObject source unexpectedly extracted"
        );
        assert!(
            !package.join("forever-package.json").exists(),
            "{name} source published package"
        );
    }
}

#[test]
fn v50_capability_values_are_exact_numeric_and_not_generic_profiles() {
    let lab = Lab::new();
    let output = lab.command("inspect", 50).output().unwrap();
    let fields = success_json(output, "inspect", None);
    for (name, value) in LIMITS {
        assert_eq!(
            fields.get(name),
            Some(&JsonValue::Number(value.to_string()))
        );
    }
    assert_eq!(
        fields.get("resource_profile"),
        Some(&JsonValue::String(
            "linux-collision-gameobject-root-encrypted-unavailable-expanded-resources-v1".into()
        ))
    );
    assert_eq!(
        fields.get("liquid_object_scope"),
        Some(&JsonValue::String("complete_source".into()))
    );
}
