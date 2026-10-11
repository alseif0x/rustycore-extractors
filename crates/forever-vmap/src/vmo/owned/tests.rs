// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: independent synthetic VMO ownership fixtures.
use super::*;

fn words(bytes: &mut Vec<u8>, values: impl IntoIterator<Item = u32>) {
    for v in values {
        bytes.extend_from_slice(&v.to_le_bytes());
    }
}
fn bounds(z: f32) -> [u32; 6] {
    [0., -0., z, 1., 1., z].map(f32::to_bits)
}
fn leaf(bits: [u32; 6], count: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    words(&mut bytes, bits);
    words(&mut bytes, [3, 0xc000_0000, count, 0, count]);
    words(&mut bytes, 0..count);
    bytes
}
fn header() -> Vec<u8> {
    let mut bytes = b"VMAP_4.EWMOD".to_vec();
    words(&mut bytes, [8, 1, 77]);
    bytes
}
fn fixture(group_count: usize, liquid: bool, empty: bool) -> Vec<u8> {
    let mut bytes = header();
    if group_count == 0 {
        return bytes;
    }
    bytes.extend_from_slice(b"GMOD");
    words(&mut bytes, [group_count as u32]);
    for index in 0..group_count {
        let z = index as f32;
        words(&mut bytes, bounds(z));
        words(&mut bytes, [0xabcd, index as u32]);
        bytes.extend_from_slice(b"VERT");
        words(&mut bytes, if empty { [4, 0] } else { [40, 3] });
        if empty {
            continue;
        }
        words(
            &mut bytes,
            [0., -0., z, 1., 0., z, 0., 1., z].map(f32::to_bits),
        );
        bytes.extend_from_slice(b"TRIM");
        words(&mut bytes, [16, 1, 0, 1, 2]);
        bytes.extend_from_slice(b"MBIH");
        bytes.extend_from_slice(&leaf(bounds(z), 1));
        bytes.extend_from_slice(b"LIQU");
        if liquid {
            words(&mut bytes, [41, 1, 1]);
            words(&mut bytes, [-0., 1., z].map(f32::to_bits));
            words(&mut bytes, [u32::MAX]);
            words(&mut bytes, [0., -0., 2., 3.].map(f32::to_bits));
            bytes.push(0xaf);
        } else {
            words(&mut bytes, [0]);
        }
    }
    bytes.extend_from_slice(b"GBIH");
    let all = [0., -0., 0., 1., 1., (group_count - 1) as f32].map(f32::to_bits);
    bytes.extend_from_slice(&leaf(all, group_count as u32));
    bytes
}
fn compare(owned: &Owned, view: &View<'_>) {
    assert_eq!(owned.bytes(), view.bytes());
    assert_eq!(owned.flags(), view.flags());
    assert_eq!(owned.root_wmo_id(), view.root_wmo_id());
    assert_eq!(owned.group_count(), view.groups().len());
    assert_eq!(
        owned.group_bih().map(|v| v.bytes()),
        view.group_bih().map(|v| v.bytes())
    );
    for (i, want) in view.groups().iter().enumerate() {
        let got = owned.group(i).unwrap();
        assert_eq!(got.bounds_bits(), want.bounds_bits());
        assert_eq!(got.flags(), want.flags());
        assert_eq!(got.group_wmo_id(), want.group_wmo_id());
        assert_eq!(got.vertex_count(), want.vertex_count());
        assert_eq!(got.triangle_count(), want.triangle_count());
        for i in 0..=want.vertex_count() {
            assert_eq!(got.vertex_bits(i), want.vertex_bits(i));
        }
        for i in 0..=want.triangle_count() {
            assert_eq!(got.triangle(i), want.triangle(i));
        }
        assert_eq!(
            got.mesh_bih().map(|v| v.bytes()),
            want.mesh_bih().map(|v| v.bytes())
        );
        if let Some(liquid) = got.liquid() {
            let other = want.liquid().unwrap();
            assert_eq!(liquid.dimensions(), other.dimensions());
            assert_eq!(liquid.corner_bits(), other.corner_bits());
            assert_eq!(liquid.type_id(), other.type_id());
            assert_eq!(liquid.tiles(), other.tiles());
            for i in 0..=other.height_count() {
                assert_eq!(liquid.height_bits(i), other.height_bits(i));
            }
        } else {
            assert!(want.liquid().is_none());
        }
    }
    assert!(owned.group(owned.group_count()).is_none());
}

