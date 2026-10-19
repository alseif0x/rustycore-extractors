// SPDX-License-Identifier: GPL-3.0-or-later
//! State classification uses only synthetic redacted probe facts.
use super::*;
use forever_casc::local_store::probe::{CandidateRead, Encoding, RouteKind, RouteRow};

fn file_trace(selected: bool, read_result: &'static str, class: Option<&'static str>) -> FileTrace {
    FileTrace {
        file_data_id: 7,
        root_rows_considered: u32::from(selected),
        root_rows: Vec::new(),
        selected_root_ordinal: selected.then_some(0),
        encoding_row_count: u32::from(selected),
        route_rows: if selected {
            vec![RouteRow {
                kind: RouteKind::Encoding,
                ordinal: 0,
                content_size: Some(1),
                encoding_count: 1,
                encodings: vec![Encoding {
                    ordinal: 0,
                    content_size: Some(1),
                    span_count: 1,
                    index_prefix_match: true,
                    read_result,
                    rejection_class: class,
                    blte_code: None,
                    candidates: vec![CandidateRead {
                        span_ordinal: 0,
                        read_result,
                        rejection_class: class,
                        blte_code: None,
                    }],
                }],
            }]
        } else {
            Vec::new()
        },
    }
}

#[test]
fn four_census_states_have_distinct_typed_reasons() {
    assert_eq!(
        classify(&file_trace(true, "decoded", None)),
        (
            State::FullyReadable,
            "complete_keyless_decode_and_ckey_check"
        )
    );
    assert_eq!(
        classify(&file_trace(true, "rejected", Some("encrypted"))),
        (
            State::EncryptedNoAvailableKey,
            "encrypted_candidate_no_key_available"
        )
    );
    assert_eq!(
        classify(&file_trace(false, "rejected", None)),
        (
            State::FileUnavailable,
            "no_central_present_eligible_root_row"
        )
    );
    assert_eq!(
        classify(&file_trace(true, "rejected", Some("unsupported"))),
        (
            State::UnsupportedOrUnknown,
            "candidate_unsupported_or_unverified"
        )
    );
}
