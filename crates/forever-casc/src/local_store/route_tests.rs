// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original synthetic route-only diagnostics.

use super::*;
use crate::local_store::routes::{self, Inspection, RequiredInput, RouteState, SpanState};

const TARGETS: [u32; 3] = [1_349_477, 1_266_278, 1_266_277];

fn builder() -> Builder {
    let mut builder = Builder::new(CONTENT);
    let key = md5::digest(CONTENT);
    builder.root = TARGETS.map(|id| (id, 0, 2, key)).to_vec();
    builder
}
fn report(store: &LocalStore) -> [routes::Diagnostic; 3] {
    store
        .diagnose_required_routes(routes::Limits::default())
        .unwrap_or_else(|e| panic!("{}", e.code()))
}
fn failed(store: &LocalStore, limits: routes::Limits) -> routes::Error {
    store
        .diagnose_required_routes(limits)
        .err()
        .expect("expected diagnostic failure")
}

#[test]
fn closed_inputs_and_uninspected_states_are_exact() {
    let fixture = builder().write();
    let store = fixture.open();
    let diagnostics = report(&store);
    for (n, d) in diagnostics.iter().enumerate() {
        assert_eq!(d.input.file_data_id(), TARGETS[n]);
        assert_eq!(
            d.input.name(),
            ["Map", "SkillLineAbility", "GameObjectDisplayInfo"][n]
        );
        assert_eq!(d.route, RouteState::Routed);
        assert_eq!(d.eligible_root_rows, 1);
        assert_eq!(d.selected_root_ordinal, Some(0));
        assert_eq!(d.expected_decoded_bytes, Some(CONTENT.len() as u32));
        assert_eq!(d.ciphertext, Inspection::NotInspected);
        assert_eq!(d.candidates.len(), 1);
        let c = &d.candidates[0];
        assert_eq!(c.encoding_ordinal, 0);
        assert_eq!(c.index_match_ordinal, Some(0));
        assert_eq!(c.encoded_record_bytes, Some(CONTENT.len() as u32 + 39));
        assert!(c.observed_archive_bytes.unwrap() >= u64::from(c.encoded_record_bytes.unwrap()));
        assert_eq!(c.span, SpanState::FitsObservedLength);
        assert_eq!(c.payload, Inspection::NotInspected);
    }
    assert_eq!(routes::PROFILE, "forever-local-required-routes-v1");
    assert_eq!(diagnostics[0].input, RequiredInput::Map);
}

#[test]
fn unrelated_fdid_does_not_activate_a_route() {
    let fixture = Builder::new(CONTENT).write();
    let store = fixture.open();
    for d in report(&store) {
        assert_eq!(d.route, RouteState::RootAbsent);
        assert_eq!(d.eligible_root_rows, 0);
        assert_eq!(d.selected_root_ordinal, None);
        assert!(d.candidates.is_empty());
    }
}

#[test]
fn eligible_root_without_central_route_is_distinct() {
    let mut b = builder();
    b.routes.clear();
    let fixture = b.write();
    let store = fixture.open();
    for d in report(&store) {
        assert_eq!(d.route, RouteState::CentralAbsent);
        assert_eq!(d.eligible_root_rows, 1);
        assert_eq!(d.selected_root_ordinal, None);
        assert!(d.candidates.is_empty());
    }
}

#[test]
fn locale_and_excluded_flags_reuse_acquisition_eligibility() {
    for (flags, locale) in [(0, 0x80), (0x80, 2), (0x100, 2)] {
        let mut b = builder();
        for row in &mut b.root {
            row.1 = flags;
            row.2 = locale;
        }
        let fixture = b.write();
        let store = fixture.open();
        assert!(
            report(&store)
                .iter()
                .all(|d| d.route == RouteState::RootIneligible)
        );
    }
}

#[test]
fn central_absent_row_can_be_skipped_but_availability_cannot() {
    let mut b = builder();
    b.root.insert(0, (TARGETS[0], 0, 2, [0x99; 16]));
    let fixture = b.write();
    let store = fixture.open();
    let d = report(&store);
    assert_eq!(d[0].eligible_root_rows, 2);
    assert_eq!(d[0].selected_root_ordinal, Some(1));
    assert_eq!(d[0].route, RouteState::Routed);
}

