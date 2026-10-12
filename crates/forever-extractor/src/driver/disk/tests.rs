// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-12: original pre-acquisition filesystem admission guards.
use super::*;
use std::{
    fs::{File, OpenOptions, Permissions},
    os::unix::fs::{PermissionsExt, symlink},
    sync::atomic::AtomicUsize,
};
static FIXTURE: AtomicUsize = AtomicUsize::new(0);
struct Fixture {
    root: PathBuf,
    scratch: PathBuf,
    client: PathBuf,
    output: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "rce-product-disk-guard-{}-{}",
            std::process::id(),
            FIXTURE.fetch_add(1, Ordering::Relaxed)
        ));
        DirBuilder::new().mode(0o700).create(&root).unwrap();
        let scratch = root.join("scratch");
        let client = root.join("client");
        let output = root.join("output");
        for path in [&scratch, &client] {
            DirBuilder::new().mode(0o700).create(path).unwrap();
        }
        Self {
            root,
            scratch,
            client,
            output,
        }
    }
    fn open(&self) -> Preflight {
        Preflight::open(budget(), &self.scratch, &self.output, &self.client, None).unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
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
fn preflight_has_no_output_or_private_namespace_side_effects() {
    let f = Fixture::new();
    let p = f.open();
    p.verify_unchanged().unwrap();
    assert!(!f.output.exists());
    assert_eq!(fs::read_dir(&f.scratch).unwrap().count(), 0);
}
#[test]
fn own_directory_changes_are_allowed_but_identity_replacement_poisons() {
    let f = Fixture::new();
    let p = f.open();
    fs::write(f.scratch.join("retained-other-run"), b"authored").unwrap();
    p.verify_unchanged().unwrap();
    let renamed = f.root.join("old-scratch");
    fs::rename(&f.scratch, &renamed).unwrap();
    DirBuilder::new().mode(0o700).create(&f.scratch).unwrap();
    assert_eq!(
        p.verify_unchanged().unwrap_err().code,
        "product_disk_changed"
    );
    fs::remove_dir(&f.scratch).unwrap();
    fs::rename(renamed, &f.scratch).unwrap();
    assert_eq!(
        p.verify_unchanged().unwrap_err().code,
        "product_disk_changed"
    );
}
#[test]
fn privacy_changes_poison_even_when_restored() {
    let f = Fixture::new();
    let p = f.open();
    fs::set_permissions(&f.scratch, Permissions::from_mode(0o750)).unwrap();
    assert!(p.verify_unchanged().is_err());
    fs::set_permissions(&f.scratch, Permissions::from_mode(0o700)).unwrap();
    assert_eq!(
        p.verify_unchanged().unwrap_err().code,
        "product_disk_changed"
    );
}
#[test]
fn git_ancestor_is_rejected_and_injected_git_poisons() {
    let f = Fixture::new();
    fs::write(f.root.join(".git"), b"authored guard").unwrap();
    assert_eq!(
        Preflight::open(budget(), &f.scratch, &f.output, &f.client, None)
            .err()
            .unwrap()
            .code,
        "product_disk_path"
    );
    fs::remove_file(f.root.join(".git")).unwrap();
    let p = f.open();
    fs::write(f.scratch.join(".git"), b"authored guard").unwrap();
    assert!(p.verify_unchanged().is_err());
    fs::remove_file(f.scratch.join(".git")).unwrap();
    assert_eq!(
        p.verify_unchanged().unwrap_err().code,
        "product_disk_changed"
    );
}
#[test]
fn symlink_ancestors_and_relative_or_dot_paths_reject() {
    let f = Fixture::new();
    let link = f.root.join("linked");
    symlink(&f.scratch, &link).unwrap();
    assert!(Preflight::open(budget(), &link, &f.output, &f.client, None).is_err());
    for path in [
        PathBuf::from("relative"),
        f.scratch.join(".."),
        f.scratch.join("."),
    ] {
        assert!(Preflight::open(budget(), &path, &f.output, &f.client, None).is_err());
    }
    assert!(!f.output.exists());
}
#[test]
fn source_and_key_containment_are_rejected() {
    let f = Fixture::new();
    assert_eq!(
        Preflight::open(budget(), &f.scratch, &f.output, &f.scratch, None)
            .err()
            .unwrap()
            .code,
        "product_disk_alias"
    );
    let key = f.scratch.join("keys");
    fs::write(&key, b"authored").unwrap();
    assert_eq!(
        Preflight::open(budget(), &f.scratch, &f.output, &f.client, Some(&key))
            .err()
            .unwrap()
            .code,
        "product_disk_alias"
    );
    assert!(
        Preflight::open(
            budget(),
            &f.scratch,
            &f.client.join("output"),
            &f.client,
            None
        )
        .is_err()
    );
    assert_eq!(fs::read_dir(&f.client).unwrap().count(), 0);
}
#[test]
fn key_metadata_is_held_and_full_metadata_changes_poison() {
    let f = Fixture::new();
    let key = f.root.join("key");
    fs::write(&key, b"authored").unwrap();
    let p = Preflight::open(budget(), &f.scratch, &f.output, &f.client, Some(&key)).unwrap();
    let file = OpenOptions::new().write(true).open(&key).unwrap();
    file.set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_secs(1))
        .unwrap();
    assert_eq!(
        p.verify_unchanged().unwrap_err().code,
        "product_disk_changed"
    );
}
#[test]
fn newly_created_output_is_bound_for_later_guards() {
    let f = Fixture::new();
    let p = f.open();
    DirBuilder::new().mode(0o700).create(&f.output).unwrap();
    p.verify_unchanged().unwrap();
    fs::write(f.output.join("own-state"), b"authored").unwrap();
    p.verify_unchanged().unwrap();
    let old = f.root.join("old-output");
    fs::rename(&f.output, old).unwrap();
    DirBuilder::new().mode(0o700).create(&f.output).unwrap();
    assert_eq!(
        p.verify_unchanged().unwrap_err().code,
        "product_disk_changed"
    );
}
#[test]
fn existing_output_admits_resume_and_invalid_budget_precedes_io() {
    let f = Fixture::new();
    DirBuilder::new().mode(0o700).create(&f.output).unwrap();
    let p = f.open();
    p.verify_unchanged().unwrap();
    let mut invalid = budget();
    invalid.raw_bytes = 31;
    assert_eq!(
        Preflight::open(
            invalid,
            Path::new("missing"),
            Path::new("output"),
            Path::new("client"),
            None
        )
        .err()
        .unwrap()
        .code,
        "product_plan_disk_budget"
    );
    // Borrowed descriptor remains usable after all checks.
    assert!(File::open(&f.scratch).unwrap().metadata().unwrap().is_dir());
}

