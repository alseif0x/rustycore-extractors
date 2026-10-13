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

#[test]
fn selected_literal_is_exact_and_rejects_other_profile_without_fallback() {
    let mut payload = ffi::test_initial_payload();
    assert_eq!(&payload[..32], &MARKER);
    assert_eq!(&payload[32..], &[0; 32]);
    assert_eq!(admit_payload(&payload), Err(Error::Unsealed));
    payload[32] = 1;
    assert_eq!(admit_payload(&payload), Ok(()));
    let other = if cfg!(all(feature = "producer-msvc-v2", target_env = "msvc")) {
        assert_eq!(
            WINDOWS_PRODUCER_PROFILE,
            "forever-windows-producer-seal-msvc-v2"
        );
        assert_eq!(MARKER, *b"RCE-FOREVER-PE-SEAL-V2\0\0\0\0\0\0\0\0\0\0");
        *b"RCE-FOREVER-PE-SEAL-V1\0\0\0\0\0\0\0\0\0\0"
    } else {
        assert_eq!(WINDOWS_PRODUCER_PROFILE, "forever-windows-producer-seal-v1");
        assert_eq!(MARKER, *b"RCE-FOREVER-PE-SEAL-V1\0\0\0\0\0\0\0\0\0\0");
        *b"RCE-FOREVER-PE-SEAL-V2\0\0\0\0\0\0\0\0\0\0"
    };
    payload[..32].copy_from_slice(&other);
    assert_eq!(admit_payload(&payload), Err(Error::Unsealed));
    payload[..32].copy_from_slice(&MARKER);
    payload[31] = 1;
    assert_eq!(admit_payload(&payload), Err(Error::Unsealed));
}

#[test]
fn machine_admission_requires_success_unknown_process_and_native_amd64() {
    assert_eq!(admit_architecture(1, 0, 0x8664), Ok(()));
    // BOOL success means nonzero, not one particular positive representation.
    assert_eq!(admit_architecture(2, 0, 0x8664), Ok(()));
    for (success, process, native) in [
        (0, 0, 0),
        (0, 0, 0x8664),
        (1, 0, 0),
        (1, 0x014c, 0x8664),
        (1, 0x8664, 0x8664),
        (1, 0, 0xaa64),
        (1, 0x8664, 0xaa64),
        (1, 0xa641, 0xaa64),
        (1, 0x01c4, 0xaa64),
        (1, 0, 0x0200),
    ] {
        assert_eq!(
            admit_architecture(success, process, native),
            Err(Error::Architecture)
        );
    }
    assert_eq!(Error::Architecture.code(), "producer_architecture");
}

#[test]
fn mocked_machine_gate_precedes_loaded_snapshot_without_path_fallback() {
    for (success, process, native) in [(0, 0, 0x8664), (1, 0x8664, 0xaa64), (1, 0, 0xaa64)] {
        let acquired = Cell::new(false);
        let result = ffi::guarded_snapshot(
            || admit_architecture(success, process, native),
            || {
                acquired.set(true);
                Err(Error::Module)
            },
        );
        assert!(matches!(result, Err(Error::Architecture)));
        assert!(!acquired.get());
    }
    let acquired = Cell::new(false);
    let result = ffi::guarded_snapshot(
        || admit_architecture(1, 0, 0x8664),
        || {
            acquired.set(true);
            Err(Error::Module)
        },
    );
    assert!(matches!(result, Err(Error::Module)));
    assert!(acquired.get());
}

#[cfg(all(feature = "producer-msvc-v2", target_env = "msvc"))]
#[test]
fn msvc_v2_native_machine_query_admits_actual_amd64_test_process() {
    // Native CI uses Windows Server2022 AMD64. The profile deliberately rejects
    // this test on ARM64/emulation instead of claiming compatible acceptance.
    ffi::native_architecture().unwrap();
    assert!(matches!(ffi::snapshot(), Err(Error::Unsealed)));
}

#[test]
fn mocked_architecture_recheck_failure_poisons_owner_before_snapshot_or_file_guard() {
    // This constructs synthetic owner fields over the authored test executable;
    // it does not claim that those bytes have a valid live PE seal.
    let path = std::env::current_exe().unwrap();
    let limits = Limits::default();
    let source = ReadFile::open(&path, limits, LinkPolicy::Allow).unwrap();
    let mut payload = ffi::test_initial_payload();
    payload[32] = 1;
    let owner = LoadedProducer {
        payload,
        payload_rva: 64,
        source,
        limits,
        invalid: Cell::new(false),
    };
    let acquired = Cell::new(false);
    let result = owner.verify_snapshot(|| {
        ffi::guarded_snapshot(
            || admit_architecture(1, 0x8664, 0xaa64),
            || {
                acquired.set(true);
                Err(Error::Module)
            },
        )
    });
    assert_eq!(result, Err(Error::Changed));
    assert!(!acquired.get());
    assert!(owner.invalid.get());
    assert_eq!(
        owner.verify_snapshot(|| {
            acquired.set(true);
            Err(Error::Module)
        }),
        Err(Error::Changed)
    );
    assert!(!acquired.get());
}
#[test]
fn mocked_payload_profile_or_rva_change_poison_without_adopting_candidate() {
    for changed_rva in [false, true] {
        let limits = Limits::default();
        let source =
            ReadFile::open(&std::env::current_exe().unwrap(), limits, LinkPolicy::Allow).unwrap();
        let mut payload = ffi::test_initial_payload();
        payload[32] = 1;
        let owner = LoadedProducer {
            payload,
            payload_rva: 64,
            source,
            limits,
            invalid: Cell::new(false),
        };
        let mut replacement = payload;
        if !changed_rva {
            replacement[21] = if replacement[21] == b'1' { b'2' } else { b'1' };
        }
        assert_eq!(
            owner.verify_snapshot(|| Ok(ffi::Snapshot {
                payload: replacement,
                payload_rva: if changed_rva { 65 } else { 64 }
            })),
            Err(Error::Changed)
        );
        assert!(owner.invalid.get());
        assert_eq!(owner.payload(), payload);
        assert_eq!(owner.payload_rva(), 64);
    }
}