#[test]
fn selected_root_never_falls_back_to_available_later_content() {
    let mut b = builder();
    let absent = [0x77; 16];
    b.root.insert(0, (TARGETS[0], 0, 2, absent));
    b.routes.insert(0, (absent, 123, vec![[0x88; 16]]));
    let fixture = b.write();
    let mut store = fixture.open();
    let d = report(&store);
    assert_eq!(d[0].selected_root_ordinal, Some(0));
    assert_eq!(d[0].expected_decoded_bytes, Some(123));
    assert_eq!(d[0].candidates[0].span, SpanState::IndexAbsent);
    assert_eq!(
        store.read_file_data_id(TARGETS[0]).unwrap_err().code(),
        "local_store_no_verified_encoding"
    );
}

#[test]
fn unknown_size_still_projects_index_but_does_not_claim_readability() {
    let mut b = builder();
    b.routes[0].1 = 0xffff_ffff;
    let fixture = b.write();
    let store = fixture.open();
    let d = report(&store);
    assert_eq!(d[0].route, RouteState::UnknownDecodedSize);
    assert_eq!(d[0].expected_decoded_bytes, None);
    assert_eq!(d[0].candidates[0].span, SpanState::FitsObservedLength);
}

#[test]
fn config_only_known_content_without_ekey_is_distinct() {
    let mut b = builder();
    b.routes.clear();
    b.auxiliary = format!(
        "install = {}\ninstall-size = {}\n",
        hex(md5::digest(CONTENT)),
        CONTENT.len()
    );
    let fixture = b.write();
    let store = fixture.open();
    let d = report(&store);
    assert_eq!(d[0].route, RouteState::EncodingAbsent);
    assert!(d[0].candidates.is_empty());
}

#[test]
fn duplicate_prefix_and_ekey_alternatives_remain_separate() {
    let mut b = builder();
    b.routes[0].2.push([0x88; 16]);
    let mut fixture = b.write();
    fixture.rows.insert(1, fixture.rows[0]);
    fixture.rewrite_rows();
    let store = fixture.open();
    let d = report(&store);
    assert_eq!(d[0].candidates.len(), 3);
    assert_eq!(d[0].candidates[0].index_match_ordinal, Some(0));
    assert_eq!(d[0].candidates[1].index_match_ordinal, Some(1));
    assert_eq!(d[0].candidates[2].encoding_ordinal, 1);
    assert_eq!(d[0].candidates[2].span, SpanState::IndexAbsent);
}

#[test]
fn full_ekey_suffix_is_not_authenticated_by_index_presence() {
    let mut b = builder();
    let mut alias = EKEY;
    alias[15] ^= 1;
    b.routes[0].2 = vec![alias];
    let fixture = b.write();
    let store = fixture.open();
    assert_eq!(
        report(&store)[0].candidates[0].span,
        SpanState::FitsObservedLength
    );
    assert_eq!(report(&store)[0].ciphertext, Inspection::NotInspected);
}

#[test]
fn invalid_blte_and_encrypted_marker_are_not_inspected() {
    for mode in [b'E', b'F', 0xff] {
        let mut b = builder();
        b.records[0].1 = mode;
        b.records[0].2 = vec![0xff; CONTENT.len()];
        let fixture = b.write();
        let mut store = fixture.open();
        assert_eq!(
            report(&store)[0].candidates[0].span,
            SpanState::FitsObservedLength
        );
        assert_eq!(report(&store)[0].ciphertext, Inspection::NotInspected);
        assert!(store.read_file_data_id(TARGETS[0]).is_err());
    }
}

fn changed_entry(offset: u64, size: u32) -> [routes::Diagnostic; 3] {
    let mut fixture = builder().write();
    fixture.rows[0] = index_entry(EKEY, offset, size);
    fixture.rewrite_rows();
    let store = fixture.open();
    report(&store)
}

