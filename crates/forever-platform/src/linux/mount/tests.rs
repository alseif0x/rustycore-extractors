// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-16: original pure policy and owned-descriptor mount tests.
use super::*;
use std::{
    fs::{self, OpenOptions},
    io::Read,
    os::{
        fd::AsRawFd,
        unix::fs::{DirBuilderExt, OpenOptionsExt},
    },
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

#[test]
fn conventional_mask_and_positive_id_are_required() {
    assert_eq!(response(0, 0, STATX_MNT_ID, 7), Ok(7));
    assert_eq!(response(0, 0, STATX_MNT_ID | 0x7ff, u64::MAX), Ok(u64::MAX));
    assert_eq!(response(0, 0, STATX_MNT_ID, 0), Err(Error::Invalid));
}

#[test]
fn absent_mask_never_uses_its_dummy_value() {
    for value in [0, 7, u64::MAX] {
        assert_eq!(response(0, 0, 0x7ff, value), Err(Error::MissingMask));
    }
}

#[test]
fn unique_mount_id_domain_is_not_a_conventional_id() {
    for mask in [STATX_MNT_ID_UNIQUE, STATX_MNT_ID | STATX_MNT_ID_UNIQUE] {
        assert_eq!(response(0, 0, mask, 7), Err(Error::Invalid));
    }
}

#[test]
fn unavailable_syscall_is_compatible_fallback_only() {
    assert_eq!(
        response(-1, ENOSYS, STATX_MNT_ID, 7),
        Err(Error::Unsupported)
    );
    assert!(Error::Unsupported.allows_fallback());
    assert!(Error::MissingMask.allows_fallback());
}

#[test]
fn seccomp_eperm_and_unsupported_operation_can_use_strict_reader() {
    for errno in [EPERM, EOPNOTSUPP] {
        assert_eq!(
            response(-1, errno, STATX_MNT_ID, 7),
            Err(Error::Unavailable)
        );
    }
    assert!(Error::Unavailable.allows_fallback());
}

#[test]
fn other_errno_values_are_fatal_without_retry_or_fallback() {
    // EINTR, EIO, EBADF, EACCES, EFAULT, EINVAL and ENOMEM, plus missing errno.
    for errno in [4, 5, 9, 13, 14, 22, 12, 0] {
        assert_eq!(response(-1, errno, STATX_MNT_ID, 7), Err(Error::Query));
    }
    assert!(!Error::Query.allows_fallback());
}

#[test]
fn invalid_return_status_is_fatal_even_with_unavailable_errno() {
    for status in [-2, 1, i32::MAX] {
        assert_eq!(
            response(status, ENOSYS, STATX_MNT_ID, 7),
            Err(Error::Invalid)
        );
    }
    assert!(!Error::Invalid.allows_fallback());
}

static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Fixture {
    root: PathBuf,
    file: File,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "rce-linux-mount-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::DirBuilder::new().mode(0o700).create(&root).unwrap();
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(root.join("authored"))
            .unwrap();
        Self { root, file }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
    }
}

// Independent bounded proc reading is only a test oracle over our own fd.
fn proc_id(file: &File) -> u64 {
    let mut bytes = Vec::new();
    File::open(format!("/proc/self/fdinfo/{}", file.as_raw_fd()))
        .unwrap()
        .take(257)
        .read_to_end(&mut bytes)
        .unwrap();
    assert!(bytes.len() <= 256 && bytes.is_ascii() && bytes.last() == Some(&b'\n'));
    let text = std::str::from_utf8(&bytes).unwrap();
    let mut ids = text.lines().filter_map(|line| line.strip_prefix("mnt_id:"));
    let value = ids.next().unwrap().trim_start_matches([' ', '\t']);
    assert!(!value.is_empty() && value.bytes().all(|b| b.is_ascii_digit()));
    let value: u64 = value.parse().unwrap();
    assert!(value > 0 && ids.next().is_none());
    value
}

fn check_equivalent(file: &File) -> u64 {
    let expected = proc_id(file);
    match id(file) {
        Ok(actual) => assert_eq!(actual, expected),
        Err(error) if error.allows_fallback() => {}
        Err(error) => panic!("mount query failed: {error}"),
    }
    expected
}

#[test]
fn held_owned_file_statx_matches_bounded_proc_when_supported() {
    let fixture = Fixture::new();
    let first = check_equivalent(&fixture.file);
    // Each query is fresh: no captured/cached mount ID in the primitive.
    assert_eq!(check_equivalent(&fixture.file), first);
}

#[test]
fn held_owned_directory_statx_matches_proc_and_its_child_file() {
    let fixture = Fixture::new();
    let directory = File::open(&fixture.root).unwrap();
    assert_eq!(
        check_equivalent(&directory),
        check_equivalent(&fixture.file)
    );
}
