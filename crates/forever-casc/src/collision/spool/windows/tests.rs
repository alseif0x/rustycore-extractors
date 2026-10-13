// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-13: native synthetic exclusive append facade checks.
use super::*;
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture {
    base: PathBuf,
    source: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let base = std::env::temp_dir().join(format!(
            "rce-win-spool-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&base).unwrap();
        let source = base.join("source");
        fs::create_dir(&source).unwrap();
        Self { base, source }
    }
    fn create(&self, name: &str, limits: Limits) -> Result<Spool, Error> {
        Spool::create(
            &self.base.join(name),
            ProtectedInputs::capture(&self.source, &[])?,
            limits,
        )
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.base).unwrap();
    }
}

#[test]
fn windows_profiles_are_explicit_and_numeric_caps_are_downward_only() {
    assert_eq!(
        BackendProfile::default().id(),
        "windows-exclusive-append-v1"
    );
    assert_eq!(
        Profile::default().id(),
        "windows-fixed-ntfs-spool-volumes-v1"
    );
    assert_eq!(
        resources::Profile::DiskWorldV1.id(),
        "windows-collision-disk-world-resources-v1"
    );
    for axis in 0..6 {
        let mut limits = resources::Profile::DiskWorldV1.spool_limits();
        match axis {
            0 => limits.max_records += 1,
            1 => limits.max_disk_bytes += 1,
            2 => limits.max_active_bytes += 1,
            3 => limits.max_descriptor_bytes += 1,
            4 => limits.max_reloads += 1,
            _ => limits.max_reload_bytes += 1,
        }
        assert_eq!(
            limits.validate_resources(resources::Profile::DiskWorldV1),
            Err(Error::InvalidLimits)
        );
    }
}

#[test]
fn empty_binary_and_chunked_records_roundtrip_exactly_once() {
    let f = Fixture::new();
    let mut spool = f.create("records", Limits::default()).unwrap();
    let bodies = [Vec::new(), vec![0, 255, 17], vec![19; IO_BYTES + 17]];
    let mut records = Vec::new();
    for body in &bodies {
        let mut calls = 0;
        records.push(
            spool
                .write(body.len(), |out| {
                    calls += 1;
                    out.bytes(body)
                })
                .unwrap(),
        );
        assert_eq!(calls, 1);
    }
    for (record, expected) in records.into_iter().zip(&bodies) {
        spool
            .with_record(record, expected.len(), |body| {
                assert_eq!(body, expected);
                Ok(())
            })
            .unwrap();
    }
    let stats = spool.stats().unwrap();
    let cost = bodies.iter().map(|b| RECORD_BYTES + b.len()).sum::<usize>();
    assert_eq!(
        (
            stats.records,
            stats.disk_bytes,
            stats.reloads,
            stats.reload_bytes
        ),
        (3, CONTAINER_BYTES + cost, 3, cost)
    );
    assert!(stats.descriptor_bytes <= spool.limits.max_descriptor_bytes);
    assert!(stats.peak_reload_bytes <= spool.limits.max_active_bytes);
    spool.verify_unchanged().unwrap();
}

#[test]
fn archive_exclusion_blocks_foreign_read_write_truncate_and_delete() {
    let f = Fixture::new();
    let mut spool = f.create("records", Limits::default()).unwrap();
    let record = spool.write(3, |w| w.bytes(b"abc")).unwrap();
    let file = f.base.join("records/records.archive");
    assert!(fs::File::open(&file).is_err());
    assert!(fs::OpenOptions::new().write(true).open(&file).is_err());
    assert!(
        fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&file)
            .is_err()
    );
    assert!(fs::remove_file(&file).is_err());
    spool
        .with_record(record, 3, |b| {
            assert_eq!(b, b"abc");
            Ok(())
        })
        .unwrap();
}

#[test]
fn fresh_namespace_rejects_existing_output_without_overwrite() {
    let f = Fixture::new();
    fs::create_dir(f.base.join("existing")).unwrap();
    fs::write(f.base.join("existing/sentinel"), b"preserved").unwrap();
    assert!(f.create("existing", Limits::default()).is_err());
    assert_eq!(
        fs::read(f.base.join("existing/sentinel")).unwrap(),
        b"preserved"
    );
    assert!(!f.base.join("existing/records.archive").exists());
}

#[test]
fn protected_source_containment_rejects_before_create() {
    let f = Fixture::new();
    let path = f.source.join("forbidden");
    let result = Spool::create(
        &path,
        ProtectedInputs::capture(&f.source, &[]).unwrap(),
        Limits::default(),
    );
    assert!(result.is_err());
    assert!(!path.exists());
}

