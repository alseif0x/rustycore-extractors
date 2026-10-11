// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11. Only synthetic local archive records and standard hashes.

use super::{Error, decode_span, plan};
use crate::{blte, local_index};

const EKEY: [u8; 16] = [0x41; 16];
const ABC_CKEY: [u8; 16] = [
    0x90, 0x01, 0x50, 0x98, 0x3c, 0xd2, 0x4f, 0xb0, 0xd6, 0x96, 0x3f, 0x7d, 0x28, 0xe1, 0x7f, 0x72,
];
const EMPTY_CKEY: [u8; 16] = [
    0xd4, 0x1d, 0x8c, 0xd9, 0x8f, 0x00, 0xb2, 0x04, 0xe9, 0x80, 0x09, 0x98, 0xec, 0xf8, 0x42, 0x7e,
];
const LIMITS: blte::Limits = blte::Limits {
    max_encoded_bytes: 1024,
    max_decoded_bytes: 1024,
    max_frames: 4,
};

fn entry(size: usize, offset: u64) -> local_index::Entry {
    local_index::Entry {
        key_prefix: [0x41; 9],
        storage_offset: offset,
        archive_index: 17,
        archive_offset: offset,
        encoded_size: size as u32,
    }
}

fn record(frame: &[u8]) -> Vec<u8> {
    let mut bytes = vec![0; 30];
    bytes.extend_from_slice(b"BLTE\0\0\0\0");
    bytes.extend_from_slice(frame);
    bytes
}

fn decode(bytes: &[u8], ckey: [u8; 16], size: u64) -> Result<Vec<u8>, Error> {
    let span = plan(&entry(bytes.len(), 47), 47 + bytes.len() as u64, 2048)?;
    decode_span(bytes, &span, EKEY, ckey, size, LIMITS)
}

#[test]
fn exact_range_retains_archive_identity_and_boundaries() {
    let entry = entry(42, 1234);
    let span = plan(&entry, 1276, 42).unwrap();
    assert_eq!(span.archive_index(), 17);
    assert_eq!(span.offset(), 1234);
    assert_eq!(span.encoded_size(), 42);
    assert_eq!(span.end(), 1276);
}

#[test]
fn range_rejects_overflow_out_of_file_and_budget_before_io() {
    assert_eq!(
        plan(&entry(42, u64::MAX - 40), u64::MAX, 42),
        Err(Error::Bounds)
    );
    assert_eq!(plan(&entry(42, 1234), 1275, 42), Err(Error::Bounds));
    assert_eq!(plan(&entry(42, 1234), 1000, 42), Err(Error::Bounds));
    assert_eq!(plan(&entry(42, 1234), 1276, 41), Err(Error::Limit));
    assert_eq!(plan(&entry(42, 0), 0, 42), Err(Error::Bounds));
}

#[test]
fn encoded_extents_require_an_actual_complete_record() {
    for size in 0..39 {
        assert_eq!(plan(&entry(size, 0), 1024, 1024), Err(Error::Extent));
    }
    assert!(plan(&entry(39, 0), 39, 39).is_ok());
    assert!(
        plan(
            &entry(u32::MAX as usize, 1),
            1 + u64::from(u32::MAX),
            u64::from(u32::MAX)
        )
        .is_ok()
    );
}

#[test]
fn n_record_decodes_only_with_exact_complete_ckey() {
    let bytes = record(b"Nabc");
    assert_eq!(decode(&bytes, ABC_CKEY, 3).unwrap(), b"abc");
    assert_eq!(
        decode(&bytes, [0; 16], 3),
        Err(Error::Blte(blte::Error::Checksum))
    );
    let mut wrong = ABC_CKEY;
    wrong[15] ^= 1;
    assert_eq!(
        decode(&bytes, wrong, 3),
        Err(Error::Blte(blte::Error::Checksum))
    );
}

