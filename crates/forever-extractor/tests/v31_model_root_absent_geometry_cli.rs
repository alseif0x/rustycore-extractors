// SPDX-License-Identifier: GPL-3.0-or-later
//! Synthetic v31 available geometry with WDT and WMO-doodad ROOT omissions.
#![cfg(target_os = "linux")]

#[path = "v31_model_root_absent_geometry_cli/coverage.rs"]
mod coverage;
#[path = "v31_model_root_absent_geometry_cli/fixture.rs"]
mod fixture;

use coverage::Coverage;
use forever_casc::{
    collision::{plan::MapSelection, run},
    integrity,
    package::{Artifact, Component, Context, Limits, PartialEncryptedSection, Run},
};
use forever_extractor::plan::{
    MODEL_ROOT_ABSENT_GEOMETRY_VERIFY_PROFILE, MODEL_ROOT_ABSENT_GEOMETRY_VERIFY_SCOPE,
};
use forever_vmap::{assembly, placement, vmo};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};

const PLAN: &str = include_str!("../../../tests/contracts/fixtures/forever-product-v31/all.tsv");
const MAP_PATH: &str = "dbc/esES/Map.db2";
static NEXT: AtomicU64 = AtomicU64::new(0);

struct Lab(PathBuf);

impl Lab {
    fn new() -> Self {
        let root = std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join(format!(
                ".local/share/rce-v31-model-root-absent-geometry-cli-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
        fs::create_dir_all(root.parent().unwrap()).unwrap();
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        fs::write(root.join("plan.tsv"), PLAN).unwrap();
        Self(root)
    }

    fn plan(&self) -> PathBuf {
        self.0.join("plan.tsv")
    }

    fn private_dir(&self, name: &str) -> PathBuf {
        let path = self.0.join(name);
        fs::create_dir(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        path
    }

    fn client(&self, name: &str) -> (PathBuf, fixture::WrittenClient) {
        self.client_with(name, fixture::ClientMutation::default())
    }

    fn client_with(
        &self,
        name: &str,
        mutation: fixture::ClientMutation,
    ) -> (PathBuf, fixture::WrittenClient) {
        let path = self.private_dir(name);
        let written = fixture::write_client_with(&path, mutation);
        (path, written)
    }

    fn command(&self, verb: &str) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_forever-extractor"));
        command
            .current_dir(&self.0)
            .env_remove("FOREVER_PRODUCT_DIAGNOSTIC")
            .args([verb, "--plan"])
            .arg(self.plan());
        command
    }

    fn produce(&self, verb: &str, client: &Path, output: &Path, scratch: &Path) -> Output {
        self.command(verb)
            .args(["--client"])
            .arg(client)
            .args(["--output"])
            .arg(output)
            .args(["--scratch"])
            .arg(scratch)
            .output()
            .unwrap()
    }

    fn extract(&self, stem: &str) -> PathBuf {
        let (client, _) = self.client(&format!("{stem}-client"));
        let output = self.0.join(format!("{stem}-package"));
        let scratch = self.private_dir(&format!("{stem}-scratch"));
        assert_success(
            self.produce("extract", &client, &output, &scratch),
            "v31 extract",
        );
        output
    }

    fn resume(&self, stem: &str, output: &Path) {
        let (client, _) = self.client(&format!("{stem}-client"));
        let scratch = self.private_dir(&format!("{stem}-scratch"));
        assert_success(
            self.produce("resume", &client, output, &scratch),
            "v31 resume",
        );
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
    String::from_utf8(output.stdout).unwrap()
}

fn assert_failure(output: Output, code: &str) {
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains(&format!("\"code\":\"{code}\"")), "{stderr}");
}

fn assert_failure_one_of(output: Output, codes: &[&str]) {
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        codes
            .iter()
            .any(|code| stderr.contains(&format!("\"code\":\"{code}\""))),
        "expected one of {codes:?}: {stderr}"
    );
}

fn client_context(package: &Path) -> Context {
    Context::parse(&fs::read_to_string(package.join(".state/context.json")).unwrap()).unwrap()
}

fn package_artifacts(package: &Path, context: Context) -> Vec<Artifact> {
    fs::read_dir(package.join(".state/files"))
        .unwrap()
        .map(|entry| {
            let text = fs::read_to_string(entry.unwrap().path()).unwrap();
            Artifact::parse(&text, context, Limits::default()).unwrap()
        })
        .collect()
}

fn artifact_inventory(package: &Path) -> Vec<String> {
    let context = client_context(package);
    let mut artifacts: Vec<_> = package_artifacts(package, context)
        .into_iter()
        .map(|artifact| artifact.json())
        .collect();
    artifacts.sort_unstable();
    artifacts
}

fn manifest(package: &Path) -> String {
    fs::read_to_string(package.join("forever-package.json")).unwrap()
}

fn declaration() -> PartialEncryptedSection {
    fixture::partial_map().declaration
}

fn assert_canonical_partial_manifest(text: &str) {
    let declaration = declaration();
    let table = format!(
        "\"partial_encrypted_tables\":[{{\"path\":\"{MAP_PATH}\",\"sections\":[{{\"lookup\":\"{}\",\"frame_ordinal\":1,\"decoded_size\":{}}}]}}]",
        declaration.lookup, declaration.decoded_size
    );
    assert!(text.contains(&table), "{text}");
    assert!(!text.contains("\"partial_encrypted_sections\":"));
    assert_eq!(
        text.matches("\"path\":\"dbc/esES/Map.db2\",\"sections\":")
            .count(),
        1
    );
    assert_eq!(
        text.matches(&format!("\"lookup\":\"{}\"", declaration.lookup))
            .count(),
        1
    );
}

fn verify_package(lab: &Lab, package: &Path) {
    for verb in ["verify", "verify-links"] {
        let report = assert_success(
            lab.command(verb)
                .args(["--output"])
                .arg(package)
                .output()
                .unwrap(),
            verb,
        );
        for (field, value) in [
            (
                "verification_profile",
                MODEL_ROOT_ABSENT_GEOMETRY_VERIFY_PROFILE,
            ),
            ("scope", MODEL_ROOT_ABSENT_GEOMETRY_VERIFY_SCOPE),
            ("coverage_authentication", "independently_derived"),
            ("root_absence_authentication", "producer_reported"),
            ("model_absence_authentication", "producer_reported"),
            ("source_authentication_flags", "producer_reported"),
            ("partial_db2_declarations", "per-table-including-Map"),
        ] {
            let identity = format!("\"{field}\":\"{value}\"");
            assert!(
                report.contains(&identity),
                "{verb} omitted {identity}: {report}"
            );
        }
        for expected in [
            "\"components\":17",
            "\"complete_global_map_coverage_claim\":false",
            "\"complete_vmap_component\":false",
            "\"full_extractor_complete\":false",
        ] {
            assert!(report.contains(expected), "{verb}: {report}");
        }
        assert!(!report.contains("requested_map"), "{verb}: {report}");
        assert!(!report.contains("\"full_world_coverage\":true"));
        if verb == "verify" {
            assert!(report.contains("\"client_reads\":false"), "{report}");
        }
    }
}

fn compare_with_baseline(lab: &Lab, baseline: &Path, resumed: &Path) {
    assert_eq!(
        fs::read(resumed.join(MAP_PATH)).unwrap(),
        fs::read(baseline.join(MAP_PATH)).unwrap()
    );
    assert_eq!(client_context(resumed), client_context(baseline));
    assert_eq!(
        fs::read(resumed.join(coverage::PATH)).unwrap(),
        fs::read(baseline.join(coverage::PATH)).unwrap()
    );
    assert_eq!(manifest(resumed), manifest(baseline));
    assert_eq!(artifact_inventory(resumed), artifact_inventory(baseline));
    assert_canonical_partial_manifest(&manifest(resumed));
    assert!(!resumed.join(".forever-package-incomplete").exists());
    assert!(resumed.join(".state/sealed/db2").is_file());
    assert!(resumed.join(".state/sealed/vmaps").is_file());
    verify_package(lab, resumed);
}

fn present_artifacts(
    package: &Path,
    id: u32,
    expected_records: usize,
) -> (BTreeSet<String>, BTreeSet<String>) {
    let missing_name = fixture::model_name(fixture::MISSING_M2);
    let proof_path = format!("vmaps/coverage/{id:04}.placements");
    let tree_path = format!("vmaps/{id:04}/{id:04}.vmtree");
    let proof = fs::read(package.join(&proof_path)).unwrap();
    let tree = fs::read(package.join(&tree_path)).unwrap();
    let placements = placement::DirBin::parse(&proof, Default::default()).unwrap();
    assert_eq!(placements.record_count(), expected_records);
    let records: Vec<_> = placements.records().collect();
    let mut model_names = BTreeSet::new();
    for record in &records {
        assert_ne!(record.id(), 0);
        let name = std::str::from_utf8(record.name()).unwrap().to_owned();
        assert_ne!(name, missing_name);
        model_names.insert(name);
    }
    let assembled = assembly::build(id, &records, Default::default()).unwrap();
    assert_eq!(proof, assembled.ordered_dir_bin());
    assert_eq!(tree, assembled.tree_bytes());
    let mut paths = BTreeSet::from([proof_path, tree_path]);
    if expected_records == 0 {
        assert!(assembled.tiles().is_empty());
    } else {
        assert!(!assembled.tiles().is_empty());
        assert!(
            assembled
                .tiles()
                .iter()
                .all(|tile| tile.payload().is_some())
        );
    }
    for tile in assembled.tiles() {
        paths.insert(format!(
            "vmaps/{id:04}/{id:04}_{:02}_{:02}.vmtileidx",
            tile.x(),
            tile.y()
        ));
        if tile.payload().is_some() {
            paths.insert(format!(
                "vmaps/{id:04}/{id:04}_{:02}_{:02}.vmtile",
                tile.x(),
                tile.y()
            ));
        }
    }
    (paths, model_names)
}

fn assert_coverage_and_inventory(package: &Path) {
    fixture::assert_sealed_map(&fs::read(package.join(MAP_PATH)).unwrap());
    let map = fixture::partial_map();
    assert_canonical_partial_manifest(&manifest(package));
    let missing_name = fixture::model_name(fixture::MISSING_M2);
    assert!(!package.join(format!("vmaps/{missing_name}.vmo")).exists());
    for excluded in [
        "maps",
        "mmaps",
        "vmaps/GameObjectModels.dtree",
        "vmaps/selected-geometry-coverage-v1.bin",
        "vmaps/available-geometry-coverage-v2.bin",
        "vmaps/available-root-absent-geometry-coverage-v3.bin",
        "dbc/esES/LiquidObject.db2",
        "dbc/esES/GameObjectDisplayInfo.db2",
    ] {
        assert!(!package.join(excluded).exists(), "unexpected {excluded}");
    }
    for id in [
        fixture::WDT_ZERO_CHILD,
        fixture::ROOT_ABSENT_PARENT,
        fixture::ROOT_ABSENT_OTHER,
        fixture::INACTIVE_MAP,
        fixture::COPIED_INACTIVE_MAP,
        fixture::HIDDEN_MAP,
    ] {
        assert!(!package.join(format!("vmaps/{id:04}")).exists());
        assert!(
            !package
                .join(format!("vmaps/coverage/{id:04}.placements"))
                .exists()
        );
    }

    let context = client_context(package);
    assert_eq!(context.build, 70450);
    assert_eq!(context.locale.name(), "esES");
    assert_eq!(context.components, 17);
    let artifacts = package_artifacts(package, context);
    assert!(artifacts.len() <= 65_536);
    assert!(
        artifacts
            .iter()
            .any(|artifact| { artifact.component == Component::Db2 && artifact.path == MAP_PATH })
    );
    assert!(artifacts.iter().all(|artifact| {
        (artifact.component == Component::Db2 && artifact.path == MAP_PATH)
            || (artifact.component == Component::Collision && artifact.path.starts_with("vmaps/"))
    }));

    let coverage_bytes = fs::read(package.join(coverage::PATH)).unwrap();
    let coverage = Coverage::parse(&coverage_bytes).unwrap();
    assert_eq!(coverage.encode(), coverage_bytes);
    assert_eq!(coverage.version, 4);
    assert_eq!(coverage.selection, MapSelection::All);
    assert_eq!(
        coverage.missing_wdt_policy,
        run::MissingWdtPolicy::DeclareRootAbsent
    );
    assert_eq!(
        coverage.missing_m2_policy,
        run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2
    );
    assert_eq!(coverage.missing_m2, [fixture::MISSING_M2]);
    assert_eq!(coverage.coverage_profile, coverage::COVERAGE_PROFILE);
    assert_eq!(coverage.source_profile, coverage::SOURCE_PROFILE);
    assert_eq!(coverage.domain_profile, coverage::DOMAIN_PROFILE);
    assert_eq!(coverage.context, context);
    assert!(!coverage.complete_map_source);
    assert!(!coverage.complete_global_map_coverage);
    assert!(!coverage.gameobject_request_made);
    assert_eq!(coverage.gameobject_startup_reads, 0);
    assert!(!coverage.liquid_object_request_made);
    assert_eq!(
        coverage.map.file_data_id,
        forever_casc::collision::world::MAP_FILE_DATA_ID
    );
    assert_eq!(coverage.map.declared_bytes, map.bytes.len() as u64);
    assert_eq!(coverage.map.available_bytes, map.hidden_start as u64);
    assert!(!coverage.map.full_ckey_verified);
    assert!(coverage.map.encoded_key_authenticated);
    assert_eq!(coverage.map.declared_direct_rows, 8);
    assert_eq!(coverage.map.declared_copy_records, 1);
    assert_eq!(coverage.map.direct_rows, 7);
    assert_eq!(coverage.map.copy_records, 1);
    assert_eq!(coverage.map.materialized_rows, fixture::MATERIALIZED_ROWS);
    assert_eq!(coverage.map.sections.len(), 2);
    assert_eq!(coverage.map.sections[0].ordinal, 0);
    assert_eq!(coverage.map.sections[0].records, 7);
    assert_eq!(coverage.map.sections[0].copies, 1);
    assert!(coverage.map.sections[0].admitted);
    assert_eq!(coverage.map.sections[1].ordinal, 1);
    assert_eq!(coverage.map.sections[1].records, 1);
    assert_eq!(coverage.map.sections[1].copies, 0);
    assert!(!coverage.map.sections[1].admitted);

    assert_eq!(coverage.liquids.len(), 2);
    for liquid in &coverage.liquids {
        assert_eq!(liquid.direct_rows, 1);
        assert_eq!(liquid.copy_records, 0);
        assert_eq!(liquid.materialized_rows, 1);
    }
    assert_eq!(coverage.maps.len(), fixture::ACTIVE_MAPS);
    assert_eq!(
        coverage.maps.iter().map(|map| map.id).collect::<Vec<_>>(),
        [
            fixture::EMPTY_MAP,
            fixture::POPULATED_MAP,
            fixture::ROOT_ABSENT_PARENT,
            fixture::PRESENT_CHILD,
            fixture::ROOT_ABSENT_OTHER,
        ]
    );
    let present = coverage.maps.iter().filter(|map| !map.root_absent).count();
    let absent = coverage.maps.iter().filter(|map| map.root_absent).count();
    assert_eq!(present, fixture::PRESENT_WDT_FILES);
    assert_eq!(absent, fixture::ROOT_ABSENT_MAPS);
    assert_eq!(coverage.maps.len(), present + absent);

    let expected = [
        (fixture::EMPTY_MAP, None, fixture::EMPTY_WDT, 0, true, false),
        (
            fixture::POPULATED_MAP,
            Some(fixture::EMPTY_MAP),
            fixture::POPULATED_WDT,
            1,
            true,
            false,
        ),
        (
            fixture::ROOT_ABSENT_PARENT,
            Some(fixture::POPULATED_MAP),
            fixture::ROOT_ABSENT_PARENT_WDT,
            2,
            true,
            true,
        ),
        (
            fixture::PRESENT_CHILD,
            Some(fixture::ROOT_ABSENT_PARENT),
            fixture::CHILD_WDT,
            3,
            false,
            false,
        ),
        (
            fixture::ROOT_ABSENT_OTHER,
            None,
            fixture::ROOT_ABSENT_OTHER_WDT,
            0,
            false,
            true,
        ),
    ];
    for (fact, (id, parent, wdt, depth, is_parent, root_absent)) in
        coverage.maps.iter().zip(expected)
    {
        assert_eq!(fact.id, id);
        assert_eq!(fact.parent, parent);
        assert_eq!(fact.wdt_file_data_id, wdt);
        assert_eq!(fact.depth, depth);
        assert_eq!(fact.is_parent, is_parent);
        assert_eq!(fact.root_absent, root_absent);
    }

    let (empty_paths, empty_models) = present_artifacts(package, fixture::EMPTY_MAP, 0);
    assert!(empty_models.is_empty());
    assert!(!package.join("vmaps/0001/0001_00_00.vmtile").exists());
    assert!(!package.join("vmaps/0001/0001_00_00.vmtileidx").exists());
    let (populated_paths, populated_models) = present_artifacts(package, fixture::POPULATED_MAP, 2);
    let (child_paths, child_models) = present_artifacts(package, fixture::PRESENT_CHILD, 2);
    let expected_models = BTreeSet::from([
        fixture::model_name(fixture::WMO_ROOT),
        fixture::model_name(fixture::MODEL),
    ]);
    assert_eq!(populated_models, expected_models);
    assert_eq!(child_models, expected_models);
    let model_names: BTreeSet<_> = populated_models.union(&child_models).collect();
    assert_eq!(model_names, expected_models.iter().collect::<BTreeSet<_>>());
    let mut expected_paths = BTreeSet::new();
    expected_paths.extend(empty_paths);
    expected_paths.extend(populated_paths);
    expected_paths.extend(child_paths);
    for name in model_names {
        expected_paths.insert(format!("vmaps/{name}.vmo"));
    }
    let actual_paths: BTreeSet<_> = coverage
        .artifacts
        .iter()
        .map(|artifact| artifact.path.clone())
        .collect();
    assert_eq!(actual_paths, expected_paths);
    assert_eq!(
        actual_paths
            .iter()
            .filter(|path| path.ends_with(".vmo"))
            .count(),
        2
    );
    assert_eq!(artifacts.len(), actual_paths.len() + 2);
    for fact in &coverage.artifacts {
        let bytes = fs::read(package.join(&fact.path)).unwrap();
        assert_eq!(fact.bytes, bytes.len() as u64);
        assert_eq!(fact.sha, integrity::digest(&bytes));
        if fact.path.ends_with(".vmo") {
            vmo::View::parse(&bytes, Default::default()).unwrap();
        }
    }
}

#[test]
fn v31_inspect_extract_verify_and_verify_links_preserve_wmo_and_present_doodad() {
    let lab = Lab::new();
    let inspection = assert_success(lab.command("inspect").output().unwrap(), "v31 inspect");
    for expected in [
        "\"contract\":\"forever-product-v31\"",
        "\"status\":\"planned\"",
        "\"build\":70450",
        "\"locale\":\"esES\"",
        "\"components\":17",
        "\"db2_files\":1",
        "\"collision_storage\":\"linux-leased-append-disk-world-v1\"",
        "\"collision_domain\":\"all-available-maps\"",
        "\"collision_source_profile\":\"linux70450-available-map-model-root-omissions-v1\"",
        "\"coverage_profile\":\"linux70450-available-map-model-root-omissions-coverage-v4\"",
        "\"coverage_authentication\":\"independently_derived\"",
        "\"root_absence_authentication\":\"producer_reported\"",
        "\"model_absence_authentication\":\"producer_reported\"",
        "\"source_authentication_flags\":\"producer_reported\"",
        "\"partial_db2_declarations\":\"per-table-including-Map\"",
        "\"liquid_object_scope\":\"out_of_scope\"",
        "\"complete_global_map_coverage_claim\":false",
        "\"complete_vmap_component\":false",
        "\"full_extractor_complete\":false",
    ] {
        assert!(inspection.contains(expected), "{inspection}");
    }
    assert!(!inspection.contains("requested_map"), "{inspection}");
    assert!(!inspection.contains("\"full_world_coverage\":true"));

    let package = lab.extract("lifecycle");
    assert_coverage_and_inventory(&package);
    verify_package(&lab, &package);
}

fn copy_tree(source: &Path, destination: &Path) {
    fs::create_dir(destination).unwrap();
    fs::set_permissions(destination, fs::Permissions::from_mode(0o700)).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let file_type = entry.file_type().unwrap();
        let target = destination.join(entry.file_name());
        if file_type.is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            assert!(file_type.is_file());
            fs::copy(entry.path(), &target).unwrap();
            fs::set_permissions(target, fs::Permissions::from_mode(0o600)).unwrap();
        }
    }
}