#[test]
fn scratch_shape_reserves_namespace_and_owner_components() {
    let mut scratch = PathBuf::from("/");
    for _ in 0..29 {
        scratch.push("x");
    }
    assert_eq!(scratch.components().count(), 30);
    scratch_shape(&scratch).unwrap();
    scratch.push("x");
    assert_eq!(scratch.components().count(), 31);
    assert_eq!(
        scratch_shape(&scratch).unwrap_err().code,
        "product_disk_path"
    );
}
#[test]
fn scratch_shape_reserves_full_namespace_bytes_and_rejects_nul() {
    let suffix = Path::new(MAX_NAMESPACE).join("placements");
    let extra = suffix.as_os_str().len() + 1;
    let scratch = PathBuf::from(format!("/{}", "x".repeat(anchors::MAX_PATH - extra - 1)));
    assert_eq!(
        scratch
            .join(MAX_NAMESPACE)
            .join("placements")
            .as_os_str()
            .len(),
        anchors::MAX_PATH
    );
    scratch_shape(&scratch).unwrap();
    let longer = PathBuf::from(format!("{}x", scratch.display()));
    assert_eq!(
        scratch_shape(&longer).unwrap_err().code,
        "product_disk_path"
    );
    assert_eq!(
        scratch_shape(Path::new("/scratch\0suffix"))
            .unwrap_err()
            .code,
        "product_disk_path"
    );
}
