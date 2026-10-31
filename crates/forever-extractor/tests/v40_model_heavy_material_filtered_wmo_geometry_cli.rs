// SPDX-License-Identifier: GPL-3.0-or-later
#![cfg(target_os = "linux")]

#[path = "v40_model_heavy_material_filtered_wmo_geometry_cli/coverage.rs"]
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

const PLAN_RELATIVE: &str = "../../tests/contracts/fixtures/forever-product-v40/all.tsv";
const V39_PLAN: &str =
    include_str!("../../../tests/contracts/fixtures/forever-product-v39/all.tsv");
static NEXT: AtomicU64 = AtomicU64::new(0);

struct Lab(PathBuf);

impl Lab {
    fn new() -> Self {
        let root = std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join(format!(
                ".local/share/rce-v40-model-heavy-material-filtered-wmo-cli-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
        fs::create_dir_all(root.parent().unwrap()).unwrap();
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        Self(root)
    }

    fn plan(&self) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join(PLAN_RELATIVE)
    }

    fn v39_plan(&self) -> PathBuf {
        let path = self.0.join("plan-v39.tsv");
        fs::write(&path, V39_PLAN).unwrap();
        path
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

    fn extract(&self, name: &str) -> PathBuf {
        self.extract_with_plan(name, &self.plan())
    }

    fn extract_with_plan(&self, name: &str, plan: &Path) -> PathBuf {
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
        assert!(
            output.status.success(),
            "{name}: synthetic extract exited {:?}: {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            output.stderr.is_empty(),
            "synthetic extract wrote to stderr"
        );
        let report = String::from_utf8(output.stdout).unwrap();
        assert!(
            report.starts_with('{') && report.ends_with('\n'),
            "{report}"
        );
        let contract = if fs::read_to_string(plan)
            .unwrap()
            .contains("forever-product\t39")
        {
            "forever-product-v39"
        } else {
            "forever-product-v40"
        };
        assert!(
            report.contains(&format!("\"contract\":\"{contract}\"")),
            "{report}"
        );
        package
    }

    fn verify(&self, plan: &Path, package: &Path) {
        for verb in ["verify", "verify-links"] {
            let output = self
                .command(verb, plan)
                .args(["--output"])
                .arg(package)
                .output()
                .unwrap();
            let report = assert_success(output, verb);
            for expected in [
                "\"verification_profile\":\"linux70450-model-heavy-material-filtered-wmo-verification-v1\"",
                "\"scope\":\"artifact_integrity_model_heavy_material_filtered_wmo_and_independently_derived_map_vmap_domain_coverage\"",
                "\"complete_global_map_coverage_claim\":false",
                "\"complete_vmap_component\":false",
                "\"full_extractor_complete\":false",
            ] {
                assert!(
                    report.contains(expected),
                    "{verb}: missing {expected}: {report}"
                );
            }
        }
    }

    fn verify_failure(&self, plan: &Path, package: &Path) {
        self.verify_failure_code(plan, package, None);
    }

    fn verify_failure_code(&self, plan: &Path, package: &Path, expected_code: Option<&str>) {
        for verb in ["verify", "verify-links"] {
            let output = self
                .command(verb, plan)
                .args(["--output"])
                .arg(package)
                .output()
                .unwrap();
            assert!(
                !output.status.success(),
                "{verb} accepted a malformed v40 package"
            );
            let text = String::from_utf8(output.stderr).unwrap();
            assert!(
                text.contains("\"contract\":\"forever-product-v40\""),
                "{text}"
            );
            if let Some(expected_code) = expected_code {
                assert!(
                    text.contains(&format!("\"code\":\"{expected_code}\"")),
                    "{verb}: expected {expected_code}: {text}"
                );
            } else {
                assert!(text.contains("\"code\":"), "{text}");
            }
        }
    }

    fn rebuilt_with_coverage(&self, base: &Path, name: &str, coverage_bytes: &[u8]) -> PathBuf {
        let package_context = package_context(base);
        let mut files = package_files(base);
        files.insert(coverage::PATH.to_owned(), coverage_bytes.to_vec());
        let output = self.0.join(name);
        let declaration = fixture::partial_map().declaration;
        let mut run = Run::create(&output, package_context, &[base], Limits::default()).unwrap();
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

    fn reject_package(&self, plan: &Path, package: &Path, contract: &str) {
        for verb in ["verify", "verify-links"] {
            let output = self
                .command(verb, plan)
                .args(["--output"])
                .arg(package)
                .output()
                .unwrap();
            assert!(
                !output.status.success(),
                "{verb} accepted a crossed or tampered package"
            );
            let text = String::from_utf8(output.stderr).unwrap();
            assert!(
                text.contains(&format!("\"contract\":\"{contract}\"")),
                "{text}"
            );
            assert!(text.contains("\"code\":"), "{text}");
        }
    }
}

impl Drop for Lab {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn assert_success(output: Output, label: &str) -> String {
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
        report.contains("\"contract\":\"forever-product-v40\""),
        "{label}: {report}"
    );
    assert!(report.contains("\"models_max\":65536"), "{label}: {report}");
    assert!(
        report.contains("\"reads_max\":1000000"),
        "{label}: {report}"
    );
    assert!(
        report.contains("linux-collision-model-heavy-resources-v1"),
        "{label}: {report}"
    );
    report
}

#[test]
fn model_heavy_profile_raises_model_identity_limits_and_preserves_non_identity_units() {
    let model = Profile::ModelHeavyV1;
    let read = Profile::ReadHeavyV1;
    assert_eq!(model.id(), "linux-collision-model-heavy-resources-v1");
    let collector = model.collector_limits();
    assert_eq!(collector.max_models, 65_536);
    assert_eq!(collector.max_reads, 1_000_000);
    assert_eq!(collector.max_requests, 16_000_000);
    assert_eq!(collector.max_dependency_requests, 16_000_000);
    assert_eq!(collector.max_source_bytes, 512 * 1024 * 1024);
    assert_eq!(collector.max_raw_bytes, 256 * 1024 * 1024);

    let startup = model.startup_limits();
    assert_eq!(startup.collision.max_models, 65_536);
    assert_eq!(startup.collision.max_reads, 1_000_000);
    assert_eq!(startup.collision.max_requests, 16_000_000);
    assert_eq!(startup.collision.max_dependency_requests, 16_000_000);
    assert_eq!(startup.collision.max_source_bytes, 512 * 1024 * 1024);
    assert_eq!(startup.collision.max_raw_bytes, 256 * 1024 * 1024);

    let inventory = model.inventory_limits();
    let read_inventory = read.inventory_limits();
    let stage = model.stage_limits();
    let read_stage = read.stage_limits();
    assert_eq!(inventory.max_entries, 4096);
    assert_eq!(inventory.max_entries, read_inventory.max_entries);
    assert_eq!(stage.max_callbacks, 4096);
    assert_eq!(stage.max_callbacks, read_stage.max_callbacks);

    let identity_caps = [
        collector.max_models,
        startup.collision.max_models,
        model.spool_limits().max_records,
        inventory.max_entries,
        stage.max_callbacks,
        stage.world.world.run.placements.max_models,
        model.model_limits().max_models,
        model.package_limits().max_models,
    ];
    assert_eq!(
        identity_caps,
        [65_536, 65_536, 65_536, 4096, 4096, 65_536, 65_536, 65_536],
        "capacity routes in model/record/entry/callback units"
    );

    let mut expected = read.collector_limits();
    expected.max_models = 65_536;
    assert_eq!(collector, expected);
}

#[test]
fn v40_plan_rejects_a_genuine_v39_package() {
    let lab = Lab::new();
    let v39_plan = lab.v39_plan();
    let genuine_v39 = lab.extract_with_plan("genuine-v39", &v39_plan);
    lab.reject_package(&lab.plan(), &genuine_v39, "forever-product-v40");
}

#[test]
fn v40_inspect_extract_resume_verify_and_verify_links_are_v13_bound() {
    let lab = Lab::new();
    let plan = lab.plan();
    assert!(
        plan.is_file(),
        "canonical v40 fixture is not integrated: {}",
        plan.display()
    );
    let inspect = assert_success(
        lab.command("inspect", &plan).output().unwrap(),
        "v40 inspect",
    );
    for expected in [
        "linux70450-model-heavy-material-filtered-wmo-coverage-v13",
        "linux-collision-model-heavy-resources-v1",
        "\"complete_global_map_coverage_claim\":false",
        "\"complete_vmap_component\":false",
        "\"full_extractor_complete\":false",
    ] {
        assert!(inspect.contains(expected), "{inspect}");
    }

    let package = lab.extract("lifecycle");
    let second = lab.extract("lifecycle-second");
    assert_eq!(package_files(&package), package_files(&second));
    assert_eq!(
        fs::read(package.join("forever-package.json")).unwrap(),
        fs::read(second.join("forever-package.json")).unwrap()
    );
    let coverage_bytes = fs::read(package.join(coverage::PATH)).unwrap();
    let parsed = coverage::Coverage::parse(&coverage_bytes).unwrap();
    assert_eq!(parsed.version, 13);
    assert_eq!(parsed.coverage_profile, coverage::COVERAGE_PROFILE);
    assert_eq!(parsed.source_profile, coverage::SOURCE_PROFILE);
    assert_eq!(parsed.domain_profile, coverage::DOMAIN_PROFILE);
    assert_eq!(parsed.resource_profile, coverage::RESOURCE_PROFILE);
    assert_eq!(parsed.omissions[0], [fixture::MISSING_M2]);
    assert_eq!(parsed.omissions[1], [fixture::MISSING_WMO]);
    assert_eq!(parsed.omissions[2], [fixture::EMPTY_WMO]);
    assert_eq!(parsed.omissions[3], [fixture::UNSELECTED_WMO]);
    assert_eq!(parsed.omissions[4], [fixture::MATERIAL_FILTERED_WMO_MIXED]);
    lab.verify(&plan, &package);

    let resumed = lab.0.join("resumed-package");
    copy_tree(&package, &resumed);
    fs::remove_file(resumed.join("forever-package.json")).unwrap();
    fs::write(resumed.join(".forever-package-incomplete"), b"incomplete\n").unwrap();
    let client = lab.private_dir("resume-client");
    fixture::write_client_with(
        &client,
        fixture::ClientMutation {
            populated_adt_mode: fixture::AdtMode::MaterialFilteredWmoMixed,
            ..Default::default()
        },
    );
    let scratch = lab.private_dir("resume-scratch");
    assert_success(
        lab.produce("resume", &plan, &client, &resumed, &scratch),
        "v40 resume",
    );
    assert_eq!(
        fs::read(resumed.join(coverage::PATH)).unwrap(),
        coverage_bytes
    );
    assert_eq!(
        fs::read(resumed.join("forever-package.json")).unwrap(),
        fs::read(package.join("forever-package.json")).unwrap()
    );
    lab.verify(&plan, &resumed);

    let v39 = lab.v39_plan();
    lab.reject_package(&v39, &package, "forever-product-v39");
    lab.reject_package(
        &plan,
        &package_with_tampered_coverage(&lab, &package),
        "forever-product-v40",
    );

    let v12_bytes = {
        let mut bytes = coverage_bytes.clone();
        bytes[8..12].copy_from_slice(&12_u32.to_le_bytes());
        bytes
    };
    let v12_resealed = lab.rebuilt_with_coverage(&package, "v12-codec-resealed", &v12_bytes);
    lab.verify_failure_code(
        &plan,
        &v12_resealed,
        Some("product_selected_geometry_coverage_schema"),
    );

    let overlaps = [
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
    for (left, right) in overlaps {
        let mut overlapping = parsed.omissions.clone();
        overlapping[left] = vec![3_000_000];
        overlapping[right] = vec![3_000_000];
        let mut bytes = coverage_bytes.clone();
        for list_index in (0..coverage::LIST_ORDER.len()).rev() {
            bytes = parsed
                .replace_omission(&bytes, list_index, &overlapping[list_index])
                .unwrap();
        }
        let rejected = lab.rebuilt_with_coverage(
            &package,
            &format!(
                "overlap-{}-{}",
                coverage::LIST_ORDER[left],
                coverage::LIST_ORDER[right]
            ),
            &bytes,
        );
        lab.verify_failure_code(
            &plan,
            &rejected,
            Some("product_selected_geometry_coverage_schema"),
        );
    }

    let boundary_ids: Vec<_> = (3_000_000..3_004_097).collect();
    for list_index in 0..coverage::LIST_ORDER.len() {
        let at_limit = &boundary_ids[..4096];
        let accepted_bytes = parsed
            .replace_omission(&coverage_bytes, list_index, at_limit)
            .unwrap();
        let accepted = lab.rebuilt_with_coverage(
            &package,
            &format!("{}-4096", coverage::LIST_ORDER[list_index]),
            &accepted_bytes,
        );
        lab.verify(&plan, &accepted);

        let over_limit = &boundary_ids[..4097];
        let rejected_bytes = parsed
            .replace_omission(&coverage_bytes, list_index, over_limit)
            .unwrap();
        let rejected = lab.rebuilt_with_coverage(
            &package,
            &format!("{}-4097", coverage::LIST_ORDER[list_index]),
            &rejected_bytes,
        );
        lab.verify_failure(&plan, &rejected);
    }
}

fn package_context(package: &Path) -> Context {
    Context::parse(&fs::read_to_string(package.join(".state/context.json")).unwrap()).unwrap()
}

fn package_artifacts(package: &Path, context: Context) -> Vec<Artifact> {
    fs::read_dir(package.join(".state/files"))
        .unwrap()
        .map(|entry| {
            Artifact::parse(
                &fs::read_to_string(entry.unwrap().path()).unwrap(),
                context,
                Limits::default(),
            )
            .unwrap()
        })
        .collect()
}

fn package_files(package: &Path) -> std::collections::BTreeMap<String, Vec<u8>> {
    package_artifacts(package, package_context(package))
        .into_iter()
        .map(|artifact| {
            let bytes = fs::read(package.join(&artifact.path)).unwrap();
            (artifact.path, bytes)
        })
        .collect()
}

fn package_with_tampered_coverage(lab: &Lab, package: &Path) -> PathBuf {
    let tampered = lab.0.join("tampered-package");
    copy_tree(package, &tampered);
    let path = tampered.join(coverage::PATH);
    let mut bytes = fs::read(&path).unwrap();
    bytes[8..12].copy_from_slice(&12_u32.to_le_bytes());
    fs::write(path, bytes).unwrap();
    tampered
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
fn synthetic_omission_lists_stay_pairwise_disjoint_and_bounded() {
    let mixed = [vec![10], vec![20], vec![30], vec![40], vec![50]];
    for left in 0..mixed.len() {
        assert!(coverage::canonical_ids(&mixed[left]));
        for right in left + 1..mixed.len() {
            assert!(mixed[left].iter().all(|id| !mixed[right].contains(id)));
        }
    }
    let maximum: Vec<_> = (1..=4096).collect();
    let too_many: Vec<_> = (1..=4097).collect();
    assert!(coverage::canonical_ids(&maximum));
    assert!(!coverage::canonical_ids(&too_many));
}

#[test]
fn v40_reduced_plan_budget_does_not_raise_nominal_resource_caps() {
    let lab = Lab::new();
    let plan = lab.plan();
    assert!(
        plan.is_file(),
        "canonical v40 fixture is not integrated: {}",
        plan.display()
    );
    let raw = fs::read_to_string(&plan).unwrap();
    let reduced = raw.replace(
        "collision-raw-bytes\t1048576",
        "collision-raw-bytes\t921600",
    );
    assert_ne!(raw, reduced);
    let reduced_plan = lab.0.join("plan-reduced.tsv");
    fs::write(&reduced_plan, reduced).unwrap();
    let inspect = assert_success(
        lab.command("inspect", &reduced_plan).output().unwrap(),
        "v40 reduced-budget inspect",
    );
    assert!(inspect.contains("\"models_max\":65536"));
    assert!(inspect.contains("\"reads_max\":1000000"));
}

#[cfg(test)]
mod json_tests {
    #[test]
    fn structural_contract_names_the_canonical_v40_fixture_without_embedding_it() {
        let contract = include_str!("../../../contracts/forever-product-contract-v40.json");
        assert!(contract.contains("tests/contracts/fixtures/forever-product-v40/all.tsv"));
        assert!(contract.contains("\"fixture_status\": \"canonical synthetic plan fixture supplied; contains no client data or real extraction evidence\""));
        let real_acceptance = contract
            .split_once("\"real_acceptance\": {")
            .unwrap()
            .1
            .split_once("\n  },")
            .unwrap()
            .0;
        assert!(real_acceptance.contains("\"run_id\": \"v40-model-heavy-wmo-20261027T000044Z\""));
        assert!(real_acceptance.contains("\"status\": \"failed\""));
        assert!(real_acceptance.contains("\"accepted\": false"));
        assert!(real_acceptance.contains("\"accepted_prefix\": false"));
    }
}
