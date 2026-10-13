// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: synthetic guarded Windows CASC consumer tests.
// Modified 2026-10-12: isolate directory replacement from the file fixture.

use super::*;
use crate::local_store::{Limits as StoreLimits, tests::CollisionFixture};
use std::{
    fs,
    io::{Seek, SeekFrom, Write},
    sync::atomic::{AtomicU64, Ordering},
};

struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "rce-windows-casc-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn complete_synthetic_store_bootstrap_admission_and_verified_acquisition() {
    let bytes = b"complete authored Windows CASC payload".to_vec();
    let fixture = CollisionFixture::new(&[(42, None, bytes.clone())]);
    assert_eq!(crate::admit(fixture.path()), Ok(70205));
    let mut store = fixture.open(false);
    let file = store.read_file_data_id(42).unwrap();
    assert_eq!(file.bytes, bytes);
    assert_eq!(file.content_key, crate::blte::md5::digest(&bytes));
    assert!(store.stats().resident_bytes >= super::super::windows_lease_reservation());
    fixture.change_metadata();
    assert!(matches!(
        store.read_file_data_id(42),
        Err(Error::InputChanged)
    ));
}

#[test]
fn snapshot_guards_reject_same_content_replacement_and_directory_changes() {
    let temp = Temp::new();
    let path = temp.0.join("input");
    fs::write(&path, b"same").unwrap();
    let guard = Guard::capture(&path).unwrap().with_bytes(b"same".to_vec());
    guard.check().unwrap();
    fs::rename(&path, temp.0.join("old")).unwrap();
    fs::write(&path, b"same").unwrap();
    assert!(matches!(guard.check(), Err(Error::InputChanged)));
    // Keep the directory replacement independent of the earlier file case:
    // rename a dedicated empty leaf instead of the shared fixture container.
    let directory = temp.0.join("directory");
    let moved = temp.0.join("directory-old");
    fs::create_dir(&directory).unwrap();
    let directory_guard = Guard::capture_directory(&directory).unwrap();
    directory_guard.check().unwrap();
    fs::rename(&directory, &moved).unwrap();
    fs::create_dir(&directory).unwrap();
    assert!(matches!(directory_guard.check(), Err(Error::InputChanged)));
    fs::remove_dir_all(&moved).unwrap();
}

#[test]
fn whole_reads_cross_primitive_chunk_boundary_and_preserve_store_limits() {
    let temp = Temp::new();
    let path = temp.0.join("large");
    let len = MAX_READ_BYTES + 1;
    let mut file = fs::File::create(&path).unwrap();
    file.set_len(len as u64).unwrap();
    file.seek(SeekFrom::Start((MAX_READ_BYTES - 1) as u64))
        .unwrap();
    file.write_all(&[0x61, 0x62]).unwrap();
    drop(file);
    let budget = Budget::new(StoreLimits::default());
    let bytes = whole(&path, len as u64, &budget).unwrap();
    assert_eq!(bytes.len(), len);
    assert_eq!(&bytes[MAX_READ_BYTES - 1..], &[0x61, 0x62]);
    assert!(matches!(
        whole(&path, (len - 1) as u64, &budget),
        Err(Error::Limit)
    ));
    let constrained = Budget::new(StoreLimits {
        max_scratch_bytes: 1,
        ..StoreLimits::default()
    });
    assert!(matches!(
        whole(&path, len as u64, &constrained),
        Err(Error::Limit)
    ));
}

#[test]
fn missing_is_distinct_from_unsupported_paths_and_changed_captured_inputs() {
    let temp = Temp::new();
    assert!(matches!(
        archive(&temp.0.join("absent")),
        Err(PlatformError::Missing)
    ));
    assert!(matches!(
        archive(Path::new("relative")),
        Err(PlatformError::Path)
    ));
    let path = temp.0.join("present");
    fs::write(&path, b"synthetic").unwrap();
    let guard = Guard::capture(&path).unwrap();
    fs::remove_file(&path).unwrap();
    assert!(matches!(guard.check(), Err(Error::InputChanged)));
}
