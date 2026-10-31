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

use forever_casc::{
    collision::gameobjects::MAX_CANDIDATE_REQUESTS,
    package::{Component, Context, Limits, Run},
};
use forever_vmap::gameobject_models::Dtree;
use std::{
    collections::BTreeMap,
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};

const PLAN: &str = include_str!("../../../tests/contracts/fixtures/forever-product-v48/all.tsv");
const V47_PLAN: &str =
    include_str!("../../../tests/contracts/fixtures/forever-product-v47/all.tsv");
static NEXT: AtomicU64 = AtomicU64::new(0);

struct Lab(PathBuf);

impl Lab {
    fn new() -> Self {
        let root = std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join(format!(
                ".local/share/rce-v48-gameobject-missing-root-cli-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
        fs::create_dir_all(root.parent().unwrap()).unwrap();
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        fs::write(root.join("plan-v48.tsv"), PLAN).unwrap();
        fs::write(root.join("plan-v47.tsv"), V47_PLAN).unwrap();
        Self(root)
    }

    fn private_dir(&self, name: &str) -> PathBuf {
        let path = self.0.join(name);
        fs::create_dir(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        path
    }

    fn plan(&self, version: u32) -> PathBuf {
        self.0.join(format!("plan-v{version}.tsv"))
    }

    fn command(&self, verb: &str, version: u32) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_forever-extractor"));
        command
            .current_dir(&self.0)
            .env_remove("FOREVER_PRODUCT_DIAGNOSTIC")
            .args([verb, "--plan"])
            .arg(self.plan(version));
        command
    }

    fn produce(&self, verb: &str, client: &Path, output: &Path, scratch: &Path) -> Output {
        self.produce_with_diagnostic(verb, client, output, scratch, None)
    }

    fn produce_with_diagnostic(
        &self,
        verb: &str,
        client: &Path,
        output: &Path,
        scratch: &Path,
        diagnostic: Option<&Path>,
    ) -> Output {
        let mut command = self.command(verb, 48);
        command
            .args(["--client"])
            .arg(client)
            .args(["--output"])
            .arg(output)
            .args(["--scratch"])
            .arg(scratch);
        if let Some(diagnostic) = diagnostic {
            command.env("FOREVER_PRODUCT_DIAGNOSTIC", diagnostic);
        }
        command.output().unwrap()
    }

    fn extract(&self, name: &str, missing_fdids: &[u32], minimal_godi: bool) -> PathBuf {
        self.extract_with_root_rows(name, missing_fdids, &[], minimal_godi)
    }

    fn extract_with_root_rows(
        &self,
        name: &str,
        missing_fdids: &[u32],
        root_present_missing_fdids: &[u32],
        minimal_godi: bool,
    ) -> PathBuf {
        let client = self.private_dir(&format!("{name}-client"));
        fixture::write_v48_client_with_root_rows(
            &client,
            missing_fdids,
            root_present_missing_fdids,
            minimal_godi,
        );
        let output = self.0.join(format!("{name}-package"));
        let scratch = self.private_dir(&format!("{name}-scratch"));
        let result = self.produce("extract", &client, &output, &scratch);
        assert_cli_json(result, "extract", Some(&output));
        output
    }

    fn verify(&self, verb: &str, version: u32, package: &Path) -> Output {
        self.command(verb, version)
            .args(["--output"])
            .arg(package)
            .output()
            .unwrap()
    }

    fn resume(&self, package: &Path, missing_fdids: &[u32], minimal_godi: bool) -> Output {
        let client = self.private_dir("resume-client");
        fixture::write_v48_client_with_root_rows(&client, missing_fdids, &[], minimal_godi);
        let scratch = self.private_dir("resume-scratch");
        self.produce("resume", &client, package, &scratch)
    }

    fn resume_with_partial_godi(&self, package: &Path, missing_fdids: &[u32]) -> Output {
        let client = self.private_dir("resume-partial-godi-client");
        fixture::write_v48_client_with_partial_godi(&client, missing_fdids, &[], false, true);
        let scratch = self.private_dir("resume-partial-godi-scratch");
        self.produce("resume", &client, package, &scratch)
    }

    fn rebuilt_with_coverage(&self, base: &Path, name: &str, bytes: &[u8]) -> PathBuf {
        let context =
            Context::parse(&fs::read_to_string(base.join(".state/context.json")).unwrap()).unwrap();
        let mut files = package_files(base);
        files.insert(coverage::V48_PATH.to_owned(), bytes.to_vec());
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
        report.contains("\"contract\":\"forever-product-v48\""),
        "{report}"
    );
    assert!(
        report.contains(&format!(
            "\"gameobject_missing_root_max\":{MAX_CANDIDATE_REQUESTS}"
        )),
        "{verb} did not report the admitted missing-ROOT ceiling: {report}"
    );
    assert!(
        report.contains("\"full_extractor_complete\":false"),
        "{report}"
    );
    report
}

fn assert_cli_json(output: Output, verb: &str, package: Option<&Path>) -> String {
    let report = assert_success(output, verb);
    let parsed = json::parse(&report).unwrap();
    let json::JsonValue::Object(fields) = parsed else {
        panic!("{verb}: JSON root is not an object: {report}");
    };
    if let Some(package) = package {
        assert!(package.is_dir());
        assert!(fields.contains_key("package"), "{verb}: {report}");
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

fn verify_both(lab: &Lab, version: u32, package: &Path) {
    for verb in ["verify", "verify-links"] {
        if version == 48 {
            assert_cli_json(lab.verify(verb, version, package), verb, Some(package));
        } else {
            let output = lab.verify(verb, version, package);
            assert!(
                !output.status.success(),
                "v{version} {verb} accepted the v48 package: {}",
                String::from_utf8_lossy(&output.stdout)
            );
        }
    }
}

#[test]
fn v48_inspect_lifecycle_resume_determinism_and_v47_rejection() {
    let lab = Lab::new();
    let inspect = lab.command("inspect", 48).output().unwrap();
    let inspect = assert_cli_json(inspect, "inspect", None);
    assert!(inspect.contains(coverage::V48_COVERAGE_PROFILE));
    assert!(inspect.contains("\"gameobject_scope\":\"available-fdid-direct-missing-root-v1\""));
    assert!(inspect.contains("\"model_scope\":\"placement-and-gameobject-with-missing-root-v1\""));

    let missing = [50_000, 50_001];
    let package = lab.extract("first", &missing, false);
    let files = package_files(&package);
    let coverage_bytes = &files[coverage::V48_PATH];
    let parsed = coverage::parse_v48(coverage_bytes).unwrap();
    assert_eq!(parsed.version, 21);
    assert_eq!(parsed.coverage_profile, coverage::V48_COVERAGE_PROFILE);
    assert_eq!(parsed.source_profile, coverage::V48_SOURCE_PROFILE);
    assert_eq!(parsed.domain_profile, coverage::V48_DOMAIN_PROFILE);
    assert_eq!(parsed.resource_profile, coverage::V48_RESOURCE_PROFILE);
    assert_eq!(parsed.gameobjects.missing_root_rows, 2);
    assert_eq!(parsed.gameobjects.missing_root_fdids, missing);
    assert_eq!(
        parsed
            .gameobjects
            .outcomes
            .iter()
            .filter(|outcome| outcome.disposition == 5)
            .map(|outcome| outcome.file_data_id)
            .collect::<Vec<_>>(),
        missing
    );
    let repeated_fdid_package = lab.extract("repeated-missing-root-fdid", &[50_000, 50_000], false);
    let repeated_fdid_coverage = fs::read(repeated_fdid_package.join(coverage::V48_PATH)).unwrap();
    let repeated_fdid = coverage::parse_v48(&repeated_fdid_coverage).unwrap();
    assert_eq!(repeated_fdid.gameobjects.missing_root_rows, 2);
    assert_eq!(repeated_fdid.gameobjects.missing_root_fdids, [50_000]);
    assert_eq!(
        repeated_fdid
            .gameobjects
            .outcomes
            .iter()
            .filter(|outcome| outcome.disposition == 5)
            .map(|outcome| (outcome.display_id, outcome.file_data_id))
            .collect::<Vec<_>>(),
        [
            (fixture::MISSING_ROOT_DISPLAY, 50_000),
            (fixture::MISSING_ROOT_DISPLAY + 1, 50_000),
        ]
    );
    verify_both(&lab, 48, &repeated_fdid_package);
    verify_both(&lab, 47, &repeated_fdid_package);
    assert!(!files.contains_key("vmaps/FILE0000C350.xxx.vmo"));
    assert!(!files.contains_key("vmaps/FILE0000C351.xxx.vmo"));
    let tree = Dtree::parse(&files["vmaps/GameObjectModels.dtree"], Default::default()).unwrap();
    let rows: Vec<_> = tree.rows().collect();
    assert!(!rows.iter().any(|row| {
        [
            fixture::MISSING_ROOT_DISPLAY,
            fixture::MISSING_ROOT_DISPLAY + 1,
        ]
        .contains(&row.display_id())
    }));
    assert_eq!(rows.len(), 4);
    verify_both(&lab, 48, &package);
    verify_both(&lab, 47, &package);

    let second = lab.extract("second", &missing, false);
    assert_eq!(files, package_files(&second));

    let resumed = lab.0.join("resumed-package");
    copy_tree(&package, &resumed);
    fs::remove_file(resumed.join("forever-package.json")).unwrap();
    fs::remove_file(resumed.join(".state/sealed/vmaps")).unwrap();
    remove_artifact(&resumed, coverage::V48_PATH);
    remove_artifact(&resumed, "vmaps/GameObjectModels.dtree");
    for path in files
        .keys()
        .filter(|path| path.starts_with("vmaps/") && path.ends_with(".vmo"))
    {
        remove_artifact(&resumed, path);
    }
    fs::write(resumed.join(".forever-package-incomplete"), b"incomplete\n").unwrap();
    let resume = lab.resume(&resumed, &missing, false);
    assert_cli_json(resume, "resume", Some(&resumed));
    assert_eq!(files, package_files(&resumed));
    assert!(!resumed.join(".forever-package-incomplete").exists());
    verify_both(&lab, 48, &resumed);
}

#[test]
fn v48_extracts_authenticated_partial_godi_and_continues_geometry() {
    let lab = Lab::new();
    let client = lab.private_dir("partial-godi-client");
    fixture::write_v48_client_with_partial_godi(&client, &[60_000], &[], false, true);
    let output = lab.0.join("partial-godi-package");
    let scratch = lab.private_dir("partial-godi-scratch");
    let extracted = lab.produce("extract", &client, &output, &scratch);
    assert_cli_json(extracted, "extract", Some(&output));

    let manifest = fs::read_to_string(output.join("forever-package.json")).unwrap();
    let declaration = fixture::partial_godi_declaration();
    let expected = format!(
        "\"partial_encrypted_tables\":[{{\"path\":\"dbc/esES/GameObjectDisplayInfo.db2\",\"sections\":[{{\"lookup\":\"{}\",\"frame_ordinal\":{},\"decoded_size\":{}}}]}}]",
        declaration.lookup, declaration.frame_ordinal, declaration.decoded_size
    );
    assert!(manifest.contains(&expected), "{manifest}");
    assert_eq!(manifest.matches(&expected).count(), 1);
    assert!(output.join("dbc/esES/GameObjectDisplayInfo.db2").is_file());
    assert!(output.join("vmaps/GameObjectModels.dtree").is_file());
    verify_both(&lab, 48, &output);

    let files = package_files(&output);
    let resumed = lab.0.join("partial-godi-resumed-package");
    copy_tree(&output, &resumed);
    fs::remove_file(resumed.join("forever-package.json")).unwrap();
    fs::remove_file(resumed.join(".state/sealed/vmaps")).unwrap();
    remove_artifact(&resumed, coverage::V48_PATH);
    remove_artifact(&resumed, "vmaps/GameObjectModels.dtree");
    for path in files
        .keys()
        .filter(|path| path.starts_with("vmaps/") && path.ends_with(".vmo"))
    {
        remove_artifact(&resumed, path);
    }
    fs::write(resumed.join(".forever-package-incomplete"), b"incomplete\n").unwrap();
    let resumed_result = lab.resume_with_partial_godi(&resumed, &[60_000]);
    assert_cli_json(resumed_result, "resume", Some(&resumed));
    assert_eq!(files, package_files(&resumed));
    verify_both(&lab, 48, &resumed);
}

#[test]
fn v48_initial_db2_acquisition_failure_finishes_redacted_diagnostic() {
    let lab = Lab::new();
    let client = lab.private_dir("initial-db2-failure-client");
    fixture::write_v48_client_without_initial_godi_encoding(&client);
    let output = lab.0.join("initial-db2-failure-package");
    let scratch = lab.private_dir("initial-db2-failure-scratch");
    let diagnostic = lab.0.join("initial-db2-failure-diagnostic.jsonl");
    let rejected =
        lab.produce_with_diagnostic("extract", &client, &output, &scratch, Some(&diagnostic));
    assert!(
        !rejected.status.success(),
        "v48 accepted GODI with no matching local encoding"
    );
    assert_eq!(rejected.status.code(), Some(9));
    let stderr = String::from_utf8_lossy(&rejected.stderr);
    assert!(
        stderr.contains("\"code\":\"local_store_no_verified_encoding\""),
        "{stderr}"
    );
    assert!(stderr.contains("\"component\":\"db2\""), "{stderr}");
    assert!(!output.join("dbc/esES/GameObjectDisplayInfo.db2").exists());

    let diagnostic = fs::read_to_string(diagnostic).unwrap();
    assert_eq!(diagnostic.lines().count(), 1, "{diagnostic}");
    for expected in [
        "\"phase\":\"db2\"",
        "\"request_kind\":\"db2_table\"",
        "\"file_data_id\":1266277",
        "\"typed_cause\":{\"variant\":\"NoLocalEncoding\"",
        "\"code\":\"local_store_no_verified_encoding\"",
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
        "unrelated authenticated content",
    ] {
        assert!(!diagnostic.contains(forbidden), "{forbidden}: {diagnostic}");
    }
}

#[test]
fn v48_only_declares_fdids_absent_from_root_and_rejects_present_unavailable_bodies() {
    let lab = Lab::new();
    let package = lab.extract("absent", &[60_000], false);
    let coverage_bytes = fs::read(package.join(coverage::V48_PATH)).unwrap();
    let parsed = coverage::parse_v48(&coverage_bytes).unwrap();
    assert_eq!(parsed.gameobjects.missing_root_fdids, [60_000]);
    verify_both(&lab, 48, &package);

    let client = lab.private_dir("root-present-client");
    fixture::write_v48_client_with_root_rows(&client, &[], &[60_000], false);
    let output = lab.0.join("root-present-package");
    let scratch = lab.private_dir("root-present-scratch");
    let diagnostic = lab.0.join("root-present-diagnostic.jsonl");
    let rejected =
        lab.produce_with_diagnostic("extract", &client, &output, &scratch, Some(&diagnostic));
    assert!(
        !rejected.status.success(),
        "v48 accepted ROOT-present FDID without a body"
    );
    let stderr = String::from_utf8_lossy(&rejected.stderr);
    assert!(
        stderr.contains("local_store_no_verified_encoding"),
        "{stderr}"
    );
    assert!(stderr.contains("\"component\":\"vmaps\""), "{stderr}");
    assert!(!output.join(coverage::V48_PATH).exists());
    let diagnostic = fs::read_to_string(diagnostic).unwrap();
    assert_eq!(diagnostic.lines().count(), 1, "{diagnostic}");
    for expected in [
        "\"phase\":\"startup\"",
        "\"request_kind\":\"gameobject_model\"",
        "\"file_data_id\":60000",
        "\"typed_cause\":{\"variant\":\"NoLocalEncoding\"",
        "\"code\":\"local_store_no_verified_encoding\"",
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
}

#[test]
fn v48_missing_root_list_maximum_and_resealed_codec_attacks() {
    let lab = Lab::new();
    let missing: Vec<_> = (70_000..70_000 + MAX_CANDIDATE_REQUESTS as u32).collect();
    let package = lab.extract("at-limit", &missing, true);
    let files = package_files(&package);
    let coverage_bytes = &files[coverage::V48_PATH];
    let parsed = coverage::parse_v48(coverage_bytes).unwrap();
    assert_eq!(
        parsed.gameobjects.missing_root_rows,
        MAX_CANDIDATE_REQUESTS as u64
    );
    assert_eq!(
        parsed.gameobjects.missing_root_fdids.len(),
        MAX_CANDIDATE_REQUESTS
    );
    assert_eq!(parsed.gameobjects.missing_root_fdids, missing);
    verify_both(&lab, 48, &package);

    let mismatched = coverage::replace_v48_missing_root_count(
        coverage_bytes,
        (MAX_CANDIDATE_REQUESTS + 1) as u64,
        &missing,
    )
    .unwrap();
    let attacked = lab.rebuilt_with_coverage(&package, "resealed-count-mismatch", &mismatched);
    for verb in ["verify", "verify-links"] {
        assert!(
            !lab.verify(verb, 48, &attacked).status.success(),
            "v48 {verb} accepted mismatched missing-ROOT row/list counts"
        );
    }

    let over_limit_ids: Vec<_> = (100_000..100_000 + MAX_CANDIDATE_REQUESTS as u32 + 1).collect();
    let over_limit = coverage::replace_v48_missing_root_count(
        coverage_bytes,
        over_limit_ids.len() as u64,
        &over_limit_ids,
    )
    .unwrap();
    let attacked = lab.rebuilt_with_coverage(&package, "resealed-count-over-limit", &over_limit);
    for verb in ["verify", "verify-links"] {
        assert!(
            !lab.verify(verb, 48, &attacked).status.success(),
            "v48 {verb} accepted a missing-ROOT list over its declared maximum"
        );
    }
}

#[test]
fn v48_profile_ceiling_matches_the_request_policy() {
    assert_eq!(coverage::V48_MISSING_ROOT_MAX, MAX_CANDIDATE_REQUESTS);
    assert_eq!(coverage::V48_MISSING_ROOT_MAX, 4096);
}
