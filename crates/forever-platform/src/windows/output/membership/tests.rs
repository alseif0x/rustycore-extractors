// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-12: original synthetic NTFS membership/parser tests.
use super::*;
use crate::windows::{ReadDirectory, output};
use std::{
    fs::{self, OpenOptions},
    os::windows::fs::OpenOptionsExt,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "forever-membership-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn parent(&self) -> ReadDirectory {
        ReadDirectory::open(&self.0, crate::path::Limits::default()).unwrap()
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn name(text: &str) -> Name {
    Name::parse(text, MAX_NAME).unwrap()
}
fn frame(units: &[u16], file_id: u64, directory: bool) -> Vec<u8> {
    let mut bytes = vec![0; NAME_OFFSET + units.len() * 2];
    bytes[4..8].copy_from_slice(&0xdead_beef_u32.to_le_bytes());
    bytes[56..60].copy_from_slice(&(if directory { 0x10_u32 } else { 0x20_u32 }).to_le_bytes());
    bytes[60..64].copy_from_slice(&(units.len() as u32 * 2).to_le_bytes());
    bytes[72..80].copy_from_slice(&file_id.to_le_bytes());
    for (index, unit) in units.iter().enumerate() {
        bytes[NAME_OFFSET + index * 2..NAME_OFFSET + index * 2 + 2]
            .copy_from_slice(&unit.to_le_bytes());
    }
    bytes
}
fn wire(text: &str, file_id: u64, directory: bool) -> Vec<u8> {
    frame(&text.encode_utf16().collect::<Vec<_>>(), file_id, directory)
}
fn record(text: &str, file_id: u64, directory: bool) -> Record {
    decode(&wire(text, file_id, directory)).unwrap()
}
fn expected() -> [Expected; 4] {
    [
        Expected {
            name: name("root"),
            parent: None,
            file_id: 10,
            directory: true,
        },
        Expected {
            name: name("file"),
            parent: Some(0),
            file_id: 11,
            directory: false,
        },
        Expected {
            name: name("dir"),
            parent: Some(0),
            file_id: 12,
            directory: true,
        },
        Expected {
            name: name("nested"),
            parent: Some(2),
            file_id: 13,
            directory: false,
        },
    ]
}

#[test]
fn empty_root_repeated_scan_restarts() {
    let tmp = Temp::new();
    let parent = tmp.parent();
    let root = PrivateRoot::create(&parent, "private", output::Limits::default()).unwrap();
    let first = verify_membership(&root, Limits::default()).unwrap();
    assert_eq!(first.directories, 1);
    assert_eq!(first.children, 0);
    assert!(first.records <= 2);
    assert_eq!(first.query_calls, first.records + 1);
    assert_eq!(verify_membership(&root, Limits::default()).unwrap(), first);
    assert_eq!(root.stats().operations, 1);
}
#[test]
fn nested_files_and_current_rename_match() {
    let tmp = Temp::new();
    let parent = tmp.parent();
    let mut root = PrivateRoot::create(&parent, "private", output::Limits::default()).unwrap();
    let a = root.create_directory(root.root(), "a").unwrap();
    let b = root.create_directory(a, "b").unwrap();
    let file = root.create_file(root.root(), "first").unwrap();
    root.append(file, b"payload").unwrap();
    root.create_file(a, "empty").unwrap();
    let first = verify_membership(&root, Limits::default()).unwrap();
    assert_eq!(first.directories, 3);
    assert_eq!(first.children, 4);
    root.rename(file, b, "final").unwrap();
    root.append(file, b"more").unwrap();
    let after = verify_membership(&root, Limits::default()).unwrap();
    assert_eq!(after, first);
    assert_eq!(verify_membership(&root, Limits::default()).unwrap(), after);
    root.verify_unchanged().unwrap();
}
#[test]
fn extra_file_rejects_and_poison_survives_removal() {
    let tmp = Temp::new();
    let parent = tmp.parent();
    let root = PrivateRoot::create(&parent, "private", output::Limits::default()).unwrap();
    let path = tmp.0.join("private/extra");
    fs::write(&path, b"unexpected").unwrap();
    root.verify_unchanged().unwrap();
    assert_eq!(
        verify_membership(&root, Limits::default()),
        Err(Error::Extra)
    );
    fs::remove_file(path).unwrap();
    assert_eq!(
        verify_membership(&root, Limits::default()),
        Err(Error::Changed)
    );
    assert_eq!(root.verify_unchanged(), Err(output::Error::Changed));
}
#[test]
fn extra_nested_directory_rejects_and_poison_survives_removal() {
    let tmp = Temp::new();
    let parent = tmp.parent();
    let mut root = PrivateRoot::create(&parent, "private", output::Limits::default()).unwrap();
    let a = root.create_directory(root.root(), "a").unwrap();
    root.create_directory(a, "b").unwrap();
    let path = tmp.0.join("private/a/b/extra");
    fs::create_dir(&path).unwrap();
    root.verify_unchanged().unwrap();
    assert_eq!(
        verify_membership(&root, Limits::default()),
        Err(Error::Extra)
    );
    fs::remove_dir(path).unwrap();
    assert_eq!(
        verify_membership(&root, Limits::default()),
        Err(Error::Changed)
    );
}
#[test]
fn noncanonical_extra_name_rejects() {
    let tmp = Temp::new();
    let parent = tmp.parent();
    let root = PrivateRoot::create(&parent, "private", output::Limits::default()).unwrap();
    fs::write(tmp.0.join("private/UPPER"), b"unexpected").unwrap();
    assert_eq!(
        verify_membership(&root, Limits::default()),
        Err(Error::Name)
    );
    assert_eq!(root.verify_unchanged(), Err(output::Error::Changed));
}
#[test]
fn invalid_limits_do_not_touch_or_poison_changed_dacl() {
    let tmp = Temp::new();
    let parent = tmp.parent();
    let root = PrivateRoot::create(&parent, "private", output::Limits::default()).unwrap();
    let mutation = OpenOptions::new()
        .access_mode(0x0006_0000)
        .share_mode(7)
        .custom_flags(0x0200_0000 | 0x0020_0000)
        .open(tmp.0.join("private"))
        .unwrap();
    output::ffi::test_dacl(&mutation, &root.sid, 0x0012_0089).unwrap();
    assert_eq!(
        output::ffi::private(root.entries[0].file(), &root.sid),
        Err(output::Error::Privacy)
    );
    assert_eq!(
        verify_membership(
            &root,
            Limits {
                max_directories: 129,
                ..Limits::default()
            }
        ),
        Err(Error::Limit)
    );
    assert!(!root.invalid.get());
    output::ffi::test_dacl(&mutation, &root.sid, 0x001f_01ff).unwrap();
    root.verify_unchanged().unwrap();
    verify_membership(&root, Limits::default()).unwrap();
}
#[test]
fn exact_global_budgets_succeed() {
    let tmp = Temp::new();
    let parent = tmp.parent();
    let mut root = PrivateRoot::create(&parent, "private", output::Limits::default()).unwrap();
    let dir = root.create_directory(root.root(), "dir").unwrap();
    root.create_file(dir, "file").unwrap();
    let summary = verify_membership(&root, Limits::default()).unwrap();
    assert_eq!(
        verify_membership(
            &root,
            Limits {
                max_directories: summary.directories,
                max_records: summary.records,
                max_query_calls: summary.query_calls
            }
        ),
        Ok(summary)
    );
}
#[test]
fn directory_budget_exhaustion_poisons() {
    let tmp = Temp::new();
    let parent = tmp.parent();
    let mut root = PrivateRoot::create(&parent, "private", output::Limits::default()).unwrap();
    root.create_directory(root.root(), "dir").unwrap();
    assert_eq!(
        verify_membership(
            &root,
            Limits {
                max_directories: 1,
                ..Limits::default()
            }
        ),
        Err(Error::Budget)
    );
    assert_eq!(
        verify_membership(&root, Limits::default()),
        Err(Error::Changed)
    );
}
#[test]
fn record_budget_exhaustion_poisons() {
    let tmp = Temp::new();
    let parent = tmp.parent();
    let mut root = PrivateRoot::create(&parent, "private", output::Limits::default()).unwrap();
    root.create_file(root.root(), "one").unwrap();
    root.create_file(root.root(), "two").unwrap();
    let summary = verify_membership(&root, Limits::default()).unwrap();
    assert!(summary.records >= 2);
    assert_eq!(
        verify_membership(
            &root,
            Limits {
                max_records: summary.records - 1,
                ..Limits::default()
            }
        ),
        Err(Error::Budget)
    );
    assert_eq!(root.verify_unchanged(), Err(output::Error::Changed));
}
#[test]
fn query_budget_exhaustion_poisons() {
    let tmp = Temp::new();
    let parent = tmp.parent();
    let mut root = PrivateRoot::create(&parent, "private", output::Limits::default()).unwrap();
    root.create_file(root.root(), "file").unwrap();
    let summary = verify_membership(&root, Limits::default()).unwrap();
    assert_eq!(
        verify_membership(
            &root,
            Limits {
                max_query_calls: summary.query_calls - 1,
                ..Limits::default()
            }
        ),
        Err(Error::Budget)
    );
    assert_eq!(
        verify_membership(&root, Limits::default()),
        Err(Error::Changed)
    );
}
#[test]
fn full_directory_cap_succeeds() {
    let tmp = Temp::new();
    let parent = tmp.parent();
    let mut root = PrivateRoot::create(&parent, "private", output::Limits::default()).unwrap();
    for index in 1..128 {
        root.create_directory(root.root(), &format!("d{index}"))
            .unwrap();
    }
    let summary = verify_membership(&root, Limits::default()).unwrap();
    assert_eq!(summary.directories, 128);
    assert_eq!(summary.children, 127);
    assert!(summary.records <= 383);
    assert_eq!(summary.query_calls, summary.records + summary.directories);
    assert_eq!(
        verify_membership(
            &root,
            Limits {
                max_directories: summary.directories,
                max_records: summary.records,
                max_query_calls: summary.query_calls
            }
        ),
        Ok(summary)
    );
}
#[test]
fn existing_v1_poison_rejects_membership_scan() {
    let tmp = Temp::new();
    let parent = tmp.parent();
    let root = PrivateRoot::create(&parent, "private", output::Limits::default()).unwrap();
    let mutation = OpenOptions::new()
        .access_mode(0x0006_0000)
        .share_mode(7)
        .custom_flags(0x0200_0000 | 0x0020_0000)
        .open(tmp.0.join("private"))
        .unwrap();
    output::ffi::test_dacl(&mutation, &root.sid, 0x0012_0089).unwrap();
    assert!(root.verify_unchanged().is_err());
    output::ffi::test_dacl(&mutation, &root.sid, 0x001f_01ff).unwrap();
    assert_eq!(
        verify_membership(&root, Limits::default()),
        Err(Error::Changed)
    );
}

#[test]
fn decoder_reads_file_id_not_undefined_file_index() {
    let bytes = wire("file", 0xfedc_ba98_7654_3210, false);
    let record = decode(&bytes).unwrap();
    assert_eq!(record.file_id, 0xfedc_ba98_7654_3210);
    assert!(!record.directory);
    assert!(matches!(record.name, RecordName::Child(value) if value == name("file")));
    let mut zero = bytes;
    zero[72..80].fill(0);
    assert_eq!(decode(&zero).err(), Some(Error::Identity));
}
#[test]
fn decoder_accepts_exact_and_aligned_native_extents() {
    for text in ["a", "ab", "abc", "abcd", ".", ".."] {
        let mut bytes = wire(text, 7, true);
        decode(&bytes).unwrap();
        let padded = (bytes.len() + 7) & !7;
        bytes.resize(padded, 0xa5);
        bytes[68..72].fill(0xa5);
        // Native structure padding is opaque, not a zero-padding contract.
        decode(&bytes).unwrap();
    }
}
#[test]
fn decoder_rejects_short_linked_and_trailing_frames() {
    let bytes = wire("a", 7, false);
    for length in 0..bytes.len() {
        assert_eq!(decode(&bytes[..length]).err(), Some(Error::Framing));
    }
    let mut linked = bytes.clone();
    linked[0..4].copy_from_slice(&88_u32.to_le_bytes());
    assert_eq!(decode(&linked).err(), Some(Error::Framing));
    let mut trailing = bytes.clone();
    trailing.push(0);
    assert_eq!(decode(&trailing).err(), Some(Error::Framing));
    let mut aligned = bytes;
    aligned.resize(96, 0);
    assert_eq!(decode(&aligned).err(), Some(Error::Framing));
    assert_eq!(
        decode(&vec![0; BUFFER_BYTES + 1]).err(),
        Some(Error::Framing)
    );
}
#[test]
fn decoder_rejects_invalid_name_lengths_and_utf16() {
    for length in [0, 1, 3, 511, 512, u32::MAX] {
        let mut bytes = wire("valid", 7, false);
        bytes[60..64].copy_from_slice(&length.to_le_bytes());
        assert_eq!(decode(&bytes).err(), Some(Error::Framing));
    }
    for units in [
        vec![0xd800],
        vec![0xdc00],
        vec![0xd800, 0x61],
        vec![0x61, 0xdc00],
    ] {
        assert_eq!(decode(&frame(&units, 7, false)).err(), Some(Error::Framing));
    }
    assert_eq!(
        decode(&frame(&[0xd800, 0xdc00], 7, false)).err(),
        Some(Error::Name)
    );
}
#[test]
fn decoder_rejects_foreign_names_and_reparse() {
    for text in [
        "UPPER", ".git", "con", "com1.bin", "a:", "a/b", "a\\b", "a.", "a ", "a~1", "é", "a\0",
    ] {
        assert_eq!(decode(&wire(text, 7, false)).err(), Some(Error::Name));
    }
    let mut reparse = wire("file", 7, false);
    reparse[56..60].copy_from_slice(&0x400_u32.to_le_bytes());
    assert_eq!(decode(&reparse).err(), Some(Error::Reparse));
}
#[test]
fn decoder_bounds_owned_and_full_foreign_names() {
    let owned = "a".repeat(64);
    assert!(decode(&wire(&owned, 7, false)).is_ok());
    let foreign = "a".repeat(255);
    let bytes = wire(&foreign, 7, false);
    assert!(bytes.len() <= BUFFER_BYTES);
    assert_eq!(decode(&bytes).err(), Some(Error::Name));
    let excessive = "a".repeat(256);
    assert_eq!(
        decode(&wire(&excessive, 7, false)).err(),
        Some(Error::Framing)
    );
}
#[test]
fn matcher_requires_exact_parent_name_identity_and_type() {
    let inventory = expected();
    let mut seen = [false; MAX_OBJECTS];
    let mut dots = [false; 2];
    assert_eq!(
        accept(
            &inventory,
            0,
            1,
            record("file", 99, false),
            &mut seen,
            &mut dots
        ),
        Err(Error::Identity)
    );
    assert_eq!(
        accept(
            &inventory,
            0,
            1,
            record("file", 11, true),
            &mut seen,
            &mut dots
        ),
        Err(Error::FileType)
    );
    assert_eq!(
        accept(
            &inventory,
            0,
            1,
            record("nested", 13, false),
            &mut seen,
            &mut dots
        ),
        Err(Error::Extra)
    );
    assert_eq!(
        accept(
            &inventory,
            2,
            1,
            record("file", 11, false),
            &mut seen,
            &mut dots
        ),
        Err(Error::Extra)
    );
    assert_eq!(
        accept(
            &inventory,
            0,
            1,
            record("unknown", 11, false),
            &mut seen,
            &mut dots
        ),
        Err(Error::Extra)
    );
    assert_eq!(
        accept(
            &inventory,
            0,
            1,
            record("file", 11, false),
            &mut seen,
            &mut dots
        ),
        Ok(true)
    );
    assert_eq!(
        accept(
            &inventory,
            0,
            1,
            record("dir", 12, true),
            &mut seen,
            &mut dots
        ),
        Ok(true)
    );
    assert_eq!(complete(&inventory, 0, &seen), Ok(()));
}
#[test]
fn matcher_rejects_duplicates_and_missing_children() {
    let inventory = expected();
    let mut seen = [false; MAX_OBJECTS];
    let mut dots = [false; 2];
    assert_eq!(complete(&inventory, 0, &seen), Err(Error::Missing));
    assert_eq!(
        accept(
            &inventory,
            0,
            1,
            record("file", 11, false),
            &mut seen,
            &mut dots
        ),
        Ok(true)
    );
    assert_eq!(
        accept(
            &inventory,
            0,
            1,
            record("file", 11, false),
            &mut seen,
            &mut dots
        ),
        Err(Error::Duplicate)
    );
    assert_eq!(complete(&inventory, 0, &seen), Err(Error::Missing));
    assert_eq!(complete(&inventory, 2, &seen), Err(Error::Missing));
    assert_eq!(
        accept(
            &inventory,
            2,
            1,
            record("nested", 13, false),
            &mut seen,
            &mut dots
        ),
        Ok(true)
    );
    assert_eq!(complete(&inventory, 2, &seen), Ok(()));
}
#[test]
fn dot_entries_require_self_parent_identity_and_directory_type() {
    let inventory = expected();
    let mut seen = [false; MAX_OBJECTS];
    let mut dots = [false; 2];
    assert_eq!(
        accept(
            &inventory,
            0,
            1,
            record(".", 10, false),
            &mut seen,
            &mut dots
        ),
        Err(Error::FileType)
    );
    assert_eq!(
        accept(
            &inventory,
            0,
            1,
            record(".", 99, true),
            &mut seen,
            &mut dots
        ),
        Err(Error::Identity)
    );
    assert_eq!(
        accept(
            &inventory,
            0,
            1,
            record(".", 10, true),
            &mut seen,
            &mut dots
        ),
        Ok(false)
    );
    assert_eq!(
        accept(
            &inventory,
            0,
            1,
            record(".", 10, true),
            &mut seen,
            &mut dots
        ),
        Err(Error::Duplicate)
    );
    assert_eq!(
        accept(
            &inventory,
            0,
            1,
            record("..", 10, true),
            &mut seen,
            &mut dots
        ),
        Err(Error::Identity)
    );
    assert_eq!(
        accept(
            &inventory,
            0,
            1,
            record("..", 1, true),
            &mut seen,
            &mut dots
        ),
        Ok(false)
    );
    assert_eq!(
        accept(
            &inventory,
            0,
            1,
            record("..", 1, true),
            &mut seen,
            &mut dots
        ),
        Err(Error::Duplicate)
    );
    let mut dots = [false; 2];
    assert_eq!(
        accept(
            &inventory,
            2,
            1,
            record("..", 10, true),
            &mut seen,
            &mut dots
        ),
        Ok(false)
    );
    assert_eq!(
        accept(
            &inventory,
            2,
            1,
            record(".", 12, true),
            &mut seen,
            &mut dots
        ),
        Ok(false)
    );
}
#[test]
fn native_completion_requires_exact_status_length_contract() {
    assert_eq!(completion(0, 0, BUFFER_BYTES), Ok(Some(BUFFER_BYTES)));
    assert_eq!(completion(0x8000_0006, 0x8000_0006, 0), Ok(None));
    for (status, io, length) in [
        (0, 0, 0),
        (0, 0, BUFFER_BYTES + 1),
        (0, 1, 88),
        (0x8000_0006, 0, 0),
        (0x8000_0006, 0x8000_0006, 1),
        (0x8000_0005, 0x8000_0005, 88),
        (0x103, 0x103, 0),
        (0x103, 0, 88),
        (0xc000_0001, 0xc000_0001, 0),
        (0x105, 0x105, 0),
    ] {
        assert_eq!(completion(status, io, length), Err(Error::Query));
    }
    // Positive extent alone is not acceptance; the safe decoder must frame it.
    assert_eq!(completion(0, 0, 1), Ok(Some(1)));
    assert_eq!(decode(&[0]).err(), Some(Error::Framing));
}
#[test]
fn downward_limits_and_static_diagnostics() {
    let defaults = Limits::default();
    assert_eq!(defaults.max_directories, 128);
    assert_eq!(defaults.max_records, 512);
    assert_eq!(defaults.max_query_calls, 640);
    for limits in [
        Limits {
            max_directories: 0,
            ..defaults
        },
        Limits {
            max_directories: 129,
            ..defaults
        },
        Limits {
            max_records: 0,
            ..defaults
        },
        Limits {
            max_records: 513,
            ..defaults
        },
        Limits {
            max_query_calls: 0,
            ..defaults
        },
        Limits {
            max_query_calls: 641,
            ..defaults
        },
    ] {
        assert_eq!(limits.validate(), Err(Error::Limit));
    }
    assert_eq!(
        Limits {
            max_directories: 1,
            max_records: 1,
            max_query_calls: 1
        }
        .validate(),
        Ok(Limits {
            max_directories: 1,
            max_records: 1,
            max_query_calls: 1
        })
    );
    for error in [
        Error::Limit,
        Error::Budget,
        Error::Changed,
        Error::Query,
        Error::Framing,
        Error::Name,
        Error::Reparse,
        Error::Extra,
        Error::Missing,
        Error::Duplicate,
        Error::Identity,
        Error::FileType,
    ] {
        assert!(error.code().starts_with("windows_membership_"));
        assert_eq!(error.to_string(), error.code());
        assert!(!error.code().contains(['/', '\\', ':']));
    }
}

#[test]
fn transient_full_directory_metadata_detects_changed_write_time() {
    let tmp = Temp::new();
    let parent = tmp.parent();
    let root = PrivateRoot::create(&parent, "private", output::Limits::default()).unwrap();
    let mut before = [None; MAX_OBJECTS];
    before[0] = Some(full_directory(&root.entries[0]).unwrap());
    let metadata_writer = OpenOptions::new()
        .access_mode(0x100)
        .share_mode(7)
        .custom_flags(0x0200_0000 | 0x0020_0000)
        .open(tmp.0.join("private"))
        .unwrap();
    metadata_writer
        .set_times(
            std::fs::FileTimes::new()
                .set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_secs(100_000)),
        )
        .unwrap();
    // V1 intentionally compares directory object identity/attributes only.
    // The new transient full-directory observation must see the explicit time.
    root.verify_unchanged().unwrap();
    assert_eq!(directories_unchanged(&root, &before), Err(Error::Changed));
}
