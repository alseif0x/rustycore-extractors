// SPDX-License-Identifier: GPL-3.0-or-later
//! Synthetic acquisition provenance; no native storage or real client bytes.
#[cfg(unix)]
use super::{fixture, pipeline::Lab};
use crate::gametables::options;
#[cfg(unix)]
use crate::{
    Locale,
    acquisition::{OptionsBuilder, Profile},
    gametables::{
        GameTable, MAX_FILE_BYTES,
        batch::{self, Plan},
        catalog::{self, Table},
        catalog_batch, catalog_manifest, manifest,
        verify::{self, Code},
    },
};
use std::ffi::OsString;
#[cfg(unix)]
use std::{ffi::OsStr, fs};
#[cfg(unix)]
fn profile(cap: u32) -> Profile {
    let mut builder = OptionsBuilder::default();
    for (key, value) in [
        ("--backend", "rust-local"),
        ("--index-layout", "contiguous"),
        ("--root-layout", "header-50893-v2"),
    ] {
        assert!(builder.push(key, OsStr::new(value)).unwrap());
    }
    builder.finish(cap).unwrap().profile().unwrap()
}
#[cfg(unix)]
fn typed(table: GameTable) -> Vec<u8> {
    fixture(table, &["x"], &vec!["-0"; table.value_columns()], "\r\n")
}
#[cfg(unix)]
fn catalog_bytes(table: Table) -> Vec<u8> {
    table.value_columns().map_or_else(
        || b"raw\0\xff".to_vec(),
        |columns| {
            format!(
                "id\t{}\nx\t{}\n",
                vec!["v"; columns].join("\t"),
                vec!["-0"; columns].join("\t")
            )
            .into_bytes()
        },
    )
}
#[cfg(unix)]
#[test]
fn rust_profiles_round_trip_all_typed_plans_without_changing_native_bytes() {
    let lab = Lab::new();
    let p = profile(4096);
    for plan in [Plan::All, Plan::Initial, Plan::SpellValues] {
        let native = batch::extract(
            plan,
            &lab.client,
            &lab.output(&format!("native-{}", plan.name())),
            70205,
            Locale::EnUs,
            |t, _| Ok((70205, typed(t))),
        )
        .unwrap();
        let root = lab.output(plan.name());
        let report = batch::extract_with_profile(
            plan,
            &lab.client,
            &root,
            70205,
            Locale::EnUs,
            Some(p),
            |t, budget| {
                assert_eq!(budget, 4096);
                Ok((70205, typed(t)))
            },
        )
        .unwrap();
        assert!(report.complete);
        assert!(report.manifest.contains("\"manifest_version\":3"));
        assert_eq!(report.acquisition, Some(p));
        let mut decoded = manifest::decode(&report.manifest).unwrap();
        assert_eq!(decoded.acquisition.take(), Some(p));
        assert_eq!(decoded.render(), native.manifest);
        let verified = verify::verify(&root);
        assert_eq!(verified.code, Code::Verified);
        assert_eq!(verified.acquisition, Some(p));
        assert!(verified.json().contains("\"complete_ckey_md5\":true"));
        for table in plan.tables() {
            assert_eq!(
                fs::read(root.join(table.relative_path())).unwrap(),
                typed(table)
            );
        }
        assert!(!report.manifest.contains(lab.root.to_str().unwrap()));
    }
}
#[cfg(unix)]
#[test]
fn rust_catalog_is_exact_twenty_and_keeps_raw_only_entries() {
    let lab = Lab::new();
    let root = lab.output("catalog-rust");
    let p = profile(4096);
    let report = catalog_batch::extract_with_profile(
        &lab.client,
        &root,
        70205,
        Locale::EnUs,
        Some(p),
        |t, b| {
            assert_eq!(b, 4096);
            Ok((70205, catalog_bytes(t)))
        },
    )
    .unwrap();
    assert!(report.complete);
    let mut decoded = catalog_manifest::decode(&report.manifest).unwrap();
    assert_eq!(decoded.acquisition.take(), Some(p));
    assert!(
        catalog_manifest::decode(&decoded.render())
            .unwrap()
            .acquisition
            .is_none()
    );
    assert!(manifest::decode(&report.manifest).is_err());
    assert_eq!(verify::verify(&root).artifacts_verified, catalog::MAX_FILES);
    for t in Table::ALL {
        assert_eq!(
            fs::read(root.join(t.relative_path())).unwrap(),
            catalog_bytes(t)
        );
    }
    assert!(report.json().contains("\"typed_loaders_executed\":false"));
}
#[cfg(unix)]
#[test]
fn provenance_is_mandatory_exact_and_bound_to_each_file_limit() {
    let lab = Lab::new();
    let p = profile(4096);
    let typed_report = batch::extract_with_profile(
        Plan::Initial,
        &lab.client,
        &lab.output("typed"),
        70205,
        Locale::EnUs,
        Some(p),
        |t, _| Ok((70205, typed(t))),
    )
    .unwrap();
    let catalog_report = catalog_batch::extract_with_profile(
        &lab.client,
        &lab.output("catalog"),
        70205,
        Locale::EnUs,
        Some(p),
        |t, _| Ok((70205, catalog_bytes(t))),
    )
    .unwrap();
    for text in [&typed_report.manifest, &catalog_report.manifest] {
        for (from, to) in [
            ("\"backend\":\"rust-local\"", "\"backend\":\"native\""),
            (
                "\"ekey_validation\":\"prefix9\"",
                "\"ekey_validation\":\"full16\"",
            ),
            ("\"complete_ckey_md5\":true", "\"complete_ckey_md5\":false"),
            ("\"file_bytes\":4096", "\"file_bytes\":1"),
            ("\"file_bytes\":4096", "\"file_bytes\":4194305"),
            ("\"file_bytes\":4096", "\"file_bytes\":0"),
            ("\"file_bytes\":4096", "\"file_bytes\":4096,\"unknown\":0"),
            (
                "\"file_bytes\":4096",
                "\"file_bytes\":4096,\"file_bytes\":4096",
            ),
            (
                "\"root_layout\":\"header-50893-v2\"",
                "\"root_layout\":\"auto\"",
            ),
            ("\"manifest_version\":3", "\"manifest_version\":4"),
        ] {
            assert!(text.contains(from));
            let bad = text.replacen(from, to, 1);
            assert!(manifest::decode(&bad).is_err());
            assert!(catalog_manifest::decode(&bad).is_err());
        }
        let no_profile = text.replace(&format!(",\"acquisition\":{}", p.render()), "");
        assert!(manifest::decode(&no_profile).is_err());
        assert!(catalog_manifest::decode(&no_profile).is_err());
    }
    for old in [1, 2] {
        let bad = typed_report.manifest.replace(
            "\"manifest_version\":3",
            &format!("\"manifest_version\":{old}"),
        );
        assert!(manifest::decode(&bad).is_err());
        assert!(catalog_manifest::decode(&bad).is_err());
    }
}
#[cfg(unix)]
#[test]
fn rust_input_change_is_global_fatal_and_stops_both_plans() {
    let lab = Lab::new();
    for catalog in [false, true] {
        let root = lab.output(if catalog { "catalog" } else { "typed" });
        let mut calls = 0;
        let mut load = |bytes| {
            calls += 1;
            if calls == 2 {
                Err(crate::Error::LocalStore(
                    crate::local_store::Error::InputChanged,
                ))
            } else {
                Ok((70205, bytes))
            }
        };
        let result = if catalog {
            catalog_batch::extract_with_profile(
                &lab.client,
                &root,
                70205,
                Locale::EnUs,
                Some(profile(4096)),
                |t, _| load(catalog_bytes(t)),
            )
            .map(|_| ())
        } else {
            batch::extract_with_profile(
                Plan::All,
                &lab.client,
                &root,
                70205,
                Locale::EnUs,
                Some(profile(4096)),
                |t, _| load(typed(t)),
            )
            .map(|_| ())
        };
        assert_eq!(calls, 2);
        assert_eq!(
            result,
            Err(crate::Error::LocalStore(crate::local_store::Error::InputChanged).code())
        );
        assert!(root.join(manifest::MARKER).is_file());
        assert!(!root.join(manifest::NAME).exists());
        assert_eq!(fs::read_dir(root.join("gt")).unwrap().count(), 1);
        assert_eq!(verify::verify(&root).code, Code::Incomplete);
    }
}
#[cfg(unix)]
#[test]
fn reduced_rust_budget_is_enforced_even_for_an_injected_reader() {
    let lab = Lab::new();
    for catalog in [false, true] {
        let root = lab.output(if catalog { "catalog" } else { "typed" });
        let mut calls = 0;
        let mut load = |budget| {
            calls += 1;
            assert_eq!(budget, 8);
            Ok((70205, vec![0; 9]))
        };
        let complete = if catalog {
            catalog_batch::extract_with_profile(
                &lab.client,
                &root,
                70205,
                Locale::EnUs,
                Some(profile(8)),
                |_, b| load(b),
            )
            .unwrap()
            .complete
        } else {
            batch::extract_with_profile(
                Plan::All,
                &lab.client,
                &root,
                70205,
                Locale::EnUs,
                Some(profile(8)),
                |_, b| load(b),
            )
            .unwrap()
            .complete
        };
        assert!(!complete);
        assert_eq!(calls, if catalog { 20 } else { 6 });
        assert_eq!(fs::read_dir(root.join("gt")).unwrap().count(), 0);
        assert_eq!(verify::verify(&root).code, Code::Incomplete);
    }
    let root = lab.output("badcap");
    assert_eq!(
        batch::extract_with_profile(
            Plan::All,
            &lab.client,
            &root,
            70205,
            Locale::EnUs,
            Some(profile(MAX_FILE_BYTES as u32 + 1)),
            |_, _| panic!()
        )
        .err(),
        Some("gt_policy_limit")
    );
    assert!(!root.exists());
}
#[test]
fn gt_cli_requires_explicit_closed_rust_profile_and_native_rejects_extras() {
    let base = ["--client", "private", "--output", "out", "--locale", "esES"];
    let args = |tail: &[&str]| {
        base.iter()
            .chain(tail)
            .map(OsString::from)
            .collect::<Vec<_>>()
    };
    let options::Parsed::Run(native) = options::parse(&args(&[])).unwrap() else {
        panic!()
    };
    assert!(!native.acquisition.is_rust_local());
    for index in ["contiguous", "paged"] {
        for root in [
            "headerless-18125",
            "header-30080",
            "header-50893-v1",
            "header-50893-v2",
        ] {
            let options::Parsed::Run(parsed) = options::parse(&args(&[
                "--backend",
                "rust-local",
                "--index-layout",
                index,
                "--root-layout",
                root,
                "--max-file-bytes",
                "4096",
            ]))
            .unwrap() else {
                panic!()
            };
            assert_eq!(parsed.acquisition.profile().unwrap().max_file_bytes(), 4096);
        }
    }
    for tail in [
        vec!["--backend", "rust-local"],
        vec!["--index-layout", "contiguous"],
        vec!["--max-file-bytes", "1"],
        vec!["--backend", "native", "--root-layout", "header-30080"],
        vec![
            "--backend",
            "rust-local",
            "--index-layout",
            "contiguous",
            "--root-layout",
            "header-50893-v2",
            "--max-file-bytes",
            "4194305",
        ],
        vec!["--backend", "rust-local", "--backend", "rust-local"],
    ] {
        assert!(options::parse(&args(&tail)).is_err());
    }
}
