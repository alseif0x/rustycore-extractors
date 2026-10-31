// SPDX-License-Identifier: GPL-3.0-or-later
#![cfg(target_os = "linux")]

#[path = "v42_model_output_expanded_material_filtered_wmo_geometry_cli/coverage.rs"]
mod coverage;
#[path = "v39_read_heavy_material_filtered_wmo_geometry_cli/fixture.rs"]
#[allow(dead_code)]
mod fixture;
use forever_casc::{
    collision::resources::Profile,
    package::{Artifact, Component, Context, Limits, Run},
};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};

const V41_PLAN_RELATIVE: &str = "../../tests/contracts/fixtures/forever-product-v41/all.tsv";
const V40_PLAN_RELATIVE: &str = "../../tests/contracts/fixtures/forever-product-v40/all.tsv";
const LIST_ORDER: [&str; 5] = [
    "missing_m2",
    "missing_wmo",
    "empty_wmo",
    "unselected_wmo",
    "material_filtered_wmo",
];
static NEXT: AtomicU64 = AtomicU64::new(0);

struct Lab(PathBuf);

impl Lab {
    fn new() -> Self {
        let root = std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join(format!(
                ".local/share/rce-v42-model-output-expanded-material-filtered-wmo-cli-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
        fs::create_dir_all(root.parent().unwrap()).unwrap();
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        Self(root)
    }

    fn fixture_plan(&self, relative: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)
    }

    fn v40_plan(&self) -> PathBuf {
        self.fixture_plan(V40_PLAN_RELATIVE)
    }

    fn v41_plan(&self) -> PathBuf {
        self.fixture_plan(V41_PLAN_RELATIVE)
    }

    fn v42_plan(&self) -> PathBuf {
        let source = self.v41_plan();
        let text = fs::read_to_string(source).unwrap();
        let transformed = text
            .replace("forever-product\t41", "forever-product\t42")
            .replace(
                "linux70450-source-expanded-material-filtered-wmo-vmap-product-v1",
                "linux70450-model-output-expanded-material-filtered-wmo-vmap-product-v1",
            )
            .replace(
                "linux70450-source-expanded-material-filtered-wmo-coverage-v14",
                "linux70450-model-output-expanded-material-filtered-wmo-coverage-v15",
            );
        assert_ne!(text, transformed);
        let plan = self.0.join("plan-v42.tsv");
        fs::write(&plan, transformed).unwrap();
        plan
    }

    fn private_dir(&self, name: &str) -> PathBuf {
        let path = self.0.join(name);
        fs::create_dir(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        path
    }

    fn command(&self, verb: &str, plan: &Path) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_forever-extractor"));
        command
            .current_dir(&self.0)
            .env_remove("FOREVER_PRODUCT_DIAGNOSTIC")
            .args([verb, "--plan"])
            .arg(plan);
        command
    }

    fn produce(
        &self,
        verb: &str,
        plan: &Path,
        client: &Path,
        output: &Path,
        scratch: &Path,
    ) -> Output {
        self.command(verb, plan)
            .args(["--client"])
            .arg(client)
            .args(["--output"])
            .arg(output)
            .args(["--scratch"])
            .arg(scratch)
            .output()
            .unwrap()
    }

    fn extract(&self, name: &str, plan: &Path, contract: &str) -> PathBuf {
        let client = self.private_dir(&format!("{name}-client"));
        fixture::write_client_with(
            &client,
            fixture::ClientMutation {
                populated_adt_mode: fixture::AdtMode::MaterialFilteredWmoMixed,
                ..Default::default()
            },
        );
        let package = self.0.join(format!("{name}-package"));
        let scratch = self.private_dir(&format!("{name}-scratch"));
        let output = self.produce("extract", plan, &client, &package, &scratch);
        let report = assert_success_for_contract(output, "extract", contract);
        assert!(
            report.contains(&format!("\"contract\":\"{contract}\"")),
            "{report}"
        );
        package
    }

    fn verify(&self, plan: &Path, package: &Path, contract: &str, expected: &[&str]) {
        for verb in ["verify", "verify-links"] {
            let output = self
                .command(verb, plan)
                .args(["--output"])
                .arg(package)
                .output()
                .unwrap();
            let report = assert_success_for_contract(output, verb, contract);
            for value in expected {
                assert!(report.contains(value), "{verb}: missing {value}: {report}");
            }
        }
    }

    fn verify_failure(&self, plan: &Path, package: &Path, expected_contract: &str) {
        for verb in ["verify", "verify-links"] {
            let output = self
                .command(verb, plan)
                .args(["--output"])
                .arg(package)
                .output()
                .unwrap();
            assert!(
                !output.status.success(),
                "{verb} accepted crossed or malformed package"
            );
            let text = String::from_utf8(output.stderr).unwrap();
            assert!(
                text.contains(&format!("\"contract\":\"{expected_contract}\"")),
                "{text}"
            );
            assert!(text.contains("\"code\":"), "{text}");
        }
    }

    fn rebuilt_with_coverage(&self, base: &Path, name: &str, bytes: &[u8]) -> PathBuf {
        let context =
            Context::parse(&fs::read_to_string(base.join(".state/context.json")).unwrap()).unwrap();
        let mut files = package_files(base);
        files.insert(coverage::PATH.to_owned(), bytes.to_vec());
        let output = self.0.join(name);
        let declaration = fixture::partial_map().declaration;
        let mut run = Run::create(&output, context, &[base], Limits::default()).unwrap();
        run.set_partial_encrypted_table("dbc/esES/Map.db2", &[declaration])
            .unwrap();
        for (path, bytes) in files {
            let component = if path == "dbc/esES/Map.db2" {
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

fn assert_success_for_contract(output: Output, label: &str, contract: &str) -> String {
    assert!(
        output.status.success(),
        "{label} exited {:?}: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty(), "{label} wrote to stderr");
    let report = String::from_utf8(output.stdout).unwrap();
    assert!(
        report.starts_with('{') && report.ends_with('\n'),
        "{label}: {report}"
    );
    assert!(
        report.contains(&format!("\"contract\":\"{contract}\"")),
        "{label}: {report}"
    );
    if contract == "forever-product-v42" {
        for expected in [
            "\"source_bytes_max\":1073741824",
            "\"models_max\":65536",
            "\"reads_max\":1000000",
            "linux-collision-model-output-expanded-resources-v1",
        ] {
            assert!(
                report.contains(expected),
                "{label}: missing {expected}: {report}"
            );
        }
    }
    report
}

fn package_files(package: &Path) -> std::collections::BTreeMap<String, Vec<u8>> {
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
            (artifact.path, bytes)
        })
        .collect()
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

#[test]
fn v42_lifecycle_determinism_resume_and_two_way_v41_cross_version_rejection() {
    let model_output = Profile::ModelOutputExpandedV1.collector_limits();
    assert_eq!(model_output.max_source_bytes, 1_073_741_824);
    assert_eq!(model_output.max_models, 65_536);
    assert_eq!(model_output.max_reads, 1_000_000);
    assert_eq!(model_output.max_requests, 16_000_000);
    assert_eq!(model_output.max_dependency_requests, 16_000_000);
    assert_eq!(model_output.max_raw_bytes, 268_435_456);

    let lab = Lab::new();
    let v40_plan = lab.v40_plan();
    let v41_plan = lab.v41_plan();
    let v42_plan = lab.v42_plan();
    let inspect = assert_success_for_contract(
        lab.command("inspect", &v42_plan).output().unwrap(),
        "v42 inspect",
        "forever-product-v42",
    );
    assert!(
        inspect.contains("linux70450-model-output-expanded-material-filtered-wmo-coverage-v15")
    );
    assert!(!inspect.contains("full_extractor_complete\\\":true"));

    let expected = [
        "\"verification_profile\":\"linux70450-model-output-expanded-material-filtered-wmo-verification-v1\"",
        "\"scope\":\"artifact_integrity_model_output_expanded_material_filtered_wmo_and_independently_derived_map_vmap_domain_coverage\"",
        "\"complete_global_map_coverage_claim\":false",
        "\"complete_vmap_component\":false",
        "\"full_extractor_complete\":false",
    ];
    let package = lab.extract("v42-first", &v42_plan, "forever-product-v42");
    let second = lab.extract("v42-second", &v42_plan, "forever-product-v42");
    let package_files_before_resume = package_files(&package);
    assert_eq!(package_files_before_resume, package_files(&second));
    let package_manifest = fs::read(package.join("forever-package.json")).unwrap();
    let bytes = fs::read(package.join(coverage::PATH)).unwrap();
    let parsed = coverage::Coverage::parse(&bytes).unwrap();
    assert_eq!(parsed.version, 15);
    assert_eq!(parsed.coverage_profile, coverage::COVERAGE_PROFILE);
    assert_eq!(parsed.source_profile, coverage::SOURCE_PROFILE);
    assert_eq!(parsed.domain_profile, coverage::DOMAIN_PROFILE);
    assert_eq!(parsed.resource_profile, coverage::RESOURCE_PROFILE);
    lab.verify(&v42_plan, &package, "forever-product-v42", &expected);

    let resumed = lab.0.join("v42-resumed-package");
    copy_tree(&package, &resumed);
    fs::remove_file(resumed.join("forever-package.json")).unwrap();
    fs::write(resumed.join(".forever-package-incomplete"), b"incomplete\n").unwrap();
    let client = lab.private_dir("v42-resume-client");
    fixture::write_client_with(
        &client,
        fixture::ClientMutation {
            populated_adt_mode: fixture::AdtMode::MaterialFilteredWmoMixed,
            ..Default::default()
        },
    );
    let scratch = lab.private_dir("v42-resume-scratch");
    let resumed_report = assert_success_for_contract(
        lab.produce("resume", &v42_plan, &client, &resumed, &scratch),
        "v42 resume",
        "forever-product-v42",
    );
    assert!(resumed_report.contains("\"contract\":\"forever-product-v42\""));
    assert_eq!(package_files(&resumed), package_files_before_resume);
    assert_eq!(fs::read(resumed.join(coverage::PATH)).unwrap(), bytes);
    assert_eq!(
        fs::read(resumed.join("forever-package.json")).unwrap(),
        package_manifest
    );
    assert!(!resumed.join(".forever-package-incomplete").exists());
    lab.verify(&v42_plan, &resumed, "forever-product-v42", &expected);

    let v41_package = lab.extract("v41-package", &v41_plan, "forever-product-v41");
    lab.verify_failure(&v42_plan, &v41_package, "forever-product-v42");
    lab.verify_failure(&v41_plan, &package, "forever-product-v41");

    let v40_package = lab.extract("v40-package", &v40_plan, "forever-product-v40");
    lab.verify_failure(&v42_plan, &v40_package, "forever-product-v42");
}

#[test]
fn v42_rejects_resealed_codec_version_and_profile_identity_attacks() {
    let lab = Lab::new();
    let plan = lab.v42_plan();
    let package = lab.extract("attacks-base", &plan, "forever-product-v42");
    let bytes = fs::read(package.join(coverage::PATH)).unwrap();

    let v14 = {
        let mut mutated = bytes.clone();
        mutated[8..12].copy_from_slice(&14_u32.to_le_bytes());
        mutated
    };
    let resealed_v14 = lab.rebuilt_with_coverage(&package, "resealed-v14", &v14);
    lab.verify_failure(&plan, &resealed_v14, "forever-product-v42");

    for identity in [
        coverage::COVERAGE_PROFILE,
        coverage::SOURCE_PROFILE,
        coverage::DOMAIN_PROFILE,
        coverage::RESOURCE_PROFILE,
    ] {
        let at = bytes
            .windows(identity.len())
            .position(|window| window == identity.as_bytes())
            .unwrap_or_else(|| panic!("missing coverage identity {identity}"));
        let mut changed = bytes.clone();
        changed[at] = b'x';
        let resealed =
            lab.rebuilt_with_coverage(&package, &format!("resealed-{}", identity.len()), &changed);
        lab.verify_failure(&plan, &resealed, "forever-product-v42");
    }
}

#[test]
fn v42_rejects_all_ten_resealed_pairwise_omission_overlaps() {
    let lab = Lab::new();
    let plan = lab.v42_plan();
    let package = lab.extract("overlap-base", &plan, "forever-product-v42");
    let bytes = fs::read(package.join(coverage::PATH)).unwrap();
    let parsed = coverage::Coverage::parse(&bytes).unwrap();
    let pairs = [
        (0, 1),
        (0, 2),
        (0, 3),
        (0, 4),
        (1, 2),
        (1, 3),
        (1, 4),
        (2, 3),
        (2, 4),
        (3, 4),
    ];
    for (left, right) in pairs {
        let mut overlapping = parsed.omissions.clone();
        overlapping[left] = vec![3_000_000];
        overlapping[right] = vec![3_000_000];
        let mut mutated = bytes.clone();
        for index in (0..LIST_ORDER.len()).rev() {
            mutated = parsed
                .replace_omission(&mutated, index, &overlapping[index])
                .unwrap();
        }
        let attacked =
            lab.rebuilt_with_coverage(&package, &format!("overlap-{left}-{right}"), &mutated);
        lab.verify_failure(&plan, &attacked, "forever-product-v42");
    }
}

#[test]
fn v42_five_omission_lists_enforce_4096_and_4097_with_both_verifiers() {
    let lab = Lab::new();
    let plan = lab.v42_plan();
    let package = lab.extract("lists-base", &plan, "forever-product-v42");
    let bytes = fs::read(package.join(coverage::PATH)).unwrap();
    let parsed = coverage::Coverage::parse(&bytes).unwrap();
    let ids: Vec<_> = (3_000_000..3_004_097).collect();
    for (index, list_name) in LIST_ORDER.iter().enumerate() {
        let at_limit = parsed
            .replace_omission(&bytes, index, &ids[..4096])
            .unwrap();
        let accepted = lab.rebuilt_with_coverage(&package, &format!("{list_name}-4096"), &at_limit);
        lab.verify(
            &plan,
            &accepted,
            "forever-product-v42",
            &["\"full_extractor_complete\":false"],
        );
        let over_limit = parsed
            .replace_omission(&bytes, index, &ids[..4097])
            .unwrap();
        let rejected =
            lab.rebuilt_with_coverage(&package, &format!("{list_name}-4097"), &over_limit);
        lab.verify_failure(&plan, &rejected, "forever-product-v42");
    }
}

#[test]
fn v42_reduced_plan_budget_keeps_nominal_source_model_and_read_limits_explicit() {
    let lab = Lab::new();
    let plan = lab.v42_plan();
    let raw = fs::read_to_string(&plan).unwrap();
    let reduced = raw.replace(
        "collision-raw-bytes\t1048576",
        "collision-raw-bytes\t921600",
    );
    assert_ne!(raw, reduced);
    let reduced_plan = lab.0.join("plan-reduced.tsv");
    fs::write(&reduced_plan, reduced).unwrap();
    let report = assert_success_for_contract(
        lab.command("inspect", &reduced_plan).output().unwrap(),
        "v42 reduced budget inspect",
        "forever-product-v42",
    );
    assert!(report.contains("\"source_bytes_max\":1073741824"));
    assert!(report.contains("\"models_max\":65536"));
    assert!(report.contains("\"reads_max\":1000000"));
    assert!(!report.contains("\"real_source_1gib_boundary\":true"));
}

#[cfg(test)]
mod json_tests {
    #[test]
    fn contract_marks_synthetic_fixture_and_failed_real_target() {
        let contract = include_str!("../../../contracts/forever-product-contract-v42.json");
        assert!(contract.contains("forever-product-v42"));
        assert!(contract.contains("linux-collision-model-output-expanded-resources-v1"));
        assert!(contract.contains("tests/contracts/fixtures/forever-product-v42/all.tsv"));
        assert!(contract.contains("synthetic-implementation-partial; real-target-failed"));
        let real_acceptance = contract
            .split("\"real_acceptance\": {")
            .nth(1)
            .and_then(|block| block.split("\n  },").next())
            .expect("real_acceptance block");
        assert!(real_acceptance.contains("\"status\": \"failed\""));
        assert!(
            real_acceptance
                .contains("\"run_id\": \"v42-model-output-expanded-wmo-20261027T044414Z\"")
        );
        assert!(real_acceptance.contains("\"accepted\": false"));
        assert!(real_acceptance.contains("\"accepted_prefix\": false"));
        assert!(contract.contains("\"real_target_attempted\": true"));
        assert!(contract.contains("\"real_extraction_accepted\": false"));
        assert!(contract.contains("\"real_source_1gib_boundary\": false"));
    }
}
