// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original synthetic ENCODING CKey-route tests.

use super::*;

const LIMITS: Limits = Limits {
    max_bytes: 1024 * 1024,
    max_pages: 128,
    max_page_bytes: 64 * 1024,
    max_espec_bytes: 4096,
    max_tail_bytes: 4096,
    max_entries: 1000,
    max_encoded_keys: 10000,
};

fn record(ckey: [u8; 16], size: u32, ekeys: &[[u8; 16]]) -> Vec<u8> {
    let mut bytes = u16::try_from(ekeys.len()).unwrap().to_le_bytes().to_vec();
    bytes.extend_from_slice(&size.to_be_bytes());
    bytes.extend_from_slice(&ckey);
    for ekey in ekeys {
        bytes.extend_from_slice(ekey);
    }
    bytes
}

fn page(records: &[Vec<u8>], size: usize) -> Vec<u8> {
    let mut bytes = Vec::new();
    for record in records {
        bytes.extend_from_slice(record);
    }
    assert!(bytes.len() <= size);
    bytes.resize(size, 0);
    bytes
}

fn fixture(pages: &[Vec<u8>], espec: &[u8], tail: &[u8]) -> Vec<u8> {
    let page_size = pages.first().map_or(1024, Vec::len);
    assert!(page_size.is_multiple_of(1024));
    let mut bytes = b"EN\x01\x10\x10".to_vec();
    bytes.extend_from_slice(&u16::try_from(page_size / 1024).unwrap().to_be_bytes());
    bytes.extend_from_slice(&1_u16.to_be_bytes());
    bytes.extend_from_slice(&u32::try_from(pages.len()).unwrap().to_be_bytes());
    bytes.extend_from_slice(&0_u32.to_be_bytes());
    bytes.push(0);
    bytes.extend_from_slice(&u32::try_from(espec.len()).unwrap().to_be_bytes());
    bytes.extend_from_slice(espec);
    for page in pages {
        assert_eq!(page.len(), page_size);
        bytes.extend_from_slice(&page[6..22]);
        bytes.extend_from_slice(&md5::digest(page));
    }
    for page in pages {
        bytes.extend_from_slice(page);
    }
    bytes.extend_from_slice(tail);
    bytes
}

fn minimal() -> Vec<u8> {
    fixture(
        &[page(&[record([0x11; 16], 0x10203040, &[[0x22; 16]])], 1024)],
        &[],
        &[],
    )
}

fn checked(bytes: &[u8]) -> Result<EncodingRoutes<'_>, Error> {
    parse_ckey_routes(bytes, md5::digest(bytes), LIMITS)
}

fn check_limited(bytes: &[u8], limits: Limits) -> Result<EncodingRoutes<'_>, Error> {
    parse_ckey_routes(bytes, md5::digest(bytes), limits)
}

#[test]
fn independent_page_and_complete_file_md5_golden() {
    // Independent Python hashlib/struct golden for this exact synthetic fixture.
    let bytes = minimal();
    let page_hash = [
        0xa0, 0xaf, 0x9d, 0x2b, 0xa4, 0x58, 0x6e, 0x10, 0xca, 0x60, 0x41, 0xe9, 0x0c, 0x00, 0x39,
        0xae,
    ];
    let file_hash = [
        0x2f, 0x46, 0xc7, 0x08, 0xd9, 0x84, 0xbd, 0x1a, 0x05, 0x3c, 0x7c, 0xa1, 0x05, 0x06, 0xd4,
        0x96,
    ];
    assert_eq!(bytes.len(), 1078);
    assert_eq!(&bytes[38..54], page_hash);
    let routes = parse_ckey_routes(&bytes, file_hash, LIMITS).unwrap();
    assert_eq!(
        routes.entries,
        [Entry {
            content_key: [0x11; 16],
            content_size: Some(0x10203040),
            encoded_keys: &[[0x22; 16]]
        }]
    );
    assert_eq!(
        routes.header,
        Header {
            ckey_page_bytes: 1024,
            ekey_page_bytes: 1024,
            ckey_page_count: 1,
            ekey_page_count: 0,
            espec_bytes: 0
        }
    );
    assert!(routes.espec.is_empty());
    assert!(routes.tail.is_empty());
}

