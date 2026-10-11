// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: synthetic Windows filesystem capability tests.
use super::*;
use std::{
    fs,
    io::Write,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Temp(PathBuf, std::cell::Cell<bool>);
impl Temp {
    fn new() -> Self {
        let name = format!(
            "forever-platform-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        );
        let path = std::env::temp_dir().join(name);
        fs::create_dir(&path).unwrap();
        Self(path, std::cell::Cell::new(false))
    }
    fn file(&self) -> PathBuf {
        let path = self.0.join("input");
        fs::write(&path, b"synthetic-content").unwrap();
        path
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
        if self.1.get() {
            let _ = fs::remove_dir_all(self.0.with_extension("moved"));
        }
    }
}

#[test]
fn complete_range_reads_identity_and_handle_count() {
    let tmp = Temp::new();
    let path = tmp.file();
    let reader = ReadFile::open(&path, Limits::default(), LinkPolicy::Reject).unwrap();
    let id = reader.identity();
    assert_ne!(id.file_index, 0);
    assert_eq!(id.links, 1);
    assert_eq!(id.size, 17);
    assert_eq!(
        reader.retained_handles(),
        AbsolutePath::parse(path.to_str().unwrap(), Limits::default())
            .unwrap()
            .ancestor_count()
            + 1
    );
    assert!(reader.retained_handles() <= 64);
    assert_eq!(reader.read_all(17).unwrap(), b"synthetic-content");
    let mut bytes = [0; 7];
    reader.read_exact_at(10, &mut bytes).unwrap();
    assert_eq!(&bytes, b"content");
    assert_eq!(reader.identity(), id);
    assert_eq!(reader.read_all(16), Err(Error::Limit));
    assert_eq!(reader.read_exact_at(17, &mut bytes), Err(Error::Read));
    assert_eq!(
        reader.read_exact_at(u64::MAX, &mut bytes),
        Err(Error::Limit)
    );
}
#[test]
fn writer_delete_and_ancestor_rename_denied_until_drop() {
    let tmp = Temp::new();
    let path = tmp.file();
    let reader = ReadFile::open(&path, Limits::default(), LinkPolicy::Reject).unwrap();
    assert!(OpenOptions::new().write(true).open(&path).is_err());
    assert!(OpenOptions::new().append(true).open(&path).is_err());
    assert!(fs::remove_file(&path).is_err());
    assert!(fs::rename(&path, tmp.0.join("moved")).is_err());
    let ancestor_rename = fs::rename(&tmp.0, tmp.0.with_extension("moved"));
    if ancestor_rename.is_ok() {
        // Track the exact tool-owned moved path for cleanup after a failed assertion.
        tmp.1.set(true);
    }
    assert!(ancestor_rename.is_err());
    reader.verify_unchanged().unwrap();
    drop(reader);
    let mut file = OpenOptions::new().write(true).open(&path).unwrap();
    file.write_all(b"updated").unwrap();
    drop(file);
    fs::rename(&path, tmp.0.join("moved")).unwrap();
}
#[test]
fn existing_writer_blocks_new_lease() {
    let tmp = Temp::new();
    let path = tmp.file();
    let writer = OpenOptions::new()
        .write(true)
        .share_mode(7)
        .open(&path)
        .unwrap();
    assert!(matches!(
        ReadFile::open(&path, Limits::default(), LinkPolicy::Reject),
        Err(Error::Open)
    ));
    drop(writer);
    assert!(ReadFile::open(&path, Limits::default(), LinkPolicy::Reject).is_ok());
}
#[test]
fn unrelated_sibling_changes_do_not_invalidate_ancestors() {
    let tmp = Temp::new();
    let path = tmp.file();
    let reader = ReadFile::open(&path, Limits::default(), LinkPolicy::Reject).unwrap();
    fs::write(tmp.0.join("sibling"), b"other synthetic input").unwrap();
    reader.verify_unchanged().unwrap();
    fs::remove_file(tmp.0.join("sibling")).unwrap();
    reader.verify_unchanged().unwrap();
}
#[test]
fn concurrent_sibling_churn_preserves_ancestor_acquisition_and_object_verification() {
    let tmp = Temp::new();
    let stable = tmp.0.join("stable");
    fs::create_dir(&stable).unwrap();
    let input = stable.join("input");
    fs::write(&input, b"synthetic-content").unwrap();
    let object = ReadDirectory::open(&tmp.0, Limits::default()).unwrap();
    let stop = std::sync::atomic::AtomicBool::new(false);
    let changes = AtomicU64::new(0);
    let started = std::sync::Barrier::new(2);
    let sibling = tmp.0.join("changing-sibling");
    std::thread::scope(|scope| {
        let churn = scope.spawn(|| {
            started.wait();
            while !stop.load(Ordering::Acquire) {
                fs::write(&sibling, b"synthetic sibling").unwrap();
                fs::remove_file(&sibling).unwrap();
                changes.fetch_add(1, Ordering::Relaxed);
            }
        });
        started.wait();
        // Stop and join even on a failed lease result, so a regression cannot
        // leave the churn thread alive or hang scoped-thread teardown.
        let result = (|| -> Result<(), Error> {
            for _ in 0..64 {
                let reader = ReadFile::open(&input, Limits::default(), LinkPolicy::Reject)?;
                if reader.read_all(17)? != b"synthetic-content" {
                    return Err(Error::Changed);
                }
                ReadDirectory::open(&stable, Limits::default())?.verify_unchanged()?;
                object.verify_same_object()?;
            }
            Ok(())
        })();
        stop.store(true, Ordering::Release);
        churn.join().unwrap();
        result.unwrap();
    });
    assert!(changes.load(Ordering::Relaxed) > 1);
    object.verify_same_object().unwrap();
}
#[test]
fn hardlink_policy_explicit_and_identity_shared() {
    let tmp = Temp::new();
    let path = tmp.file();
    let alias = tmp.0.join("alias");
    fs::hard_link(&path, &alias).unwrap();
    assert!(matches!(
        ReadFile::open(&path, Limits::default(), LinkPolicy::Reject),
        Err(Error::HardLinks)
    ));
    let first = ReadFile::open(&path, Limits::default(), LinkPolicy::Allow).unwrap();
    let second = ReadFile::open(&alias, Limits::default(), LinkPolicy::Allow).unwrap();
    assert_eq!(first.identity(), second.identity());
    assert_eq!(first.identity().links, 2);
    assert!(OpenOptions::new().write(true).open(&alias).is_err());
}
#[test]
fn directory_final_and_file_ancestor_rejected() {
    let tmp = Temp::new();
    let path = tmp.file();
    assert!(matches!(
        ReadFile::open(&tmp.0, Limits::default(), LinkPolicy::Reject),
        Err(Error::FileType)
    ));
    assert!(ReadFile::open(&path.join("child"), Limits::default(), LinkPolicy::Reject).is_err());
}
#[test]
fn reparse_attributes_reject_before_type_or_link_policy() {
    let id = Identity {
        volume: 1,
        file_index: 1,
        links: 1,
        attributes: FILE_ATTRIBUTE_REPARSE_POINT,
        size: 0,
        creation_time: 0,
        write_time: 0,
        change_time: 0,
    };
    assert_eq!(admit(id, false, LinkPolicy::Allow), Err(Error::Reparse));
    assert_eq!(
        admit(
            Identity {
                attributes: FILE_ATTRIBUTE_DIRECTORY | FILE_ATTRIBUTE_REPARSE_POINT,
                ..id
            },
            true,
            LinkPolicy::Allow
        ),
        Err(Error::Reparse)
    );
}
#[test]
#[ignore = "requires actual Windows symlink privilege and reparse implementation; run explicitly"]
fn actual_file_and_ancestor_symlinks_rejected() {
    let tmp = Temp::new();
    let path = tmp.file();
    let alias = tmp.0.join("alias");
    std::os::windows::fs::symlink_file(&path, &alias).unwrap();
    assert!(matches!(
        ReadFile::open(&alias, Limits::default(), LinkPolicy::Allow),
        Err(Error::Reparse)
    ));
    let child = tmp.0.join("child");
    fs::create_dir(&child).unwrap();
    fs::write(child.join("file"), b"synthetic").unwrap();
    let link = tmp.0.join("directory-link");
    std::os::windows::fs::symlink_dir(&child, &link).unwrap();
    assert!(matches!(
        ReadFile::open(&link.join("file"), Limits::default(), LinkPolicy::Reject),
        Err(Error::Reparse)
    ));
    fs::remove_file(alias).unwrap();
    fs::remove_dir(link).unwrap();
}

#[test]
fn observed_identity_mismatch_poison_survives_restoration() {
    let tmp = Temp::new();
    let path = tmp.file();
    let mut reader = ReadFile::open(&path, Limits::default(), LinkPolicy::Reject).unwrap();
    let original = reader.leaf.identity;
    // Synthetic expected-snapshot corruption exercises permanent invalidation;
    // this is not evidence that a writable external handle bypasses sharing.
    reader.leaf.identity.file_index ^= 1;
    assert_eq!(reader.verify_unchanged(), Err(Error::Changed));
    reader.leaf.identity = original;
    assert_eq!(reader.verify_unchanged(), Err(Error::Changed));
    assert_eq!(reader.read_all(100), Err(Error::Changed));
}
#[test]
fn explicit_read_limit_is_enforced_without_mutating_lease() {
    let tmp = Temp::new();
    let path = tmp.file();
    let reader = ReadFile::open(
        &path,
        Limits {
            max_read_bytes: 3,
            ..Limits::default()
        },
        LinkPolicy::Reject,
    )
    .unwrap();
    assert_eq!(reader.read_all(17), Err(Error::Limit));
    assert_eq!(reader.read_all(3), Err(Error::Limit));
    let mut bytes = [0; 3];
    reader.read_exact_at(0, &mut bytes).unwrap();
    assert_eq!(&bytes, b"syn");
    assert_eq!(reader.read_exact_at(0, &mut [0; 4]), Err(Error::Limit));
    reader.verify_unchanged().unwrap();
}
#[test]
fn ancestor_metadata_changes_do_not_change_object_comparison() {
    let first = Identity {
        volume: 1,
        file_index: 7,
        links: 1,
        attributes: FILE_ATTRIBUTE_DIRECTORY,
        size: 5,
        creation_time: 1,
        write_time: 2,
        change_time: 3,
    };
    assert!(first.same_object(Identity {
        size: 99,
        write_time: 9,
        change_time: 8,
        links: 3,
        ..first
    }));
    assert!(!first.same_object(Identity {
        file_index: 8,
        ..first
    }));
    assert!(!first.same_object(Identity { volume: 2, ..first }));
    assert!(!first.same_object(Identity {
        attributes: FILE_ATTRIBUTE_DIRECTORY | FILE_ATTRIBUTE_REPARSE_POINT,
        ..first
    }));
}

#[test]
fn ancestor_delete_access_denied_directly_not_via_leaf_rename() {
    let tmp = Temp::new();
    let path = tmp.file();
    let open_delete = || {
        OpenOptions::new()
            .access_mode(0x0001_0000)
            .share_mode(7)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
            .open(&tmp.0)
    };
    let reader = ReadFile::open(&path, Limits::default(), LinkPolicy::Reject).unwrap();
    assert!(open_delete().is_err());
    reader.verify_unchanged().unwrap();
    drop(reader);
    assert!(open_delete().is_ok());
}
#[test]
fn injected_eof_probe_failure_permanently_invalidates_lease() {
    let tmp = Temp::new();
    let path = tmp.file();
    let reader = ReadFile::open(&path, Limits::default(), LinkPolicy::Reject).unwrap();
    let failure = std::io::Error::from(std::io::ErrorKind::Other);
    assert_eq!(reader.check_eof_result(Err(failure)), Err(Error::Read));
    assert_eq!(reader.verify_unchanged(), Err(Error::Changed));
    assert_eq!(reader.read_all(17), Err(Error::Changed));
}

#[test]
fn directory_lease_checks_identity_and_denies_direct_delete_access() {
    let tmp = Temp::new();
    let lease = ReadDirectory::open(&tmp.0, Limits::default()).unwrap();
    let snapshot = lease.identity();
    assert_ne!(snapshot.attributes & FILE_ATTRIBUTE_DIRECTORY, 0);
    assert_eq!(
        lease.retained_handles(),
        AbsolutePath::parse(tmp.0.to_str().unwrap(), Limits::default())
            .unwrap()
            .ancestor_count()
            + 1
    );
    lease.verify_unchanged().unwrap();
    lease.verify_same_object().unwrap();
    let open_delete = || {
        OpenOptions::new()
            .access_mode(0x0001_0000)
            .share_mode(7)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
            .open(&tmp.0)
    };
    assert!(open_delete().is_err());
    drop(lease);
    assert!(open_delete().is_ok());
}
#[test]
fn directory_object_policy_explicitly_allows_sibling_entry_changes() {
    let tmp = Temp::new();
    let lease = ReadDirectory::open(&tmp.0, Limits::default()).unwrap();
    let initial = lease.identity();
    fs::write(tmp.0.join("new-child"), b"synthetic").unwrap();
    lease.verify_same_object().unwrap();
    let current = ReadDirectory::open(&tmp.0, Limits::default())
        .unwrap()
        .identity();
    assert!(initial.same_object(current));
    fs::remove_file(tmp.0.join("new-child")).unwrap();
    lease.verify_same_object().unwrap();
}
#[test]
fn directory_full_snapshot_rejects_metadata_change_and_poison_is_permanent() {
    let tmp = Temp::new();
    let mut lease = ReadDirectory::open(&tmp.0, Limits::default()).unwrap();
    // Controlled expected-snapshot mismatch isolates full comparator semantics;
    // filesystem sibling timestamp update timing is not assumed.
    let original = lease.inner.leaf.identity;
    lease.inner.leaf.identity.write_time ^= 1;
    assert_eq!(lease.verify_unchanged(), Err(Error::Changed));
    lease.inner.leaf.identity = original;
    assert_eq!(lease.verify_same_object(), Err(Error::Changed));
}
#[test]
fn directory_snapshot_is_transient_and_rejects_files_and_roots() {
    let tmp = Temp::new();
    let path = tmp.file();
    let first = {
        ReadDirectory::open(&tmp.0, Limits::default())
            .unwrap()
            .identity()
    };
    let second = {
        ReadDirectory::open(&tmp.0, Limits::default())
            .unwrap()
            .identity()
    };
    assert_eq!(first, second);
    assert!(matches!(
        ReadDirectory::open(&path, Limits::default()),
        Err(Error::FileType)
    ));
    assert!(matches!(
        ReadDirectory::open(Path::new("C:\\"), Limits::default()),
        Err(Error::Path)
    ));
}
#[test]
fn missing_open_is_distinct_from_type_or_read_failures() {
    let tmp = Temp::new();
    let missing = tmp.0.join("missing");
    assert!(matches!(
        ReadFile::open(&missing, Limits::default(), LinkPolicy::Reject),
        Err(Error::Missing)
    ));
    assert!(matches!(
        ReadDirectory::open(&missing, Limits::default()),
        Err(Error::Missing)
    ));
    assert!(matches!(
        ReadFile::open(
            &missing.join("child"),
            Limits::default(),
            LinkPolicy::Reject
        ),
        Err(Error::Missing)
    ));
}
#[test]
fn operator_paths_are_exact_and_do_not_access_or_normalize_filesystem() {
    let normal = Path::new("C:\\not-present-component\\synthetic.db2");
    assert_eq!(operator_path(normal, Limits::default()).unwrap(), normal);
    for invalid in [
        "relative",
        "C:\\a\\..\\b",
        "\\\\?\\C:\\x",
        "\\\\server\\share\\file",
        "C:/file",
        "C:\\",
    ] {
        assert_eq!(
            operator_path(Path::new(invalid), Limits::default()),
            Err(Error::Path)
        );
    }
}

#[test]
fn directory_full_snapshot_detects_actual_write_attribute_change() {
    let tmp = Temp::new();
    let lease = ReadDirectory::open(&tmp.0, Limits::default()).unwrap();
    // Metadata-only access is not write-data sharing. Changing directory times
    // remains permitted and must be detected by full-snapshot verification.
    let metadata_writer = OpenOptions::new()
        .access_mode(0x100)
        .share_mode(7)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
        .open(&tmp.0)
        .unwrap();
    metadata_writer
        .set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_secs(946_684_800))
        .unwrap();
    drop(metadata_writer);
    let changed = ReadDirectory::open(&tmp.0, Limits::default())
        .unwrap()
        .identity();
    assert!(lease.identity().same_object(changed));
    assert_ne!(lease.identity().write_time, changed.write_time);
    lease.verify_same_object().unwrap();
    assert_eq!(lease.verify_unchanged(), Err(Error::Changed));
    assert_eq!(lease.verify_same_object(), Err(Error::Changed));
}

#[test]
fn file_full_snapshot_still_detects_actual_write_attribute_change() {
    let tmp = Temp::new();
    let path = tmp.file();
    let reader = ReadFile::open(&path, Limits::default(), LinkPolicy::Reject).unwrap();
    let metadata_writer = OpenOptions::new()
        .access_mode(0x100)
        .share_mode(7)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
        .open(&path)
        .unwrap();
    metadata_writer
        .set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_secs(946_684_800))
        .unwrap();
    drop(metadata_writer);
    assert_eq!(reader.verify_unchanged(), Err(Error::Changed));
    assert_eq!(reader.read_all(17), Err(Error::Changed));
}

