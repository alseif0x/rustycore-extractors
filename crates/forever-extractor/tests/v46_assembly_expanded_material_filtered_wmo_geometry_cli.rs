// SPDX-License-Identifier: GPL-3.0-or-later
#![cfg(target_os = "linux")]

#[path = "v46_assembly_expanded_material_filtered_wmo_geometry_cli/coverage.rs"]
mod coverage;
#[path = "v39_read_heavy_material_filtered_wmo_geometry_cli/fixture.rs"]
#[allow(dead_code)]
mod fixture;
#[path = "v46_assembly_expanded_material_filtered_wmo_geometry_cli/json.rs"]
mod json;
#[path = "v43_model_bytes_expanded_material_filtered_wmo_geometry_cli/coverage.rs"]
#[allow(dead_code)]
mod v43_coverage;

use forever_casc::{
    collision::{plan, resources::Profile},
    package::{Artifact, Component, Context, Limits, Run},
};
use forever_db2::availability;
use json::JsonValue;
use std::{
    collections::BTreeMap,
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};

const V45_PLAN_RELATIVE: &str = "../../tests/contracts/fixtures/forever-product-v45/all.tsv";
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
                ".local/share/rce-v46-assembly-expanded-material-filtered-wmo-cli-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
        fs::create_dir_all(root.parent().unwrap()).unwrap();
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        Self(root)
    }

    fn v45_plan(&self) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join(V45_PLAN_RELATIVE)
    }

    fn v46_plan(&self) -> PathBuf {
        let source = fs::read_to_string(self.v45_plan()).unwrap();
        let mut transformed = source.clone();
        for (from, to) in [
            ("forever-product\t45", "forever-product\t46"),
            (
                "linux70450-placement-expanded-material-filtered-wmo-vmap-product-v1",
                "linux70450-assembly-expanded-material-filtered-wmo-vmap-product-v1",
            ),
            (
                "linux70450-placement-expanded-material-filtered-wmo-coverage-v18",
                "linux70450-assembly-expanded-material-filtered-wmo-coverage-v19",
            ),
        ] {
            assert!(
                transformed.contains(from),
                "missing v45 source PLAN value {from}"
            );
            transformed = transformed.replace(from, to);
        }
        assert_ne!(source, transformed);
        let plan = self.0.join("plan-v46.tsv");
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
        let expected_placements =
            (contract == "forever-product-v46").then(|| placement_diagnostics(&package));
        let report =
            assert_success_for_contract(output, "extract", contract, expected_placements.as_ref());
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
            let expected_placements =
                (contract == "forever-product-v46").then(|| placement_diagnostics(package));
            let report =
                assert_success_for_contract(output, verb, contract, expected_placements.as_ref());
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

#[derive(Debug, PartialEq, Eq)]
struct PlacementDiagnostics {
    total: usize,
    per_map: std::collections::BTreeMap<u32, usize>,
    domain: Vec<u32>,
}

struct MapBytes<'a>(&'a [u8]);

impl availability::Source for MapBytes<'_> {
    fn declared_bytes(&self) -> u64 {
        self.0.len() as u64
    }

    fn range(&self, offset: u64, bytes: usize) -> Result<&[u8], availability::Unavailable> {
        let start = usize::try_from(offset).map_err(|_| availability::Unavailable)?;
        self.0
            .get(start..start.checked_add(bytes).ok_or(availability::Unavailable)?)
            .ok_or(availability::Unavailable)
    }
}

