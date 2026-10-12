// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: temporary MAPS-root identities, bounds and absence checks.
// Modified 2026-10-12: give mutation fixtures a distinct pre-capture write time.
use super::super::{Artifact, Memory};
use super::*;
use crate::tilebuilder::Coordinate;
use std::sync::atomic::{AtomicU64, Ordering};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Temp(std::path::PathBuf);
impl Temp {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "rce-maps-source-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        Self(p)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
// These tests need an observable metadata change. Set the fixture's write time
// well in the past BEFORE any readonly lease, and close the attribute writer.
// SetFileTime requires FILE_WRITE_ATTRIBUTES; directories need BACKUP_SEMANTICS.
// https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-setfiletime
fn seed_write_time(path: &Path) {
    use std::os::windows::fs::OpenOptionsExt;
    let past = std::time::UNIX_EPOCH + std::time::Duration::from_secs(86_400);
    let directory = fs::OpenOptions::new()
        .access_mode(0x0100) // FILE_WRITE_ATTRIBUTES
        .share_mode(7)
        .custom_flags(0x0200_0000) // FILE_FLAG_BACKUP_SEMANTICS
        .open(path)
        .unwrap();
    directory
        .set_times(fs::FileTimes::new().set_modified(past))
        .unwrap();
    drop(directory);
    assert_eq!(fs::metadata(path).unwrap().modified().unwrap(), past);
}
fn artifact() -> Artifact {
    Artifact {
        map_id: 70205,
        tile: Coordinate {
            row: 24,
            column: 39,
        },
    }
}
fn name() -> &'static str {
    "70205_24_39.map"
}
fn read(s: &mut RootedSource, cap: usize) -> Result<(Presence, Vec<u8>), SourceError> {
    let mut m = Memory {
        live: 0,
        peak: 0,
        limit: 2 * cap,
    };
    let mut b = ReadBuffer {
        bytes: Vec::new(),
        limit: cap,
        memory: &mut m,
    };
    let p = s.read(artifact(), &mut b)?;
    Ok((p, b.bytes))
}
#[test]
fn complete_reads_hash_and_reuse_identity_across_chunk_boundaries() {
    let t = Temp::new();
    let b: Vec<_> = (0..33000).map(|n| (n % 251) as u8).collect();
    fs::write(t.0.join(name()), &b).unwrap();
    let mut s = RootedSource::new(&t.0, FsLimits::default()).unwrap();
    for _ in 0..2 {
        assert_eq!(
            read(&mut s, b.len()).unwrap(),
            (Presence::Present, b.clone())
        );
    }
    s.verify_unchanged().unwrap();
    assert_eq!(s.stats().entries, 1);
    assert_eq!(s.stats().read_bytes, 66000);
}
#[test]
fn genuine_absence_is_recorded_and_creation_poisoned() {
    let t = Temp::new();
    seed_write_time(&t.0);
    let mut s = RootedSource::new(&t.0, FsLimits::default()).unwrap();
    assert_eq!(read(&mut s, 44).unwrap(), (Presence::Missing, vec![]));
    fs::write(t.0.join(name()), b"new").unwrap();
    assert_eq!(s.verify_unchanged(), Err(SourceError::InputChanged));
    assert!(s.failed());
    assert_eq!(read(&mut s, 44), Err(SourceError::InputChanged));
}
#[test]
fn changed_contents_and_deleted_files_poison_recorded_snapshots() {
    for remove in [false, true] {
        let t = Temp::new();
        fs::write(t.0.join(name()), b"original").unwrap();
        seed_write_time(&t.0.join(name()));
        seed_write_time(&t.0);
        let mut s = RootedSource::new(&t.0, FsLimits::default()).unwrap();
        read(&mut s, 44).unwrap();
        if remove {
            fs::remove_file(t.0.join(name())).unwrap();
        } else {
            fs::write(t.0.join(name()), b"modified").unwrap();
        }
        assert_eq!(s.verify_unchanged(), Err(SourceError::InputChanged));
    }
}

