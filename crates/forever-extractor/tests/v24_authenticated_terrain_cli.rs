// SPDX-License-Identifier: GPL-3.0-or-later
//! Synthetic v24 CLI admission checks only; no real client, keys or network.
#![cfg(target_os = "linux")]

#[path = "v23_selected_navigation_cli/fixture.rs"]
mod fixture;

use forever_casc::{
    integrity,
    package::{Artifact, Component, Context, Limits},
};
use forever_db2::{
    availability::{self, Source, Unavailable},
    catalog::{self, FieldType},
};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};

const MAP: &str = include_str!("../../../tests/contracts/fixtures/forever-product-v24/map2.tsv");
const ALL: &str = include_str!("../../../tests/contracts/fixtures/forever-product-v24/all.tsv");
static NEXT: AtomicU64 = AtomicU64::new(0);

struct Lab(PathBuf);
impl Lab {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "rce-v24-auth-terrain-cli-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let root = std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join(".local/share")
            .join(root.file_name().unwrap());
        fs::create_dir_all(root.parent().unwrap()).unwrap();
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        Self(root)
    }
    fn plan(&self, name: &str, contents: &str) -> PathBuf {
        let path = self.0.join(name);
        fs::write(&path, contents).unwrap();
        path
    }
    fn command(&self, verb: &str, plan: &Path) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_forever-extractor"));
        command
            .current_dir(&self.0)
            .args([verb, "--plan"])
            .arg(plan);
        command
    }
}
impl Drop for Lab {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn assert_code(output: Output, code: &str) {
    assert_eq!(
        output.status.code(),
        Some(2),
        "expected {code}, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains(&format!("\"code\":\"{code}\"")), "{stderr}");
}

fn assert_typed_failure(output: Output, status: i32, code: &str) {
    assert_eq!(
        output.status.code(),
        Some(status),
        "expected {code}, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains(&format!("\"code\":\"{code}\"")), "{stderr}");
}

fn assert_success(output: Output, label: &str) -> String {
    assert!(
        output.status.success(),
        "{label} failed (status {:?}): {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn field_width(kind: FieldType) -> usize {
    match kind {
        FieldType::String | FieldType::NonLocalizedString | FieldType::Int | FieldType::Float => 32,
        FieldType::Byte => 8,
        FieldType::Short => 16,
        FieldType::Long => 64,
    }
}

struct ByteSource<'a>(&'a [u8]);
impl Source for ByteSource<'_> {
    fn declared_bytes(&self) -> u64 {
        self.0.len() as u64
    }

    fn range(&self, offset: u64, bytes: usize) -> Result<&[u8], Unavailable> {
        let start = usize::try_from(offset).map_err(|_| Unavailable)?;
        self.0
            .get(start..start.checked_add(bytes).ok_or(Unavailable)?)
            .ok_or(Unavailable)
    }
}

fn map_wdt_offset(bytes: &[u8]) -> usize {
    map_field_offset(bytes, 21)
}

fn map_field_offset(bytes: &[u8], field_index: usize) -> usize {
    let metadata = catalog::resolve_for(catalog::WOWDBDEFS_70450_CATALOG_ID, "Map")
        .unwrap()
        .unwrap()
        .metadata();
    let fields = &metadata.fields[..metadata.file_field_count];
    let field = fields[field_index];
    assert_eq!(field_width(field.kind) * usize::from(field.array_size), 32);
    let field_bits: usize = fields[..field_index]
        .iter()
        .map(|field| field_width(field.kind) * usize::from(field.array_size))
        .sum();
    let record_start = u32::from_le_bytes(bytes[212..216].try_into().unwrap()) as usize;
    record_start + field_bits / 8
}

fn reseal_with_changed_map_wdt_id(package: &Path) {
    reseal_map_artifact(package, |map| {
        let offset = map_wdt_offset(map);
        assert_eq!(
            u32::from_le_bytes(map[offset..offset + 4].try_into().unwrap()),
            100
        );
        map[offset..offset + 4].copy_from_slice(&101_u32.to_le_bytes());
        let materialized = availability::read_map_70450(&ByteSource(map))
            .unwrap()
            .materialize()
            .unwrap();
        assert_eq!(materialized.rows().len(), 1);
        assert_eq!(materialized.rows()[0].id, 48);
        assert_eq!(materialized.rows()[0].fields.wdt_file_data_id, 101);
    });
}

