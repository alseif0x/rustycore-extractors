// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: encrypted synthetic span boundary/identity checks.
use super::*;
use crate::keys::{self, Keyring};
const KEY: [u8; 16] = [0; 16];
fn ring() -> Keyring {
    Keyring::parse(b"forever-tact-keys\t1\n0000000000000000\t00000000000000000000000000000000\n")
        .unwrap()
}
fn record() -> Vec<u8> {
    let mut record = vec![0xa5; 30];
    record.extend_from_slice(b"BLTE\0\0\0\0E\0\x04\0\0\0\0S");
    let mut inner = b"Nabc".to_vec();
    keys::xor(&KEY, [0; 8], &mut inner);
    record.extend_from_slice(&inner);
    record
}
fn span(record: &[u8]) -> SpanPlan {
    plan(
        &Entry {
            key_prefix: [0x42; 9],
            storage_offset: 0,
            archive_index: 0,
            archive_offset: 0,
            encoded_size: record.len() as u32,
        },
        record.len() as u64,
        1024,
    )
    .unwrap()
}
fn limits() -> blte::Limits {
    blte::Limits {
        max_encoded_bytes: 1024,
        max_decoded_bytes: 1024,
        max_frames: 4,
    }
}
#[test]
fn encrypted_span_is_opt_in_and_requires_full_ckey() {
    let record = record();
    let span = span(&record);
    let ckey = blte::md5::digest(b"abc");
    assert_eq!(
        decode_span(&record, &span, [0x42; 16], ckey, 3, limits()),
        Err(Error::Blte(blte::Error::EncryptedUnsupported))
    );
    assert_eq!(
        decode_span_with_keys(&record, &span, [0x42; 16], ckey, 3, limits(), &ring()).unwrap(),
        b"abc"
    );
    assert_eq!(
        decode_span_with_keys(&record, &span, [0x42; 16], [0; 16], 3, limits(), &ring()),
        Err(Error::Blte(blte::Error::Checksum))
    );
}
#[test]
fn encrypted_span_still_checks_exact_extent_and_selected_ekey_prefix_first() {
    let record = record();
    let span = span(&record);
    let ckey = blte::md5::digest(b"abc");
    assert_eq!(
        decode_span_with_keys(
            &record[..record.len() - 1],
            &span,
            [0x42; 16],
            ckey,
            3,
            limits(),
            &ring()
        ),
        Err(Error::Size)
    );
    assert_eq!(
        decode_span_with_keys(&record, &span, [0x43; 16], ckey, 3, limits(), &ring()),
        Err(Error::KeyPrefix)
    );
}
