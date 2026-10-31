// SPDX-License-Identifier: GPL-3.0-or-later
#![cfg(target_os = "linux")]

#[path = "v43_model_bytes_expanded_material_filtered_wmo_geometry_cli/coverage.rs"]
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

const V42_PLAN_RELATIVE: &str = "../../tests/contracts/fixtures/forever-product-v42/all.tsv";
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
                ".local/share/rce-v43-model-bytes-expanded-material-filtered-wmo-cli-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
        fs::create_dir_all(root.parent().unwrap()).unwrap();
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        Self(root)
    }

    fn fixture_plan(&self) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join(V42_PLAN_RELATIVE)
    }

    fn v42_plan(&self) -> PathBuf {
        self.fixture_plan()
    }

    fn v43_plan(&self) -> PathBuf {
        let source = fs::read_to_string(self.fixture_plan()).unwrap();
        let transformed = source
            .replace("forever-product\t42", "forever-product\t43")
            .replace("product_version\t42", "product_version\t43")
            .replace(
                "linux70450-model-output-expanded-material-filtered-wmo-vmap-product-v1",
                "linux70450-model-bytes-expanded-material-filtered-wmo-vmap-product-v1",
            )
            .replace(
                "linux70450-model-output-expanded-material-filtered-wmo-coverage-v15",
                "linux70450-model-bytes-expanded-material-filtered-wmo-coverage-v16",
            );
        assert_ne!(source, transformed);
        let plan = self.0.join("plan-v43.tsv");
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

    fn verify_failure_for_package(&self, plan: &Path, package: &Path, expected_contract: &str) {
        for verb in ["verify", "verify-links"] {
            let output = self
                .command(verb, plan)
                .args(["--output"])
                .arg(package)
                .output()
                .unwrap();
            assert!(
                !output.status.success(),
                "{verb} accepted a crossed or malformed package"
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
    if contract == "forever-product-v43" {
        for expected in [
            "\"source_bytes_max\":1073741824",
            "\"models_max\":65536",
            "\"reads_max\":1000000",
            "\"model_bytes_max\":536870912",
            "linux-collision-model-bytes-expanded-resources-v1",
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
fn v43_model_bytes_profile_preserves_acquisition_caps_and_raises_byte_budgets() {
    let profile = Profile::ModelBytesExpandedV1;
    assert_eq!(
        profile.id(),
        "linux-collision-model-bytes-expanded-resources-v1"
    );
    let limits = profile.collector_limits();
    assert_eq!(limits.max_source_bytes, 1_073_741_824);
    assert_eq!(limits.max_models, 65_536);
    assert_eq!(limits.max_reads, 1_000_000);
    assert_eq!(limits.max_requests, 16_000_000);
    assert_eq!(limits.max_dependency_requests, 16_000_000);
    assert_eq!(limits.max_raw_bytes, 268_435_456);
}

#[test]
fn v43_lifecycle_resume_determinism_and_bidirectional_v42_cross_version_rejection() {
    let lab = Lab::new();
    let v42_plan = lab.v42_plan();
    let v43_plan = lab.v43_plan();
    let inspect = assert_success_for_contract(
        lab.command("inspect", &v43_plan).output().unwrap(),
        "v43 inspect",
        "forever-product-v43",
    );
    assert!(inspect.contains(coverage::COVERAGE_PROFILE));
    assert!(inspect.contains("\"full_extractor_complete\":false"));

    let expected = [
        "\"verification_profile\":\"linux70450-model-bytes-expanded-material-filtered-wmo-verification-v1\"",
        "\"scope\":\"artifact_integrity_model_bytes_expanded_material_filtered_wmo_and_independently_derived_map_vmap_domain_coverage\"",
        "\"complete_global_map_coverage_claim\":false",
        "\"complete_vmap_component\":false",
        "\"full_extractor_complete\":false",
    ];
    let package = lab.extract("v43-first", &v43_plan, "forever-product-v43");
    let second = lab.extract("v43-second", &v43_plan, "forever-product-v43");
    let files_before_resume = package_files(&package);
    assert_eq!(files_before_resume, package_files(&second));
    let manifest = fs::read(package.join("forever-package.json")).unwrap();
    let bytes = fs::read(package.join(coverage::PATH)).unwrap();
    let parsed = coverage::Coverage::parse(&bytes).unwrap();
    assert_eq!(parsed.version, 16);
    assert_eq!(parsed.coverage_profile, coverage::COVERAGE_PROFILE);
    assert_eq!(parsed.source_profile, coverage::SOURCE_PROFILE);
    assert_eq!(parsed.domain_profile, coverage::DOMAIN_PROFILE);
    assert_eq!(parsed.resource_profile, coverage::RESOURCE_PROFILE);
    lab.verify(&v43_plan, &package, "forever-product-v43", &expected);

    let resumed = lab.0.join("v43-resumed-package");
    copy_tree(&package, &resumed);
    fs::remove_file(resumed.join("forever-package.json")).unwrap();
    fs::write(resumed.join(".forever-package-incomplete"), b"incomplete\n").unwrap();
    let client = lab.private_dir("v43-resume-client");
    fixture::write_client_with(
        &client,
        fixture::ClientMutation {
            populated_adt_mode: fixture::AdtMode::MaterialFilteredWmoMixed,
            ..Default::default()
        },
    );
    let scratch = lab.private_dir("v43-resume-scratch");
    let report = assert_success_for_contract(
        lab.produce("resume", &v43_plan, &client, &resumed, &scratch),
        "v43 resume",
        "forever-product-v43",
    );
    assert!(report.contains("\"contract\":\"forever-product-v43\""));
    assert_eq!(package_files(&resumed), files_before_resume);
    assert_eq!(fs::read(resumed.join(coverage::PATH)).unwrap(), bytes);
    assert_eq!(
        fs::read(resumed.join("forever-package.json")).unwrap(),
        manifest
    );
    assert!(!resumed.join(".forever-package-incomplete").exists());
    lab.verify(&v43_plan, &resumed, "forever-product-v43", &expected);

    let v42_package = lab.extract("v42-package", &v42_plan, "forever-product-v42");
    lab.verify_failure_for_package(&v43_plan, &v42_package, "forever-product-v43");
    lab.verify_failure_for_package(&v42_plan, &package, "forever-product-v42");
}

#[test]
fn v43_rejects_resealed_codec_version_and_profile_identity_attacks() {
    let lab = Lab::new();
    let plan = lab.v43_plan();
    let package = lab.extract("attacks-base", &plan, "forever-product-v43");
    let bytes = fs::read(package.join(coverage::PATH)).unwrap();

    let v15 = {
        let mut mutated = bytes.clone();
        mutated[8..12].copy_from_slice(&15_u32.to_le_bytes());
        mutated
    };
    let resealed = lab.rebuilt_with_coverage(&package, "resealed-v15", &v15);
    lab.verify_failure_for_package(&plan, &resealed, "forever-product-v43");

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
        let attacked =
            lab.rebuilt_with_coverage(&package, &format!("resealed-{}", identity.len()), &changed);
        lab.verify_failure_for_package(&plan, &attacked, "forever-product-v43");
    }
}

#[test]
fn v43_rejects_all_ten_resealed_pairwise_omission_overlaps() {
    let lab = Lab::new();
    let plan = lab.v43_plan();
    let package = lab.extract("overlap-base", &plan, "forever-product-v43");
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
        lab.verify_failure_for_package(&plan, &attacked, "forever-product-v43");
    }
}

#[test]
fn v43_five_omission_lists_enforce_4096_and_4097_with_both_verifiers() {
    let lab = Lab::new();
    let plan = lab.v43_plan();
    let package = lab.extract("lists-base", &plan, "forever-product-v43");
    let bytes = fs::read(package.join(coverage::PATH)).unwrap();
    let parsed = coverage::Coverage::parse(&bytes).unwrap();
    let ids: Vec<_> = (3_000_000..3_004_097).collect();
    for (index, name) in LIST_ORDER.iter().enumerate() {
        let at_limit = parsed
            .replace_omission(&bytes, index, &ids[..4096])
            .unwrap();
        let accepted = lab.rebuilt_with_coverage(&package, &format!("{name}-4096"), &at_limit);
        lab.verify(
            &plan,
            &accepted,
            "forever-product-v43",
            &["\"full_extractor_complete\":false"],
        );
        let over_limit = parsed
            .replace_omission(&bytes, index, &ids[..4097])
            .unwrap();
        let rejected = lab.rebuilt_with_coverage(&package, &format!("{name}-4097"), &over_limit);
        lab.verify_failure_for_package(&plan, &rejected, "forever-product-v43");
    }
}

#[test]
fn v43_reduced_plan_preserves_nominal_resource_metadata_without_boundary_claim() {
    let lab = Lab::new();
    let plan = lab.v43_plan();
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
        "v43 reduced-budget inspect",
        "forever-product-v43",
    );
    for expected in [
        "\"source_bytes_max\":1073741824",
        "\"models_max\":65536",
        "\"reads_max\":1000000",
        "\"model_bytes_max\":536870912",
    ] {
        assert!(report.contains(expected), "missing {expected}: {report}");
    }
    assert!(!report.contains("\"real_512_mib_boundary\":true"));
    assert!(!report.contains("\"full_extractor_complete\":true"));
}

#[cfg(test)]
mod json_tests {
    #[test]
    fn contract_marks_real_target_failure_and_preserves_v42_failure_scope() {
        let contract = include_str!("../../../contracts/forever-product-contract-v43.json");
        assert!(contract.contains("forever-product-v43"));
        assert!(contract.contains("linux-collision-model-bytes-expanded-resources-v1"));
        assert!(contract.contains("real-target-failed"));
        assert!(contract.contains("\"run_id\": \"v43-model-bytes-expanded-wmo-20261027T070025Z\""));
        assert!(contract.contains("\"real_target_attempted\": true"));
        assert!(contract.contains("\"accepted_prefix\": false"));
        assert!(contract.contains("\"accepted\": false"));
        assert!(contract.contains("\"real_512_mib_boundary\": false"));
        assert!(
            contract.contains("347548262472d679b08efa89b4a988dfa3e05c9cb5307ff4586fe5af24ccc651")
        );
        assert!(contract.contains("\"observed_models_at_failure\": null"));
    }
}
