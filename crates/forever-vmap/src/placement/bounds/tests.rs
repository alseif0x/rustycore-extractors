// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: synthetic bound projection/admission/placement fixtures.
use super::*;

fn bits<const N: usize>(values: [f32; N]) -> [u32; N] {
    values.map(f32::to_bits)
}
fn box_bits() -> [u32; 6] {
    bits([-1., -2., -3., 4., 5., 6.])
}
fn spawn() -> Placement<'static> {
    Placement {
        flags: 6,
        name_set: 239,
        id: u32::MAX,
        position: bits([10., 20., 30.]),
        rotation: [0; 3],
        scale: 2.0_f32.to_bits(),
        bounds: None,
        name: b"Reviewed Model.m2",
    }
}
fn wire(placement: Placement<'_>) -> Vec<u8> {
    super::super::encode(&[placement], Limits::default()).unwrap()
}

#[test]
fn identity_and_translation_have_literal_exact_bounds() {
    assert_eq!(
        transform(box_bits(), [0; 3], [0; 3], 1.0_f32.to_bits()).unwrap(),
        box_bits()
    );
    assert_eq!(
        transform(box_bits(), bits([10., 20., 30.]), [0; 3], 1.0_f32.to_bits()).unwrap(),
        bits([9., 18., 27., 14., 25., 36.])
    );
}

#[test]
fn positive_scale_precedes_translation_and_negative_scale_reorders_extrema() {
    assert_eq!(
        transform(box_bits(), bits([10., 20., 30.]), [0; 3], 2.0_f32.to_bits()).unwrap(),
        bits([8., 16., 24., 18., 30., 42.])
    );
    assert_eq!(
        transform(
            box_bits(),
            bits([10., 20., 30.]),
            [0; 3],
            (-2.0_f32).to_bits()
        )
        .unwrap(),
        bits([2., 10., 18., 12., 24., 36.])
    );
}

#[test]
fn either_zero_scale_collapses_all_corners_to_position() {
    for scale in [0, 0x8000_0000] {
        let output = transform(
            box_bits(),
            bits([1., 2., 3.]),
            bits([23., -47., 81.]),
            scale,
        )
        .unwrap();
        assert_eq!(output, bits([1., 2., 3., 1., 2., 3.]));
    }
}

#[test]
fn original_corner_order_is_explicit_and_complete() {
    assert_eq!(
        CORNERS,
        [
            [0, 1, 5],
            [3, 1, 5],
            [3, 4, 5],
            [0, 4, 5],
            [0, 1, 2],
            [3, 1, 2],
            [3, 4, 2],
            [0, 4, 2]
        ]
    );
    let source = [1, 2, 3, 4, 5, 6];
    assert_eq!(
        CORNERS.map(|corner| corner.map(|i| source[i])),
        [
            [1, 2, 6],
            [4, 2, 6],
            [4, 5, 6],
            [1, 5, 6],
            [1, 2, 3],
            [4, 2, 3],
            [4, 5, 3],
            [1, 5, 3]
        ]
    );
}

#[test]
fn zero_rotations_still_execute_dot_products_and_final_positive_zero_add() {
    // All input coordinates -0, identity multiplications/additions and the
    // final +0 translation normalize each emitted coordinate to positive zero.
    assert_eq!(
        transform([0x8000_0000; 6], [0; 3], [0; 3], 1.0_f32.to_bits()).unwrap(),
        [0; 6]
    );
    // One nonzero axis cannot be discarded when another scale operand is -0.
    assert_eq!(
        transform(bits([0., 0., 0., 1., 2., 3.]), [0; 3], [0; 3], 0x8000_0000).unwrap(),
        [0; 6]
    );
}

#[test]
fn z_y_x_rotation_components_are_mapped_to_source_axes() {
    let point = bits([2., 3., 5., 2., 3., 5.]);
    // Exact quarter turns use the original f32 pi approximation; test which
    // axis remains invariant, without pretending the other values are integers.
    for (rotation, axis, value) in [
        ([0., 90., 0.], 2, 5.),
        ([90., 0., 0.], 1, 3.),
        ([0., 0., 90.], 0, 2.),
    ] {
        let out = transform(point, [0; 3], bits(rotation), 1.0_f32.to_bits()).unwrap();
        assert_eq!(out[axis], f32::to_bits(value));
        assert_eq!(out[axis + 3], f32::to_bits(value));
    }
}

