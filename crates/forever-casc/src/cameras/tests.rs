// SPDX-License-Identifier: GPL-3.0-or-later
//! Original synthetic WDC5 bytes and temporary clients only.
use super::manifest::{self, Manifest};
#[cfg(unix)]
use super::verify::Code;
use super::*;
#[cfg(unix)]
use crate::Error;
use crate::Locale;
#[cfg(unix)]
use std::path::Path;
use std::{
    ffi::OsString,
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
fn put32(out: &mut [u8], at: usize, n: u32) {
    out[at..at + 4].copy_from_slice(&n.to_le_bytes());
}
fn put16(out: &mut [u8], at: usize, n: u16) {
    out[at..at + 2].copy_from_slice(&n.to_le_bytes());
}
fn source(ids: &[u32], copies: &[(u32, u32)]) -> Vec<u8> {
    let mut out = vec![0; 384];
    out[..4].copy_from_slice(b"WDC5");
    put32(&mut out, 4, 5);
    put32(&mut out, 136, ids.len() as u32);
    put32(&mut out, 140, 5);
    put32(&mut out, 144, 28);
    put32(&mut out, 156, SOURCE_LAYOUT);
    put32(&mut out, 160, 1);
    let max = (ids.len() as u32)
        .max(copies.iter().map(|(id, _)| *id).max().unwrap_or(0))
        .max(1);
    put32(&mut out, 164, max);
    put16(&mut out, 174, u16::MAX);
    put32(&mut out, 176, 5);
    put32(&mut out, 188, 120);
    put32(&mut out, 200, 1);
    put32(&mut out, 212, 384);
    put32(&mut out, 216, ids.len() as u32);
    put32(&mut out, 228, ids.len() as u32 * 4);
    put32(&mut out, 240, copies.len() as u32);
    for (field, (offset, count)) in [(0, 3), (96, 1), (128, 1), (160, 1), (192, 1)]
        .into_iter()
        .enumerate()
    {
        let at = 264 + 24 * field;
        put16(&mut out, at, offset);
        put32(&mut out, at + 16, 32);
        put32(&mut out, at + 20, count);
    }
    for &id in ids {
        let at = out.len();
        out.resize(at + 28, 0);
        put32(&mut out, at + 20, id);
    }
    for id in 1..=ids.len() as u32 {
        out.extend_from_slice(&id.to_le_bytes());
    }
    for &(new, old) in copies {
        out.extend_from_slice(&new.to_le_bytes());
        out.extend_from_slice(&old.to_le_bytes());
    }
    out
}
fn metadata(build: u32, locale: &str) -> String {
    format!(
        "Product!STRING:0|Active!DEC:1|Version!STRING:0|Build Key!HEX:16|CDN Key!HEX:16|Tags!STRING:0\nwow_classic_beta|1|1.60.1.{build}|11111111111111111111111111111111|22222222222222222222222222222222|{locale}\n"
    )
}
struct Lab {
    root: PathBuf,
    client: PathBuf,
}
impl Lab {
    fn new() -> Self {
        let mut base = std::env::temp_dir();
        if base.ancestors().any(|p| p.join(".git").exists()) {
            base = PathBuf::from("/tmp");
        }
        let root = base.join(format!(
            "rce-camera-synthetic-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        let client = root.join("client");
        fs::create_dir(&client).unwrap();
        fs::write(client.join(".build.info"), metadata(70205, "enUS")).unwrap();
        Self { root, client }
    }
    fn output(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }
    #[cfg(unix)]
    fn extract(&self, name: &str) -> batch::Report {
        batch::extract(
            &self.client,
            &self.output(name),
            70205,
            Locale::EnUs,
            |id, budget| {
                assert_eq!(budget as usize, MAX_FILE_BYTES);
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
        .unwrap()
    }
}
impl Drop for Lab {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
#[cfg(unix)]
fn raw(id: u32) -> Vec<u8> {
    let mut bytes = b"not parsed as M2\0\xff".to_vec();
    bytes.extend_from_slice(&id.to_le_bytes());
    bytes
}
#[cfg(unix)]
fn rewrite(path: &Path, from: &str, to: &str) {
    let text = fs::read_to_string(path).unwrap();
    assert!(text.contains(from));
    fs::write(path, text.replacen(from, to, 1)).unwrap();
}

#[test]
fn discovery_uses_direct_fields_preserves_zero_and_ignores_copy_overwrites() {
    let bytes = source(&[22, 11, 0, 22], &[(2, 1), (5, 2)]);
    let plan = discover(&bytes).unwrap();
    assert_eq!(plan.file_ids(), &[0, 11, 22].into_iter().collect());
    assert_eq!(plan.facts().direct_records, 4);
    assert_eq!(plan.facts().unique_file_ids, 3);
    assert_eq!(
        plan.facts().file_id_set_sha256,
        file_id_set_hash(plan.file_ids())
    );
    assert_eq!(file_name(0), "FILE00000000.xxx");
    assert_eq!(file_name(0xabcd1234), "FILEABCD1234.xxx");
    assert_eq!(file_name(u32::MAX), "FILEFFFFFFFF.xxx");
}
#[test]
fn discovery_never_returns_a_projection_before_tail_and_copies_are_validated() {
    let mut trailing = source(&[11], &[]);
    trailing.push(0);
    assert!(discover(&trailing).is_err());
    assert!(discover(&source(&[11], &[(1, 1)])).is_err());
    let mut layout = source(&[11], &[]);
    put32(&mut layout, 156, 0);
    assert_eq!(discover(&layout).err(), Some("camera_source_schema"));
    let mut sparse = source(&[11], &[]);
    put16(&mut sparse, 172, 1);
    assert!(discover(&sparse).is_err());
    let mut encrypted = source(&[11], &[]);
    put32(&mut encrypted, 204, 1);
    assert!(discover(&encrypted).is_err());
    let mut truncated = source(&[11], &[]);
    truncated.pop();
    assert!(discover(&truncated).is_err());
}
#[test]
fn discovery_limits_the_whole_set_and_source_bytes() {
    let ids = (1..=MAX_FILES as u32).collect::<Vec<_>>();
    assert_eq!(
        discover(&source(&ids, &[])).unwrap().file_ids().len(),
        MAX_FILES
    );
    let ids = (0..=MAX_FILES as u32).collect::<Vec<_>>();
    assert_eq!(
        discover(&source(&ids, &[])).err(),
        Some("camera_policy_limit")
    );
    assert_eq!(
        discover(&vec![0; MAX_SOURCE_BYTES + 1]).err(),
        Some("camera_policy_limit")
    );
}
#[cfg(unix)]
#[test]
fn complete_pipeline_retains_source_raw_bytes_names_and_redacted_claims() {
    let lab = Lab::new();
    let report = lab.extract("complete");
    assert!(report.complete);
    assert_eq!(report.discovered_files, Some(2));
    assert_eq!(report.files_published, 3);
    let root = lab.output("complete");
    assert_eq!(
        fs::read(root.join(source_path(Locale::EnUs))).unwrap(),
        source(&[12, 11, 12], &[(2, 1)])
    );
    for id in [11, 12] {
        assert_eq!(fs::read(root.join(relative_path(id))).unwrap(), raw(id));
    }
    assert!(!root.join(manifest::MARKER).exists());
    assert_eq!(verify::verify(&root).code, Code::Verified);
    assert_eq!(verify::verify(&root).cameras_verified, 2);
    assert!(report.json().contains("\"m2_parsed\":false"));
    assert!(!report.manifest.contains("not parsed"));
    assert!(!report.manifest.contains(lab.root.to_str().unwrap()));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(&root).unwrap().permissions().mode() & 0o777,
            0o700
        );
        for path in [
            root.join(manifest::NAME),
            root.join(relative_path(11)),
            root.join(source_path(Locale::EnUs)),
        ] {
            assert_eq!(
                fs::metadata(path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
    }
}
#[cfg(unix)]
#[test]
fn a_valid_empty_source_is_distinct_from_missing_source() {
    let lab = Lab::new();
    let root = lab.output("empty");
    let report = batch::extract(&lab.client, &root, 70205, Locale::EnUs, |id, _| {
        assert_eq!(id, SOURCE_FILE_ID);
        Ok((70205, source(&[], &[])))
    })
    .unwrap();
    assert!(report.complete);
    assert_eq!(report.discovered_files, Some(0));
    assert_eq!(report.files_published, 1);
    assert_eq!(verify::verify(&root).code, Code::Verified);
    let root = lab.output("missing");
    let report = batch::extract(&lab.client, &root, 70205, Locale::EnUs, |id, _| {
        assert_eq!(id, SOURCE_FILE_ID);
        Err(Error::MissingFile)
    })
    .unwrap();
    assert!(!report.complete);
    assert_eq!(report.discovered_files, None);
    assert_eq!(report.files_published, 0);
    assert!(manifest::decode(&report.manifest).is_ok());
    assert_eq!(verify::verify(&root).code, Code::Incomplete);
    fs::remove_file(root.join(manifest::MARKER)).unwrap();
    assert_eq!(verify::verify(&root).code, Code::Incomplete);
}
#[cfg(unix)]
#[test]
fn failed_zero_missing_and_incomplete_camera_ids_are_never_optional() {
    let lab = Lab::new();
    for (name, error) in [
        ("zero", Error::InvalidId),
        ("missing", Error::MissingFile),
        ("short", Error::Incomplete),
        ("encrypted", Error::Encrypted),
    ] {
        let root = lab.output(name);
        let report = batch::extract(&lab.client, &root, 70205, Locale::EnUs, |id, _| {
            if id == SOURCE_FILE_ID {
                Ok((70205, source(&[0, 11], &[])))
            } else if id == 0 {
                Err(error)
            } else {
                Ok((70205, raw(id)))
            }
        })
        .unwrap();
        assert!(!report.complete);
        assert_eq!(report.discovered_files, Some(2));
        assert_eq!(report.files_published, 2);
        assert!(!root.join(relative_path(0)).exists());
        assert!(manifest::decode(&report.manifest).is_ok());
        assert_eq!(verify::verify(&root).code, Code::Incomplete);
        fs::remove_file(root.join(manifest::MARKER)).unwrap();
        assert_eq!(verify::verify(&root).code, Code::Incomplete);
    }
}
#[cfg(unix)]
#[test]
fn invalid_source_does_not_publish_or_start_camera_reads() {
    let lab = Lab::new();
    let root = lab.output("invalid");
    let mut calls = 0;
    let report = batch::extract(&lab.client, &root, 70205, Locale::EnUs, |id, _| {
        calls += 1;
        assert_eq!(id, SOURCE_FILE_ID);
        Ok((70205, b"bad".to_vec()))
    })
    .unwrap();
    assert_eq!(calls, 1);
    assert!(!report.complete);
    assert_eq!(report.files_published, 0);
    assert!(!root.join(source_path(Locale::EnUs)).exists());
    assert_eq!(verify::verify(&root).code, Code::Incomplete);
}
#[cfg(unix)]
#[test]
fn manifest_fields_provenance_identity_and_claims_are_closed() {
    let lab = Lab::new();
    let report = lab.extract("mutations");
    for (from, to) in [
        ("\"manifest_version\":1", "\"manifest_version\":2"),
        ("\"manifest_version\":1", "\"manifest_version\":1.0"),
        (
            "\"manifest_version\":1",
            "\"manifest_version\":1,\"manifest_version\":1",
        ),
        (
            "\"manifest_version\":1",
            "\"manifest_version\":1,\"extra\":0",
        ),
        ("\"file_id\":1294214", "\"file_id\":1134583"),
        (
            "dbc/enUS/CinematicCamera.db2",
            "dbc/enUS/CinematicSequences.db2",
        ),
        ("cameras/FILE0000000B.xxx", "../FILE0000000B.xxx"),
        ("cameras/FILE0000000B.xxx", "cameras/file0000000b.xxx"),
        ("\"max_files\":4096", "\"max_files\":4097"),
        ("\"unique_file_ids\":2", "\"unique_file_ids\":1"),
        ("\"direct_records\":3", "\"direct_records\":1"),
        ("\"file_count\":3", "\"file_count\":2"),
        ("\"offline\":true", "\"offline\":false"),
        ("\"m2_parsed\":false", "\"m2_parsed\":true"),
        ("\"runtime_accepted\":false", "\"runtime_accepted\":true"),
        ("direct_rows_u32_field3_v1", "final_logical_rows"),
    ] {
        assert!(report.manifest.contains(from));
        assert!(
            manifest::decode(&report.manifest.replacen(from, to, 1)).is_err(),
            "accepted {from}"
        );
    }
    assert!(manifest::decode(&report.manifest.replace(',', ",\n ")).is_ok());
    assert!(manifest::decode(&" ".repeat(manifest::MAX_MANIFEST_BYTES + 1)).is_err());
    assert!(manifest::decode(&format!("{}0{}", "[".repeat(40), "]".repeat(40))).is_err());
}
#[cfg(unix)]
#[test]
fn missing_extra_case_pending_and_raw_tampering_fail_verification() {
    let lab = Lab::new();
    for name in [
        "missing",
        "extra",
        "case",
        "pending",
        "rootextra",
        "tamper",
        "source",
    ] {
        lab.extract(name);
        let root = lab.output(name);
        let expected = match name {
            "missing" => {
                fs::remove_file(root.join(relative_path(11))).unwrap();
                Code::Inventory
            }
            "extra" => {
                fs::write(root.join(relative_path(13)), b"x").unwrap();
                Code::Inventory
            }
            "case" => {
                fs::rename(
                    root.join(relative_path(11)),
                    root.join("cameras/FILE0000000b.xxx"),
                )
                .unwrap();
                Code::Inventory
            }
            "pending" => {
                fs::write(root.join("cameras/pending.part"), b"x").unwrap();
                Code::Incomplete
            }
            "rootextra" => {
                fs::write(root.join("extra"), b"x").unwrap();
                Code::Inventory
            }
            "source" => {
                fs::write(root.join(source_path(Locale::EnUs)), b"x").unwrap();
                Code::Inventory
            }
            _ => {
                fs::write(root.join(relative_path(11)), b"x").unwrap();
                Code::Inventory
            }
        };
        assert_eq!(verify::verify(&root).code, expected);
    }
}
#[cfg(unix)]
#[test]
fn rediscovery_rejects_a_forged_but_self_consistent_manifest_file_set() {
    let lab = Lab::new();
    let report = lab.extract("forged");
    let root = lab.output("forged");
    let mut manifest: Manifest = manifest::decode(&report.manifest).unwrap();
    manifest.cameras.remove(0);
    let ids = manifest.cameras.iter().map(|entry| entry.file_id).collect();
    let facts = manifest.source.discovery.as_mut().unwrap();
    facts.unique_file_ids = 1;
    facts.file_id_set_sha256 = file_id_set_hash(&ids);
    let text = manifest.render().unwrap();
    assert!(manifest::decode(&text).is_ok());
    fs::write(root.join(manifest::NAME), text).unwrap();
    fs::remove_file(root.join(relative_path(11))).unwrap();
    assert_eq!(verify::verify(&root).code, Code::Discovery);
}
#[cfg(unix)]
#[test]
fn camera_manifest_order_and_duplicate_ids_are_rejected() {
    let lab = Lab::new();
    let report = lab.extract("order");
    let mut manifest = manifest::decode(&report.manifest).unwrap();
    manifest.cameras.swap(0, 1);
    assert!(manifest::decode(&manifest.render().unwrap()).is_err());
    manifest.cameras[0].file_id = manifest.cameras[1].file_id;
    assert!(manifest::decode(&manifest.render().unwrap()).is_err());
}
#[test]
fn manifest_inventory_rejects_total_budget_before_any_usize_overflow() {
    // Descriptor-only fixtures: no large allocation or client payload is needed.
    let descriptor = |count: u32, camera_bytes: usize| {
        let ids: std::collections::BTreeSet<_> = (1..=count).collect();
        Manifest {
            contract: CameraContract::Legacy,
            acquisition: None,
            build: 70205,
            locale: Locale::EnUs,
            source: manifest::Source {
                file: manifest::FileOutcome::complete(&[]),
                discovery: Some(DiscoveryFacts {
                    direct_records: count,
                    unique_file_ids: ids.len(),
                    file_id_set_sha256: file_id_set_hash(&ids),
                }),
            },
            cameras: ids
                .into_iter()
                .map(|file_id| {
                    let mut file = manifest::FileOutcome::complete(&[]);
                    file.bytes = Some(camera_bytes);
                    manifest::Camera { file_id, file }
                })
                .collect(),
        }
    };
    let mut boundary = descriptor(16, MAX_FILE_BYTES);
    assert_eq!(boundary.inventory().unwrap().1, MAX_TOTAL_BYTES);
    assert!(manifest::decode(&boundary.render().unwrap()).is_ok());
    boundary.source.file.bytes = Some(1);
    assert_eq!(boundary.inventory().err(), Some("camera_policy_limit"));
    assert_eq!(boundary.render().err(), Some("camera_policy_limit"));
    boundary.source.file.bytes = Some(usize::MAX);
    assert_eq!(boundary.inventory().err(), Some("camera_policy_limit"));

    let small = descriptor(256, 0).render().unwrap();
    assert!(manifest::decode(&small).is_ok());
    // Each entry is within the 16 MiB bound, but 257 entries exceed both the
    // batch budget and a 32-bit usize. Parsing must reject without wrapping.
    // Report the wrapped total that the old release-mode 32-bit sum produced.
    // The inventory hashes paths and file hashes only, so it is unchanged.
    let forged = small
        .replace("\"bytes\":0", &format!("\"bytes\":{MAX_FILE_BYTES}"))
        .replace(
            "\"total_bytes\":0",
            &format!("\"total_bytes\":{MAX_FILE_BYTES}"),
        );
    assert!(manifest::decode(&forged).is_err());
}
#[cfg(unix)]
#[test]
fn changed_discovery_facts_fail_even_when_manifest_is_structurally_valid() {
    let lab = Lab::new();
    lab.extract("facts");
    let root = lab.output("facts");
    rewrite(
        &root.join(manifest::NAME),
        "\"direct_records\":3",
        "\"direct_records\":4",
    );
    assert_eq!(verify::verify(&root).code, Code::Discovery);
}
#[cfg(unix)]
#[test]
fn per_file_limits_apply_to_source_and_camera_before_publication() {
    let lab = Lab::new();
    for source_large in [true, false] {
        let root = lab.output(if source_large {
            "source-large"
        } else {
            "camera-large"
        });
        let report = batch::extract(&lab.client, &root, 70205, Locale::EnUs, |id, limit| {
            Ok((
                70205,
                if (id == SOURCE_FILE_ID) == source_large {
                    vec![0; limit as usize + 1]
                } else if id == SOURCE_FILE_ID {
                    source(&[11], &[])
                } else {
                    raw(id)
                },
            ))
        })
        .unwrap();
        assert!(!report.complete);
        assert_eq!(report.files_published, usize::from(!source_large));
        assert_eq!(verify::verify(&root).code, Code::Incomplete);
    }
}
#[cfg(unix)]
#[test]
fn build_and_metadata_guards_cover_discovery_and_each_camera() {
    let lab = Lab::new();
    let root = lab.output("observed");
    let report = batch::extract(&lab.client, &root, 70205, Locale::EnUs, |_, _| {
        Ok((70170, source(&[11], &[])))
    })
    .unwrap();
    assert!(!report.complete);
    assert_eq!(report.discovered_files, None);
    let root = lab.output("metadata");
    let result = batch::extract(&lab.client, &root, 70205, Locale::EnUs, |id, _| {
        if id == SOURCE_FILE_ID {
            Ok((70205, source(&[11], &[])))
        } else {
            fs::write(lab.client.join(".build.info"), metadata(70205, "esES")).unwrap();
            Ok((70205, raw(id)))
        }
    });
    assert_eq!(result.err(), Some("client_metadata_changed"));
    assert!(root.join(manifest::MARKER).exists());
    assert!(!root.join(manifest::NAME).exists());
    let root = lab.output("build");
    let mut calls = 0;
    let result = batch::extract(&lab.client, &root, 70205, Locale::EnUs, |_, _| {
        calls += 1;
        fs::write(lab.client.join(".build.info"), metadata(70170, "esES")).unwrap();
        Ok((70205, source(&[11], &[])))
    });
    assert_eq!(result.err(), Some("client_build_changed"));
    assert_eq!(calls, 1);
}
#[test]
fn remaining_total_budget_is_bounded_and_never_wraps() {
    assert_eq!(batch::remaining_budget(0), Ok(MAX_FILE_BYTES));
    assert_eq!(
        batch::remaining_budget(MAX_SOURCE_BYTES),
        Ok(MAX_FILE_BYTES)
    );
    assert_eq!(
        batch::remaining_budget(MAX_TOTAL_BYTES - MAX_FILE_BYTES),
        Ok(MAX_FILE_BYTES)
    );
    assert_eq!(
        batch::remaining_budget(MAX_TOTAL_BYTES - MAX_FILE_BYTES + 1),
        Ok(MAX_FILE_BYTES - 1)
    );
    assert_eq!(batch::remaining_budget(MAX_TOTAL_BYTES - 1), Ok(1));
    assert_eq!(batch::remaining_budget(MAX_TOTAL_BYTES), Ok(0));
    assert_eq!(
        batch::remaining_budget(MAX_TOTAL_BYTES + 1),
        Err("camera_policy_limit")
    );
    assert_eq!(
        batch::remaining_budget(usize::MAX),
        Err("camera_policy_limit")
    );
}
#[test]
fn unsupported_or_changed_initial_build_never_creates_output() {
    let lab = Lab::new();
    let root = lab.output("unsupported");
    assert_eq!(
        batch::extract(&lab.client, &root, 123, Locale::EnUs, |_, _| panic!()).err(),
        Some("unsupported_client")
    );
    assert!(!root.exists());
    assert_eq!(
        batch::extract(&lab.client, &root, 70170, Locale::EnUs, |_, _| panic!()).err(),
        Some("client_build_changed")
    );
    assert!(!root.exists());
}
#[cfg(unix)]
#[test]
fn output_guards_reject_existing_client_git_and_traversal_paths() {
    let lab = Lab::new();
    let first = lab.extract("existing");
    let root = lab.output("existing");
    assert_eq!(
        batch::extract(&lab.client, &root, 70205, Locale::EnUs, |_, _| panic!()).err(),
        Some("output_exists_or_unavailable")
    );
    assert_eq!(
        fs::read_to_string(root.join(manifest::NAME)).unwrap(),
        first.manifest
    );
    for output in [
        lab.client.join("out"),
        lab.root.clone(),
        lab.root.join("client/../out"),
    ] {
        assert_eq!(
            batch::extract(&lab.client, &output, 70205, Locale::EnUs, |_, _| panic!()).err(),
            Some("unsafe_output_path")
        );
    }
    let git = lab.output("git");
    fs::create_dir(&git).unwrap();
    fs::write(git.join(".git"), b"synthetic").unwrap();
    assert_eq!(
        batch::extract(
            &lab.client,
            &git.join("out"),
            70205,
            Locale::EnUs,
            |_, _| panic!()
        )
        .err(),
        Some("unsafe_output_path")
    );
}
#[cfg(unix)]
#[test]
fn symlink_roots_sources_cameras_and_client_data_aliases_are_rejected() {
    use std::os::unix::fs::symlink;
    let lab = Lab::new();
    lab.extract("ok");
    let root = lab.output("ok");
    let alias = lab.output("alias");
    symlink(&root, &alias).unwrap();
    assert_eq!(verify::verify(&alias).code, Code::UnsafePath);
    let parent_alias = lab.output("parent-alias");
    symlink(&lab.root, &parent_alias).unwrap();
    assert_eq!(
        verify::verify(&parent_alias.join("ok")).code,
        Code::UnsafePath
    );
    for name in ["camera", "source"] {
        lab.extract(name);
        let root = lab.output(name);
        let path = if name == "camera" {
            root.join(relative_path(11))
        } else {
            root.join(source_path(Locale::EnUs))
        };
        fs::remove_file(&path).unwrap();
        symlink(lab.client.join(".build.info"), path).unwrap();
        assert_eq!(verify::verify(&root).code, Code::UnsafePath);
    }
    let data = lab.output("data");
    fs::create_dir(&data).unwrap();
    symlink(&data, lab.client.join("Data")).unwrap();
    assert_eq!(
        batch::extract(
            &lab.client,
            &data.join("out"),
            70205,
            Locale::EnUs,
            |_, _| panic!()
        )
        .err(),
        Some("unsafe_output_path")
    );
}
#[test]
fn cli_has_no_arbitrary_ids_source_paths_or_network_options() {
    let args = |parts: &[&str]| parts.iter().map(OsString::from).collect::<Vec<_>>();
    let base = ["--client", "client", "--locale", "enUS", "--output", "out"];
    assert!(options::parse(&args(&base)).is_ok());
    assert_eq!(
        options::parse(&args(&["--help"])).unwrap(),
        options::Parsed::Help
    );
    for tail in [
        ["--id", "11"],
        ["--source", "source.db2"],
        ["--allow-network", "true"],
        ["--group", "all"],
    ] {
        let mut values = base.to_vec();
        values.extend(tail);
        assert!(options::parse(&args(&values)).is_err());
    }
    assert!(
        options::parse(&args(&[
            "--client", "x", "--locale", "None", "--output", "out"
        ]))
        .is_err()
    );
    assert!(options::verify_output(&args(&["--output", "out", "--repair", "true"])).is_err());
}

mod acquisition;

#[cfg(unix)]
mod build70338;
