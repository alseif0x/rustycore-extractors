// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original synthetic loaded producer acquisition checks.

use super::*;

#[test]
fn whole_candidate_and_chunk_limits_are_separate() {
    let limits = Limits::default();
    assert_eq!(
        admitted_length(MAX_PRODUCER_BYTES as u64, MAX_PRODUCER_BYTES, limits),
        Ok(MAX_PRODUCER_BYTES)
    );
    assert_eq!(
        admitted_length(MAX_PRODUCER_BYTES as u64 + 1, MAX_PRODUCER_BYTES, limits),
        Err(Error::Limit)
    );
    assert_eq!(
        admitted_length(1, MAX_PRODUCER_BYTES + 1, limits),
        Err(Error::Limit)
    );
    assert_eq!(admitted_length(2, 1, limits), Err(Error::Limit));
    assert_eq!(
        admitted_length(u64::MAX, MAX_PRODUCER_BYTES, limits),
        Err(Error::Limit)
    );
    assert_eq!(admitted_length(0, 0, limits), Ok(0));
    let tiny = Limits {
        max_read_bytes: 1,
        ..limits
    };
    assert_eq!(admitted_length(4096, 4096, tiny), Ok(4096));
    assert_eq!(admitted_length(4097, 4097, tiny), Err(Error::Limit));
    let invalid = Limits {
        max_read_bytes: 0,
        ..limits
    };
    assert_eq!(admitted_length(0, 0, invalid), Err(Error::Limit));
}

#[test]
fn diagnostics_are_static() {
    assert_eq!(Error::Unsealed.to_string(), "producer_unsealed");
    assert_eq!(Error::Module.code(), "producer_module");
    assert_eq!(Error::Path.code(), "producer_path");
    assert_eq!(Error::Limit.code(), "producer_limit");
    assert_eq!(Error::Input.code(), "producer_input");
    assert_eq!(Error::Changed.code(), "producer_changed");
    assert_eq!(Error::Allocation.code(), "producer_allocation");
}

#[test]
fn ordinary_unsealed_link_rejects_before_path_acquisition() {
    // The source initializer is zero. This test must run before any test-exe
    // post-link sealing. snapshot rejects before module_path/file calls, even
    // with a four-unit path budget that cannot hold this synthetic test path.
    let result = LoadedProducer::capture(Limits {
        max_path_units: 4,
        ..Limits::default()
    });
    assert!(matches!(result, Err(Error::Unsealed)));
}

#[test]
#[ignore = "requires a post-link sealed copy of this synthetic test executable"]
fn sealed_live_main_module_matches_guarded_candidate() {
    let producer = LoadedProducer::capture(Limits {
        max_read_bytes: 64 * 1024,
        ..Limits::default()
    })
    .unwrap();
    let payload = producer.payload();
    assert_eq!(&payload[..32], &MARKER);
    assert!(payload[32..].iter().any(|byte| *byte != 0));
    assert!(producer.payload_rva() > 0);
    let bytes = producer.read_all(MAX_PRODUCER_BYTES).unwrap();
    assert_eq!(&bytes[..2], b"MZ");
    assert!(bytes.len() > 64 * 1024);
    assert_eq!(bytes.len() as u64, producer.source.identity().size);
    producer.verify_unchanged().unwrap();
    let hex: String = payload.iter().map(|byte| format!("{byte:02x}")).collect();
    println!(
        "loaded_payload={hex} payload_rva={} file_bytes={}",
        producer.payload_rva(),
        bytes.len()
    );
    assert!(matches!(producer.read_all(0), Err(Error::Limit)));
    assert!(matches!(
        producer.read_all(MAX_PRODUCER_BYTES + 1),
        Err(Error::Limit)
    ));
    producer.verify_unchanged().unwrap();
    // Attempted writable open must fail while the candidate lease is alive.
    let path = ffi::module_path(Limits::default()).unwrap();
    assert!(std::fs::OpenOptions::new().write(true).open(path).is_err());
    producer.invalid.set(true);
    assert_eq!(producer.verify_unchanged(), Err(Error::Changed));
    assert!(matches!(
        producer.read_all(MAX_PRODUCER_BYTES),
        Err(Error::Changed)
    ));
}

#[test]
#[ignore = "requires a post-link sealed copy of this synthetic test executable"]
fn sealed_chunk_work_limit_rejects_without_poisoning() {
    let producer = LoadedProducer::capture(Limits {
        max_read_bytes: 1,
        ..Limits::default()
    })
    .unwrap();
    assert!(producer.source.identity().size > MAX_PRODUCER_CHUNKS as u64);
    assert!(matches!(
        producer.read_all(MAX_PRODUCER_BYTES),
        Err(Error::Limit)
    ));
    producer.verify_unchanged().unwrap();
}
