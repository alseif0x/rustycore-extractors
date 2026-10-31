// SPDX-License-Identifier: GPL-3.0-or-later
#![cfg(target_os = "linux")]

#[allow(dead_code)]
#[path = "v47_gameobject_expanded_material_filtered_wmo_geometry_cli/coverage.rs"]
mod coverage;
#[allow(dead_code)]
#[path = "v47_gameobject_expanded_material_filtered_wmo_geometry_cli/fixture.rs"]
mod fixture;
#[path = "v46_assembly_expanded_material_filtered_wmo_geometry_cli/json.rs"]
mod json;

use forever_casc::package::{Component, Context, Limits, Run};
use std::{
    collections::BTreeMap,
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};

const PLAN: &str = include_str!("../../../tests/contracts/fixtures/forever-product-v49/all.tsv");
const V48_PLAN: &str =
    include_str!("../../../tests/contracts/fixtures/forever-product-v48/all.tsv");
const V47_PLAN: &str =
    include_str!("../../../tests/contracts/fixtures/forever-product-v47/all.tsv");
const COVERAGE_PATH: &str = "vmaps/available-gameobject-root-encrypted-unavailable-material-filtered-wmo-geometry-coverage-v22.bin";
static NEXT: AtomicU64 = AtomicU64::new(0);

struct Lab(PathBuf);