#[test]
fn asymmetric_mixed_rotation_uses_z_times_y_times_x_without_fusion() {
    let m = rotation_matrix([17., 31., 53.]);
    let z = (PI * 31.0_f32) / 180.0_f32;
    let y = (PI * 17.0_f32) / 180.0_f32;
    let x = (PI * 53.0_f32) / 180.0_f32;
    let rz = [
        [z.cos(), -z.sin(), 0.],
        [z.sin(), z.cos(), 0.],
        [0., 0., 1.],
    ];
    let ry = [
        [y.cos(), 0., y.sin()],
        [0., 1., 0.],
        [-y.sin(), 0., y.cos()],
    ];
    let rx = [
        [1., 0., 0.],
        [0., x.cos(), -x.sin()],
        [0., x.sin(), x.cos()],
    ];
    assert_eq!(
        m.map(|row| row.map(f32::to_bits)),
        multiply(&rz, &multiply(&ry, &rx)).map(|row| row.map(f32::to_bits))
    );
    assert_ne!(
        m.map(|row| row.map(f32::to_bits)),
        multiply(&multiply(&rx, &ry), &rz).map(|row| row.map(f32::to_bits))
    );
    let out = transform(
        box_bits(),
        bits([3., 4., 5.]),
        bits([17., 31., 53.]),
        (-2.0_f32).to_bits(),
    )
    .unwrap();
    valid_bounds(&out).unwrap();
}

#[test]
fn new_corner_wins_zero_extrema_before_position_is_added() {
    // The production merge's comparisons have the source's NEW-on-equality
    // behavior; both input zero orders remain ordered valid model bounds.
    for b in [
        [0, 0, 0, 0x8000_0000, 0x8000_0000, 0x8000_0000],
        [0x8000_0000, 0x8000_0000, 0x8000_0000, 0, 0, 0],
    ] {
        let out = transform(b, [0; 3], bits([-0., 0., -0.]), 1.0_f32.to_bits()).unwrap();
        assert_eq!(out, [0; 6]);
    }
}

#[test]
fn complete_adds_only_bounds_and_has_bound_flag_preserving_other_fields() {
    let original = spawn();
    let before = wire(original);
    let completed = complete(original, Some(box_bits()), Limits::default()).unwrap();
    assert_eq!(completed.flags(), 7);
    assert_eq!(completed.name_set(), 239);
    assert_eq!(completed.id(), u32::MAX);
    assert_eq!(completed.name(), original.name());
    assert_eq!(completed.position_bits(), original.position_bits());
    assert_eq!(completed.rotation_bits(), original.rotation_bits());
    assert_eq!(completed.scale_bits(), original.scale_bits());
    assert_eq!(
        completed.bounds_bits(),
        Some(bits([8., 16., 24., 18., 30., 42.]))
    );
    assert_eq!(wire(original), before);
    assert_eq!(wire(completed).len(), before.len() + 24);
}

#[test]
fn existing_bounds_preserve_every_byte_without_model_even_if_supplied_model_invalid() {
    let mut original = spawn();
    original.flags |= 1;
    original.bounds = Some(bits([-7., -6., -5., 4., 3., 2.]));
    let expected = wire(original);
    for supplied in [
        None,
        Some([f32::NAN.to_bits(); 6]),
        Some(bits([9., 9., 9., -9., -9., -9.])),
    ] {
        assert_eq!(
            wire(complete(original, supplied, Limits::default()).unwrap()),
            expected
        );
    }
}

#[test]
fn missing_model_is_distinct_static_error_after_validating_placement() {
    assert_eq!(
        complete(spawn(), None, Limits::default()).err(),
        Some(Error::MissingModel)
    );
    assert_eq!(Error::MissingModel.code(), "placement_bounds_missing_model");
    let mut invalid = spawn();
    invalid.flags = 8;
    assert_eq!(
        complete(invalid, None, Limits::default()).err(),
        Some(Error::Placement(PlacementError::Flags))
    );
}

