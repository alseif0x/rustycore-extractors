// SPDX-License-Identifier: GPL-3.0-or-later
//! Synthetic v25 end-to-end lifecycle and manifest verification only.
#![cfg(target_os = "linux")]

#[path = "v25_partial_map_cli/fixture.rs"]
mod synthetic_client;
mod db2_fixture {
    include!("v25_partial_map_db2.rs");
}

use forever_casc::{
    local_index,
    local_store::{LocalStore, OpenOptions},
    package::{Artifact, Component, Context, Limits, PartialEncryptedSection, Run},
    root,
};
use forever_db2::catalog;
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};

const PLAN: &str = include_str!("../../../tests/contracts/fixtures/forever-product-v25/map2.tsv");
static NEXT: AtomicU64 = AtomicU64::new(0);

struct Lab(PathBuf);
impl Lab {
    fn new() -> Self {
        let root = std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join(format!(
                ".local/share/rce-v25-partial-map-cli-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
        fs::create_dir_all(root.parent().unwrap()).unwrap();
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        Self(root)
    }
    fn plan(&self) -> PathBuf {
        let path = self.0.join("plan.tsv");
        fs::write(&path, PLAN.replace("terrain\tmap:2", "terrain\tmap:48")).unwrap();
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

fn rebuilt_package(
    base: &Path,
    output: &Path,
    partial: Option<Vec<PartialEncryptedSection>>,
    mutate_map: impl FnOnce(&mut Vec<u8>),
) {
    let context = client_context(base);
    let mut run = Run::create(output, context, &[base], Limits::default()).unwrap();
    if let Some(sections) = partial {
        run.set_partial_encrypted_sections(&sections).unwrap();
    }
    let mut mutate_map = Some(mutate_map);
    for artifact in package_artifacts(base, context) {
        let mut bytes = fs::read(base.join(&artifact.path)).unwrap();
        if artifact.path == "dbc/enUS/Map.db2" {
            mutate_map.take().unwrap()(&mut bytes);
        }
        run.put(artifact.component, &artifact.path, &bytes).unwrap();
    }
    run.finish_component(Component::Db2).unwrap();
    run.finish_component(Component::Terrain).unwrap();
    run.finish().unwrap();
}

#[test]
fn v25_partial_map_extract_resume_verify_and_verify_links() {
    let lab = Lab::new();
    let plan = lab.plan();
    let client = lab.0.join("client");
    fs::create_dir(&client).unwrap();
    let _wdt_offset = synthetic_client::write_partial_client(&client, b'S');
    let options = OpenOptions {
        locale: forever_casc::Locale::EnUs,
        index_layouts: [local_index::Layout::Contiguous; 16],
        root_layout: root::Layout::Header50893V2,
        limits: Default::default(),
    };
    let mut store = LocalStore::open_db2_70450(&client, options).unwrap();
    for id in [100, 200].into_iter().chain([
        catalog::resolve_for(catalog::WOWDBDEFS_70450_CATALOG_ID, "LiquidType")
            .unwrap()
            .unwrap()
            .metadata()
            .file_data_id,
        catalog::resolve_for(catalog::WOWDBDEFS_70450_CATALOG_ID, "LiquidMaterial")
            .unwrap()
            .unwrap()
            .metadata()
            .file_data_id,
        catalog::resolve_for(catalog::WOWDBDEFS_70450_CATALOG_ID, "LiquidObject")
            .unwrap()
            .unwrap()
            .metadata()
            .file_data_id,
    ]) {
        assert!(
            store
                .read_file_data_id_with_limit(id, 16 * 1024 * 1024)
                .is_ok(),
            "missing authored source {id}"
        );
    }
    let package = lab.0.join("package");
    let extracted = lab
        .command("extract", &plan)
        .args(["--client"])
        .arg(&client)
        .args(["--output"])
        .arg(&package)
        .output()
        .unwrap();
    assert_success(extracted, "partial extract");
    let manifest = fs::read_to_string(package.join("forever-package.json")).unwrap();
    assert!(manifest.contains("\"partial_encrypted_sections\":[{"));
    assert!(manifest.contains("\"lookup\":\"1122334455667788\""));

    assert_success(
        lab.command("verify", &plan)
            .args(["--output"])
            .arg(&package)
            .output()
            .unwrap(),
        "partial verify",
    );
    assert_success(
        lab.command("verify-links", &plan)
            .args(["--output"])
            .arg(&package)
            .output()
            .unwrap(),
        "partial verify-links",
    );

    let resume_client = lab.0.join("resume-client");
    fs::create_dir(&resume_client).unwrap();
    let resume_wdt = synthetic_client::write_partial_client(&resume_client, b'S');
    let archive_path = resume_client.join("Data/data/data.000");
    let mut archive = fs::read(&archive_path).unwrap();
    let corrupt_at = resume_wdt as usize + 30 + 8 + 1 + 8;
    archive[corrupt_at] ^= 0xff;
    fs::write(&archive_path, archive).unwrap();
    let interrupted_package = lab.0.join("resume-package");
    let interrupted = lab
        .command("extract", &plan)
        .args(["--client"])
        .arg(&resume_client)
        .args(["--output"])
        .arg(&interrupted_package)
        .output()
        .unwrap();
    assert!(!interrupted.status.success());
    assert!(
        interrupted_package
            .join(".forever-package-incomplete")
            .is_file()
    );
    synthetic_client::write_partial_client(&resume_client, b'S');
    assert_success(
        lab.command("resume", &plan)
            .args(["--client"])
            .arg(&resume_client)
            .args(["--output"])
            .arg(&interrupted_package)
            .output()
            .unwrap(),
        "partial resume",
    );
    assert_success(
        lab.command("verify", &plan)
            .args(["--output"])
            .arg(&interrupted_package)
            .output()
            .unwrap(),
        "resumed partial verify",
    );
}

#[test]
fn v25_rejects_undeclared_inconsistent_and_corrupt_partial_maps_and_non_s_cipher() {
    let lab = Lab::new();
    let plan = lab.plan();
    let client = lab.0.join("client");
    fs::create_dir(&client).unwrap();
    synthetic_client::write_partial_client(&client, b'S');
    let package = lab.0.join("package");
    assert_success(
        lab.command("extract", &plan)
            .args(["--client"])
            .arg(&client)
            .args(["--output"])
            .arg(&package)
            .output()
            .unwrap(),
        "base partial extract",
    );
    let declaration = PartialEncryptedSection {
        lookup: "1122334455667788".into(),
        frame_ordinal: 1,
        decoded_size: 138,
    };

    let undeclared = lab.0.join("undeclared");
    rebuilt_package(&package, &undeclared, None, |_| {});
    assert_failure(
        lab.command("verify", &plan)
            .args(["--output"])
            .arg(&undeclared)
            .output()
            .unwrap(),
        "product_partial_undeclared",
    );

    let incoherent = lab.0.join("incoherent");
    rebuilt_package(
        &package,
        &incoherent,
        Some(vec![
            declaration.clone(),
            PartialEncryptedSection {
                lookup: "8877665544332211".into(),
                frame_ordinal: 3,
                decoded_size: 138,
            },
        ]),
        |_| {},
    );
    assert_failure(
        lab.command("verify", &plan)
            .args(["--output"])
            .arg(&incoherent)
            .output()
            .unwrap(),
        "product_partial_section_count",
    );

    let no_tact = lab.0.join("declaration-without-tact");
    rebuilt_package(
        &package,
        &no_tact,
        Some(vec![declaration.clone()]),
        |bytes| *bytes = synthetic_client::complete_map(),
    );
    assert_failure(
        lab.command("verify", &plan)
            .args(["--output"])
            .arg(&no_tact)
            .output()
            .unwrap(),
        "product_partial_tact_sections",
    );

    let (_zero_map, hidden_start, _) = db2_fixture::partial_map();
    let corrupted = lab.0.join("corrupted-zero-fill");
    rebuilt_package(&package, &corrupted, Some(vec![declaration]), |bytes| {
        bytes[hidden_start] = 1
    });
    assert_failure(
        lab.command("verify", &plan)
            .args(["--output"])
            .arg(&corrupted)
            .output()
            .unwrap(),
        "product_partial_zerofill",
    );

    let bad_cipher_client = lab.0.join("bad-cipher-client");
    fs::create_dir(&bad_cipher_client).unwrap();
    synthetic_client::write_partial_client(&bad_cipher_client, b'X');
    let bad_cipher_package = lab.0.join("bad-cipher-package");
    assert_failure(
        lab.command("extract", &plan)
            .args(["--client"])
            .arg(&bad_cipher_client)
            .args(["--output"])
            .arg(&bad_cipher_package)
            .output()
            .unwrap(),
        "blte_cipher_unsupported",
    );
}