fn placement_diagnostics(package: &Path) -> PlacementDiagnostics {
    let map_bytes = fs::read(package.join("dbc/esES/Map.db2")).unwrap();
    let available = availability::read_vmap_maps_70450(&MapBytes(&map_bytes)).unwrap();
    let direct_count = available.direct_records().len();
    let maps = available.materialize().unwrap();
    let selected = plan::Maps::from_materialized_selection(
        maps.rows(),
        direct_count,
        plan::MapSelection::All,
        Default::default(),
    )
    .unwrap();
    let mut domain: Vec<_> = selected.selected_ids().collect();
    domain.sort_unstable();
    let coverage_dir = package.join("vmaps/coverage");
    let total = match fs::read_dir(&coverage_dir) {
        Ok(entries) => entries
            .map(|entry| entry.unwrap().path())
            .filter(|path| {
                path.extension()
                    .is_some_and(|extension| extension == "placements")
            })
            .map(|path| {
                let bytes = fs::read(path).unwrap();
                forever_vmap::placement::DirBin::parse(&bytes, Default::default())
                    .unwrap()
                    .record_count()
            })
            .sum(),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => 0,
        Err(error) => panic!("cannot read placement coverage directory: {error}"),
    };
    let mut per_map = std::collections::BTreeMap::new();
    for id in &domain {
        let path = coverage_dir.join(format!("{id:04}.placements"));
        let count = if path.exists() {
            let bytes = fs::read(path).unwrap();
            forever_vmap::placement::DirBin::parse(&bytes, Default::default())
                .unwrap()
                .record_count()
        } else {
            0
        };
        assert!(per_map.insert(*id, count).is_none());
    }
    PlacementDiagnostics {
        total,
        per_map,
        domain,
    }
}

