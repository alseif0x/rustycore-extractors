// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-12: original synthetic append-archive integrity tests.
use super::*;
use crate::collision::{
    resources,
    spool::{self, BackendProfile, Limits, ProtectedInputs, Record, Spool, tests::Paths},
};
use std::{
    os::unix::fs::PermissionsExt,
    sync::atomic::{AtomicUsize, Ordering},
};
fn create(p: &Paths, limits: Limits) -> Spool {
    Spool::create_with_backend(
        &p.path,
        ProtectedInputs::capture(&p.source, &[]).unwrap(),
        limits,
        spool::Profile::SameMountV1,
        resources::Profile::DiskWorldV1,
        BackendProfile::LeasedAppendV1,
    )
    .unwrap()
}
fn store(s: &mut Spool, bytes: &[u8]) -> Record {
    s.write(bytes.len(), |w| w.bytes(bytes)).unwrap()
}
#[test]
fn append_archive_roundtrip_one_file_exact_private_framing() {
    let p = Paths::new();
    let mut s = create(&p, Limits::default());
    assert_eq!(s.backend(), BackendProfile::LeasedAppendV1);
    assert_eq!(s.stats().unwrap().disk_bytes, CONTAINER_BYTES);
    let a = store(&mut s, b"abc");
    let b = store(&mut s, b"");
    assert_eq!(s.with_record(a, 3, |b| Ok(b.to_vec())).unwrap(), b"abc");
    assert_eq!(s.with_record(b, 0, |b| Ok(b.len())).unwrap(), 0);
    assert_eq!(s.stats().unwrap().records, 2);
    assert_eq!(
        s.stats().unwrap().disk_bytes,
        CONTAINER_BYTES + 2 * RECORD_BYTES + 3
    );
    assert_eq!(s.stats().unwrap().reload_bytes, 2 * RECORD_BYTES + 3);
    assert_eq!(fs::read_dir(&p.path).unwrap().count(), 1);
    assert_eq!(
        fs::metadata(p.path.join(NAME))
            .unwrap()
            .permissions()
            .mode()
            & 0o7777,
        0o600
    );
    drop(s);
    let bytes = fs::read(p.path.join(NAME)).unwrap();
    assert_eq!(&bytes[..CONTAINER_BYTES], &container());
    let mut hash = Sha256::new();
    hash.update(b"abc");
    assert_eq!(
        &bytes[CONTAINER_BYTES..CONTAINER_BYTES + RECORD_BYTES],
        &header(0, 3, hash.finish())
    );
    assert_eq!(
        &bytes[CONTAINER_BYTES + RECORD_BYTES..CONTAINER_BYTES + RECORD_BYTES + 3],
        b"abc"
    );
}
#[test]
fn append_archive_physical_disk_boundary_includes_all_framing() {
    // Initial parent admission fits, but the new captured leaf must also fit
    // beside the helper reservation. Reject before creating records.archive.
    let bounded = Paths::new();
    let protected = ProtectedInputs::capture(&bounded.source, &[]).unwrap();
    let parent = spool::Chain::open(bounded.path.parent().unwrap(), true).unwrap();
    let descriptor_limit =
        protected.bytes().unwrap() + parent.bytes().unwrap() + lease::HELPER_RESERVATION_BYTES;
    assert_eq!(
        Spool::create_with_backend(
            &bounded.path,
            protected,
            Limits {
                max_descriptor_bytes: descriptor_limit,
                ..Limits::default()
            },
            spool::Profile::SameMountV1,
            resources::Profile::DiskWorldV1,
            BackendProfile::LeasedAppendV1,
        )
        .err(),
        Some(Error::Limit)
    );
    assert!(bounded.path.is_dir());
    assert!(!bounded.path.join(NAME).exists());
    let p = Paths::new();
    let mut s = create(
        &p,
        Limits {
            max_disk_bytes: 100,
            ..Limits::default()
        },
    );
    store(&mut s, b"1234");
    assert_eq!(s.stats().unwrap().disk_bytes, 100);
    assert_eq!(s.write(0, |_| Ok(())).err(), Some(Error::Limit));
    assert_eq!(s.verify_unchanged(), Err(Error::Limit));
}
#[test]
fn append_archive_partial_writer_preserves_cause_and_full_reservation() {
    let p = Paths::new();
    let mut s = create(&p, Limits::default());
    assert_eq!(
        s.write(4, |w| {
            w.bytes(b"x")?;
            Err(Error::Callback)
        })
        .err(),
        Some(Error::Callback)
    );
    assert_eq!(
        s.stats().unwrap().disk_bytes,
        CONTAINER_BYTES + RECORD_BYTES + 4
    );
    assert_eq!(s.stats().unwrap().records, 0);
    assert_eq!(
        fs::metadata(p.path.join(NAME)).unwrap().len() as usize,
        CONTAINER_BYTES + RECORD_BYTES + 1
    );
    assert_eq!(s.verify_unchanged(), Err(Error::Callback));
}
#[test]
fn append_archive_caught_writer_panic_cannot_reuse_owner() {
    let p = Paths::new();
    let mut s = create(&p, Limits::default());
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| s.write(4, |w| {
            w.bytes(b"x")?;
            panic!("authored failure");
        })))
        .is_err()
    );
    assert_eq!(s.verify_unchanged(), Err(Error::Changed));
    assert_eq!(s.write(1, |w| w.bytes(b"x")).err(), Some(Error::Changed));
}
#[test]
fn append_archive_lease_breaks_on_reader_and_writer_and_stays_failed() {
    for write in [false, true] {
        let p = Paths::new();
        let mut s = create(&p, Limits::default());
        let r = store(&mut s, b"x");
        let opened = OpenOptions::new()
            .read(!write)
            .write(write)
            .custom_flags(NONBLOCK)
            .open(p.path.join(NAME));
        assert!(opened.is_err());
        assert_eq!(s.verify_unchanged(), Err(Error::Changed));
        assert_eq!(s.with_record(r, 1, |_| Ok(())).err(), Some(Error::Changed));
    }
}
#[test]
fn append_archive_name_mode_and_link_changes_reject() {
    for change in 0..3 {
        let p = Paths::new();
        let mut s = create(&p, Limits::default());
        match change {
            0 => fs::rename(p.path.join(NAME), p.path.join("renamed")).unwrap(),
            1 => fs::set_permissions(p.path.join(NAME), fs::Permissions::from_mode(0o400)).unwrap(),
            _ => fs::hard_link(p.path.join(NAME), p.path.join("alias")).unwrap(),
        }
        assert_eq!(s.verify_unchanged(), Err(Error::Changed));
    }
}
#[test]
fn append_archive_selected_body_and_reserved_header_hashes_are_checked() {
    for header_change in [false, true] {
        let p = Paths::new();
        let mut s = create(&p, Limits::default());
        let r = store(&mut s, b"abc");
        let a = s.archive.as_mut().unwrap();
        // QA-only access to the otherwise private, unshared description. Forge
        // expected metadata to prove byte checks independently of metadata.
        let offset = if header_change {
            CONTAINER_BYTES + 63
        } else {
            CONTAINER_BYTES + RECORD_BYTES
        };
        a.file.write_all_at(b"x", offset as u64).unwrap();
        a.identity = Identity::of(&a.file.metadata().unwrap());
        let called = AtomicUsize::new(0);
        assert_eq!(
            s.with_record(r, 3, |_| {
                called.fetch_add(1, Ordering::Relaxed);
                Ok(())
            })
            .err(),
            Some(Error::Changed)
        );
        assert_eq!(called.load(Ordering::Relaxed), 0);
    }
}
#[test]
fn append_archive_source_and_parent_guards_remain_independent() {
    let p = Paths::new();
    let key = p.base.join("keys.txt");
    fs::write(&key, b"authored test key declaration only").unwrap();
    let mut s = Spool::create_with_backend(
        &p.path,
        ProtectedInputs::capture(&p.source, &[&key]).unwrap(),
        Limits::default(),
        spool::Profile::SameMountV1,
        resources::Profile::BoundedV1,
        BackendProfile::LeasedAppendV1,
    )
    .unwrap();
    fs::write(&key, b"changed declaration").unwrap();
    assert_eq!(s.verify_unchanged(), Err(Error::Changed));
}
#[test]
fn append_archive_read_allocation_and_framing_reload_caps_are_terminal() {
    let p = Paths::new();
    let mut s = create(
        &p,
        Limits {
            max_reload_bytes: RECORD_BYTES + 2,
            ..Limits::default()
        },
    );
    let r = store(&mut s, b"abc");
    assert_eq!(s.with_record(r, 3, |_| Ok(())).err(), Some(Error::Limit));
    assert_eq!(s.stats().unwrap().reloads, 0);
    let p = Paths::new();
    let mut s = create(&p, Limits::default());
    let r = store(&mut s, b"abc");
    assert_eq!(
        s.with_record_alloc(r, 3, |_| Err(Error::Allocation), |_| Ok(()))
            .err(),
        Some(Error::Allocation)
    );
    assert_eq!(s.stats().unwrap().reload_bytes, RECORD_BYTES + 3);
}
#[test]
fn append_archive_small_roundtrip_has_constant_record_guard_count() {
    let p = Paths::new();
    let mut s = create(&p, Limits::default());
    let before = s.archive.as_ref().unwrap().guard_count();
    let mut records = Vec::new();
    for i in 0..32u64 {
        records.push(store(&mut s, &i.to_le_bytes()));
    }
    for (i, r) in records.into_iter().enumerate() {
        assert_eq!(
            s.with_record(r, 8, |b| Ok(u64::from_le_bytes(b.try_into().unwrap())))
                .unwrap(),
            i as u64
        );
    }
    assert_eq!(s.archive.as_ref().unwrap().guard_count() - before, 5 * 32);
}
#[test]
#[ignore = "explicit 4097 distinct actual archive records and exact guard/FD corpus"]
fn explicit_append_archive_4097_distinct_records_linear_guard_and_bounded_fds() {
    let p = Paths::new();
    let mut s = create(&p, resources::Profile::DiskWorldV1.spool_limits());
    let fds = || fs::read_dir("/proc/self/fd").unwrap().count();
    let before_fds = fds();
    let before_guards = s.archive.as_ref().unwrap().guard_count();
    let mut records = Vec::new();
    let mut changes = 0;
    let mut cap = 0;
    for i in 0..4097u64 {
        records.push(store(&mut s, &i.to_le_bytes()));
        let next = s.archive.as_ref().unwrap().capacity();
        if next != cap {
            changes += 1;
            cap = next;
        }
    }
    for (i, r) in records.into_iter().enumerate() {
        assert_eq!(
            s.with_record(r, 8, |b| Ok(u64::from_le_bytes(b.try_into().unwrap())))
                .unwrap(),
            i as u64
        );
    }
    assert_eq!(
        s.archive.as_ref().unwrap().guard_count() - before_guards,
        5 * 4097
    );
    assert!(changes <= 14);
    assert_eq!(fds(), before_fds);
    assert!(before_fds < 1024);
    assert_eq!(s.stats().unwrap().records, 4097);
    assert_eq!(
        s.stats().unwrap().disk_bytes,
        CONTAINER_BYTES + 4097 * (RECORD_BYTES + 8)
    );
    println!(
        "append corpus: 4097 distinct actual records; 20485 archive guards; {changes} descriptor growths; {before_fds} steady FDs; {} disk bytes",
        s.stats().unwrap().disk_bytes
    );
}