#[test]
fn wrong_file_type_and_invalid_coordinates_reject() {
    let t = Temp::new();
    fs::create_dir(t.0.join(name())).unwrap();
    let mut s = RootedSource::new(&t.0, FsLimits::default()).unwrap();
    assert_eq!(read(&mut s, 44), Err(SourceError::Io));
    let mut m = Memory {
        live: 0,
        peak: 0,
        limit: 100,
    };
    let mut b = ReadBuffer {
        bytes: Vec::new(),
        limit: 44,
        memory: &mut m,
    };
    assert_eq!(
        s.read(
            Artifact {
                map_id: 1,
                tile: Coordinate { row: 64, column: 0 }
            },
            &mut b
        ),
        Err(SourceError::Io)
    );
}
#[test]
fn file_byte_metadata_and_verification_limits_only_reduce() {
    let t = Temp::new();
    fs::write(t.0.join(name()), [0u8; 100]).unwrap();
    let mut s = RootedSource::new(&t.0, FsLimits::default()).unwrap();
    assert_eq!(read(&mut s, 99), Err(SourceError::Limit));
    let mut s = RootedSource::new(
        &t.0,
        FsLimits {
            max_entries: 0,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(read(&mut s, 100), Err(SourceError::Limit));
    assert!(matches!(
        RootedSource::new(
            &t.0,
            FsLimits {
                max_entries: MAX_ENTRIES + 1,
                ..Default::default()
            }
        ),
        Err(SourceError::Limit)
    ));
}
#[test]
fn unrelated_root_entry_mutation_invalidates_inventory_identity() {
    let t = Temp::new();
    fs::write(t.0.join(name()), b"data").unwrap();
    seed_write_time(&t.0);
    let mut s = RootedSource::new(&t.0, FsLimits::default()).unwrap();
    read(&mut s, 44).unwrap();
    fs::write(t.0.join("other"), b"unrelated").unwrap();
    assert_eq!(s.verify_unchanged(), Err(SourceError::InputChanged));
}
#[test]
fn dotted_root_aliases_reject_and_empty_complete_file_is_present() {
    let t = Temp::new();
    fs::write(t.0.join(name()), []).unwrap();
    assert!(matches!(
        RootedSource::new(&t.0.join(".."), FsLimits::default()),
        Err(SourceError::Io)
    ));
    let mut s = RootedSource::new(&t.0, FsLimits::default()).unwrap();
    assert_eq!(read(&mut s, 0).unwrap(), (Presence::Present, vec![]));
}

#[test]
fn absolute_paths_exact_case_and_work_admission() {
    let t = Temp::new();
    assert!(RootedSource::new(Path::new("relative"), Default::default()).is_err());
    assert!(matches!(
        RootedSource::new(
            &t.0,
            FsLimits {
                max_verify_steps: 0,
                ..Default::default()
            }
        ),
        Err(SourceError::Limit)
    ));
    fs::write(t.0.join("70205_24_39.MAP"), b"case alias").unwrap();
    let mut s = RootedSource::new(&t.0, Default::default()).unwrap();
    assert_eq!(read(&mut s, 44), Err(SourceError::Io));
}

#[test]
fn observed_open_changes_and_recorded_artifact_failures_win_over_guard_limits() {
    let dir = Temp::new();
    fs::write(dir.0.join(name()), b"x").unwrap();
    let key = request(artifact()).unwrap().full;
    for (recorded, error) in [
        (false, PlatformError::Changed),
        (false, PlatformError::Read),
        (true, PlatformError::Open),
        (true, PlatformError::FileType),
        (true, PlatformError::Reparse),
    ] {
        let mut source = RootedSource::new(&dir.0, Default::default()).unwrap();
        if recorded {
            read(&mut source, 1).unwrap();
        }
        let mut work = Work { used: 0, cap: 0 };
        let result = source.failed_open(&key, error, None, &mut work);
        assert!(matches!(result, Err(SourceError::InputChanged)));
        assert_eq!(work.used, 0);
        assert!(matches!(
            source.finish_operation(result, work),
            Err(SourceError::InputChanged)
        ));
        assert!(source.failed());
        assert_eq!(source.verify_unchanged(), Err(SourceError::InputChanged));
    }
    for (recorded, error) in [
        (false, PlatformError::Open),
        (true, PlatformError::Limit),
        (true, PlatformError::Allocation),
    ] {
        let mut source = RootedSource::new(&dir.0, Default::default()).unwrap();
        if recorded {
            read(&mut source, 1).unwrap();
        }
        let mut work = Work { used: 0, cap: 0 };
        let result = source.failed_open(&key, error, None, &mut work);
        assert!(matches!(
            source.finish_operation(result, work),
            Err(SourceError::Limit)
        ));
        assert!(!source.failed());
    }
}

#[test]
fn protected_read_cleanup_detects_change_after_body_budget_is_exhausted() {
    let t = Temp::new();
    fs::write(t.0.join(name()), b"complete").unwrap();
    seed_write_time(&t.0);
    let mut source = RootedSource::new(&t.0, Default::default()).unwrap();
    let path = source
        .named_path(&request(artifact()).unwrap().full)
        .unwrap();
    let mut work = Work {
        used: 0,
        cap: MAX_VERIFY_STEPS,
    };
    let mut cleanup = source.reserve_after(&path, None, &mut work).unwrap();
    // Exhaust the BODY budget only; final checks were already admitted.
    work.cap = work.used;
    let file = ReadFile::open(&path, PlatformLimits::default(), LinkPolicy::Allow).unwrap();
    fs::write(t.0.join("new-after-capture"), b"other").unwrap();
    let body: Result<(), SourceError> = Err(SourceError::Limit);
    let result = source.finish_read(file, &path, None, &mut cleanup, body);
    assert_eq!(result, Err(SourceError::InputChanged));
    assert_eq!(cleanup.used, cleanup.cap);
    assert_eq!(
        source.finish_operation(result, work),
        Err(SourceError::InputChanged)
    );
    assert!(source.failed());
    assert_eq!(read(&mut source, 20), Err(SourceError::InputChanged));
}
#[test]
fn protected_read_cleanup_preserves_plain_limits_and_observed_changes() {
    let t = Temp::new();
    fs::write(t.0.join(name()), b"complete").unwrap();
    let source = RootedSource::new(&t.0, Default::default()).unwrap();
    let path = source
        .named_path(&request(artifact()).unwrap().full)
        .unwrap();
    for error in [SourceError::Limit, SourceError::InputChanged] {
        let mut work = Work {
            used: 0,
            cap: MAX_VERIFY_STEPS,
        };
        let mut cleanup = source.reserve_after(&path, None, &mut work).unwrap();
        work.cap = work.used;
        assert_eq!(work.used, work.cap);
        let file = ReadFile::open(&path, PlatformLimits::default(), LinkPolicy::Allow).unwrap();
        let result: Result<(), SourceError> =
            source.finish_read(file, &path, None, &mut cleanup, Err(error));
        assert_eq!(result, Err(error));
        assert_eq!(cleanup.used, cleanup.cap);
    }
}
#[test]
fn protected_enumeration_recheck_changes_override_collection_limits() {
    let t = Temp::new();
    seed_write_time(&t.0);
    let lease = ReadDirectory::open(&t.0, PlatformLimits::default()).unwrap();
    fs::write(t.0.join("new-after-enumeration"), b"other").unwrap();
    let result: Result<bool, SourceError> = finish_directory(lease, Err(SourceError::Limit));
    assert_eq!(result, Err(SourceError::InputChanged));
}
#[test]
fn enumeration_limit_cannot_spend_pre_admitted_final_check_work() {
    let t = Temp::new();
    fs::write(t.0.join(name()), b"data").unwrap();
    let source = RootedSource::new(&t.0, Default::default()).unwrap();
    let mut probe = Work {
        used: 0,
        cap: MAX_VERIFY_STEPS,
    };
    platform_work(&t.0, &mut probe).unwrap();
    let mut work = Work {
        used: 0,
        cap: probe.used * 2,
    };
    let result = source.exact_name(
        &t.0,
        source.identity,
        &request(artifact()).unwrap().leaf,
        &mut work,
    );
    assert_eq!(result, Err(SourceError::Limit));
    assert_eq!(work.used, work.cap);
    assert!(!source.failed());
}
#[test]
fn final_read_work_reservation_is_downward_and_precedes_payload_open() {
    let t = Temp::new();
    fs::write(t.0.join(name()), b"data").unwrap();
    let mut source = RootedSource::new(&t.0, Default::default()).unwrap();
    let path = source
        .named_path(&request(artifact()).unwrap().full)
        .unwrap();
    let mut estimate = Work {
        used: 0,
        cap: MAX_VERIFY_STEPS,
    };
    let cleanup = source.reserve_after(&path, None, &mut estimate).unwrap();
    assert_eq!(estimate.used, cleanup.cap);
    let mut below = Work {
        used: 0,
        cap: estimate.used - 1,
    };
    assert!(matches!(
        source.reserve_after(&path, None, &mut below),
        Err(SourceError::Limit)
    ));
    assert_eq!(below.used, 0);
    source.limits.max_verify_steps = 0;
    assert_eq!(read(&mut source, 4), Err(SourceError::Limit));
    assert_eq!(source.stats().read_bytes, 0);
    assert!(!source.failed());
}
