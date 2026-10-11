// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original synthetic WoW ROOT parser/selection tests.

use super::*;

const LAYOUTS: [Layout; 4] = [
    Layout::Headerless18125,
    Layout::Header30080,
    Layout::Header50893V1,
    Layout::Header50893V2,
];
const LIMITS: Limits = Limits {
    max_bytes: 1024 * 1024,
    max_header_bytes: 1024,
    max_blocks: 100,
    max_entries: 1000,
};
type Row = (u32, [u8; 16], u64);

fn row(delta: u32, key: u8) -> Row {
    (delta, [key; 16], 0x0102030405060708)
}

fn group(layout: Layout, flags: u32, locale: u32, rows: &[Row]) -> Vec<u8> {
    let mut bytes = u32::try_from(rows.len()).unwrap().to_le_bytes().to_vec();
    if layout == Layout::Header50893V2 {
        bytes.extend_from_slice(&locale.to_le_bytes());
        bytes.extend_from_slice(&flags.to_le_bytes());
        bytes.extend_from_slice(&0_u32.to_le_bytes());
        bytes.push(0);
    } else {
        bytes.extend_from_slice(&flags.to_le_bytes());
        bytes.extend_from_slice(&locale.to_le_bytes());
    }
    for row in rows {
        bytes.extend_from_slice(&row.0.to_le_bytes());
    }
    if layout == Layout::Headerless18125 {
        for row in rows {
            bytes.extend_from_slice(&row.1);
            bytes.extend_from_slice(&row.2.to_le_bytes());
        }
    } else {
        for row in rows {
            bytes.extend_from_slice(&row.1);
        }
        if flags & NO_NAME_HASH == 0 {
            for row in rows {
                bytes.extend_from_slice(&row.2.to_le_bytes());
            }
        }
    }
    bytes
}

fn fixture(layout: Layout, groups: &[Vec<u8>], extension: &[u8]) -> Vec<u8> {
    let mut total = 0_u32;
    let mut named = 0_u32;
    for group in groups {
        let count = u32::from_le_bytes(group[..4].try_into().unwrap());
        total += count;
        let flags = if layout == Layout::Header50893V2 {
            u32::from_le_bytes(group[8..12].try_into().unwrap())
                | u32::from_le_bytes(group[12..16].try_into().unwrap())
                | (u32::from(group[16]) << 17)
        } else {
            u32::from_le_bytes(group[4..8].try_into().unwrap())
        };
        if layout == Layout::Headerless18125 || flags & NO_NAME_HASH == 0 {
            named += count;
        }
    }
    let mut bytes = Vec::new();
    if layout != Layout::Headerless18125 {
        bytes.extend_from_slice(b"TSFM");
        if layout != Layout::Header30080 {
            bytes.extend_from_slice(&u32::try_from(20 + extension.len()).unwrap().to_le_bytes());
            bytes.extend_from_slice(
                &(if layout == Layout::Header50893V1 {
                    1_u32
                } else {
                    2
                })
                .to_le_bytes(),
            );
        }
        bytes.extend_from_slice(&total.to_le_bytes());
        bytes.extend_from_slice(&named.to_le_bytes());
        bytes.extend_from_slice(extension);
    }
    for group in groups {
        bytes.extend_from_slice(group);
    }
    bytes
}

fn minimal(layout: Layout) -> Vec<u8> {
    fixture(layout, &[group(layout, 0, 2, &[row(7, 0x11)])], &[])
}

fn checked(bytes: &[u8], layout: Layout) -> Result<Root<'_>, Error> {
    parse_root(bytes, md5::digest(bytes), layout, LIMITS)
}

fn check_limited(bytes: &[u8], layout: Layout, limits: Limits) -> Result<Root<'_>, Error> {
    parse_root(bytes, md5::digest(bytes), layout, limits)
}

