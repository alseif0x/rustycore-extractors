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
    collision::plan::{MapSelection, Maps},
    package::{Component, Context, Limits, Run},
};
use forever_db2::availability;
use forever_vmap::{gameobject_models::Dtree, placement::DirBin, vmo};
use std::{
    collections::BTreeMap,
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};

const PLAN: &str = include_str!("../../../tests/contracts/fixtures/forever-product-v47/all.tsv");
const V46_PLAN: &str =
    include_str!("../../../tests/contracts/fixtures/forever-product-v46/all.tsv");
static NEXT: AtomicU64 = AtomicU64::new(0);

struct Lab(PathBuf);

impl Lab {
    fn new() -> Self {
        let root = std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join(format!(
                ".local/share/rce-v47-gameobject-expanded-material-filtered-wmo-cli-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
        fs::create_dir_all(root.parent().unwrap()).unwrap();
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        fs::write(root.join("plan-v47.tsv"), PLAN).unwrap();
        fs::write(root.join("plan-v46.tsv"), V46_PLAN).unwrap();
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
        self.command(verb, 47)
            .args(["--client"])
            .arg(client)
            .args(["--output"])
            .arg(output)
            .args(["--scratch"])
            .arg(scratch)
            .output()
            .unwrap()
    }

    fn extract(&self, name: &str, include_godi_only_model: bool) -> PathBuf {
        let client = self.private_dir(&format!("{name}-client"));
        fixture::write_client(&client, include_godi_only_model);
        let output = self.0.join(format!("{name}-package"));
        let scratch = self.private_dir(&format!("{name}-scratch"));
        assert_cli_json(
            self.produce("extract", &client, &output, &scratch),
            "extract",
            Some(&output),
        );
        output
    }

    fn verify(&self, verb: &str, version: u32, package: &Path) -> Output {
        self.command(verb, version)
            .args(["--output"])
            .arg(package)
            .output()
            .unwrap()
    }

    fn resume(&self, package: &Path, include_godi_only_model: bool) -> Output {
        let client = self.private_dir("resume-client");
        fixture::write_client(&client, include_godi_only_model);
        let scratch = self.private_dir("resume-scratch");
        self.produce("resume", &client, package, &scratch)
    }

    fn verify_failure_for_package(&self, version: u32, package: &Path) {
        for verb in ["verify", "verify-links"] {
            let output = self.verify(verb, version, package);
            assert!(
                !output.status.success(),
                "{verb} accepted invalid package {}",
                package.display()
            );
        }
    }

    fn rebuilt_with_coverage(&self, base: &Path, name: &str, bytes: &[u8]) -> PathBuf {
        let context =
            Context::parse(&fs::read_to_string(base.join(".state/context.json")).unwrap()).unwrap();
        let mut files = package_files(base);
        files.insert(coverage::PATH.to_owned(), bytes.to_vec());
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

fn assert_success(output: Output, verb: &str, version: u32) -> String {
    assert!(
        output.status.success(),
        "{verb} exited {:?}: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty(), "{verb} wrote stderr");
    let report = String::from_utf8(output.stdout).unwrap();
    assert!(
        report.contains(&format!("\"contract\":\"forever-product-v{version}\"")),
        "{report}"
    );
    report
}

struct MapBytes<'a>(&'a [u8]);

impl availability::Source for MapBytes<'_> {
    fn declared_bytes(&self) -> u64 {
        self.0.len() as u64
    }

    fn range(&self, offset: u64, size: usize) -> Result<&[u8], availability::Unavailable> {
        let start = usize::try_from(offset).map_err(|_| availability::Unavailable)?;
        self.0
            .get(start..start.checked_add(size).ok_or(availability::Unavailable)?)
            .ok_or(availability::Unavailable)
    }
}

#[derive(Debug, PartialEq, Eq)]
struct PlacementOracle {
    total: usize,
    per_map: BTreeMap<u32, usize>,
    bounded_per_map: BTreeMap<u32, usize>,
    placement_ids: Vec<u32>,
    bounded_ids: Vec<u32>,
    domain: Vec<u32>,
}

fn placement_oracle(package: &Path) -> PlacementOracle {
    let map = fs::read(package.join("dbc/esES/Map.db2")).unwrap();
    let available = availability::read_vmap_maps_70450(&MapBytes(&map)).unwrap();
    let direct_count = available.direct_records().len();
    let materialized = available.materialize().unwrap();
    let selection = Maps::from_materialized_selection(
        materialized.rows(),
        direct_count,
        MapSelection::All,
        Default::default(),
    )
    .unwrap();
    let mut domain: Vec<_> = selection.selected_ids().collect();
    domain.sort_unstable();
    let directory = package.join("vmaps/coverage");
    let mut per_map = BTreeMap::new();
    let mut bounded_per_map = BTreeMap::new();
    let mut placement_ids = Vec::new();
    let mut bounded_ids = Vec::new();
    for id in &domain {
        let path = directory.join(format!("{id:04}.placements"));
        let (count, bounded) = if path.exists() {
            let bytes = fs::read(path).unwrap();
            let parsed = DirBin::parse(&bytes, Default::default()).unwrap();
            let records: Vec<_> = parsed.records().collect();
            let bounded = records
                .iter()
                .filter(|placement| placement.bounds_bits().is_some())
                .count();
            placement_ids.extend(records.iter().map(|placement| placement.id()));
            bounded_ids.extend(
                records
                    .iter()
                    .filter(|placement| placement.bounds_bits().is_some())
                    .map(|placement| placement.id()),
            );
            (parsed.record_count(), bounded)
        } else {
            (0, 0)
        };
        assert!(per_map.insert(*id, count).is_none());
        bounded_per_map.insert(*id, bounded);
    }
    let total = per_map.values().sum();
    PlacementOracle {
        total,
        per_map,
        bounded_per_map,
        placement_ids,
        bounded_ids,
        domain,
    }
}

fn assert_cli_json(output: Output, verb: &str, package: Option<&Path>) -> String {
    let report = assert_success(output, verb, 47);
    assert_eq!(report.matches('\n').count(), 1, "{verb}: {report}");
    let parsed = json::parse(&report)
        .unwrap_or_else(|error| panic!("{verb}: invalid JSON ({error}): {report}"));
    let json::JsonValue::Object(fields) = parsed else {
        panic!("{verb}: CLI JSON root is not an object: {report}");
    };
    assert_v47_limits(&fields).unwrap_or_else(|error| panic!("{verb}: {error}: {report}"));
    let expected_placements = package.map(placement_oracle);
    assert_placement_json(&fields, expected_placements.as_ref())
        .unwrap_or_else(|error| panic!("{verb}: {error}: {report}"));
    report
}

fn assert_v47_limits(fields: &BTreeMap<String, json::JsonValue>) -> Result<(), String> {
    for (name, value) in V47_LIMITS {
        if fields.get(name) != Some(&json::JsonValue::Number(value.to_string())) {
            return Err(format!("{name} must be {value}"));
        }
    }
    Ok(())
}

const V47_LIMITS: [(&str, u64); 10] = [
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

fn assert_invalid_v47_limits(fields: &BTreeMap<String, json::JsonValue>) {
    assert!(assert_v47_limits(fields).is_err());
}

fn assert_cli_package(lab: &Lab, verb: &str, version: u32, package: &Path) -> String {
    if version == 47 {
        assert_cli_json(lab.verify(verb, version, package), verb, Some(package))
    } else {
        assert_success(lab.verify(verb, version, package), verb, version)
    }
}

#[test]
fn v47_json_limits_require_exact_numeric_values_and_types() {
    let valid: BTreeMap<_, _> = V47_LIMITS
        .iter()
        .map(|(name, value)| {
            (
                (*name).to_owned(),
                json::JsonValue::Number(value.to_string()),
            )
        })
        .collect();
    assert!(assert_v47_limits(&valid).is_ok());
    for (name, _) in V47_LIMITS {
        let mut too_large = BTreeMap::new();
        let mut wrong_type = BTreeMap::new();
        let mut missing = BTreeMap::new();
        for (other_name, other_value) in V47_LIMITS {
            if other_name == name {
                too_large.insert(
                    other_name.to_owned(),
                    json::JsonValue::Number(other_value.saturating_add(1).to_string()),
                );
                wrong_type.insert(
                    other_name.to_owned(),
                    json::JsonValue::String(other_value.to_string()),
                );
            } else {
                too_large.insert(
                    other_name.to_owned(),
                    json::JsonValue::Number(other_value.to_string()),
                );
                wrong_type.insert(
                    other_name.to_owned(),
                    json::JsonValue::Number(other_value.to_string()),
                );
                missing.insert(
                    other_name.to_owned(),
                    json::JsonValue::Number(other_value.to_string()),
                );
            }
        }
        assert_invalid_v47_limits(&too_large);
        assert_invalid_v47_limits(&wrong_type);
        assert_invalid_v47_limits(&missing);
    }
}

fn assert_placement_json(
    fields: &BTreeMap<String, json::JsonValue>,
    expected: Option<&PlacementOracle>,
) -> Result<(), String> {
    let names = [
        "total_placements",
        "per_map_placements",
        "bounded_per_map_placements",
        "placement_domain",
    ];
    if !names.iter().any(|name| fields.contains_key(*name)) {
        return Ok(());
    }
    let expected = expected.ok_or("placement diagnostics require independent oracle")?;
    if let Some(value) = fields.get("total_placements")
        && value != &json::JsonValue::Number(expected.total.to_string())
    {
        return Err("total_placements mismatch".to_owned());
    }
    if let Some(value) = fields.get("per_map_placements") {
        let expected_map = expected
            .per_map
            .iter()
            .map(|(id, count)| (id.to_string(), json::JsonValue::Number(count.to_string())))
            .collect();
        if value != &json::JsonValue::Object(expected_map) {
            return Err("per_map_placements mismatch".to_owned());
        }
    }
    if let Some(value) = fields.get("bounded_per_map_placements") {
        let expected_map = expected
            .bounded_per_map
            .iter()
            .map(|(id, count)| (id.to_string(), json::JsonValue::Number(count.to_string())))
            .collect();
        if value != &json::JsonValue::Object(expected_map) {
            return Err("bounded_per_map_placements mismatch".to_owned());
        }
    }
    if let Some(value) = fields.get("placement_domain") {
        let expected_domain = expected
            .domain
            .iter()
            .map(|id| json::JsonValue::Number(id.to_string()))
            .collect();
        if value != &json::JsonValue::Array(expected_domain) {
            return Err("placement_domain mismatch".to_owned());
        }
    }
    Ok(())
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
        .unwrap_or_else(|| panic!("missing v47 coverage text field {value}"));
    bytes[start] ^= 1;
}

#[test]
fn v47_godi_lifecycle_resume_determinism_and_cross_version_rejection() {
    let lab = Lab::new();
    assert!(PLAN.contains("db2\tGameObjectDisplayInfo\t1266277\n"));
    assert!(PLAN.contains("db2\tMap\t1349477\n"));

    let inspect = lab.command("inspect", 47).output().unwrap();
    let inspect = assert_cli_json(inspect, "inspect", None);
    assert!(inspect.contains(coverage::COVERAGE_PROFILE));
    assert!(inspect.contains("\"gameobject_scope\":\"available-fdid-direct-v1\""));
    assert!(inspect.contains("\"model_scope\":\"placement-and-gameobject-v1\""));

    let package = lab.extract("first", true);
    let package_bytes = package_files(&package);
    for verb in ["verify", "verify-links"] {
        assert_cli_package(&lab, verb, 47, &package);
    }
    let placements = placement_oracle(&package);
    assert_eq!(placements.total, 1, "placement oracle: {placements:?}");
    assert_eq!(placements.domain, [1]);
    assert_eq!(placements.per_map, [(1, 1)].into());
    assert_eq!(placements.placement_ids, [u32::MAX - 1]);
    assert_eq!(placements.bounded_ids, [u32::MAX - 1]);
    let placement_bytes = fs::read(package.join("vmaps/coverage/0001.placements")).unwrap();
    let parsed_placements = DirBin::parse(&placement_bytes, Default::default()).unwrap();
    assert_eq!(
        parsed_placements
            .records()
            .map(|placement| placement.name())
            .collect::<Vec<_>>(),
        [b"FILE0000012C.xxx".as_slice()]
    );
    let second = lab.extract("second", true);
    assert_eq!(package_bytes, package_files(&second));
    assert_eq!(placements, placement_oracle(&second));

    let tree_bytes = fs::read(package.join("vmaps/GameObjectModels.dtree")).unwrap();
    let tree = Dtree::parse(&tree_bytes, Default::default()).unwrap();
    let rows: Vec<_> = tree.rows().collect();
    assert_eq!(rows.len(), 4);
    assert_eq!(
        rows.iter().map(|row| row.display_id()).collect::<Vec<_>>(),
        [
            fixture::DISPLAY,
            fixture::DUPLICATE_DISPLAY,
            fixture::WMO_DISPLAY,
            fixture::GODI_ONLY_DISPLAY,
        ]
    );
    assert_eq!(
        rows.iter().map(|row| row.name()).collect::<Vec<_>>(),
        [
            b"FILE0000012C.xxx".as_slice(),
            b"FILE0000012C.xxx".as_slice(),
            b"FILE0000012D.xxx".as_slice(),
            b"FILE0000012E.xxx".as_slice(),
        ]
    );
    let bounds = [
        0.0_f32.to_bits(),
        0.0_f32.to_bits(),
        0.0_f32.to_bits(),
        1.0_f32.to_bits(),
        1.0_f32.to_bits(),
        0.0_f32.to_bits(),
    ];
    assert_eq!(
        rows.iter().map(|row| row.bounds_bits()).collect::<Vec<_>>(),
        [bounds, bounds, bounds, bounds]
    );
    let coverage_bytes = fs::read(package.join(coverage::PATH)).unwrap();
    let parsed = coverage::parse(&coverage_bytes).unwrap();
    assert_eq!(parsed.version, 20);
    assert_eq!(parsed.coverage_profile, coverage::COVERAGE_PROFILE);
    assert_eq!(parsed.source_profile, coverage::SOURCE_PROFILE);
    assert_eq!(parsed.domain_profile, coverage::DOMAIN_PROFILE);
    assert_eq!(parsed.resource_profile, coverage::RESOURCE_PROFILE);
    assert_eq!(parsed.gameobjects.declared_direct_rows, 9);
    assert_eq!(parsed.gameobjects.declared_copy_records, 1);
    assert_eq!(parsed.gameobjects.direct_rows, 9);
    assert_eq!(parsed.gameobjects.copy_records, 1);
    assert_eq!(parsed.gameobjects.direct_outcomes, 9);
    assert_eq!(parsed.gameobjects.zero_fdids, 2);
    assert_eq!(parsed.gameobjects.textures, 1);
    assert_eq!(parsed.gameobjects.m2_no_collision, 1);
    assert_eq!(parsed.gameobjects.wmo_doodad_only, 1);
    assert_eq!(parsed.gameobjects.collision_rows, 4);
    assert_eq!(parsed.gameobjects.list_rows, 4);
    assert_eq!(parsed.gameobjects.list_m2_rows, 3);
    assert_eq!(parsed.gameobjects.list_wmo_rows, 1);
    assert_eq!(parsed.gameobjects.sections.len(), 1);
    assert_eq!(
        parsed.gameobjects.sections[0],
        coverage::GameObjectSection {
            ordinal: 0,
            records: 9,
            copies: 1,
            admitted: true,
        }
    );
    assert_eq!(
        parsed
            .gameobjects
            .outcomes
            .iter()
            .map(|outcome| (
                outcome.ordinal,
                outcome.display_id,
                outcome.file_data_id,
                outcome.disposition
            ))
            .collect::<Vec<_>>(),
        [
            (0, fixture::ZERO_DISPLAY, 0, 0),
            (1, fixture::DISPLAY, fixture::MODEL, 2),
            (2, fixture::DUPLICATE_DISPLAY, fixture::MODEL, 2),
            (3, fixture::WMO_DISPLAY, fixture::WMO, 2),
            (4, fixture::GODI_ONLY_DISPLAY, fixture::GODI_ONLY_MODEL, 2),
            (5, fixture::TEXTURE_DISPLAY, fixture::TEXTURE, 1),
            (
                6,
                fixture::NO_COLLISION_DISPLAY,
                fixture::NO_COLLISION_MODEL,
                3
            ),
            (7, fixture::DOODAD_ONLY_DISPLAY, fixture::DOODAD_ONLY_WMO, 4),
            (8, fixture::COPY_SOURCE_DISPLAY, 0, 0),
        ]
    );
    assert!(parsed.omissions.iter().all(Vec::is_empty));
    assert_eq!(
        parsed.gameobjects.temp_bytes,
        8 + rows
            .iter()
            .map(|row| 8 + row.name().len() as u64)
            .sum::<u64>()
    );
    assert_eq!(parsed.gameobjects.dtree_bytes, tree_bytes.len() as u64);
    assert_eq!(
        parsed.gameobjects.temp_sha,
        coverage::reconstructed_temp_sha(&tree)
    );
    assert_eq!(
        parsed.gameobjects.dtree_sha,
        coverage::dtree_sha(&tree_bytes)
    );
    assert_eq!(
        parsed.gameobjects.dtree_bytes,
        package_bytes["vmaps/GameObjectModels.dtree"].len() as u64
    );
    assert_eq!(
        parsed.gameobjects.dtree_sha,
        coverage::dtree_sha(&package_bytes["vmaps/GameObjectModels.dtree"])
    );
    for table in ["GameObjectDisplayInfo", "Map"] {
        let path = format!("dbc/esES/{table}.db2");
        assert!(package_bytes.contains_key(&path));
        forever_db2::DecodedTable::validate_catalog_raw_source_bytes(
            &package_bytes[&path],
            forever_db2::catalog::WOWDBDEFS_70450_CATALOG_ID,
            table,
        )
        .unwrap();
    }
    let model_paths: Vec<_> = package_bytes
        .keys()
        .filter(|path| path.starts_with("vmaps/") && path.ends_with(".vmo"))
        .collect();
    assert_eq!(model_paths.len(), 3);
    assert!(package_bytes.contains_key("vmaps/FILE0000012C.xxx.vmo"));
    assert!(package_bytes.contains_key("vmaps/FILE0000012D.xxx.vmo"));
    assert!(package_bytes.contains_key("vmaps/FILE0000012E.xxx.vmo"));
    assert!(!package_bytes.contains_key("vmaps/FILE0000012F.xxx.vmo"));
    assert!(!package_bytes.contains_key("vmaps/FILE00000130.xxx.vmo"));
    assert!(!package_bytes.contains_key("vmaps/FILE00000131.xxx.vmo"));
    for path in &model_paths {
        vmo::View::parse(&package_bytes[*path], Default::default()).unwrap();
    }
    assert_eq!(
        rows.iter().map(|row| row.display_id()).collect::<Vec<_>>(),
        parsed
            .gameobjects
            .outcomes
            .iter()
            .filter(|outcome| outcome.disposition == 2)
            .map(|outcome| outcome.display_id)
            .collect::<Vec<_>>()
    );
    let resumed = lab.0.join("resumed-package");
    copy_tree(&package, &resumed);
    fs::remove_file(resumed.join("forever-package.json")).unwrap();
    fs::remove_file(resumed.join(".state/sealed/vmaps")).unwrap();
    remove_artifact(&resumed, coverage::PATH);
    remove_artifact(&resumed, "vmaps/GameObjectModels.dtree");
    remove_artifact(&resumed, "vmaps/FILE0000012C.xxx.vmo");
    remove_artifact(&resumed, "vmaps/FILE0000012D.xxx.vmo");
    remove_artifact(&resumed, "vmaps/FILE0000012E.xxx.vmo");
    fs::write(resumed.join(".forever-package-incomplete"), b"incomplete\n").unwrap();
    assert_cli_json(lab.resume(&resumed, true), "resume", Some(&resumed));
    assert_eq!(package_bytes, package_files(&resumed));
    assert_eq!(
        fs::read(resumed.join(coverage::PATH)).unwrap(),
        coverage_bytes
    );
    assert!(!resumed.join(".forever-package-incomplete").exists());
    for verb in ["verify", "verify-links"] {
        assert_cli_package(&lab, verb, 47, &resumed);
    }

    let missing_client = lab.private_dir("missing-model-client");
    fixture::write_client(&missing_client, false);
    let missing_output = lab.0.join("missing-model-package");
    let missing_scratch = lab.private_dir("missing-model-scratch");
    let missing = lab.produce(
        "extract",
        &missing_client,
        &missing_output,
        &missing_scratch,
    );
    assert!(
        !missing.status.success(),
        "v47 accepted GODI with absent VMO"
    );
    assert!(
        String::from_utf8_lossy(&missing.stderr).contains("file_unavailable"),
        "{}",
        String::from_utf8_lossy(&missing.stderr)
    );

    let v46_client = lab.private_dir("v46-client");
    fixture::write_v46_client(&v46_client);
    let v46_package = lab.0.join("v46-package");
    let v46_scratch = lab.private_dir("v46-scratch");
    let v46 = lab
        .command("extract", 46)
        .args(["--client"])
        .arg(&v46_client)
        .args(["--output"])
        .arg(&v46_package)
        .args(["--scratch"])
        .arg(&v46_scratch)
        .output()
        .unwrap();
    assert!(
        v46.status.success(),
        "{}",
        String::from_utf8_lossy(&v46.stderr)
    );
    let v46_files = package_files(&v46_package);
    assert!(!v46_files.contains_key("vmaps/GameObjectModels.dtree"));
    assert!(!v46_files.contains_key("vmaps/FILE0000012E.xxx.vmo"));
    for verb in ["verify", "verify-links"] {
        assert_success(lab.verify(verb, 46, &v46_package), verb, 46);
    }
    for (version, crossed) in [(47, &v46_package), (46, &package)] {
        for verb in ["verify", "verify-links"] {
            let rejected = lab.verify(verb, version, crossed);
            assert!(
                !rejected.status.success(),
                "{verb} accepted a crossed package"
            );
        }
    }

    let v47_coverage = fs::read(package.join(coverage::PATH)).unwrap();
    let old_version = {
        let mut bytes = v47_coverage.clone();
        bytes[8..12].copy_from_slice(&19_u32.to_le_bytes());
        bytes
    };
    let resealed = lab.rebuilt_with_coverage(&package, "resealed-v19", &old_version);
    lab.verify_failure_for_package(47, &resealed);
    for identity in [
        coverage::COVERAGE_PROFILE,
        coverage::SOURCE_PROFILE,
        coverage::DOMAIN_PROFILE,
        coverage::RESOURCE_PROFILE,
    ] {
        let mut changed = v47_coverage.clone();
        mutate_text_field(&mut changed, identity);
        let attacked = lab.rebuilt_with_coverage(
            &package,
            &format!("resealed-identity-{}", identity.len()),
            &changed,
        );
        lab.verify_failure_for_package(47, &attacked);
    }

    const OVERLAPS: [(usize, usize); 10] = [
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
    for (left, right) in OVERLAPS {
        let mut lists = coverage::parse(&v47_coverage).unwrap().omissions;
        lists[left] = vec![3_000_000];
        lists[right] = vec![3_000_000];
        let attacked_bytes = coverage::replace_omissions(&v47_coverage, &lists).unwrap();
        let attacked = lab.rebuilt_with_coverage(
            &package,
            &format!("overlap-{left}-{right}"),
            &attacked_bytes,
        );
        lab.verify_failure_for_package(47, &attacked);
    }
    let ids: Vec<_> = (3_000_000..3_004_097).collect();
    for (index, name) in coverage::LIST_ORDER.iter().enumerate() {
        let mut lists = coverage::parse(&v47_coverage).unwrap().omissions;
        lists[index] = ids[..4096].to_vec();
        let accepted_bytes = coverage::replace_omissions(&v47_coverage, &lists).unwrap();
        let accepted =
            lab.rebuilt_with_coverage(&package, &format!("{name}-4096"), &accepted_bytes);
        for verb in ["verify", "verify-links"] {
            assert_cli_package(&lab, verb, 47, &accepted);
        }

        lists[index] = ids[..4097].to_vec();
        let rejected_bytes = coverage::replace_omissions(&v47_coverage, &lists).unwrap();
        let rejected =
            lab.rebuilt_with_coverage(&package, &format!("{name}-4097"), &rejected_bytes);
        lab.verify_failure_for_package(47, &rejected);
    }

    for table in ["GameObjectDisplayInfo", "Map"] {
        let path = format!("dbc/esES/{table}.db2");
        assert!(package_bytes.contains_key(&path));
        assert!(
            forever_db2::DecodedTable::validate_catalog_raw_source_bytes(
                &package_bytes[&path],
                forever_db2::catalog::WOWDBDEFS_70450_CATALOG_ID,
                table,
            )
            .is_ok()
        );
    }
}
