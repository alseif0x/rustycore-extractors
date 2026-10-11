// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: authored synthetic guarded local framing only.
use super::*;
use crate::blte::framing::Form;
use crate::local_store::{
    framing as f,
    routes::{RouteState, SpanState},
};

const TARGETS: [u32; 3] = [1_349_477, 1_266_278, 1_266_277];
fn builder() -> Builder {
    let mut b = Builder::new(CONTENT);
    b.root = TARGETS.map(|id| (id, 0, 2, md5::digest(CONTENT))).to_vec();
    b
}
fn report(store: &LocalStore) -> [f::Diagnostic; 3] {
    store
        .diagnose_required_framing(f::Limits::default())
        .unwrap_or_else(|e| panic!("{}", e.code()))
}
fn error(store: &LocalStore, limits: f::Limits) -> &'static str {
    store
        .diagnose_required_framing(limits)
        .err()
        .expect("expected failure")
        .code()
}
fn framed(frames: &[(&[u8], u32)]) -> Vec<u8> {
    let mut bytes = b"BLTE".to_vec();
    bytes.extend_from_slice(&(12 + 24 * frames.len() as u32).to_be_bytes());
    bytes.push(0x0f);
    bytes.extend_from_slice(&(frames.len() as u32).to_be_bytes()[1..]);
    for &(frame, size) in frames {
        bytes.extend_from_slice(&(frame.len() as u32).to_be_bytes());
        bytes.extend_from_slice(&size.to_be_bytes());
        bytes.extend_from_slice(&md5::digest(frame));
    }
    for &(frame, _) in frames {
        bytes.extend_from_slice(frame);
    }
    bytes
}
fn encrypted() -> Vec<u8> {
    let mut out = vec![b'E', 8];
    out.extend_from_slice(&[0xa1; 8]);
    out.push(4);
    out.extend_from_slice(&[0xb2; 4]);
    out.push(b'S');
    out.extend_from_slice(b"opaque synthetic private payload");
    out
}
fn mixed() -> Vec<u8> {
    framed(&[(&encrypted(), CONTENT.len() as u32 - 3), (b"Nabc", 3)])
}
fn supplied(encoded: &[u8]) -> Fixture {
    let mut fixture = builder().write();
    let path = fixture.path.join("Data/data/data.000");
    let mut archive = fs::read(&path).unwrap();
    let offset = archive.len() as u64;
    archive.extend_from_slice(&[0xff; 30]); // Deliberately unauthenticated prelude.
    archive.extend_from_slice(encoded);
    fs::write(path, archive).unwrap();
    fixture.rows[0] = index_entry(EKEY, offset, (encoded.len() + 30) as u32);
    fixture.rewrite_rows();
    fixture
}
fn complete(inspection: f::Inspection) -> crate::blte::framing::Evidence {
    match inspection {
        f::Inspection::Complete(e) => e,
        other => panic!("{other:?}"),
    }
}

