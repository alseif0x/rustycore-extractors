// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: synthetic byte-exact cache replay fixtures.
use super::*;

fn put(bytes: &mut [u8], at: usize, value: u32) {
    bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
}
fn record(flags: u8) -> Vec<u8> {
    let name = b"Name.M2";
    let fixed = if flags & 1 != 0 { 62 } else { 38 };
    let mut bytes = vec![0; fixed];
    bytes[0] = flags;
    bytes[1] = 255;
    put(&mut bytes, 2, 0xdead_beef);
    for (at, bits) in [
        (6, 0x8000_0000),
        (10, 1),
        (14, 0x7f7f_ffff),
        (18, 0xff7f_ffff),
        (22, 0),
        (26, 0x8000_0001),
        (30, 0x8000_0000),
    ] {
        put(&mut bytes, at, bits);
    }
    if flags & 1 != 0 {
        for (index, bits) in [
            0x8000_0000,
            (-2.0_f32).to_bits(),
            0,
            0,
            0,
            3.0_f32.to_bits(),
        ]
        .into_iter()
        .enumerate()
        {
            put(&mut bytes, 34 + index * 4, bits);
        }
    }
    put(&mut bytes, fixed - 4, name.len() as u32);
    bytes.extend_from_slice(name);
    bytes
}

#[test]
fn every_flag_value_clears_only_parent_bit_then_replays_context() {
    for flags in 0..=7 {
        let input = record(flags);
        let snapshot = input.clone();
        let cache = Cached::from_bytes(&input, Limits::default()).unwrap();
        assert_eq!(cache.record_count(), 1);
        assert_eq!(cache.bytes()[0], flags & !2);
        assert_eq!(&cache.bytes()[1..], &input[1..]);
        for (map, original, expected) in [(7, 7, flags & !2), (0, u32::MAX, flags | 2)] {
            let out = cache.replay(map, original, Limits::default()).unwrap();
            assert_eq!(out[0], expected);
            assert_eq!(&out[1..], &input[1..]);
            let parsed = DirBin::parse(&out, super::super::Limits::default()).unwrap();
            assert_eq!(parsed.record_count(), 1);
            let value = parsed.records().next().unwrap();
            assert_eq!(value.id(), 0xdead_beef);
            assert_eq!(value.name_set(), 255);
            assert_eq!(value.scale_bits(), 0x8000_0000);
        }
        assert_eq!(input, snapshot);
    }
}

#[test]
fn mixed_record_sizes_and_duplicate_ids_preserve_order_and_entire_bodies() {
    let mut input = Vec::new();
    let mut offsets = Vec::new();
    for flags in [7, 0, 1, 2, 4, 5, 6, 3, 0] {
        offsets.push(input.len());
        input.extend(record(flags));
    }
    let cache = Cached::from_bytes(&input, Limits::default()).unwrap();
    let out = cache.replay(11, 4, Limits::default()).unwrap();
    assert_eq!(cache.record_count(), offsets.len());
    for (at, (source, replay)) in input.iter().zip(&out).enumerate() {
        assert_eq!(
            *replay,
            if offsets.contains(&at) {
                *source | 2
            } else {
                *source
            }
        );
    }
    assert_eq!(
        DirBin::parse(&out, super::super::Limits::default())
            .unwrap()
            .records()
            .map(|value| value.id())
            .collect::<Vec<_>>(),
        vec![0xdead_beef; offsets.len()]
    );
}

#[test]
fn repeated_replays_are_independent_and_never_mutate_cache_or_previous_outputs() {
    let input = record(7);
    let cache = Cached::from_bytes(&input, Limits::default()).unwrap();
    let cached = cache.bytes().to_vec();
    let first = cache.replay(1, 0, Limits::default()).unwrap();
    let second = cache.replay(1, 1, Limits::default()).unwrap();
    let third = cache.replay(2, 0, Limits::default()).unwrap();
    assert_eq!(first[0], 7);
    assert_eq!(second[0], 5);
    assert_eq!(third, first);
    assert_eq!(cache.bytes(), cached);
}

#[test]
fn validated_view_constructor_revalidates_reduced_limits() {
    let bytes = record(2);
    let view = DirBin::parse(&bytes, super::super::Limits::default()).unwrap();
    let cache = Cached::from_dir_bin(&view, Limits::default()).unwrap();
    assert_eq!(cache.bytes()[0], 0);
    assert_eq!(
        Cached::from_dir_bin(
            &view,
            Limits {
                records: super::super::Limits {
                    max_records: 0,
                    ..super::super::Limits::default()
                },
                ..Limits::default()
            }
        )
        .err(),
        Some(Error::Limit)
    );
}

#[test]
fn empty_stream_constructs_and_replays_with_all_zero_caps() {
    let limits = Limits {
        records: super::super::Limits {
            max_bytes: 0,
            max_records: 0,
            max_name_bytes: 0,
        },
        max_memory_bytes: 0,
    };
    let cache = Cached::from_bytes(&[], limits).unwrap();
    assert_eq!(cache.record_count(), 0);
    assert_eq!(cache.resident_bytes(), 0);
    assert!(cache.replay(0, 0, limits).unwrap().is_empty());
    assert!(cache.replay(0, u32::MAX, limits).unwrap().is_empty());
}

#[test]
fn malformed_tail_or_truncation_never_returns_a_valid_prefix() {
    let valid = record(1);
    for length in 1..valid.len() {
        assert!(Cached::from_bytes(&valid[..length], Limits::default()).is_err());
    }
    for tail in [&[0][..], &record(0)[..10], &[8][..]] {
        let mut input = valid.clone();
        input.extend_from_slice(tail);
        assert!(Cached::from_bytes(&input, Limits::default()).is_err());
    }
}

