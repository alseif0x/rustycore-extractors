// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11. Synthetic inputs only; no client data.

use crate::{Error, Limits, M2Collision, MAX_INPUT_BYTES, MAX_RAW_BYTES, RawM2};

fn put(bytes: &mut [u8], at: usize, value: u32) {
    bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

fn vertices() -> [[u32; 3]; 3] {
    [
        [1.0f32.to_bits(), (-0.0f32).to_bits(), 3.0f32.to_bits()],
        [(-1.0f32).to_bits(), 2.0f32.to_bits(), 0.0f32.to_bits()],
        [0.0f32.to_bits(), 0.0f32.to_bits(), (-3.0f32).to_bits()],
    ]
}

fn fixture_with(points: &[[u32; 3]], indices: &[u16], prefix: &[u8]) -> Vec<u8> {
    let mut bytes = vec![0; 305]; // Deliberately unaligned vertex offset.
    bytes[..4].copy_from_slice(b"MD20");
    put(&mut bytes, 4, 0xaabb_ccdd); // Source does not gate this version value.
    for (i, value) in [-1.0f32, -2.0, -3.0, 4.0, 5.0, 6.0].into_iter().enumerate() {
        put(&mut bytes, 188 + i * 4, value.to_bits());
    }
    put(&mut bytes, 216, indices.len() as u32);
    put(&mut bytes, 224, points.len() as u32);
    put(&mut bytes, 228, 305);
    for vertex in points {
        for value in vertex {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
    }
    bytes.extend_from_slice(&[0xaa, 0xbb, 0xcc]); // Unrelated body padding.
    let offset = bytes.len() as u32;
    put(&mut bytes, 220, offset);
    for index in indices {
        bytes.extend_from_slice(&index.to_le_bytes());
    }
    bytes.extend_from_slice(b"unrelated tail");
    let mut output = prefix.to_vec();
    output.extend_from_slice(&bytes);
    output
}

fn fixture() -> Vec<u8> {
    fixture_with(&vertices(), &[0, 1, 2], b"")
}

fn m2(bytes: &[u8]) -> M2Collision<'_> {
    M2Collision::parse(bytes, Limits::default())
        .unwrap_or_else(|e| panic!("{}", e.code()))
        .unwrap_or_else(|| panic!("synthetic collision unexpectedly absent"))
}

fn parsed_error(bytes: &[u8], limits: Limits) -> Error {
    match M2Collision::parse(bytes, limits) {
        Err(error) => error,
        Ok(_) => panic!("synthetic M2 unexpectedly admitted"),
    }
}

fn raw_error(bytes: &[u8], limits: Limits) -> Error {
    match RawM2::parse(bytes, limits) {
        Err(error) => error,
        Ok(_) => panic!("synthetic raw unexpectedly admitted"),
    }
}

fn raw() -> Vec<u8> {
    m2(&fixture()).write_raw(MAX_RAW_BYTES).unwrap()
}

#[test]
fn parses_borrowed_collision_and_reports_index_count() {
    let bytes = fixture();
    let parsed = m2(&bytes);
    assert_eq!(parsed.header_offset(), 0);
    assert_eq!(parsed.vertex_count(), 3);
    assert_eq!(parsed.index_count(), 3);
    assert_eq!(parsed.triangle_count(), 1);
    assert_eq!(parsed.source_index(1), Some(1));
    assert_eq!(parsed.raw_triangle(0), Some([0, 2, 1]));
    for (index, expected) in vertices().into_iter().enumerate() {
        assert_eq!(parsed.vertex_bits(index), Some(expected));
    }
}

#[test]
fn wrapped_marker_uses_relative_offsets_without_claiming_md21_parser() {
    let prefix = b"MD21\xff\xff\xff\xffarbitrary";
    let bytes = fixture_with(&vertices(), &[0, 1, 2], prefix);
    let parsed = m2(&bytes);
    assert_eq!(parsed.header_offset(), prefix.len());
    assert_eq!(parsed.write_raw(MAX_RAW_BYTES).unwrap(), raw());
}

#[test]
fn first_marker_is_authoritative_even_when_a_later_header_is_valid() {
    let mut bytes = fixture();
    put(&mut bytes, 216, 1);
    bytes.extend_from_slice(&fixture());
    assert_eq!(parsed_error(&bytes, Limits::default()), Error::Geometry);
}

#[test]
fn first_empty_collision_does_not_search_for_later_geometry() {
    let mut bytes = fixture();
    put(&mut bytes, 216, 0);
    bytes.extend_from_slice(&fixture());
    assert!(
        M2Collision::parse(&bytes, Limits::default())
            .unwrap()
            .is_none()
    );
}

#[test]
fn missing_and_truncated_markers_are_errors_not_absence() {
    for bytes in [b"".as_slice(), b"MD2", b"MD21", &[0; 400]] {
        assert_eq!(parsed_error(bytes, Limits::default()), Error::MissingMd20);
    }
    for length in [4, 216, 303] {
        let mut bytes = vec![0; length];
        bytes[..4].copy_from_slice(b"MD20");
        assert_eq!(parsed_error(&bytes, Limits::default()), Error::Truncated);
    }
    let mut bytes = vec![0; 400];
    bytes.extend_from_slice(b"MD20");
    assert_eq!(parsed_error(&bytes, Limits::default()), Error::Truncated);
}

#[test]
fn empty_collision_needs_header_but_ignores_unused_geometry_fields() {
    let mut bytes = vec![0xff; 304];
    bytes[..4].copy_from_slice(b"MD20");
    put(&mut bytes, 216, 0);
    let limits = Limits {
        max_vertices: 0,
        max_indices: 0,
        ..Limits::default()
    };
    assert!(M2Collision::parse(&bytes, limits).unwrap().is_none());
}

#[test]
fn unused_header_fields_are_not_interpreted() {
    let mut bytes = fixture();
    // Version, general vertex fields, sphere, normal counts/offsets, and the
    // rest of the complete header do not participate in this source consumer.
    for at in [4, 60, 64, 184, 212, 232, 236, 300] {
        put(&mut bytes, at, u32::MAX);
    }
    assert_eq!(m2(&bytes).write_raw(MAX_RAW_BYTES).unwrap(), raw());
}

#[test]
fn overlapping_used_arrays_are_admitted_when_their_values_are_valid() {
    let mut bytes = fixture();
    put(&mut bytes, 220, 305); // First vertex bytes now also provide u16 indices.
    bytes[305..311].fill(0); // Three references to vertex zero; finite vertex.
    let parsed = m2(&bytes);
    assert_eq!(parsed.raw_triangle(0), Some([0, 0, 0]));
}

#[test]
fn used_offsets_may_reference_header_bytes() {
    let mut bytes = fixture();
    put(&mut bytes, 220, 8); // Three zero u16 indices from ignored header fields.
    put(&mut bytes, 228, 32); // Three zero vertices from ignored header fields.
    let parsed = m2(&bytes);
    assert_eq!(parsed.raw_triangle(0), Some([0; 3]));
    assert_eq!(parsed.vertex_bits(2), Some([0; 3]));
}

#[test]
fn body_offsets_and_count_extents_are_checked() {
    for at in [220, 228] {
        let mut bytes = fixture();
        put(&mut bytes, at, u32::MAX);
        assert_eq!(parsed_error(&bytes, Limits::default()), Error::Bounds);
    }
    let mut bytes = fixture();
    let past = bytes.len() as u32 - 1;
    put(&mut bytes, 220, past);
    assert_eq!(parsed_error(&bytes, Limits::default()), Error::Bounds);
}

#[test]
fn collision_requires_vertices_and_complete_triangles() {
    for count in [1, 2, 4, 5] {
        let mut bytes = fixture();
        put(&mut bytes, 216, count);
        assert_eq!(parsed_error(&bytes, Limits::default()), Error::Geometry);
    }
    let mut bytes = fixture();
    put(&mut bytes, 224, 0);
    assert_eq!(parsed_error(&bytes, Limits::default()), Error::Geometry);
}

#[test]
fn source_indices_are_checked_against_vertices() {
    let bytes = fixture_with(&vertices(), &[0, 1, 3], b"");
    assert_eq!(parsed_error(&bytes, Limits::default()), Error::Index);
}

#[test]
fn all_emitted_vertex_components_must_be_finite() {
    for nonfinite in [0x7f80_0000, 0xff80_0000, 0x7fc0_0001] {
        for component in 0..3 {
            let mut points = vertices();
            points[2][component] = nonfinite;
            // Vertex 2 is unreferenced but is still emitted by the source.
            let bytes = fixture_with(&points, &[0, 1, 0], b"");
            assert_eq!(parsed_error(&bytes, Limits::default()), Error::NonFinite);
        }
    }
}

#[test]
fn bounds_are_finite_and_ordered_without_requiring_vertex_enclosure() {
    for index in 0..6 {
        let mut bytes = fixture();
        put(&mut bytes, 188 + index * 4, f32::NAN.to_bits());
        assert_eq!(parsed_error(&bytes, Limits::default()), Error::NonFinite);
    }
    let mut bytes = fixture();
    put(&mut bytes, 188, 100.0f32.to_bits());
    assert_eq!(
        parsed_error(&bytes, Limits::default()),
        Error::InvertedBounds
    );
    for index in 0..6 {
        put(&mut bytes, 188 + index * 4, 0);
    }
    assert!(M2Collision::parse(&bytes, Limits::default()).is_ok());
}

#[test]
fn input_and_count_budgets_are_reducible() {
    let bytes = fixture();
    let exact = Limits {
        max_input_bytes: bytes.len(),
        max_vertices: 3,
        max_indices: 3,
    };
    assert!(M2Collision::parse(&bytes, exact).is_ok());
    for limits in [
        Limits {
            max_input_bytes: bytes.len() - 1,
            ..exact
        },
        Limits {
            max_vertices: 2,
            ..exact
        },
        Limits {
            max_indices: 2,
            ..exact
        },
        Limits {
            max_input_bytes: 0,
            ..exact
        },
    ] {
        assert_eq!(parsed_error(&bytes, limits), Error::Limit);
    }
}

#[test]
fn policies_cannot_raise_any_ceiling_even_for_empty_geometry() {
    let bytes = fixture_with(&[], &[], b"");
    for limits in [
        Limits {
            max_input_bytes: MAX_INPUT_BYTES + 1,
            ..Limits::default()
        },
        Limits {
            max_vertices: crate::MAX_VERTICES + 1,
            ..Limits::default()
        },
        Limits {
            max_indices: crate::MAX_INDICES + 1,
            ..Limits::default()
        },
    ] {
        assert_eq!(parsed_error(&bytes, limits), Error::InvalidLimits);
        assert_eq!(raw_error(&raw(), limits), Error::InvalidLimits);
    }
}

#[test]
fn huge_counts_fail_before_array_access_or_allocation() {
    for field in [216, 224] {
        let mut bytes = fixture();
        put(&mut bytes, field, u32::MAX);
        assert_eq!(parsed_error(&bytes, Limits::default()), Error::Limit);
    }
}

#[test]
fn writer_matches_explicit_raw_layout_golden() {
    let bytes = raw();
    let mut expected = b"VMAP04E\0".to_vec();
    // Fixed raw root/group fields through liquidflags, independently specified.
    for value in [3u32, 1, 0, 1, 0, 0] {
        expected.extend_from_slice(&value.to_le_bytes());
    }
    for value in [-1.0f32, -2.0, -3.0, 4.0, 5.0, 6.0] {
        expected.extend_from_slice(&value.to_bits().to_le_bytes());
    }
    expected.extend_from_slice(&[0; 4]);
    expected.extend_from_slice(b"GRP \x08\0\0\0\x01\0\0\0\x03\0\0\0");
    expected.extend_from_slice(b"INDX\x0a\0\0\0\x03\0\0\0");
    expected.extend_from_slice(b"\0\0\0\0\x02\0\0\0\x01\0\0\0");
    expected.extend_from_slice(b"VERT\x28\0\0\0\x03\0\0\0");
    for vertex in vertices() {
        for value in vertex {
            expected.extend_from_slice(&value.to_le_bytes());
        }
    }
    assert_eq!(bytes, expected);
    assert_eq!(bytes.len(), 148);
}

#[test]
fn signed_zero_and_subnormal_bits_survive_both_source_transforms() {
    let points = [
        [0x8000_0000, 0, 0x8000_0000],
        [0, 0x8000_0000, 0],
        [1, 0x8000_0001, f32::MAX.to_bits()],
    ];
    let mut input = fixture_with(&points, &[0, 1, 2], b"");
    // Signed zeros in bounds must also be preserved rather than canonicalized.
    put(&mut input, 188, 0x8000_0000);
    put(&mut input, 200, 0);
    let output = m2(&input).write_raw(MAX_RAW_BYTES).unwrap();
    let parsed = RawM2::parse(&output, Limits::default()).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(parsed.bounds_bits()[0], 0x8000_0000);
    for (index, value) in points.into_iter().enumerate() {
        assert_eq!(parsed.vertex_bits(index), Some(value));
    }
}

#[test]
fn each_triangle_is_swapped_independently_without_mutating_input() {
    let input = fixture_with(&vertices(), &[0, 1, 2, 2, 0, 1], b"");
    let original = input.clone();
    let parsed = m2(&input);
    let output = parsed.write_raw(MAX_RAW_BYTES).unwrap();
    assert_eq!(parsed.write_raw(MAX_RAW_BYTES).unwrap(), output);
    assert_eq!(input, original);
    let view = RawM2::parse(&output, Limits::default()).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        (0..6).map(|i| view.index(i).unwrap()).collect::<Vec<_>>(),
        [0, 2, 1, 2, 1, 0]
    );
}

