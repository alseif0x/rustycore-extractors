// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11. Synthetic local index bytes only.

use super::{
    Checksums, Error, IndexFilename, Layout, Limits, jenkins, parse, parse_filename, select_latest,
};

const LIMITS: Limits = Limits {
    max_bytes: 1 << 20,
    max_entries: 1000,
    max_pages: 64,
};

fn header(bucket: u8, bits: u8) -> Vec<u8> {
    let mut bytes = vec![0; 32];
    bytes[..4].copy_from_slice(&16_u32.to_le_bytes());
    bytes[8..10].copy_from_slice(&7_u16.to_le_bytes());
    bytes[10] = bucket;
    bytes[12..15].copy_from_slice(&[4, 5, 9]);
    bytes[15] = bits;
    bytes[16..24].copy_from_slice(&0x0123456789abcdef_u64.to_le_bytes());
    refresh_header(&mut bytes);
    bytes
}

fn refresh_header(bytes: &mut [u8]) {
    let checksum = jenkins::hashlittle(&bytes[8..24], 0);
    bytes[4..8].copy_from_slice(&checksum.to_le_bytes());
}

fn entry(key: u8, storage: u64, size: u32) -> [u8; 18] {
    assert!(storage < (1 << 40));
    let mut entry = [key; 18];
    entry[9..14].copy_from_slice(&storage.to_be_bytes()[3..]);
    entry[14..18].copy_from_slice(&size.to_le_bytes());
    entry
}

fn contiguous(entries: &[[u8; 18]], checksum: Checksums) -> Vec<u8> {
    let mut bytes = header(3, 30);
    bytes.extend_from_slice(&(entries.len() as u32 * 18).to_le_bytes());
    let mut pair = (0, 0);
    let mut single = 0;
    for entry in entries {
        pair = jenkins::hashlittle2(entry, pair.0, pair.1);
        single = jenkins::hashlittle(entry, single);
    }
    bytes.extend_from_slice(
        &match checksum {
            Checksums::Blizzard => pair.0,
            Checksums::BlizzGet => single,
            Checksums::Paged => unreachable!(),
        }
        .to_le_bytes(),
    );
    for entry in entries {
        bytes.extend_from_slice(entry);
    }
    bytes
}

fn paged(page_entries: &[Vec<[u8; 18]>]) -> Vec<u8> {
    let mut bytes = header(3, 30);
    bytes.resize(4096 + page_entries.len() * 512, 0);
    for (page_index, entries) in page_entries.iter().enumerate() {
        assert!(entries.len() <= 21);
        for (slot_index, entry) in entries.iter().enumerate() {
            let start = 4096 + page_index * 512 + slot_index * 24;
            bytes[start + 4..start + 22].copy_from_slice(entry);
            let checksum = jenkins::hashlittle(&bytes[start + 4..start + 23], 0) | 0x80000000;
            bytes[start..start + 4].copy_from_slice(&checksum.to_le_bytes());
        }
    }
    bytes
}

fn read(bytes: &[u8], layout: Layout) -> Result<super::Index, Error> {
    parse(bytes, 3, layout, LIMITS)
}

#[test]
fn contiguous_blizzard_and_blizzget_hashes_are_recognized() {
    let entries = [
        entry(1, (17 << 30) | 1234, 0x01020304),
        entry(2, (7 << 30) | 99, 123),
    ];
    for checksums in [Checksums::Blizzard, Checksums::BlizzGet] {
        let decoded = read(&contiguous(&entries, checksums), Layout::Contiguous).unwrap();
        assert_eq!(decoded.checksums, checksums);
        assert_eq!(decoded.header.bucket, 3);
        assert_eq!(decoded.header.segment_size, 0x0123456789abcdef);
        assert_eq!(decoded.entries[0].key_prefix, [1; 9]);
        assert_eq!(decoded.entries[0].archive_index, 17);
        assert_eq!(decoded.entries[0].archive_offset, 1234);
        assert_eq!(decoded.entries[0].encoded_size, 0x01020304);
        assert_eq!(decoded.entries[1].archive_index, 7);
    }
}

#[test]
fn storage_offset_geometry_is_not_hardcoded_to_thirty_bits() {
    for bits in [1, 17, 30, 39, 40] {
        let mut bytes = contiguous(&[entry(9, 0xffffffffff, u32::MAX)], Checksums::Blizzard);
        bytes[15] = bits;
        refresh_header(&mut bytes);
        let decoded = read(&bytes, Layout::Contiguous).unwrap();
        assert_eq!(decoded.entries[0].storage_offset, 0xffffffffff);
        assert_eq!(decoded.entries[0].archive_offset, (1_u64 << bits) - 1);
        assert_eq!(decoded.entries[0].archive_index, 0xffffffffff >> bits);
    }
}

