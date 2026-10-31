// SPDX-License-Identifier: GPL-3.0-or-later
#![cfg(target_os = "linux")]

#[path = "v44_source_expanded_v2_material_filtered_wmo_geometry_cli/coverage.rs"]
mod coverage;
#[path = "v39_read_heavy_material_filtered_wmo_geometry_cli/fixture.rs"]
#[allow(dead_code)]
mod fixture;
#[path = "v43_model_bytes_expanded_material_filtered_wmo_geometry_cli/coverage.rs"]
#[allow(dead_code)]
mod v43_coverage;

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

const V43_PLAN_RELATIVE: &str = "../../tests/contracts/fixtures/forever-product-v43/all.tsv";
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
                ".local/share/rce-v44-source-expanded-v2-material-filtered-wmo-cli-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
        fs::create_dir_all(root.parent().unwrap()).unwrap();
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        Self(root)
    }

    fn v43_plan(&self) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join(V43_PLAN_RELATIVE)
    }

    fn v44_plan(&self) -> PathBuf {
        let source = fs::read_to_string(self.v43_plan()).unwrap();
        let mut transformed = source.clone();
        for (from, to) in [
            ("forever-product\t43", "forever-product\t44"),
            (
                "linux70450-model-bytes-expanded-material-filtered-wmo-vmap-product-v1",
                "linux70450-source-expanded-v2-material-filtered-wmo-vmap-product-v1",
            ),
            (
                "linux70450-model-bytes-expanded-material-filtered-wmo-coverage-v16",
                "linux70450-source-expanded-v2-material-filtered-wmo-coverage-v17",
            ),
        ] {
            assert!(
                transformed.contains(from),
                "missing source PLAN value {from}"
            );
            transformed = transformed.replace(from, to);
        }
        assert_ne!(source, transformed);
        let plan = self.0.join("plan-v44.tsv");
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
        let is_partial = base.join("forever-package.json").exists();
        let mut files = package_files(base);
        files.insert(coverage::PATH.to_owned(), bytes.to_vec());
        let output = self.0.join(name);
        let mut run = Run::create(&output, context, &[base], Limits::default()).unwrap();
        if is_partial {
            let declaration = fixture::partial_map().declaration;
            run.set_partial_encrypted_table("dbc/esES/Map.db2", &[declaration])
                .unwrap();
        }
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
    if contract == "forever-product-v44" {
        for expected in [
            "\"source_bytes_max\":2147483648",
            "\"models_max\":65536",
            "\"reads_max\":1000000",
            "\"model_bytes_max\":536870912",
            "linux-collision-source-expanded-resources-v2",
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
fn v44_source_expanded_v2_only_raises_source_cap() {
    let limits = Profile::SourceExpandedV2.collector_limits();
    assert_eq!(limits.max_source_bytes, 2_147_483_648);
    assert_eq!(limits.max_models, 65_536);
    assert_eq!(limits.max_reads, 1_000_000);
    assert_eq!(limits.max_requests, 16_000_000);
    assert_eq!(limits.max_dependency_requests, 16_000_000);
    assert_eq!(limits.max_raw_bytes, 268_435_456);
    assert_eq!(Profile::SourceExpandedV2.id(), coverage::RESOURCE_PROFILE);
    let prior = Profile::ModelBytesExpandedV1.collector_limits();
    assert_eq!(prior.max_source_bytes, 1_073_741_824);
    assert_eq!(prior.max_models, limits.max_models);
    assert_eq!(prior.max_reads, limits.max_reads);
    assert_eq!(prior.max_requests, limits.max_requests);
    assert_eq!(
        prior.max_dependency_requests,
        limits.max_dependency_requests
    );
    assert_eq!(prior.max_raw_bytes, limits.max_raw_bytes);
}

#[test]
fn v44_lifecycle_resume_determinism_and_bidirectional_v43_cross_version_rejection() {
    let lab = Lab::new();
    let v43_plan = lab.v43_plan();
    let v44_plan = lab.v44_plan();
    let inspect = assert_success_for_contract(
        lab.command("inspect", &v44_plan).output().unwrap(),
        "v44 inspect",
        "forever-product-v44",
    );
    assert!(inspect.contains(coverage::COVERAGE_PROFILE));
    assert!(inspect.contains("\"full_extractor_complete\":false"));
    assert!(!inspect.contains("\"real_2_gib_boundary\":true"));

    let expected = [
        "\"verification_profile\":\"linux70450-source-expanded-v2-material-filtered-wmo-verification-v1\"",
        "\"scope\":\"artifact_integrity_source_expanded_v2_material_filtered_wmo_and_independently_derived_map_vmap_domain_coverage\"",
        "\"complete_global_map_coverage_claim\":false",
        "\"complete_vmap_component\":false",
        "\"full_extractor_complete\":false",
    ];
    let package = lab.extract("v44-first", &v44_plan, "forever-product-v44");
    let second = lab.extract("v44-second", &v44_plan, "forever-product-v44");
    let files_before_resume = package_files(&package);
    assert_eq!(files_before_resume, package_files(&second));
    let manifest = fs::read(package.join("forever-package.json")).unwrap();
    let bytes = fs::read(package.join(coverage::PATH)).unwrap();
    assert_eq!(u32::from_le_bytes(bytes[8..12].try_into().unwrap()), 17);
    let parsed = coverage::parse(&bytes);
    assert_eq!(parsed.version, 16);
    assert_eq!(parsed.coverage_profile, coverage::V43_COVERAGE_PROFILE);
    assert_eq!(parsed.source_profile, coverage::V43_SOURCE_PROFILE);
    assert_eq!(parsed.domain_profile, coverage::V43_DOMAIN_PROFILE);
    assert_eq!(parsed.resource_profile, coverage::V43_RESOURCE_PROFILE);
    lab.verify(&v44_plan, &package, "forever-product-v44", &expected);

    let resumed = lab.0.join("v44-resumed-package");
    copy_tree(&package, &resumed);
    fs::remove_file(resumed.join("forever-package.json")).unwrap();
    fs::write(resumed.join(".forever-package-incomplete"), b"incomplete\n").unwrap();
    let client = lab.private_dir("v44-resume-client");
    fixture::write_client_with(
        &client,
        fixture::ClientMutation {
            populated_adt_mode: fixture::AdtMode::MaterialFilteredWmoMixed,
            ..Default::default()
        },
    );
    let scratch = lab.private_dir("v44-resume-scratch");
    assert_success_for_contract(
        lab.produce("resume", &v44_plan, &client, &resumed, &scratch),
        "v44 resume",
        "forever-product-v44",
    );
    assert_eq!(package_files(&resumed), files_before_resume);
    assert_eq!(fs::read(resumed.join(coverage::PATH)).unwrap(), bytes);
    assert_eq!(
        fs::read(resumed.join("forever-package.json")).unwrap(),
        manifest
    );
    assert!(!resumed.join(".forever-package-incomplete").exists());
    lab.verify(&v44_plan, &resumed, "forever-product-v44", &expected);

    let v43_package = lab.extract("v43-package", &v43_plan, "forever-product-v43");
    lab.verify_failure_for_package(&v44_plan, &v43_package, "forever-product-v44");
    lab.verify_failure_for_package(&v43_plan, &package, "forever-product-v43");
}

#[test]
fn v44_rejects_resealed_v16_coverage_and_wrong_profile_identity_attacks() {
    let lab = Lab::new();
    let plan = lab.v44_plan();
    let package = lab.extract("attacks-base", &plan, "forever-product-v44");
    let bytes = fs::read(package.join(coverage::PATH)).unwrap();

    let v16 = {
        let mut mutated = bytes.clone();
        mutated[8..12].copy_from_slice(&16_u32.to_le_bytes());
        mutated
    };
    let resealed = lab.rebuilt_with_coverage(&package, "resealed-v16", &v16);
    lab.verify_failure_for_package(&plan, &resealed, "forever-product-v44");

    for identity in [
        coverage::COVERAGE_PROFILE,
        coverage::SOURCE_PROFILE,
        coverage::DOMAIN_PROFILE,
        coverage::RESOURCE_PROFILE,
    ] {
        let mut changed = bytes.clone();
        mutate_text_field(&mut changed, identity);
        let attacked =
            lab.rebuilt_with_coverage(&package, &format!("resealed-{}", identity.len()), &changed);
        lab.verify_failure_for_package(&plan, &attacked, "forever-product-v44");
    }
}

fn mutate_text_field(bytes: &mut [u8], value: &str) {
    let encoded = value.as_bytes();
    let start = bytes
        .windows(encoded.len())
        .enumerate()
        .find_map(|(at, window)| {
            if window != encoded || at < 4 {
                return None;
            }
            let length = u32::from_le_bytes(bytes[at - 4..at].try_into().unwrap()) as usize;
            (length == encoded.len()).then_some(at)
        })
        .unwrap_or_else(|| panic!("missing v44 coverage text field {value}"));
    bytes[start] ^= 1;
}

#[test]
fn v44_rejects_all_ten_resealed_pairwise_omission_overlaps() {
    let lab = Lab::new();
    let plan = lab.v44_plan();
    let package = lab.extract("overlap-base", &plan, "forever-product-v44");
    let bytes = fs::read(package.join(coverage::PATH)).unwrap();
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
        let parsed = coverage::parse(&bytes);
        let mut overlapping = parsed.omissions.clone();
        overlapping[left] = vec![3_000_000];
        overlapping[right] = vec![3_000_000];
        let mutated = coverage::replace_omissions(&bytes, &overlapping).unwrap();
        let attacked =
            lab.rebuilt_with_coverage(&package, &format!("overlap-{left}-{right}"), &mutated);
        lab.verify_failure_for_package(&plan, &attacked, "forever-product-v44");
    }
}

#[test]
fn v44_five_omission_lists_enforce_4096_and_4097_with_both_verifiers() {
    let lab = Lab::new();
    let plan = lab.v44_plan();
    let package = lab.extract("lists-base", &plan, "forever-product-v44");
    let bytes = fs::read(package.join(coverage::PATH)).unwrap();
    let ids: Vec<_> = (3_000_000..3_004_097).collect();
    for (index, name) in LIST_ORDER.iter().enumerate() {
        let at_limit = coverage::replace_omission(&bytes, index, &ids[..4096]).unwrap();
        let accepted = lab.rebuilt_with_coverage(&package, &format!("{name}-4096"), &at_limit);
        lab.verify(
            &plan,
            &accepted,
            "forever-product-v44",
            &["\"full_extractor_complete\":false"],
        );
        let over_limit = coverage::replace_omission(&bytes, index, &ids[..4097]).unwrap();
        let rejected = lab.rebuilt_with_coverage(&package, &format!("{name}-4097"), &over_limit);
        lab.verify_failure_for_package(&plan, &rejected, "forever-product-v44");
    }
}

#[test]
fn v44_reduced_plan_reports_preserved_limits_without_a_real_boundary_claim() {
    let lab = Lab::new();
    let plan = lab.v44_plan();
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
        "v44 reduced-budget inspect",
        "forever-product-v44",
    );
    for expected in [
        "\"source_bytes_max\":2147483648",
        "\"models_max\":65536",
        "\"reads_max\":1000000",
        "\"model_bytes_max\":536870912",
    ] {
        assert!(report.contains(expected), "missing {expected}: {report}");
    }
    assert!(!report.contains("\"real_2_gib_boundary\":true"));
    assert!(!report.contains("\"full_extractor_complete\":true"));
}

#[cfg(test)]
mod json_tests {
    #[test]
    fn contract_records_v44_failed_real_attempt_without_acceptance() {
        let contract = include_str!("../../../contracts/forever-product-contract-v44.json");
        assert!(contract.contains("forever-product-v44"));
        assert!(contract.contains("linux-collision-source-expanded-resources-v2"));
        assert!(contract.contains("target-not-run"));
        assert!(contract.contains("\"real_target_attempted\": true"));
        assert!(contract.contains("\"real_extraction_accepted\": false"));
        assert!(contract.contains("\"real_2_gib_boundary\": false"));
        assert!(
            contract.contains("2a29530f77a5f6f72415288f6c6c266d8697fc424d95b6e0c7db8e7891a1966d")
        );
        assert!(
            contract.contains("69c44bfae09f60200640758c741d2f0607dd893fa5bbede96fae3bfed96e1a81")
        );
        assert!(contract.contains("\"observed_excess_bytes\": 3245"));
        assert!(contract.contains("\"wmo_462695_admission\": false"));
        assert!(contract.contains("\"full_world_coverage\": false"));
        assert!(contract.contains("\"runtime_acceptance\": false"));
    }
}
