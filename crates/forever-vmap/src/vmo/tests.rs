// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: independent synthetic RAW/VMO fixtures, no client data.
use super::*;

fn words(output: &mut Vec<u8>, values: impl IntoIterator<Item = u32>) {
    for value in values {
        output.extend_from_slice(&value.to_le_bytes());
    }
}
fn put(bytes: &mut [u8], at: usize, value: u32) {
    bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
}
fn f(values: &[f32]) -> Vec<u32> {
    values.iter().map(|v| v.to_bits()).collect()
}
fn bound() -> [u32; 6] {
    [-10., -10., -10., 10., 10., 10.].map(f32::to_bits)
}
fn vertices() -> Vec<[u32; 3]> {
    vec![
        [0x8000_0000, 0, 0],
        [1.0_f32.to_bits(), 0, 0],
        [0, 1.0_f32.to_bits(), 0],
    ]
}
fn array(
    output: &mut Vec<u8>,
    tag: &[u8; 4],
    count: usize,
    width: usize,
    values: impl IntoIterator<Item = u32>,
) {
    output.extend_from_slice(tag);
    words(output, [4 + (count * width) as u32, count as u32]);
    words(output, values);
}
fn raw_group(
    points: &[[u32; 3]],
    indices: &[u32],
    bounds: [u32; 6],
    flags: u32,
    id: u32,
    liquid_flags: u32,
    liquid: &[u8],
) -> Vec<u8> {
    let mut out = Vec::new();
    words(&mut out, [flags, id]);
    words(&mut out, bounds);
    words(&mut out, [liquid_flags]);
    array(&mut out, b"GRP ", 1, 4, [3]);
    array(&mut out, b"INDX", indices.len(), 2, indices.iter().copied());
    array(
        &mut out,
        b"VERT",
        points.len(),
        12,
        points.iter().flatten().copied(),
    );
    if liquid_flags != 0 {
        out.extend_from_slice(b"LIQU");
        words(&mut out, [liquid.len() as u32]);
        out.extend_from_slice(liquid);
    }
    out
}
fn raw_wmo(groups: &[Vec<u8>], triangles: u32) -> Vec<u8> {
    let mut out = crate::RAW_MAGIC.to_vec();
    words(&mut out, [triangles, groups.len() as u32, 123, 0]);
    for group in groups {
        out.extend_from_slice(group);
    }
    out
}
fn simple_raw_m2() -> Vec<u8> {
    let mut out = crate::RAW_MAGIC.to_vec();
    words(&mut out, [3, 1, 0, 1]);
    out.extend(raw_group(&vertices(), &[2, 0, 1], bound(), 0, 0, 0, &[]));
    out
}
fn m2(bytes: &[u8], limits: Limits) -> Result<Model, Error> {
    convert_m2(
        &crate::RawM2::parse(bytes, crate::Limits::default()).unwrap(),
        limits,
    )
}
fn wmo(bytes: &[u8], limits: Limits) -> Result<Model, Error> {
    convert_wmo(
        &crate::wmo::raw::RawWmo::parse(bytes, crate::wmo::raw::ConversionLimits::default())
            .unwrap(),
        limits,
    )
}
fn leaf(bounds: [u32; 6], count: u32) -> Vec<u8> {
    let mut out = Vec::new();
    words(&mut out, bounds);
    words(&mut out, [3, 0xc000_0000, count, 0, count]);
    words(&mut out, 0..count);
    out
}
fn header(flags: u32, root: u32) -> Vec<u8> {
    let mut out = MAGIC.to_vec();
    out.extend_from_slice(b"WMOD");
    words(&mut out, [8, flags, root]);
    out
}
fn single_golden(
    flags: u32,
    root: u32,
    id: u32,
    groupflags: u32,
    groupbounds: [u32; 6],
) -> Vec<u8> {
    let mut out = header(flags, root);
    out.extend_from_slice(b"GMOD");
    words(&mut out, [1]);
    words(&mut out, groupbounds);
    words(&mut out, [groupflags, id]);
    array(
        &mut out,
        b"VERT",
        3,
        12,
        vertices().iter().flatten().copied(),
    );
    array(&mut out, b"TRIM", 1, 12, [2, 0, 1]);
    out.extend_from_slice(b"MBIH");
    // Triangle order 2,0,1: x low +0 from the first point, then -0,
    // then +1; y low +0, high +1. No new coordinate/winding transform.
    out.extend(leaf(
        [0x8000_0000, 0, 0, 1.0_f32.to_bits(), 1.0_f32.to_bits(), 0],
        1,
    ));
    out.extend_from_slice(b"LIQU");
    words(&mut out, [0]);
    out.extend_from_slice(b"GBIH");
    out.extend(leaf(groupbounds, 1));
    out
}
fn flat(kind: u32) -> Vec<u8> {
    kind.to_le_bytes().to_vec()
}
fn surface() -> Vec<u8> {
    let mut out = Vec::new();
    words(
        &mut out,
        [
            0xfedc_ba98,
            2,
            3,
            1,
            2,
            0x8000_0000,
            1.0_f32.to_bits(),
            2.0_f32.to_bits(),
        ],
    );
    out.extend_from_slice(&(-7i16).to_le_bytes());
    words(&mut out, f(&[-0., 1., 2., 3., 4., 5.]));
    out.extend_from_slice(&[0x8f, 0xff]);
    out
}
fn tag_at(bytes: &[u8], tag: &[u8; 4]) -> usize {
    bytes.windows(4).position(|p| p == tag).unwrap()
}