#[test]
fn entries_preserve_collisions_duplicates_order_and_zero_fields() {
    let first = entry(0, 0, 0);
    let collision = entry(0, 123, 456);
    let decoded = read(
        &contiguous(&[first, collision, first], Checksums::Blizzard),
        Layout::Contiguous,
    )
    .unwrap();
    assert_eq!(decoded.entries.len(), 3);
    assert_eq!(decoded.matching(&[0; 9]).count(), 3);
    assert_eq!(decoded.matching(&[1; 9]).count(), 0);
    assert_eq!(decoded.entries[0], decoded.entries[2]);
    assert_eq!(decoded.entries[1].encoded_size, 456);
}

#[test]
fn header_alignment_is_ignored_and_contiguous_zero_tail_allowed() {
    let mut bytes = contiguous(&[entry(3, 12, 98)], Checksums::Blizzard);
    bytes[24..32].fill(0xee);
    bytes.resize(bytes.len() + 512, 0);
    assert_eq!(read(&bytes, Layout::Contiguous).unwrap().entries.len(), 1);
    *bytes.last_mut().unwrap() = 1;
    assert_eq!(read(&bytes, Layout::Contiguous), Err(Error::Trailing));
}

#[test]
fn all_truncated_contiguous_prefixes_reject() {
    let bytes = contiguous(&[entry(1, 17, 123), entry(2, 19, 456)], Checksums::Blizzard);
    for end in 0..bytes.len() {
        assert!(
            read(&bytes[..end], Layout::Contiguous).is_err(),
            "prefix {end}"
        );
    }
}

#[test]
fn header_guard_requires_exact_size_and_checksum() {
    let bytes = contiguous(&[entry(1, 17, 123)], Checksums::Blizzard);
    for size in [0_u32, 15, 17, u32::MAX] {
        let mut wrong = bytes.clone();
        wrong[..4].copy_from_slice(&size.to_le_bytes());
        assert_eq!(read(&wrong, Layout::Contiguous), Err(Error::Header));
    }
    for offset in [4, 8, 10, 23] {
        let mut wrong = bytes.clone();
        wrong[offset] ^= 1;
        assert_eq!(read(&wrong, Layout::Contiguous), Err(Error::Checksum));
    }
}

#[test]
fn header_bucket_version_widths_and_geometry_reject() {
    let bytes = contiguous(&[entry(1, 17, 123)], Checksums::Blizzard);
    for (offset, value, error) in [
        (8, 5, Error::Unsupported),
        (9, 1, Error::Unsupported),
        (10, 2, Error::Header),
        (10, 16, Error::Header),
        (11, 1, Error::Header),
        (12, 3, Error::Unsupported),
        (13, 4, Error::Unsupported),
        (14, 16, Error::Unsupported),
        (15, 0, Error::Unsupported),
        (15, 41, Error::Unsupported),
        (15, 255, Error::Unsupported),
    ] {
        let mut wrong = bytes.clone();
        wrong[offset] = value;
        refresh_header(&mut wrong);
        assert_eq!(read(&wrong, Layout::Contiguous), Err(error));
    }
    assert_eq!(
        parse(&bytes, 16, Layout::Contiguous, LIMITS),
        Err(Error::Header)
    );
}

#[test]
fn contiguous_guard_requires_whole_nonempty_entries() {
    let bytes = contiguous(&[entry(1, 17, 123)], Checksums::Blizzard);
    for size in [0_u32, 1, 17, 19, u32::MAX] {
        let mut wrong = bytes.clone();
        wrong[32..36].copy_from_slice(&size.to_le_bytes());
        assert_eq!(read(&wrong, Layout::Contiguous), Err(Error::Header));
    }
    let mut oversized = bytes;
    oversized[32..36].copy_from_slice(&36_u32.to_le_bytes());
    assert_eq!(read(&oversized, Layout::Contiguous), Err(Error::Truncated));
}

#[test]
fn contiguous_payload_and_checksum_corruption_reject() {
    let bytes = contiguous(&[entry(1, 17, 123), entry(2, 31, 455)], Checksums::Blizzard);
    for offset in [36, 39, 40, 57, 58, 75] {
        let mut wrong = bytes.clone();
        wrong[offset] ^= 1;
        assert_eq!(read(&wrong, Layout::Contiguous), Err(Error::Checksum));
    }
    let mut continuous_hash = bytes.clone();
    let hash = jenkins::hashlittle(&bytes[40..], 0);
    continuous_hash[36..40].copy_from_slice(&hash.to_le_bytes());
    assert_eq!(
        read(&continuous_hash, Layout::Contiguous),
        Err(Error::Checksum)
    );
}