#[test]
fn completed_wire_must_fit_downward_byte_record_and_name_limits() {
    let s = spawn();
    let complete_bytes = 62 + s.name().len();
    assert!(
        complete(
            s,
            Some(box_bits()),
            Limits {
                max_bytes: complete_bytes,
                ..Limits::default()
            }
        )
        .is_ok()
    );
    for limits in [
        Limits {
            max_bytes: complete_bytes - 1,
            ..Limits::default()
        },
        Limits {
            max_records: 0,
            ..Limits::default()
        },
        Limits {
            max_name_bytes: s.name().len() - 1,
            ..Limits::default()
        },
    ] {
        assert_eq!(
            complete(s, Some(box_bits()), limits).err(),
            Some(Error::Placement(PlacementError::Limit))
        );
    }
}

#[test]
fn existing_bounds_still_validate_reduced_and_raised_limits() {
    let mut s = spawn();
    s.flags |= 1;
    s.bounds = Some(box_bits());
    assert_eq!(
        complete(
            s,
            None,
            Limits {
                max_bytes: 38 + s.name().len(),
                ..Limits::default()
            }
        )
        .err(),
        Some(Error::Placement(PlacementError::Limit))
    );
    assert_eq!(
        complete(
            s,
            None,
            Limits {
                max_records: super::super::MAX_RECORDS + 1,
                ..Limits::default()
            }
        )
        .err(),
        Some(Error::Placement(PlacementError::InvalidLimits))
    );
}

#[test]
fn every_nonfinite_input_kind_and_inverted_model_bounds_reject() {
    for bad in [
        f32::NAN.to_bits(),
        f32::INFINITY.to_bits(),
        f32::NEG_INFINITY.to_bits(),
    ] {
        for axis in 0..6 {
            let mut b = box_bits();
            b[axis] = bad;
            assert_eq!(
                transform(b, [0; 3], [0; 3], 1.0_f32.to_bits()).err(),
                Some(PlacementError::NonFinite)
            );
        }
        for axis in 0..3 {
            let mut v = [0; 3];
            v[axis] = bad;
            assert_eq!(
                transform(box_bits(), v, [0; 3], 1.0_f32.to_bits()).err(),
                Some(PlacementError::NonFinite)
            );
            assert_eq!(
                transform(box_bits(), [0; 3], v, 1.0_f32.to_bits()).err(),
                Some(PlacementError::NonFinite)
            );
        }
        assert_eq!(
            transform(box_bits(), [0; 3], [0; 3], bad).err(),
            Some(PlacementError::NonFinite)
        );
    }
    let mut b = box_bits();
    b[0] = 5.0_f32.to_bits();
    assert_eq!(
        transform(b, [0; 3], [0; 3], 1.0_f32.to_bits()).err(),
        Some(PlacementError::InvertedBounds)
    );
}

#[test]
fn finite_inputs_with_nonfinite_computed_results_reject() {
    assert_eq!(
        transform(box_bits(), [0; 3], [0; 3], f32::MAX.to_bits()).err(),
        Some(PlacementError::NonFinite)
    );
    let b = [f32::MAX.to_bits(); 6];
    assert_eq!(
        transform(b, [f32::MAX.to_bits(); 3], [0; 3], 1.0_f32.to_bits()).err(),
        Some(PlacementError::NonFinite)
    );
    assert_eq!(
        transform(
            box_bits(),
            [0; 3],
            [f32::MAX.to_bits(); 3],
            1.0_f32.to_bits()
        )
        .err(),
        Some(PlacementError::NonFinite)
    );
}

#[test]
fn complete_propagates_model_failure_without_changing_original() {
    let s = spawn();
    let original = wire(s);
    assert_eq!(
        complete(s, Some([f32::NAN.to_bits(); 6]), Limits::default()).err(),
        Some(Error::Placement(PlacementError::NonFinite))
    );
    assert_eq!(wire(s), original);
}