#[test]
fn z_record_decodes_with_the_same_complete_content_expectation() {
    // Independent Python zlib.compress(b"abc") golden.
    let bytes = record(b"Z\x78\x9c\x4b\x4c\x4a\x06\x00\x02\x4d\x01\x27");
    assert_eq!(decode(&bytes, ABC_CKEY, 3).unwrap(), b"abc");
    assert_eq!(
        decode(&bytes, EMPTY_CKEY, 3),
        Err(Error::Blte(blte::Error::Checksum))
    );
}

#[test]
fn empty_content_requires_complete_n_or_z_encoding() {
    for frame in [&b"N"[..], &b"Z\x78\x9c\x03\x00\x00\x00\x00\x01"[..]] {
        let bytes = record(frame);
        assert_eq!(decode(&bytes, EMPTY_CKEY, 0).unwrap(), b"");
        assert_eq!(
            decode(&bytes, ABC_CKEY, 0),
            Err(Error::Blte(blte::Error::Checksum))
        );
    }
}

#[test]
fn framed_record_checks_frame_md5_and_content_md5_independently() {
    let mut bytes = vec![0; 30];
    bytes.extend_from_slice(b"BLTE\0\0\0\x24\x0f\0\0\x01\0\0\0\x04\0\0\0\x03");
    // Python hashlib.md5(b"Nabc").digest().
    bytes.extend_from_slice(&[
        0xcc, 0x62, 0xf2, 0xc3, 0x81, 0x96, 0x19, 0x69, 0x81, 0x37, 0xcc, 0x31, 0xf9, 0xd0, 0x2b,
        0x42,
    ]);
    bytes.extend_from_slice(b"Nabc");
    assert_eq!(decode(&bytes, ABC_CKEY, 3).unwrap(), b"abc");
    assert_eq!(
        decode(&bytes, EMPTY_CKEY, 3),
        Err(Error::Blte(blte::Error::Checksum))
    );
    *bytes.last_mut().unwrap() ^= 1;
    assert_eq!(
        decode(&bytes, ABC_CKEY, 3),
        Err(Error::Blte(blte::Error::Checksum))
    );
}

#[test]
fn arbitrary_prelude_is_ignored_even_if_it_contains_a_fake_blte_signature() {
    let mut bytes = record(b"Nabc");
    for (index, byte) in bytes[..30].iter_mut().enumerate() {
        *byte = (index * 29 + 7) as u8;
    }
    bytes[..4].copy_from_slice(b"BLTE");
    assert_eq!(decode(&bytes, ABC_CKEY, 3).unwrap(), b"abc");
}

#[test]
fn raw_blte_and_wrong_signature_offsets_never_auto_detect() {
    let mut raw = b"BLTE\0\0\0\0Nabc".to_vec();
    raw.resize(42, 0); // Long enough for a range plan, but not archive30.
    assert_eq!(
        decode(&raw, ABC_CKEY, 3),
        Err(Error::Blte(blte::Error::Header))
    );
    let mut shifted = vec![0; 31];
    shifted.extend_from_slice(b"BLTE\0\0\0\0Nabc");
    assert_eq!(
        decode(&shifted, ABC_CKEY, 3),
        Err(Error::Blte(blte::Error::Header))
    );
    assert_eq!(
        blte::decode(b"BLTE\0\0\0\0Nabc", 3, Some(ABC_CKEY), LIMITS).unwrap(),
        b"abc"
    );
}

#[test]
fn supplied_span_must_equal_the_planned_extent() {
    let bytes = record(b"Nabc");
    let span = plan(&entry(bytes.len(), 0), bytes.len() as u64, 2048).unwrap();
    for end in 0..bytes.len() {
        assert_eq!(
            decode_span(&bytes[..end], &span, EKEY, ABC_CKEY, 3, LIMITS),
            Err(Error::Size)
        );
    }
    let mut extra = bytes;
    extra.push(0);
    assert_eq!(
        decode_span(&extra, &span, EKEY, ABC_CKEY, 3, LIMITS),
        Err(Error::Size)
    );
}