impl Lab {
    fn new() -> Self {
        let root = std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join(format!(
                ".local/share/rce-v49-gameobject-encrypted-unavailable-cli-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
        fs::create_dir_all(root.parent().unwrap()).unwrap();
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        fs::write(root.join("plan-v49.tsv"), PLAN).unwrap();
        fs::write(root.join("plan-v48.tsv"), V48_PLAN).unwrap();
        fs::write(root.join("plan-v47.tsv"), V47_PLAN).unwrap();
        Self(root)
    }

    fn private_dir(&self, name: &str) -> PathBuf {
        let path = self.0.join(name);
        fs::create_dir(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        path
    }

    fn command(&self, verb: &str, version: u32) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_forever-extractor"));
        command
            .current_dir(&self.0)
            .env_remove("FOREVER_PRODUCT_DIAGNOSTIC")
            .args([verb, "--plan"])
            .arg(self.0.join(format!("plan-v{version}.tsv")));
        command
    }

    fn produce(&self, verb: &str, version: u32, client: &Path, output: &Path) -> Output {
        let scratch = self.private_dir(&format!(
            "{verb}-{version}-scratch-{}",
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        self.command(verb, version)
            .args(["--client"])
            .arg(client)
            .args(["--output"])
            .arg(output)
            .args(["--scratch"])
            .arg(scratch)
            .output()
            .unwrap()
    }

    fn verify(&self, verb: &str, version: u32, package: &Path) -> Output {
        self.command(verb, version)
            .args(["--output"])
            .arg(package)
            .output()
            .unwrap()
    }

    fn extract(&self, name: &str, missing: &[u32], encrypted: &[u32]) -> PathBuf {
        self.extract_with_partial_godi(name, missing, encrypted, false)
    }

    fn extract_with_partial_godi(
        &self,
        name: &str,
        missing: &[u32],
        encrypted: &[u32],
        partial_godi: bool,
    ) -> PathBuf {
        let client = self.private_dir(&format!("{name}-client"));
        fixture::write_v49_client_with_partial_godi(&client, missing, encrypted, partial_godi);
        let package = self.0.join(format!("{name}-package"));
        let result = self.produce("extract", 49, &client, &package);
        assert_cli_json(result, "extract", Some(&package));
        package
    }

    fn produce_with_diagnostic(
        &self,
        verb: &str,
        version: u32,
        client: &Path,
        output: &Path,
        diagnostic: &Path,
    ) -> Output {
        let scratch = self.private_dir(&format!(
            "{verb}-{version}-diagnostic-scratch-{}",
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        self.command(verb, version)
            .env("FOREVER_PRODUCT_DIAGNOSTIC", diagnostic)
            .args(["--client"])
            .arg(client)
            .args(["--output"])
            .arg(output)
            .args(["--scratch"])
            .arg(scratch)
            .output()
            .unwrap()
    }

    fn extract_legacy(&self, name: &str, version: u32) -> PathBuf {
        let client = self.private_dir(&format!("{name}-client"));
        if version == 48 {
            fixture::write_v48_client(&client, &[]);
        } else {
            fixture::write_client(&client, true);
        }
        let package = self.0.join(format!("{name}-package"));
        let result = self.produce("extract", version, &client, &package);
        assert!(
            result.status.success(),
            "v{version} extract exited {:?}: {}",
            result.status.code(),
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(package.is_dir());
        package
    }

    fn resume(&self, package: &Path, missing: &[u32], encrypted: &[u32]) -> Output {
        self.resume_with_partial_godi(package, missing, encrypted, false)
    }

    fn resume_with_partial_godi(
        &self,
        package: &Path,
        missing: &[u32],
        encrypted: &[u32],
        partial_godi: bool,
    ) -> Output {
        let client = self.private_dir("resume-client");
        fixture::write_v49_client_with_partial_godi(&client, missing, encrypted, partial_godi);
        self.produce("resume", 49, &client, package)
    }

    fn rebuilt_with_coverage(&self, base: &Path, name: &str, bytes: &[u8]) -> PathBuf {
        self.rebuilt_with_coverage_and_omissions(base, name, bytes, None)
    }

    fn rebuilt_with_coverage_and_omissions(
        &self,
        base: &Path,
        name: &str,
        bytes: &[u8],
        omissions: Option<&[Vec<u32>; 5]>,
    ) -> PathBuf {
        let context =
            Context::parse(&fs::read_to_string(base.join(".state/context.json")).unwrap()).unwrap();
        let mut files = package_files(base);
        let coverage_bytes = match omissions {
            Some(omissions) => coverage::replace_omissions(bytes, omissions).unwrap(),
            None => bytes.to_vec(),
        };
        files.insert(COVERAGE_PATH.to_owned(), coverage_bytes);
        let output = self.0.join(name);
        let mut run = Run::create(&output, context, &[base], Limits::default()).unwrap();
        for (path, bytes) in files {
            let component = if path.starts_with("dbc/") {
                Component::Db2
            } else {
                Component::Collision
            };
            run.put(component, &path, &bytes).unwrap();
        }
        run.finish_component(Component::Db2).unwrap();
        run.finish_component(Component::Collision).unwrap();
        run.finish().unwrap();
        output
    }
}

impl Drop for Lab {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn assert_success(output: Output, verb: &str) -> String {
    assert!(
        output.status.success(),
        "{verb} exited {:?}: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty(), "{verb} wrote stderr");
    let report = String::from_utf8(output.stdout).unwrap();
    assert_eq!(report.matches('\n').count(), 1, "{verb}: {report}");
    let parsed = json::parse(&report)
        .unwrap_or_else(|error| panic!("{verb}: invalid JSON ({error}): {report}"));
    assert!(
        matches!(parsed, json::JsonValue::Object(_)),
        "{verb}: {report}"
    );
    assert!(
        report.contains("\"contract\":\"forever-product-v49\""),
        "{report}"
    );
    assert!(
        report.contains("\"gameobject_encrypted_unavailable_max\":4096"),
        "{report}"
    );
    assert!(
        report.contains("\"full_extractor_complete\":false"),
        "{report}"
    );
    report
}

fn assert_cli_json(output: Output, verb: &str, package: Option<&Path>) -> String {
    let report = assert_success(output, verb);
    if let Some(package) = package {
        assert!(package.is_dir());
        assert!(report.contains("\"package\":"));
    }
    report
}

fn package_files(package: &Path) -> BTreeMap<String, Vec<u8>> {
    let context =
        Context::parse(&fs::read_to_string(package.join(".state/context.json")).unwrap()).unwrap();
    fs::read_dir(package.join(".state/files"))
        .unwrap()
        .map(|entry| {
            let path = entry.unwrap().path();
            let artifact = forever_casc::package::Artifact::parse(
                &fs::read_to_string(path).unwrap(),
                context,
                Limits::default(),
            )
            .unwrap();
            let bytes = fs::read(package.join(&artifact.path)).unwrap();
            (artifact.path, bytes)
        })
        .collect()
}

fn assert_v49_verify(lab: &Lab, package: &Path) {
    for verb in ["verify", "verify-links"] {
        let report = assert_cli_json(lab.verify(verb, 49, package), verb, Some(package));
        assert!(report.contains("artifact_integrity_gameobject_root_and_encrypted_unavailable"));
    }
}

#[test]
fn v49_inspect_extract_verify_resume_determinism_and_cross_rejection() {
    let lab = Lab::new();
    let inspect = lab.command("inspect", 49).output().unwrap();
    let inspect = assert_cli_json(inspect, "inspect", None);
    assert!(inspect.contains(
        "linux70450-gameobject-root-encrypted-unavailable-material-filtered-wmo-coverage-v22"
    ));
    assert!(inspect.contains("available-fdid-direct-root-absent-or-encrypted-unavailable-v1"));

    let missing = [50_000];
    let encrypted = [50_001];
    let package = lab.extract("first", &missing, &encrypted);
    let files = package_files(&package);
    let coverage_bytes = &files[COVERAGE_PATH];
    let parsed = coverage::parse_v49(coverage_bytes).unwrap();
    assert_eq!(parsed.version, 22);
    assert_eq!(parsed.gameobjects.missing_root_fdids, missing);
    assert_eq!(parsed.gameobjects.encrypted_unavailable_fdids, encrypted);
    assert_eq!(parsed.gameobjects.missing_root_rows, 1);
    assert_eq!(parsed.gameobjects.encrypted_unavailable_rows, 1);
    assert_eq!(parsed.gameobjects.candidate_requests, 9);
    assert_eq!(
        parsed
            .gameobjects
            .outcomes
            .iter()
            .find(|outcome| outcome.file_data_id == missing[0])
            .unwrap()
            .disposition,
        5
    );
    assert_eq!(
        parsed
            .gameobjects
            .outcomes
            .iter()
            .find(|outcome| outcome.file_data_id == encrypted[0])
            .unwrap()
            .disposition,
        6
    );
    assert!(!files.contains_key("vmaps/FILE0000C350.xxx.vmo"));
    assert!(!files.contains_key("vmaps/FILE0000C351.xxx.vmo"));
    assert_v49_verify(&lab, &package);
    for version in [48, 47] {
        for verb in ["verify", "verify-links"] {
            assert!(
                !lab.verify(verb, version, &package).status.success(),
                "v{version} {verb} accepted the v49 package"
            );
        }
        let legacy = lab.extract_legacy(&format!("v{version}-package",), version);
        for verb in ["verify", "verify-links"] {
            assert!(
                !lab.verify(verb, 49, &legacy).status.success(),
                "v49 {verb} accepted the v{version} package"
            );
        }
    }

    let second = lab.extract("second", &missing, &encrypted);
    assert_eq!(files, package_files(&second));
    let resumed = lab.0.join("resumed-package");
    copy_tree(&package, &resumed);
    fs::remove_file(resumed.join("forever-package.json")).unwrap();
    fs::remove_file(resumed.join(".state/sealed/vmaps")).unwrap();
    remove_artifact(&resumed, COVERAGE_PATH);
    remove_artifact(&resumed, "vmaps/GameObjectModels.dtree");
    for path in files
        .keys()
        .filter(|path| path.starts_with("vmaps/") && path.ends_with(".vmo"))
    {
        remove_artifact(&resumed, path);
    }
    fs::write(resumed.join(".forever-package-incomplete"), b"incomplete\n").unwrap();
    assert_cli_json(
        lab.resume(&resumed, &missing, &encrypted),
        "resume",
        Some(&resumed),
    );
    assert_eq!(files, package_files(&resumed));
    assert_v49_verify(&lab, &resumed);
}

#[test]
fn v49_partial_godi_and_partial_resume_keep_root_outcomes() {
    let lab = Lab::new();
    let missing = [50_000];
    let encrypted = [50_001];
    let client = lab.private_dir("partial-godi-client");
    fixture::write_v49_client_with_partial_godi(&client, &missing, &encrypted, true);
    let package = lab.0.join("partial-godi-package");
    let diagnostic = lab.0.join("partial-godi-diagnostic.jsonl");
    let extracted = lab.produce_with_diagnostic("extract", 49, &client, &package, &diagnostic);
    assert!(
        extracted.status.success(),
        "partial v49 extraction: {}",
        String::from_utf8_lossy(&extracted.stderr)
    );
    let manifest = fs::read_to_string(package.join("forever-package.json")).unwrap();
    let declaration = fixture::partial_godi_declaration();
    let expected = format!(
        "\"partial_encrypted_tables\":[{{\"path\":\"dbc/esES/GameObjectDisplayInfo.db2\",\"sections\":[{{\"lookup\":\"{}\",\"frame_ordinal\":{},\"decoded_size\":{}}}]}}]",
        declaration.lookup, declaration.frame_ordinal, declaration.decoded_size
    );
    assert!(manifest.contains(&expected), "{manifest}");
    let files = package_files(&package);
    let parsed = coverage::parse_v49(&files[COVERAGE_PATH]).unwrap();
    assert_eq!(parsed.gameobjects.missing_root_fdids, missing);
    assert_eq!(parsed.gameobjects.encrypted_unavailable_fdids, encrypted);
    assert_v49_verify(&lab, &package);

    let resumed = lab.0.join("partial-godi-resumed-package");
    copy_tree(&package, &resumed);
    fs::remove_file(resumed.join("forever-package.json")).unwrap();
    fs::remove_file(resumed.join(".state/sealed/vmaps")).unwrap();
    remove_artifact(&resumed, COVERAGE_PATH);
    remove_artifact(&resumed, "vmaps/GameObjectModels.dtree");
    for path in files
        .keys()
        .filter(|path| path.starts_with("vmaps/") && path.ends_with(".vmo"))
    {
        remove_artifact(&resumed, path);
    }
    fs::write(resumed.join(".forever-package-incomplete"), b"incomplete\n").unwrap();
    assert!(
        lab.resume_with_partial_godi(&resumed, &missing, &encrypted, true)
            .status
            .success()
    );
    assert_eq!(files, package_files(&resumed));
    assert_v49_verify(&lab, &resumed);
}

#[test]
fn v49_repeated_encrypted_candidate_limit_counts_rows_not_unique_fdids() {
    let lab = Lab::new();
    for count in [4096, 4097] {
        let client = lab.private_dir(&format!("repeated-encrypted-{count}-client"));
        fixture::write_v49_client_with_repeated_encrypted_candidates(&client, count);
        let package = lab.0.join(format!("repeated-encrypted-{count}-package"));
        let result = lab.produce("extract", 49, &client, &package);
        if count == 4096 {
            assert_cli_json(result, "extract", Some(&package));
            assert_eq!(
                coverage::parse_v49(&package_files(&package)[COVERAGE_PATH])
                    .unwrap()
                    .gameobjects
                    .candidate_requests,
                4096
            );
            let parsed = coverage::parse_v49(&package_files(&package)[COVERAGE_PATH]).unwrap();
            assert_eq!(parsed.gameobjects.encrypted_unavailable_rows, 4096);
            assert_eq!(parsed.gameobjects.encrypted_unavailable_fdids, [60_000]);
            assert_eq!(parsed.gameobjects.outcomes.len(), 4097);
            assert_v49_verify(&lab, &package);
        } else {
            assert!(!result.status.success());
            assert!(
                String::from_utf8_lossy(&result.stderr).contains("gameobject_startup_policy_limit")
            );
            assert!(package.join(".forever-package-incomplete").is_file());
            assert!(!package.join("forever-package.json").exists());
            assert!(!package.join(COVERAGE_PATH).exists());
            assert!(!package.join("vmaps/GameObjectModels.dtree").exists());
        }
    }
}

#[test]
fn v49_partial_godi_diagnostics_preserve_db2_and_model_failures() {
    let lab = Lab::new();
    let broken = lab.private_dir("partial-db2-broken-client");
    fixture::write_v49_client_without_initial_godi_encoding(&broken);
    let output = lab.0.join("partial-db2-failure-package");
    let diagnostic = lab.0.join("partial-db2-failure.jsonl");
    let result = lab.produce_with_diagnostic("extract", 49, &broken, &output, &diagnostic);
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(stderr.contains("\"component\":\"db2\""), "{stderr}");
    let diagnostic = fs::read_to_string(diagnostic).unwrap();
    assert!(diagnostic.contains("\"phase\":\"db2\""), "{diagnostic}");
    assert!(
        diagnostic.contains("\"request_kind\":\"db2_table\""),
        "{diagnostic}"
    );

    let client = lab.private_dir("model-failure-client");
    fixture::write_v49_client_with_corrupt_candidate(&client, 60_000, false);
    let output = lab.0.join("model-failure-package");
    let diagnostic_path = lab.0.join("model-failure.jsonl");
    let result = lab.produce_with_diagnostic("extract", 49, &client, &output, &diagnostic_path);
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(stderr.contains("\"component\":\"vmaps\""), "{stderr}");
    let diagnostic = fs::read_to_string(diagnostic_path).unwrap();
    for expected in [
        "\"phase\":\"startup\"",
        "\"request_kind\":\"gameobject_model\"",
        "\"file_data_id\":60000",
        "\"typed_cause\":{\"variant\":\"NoLocalEncoding\"",
    ] {
        assert!(diagnostic.contains(expected), "{expected}: {diagnostic}");
    }
    for forbidden in [
        client.to_string_lossy().as_ref(),
        output.to_string_lossy().as_ref(),
        "Data/data",
        "content_key",
        "encoded_key",
        "ciphertext",
    ] {
        assert!(!diagnostic.contains(forbidden), "{forbidden}: {diagnostic}");
    }

    for multiframe in [false, true] {
        let client = lab.private_dir(if multiframe {
            "corrupt-multiframe-model-client"
        } else {
            "corrupt-model-client"
        });
        fixture::write_v49_client_with_corrupt_candidate(&client, 60_000, multiframe);
        let output = lab.0.join(if multiframe {
            "corrupt-multiframe-model-package"
        } else {
            "corrupt-model-package"
        });
        let diagnostic_path = lab.0.join(if multiframe {
            "corrupt-multiframe-model.jsonl"
        } else {
            "corrupt-model.jsonl"
        });
        let result = lab.produce_with_diagnostic("extract", 49, &client, &output, &diagnostic_path);
        assert!(!result.status.success());
        let diagnostic = fs::read_to_string(diagnostic_path).unwrap();
        assert!(
            diagnostic.contains("\"request_kind\":\"gameobject_model\""),
            "{diagnostic}"
        );
        assert!(
            !diagnostic.contains("blte_encrypted_unsupported"),
            "{diagnostic}"
        );
    }
}

#[test]
fn v49_rejects_crossed_and_noncanonical_outcome_lists() {
    let lab = Lab::new();
    let package = lab.extract("attacks", &[50_000], &[50_001]);
    let original = fs::read(package.join(COVERAGE_PATH)).unwrap();
    coverage::parse_v49(&original).unwrap();
    let resized = coverage::replace_v49_outcome_lists(&original, &[50_000], &[50_001]).unwrap();
    let resized_parsed = coverage::parse_v49(&resized).unwrap();
    assert_eq!(resized_parsed.gameobjects.missing_root_fdids, [50_000]);
    assert_eq!(
        resized_parsed.gameobjects.encrypted_unavailable_fdids,
        [50_001]
    );
    let resized_missing =
        coverage::replace_v49_outcome_lists(&original, &[50_000, 50_002], &[50_001]).unwrap();
    assert!(coverage::parse_v49(&resized_missing).is_err());
    let attacked =
        lab.rebuilt_with_coverage(&package, "resized-missing-outcome-list", &resized_missing);
    for verb in ["verify", "verify-links"] {
        assert!(!lab.verify(verb, 49, &attacked).status.success());
    }
    let resized_encrypted =
        coverage::replace_v49_outcome_lists(&original, &[50_000], &[50_001, 50_002]).unwrap();
    assert!(coverage::parse_v49(&resized_encrypted).is_err());
    let attacked = lab.rebuilt_with_coverage(
        &package,
        "resized-encrypted-outcome-list",
        &resized_encrypted,
    );
    for verb in ["verify", "verify-links"] {
        assert!(!lab.verify(verb, 49, &attacked).status.success());
    }
    let crossed = coverage::replace_v49_outcome_lists(&original, &[50_001], &[50_000]).unwrap();
    assert!(coverage::parse_v49(&crossed).is_err());
    let attacked = lab.rebuilt_with_coverage(&package, "crossed-outcome-lists", &crossed);
    for verb in ["verify", "verify-links"] {
        assert!(!lab.verify(verb, 49, &attacked).status.success());
    }
    let mismatched_rows = coverage::replace_v49_count(
        &original,
        coverage::parse_v49(&original)
            .unwrap()
            .missing_root_rows_range
            .unwrap(),
        2,
    )
    .unwrap();
    assert!(coverage::parse_v49(&mismatched_rows).is_err());
    let attacked = lab.rebuilt_with_coverage(&package, "mismatched-outcome-rows", &mismatched_rows);
    for verb in ["verify", "verify-links"] {
        assert!(!lab.verify(verb, 49, &attacked).status.success());
    }

    for list_index in 0..coverage::LIST_ORDER.len() {
        let attacked_fdid = if list_index % 2 == 0 { 50_000 } else { 50_001 };
        let attacked_bytes =
            coverage::replace_omission(&original, list_index, &[attacked_fdid]).unwrap();
        assert!(coverage::parse_v49(&attacked_bytes).is_err());
    }

    let candidate_requests = coverage::replace_v49_candidate_requests(&original, 10).unwrap();
    assert!(coverage::parse_v49(&candidate_requests).is_err());
    let attacked =
        lab.rebuilt_with_coverage(&package, "mismatched-candidate-count", &candidate_requests);
    for verb in ["verify", "verify-links"] {
        assert!(!lab.verify(verb, 49, &attacked).status.success());
    }

    let noncanonical =
        coverage::replace_v49_outcome_lists(&original, &[], &[50_000, 50_001]).unwrap();
    assert!(coverage::parse_v49(&noncanonical).is_err());
    let over_limit: Vec<_> = (70_000..70_000 + 4097).collect();
    assert!(coverage::replace_v49_outcome_lists(&original, &[], &over_limit).is_err());
    let attacked = lab.rebuilt_with_coverage(&package, "noncanonical-outcome-lists", &noncanonical);
    for verb in ["verify", "verify-links"] {
        assert!(!lab.verify(verb, 49, &attacked).status.success());
    }
}

fn remove_artifact(package: &Path, path: &str) {
    let context =
        Context::parse(&fs::read_to_string(package.join(".state/context.json")).unwrap()).unwrap();
    let receipt = fs::read_dir(package.join(".state/files"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|candidate| {
            forever_casc::package::Artifact::parse(
                &fs::read_to_string(candidate).unwrap(),
                context,
                Limits::default(),
            )
            .unwrap()
            .path
                == path
        })
        .unwrap();
    fs::remove_file(receipt).unwrap();
    fs::remove_file(package.join(path)).unwrap();
}

fn copy_tree(source: &Path, destination: &Path) {
    fs::create_dir(destination).unwrap();
    fs::set_permissions(destination, fs::Permissions::from_mode(0o700)).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let kind = entry.file_type().unwrap();
        let target = destination.join(entry.file_name());
        if kind.is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            assert!(kind.is_file());
            fs::copy(entry.path(), &target).unwrap();
            fs::set_permissions(target, fs::Permissions::from_mode(0o600)).unwrap();
        }
    }
}
