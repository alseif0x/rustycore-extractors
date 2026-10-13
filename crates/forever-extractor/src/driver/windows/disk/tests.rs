// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-13: native synthetic Windows disk preflight guards only.
use super::*;
use forever_platform::windows::{LinkPolicy, package as native};
use std::fs;

struct Lab(PathBuf);
impl Lab {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "rce-windows-disk-preflight-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn private(&self, name: &str) -> native::Root {
        native::Root::create(&self.0.join(name), native::Limits::default()).unwrap()
    }
}
impl Drop for Lab {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
fn budget() -> DiskBudget {
    DiskBudget {
        raw_bytes: 32,
        wdt_bytes: 32,
        cache_bytes: 32,
        placement_bytes: 32,
        vmo_bytes: 32,
        map_bytes: 32,
        output_bytes: 1,
    }
}

#[test]
fn readonly_preflight_accepts_private_siblings_without_output_or_namespace_creation() {
    let lab = Lab::new();
    let _scratch = lab.private("scratch");
    let _client = lab.private("client");
    let output = lab.0.join("output");
    let p = Preflight::open(
        budget(),
        &lab.0.join("scratch"),
        &output,
        &lab.0.join("client"),
        None,
        None,
    )
    .unwrap();
    p.verify_unchanged().unwrap();
    assert!(!output.exists());
    assert_eq!(fs::read_dir(lab.0.join("scratch")).unwrap().count(), 0);
}

#[test]
fn preflight_rejects_source_output_and_key_aliases_before_mutation() {
    let lab = Lab::new();
    let _scratch = lab.private("scratch");
    let _client = lab.private("client");
    let scratch = lab.0.join("scratch");
    let client = lab.0.join("client");
    for (scratch_path, output) in [
        (&client, lab.0.join("output")),
        (&scratch, scratch.join("output")),
        (&scratch, client.join("output")),
        (&scratch, lab.0.clone()),
    ] {
        assert!(Preflight::open(budget(), scratch_path, &output, &client, None, None).is_err());
    }
    let key_path = scratch.join("authored-toy-key");
    fs::write(&key_path, b"synthetic key path guard only").unwrap();
    let key = ReadFile::open(&key_path, PathLimits::default(), LinkPolicy::Reject).unwrap();
    assert!(
        Preflight::open(
            budget(),
            &scratch,
            &lab.0.join("output"),
            &client,
            Some(&key_path),
            Some(&key)
        )
        .is_err()
    );
    assert!(!lab.0.join("output").exists());
    assert_eq!(fs::read_dir(&scratch).unwrap().count(), 1);
}

#[test]
fn own_directory_changes_are_allowed_and_observed_guard_failure_is_permanent() {
    let lab = Lab::new();
    let _scratch = lab.private("scratch");
    let _client = lab.private("client");
    let scratch = lab.0.join("scratch");
    let p = Preflight::open(
        budget(),
        &scratch,
        &lab.0.join("output"),
        &lab.0.join("client"),
        None,
        None,
    )
    .unwrap();
    fs::write(scratch.join("unrelated-attempt-evidence"), b"synthetic").unwrap();
    p.verify_unchanged().unwrap();
    fs::write(scratch.join(".git"), b"synthetic guard violation").unwrap();
    assert!(p.verify_unchanged().is_err());
    fs::remove_file(scratch.join(".git")).unwrap();
    assert_eq!(
        p.verify_unchanged().unwrap_err().code,
        "product_disk_changed"
    );
}

#[test]
fn scratch_requires_explicit_private_dacl_before_output_creation() {
    let lab = Lab::new();
    let _client = lab.private("client");
    let output = lab.0.join("output");
    assert_eq!(
        Preflight::open(budget(), &lab.0, &output, &lab.0.join("client"), None, None)
            .err()
            .unwrap()
            .code,
        "product_disk_privacy"
    );
    assert!(!output.exists());
}
