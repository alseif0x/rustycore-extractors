// SPDX-License-Identifier: GPL-3.0-or-later
//! Closed catalog v2 tests: all inputs and clients are synthetic.
#[cfg(unix)]
use super::pipeline::{Lab, metadata, rewrite};
use super::*;
use crate::gametables::{
    batch::Plan,
    catalog::{self, Table},
    manifest,
    options::{self, Selection},
};
#[cfg(unix)]
use crate::gametables::{
    catalog_batch, catalog_manifest,
    verify::{self, Code},
};
use crate::verify::json::{self, Json};
use std::ffi::OsString;
#[cfg(unix)]
use std::fs;

fn numeric(columns: usize) -> Vec<u8> {
    let header = (0..columns)
        .map(|index| format!("v{index}"))
        .collect::<Vec<_>>()
        .join("\t");
    let first = (0..columns)
        .map(|index| format!("{index}.5"))
        .collect::<Vec<_>>()
        .join("\t");
    let second = vec!["-0"; columns].join("\t");
    format!("ignored\t{header}\r\nduplicate\t{first}\r\nduplicate\t{second}\r\n").into_bytes()
}
fn bytes(table: Table) -> Vec<u8> {
    table
        .value_columns()
        .map_or_else(|| b"opaque\0\xff\tNaN\r\n".to_vec(), numeric)
}
#[cfg(unix)]
fn extract(lab: &Lab, name: &str) -> catalog_batch::Report {
    catalog_batch::extract(
        &lab.client,
        &lab.output(name),
        70205,
        crate::Locale::EnUs,
        |table, budget| {
            assert_eq!(budget as usize, MAX_FILE_BYTES);
            Ok((70205, bytes(table)))
        },
    )
    .unwrap()
}

#[test]
fn catalog_identities_columns_and_source_fingerprint_are_pinned() {
    let expected = [
        ("ArtifactKnowledgeMultiplier", 1582086, Some(1)),
        ("ArtifactLevelXP", 1391662, Some(2)),
        ("BarberShopCostBase", 1391663, Some(1)),
        ("BaseMp", 1391664, Some(15)),
        ("BaseProfessionRatings", 4494528, None),
        ("BattlePetTypeDamageMod", 1391665, None),
        ("BattlePetXP", 1391666, Some(2)),
        ("CombatRatings", 1391669, Some(32)),
        ("CombatRatingsMultByILvl", 1391670, Some(4)),
        ("HonorLevel", 1391671, None),
        ("HpPerSta", 1391642, Some(1)),
        ("ItemLevelByLevel", 2012881, Some(1)),
        ("ItemLevelSquish", 1726830, None),
        ("ItemSocketCostPerLevel", 1391643, Some(1)),
        ("NPCManaCostScaler", 1391651, Some(1)),
        ("ProfessionRatings", 4492239, None),
        ("SandboxScaling", 1391659, None),
        ("SpellScaling", 1391660, Some(24)),
        ("StaminaMultByILvl", 1980632, Some(4)),
        ("xp", 1391661, Some(5)),
    ];
    assert_eq!(
        catalog::fingerprint(),
        "45fbc37f023eb54d71e1e47ab2fe252fabb139a5c923e25b1f9244399c29fec4"
    );
    let facts = json::parse(catalog::SOURCE_FACTS).unwrap();
    let Json::Array(entries) = manifest::field(&facts, "tables").unwrap() else {
        panic!()
    };
    assert_eq!(entries.len(), 20);
    for (((name, id, columns), table), entry) in expected.into_iter().zip(Table::ALL).zip(entries) {
        assert_eq!(
            (table.name(), table.file_id(), table.value_columns()),
            (name, id, columns)
        );
        assert_eq!(table.file_name(), format!("{name}.txt"));
        assert_eq!(table.relative_path(), format!("gt/{name}.txt"));
        assert_eq!(Table::named(name), Some(table));
        assert_eq!(Table::named(table.file_name()), None);
        manifest::equal_string(entry, "name", table.name()).unwrap();
        manifest::equal_string(entry, "relative_path", table.relative_path()).unwrap();
        manifest::equal_string(entry, "validation_mode", table.validation_mode()).unwrap();
        manifest::equal_number(entry, "file_id", id as usize).unwrap();
        match columns {
            Some(count) => manifest::equal_number(entry, "value_columns", count).unwrap(),
            None => assert!(matches!(
                manifest::field(entry, "value_columns").unwrap(),
                Json::Null
            )),
        }
    }
    assert_eq!(
        Table::ALL
            .into_iter()
            .filter(|table| table.value_columns().is_some())
            .count(),
        14
    );
    assert_eq!(Table::named("Xp"), None);
    assert_eq!(Table::named("../xp"), None);
    assert_eq!(Plan::All.tables().len(), 6);
    assert_eq!(Plan::named("catalog"), None);
}