#[test]
fn multiple_pages_preserve_all_rows_and_all_encoded_keys() {
    let bytes = fixture(
        &[
            page(
                &[
                    record([3; 16], 30, &[[7; 16], [8; 16]]),
                    record([1; 16], 10, &[[9; 16]]),
                ],
                1024,
            ),
            page(&[record([2; 16], 20, &[[5; 16], [6; 16], [4; 16]])], 1024),
        ],
        &[],
        &[],
    );
    let routes = checked(&bytes).unwrap();
    assert_eq!(
        routes
            .entries
            .iter()
            .map(|entry| entry.content_key)
            .collect::<Vec<_>>(),
        [[3; 16], [1; 16], [2; 16]]
    );
    assert_eq!(routes.entries[0].encoded_keys, [[7; 16], [8; 16]]);
    assert_eq!(routes.entries[2].encoded_keys, [[5; 16], [6; 16], [4; 16]]);
    let mut iter = routes.lookup([2; 16]);
    assert_eq!(iter.next().unwrap().content_size, Some(20));
    assert!(iter.next().is_none());
}

#[test]
fn full_ckey_lookup_keeps_duplicates_and_conflicting_sizes() {
    let key = [0x55; 16];
    let mut collision = key;
    collision[15] ^= 1;
    let bytes = fixture(
        &[
            page(
                &[
                    record(key, 12, &[[7; 16], [7; 16]]),
                    record(collision, 13, &[[8; 16]]),
                ],
                1024,
            ),
            page(&[record(key, 14, &[[9; 16]])], 1024),
        ],
        &[],
        &[],
    );
    let routes = checked(&bytes).unwrap();
    let matches: Vec<_> = routes.lookup(key).collect();
    assert_eq!(matches.len(), 2);
    assert_eq!(matches[0].content_size, Some(12));
    assert_eq!(matches[0].encoded_keys, [[7; 16], [7; 16]]);
    assert_eq!(matches[1].content_size, Some(14));
    assert_eq!(routes.lookup(collision).count(), 1);
    assert_eq!(routes.lookup([0x99; 16]).count(), 0);
}

#[test]
fn complete_ekeys_with_colliding_nine_byte_prefixes_remain_distinct() {
    let one = [0x44; 16];
    let mut two = one;
    two[9] ^= 1;
    let bytes = fixture(&[page(&[record([1; 16], 1, &[one, two])], 1024)], &[], &[]);
    let routes = checked(&bytes).unwrap();
    assert_eq!(routes.entries[0].encoded_keys, [one, two]);
    assert_eq!(&one[..9], &two[..9]);
    assert_ne!(
        routes.entries[0].encoded_keys[0],
        routes.entries[0].encoded_keys[1]
    );
}

#[test]
fn encoded_keys_are_borrowed_and_count_is_little_endian_u16() {
    let keys = vec![[0x77; 16]; 258];
    let bytes = fixture(&[page(&[record([0x33; 16], 0, &keys)], 8192)], &[], &[]);
    assert_eq!(&bytes[54..56], [2, 1]);
    let routes = checked(&bytes).unwrap();
    assert_eq!(routes.entries[0].encoded_keys, keys);
    assert_eq!(
        routes.entries[0].encoded_keys.as_ptr().cast::<u8>(),
        bytes[76..].as_ptr()
    );
}

#[test]
fn content_sizes_preserve_zero_and_unknown_sentinel_semantics() {
    let records =
        [0, 0x80000000, u32::MAX - 1, u32::MAX].map(|size| record([1; 16], size, &[[2; 16]]));
    let bytes = fixture(&[page(&records, 1024)], &[], &[]);
    let routes = checked(&bytes).unwrap();
    assert_eq!(
        routes
            .entries
            .iter()
            .map(|entry| entry.content_size)
            .collect::<Vec<_>>(),
        [Some(0), Some(0x80000000), Some(u32::MAX - 1), None]
    );
}

