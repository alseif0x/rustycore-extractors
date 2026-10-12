// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original synthetic private repository adversarial tests.
use super::*;
use std::sync::atomic::{AtomicU64, Ordering};
static NEXT: AtomicU64 = AtomicU64::new(0);
pub(crate) struct Paths {
    pub(crate) base: PathBuf,
    pub(crate) source: PathBuf,
    pub(crate) parent: PathBuf,
    pub(crate) path: PathBuf,
}
impl Paths {
    pub(crate) fn new() -> Self {
        let base = std::env::temp_dir().join(format!(
            "rce-spool-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        DirBuilder::new().mode(0o700).create(&base).unwrap();
        let source = base.join("source");
        let parent = base.join("private");
        for p in [&source, &parent] {
            DirBuilder::new().mode(0o700).create(p).unwrap();
        }
        let path = parent.join("namespace");
        Self {
            base,
            source,
            parent,
            path,
        }
    }
    pub(crate) fn create(&self, limits: Limits) -> Spool {
        Spool::create(
            &self.path,
            ProtectedInputs::capture(&self.source, &[]).unwrap(),
            limits,
        )
        .unwrap()
    }
}
impl Drop for Paths {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.base).unwrap();
    }
}
fn write(spool: &mut Spool, bytes: &[u8]) -> Record {
    spool.write(bytes.len(), |out| out.bytes(bytes)).unwrap()
}
#[test]
fn roundtrip_ordinal_private_files_and_no_drop_cleanup() {
    let p = Paths::new();
    let mut s = p.create(Limits::default());
    let a = write(&mut s, b"abc");
    let b = write(&mut s, b"second");
    assert_eq!(s.with_record(a, 3, |b| Ok(b.to_vec())).unwrap(), b"abc");
    assert_eq!(s.with_record(b, 6, |b| Ok(b.to_vec())).unwrap(), b"second");
    assert_eq!(fs::metadata(&p.path).unwrap().mode() & 0o777, 0o700);
    assert_eq!(
        fs::metadata(p.path.join("00000000.payload"))
            .unwrap()
            .mode()
            & 0o777,
        0o400
    );
    assert_eq!(s.stats().unwrap().disk_bytes, 9);
    assert_eq!(s.stats().unwrap().reload_bytes, 9);
    drop(s);
    assert_eq!(fs::read(p.path.join("00000000.payload")).unwrap(), b"abc");
}
#[test]
fn disk_quota_is_terminal_preserves_successful_files() {
    let p = Paths::new();
    let mut s = p.create(Limits {
        max_disk_bytes: 3,
        ..Limits::default()
    });
    let a = write(&mut s, b"abc");
    assert_eq!(s.write(1, |out| out.bytes(b"x")).err(), Some(Error::Limit));
    assert_eq!(s.with_record(a, 3, |_| Ok(())).err(), Some(Error::Limit));
    assert_eq!(fs::read(p.path.join("00000000.payload")).unwrap(), b"abc");
}
#[test]
fn failed_write_keeps_created_partial_and_no_quota_refund() {
    let p = Paths::new();
    let mut s = p.create(Limits::default());
    assert_eq!(
        s.write(8, |out| {
            out.bytes(b"x")?;
            Err(Error::Callback)
        })
        .err(),
        Some(Error::Callback)
    );
    assert_eq!(fs::read(p.path.join("00000000.payload")).unwrap(), b"x");
    assert_eq!(s.stats().unwrap().disk_bytes, 8);
    assert_eq!(s.verify_unchanged(), Err(Error::Callback));
}
#[test]
fn short_writer_is_terminal_format_failure() {
    let p = Paths::new();
    let mut s = p.create(Limits::default());
    assert_eq!(s.write(2, |out| out.bytes(b"x")).err(), Some(Error::Format));
    assert_eq!(fs::read(p.path.join("00000000.payload")).unwrap(), b"x");
}
#[test]
fn too_long_writer_never_writes_beyond_declared_size() {
    let p = Paths::new();
    let mut s = p.create(Limits::default());
    assert_eq!(
        s.write(1, |out| out.bytes(b"xx")).err(),
        Some(Error::Format)
    );
    assert_eq!(
        fs::metadata(p.path.join("00000000.payload")).unwrap().len(),
        0
    );
}
#[test]
fn exact_read_budget_and_cumulative_reload_charge() {
    let p = Paths::new();
    let mut s = p.create(Limits {
        max_reload_bytes: 6,
        ..Limits::default()
    });
    let a = write(&mut s, b"abc");
    s.with_record(a, 3, |_| Ok(())).unwrap();
    s.with_record(a, 3, |_| Ok(())).unwrap();
    assert_eq!(s.with_record(a, 3, |_| Ok(())).err(), Some(Error::Limit));
    assert_eq!(s.stats().unwrap().reload_bytes, 6);
}
#[test]
fn reload_budget_failure_before_allocation_is_sticky() {
    let p = Paths::new();
    let mut s = p.create(Limits::default());
    let a = write(&mut s, b"abc");
    assert_eq!(s.with_record(a, 2, |_| Ok(())).err(), Some(Error::Limit));
    assert_eq!(s.stats().unwrap().reloads, 0);
    assert_eq!(s.with_record(a, 3, |_| Ok(())).err(), Some(Error::Limit));
}
#[test]
fn parser_failure_still_checks_held_named_identity() {
    let p = Paths::new();
    let mut s = p.create(Limits::default());
    let a = write(&mut s, b"abc");
    let f = p.path.join("00000000.payload");
    assert_eq!(
        s.with_record(a, 3, |_| {
            fs::rename(&f, f.with_extension("old")).unwrap();
            fs::write(&f, b"abc").unwrap();
            Err::<(), _>(Error::Callback)
        })
        .err(),
        Some(Error::Changed)
    );
    assert_eq!(s.verify_unchanged(), Err(Error::Changed));
}
#[test]
fn metadata_and_hash_mutation_are_rejected() {
    let p = Paths::new();
    let mut s = p.create(Limits::default());
    let a = write(&mut s, b"abc");
    let f = p.path.join("00000000.payload");
    fs::set_permissions(&f, Permissions::from_mode(0o600)).unwrap();
    fs::write(&f, b"abd").unwrap();
    assert_eq!(s.with_record(a, 3, |_| Ok(())).err(), Some(Error::Changed));
}
#[test]
fn digest_check_is_independent_of_metadata_check() {
    let p = Paths::new();
    let mut s = p.create(Limits::default());
    let a = write(&mut s, b"abc");
    s.entries[a.0].digest = [0; 32];
    assert_eq!(s.with_record(a, 3, |_| Ok(())).err(), Some(Error::Changed));
}
#[test]
fn hardlink_alias_of_record_rejected() {
    let p = Paths::new();
    let mut s = p.create(Limits::default());
    let a = write(&mut s, b"abc");
    fs::hard_link(p.path.join("00000000.payload"), p.parent.join("alias")).unwrap();
    assert_eq!(s.with_record(a, 3, |_| Ok(())).err(), Some(Error::Changed));
}
#[test]
fn symlink_record_replacement_rejected() {
    let p = Paths::new();
    let mut s = p.create(Limits::default());
    let a = write(&mut s, b"abc");
    let f = p.path.join("00000000.payload");
    fs::rename(&f, p.parent.join("held")).unwrap();
    std::os::unix::fs::symlink(p.parent.join("held"), &f).unwrap();
    assert_eq!(s.with_record(a, 3, |_| Ok(())).err(), Some(Error::Changed));
}
#[test]
fn namespace_replacement_and_ancestor_rename_rejected() {
    let p = Paths::new();
    let mut s = p.create(Limits::default());
    write(&mut s, b"abc");
    fs::rename(&p.path, p.parent.join("old")).unwrap();
    DirBuilder::new().mode(0o700).create(&p.path).unwrap();
    assert_eq!(s.verify_unchanged(), Err(Error::Changed));
}
#[test]
fn namespace_mode_change_stays_terminal_after_restore() {
    let p = Paths::new();
    let mut s = p.create(Limits::default());
    fs::set_permissions(&p.path, Permissions::from_mode(0o755)).unwrap();
    assert_eq!(s.verify_unchanged(), Err(Error::Changed));
    fs::set_permissions(&p.path, Permissions::from_mode(0o700)).unwrap();
    assert_eq!(s.verify_unchanged(), Err(Error::Changed));
}
#[test]
fn protected_key_mutation_checked_after_callback_error() {
    let p = Paths::new();
    let key = p.source.join("provided.keys");
    fs::write(&key, b"synthetic-not-a-key").unwrap();
    let mut s = Spool::create(
        &p.path,
        ProtectedInputs::capture(&p.source, &[&key]).unwrap(),
        Limits::default(),
    )
    .unwrap();
    let a = write(&mut s, b"abc");
    assert_eq!(
        s.with_record(a, 3, |_| {
            fs::write(&key, b"changed").unwrap();
            Err::<(), _>(Error::Callback)
        })
        .err(),
        Some(Error::Changed)
    );
}
#[test]
fn protected_source_containment_and_existing_output_rejected() {
    let p = Paths::new();
    let inside = p.source.join("new");
    assert_eq!(
        Spool::create(
            &inside,
            ProtectedInputs::capture(&p.source, &[]).unwrap(),
            Limits::default()
        )
        .err(),
        Some(Error::Alias)
    );
    let s = p.create(Limits::default());
    assert_eq!(
        Spool::create(
            &p.path,
            ProtectedInputs::capture(&p.source, &[]).unwrap(),
            Limits::default()
        )
        .err(),
        Some(Error::Exists)
    );
    drop(s);
}
#[test]
fn symlink_ancestor_and_nonprivate_parent_rejected() {
    let p = Paths::new();
    let alias = p.base.join("alias");
    std::os::unix::fs::symlink(&p.parent, &alias).unwrap();
    assert_eq!(
        Spool::create(
            &alias.join("new"),
            ProtectedInputs::capture(&p.source, &[]).unwrap(),
            Limits::default()
        )
        .err(),
        Some(Error::Path)
    );
    fs::set_permissions(&p.parent, Permissions::from_mode(0o755)).unwrap();
    assert_eq!(
        Spool::create(
            &p.path,
            ProtectedInputs::capture(&p.source, &[]).unwrap(),
            Limits::default()
        )
        .err(),
        Some(Error::Privacy)
    );
}
#[test]
fn limits_are_downward_and_descriptors_charged() {
    let p = Paths::new();
    assert_eq!(
        Spool::create(
            &p.path,
            ProtectedInputs::capture(&p.source, &[]).unwrap(),
            Limits {
                max_records: 4097,
                ..Limits::default()
            }
        )
        .err(),
        Some(Error::InvalidLimits)
    );
    assert_eq!(
        Spool::create(
            &p.path,
            ProtectedInputs::capture(&p.source, &[]).unwrap(),
            Limits {
                max_descriptor_bytes: 0,
                ..Limits::default()
            }
        )
        .err(),
        Some(Error::Limit)
    );
    assert!(!p.path.exists());
}