#[test]
fn independent_md5_goldens_cover_all_four_layouts() {
    // Python hashlib/struct goldens over independently assembled synthetic files.
    let goldens = [
        (
            Layout::Headerless18125,
            40,
            [
                0x13, 0xb2, 0xe8, 0x4f, 0x49, 0x60, 0x20, 0x2b, 0x2a, 0xcb, 0x04, 0xd0, 0x1a, 0x87,
                0xd3, 0x4c,
            ],
        ),
        (
            Layout::Header30080,
            52,
            [
                0x05, 0xd7, 0xf8, 0x0b, 0xdb, 0xff, 0x87, 0x27, 0xda, 0x42, 0xa1, 0xb5, 0x11, 0x92,
                0x27, 0xf9,
            ],
        ),
        (
            Layout::Header50893V1,
            60,
            [
                0xb2, 0x01, 0xff, 0x2e, 0x32, 0x6e, 0x34, 0x85, 0x13, 0xbd, 0x0d, 0x43, 0x05, 0xc2,
                0x04, 0x0a,
            ],
        ),
        (
            Layout::Header50893V2,
            65,
            [
                0x94, 0xea, 0xde, 0x20, 0x12, 0x8a, 0x27, 0xc6, 0xd7, 0xfe, 0x99, 0x4c, 0x89, 0x0b,
                0xf2, 0x07,
            ],
        ),
    ];
    for (layout, size, digest) in goldens {
        let bytes = minimal(layout);
        assert_eq!(bytes.len(), size);
        let root = parse_root(&bytes, digest, layout, LIMITS).unwrap();
        assert_eq!(
            root.entries(),
            [Entry {
                file_data_id: 7,
                content_key: [0x11; 16],
                file_name_hash: Some(0x0102030405060708),
                block_index: 0
            }]
        );
        assert_eq!(root.blocks()[0].entries, 0..1);
        assert_eq!(root.blocks()[0].locale_flags, 2);
        assert_eq!(root.header.layout, layout);
    }
}

#[test]
fn deltas_increment_after_each_row_and_reset_for_each_block() {
    for layout in LAYOUTS {
        let bytes = fixture(
            layout,
            &[
                group(layout, 0, 2, &[row(7, 1), row(0, 2), row(10, 3)]),
                group(layout, 0, 2, &[row(7, 4), row(5, 5)]),
            ],
            &[],
        );
        let root = checked(&bytes, layout).unwrap();
        assert_eq!(
            root.entries()
                .iter()
                .map(|entry| entry.file_data_id)
                .collect::<Vec<_>>(),
            [7, 8, 19, 7, 13]
        );
        assert_eq!(root.blocks()[1].entries, 3..5);
        assert_eq!(root.entries()[3].block_index, 1);
    }
}

#[test]
fn split_and_interleaved_name_hash_arrays_are_preserved() {
    for layout in LAYOUTS {
        let rows = [(1, [1; 16], 0), (1, [2; 16], u64::MAX)];
        let bytes = fixture(
            layout,
            &[
                group(layout, 0, 2, &rows),
                group(layout, NO_NAME_HASH, 2, &rows),
            ],
            &[],
        );
        let root = checked(&bytes, layout).unwrap();
        assert_eq!(root.entries()[0].file_name_hash, Some(0));
        assert_eq!(root.entries()[1].file_name_hash, Some(u64::MAX));
        let expected = if layout == Layout::Headerless18125 {
            Some(0)
        } else {
            None
        };
        assert_eq!(root.entries()[2].file_name_hash, expected);
        assert_eq!(root.entries()[3].content_key, [2; 16]);
    }
}

#[test]
fn v2_packed_flags_preserve_original_parts_and_effective_or() {
    let layout = Layout::Header50893V2;
    let mut block = group(layout, 0x08, 0x80, &[row(42, 1)]);
    block[12..16].copy_from_slice(&0x80000040_u32.to_le_bytes());
    block[16] = 0xa5;
    let bytes = fixture(layout, &[block], &[]);
    let root = checked(&bytes, layout).unwrap();
    assert_eq!(
        root.blocks()[0].content_flags,
        ContentFlags::Modern {
            flags1: 8,
            flags2: 0x80000040,
            flags3: 0xa5
        }
    );
    assert_eq!(root.blocks()[0].content_flags.effective(), 0x814a0048);
    assert_eq!(root.entries()[0].file_data_id, 42);
    assert_eq!(root.candidates(42, Locale::EsEs).count(), 1);
}