#[test]
fn m2_exact_complete_literal_wire_and_no_transform() {
    let raw = simple_raw_m2();
    let model = m2(&raw, Limits::default()).unwrap();
    assert_eq!(model.bytes(), single_golden(1, 0, 0, 0, bound()));
    let view = View::parse(model.bytes(), Limits::default()).unwrap();
    assert_eq!(
        (
            view.flags(),
            view.root_wmo_id(),
            view.stats().groups,
            view.stats().triangles
        ),
        (1, 0, 1, 1)
    );
    assert_eq!(view.groups()[0].triangle(0), Some([2, 0, 1]));
    assert_eq!(view.groups()[0].vertex_bits(0), Some(vertices()[0]));
    assert_eq!(
        view.groups()[0]
            .mesh_bih()
            .unwrap()
            .objects()
            .collect::<Vec<_>>(),
        [0]
    );
}

#[test]
fn wmo_preserves_root_group_fields_and_declared_bounds_outside_geometry() {
    let raw = raw_wmo(
        &[raw_group(
            &vertices(),
            &[2, 0, 1],
            [0; 6],
            0x1234_5678,
            0xffff_fffe,
            0,
            &[],
        )],
        1,
    );
    let model = wmo(&raw, Limits::default()).unwrap();
    assert_eq!(
        model.bytes(),
        single_golden(0, 123, 0xffff_fffe, 0x1234_5678, [0; 6])
    );
    let view = View::parse(model.bytes(), Limits::default()).unwrap();
    assert_eq!(view.groups()[0].bounds_bits(), [0; 6]);
    assert_eq!(view.group_bih().unwrap().bounds_bits(), [0; 6]);
    assert_eq!(view.groups()[0].flags(), 0x1234_5678);
}

#[test]
fn surface_liquid_preserves_type_corner_heights_flags_and_discards_material() {
    for flags in [1, 3] {
        let raw = raw_wmo(
            &[raw_group(
                &vertices(),
                &[0, 1, 2],
                bound(),
                0,
                1,
                flags,
                &surface(),
            )],
            1,
        );
        let model = wmo(&raw, Limits::default()).unwrap();
        let view = View::parse(model.bytes(), Limits::default()).unwrap();
        let liquid = view.groups()[0].liquid().unwrap();
        assert_eq!(liquid.dimensions(), [1, 2]);
        assert_eq!(liquid.type_id(), 0xfedc_ba98);
        assert_eq!(
            liquid.corner_bits(),
            [0x8000_0000, 1.0_f32.to_bits(), 2.0_f32.to_bits()]
        );
        assert_eq!(liquid.height_count(), 6);
        assert_eq!(liquid.height_bits(0), Some(0x8000_0000));
        assert_eq!(liquid.height_bits(5), Some(5.0_f32.to_bits()));
        assert_eq!(liquid.tiles(), [0x8f, 0xff]);
        let at = tag_at(model.bytes(), b"LIQU");
        assert_eq!(word(model.bytes(), at + 4).unwrap(), 50);
    }
}