#[test]
fn fourteen_numeric_schemas_preserve_every_cell_and_implicit_zero() {
    for table in Table::ALL
        .into_iter()
        .filter(|table| table.value_columns().is_some())
    {
        let columns = table.value_columns().unwrap();
        let parsed = catalog::parse_numeric(&bytes(table), table).unwrap();
        assert_eq!(parsed.facts().data_rows, 2);
        assert_eq!(parsed.facts().value_columns, columns);
        assert_eq!(parsed.rows().len(), 3);
        assert_eq!(parsed.row(0).unwrap(), vec![0; columns]);
        assert_eq!(
            parsed.row(1).unwrap(),
            (0..columns)
                .map(|index| (index as f32 + 0.5).to_bits())
                .collect::<Vec<_>>()
        );
        assert_eq!(parsed.row(2).unwrap(), vec![(-0.0f32).to_bits(); columns]);
        assert!(parsed.row(3).is_none());
    }
}

#[test]
fn six_unknown_schemas_cannot_produce_numeric_facts() {
    for table in Table::ALL
        .into_iter()
        .filter(|table| table.value_columns().is_none())
    {
        for raw in [bytes(table), numeric(1)] {
            assert_eq!(
                catalog::parse_numeric(&raw, table).err(),
                Some(Error::Schema)
            );
        }
    }
}

#[test]
fn v1_numeric_policy_and_known_six_bits_are_unchanged() {
    for old in GameTable::ALL {
        let new = Table::named(old.name()).unwrap();
        let data = numeric(old.value_columns());
        let left = parse(&data, old).unwrap();
        let right = catalog::parse_numeric(&data, new).unwrap();
        assert_eq!(left.facts(), right.facts());
        assert_eq!(left.rows(), right.rows());
    }
    for token in ["NaN", "inf", "0x1", "1e999", "1e-999", "\u{b}1", "1 "] {
        let data = format!("id\tv\n1\t{token}\n");
        assert_eq!(
            parse(data.as_bytes(), GameTable::HpPerSta).err(),
            catalog::parse_numeric(data.as_bytes(), Table::HpPerSta).err()
        );
    }
}

#[test]
fn numeric_catalog_rejects_malformed_text_and_checks_32_column_budgets() {
    let good = numeric(32);
    let limits = Limits {
        bytes: good.len(),
        data_rows: 2,
        values: 96,
    };
    assert_eq!(
        parse_columns(&good, 32, limits)
            .unwrap()
            .facts()
            .consumer_rows_including_zero,
        3
    );
    for bounded in [
        Limits {
            values: 95,
            ..limits
        },
        Limits {
            bytes: good.len() - 1,
            ..limits
        },
        Limits {
            data_rows: 1,
            ..limits
        },
    ] {
        assert_eq!(parse_columns(&good, 32, bounded).err(), Some(Error::Limit));
    }
    for count in [0, 33] {
        assert_eq!(
            parse_columns(&good, count, limits).err(),
            Some(Error::Schema)
        );
    }
    assert_eq!(
        catalog::parse_numeric(&numeric(31), Table::CombatRatings).err(),
        Some(Error::Header)
    );
    assert_eq!(
        catalog::parse_numeric(b"id\tv\n1\tNaN\n", Table::BarberShopCostBase).err(),
        Some(Error::Numeric)
    );
    assert_eq!(
        catalog::parse_numeric(b"id\tv\n1\t1\t2\n", Table::BarberShopCostBase).err(),
        Some(Error::Row)
    );
    assert_eq!(
        catalog::parse_numeric(b"id\tv\n1\t\xff", Table::BarberShopCostBase).err(),
        Some(Error::Utf8)
    );
    assert_eq!(
        catalog::parse_numeric(b"id\tv\n1\t1\0", Table::BarberShopCostBase).err(),
        Some(Error::Text)
    );
}