#[test]
fn explicit_layout_never_falls_back_to_another_variant() {
    let contiguous_bytes = contiguous(&[entry(1, 17, 123)], Checksums::Blizzard);
    assert_eq!(
        read(&contiguous_bytes, Layout::Paged),
        Err(Error::Truncated)
    );
    let paged_bytes = paged(&[vec![entry(1, 17, 123)]]);
    assert_eq!(read(&paged_bytes, Layout::Contiguous), Err(Error::Header));
}

#[test]
fn paged_entries_include_every_page_and_preserve_duplicates() {
    let first = entry(1, (3 << 30) | 91, 321);
    let second = entry(2, (5 << 30) | 71, 456);
    let decoded = read(
        &paged(&[vec![first], vec![], vec![second, first]]),
        Layout::Paged,
    )
    .unwrap();
    assert_eq!(decoded.checksums, Checksums::Paged);
    assert_eq!(decoded.entries.len(), 3);
    assert_eq!(decoded.entries[0], decoded.entries[2]);
    assert_eq!(decoded.entries[1].archive_index, 5);
    assert_eq!(decoded.entries[1].archive_offset, 71);
}

#[test]
fn full_pages_and_empty_pages_are_valid() {
    let entries: Vec<_> = (0..21)
        .map(|value| entry(value, u64::from(value), u32::from(value)))
        .collect();
    assert_eq!(
        read(&paged(&[entries]), Layout::Paged)
            .unwrap()
            .entries
            .len(),
        21
    );
    assert!(
        read(&paged(&[vec![], vec![]]), Layout::Paged)
            .unwrap()
            .entries
            .is_empty()
    );
}

#[test]
fn page_hash_covers_first_alignment_byte_but_not_second() {
    let mut bytes = paged(&[vec![entry(1, 17, 123)]]);
    bytes[4096 + 22] = 0x71;
    assert_eq!(read(&bytes, Layout::Paged), Err(Error::Checksum));
    let hash = jenkins::hashlittle(&bytes[4100..4119], 0) | 0x80000000;
    bytes[4096..4100].copy_from_slice(&hash.to_le_bytes());
    assert_eq!(read(&bytes, Layout::Paged).unwrap().entries.len(), 1);
    bytes[4096 + 23] = 0xfe;
    assert_eq!(read(&bytes, Layout::Paged).unwrap().entries.len(), 1);
}

#[test]
fn page_nonzero_corruption_never_returns_a_prefix() {
    let bytes = paged(&[vec![entry(1, 17, 123)], vec![entry(2, 31, 456)]]);
    for offset in [4096, 4100, 4117, 4608, 4612, 4629] {
        let mut wrong = bytes.clone();
        wrong[offset] ^= 1;
        assert_eq!(read(&wrong, Layout::Paged), Err(Error::Checksum));
    }
    let mut no_high_bit = bytes;
    no_high_bit[4099] &= 0x7f;
    assert_eq!(read(&no_high_bit, Layout::Paged), Err(Error::Checksum));
}

#[test]
fn page_terminator_requires_the_entire_remaining_region_to_be_zero() {
    let bytes = paged(&[vec![entry(1, 17, 123)]]);
    for offset in [4096 + 24 + 4, 4096 + 48, 4096 + 511] {
        let mut wrong = bytes.clone();
        wrong[offset] = 1;
        assert_eq!(read(&wrong, Layout::Paged), Err(Error::Trailing));
    }
    let mut missing_hash = bytes;
    missing_hash[4096..4100].fill(0);
    assert_eq!(read(&missing_hash, Layout::Paged), Err(Error::Trailing));
}

#[test]
fn page_remainder_and_leading_gap_must_be_zero() {
    let entries: Vec<_> = (0..21)
        .map(|value| entry(value, u64::from(value), 123))
        .collect();
    let mut bytes = paged(&[entries]);
    bytes[4096 + 504] = 1;
    assert_eq!(read(&bytes, Layout::Paged), Err(Error::Trailing));
    let mut bytes = paged(&[vec![]]);
    bytes[32] = 1;
    assert_eq!(read(&bytes, Layout::Paged), Err(Error::Trailing));
}

#[test]
fn page_extents_are_exact_and_no_prefix_acceptance_is_possible() {
    let bytes = paged(&[vec![entry(1, 17, 123)]]);
    for end in [0, 31, 32, 4095, 4096, 4097, 4119, 4607] {
        assert!(read(&bytes[..end], Layout::Paged).is_err());
    }
    let mut extra = bytes;
    extra.push(0);
    assert_eq!(read(&extra, Layout::Paged), Err(Error::Truncated));
}

