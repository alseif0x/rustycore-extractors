// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11. All inputs are synthetic, never client-derived.

use super::{Error, Limits, decode, md5};

const LIMITS: Limits = Limits {
    max_encoded_bytes: 4096,
    max_decoded_bytes: 4096,
    max_frames: 16,
};

fn single(frame: &[u8]) -> Vec<u8> {
    let mut encoded = b"BLTE\0\0\0\0".to_vec();
    encoded.extend_from_slice(frame);
    encoded
}

fn framed(frames: &[(&[u8], u32)]) -> Vec<u8> {
    let mut encoded = b"BLTE".to_vec();
    encoded.extend_from_slice(&(12_u32 + 24 * frames.len() as u32).to_be_bytes());
    encoded.push(0x0f);
    encoded.extend_from_slice(&(frames.len() as u32).to_be_bytes()[1..]);
    for (frame, decoded_size) in frames {
        encoded.extend_from_slice(&(frame.len() as u32).to_be_bytes());
        encoded.extend_from_slice(&decoded_size.to_be_bytes());
        encoded.extend_from_slice(&md5::digest(frame));
    }
    for (frame, _) in frames {
        encoded.extend_from_slice(frame);
    }
    encoded
}

fn read(encoded: &[u8], size: u64) -> Result<Vec<u8>, Error> {
    decode(encoded, size, None, LIMITS)
}

// Independent Python stdlib zlib goldens, produced with compressobj(level,
// DEFLATED, 15, 8, strategy). The stored, fixed and dynamic block types are
// asserted below. No Rust encoder is used to generate the tested inputs.
const Z_EMPTY: &str = "789c030000000001";
const Z_ABC: &str = "789c4b4c4a0600024d0127";
const Z_STORED: &str = "7801014000bfff000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e3faae007e1";
const Z_FIXED: &str =
    "780173cb2f4a2d4b2d5270f209715548cbac484d51282e294a4dccd553701b951a951a95a24c0a00afaa2950";
const Z_DYNAMIC: &str = "78daedccb10980301005d0de29fe040e602998cad2058e78102167423cc56cef1a16ff0df04269fa68c3bc6e0bf67e8a1d119737159be049118bd5acaea8d273911d765f0e138f09fa4af4dcc72170e1c2850b172e5cb870e1f2cbe503438ef5c6";

