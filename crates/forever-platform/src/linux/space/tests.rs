// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-12: original directory-space and checked arithmetic tests.
use super::*;
use std::{
    fs::{self, OpenOptions},
    os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt},
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "rce-linux-space-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::DirBuilder::new().mode(0o700).create(&path).unwrap();
        Self(path)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn actual_directory_observation_retains_descriptor_and_metadata() {
    assert!(supported());
    let fixture = Fixture::new();
    let directory = File::open(&fixture.0).unwrap();
    let before = directory.metadata().unwrap();
    let observed = available(&directory).unwrap();
    assert_ne!(observed.allocation_unit, 0);
    assert_eq!(observed.bytes % observed.allocation_unit, 0);
    // A full filesystem is a valid observation; no assumption of free capacity.
    assert!(observed.inodes.is_none_or(|count| count != u64::MAX));
    let after = directory.metadata().unwrap();
    assert_eq!(before.dev(), after.dev());
    assert_eq!(before.ino(), after.ino());
    assert_eq!(before.mode(), after.mode());
    assert_eq!(before.uid(), after.uid());
    assert_eq!(before.gid(), after.gid());
    assert_eq!(before.mtime(), after.mtime());
    assert_eq!(before.mtime_nsec(), after.mtime_nsec());
    assert_eq!(before.ctime(), after.ctime());
    assert_eq!(before.ctime_nsec(), after.ctime_nsec());
    // Continued metadata access demonstrates the borrowed descriptor stays open.
    assert!(directory.metadata().unwrap().is_dir());
}

#[test]
fn ordinary_file_rejects_as_file_type() {
    let fixture = Fixture::new();
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(fixture.0.join("not-a-directory"))
        .unwrap();
    assert_eq!(available(&file), Err(Error::FileType));
}

#[test]
fn syscall_failure_is_static_and_redacted() {
    assert_eq!(ffi::query(-1), Err(Error::Unavailable));
    assert_eq!(Error::Unavailable.to_string(), "linux_space_unavailable");
}

#[test]
fn block_bytes_use_fragment_size_and_allow_full_filesystem() {
    assert_eq!(
        from_counts(4096, 3, 8, 2),
        Ok(Available {
            allocation_unit: 4096,
            bytes: 12288,
            inodes: Some(2)
        })
    );
    assert_eq!(
        from_counts(4096, 0, 8, 0),
        Ok(Available {
            allocation_unit: 4096,
            bytes: 0,
            inodes: Some(0)
        })
    );
}

#[test]
fn multiplication_overflow_rejects_without_saturation() {
    assert_eq!(from_counts(2, u64::MAX, 8, 2), Err(Error::Overflow));
    assert_eq!(from_counts(u64::MAX, 2, 8, 2), Err(Error::Overflow));
    assert_eq!(from_counts(1, u64::MAX, 8, 2).unwrap().bytes, u64::MAX);
}

#[test]
fn unknown_inode_counts_are_distinct_from_zero() {
    for (files, available) in [(0, 0), (0, 50), (u64::MAX, 2), (8, u64::MAX)] {
        assert_eq!(from_counts(1, 4, files, available).unwrap().inodes, None);
    }
    assert_eq!(from_counts(1, 4, 8, 0).unwrap().inodes, Some(0));
}

#[test]
fn invalid_size_and_inconsistent_finite_inode_counts_reject() {
    assert_eq!(from_counts(0, 0, 8, 2), Err(Error::Invalid));
    assert_eq!(from_counts(1, 0, 8, 9), Err(Error::Invalid));
}
