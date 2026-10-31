// SPDX-License-Identifier: GPL-3.0-or-later
//! Synthetic v28 authenticated selected-geometry CLI lifecycle only.
#![cfg(target_os = "linux")]

#[path = "v28_authenticated_geometry_cli/coverage.rs"]
mod coverage;
#[path = "v28_authenticated_geometry_cli/fixture.rs"]
mod fixture;

use coverage::Coverage;
use forever_casc::{
    collision::world,
    integrity,
    package::{Artifact, Component, Context, Limits, PartialEncryptedSection, Run},
};
use forever_extractor::plan::{
    AUTHENTICATED_SELECTED_GEOMETRY_VERIFY_PROFILE, AUTHENTICATED_SELECTED_GEOMETRY_VERIFY_SCOPE,
};
use forever_vmap::{placement, vmo};
use std::{
    collections::BTreeSet,
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};

const PLAN: &str = include_str!("../../../tests/contracts/fixtures/forever-product-v28/map2.tsv");
const MAP_PATH: &str = "dbc/enUS/Map.db2";
static NEXT: AtomicU64 = AtomicU64::new(0);

struct Lab(PathBuf);

impl Lab {
    fn new() -> Self {
        let root = std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join(format!(
                ".local/share/rce-v28-authenticated-geometry-cli-{}-{}",
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
            "v28 extract",
        );
        output
    }

    fn resume(&self, stem: &str, output: &Path) {
        let (client, _) = self.client(&format!("{stem}-client"));
        let scratch = self.private_dir(&format!("{stem}-scratch"));
        assert_success(
            self.produce("resume", &client, output, &scratch),
            "v28 resume",
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
        text.matches("\"path\":\"dbc/enUS/Map.db2\",\"sections\":")
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
                AUTHENTICATED_SELECTED_GEOMETRY_VERIFY_PROFILE,
            ),
            ("scope", AUTHENTICATED_SELECTED_GEOMETRY_VERIFY_SCOPE),
            ("coverage_authentication", "independently_derived"),
        ] {
            let identity = format!("\"{field}\":\"{value}\"");
            assert!(
                report.contains(&identity),
                "{verb} omitted {identity}: {report}"
            );
        }
        assert!(report.contains("\"components\":17"), "{report}");
        assert!(
            report.contains("\"full_extractor_complete\":false"),
            "{report}"
        );
    }
}