#[test]
fn flat_liquid_uses_high_z_and_positive_zero_corner_even_for_signed_zero() {
    let mut b = bound();
    b[2] = (-1.0_f32).to_bits();
    b[5] = 0x8000_0000;
    let raw = raw_wmo(
        &[raw_group(
            &vertices(),
            &[0, 1, 2],
            b,
            0,
            1,
            2,
            &flat(u32::MAX),
        )],
        1,
    );
    let model = wmo(&raw, Limits::default()).unwrap();
    let view = View::parse(model.bytes(), Limits::default()).unwrap();
    let liquid = view.groups()[0].liquid().unwrap();
    assert_eq!(liquid.dimensions(), [0, 0]);
    assert_eq!(liquid.corner_bits(), [0; 3]);
    assert_eq!(liquid.type_id(), u32::MAX);
    assert_eq!(liquid.height_bits(0), Some(0x8000_0000));
    assert!(liquid.tiles().is_empty());
}

#[test]
fn zero_vertex_groups_end_after_vert_and_drop_even_valid_raw_liquids() {
    let groups = [
        raw_group(&[], &[], bound(), 7, 11, 1, &surface()),
        raw_group(&vertices(), &[0, 1, 2], bound(), 8, 12, 0, &[]),
        raw_group(&[], &[], bound(), 9, 13, 2, &flat(4)),
    ];
    let model = wmo(&raw_wmo(&groups, 1), Limits::default()).unwrap();
    let view = View::parse(model.bytes(), Limits::default()).unwrap();
    assert_eq!(view.groups().len(), 3);
    assert_eq!(view.groups()[0].group_wmo_id(), 11);
    assert_eq!(view.groups()[2].group_wmo_id(), 13);
    for index in [0, 2] {
        let g = &view.groups()[index];
        assert_eq!(g.vertex_count(), 0);
        assert!(g.mesh_bih().is_none());
        assert!(g.liquid().is_none());
    }
    assert_eq!(&model.bytes()[64..68], b"VERT");
    assert_eq!(view.stats().liquid_groups, 0);
    assert_eq!(view.group_bih().unwrap().stats().primitive_count, 3);
}

#[test]
fn odd_surface_payload_and_group_order_are_preserved() {
    let mut liq = surface();
    put(&mut liq, 8, 2);
    put(&mut liq, 16, 1);
    liq.truncate(34 + 16);
    liq.push(0x81);
    let raw = raw_wmo(
        &[
            raw_group(&vertices(), &[0, 1, 2], bound(), 0, 10, 1, &liq),
            raw_group(&vertices(), &[2, 1, 0], bound(), 0, 20, 0, &[]),
        ],
        2,
    );
    let model = wmo(&raw, Limits::default()).unwrap();
    let view = View::parse(model.bytes(), Limits::default()).unwrap();
    assert_eq!(view.groups()[1].group_wmo_id(), 20);
    assert_eq!(view.groups()[1].triangle(0), Some([2, 1, 0]));
    assert_eq!(view.groups()[0].liquid().unwrap().tiles(), [0x81]);
}

#[test]
fn empty_header_is_complete_only_without_any_gmod_or_gbih() {
    for flags in [0, 1] {
        let bytes = header(flags, 99);
        let view = View::parse(
            &bytes,
            Limits {
                max_groups: 0,
                max_memory_bytes: 0,
                ..Limits::default()
            },
        )
        .unwrap();
        assert!(view.groups().is_empty());
        assert!(view.group_bih().is_none());
    }
    let mut bytes = header(0, 0);
    bytes.extend_from_slice(b"GMOD");
    words(&mut bytes, [0]);
    assert!(View::parse(&bytes, Limits::default()).is_err());
}