#[test]
fn missing_archive_is_not_missing_index() {
    let d = changed_entry(1 << 30, 39);
    assert_eq!(d[0].candidates[0].span, SpanState::ArchiveAbsent);
    assert_eq!(d[0].candidates[0].observed_archive_bytes, None);
}
#[test]
fn incomplete_span_is_distinct_from_missing_archive() {
    let d = changed_entry((1 << 30) - 1, 39);
    assert_eq!(d[0].candidates[0].span, SpanState::IncompleteSpan);
    assert!(d[0].candidates[0].observed_archive_bytes.is_some());
}
#[test]
fn invalid_extent_is_not_complete_empty_content() {
    let d = changed_entry(0, 38);
    assert_eq!(d[0].candidates[0].span, SpanState::InvalidExtent);
}
#[test]
fn directory_archive_is_rejected_without_reading() {
    let mut fixture = builder().write();
    fixture.rows[0] = index_entry(EKEY, 1 << 30, 39);
    fixture.rewrite_rows();
    fs::create_dir(fixture.path.join("Data/data/data.001")).unwrap();
    let store = fixture.open();
    assert_eq!(
        report(&store)[0].candidates[0].span,
        SpanState::ArchiveRejected
    );
}
#[cfg(target_os = "linux")]
#[test]
fn symlink_archive_is_rejected_without_following() {
    let mut fixture = builder().write();
    fixture.rows[0] = index_entry(EKEY, 1 << 30, 39);
    fixture.rewrite_rows();
    std::os::unix::fs::symlink("data.000", fixture.path.join("Data/data/data.001")).unwrap();
    let store = fixture.open();
    assert_eq!(
        report(&store)[0].candidates[0].span,
        SpanState::ArchiveRejected
    );
}

#[test]
fn downward_candidate_limit_rejects_whole_report() {
    let fixture = builder().write();
    let store = fixture.open();
    let limits = routes::Limits {
        max_candidates_total: 2,
        ..routes::Limits::default()
    };
    assert_eq!(failed(&store, limits).code(), "local_store_policy_limit");
    let limits = routes::Limits {
        max_candidates_per_input: 0,
        ..routes::Limits::default()
    };
    assert_eq!(failed(&store, limits).code(), "local_store_policy_limit");
}
#[test]
fn collision_attempts_and_missing_index_both_count() {
    let mut fixture = builder().write();
    fixture.rows.insert(1, fixture.rows[0]);
    fixture.rewrite_rows();
    let store = fixture.open();
    let limits = routes::Limits {
        max_candidates_per_input: 1,
        ..routes::Limits::default()
    };
    assert_eq!(failed(&store, limits).code(), "local_store_policy_limit");
    drop(store);
    let mut b = builder();
    b.routes[0].2 = vec![[0x88; 16], [0x99; 16]];
    let fixture = b.write();
    let store = fixture.open();
    assert_eq!(failed(&store, limits).code(), "local_store_policy_limit");
}
#[test]
fn root_duplicate_work_is_bounded() {
    let mut b = builder();
    b.root.push(b.root[0]);
    let fixture = b.write();
    let store = fixture.open();
    let limits = routes::Limits {
        max_root_rows_per_input: 1,
        ..routes::Limits::default()
    };
    assert_eq!(failed(&store, limits).code(), "local_store_policy_limit");
}
#[test]
fn caps_cannot_be_raised() {
    let fixture = builder().write();
    let store = fixture.open();
    for limits in [
        routes::Limits {
            max_root_rows_per_input: 65,
            ..routes::Limits::default()
        },
        routes::Limits {
            max_candidates_per_input: 65,
            ..routes::Limits::default()
        },
        routes::Limits {
            max_candidates_total: 193,
            ..routes::Limits::default()
        },
    ] {
        assert_eq!(failed(&store, limits).code(), "local_store_policy_limit");
    }
}
#[test]
fn store_attempt_bound_and_resident_budget_are_preserved() {
    let fixture = builder().write();
    let mut store = fixture.open();
    store.limits.max_candidate_attempts = 0;
    assert_eq!(
        failed(&store, routes::Limits::default()).code(),
        "local_store_policy_limit"
    );
    store.limits.max_candidate_attempts = 64;
    store.limits.max_resident_bytes = store.stats.resident_bytes;
    assert_eq!(
        failed(&store, routes::Limits::default()).code(),
        "local_store_policy_limit"
    );
}
#[test]
fn span_budget_never_becomes_a_positive_status() {
    let fixture = builder().write();
    let mut store = fixture.open();
    store.limits.max_archive_span_bytes = 38;
    assert_eq!(
        failed(&store, routes::Limits::default()).code(),
        "local_store_policy_limit"
    );
}
#[test]
fn preexisting_change_rejects_before_diagnostic() {
    let fixture = builder().write();
    let store = fixture.open();
    fs::write(&fixture.config_path, b"changed synthetic config").unwrap();
    assert_eq!(
        failed(&store, routes::Limits::default()).code(),
        "local_store_input_changed"
    );
}
#[test]
fn final_guards_take_precedence_even_when_operation_fails() {
    let fixture = builder().write();
    let store = fixture.open();
    let result: Result<(), routes::Error> = routes::guarded(&store, || {
        fs::write(
            &fixture.config_path,
            b"changed during failing synthetic diagnostic",
        )
        .unwrap();
        Err(routes::Error::Store(Error::Limit))
    });
    assert_eq!(result.unwrap_err().code(), "local_store_input_changed");
}
#[test]
fn key_bearing_store_is_explicitly_rejected() {
    let fixture = builder().write();
    let mut store = fixture.open();
    store.keys = Some(
        crate::keys::Keyring::parse(
            b"forever-tact-keys\t1\n0000000000000001\t00000000000000000000000000000000\n",
        )
        .unwrap(),
    );
    assert_eq!(
        failed(&store, routes::Limits::default()),
        routes::Error::KeysPresent
    );
}

