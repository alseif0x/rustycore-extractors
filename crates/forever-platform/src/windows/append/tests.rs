// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-13: synthetic native exclusive-append filesystem cases.
use super::*;
use std::{
    fs::{self, OpenOptions},
    os::windows::fs::OpenOptionsExt,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "forever-append-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        fs::create_dir(path.join("source")).unwrap();
        Self(path)
    }
    fn inputs(&self) -> ProtectedInputs {
        ProtectedInputs::capture(&self.0.join("source"), &[]).unwrap()
    }
    fn archive(&self) -> OwnedArchive {
        OwnedArchive::create(&self.0.join("owner"), self.inputs(), Limits::default()).unwrap()
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
#[test]
fn native_architecture_gate_rejects_failure_and_emulation() {
    assert_eq!(PROFILE, "windows-exclusive-append-v1");
    assert!(supported());
    assert_eq!(architecture(true, 0, 0x8664), Ok(()));
    for (success, process, native) in [
        (false, 0, 0x8664),
        (true, 0x14c, 0x8664),
        (true, 0x8664, 0xaa64),
        (true, 0, 0xaa64),
        (true, 0, 0),
    ] {
        assert_eq!(
            architecture(success, process, native),
            Err(Error::Unsupported)
        );
    }
    admit().unwrap();
}
#[test]
fn limits_are_downward_and_reject_before_owner_creation() {
    for limits in [
        Limits {
            max_bytes: MAX_BYTES + 1,
            ..Limits::default()
        },
        Limits {
            max_io_bytes: 0,
            ..Limits::default()
        },
        Limits {
            max_handles: MAX_HANDLES + 1,
            ..Limits::default()
        },
        Limits {
            max_work: MAX_WORK + 1,
            ..Limits::default()
        },
        Limits {
            max_metadata_bytes: MAX_METADATA + 1,
            ..Limits::default()
        },
    ] {
        assert_eq!(limits.validate().unwrap_err(), Error::Limit);
    }
    let t = Temp::new();
    for limits in [
        Limits {
            max_handles: 1,
            ..Limits::default()
        },
        Limits {
            max_metadata_bytes: 1,
            ..Limits::default()
        },
        Limits {
            max_work: 1,
            ..Limits::default()
        },
    ] {
        assert!(matches!(
            OwnedArchive::create(&t.0.join("owner"), t.inputs(), limits),
            Err(Error::Limit)
        ));
        assert!(!t.0.join("owner").exists());
    }
}
#[test]
fn exact_extent_patch_flush_same_handle_read_and_committed_prefix() {
    let t = Temp::new();
    let mut owner = t.archive();
    let initial = owner.resident_bytes().unwrap();
    let handles = owner.retained_handles();
    for value in 0..40_u8 {
        let mut txn = owner.begin_append(8).unwrap();
        assert_eq!(txn.start(), u64::from(value) * 8);
        txn.write_all(&[0; 4]).unwrap();
        txn.write_all(&[value; 4]).unwrap();
        txn.patch_at(0, &[value + 1; 4]).unwrap();
        txn.flush().unwrap();
        let mut b = [0; 8];
        txn.read_exact_at(0, &mut b).unwrap();
        assert_eq!(
            b,
            [
                value + 1,
                value + 1,
                value + 1,
                value + 1,
                value,
                value,
                value,
                value
            ]
        );
        txn.commit().unwrap();
        let mut prefix = [0; 8];
        owner.read_exact_at(0, &mut prefix).unwrap();
        assert_eq!(prefix, [1, 1, 1, 1, 0, 0, 0, 0]);
    }
    assert_eq!(owner.len(), 320);
    assert_eq!(owner.resident_bytes().unwrap(), initial);
    assert_eq!(owner.retained_handles(), handles);
    owner.guard().unwrap();
}
#[test]
fn uncommitted_drop_and_panic_stick_without_truncation() {
    for panic in [false, true] {
        let t = Temp::new();
        let mut owner = t.archive();
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut txn = owner.begin_append(8).unwrap();
            txn.write_all(b"part").unwrap();
            if panic {
                panic!("synthetic callback");
            }
        }));
        assert_eq!(outcome.is_err(), panic);
        assert_eq!(owner.len(), 0);
        assert_eq!(owner.physical.get(), 4);
        assert_eq!(owner.guard(), Err(Error::Poisoned));
        assert!(matches!(owner.begin_append(1), Err(Error::Poisoned)));
        assert_eq!(owner.file.metadata().unwrap().len(), 4);
    }
}
#[test]
fn pending_bounds_cannot_patch_committed_prefix_or_read_unwritten_bytes() {
    let t = Temp::new();
    let mut owner = t.archive();
    {
        let mut txn = owner.begin_append(4).unwrap();
        txn.write_all(b"old!").unwrap();
        txn.commit().unwrap();
    }
    {
        let mut txn = owner.begin_append(4).unwrap();
        txn.write_all(b"n").unwrap();
        assert_eq!(txn.patch_at(u64::MAX, b"x"), Err(Error::Limit));
    }
    assert_eq!(owner.failure(), Some(Error::Limit));
    assert_eq!(owner.len(), 4);
    let mut original = [0; 4];
    owner.file.seek_read(&mut original, 0).unwrap();
    assert_eq!(&original, b"old!");
    let t = Temp::new();
    let mut owner = t.archive();
    let txn = owner.begin_append(4).unwrap();
    assert_eq!(txn.read_exact_at(0, &mut [0; 1]), Err(Error::Limit));
}
#[test]
fn io_chunk_total_and_incomplete_commit_limits_are_terminal() {
    let t = Temp::new();
    let mut owner = OwnedArchive::create(
        &t.0.join("owner"),
        t.inputs(),
        Limits {
            max_bytes: 4,
            max_io_bytes: 2,
            ..Limits::default()
        },
    )
    .unwrap();
    assert!(matches!(owner.begin_append(5), Err(Error::Limit)));
    assert_eq!(owner.failure(), None);
    {
        let mut txn = owner.begin_append(4).unwrap();
        assert_eq!(txn.write_all(b"123"), Err(Error::Limit));
    }
    assert_eq!(owner.failure(), Some(Error::Limit));
    let t = Temp::new();
    let mut owner = t.archive();
    assert_eq!(owner.begin_append(4).unwrap().commit(), Err(Error::Limit));
    assert_eq!(owner.len(), 0);
}
#[test]
fn share_zero_blocks_foreign_reads_writes_delete_and_reopen() {
    let t = Temp::new();
    let owner = t.archive();
    let path = t.0.join("owner/records.archive");
    for (read, write, access) in [(true, false, 0), (false, true, 0), (false, false, 0x10000)] {
        assert!(
            OpenOptions::new()
                .read(read)
                .write(write)
                .access_mode(
                    access | if read { 0x80000000 } else { 0 } | if write { 0x40000000 } else { 0 }
                )
                .share_mode(7)
                .open(&path)
                .is_err()
        );
    }
    assert!(fs::remove_file(&path).is_err());
    assert!(matches!(
        OwnedArchive::create(&t.0.join("owner"), t.inputs(), Limits::default()),
        Err(Error::Exists)
    ));
    let mut second = package::Root::open(&t.0.join("owner"), package::Limits::default()).unwrap();
    assert!(matches!(second.lock(true), Err(package::Error::Busy)));
    assert!(matches!(second.lock(false), Err(package::Error::Busy)));
    owner.guard().unwrap();
}
#[test]
fn unexpected_membership_and_eof_changes_poison_permanently() {
    let t = Temp::new();
    let owner = t.archive();
    fs::write(t.0.join("owner/extra"), b"x").unwrap();
    assert_eq!(owner.guard(), Err(Error::Changed));
    fs::remove_file(t.0.join("owner/extra")).unwrap();
    assert_eq!(owner.guard(), Err(Error::Changed));
    let t = Temp::new();
    let owner = t.archive();
    owner.file.set_len(1).unwrap();
    assert_eq!(owner.guard(), Err(Error::Changed));
    owner.file.set_len(0).unwrap();
    assert_eq!(owner.guard(), Err(Error::Changed));
}
#[test]
fn protected_inputs_exclude_containment_and_observed_changes_win_poison() {
    let t = Temp::new();
    fs::write(t.0.join("key"), b"key").unwrap();
    let inputs = ProtectedInputs::capture(&t.0.join("source"), &[&t.0.join("key")]).unwrap();
    assert!(matches!(
        OwnedArchive::create(&t.0.join("source/owner"), inputs.clone(), Limits::default()),
        Err(Error::Path)
    ));
    assert!(matches!(
        OwnedArchive::create(&t.0, inputs.clone(), Limits::default()),
        Err(Error::Path)
    ));
    let owner = OwnedArchive::create(&t.0.join("owner"), inputs, Limits::default()).unwrap();
    owner.poison();
    // Source directory metadata can change without changing its identity.
    fs::write(t.0.join("source/sibling"), b"okay").unwrap();
    assert_eq!(owner.guard(), Err(Error::Poisoned));
    // Controlled mutation through the same retained key handle is not permitted;
    // external writes are excluded by the existing readonly source lease.
    assert!(fs::write(t.0.join("key"), b"changed").is_err());
    owner.file.set_len(1).unwrap();
    assert_eq!(owner.guard(), Err(Error::Changed));
    assert_eq!(owner.failure(), Some(Error::Changed));
}
#[test]
fn private_namespace_is_fresh_and_accepts_independent_six_owners() {
    let t = Temp::new();
    let mut root = package::Root::create(&t.0.join("scratch"), package::Limits::default()).unwrap();
    let _lock = root.lock(true).unwrap();
    let directory = ReadDirectory::open(&t.0.join("scratch"), PathLimits::default()).unwrap();
    private_directory(&directory).unwrap();
    let namespace = create_namespace(&directory, "six-owners").unwrap();
    let inputs = t.inputs();
    let mut owners = Vec::new();
    for index in 0..6 {
        owners.push(
            OwnedArchive::create(
                &namespace.path().join(format!("owner{index}")),
                inputs.clone(),
                Limits::default(),
            )
            .unwrap(),
        );
    }
    for owner in &mut owners {
        let mut txn = owner.begin_append(1).unwrap();
        txn.write_all(b"x").unwrap();
        txn.commit().unwrap();
    }
    namespace.guard().unwrap();
    directory.verify_same_object().unwrap();
    assert!(matches!(
        create_namespace(&directory, "six-owners"),
        Err(Error::Exists)
    ));
    assert!(matches!(
        create_namespace(&directory, "SIX-OWNERS"),
        Err(Error::Alias)
    ));
    assert!(matches!(
        create_namespace(&directory, ".git"),
        Err(Error::Path)
    ));
    assert_eq!(owners.len(), 6);
}
#[test]
fn streaming_more_than_256_mib_retains_constant_owner_metadata_and_handles() {
    let t = Temp::new();
    let mut owner = t.archive();
    let resident = owner.resident_bytes().unwrap();
    let handles = owner.retained_handles();
    let mut chunk = vec![0_u8; MAX_IO];
    let mut reread = vec![0_u8; 8192];
    for index in 0..257_u64 {
        chunk.fill((index % 251) as u8);
        let mut txn = owner.begin_append(MAX_IO as u64).unwrap();
        txn.write_all(&chunk).unwrap();
        txn.flush().unwrap();
        txn.read_exact_at(MAX_IO as u64 - reread.len() as u64, &mut reread)
            .unwrap();
        assert!(reread.iter().all(|b| *b == (index % 251) as u8));
        txn.commit().unwrap();
        assert_eq!(owner.resident_bytes().unwrap(), resident);
        assert_eq!(owner.retained_handles(), handles);
    }
    assert_eq!(owner.len(), 257 * 1024 * 1024);
    assert!(owner.len() > 256 * 1024 * 1024);
    owner.guard().unwrap();
}
#[test]
fn ordinary_scratch_directory_requires_exact_private_owner_dacl() {
    let t = Temp::new();
    let ordinary = ReadDirectory::open(&t.0.join("source"), PathLimits::default()).unwrap();
    // The authored ordinary directory inherits permissions; it is not the
    // profile's explicit protected owner-only descriptor.
    assert_eq!(private_directory(&ordinary), Err(Error::Changed));
    let owner = t.archive();
    let security = OpenOptions::new()
        .access_mode(0x0006_0000)
        .share_mode(7)
        .open(t.0.join("owner/records.archive"))
        .unwrap();
    package::append_support::test_dacl(&security, 0x0012_0081).unwrap();
    assert_eq!(owner.guard(), Err(Error::Changed));
    package::append_support::test_dacl(&security, 0x001f_01ff).unwrap();
    assert_eq!(owner.guard(), Err(Error::Changed));
}
#[test]
fn hardlink_creation_is_excluded_or_observed_as_integrity_failure() {
    let t = Temp::new();
    let owner = t.archive();
    match fs::hard_link(t.0.join("owner/records.archive"), t.0.join("alias")) {
        Ok(()) => assert_eq!(owner.guard(), Err(Error::Changed)),
        Err(_) => owner.guard().unwrap(),
    }
}
#[test]
fn ordinary_junction_owner_and_parent_are_rejected_without_symlink_privilege() {
    let t = Temp::new();
    fs::create_dir(t.0.join("target")).unwrap();
    let junction = t.0.join("junction");
    let status = std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(&junction)
        .arg(t.0.join("target"))
        .output()
        .unwrap();
    assert!(status.status.success());
    assert!(OwnedArchive::create(&junction, t.inputs(), Limits::default()).is_err());
    assert!(OwnedArchive::create(&junction.join("owner"), t.inputs(), Limits::default()).is_err());
    assert!(!t.0.join("target/owner").exists());
    // Delete the junction itself before Temp's recursive cleanup touches target.
    fs::remove_dir(&junction).unwrap();
}
#[test]
fn git_ancestor_is_rejected_before_fresh_owner_mutation() {
    let t = Temp::new();
    fs::write(t.0.join(".git"), b"authored marker").unwrap();
    assert!(matches!(
        OwnedArchive::create(&t.0.join("owner"), t.inputs(), Limits::default()),
        Err(Error::Path)
    ));
    assert!(!t.0.join("owner").exists());
}
#[test]
fn forgotten_transaction_cannot_reuse_or_overwrite_pending_suffix() {
    let t = Temp::new();
    let mut owner = t.archive();
    let mut txn = owner.begin_append(4).unwrap();
    txn.write_all(b"part").unwrap();
    std::mem::forget(txn);
    assert_eq!(owner.guard(), Err(Error::Poisoned));
    assert!(matches!(owner.begin_append(4), Err(Error::Poisoned)));
    assert_eq!(owner.len(), 0);
    assert_eq!(owner.file.metadata().unwrap().len(), 4);
    let mut bytes = [0; 4];
    owner.file.seek_read(&mut bytes, 0).unwrap();
    assert_eq!(&bytes, b"part");
}
#[test]
fn subdirectory_drive_alias_cannot_hide_protected_ancestors() {
    struct Alias(String);
    impl Drop for Alias {
        fn drop(&mut self) {
            let _ = std::process::Command::new("subst")
                .args([&self.0, "/D"])
                .status();
        }
    }
    let t = Temp::new();
    fs::create_dir(t.0.join("source/deep")).unwrap();
    fs::create_dir(t.0.join("source/deep/another")).unwrap();
    let drive = (b'L'..=b'Q')
        .map(|b| format!("{}:", char::from(b)))
        .find(|name| !Path::new(&format!("{name}\\")).exists())
        .expect("free synthetic alias drive");
    assert!(
        std::process::Command::new("subst")
            .arg(&drive)
            .arg(t.0.join("source/deep"))
            .status()
            .unwrap()
            .success()
    );
    let alias = Alias(drive);
    let candidate = PathBuf::from(format!("{}\\owner", alias.0));
    assert!(matches!(
        OwnedArchive::create(&candidate, t.inputs(), Limits::default()),
        Err(Error::Unsupported)
    ));
    assert!(!t.0.join("source/deep/owner").exists());
    assert!(matches!(
        ProtectedInputs::capture(&PathBuf::from(format!("{}\\another", alias.0)), &[]),
        Err(Error::Unsupported)
    ));
    let drive = alias.0.clone();
    drop(alias);
    let physical = t.0.to_str().unwrap();
    assert!(
        std::process::Command::new("subst")
            .arg(&drive)
            .arg(&physical[..3])
            .status()
            .unwrap()
            .success()
    );
    let alias = Alias(drive);
    let aliased_source = PathBuf::from(format!("{}\\{}\\source", alias.0, &physical[3..]));
    let inputs = ProtectedInputs::capture(&aliased_source, &[]).unwrap();
    assert!(
        inputs
            .inner
            .source
            .identity()
            .same_object(t.inputs().inner.source.identity())
    );
    assert!(matches!(
        OwnedArchive::create(&aliased_source.join("owner"), t.inputs(), Limits::default()),
        Err(Error::Path)
    ));
    assert!(!t.0.join("source/owner").exists());
}
#[test]
fn protected_hardlinked_key_rejects_before_owner_creation() {
    let t = Temp::new();
    fs::write(t.0.join("key"), b"authored").unwrap();
    fs::hard_link(t.0.join("key"), t.0.join("key-alias")).unwrap();
    assert!(matches!(
        ProtectedInputs::capture(&t.0.join("source"), &[&t.0.join("key")]),
        Err(Error::Changed)
    ));
    assert!(!t.0.join("owner").exists());
}
#[test]
fn mutable_snapshot_rejects_observed_link_eof_and_identity_churn() {
    let base = Identity {
        volume: 7,
        file_index: 11,
        links: 1,
        attributes: 0x80,
        size: 19,
        creation_time: 23,
        write_time: 29,
        change_time: 31,
    };
    let mut changing = base;
    changing.write_time = 37;
    changing.change_time = 41;
    changing.attributes = 0x20;
    assert_eq!(mutable_snapshot(base, changing), Ok(base));
    for field in 0..7 {
        let mut changed = base;
        match field {
            0 => changed.links = 2,
            1 => changed.size += 1,
            2 => changed.file_index += 1,
            3 => changed.volume += 1,
            4 => changed.creation_time += 1,
            5 => changed.attributes |= 0x400,
            _ => changed.links = 0,
        }
        assert_eq!(mutable_snapshot(base, changed), Err(Error::Changed));
        assert_eq!(mutable_snapshot(changed, base), Err(Error::Changed));
    }
}
#[test]
fn observed_protected_key_change_overrides_logical_poison_and_stays_sticky() {
    let t = Temp::new();
    let key = t.0.join("key");
    fs::write(&key, b"synthetic unchanged bytes").unwrap();
    let inputs = ProtectedInputs::capture(&t.0.join("source"), &[&key]).unwrap();
    let creation = inputs.inner.keys[0].identity().creation_time;
    let owner = OwnedArchive::create(&t.0.join("owner"), inputs, Limits::default()).unwrap();
    owner.poison();
    // SHARE_READ excludes competing data writes, but an attributes-only open
    // can change timestamps. This exercises the retained source guard itself.
    let metadata = OpenOptions::new()
        .access_mode(0x100)
        .share_mode(7)
        .open(&key)
        .unwrap();
    ffi::test_creation(&metadata, creation + 10_000_000).unwrap();
    assert_eq!(owner.guard(), Err(Error::Changed));
    ffi::test_creation(&metadata, creation).unwrap();
    drop(metadata);
    assert_eq!(owner.guard(), Err(Error::Changed));
    assert_eq!(owner.failure(), Some(Error::Changed));
}