#[test]
fn modern_header_extensions_are_borrowed_and_authenticated() {
    for layout in [Layout::Header50893V1, Layout::Header50893V2] {
        let bytes = fixture(layout, &[group(layout, 0, 2, &[row(7, 1)])], b"\xff\0extra");
        let root = checked(&bytes, layout).unwrap();
        assert_eq!(root.header.extension, b"\xff\0extra");
        assert_eq!(root.header.extension.as_ptr(), bytes[20..].as_ptr());
        let mut altered = bytes.clone();
        altered[20] ^= 1;
        assert_eq!(
            parse_root(&altered, md5::digest(&bytes), layout, LIMITS),
            Err(Error::Checksum)
        );
    }
}

#[test]
fn declared_totals_are_metadata_not_unproved_extent_constraints() {
    for layout in [
        Layout::Header30080,
        Layout::Header50893V1,
        Layout::Header50893V2,
    ] {
        let mut bytes = minimal(layout);
        let offset = if layout == Layout::Header30080 { 4 } else { 12 };
        bytes[offset..offset + 4].copy_from_slice(&u32::MAX.to_le_bytes());
        bytes[offset + 4..offset + 8].copy_from_slice(&2_u32.to_le_bytes());
        let root = checked(&bytes, layout).unwrap();
        assert_eq!(root.header.total_files, Some(u32::MAX));
        assert_eq!(root.header.files_with_name_hash, Some(2));
        assert_eq!(root.entries().len(), 1);
    }
}

#[test]
fn enus_and_eses_select_neutral_and_intersecting_masks_only() {
    let layout = Layout::Header50893V2;
    let bytes = fixture(
        layout,
        &[
            group(layout, 0, 0, &[row(7, 1)]),
            group(layout, 0, 2, &[row(7, 2)]),
            group(layout, 0, 0x80, &[row(7, 3)]),
            group(layout, 0, 0x82, &[row(7, 4)]),
            group(layout, 0, 0x10, &[row(7, 5)]),
        ],
        &[],
    );
    let root = checked(&bytes, layout).unwrap();
    assert_eq!(
        root.candidates(7, Locale::EnUs)
            .map(|entry| entry.content_key[0])
            .collect::<Vec<_>>(),
        [1, 2, 4]
    );
    assert_eq!(
        root.candidates(7, Locale::EsEs)
            .map(|entry| entry.content_key[0])
            .collect::<Vec<_>>(),
        [1, 3, 4]
    );
}

#[test]
fn eses_has_no_english_fallback() {
    for layout in LAYOUTS {
        let bytes = fixture(layout, &[group(layout, 0, 2, &[row(7, 1)])], &[]);
        let root = checked(&bytes, layout).unwrap();
        assert!(root.resolve(7, Locale::EnUs, |_| true).is_some());
        assert!(root.resolve(7, Locale::EsEs, |_| true).is_none());
    }
}

#[test]
fn physical_order_decides_neutral_versus_specific_locale() {
    let layout = Layout::Header30080;
    for locales in [[0, 0x80], [0x80, 0]] {
        let bytes = fixture(
            layout,
            &[
                group(layout, 0, locales[0], &[row(7, 1)]),
                group(layout, 0, locales[1], &[row(7, 2)]),
            ],
            &[],
        );
        let root = checked(&bytes, layout).unwrap();
        assert_eq!(
            root.resolve(7, Locale::EsEs, |_| true).unwrap().content_key,
            [1; 16]
        );
        assert_eq!(root.candidates(7, Locale::EsEs).count(), 2);
    }
}

#[test]
fn audio_zero_pass_precedes_audio_one_then_keeps_physical_order() {
    for layout in LAYOUTS {
        let bytes = fixture(
            layout,
            &[
                group(layout, 0x80000000, 2, &[row(7, 1)]),
                group(layout, 0, 2, &[row(7, 2)]),
                group(layout, 0x80000000, 2, &[row(7, 3)]),
                group(layout, 0, 2, &[row(7, 4)]),
            ],
            &[],
        );
        let root = checked(&bytes, layout).unwrap();
        assert_eq!(
            root.candidates(7, Locale::EnUs)
                .map(|entry| entry.content_key[0])
                .collect::<Vec<_>>(),
            [2, 4, 1, 3]
        );
        assert_eq!(
            root.resolve(7, Locale::EnUs, |_| true).unwrap().content_key,
            [2; 16]
        );
    }
}