#[test]
fn exact_candidate_caps_pass_and_one_more_rejects() {
    let mut fixture = builder().write();
    let row = fixture.rows[0];
    fixture.rows.splice(1..1, std::iter::repeat_n(row, 63));
    fixture.rewrite_rows();
    let store = fixture.open();
    let d = report(&store);
    assert_eq!(d.iter().map(|r| r.candidates.len()).sum::<usize>(), 192);
    assert_eq!(d[0].candidates[63].index_match_ordinal, Some(63));
    drop(store);
    fixture.rows.insert(1, row);
    fixture.rewrite_rows();
    let store = fixture.open();
    assert_eq!(
        failed(&store, routes::Limits::default()).code(),
        "local_store_policy_limit"
    );
}

#[test]
fn ineligible_root_rows_also_charge_the_work_cap() {
    let mut b = builder();
    b.root = vec![(TARGETS[0], 0, 0x80, md5::digest(CONTENT)); 65];
    let fixture = b.write();
    let store = fixture.open();
    assert_eq!(
        failed(&store, routes::Limits::default()).code(),
        "local_store_policy_limit"
    );
}

#[test]
fn empty_decoded_content_still_leaves_payload_uninspected() {
    let mut b = Builder::new(b"");
    b.root = TARGETS.map(|id| (id, 0, 2, md5::digest(b""))).to_vec();
    let fixture = b.write();
    let store = fixture.open();
    let d = report(&store);
    assert_eq!(d[0].expected_decoded_bytes, Some(0));
    assert_eq!(d[0].candidates[0].encoded_record_bytes, Some(39));
    assert_eq!(d[0].candidates[0].payload, Inspection::NotInspected);
}

#[test]
fn impossible_geometry_is_bounded_without_overflow_or_archive_guessing() {
    let fixture = builder().write();
    let mut store = fixture.open();
    let entry = store
        .indexes
        .iter_mut()
        .find(|entry| entry.key_prefix == EKEY[..9])
        .unwrap();
    entry.archive_offset = u64::MAX;
    assert_eq!(
        report(&store)[0].candidates[0].span,
        SpanState::IncompleteSpan
    );
    let entry = store
        .indexes
        .iter_mut()
        .find(|entry| entry.key_prefix == EKEY[..9])
        .unwrap();
    entry.archive_index = 0x1000;
    assert_eq!(
        report(&store)[0].candidates[0].span,
        SpanState::UnsupportedArchive
    );
}

#[test]
fn scratch_and_path_budgets_are_downward_only() {
    let fixture = builder().write();
    let mut store = fixture.open();
    let original = store.limits;
    store.limits.max_scratch_bytes = 0;
    assert_eq!(
        failed(&store, routes::Limits::default()).code(),
        "local_store_policy_limit"
    );
    store.limits = original;
    store.limits.max_path_bytes = 0;
    assert_eq!(
        failed(&store, routes::Limits::default()).code(),
        "local_store_policy_limit"
    );
}

#[test]
fn root_audio_selection_and_neutral_locale_remain_shared() {
    let mut b = builder();
    b.root[0].2 = 0;
    b.root.insert(0, (TARGETS[0], 0x8000_0000, 2, [0x77; 16]));
    b.routes.push(([0x77; 16], 1, vec![[0x88; 16]]));
    let fixture = b.write();
    let store = fixture.open();
    let d = report(&store);
    assert_eq!(d[0].root_rows, 2);
    assert_eq!(d[0].eligible_root_rows, 2);
    assert_eq!(d[0].selected_root_ordinal, Some(0));
    assert_eq!(d[0].expected_decoded_bytes, Some(CONTENT.len() as u32));
}