#[test]
fn lexical_dot_and_parent_paths_reject_before_creation() {
    let p = Paths::new();
    for path in [
        p.parent.join(".").join("bad"),
        p.parent.join("..").join("bad"),
    ] {
        assert_eq!(
            Spool::create(
                &path,
                ProtectedInputs::capture(&p.source, &[]).unwrap(),
                Limits::default()
            )
            .err(),
            Some(Error::Path)
        );
    }
    assert_eq!(fs::read_dir(&p.parent).unwrap().count(), 0);
}
#[test]
fn preexisting_ordinal_key_alias_is_never_overwritten() {
    let p = Paths::new();
    let key = p.parent.join("key");
    fs::write(&key, b"synthetic-not-key-data").unwrap();
    let mut s = Spool::create(
        &p.path,
        ProtectedInputs::capture(&p.source, &[&key]).unwrap(),
        Limits::default(),
    )
    .unwrap();
    // A new namespace beside a key is safe; it cannot overwrite the key or use
    // an existing key alias as its new ordinal record.
    fs::hard_link(&key, p.path.join("00000000.payload")).unwrap();
    assert_eq!(s.write(3, |w| w.bytes(b"abc")).err(), Some(Error::Changed));
    assert_eq!(fs::read(&key).unwrap(), b"synthetic-not-key-data");
}