#[test]
fn writer_cap_is_checked_before_output_allocation() {
    let bytes = fixture();
    let parsed = m2(&bytes);
    assert_eq!(parsed.write_raw(148).unwrap().len(), 148);
    assert_eq!(parsed.write_raw(147).unwrap_err(), Error::Limit);
    assert_eq!(parsed.write_raw(0).unwrap_err(), Error::Limit);
    assert_eq!(
        parsed.write_raw(MAX_RAW_BYTES + 1).unwrap_err(),
        Error::InvalidLimits
    );
}

#[test]
fn getters_handle_out_of_range_and_overflow_indices() {
    let input = fixture();
    let parsed = m2(&input);
    let bytes = parsed.write_raw(MAX_RAW_BYTES).unwrap();
    let view = RawM2::parse(&bytes, Limits::default()).unwrap_or_else(|e| panic!("{e}"));
    for at in [3, usize::MAX] {
        assert_eq!(parsed.vertex_bits(at), None);
        assert_eq!(parsed.source_index(at), None);
        assert_eq!(parsed.raw_triangle(at), None);
        assert_eq!(view.vertex_bits(at), None);
        assert_eq!(view.index(at), None);
    }
}

#[test]
fn raw_view_is_borrowed_and_preserves_header_facts() {
    let bytes = raw();
    let view = RawM2::parse(&bytes, Limits::default()).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(view.bytes().as_ptr(), bytes.as_ptr());
    assert_eq!(view.bytes(), bytes);
    assert_eq!(view.vertex_count(), 3);
    assert_eq!(view.index_count(), 3);
    assert_eq!(view.triangle_count(), 1);
}