fn compare_with_baseline(lab: &Lab, baseline: &Path, resumed: &Path) {
    assert_eq!(
        fs::read(resumed.join(MAP_PATH)).unwrap(),
        fs::read(baseline.join(MAP_PATH)).unwrap()
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
        "dbc/enUS/LiquidObject.db2",
        "dbc/enUS/GameObjectDisplayInfo.db2",
        "vmaps/0003",
        "vmaps/0004",
        "vmaps/coverage/0003.placements",
        "vmaps/coverage/0004.placements",
    ] {
        assert!(!package.join(excluded).exists(), "unexpected {excluded}");
    }

    let context = client_context(package);
    assert_eq!(context.build, 70450);
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

    let coverage = Coverage::parse(&fs::read(package.join(coverage::PATH)).unwrap()).unwrap();
    assert_eq!(coverage.context, context);
    assert_eq!(coverage.requested_map, fixture::SELECTED_MAP);
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
    let root = &coverage.maps[0];
    assert_eq!(root.id, fixture::ROOT_MAP);
    assert_eq!(root.parent, None);
    assert_eq!(root.wdt_file_data_id, fixture::ROOT_WDT);
    assert_eq!(root.depth, 0);
    assert!(root.is_parent);
    let selected = &coverage.maps[1];
    assert_eq!(selected.id, fixture::SELECTED_MAP);
    assert_eq!(selected.parent, Some(fixture::ROOT_MAP));
    assert_eq!(selected.wdt_file_data_id, fixture::SELECTED_WDT);
    assert_eq!(selected.depth, 1);
    assert!(selected.is_parent);

    let mut expected_paths = BTreeSet::new();
    let mut model_names = BTreeSet::new();
    for (map, assigned_id) in [
        (fixture::ROOT_MAP, u32::MAX - 1),
        (fixture::SELECTED_MAP, u32::MAX - 2),
    ] {
        let proof_path = format!("vmaps/coverage/{map:04}.placements");
        let proof = fs::read(package.join(&proof_path)).unwrap();
        let placements = placement::DirBin::parse(&proof, Default::default()).unwrap();
        assert_eq!(placements.record_count(), 1);
        let record = placements.records().next().unwrap();
        assert_eq!(record.id(), assigned_id);
        model_names.insert(std::str::from_utf8(record.name()).unwrap().to_owned());
        expected_paths.insert(proof_path);
        expected_paths.insert(format!("vmaps/{map:04}/{map:04}.vmtree"));
        expected_paths.insert(format!("vmaps/{map:04}/{map:04}_00_00.vmtileidx"));
        expected_paths.insert(format!("vmaps/{map:04}/{map:04}_00_00.vmtile"));
    }
    assert_eq!(model_names.len(), 1);
    for name in model_names {
        expected_paths.insert(format!("vmaps/{name}.vmo"));
    }
    let actual_paths: BTreeSet<_> = coverage
        .artifacts
        .iter()
        .map(|artifact| artifact.path.clone())
        .collect();
    assert_eq!(actual_paths, expected_paths);
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
fn v28_inspect_extract_verify_and_verify_links_use_only_authenticated_selected_geometry() {
    let lab = Lab::new();
    let inspection = assert_success(lab.command("inspect").output().unwrap(), "v28 inspect");
    for expected in [
        "\"contract\":\"forever-product-v28\"",
        "\"status\":\"planned\"",
        "\"build\":70450",
        "\"components\":17",
        "\"db2_files\":1",
        "\"collision_storage\":\"linux-leased-append-disk-world-v1\"",
        "\"collision_domain\":\"selected-map-and-active-ancestors\"",
        "\"requested_map\":2",
        "\"coverage_authentication\":\"independently_derived\"",
        "\"map_source_scope\":\"available_sections+vmap_materialization\"",
        "\"partial_db2_declarations\":\"per-table-including-Map\"",
        "\"liquid_object_scope\":\"out_of_scope\"",
        "\"complete_global_map_coverage_claim\":false",
        "\"full_extractor_complete\":false",
    ] {
        assert!(inspection.contains(expected), "{inspection}");
    }
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
fn v28_resume_matches_uninterrupted_output_at_db2_collision_and_final_seal_boundaries() {
    let lab = Lab::new();
    let baseline = lab.extract("resume-baseline");
    assert_coverage_and_inventory(&baseline);

    let (early_client, early_written) = lab.client("resume-early-source");
    fixture::corrupt_wdt(&early_client, &early_written, fixture::ROOT_WDT);
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

fn rebuilt_package(
    base: &Path,
    output: &Path,
    global_sections: Option<Vec<PartialEncryptedSection>>,
    map_sections: Option<Vec<PartialEncryptedSection>>,
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

#[derive(Clone, Copy)]
enum CoverageTamper {
    Parent,
    Wdt,
    Depth,
    IsParent,
    MaterializedRows,
    Sections,
}

#[test]
fn v28_resealed_partial_and_authentication_negatives_reach_product_checks() {
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
        |bytes, _| {
            assert_eq!(bytes[map.hidden_start], 0);
            bytes[map.hidden_start] = 1;
        },
    );
    verify_failure(&lab, &zero_fill, "product_partial_zerofill");

    for (name, tamper) in [
        ("parent", CoverageTamper::Parent),
        ("wdt", CoverageTamper::Wdt),
        ("depth", CoverageTamper::Depth),
        ("is-parent", CoverageTamper::IsParent),
        ("materialized-rows", CoverageTamper::MaterializedRows),
        ("sections", CoverageTamper::Sections),
    ] {
        let output = lab.0.join(format!("negative-coverage-{name}"));
        rebuilt_package(
            &baseline,
            &output,
            None,
            Some(vec![declaration.clone()]),
            |_, bytes| {
                let coverage = Coverage::parse(bytes).unwrap();
                match tamper {
                    CoverageTamper::Parent => {
                        let selected = coverage
                            .maps
                            .iter()
                            .find(|map| map.id == fixture::SELECTED_MAP)
                            .unwrap();
                        coverage::put_u32(bytes, selected.offsets.parent, 0);
                    }
                    CoverageTamper::Wdt => {
                        let selected = coverage
                            .maps
                            .iter()
                            .find(|map| map.id == fixture::SELECTED_MAP)
                            .unwrap();
                        coverage::put_u32(bytes, selected.offsets.wdt, fixture::ROOT_WDT);
                    }
                    CoverageTamper::Depth => {
                        let selected = coverage
                            .maps
                            .iter()
                            .find(|map| map.id == fixture::SELECTED_MAP)
                            .unwrap();
                        coverage::put_u32(bytes, selected.offsets.depth, 0);
                    }
                    CoverageTamper::IsParent => {
                        let selected = coverage
                            .maps
                            .iter()
                            .find(|map| map.id == fixture::SELECTED_MAP)
                            .unwrap();
                        coverage::put_bool(bytes, selected.offsets.is_parent, false);
                    }
                    CoverageTamper::MaterializedRows => {
                        coverage::put_u64(bytes, coverage.map.offsets.materialized_rows, 4);
                    }
                    CoverageTamper::Sections => {
                        coverage::put_u32(bytes, coverage.map.sections[1].records_offset, 0);
                        coverage::put_u64(bytes, coverage.map.offsets.declared_direct_rows, 4);
                    }
                }
            },
        );
        assert_canonical_partial_manifest(&manifest(&output));
        verify_failure(&lab, &output, "product_collision_association");
        if matches!(tamper, CoverageTamper::Wdt) {
            assert_failure(
                lab.command("verify-links")
                    .args(["--output"])
                    .arg(&output)
                    .output()
                    .unwrap(),
                "product_collision_association",
            );
        }
    }
}