#[test]
fn reader_accepts_nonempty_vertices_with_empty_triangle_bih() {
    let mut bytes = header(0, 0);
    bytes.extend_from_slice(b"GMOD");
    words(&mut bytes, [1]);
    words(&mut bytes, bound());
    words(&mut bytes, [0, 1]);
    array(&mut bytes, b"VERT", 1, 12, [0, 0, 0]);
    array(&mut bytes, b"TRIM", 0, 12, []);
    bytes.extend_from_slice(b"MBIH");
    bytes.extend(leaf([0x7fc0_0000; 6], 0));
    bytes.extend_from_slice(b"LIQU");
    words(&mut bytes, [0]);
    bytes.extend_from_slice(b"GBIH");
    bytes.extend(leaf(bound(), 1));
    let view = View::parse(&bytes, Limits::default()).unwrap();
    assert_eq!(view.groups()[0].triangle_count(), 0);
    assert_eq!(
        view.groups()[0].mesh_bih().unwrap().stats().primitive_count,
        0
    );
}

#[test]
fn strict_eof_rejects_every_truncation_and_tail() {
    let model = m2(&simple_raw_m2(), Limits::default()).unwrap();
    for end in 0..model.bytes().len() {
        if end != 24 {
            assert!(
                View::parse(&model.bytes()[..end], Limits::default()).is_err(),
                "{end}"
            );
        }
    }
    // Header-only is a valid different model, so its 24-byte prefix is admitted.
    let mut bytes = model.into_bytes();
    bytes.push(0);
    assert!(View::parse(&bytes, Limits::default()).is_err());
}

#[test]
fn sizes_tags_model_flags_and_indices_are_checked() {
    let original = single_golden(1, 0, 0, 0, bound());
    for (at, value) in [(12, 7), (16, 2), (28, 0), (68, 999), (72, 0)] {
        let mut bytes = original.clone();
        put(&mut bytes, at, value);
        assert!(View::parse(&bytes, Limits::default()).is_err(), "{at}");
    }
    let mut bytes = original.clone();
    let trim = tag_at(&bytes, b"TRIM");
    put(&mut bytes, trim + 12, 3);
    assert_eq!(
        View::parse(&bytes, Limits::default()).err(),
        Some(Error::Index)
    );
    for tag in [b"VERT", b"TRIM", b"MBIH", b"LIQU", b"GBIH"] {
        let mut bytes = original.clone();
        let at = tag_at(&bytes, tag);
        bytes[at] = b'!';
        assert!(View::parse(&bytes, Limits::default()).is_err());
    }
}

#[test]
fn finite_geometry_and_bounds_are_checked() {
    let original = single_golden(1, 0, 0, 0, bound());
    for at in [32, 76] {
        let mut bytes = original.clone();
        put(&mut bytes, at, f32::NAN.to_bits());
        assert!(View::parse(&bytes, Limits::default()).is_err());
    }
    let mut bytes = original;
    put(&mut bytes, 32, 11.0_f32.to_bits());
    assert!(View::parse(&bytes, Limits::default()).is_err());
}

#[test]
fn embedded_bih_objects_and_ranges_are_validated() {
    let original = single_golden(1, 0, 0, 0, bound());
    let at = tag_at(&original, b"MBIH") + 4;
    for (offset, value) in [(24, 0), (28, 0xc000_0001), (32, 2), (40, 0), (44, 1)] {
        let mut bytes = original.clone();
        put(&mut bytes, at + offset, value);
        assert!(View::parse(&bytes, Limits::default()).is_err());
    }
}

#[test]
fn mesh_and_group_bih_bounds_must_contain_referenced_primitives() {
    let original = single_golden(1, 0, 0, 0, bound());
    let mut mesh = original.clone();
    let at = tag_at(&mesh, b"MBIH") + 4;
    put(&mut mesh, at + 12, 0);
    assert_eq!(
        View::parse(&mesh, Limits::default()).err(),
        Some(Error::Containment)
    );
    let mut group = original;
    let at = tag_at(&group, b"GBIH") + 4;
    put(&mut group, at + 12, 0);
    assert_eq!(
        View::parse(&group, Limits::default()).err(),
        Some(Error::Containment)
    );
}

#[test]
fn conservative_noncanonical_bih_bounds_are_valid_not_constructor_proof() {
    let mut bytes = single_golden(1, 0, 0, 0, bound());
    let at = tag_at(&bytes, b"MBIH") + 4;
    put(&mut bytes, at, (-1.0_f32).to_bits());
    put(&mut bytes, at + 12, 2.0_f32.to_bits());
    assert!(View::parse(&bytes, Limits::default()).is_ok());
}

