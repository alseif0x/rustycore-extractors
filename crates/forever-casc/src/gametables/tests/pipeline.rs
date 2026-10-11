// SPDX-License-Identifier: GPL-3.0-or-later
//! Synthetic clients/files only; acquisition is an injected Rust closure.
#[cfg(unix)]
use super::*;
use crate::gametables::{
    batch::{self, Plan},
    options,
};
#[cfg(unix)]
use crate::gametables::{
    manifest,
    verify::{self, Code},
};
use std::{
    ffi::OsString,
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

#[cfg(unix)]
use std::path::Path;
static NEXT: AtomicU64 = AtomicU64::new(0);
pub(super) struct Lab {
    pub(super) root: PathBuf,
    pub(super) client: PathBuf,
}
impl Lab {
    pub(super) fn new() -> Self {
        let mut base = std::env::temp_dir();
        if base.ancestors().any(|path| path.join(".git").exists()) {
            base = PathBuf::from("/tmp");
        }
        let root = base.join(format!(
            "rce-gt-synthetic-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        let client = root.join("client");
        fs::create_dir(&client).unwrap();
        fs::write(client.join(".build.info"), metadata(70205, "enUS")).unwrap();
        Self { root, client }
    }
    pub(super) fn output(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }
    #[cfg(unix)]
    pub(super) fn extract(&self, plan: Plan, name: &str) -> batch::Report {
        batch::extract(
            plan,
            &self.client,
            &self.output(name),
            70205,
            crate::Locale::EnUs,
            |table, budget| {
                assert_eq!(budget, MAX_FILE_BYTES as u32);
                Ok((70205, data(table)))
            },
        )
        .unwrap()
    }
}
impl Drop for Lab {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
pub(super) fn metadata(build: u32, tags: &str) -> String {
    format!(
        "Product!STRING:0|Active!DEC:1|Version!STRING:0|Build Key!HEX:16|CDN Key!HEX:16|Tags!STRING:0\nwow_classic_beta|1|1.60.1.{build}|11111111111111111111111111111111|22222222222222222222222222222222|{tags}\n"
    )
}
#[cfg(unix)]
fn data(table: GameTable) -> Vec<u8> {
    fixture(
        table,
        &["91", "91"],
        &vec!["-0"; table.value_columns()],
        "\r\n",
    )
}
#[cfg(unix)]
pub(super) fn rewrite(path: &Path, from: &str, to: &str) {
    let text = fs::read_to_string(path).unwrap();
    assert!(text.contains(from));
    fs::write(path, text.replacen(from, to, 1)).unwrap();
}

#[cfg(unix)]
#[test]
fn all_closed_plans_publish_exact_raw_bytes_and_verify() {
    let lab = Lab::new();
    for plan in [Plan::All, Plan::Initial, Plan::SpellValues] {
        let report = lab.extract(plan, plan.name());
        assert!(report.complete);
        let root = lab.output(plan.name());
        assert!(!root.join(manifest::MARKER).exists());
        assert_eq!(
            fs::read_to_string(root.join(manifest::NAME)).unwrap(),
            report.manifest
        );
        let verified = verify::verify(&root);
        assert_eq!(verified.code, Code::Verified);
        assert_eq!(verified.artifacts_verified, plan.tables().len());
        for table in plan.tables() {
            assert_eq!(
                fs::read(root.join(table.relative_path())).unwrap(),
                data(table)
            );
        }
        assert!(report.json().contains("\"typed_loaders_executed\":false"));
        assert!(!report.manifest.contains(lab.root.to_str().unwrap()));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&root).unwrap().permissions().mode() & 0o777,
                0o700
            );
            assert_eq!(
                fs::metadata(root.join(manifest::NAME))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
        }
    }
}

#[cfg(unix)]
#[test]
fn missing_short_or_unsupported_numeric_file_never_accepts_the_group() {
    let lab = Lab::new();
    for (name, error) in [
        ("missing", crate::Error::MissingFile),
        ("short", crate::Error::Incomplete),
        ("key", crate::Error::Encrypted),
    ] {
        let root = lab.output(name);
        let report = batch::extract(
            Plan::Initial,
            &lab.client,
            &root,
            70205,
            crate::Locale::EnUs,
            |table, _| {
                if table == GameTable::HpPerSta {
                    Err(error)
                } else {
                    Ok((70205, data(table)))
                }
            },
        )
        .unwrap();
        assert!(!report.complete);
        assert!(root.join(manifest::MARKER).is_file());
        assert!(!root.join(GameTable::HpPerSta.relative_path()).exists());
        assert_eq!(verify::verify(&root).code, Code::Incomplete);
        fs::remove_file(root.join(manifest::MARKER)).unwrap();
        assert_eq!(verify::verify(&root).code, Code::Incomplete);
    }
    let root = lab.output("numeric");
    let report = batch::extract(
        Plan::Initial,
        &lab.client,
        &root,
        70205,
        crate::Locale::EnUs,
        |table, _| {
            Ok((
                70205,
                fixture(table, &["id"], &vec!["NaN"; table.value_columns()], "\n"),
            ))
        },
    )
    .unwrap();
    assert!(!report.complete);
    assert_eq!(fs::read_dir(root.join("gt")).unwrap().count(), 0);
}

#[cfg(unix)]
#[test]
fn changed_reported_build_is_partial_and_final_metadata_mutation_is_fatal() {
    let lab = Lab::new();
    let root = lab.output("build");
    let report = batch::extract(
        Plan::All,
        &lab.client,
        &root,
        70205,
        crate::Locale::EnUs,
        |table, _| Ok((70170, data(table))),
    )
    .unwrap();
    assert!(!report.complete);
    assert_eq!(verify::verify(&root).code, Code::Incomplete);
    let root = lab.output("metadata");
    let result = batch::extract(
        Plan::Initial,
        &lab.client,
        &root,
        70205,
        crate::Locale::EnUs,
        |table, _| {
            if table == GameTable::Xp {
                fs::write(lab.client.join(".build.info"), metadata(70205, "esES")).unwrap();
            }
            Ok((70205, data(table)))
        },
    );
    assert_eq!(result.err(), Some("client_metadata_changed"));
    assert!(root.join(manifest::MARKER).is_file());
    assert!(!root.join(manifest::NAME).exists());
}

#[test]
fn unsupported_build_and_invalid_client_fail_before_output_creation() {
    let lab = Lab::new();
    let root = lab.output("unsupported");
    assert_eq!(
        batch::extract(
            Plan::All,
            &lab.client,
            &root,
            70206,
            crate::Locale::EnUs,
            |_, _| panic!()
        )
        .err(),
        Some("unsupported_client")
    );
    assert!(!root.exists());
    fs::write(lab.client.join(".build.info"), b"bad").unwrap();
    assert!(
        batch::extract(
            Plan::All,
            &lab.client,
            &root,
            70205,
            crate::Locale::EnUs,
            |_, _| panic!()
        )
        .is_err()
    );
    assert!(!root.exists());
}

#[cfg(unix)]
#[test]
fn oversized_returned_bytes_are_rejected_without_publication() {
    let lab = Lab::new();
    let root = lab.output("large");
    let report = batch::extract(
        Plan::Initial,
        &lab.client,
        &root,
        70205,
        crate::Locale::EnUs,
        |_, limit| Ok((70205, vec![b' '; limit as usize + 1])),
    )
    .unwrap();
    assert!(!report.complete);
    assert_eq!(fs::read_dir(root.join("gt")).unwrap().count(), 0);
    assert_eq!(verify::verify(&root).code, Code::Incomplete);
}

#[cfg(unix)]
#[test]
fn output_guards_preserve_existing_outputs_and_reject_client_git_or_traversal() {
    let lab = Lab::new();
    lab.extract(Plan::Initial, "existing");
    let existing = lab.output("existing");
    let original = fs::read(existing.join(manifest::NAME)).unwrap();
    assert_eq!(
        batch::extract(
            Plan::All,
            &lab.client,
            &existing,
            70205,
            crate::Locale::EnUs,
            |_, _| panic!()
        )
        .err(),
        Some("output_exists_or_unavailable")
    );
    assert_eq!(fs::read(existing.join(manifest::NAME)).unwrap(), original);
    for root in [
        lab.client.join("output"),
        lab.root.clone(),
        lab.root.join("client/../bad"),
    ] {
        assert_eq!(
            batch::extract(
                Plan::All,
                &lab.client,
                &root,
                70205,
                crate::Locale::EnUs,
                |_, _| panic!()
            )
            .err(),
            Some("unsafe_output_path")
        );
    }
    let git = lab.output("checkout");
    fs::create_dir(&git).unwrap();
    fs::write(git.join(".git"), b"synthetic git marker").unwrap();
    assert_eq!(
        batch::extract(
            Plan::All,
            &lab.client,
            &git.join("out"),
            70205,
            crate::Locale::EnUs,
            |_, _| panic!()
        )
        .err(),
        Some("unsafe_output_path")
    );
}

#[cfg(unix)]
#[test]
fn verifier_detects_raw_tampering_missing_extra_and_pending_files() {
    let lab = Lab::new();
    for scenario in ["tamper", "missing", "extra", "pending", "rootextra"] {
        lab.extract(Plan::Initial, scenario);
        let root = lab.output(scenario);
        let file = root.join(GameTable::BaseMp.relative_path());
        let expected = match scenario {
            "tamper" => {
                let mut bytes = fs::read(&file).unwrap();
                *bytes.last_mut().unwrap() = b' ';
                fs::write(file, bytes).unwrap();
                Code::Inventory
            }
            "missing" => {
                fs::remove_file(file).unwrap();
                Code::Inventory
            }
            "extra" => {
                fs::write(root.join("gt/extra.txt"), b"x").unwrap();
                Code::Inventory
            }
            "pending" => {
                fs::write(root.join("gt/new.part"), b"x").unwrap();
                Code::Incomplete
            }
            _ => {
                fs::write(root.join("extra"), b"x").unwrap();
                Code::Inventory
            }
        };
        assert_eq!(verify::verify(&root).code, expected);
    }
}

#[cfg(unix)]
#[test]
fn manifest_identifiers_paths_counts_and_claims_are_validated() {
    let lab = Lab::new();
    for (index, (from, to)) in [
        ("\"file_id\":1391664", "\"file_id\":1"),
        (
            "\"relative_path\":\"gt/BaseMp.txt\"",
            "\"relative_path\":\"../BaseMp.txt\"",
        ),
        (
            "\"relative_path\":\"gt/BaseMp.txt\"",
            "\"relative_path\":\"gt/basemp.txt\"",
        ),
        ("\"value_columns\":15", "\"value_columns\":1"),
        ("\"offline\":true", "\"offline\":false"),
        (
            "\"typed_loaders_executed\":false",
            "\"typed_loaders_executed\":true",
        ),
        ("\"plan\":\"initial\"", "\"plan\":\"all\""),
        ("\"file_count\":3", "\"file_count\":2"),
        (
            "\"manifest_version\":1",
            "\"manifest_version\":1,\"unknown\":0",
        ),
        (
            "\"manifest_version\":1",
            "\"manifest_version\":1,\"manifest_version\":1",
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let name = format!("bad-{index}");
        lab.extract(Plan::Initial, &name);
        let root = lab.output(&name);
        rewrite(&root.join(manifest::NAME), from, to);
        assert_eq!(verify::verify(&root).code, Code::Manifest);
    }
}

#[cfg(unix)]
#[test]
fn valid_schema_but_false_row_facts_fail_reparse() {
    let lab = Lab::new();
    lab.extract(Plan::Initial, "facts");
    let root = lab.output("facts");
    rewrite(
        &root.join(manifest::NAME),
        "\"data_rows\":2,\"consumer_rows_including_zero\":3",
        "\"data_rows\":1,\"consumer_rows_including_zero\":2",
    );
    assert_eq!(verify::verify(&root).code, Code::Numeric);
}

#[cfg(unix)]
#[test]
fn manifest_parser_is_bounded_and_not_sensitive_to_whitespace() {
    let lab = Lab::new();
    let report = lab.extract(Plan::Initial, "json");
    assert!(manifest::decode(&report.manifest.replace(',', ",\n ")).is_ok());
    assert!(manifest::decode(&format!("{}0{}", "[".repeat(10000), "]".repeat(10000))).is_err());
    assert!(manifest::decode(&" ".repeat(manifest::MAX_MANIFEST_BYTES + 1)).is_err());
    assert!(
        manifest::decode(
            &report
                .manifest
                .replace("\"manifest_version\":1", "\"manifest_version\":1.0")
        )
        .is_err()
    );
}

#[cfg(unix)]
#[test]
fn symlink_roots_children_outputs_and_client_data_aliases_are_rejected() {
    use std::os::unix::fs::symlink;
    let lab = Lab::new();
    lab.extract(Plan::Initial, "ok");
    let root = lab.output("ok");
    let alias = lab.output("alias");
    symlink(&root, &alias).unwrap();
    assert_eq!(verify::verify(&alias).code, Code::UnsafePath);
    assert_eq!(
        batch::extract(
            Plan::Initial,
            &lab.client,
            &alias,
            70205,
            crate::Locale::EnUs,
            |_, _| panic!()
        )
        .err(),
        Some("output_exists_or_unavailable")
    );
    let file = root.join(GameTable::BaseMp.relative_path());
    fs::remove_file(&file).unwrap();
    symlink(lab.client.join(".build.info"), file).unwrap();
    assert_eq!(verify::verify(&root).code, Code::UnsafePath);
    let data = lab.output("data");
    fs::create_dir(&data).unwrap();
    symlink(&data, lab.client.join("Data")).unwrap();
    assert_eq!(
        batch::extract(
            Plan::Initial,
            &lab.client,
            &data.join("out"),
            70205,
            crate::Locale::EnUs,
            |_, _| panic!()
        )
        .err(),
        Some("unsafe_output_path")
    );
}

#[test]
fn offline_options_allow_only_closed_groups_and_reject_network_or_missing_values() {
    let args = |values: &[&str]| values.iter().map(OsString::from).collect::<Vec<_>>();
    let base = ["--client", "private", "--output", "out", "--locale", "esES"];
    let options::Parsed::Run(parsed) = options::parse(&args(&base)).unwrap() else {
        panic!()
    };
    assert_eq!(parsed.selection, options::Selection::Typed(Plan::All));
    for group in ["all", "initial", "spell-values"] {
        let mut values = base.to_vec();
        values.extend(["--group", group]);
        assert!(options::parse(&args(&values)).is_ok());
    }
    for tail in [
        ["--group", "unknown"],
        ["--allow-network", "true"],
        ["--remote-casc", "true"],
        ["--locale", "enUS"],
        ["--group", ""],
    ] {
        let mut values = base.to_vec();
        values.extend(tail);
        assert!(options::parse(&args(&values)).is_err());
    }
    assert!(options::parse(&args(&["--client"])).is_err());
    assert!(options::verify_output(&args(&["--output", "private", "--repair", "true"])).is_err());
    assert!(options::verify_output(&args(&["--output", "private"])).is_ok());
}
