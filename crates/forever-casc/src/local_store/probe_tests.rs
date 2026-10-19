// SPDX-License-Identifier: GPL-3.0-or-later
//! Synthetic ROOT/ENCODING/index probes. No client or real key data.
use super::*;
use crate::local_store::tests::{CollisionFile, CollisionFixture};
use crate::{Locale, blte::md5, local_index, root};
use std::collections::BTreeSet;

fn open_70450(fixture: &CollisionFixture) -> LocalStore {
    LocalStore::open_db2_70450(
        fixture.path(),
        super::super::OpenOptions {
            locale: Locale::EnUs,
            index_layouts: [local_index::Layout::Contiguous; 16],
            root_layout: root::Layout::Header50893V2,
            limits: super::super::Limits::default(),
        },
    )
    .unwrap()
}

fn root_rows(rows: &[(u32, u32, u32, [u8; 16])]) -> Vec<u8> {
    let mut bytes = b"TSFM".to_vec();
    for value in [20u32, 2, rows.len() as u32, 0] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    for &(id, content_flags, locale_flags, ckey) in rows {
        for value in [1u32, locale_flags, content_flags | 0x1000_0000, 0] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes.push(0);
        bytes.extend_from_slice(&id.to_le_bytes());
        bytes.extend_from_slice(&ckey);
    }
    bytes
}