#[test]
fn closed_three_inputs_headerless_unhashed_no_plaintext_claim() {
    let fixture = builder().write();
    let store = fixture.open();
    for (row, id) in report(&store).iter().zip(TARGETS) {
        assert_eq!(row.input.file_data_id(), id);
        assert_eq!(row.route, RouteState::Routed);
        assert_eq!(row.root_rows, 1);
        assert_eq!(row.candidates.len(), 1);
        let e = complete(row.candidates[0].inspection);
        assert_eq!(e.form, Form::HeaderlessUnhashed);
        assert_eq!(e.verified_frame_md5_count, 0);
        assert_eq!(e.uncompressed_frames, 1);
        assert_eq!(e.expected_content_size, CONTENT.len() as u64);
    }
    assert_eq!(f::PROFILE, "forever-local-required-framing-v1");
}
#[test]
fn complete_encrypted_first_and_later_n_frame_all_hashes_verified() {
    let fixture = supplied(&mixed());
    for row in report(&fixture.open()) {
        let e = complete(row.candidates[0].inspection);
        assert_eq!(e.form, Form::FramedMd5);
        assert_eq!(e.verified_frame_md5_count, 2);
        assert_eq!(e.encrypted_frames, 1);
        assert_eq!(e.uncompressed_frames, 1);
    }
}
#[test]
fn encrypted_first_cannot_hide_later_bad_md5_or_truncation() {
    let mut bad_hash = mixed();
    *bad_hash.last_mut().unwrap() ^= 1;
    let mut truncated = mixed();
    truncated.pop();
    for (encoded, code) in [
        (bad_hash, "blte_checksum_mismatch"),
        (truncated, "blte_truncated"),
    ] {
        let fixture = supplied(&encoded);
        for row in report(&fixture.open()) {
            assert_eq!(row.candidates[0].span, SpanState::FitsObservedLength);
            assert_eq!(row.candidates[0].inspection, f::Inspection::Rejected(code));
        }
    }
}
#[test]
fn zero_frame_md5_rejected_without_partial_evidence() {
    let mut encoded = mixed();
    encoded[20..36].fill(0);
    let fixture = supplied(&encoded);
    for row in report(&fixture.open()) {
        assert_eq!(
            row.candidates[0].inspection,
            f::Inspection::Rejected("blte_checksum_mismatch")
        );
    }
}
#[test]
fn report_evidence_has_no_payload_identifier_nonce_or_digest() {
    let fixture = supplied(&mixed());
    let rows = report(&fixture.open());
    let row = &rows[0];
    let debug = format!("{:?}", row.candidates[0].inspection);
    for raw in [
        "opaque synthetic",
        "a1a1",
        "b2b2",
        "data.000",
        &hex(EKEY),
        &hex(md5::digest(CONTENT)),
    ] {
        assert!(!debug.contains(raw));
    }
}
#[test]
fn full_ekey_suffix_and_prelude_are_not_authenticated() {
    let mut b = builder();
    let mut ekey = EKEY;
    ekey[15] ^= 1;
    b.routes[0].2 = vec![ekey];
    let fixture = b.write();
    assert!(
        report(&fixture.open())
            .iter()
            .all(|r| matches!(r.candidates[0].inspection, f::Inspection::Complete(_)))
    );
}
#[test]
fn pure_z_is_opaque_and_unsupported_modes_are_static_rejections() {
    for (mode, expected) in [
        (b'Z', None),
        (b'F', Some("blte_recursive_unsupported")),
        (0xff, Some("blte_unknown_mode")),
        (b'E', Some("blte_invalid_encryption_header")),
    ] {
        let mut b = builder();
        b.records[0].1 = mode;
        let fixture = b.write();
        let rows = report(&fixture.open());
        match expected {
            Some(code) => assert_eq!(
                rows[0].candidates[0].inspection,
                f::Inspection::Rejected(code)
            ),
            None => assert_eq!(complete(rows[0].candidates[0].inspection).zlib_frames, 1),
        }
    }
}
#[test]
fn missing_incomplete_invalid_and_unsafe_spans_remain_not_read() {
    for (offset, size, expected) in [
        (1u64 << 30, 39, SpanState::ArchiveAbsent),
        ((1 << 30) - 1, 39, SpanState::IncompleteSpan),
        (0, 38, SpanState::InvalidExtent),
    ] {
        let mut fixture = builder().write();
        fixture.rows[0] = index_entry(EKEY, offset, size);
        fixture.rewrite_rows();
        let rows = report(&fixture.open());
        assert_eq!(rows[0].candidates[0].span, expected);
        assert_eq!(rows[0].candidates[0].inspection, f::Inspection::NotRead);
    }
    let mut fixture = builder().write();
    fixture.rows[0] = index_entry(EKEY, 1 << 30, 39);
    fixture.rewrite_rows();
    fs::create_dir(fixture.path.join("Data/data/data.001")).unwrap();
    assert_eq!(
        report(&fixture.open())[0].candidates[0].span,
        SpanState::ArchiveRejected
    );
}
#[test]
fn unknown_expected_size_is_metadata_only_and_not_inspected() {
    let mut b = builder();
    b.routes[0].1 = u32::MAX;
    b.records[0].1 = 0xff;
    let fixture = b.write();
    let rows = report(&fixture.open());
    assert_eq!(rows[0].route, RouteState::UnknownDecodedSize);
    assert_eq!(rows[0].candidates[0].inspection, f::Inspection::NotRead);
}
#[test]
fn root_selection_never_falls_back_to_available_later_root() {
    let mut b = builder();
    let key = [0x77; 16];
    b.root.insert(0, (TARGETS[0], 0, 2, key));
    b.routes.insert(0, (key, 123, vec![[0x88; 16]]));
    let fixture = b.write();
    let rows = report(&fixture.open());
    assert_eq!(rows[0].selected_root_ordinal, Some(0));
    assert_eq!(rows[0].candidates[0].span, SpanState::IndexAbsent);
    assert_eq!(rows[0].candidates[0].inspection, f::Inspection::NotRead);
}
#[test]
fn root_absent_ineligible_and_central_absent_remain_distinct() {
    let fixture = Builder::new(CONTENT).write();
    assert_eq!(report(&fixture.open())[0].route, RouteState::RootAbsent);
    let mut b = builder();
    b.root[0].2 = 0x80;
    let fixture = b.write();
    assert_eq!(report(&fixture.open())[0].route, RouteState::RootIneligible);
    let mut b = builder();
    b.routes.clear();
    let fixture = b.write();
    assert_eq!(report(&fixture.open())[0].route, RouteState::CentralAbsent);
}
#[test]
fn encoding_alternatives_and_prefix_collisions_preserve_every_ordinal() {
    let mut b = builder();
    let mut alt = EKEY;
    alt[15] ^= 1;
    b.routes[0].2 = vec![EKEY, alt];
    let mut fixture = b.write();
    fixture.rows.insert(1, index_entry(EKEY, 1 << 30, 39));
    fixture.rewrite_rows();
    let rows = report(&fixture.open());
    let c = &rows[0].candidates;
    assert_eq!(c.len(), 4);
    assert_eq!(
        c.iter()
            .map(|v| (v.encoding_ordinal, v.index_match_ordinal))
            .collect::<Vec<_>>(),
        vec![(0, Some(0)), (0, Some(1)), (1, Some(0)), (1, Some(1))]
    );
    assert_eq!(c[0].span, SpanState::FitsObservedLength);
    assert_eq!(c[1].span, SpanState::ArchiveAbsent);
}
#[test]
fn every_cap_only_reduces_and_all_candidate_bytes_count() {
    let fixture = builder().write();
    let store = fixture.open();
    let d = f::Limits::default();
    for l in [
        f::Limits {
            max_record_bytes: d.max_record_bytes + 1,
            ..d
        },
        f::Limits {
            max_total_record_bytes: d.max_total_record_bytes + 1,
            ..d
        },
        f::Limits {
            max_declared_content_bytes: d.max_declared_content_bytes + 1,
            ..d
        },
        f::Limits {
            max_frames: d.max_frames + 1,
            ..d
        },
        f::Limits {
            max_report_bytes: d.max_report_bytes + 1,
            ..d
        },
        f::Limits {
            routes: routes::Limits {
                max_root_rows_per_input: 65,
                ..d.routes
            },
            ..d
        },
        f::Limits {
            routes: routes::Limits {
                max_candidates_per_input: 65,
                ..d.routes
            },
            ..d
        },
        f::Limits {
            routes: routes::Limits {
                max_candidates_total: 193,
                ..d.routes
            },
            ..d
        },
        f::Limits {
            max_record_bytes: 38,
            ..d
        },
        f::Limits {
            max_total_record_bytes: 1,
            ..d
        },
        f::Limits {
            max_declared_content_bytes: 1,
            ..d
        },
        f::Limits {
            max_report_bytes: 1,
            ..d
        },
        f::Limits {
            routes: routes::Limits {
                max_candidates_total: 2,
                ..d.routes
            },
            ..d
        },
    ] {
        assert_eq!(error(&store, l), "local_store_policy_limit");
    }
    let mut fixture = builder().write();
    fixture.rows.insert(1, fixture.rows[0]);
    fixture.rewrite_rows();
    let store = fixture.open();
    assert_eq!(
        error(
            &store,
            f::Limits {
                routes: routes::Limits {
                    max_candidates_per_input: 1,
                    ..d.routes
                },
                ..d
            }
        ),
        "local_store_policy_limit"
    );
}
#[test]
fn exact_record_total_and_report_cap_boundaries() {
    let fixture = builder().write();
    let store = fixture.open();
    let d = f::Limits::default();
    let n = CONTENT.len() as u64 + 39;
    assert!(
        store
            .diagnose_required_framing(f::Limits {
                max_record_bytes: n,
                max_total_record_bytes: 3 * n,
                ..d
            })
            .is_ok()
    );
    assert_eq!(
        error(
            &store,
            f::Limits {
                max_total_record_bytes: 3 * n - 1,
                ..d
            }
        ),
        "local_store_policy_limit"
    );
    let rows = report(&store);
    let bytes = std::mem::size_of_val(&rows)
        + rows
            .iter()
            .map(|r| r.candidates.capacity() * std::mem::size_of::<f::Candidate>())
            .sum::<usize>();
    assert!(
        store
            .diagnose_required_framing(f::Limits {
                max_report_bytes: bytes,
                ..d
            })
            .is_ok()
    );
    assert_eq!(
        error(
            &store,
            f::Limits {
                max_report_bytes: bytes - 1,
                ..d
            }
        ),
        "local_store_policy_limit"
    );
}
#[test]
fn frame_budget_failure_is_static_rejection_after_complete_read() {
    let fixture = supplied(&mixed());
    let store = fixture.open();
    let rows = store
        .diagnose_required_framing(f::Limits {
            max_frames: 1,
            ..f::Limits::default()
        })
        .unwrap_or_else(|e| panic!("{}", e.code()));
    assert_eq!(
        rows[0].candidates[0].inspection,
        f::Inspection::Rejected("blte_policy_limit")
    );
}
#[test]
fn key_bearing_store_rejects_without_key_lookup() {
    let fixture = builder().write();
    let keys = crate::keys::Keyring::parse(
        b"forever-tact-keys\t1\n0000000000000001\t00000000000000000000000000000000\n",
    )
    .unwrap();
    let store = LocalStore::open_with_keys(&fixture.path, options(), keys).unwrap();
    assert_eq!(
        error(&store, f::Limits::default()),
        "route_diagnostic_requires_key_free_store"
    );
}
#[test]
fn scratch_resident_and_combined_admission_fail_without_partial_report() {
    let fixture = builder().write();
    let mut store = fixture.open();
    let d = store.limits;
    for (scratch, resident, total) in [
        (1, d.max_resident_bytes, d.max_memory_bytes),
        (
            d.max_scratch_bytes,
            store.stats.resident_bytes,
            d.max_memory_bytes,
        ),
        (
            d.max_scratch_bytes,
            d.max_resident_bytes,
            store.stats.resident_bytes,
        ),
    ] {
        store.limits = Limits {
            max_scratch_bytes: scratch,
            max_resident_bytes: resident,
            max_memory_bytes: total,
            ..d
        };
        assert_eq!(
            error(&store, f::Limits::default()),
            "local_store_policy_limit"
        );
    }
}
#[cfg(target_os = "linux")]
#[test]
fn held_lease_change_overrides_success_parse_failure_and_allocation_limit() {
    for which in 0..3 {
        let fixture = if which == 1 {
            let mut b = mixed();
            b.pop();
            supplied(&b)
        } else {
            supplied(&mixed())
        };
        let store = fixture.open();
        let entry = store
            .indexes
            .iter()
            .find(|e| e.key_prefix == EKEY[..9])
            .unwrap();
        let budget = Budget {
            limits: Limits {
                max_scratch_bytes: if which == 2 { 0 } else { usize::MAX },
                ..store.limits
            },
            resident: store.stats.resident_bytes,
            scratch: Cell::new(0),
        };
        let path = fixture.path.join("Data/data/data.000");
        let result = io::framing::inspect_with_hook(
            io::framing::Request {
                directory: &store.data_dir,
                entry,
                ekey: EKEY,
                expected: CONTENT.len() as u64,
                store_limits: store.limits,
                limits: f::Limits::default(),
                budget: &budget,
                scratch: 0,
            },
            || {
                let file = fs::OpenOptions::new().write(true).open(path).unwrap();
                file.set_len(0).unwrap();
            },
        );
        assert_eq!(result.err(), Some(Error::InputChanged));
    }
}
#[test]
fn final_store_change_overrides_preceding_operation_limit() {
    let fixture = builder().write();
    let store = fixture.open();
    let result = routes::guarded::<()>(&store, || {
        fs::write(&fixture.config_path, b"changed").unwrap();
        Err(Error::Limit.into())
    });
    assert_eq!(
        result.err(),
        Some(routes::Error::Store(Error::InputChanged))
    );
    assert_eq!(
        error(&store, f::Limits::default()),
        "local_store_input_changed"
    );
}