fn remove_artifact(package: &Path, path: &str) {
    let context = client_context(package);
    let receipt = fs::read_dir(package.join(".state/files"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|candidate| {
            let text = fs::read_to_string(candidate).unwrap();
            Artifact::parse(&text, context, Limits::default())
                .unwrap()
                .path
                == path
        })
        .unwrap();
    fs::remove_file(receipt).unwrap();
    fs::remove_file(package.join(path)).unwrap();
}

fn interrupted_clone(base: &Path, output: &Path, collision_sealed: bool) {
    copy_tree(base, output);
    fs::remove_file(output.join("forever-package.json")).unwrap();
    fs::write(output.join(".forever-package-incomplete"), b"incomplete\n").unwrap();
    if !collision_sealed {
        fs::remove_file(output.join(".state/sealed/vmaps")).unwrap();
        remove_artifact(output, coverage::PATH);
    }
}

#[test]
fn v31_resume_matches_baseline_at_db2_collision_and_final_seal_boundaries() {
    let lab = Lab::new();
    let baseline = lab.extract("resume-baseline");
    assert_coverage_and_inventory(&baseline);

    let (early_client, early_written) = lab.client("resume-early-source");
    fixture::corrupt_input(&early_client, &early_written, fixture::POPULATED_WDT);
    let early = lab.0.join("resume-early-package");
    let early_scratch = lab.private_dir("resume-early-initial-scratch");
    let interrupted = lab.produce("extract", &early_client, &early, &early_scratch);
    assert!(!interrupted.status.success());
    assert!(early.join(".forever-package-incomplete").is_file());
    assert!(early.join(".state/sealed/db2").is_file());
    assert!(!early.join(".state/sealed/vmaps").exists());
    assert!(!early.join("forever-package.json").exists());
    assert!(!early.join(coverage::PATH).exists());
    fixture::write_client(&early_client);
    let early_resume_scratch = lab.private_dir("resume-early-resume-scratch");
    assert_success(
        lab.produce("resume", &early_client, &early, &early_resume_scratch),
        "resume after DB2 seal",
    );
    compare_with_baseline(&lab, &baseline, &early);

    let sealed = lab.0.join("resume-collision-sealed-package");
    interrupted_clone(&baseline, &sealed, true);
    assert!(sealed.join(".state/sealed/db2").is_file());
    assert!(sealed.join(".state/sealed/vmaps").is_file());
    assert!(sealed.join(coverage::PATH).is_file());
    assert!(!sealed.join("forever-package.json").exists());
    lab.resume("resume-collision-sealed", &sealed);
    compare_with_baseline(&lab, &baseline, &sealed);

    let incomplete = lab.0.join("resume-collision-incomplete-package");
    interrupted_clone(&baseline, &incomplete, false);
    assert!(incomplete.join(".state/sealed/db2").is_file());
    assert!(!incomplete.join(".state/sealed/vmaps").exists());
    assert!(!incomplete.join(coverage::PATH).exists());
    for id in [
        fixture::EMPTY_MAP,
        fixture::POPULATED_MAP,
        fixture::PRESENT_CHILD,
    ] {
        assert!(
            incomplete
                .join(format!("vmaps/{id:04}/{id:04}.vmtree"))
                .is_file()
        );
    }
    lab.resume("resume-collision-incomplete", &incomplete);
    compare_with_baseline(&lab, &baseline, &incomplete);
}

fn inject_global_declaration(package: &Path, sections: &[PartialEncryptedSection]) {
    let path = package.join("forever-package.json");
    let mut text = fs::read_to_string(&path).unwrap();
    let declarations = sections
        .iter()
        .map(PartialEncryptedSection::json)
        .collect::<Vec<_>>()
        .join(",");
    let insertion = format!("\"partial_encrypted_sections\":[{declarations}],");
    let at = text.find("\"partial_encrypted_tables\"").unwrap();
    text.insert_str(at, &insertion);
    fs::write(path, text).unwrap();
}

fn inject_foreign_table(package: &Path, sections: &[PartialEncryptedSection]) {
    let path = package.join("forever-package.json");
    let mut text = fs::read_to_string(&path).unwrap();
    let declarations = sections
        .iter()
        .map(PartialEncryptedSection::json)
        .collect::<Vec<_>>()
        .join(",");
    let insertion = format!("{{\"path\":\"dbc/esES/Foreign.db2\",\"sections\":[{declarations}]}},");
    let key = "\"partial_encrypted_tables\":[";
    let at = text.find(key).unwrap() + key.len();
    text.insert_str(at, &insertion);
    fs::write(path, text).unwrap();
}

fn assert_resume_rejects_foreign_declaration(lab: &Lab, baseline: &Path, stem: &str, global: bool) {
    let output = lab.0.join(stem);
    copy_tree(baseline, &output);
    if global {
        inject_global_declaration(&output, &[declaration()]);
    } else {
        inject_foreign_table(&output, &[declaration()]);
    }
    fs::write(output.join(".forever-package-incomplete"), b"incomplete\n").unwrap();
    let (client, _) = lab.client(&format!("{stem}-client"));
    let scratch = lab.private_dir(&format!("{stem}-scratch"));
    assert_failure(
        lab.produce("resume", &client, &output, &scratch),
        "product_partial_manifest_scope",
    );
    assert!(output.join(".forever-package-incomplete").is_file());
}

#[test]
fn v31_resume_rejects_recovered_global_and_foreign_table_declarations() {
    let lab = Lab::new();
    let baseline = lab.extract("resume-foreign-baseline");
    assert_resume_rejects_foreign_declaration(&lab, &baseline, "resume-global", true);
    assert_resume_rejects_foreign_declaration(&lab, &baseline, "resume-table", false);
}

fn sync_coverage_artifacts(files: &BTreeMap<String, Vec<u8>>, coverage: &mut Coverage) {
    coverage.artifacts = files
        .iter()
        .filter(|(path, _)| path.as_str() != MAP_PATH && path.as_str() != coverage::PATH)
        .map(|(path, bytes)| coverage::ArtifactFact {
            path: path.clone(),
            bytes: bytes.len() as u64,
            sha: integrity::digest(bytes),
        })
        .collect();
}

fn rebuilt_package(
    base: &Path,
    output: &Path,
    global_sections: Option<Vec<PartialEncryptedSection>>,
    map_sections: Option<Vec<PartialEncryptedSection>>,
    mutate: impl FnOnce(&mut BTreeMap<String, Vec<u8>>, &mut Coverage),
) {
    let context = client_context(base);
    let mut files: BTreeMap<String, Vec<u8>> = package_artifacts(base, context)
        .into_iter()
        .map(|artifact| {
            let bytes = fs::read(base.join(&artifact.path)).unwrap();
            (artifact.path, bytes)
        })
        .collect();
    let mut coverage = Coverage::parse(&files[coverage::PATH]).unwrap();
    mutate(&mut files, &mut coverage);
    files.insert(coverage::PATH.into(), coverage.encode());
    let mut run = Run::create(output, context, &[base], Limits::default()).unwrap();
    if let Some(sections) = map_sections {
        run.set_partial_encrypted_table(MAP_PATH, &sections)
            .unwrap();
    }
    for (path, bytes) in files {
        let component = if path == MAP_PATH {
            Component::Db2
        } else {
            Component::Collision
        };
        run.put(component, &path, &bytes).unwrap();
    }
    run.finish_component(Component::Db2).unwrap();
    run.finish_component(Component::Collision).unwrap();
    run.finish().unwrap();
    if let Some(sections) = global_sections {
        inject_global_declaration(output, &sections);
    }
}

fn verify_failure(lab: &Lab, package: &Path, code: &str) {
    assert_failure(
        lab.command("verify")
            .args(["--output"])
            .arg(package)
            .output()
            .unwrap(),
        code,
    );
}

fn verify_links_failure(lab: &Lab, package: &Path, code: &str) {
    assert_failure(
        lab.command("verify-links")
            .args(["--output"])
            .arg(package)
            .output()
            .unwrap(),
        code,
    );
}

fn verify_failure_one_of(lab: &Lab, package: &Path, codes: &[&str]) {
    assert_failure_one_of(
        lab.command("verify")
            .args(["--output"])
            .arg(package)
            .output()
            .unwrap(),
        codes,
    );
}

#[test]
fn v31_resealed_manifest_codec_policy_and_context_negatives_reach_product_checks() {
    let lab = Lab::new();
    let baseline = lab.extract("negative-codec-baseline");
    let map = fixture::partial_map();
    let declaration = map.declaration.clone();

    let global = lab.0.join("negative-global-declaration");
    rebuilt_package(
        &baseline,
        &global,
        Some(vec![declaration.clone()]),
        Some(vec![declaration.clone()]),
        |_, _| {},
    );
    verify_failure(&lab, &global, "product_partial_manifest_scope");

    let undeclared = lab.0.join("negative-undeclared-map");
    rebuilt_package(&baseline, &undeclared, None, None, |_, _| {});
    verify_failure(&lab, &undeclared, "product_partial_undeclared");

    let mut wrong_size = declaration.clone();
    wrong_size.decoded_size -= 1;
    let inconsistent = lab.0.join("negative-wrong-decoded-size");
    rebuilt_package(
        &baseline,
        &inconsistent,
        None,
        Some(vec![wrong_size]),
        |_, _| {},
    );
    verify_failure(&lab, &inconsistent, "product_partial_section_count");

    let zero_fill = lab.0.join("negative-zero-fill");
    rebuilt_package(
        &baseline,
        &zero_fill,
        None,
        Some(vec![declaration.clone()]),
        |files, _| {
            let bytes = files.get_mut(MAP_PATH).unwrap();
            assert_eq!(bytes[map.hidden_start], 0);
            bytes[map.hidden_start] = 1;
        },
    );
    verify_failure(&lab, &zero_fill, "product_partial_zerofill");

    for (name, version) in [("v30", 3), ("v29", 2), ("v28", 1)] {
        let output = lab.0.join(format!("negative-cross-{name}"));
        rebuilt_package(
            &baseline,
            &output,
            None,
            Some(vec![declaration.clone()]),
            |_, coverage| {
                coverage.version = version;
                coverage.missing_m2_policy = run::MissingModelPolicy::Reject;
                match version {
                    3 => {
                        coverage.selection = MapSelection::All;
                        coverage.missing_wdt_policy = run::MissingWdtPolicy::DeclareRootAbsent;
                        coverage.coverage_profile = coverage::V30_COVERAGE_PROFILE.into();
                        coverage.source_profile = coverage::V30_SOURCE_PROFILE.into();
                        coverage.domain_profile = coverage::V30_DOMAIN_PROFILE.into();
                    }
                    2 => {
                        coverage.selection = MapSelection::All;
                        coverage.missing_wdt_policy = run::MissingWdtPolicy::Reject;
                        coverage.coverage_profile = coverage::V29_COVERAGE_PROFILE.into();
                        coverage.source_profile = coverage::V29_SOURCE_PROFILE.into();
                        coverage.domain_profile = coverage::V29_DOMAIN_PROFILE.into();
                    }
                    _ => {
                        coverage.selection = MapSelection::MapAndAncestors(fixture::POPULATED_MAP);
                        coverage.missing_wdt_policy = run::MissingWdtPolicy::Reject;
                        coverage.coverage_profile = coverage::SELECTED_COVERAGE_PROFILE.into();
                        coverage.source_profile = coverage::SELECTED_SOURCE_PROFILE.into();
                        coverage.domain_profile = coverage::SELECTED_DOMAIN_PROFILE.into();
                    }
                }
                coverage.missing_m2.clear();
            },
        );
        verify_failure(&lab, &output, "product_selected_geometry_coverage_schema");
        if version == 3 {
            verify_links_failure(&lab, &output, "product_selected_geometry_coverage_schema");
        }
    }

    for name in ["coverage-profile", "source-profile", "domain-profile"] {
        let output = lab.0.join(format!("negative-{name}"));
        rebuilt_package(
            &baseline,
            &output,
            None,
            Some(vec![declaration.clone()]),
            |_, coverage| match name {
                "coverage-profile" => {
                    coverage.coverage_profile = coverage::V30_COVERAGE_PROFILE.into();
                }
                "source-profile" => {
                    coverage.source_profile = coverage::V30_SOURCE_PROFILE.into();
                }
                "domain-profile" => {
                    coverage.domain_profile = coverage::V30_DOMAIN_PROFILE.into();
                }
                _ => unreachable!(),
            },
        );
        verify_failure(&lab, &output, "product_selected_geometry_coverage_schema");
    }

    for (name, wdt, model) in [
        (
            "wdt-policy",
            run::MissingWdtPolicy::Reject,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
        ),
        (
            "model-policy",
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::Reject,
        ),
    ] {
        let output = lab.0.join(format!("negative-{name}"));
        rebuilt_package(
            &baseline,
            &output,
            None,
            Some(vec![declaration.clone()]),
            |_, coverage| {
                coverage.missing_wdt_policy = wdt;
                coverage.missing_m2_policy = model;
            },
        );
        verify_failure(&lab, &output, "product_selected_geometry_coverage_schema");
    }

    let wrong_mask = lab.0.join("negative-context-mask");
    rebuilt_package(
        &baseline,
        &wrong_mask,
        None,
        Some(vec![declaration.clone()]),
        |_, coverage| coverage.context.components = 16,
    );
    verify_failure(
        &lab,
        &wrong_mask,
        "product_selected_geometry_coverage_schema",
    );

    let wrong_plan = lab.0.join("negative-context-plan");
    rebuilt_package(
        &baseline,
        &wrong_plan,
        None,
        Some(vec![declaration.clone()]),
        |_, coverage| coverage.context.plan_sha256 = [0x99; 32],
    );
    verify_failure(&lab, &wrong_plan, "product_plan_mismatch");

    let global_claim = lab.0.join("negative-global-coverage-claim");
    rebuilt_package(
        &baseline,
        &global_claim,
        None,
        Some(vec![declaration.clone()]),
        |_, coverage| coverage.complete_global_map_coverage = true,
    );
    verify_failure(
        &lab,
        &global_claim,
        "product_selected_geometry_coverage_schema",
    );
}

#[test]
fn v31_resealed_omission_artifact_and_parent_replay_negatives_are_rejected() {
    let lab = Lab::new();
    let baseline = lab.extract("negative-omission-baseline");
    let declaration = declaration();

    let false_omission = lab.0.join("negative-false-wdt-omission");
    rebuilt_package(
        &baseline,
        &false_omission,
        None,
        Some(vec![declaration.clone()]),
        |_, coverage| coverage.maps[0].root_absent = true,
    );
    verify_failure(&lab, &false_omission, "product_collision_association");

    let overlapping = lab.0.join("negative-overlapping-wdt-omission");
    rebuilt_package(
        &baseline,
        &overlapping,
        None,
        Some(vec![declaration.clone()]),
        |_, coverage| {
            let duplicate = coverage.maps[2].clone();
            coverage.maps.insert(3, duplicate);
        },
    );
    verify_failure(
        &lab,
        &overlapping,
        "product_selected_geometry_coverage_schema",
    );

    let wrong_fdid = lab.0.join("negative-wdt-omission-wrong-fdid");
    rebuilt_package(
        &baseline,
        &wrong_fdid,
        None,
        Some(vec![declaration.clone()]),
        |_, coverage| coverage.maps[2].wdt_file_data_id += 1,
    );
    verify_failure(&lab, &wrong_fdid, "product_collision_association");

    let missing_omission = lab.0.join("negative-required-wdt-omission-missing");
    rebuilt_package(
        &baseline,
        &missing_omission,
        None,
        Some(vec![declaration.clone()]),
        |_, coverage| coverage.maps[2].root_absent = false,
    );
    verify_failure(&lab, &missing_omission, "product_collision_association");

    for (name, omitted_path, source_path) in [
        ("tree", "vmaps/0004/0004.vmtree", "vmaps/0001/0001.vmtree"),
        (
            "proof",
            "vmaps/coverage/0004.placements",
            "vmaps/coverage/0001.placements",
        ),
        (
            "tile",
            "vmaps/0004/0004_00_00.vmtile",
            "vmaps/0002/0002_00_00.vmtile",
        ),
    ] {
        let output = lab.0.join(format!("negative-omitted-artifact-{name}"));
        let source = fs::read(baseline.join(source_path)).unwrap();
        rebuilt_package(
            &baseline,
            &output,
            None,
            Some(vec![declaration.clone()]),
            move |files, coverage| {
                files.insert(omitted_path.into(), source);
                sync_coverage_artifacts(files, coverage);
            },
        );
        verify_failure(&lab, &output, "product_collision_association");
    }

    let missing_empty_proof = lab.0.join("negative-present-empty-without-proof");
    rebuilt_package(
        &baseline,
        &missing_empty_proof,
        None,
        Some(vec![declaration.clone()]),
        |files, coverage| {
            files.remove("vmaps/coverage/0001.placements");
            sync_coverage_artifacts(files, coverage);
        },
    );
    verify_failure(&lab, &missing_empty_proof, "product_collision_association");
    verify_links_failure(&lab, &missing_empty_proof, "product_collision_association");

    for (name, ids) in [
        ("duplicate", vec![fixture::MISSING_M2, fixture::MISSING_M2]),
        ("zero", vec![0]),
        ("unsorted", vec![fixture::MISSING_M2, fixture::MODEL]),
        ("present-fdid", vec![fixture::MODEL]),
    ] {
        let output = lab.0.join(format!("negative-missing-m2-{name}"));
        rebuilt_package(
            &baseline,
            &output,
            None,
            Some(vec![declaration.clone()]),
            |_, coverage| coverage.missing_m2 = ids,
        );
        verify_failure_one_of(
            &lab,
            &output,
            &[
                "product_selected_geometry_coverage_schema",
                "product_collision_association",
                "product_selected_geometry_inventory",
                "product_selected_geometry_model_binding",
            ],
        );
    }

    let omitted_vmo = lab.0.join("negative-vmo-for-omitted-doodad");
    let omitted_name = fixture::model_name(fixture::MISSING_M2);
    let present_vmo =
        fs::read(baseline.join(format!("vmaps/{}.vmo", fixture::model_name(fixture::MODEL))))
            .unwrap();
    rebuilt_package(
        &baseline,
        &omitted_vmo,
        None,
        Some(vec![declaration.clone()]),
        move |files, coverage| {
            files.insert(format!("vmaps/{omitted_name}.vmo"), present_vmo);
            sync_coverage_artifacts(files, coverage);
        },
    );
    verify_failure_one_of(
        &lab,
        &omitted_vmo,
        &[
            "product_selected_geometry_coverage_schema",
            "product_collision_association",
            "product_selected_geometry_inventory",
        ],
    );

    let omitted_spawn = lab.0.join("negative-spawn-for-omitted-doodad");
    rebuilt_package(
        &baseline,
        &omitted_spawn,
        None,
        Some(vec![declaration.clone()]),
        |files, _| {
            let old_name = fixture::model_name(fixture::MODEL);
            let new_name = fixture::model_name(fixture::MISSING_M2);
            assert_eq!(old_name.len(), new_name.len());
            let proof = files.get_mut("vmaps/coverage/0002.placements").unwrap();
            let at = proof
                .windows(old_name.len())
                .position(|window| window == old_name.as_bytes())
                .unwrap();
            proof[at..at + new_name.len()].copy_from_slice(new_name.as_bytes());
            placement::DirBin::parse(proof, Default::default()).unwrap();
        },
    );
    verify_failure_one_of(
        &lab,
        &omitted_spawn,
        &[
            "product_selected_geometry_coverage_schema",
            "product_collision_association",
            "product_selected_geometry_model_binding",
            "product_selected_geometry_inventory",
        ],
    );

    let hidden_unknown = lab.0.join("negative-hidden-unknown-invented");
    rebuilt_package(
        &baseline,
        &hidden_unknown,
        None,
        Some(vec![declaration.clone()]),
        |_, coverage| {
            coverage.maps.push(coverage::DomainMapFact {
                id: fixture::HIDDEN_MAP,
                parent: None,
                wdt_file_data_id: 109,
                depth: 0,
                is_parent: false,
                root_absent: true,
            });
        },
    );
    verify_failure(&lab, &hidden_unknown, "product_collision_association");
}

fn make_parent_only(files: &mut BTreeMap<String, Vec<u8>>, coverage: &mut Coverage) {
    let proof_path = "vmaps/coverage/0005.placements";
    let mut proof = files["vmaps/coverage/0002.placements"].clone();
    let lengths: Vec<usize> = placement::DirBin::parse(&proof, Default::default())
        .unwrap()
        .records()
        .map(|record| (if record.flags() & 1 != 0 { 62 } else { 38 }) + record.name().len())
        .collect();
    let mut at = 0;
    for length in lengths {
        proof[at] |= 2;
        at += length;
    }
    assert_eq!(at, proof.len());
    let placements = placement::DirBin::parse(&proof, Default::default()).unwrap();
    let records: Vec<_> = placements.records().collect();
    let child = assembly::build(fixture::PRESENT_CHILD, &records, Default::default()).unwrap();
    assert!(child.stats().parent_only_tiles > 0);
    assert!(child.tiles().iter().all(|tile| tile.payload().is_none()));
    let grandparent = files["vmaps/0002/0002_00_00.vmtile"].as_slice();
    assembly::verify_inherited_associations(
        grandparent,
        child.tiles()[0].index_bytes(),
        child.ordered_dir_bin(),
        Default::default(),
    )
    .unwrap();
    files.insert(proof_path.into(), child.ordered_dir_bin().to_vec());
    files.insert("vmaps/0005/0005.vmtree".into(), child.tree_bytes().to_vec());
    for tile in child.tiles() {
        files.insert(
            format!("vmaps/0005/0005_{:02}_{:02}.vmtileidx", tile.x(), tile.y()),
            tile.index_bytes().to_vec(),
        );
        files.remove(&format!(
            "vmaps/0005/0005_{:02}_{:02}.vmtile",
            tile.x(),
            tile.y()
        ));
    }
    sync_coverage_artifacts(files, coverage);
}

#[test]
fn v31_parent_replay_accepts_present_parent_and_rejects_root_absent_parent() {
    let lab = Lab::new();
    let baseline = lab.extract("parent-replay-baseline");
    let declaration = declaration();

    let valid = lab.0.join("valid-present-parent-inheritance");
    rebuilt_package(
        &baseline,
        &valid,
        None,
        Some(vec![declaration.clone()]),
        |files, coverage| {
            fixture::set_parent(files.get_mut(MAP_PATH).unwrap(), 4, 2);
            coverage.maps[3].parent = Some(2);
            coverage.maps[3].depth = 2;
            coverage.maps[2].is_parent = false;
            make_parent_only(files, coverage);
        },
    );
    verify_package(&lab, &valid);

    let forbidden = lab.0.join("negative-parent-only-through-root-absent");
    rebuilt_package(
        &baseline,
        &forbidden,
        None,
        Some(vec![declaration.clone()]),
        make_parent_only,
    );
    verify_failure_one_of(
        &lab,
        &forbidden,
        &[
            "product_collision_association",
            "product_selected_geometry_parent_payload",
        ],
    );
}

#[test]
fn v31_direct_m2_and_other_source_failures_never_become_doodad_omissions() {
    let lab = Lab::new();
    for (stem, mutation) in [
        (
            "direct-m2-absent",
            fixture::ClientMutation {
                populated_adt_mode: fixture::AdtMode::DirectM2Absent,
                ..Default::default()
            },
        ),
        (
            "wmo-root-absent",
            fixture::ClientMutation {
                populated_adt_mode: fixture::AdtMode::MissingWmo,
                ..Default::default()
            },
        ),
        (
            "wmo-group-absent",
            fixture::ClientMutation {
                populated_adt_mode: fixture::AdtMode::MissingWmoGroup,
                ..Default::default()
            },
        ),
        (
            "logical-m2-name",
            fixture::ClientMutation {
                populated_adt_mode: fixture::AdtMode::LogicalM2Name,
                ..Default::default()
            },
        ),
        (
            "adt-absent",
            fixture::ClientMutation {
                omit_root: Some(fixture::POPULATED_ADT),
                ..Default::default()
            },
        ),
        (
            "ineligible-m2-root",
            fixture::ClientMutation {
                ineligible_root: Some(fixture::MODEL),
                ..Default::default()
            },
        ),
        (
            "m2-central-absent",
            fixture::ClientMutation {
                omit_central: Some(fixture::MODEL),
                ..Default::default()
            },
        ),
        (
            "m2-encoding-absent",
            fixture::ClientMutation {
                omit_encoding: Some(fixture::MODEL),
                ..Default::default()
            },
        ),
        (
            "malformed-m2",
            fixture::ClientMutation {
                malformed_model: Some(fixture::MODEL),
                ..Default::default()
            },
        ),
        (
            "ineligible-wdt-root",
            fixture::ClientMutation {
                ineligible_root: Some(fixture::POPULATED_WDT),
                ..Default::default()
            },
        ),
        (
            "malformed-wdt",
            fixture::ClientMutation {
                malformed_wdt: Some(fixture::POPULATED_WDT),
                ..Default::default()
            },
        ),
    ] {
        let (client, _) = lab.client_with(&format!("fatal-{stem}-client"), mutation);
        let output = lab.0.join(format!("fatal-{stem}-package"));
        let scratch = lab.private_dir(&format!("fatal-{stem}-scratch"));
        let result = lab.produce("extract", &client, &output, &scratch);
        assert!(!result.status.success(), "{stem} unexpectedly passed");
        assert!(!output.join(coverage::PATH).exists(), "{stem}");
        assert!(!output.join("forever-package.json").exists(), "{stem}");
        assert!(!output.join(".state/sealed/vmaps").exists(), "{stem}");
    }

    for (stem, id) in [
        ("corrupt-m2", fixture::MODEL),
        ("corrupt-wdt", fixture::POPULATED_WDT),
    ] {
        let (client, written) = lab.client(&format!("fatal-{stem}-client"));
        fixture::corrupt_input(&client, &written, id);
        let output = lab.0.join(format!("fatal-{stem}-package"));
        let scratch = lab.private_dir(&format!("fatal-{stem}-scratch"));
        let result = lab.produce("extract", &client, &output, &scratch);
        assert!(!result.status.success());
        assert!(!output.join(coverage::PATH).exists());
        assert!(!output.join("forever-package.json").exists());
        assert!(!output.join(".state/sealed/vmaps").exists());
    }
}

fn produce_diagnostic(
    lab: &Lab,
    client: &Path,
    output: &Path,
    scratch: &Path,
    diagnostic: &Path,
) -> Output {
    lab.command("extract")
        .args(["--client"])
        .arg(client)
        .args(["--output"])
        .arg(output)
        .args(["--scratch"])
        .arg(scratch)
        .env("FOREVER_PRODUCT_DIAGNOSTIC", diagnostic)
        .output()
        .unwrap()
}

fn assert_redacted_diagnostic(record: &str, client: &Path, package: &Path) {
    assert_eq!(record.lines().count(), 1, "{record}");
    for expected in [
        "\"phase\":\"models\"",
        "\"request_kind\":\"child\"",
        &format!("\"file_data_id\":{}", fixture::MODEL),
        "\"ordinal\":",
        "\"typed_cause\":{",
        "\"variant\":\"NoLocalEncoding\"",
        "\"corrupt\":1",
        "\"blte_code\":\"blte_checksum_mismatch\"",
        "\"code\":\"local_store_no_verified_encoding\"",
        "\"guards_observed\":",
        "\"tolerated_absences\":",
    ] {
        assert!(record.contains(expected), "missing {expected}: {record}");
    }
    for forbidden in [
        "\"path\"",
        "\"payload\"",
        "\"bytes\"",
        "content_key",
        "encoded_key",
        "ciphertext",
        "Data/data",
        "vmaps/",
        "BLTE",
        "MD20",
    ] {
        assert!(
            !record.contains(forbidden),
            "forbidden {forbidden}: {record}"
        );
    }
    for path in [client, package] {
        let path = path.to_string_lossy();
        assert!(!record.contains(path.as_ref()), "path leaked: {record}");
    }
}

#[test]
fn v31_diagnostic_is_opt_in_redacted_and_preserves_the_terminal_failure() {
    let lab = Lab::new();
    let (client, written) = lab.client("diagnostic-opt-in-client");
    fixture::corrupt_input(&client, &written, fixture::MODEL);
    let diagnostic = lab.0.join("diagnostic-opt-in.jsonl");

    let without_output = lab.0.join("diagnostic-without-package");
    let without_scratch = lab.private_dir("diagnostic-without-scratch");
    let without = lab.produce("extract", &client, &without_output, &without_scratch);
    let without_status = without.status.code();
    let without_stdout = without.stdout.clone();
    let without_stderr = without.stderr.clone();
    assert!(!diagnostic.exists());

    let with_output = lab.0.join("diagnostic-with-package");
    let with_scratch = lab.private_dir("diagnostic-with-scratch");
    let with = produce_diagnostic(&lab, &client, &with_output, &with_scratch, &diagnostic);
    assert_eq!(with.status.code(), without_status);
    assert_eq!(with.stdout, without_stdout);
    assert_eq!(with.stderr, without_stderr);
    assert!(String::from_utf8_lossy(&with.stderr).contains("local_store_no_verified_encoding"));
    let record = fs::read_to_string(&diagnostic).unwrap();
    assert_redacted_diagnostic(&record, &client, &with_output);
}
