// SPDX-License-Identifier: GPL-3.0-or-later
//! Authored complete synthetic GT vectors; no client assets or native reads.
use super::pipeline::{Lab, metadata};
use crate::{
    Error, Locale,
    acquisition::{OptionsBuilder, Profile},
    gametables::{
        batch::{self, Plan},
        catalog::Table,
        catalog_batch::{self, CatalogContract},
        catalog_manifest, manifest, options,
        verify::{self, Code},
    },
};
use std::{
    ffi::{OsStr, OsString},
    fs,
};

fn profile(cap: u32) -> Profile {
    let mut b = OptionsBuilder::default();
    for (k, v) in [
        ("--backend", "rust-local"),
        ("--index-layout", "contiguous"),
        ("--root-layout", "header-50893-v2"),
    ] {
        b.push(k, OsStr::new(v)).unwrap();
    }
    b.finish(cap).unwrap().profile().unwrap()
}
fn bytes(columns: Option<usize>) -> Vec<u8> {
    columns.map_or_else(
        || b"raw\0\xff".to_vec(),
        |n| {
            format!(
                "id\t{}\nx\t{}\n",
                vec!["v"; n].join("\t"),
                vec!["-0"; n].join("\t")
            )
            .into_bytes()
        },
    )
}
fn extract(lab: &Lab, name: &str, cap: u32) -> catalog_batch::Report {
    fs::write(lab.client.join(".build.info"), metadata(70338, "enUS")).unwrap();
    catalog_batch::extract_with_contract(
        CatalogContract::Build70338V1,
        &lab.client,
        &lab.output(name),
        70338,
        Locale::EnUs,
        profile(cap),
        |t, b| {
            assert!(b <= cap);
            Ok((70338, bytes(t.value_columns())))
        },
    )
    .unwrap()
}
#[test]
fn legacy_serializers_match_independently_authored_fixed_vectors() {
    let lab = Lab::new();
    for (p, expected) in [
        (
            None,
            include_str!("../../../../../tests/contracts/fixtures/forever-gt/legacy-v1-six.json"),
        ),
        (
            Some(profile(4096)),
            include_str!("../../../../../tests/contracts/fixtures/forever-gt/legacy-v3-six.json"),
        ),
    ] {
        let report = batch::extract_with_profile(
            Plan::All,
            &lab.client,
            &lab.output(if p.is_some() { "typed3" } else { "typed1" }),
            70205,
            Locale::EnUs,
            p,
            |t, _| Ok((70205, bytes(Some(t.value_columns())))),
        )
        .unwrap();
        assert_eq!(report.manifest, expected);
        assert_eq!(manifest::decode(expected).unwrap().render(), expected);
    }
    for (p, expected) in [
        (
            None,
            include_str!(
                "../../../../../tests/contracts/fixtures/forever-gt/legacy-v2-catalog.json"
            ),
        ),
        (
            Some(profile(4096)),
            include_str!(
                "../../../../../tests/contracts/fixtures/forever-gt/legacy-v3-catalog.json"
            ),
        ),
    ] {
        let report = catalog_batch::extract_with_profile(
            &lab.client,
            &lab.output(if p.is_some() { "catalog3" } else { "catalog2" }),
            70205,
            Locale::EnUs,
            p,
            |t, _| Ok((70205, bytes(t.value_columns()))),
        )
        .unwrap();
        assert_eq!(report.manifest, expected);
        assert_eq!(
            catalog_manifest::decode(expected).unwrap().render(),
            expected
        );
    }
}
#[test]
fn legacy_apis_reject_70338_before_any_path_or_callback_access() {
    let lab = Lab::new();
    let absent = lab.output("absent-client");
    let out = lab.output("out");
    for plan in [Plan::All, Plan::Initial, Plan::SpellValues] {
        for p in [None, Some(profile(4096))] {
            assert_eq!(
                batch::extract_with_profile(
                    plan,
                    &absent,
                    &out,
                    70338,
                    Locale::EnUs,
                    p,
                    |_, _| panic!("callback")
                )
                .err(),
                Some("unsupported_client")
            );
        }
    }
    for p in [None, Some(profile(4096))] {
        assert_eq!(
            catalog_batch::extract_with_profile(
                &absent,
                &out,
                70338,
                Locale::EnUs,
                p,
                |_, _| panic!("callback")
            )
            .err(),
            Some("unsupported_client")
        );
    }
    for build in [0, 70170, 70205, 70339] {
        assert_eq!(
            catalog_batch::extract_with_contract(
                CatalogContract::Build70338V1,
                &absent,
                &out,
                build,
                Locale::EnUs,
                profile(4096),
                |_, _| panic!("callback")
            )
            .err(),
            Some("unsupported_client")
        );
    }
    assert!(!out.exists());
}
#[test]
fn legacy_decoders_reject_70338_and_extra_profile_fields() {
    for text in [
        include_str!("../../../../../tests/contracts/fixtures/forever-gt/legacy-v1-six.json"),
        include_str!("../../../../../tests/contracts/fixtures/forever-gt/legacy-v3-six.json"),
        include_str!("../../../../../tests/contracts/fixtures/forever-gt/legacy-v2-catalog.json"),
        include_str!("../../../../../tests/contracts/fixtures/forever-gt/legacy-v3-catalog.json"),
    ] {
        for build in [70170, 70205] {
            let old = text.replace("\"build\":70205", &format!("\"build\":{build}"));
            assert!(manifest::decode(&old).is_ok() || catalog_manifest::decode(&old).is_ok());
        }
        for bad in [
            text.replace("\"build\":70205", "\"build\":70338"),
            text.replacen('{', "{\"component_profile\":\"gt-catalog-70338-v1\",", 1),
        ] {
            assert!(manifest::decode(&bad).is_err());
            assert!(catalog_manifest::decode(&bad).is_err());
        }
    }
}
#[test]
fn explicit_v4_matches_fixed_catalog_vector_and_verifies_all_twenty() {
    let lab = Lab::new();
    let report = extract(&lab, "v4", 4096);
    let expected =
        include_str!("../../../../../tests/contracts/fixtures/forever-gt/v4-70338-catalog.json");
    assert_eq!(report.manifest, expected);
    assert_eq!(report.contract, CatalogContract::Build70338V1);
    assert_eq!(
        catalog_manifest::decode(expected).unwrap().render(),
        expected
    );
    assert!(manifest::decode(expected).is_err());
    assert!(
        report
            .json()
            .contains("\"report_version\":4,\"component_profile\":\"gt-catalog-70338-v1\"")
    );
    let v = verify::verify(&lab.output("v4"));
    assert_eq!(v.code, Code::Verified);
    assert_eq!(v.artifacts_verified, 20);
    assert!(v.json().contains("\"report_version\":4"));
    for t in Table::ALL {
        assert_eq!(
            fs::read(lab.output("v4").join(t.relative_path())).unwrap(),
            bytes(t.value_columns())
        );
    }
}
#[test]
fn v4_rejects_version_build_profile_backend_and_limit_mutations() {
    let text =
        include_str!("../../../../../tests/contracts/fixtures/forever-gt/v4-70338-catalog.json");
    for (from, to) in [
        ("\"manifest_version\":4", "\"manifest_version\":3"),
        ("\"manifest_version\":4", "\"manifest_version\":2"),
        ("\"build\":70338", "\"build\":70205"),
        ("gt-catalog-70338-v1", "gt-catalog-70338-v2"),
        ("\"backend\":\"rust-local\"", "\"backend\":\"native\""),
        ("\"plan\":\"catalog\"", "\"plan\":\"all\""),
        ("\"file_bytes\":4096", "\"file_bytes\":1"),
        ("\"file_bytes\":4096", "\"file_bytes\":4194305"),
        (
            "\"manifest_version\":4",
            "\"manifest_version\":4,\"unexpected\":0",
        ),
        ("\"component_profile\":\"gt-catalog-70338-v1\",", ""),
        (
            "\"component_profile\":\"gt-catalog-70338-v1\"",
            "\"component_profile\":\"gt-catalog-70338-v1\",\"component_profile\":\"gt-catalog-70338-v1\"",
        ),
    ] {
        assert!(text.contains(from));
        let bad = text.replacen(from, to, 1);
        assert!(catalog_manifest::decode(&bad).is_err(), "{from}");
    }
    let no_acq = text.replace(&format!("\"acquisition\":{},", profile(4096).render()), "");
    assert!(catalog_manifest::decode(&no_acq).is_err());
}
#[test]
fn v4_missing_numeric_or_raw_and_small_caps_never_accept_prefix() {
    for missing in [Table::BaseMp, Table::BaseProfessionRatings] {
        let lab = Lab::new();
        fs::write(lab.client.join(".build.info"), metadata(70338, "enUS")).unwrap();
        let r = catalog_batch::extract_with_contract(
            CatalogContract::Build70338V1,
            &lab.client,
            &lab.output("out"),
            70338,
            Locale::EnUs,
            profile(4096),
            |t, _| {
                if t == missing {
                    Err(Error::Input)
                } else {
                    Ok((70338, bytes(t.value_columns())))
                }
            },
        )
        .unwrap();
        assert!(!r.complete);
        assert!(lab.output("out").join(manifest::MARKER).exists());
        assert_eq!(verify::verify(&lab.output("out")).code, Code::Incomplete);
        fs::remove_file(lab.output("out").join(manifest::MARKER)).unwrap();
        assert_eq!(verify::verify(&lab.output("out")).code, Code::Incomplete);
    }
    let lab = Lab::new();
    assert!(!extract(&lab, "small", 1).complete);
}
#[test]
fn v4_source_build_change_is_fatal_and_stops_callback_sequence() {
    for changed in [false, true] {
        let lab = Lab::new();
        fs::write(lab.client.join(".build.info"), metadata(70338, "enUS")).unwrap();
        let mut calls = 0;
        let r = catalog_batch::extract_with_contract(
            CatalogContract::Build70338V1,
            &lab.client,
            &lab.output("out"),
            70338,
            Locale::EnUs,
            profile(4096),
            |t, _| {
                calls += 1;
                if calls == 2 {
                    if changed {
                        Err(Error::LocalStore(crate::local_store::Error::InputChanged))
                    } else {
                        Ok((70205, bytes(t.value_columns())))
                    }
                } else {
                    Ok((70338, bytes(t.value_columns())))
                }
            },
        );
        assert!(r.is_err());
        assert_eq!(calls, 2);
        assert!(lab.output("out").join(manifest::MARKER).exists());
        assert!(!lab.output("out").join(manifest::NAME).exists());
    }
}
#[test]
fn v4_offline_inventory_and_numeric_tampering_are_rejected() {
    for (name, data) in [
        ("BaseMp.txt", b"invalid".as_slice()),
        ("BaseProfessionRatings.txt", b"changed".as_slice()),
    ] {
        let lab = Lab::new();
        extract(&lab, "out", 4096);
        fs::write(lab.output("out").join("gt").join(name), data).unwrap();
        assert_eq!(verify::verify(&lab.output("out")).code, Code::Inventory);
    }
    let lab = Lab::new();
    extract(&lab, "out", 4096);
    fs::write(lab.output("out").join("gt/extra.txt"), b"extra").unwrap();
    assert_eq!(verify::verify(&lab.output("out")).code, Code::Inventory);
}
#[test]
fn explicit_profile_cli_combinations_and_defaults_are_closed() {
    let base = ["--client", "absent", "--output", "out", "--locale", "enUS"];
    let args = |tail: &[&str]| {
        base.iter()
            .chain(tail)
            .map(OsString::from)
            .collect::<Vec<_>>()
    };
    let valid = [
        "--group",
        "catalog",
        "--backend",
        "rust-local",
        "--index-layout",
        "contiguous",
        "--root-layout",
        "header-50893-v2",
        "--profile",
        "gt-catalog-70338-v1",
    ];
    let options::Parsed::Run(o) = options::parse(&args(&valid)).unwrap() else {
        panic!()
    };
    assert_eq!(o.contract, CatalogContract::Build70338V1);
    let options::Parsed::Run(o) = options::parse(&args(&[])).unwrap() else {
        panic!()
    };
    assert_eq!(o.contract, CatalogContract::Legacy);
    assert_eq!(o.selection, options::Selection::Typed(Plan::All));
    assert!(!o.acquisition.is_rust_local());
    for (from, to) in [
        ("catalog", "all"),
        ("catalog", "initial"),
        ("gt-catalog-70338-v1", "auto"),
        ("rust-local", "native"),
        ("contiguous", "auto"),
    ] {
        let bad = valid.map(|s| if s == from { to } else { s });
        assert!(options::parse(&args(&bad)).is_err());
    }
    for flag in ["--group", "--backend", "--index-layout", "--root-layout"] {
        let bad = valid
            .as_chunks::<2>()
            .0
            .iter()
            .filter(|p| p[0] != flag)
            .flat_map(|p| p.iter().copied())
            .collect::<Vec<_>>();
        assert!(options::parse(&args(&bad)).is_err());
    }
    let mut duplicate = valid.to_vec();
    duplicate.extend(["--profile", CatalogContract::PROFILE]);
    assert!(options::parse(&args(&duplicate)).is_err());
}