#[test]
fn dont_load_and_low_violence_blocks_are_excluded_but_preserved() {
    let layout = Layout::Header50893V2;
    let bytes = fixture(
        layout,
        &[
            group(layout, 0x100, 2, &[row(7, 1)]),
            group(layout, 0x80, 2, &[row(7, 2)]),
            group(layout, 0x180, 2, &[row(7, 3)]),
            group(layout, 0, 2, &[row(7, 4)]),
        ],
        &[],
    );
    let root = checked(&bytes, layout).unwrap();
    assert_eq!(root.entries().len(), 4);
    let mut evaluated = Vec::new();
    let selected = root
        .resolve(7, Locale::EnUs, |key| {
            evaluated.push(key[0]);
            true
        })
        .unwrap();
    assert_eq!(selected.content_key, [4; 16]);
    assert_eq!(evaluated, [4]);
}

#[test]
fn other_content_flags_do_not_invent_architecture_or_platform_priority() {
    let layout = Layout::Header30080;
    let flags = [
        0x10 | 0x20,
        0x8 | 0x40,
        0x8000,
        0x08000000,
        0x20000000,
        0x40000000,
    ];
    let groups: Vec<_> = flags
        .iter()
        .enumerate()
        .map(|(i, &flags)| group(layout, flags, 2, &[row(7, u8::try_from(i + 1).unwrap())]))
        .collect();
    let bytes = fixture(layout, &groups, &[]);
    let root = checked(&bytes, layout).unwrap();
    assert_eq!(root.candidates(7, Locale::EnUs).count(), flags.len());
    assert_eq!(
        root.resolve(7, Locale::EnUs, |_| true).unwrap().content_key,
        [1; 16]
    );
}

#[test]
fn resolve_uses_first_central_ckey_and_stops_after_presence() {
    let layout = Layout::Header50893V1;
    let bytes = fixture(
        layout,
        &[
            group(layout, 0, 2, &[row(7, 1)]),
            group(layout, 0, 2, &[row(7, 2)]),
            group(layout, 0, 2, &[row(7, 3)]),
        ],
        &[],
    );
    let root = checked(&bytes, layout).unwrap();
    let mut evaluated = Vec::new();
    let selected = root
        .resolve(7, Locale::EnUs, |key| {
            evaluated.push(key[0]);
            key[0] >= 2
        })
        .unwrap();
    assert_eq!(selected.content_key, [2; 16]);
    assert_eq!(evaluated, [1, 2]);
    // Availability of archive bytes is deliberately absent from this API.
    assert!(root.resolve(7, Locale::EnUs, |_| false).is_none());
    assert!(
        root.resolve(99, Locale::EnUs, |_| panic!("nonmatching ID evaluated"))
            .is_none()
    );
}

#[test]
fn identical_duplicates_and_name_hashes_do_not_change_ckey_priority() {
    let layout = Layout::Header50893V2;
    let bytes = fixture(
        layout,
        &[
            group(layout, 0, 2, &[(7, [1; 16], 0)]),
            group(layout, 0, 2, &[(7, [2; 16], 99)]),
            group(layout, NO_NAME_HASH, 2, &[(7, [1; 16], 0)]),
        ],
        &[],
    );
    let root = checked(&bytes, layout).unwrap();
    assert_eq!(root.candidates(7, Locale::EnUs).count(), 3);
    assert_eq!(
        root.resolve(7, Locale::EnUs, |_| true).unwrap().content_key,
        [1; 16]
    );
    assert_eq!(root.entries()[1].file_name_hash, Some(99));
    assert_eq!(root.entries()[2].file_name_hash, None);
}

#[test]
fn zero_and_largest_valid_file_id_need_no_max_id_sized_allocation() {
    for layout in LAYOUTS {
        let bytes = fixture(
            layout,
            &[group(layout, 0, 2, &[row(0, 1), row(u32::MAX - 2, 2)])],
            &[],
        );
        let root = check_limited(
            &bytes,
            layout,
            Limits {
                max_entries: 2,
                ..LIMITS
            },
        )
        .unwrap();
        assert_eq!(root.entries()[0].file_data_id, 0);
        assert_eq!(root.entries()[1].file_data_id, u32::MAX - 1);
        assert!(root.resolve(u32::MAX - 1, Locale::EnUs, |_| true).is_some());
    }
}