#[test]
fn interior_mesh_clips_and_permutation_bind_to_triangles() {
    let points: Vec<_> = [0.0_f32, 10., 20., 30.]
        .map(|x| [x.to_bits(), 0, 0])
        .into_iter()
        .collect();
    let raw = raw_wmo(
        &[raw_group(
            &points,
            &[0, 0, 0, 1, 1, 1, 2, 2, 2, 3, 3, 3],
            bound(),
            0,
            1,
            0,
            &[],
        )],
        4,
    );
    let model = wmo(&raw, Limits::default()).unwrap();
    let at = tag_at(model.bytes(), b"MBIH") + 4;
    assert_eq!(word(model.bytes(), at + 28).unwrap() >> 30, 0);
    for (offset, value) in [(32, 5.0_f32.to_bits()), (36, 25.0_f32.to_bits())] {
        let mut bytes = model.bytes().to_vec();
        put(&mut bytes, at + offset, value);
        assert_eq!(
            View::parse(&bytes, Limits::default()).err(),
            Some(Error::Containment)
        );
    }
    let mut bytes = model.into_bytes();
    let object_start = at + 32 + word(&bytes, at + 24).unwrap() as usize * 4;
    for (index, id) in [3, 2, 1, 0].into_iter().enumerate() {
        put(&mut bytes, object_start + 4 * index, id);
    }
    assert_eq!(
        View::parse(&bytes, Limits::default()).err(),
        Some(Error::Containment)
    );
}

#[test]
fn group_bih_clips_bind_even_empty_groups_to_their_declared_bounds() {
    let low = [0; 6];
    let high = [10.0_f32.to_bits(), 0, 0, 10.0_f32.to_bits(), 0, 0];
    let raw = raw_wmo(
        &[
            raw_group(&[], &[], low, 0, 1, 0, &[]),
            raw_group(&vertices(), &[0, 1, 2], high, 0, 2, 0, &[]),
        ],
        1,
    );
    let model = wmo(&raw, Limits::default()).unwrap();
    let mut bytes = model.into_bytes();
    let at = tag_at(&bytes, b"GBIH") + 4;
    // Right primitive's low X is 10. A conservative clip of 9 remains valid.
    put(&mut bytes, at + 36, 9.0_f32.to_bits());
    assert!(View::parse(&bytes, Limits::default()).is_ok());
    // Assign the physical group bounds to the opposite leaves while preserving
    // a perfect object permutation and all structural/range checks.
    let objects = at + 32 + word(&bytes, at + 24).unwrap() as usize * 4;
    put(&mut bytes, objects, 1);
    put(&mut bytes, objects + 4, 0);
    assert_eq!(
        View::parse(&bytes, Limits::default()).err(),
        Some(Error::Containment)
    );
}

#[test]
fn malformed_liquid_dimensions_extent_and_nonfinite_values_reject() {
    let model = wmo(
        &raw_wmo(
            &[raw_group(
                &vertices(),
                &[0, 1, 2],
                bound(),
                0,
                1,
                1,
                &surface(),
            )],
            1,
        ),
        Limits::default(),
    )
    .unwrap();
    let original = model.bytes();
    let at = tag_at(original, b"LIQU");
    for (offset, value) in [
        (4, 49),
        (8, 0),
        (8, u32::MAX),
        (16, f32::NAN.to_bits()),
        (32, f32::INFINITY.to_bits()),
    ] {
        let mut bytes = original.to_vec();
        put(&mut bytes, at + offset, value);
        assert!(View::parse(&bytes, Limits::default()).is_err());
    }
}

#[test]
fn source_reader_limits_are_rechecked_after_larger_initial_parse() {
    let raw = simple_raw_m2();
    for limits in [
        Limits {
            max_raw_bytes: raw.len() - 1,
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
            max_groups: 0,
            ..Limits::default()
        },
    ] {
        assert!(m2(&raw, limits).is_err());
    }
    let raw = raw_wmo(
        &[raw_group(
            &vertices(),
            &[0, 1, 2],
            bound(),
            0,
            1,
            1,
            &surface(),
        )],
        1,
    );
    for limits in [
        Limits {
            max_liquid_heights: 5,
            ..Limits::default()
        },
        Limits {
            max_liquid_tiles: 1,
            ..Limits::default()
        },
        Limits {
            max_groups: 0,
            ..Limits::default()
        },
    ] {
        assert!(wmo(&raw, limits).is_err());
    }
}

