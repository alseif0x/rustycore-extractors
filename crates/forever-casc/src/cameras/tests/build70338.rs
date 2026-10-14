// SPDX-License-Identifier: GPL-3.0-or-later
//! Authored target camera records; no real source, IDs or M2 assets.
use super::*;
use crate::acquisition::{OptionsBuilder, Profile};
use std::ffi::OsStr;
fn profile(cap: u32) -> Profile {
    let mut b = OptionsBuilder::default();
    for (k, v) in [
        ("--backend", "rust-local"),
        ("--index-layout", "paged"),
        ("--root-layout", "header-50893-v2"),
    ] {
        b.push(k, OsStr::new(v)).unwrap();
    }
    b.finish(cap).unwrap().profile().unwrap()
}
fn target(ids: &[u32], copies: &[(u32, u32)]) -> Vec<u8> {
    let mut bytes = source(ids, copies);
    put32(&mut bytes, 152, CameraContract::TABLE_HASH);
    bytes
}
fn extract(lab: &Lab, name: &str) -> batch::Report {
    fs::write(lab.client.join(".build.info"), metadata(70338, "enUS")).unwrap();
    batch::extract_with_contract(
        CameraContract::Build70338V1,
        &lab.client,
        &lab.output(name),
        70338,
        Locale::EnUs,
        profile(4096),
        |id, b| {
            assert_eq!(b, 4096);
            Ok((
                70338,
                if id == SOURCE_FILE_ID {
                    target(&[12, 11, 12], &[(2, 1)])
                } else {
                    raw(id)
                },
            ))
        },
    )
    .unwrap()
}
#[test]
fn target_descriptor_checks_hash_and_retains_direct_zero_before_copies() {
    let bytes = target(&[22, 11, 0, 22], &[(2, 1), (5, 2)]);
    let p = discover_with_contract(CameraContract::Build70338V1, &bytes).unwrap();
    assert_eq!(p.file_ids(), &[0, 11, 22].into_iter().collect());
    assert_eq!(p.facts().direct_records, 4);
    let legacy = source(&[11], &[]);
    assert!(discover(&legacy).is_ok());
    assert!(discover_with_contract(CameraContract::Build70338V1, &legacy).is_err());
    let mut bad = bytes.clone();
    put32(&mut bad, 152, 0xC2CEFDF8);
    assert!(discover_with_contract(CameraContract::Build70338V1, &bad).is_err());
    put32(&mut bad, 152, CameraContract::TABLE_HASH);
    put32(&mut bad, 156, 0);
    assert!(discover_with_contract(CameraContract::Build70338V1, &bad).is_err());
}
#[test]
fn target_projection_waits_for_tail_copies_and_set_limits() {
    let mut trailing = target(&[11], &[]);
    trailing.push(0);
    assert!(discover_with_contract(CameraContract::Build70338V1, &trailing).is_err());
    assert!(
        discover_with_contract(CameraContract::Build70338V1, &target(&[11], &[(1, 1)])).is_err()
    );
    let ids = (1..=MAX_FILES as u32).collect::<Vec<_>>();
    assert_eq!(
        discover_with_contract(CameraContract::Build70338V1, &target(&ids, &[]))
            .unwrap()
            .file_ids()
            .len(),
        MAX_FILES
    );
    let ids = (1..=MAX_FILES as u32 + 1).collect::<Vec<_>>();
    assert_eq!(
        discover_with_contract(CameraContract::Build70338V1, &target(&ids, &[])).err(),
        Some("camera_policy_limit")
    );
}
#[test]
fn legacy_renderers_match_independently_authored_vectors() {
    let lab = Lab::new();
    let native = lab.extract("native");
    assert_eq!(
        native.manifest,
        include_str!("../../../../../tests/contracts/fixtures/forever-camera/legacy-v1.json")
    );
    let r = batch::extract_with_profile(
        &lab.client,
        &lab.output("rust"),
        70205,
        Locale::EnUs,
        Some(profile(4096)),
        |id, _| {
            Ok((
                70205,
                if id == SOURCE_FILE_ID {
                    source(&[12, 11, 12], &[(2, 1)])
                } else {
                    raw(id)
                },
            ))
        },
    )
    .unwrap();
    assert_eq!(
        r.manifest,
        include_str!("../../../../../tests/contracts/fixtures/forever-camera/legacy-v2.json")
    );
    for text in [&native.manifest, &r.manifest] {
        assert_eq!(manifest::decode(text).unwrap().render().unwrap(), *text);
        for build in [70170, 70205] {
            assert!(
                manifest::decode(&text.replace("\"build\":70205", &format!("\"build\":{build}")))
                    .is_ok()
            );
        }
        assert!(manifest::decode(&text.replace("\"build\":70205", "\"build\":70338")).is_err());
    }
}
#[test]
fn old_apis_and_new_wrong_builds_reject_before_paths_and_callbacks() {
    let lab = Lab::new();
    let absent = lab.output("absent-client");
    let out = lab.output("out");
    for p in [None, Some(profile(4096))] {
        assert_eq!(
            batch::extract_with_profile(&absent, &out, 70338, Locale::EnUs, p, |_, _| panic!(
                "callback"
            ))
            .err(),
            Some("unsupported_client")
        );
    }
    for build in [0, 70170, 70205, 70339] {
        assert_eq!(
            batch::extract_with_contract(
                CameraContract::Build70338V1,
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
fn v3_vector_provenance_and_exact_opaque_outputs_verify() {
    let lab = Lab::new();
    let r = extract(&lab, "out");
    assert!(r.complete);
    assert_eq!(
        r.manifest,
        include_str!("../../../../../tests/contracts/fixtures/forever-camera/v3-70338.json")
    );
    assert!(!r.manifest.contains("db2_metadata_sha256"));
    assert!(r.manifest.contains("\"schema_catalog\":"));
    assert_eq!(
        manifest::decode(&r.manifest).unwrap().render().unwrap(),
        r.manifest
    );
    let checked = verify::verify(&lab.output("out"));
    assert_eq!(checked.code, Code::Verified);
    assert_eq!(checked.cameras_verified, 2);
    assert!(
        checked
            .json()
            .contains("\"report_version\":3,\"component_profile\":\"camera-raw-70338-v1\"")
    );
    assert!(r.json().contains("\"report_version\":3"));
    for id in [11, 12] {
        assert_eq!(
            fs::read(lab.output("out").join(relative_path(id))).unwrap(),
            raw(id)
        );
    }
}
#[test]
fn v3_rejects_profile_version_catalog_semantics_and_acquisition_mutations() {
    let text = include_str!("../../../../../tests/contracts/fixtures/forever-camera/v3-70338.json");
    for (from, to) in [
        ("\"manifest_version\":3", "\"manifest_version\":2"),
        ("\"manifest_version\":3", "\"manifest_version\":1"),
        ("\"build\":70338", "\"build\":70205"),
        ("camera-raw-70338-v1", "auto"),
        ("wowdbdefs-70338-v1", "wowdbdefs-70205-v1"),
        ("\"physical_field\":3", "\"physical_field\":4"),
        ("\"decoded_scalar_count\":7", "\"decoded_scalar_count\":5"),
        ("\"external_id\":true", "\"external_id\":false"),
        (
            CameraContract::DEFINITION_BLOB,
            "0000000000000000000000000000000000000000",
        ),
        (
            CameraContract::DEFINITION_SHA256,
            "0000000000000000000000000000000000000000000000000000000000000000",
        ),
        ("\"backend\":\"rust-local\"", "\"backend\":\"native\""),
        ("\"file_bytes\":4096", "\"file_bytes\":1"),
        ("\"file_bytes\":4096", "\"file_bytes\":16777217"),
        (
            "\"manifest_version\":3",
            "\"manifest_version\":3,\"unknown\":0",
        ),
        ("\"component_profile\":\"camera-raw-70338-v1\",", ""),
    ] {
        assert!(text.contains(from));
        assert!(
            manifest::decode(&text.replacen(from, to, 1)).is_err(),
            "{from}"
        );
    }
    let no_acq = text.replace(&format!("\"acquisition\":{},", profile(4096).render()), "");
    assert!(manifest::decode(&no_acq).is_err());
    let extra = text.replacen(
        "\"schema_catalog\":{",
        "\"schema_catalog\":{\"unknown\":0,",
        1,
    );
    assert!(manifest::decode(&extra).is_err());
    let mut value = crate::verify::json::parse(&contract::catalog_json()).unwrap();
    let crate::verify::json::Json::Object(fields) = &mut value else {
        panic!()
    };
    fields.reverse();
    assert!(contract::validate_catalog(&value).is_ok());
}
#[test]
fn v3_verifier_rediscovery_uses_target_hash_even_for_coherent_manifest() {
    let lab = Lab::new();
    let r = extract(&lab, "out");
    let mut m = manifest::decode(&r.manifest).unwrap();
    let wrong = source(&[12, 11, 12], &[(2, 1)]);
    m.source.file = manifest::FileOutcome::complete(&wrong);
    fs::write(lab.output("out").join(source_path(Locale::EnUs)), &wrong).unwrap();
    fs::write(lab.output("out").join(manifest::NAME), m.render().unwrap()).unwrap();
    assert_eq!(verify::verify(&lab.output("out")).code, Code::Discovery);
}
#[test]
fn v3_empty_valid_source_is_complete_but_missing_source_or_camera_is_not() {
    for missing in [SOURCE_FILE_ID, 11] {
        let lab = Lab::new();
        fs::write(lab.client.join(".build.info"), metadata(70338, "enUS")).unwrap();
        let r = batch::extract_with_contract(
            CameraContract::Build70338V1,
            &lab.client,
            &lab.output("out"),
            70338,
            Locale::EnUs,
            profile(4096),
            |id, _| {
                if id == missing {
                    Err(Error::Input)
                } else {
                    Ok((
                        70338,
                        if id == SOURCE_FILE_ID {
                            target(&[11], &[])
                        } else {
                            raw(id)
                        },
                    ))
                }
            },
        )
        .unwrap();
        assert!(!r.complete);
        assert_eq!(verify::verify(&lab.output("out")).code, Code::Incomplete);
        fs::remove_file(lab.output("out").join(manifest::MARKER)).unwrap();
        assert_eq!(verify::verify(&lab.output("out")).code, Code::Incomplete);
    }
    let lab = Lab::new();
    fs::write(lab.client.join(".build.info"), metadata(70338, "enUS")).unwrap();
    let mut calls = 0;
    let r = batch::extract_with_contract(
        CameraContract::Build70338V1,
        &lab.client,
        &lab.output("out"),
        70338,
        Locale::EnUs,
        profile(4096),
        |id, _| {
            calls += 1;
            assert_eq!(id, SOURCE_FILE_ID);
            Ok((70338, target(&[], &[])))
        },
    )
    .unwrap();
    assert_eq!(calls, 1);
    assert!(r.complete);
    assert_eq!(r.files_published, 1);
    assert_eq!(verify::verify(&lab.output("out")).code, Code::Verified);
}
#[test]
fn v3_input_or_observed_build_change_stops_reads_and_keeps_marker() {
    for changed in [false, true] {
        let lab = Lab::new();
        fs::write(lab.client.join(".build.info"), metadata(70338, "enUS")).unwrap();
        let mut calls = 0;
        let r = batch::extract_with_contract(
            CameraContract::Build70338V1,
            &lab.client,
            &lab.output("out"),
            70338,
            Locale::EnUs,
            profile(4096),
            |id, _| {
                calls += 1;
                if calls == 2 {
                    if changed {
                        Err(Error::LocalStore(crate::local_store::Error::InputChanged))
                    } else {
                        Ok((70205, raw(id)))
                    }
                } else {
                    Ok((70338, target(&[11, 12], &[])))
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
fn v3_downward_source_budget_and_descriptor_total_limits_remain() {
    let lab = Lab::new();
    fs::write(lab.client.join(".build.info"), metadata(70338, "enUS")).unwrap();
    let mut calls = 0;
    let r = batch::extract_with_contract(
        CameraContract::Build70338V1,
        &lab.client,
        &lab.output("out"),
        70338,
        Locale::EnUs,
        profile(1),
        |id, cap| {
            calls += 1;
            assert_eq!(id, SOURCE_FILE_ID);
            assert_eq!(cap, 1);
            Ok((70338, target(&[11], &[])))
        },
    )
    .unwrap();
    assert_eq!(calls, 1);
    assert!(!r.complete);
    assert_eq!(r.files_published, 0);
    let mut m = manifest::decode(include_str!(
        "../../../../../tests/contracts/fixtures/forever-camera/v3-70338.json"
    ))
    .unwrap();
    m.cameras = (1..=20)
        .map(|id| {
            let mut file = manifest::FileOutcome::complete(&[]);
            file.bytes = Some(MAX_FILE_BYTES);
            manifest::Camera { file_id: id, file }
        })
        .collect();
    assert_eq!(m.render().err(), Some("camera_policy_limit"));
    assert_eq!(batch::remaining_budget(MAX_TOTAL_BYTES - 1).unwrap(), 1);
}
#[test]
fn camera_profile_options_are_explicit_and_keep_native_default() {
    let base = ["--client", "absent", "--locale", "enUS", "--output", "out"];
    let args = |tail: &[&str]| {
        base.iter()
            .chain(tail)
            .map(OsString::from)
            .collect::<Vec<_>>()
    };
    let valid = [
        "--backend",
        "rust-local",
        "--index-layout",
        "paged",
        "--root-layout",
        "header-50893-v2",
        "--profile",
        "camera-raw-70338-v1",
    ];
    let options::Parsed::Run(o) = options::parse(&args(&valid)).unwrap() else {
        panic!()
    };
    assert_eq!(o.contract, CameraContract::Build70338V1);
    let options::Parsed::Run(o) = options::parse(&args(&[])).unwrap() else {
        panic!()
    };
    assert_eq!(o.contract, CameraContract::Legacy);
    assert!(!o.acquisition.is_rust_local());
    for flag in ["--backend", "--index-layout", "--root-layout"] {
        let bad = valid
            .as_chunks::<2>()
            .0
            .iter()
            .filter(|p| p[0] != flag)
            .flat_map(|p| p.iter().copied())
            .collect::<Vec<_>>();
        assert!(options::parse(&args(&bad)).is_err());
    }
    for (from, to) in [("camera-raw-70338-v1", "auto"), ("rust-local", "native")] {
        let bad = valid.map(|v| if v == from { to } else { v });
        assert!(options::parse(&args(&bad)).is_err());
    }
    let mut bad = valid.to_vec();
    bad.extend(["--profile", CameraContract::PROFILE]);
    assert!(options::parse(&args(&bad)).is_err());
}