#[test]
fn explicit_eof_probes_empty_and_chunked_complete_files() {
    let tmp = Temp::new();
    let path = tmp.0.join("eof");
    for content in [b"".as_slice(), b"synthetic EOF content"] {
        fs::write(&path, content).unwrap();
        let lease = ReadFile::open(
            &path,
            Limits {
                max_read_bytes: 3,
                ..Limits::default()
            },
            LinkPolicy::Reject,
        )
        .unwrap();
        for (i, expected) in content.chunks(3).enumerate() {
            let mut bytes = [0; 3];
            lease
                .read_exact_at((i * 3) as u64, &mut bytes[..expected.len()])
                .unwrap();
            assert_eq!(&bytes[..expected.len()], expected);
        }
        lease.verify_eof().unwrap();
        lease.verify_eof().unwrap();
    }
}
#[test]
fn explicit_eof_preserves_observed_extent_failure_poison() {
    let tmp = Temp::new();
    let path = tmp.file();
    let lease = ReadFile::open(&path, Limits::default(), LinkPolicy::Reject).unwrap();
    assert_eq!(lease.check_eof_result(Ok(1)), Err(Error::Changed));
    assert_eq!(lease.verify_eof(), Err(Error::Changed));
    assert_eq!(lease.verify_unchanged(), Err(Error::Changed));
}
#[test]
fn explicit_eof_preserves_observed_read_failure_poison() {
    let tmp = Temp::new();
    let path = tmp.file();
    let lease = ReadFile::open(&path, Limits::default(), LinkPolicy::Reject).unwrap();
    assert_eq!(
        lease.check_eof_result(Err(std::io::Error::other("synthetic"))),
        Err(Error::Read)
    );
    assert_eq!(lease.verify_eof(), Err(Error::Changed));
}
