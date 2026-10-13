// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: synthetic exact-span framing inspection checks.
use super::*;

fn limits() -> Limits {
    Limits {
        max_encoded_bytes: 4096,
        max_decoded_bytes: 4096,
        max_frames: 16,
    }
}

fn framed(frames: &[(&[u8], u32)]) -> Vec<u8> {
    let count = frames.len() as u32;
    let mut bytes = b"BLTE".to_vec();
    bytes.extend_from_slice(&(12 + count * 24).to_be_bytes());
    bytes.push(0x0f);
    bytes.extend_from_slice(&count.to_be_bytes()[1..]);
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

fn headerless(frame: &[u8]) -> Vec<u8> {
    let mut bytes = b"BLTE\0\0\0\0".to_vec();
    bytes.extend_from_slice(frame);
    bytes
}

fn encrypted(name: u8, nonce: u8, cipher: u8, ciphertext: &[u8]) -> Vec<u8> {
    let mut frame = vec![b'E', name];
    frame.extend(std::iter::repeat_n(0xa1, usize::from(name)));
    frame.push(nonce);
    frame.extend(std::iter::repeat_n(0xb2, usize::from(nonce)));
    frame.push(cipher);
    frame.extend_from_slice(ciphertext);
    frame
}

#[test]
fn mixed_nze_returns_only_complete_redacted_counts_and_declarations() {
    let e = encrypted(8, 4, b'S', &[0xff]);
    let bytes = framed(&[(b"Nabc", 3), (b"Znot-zlib", 91), (&e, 52)]);
    assert_eq!(
        inspect(&bytes, 146, limits()),
        Ok(Evidence {
            form: Form::FramedMd5,
            encoded_bytes: bytes.len() as u64,
            expected_content_size: 146,
            declared_content_size: Some(146),
            frames: 3,
            uncompressed_frames: 1,
            zlib_frames: 1,
            encrypted_frames: 1,
            verified_frame_md5_count: 3,
        })
    );
}

#[test]
fn encrypted_first_does_not_hide_later_checksum_failure() {
    let e = encrypted(0, 8, b'S', &[7]);
    let mut bytes = framed(&[(&e, 11), (b"Nabc", 3)]);
    *bytes.last_mut().unwrap() ^= 1;
    assert_eq!(inspect(&bytes, 14, limits()), Err(Error::Checksum));
    // The existing decoder keeps its separate unsupported-encryption policy.
    assert_eq!(
        super::super::decode(&bytes, 14, None, limits()),
        Err(Error::EncryptedUnsupported)
    );
}

#[test]
fn encrypted_first_does_not_hide_later_truncation() {
    let e = encrypted(8, 8, b'S', &[7]);
    let mut bytes = framed(&[(&e, 11), (b"Nabc", 3)]);
    bytes.pop();
    assert_eq!(inspect(&bytes, 14, limits()), Err(Error::Truncated));
}

#[test]
fn all_valid_outer_encryption_name_and_nonce_sizes_are_key_free() {
    for name in [0, 8] {
        for nonce in [4, 8] {
            let e = encrypted(name, nonce, b'S', &[0xff]);
            let evidence = inspect(&framed(&[(&e, 0)]), 0, limits()).unwrap();
            assert_eq!(evidence.encrypted_frames, 1);
            assert_eq!(evidence.verified_frame_md5_count, 1);
        }
    }
}

#[test]
fn encrypted_ciphertext_inner_mode_is_not_claimed_valid() {
    let e = encrypted(0, 4, b'S', &[0xff, 0, 0, 0]);
    assert!(inspect(&framed(&[(&e, 123)]), 123, limits()).is_ok());
}

#[test]
fn invalid_key_name_sizes_reject_without_key_lookup() {
    for name in [1, 7, 9, 255] {
        let e = encrypted(name, 4, b'S', &[1]);
        assert_eq!(
            inspect(&framed(&[(&e, 1)]), 1, limits()),
            Err(Error::EncryptionHeader)
        );
    }
}

#[test]
fn invalid_nonce_sizes_reject() {
    for nonce in [0, 1, 3, 5, 7, 9, 255] {
        let e = encrypted(8, nonce, b'S', &[1]);
        assert_eq!(
            inspect(&framed(&[(&e, 1)]), 1, limits()),
            Err(Error::EncryptionHeader)
        );
    }
}

#[test]
fn unsupported_cipher_rejects() {
    let e = encrypted(0, 4, b'A', &[1]);
    assert_eq!(
        inspect(&framed(&[(&e, 1)]), 1, limits()),
        Err(Error::EncryptionUnsupported)
    );
}

#[test]
fn empty_ciphertext_rejects() {
    let e = encrypted(8, 8, b'S', &[]);
    assert_eq!(
        inspect(&framed(&[(&e, 1)]), 1, limits()),
        Err(Error::EncryptionHeader)
    );
}

#[test]
fn every_truncated_encryption_header_rejects() {
    let e = encrypted(8, 8, b'S', &[1]);
    for end in 1..e.len() {
        assert_eq!(
            inspect(&framed(&[(&e[..end], 1)]), 1, limits()),
            Err(Error::EncryptionHeader)
        );
    }
}

#[test]
fn normal_frame_length_matches_its_own_declaration() {
    assert_eq!(
        inspect(&framed(&[(b"Nabc", 2)]), 2, limits()),
        Err(Error::Size)
    );
}

#[test]
fn zero_length_normal_payload_is_admitted() {
    let evidence = inspect(&framed(&[(b"N", 0)]), 0, limits()).unwrap();
    assert_eq!((evidence.frames, evidence.uncompressed_frames), (1, 1));
}

#[test]
fn zlib_interior_and_termination_are_explicitly_unvalidated() {
    for frame in [b"Z".as_slice(), b"Znot a zlib stream"] {
        let bytes = framed(&[(frame, 9)]);
        assert_eq!(inspect(&bytes, 9, limits()).unwrap().zlib_frames, 1);
        assert!(super::super::decode(&bytes, 9, None, limits()).is_err());
    }
}

#[test]
fn trailing_bytes_after_last_framed_extent_reject() {
    let mut bytes = framed(&[(b"Na", 1)]);
    bytes.push(0);
    assert_eq!(inspect(&bytes, 1, limits()), Err(Error::Size));
}

#[test]
fn declared_content_sum_must_equal_independent_expected_size() {
    let bytes = framed(&[(b"Na", 1), (b"Z", 22)]);
    assert_eq!(inspect(&bytes, 24, limits()), Err(Error::Size));
}

#[test]
fn zero_expected_md5_is_checked_instead_of_skipped() {
    let mut bytes = framed(&[(b"Na", 1)]);
    bytes[20..36].fill(0);
    assert_eq!(inspect(&bytes, 1, limits()), Err(Error::Checksum));
}

#[test]
fn zero_expected_md5_on_later_frame_still_rejects_after_encryption() {
    let e = encrypted(0, 4, b'S', &[1]);
    let mut bytes = framed(&[(&e, 3), (b"Na", 1)]);
    bytes[44..60].fill(0);
    assert_eq!(inspect(&bytes, 4, limits()), Err(Error::Checksum));
}

#[test]
fn every_truncated_framed_prefix_rejects() {
    let e = encrypted(8, 4, b'S', &[1, 2]);
    let bytes = framed(&[(&e, 10), (b"Nabc", 3), (b"Zopaque", 15)]);
    for end in 0..bytes.len() {
        assert!(
            inspect(&bytes[..end], 28, limits()).is_err(),
            "prefix {end}"
        );
    }
}

#[test]
fn invalid_signature_rejects() {
    let mut bytes = framed(&[(b"Na", 1)]);
    bytes[0] = b'X';
    assert_eq!(inspect(&bytes, 1, limits()), Err(Error::Header));
}

#[test]
fn small_nonzero_header_size_rejects() {
    for size in 1_u32..12 {
        let mut bytes = framed(&[(b"Na", 1)]);
        bytes[4..8].copy_from_slice(&size.to_be_bytes());
        assert_eq!(inspect(&bytes, 1, limits()), Err(Error::Header));
    }
}

#[test]
fn incorrect_header_marker_rejects() {
    let mut bytes = framed(&[(b"Na", 1)]);
    bytes[8] = 0;
    assert_eq!(inspect(&bytes, 1, limits()), Err(Error::Header));
}

#[test]
fn zero_frame_count_rejects() {
    assert_eq!(inspect(&framed(&[]), 0, limits()), Err(Error::Header));
}

#[test]
fn table_size_must_match_declared_header_exactly() {
    let mut bytes = framed(&[(b"Na", 1)]);
    bytes[4..8].copy_from_slice(&37_u32.to_be_bytes());
    assert_eq!(inspect(&bytes, 1, limits()), Err(Error::Header));
}

#[test]
fn empty_encoded_frame_rejects_even_with_matching_empty_md5() {
    assert_eq!(
        inspect(&framed(&[(b"", 0)]), 0, limits()),
        Err(Error::Truncated)
    );
}

#[test]
fn oversized_frame_extent_rejects() {
    let mut bytes = framed(&[(b"Na", 1)]);
    bytes[12..16].copy_from_slice(&u32::MAX.to_be_bytes());
    assert_eq!(inspect(&bytes, 1, limits()), Err(Error::Truncated));
}

#[test]
fn frame_count_cap_applies_before_table_access() {
    let mut bytes = framed(&[(b"Na", 1)]);
    bytes[9..12].copy_from_slice(&[0xff; 3]);
    assert_eq!(inspect(&bytes, 1, limits()), Err(Error::Limit));
}

#[test]
fn encoded_byte_cap_rejects_without_partial_evidence() {
    let bytes = framed(&[(b"Na", 1)]);
    let cap = Limits {
        max_encoded_bytes: bytes.len() as u64 - 1,
        ..limits()
    };
    assert_eq!(inspect(&bytes, 1, cap), Err(Error::Limit));
}

#[test]
fn expected_decoded_byte_cap_rejects() {
    let cap = Limits {
        max_decoded_bytes: 0,
        ..limits()
    };
    assert_eq!(inspect(&framed(&[(b"Na", 1)]), 1, cap), Err(Error::Limit));
}

#[test]
fn cumulative_declared_size_cap_applies_even_with_smaller_expected_size() {
    let cap = Limits {
        max_decoded_bytes: 5,
        ..limits()
    };
    assert_eq!(
        inspect(&framed(&[(b"Z", 3), (b"Z", 3)]), 1, cap),
        Err(Error::Limit)
    );
}

#[test]
fn boundary_caps_allow_exact_complete_object() {
    let bytes = framed(&[(b"Na", 1)]);
    let cap = Limits {
        max_encoded_bytes: bytes.len() as u64,
        max_decoded_bytes: 1,
        max_frames: 1,
    };
    assert!(inspect(&bytes, 1, cap).is_ok());
}

#[test]
fn recursive_and_unknown_modes_reject() {
    for (frame, error) in [
        (b"F".as_slice(), Error::RecursiveUnsupported),
        (b"?".as_slice(), Error::UnknownMode),
    ] {
        assert_eq!(inspect(&framed(&[(frame, 0)]), 0, limits()), Err(error));
    }
}

#[test]
fn headerless_is_explicitly_unhashed_and_has_no_table_declaration() {
    let bytes = headerless(b"Nabc");
    let e = inspect(&bytes, 3, limits()).unwrap();
    assert_eq!(e.form, Form::HeaderlessUnhashed);
    assert_eq!(e.declared_content_size, None);
    assert_eq!(
        (e.frames, e.uncompressed_frames, e.verified_frame_md5_count),
        (1, 1, 0)
    );
}

#[test]
fn headerless_encryption_checks_only_outer_structure_and_has_no_hash() {
    let bytes = headerless(&encrypted(0, 4, b'S', &[0xff]));
    let e = inspect(&bytes, 200, limits()).unwrap();
    assert_eq!(e.form, Form::HeaderlessUnhashed);
    assert_eq!((e.encrypted_frames, e.verified_frame_md5_count), (1, 0));
}

#[test]
fn headerless_zlib_suffix_is_opaque_without_termination_claim() {
    let bytes = headerless(b"Zopaque trailing data");
    assert_eq!(inspect(&bytes, 40, limits()).unwrap().zlib_frames, 1);
}

#[test]
fn headerless_empty_frame_rejects() {
    assert_eq!(
        inspect(&headerless(b""), 0, limits()),
        Err(Error::Truncated)
    );
}

#[test]
fn headerless_normal_trailing_bytes_fail_length_check() {
    assert_eq!(
        inspect(&headerless(b"Nabc-extra"), 3, limits()),
        Err(Error::Size)
    );
}

#[test]
fn headerless_frame_cap_zero_rejects() {
    assert_eq!(
        inspect(
            &headerless(b"Na"),
            1,
            Limits {
                max_frames: 0,
                ..limits()
            }
        ),
        Err(Error::Limit)
    );
}