#[test]
fn opaque_sections_are_borrowed_without_grammar_or_padding_claims() {
    let espec = b"unknown\xff\0unterminated";
    let tail = b"\xff\0\x91opaque nonzero tail";
    let mut bytes = fixture(
        &[page(&[record([1; 16], 2, &[[3; 16]])], 1024)],
        espec,
        tail,
    );
    bytes[13..17].copy_from_slice(&3_u32.to_be_bytes());
    let routes = checked(&bytes).unwrap();
    assert_eq!(routes.espec, espec);
    assert_eq!(routes.espec.as_ptr(), bytes[22..].as_ptr());
    assert_eq!(routes.tail, tail);
    assert_eq!(
        routes.tail.as_ptr(),
        bytes[bytes.len() - tail.len()..].as_ptr()
    );
    assert_eq!(routes.header.ekey_page_count, 3);
    // The declared EKey geometry is metadata, not an invented extent rule.
    assert_ne!(routes.tail.len(), 3 * (32 + 1024));
}

#[test]
fn every_byte_including_opaque_sections_is_bound_by_complete_ckey() {
    let original = fixture(
        &[page(&[record([1; 16], 1, &[[2; 16]])], 1024)],
        b"abc",
        b"xyz",
    );
    let digest = md5::digest(&original);
    for offset in [0, 22, 25, 57, original.len() - 1] {
        let mut altered = original.clone();
        altered[offset] ^= 1;
        assert_eq!(
            parse_ckey_routes(&altered, digest, LIMITS),
            Err(Error::Checksum)
        );
    }
    let mut appended = original.clone();
    appended.push(0);
    assert_eq!(
        parse_ckey_routes(&appended, digest, LIMITS),
        Err(Error::Checksum)
    );
    assert_eq!(
        parse_ckey_routes(&original, [0; 16], LIMITS),
        Err(Error::Checksum)
    );
}

#[test]
fn checksum_precedes_parsing_but_byte_limit_precedes_checksum() {
    assert_eq!(
        parse_ckey_routes(b"bad", [0; 16], LIMITS),
        Err(Error::Checksum)
    );
    assert_eq!(
        parse_ckey_routes(
            b"bad",
            [0; 16],
            Limits {
                max_bytes: 2,
                ..LIMITS
            }
        ),
        Err(Error::Limit)
    );
}

#[test]
fn header_magic_version_widths_and_reserved_byte_are_checked() {
    for (offset, value, error) in [
        (0, b'X', Error::Header),
        (1, b'X', Error::Header),
        (2, 2, Error::Unsupported),
        (3, 9, Error::Unsupported),
        (4, 9, Error::Unsupported),
        (17, 1, Error::Unsupported),
    ] {
        let mut bytes = minimal();
        bytes[offset] = value;
        assert_eq!(checked(&bytes), Err(error), "offset {offset}");
    }
}

#[test]
fn each_header_and_reviewed_section_truncation_is_rejected() {
    let bytes = minimal();
    for length in 0..bytes.len() {
        assert_eq!(
            checked(&bytes[..length]),
            Err(Error::Truncated),
            "length {length}"
        );
    }
    let bytes = fixture(
        &[page(&[record([1; 16], 1, &[[2; 16]])], 1024)],
        b"n\0",
        &[],
    );
    assert_eq!(checked(&bytes[..23]), Err(Error::Truncated));
}

#[test]
fn declared_page_sets_require_nonzero_page_size() {
    let mut bytes = minimal();
    bytes[5..7].fill(0);
    assert_eq!(checked(&bytes), Err(Error::Header));
    let mut bytes = minimal();
    bytes[13..17].copy_from_slice(&1_u32.to_be_bytes());
    bytes[7..9].fill(0);
    assert_eq!(checked(&bytes), Err(Error::Header));
}