#[test]
fn entry_reallocation_requires_old_plus_full_new_capacity() {
    let p = Paths::new();
    let mut s = p.create(Limits::default());
    write(&mut s, b"a");
    let old = s.entries.capacity();
    assert_eq!(old, 1);
    // Net growth would fit one Entry, but full new allocation needs two.
    assert_eq!(s.prepare_records(2, size_of::<Entry>()), Err(Error::Limit));
    assert_eq!(s.entries.capacity(), old);
    s.prepare_records(2, 2 * size_of::<Entry>()).unwrap();
    assert_eq!(s.entries.capacity(), 2);
}
#[test]
fn allocation_failure_after_open_rechecks_named_and_held_file() {
    let p = Paths::new();
    let mut s = p.create(Limits::default());
    let a = write(&mut s, b"abc");
    let path = p.path.join("00000000.payload");
    assert_eq!(
        s.with_record_alloc(
            a,
            3,
            |_| {
                fs::rename(&path, p.parent.join("old")).unwrap();
                fs::write(&path, b"abc").unwrap();
                Err(Error::Allocation)
            },
            |_| Ok(())
        )
        .err(),
        Some(Error::Changed)
    );
    assert_eq!(s.stats().unwrap().reload_bytes, 3);
    assert_eq!(s.verify_unchanged(), Err(Error::Changed));
}
#[test]
fn actual_capacity_refusal_after_open_still_rechecks_file() {
    let p = Paths::new();
    let mut s = p.create(Limits::default());
    let a = write(&mut s, b"abc");
    let path = p.path.join("00000000.payload");
    assert_eq!(
        s.with_record_alloc(
            a,
            3,
            |_| {
                fs::rename(&path, p.parent.join("old")).unwrap();
                fs::write(&path, b"abc").unwrap();
                Ok(Vec::with_capacity(4))
            },
            |_| Ok(())
        )
        .err(),
        Some(Error::Changed)
    );
    assert_eq!(s.stats().unwrap().reload_bytes, 3);
}
#[test]
fn ordinary_allocation_failure_is_sticky_without_reading_payload() {
    let p = Paths::new();
    let mut s = p.create(Limits::default());
    let a = write(&mut s, b"abc");
    assert_eq!(
        s.with_record_alloc(a, 3, |_| Err(Error::Allocation), |_| Ok(()))
            .err(),
        Some(Error::Allocation)
    );
    assert_eq!(s.verify_unchanged(), Err(Error::Allocation));
    assert_eq!(s.stats().unwrap().peak_reload_bytes, 0);
}
#[test]
fn failed_writer_checks_created_object_not_nonexistent_entry() {
    let p = Paths::new();
    let mut s = p.create(Limits::default());
    let path = p.path.join("00000000.payload");
    assert_eq!(
        s.write(3, |out| {
            out.bytes(b"x")?;
            fs::rename(&path, p.parent.join("old")).unwrap();
            fs::write(&path, b"abc").unwrap();
            Err(Error::Callback)
        })
        .err(),
        Some(Error::Changed)
    );
    assert_eq!(fs::read(p.parent.join("old")).unwrap(), b"x");
    assert_eq!(fs::read(&path).unwrap(), b"abc");
    assert_eq!(s.entries.len(), 0);
}
#[test]
fn completed_writer_checks_actual_disk_sha_and_eof_before_descriptor_commit() {
    let p = Paths::new();
    let mut s = p.create(Limits::default());
    let path = p.path.join("00000000.payload");
    assert_eq!(
        s.write(3, |out| {
            out.bytes(b"abc")?;
            fs::write(&path, b"abd").unwrap();
            Ok(())
        })
        .err(),
        Some(Error::Changed)
    );
    assert_eq!(s.entries.len(), 0);
    assert_eq!(fs::read(&path).unwrap(), b"abd");
}
#[test]
fn sticky_allocation_failure_does_not_skip_later_namespace_guards() {
    let p = Paths::new();
    let mut s = p.create(Limits::default());
    let a = write(&mut s, b"abc");
    s.with_record_alloc(a, 3, |_| Err(Error::Allocation), |_| Ok(()))
        .unwrap_err();
    fs::set_permissions(&p.path, Permissions::from_mode(0o755)).unwrap();
    assert_eq!(s.verify_unchanged(), Err(Error::Changed));
    fs::set_permissions(&p.path, Permissions::from_mode(0o700)).unwrap();
    assert_eq!(s.verify_unchanged(), Err(Error::Changed));
}