#[test]
fn raw_requires_exact_m2_defaults_and_known_tags() {
    for at in [
        0, 12, 16, 20, 24, 28, 56, 60, 64, 68, 72, 76, 80, 100, 104, 108,
    ] {
        let mut bytes = raw();
        bytes[at] ^= 0x40;
        assert_eq!(
            raw_error(&bytes, Limits::default()),
            Error::RawFormat,
            "field {at}"
        );
    }
}

#[test]
fn raw_rejects_fixed_indx_size_instead_of_silently_accepting_another_format() {
    let mut bytes = raw();
    put(&mut bytes, 80, 16); // Physical payload is 4+3*4; pin declares 4+3*2.
    assert_eq!(raw_error(&bytes, Limits::default()), Error::RawFormat);
}

#[test]
fn raw_has_no_padding_or_trailing_bytes() {
    let original = raw();
    for at in [60, 76, 88, 100, 112, 148] {
        let mut bytes = original.clone();
        bytes.insert(at, 0);
        assert!(RawM2::parse(&bytes, Limits::default()).is_err());
    }
}

#[test]
fn every_raw_truncation_is_rejected() {
    let bytes = raw();
    for length in 0..bytes.len() {
        assert!(RawM2::parse(&bytes[..length], Limits::default()).is_err());
    }
}

