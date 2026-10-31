// SPDX-License-Identifier: GPL-3.0-or-later
//! Synthetic v29 authenticated all-active available-geometry CLI lifecycle.
#![cfg(target_os = "linux")]

#[path = "v29_available_geometry_cli/coverage.rs"]
mod coverage;
#[path = "v29_available_geometry_cli/fixture.rs"]
mod fixture;

use coverage::Coverage;
use forever_casc::{
    collision::{plan::MapSelection, world},
    integrity,
    package::{Artifact, Component, Context, Limits, PartialEncryptedSection, Run},
};
use forever_extractor::plan::{
    AUTHENTICATED_AVAILABLE_GEOMETRY_VERIFY_PROFILE, AUTHENTICATED_AVAILABLE_GEOMETRY_VERIFY_SCOPE,
};
use forever_vmap::{assembly, placement, vmo};
use std::{
    collections::BTreeSet,
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};

const PLAN: &str = include_str!("../../../tests/contracts/fixtures/forever-product-v29/all.tsv");
const MAP_PATH: &str = "dbc/esES/Map.db2";
static NEXT: AtomicU64 = AtomicU64::new(0);

struct Lab(PathBuf);

impl Lab {
    fn new() -> Self {
        let root = std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join(format!(
                ".local/share/rce-v29-available-geometry-cli-{}-{}",
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
        let path = self.private_dir(name);
        let written = fixture::write_client(&path);
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
            "v29 extract",
        );
        output
    }

    fn resume(&self, stem: &str, output: &Path) {
        let (client, _) = self.client(&format!("{stem}-client"));
        let scratch = self.private_dir(&format!("{stem}-scratch"));
        assert_success(
            self.produce("resume", &client, output, &scratch),
            "v29 resume",
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
                AUTHENTICATED_AVAILABLE_GEOMETRY_VERIFY_PROFILE,
            ),
            ("scope", AUTHENTICATED_AVAILABLE_GEOMETRY_VERIFY_SCOPE),
            ("coverage_authentication", "independently_derived"),
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

fn assert_coverage_and_inventory(package: &Path) {
    fixture::assert_sealed_map(&fs::read(package.join(MAP_PATH)).unwrap());
    let map = fixture::partial_map();
    assert_canonical_partial_manifest(&manifest(package));
    for excluded in [
        "maps",
        "mmaps",
        "vmaps/GameObjectModels.dtree",
        "vmaps/selected-geometry-coverage-v1.bin",
        "dbc/esES/LiquidObject.db2",
        "dbc/esES/GameObjectDisplayInfo.db2",
    ] {
        assert!(!package.join(excluded).exists(), "unexpected {excluded}");
    }
    for id in [
        fixture::WDT_ZERO_CHILD,
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
    assert_eq!(coverage.version, 2);
    assert_eq!(coverage.selection, MapSelection::All);
    assert_eq!(coverage.coverage_profile, coverage::COVERAGE_PROFILE);
    assert_eq!(coverage.source_profile, coverage::SOURCE_PROFILE);
    assert_eq!(coverage.domain_profile, coverage::DOMAIN_PROFILE);
    assert_eq!(coverage.context, context);
    assert!(!coverage.complete_map_source);
    assert!(!coverage.complete_global_map_coverage);
    assert!(!coverage.gameobject_request_made);
    assert_eq!(coverage.gameobject_startup_reads, 0);
    assert!(!coverage.liquid_object_request_made);
    assert_eq!(coverage.map.file_data_id, world::MAP_FILE_DATA_ID);
    assert_eq!(coverage.map.declared_bytes, map.bytes.len() as u64);
    assert_eq!(coverage.map.available_bytes, map.hidden_start as u64);
    assert!(!coverage.map.full_ckey_verified);
    assert!(coverage.map.encoded_key_authenticated);
    assert_eq!(coverage.map.declared_direct_rows, 5);
    assert_eq!(coverage.map.declared_copy_records, 1);
    assert_eq!(coverage.map.direct_rows, 4);
    assert_eq!(coverage.map.copy_records, 1);
    assert_eq!(coverage.map.materialized_rows, 5);
    assert_eq!(coverage.map.sections.len(), 2);
    assert_eq!(coverage.map.sections[0].ordinal, 0);
    assert_eq!(coverage.map.sections[0].records, 4);
    assert_eq!(coverage.map.sections[0].copies, 1);
    assert!(coverage.map.sections[0].admitted);
    assert_eq!(coverage.map.sections[1].ordinal, 1);
    assert_eq!(coverage.map.sections[1].records, 1);
    assert_eq!(coverage.map.sections[1].copies, 0);
    assert!(!coverage.map.sections[1].admitted);

    assert_eq!(coverage.liquids.len(), 2);
    assert_eq!(
        coverage.liquids[0].file_data_id,
        world::MATERIAL_FILE_DATA_ID
    );
    assert_eq!(coverage.liquids[1].file_data_id, world::TYPE_FILE_DATA_ID);
    for liquid in &coverage.liquids {
        assert_eq!(liquid.direct_rows, 1);
        assert_eq!(liquid.copy_records, 0);
        assert_eq!(liquid.materialized_rows, 1);
    }
    assert_eq!(coverage.maps.len(), 2);
    assert_eq!(
        coverage.maps.iter().map(|map| map.id).collect::<Vec<_>>(),
        [fixture::EMPTY_MAP, fixture::POPULATED_MAP]
    );
    let empty = &coverage.maps[0];
    assert_eq!(empty.parent, None);
    assert_eq!(empty.wdt_file_data_id, fixture::EMPTY_WDT);
    assert_eq!(empty.depth, 0);
    assert!(empty.is_parent);
    let populated = &coverage.maps[1];
    assert_eq!(populated.parent, Some(fixture::EMPTY_MAP));
    assert_eq!(populated.wdt_file_data_id, fixture::POPULATED_WDT);
    assert_eq!(populated.depth, 1);
    assert!(populated.is_parent);

    let empty_proof_path = format!("vmaps/coverage/{:04}.placements", fixture::EMPTY_MAP);
    let empty_tree_path = format!("vmaps/{0:04}/{0:04}.vmtree", fixture::EMPTY_MAP);
    let empty_proof = fs::read(package.join(&empty_proof_path)).unwrap();
    let empty_tree = fs::read(package.join(&empty_tree_path)).unwrap();
    let empty_placements = placement::DirBin::parse(&empty_proof, Default::default()).unwrap();
    assert_eq!(empty_placements.record_count(), 0);
    let empty_assembled = assembly::build(fixture::EMPTY_MAP, &[], Default::default()).unwrap();
    assert!(empty_assembled.tiles().is_empty());
    assert_eq!(empty_proof, empty_assembled.ordered_dir_bin());
    assert_eq!(empty_tree, empty_assembled.tree_bytes());
    assert!(!package.join("vmaps/0001/0001_00_00.vmtile").exists());
    assert!(!package.join("vmaps/0001/0001_00_00.vmtileidx").exists());

    let populated_proof_path = format!("vmaps/coverage/{:04}.placements", fixture::POPULATED_MAP);
    let populated_tree_path = format!("vmaps/{0:04}/{0:04}.vmtree", fixture::POPULATED_MAP);
    let populated_proof = fs::read(package.join(&populated_proof_path)).unwrap();
    let populated_tree = fs::read(package.join(&populated_tree_path)).unwrap();
    let populated_placements =
        placement::DirBin::parse(&populated_proof, Default::default()).unwrap();
    assert_eq!(populated_placements.record_count(), 1);
    let records: Vec<_> = populated_placements.records().collect();
    assert_ne!(records[0].id(), 0);
    let model_name = std::str::from_utf8(records[0].name()).unwrap().to_owned();
    let populated_assembled =
        assembly::build(fixture::POPULATED_MAP, &records, Default::default()).unwrap();
    assert_eq!(populated_proof, populated_assembled.ordered_dir_bin());
    assert_eq!(populated_tree, populated_assembled.tree_bytes());
    assert!(!populated_assembled.tiles().is_empty());
    assert!(
        populated_assembled
            .tiles()
            .iter()
            .any(|tile| tile.payload().is_some())
    );

    let mut expected_paths = BTreeSet::from([
        empty_proof_path,
        empty_tree_path,
        populated_proof_path,
        populated_tree_path,
        format!("vmaps/{model_name}.vmo"),
    ]);
    for tile in populated_assembled.tiles() {
        expected_paths.insert(format!(
            "vmaps/{0:04}/{0:04}_{1:02}_{2:02}.vmtileidx",
            fixture::POPULATED_MAP,
            tile.x(),
            tile.y()
        ));
        if tile.payload().is_some() {
            expected_paths.insert(format!(
                "vmaps/{0:04}/{0:04}_{1:02}_{2:02}.vmtile",
                fixture::POPULATED_MAP,
                tile.x(),
                tile.y()
            ));
        }
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
        1
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
fn v29_inspect_extract_verify_and_verify_links_cover_all_active_available_geometry() {
    let lab = Lab::new();
    let inspection = assert_success(lab.command("inspect").output().unwrap(), "v29 inspect");
    for expected in [
        "\"contract\":\"forever-product-v29\"",
        "\"status\":\"planned\"",
        "\"build\":70450",
        "\"locale\":\"esES\"",
        "\"components\":17",
        "\"db2_files\":1",
        "\"collision_storage\":\"linux-leased-append-disk-world-v1\"",
        "\"collision_domain\":\"all-available-maps\"",
        "\"collision_source_profile\":\"linux70450-available-map-geometry-v1\"",
        "\"coverage_profile\":\"linux70450-available-map-geometry-coverage-v2\"",
        "\"coverage_authentication\":\"independently_derived\"",
        "\"source_authentication_flags\":\"producer_reported\"",
        "\"map_source_scope\":\"available_sections+vmap_materialization\"",
        "\"partial_db2_declarations\":\"per-table-including-Map\"",
        "\"liquid_object_scope\":\"out_of_scope\"",
        "\"complete_global_map_coverage_claim\":false",
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
fn v29_resume_matches_baseline_at_db2_collision_and_final_seal_boundaries() {
    let lab = Lab::new();
    let baseline = lab.extract("resume-baseline");
    assert_coverage_and_inventory(&baseline);

    let (early_client, early_written) = lab.client("resume-early-source");
    fixture::corrupt_wdt(&early_client, &early_written, fixture::POPULATED_WDT);
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
    assert!(incomplete.join("vmaps/0001/0001.vmtree").is_file());
    assert!(incomplete.join("vmaps/0002/0002.vmtree").is_file());
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
fn v29_resume_rejects_foreign_global_and_table_declarations() {
    let lab = Lab::new();
    let baseline = lab.extract("resume-foreign-baseline");
    assert_resume_rejects_foreign_declaration(&lab, &baseline, "resume-global", true);
    assert_resume_rejects_foreign_declaration(&lab, &baseline, "resume-table", false);
}

fn rebuilt_package(
    base: &Path,
    output: &Path,
    global_sections: Option<Vec<PartialEncryptedSection>>,
    map_sections: Option<Vec<PartialEncryptedSection>>,
    omitted: Option<&str>,
    mutate: impl FnOnce(&mut Vec<u8>, &mut Vec<u8>),
) {
    let context = client_context(base);
    let mut map = fs::read(base.join(MAP_PATH)).unwrap();
    let mut coverage = fs::read(base.join(coverage::PATH)).unwrap();
    mutate(&mut map, &mut coverage);
    let mut run = Run::create(output, context, &[base], Limits::default()).unwrap();
    if let Some(sections) = map_sections {
        run.set_partial_encrypted_table(MAP_PATH, &sections)
            .unwrap();
    }
    for artifact in package_artifacts(base, context) {
        if omitted == Some(artifact.path.as_str()) {
            continue;
        }
        let bytes = if artifact.path == MAP_PATH {
            map.clone()
        } else if artifact.path == coverage::PATH {
            coverage.clone()
        } else {
            fs::read(base.join(&artifact.path)).unwrap()
        };
        run.put(artifact.component, &artifact.path, &bytes).unwrap();
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

#[test]
fn v29_resealed_negatives_require_partial_manifests_empty_map_artifacts_and_exact_v2_context() {
    let lab = Lab::new();
    let baseline = lab.extract("negative-baseline");
    let map = fixture::partial_map();
    let declaration = map.declaration.clone();

    let global = lab.0.join("negative-global-declaration");
    rebuilt_package(
        &baseline,
        &global,
        Some(vec![declaration.clone()]),
        Some(vec![declaration.clone()]),
        None,
        |_, _| {},
    );
    verify_failure(&lab, &global, "product_partial_manifest_scope");

    let undeclared = lab.0.join("negative-undeclared-map");
    rebuilt_package(&baseline, &undeclared, None, None, None, |_, _| {});
    verify_failure(&lab, &undeclared, "product_partial_undeclared");

    let mut wrong_size = declaration.clone();
    wrong_size.decoded_size -= 1;
    let inconsistent = lab.0.join("negative-wrong-decoded-size");
    rebuilt_package(
        &baseline,
        &inconsistent,
        None,
        Some(vec![wrong_size]),
        None,
        |_, _| {},
    );
    verify_failure(&lab, &inconsistent, "product_partial_section_count");

    let zero_fill = lab.0.join("negative-zero-fill");
    rebuilt_package(
        &baseline,
        &zero_fill,
        None,
        Some(vec![declaration.clone()]),
        None,
        |bytes, _| {
            assert_eq!(bytes[map.hidden_start], 0);
            bytes[map.hidden_start] = 1;
        },
    );
    verify_failure(&lab, &zero_fill, "product_partial_zerofill");

    for (name, omitted) in [
        ("empty-tree", "vmaps/0001/0001.vmtree"),
        ("empty-proof", "vmaps/coverage/0001.placements"),
    ] {
        let output = lab.0.join(format!("negative-missing-{name}"));
        rebuilt_package(
            &baseline,
            &output,
            None,
            Some(vec![declaration.clone()]),
            Some(omitted),
            |_, _| {},
        );
        verify_failure(&lab, &output, "product_collision_association");
        if name == "empty-proof" {
            verify_links_failure(&lab, &output, "product_collision_association");
        }
    }

    let wrong_version = lab.0.join("negative-version");
    rebuilt_package(
        &baseline,
        &wrong_version,
        None,
        Some(vec![declaration.clone()]),
        None,
        |_, bytes| coverage::mutate(bytes, |coverage| coverage.version = 1),
    );
    verify_failure(
        &lab,
        &wrong_version,
        "product_selected_geometry_coverage_schema",
    );

    let selected_v28 = lab.0.join("negative-selected-v28-selection");
    rebuilt_package(
        &baseline,
        &selected_v28,
        None,
        Some(vec![declaration.clone()]),
        None,
        |_, bytes| {
            coverage::mutate(bytes, |coverage| {
                coverage.version = 1;
                coverage.selection = MapSelection::MapAndAncestors(fixture::POPULATED_MAP);
                coverage.coverage_profile = coverage::SELECTED_COVERAGE_PROFILE.into();
                coverage.source_profile = coverage::SELECTED_SOURCE_PROFILE.into();
                coverage.domain_profile = coverage::SELECTED_DOMAIN_PROFILE.into();
            });
        },
    );
    verify_failure(
        &lab,
        &selected_v28,
        "product_selected_geometry_coverage_schema",
    );
    verify_links_failure(
        &lab,
        &selected_v28,
        "product_selected_geometry_coverage_schema",
    );

    for name in ["coverage-profile", "source-profile", "domain-profile"] {
        let output = lab.0.join(format!("negative-{name}"));
        rebuilt_package(
            &baseline,
            &output,
            None,
            Some(vec![declaration.clone()]),
            None,
            |_, bytes| {
                coverage::mutate(bytes, |coverage| match name {
                    "coverage-profile" => {
                        coverage.coverage_profile = coverage::SELECTED_COVERAGE_PROFILE.into();
                    }
                    "source-profile" => {
                        coverage.source_profile = coverage::SELECTED_SOURCE_PROFILE.into();
                    }
                    "domain-profile" => {
                        coverage.domain_profile = coverage::SELECTED_DOMAIN_PROFILE.into();
                    }
                    _ => unreachable!(),
                });
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
        None,
        |_, bytes| coverage::mutate(bytes, |coverage| coverage.context.components = 16),
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
        None,
        |_, bytes| coverage::mutate(bytes, |coverage| coverage.context.plan_sha256 = [0x99; 32]),
    );
    verify_failure(&lab, &wrong_plan, "product_plan_mismatch");

    let global_claim = lab.0.join("negative-global-coverage-claim");
    rebuilt_package(
        &baseline,
        &global_claim,
        None,
        Some(vec![declaration.clone()]),
        None,
        |_, bytes| {
            coverage::mutate(bytes, |coverage| {
                coverage.complete_global_map_coverage = true
            })
        },
    );
    verify_failure(
        &lab,
        &global_claim,
        "product_selected_geometry_coverage_schema",
    );

    let wrong_parent = lab.0.join("negative-wdt-zero-child-parent-fact");
    rebuilt_package(
        &baseline,
        &wrong_parent,
        None,
        Some(vec![declaration.clone()]),
        None,
        |_, bytes| {
            coverage::mutate(bytes, |coverage| {
                coverage.maps[1].is_parent = false;
            });
        },
    );
    verify_failure(&lab, &wrong_parent, "product_collision_association");

    let reduced_rows = lab.0.join("negative-reduced-materialized-rows");
    rebuilt_package(
        &baseline,
        &reduced_rows,
        None,
        Some(vec![declaration.clone()]),
        None,
        |_, bytes| coverage::mutate(bytes, |coverage| coverage.map.materialized_rows = 4),
    );
    verify_failure(&lab, &reduced_rows, "product_collision_association");

    let hidden_unknown = lab.0.join("negative-hidden-unknown-invented");
    rebuilt_package(
        &baseline,
        &hidden_unknown,
        None,
        Some(vec![declaration.clone()]),
        None,
        |_, bytes| {
            coverage::mutate(bytes, |coverage| {
                coverage.maps.push(coverage::DomainMapFact {
                    id: fixture::HIDDEN_MAP,
                    parent: None,
                    wdt_file_data_id: 109,
                    depth: 0,
                    is_parent: false,
                });
            });
        },
    );
    verify_failure(&lab, &hidden_unknown, "product_collision_association");
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

fn assert_redacted_diagnostic(
    record: &str,
    client: &Path,
    package: &Path,
    phase: &str,
    request_kind: &str,
    file_data_id: u32,
    ordinal: u32,
) {
    assert_eq!(record.lines().count(), 1, "{record}");
    for expected in [
        format!("\"phase\":\"{phase}\""),
        format!("\"request_kind\":\"{request_kind}\""),
        format!("\"file_data_id\":{file_data_id}"),
        format!("\"ordinal\":{ordinal}"),
        "\"typed_cause\":{".into(),
        "\"variant\":\"NoLocalEncoding\"".into(),
        "\"corrupt\":1".into(),
        "\"blte_code\":\"blte_checksum_mismatch\"".into(),
        "\"code\":\"local_store_no_verified_encoding\"".into(),
        "\"guards_observed\":".into(),
        "\"tolerated_absences\":".into(),
    ] {
        assert!(record.contains(&expected), "missing {expected}: {record}");
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
fn v29_diagnostics_classify_wdt_and_model_failures_with_redacted_records() {
    let lab = Lab::new();
    for (stem, file_data_id, phase, request_kind, ordinal) in [
        ("wdt", fixture::POPULATED_WDT, "placements", "wdt", 1),
        ("model", fixture::MODEL, "models", "root_m2", 0),
    ] {
        let (client, written) = lab.client(&format!("diagnostic-{stem}-client"));
        fixture::corrupt_wdt(&client, &written, file_data_id);
        let output = lab.0.join(format!("diagnostic-{stem}-package"));
        let scratch = lab.private_dir(&format!("diagnostic-{stem}-scratch"));
        let diagnostic = lab.0.join(format!("diagnostic-{stem}.jsonl"));
        assert_failure(
            produce_diagnostic(&lab, &client, &output, &scratch, &diagnostic),
            "local_store_no_verified_encoding",
        );
        let record = fs::read_to_string(&diagnostic).unwrap();
        assert_redacted_diagnostic(
            &record,
            &client,
            &output,
            phase,
            request_kind,
            file_data_id,
            ordinal,
        );
    }
}

#[test]
fn v29_diagnostic_is_opt_in_and_preserves_the_terminal_result() {
    let lab = Lab::new();
    let (client, written) = lab.client("diagnostic-opt-in-client");
    fixture::corrupt_wdt(&client, &written, fixture::POPULATED_WDT);
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
    assert_redacted_diagnostic(
        &record,
        &client,
        &with_output,
        "placements",
        "wdt",
        fixture::POPULATED_WDT,
        1,
    );
}

#[test]
fn v29_diagnostic_rejects_client_and_package_output_paths() {
    let lab = Lab::new();
    let (client, _) = lab.client("diagnostic-path-client");

    let output = lab.0.join("diagnostic-client-path-package");
    let scratch = lab.private_dir("diagnostic-client-path-scratch");
    let inside_client = client.join("diagnostic.jsonl");
    assert_failure(
        produce_diagnostic(&lab, &client, &output, &scratch, &inside_client),
        "product_diagnostic_path",
    );
    assert!(!inside_client.exists());

    let output = lab.0.join("diagnostic-package-path-package");
    fs::create_dir(&output).unwrap();
    let scratch = lab.private_dir("diagnostic-package-path-scratch");
    let inside_package = output.join("diagnostic.jsonl");
    assert_failure(
        produce_diagnostic(&lab, &client, &output, &scratch, &inside_package),
        "product_diagnostic_path",
    );
    assert!(!inside_package.exists());
}