fn decimal_archive_fixture(index: u64, state: SpanState) -> Fixture {
    let mut fixture = builder().write();
    let size = CONTENT.len() as u32 + 39;
    let archive = fs::read(fixture.path.join("Data/data/data.000")).unwrap();
    let record = &archive[..size as usize];
    let directory = fixture.path.join("Data/data");
    let decimal = directory.join(format!("data.{index:03}"));
    let hexadecimal_decoy = directory.join(format!("data.{index:03x}"));
    assert_ne!(decimal, hexadecimal_decoy);
    match state {
        SpanState::FitsObservedLength => {
            fs::write(decimal, record).unwrap();
            fs::write(hexadecimal_decoy, b"").unwrap();
        }
        SpanState::IncompleteSpan => {
            fs::write(decimal, b"").unwrap();
            fs::write(hexadecimal_decoy, record).unwrap();
        }
        SpanState::ArchiveAbsent => fs::write(hexadecimal_decoy, record).unwrap(),
        _ => unreachable!(),
    }
    // Explicit reviewed 28-bit archive-offset geometry fits all 12 archive
    // index bits in the physical 40-bit storage offset, including 4095.
    fixture.rows[0] = index_entry(EKEY, index << 28, size);
    let mut index_bytes = index_file(0, &fixture.rows);
    index_bytes[15] = 28;
    let hash = jenkins::hashlittle(&index_bytes[8..24], 0);
    index_bytes[4..8].copy_from_slice(&hash.to_le_bytes());
    fs::write(fixture.index_path(0), index_bytes).unwrap();
    fixture
}

#[test]
fn decimal_archives_10_16_4095_match_actual_acquisition() {
    for index in [10, 16, 4095] {
        let fixture = decimal_archive_fixture(index, SpanState::FitsObservedLength);
        let mut store = fixture.open();
        assert_eq!(
            report(&store)[0].candidates[0].span,
            SpanState::FitsObservedLength
        );
        assert_eq!(store.read_file_data_id(TARGETS[0]).unwrap().bytes, CONTENT);
    }
}

#[test]
fn complete_hex_decoys_cannot_hide_incomplete_decimal_spans() {
    for index in [10, 16, 4095] {
        let fixture = decimal_archive_fixture(index, SpanState::IncompleteSpan);
        let mut store = fixture.open();
        assert_eq!(
            report(&store)[0].candidates[0].span,
            SpanState::IncompleteSpan
        );
        assert_eq!(
            store.read_file_data_id(TARGETS[0]).unwrap_err(),
            Error::NoLocalEncoding(CandidateFailures {
                incomplete: 1,
                ..CandidateFailures::default()
            })
        );
    }
}

#[test]
fn complete_hex_decoys_cannot_hide_absent_decimal_archives() {
    for index in [10, 16, 4095] {
        let fixture = decimal_archive_fixture(index, SpanState::ArchiveAbsent);
        let mut store = fixture.open();
        assert_eq!(
            report(&store)[0].candidates[0].span,
            SpanState::ArchiveAbsent
        );
        assert_eq!(
            store.read_file_data_id(TARGETS[0]).unwrap_err(),
            Error::NoLocalEncoding(CandidateFailures {
                missing: 1,
                ..CandidateFailures::default()
            })
        );
    }
}

#[test]
fn shared_decimal_paths_have_exact_separator_and_downward_capacity() {
    let fixture = builder().write();
    let directory = fixture.path.join("Data/data");
    for index in [0, 9, 10, 16, 999, 1000, 4095] {
        let expected = directory.join(format!("data.{index:03}"));
        let length = expected.as_os_str().len();
        assert_eq!(
            io::archive_path(&directory, index, length).unwrap(),
            expected
        );
        assert_eq!(
            io::archive_path(&directory, index, length - 1).unwrap_err(),
            Error::Limit
        );
        let mut terminated = directory.clone().into_os_string();
        terminated.push(std::path::MAIN_SEPARATOR_STR);
        assert_eq!(
            io::archive_path(Path::new(&terminated), index, length).unwrap(),
            expected
        );
    }
    assert_eq!(
        io::archive_path(&directory, 4096, usize::MAX).unwrap_err(),
        Error::Input
    );
    assert_eq!(
        io::archive_path(&directory, 0, 0).unwrap_err(),
        Error::Limit
    );
}