fn swap_map_identity_rows_and_reseal(package: &Path) {
    reseal_map_artifact(package, |map| {
        let record_start = u32::from_le_bytes(map[212..216].try_into().unwrap()) as usize;
        let record_count = u32::from_le_bytes(map[136..140].try_into().unwrap()) as usize;
        let record_size = u32::from_le_bytes(map[144..148].try_into().unwrap()) as usize;
        assert_eq!(record_count, 2);
        let string_size = u32::from_le_bytes(map[220..224].try_into().unwrap()) as usize;
        let id_table = record_start + record_count * record_size + string_size;
        for byte in 0..4 {
            map.swap(id_table + byte, id_table + 4 + byte);
        }
        let wdt_offset = map_wdt_offset(map);
        for byte in 0..4 {
            map.swap(wdt_offset + byte, wdt_offset + record_size + byte);
        }
        let materialized = availability::read_map_70450(&ByteSource(map))
            .unwrap()
            .materialize()
            .unwrap();
        assert_eq!(
            materialized
                .rows()
                .iter()
                .map(|row| (row.id, row.fields.wdt_file_data_id))
                .collect::<Vec<_>>(),
            [(49, 101), (48, 100)]
        );
    });
}

fn reseal_map_artifact(package: &Path, mutate: impl FnOnce(&mut Vec<u8>)) {
    let map_path = "dbc/enUS/Map.db2";
    let map_file = package.join(map_path);
    let mut map = fs::read(&map_file).unwrap();
    mutate(&mut map);
    fs::write(&map_file, &map).unwrap();

    let context_text = fs::read_to_string(package.join(".state/context.json")).unwrap();
    let context = Context::parse(&context_text).unwrap();
    let receipt_dir = package.join(".state/files");
    let mut artifacts = fs::read_dir(&receipt_dir)
        .unwrap()
        .map(|entry| {
            let text = fs::read_to_string(entry.unwrap().path()).unwrap();
            Artifact::parse(&text, context, Limits::default()).unwrap()
        })
        .collect::<Vec<_>>();
    let map_artifact = artifacts
        .iter_mut()
        .find(|item| item.path == map_path)
        .unwrap();
    map_artifact.bytes = map.len() as u64;
    map_artifact.sha256 = integrity::digest(&map);
    artifacts.sort_unstable_by(|a, b| {
        a.path
            .to_ascii_lowercase()
            .cmp(&b.path.to_ascii_lowercase())
    });
    for artifact in &artifacts {
        let receipt = format!(
            ".state/files/{}.json",
            hex(&integrity::digest(
                artifact.path.to_ascii_lowercase().as_bytes(),
            ))
        );
        fs::write(package.join(receipt), artifact.json()).unwrap();
    }

    let component_seal = |component: Component| {
        let selected: Vec<_> = artifacts
            .iter()
            .filter(|artifact| artifact.component == component)
            .collect();
        let mut inventory = Vec::new();
        let mut bytes = 0_u64;
        for artifact in &selected {
            inventory.extend_from_slice(artifact.json().as_bytes());
            inventory.push(b'\n');
            bytes += artifact.bytes;
        }
        format!(
            "{{\"component\":\"{}\",\"context_sha256\":\"{}\",\"files\":{},\"bytes\":{},\"inventory_sha256\":\"{}\"}}",
            component.name(),
            hex(&integrity::digest(context.json().as_bytes())),
            selected.len(),
            bytes,
            hex(&integrity::digest(&inventory)),
        )
    };
    fs::write(
        package.join(format!(".state/sealed/{}", Component::Db2.name())),
        component_seal(Component::Db2),
    )
    .unwrap();

    let mut inventory = Vec::new();
    let mut total_bytes = 0_u64;
    let mut manifest = format!(
        "{{\"manifest_version\":1,\"scope\":\"artifact_integrity\",\"status\":\"complete\",\"context\":{},\"artifacts\":[\n",
        context.json()
    );
    for (index, artifact) in artifacts.iter().enumerate() {
        let json = artifact.json();
        inventory.extend_from_slice(json.as_bytes());
        inventory.push(b'\n');
        total_bytes += artifact.bytes;
        manifest.push_str(&json);
        manifest.push_str(if index + 1 == artifacts.len() {
            "\n"
        } else {
            ",\n"
        });
    }
    manifest.push_str(&format!(
        "],\"file_count\":{},\"total_bytes\":{},\"inventory_sha256\":\"{}\"}}\n",
        artifacts.len(),
        total_bytes,
        hex(&integrity::digest(&inventory)),
    ));
    fs::write(package.join("forever-package.json"), manifest).unwrap();
}

