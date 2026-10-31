// SPDX-License-Identifier: GPL-3.0-or-later
#![cfg(target_os = "linux")]

#[allow(dead_code)]
#[path = "v47_gameobject_expanded_material_filtered_wmo_geometry_cli/fixture.rs"]
mod fixture;
#[path = "v46_assembly_expanded_material_filtered_wmo_geometry_cli/json.rs"]
mod json;

use forever_casc::package::{Artifact, Component, Context, Limits};
use std::{
    collections::BTreeMap,
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
const PLAN: &str = include_str!("../../../tests/contracts/fixtures/forever-product-v51/all.tsv");
const V25: &str = include_str!("../../../tests/contracts/fixtures/forever-product-v25/all.tsv");
const V50: &str = include_str!("../../../tests/contracts/fixtures/forever-product-v50/all.tsv");
const GEOMETRY: &str = "vmaps/available-db2-terrain-liquid-gameobject-root-encrypted-unavailable-geometry-coverage-v1.bin";
const TERRAIN: &str = "maps/compatible-terrain-coverage-v3.tsv";
const LIQUID: &str = "vmaps/liquid-object-complete-source-v1.bin";
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
            "rce-v51-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        for (version, plan) in [(51, PLAN), (25, V25), (50, V50)] {
            fs::write(
                root.join(format!("v{version}.tsv")),
                plan.replace("locale\tenUS", "locale\tesES"),
            )
            .unwrap();
        }
        Self(root)
    }
    fn command(&self, verb: &str, version: u8, package: &Path) -> Command {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_forever-extractor"));
        cmd.current_dir(&self.0)
            .env_remove("FOREVER_PRODUCT_DIAGNOSTIC")
            .args([verb, "--plan"])
            .arg(self.0.join(format!("v{version}.tsv")));
        if verb != "inspect" {
            cmd.arg("--output").arg(package);
        }
        cmd
    }
    fn run(&self, verb: &str, version: u8, client: &Path, package: &Path) -> Output {
        let mut cmd = self.command(verb, version, package);
        cmd.arg("--client").arg(client);
        if version != 25 {
            let scratch = self
                .0
                .join(format!("scratch-{}", NEXT.fetch_add(1, Ordering::Relaxed)));
            fs::create_dir(&scratch).unwrap();
            fs::set_permissions(&scratch, fs::Permissions::from_mode(0o700)).unwrap();
            cmd.arg("--scratch").arg(scratch);
        }
        if version == 51 {
            cmd.env(
                "FOREVER_PRODUCT_DIAGNOSTIC",
                self.0.join("diagnostic.jsonl"),
            );
        }
        cmd.output().unwrap()
    }
    fn client(&self, name: &str, partial: Option<&str>) -> PathBuf {
        let path = self.0.join(name);
        fs::create_dir(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        fixture::write_v51_client(&path, partial);
        path
    }
}
impl Drop for Lab {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn success(output: Output) {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    let text = String::from_utf8(output.stdout).unwrap();
    let json::JsonValue::Object(fields) = json::parse(&text).unwrap() else {
        panic!("{text}");
    };
    assert_eq!(
        fields.get("contract"),
        Some(&json::JsonValue::String("forever-product-v51".into()))
    );
    assert!(text.contains("\"full_extractor_complete\":false"));
    for (name, value) in LIMITS {
        assert_eq!(
            fields.get(name),
            Some(&json::JsonValue::Number(value.to_string())),
            "{name}: {text}"
        );
    }
    for (name, value) in [
        (
            "collision_source_profile",
            "linux70450-db2-terrain-liquid-vmap-product-v1",
        ),
        (
            "coverage_profile",
            "linux70450-db2-terrain-liquid-vmap-coverage-v1",
        ),
        (
            "codec_profile",
            "linux70450-db2-terrain-liquid-vmap-geometry-v1",
        ),
        (
            "domain_profile",
            "db2-terrain-liquid-vmap-available-map-domain-v1",
        ),
        (
            "verification_profile",
            "linux70450-db2-terrain-liquid-vmap-verification-v1",
        ),
    ] {
        assert_eq!(
            fields.get(name),
            Some(&json::JsonValue::String(value.into())),
            "{text}"
        );
    }
}
fn artifacts(package: &Path) -> BTreeMap<String, (Component, Vec<u8>)> {
    let context =
        Context::parse(&fs::read_to_string(package.join(".state/context.json")).unwrap()).unwrap();
    fs::read_dir(package.join(".state/files"))
        .unwrap()
        .map(|entry| {
            let artifact = Artifact::parse(
                &fs::read_to_string(entry.unwrap().path()).unwrap(),
                context,
                Limits::default(),
            )
            .unwrap();
            let bytes = fs::read(package.join(&artifact.path)).unwrap();
            (artifact.path, (artifact.component, bytes))
        })
        .collect()
}
#[test]
fn v51_lifecycle_parity_cross_profiles_and_determinism() {
    let lab = Lab::new();
    let client = lab.client("client", None);
    let package = lab.0.join("package");
    success(lab.command("inspect", 51, &package).output().unwrap());
    success(lab.run("extract", 51, &client, &package));
    assert!(!lab.0.join("diagnostic.jsonl").exists());
    let expected = artifacts(&package);
    let context =
        Context::parse(&fs::read_to_string(package.join(".state/context.json")).unwrap()).unwrap();
    assert_eq!(context.components, 25);
    assert_eq!(
        expected
            .values()
            .filter(|(c, _)| *c == Component::Db2)
            .count(),
        5
    );
    assert!(expected.keys().any(|p| p.ends_with(".map")));
    assert_eq!(&expected[LIQUID].1[..8], b"RCLIQ01\0");
    assert_eq!(
        u32::from_le_bytes(expected[GEOMETRY].1[8..12].try_into().unwrap()),
        22
    );
    for verb in ["verify", "verify-links"] {
        success(lab.command(verb, 51, &package).output().unwrap());
    }
    let second = lab.0.join("second");
    success(lab.run("extract", 51, &client, &second));
    assert_eq!(expected, artifacts(&second));
    fs::remove_file(second.join("forever-package.json")).unwrap();
    for component in ["maps", "vmaps"] {
        fs::remove_file(second.join(format!(".state/sealed/{component}"))).unwrap();
    }
    for entry in fs::read_dir(second.join(".state/files")).unwrap() {
        let path = entry.unwrap().path();
        let artifact = Artifact::parse(
            &fs::read_to_string(&path).unwrap(),
            context,
            Limits::default(),
        )
        .unwrap();
        if [TERRAIN, GEOMETRY, LIQUID].contains(&artifact.path.as_str()) {
            fs::remove_file(second.join(&artifact.path)).unwrap();
            fs::remove_file(path).unwrap();
        }
    }
    fs::write(second.join(".forever-package-incomplete"), b"incomplete\n").unwrap();
    success(lab.run("resume", 51, &client, &second));
    assert_eq!(expected, artifacts(&second));
    for version in [25, 50] {
        let baseline = lab.0.join(format!("baseline-{version}"));
        let result = lab.run("extract", version, &client, &baseline);
        assert!(
            result.status.success(),
            "baseline {version}: {}",
            String::from_utf8_lossy(&result.stderr)
        );
        for verb in ["verify", "verify-links"] {
            assert!(
                !lab.command(verb, version, &package)
                    .output()
                    .unwrap()
                    .status
                    .success()
            );
            assert!(
                !lab.command(verb, 51, &baseline)
                    .output()
                    .unwrap()
                    .status
                    .success()
            );
        }
        assert!(
            !lab.run("resume", version, &client, &package)
                .status
                .success()
        );
        assert!(!lab.run("resume", 51, &client, &baseline).status.success());
        for (path, value) in artifacts(&baseline) {
            if path.contains("coverage") {
                continue;
            }
            assert_eq!(expected.get(&path), Some(&value), "v{version}: {path}");
        }
        if version == 25 {
            let coverage = fs::read_to_string(baseline.join(TERRAIN)).unwrap();
            let combined = String::from_utf8(expected[TERRAIN].1.clone()).unwrap();
            assert_eq!(
                coverage
                    .lines()
                    .filter(|line| !line.starts_with("context\t"))
                    .collect::<Vec<_>>(),
                combined
                    .lines()
                    .filter(|line| !line.starts_with("context\t"))
                    .collect::<Vec<_>>()
            );
        }
    }
}
#[test]
fn v51_sealed_partial_liquid_declarations_and_tampered_payloads_are_rejected() {
    let lab = Lab::new();
    let client = lab.client("client", None);
    let package = lab.0.join("package");
    success(lab.run("extract", 51, &client, &package));
    let context =
        Context::parse(&fs::read_to_string(package.join(".state/context.json")).unwrap()).unwrap();
    let original = artifacts(&package);
    for table in ["LiquidObject", "LiquidMaterial", "LiquidType"] {
        let forged = lab.0.join(format!("forged-{table}"));
        let mut run =
            forever_casc::package::Run::create(&forged, context, &[], Limits::default()).unwrap();
        run.set_partial_encrypted_table(
            &format!("dbc/esES/{table}.db2"),
            &[forever_casc::package::PartialEncryptedSection {
                lookup: "1122334455667788".into(),
                frame_ordinal: 1,
                decoded_size: 4,
            }],
        )
        .unwrap();
        for (path, (component, bytes)) in &original {
            run.put(*component, path, bytes).unwrap();
        }
        for component in [Component::Db2, Component::Terrain, Component::Collision] {
            run.finish_component(component).unwrap();
        }
        run.finish().unwrap();
        for verb in ["verify", "verify-links"] {
            assert!(
                !lab.command(verb, 51, &forged)
                    .output()
                    .unwrap()
                    .status
                    .success(),
                "{table}: {verb}"
            );
        }
        assert!(!lab.run("resume", 51, &client, &forged).status.success());
    }
    for path in [
        TERRAIN,
        GEOMETRY,
        LIQUID,
        "dbc/esES/LiquidMaterial.db2",
        "dbc/esES/LiquidType.db2",
    ] {
        let bytes = &original[path].1;
        let mut corrupt = bytes.clone();
        corrupt[0] ^= 1;
        fs::write(package.join(path), corrupt).unwrap();
        for verb in ["verify", "verify-links"] {
            assert!(
                !lab.command(verb, 51, &package)
                    .output()
                    .unwrap()
                    .status
                    .success(),
                "{path}: {verb}"
            );
        }
        fs::write(package.join(path), bytes).unwrap();
    }
}

#[test]
fn v51_partial_liquid_sources_are_fatal() {
    let lab = Lab::new();
    for table in ["LiquidObject", "LiquidMaterial", "LiquidType"] {
        let client = lab.client(table, Some(table));
        let package = lab.0.join(format!("{table}-package"));
        let output = lab.run("extract", 51, &client, &package);
        assert!(!output.status.success(), "partial {table} admitted");
        assert!(!package.join("forever-package.json").exists());
        let diagnostic = fs::read_to_string(lab.0.join("diagnostic.jsonl")).unwrap();
        assert!(diagnostic.contains("\"request_kind\":\"db2_table\""));
        assert!(diagnostic.contains("\"phase\":\"db2\""));
        assert!(!diagnostic.contains(client.to_str().unwrap()));
        fs::remove_file(lab.0.join("diagnostic.jsonl")).unwrap();
    }
}
