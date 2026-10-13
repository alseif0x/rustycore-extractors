// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: synthetic owned complete map-tree storage tests.
use super::*;

fn wire(count: usize) -> Vec<u8> {
    let bounds: Vec<_> = (0..count)
        .map(|i| {
            let x = i as f32;
            bih::Bounds::from_bits([x, 0., 0., x + 1., 1., 1.].map(f32::to_bits)).unwrap()
        })
        .collect();
    let tree = bih::build(&bounds, 3, bih::Limits::default()).unwrap();
    let mut bytes = b"VMAP_4.ENODE".to_vec();
    bytes.extend_from_slice(tree.bytes());
    bytes
}
#[test]
fn intrinsic_empty_tree_and_owned_storage_preserve_envelope() {
    let bytes = wire(0);
    let owned = OwnedTree::parse(bytes.clone(), Limits::default()).unwrap();
    assert_eq!(owned.bytes(), bytes);
    assert_eq!(owned.primitive_count(), 0);
    assert_eq!(owned.bih().bytes(), &bytes[12..]);
    assert_eq!(owned.validation_peak_bytes(), owned.resident_bytes());
    assert_eq!(
        Tree::parse_intrinsic(&bytes, Limits::default())
            .unwrap()
            .bih()
            .stats()
            .primitive_count,
        0
    );
}
#[test]
fn moved_owned_tree_reuses_allocation_and_borrowed_parser() {
    let bytes = wire(11);
    let pointer = bytes.as_ptr();
    let capacity = bytes.capacity();
    let owned = OwnedTree::parse(bytes, Limits::default()).unwrap();
    let moved = std::hint::black_box(owned);
    for _ in 0..20 {
        assert_eq!(moved.bytes().as_ptr(), pointer);
        assert_eq!(moved.bih().stats().primitive_count, 11);
        assert_eq!(moved.resident_bytes(), capacity);
    }
    let bytes = moved.into_bytes();
    assert_eq!(bytes.as_ptr(), pointer);
}
#[test]
fn complete_header_tail_and_intrinsic_count_are_checked() {
    let bytes = wire(3);
    for end in 0..bytes.len() {
        assert!(OwnedTree::parse(bytes[..end].to_vec(), Limits::default()).is_err());
    }
    let mut bad = bytes.clone();
    bad[8] = b'X';
    assert_eq!(
        OwnedTree::parse(bad, Limits::default()).err(),
        Some(Error::Framing)
    );
    let mut bad = bytes;
    bad.push(0);
    assert!(OwnedTree::parse(bad, Limits::default()).is_err());
}
#[test]
fn descriptor_free_tree_still_charges_spare_bytes_and_scratch() {
    let source = wire(5);
    let mut bytes = Vec::with_capacity(4096);
    bytes.extend_from_slice(&source);
    let owned = OwnedTree::parse(bytes, Limits::default()).unwrap();
    let cap = owned.resident_bytes();
    let peak = owned.validation_peak_bytes();
    assert!(cap >= 4096 && peak > cap);
    for limit in [cap - 1, peak - 1, peak] {
        let mut bytes = Vec::with_capacity(cap);
        bytes.extend_from_slice(&source);
        assert_eq!(
            OwnedTree::parse(
                bytes,
                Limits {
                    max_memory_bytes: limit,
                    ..Limits::default()
                }
            )
            .is_ok(),
            limit == peak
        );
    }
}
#[test]
fn map_spawn_ceiling_applies_before_intrinsic_tree_allocation() {
    let bytes = wire(4);
    assert!(
        Tree::parse_intrinsic(
            &bytes,
            Limits {
                max_spawns: 3,
                ..Limits::default()
            }
        )
        .is_err()
    );
    assert!(
        OwnedTree::parse(
            bytes.clone(),
            Limits {
                max_output_bytes: bytes.len() - 1,
                ..Limits::default()
            }
        )
        .is_err()
    );
    assert_eq!(
        OwnedTree::parse(
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