fn assert_success_for_contract(
    output: Output,
    label: &str,
    contract: &str,
    expected_placements: Option<&PlacementDiagnostics>,
) -> String {
    assert!(
        output.status.success(),
        "{label} exited {:?}: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty(), "{label} wrote to stderr");
    let report = String::from_utf8(output.stdout).unwrap();
    assert_eq!(report.matches('\n').count(), 1, "{label}: {report}");
    let parsed = json::parse(&report)
        .unwrap_or_else(|error| panic!("{label}: invalid CLI JSON ({error}): {report}"));
    let JsonValue::Object(fields) = parsed else {
        panic!("{label}: CLI JSON root must be an object: {report}");
    };
    assert_eq!(
        fields.get("contract"),
        Some(&JsonValue::String(contract.to_owned())),
        "{label}: {report}"
    );
    match contract {
        "forever-product-v46" => {
            assert_v46_resource_fields(&fields)
                .unwrap_or_else(|error| panic!("{label}: {error}: {report}"));
            assert_eq!(
                fields.get("resource_profile"),
                Some(&JsonValue::String(
                    "linux-collision-assembly-expanded-resources-v1".to_owned()
                )),
                "{label}: {report}"
            );
            assert_eq!(
                fields.get("collision_source_profile"),
                Some(&JsonValue::String(
                    "linux70450-assembly-expanded-material-filtered-wmo-v1".to_owned()
                )),
                "{label}: {report}"
            );
            check_optional_placement_diagnostics(&fields, expected_placements)
                .unwrap_or_else(|error| panic!("{label}: {error}: {report}"));
        }
        "forever-product-v45" => {
            for expected in [
                "\"source_bytes_max\":2147483648",
                "\"models_max\":65536",
                "\"reads_max\":1000000",
                "\"model_bytes_max\":536870912",
                "linux-collision-placement-expanded-resources-v1",
            ] {
                assert!(
                    report.contains(expected),
                    "{label}: missing {expected}: {report}"
                );
            }
        }
        _ => {}
    }
    report
}

fn assert_v46_resource_fields(
    fields: &std::collections::BTreeMap<String, JsonValue>,
) -> Result<(), String> {
    for (name, expected) in [
        ("source_bytes_max", 2_147_483_648_u64),
        ("models_max", 65_536),
        ("reads_max", 1_000_000),
        ("request_calls_max", 16_000_000),
        ("dependency_requests_max", 16_000_000),
        ("raw_bytes_max", 268_435_456),
        ("model_bytes_max", 536_870_912),
        ("placement_bytes_max", 268_435_456),
        ("placement_bytes_max_per_map", 67_108_864),
        ("assembled_domain_bytes_max", 268_435_456),
    ] {
        let expected = JsonValue::Number(expected.to_string());
        if fields.get(name) != Some(&expected) {
            return Err(format!("{name} must be exactly {expected:?}"));
        }
    }
    Ok(())
}

fn check_optional_placement_diagnostics(
    fields: &std::collections::BTreeMap<String, JsonValue>,
    expected: Option<&PlacementDiagnostics>,
) -> Result<(), String> {
    let names = ["total_placements", "per_map_placements", "placement_domain"];
    let present = names.iter().any(|name| fields.contains_key(*name));
    if !present {
        return Ok(());
    }
    let expected = expected.ok_or_else(|| {
        "placement diagnostics were emitted without an independent synthetic oracle".to_owned()
    })?;

    if let Some(value) = fields.get("total_placements")
        && value != &JsonValue::Number(expected.total.to_string())
    {
        return Err(format!(
            "total_placements must be the independently counted value {}",
            expected.total
        ));
    }
    if let Some(value) = fields.get("per_map_placements") {
        let expected_map: BTreeMap<_, _> = expected
            .per_map
            .iter()
            .map(|(id, count)| (id.to_string(), JsonValue::Number(count.to_string())))
            .collect();
        if value != &JsonValue::Object(expected_map) {
            return Err(
                "per_map_placements must exactly match every active map ID and count".into(),
            );
        }
    }
    if let Some(value) = fields.get("placement_domain") {
        let expected_domain: Vec<_> = expected
            .domain
            .iter()
            .map(|id| JsonValue::Number(id.to_string()))
            .collect();
        if value != &JsonValue::Array(expected_domain) {
            return Err(
                "placement_domain must exactly match the independently derived ordered map IDs"
                    .into(),
            );
        }
    }
    Ok(())
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
fn v46_assembly_expansion_preserves_v45_resource_limits() {
    let assembly = Profile::AssemblyExpandedV1;
    let placement = Profile::PlacementExpandedV1;
    assert_eq!(assembly.id(), coverage::RESOURCE_PROFILE);
    assert_eq!(assembly.collector_limits(), placement.collector_limits());
    assert_eq!(assembly.startup_limits(), placement.startup_limits());
    assert_eq!(assembly.spool_limits(), placement.spool_limits());
    assert_eq!(assembly.list_profile(), placement.list_profile());
    assert_eq!(assembly.inventory_limits(), placement.inventory_limits());
    assert_eq!(assembly.stage_limits(), placement.stage_limits());
    assert_eq!(assembly.model_limits(), placement.model_limits());
    assert_eq!(assembly.package_limits(), placement.package_limits());
}

#[test]
fn v46_lifecycle_resume_determinism_and_bidirectional_v45_cross_version_rejection() {
    let lab = Lab::new();
    let v45_plan = lab.v45_plan();
    let v46_plan = lab.v46_plan();
    let inspect = assert_success_for_contract(
        lab.command("inspect", &v46_plan).output().unwrap(),
        "v46 inspect",
        "forever-product-v46",
        None,
    );
    assert!(inspect.contains(coverage::COVERAGE_PROFILE));
    assert!(inspect.contains(
        "\"collision_source_profile\":\"linux70450-assembly-expanded-material-filtered-wmo-v1\""
    ));
    assert!(
        inspect.contains("\"resource_profile\":\"linux-collision-assembly-expanded-resources-v1\"")
    );
    assert!(inspect.contains("\"assembled_domain_bytes_max\":268435456"));
    assert!(!inspect.contains("\"real_256_mib_assembled_domain_boundary\":true"));
    assert!(inspect.contains("\"full_extractor_complete\":false"));

    let expected = [
        "\"verification_profile\":\"linux70450-assembly-expanded-material-filtered-wmo-verification-v1\"",
        "\"scope\":\"artifact_integrity_assembly_expanded_material_filtered_wmo_and_independently_derived_map_vmap_domain_coverage\"",
        "\"complete_global_map_coverage_claim\":false",
        "\"complete_vmap_component\":false",
        "\"full_extractor_complete\":false",
    ];
    let package = lab.extract("v46-first", &v46_plan, "forever-product-v46");
    let second = lab.extract("v46-second", &v46_plan, "forever-product-v46");
    let files_before_resume = package_files(&package);
    assert_eq!(files_before_resume, package_files(&second));
    let manifest = fs::read(package.join("forever-package.json")).unwrap();
    let bytes = fs::read(package.join(coverage::PATH)).unwrap();
    assert_eq!(u32::from_le_bytes(bytes[8..12].try_into().unwrap()), 19);
    let parsed = coverage::parse(&bytes);
    assert_eq!(parsed.version, 16);
    assert_eq!(parsed.coverage_profile, coverage::V43_COVERAGE_PROFILE);
    assert_eq!(parsed.source_profile, coverage::V43_SOURCE_PROFILE);
    assert_eq!(parsed.domain_profile, coverage::V43_DOMAIN_PROFILE);
    assert_eq!(parsed.resource_profile, coverage::V43_RESOURCE_PROFILE);
    lab.verify(&v46_plan, &package, "forever-product-v46", &expected);
    assert!(!files_before_resume.contains_key("vmaps/GameObjectModels.dtree"));
    assert!(!files_before_resume.contains_key("vmaps/FILE0000012E.xxx.vmo"));

    let resumed = lab.0.join("v46-resumed-package");
    copy_tree(&package, &resumed);
    fs::remove_file(resumed.join("forever-package.json")).unwrap();
    fs::write(resumed.join(".forever-package-incomplete"), b"incomplete\n").unwrap();
    let client = lab.private_dir("v46-resume-client");
    fixture::write_client_with(
        &client,
        fixture::ClientMutation {
            populated_adt_mode: fixture::AdtMode::MaterialFilteredWmoMixed,
            ..Default::default()
        },
    );
    let scratch = lab.private_dir("v46-resume-scratch");
    assert_success_for_contract(
        lab.produce("resume", &v46_plan, &client, &resumed, &scratch),
        "v46 resume",
        "forever-product-v46",
        Some(&placement_diagnostics(&resumed)),
    );
    assert_eq!(package_files(&resumed), files_before_resume);
    assert_eq!(fs::read(resumed.join(coverage::PATH)).unwrap(), bytes);
    assert_eq!(
        fs::read(resumed.join("forever-package.json")).unwrap(),
        manifest
    );
    assert!(!resumed.join(".forever-package-incomplete").exists());
    lab.verify(&v46_plan, &resumed, "forever-product-v46", &expected);

    let v45_package = lab.extract("v45-package", &v45_plan, "forever-product-v45");
    lab.verify_failure_for_package(&v46_plan, &v45_package, "forever-product-v46");
    lab.verify_failure_for_package(&v45_plan, &package, "forever-product-v45");
}

#[test]
fn v46_rejects_resealed_prior_version_and_wrong_profile_identity_attacks() {
    let lab = Lab::new();
    let plan = lab.v46_plan();
    let package = lab.extract("attacks-base", &plan, "forever-product-v46");
    let bytes = fs::read(package.join(coverage::PATH)).unwrap();

    let v18 = {
        let mut mutated = bytes.clone();
        mutated[8..12].copy_from_slice(&18_u32.to_le_bytes());
        mutated
    };
    let resealed = lab.rebuilt_with_coverage(&package, "resealed-v18", &v18);
    lab.verify_failure_for_package(&plan, &resealed, "forever-product-v46");

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
        lab.verify_failure_for_package(&plan, &attacked, "forever-product-v46");
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
        .unwrap_or_else(|| panic!("missing v46 coverage text field {value}"));
    bytes[start] ^= 1;
}

#[test]
fn v46_rejects_all_ten_resealed_pairwise_omission_overlaps() {
    let lab = Lab::new();
    let plan = lab.v46_plan();
    let package = lab.extract("overlap-base", &plan, "forever-product-v46");
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
        lab.verify_failure_for_package(&plan, &attacked, "forever-product-v46");
    }
}

#[test]
fn v46_five_omission_lists_enforce_4096_and_4097_with_both_verifiers() {
    let lab = Lab::new();
    let plan = lab.v46_plan();
    let package = lab.extract("lists-base", &plan, "forever-product-v46");
    let bytes = fs::read(package.join(coverage::PATH)).unwrap();
    let ids: Vec<_> = (3_000_000..3_004_097).collect();
    for (index, name) in LIST_ORDER.iter().enumerate() {
        let at_limit = coverage::replace_omission(&bytes, index, &ids[..4096]).unwrap();
        let accepted = lab.rebuilt_with_coverage(&package, &format!("{name}-4096"), &at_limit);
        lab.verify(
            &plan,
            &accepted,
            "forever-product-v46",
            &["\"full_extractor_complete\":false"],
        );
        let over_limit = coverage::replace_omission(&bytes, index, &ids[..4097]).unwrap();
        let rejected = lab.rebuilt_with_coverage(&package, &format!("{name}-4097"), &over_limit);
        lab.verify_failure_for_package(&plan, &rejected, "forever-product-v46");
    }
}

#[test]
fn v46_reduced_plan_inspect_reports_placement_and_preserved_limits() {
    let lab = Lab::new();
    let plan = lab.v46_plan();
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
        "v46 reduced-budget inspect",
        "forever-product-v46",
        None,
    );
    for expected in [
        "\"source_bytes_max\":2147483648",
        "\"models_max\":65536",
        "\"reads_max\":1000000",
        "\"model_bytes_max\":536870912",
        "\"placement_bytes_max\":268435456",
        "\"placement_bytes_max_per_map\":67108864",
        "\"assembled_domain_bytes_max\":268435456",
    ] {
        assert!(report.contains(expected), "missing {expected}: {report}");
    }
    assert!(!report.contains("\"real_256_mib_assembled_domain_boundary\":true"));
    assert!(!report.contains("\"full_extractor_complete\":true"));
}