#[cfg(unix)]
#[test]
fn full_catalog_preserves_opaque_raw_bytes_and_verifies_all_twenty() {
    let lab = Lab::new();
    let report = extract(&lab, "complete20");
    assert!(report.complete);
    assert_eq!(report.files_published, 20);
    let root = lab.output("complete20");
    assert!(!root.join(manifest::MARKER).exists());
    assert_eq!(verify::verify(&root).code, Code::Verified);
    assert_eq!(verify::verify(&root).artifacts_verified, 20);
    let decoded = catalog_manifest::decode(&report.manifest).unwrap();
    for artifact in decoded.artifacts {
        assert_eq!(
            fs::read(root.join(artifact.table.relative_path())).unwrap(),
            bytes(artifact.table)
        );
        assert_eq!(
            artifact.facts.is_some(),
            artifact.table.value_columns().is_some()
        );
    }
    assert!(report.json().contains("\"scope\":\"pinned_gt_catalog\""));
    assert!(report.json().contains("\"typed_loaders_executed\":false"));
    assert!(!report.manifest.contains(lab.root.to_str().unwrap()));
    assert!(!report.manifest.contains("opaque"));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(&root).unwrap().permissions().mode() & 0o777,
            0o700
        );
        for table in Table::ALL {
            assert_eq!(
                fs::metadata(root.join(table.relative_path()))
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
fn catalog_missing_numeric_or_raw_is_partial_with_no_optional_success() {
    let lab = Lab::new();
    for missing in [Table::CombatRatings, Table::HonorLevel] {
        let root = lab.output(missing.name());
        let report = catalog_batch::extract(
            &lab.client,
            &root,
            70205,
            crate::Locale::EnUs,
            |table, _| {
                if table == missing {
                    Err(crate::Error::MissingFile)
                } else {
                    Ok((70205, bytes(table)))
                }
            },
        )
        .unwrap();
        assert!(!report.complete);
        assert_eq!(report.files_published, 19);
        assert!(catalog_manifest::decode(&report.manifest).is_ok());
        assert!(!root.join(missing.relative_path()).exists());
        assert!(
            fs::read_to_string(root.join(manifest::MARKER))
                .unwrap()
                .contains("pinned_gt_catalog")
        );
        assert_eq!(verify::verify(&root).code, Code::Incomplete);
        fs::remove_file(root.join(manifest::MARKER)).unwrap();
        assert_eq!(verify::verify(&root).code, Code::Incomplete);
    }
}

#[cfg(unix)]
#[test]
fn incomplete_acquisition_and_bad_numeric_values_never_publish_prefixes() {
    let lab = Lab::new();
    for (name, error) in [
        ("short", crate::Error::Incomplete),
        ("encrypted", crate::Error::Encrypted),
    ] {
        let root = lab.output(name);
        let report = catalog_batch::extract(
            &lab.client,
            &root,
            70205,
            crate::Locale::EnUs,
            |table, _| {
                if table == Table::SandboxScaling {
                    Err(error)
                } else {
                    Ok((70205, bytes(table)))
                }
            },
        )
        .unwrap();
        assert!(!report.complete);
        assert!(!root.join(Table::SandboxScaling.relative_path()).exists());
        assert_eq!(verify::verify(&root).code, Code::Incomplete);
    }
    let root = lab.output("badnumeric");
    let report = catalog_batch::extract(
        &lab.client,
        &root,
        70205,
        crate::Locale::EnUs,
        |table, _| {
            Ok((
                70205,
                if table == Table::BarberShopCostBase {
                    b"id\tv\n1\tNaN\n".to_vec()
                } else {
                    bytes(table)
                },
            ))
        },
    )
    .unwrap();
    assert!(!report.complete);
    assert!(
        !root
            .join(Table::BarberShopCostBase.relative_path())
            .exists()
    );
    assert_eq!(report.files_published, 19);
}

#[cfg(unix)]
#[test]
fn version_dispatch_preserves_v1_and_rejects_cross_version_manifests() {
    let lab = Lab::new();
    let old = lab.extract(Plan::All, "old6");
    let new = extract(&lab, "new20");
    assert_eq!(verify::verify(&lab.output("old6")).artifacts_verified, 6);
    assert!(manifest::decode(&old.manifest).is_ok());
    assert!(catalog_manifest::decode(&old.manifest).is_err());
    assert!(manifest::decode(&new.manifest).is_err());
    assert_eq!(manifest::version(&old.manifest), Ok(1));
    assert_eq!(manifest::version(&new.manifest), Ok(2));
    for version in [1, 3] {
        assert!(
            catalog_manifest::decode(&new.manifest.replacen(
                "\"manifest_version\":2",
                &format!("\"manifest_version\":{version}"),
                1
            ))
            .is_err()
        );
    }
    assert_eq!(
        fs::read_to_string(lab.output("old6").join(manifest::NAME)).unwrap(),
        old.manifest
    );
}

#[cfg(unix)]
#[test]
fn v2_closed_identity_mode_counts_and_claims_reject_mutations() {
    let lab = Lab::new();
    let report = extract(&lab, "schema");
    for (from, to) in [
        ("\"file_id\":1582086", "\"file_id\":1"),
        (
            "gt/ArtifactKnowledgeMultiplier.txt",
            "gt/artifactKnowledgeMultiplier.txt",
        ),
        (
            "gt/ArtifactKnowledgeMultiplier.txt",
            "../ArtifactKnowledgeMultiplier.txt",
        ),
        (
            "\"name\":\"ArtifactKnowledgeMultiplier\"",
            "\"name\":\"Unknown\"",
        ),
        ("\"validation_mode\":\"f32\"", "\"validation_mode\":\"raw\""),
        ("\"validation_mode\":\"raw\"", "\"validation_mode\":\"f32\""),
        ("\"value_columns\":32", "\"value_columns\":31"),
        ("\"value_columns\":null", "\"value_columns\":1"),
        ("\"data_rows\":null", "\"data_rows\":0"),
        ("\"max_files\":20", "\"max_files\":6"),
        (
            "\"max_batch_bytes\":83886080",
            "\"max_batch_bytes\":25165824",
        ),
        ("\"plan\":\"catalog\"", "\"plan\":\"all\""),
        (
            "\"scope\":\"pinned_gt_catalog\"",
            "\"scope\":\"explicit_gt_batch\"",
        ),
        ("\"offline\":true", "\"offline\":false"),
        (
            "\"typed_loaders_executed\":false",
            "\"typed_loaders_executed\":true",
        ),
        ("\"file_count\":20", "\"file_count\":19"),
        (
            "\"manifest_version\":2",
            "\"manifest_version\":2,\"manifest_version\":2",
        ),
        (
            "\"manifest_version\":2",
            "\"manifest_version\":2,\"unknown\":0",
        ),
        ("\"manifest_version\":2", "\"manifest_version\":2.0"),
        (
            "45fbc37f023eb54d71e1e47ab2fe252fabb139a5c923e25b1f9244399c29fec4",
            "0000000000000000000000000000000000000000000000000000000000000000",
        ),
    ] {
        assert!(report.manifest.contains(from), "missing mutation key");
        assert!(
            catalog_manifest::decode(&report.manifest.replacen(from, to, 1)).is_err(),
            "accepted mutation {from}"
        );
    }
    assert!(catalog_manifest::decode(&report.manifest.replace(',', ",\n ")).is_ok());
    assert!(catalog_manifest::decode(&" ".repeat(manifest::MAX_MANIFEST_BYTES + 1)).is_err());
    assert!(catalog_manifest::decode(&format!("{}0{}", "[".repeat(20), "]".repeat(20))).is_err());
}

#[cfg(unix)]
#[test]
fn catalog_manifest_cannot_reorder_or_omit_an_artifact() {
    let lab = Lab::new();
    let report = extract(&lab, "order");
    let mut decoded = catalog_manifest::decode(&report.manifest).unwrap();
    decoded.artifacts.swap(0, 1);
    assert!(catalog_manifest::decode(&decoded.render()).is_err());
    decoded.artifacts.pop();
    assert!(catalog_manifest::decode(&decoded.render()).is_err());
}

#[cfg(unix)]
#[test]
fn verifier_reparses_numeric_facts_and_handles_raw_only_tampering() {
    let lab = Lab::new();
    extract(&lab, "facts");
    rewrite(
        &lab.output("facts").join(manifest::NAME),
        "\"data_rows\":2,\"consumer_rows_including_zero\":3",
        "\"data_rows\":1,\"consumer_rows_including_zero\":2",
    );
    assert_eq!(verify::verify(&lab.output("facts")).code, Code::Numeric);
    for (name, table) in [
        ("raw", Table::HonorLevel),
        ("numeric", Table::CombatRatings),
    ] {
        extract(&lab, name);
        let path = lab.output(name).join(table.relative_path());
        let mut data = fs::read(&path).unwrap();
        data[0] ^= 1;
        fs::write(path, data).unwrap();
        assert_eq!(verify::verify(&lab.output(name)).code, Code::Inventory);
    }
}

#[cfg(unix)]
#[test]
fn catalog_verifier_rejects_missing_extra_case_and_pending_outputs() {
    let lab = Lab::new();
    for name in ["missing", "extra", "case", "pending", "rootextra"] {
        extract(&lab, name);
        let root = lab.output(name);
        let expected = match name {
            "missing" => {
                fs::remove_file(root.join(Table::HonorLevel.relative_path())).unwrap();
                Code::Inventory
            }
            "extra" => {
                fs::write(root.join("gt/other.txt"), b"x").unwrap();
                Code::Inventory
            }
            "case" => {
                fs::rename(root.join("gt/xp.txt"), root.join("gt/XP.txt")).unwrap();
                Code::Inventory
            }
            "pending" => {
                fs::write(root.join("gt/other.part"), b"x").unwrap();
                Code::Incomplete
            }
            _ => {
                fs::write(root.join("other"), b"x").unwrap();
                Code::Inventory
            }
        };
        assert_eq!(verify::verify(&root).code, expected);
    }
}

#[cfg(unix)]
#[test]
fn catalog_enforces_file_budget_for_both_numeric_and_opaque_entries() {
    let lab = Lab::new();
    assert_eq!(catalog::MAX_BATCH_BYTES, 80 * 1024 * 1024);
    for table in [Table::BarberShopCostBase, Table::HonorLevel] {
        let root = lab.output(table.name());
        let report = catalog_batch::extract(
            &lab.client,
            &root,
            70205,
            crate::Locale::EnUs,
            |entry, limit| {
                Ok((
                    70205,
                    if entry == table {
                        vec![b'x'; limit as usize + 1]
                    } else {
                        bytes(entry)
                    },
                ))
            },
        )
        .unwrap();
        assert!(!report.complete);
        assert_eq!(report.files_published, 19);
        assert!(!root.join(table.relative_path()).exists());
    }
}

#[cfg(unix)]
#[test]
fn catalog_admits_build_each_read_and_guards_final_metadata() {
    let lab = Lab::new();
    let root = lab.output("observed");
    let report = catalog_batch::extract(
        &lab.client,
        &root,
        70205,
        crate::Locale::EnUs,
        |table, _| Ok((70170, bytes(table))),
    )
    .unwrap();
    assert!(!report.complete);
    assert_eq!(report.files_published, 0);
    let root = lab.output("metadata");
    let result = catalog_batch::extract(
        &lab.client,
        &root,
        70205,
        crate::Locale::EnUs,
        |table, _| {
            if table == Table::Xp {
                fs::write(lab.client.join(".build.info"), metadata(70205, "esES")).unwrap();
            }
            Ok((70205, bytes(table)))
        },
    );
    assert_eq!(result.err(), Some("client_metadata_changed"));
    assert!(root.join(manifest::MARKER).exists());
    assert!(!root.join(manifest::NAME).exists());
}

#[cfg(unix)]
#[test]
fn catalog_output_guards_preserve_existing_and_reject_unsafe_paths() {
    let lab = Lab::new();
    let first = extract(&lab, "existing");
    assert_eq!(
        catalog_batch::extract(
            &lab.client,
            &lab.output("existing"),
            70205,
            crate::Locale::EnUs,
            |_, _| panic!()
        )
        .err(),
        Some("output_exists_or_unavailable")
    );
    assert_eq!(
        fs::read_to_string(lab.output("existing").join(manifest::NAME)).unwrap(),
        first.manifest
    );
    for path in [
        lab.client.join("out"),
        lab.root.clone(),
        lab.root.join("client/../out"),
    ] {
        assert_eq!(
            catalog_batch::extract(
                &lab.client,
                &path,
                70205,
                crate::Locale::EnUs,
                |_, _| panic!()
            )
            .err(),
            Some("unsafe_output_path")
        );
    }
    let git = lab.output("git");
    fs::create_dir(&git).unwrap();
    fs::write(git.join(".git"), b"synthetic").unwrap();
    assert_eq!(
        catalog_batch::extract(
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
fn catalog_symlink_artifacts_and_roots_are_rejected() {
    use std::os::unix::fs::symlink;
    let lab = Lab::new();
    extract(&lab, "ok");
    let root = lab.output("ok");
    let alias = lab.output("alias");
    symlink(&root, &alias).unwrap();
    assert_eq!(verify::verify(&alias).code, Code::UnsafePath);
    let file = root.join(Table::HonorLevel.relative_path());
    fs::remove_file(&file).unwrap();
    symlink(lab.client.join(".build.info"), file).unwrap();
    assert_eq!(verify::verify(&root).code, Code::UnsafePath);
}

#[test]
fn catalog_cli_is_opt_in_and_keeps_old_default_and_rejects_network() {
    let base = ["--client", "client", "--output", "out", "--locale", "esES"];
    let args = |parts: &[&str]| parts.iter().map(OsString::from).collect::<Vec<_>>();
    let options::Parsed::Run(old) = options::parse(&args(&base)).unwrap() else {
        panic!()
    };
    assert_eq!(old.selection, Selection::Typed(Plan::All));
    let mut full = base.to_vec();
    full.extend(["--group", "catalog"]);
    let options::Parsed::Run(new) = options::parse(&args(&full)).unwrap() else {
        panic!()
    };
    assert_eq!(new.selection, Selection::Catalog);
    for tail in [
        ["--group", "Catalog"],
        ["--group", "all20"],
        ["--allow-network", "true"],
        ["--plan", "catalog"],
    ] {
        let mut values = base.to_vec();
        values.extend(tail);
        assert!(options::parse(&args(&values)).is_err());
    }
}

#[cfg(unix)]
#[test]
fn verifier_rejects_rehashed_invalid_numeric_payload() {
    let lab = Lab::new();
    let report = extract(&lab, "rehashed");
    let root = lab.output("rehashed");
    let mut decoded = catalog_manifest::decode(&report.manifest).unwrap();
    let target = decoded
        .artifacts
        .iter_mut()
        .find(|artifact| artifact.table == Table::BarberShopCostBase)
        .unwrap();
    let raw = b"id\tv\n1\tNaN\n2\t0\n";
    target.bytes = Some(raw.len());
    target.sha256 = Some(crate::sha256::hex(raw));
    target.facts.as_mut().unwrap().bytes = raw.len();
    fs::write(root.join(target.table.relative_path()), raw).unwrap();
    let forged = decoded.render();
    assert!(catalog_manifest::decode(&forged).is_ok());
    fs::write(root.join(manifest::NAME), forged).unwrap();
    assert_eq!(verify::verify(&root).code, Code::Numeric);
}

#[cfg(unix)]
#[test]
fn catalog_unsupported_client_and_midstream_build_changes_fail_closed() {
    let lab = Lab::new();
    let root = lab.output("unsupported");
    assert_eq!(
        catalog_batch::extract(
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
    let root = lab.output("changed");
    let mut calls = 0;
    let result = catalog_batch::extract(
        &lab.client,
        &root,
        70205,
        crate::Locale::EnUs,
        |table, _| {
            calls += 1;
            fs::write(lab.client.join(".build.info"), metadata(70170, "enUS")).unwrap();
            Ok((70205, bytes(table)))
        },
    );
    assert_eq!(result.err(), Some("client_build_changed"));
    assert_eq!(calls, 1);
    assert!(root.join(manifest::MARKER).exists());
    assert!(!root.join(manifest::NAME).exists());
}
