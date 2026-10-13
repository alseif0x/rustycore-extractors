// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: temporary MAPS-root identities, bounds and absence checks.
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
        let mut s = RootedSource::new(&t.0, FsLimits::default()).unwrap();
        read(&mut s, 44).unwrap();
        if remove {
            fs::remove_file(t.0.join(name())).unwrap();
        } else {
            // Exercise metadata verification without relying on clock granularity.
            // Post-read verification does not rehash contents or freeze the filesystem.
            let changed = t.0.join(name());
            let captured = fs::metadata(&changed).unwrap().modified().unwrap();
            fs::write(&changed, b"modified").unwrap();
            fs::File::open(&changed)
                .unwrap()
                .set_times(
                    fs::FileTimes::new().set_modified(captured + std::time::Duration::from_secs(1)),
                )
                .unwrap();
        }
        assert_eq!(s.verify_unchanged(), Err(SourceError::InputChanged));
    }
}
#[test]
fn symlink_file_and_symlink_root_are_rejected() {
    let t = Temp::new();
    fs::write(t.0.join("real"), b"data").unwrap();
    std::os::unix::fs::symlink("real", t.0.join(name())).unwrap();
    let mut s = RootedSource::new(&t.0, FsLimits::default()).unwrap();
    assert_eq!(read(&mut s, 44), Err(SourceError::Io));
    std::os::unix::fs::symlink(&t.0, t.0.join("alias")).unwrap();
    assert!(matches!(
        RootedSource::new(&t.0.join("alias"), FsLimits::default()),
        Err(SourceError::Io)
    ));
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
    for l in [
        FsLimits {
            max_entries: 0,
            ..Default::default()
        },
        FsLimits {
            max_verify_steps: 0,
            ..Default::default()
        },
    ] {
        let mut s = RootedSource::new(&t.0, l).unwrap();
        assert_eq!(read(&mut s, 100), Err(SourceError::Limit));
    }
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
    let mut s = RootedSource::new(&t.0, FsLimits::default()).unwrap();
    read(&mut s, 44).unwrap();
    fs::write(t.0.join("other"), b"unrelated").unwrap();
    changed_mtime(&t.0);
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

// Metadata-only verification requires a distinct observation; do not depend on
// the filesystem clock advancing between a capture and a synthetic mutation.
fn changed_mtime(path: &Path) {
    let time = fs::metadata(path).unwrap().modified().unwrap();
    fs::File::open(path)
        .unwrap()
        .set_times(fs::FileTimes::new().set_modified(time + std::time::Duration::from_secs(1)))
        .unwrap();
}