#[test]
fn mount_parser_accepts_only_complete_bounded_decimal_record() {
    assert_eq!(
        parse_mount(b"pos:\t0\nflags:\t0100000\nmnt_id:\t1234\nino:\t42\n"),
        Ok(1234)
    );
    assert_eq!(parse_mount(b"mnt_id: 18446744073709551615\n"), Ok(u64::MAX));
    for data in [
        &b""[..],
        &b"pos: 0\n"[..],
        &b"mnt_id: 1"[..],
        &b"mnt_id: 1\nmnt_id: 1\n"[..],
        &b"mnt_id: \n"[..],
        &b"mnt_id: 0\n"[..],
        &b"mnt_id: -1\n"[..],
        &b"mnt_id: 1x\n"[..],
        &b"mnt_id: 1 \n"[..],
        &b"mnt_id: 18446744073709551616\n"[..],
        &b"mnt_id: 1\nbroken\n"[..],
        &b"mnt_id: 1\n\xff: 2\n"[..],
    ] {
        assert_eq!(parse_mount(data), Err(Error::Mount));
    }
    let mut long = Vec::from(&b"mnt_id: 1\nother:"[..]);
    long.resize(MAX_FDINFO, b'0');
    long.push(b'\n');
    assert_eq!(parse_mount(&long), Err(Error::Mount));
}
#[test]
fn mount_mismatch_rejects_before_namespace_creation() {
    let p = Paths::new();
    let mut protected = ProtectedInputs::capture(&p.source, &[]).unwrap();
    let node = protected.chains[0].nodes.last_mut().unwrap();
    node.mount = node.mount.checked_add(1).unwrap();
    assert_eq!(
        Spool::create(&p.path, protected, Limits::default()).err(),
        Some(Error::Changed)
    );
    assert!(!p.path.exists());
    assert_eq!(same_mount(10, 11), Err(Error::Mount));
    assert_eq!(same_mount(10, 10), Ok(()));
}
#[test]
fn retained_mount_mismatch_rejects_before_first_record_write() {
    let p = Paths::new();
    let mut s = p.create(Limits::default());
    let node = s.root.nodes.last_mut().unwrap();
    node.mount = node.mount.checked_add(1).unwrap();
    let mut invoked = false;
    assert_eq!(
        s.write(3, |out| {
            invoked = true;
            out.bytes(b"abc")
        })
        .err(),
        Some(Error::Changed)
    );
    assert!(!invoked);
    assert_eq!(fs::read_dir(&p.path).unwrap().count(), 0);
    assert_eq!(s.stats().unwrap().disk_bytes, 0);
}
#[test]
fn record_mount_mismatch_rejects_before_reload_callback() {
    let p = Paths::new();
    let mut s = p.create(Limits::default());
    let a = write(&mut s, b"abc");
    s.entries[a.0].mount = s.entries[a.0].mount.checked_add(1).unwrap();
    let mut invoked = false;
    assert_eq!(
        s.with_record(a, 3, |_| {
            invoked = true;
            Ok(())
        })
        .err(),
        Some(Error::Changed)
    );
    assert!(!invoked);
    assert_eq!(s.stats().unwrap().reload_bytes, 0);
}