#[test]
fn descriptors_charge_full_growth_overlap_without_records() {
    let f = Fixture::new();
    let mut spool = f.create("records", Limits::default()).unwrap();
    assert_eq!(
        spool.prepare_records(1, size_of::<Entry>() - 1),
        Err(Error::Limit)
    );
    spool.prepare_records(1, size_of::<Entry>()).unwrap();
    let capacity = spool.entries.capacity();
    let before = spool.resident_bytes().unwrap();
    assert_eq!(
        spool.prepare_records(capacity + 1, allocation(capacity + 1).unwrap() - 1),
        Err(Error::Limit)
    );
    assert_eq!(spool.resident_bytes().unwrap(), before);
    assert_eq!(spool.stats().unwrap().records, 0);
}

#[test]
fn record_disk_active_and_reload_boundaries_count_framing() {
    let f = Fixture::new();
    let limits = Limits {
        max_records: 1,
        max_disk_bytes: CONTAINER_BYTES + RECORD_BYTES + 3,
        max_active_bytes: 3,
        max_reloads: 1,
        max_reload_bytes: RECORD_BYTES + 3,
        ..Limits::default()
    };
    let mut spool = f.create("records", limits).unwrap();
    let record = spool.write(3, |w| w.bytes(b"abc")).unwrap();
    spool.with_record(record, 3, |_| Ok(())).unwrap();
    assert_eq!(spool.with_record(record, 3, |_| Ok(())), Err(Error::Limit));
    assert_eq!(spool.stats().unwrap().reloads, 1);
    assert_eq!(spool.write(0, |_| Ok(())).err(), Some(Error::Limit));
}

#[test]
fn insufficient_container_budget_rejects_before_namespace_creation() {
    let f = Fixture::new();
    assert_eq!(
        f.create(
            "records",
            Limits {
                max_disk_bytes: CONTAINER_BYTES - 1,
                ..Limits::default()
            }
        )
        .err(),
        Some(Error::Limit)
    );
    assert!(!f.base.join("records").exists());
}

#[test]
fn short_or_long_stream_poison_without_committing_a_record() {
    for long in [false, true] {
        let f = Fixture::new();
        let mut spool = f.create("records", Limits::default()).unwrap();
        let error = spool
            .write(2, |w| w.bytes(if long { b"abc" } else { b"a" }))
            .err()
            .unwrap();
        assert_eq!(error, Error::Format);
        assert_eq!(spool.stats().unwrap().records, 0);
        assert_eq!(spool.verify_unchanged(), Err(error));
        assert_eq!(spool.write(0, |_| Ok(())).err(), Some(error));
    }
}

#[test]
fn write_callback_error_is_preserved_and_partial_bytes_are_not_reused() {
    let f = Fixture::new();
    let mut spool = f.create("records", Limits::default()).unwrap();
    assert_eq!(
        spool
            .write(2, |w| {
                w.bytes(b"a")?;
                Err(Error::Callback)
            })
            .err(),
        Some(Error::Callback)
    );
    assert_eq!(spool.stats().unwrap().records, 0);
    assert_eq!(spool.verify_unchanged(), Err(Error::Callback));
    assert!(f.base.join("records/records.archive").exists());
}

#[test]
fn caught_write_panic_permanently_invalidates_the_owner() {
    let f = Fixture::new();
    let mut spool = f.create("records", Limits::default()).unwrap();
    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = spool.write(2, |w| {
            w.bytes(b"a")?;
            panic!("synthetic callback panic")
        });
    }));
    assert!(panic.is_err());
    assert_eq!(spool.failure(), Some(Error::State));
    assert!(spool.verify_unchanged().is_err());
    assert_eq!(spool.stats().unwrap().records, 0);
    assert!(spool.write(0, |_| Ok(())).is_err());
}

#[test]
fn caught_read_panic_is_visible_to_metadata_only_owner_checks() {
    let f = Fixture::new();
    let mut spool = f.create("records", Limits::default()).unwrap();
    let record = spool.write(1, |w| w.bytes(b"x")).unwrap();
    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _: Result<(), _> = spool.with_record(record, 1, |_| panic!("synthetic read panic"));
    }));
    assert!(panic.is_err());
    assert_eq!(spool.failure(), Some(Error::State));
    assert!(spool.verify_unchanged().is_err());
}

