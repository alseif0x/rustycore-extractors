// SPDX-License-Identifier: GPL-3.0-or-later
//! Synthetic raw camera provenance and global input-change behavior.
use super::*;
#[cfg(unix)]
use crate::acquisition::{OptionsBuilder, Profile};
#[cfg(unix)]
use std::ffi::OsStr;
#[cfg(unix)]
fn profile(cap: u32) -> Profile {
    let mut b = OptionsBuilder::default();
    for (key, value) in [
        ("--backend", "rust-local"),
        ("--index-layout", "paged"),
        ("--root-layout", "header-50893-v2"),
    ] {
        assert!(b.push(key, OsStr::new(value)).unwrap());
    }
    b.finish(cap).unwrap().profile().unwrap()
}
#[cfg(unix)]
fn bytes(id: u32) -> Vec<u8> {
    if id == SOURCE_FILE_ID {
        source(&[12, 11, 12], &[(2, 1)])
    } else {
        raw(id)
    }
}
#[cfg(unix)]
#[test]
fn rust_camera_round_trip_preserves_native_manifest_and_all_raw_bytes() {
    let lab = Lab::new();
    let native = lab.extract("native");
    let p = profile(4096);
    let root = lab.output("rust");
    let mut read_ids = Vec::new();
    let report = batch::extract_with_profile(
        &lab.client,
        &root,
        70205,
        Locale::EnUs,
        Some(p),
        |id, cap| {
            assert_eq!(cap, 4096);
            read_ids.push(id);
            Ok((70205, bytes(id)))
        },
    )
    .unwrap();
    assert!(report.complete);
    assert_eq!(read_ids, [SOURCE_FILE_ID, 11, 12]);
    assert_eq!(report.acquisition, Some(p));
    assert!(report.manifest.contains("\"manifest_version\":2"));
    let mut decoded = manifest::decode(&report.manifest).unwrap();
    assert_eq!(decoded.acquisition.take(), Some(p));
    assert_eq!(decoded.render().unwrap(), native.manifest);
    let checked = verify::verify(&root);
    assert_eq!(checked.code, Code::Verified);
    assert_eq!(checked.acquisition, Some(p));
    assert!(checked.json().contains("\"acquisition\":"));
    assert_eq!(
        fs::read(root.join(source_path(Locale::EnUs))).unwrap(),
        bytes(SOURCE_FILE_ID)
    );
    for id in [11, 12] {
        assert_eq!(fs::read(root.join(relative_path(id))).unwrap(), bytes(id));
    }
    assert!(!report.manifest.contains(lab.root.to_str().unwrap()));
}
#[cfg(unix)]
#[test]
fn rust_camera_manifest_requires_exact_provenance_and_limits_for_source_and_files() {
    let lab = Lab::new();
    let p = profile(4096);
    let report = batch::extract_with_profile(
        &lab.client,
        &lab.output("ok"),
        70205,
        Locale::EnUs,
        Some(p),
        |id, _| Ok((70205, bytes(id))),
    )
    .unwrap();
    for (from, to) in [
        ("\"manifest_version\":2", "\"manifest_version\":1"),
        ("\"manifest_version\":2", "\"manifest_version\":3"),
        ("\"backend\":\"rust-local\"", "\"backend\":\"native\""),
        ("\"complete_ckey_md5\":true", "\"complete_ckey_md5\":false"),
        ("\"file_bytes\":4096", "\"file_bytes\":1"),
        ("\"file_bytes\":4096", "\"file_bytes\":16777217"),
        ("\"file_bytes\":4096", "\"file_bytes\":0"),
        ("\"file_bytes\":4096", "\"file_bytes\":4096,\"secret\":0"),
        (
            "\"file_bytes\":4096",
            "\"file_bytes\":4096,\"file_bytes\":4096",
        ),
        (
            "\"root_layout\":\"header-50893-v2\"",
            "\"root_layout\":\"auto\"",
        ),
    ] {
        assert!(report.manifest.contains(from));
        assert!(manifest::decode(&report.manifest.replacen(from, to, 1)).is_err());
    }
    let missing = report
        .manifest
        .replace(&format!(",\"acquisition\":{}", p.render()), "");
    assert!(manifest::decode(&missing).is_err());
    // Source itself fits 512; an oversized camera declaration must still fail.
    let large = batch::extract_with_profile(
        &lab.client,
        &lab.output("large"),
        70205,
        Locale::EnUs,
        Some(p),
        |id, _| {
            Ok((
                70205,
                if id == SOURCE_FILE_ID {
                    source(&[11], &[])
                } else {
                    vec![0; 1024]
                },
            ))
        },
    )
    .unwrap();
    assert!(
        manifest::decode(
            &large
                .manifest
                .replace("\"file_bytes\":4096", "\"file_bytes\":512")
        )
        .is_err()
    );
}
#[cfg(unix)]
#[test]
fn camera_input_change_during_discovery_or_later_camera_stops_all_reads() {
    let lab = Lab::new();
    for fail_at in [1, 3] {
        let root = lab.output(&format!("fail-{fail_at}"));
        let mut calls = 0;
        let result = batch::extract_with_profile(
            &lab.client,
            &root,
            70205,
            Locale::EnUs,
            Some(profile(4096)),
            |id, _| {
                calls += 1;
                if calls == fail_at {
                    Err(Error::LocalStore(crate::local_store::Error::InputChanged))
                } else {
                    Ok((70205, bytes(id)))
                }
            },
        );
        assert_eq!(calls, fail_at);
        assert_eq!(
            result.err(),
            Some(Error::LocalStore(crate::local_store::Error::InputChanged).code())
        );
        assert!(root.join(manifest::MARKER).is_file());
        assert!(!root.join(manifest::NAME).exists());
        assert_eq!(verify::verify(&root).code, Code::Incomplete);
    }
}
#[cfg(unix)]
#[test]
fn camera_reduced_limit_applies_to_source_and_every_camera() {
    let lab = Lab::new();
    for source_failure in [true, false] {
        let root = lab.output(if source_failure { "source" } else { "camera" });
        let cap = if source_failure { 1 } else { 512 };
        let mut calls = 0;
        let report = batch::extract_with_profile(
            &lab.client,
            &root,
            70205,
            Locale::EnUs,
            Some(profile(cap)),
            |id, budget| {
                calls += 1;
                assert_eq!(budget, cap);
                Ok((
                    70205,
                    if id == SOURCE_FILE_ID {
                        source(&[11, 12], &[])
                    } else {
                        vec![0; cap as usize + 1]
                    },
                ))
            },
        )
        .unwrap();
        assert!(!report.complete);
        assert_eq!(calls, if source_failure { 1 } else { 3 });
        assert!(manifest::decode(&report.manifest).is_ok());
        assert_eq!(fs::read_dir(root.join("cameras")).unwrap().count(), 0);
        assert_eq!(verify::verify(&root).code, Code::Incomplete);
        fs::remove_file(root.join(manifest::MARKER)).unwrap();
        assert_eq!(verify::verify(&root).code, Code::Incomplete);
    }
}
#[cfg(unix)]
#[test]
fn camera_rust_zero_id_remains_discovered_and_failure_is_partial() {
    let lab = Lab::new();
    let root = lab.output("zero");
    let mut reads = Vec::new();
    let report = batch::extract_with_profile(
        &lab.client,
        &root,
        70205,
        Locale::EnUs,
        Some(profile(4096)),
        |id, _| {
            reads.push(id);
            if id == SOURCE_FILE_ID {
                Ok((70205, source(&[0, 11], &[])))
            } else if id == 0 {
                Err(Error::MissingFile)
            } else {
                Ok((70205, raw(id)))
            }
        },
    )
    .unwrap();
    assert_eq!(reads, [SOURCE_FILE_ID, 0, 11]);
    assert!(!report.complete);
    assert!(manifest::decode(&report.manifest).is_ok());
    assert_eq!(verify::verify(&root).code, Code::Incomplete);
}
#[test]
fn camera_cli_rust_options_are_explicit_and_cap_cannot_be_increased() {
    let base = ["--client", "private", "--output", "new", "--locale", "esES"];
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
    let options::Parsed::Run(rust) = options::parse(&args(&[
        "--backend",
        "rust-local",
        "--index-layout",
        "paged",
        "--root-layout",
        "header-50893-v2",
        "--max-file-bytes",
        "512",
    ]))
    .unwrap() else {
        panic!()
    };
    assert_eq!(rust.acquisition.profile().unwrap().max_file_bytes(), 512);
    for tail in [
        vec!["--backend", "rust-local"],
        vec!["--root-layout", "header-50893-v2"],
        vec!["--max-file-bytes", "512"],
        vec!["--backend", "native", "--index-layout", "paged"],
        vec![
            "--backend",
            "rust-local",
            "--index-layout",
            "paged",
            "--root-layout",
            "header-50893-v2",
            "--max-file-bytes",
            "16777217",
        ],
        vec!["--backend", "rust-local", "--backend", "rust-local"],
    ] {
        assert!(options::parse(&args(&tail)).is_err());
    }
}