#[test]
fn empty_page_sets_and_zero_filled_pages_are_explicit() {
    let mut bytes = fixture(&[], &[], &[]);
    bytes[5..9].fill(0);
    let limits = Limits {
        max_pages: 0,
        max_page_bytes: 0,
        max_entries: 0,
        max_encoded_keys: 0,
        ..LIMITS
    };
    assert!(check_limited(&bytes, limits).unwrap().entries.is_empty());
    let bytes = fixture(&[vec![0; 1024]], &[], &[]);
    assert!(checked(&bytes).unwrap().entries.is_empty());
    let mut bytes = bytes;
    bytes[22] = 1;
    assert_eq!(checked(&bytes), Err(Error::FirstKey));
}

#[test]
fn every_page_md5_is_required_including_zero_descriptors() {
    let original = fixture(
        &[
            page(&[record([1; 16], 1, &[[2; 16]])], 1024),
            page(&[record([3; 16], 2, &[[4; 16]])], 1024),
        ],
        &[],
        &[],
    );
    for offset in [38, 70, 86 + 38, 86 + 1024 + 38] {
        let mut altered = original.clone();
        altered[offset] ^= 1;
        assert_eq!(
            checked(&altered),
            Err(Error::PageChecksum),
            "offset {offset}"
        );
    }
    let mut bytes = minimal();
    bytes[38..54].fill(0);
    assert_eq!(checked(&bytes), Err(Error::PageChecksum));
}

#[test]
fn first_key_is_checked_in_each_descriptor() {
    let original = fixture(
        &[
            page(&[record([1; 16], 1, &[[2; 16]])], 1024),
            page(&[record([3; 16], 2, &[[4; 16]])], 1024),
        ],
        &[],
        &[],
    );
    for offset in [22, 54] {
        let mut bytes = original.clone();
        bytes[offset] ^= 1;
        assert_eq!(checked(&bytes), Err(Error::FirstKey));
    }
}

#[test]
fn entry_cannot_cross_its_page_even_when_next_page_contains_bytes() {
    let mut malformed = page(&[record([1; 16], 1, &[[2; 16]])], 1024);
    malformed[..2].copy_from_slice(&63_u16.to_le_bytes()); // 22 + 63 * 16 > 1024
    let bytes = fixture(&[malformed, vec![0; 1024]], &[], &[]);
    assert_eq!(checked(&bytes), Err(Error::Truncated));
}

#[test]
fn nonzero_count_in_short_page_remainder_rejects() {
    let records = vec![record([1; 16], 1, &[[2; 16]]); 26];
    let mut malformed = page(&records, 1024); // 988 used; only 36 bytes remain.
    malformed[988] = 1;
    let bytes = fixture(&[malformed], &[], &[]);
    assert_eq!(checked(&bytes), Err(Error::Truncated));
}

#[test]
fn zero_count_terminator_requires_all_remaining_bytes_zero() {
    for offset in [40, 100, 1023] {
        let mut malformed = page(&[record([1; 16], 1, &[[2; 16]])], 1024);
        malformed[offset] = 1;
        let bytes = fixture(&[malformed], &[], &[]);
        assert_eq!(checked(&bytes), Err(Error::Padding));
    }
}

#[test]
fn complete_page_needs_no_terminator() {
    let mut records = vec![record([1; 16], 1, &[[2; 16]]); 7];
    records.push(record([3; 16], 2, &[[4; 16]; 46]));
    assert_eq!(records.iter().map(Vec::len).sum::<usize>(), 1024);
    let bytes = fixture(&[page(&records, 1024)], &[], &[]);
    let routes = checked(&bytes).unwrap();
    assert_eq!(routes.entries.len(), 8);
    assert_eq!(routes.entries[7].encoded_keys.len(), 46);
}