#[test]
fn invalid_sentinel_and_delta_overflow_fail_even_for_filtered_blocks() {
    for layout in LAYOUTS {
        for rows in [
            vec![row(u32::MAX, 1)],
            vec![row(u32::MAX - 1, 1), row(0, 2)],
            vec![row(u32::MAX - 1, 1), row(1, 2)],
        ] {
            for flags in [0, 0x100, 0x80] {
                let bytes = fixture(layout, &[group(layout, flags, 0x10, &rows)], &[]);
                assert_eq!(checked(&bytes, layout), Err(Error::FileDataId));
            }
        }
    }
}

#[test]
fn complete_empty_headers_and_groups_have_explicit_boundaries() {
    for layout in LAYOUTS {
        let empty = fixture(layout, &[], &[]);
        let root = checked(&empty, layout).unwrap();
        assert!(root.entries().is_empty());
        assert!(root.blocks().is_empty());
        let bytes = fixture(
            layout,
            &[group(layout, 0, 2, &[]), group(layout, 0x100, 0x80, &[])],
            &[],
        );
        let root = checked(&bytes, layout).unwrap();
        assert!(root.entries().is_empty());
        assert_eq!(root.blocks().len(), 2);
        assert_eq!(root.blocks()[1].entries, 0..0);
    }
}

#[test]
fn full_digest_precedes_parse_and_has_no_zero_bypass() {
    let layout = Layout::Header50893V2;
    let bytes = minimal(layout);
    assert_eq!(
        parse_root(&bytes, [0; 16], layout, LIMITS),
        Err(Error::Checksum)
    );
    assert_eq!(
        parse_root(b"bad", [0; 16], layout, LIMITS),
        Err(Error::Checksum)
    );
    for offset in [0, 4, 8, 20, 37, 50, 64] {
        let mut altered = bytes.clone();
        altered[offset] ^= 1;
        assert_eq!(
            parse_root(&altered, md5::digest(&bytes), layout, LIMITS),
            Err(Error::Checksum)
        );
    }
    assert_eq!(
        parse_root(
            &bytes,
            [0; 16],
            layout,
            Limits {
                max_bytes: 0,
                ..LIMITS
            }
        ),
        Err(Error::Limit)
    );
}

#[test]
fn truncation_at_every_required_byte_is_rejected() {
    for layout in LAYOUTS {
        let bytes = minimal(layout);
        let header_size = match layout {
            Layout::Headerless18125 => 0,
            Layout::Header30080 => 12,
            _ => 20,
        };
        for length in 0..bytes.len() {
            if length == header_size {
                continue;
            } // Complete explicit empty root.
            assert_eq!(
                checked(&bytes[..length], layout),
                Err(Error::Truncated),
                "{layout:?} at {length}"
            );
        }
    }
}

#[test]
fn partial_trailing_groups_reject_without_returning_prefix() {
    for layout in LAYOUTS {
        let header_size = if layout == Layout::Header50893V2 {
            17
        } else {
            12
        };
        for tail_len in 1..header_size {
            let mut bytes = minimal(layout);
            bytes.resize(bytes.len() + tail_len, 0);
            assert_eq!(checked(&bytes, layout), Err(Error::Truncated));
        }
    }
}

#[test]
fn malformed_arrays_are_checked_even_in_discarded_locale_or_flags() {
    for layout in LAYOUTS {
        for (flags, locale) in [(0x100, 2), (0x80, 2), (0, 0x10), (0x80000000, 2)] {
            let mut bytes = fixture(
                layout,
                &[
                    group(layout, 0, 2, &[row(7, 1)]),
                    group(layout, flags, locale, &[row(8, 2)]),
                ],
                &[],
            );
            bytes.pop();
            assert_eq!(checked(&bytes, layout), Err(Error::Truncated));
        }
    }
}