#[test]
fn raw_count_disagreement_and_budget_overruns_fail() {
    let mut bytes = raw();
    put(&mut bytes, 8, 4);
    assert_eq!(raw_error(&bytes, Limits::default()), Error::RawExtent);
    for field in [8, 84] {
        let mut bytes = raw();
        put(&mut bytes, field, u32::MAX);
        assert_eq!(raw_error(&bytes, Limits::default()), Error::Limit);
    }
    for limits in [
        Limits {
            max_input_bytes: 147,
            ..Limits::default()
        },
        Limits {
            max_vertices: 2,
            ..Limits::default()
        },
        Limits {
            max_indices: 2,
            ..Limits::default()
        },
    ] {
        assert_eq!(raw_error(&raw(), limits), Error::Limit);
    }
}

#[test]
fn raw_checks_triangle_shape_and_empty_geometry() {
    for (field, value) in [(8, 0), (84, 0), (84, 1), (84, 2), (84, 4)] {
        let mut bytes = raw();
        put(&mut bytes, field, value);
        assert_eq!(raw_error(&bytes, Limits::default()), Error::Geometry);
    }
}

#[test]
fn raw_checks_finite_vertices_and_bounds() {
    for at in [32, 36, 40, 44, 48, 52, 112, 128, 144] {
        let mut bytes = raw();
        put(&mut bytes, at, f32::INFINITY.to_bits());
        assert_eq!(raw_error(&bytes, Limits::default()), Error::NonFinite);
    }
    let mut bytes = raw();
    put(&mut bytes, 32, 5.0f32.to_bits());
    assert_eq!(raw_error(&bytes, Limits::default()), Error::InvertedBounds);
}