fn json_keys(input: &str) -> BTreeSet<String> {
    let bytes = input.as_bytes();
    let mut fields = BTreeSet::new();
    let mut cursor = 0;
    while cursor < bytes.len() {
        if bytes[cursor] != b'"' {
            cursor += 1;
            continue;
        }
        let start = cursor + 1;
        cursor = start;
        let mut escaped = false;
        while cursor < bytes.len() {
            if escaped {
                escaped = false;
            } else if bytes[cursor] == b'\\' {
                escaped = true;
            } else if bytes[cursor] == b'"' {
                break;
            }
            cursor += 1;
        }
        let end = cursor;
        cursor = cursor.saturating_add(1);
        let mut next = cursor;
        while next < bytes.len() && bytes[next].is_ascii_whitespace() {
            next += 1;
        }
        if bytes.get(next) == Some(&b':') {
            fields.insert(input[start..end].to_owned());
        }
    }
    fields
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[test]
fn probe_reports_each_root_row_and_later_matching_encoding_without_key_bytes() {
    const FILE_ID: u32 = 42;
    let payload = b"probe synthetic local file data".to_vec();
    let ckey = md5::digest(&payload);
    let absent = [0x71; 16];
    let absent_ekey = [0x99; 16];
    let later_ekey = [0x88; 16];
    let rows = [
        (FILE_ID, 0x100, 2, ckey),
        (FILE_ID, 0x8000_0000, 2, ckey),
        (FILE_ID, 0, 0x80, ckey),
        (FILE_ID, 0, 2, absent),
        (FILE_ID, 0, 2, ckey),
    ];
    let inputs: [CollisionFile<'_>; 1] = [(FILE_ID, None, payload.clone())];
    let fixture = CollisionFixture::with_probe_root_and_later_encoding(
        &inputs,
        70450,
        root_rows(&rows),
        FILE_ID,
    );
    let report = open_70450(&fixture)
        .probe_file_data_ids(&[FILE_ID], Limits::default())
        .unwrap();
    assert_eq!(
        (report.build, report.locale, report.locale_mask),
        (70450, Locale::EnUs, 2)
    );
    assert_eq!(report.build_info_tags, "enUS");
    let file = &report.files[0];
    assert_eq!(file.root_rows_considered, 5);
    assert_eq!(file.selected_root_ordinal, Some(4));
    assert_eq!(file.root_rows.len(), 5);
    assert_eq!(file.root_rows[0].block_index, 0);
    assert_eq!(file.root_rows[0].effective_content_flags, 0x1000_0100);
    assert!(!file.root_rows[0].eligible_for_locale);
    assert_eq!(file.root_rows[1].effective_content_flags, 0x9000_0000);
    assert!(file.root_rows[1].audio_locale_bit);
    assert!(file.root_rows[1].eligible_for_locale);
    assert!(!file.root_rows[1].selected);
    assert_eq!(file.root_rows[2].locale_flags, 0x80);
    assert!(!file.root_rows[2].eligible_for_locale);
    assert!(file.root_rows[3].eligible_for_locale);
    assert!(!file.root_rows[3].ckey_central_present);
    assert!(!file.root_rows[3].selected);
    assert!(file.root_rows[4].selected);
    assert_eq!(file.encoding_row_count, 2);
    let encoding_rows: Vec<_> = file
        .route_rows
        .iter()
        .filter(|row| row.kind == RouteKind::Encoding)
        .collect();
    assert_eq!(encoding_rows.len(), 2);
    assert_eq!(encoding_rows[0].ordinal, 0);
    assert_eq!(encoding_rows[0].encoding_count, 1);
    assert_eq!(encoding_rows[0].encodings[0].span_count, 0);
    assert!(!encoding_rows[0].encodings[0].index_prefix_match);
    assert_eq!(
        encoding_rows[0].encodings[0].rejection_class,
        Some("missing")
    );
    assert_eq!(encoding_rows[1].ordinal, 1);
    assert_eq!(encoding_rows[1].encoding_count, 1);
    assert_eq!(encoding_rows[1].encodings[0].span_count, 1);
    assert!(encoding_rows[1].encodings[0].index_prefix_match);
    assert_eq!(encoding_rows[1].encodings[0].read_result, "decoded");

    let trace = report.render().unwrap();
    assert_eq!(
        json_keys(&trace),
        BTreeSet::from([
            "schema".to_owned(),
            "build".to_owned(),
            "locale".to_owned(),
            "locale_mask".to_owned(),
            "build_info_tags".to_owned(),
            "files".to_owned(),
            "file_data_id".to_owned(),
            "root_rows_considered".to_owned(),
            "selected_root_ordinal".to_owned(),
            "encoding_row_count".to_owned(),
            "root_rows".to_owned(),
            "ordinal".to_owned(),
            "block_index".to_owned(),
            "locale_flags".to_owned(),
            "effective_content_flags".to_owned(),
            "audio_locale_bit".to_owned(),
            "eligible_for_locale".to_owned(),
            "ckey_central_present".to_owned(),
            "selected".to_owned(),
            "route_rows".to_owned(),
            "kind".to_owned(),
            "content_size".to_owned(),
            "encoding_count".to_owned(),
            "encodings".to_owned(),
            "span_count".to_owned(),
            "index_prefix_match".to_owned(),
            "read_result".to_owned(),
            "rejection_class".to_owned(),
            "blte_code".to_owned(),
            "candidates".to_owned(),
            "span_ordinal".to_owned(),
        ])
    );
    for forbidden in [
        "\"ckey\":",
        "\"ekey\":",
        "\"key\":",
        "\"bytes\":",
        "\"payload\":",
        "data.000",
        &hex(&ckey),
        &hex(&absent),
        &hex(&absent_ekey),
        &hex(&later_ekey),
        &hex(&later_ekey[..9]),
    ] {
        assert!(
            !trace.contains(forbidden),
            "unexpected trace field/value: {forbidden}"
        );
    }
}

#[test]
fn probe_records_encrypted_rejection_blte_code_and_refuses_a_keyring() {
    const FILE_ID: u32 = 42;
    let payload = b"synthetic encrypted probe payload with enough bytes";
    let inputs: [CollisionFile<'_>; 1] = [(FILE_ID, None, payload.to_vec())];
    let fixture = CollisionFixture::with_available_holes_for_build(
        &inputs,
        &[(FILE_ID, 4..payload.len() - 5)],
        70450,
    );
    let report = open_70450(&fixture)
        .probe_file_data_ids(&[FILE_ID], Limits::default())
        .unwrap();
    let encoding = report.files[0]
        .route_rows
        .iter()
        .flat_map(|row| &row.encodings)
        .find(|encoding| encoding.index_prefix_match)
        .unwrap();
    assert_eq!(encoding.read_result, "rejected");
    assert_eq!(encoding.rejection_class, Some("encrypted"));
    assert_eq!(encoding.blte_code, Some("blte_encrypted_unsupported"));
    let trace = report.render().unwrap();
    assert!(!trace.contains("1111111111111111"));

    let keyring = crate::keys::Keyring::parse(
        b"forever-tact-keys\t1\n0000000000000001\t00000000000000000000000000000000\n",
    )
    .unwrap();
    let keyed = LocalStore::open_db2_70450_with_keys(
        fixture.path(),
        super::super::OpenOptions {
            locale: Locale::EnUs,
            index_layouts: [local_index::Layout::Contiguous; 16],
            root_layout: root::Layout::Header50893V2,
            limits: super::super::Limits::default(),
        },
        keyring,
    )
    .unwrap();
    assert_eq!(
        keyed
            .probe_file_data_ids(&[FILE_ID], Limits::default())
            .err(),
        Some(Error::KeysPresent)
    );
}