fn unhex(hex: &str) -> Vec<u8> {
    assert_eq!(hex.len() % 2, 0);
    hex.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

fn zframe(stream: &[u8]) -> Vec<u8> {
    let mut frame = vec![b'Z'];
    frame.extend_from_slice(stream);
    frame
}

#[test]
fn zlib_empty_and_small_streams_match_python() {
    for (hex, expected) in [(Z_EMPTY, &b""[..]), (Z_ABC, &b"abc"[..])] {
        let frame = zframe(&unhex(hex));
        for encoded in [single(&frame), framed(&[(&frame, expected.len() as u32)])] {
            assert_eq!(
                decode(
                    &encoded,
                    expected.len() as u64,
                    Some(md5::digest(expected)),
                    LIMITS
                )
                .unwrap(),
                expected
            );
        }
    }
}

#[test]
fn zlib_stored_fixed_and_dynamic_blocks_match_python() {
    for (hex, block_type, expected) in [
        (Z_STORED, 0, (0..64).collect::<Vec<u8>>()),
        (Z_FIXED, 1, b"Forever BLTE fixed stream. ".repeat(32)),
        (
            Z_DYNAMIC,
            2,
            b"Forever BLTE dynamic stream: the complete payload must match exactly.\n".repeat(40),
        ),
    ] {
        let stream = unhex(hex);
        assert_eq!((stream[2] >> 1) & 3, block_type);
        let frame = zframe(&stream);
        for encoded in [single(&frame), framed(&[(&frame, expected.len() as u32)])] {
            assert_eq!(read(&encoded, expected.len() as u64).unwrap(), expected);
        }
    }
}

#[test]
fn mixed_n_and_z_frames_reset_the_zlib_dictionary() {
    let z = zframe(&unhex(Z_ABC));
    let empty = zframe(&unhex(Z_EMPTY));
    let encoded = framed(&[
        (b"Nprefix:", 7),
        (&z, 3),
        (&empty, 0),
        (&z, 3),
        (b"N:suffix", 7),
    ]);
    assert_eq!(
        decode(
            &encoded,
            20,
            Some(md5::digest(b"prefix:abcabc:suffix")),
            LIMITS
        )
        .unwrap(),
        b"prefix:abcabc:suffix"
    );
    assert_eq!(read(&encoded, 19), Err(Error::Size));
}

#[test]
fn zlib_checksums_are_checked_after_frame_integrity() {
    let mut stream = unhex(Z_ABC);
    *stream.last_mut().unwrap() ^= 1;
    let frame = zframe(&stream);
    for encoded in [single(&frame), framed(&[(&frame, 3)])] {
        assert_eq!(read(&encoded, 3), Err(Error::Checksum));
    }
    let mut encoded = framed(&[(&zframe(&unhex(Z_ABC)), 3)]);
    *encoded.last_mut().unwrap() ^= 1;
    assert_eq!(read(&encoded, 3), Err(Error::Checksum));
}

#[test]
fn zlib_requires_the_wrapper_and_disallows_dictionary_headers() {
    let original = unhex(Z_ABC);
    let mut bad_check = original.clone();
    bad_check[1] ^= 1;
    let mut dictionary = original.clone();
    dictionary[1] = 0xbb; // Valid FCHECK with FDICT; no external dictionary is accepted.
    for stream in [
        bad_check,
        dictionary,
        original[2..original.len() - 4].to_vec(),
    ] {
        assert_eq!(read(&single(&zframe(&stream)), 3), Err(Error::Compression));
    }
}

#[test]
fn zlib_all_truncated_prefixes_fail_without_returned_content() {
    let stream = unhex(Z_ABC);
    for size in 0..stream.len() {
        assert!(
            read(&single(&zframe(&stream[..size])), 3).is_err(),
            "prefix {size}"
        );
    }
    let encoded = framed(&[(b"Nok", 2), (&zframe(&stream[..stream.len() - 1]), 3)]);
    assert!(read(&encoded, 5).is_err());
}

#[test]
fn zlib_rejects_trailing_bytes_and_concatenated_streams() {
    let stream = unhex(Z_ABC);
    for tail in [vec![0], unhex(Z_EMPTY), stream.clone()] {
        let mut extra = stream.clone();
        extra.extend_from_slice(&tail);
        let frame = zframe(&extra);
        for encoded in [single(&frame), framed(&[(&frame, 3)])] {
            assert_eq!(read(&encoded, 3), Err(Error::Size));
        }
    }
}

#[test]
fn zlib_size_mismatch_never_pads_or_returns_a_prefix() {
    let frame = zframe(&unhex(Z_ABC));
    for size in [0, 2, 4] {
        for encoded in [single(&frame), framed(&[(&frame, size)])] {
            assert_eq!(read(&encoded, u64::from(size)), Err(Error::Size));
        }
    }
    assert_eq!(read(&single(&zframe(&unhex(Z_EMPTY))), 3), Err(Error::Size));
}

#[test]
fn zlib_expansion_stops_at_the_declared_buffer_and_policy_limit() {
    // Python zlib.compress(b'A' * 4096, 9), only 28 encoded zlib bytes.
    let stream = unhex("78daedc1010d000000c2a06cef5fca1e0e28000000e0dd00ffad103d");
    let encoded = single(&zframe(&stream));
    assert_eq!(read(&encoded, 3), Err(Error::Size));
    assert_eq!(
        decode(
            &encoded,
            4096,
            None,
            Limits {
                max_decoded_bytes: 4095,
                ..LIMITS
            }
        ),
        Err(Error::Limit)
    );
    assert_eq!(read(&encoded, 4096).unwrap(), vec![b'A'; 4096]);
}

#[test]
fn headerless_n_exact_bytes_and_optional_content_key() {
    let encoded = single(b"Nabc");
    assert_eq!(read(&encoded, 3).unwrap(), b"abc");
    let expected = [
        0x90, 0x01, 0x50, 0x98, 0x3c, 0xd2, 0x4f, 0xb0, 0xd6, 0x96, 0x3f, 0x7d, 0x28, 0xe1, 0x7f,
        0x72,
    ];
    assert_eq!(decode(&encoded, 3, Some(expected), LIMITS).unwrap(), b"abc");
    assert_eq!(
        decode(&encoded, 3, Some([0; 16]), LIMITS),
        Err(Error::Checksum)
    );
}

#[test]
fn framed_n_uses_independent_md5_golden() {
    // Python stdlib hashlib.md5(b"Nabc"), not generated by the tested MD5.
    let mut encoded = b"BLTE\0\0\0\x24\x0f\0\0\x01\0\0\0\x04\0\0\0\x03".to_vec();
    encoded.extend_from_slice(&[
        0xcc, 0x62, 0xf2, 0xc3, 0x81, 0x96, 0x19, 0x69, 0x81, 0x37, 0xcc, 0x31, 0xf9, 0xd0, 0x2b,
        0x42,
    ]);
    encoded.extend_from_slice(b"Nabc");
    assert_eq!(read(&encoded, 3).unwrap(), b"abc");
}

#[test]
fn multiple_frames_preserve_order_and_empty_frames() {
    let encoded = framed(&[(b"Nfirst", 5), (b"N", 0), (b"Nsecond", 6)]);
    assert_eq!(read(&encoded, 11).unwrap(), b"firstsecond");
    assert_eq!(
        decode(&encoded, 11, Some(md5::digest(b"firstsecond")), LIMITS).unwrap(),
        b"firstsecond"
    );
}

#[test]
fn empty_content_still_requires_a_valid_frame() {
    for encoded in [single(b"N"), framed(&[(b"N", 0)])] {
        assert_eq!(read(&encoded, 0).unwrap(), b"");
    }
    assert_eq!(read(&single(b""), 0), Err(Error::Truncated));
    assert_eq!(read(&framed(&[]), 0), Err(Error::Header));
}

#[test]
fn binary_n_content_is_not_interpreted() {
    let payload: Vec<u8> = (0..=255).collect();
    let mut frame = vec![b'N'];
    frame.extend_from_slice(&payload);
    for encoded in [single(&frame), framed(&[(&frame, 256)])] {
        assert_eq!(read(&encoded, 256).unwrap(), payload);
    }
}

#[test]
fn incorrect_magic_and_segment_prelude_are_rejected() {
    let mut wrong = single(b"Nabc");
    wrong[..4].copy_from_slice(b"PLTE");
    assert_eq!(read(&wrong, 3), Err(Error::Header));
    let mut archive_span = vec![0; 30];
    archive_span.extend_from_slice(&single(b"Nabc"));
    assert_eq!(read(&archive_span, 3), Err(Error::Header));
}

#[test]
fn every_truncated_prefix_of_a_framed_object_rejects() {
    let encoded = framed(&[(b"Nfirst", 5), (b"Nsecond", 6)]);
    for size in 0..encoded.len() {
        assert!(read(&encoded[..size], 11).is_err(), "prefix {size}");
    }
}

#[test]
fn malformed_frame_header_rejects() {
    let encoded = framed(&[(b"Nabc", 3)]);
    for header_size in [1_u32, 7, 8, 11, 12, 35, 37, u32::MAX] {
        let mut wrong = encoded.clone();
        wrong[4..8].copy_from_slice(&header_size.to_be_bytes());
        assert_eq!(read(&wrong, 3), Err(Error::Header));
    }
    let mut wrong = encoded.clone();
    wrong[8] = 0;
    assert_eq!(read(&wrong, 3), Err(Error::Header));
    let mut wrong = encoded;
    wrong[9..12].fill(0);
    assert_eq!(read(&wrong, 3), Err(Error::Header));
}

#[test]
fn frame_ranges_and_trailing_bytes_are_exact() {
    let encoded = framed(&[(b"Nabc", 3)]);
    let mut missing = encoded.clone();
    missing[12..16].copy_from_slice(&u32::MAX.to_be_bytes());
    assert_eq!(read(&missing, 3), Err(Error::Truncated));
    let mut empty = encoded.clone();
    empty[12..16].fill(0);
    assert_eq!(read(&empty, 3), Err(Error::Truncated));
    let mut trailing = encoded;
    trailing.push(0);
    assert_eq!(read(&trailing, 3), Err(Error::Size));
}

#[test]
fn wrong_declared_content_sizes_never_produce_padding_or_prefixes() {
    for encoded in [single(b"Nabc"), framed(&[(b"Nabc", 3)])] {
        assert_eq!(read(&encoded, 2), Err(Error::Size));
        assert_eq!(read(&encoded, 4), Err(Error::Size));
    }
    for size in [2, 4] {
        assert_eq!(
            read(&framed(&[(b"Nabc", size)]), u64::from(size)),
            Err(Error::Size)
        );
    }
}

#[test]
fn corrupt_frame_digest_or_content_rejects() {
    let encoded = framed(&[(b"Nabc", 3)]);
    for offset in [20, 35, 36, 39] {
        let mut wrong = encoded.clone();
        wrong[offset] ^= 1;
        assert_eq!(read(&wrong, 3), Err(Error::Checksum));
    }
    let mut zero_hash = encoded;
    zero_hash[20..36].fill(0);
    assert_eq!(read(&zero_hash, 3), Err(Error::Checksum));
}

#[test]
fn corrupt_later_frame_invalidates_the_entire_object() {
    let mut encoded = framed(&[(b"Nfirst", 5), (b"Nsecond", 6)]);
    *encoded.last_mut().unwrap() ^= 1;
    assert_eq!(read(&encoded, 11), Err(Error::Checksum));
}

#[test]
fn unsupported_modes_are_distinct_and_never_fall_back_to_plain() {
    for (mode, error) in [
        (b'E', Error::EncryptedUnsupported),
        (b'F', Error::RecursiveUnsupported),
        (b'X', Error::UnknownMode),
        (0, Error::UnknownMode),
    ] {
        let frame = [mode, 1, 2, 3];
        for encoded in [single(&frame), framed(&[(&frame, 3)])] {
            assert_eq!(read(&encoded, 3), Err(error));
        }
    }
}

#[test]
fn limits_include_encoding_decoding_and_all_frames() {
    let encoded = framed(&[(b"Nabc", 3)]);
    let exact = Limits {
        max_encoded_bytes: encoded.len() as u64,
        max_decoded_bytes: 3,
        max_frames: 1,
    };
    assert_eq!(decode(&encoded, 3, None, exact).unwrap(), b"abc");
    for limits in [
        Limits {
            max_encoded_bytes: exact.max_encoded_bytes - 1,
            ..exact
        },
        Limits {
            max_decoded_bytes: 2,
            ..exact
        },
        Limits {
            max_frames: 0,
            ..exact
        },
    ] {
        assert_eq!(decode(&encoded, 3, None, limits), Err(Error::Limit));
    }
    assert_eq!(
        decode(
            &single(b"Nabc"),
            3,
            None,
            Limits {
                max_frames: 0,
                ..LIMITS
            }
        ),
        Err(Error::Limit)
    );
    assert_eq!(read(&single(b"N"), u64::MAX), Err(Error::Limit));
    assert_eq!(read(&framed(&[(b"N", u32::MAX)]), 0), Err(Error::Limit));
}

#[test]
fn maximum_frame_count_is_rejected_before_table_allocation() {
    let encoded = b"BLTE\0\0\0\x24\x0f\xff\xff\xff";
    assert_eq!(read(encoded, 0), Err(Error::Limit));
}

#[test]
fn mutations_of_checked_framed_data_do_not_escape_integrity() {
    let original = framed(&[(b"Nabc", 3)]);
    for offset in 0..original.len() {
        for bit in 0..8 {
            let mut changed = original.clone();
            changed[offset] ^= 1 << bit;
            assert!(
                decode(&changed, 3, Some(md5::digest(b"abc")), LIMITS).is_err(),
                "offset {offset}, bit {bit}"
            );
        }
    }
}

#[test]
fn arbitrary_short_inputs_fail_without_panics_or_large_allocations() {
    let mut state = 0x7f4a7c15_u32;
    for size in 0..128 {
        let mut bytes = Vec::new();
        for _ in 0..size {
            state = state.wrapping_mul(1664525).wrapping_add(1013904223);
            bytes.push((state >> 24) as u8);
        }
        assert!(read(&bytes, 8).is_err());
    }
}

#[test]
fn diagnostics_are_stable_path_free_codes() {
    for (error, code) in [
        (Error::Header, "blte_invalid_header"),
        (Error::Truncated, "blte_truncated"),
        (Error::Size, "blte_size_mismatch"),
        (Error::Checksum, "blte_checksum_mismatch"),
        (Error::Limit, "blte_policy_limit"),
        (Error::Compression, "blte_invalid_zlib"),
        (Error::EncryptedUnsupported, "blte_encrypted_unsupported"),
        (Error::RecursiveUnsupported, "blte_recursive_unsupported"),
        (Error::UnknownMode, "blte_unknown_mode"),
    ] {
        assert_eq!(error.code(), code);
        assert_eq!(error.to_string(), code);
    }
}