fn coverage_pairs(package: &Path) -> Vec<(u32, u32)> {
    let coverage =
        fs::read_to_string(package.join("maps/compatible-terrain-coverage-v3.tsv")).unwrap();
    assert!(coverage.starts_with("terrain-coverage\t3\n"));
    coverage
        .lines()
        .filter(|line| line.starts_with("map\t"))
        .map(|line| {
            let fields: Vec<_> = line.split('\t').collect();
            assert_eq!(fields.len(), 10, "unexpected coverage map row: {line}");
            (fields[3].parse().unwrap(), fields[4].parse().unwrap())
        })
        .collect()
}

#[test]
fn v24_plan_inspect_reports_mask_nine_and_narrow_claim_boundary() {
    for (index, source) in [MAP, ALL].into_iter().enumerate() {
        let lab = Lab::new();
        let plan = lab.plan("plan.tsv", source);
        let output = lab.command("inspect", &plan).output().unwrap();
        assert!(output.status.success());
        let report = String::from_utf8(output.stdout).unwrap();
        for expected in [
            "\"contract\":\"forever-product-v24\"",
            "\"status\":\"planned\"",
            "\"build\":70450",
            "\"components\":9",
            "\"schema_catalog\":\"wowdbdefs-70450-v1\"",
            "\"verification_coverage_authentication\":\"independently_derived\"",
            "\"db2_output\":[\"Map\"]",
            "\"whole_client_map_coverage\":false",
            "\"full_extractor_complete\":false",
        ] {
            assert!(
                report.contains(expected),
                "plan {index} missing {expected}: {report}"
            );
        }
        assert!(!report.contains("navigation_source_profile"));
    }
}

#[test]
fn v24_refuses_mask_twelve_and_key_or_scratch_inputs_before_path_access() {
    let lab = Lab::new();
    let wrong_mask = MAP.replace("components\tdb2,maps", "components\tcameras,maps");
    let wrong = lab.plan("wrong-mask.tsv", &wrong_mask);
    assert_code(
        lab.command("inspect", &wrong).output().unwrap(),
        "product_plan_component_scope",
    );

    let plan = lab.plan("plan.tsv", MAP);
    let output = lab.0.join("output-must-not-exist");
    let keys = lab.0.join("keys-must-not-be-read");
    assert_code(
        lab.command("extract", &plan)
            .args(["--client", "missing-client", "--output"])
            .arg(&output)
            .args(["--keys"])
            .arg(&keys)
            .output()
            .unwrap(),
        "product_keys_forbidden",
    );
    assert!(!output.exists());

    assert_code(
        lab.command("resume", &plan)
            .args(["--client", "missing-client", "--output"])
            .arg(&output)
            .args(["--scratch", "missing-scratch"])
            .output()
            .unwrap(),
        "product_scratch_forbidden",
    );
    assert!(!output.exists());
}