#[cfg(test)]
mod json_tests {
    use super::{
        BTreeMap, JsonValue, PlacementDiagnostics, assert_v46_resource_fields,
        check_optional_placement_diagnostics, json,
    };

    fn parse_fields(report: &str) -> BTreeMap<String, JsonValue> {
        let JsonValue::Object(fields) = json::parse(report).unwrap() else {
            panic!("expected JSON object")
        };
        fields
    }

    fn expected_placements() -> PlacementDiagnostics {
        PlacementDiagnostics {
            total: 3,
            per_map: [(1, 0), (2, 3), (4, 0), (5, 0), (6, 0)].into(),
            domain: vec![1, 2, 4, 5, 6],
        }
    }

    #[test]
    fn contract_records_scoped_v46_acceptance_and_preserves_open_boundaries() {
        let contract = include_str!("../../../contracts/forever-product-contract-v46.json");
        assert!(contract.contains("forever-product-v46"));
        assert!(contract.contains("linux-collision-assembly-expanded-resources-v1"));
        assert!(contract.contains("scoped-real-target-accepted"));
        assert!(contract.contains("\"target_status\": \"accepted\""));
        assert!(contract.contains("\"real_target_attempted\": true"));
        assert!(contract.contains("\"real_extraction_accepted\": true"));
        assert!(
            contract.contains(
                "available_geometry_with_assembly_expanded_material_filtered_wmo_accepted"
            )
        );
        assert!(contract.contains("\"accepted_prefix\": false"));
        assert!(contract.contains("\"full_extractor_complete\": false"));
        assert!(contract.contains("\"real_256_mib_assembled_domain_boundary\": false"));
        assert!(contract.contains("\"assembled_domain_bytes_max\": 268435456"));
        assert!(contract.contains("\"root_cause_confirmed\": false"));
        assert!(contract.contains("product_selected_geometry_limit"));
        assert!(contract.contains("\"placement_capacity_sufficiency\": false"));
        assert!(contract.contains("\"assembled_domain_capacity_sufficiency\": false"));
        assert!(contract.contains("\"wmo_462695_admission\": false"));
        assert!(contract.contains("\"full_world_coverage\": false"));
        assert!(contract.contains("\"runtime_acceptance\": false"));
        assert!(
            json::parse(r#"{"placement_domain":[1,2,4,5,6],"placement_domain":[1,2,4,5,6]}"#)
                .is_err()
        );
        assert!(json::parse(r#"{"outer":{"key":1,"key":1}}"#).is_err());

        let valid_resources = parse_fields(
            r#"{"source_bytes_max":2147483648,"models_max":65536,"reads_max":1000000,"request_calls_max":16000000,"dependency_requests_max":16000000,"raw_bytes_max":268435456,"model_bytes_max":536870912,"placement_bytes_max":268435456,"placement_bytes_max_per_map":67108864,"assembled_domain_bytes_max":268435456}"#,
        );
        assert!(assert_v46_resource_fields(&valid_resources).is_ok());
        for report in [
            r#"{"source_bytes_max":2147483648,"models_max":65536,"reads_max":1000000,"request_calls_max":160000000,"dependency_requests_max":16000000,"raw_bytes_max":268435456,"model_bytes_max":536870912,"placement_bytes_max":268435456,"placement_bytes_max_per_map":67108864,"assembled_domain_bytes_max":268435456}"#,
            r#"{"source_bytes_max":2147483648,"models_max":65536,"reads_max":1000000,"request_calls_max":16000000,"dependency_requests_max":16000000,"raw_bytes_max":2684354560,"model_bytes_max":536870912,"placement_bytes_max":268435456,"placement_bytes_max_per_map":67108864,"assembled_domain_bytes_max":268435456}"#,
            r#"{"source_bytes_max":2147483648,"models_max":65536,"reads_max":1000000,"request_calls_max":16000000,"dependency_requests_max":16000000,"raw_bytes_max":"268435456","model_bytes_max":536870912,"placement_bytes_max":268435456,"placement_bytes_max_per_map":67108864,"assembled_domain_bytes_max":268435456}"#,
            r#"{"source_bytes_max":2147483648,"models_max":65536,"reads_max":1000000,"request_calls_max":16000000,"dependency_requests_max":16000000,"raw_bytes_max":268435456,"model_bytes_max":536870912,"placement_bytes_max":268435456,"placement_bytes_max_per_map":67108864,"assembled_domain_bytes_max":2684354560}"#,
            r#"{"source_bytes_max":2147483648,"models_max":65536,"reads_max":1000000,"request_calls_max":16000000,"dependency_requests_max":16000000,"raw_bytes_max":268435456,"model_bytes_max":536870912,"placement_bytes_max":268435456,"placement_bytes_max_per_map":67108864,"assembled_domain_bytes_max":"268435456"}"#,
        ] {
            assert!(
                assert_v46_resource_fields(&parse_fields(report)).is_err(),
                "accepted resource fields {report}"
            );
        }

        let expected = expected_placements();
        let absent = parse_fields(r#"{"contract":"forever-product-v46"}"#);
        assert!(check_optional_placement_diagnostics(&absent, None).is_ok());
        assert!(check_optional_placement_diagnostics(&absent, Some(&expected)).is_ok());

        for report in [
            r#"{"total_placements":3}"#,
            r#"{"per_map_placements":{"1":0,"2":3,"4":0,"5":0,"6":0}}"#,
            r#"{"placement_domain":[1,2,4,5,6]}"#,
            r#"{"total_placements":3,"per_map_placements":{"1":0,"2":3,"4":0,"5":0,"6":0},"placement_domain":[1,2,4,5,6]}"#,
        ] {
            assert!(
                check_optional_placement_diagnostics(&parse_fields(report), Some(&expected))
                    .is_ok(),
                "{report}"
            );
        }

        for report in [
            r#"{"total_placements":"3"}"#,
            r#"{"total_placements":-1}"#,
            r#"{"total_placements":4}"#,
            r#"{"per_map_placements":[0,3,0,0,0]}"#,
            r#"{"per_map_placements":{"1":0,"2":3,"4":0,"5":0}}"#,
            r#"{"per_map_placements":{"1":0,"2":2,"4":0,"5":0,"6":1}}"#,
            r#"{"placement_domain":{"1":true}}"#,
            r#"{"placement_domain":[1,2,4,5]}"#,
            r#"{"placement_domain":[1,2,4,5,7]}"#,
        ] {
            assert!(
                check_optional_placement_diagnostics(&parse_fields(report), Some(&expected))
                    .is_err(),
                "accepted {report}"
            );
        }
        let emitted_without_oracle = parse_fields(r#"{"placement_domain":[1,2,4,5,6]}"#);
        assert!(check_optional_placement_diagnostics(&emitted_without_oracle, None).is_err());
    }
}

#[cfg(test)]
mod coverage_adapter_tests {
    #[test]
    fn v19_identity_normalization_round_trips() {
        let mut source = b"RCVGEO1\0".to_vec();
        source.extend_from_slice(&19_u32.to_le_bytes());
        for identity in [
            super::coverage::COVERAGE_PROFILE,
            super::coverage::SOURCE_PROFILE,
            super::coverage::DOMAIN_PROFILE,
            super::coverage::RESOURCE_PROFILE,
        ] {
            source.extend_from_slice(&u32::try_from(identity.len()).unwrap().to_le_bytes());
            source.extend_from_slice(identity.as_bytes());
        }
        let normalized = super::coverage::normalize_for_v43(&source);
        assert_eq!(
            u32::from_le_bytes(normalized[8..12].try_into().unwrap()),
            16
        );
        assert_eq!(super::coverage::denormalize_from_v43(normalized), source);
    }
}

const _: [&str; 5] = LIST_ORDER;