#[test]
fn read_callback_failure_poison_preserves_cumulative_attempts() {
    let f = Fixture::new();
    let mut spool = f.create("records", Limits::default()).unwrap();
    let record = spool.write(3, |w| w.bytes(b"abc")).unwrap();
    assert_eq!(
        spool.with_record::<()>(record, 3, |_| Err(Error::Callback)),
        Err(Error::Callback)
    );
    assert_eq!(spool.verify_unchanged(), Err(Error::Callback));
    assert_eq!(
        (
            spool.stats().unwrap().reloads,
            spool.stats().unwrap().reload_bytes
        ),
        (1, RECORD_BYTES + 3)
    );
}

#[test]
fn failed_callback_capture_drops_before_final_namespace_guards() {
    struct OnDrop(PathBuf);
    impl Drop for OnDrop {
        fn drop(&mut self) {
            fs::write(self.0.join("new-entry"), b"synthetic").unwrap();
        }
    }
    let f = Fixture::new();
    let mut spool = f.create("records", Limits::default()).unwrap();
    let record = spool.write(1, |w| w.bytes(b"x")).unwrap();
    let capture = OnDrop(f.base.join("records"));
    let result = spool.with_record::<()>(record, 1, move |_| {
        drop(capture);
        Err(Error::Callback)
    });
    assert_eq!(result, Err(Error::Changed));
    assert_eq!(spool.verify_unchanged(), Err(Error::Changed));
}

#[test]
fn refused_reload_drops_uninvoked_capture_before_final_namespace_guards() {
    struct OnDrop(PathBuf);
    impl Drop for OnDrop {
        fn drop(&mut self) {
            fs::write(self.0.join("unknown"), b"synthetic").unwrap();
        }
    }
    let f = Fixture::new();
    let mut spool = f.create("records", Limits::default()).unwrap();
    let record = spool.write(1, |w| w.bytes(b"x")).unwrap();
    let capture = OnDrop(f.base.join("records"));
    let mut invoked = false;
    let result = spool.with_record(record, 0, |_| {
        invoked = true;
        drop(capture);
        Ok(())
    });
    assert!(!invoked);
    assert_eq!(result, Err(Error::Changed));
    assert_eq!(spool.stats().unwrap().reloads, 0);
}

#[test]
fn disk_world_streams_more_than_4096_committed_records_in_one_file() {
    let f = Fixture::new();
    let policy = resources::Profile::DiskWorldV1;
    let mut spool = Spool::create_with_policy(
        &f.base.join("records"),
        ProtectedInputs::capture(&f.source, &[]).unwrap(),
        Limits {
            max_records: 4097,
            ..policy.spool_limits()
        },
        Profile::FixedNtfsV1,
        policy,
    )
    .unwrap();
    for ordinal in 0..4097 {
        let record = spool.write(0, |_| Ok(())).unwrap();
        assert_eq!(record.0, ordinal);
    }
    assert_eq!(spool.stats().unwrap().records, 4097);
    assert_eq!(fs::read_dir(f.base.join("records")).unwrap().count(), 2);
    assert!(spool.stats().unwrap().descriptor_bytes < 1024 * 1024);
    spool.verify_unchanged().unwrap();
    drop(spool);
    let bytes = fs::read(f.base.join("records/records.archive")).unwrap();
    assert_eq!(&bytes[..CONTAINER_BYTES], &container());
    let digest = Sha256::new().finish();
    let (records, remainder) = bytes[CONTAINER_BYTES..].as_chunks::<RECORD_BYTES>();
    assert!(remainder.is_empty());
    for (ordinal, bytes) in records.iter().enumerate() {
        assert_eq!(bytes, &header(ordinal, 0, digest));
    }
}

#[test]
fn streamed_aggregate_above_256_mib_keeps_only_one_bounded_reload_active() {
    let f = Fixture::new();
    let size = 129 * 1024 * 1024;
    let mut spool = f
        .create(
            "records",
            Limits {
                max_active_bytes: size,
                ..Limits::default()
            },
        )
        .unwrap();
    let chunk = vec![23u8; 64 * 1024];
    let mut records = Vec::new();
    for _ in 0..2 {
        records.push(
            spool
                .write(size, |writer| {
                    for _ in 0..size / chunk.len() {
                        writer.bytes(&chunk)?;
                    }
                    Ok(())
                })
                .unwrap(),
        );
    }
    assert!(spool.stats().unwrap().disk_bytes > 256 * 1024 * 1024);
    let resident = spool.resident_bytes().unwrap();
    for record in records {
        spool
            .with_record(record, size, |body| {
                assert_eq!(body.len(), size);
                assert!(body.iter().all(|byte| *byte == 23));
                Ok(())
            })
            .unwrap();
        assert_eq!(spool.resident_bytes().unwrap(), resident);
    }
    assert!(spool.stats().unwrap().peak_reload_bytes <= size);
    assert_eq!(spool.stats().unwrap().reloads, 2);
}