#[test]
fn exact_closed_record_ceiling_is_supported_and_next_byte_rejects() {
    for extra in [0usize, 1] {
        let mut b = builder();
        b.root.truncate(1);
        let mut fixture = b.write();
        let path = fixture.path.join("Data/data/data.000");
        let mut archive = fs::read(&path).unwrap();
        let offset = archive.len() as u64;
        let record_len = 128 * 1024 + extra;
        archive.extend_from_slice(&[0; 30]);
        archive.extend_from_slice(b"BLTE\0\0\0\0Z");
        archive.resize(offset as usize + record_len, 0xff);
        fs::write(&path, archive).unwrap();
        fixture.rows[0] = index_entry(EKEY, offset, record_len as u32);
        fixture.rewrite_rows();
        let store = fixture.open();
        if extra == 0 {
            let rows = report(&store);
            assert_eq!(rows[0].candidates[0].encoded_record_bytes, Some(128 * 1024));
            assert_eq!(complete(rows[0].candidates[0].inspection).zlib_frames, 1);
        } else {
            assert_eq!(
                error(&store, f::Limits::default()),
                "local_store_policy_limit"
            );
        }
    }
}
#[test]
fn missing_index_attempts_and_duplicate_root_rows_use_closed_caps() {
    let mut b = builder();
    b.routes[0].2 = vec![[0x88; 16], [0x99; 16]];
    let fixture = b.write();
    let store = fixture.open();
    assert_eq!(
        error(
            &store,
            f::Limits {
                routes: routes::Limits {
                    max_candidates_per_input: 1,
                    ..routes::Limits::default()
                },
                ..f::Limits::default()
            }
        ),
        "local_store_policy_limit"
    );
    let mut b = builder();
    b.root.push(b.root[0]);
    let fixture = b.write();
    let store = fixture.open();
    assert_eq!(
        error(
            &store,
            f::Limits {
                routes: routes::Limits {
                    max_root_rows_per_input: 1,
                    ..routes::Limits::default()
                },
                ..f::Limits::default()
            }
        ),
        "local_store_policy_limit"
    );
}
#[cfg(target_os = "linux")]
#[test]
fn symlink_archive_rejects_and_source_guard_change_never_becomes_framing_rejection() {
    let mut fixture = builder().write();
    fixture.rows[0] = index_entry(EKEY, 1 << 30, 39);
    fixture.rewrite_rows();
    std::os::unix::fs::symlink("data.000", fixture.path.join("Data/data/data.001")).unwrap();
    let store = fixture.open();
    assert_eq!(
        report(&store)[0].candidates[0].span,
        SpanState::ArchiveRejected
    );
    fs::write(&fixture.config_path, b"changed").unwrap();
    assert_eq!(
        error(&store, f::Limits::default()),
        "local_store_input_changed"
    );
}

#[test]
fn mismatching_index_prefix_rejects_without_frame_evidence() {
    let fixture = supplied(&mixed());
    let store = fixture.open();
    let entry = store
        .indexes
        .iter()
        .find(|e| e.key_prefix == EKEY[..9])
        .unwrap();
    let budget = Budget {
        limits: store.limits,
        resident: store.stats.resident_bytes,
        scratch: Cell::new(0),
    };
    let result = io::framing::inspect_span(io::framing::Request {
        directory: &store.data_dir,
        entry,
        ekey: [0x99; 16],
        expected: CONTENT.len() as u64,
        store_limits: store.limits,
        limits: f::Limits::default(),
        budget: &budget,
        scratch: store.limits.max_path_bytes * 2,
    })
    .unwrap();
    assert_eq!(
        result.2,
        f::Inspection::Rejected("archive_ekey_prefix_mismatch")
    );
}