#[test]
fn raw_checks_indices_including_original_u16_domain() {
    let mut bytes = raw();
    put(&mut bytes, 88, 3);
    assert_eq!(raw_error(&bytes, Limits::default()), Error::Index);

    let points = vec![[0; 3]; 65_537];
    let input = fixture_with(&points, &[0, 1, 2], b"");
    let mut bytes = m2(&input).write_raw(MAX_RAW_BYTES).unwrap();
    put(&mut bytes, 88, 65_536); // In vertex range, but impossible as source u16.
    assert_eq!(raw_error(&bytes, Limits::default()), Error::Index);
}

#[test]
fn largest_u16_index_and_unreferenced_vertices_are_supported() {
    let points = vec![[0; 3]; 65_537];
    let bytes = fixture_with(&points, &[0, u16::MAX, 2], b"");
    let output = m2(&bytes).write_raw(MAX_RAW_BYTES).unwrap();
    let parsed = RawM2::parse(&output, Limits::default()).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(parsed.index(2), Some(u32::from(u16::MAX)));
    assert_eq!(parsed.vertex_count(), 65_537);
}

#[test]
fn public_error_display_is_a_static_code() {
    for error in [
        Error::MissingMd20,
        Error::RawExtent,
        Error::NonFinite,
        Error::Index,
    ] {
        assert_eq!(error.to_string(), error.code());
        assert!(error.code().starts_with("vmap_"));
    }
}