#[test]
fn exact_budgets_accept_and_one_below_each_rejects() {
    let mut bytes = fixture(
        &[page(&[record([1; 16], 1, &[[2; 16], [3; 16]])], 1024)],
        b"abc",
        b"de",
    );
    bytes[13..17].copy_from_slice(&2_u32.to_be_bytes());
    let exact = Limits {
        max_bytes: bytes.len() as u64,
        max_pages: 3,
        max_page_bytes: 1024,
        max_espec_bytes: 3,
        max_tail_bytes: 2,
        max_entries: 1,
        max_encoded_keys: 2,
    };
    assert!(check_limited(&bytes, exact).is_ok());
    for limits in [
        Limits {
            max_bytes: exact.max_bytes - 1,
            ..exact
        },
        Limits {
            max_pages: 2,
            ..exact
        },
        Limits {
            max_page_bytes: 1023,
            ..exact
        },
        Limits {
            max_espec_bytes: 2,
            ..exact
        },
        Limits {
            max_tail_bytes: 1,
            ..exact
        },
        Limits {
            max_entries: 0,
            ..exact
        },
        Limits {
            max_encoded_keys: 1,
            ..exact
        },
    ] {
        assert_eq!(
            check_limited(&bytes, limits),
            Err(Error::Limit),
            "{limits:?}"
        );
    }
}

#[test]
fn row_and_key_limits_are_global_and_count_duplicates() {
    let same = page(&[record([1; 16], 1, &[[2; 16], [2; 16]])], 1024);
    let bytes = fixture(&[same.clone(), same], &[], &[]);
    assert_eq!(
        check_limited(
            &bytes,
            Limits {
                max_entries: 1,
                ..LIMITS
            }
        ),
        Err(Error::Limit)
    );
    assert_eq!(
        check_limited(
            &bytes,
            Limits {
                max_encoded_keys: 3,
                ..LIMITS
            }
        ),
        Err(Error::Limit)
    );
    assert!(
        check_limited(
            &bytes,
            Limits {
                max_entries: 2,
                max_encoded_keys: 4,
                ..LIMITS
            }
        )
        .is_ok()
    );
}

#[test]
fn declared_counts_and_sizes_hit_limits_before_allocations() {
    for range in [9..13, 13..17, 18..22] {
        let mut bytes = minimal();
        bytes[range].fill(0xff);
        assert_eq!(checked(&bytes), Err(Error::Limit));
    }
    for range in [5..7, 7..9] {
        let mut bytes = minimal();
        bytes[range].fill(0xff);
        assert_eq!(checked(&bytes), Err(Error::Limit));
    }
    let mut bytes = minimal();
    bytes[9..13].copy_from_slice(&2_u32.to_be_bytes());
    assert_eq!(checked(&bytes), Err(Error::Truncated));
}

#[test]
fn invalid_later_page_returns_no_successful_prefix() {
    let mut second = page(&[record([3; 16], 2, &[[4; 16]])], 1024);
    second[42] = 1;
    let bytes = fixture(
        &[page(&[record([1; 16], 1, &[[2; 16]])], 1024), second],
        &[],
        &[],
    );
    assert_eq!(checked(&bytes), Err(Error::Padding));
}

#[test]
fn diagnostics_do_not_include_content_identifiers_or_bytes() {
    for (error, code) in [
        (Error::Limit, "encoding_policy_limit"),
        (Error::Checksum, "encoding_content_checksum_mismatch"),
        (Error::Truncated, "encoding_truncated"),
        (Error::Header, "encoding_invalid_header"),
        (Error::Unsupported, "encoding_unsupported_profile"),
        (Error::PageChecksum, "encoding_page_checksum_mismatch"),
        (Error::FirstKey, "encoding_first_key_mismatch"),
        (Error::Padding, "encoding_nonzero_page_padding"),
    ] {
        assert_eq!(error.code(), code);
        assert_eq!(error.to_string(), code);
    }
}