#[test]
fn signatures_versions_and_named_counts_are_checked_without_fallback() {
    for layout in [
        Layout::Header30080,
        Layout::Header50893V1,
        Layout::Header50893V2,
    ] {
        let mut bytes = minimal(layout);
        bytes[0] = b'X';
        assert_eq!(checked(&bytes, layout), Err(Error::Header));
        let mut bytes = minimal(layout);
        let named = if layout == Layout::Header30080 { 8 } else { 16 };
        bytes[named..named + 4].copy_from_slice(&2_u32.to_le_bytes());
        assert_eq!(checked(&bytes, layout), Err(Error::Header));
    }
    for layout in [Layout::Header50893V1, Layout::Header50893V2] {
        for version in [0_u32, 3, u32::MAX] {
            let mut bytes = minimal(layout);
            bytes[8..12].copy_from_slice(&version.to_le_bytes());
            assert_eq!(checked(&bytes, layout), Err(Error::Unsupported));
        }
    }
    let bytes = minimal(Layout::Header50893V1);
    assert_eq!(
        checked(&bytes, Layout::Header50893V2),
        Err(Error::Unsupported)
    );
}

#[test]
fn modern_header_size_rejects_clamps_out_of_bounds_and_limits() {
    for layout in [Layout::Header50893V1, Layout::Header50893V2] {
        for size in [0_u32, 3, 4, 12, 19] {
            let mut bytes = minimal(layout);
            bytes[4..8].copy_from_slice(&size.to_le_bytes());
            assert_eq!(checked(&bytes, layout), Err(Error::Header));
        }
        let mut bytes = minimal(layout);
        let beyond = u32::try_from(bytes.len() + 1).unwrap();
        bytes[4..8].copy_from_slice(&beyond.to_le_bytes());
        assert_eq!(checked(&bytes, layout), Err(Error::Truncated));
        bytes[4..8].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(checked(&bytes, layout), Err(Error::Limit));
    }
}

#[test]
fn exact_limits_accept_and_one_below_each_rejects() {
    let layout = Layout::Header50893V2;
    let bytes = fixture(
        layout,
        &[
            group(layout, 0, 2, &[row(7, 1), row(1, 2)]),
            group(layout, 0, 2, &[row(7, 3)]),
        ],
        b"extra",
    );
    let exact = Limits {
        max_bytes: bytes.len() as u64,
        max_header_bytes: 25,
        max_blocks: 2,
        max_entries: 3,
    };
    assert!(check_limited(&bytes, layout, exact).is_ok());
    for limits in [
        Limits {
            max_bytes: exact.max_bytes - 1,
            ..exact
        },
        Limits {
            max_header_bytes: 24,
            ..exact
        },
        Limits {
            max_blocks: 1,
            ..exact
        },
        Limits {
            max_entries: 2,
            ..exact
        },
    ] {
        assert_eq!(check_limited(&bytes, layout, limits), Err(Error::Limit));
    }
}

#[test]
fn counts_hit_limits_before_reservation_and_duplicates_consume_budget() {
    for layout in LAYOUTS {
        let mut block = group(layout, 0, 2, &[row(7, 1)]);
        block[..4].copy_from_slice(&u32::MAX.to_le_bytes());
        let mut bytes = minimal(layout);
        let header = match layout {
            Layout::Headerless18125 => 0,
            Layout::Header30080 => 12,
            _ => 20,
        };
        bytes.truncate(header);
        bytes.extend_from_slice(&block);
        assert_eq!(checked(&bytes, layout), Err(Error::Limit));
        let bytes = fixture(
            layout,
            &[
                group(layout, 0, 2, &[row(7, 1)]),
                group(layout, 0x100, 2, &[row(7, 1)]),
            ],
            &[],
        );
        assert_eq!(
            check_limited(
                &bytes,
                layout,
                Limits {
                    max_entries: 1,
                    ..LIMITS
                }
            ),
            Err(Error::Limit)
        );
    }
}

#[test]
fn diagnostics_are_fixed_codes_without_paths_or_keys() {
    for (error, code) in [
        (Error::Limit, "root_policy_limit"),
        (Error::Checksum, "root_content_checksum_mismatch"),
        (Error::Truncated, "root_truncated"),
        (Error::Header, "root_invalid_header"),
        (Error::Unsupported, "root_unsupported_layout"),
        (Error::FileDataId, "root_invalid_file_data_id"),
    ] {
        assert_eq!(error.code(), code);
        assert_eq!(error.to_string(), code);
    }
}