#[test]
fn byte_memory_and_cumulative_work_caps_include_both_tree_levels() {
    let raw = simple_raw_m2();
    let baseline = m2(&raw, Limits::default()).unwrap();
    let stats = baseline.stats();
    assert!(
        m2(
            &raw,
            Limits {
                max_bytes: baseline.bytes().len(),
                ..Limits::default()
            }
        )
        .is_ok()
    );
    assert!(
        m2(
            &raw,
            Limits {
                max_bytes: baseline.bytes().len() - 1,
                ..Limits::default()
            }
        )
        .is_err()
    );
    assert!(
        m2(
            &raw,
            Limits {
                max_memory_bytes: stats.peak_bytes,
                ..Limits::default()
            }
        )
        .is_ok()
    );
    assert!(
        m2(
            &raw,
            Limits {
                max_memory_bytes: stats.peak_bytes - 1,
                ..Limits::default()
            }
        )
        .is_err()
    );
    assert_eq!(stats.build_work, 2);
    assert!(
        m2(
            &raw,
            Limits {
                max_work: 2,
                ..Limits::default()
            }
        )
        .is_ok()
    );
    assert!(
        m2(
            &raw,
            Limits {
                max_work: 1,
                ..Limits::default()
            }
        )
        .is_err()
    );
    assert!(
        m2(
            &raw,
            Limits {
                bih: bih::Limits {
                    max_primitives: 0,
                    ..bih::Limits::default()
                },
                ..Limits::default()
            }
        )
        .is_err()
    );
}

#[test]
fn reader_descriptor_and_tree_scratch_caps_are_accounted() {
    let model = m2(&simple_raw_m2(), Limits::default()).unwrap();
    let stats = View::parse(model.bytes(), Limits::default())
        .unwrap()
        .stats();
    assert!(stats.resident_bytes >= size_of::<Group<'_>>());
    assert!(stats.peak_bytes > stats.resident_bytes);
    assert!(
        View::parse(
            model.bytes(),
            Limits {
                max_memory_bytes: stats.peak_bytes,
                ..Limits::default()
            }
        )
        .is_ok()
    );
    assert!(
        View::parse(
            model.bytes(),
            Limits {
                max_memory_bytes: stats.peak_bytes - 1,
                ..Limits::default()
            }
        )
        .is_err()
    );
    assert!(
        View::parse(
            model.bytes(),
            Limits {
                max_groups: 0,
                ..Limits::default()
            }
        )
        .is_err()
    );
}

#[test]
fn raised_limits_reject_before_empty_header_or_conversion() {
    let raw = simple_raw_m2();
    let bytes = header(0, 0);
    for limits in [
        Limits {
            max_bytes: MAX_BYTES + 1,
            ..Limits::default()
        },
        Limits {
            max_memory_bytes: MAX_MEMORY_BYTES + 1,
            ..Limits::default()
        },
        Limits {
            max_groups: MAX_GROUPS + 1,
            ..Limits::default()
        },
        Limits {
            bih: bih::Limits {
                max_depth: bih::MAX_DEPTH + 1,
                ..bih::Limits::default()
            },
            ..Limits::default()
        },
    ] {
        assert_eq!(
            View::parse(&bytes, limits).err(),
            Some(Error::InvalidLimits)
        );
        assert_eq!(m2(&raw, limits).err(), Some(Error::InvalidLimits));
    }
}

#[test]
fn invalid_raw_liquid_cannot_be_hidden_in_empty_group_before_conversion() {
    let mut raw = raw_wmo(
        &[
            raw_group(&[], &[], bound(), 0, 1, 1, &surface()),
            raw_group(&vertices(), &[0, 1, 2], bound(), 0, 2, 0, &[]),
        ],
        1,
    );
    let at = tag_at(&raw, b"LIQU");
    put(&mut raw, at + 8 + 34, f32::NAN.to_bits());
    assert!(
        crate::wmo::raw::RawWmo::parse(&raw, crate::wmo::raw::ConversionLimits::default()).is_err()
    );
}