#[test]
fn source_flags_finite_values_bounds_and_names_are_fully_validated() {
    let mut flags = record(0);
    flags[0] = 8;
    assert_eq!(
        Cached::from_bytes(&flags, Limits::default()).err(),
        Some(Error::Flags)
    );
    for at in [6, 10, 14, 18, 22, 26, 30, 34, 46] {
        let mut bytes = record(1);
        put(&mut bytes, at, f32::INFINITY.to_bits());
        assert_eq!(
            Cached::from_bytes(&bytes, Limits::default()).err(),
            Some(Error::NonFinite)
        );
    }
    let mut inverted = record(1);
    put(&mut inverted, 34, 1.0_f32.to_bits());
    assert_eq!(
        Cached::from_bytes(&inverted, Limits::default()).err(),
        Some(Error::InvertedBounds)
    );
    for invalid in [0, 0x80] {
        let mut bytes = record(0);
        bytes[38] = invalid;
        assert_eq!(
            Cached::from_bytes(&bytes, Limits::default()).err(),
            Some(Error::Name)
        );
    }
}

#[test]
fn construction_memory_counts_actual_owned_capacity() {
    let input = record(0);
    let baseline = Cached::from_bytes(&input, Limits::default()).unwrap();
    let exact = Limits {
        max_memory_bytes: baseline.resident_bytes(),
        ..Limits::default()
    };
    assert!(Cached::from_bytes(&input, exact).is_ok());
    assert_eq!(
        Cached::from_bytes(
            &input,
            Limits {
                max_memory_bytes: exact.max_memory_bytes - 1,
                ..exact
            }
        )
        .err(),
        Some(Error::Limit)
    );
}

#[test]
fn replay_memory_counts_old_cache_plus_new_output_but_not_previous_outputs() {
    let cache = Cached::from_bytes(&record(3), Limits::default()).unwrap();
    let first = cache.replay(0, 0, Limits::default()).unwrap();
    let exact = Limits {
        max_memory_bytes: cache.resident_bytes() + first.capacity(),
        ..Limits::default()
    };
    let second = cache.replay(1, 0, exact).unwrap();
    let third = cache.replay(1, 0, exact).unwrap();
    assert_eq!(second, third);
    assert_eq!(first[0], 1);
    assert_eq!(second[0], 3);
    assert_eq!(
        cache.replay(
            1,
            0,
            Limits {
                max_memory_bytes: exact.max_memory_bytes - 1,
                ..exact
            }
        ),
        Err(Error::Limit)
    );
    assert_eq!(cache.replay(1, 0, exact).unwrap(), second);
}

#[test]
fn complete_byte_record_and_name_caps_apply_to_construction_and_replay() {
    let bytes = record(3);
    let cache = Cached::from_bytes(&bytes, Limits::default()).unwrap();
    for records in [
        super::super::Limits {
            max_bytes: bytes.len() - 1,
            ..super::super::Limits::default()
        },
        super::super::Limits {
            max_records: 0,
            ..super::super::Limits::default()
        },
        super::super::Limits {
            max_name_bytes: 6,
            ..super::super::Limits::default()
        },
    ] {
        let limits = Limits {
            records,
            ..Limits::default()
        };
        assert_eq!(Cached::from_bytes(&bytes, limits).err(), Some(Error::Limit));
        assert_eq!(cache.replay(1, 0, limits), Err(Error::Limit));
    }
    let exact = Limits {
        records: super::super::Limits {
            max_bytes: bytes.len(),
            max_records: 1,
            max_name_bytes: 7,
        },
        ..Limits::default()
    };
    assert!(Cached::from_bytes(&bytes, exact).is_ok());
    assert_eq!(cache.replay(1, 0, exact).unwrap(), bytes);
}

#[test]
fn cumulative_record_and_byte_caps_reject_late_complete_records() {
    let mut bytes = record(0);
    let one_len = bytes.len();
    bytes.extend(record(1));
    for records in [
        super::super::Limits {
            max_bytes: bytes.len() - 1,
            ..super::super::Limits::default()
        },
        super::super::Limits {
            max_records: 1,
            ..super::super::Limits::default()
        },
    ] {
        assert_eq!(
            Cached::from_bytes(
                &bytes,
                Limits {
                    records,
                    ..Limits::default()
                }
            )
            .err(),
            Some(Error::Limit)
        );
    }
    assert!(one_len < bytes.len());
}

#[test]
fn raised_limits_reject_even_empty_streams_and_replays() {
    let cache = Cached::from_bytes(&[], Limits::default()).unwrap();
    for limits in [
        Limits {
            max_memory_bytes: MAX_MEMORY_BYTES + 1,
            ..Limits::default()
        },
        Limits {
            records: super::super::Limits {
                max_bytes: super::super::MAX_BYTES + 1,
                ..super::super::Limits::default()
            },
            ..Limits::default()
        },
        Limits {
            records: super::super::Limits {
                max_records: super::super::MAX_RECORDS + 1,
                ..super::super::Limits::default()
            },
            ..Limits::default()
        },
        Limits {
            records: super::super::Limits {
                max_name_bytes: super::super::MAX_NAME_BYTES + 1,
                ..super::super::Limits::default()
            },
            ..Limits::default()
        },
    ] {
        assert_eq!(
            Cached::from_bytes(&[], limits).err(),
            Some(Error::InvalidLimits)
        );
        assert_eq!(cache.replay(1, 0, limits), Err(Error::InvalidLimits));
    }
}