#[test]
fn index_key_prefix_is_checked_but_full_ekey_identity_is_not_claimed() {
    let bytes = record(b"Nabc");
    let span = plan(&entry(bytes.len(), 0), bytes.len() as u64, 2048).unwrap();
    for offset in 0..9 {
        let mut wrong = EKEY;
        wrong[offset] ^= 1;
        assert_eq!(
            decode_span(&bytes, &span, wrong, ABC_CKEY, 3, LIMITS),
            Err(Error::KeyPrefix)
        );
    }
    let mut same_prefix = EKEY;
    same_prefix[9..].fill(0xff);
    assert_eq!(
        decode_span(&bytes, &span, same_prefix, ABC_CKEY, 3, LIMITS).unwrap(),
        b"abc"
    );
}

#[test]
fn complete_content_validation_rejects_a_different_candidate_with_same_prefix() {
    let first = record(b"Nabc");
    let collision = record(b"Nxyz");
    let span = plan(&entry(first.len(), 0), first.len() as u64, 2048).unwrap();
    assert_eq!(
        decode_span(&first, &span, EKEY, ABC_CKEY, 3, LIMITS).unwrap(),
        b"abc"
    );
    assert_eq!(
        decode_span(&collision, &span, EKEY, ABC_CKEY, 3, LIMITS),
        Err(Error::Blte(blte::Error::Checksum))
    );
}

#[test]
fn decoder_budgets_preserve_the_separate_archive_and_blte_extents() {
    let bytes = record(b"Nabc");
    let span = plan(
        &entry(bytes.len(), 0),
        bytes.len() as u64,
        bytes.len() as u64,
    )
    .unwrap();
    let exact = blte::Limits {
        max_encoded_bytes: 12,
        max_decoded_bytes: 3,
        max_frames: 1,
    };
    assert_eq!(
        decode_span(&bytes, &span, EKEY, ABC_CKEY, 3, exact).unwrap(),
        b"abc"
    );
    for limits in [
        blte::Limits {
            max_encoded_bytes: 11,
            ..exact
        },
        blte::Limits {
            max_decoded_bytes: 2,
            ..exact
        },
        blte::Limits {
            max_frames: 0,
            ..exact
        },
    ] {
        assert_eq!(
            decode_span(&bytes, &span, EKEY, ABC_CKEY, 3, limits),
            Err(Error::Blte(blte::Error::Limit))
        );
    }
}

#[test]
fn content_lengths_and_unsupported_modes_return_no_prefix() {
    let bytes = record(b"Nabc");
    for size in [0, 2, 4] {
        assert_eq!(
            decode(&bytes, ABC_CKEY, size),
            Err(Error::Blte(blte::Error::Size))
        );
    }
    for (mode, error) in [
        (b'E', blte::Error::EncryptedUnsupported),
        (b'F', blte::Error::RecursiveUnsupported),
        (b'X', blte::Error::UnknownMode),
    ] {
        assert_eq!(
            decode(&record(&[mode]), EMPTY_CKEY, 0),
            Err(Error::Blte(error))
        );
    }
}

#[test]
fn errors_are_stable_path_free_and_preserve_blte_causes() {
    for (error, code) in [
        (Error::Extent, "archive_span_invalid_extent"),
        (Error::Bounds, "archive_span_out_of_bounds"),
        (Error::Size, "archive_span_size_mismatch"),
        (Error::Limit, "archive_span_policy_limit"),
        (Error::KeyPrefix, "archive_ekey_prefix_mismatch"),
        (Error::Blte(blte::Error::Checksum), "blte_checksum_mismatch"),
    ] {
        assert_eq!(error.code(), code);
        assert_eq!(error.to_string(), code);
    }
    assert!(std::error::Error::source(&Error::Blte(blte::Error::Checksum)).is_some());
    assert!(std::error::Error::source(&Error::Bounds).is_none());
}