#[test]
fn v24_synthetic_inspect_extract_resume_verify_and_wdt_identity_tampering() {
    let lab = Lab::new();
    let plan_text = MAP.replace("terrain\tmap:2", "terrain\tmap:48");
    let plan = lab.plan("plan.tsv", &plan_text);

    let inspected = lab.command("inspect", &plan).output().unwrap();
    let report = assert_success(inspected, "inspect");
    assert!(report.contains("\"components\":9"));
    assert!(report.contains("\"verification_coverage_authentication\":\"independently_derived\""));

    let client = lab.0.join("synthetic-client");
    fs::create_dir(&client).unwrap();
    fixture::write_client(&client);
    let package = lab.0.join("package");
    let extracted = lab
        .command("extract", &plan)
        .args(["--client"])
        .arg(&client)
        .args(["--output"])
        .arg(&package)
        .output()
        .unwrap();
    let report = assert_success(extracted, "extract");
    assert!(report.contains("\"status\":\"selected_producers_complete\""));
    assert!(package.join("dbc/enUS/Map.db2").is_file());
    assert!(
        package
            .join("maps/compatible-terrain-coverage-v3.tsv")
            .is_file()
    );

    let verified = lab
        .command("verify", &plan)
        .args(["--output"])
        .arg(&package)
        .output()
        .unwrap();
    let report = assert_success(verified, "verify");
    assert!(report.contains("\"coverage_authentication\":\"independently_derived\""));
    let linked = lab
        .command("verify-links", &plan)
        .args(["--output"])
        .arg(&package)
        .output()
        .unwrap();
    assert_success(linked, "verify-links");

    let v20_plan = lab.plan(
        "v20-plan.tsv",
        include_str!("../../../tests/contracts/fixtures/forever-product-v20/map2.tsv"),
    );
    assert_typed_failure(
        lab.command("verify", &v20_plan)
            .args(["--output"])
            .arg(&package)
            .output()
            .unwrap(),
        9,
        "product_terrain_coverage",
    );

    reseal_with_changed_map_wdt_id(&package);
    assert_typed_failure(
        lab.command("verify", &plan)
            .args(["--output"])
            .arg(&package)
            .output()
            .unwrap(),
        9,
        "product_terrain_coverage_authentication",
    );

    // Create a provisional package by corrupting only the synthetic WDT's
    // archive bytes, then restore the deterministic fixture and resume it.
    let resume_client = lab.0.join("resume-client");
    fs::create_dir(&resume_client).unwrap();
    fixture::write_client(&resume_client);
    let archive_path = resume_client.join("Data/data/data.000");
    let mut archive = fs::read(&archive_path).unwrap();
    let mphd = archive
        .windows(4)
        .position(|window| window == b"DHPM")
        .expect("synthetic WDT MPHD chunk");
    archive[mphd + 8..mphd + 12].fill(0);
    fs::write(&archive_path, archive).unwrap();
    let resumed_package = lab.0.join("resumed-package");
    let interrupted = lab
        .command("extract", &plan)
        .args(["--client"])
        .arg(&resume_client)
        .args(["--output"])
        .arg(&resumed_package)
        .output()
        .unwrap();
    assert!(
        !interrupted.status.success(),
        "corrupt synthetic WDT unexpectedly extracted"
    );
    assert!(
        resumed_package
            .join(forever_casc::package::INCOMPLETE)
            .is_file()
    );

    fixture::write_client(&resume_client);
    let resumed = lab
        .command("resume", &plan)
        .args(["--client"])
        .arg(&resume_client)
        .args(["--output"])
        .arg(&resumed_package)
        .output()
        .unwrap();
    assert_success(resumed, "resume");
    let resumed_verify = lab
        .command("verify", &plan)
        .args(["--output"])
        .arg(&resumed_package)
        .output()
        .unwrap();
    assert_success(resumed_verify, "verify resumed package");
}

#[test]
fn v24_all_available_maps_authenticates_ordered_map_wdt_pairs() {
    let lab = Lab::new();
    let plan = lab.plan("all-plan.tsv", ALL);

    let inspected = lab.command("inspect", &plan).output().unwrap();
    let report = assert_success(inspected, "inspect all-available-maps");
    assert!(report.contains("\"components\":9"));
    assert!(report.contains("\"whole_client_map_coverage\":false"));

    let client = lab.0.join("synthetic-client");
    fs::create_dir(&client).unwrap();
    fixture::write_client_with_map_rows(&client, &[(48, 100), (49, 101)]);
    let package = lab.0.join("package");
    let extracted = lab
        .command("extract", &plan)
        .args(["--client"])
        .arg(&client)
        .args(["--output"])
        .arg(&package)
        .output()
        .unwrap();
    let report = assert_success(extracted, "extract all-available-maps");
    assert!(report.contains("\"status\":\"selected_producers_complete\""));
    assert!(package.join("dbc/enUS/Map.db2").is_file());

    let map_bytes = fs::read(package.join("dbc/enUS/Map.db2")).unwrap();
    let materialized = availability::read_map_70450(&ByteSource(&map_bytes))
        .unwrap()
        .materialize()
        .unwrap();
    let db2_pairs: Vec<_> = materialized
        .rows()
        .iter()
        .map(|row| (row.id, u32::try_from(row.fields.wdt_file_data_id).unwrap()))
        .collect();
    assert_eq!(db2_pairs, [(48, 100), (49, 101)]);
    assert_eq!(coverage_pairs(&package), db2_pairs);

    let verified = lab
        .command("verify", &plan)
        .args(["--output"])
        .arg(&package)
        .output()
        .unwrap();
    let report = assert_success(verified, "verify all-available-maps");
    assert!(report.contains("\"coverage_authentication\":\"independently_derived\""));

    swap_map_identity_rows_and_reseal(&package);
    assert_typed_failure(
        lab.command("verify", &plan)
            .args(["--output"])
            .arg(&package)
            .output()
            .unwrap(),
        9,
        "product_terrain_coverage_authentication",
    );
}