#[test]
fn parser_limits_apply_before_entry_storage_allocation() {
    let continuous = contiguous(&[entry(1, 17, 123)], Checksums::Blizzard);
    assert!(
        parse(
            &continuous,
            3,
            Layout::Contiguous,
            Limits {
                max_bytes: continuous.len() as u64,
                max_entries: 1,
                max_pages: 0
            }
        )
        .is_ok()
    );
    for limits in [
        Limits {
            max_bytes: continuous.len() as u64 - 1,
            ..LIMITS
        },
        Limits {
            max_entries: 0,
            ..LIMITS
        },
    ] {
        assert_eq!(
            parse(&continuous, 3, Layout::Contiguous, limits),
            Err(Error::Limit)
        );
    }
    let pages = paged(&[vec![entry(1, 17, 123)], vec![entry(2, 18, 456)]]);
    for limits in [
        Limits {
            max_pages: 1,
            ..LIMITS
        },
        Limits {
            max_entries: 1,
            ..LIMITS
        },
        Limits {
            max_bytes: pages.len() as u64 - 1,
            ..LIMITS
        },
    ] {
        assert_eq!(parse(&pages, 3, Layout::Paged, limits), Err(Error::Limit));
    }
}

#[test]
fn filenames_are_exact_basenames_with_hex_numeric_generations() {
    assert_eq!(
        parse_filename("0f1234abcd.idx").unwrap(),
        IndexFilename {
            bucket: 15,
            generation: 0x1234abcd
        }
    );
    assert_eq!(
        parse_filename("0F1234ABCD.IDX").unwrap(),
        IndexFilename {
            bucket: 15,
            generation: 0x1234abcd
        }
    );
    assert_eq!(
        parse_filename("0000000000.idx").unwrap(),
        IndexFilename {
            bucket: 0,
            generation: 0
        }
    );
    for name in [
        "",
        "00ffffffff.idxx",
        "10ffffffff.idx",
        "00fffffffz.idx",
        "00ffffffff.idz",
        "data.i00",
        "../0000000000.idx",
        "\0f00000000.idx",
        "é000000000.idx",
    ] {
        assert_eq!(parse_filename(name), Err(Error::Filename), "{name:?}");
    }
}

#[test]
fn latest_generation_selection_is_numeric_per_bucket_and_order_independent() {
    let mut candidates = vec![
        IndexFilename {
            bucket: 3,
            generation: 0xff,
        },
        IndexFilename {
            bucket: 0,
            generation: 0,
        },
        IndexFilename {
            bucket: 3,
            generation: 0x100,
        },
        IndexFilename {
            bucket: 15,
            generation: u32::MAX,
        },
    ];
    let expected = select_latest(&candidates, 4).unwrap();
    candidates.reverse();
    assert_eq!(select_latest(&candidates, 4).unwrap(), expected);
    assert_eq!(expected[3].unwrap().generation, 0x100);
    assert_eq!(expected[15].unwrap().generation, u32::MAX);
    assert!(expected[1].is_none());
    assert_eq!(select_latest(&[], 0).unwrap(), [None; 16]);
}

#[test]
fn generation_aliases_invalid_buckets_and_candidate_limits_reject() {
    let aliases = [
        parse_filename("03000000ff.idx").unwrap(),
        parse_filename("03000000FF.IDX").unwrap(),
    ];
    assert_eq!(select_latest(&aliases, 2), Err(Error::Ambiguous));
    assert_eq!(select_latest(&aliases, 1), Err(Error::Limit));
    assert_eq!(
        select_latest(
            &[IndexFilename {
                bucket: 16,
                generation: 0
            }],
            1
        ),
        Err(Error::Filename)
    );
}

#[test]
fn diagnostic_codes_are_stable_and_contain_no_paths() {
    for (error, code) in [
        (Error::Truncated, "casc_index_truncated"),
        (Error::Header, "casc_index_invalid_header"),
        (Error::Unsupported, "casc_index_unsupported_layout"),
        (Error::Checksum, "casc_index_checksum_mismatch"),
        (Error::Trailing, "casc_index_nonzero_padding"),
        (Error::Limit, "casc_index_policy_limit"),
        (Error::Filename, "casc_index_invalid_filename"),
        (Error::Ambiguous, "casc_index_ambiguous_generation"),
    ] {
        assert_eq!(error.code(), code);
        assert_eq!(error.to_string(), code);
    }
}
