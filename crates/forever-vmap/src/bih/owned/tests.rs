// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: synthetic ownership, intrinsic count and capacity tests.
use super::*;

fn wire(count: usize) -> Vec<u8> {
    let primitives: Vec<_> = (0..count)
        .map(|i| {
            let x = i as f32;
            Bounds::from_bits([x, 0., 0., x + 1., 1., 1.].map(f32::to_bits)).unwrap()
        })
        .collect();
    build(&primitives, 1, Limits::default())
        .unwrap()
        .into_bytes()
}
fn spare(bytes: &[u8], capacity: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(capacity);
    out.extend_from_slice(bytes);
    out
}

#[test]
fn empty_owned_retains_exact_bytes_and_charges_capacity() {
    let bytes = wire(0);
    let input = spare(&bytes, 100);
    let capacity = input.capacity();
    let owned = Owned::parse_intrinsic(
        input,
        Limits {
            max_memory_bytes: capacity,
            ..Limits::default()
        },
    )
    .unwrap();
    assert_eq!(owned.bytes(), bytes);
    assert_eq!(owned.primitive_count(), 0);
    assert_eq!(owned.resident_bytes(), capacity);
    assert_eq!(owned.validation_peak_bytes(), capacity);
    assert_eq!(owned.view().stats().validation_peak_bytes, 0);
}
#[test]
fn owned_move_and_repeated_views_keep_original_allocation() {
    let bytes = wire(12);
    let pointer = bytes.as_ptr();
    let owned = Owned::parse(bytes, 12, Limits::default()).unwrap();
    let moved = std::hint::black_box(owned);
    for _ in 0..100 {
        let view = moved.view();
        assert_eq!(view.bytes().as_ptr(), pointer);
        assert_eq!(view.stats(), moved.stats());
        let mut ids: Vec<_> = view.objects().collect();
        ids.sort_unstable();
        assert_eq!(ids, (0..12).collect::<Vec<_>>());
    }
    let bytes = moved.into_bytes();
    assert_eq!(bytes.as_ptr(), pointer);
}
#[test]
fn intrinsic_count_is_complete_validation_not_a_header_shortcut() {
    let bytes = wire(5);
    for n in 0..bytes.len() {
        assert!(Owned::parse_intrinsic(bytes[..n].to_vec(), Limits::default()).is_err());
    }
    let mut tail = bytes.clone();
    tail.push(0);
    assert!(Owned::parse_intrinsic(tail, Limits::default()).is_err());
    let mut bad = bytes;
    let end = bad.len();
    bad[end - 4..].copy_from_slice(&u32::MAX.to_le_bytes());
    assert_eq!(
        Owned::parse_intrinsic(bad, Limits::default()).err(),
        Some(Error::Permutation)
    );
}
#[test]
fn input_spare_capacity_cannot_hide_behind_length() {
    let bytes = wire(0);
    let input = spare(&bytes, 256);
    let cap = input.capacity();
    assert!(cap > bytes.len());
    assert_eq!(
        Owned::parse(
            input,
            0,
            Limits {
                max_memory_bytes: cap - 1,
                ..Limits::default()
            }
        )
        .err(),
        Some(Error::Limit)
    );
}
#[test]
fn validation_scratch_overlaps_retained_bytes() {
    let bytes = wire(9);
    let owned = Owned::parse(bytes.clone(), 9, Limits::default()).unwrap();
    let peak = owned.validation_peak_bytes();
    assert_eq!(
        peak,
        owned.resident_bytes() + owned.stats().validation_peak_bytes
    );
    assert!(peak > owned.resident_bytes());
    assert!(
        Owned::parse(
            bytes.clone(),
            9,
            Limits {
                max_memory_bytes: peak,
                ..Limits::default()
            }
        )
        .is_ok()
    );
    assert_eq!(
        Owned::parse(
            bytes,
            9,
            Limits {
                max_memory_bytes: peak - 1,
                ..Limits::default()
            }
        )
        .err(),
        Some(Error::Limit)
    );
}
#[test]
fn expected_count_still_binds_external_count() {
    let bytes = wire(2);
    assert_eq!(
        Owned::parse(bytes.clone(), 1, Limits::default()).err(),
        Some(Error::PrimitiveCount)
    );
    assert_eq!(
        Owned::parse_intrinsic(bytes, Limits::default())
            .unwrap()
            .primitive_count(),
        2
    );
}
#[test]
fn intrinsic_and_owned_limits_reject_raised_or_reduced_profiles() {
    let bytes = wire(3);
    for limits in [
        Limits {
            max_primitives: 2,
            ..Limits::default()
        },
        Limits {
            max_bytes: bytes.len() - 1,
            ..Limits::default()
        },
        Limits {
            max_depth: 0,
            ..Limits::default()
        },
    ] {
        assert!(Owned::parse_intrinsic(bytes.clone(), limits).is_err());
    }
    assert_eq!(
        Owned::parse_intrinsic(
            bytes,
            Limits {
                max_memory_bytes: MAX_MEMORY_BYTES + 1,
                ..Limits::default()
            }
        )
        .err(),
        Some(Error::InvalidLimits)
    );
}