#[test]
fn owned_header_only_requires_no_descriptors_but_keeps_capacity() {
    let mut bytes = Vec::with_capacity(128);
    bytes.extend_from_slice(&header());
    let cap = bytes.capacity();
    let owned = Owned::parse(
        bytes,
        Limits {
            max_memory_bytes: cap,
            ..Limits::default()
        },
    )
    .unwrap();
    assert_eq!(owned.group_count(), 0);
    assert!(owned.group_bih().is_none());
    assert_eq!(owned.resident_bytes(), cap);
    assert_eq!(owned.validation_peak_bytes(), cap);
}
#[test]
fn exact_group_geometry_and_liquid_bits_survive_owned_offsets() {
    for liquid in [false, true] {
        let bytes = fixture(3, liquid, false);
        let owned = Owned::parse(bytes.clone(), Limits::default()).unwrap();
        let view = View::parse(&bytes, Limits::default()).unwrap();
        compare(&owned, &view);
    }
}
#[test]
fn empty_group_sentinels_do_not_require_pointers_inside_owner() {
    let bytes = fixture(2, false, true);
    let owned = Owned::parse(bytes.clone(), Limits::default()).unwrap();
    compare(&owned, &View::parse(&bytes, Limits::default()).unwrap());
    assert_eq!(owned.group(0).unwrap().triangle_count(), 0);
    assert!(owned.group(0).unwrap().mesh_bih().is_none());
}
#[test]
fn moving_owner_preserves_byte_allocation_and_repeated_stack_views() {
    let bytes = fixture(2, true, false);
    let original = bytes.as_ptr();
    let owned = Owned::parse(bytes, Limits::default()).unwrap();
    let owned = std::hint::black_box(owned);
    let resident = owned.resident_bytes();
    for _ in 0..100 {
        assert_eq!(owned.bytes().as_ptr(), original);
        assert_eq!(owned.group(1).unwrap().group_wmo_id(), 1);
        assert_eq!(owned.group_bih().unwrap().stats().primitive_count, 2);
        assert_eq!(owned.resident_bytes(), resident);
    }
    let bytes = owned.into_bytes();
    assert_eq!(bytes.as_ptr(), original);
}
#[test]
fn complete_parser_rejects_each_truncation_and_tail_before_ownership() {
    let bytes = fixture(1, true, false);
    for end in 0..bytes.len() {
        // The complete 24-byte header is independently a valid empty model;
        // it contains no declaration that any later GMOD must exist.
        if end == 24 {
            assert_eq!(
                Owned::parse(bytes[..end].to_vec(), Limits::default())
                    .unwrap()
                    .group_count(),
                0
            );
        } else {
            assert!(Owned::parse(bytes[..end].to_vec(), Limits::default()).is_err());
        }
    }
    let mut tail = bytes;
    tail.push(0);
    assert!(Owned::parse(tail, Limits::default()).is_err());
}
#[test]
fn owned_admission_preserves_geometry_and_bih_containment_checks() {
    let mut bytes = fixture(1, false, false);
    // First vertex follows header32 + bounds/flags/id32 + VERT12.
    bytes[76..80].copy_from_slice(&f32::NAN.to_bits().to_le_bytes());
    assert_eq!(
        Owned::parse(bytes, Limits::default()).err(),
        Some(Error::NonFinite)
    );
    let mut bytes = fixture(1, false, false);
    let at = bytes.windows(4).position(|b| b == b"MBIH").unwrap() + 4;
    // BIH root x low becomes 0.5, excluding the triangle's x=0 vertex.
    bytes[at..at + 4].copy_from_slice(&0.5f32.to_bits().to_le_bytes());
    assert_eq!(
        Owned::parse(bytes, Limits::default()).err(),
        Some(Error::Containment)
    );
}
#[test]
fn memory_charges_backing_borrowed_and_owned_descriptors_during_transfer() {
    let bytes = fixture(3, true, false);
    let borrowed = View::parse(&bytes, Limits::default()).unwrap();
    let owned = Owned::parse(bytes.clone(), Limits::default()).unwrap();
    let descriptors = owned.groups.capacity() * size_of::<GroupLayout>();
    assert_eq!(owned.resident_bytes(), owned.bytes.capacity() + descriptors);
    assert_eq!(
        owned.validation_peak_bytes(),
        (owned.bytes.capacity() + borrowed.stats().peak_bytes)
            .max(owned.bytes.capacity() + borrowed.stats().resident_bytes + descriptors)
    );
    let peak = owned.validation_peak_bytes();
    assert!(
        Owned::parse(
            bytes.clone(),
            Limits {
                max_memory_bytes: peak,
                ..Limits::default()
            }
        )
        .is_ok()
    );
    assert!(
        Owned::parse(
            bytes,
            Limits {
                max_memory_bytes: peak - 1,
                ..Limits::default()
            }
        )
        .is_err()
    );
}
#[test]
fn large_unused_capacity_is_charged_even_for_empty_vmo() {
    let mut bytes = Vec::with_capacity(1024);
    bytes.extend_from_slice(&header());
    let cap = bytes.capacity();
    assert_eq!(
        Owned::parse(
            bytes,
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
fn downward_and_raised_limits_remain_closed() {
    let bytes = fixture(1, false, false);
    for limits in [
        Limits {
            max_groups: 0,
            ..Limits::default()
        },
        Limits {
            max_vertices: 2,
            ..Limits::default()
        },
        Limits {
            max_triangles: 0,
            ..Limits::default()
        },
        Limits {
            max_bytes: bytes.len() - 1,
            ..Limits::default()
        },
    ] {
        assert!(Owned::parse(bytes.clone(), limits).is_err());
    }
    assert_eq!(
        Owned::parse(
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
