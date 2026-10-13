// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: synthetic ray/instance geometry and operation tests.
use super::*;
use crate::{RawM2, placement, query::Limits, wmo::raw};
fn words(out: &mut Vec<u8>, values: impl IntoIterator<Item = u32>) {
    for value in values {
        out.extend_from_slice(&value.to_le_bytes());
    }
}
fn array(
    out: &mut Vec<u8>,
    tag: &[u8; 4],
    count: usize,
    width: usize,
    values: impl IntoIterator<Item = u32>,
) {
    out.extend_from_slice(tag);
    words(out, [4 + (count * width) as u32, count as u32]);
    words(out, values);
}
fn triangle(z: f32) -> [[f32; 3]; 3] {
    [[0., 0., z], [1., 0., z], [0., 1., z]]
}
fn group(triangles: &[[[f32; 3]; 3]], bound: [f32; 6]) -> Vec<u8> {
    let mut out = Vec::new();
    words(&mut out, [0, 7]);
    words(&mut out, bound.map(f32::to_bits));
    words(&mut out, [0]);
    array(&mut out, b"GRP ", 1, 4, [(triangles.len() * 3) as u32]);
    array(
        &mut out,
        b"INDX",
        triangles.len() * 3,
        2,
        0..(triangles.len() * 3) as u32,
    );
    array(
        &mut out,
        b"VERT",
        triangles.len() * 3,
        12,
        triangles.iter().flatten().flatten().map(|v| v.to_bits()),
    );
    out
}
fn converted(groups: &[Vec<u8>], triangles: usize, m2: bool) -> vmo::Model {
    let mut bytes = crate::RAW_MAGIC.to_vec();
    words(
        &mut bytes,
        [
            if m2 {
                (triangles * 3) as u32
            } else {
                triangles as u32
            },
            groups.len() as u32,
            0,
            u32::from(m2),
        ],
    );
    for g in groups {
        bytes.extend(g);
    }
    if m2 {
        // The M2 writer fixes group flags/ID to zero; the generic synthetic
        // WMO group above deliberately carries ID 7. Preserve each domain.
        bytes[28..32].copy_from_slice(&0u32.to_le_bytes());
        vmo::convert_m2(
            &RawM2::parse(&bytes, crate::Limits::default()).unwrap(),
            vmo::Limits::default(),
        )
        .unwrap()
    } else {
        vmo::convert_wmo(
            &raw::RawWmo::parse(&bytes, raw::ConversionLimits::default()).unwrap(),
            vmo::Limits::default(),
        )
        .unwrap()
    }
}
fn one(m2: bool) -> vmo::Model {
    converted(
        &[group(&[triangle(0.)], [-1., -1., -1., 2., 2., 2.])],
        1,
        m2,
    )
}
fn budget() -> Budget {
    Budget::new(Limits::default()).unwrap()
}
fn down(z: f32) -> Ray {
    Ray::new([0.25, 0.25, z], [0., 0., -1.]).unwrap()
}
fn query(model: &vmo::Model, ray: Ray, max: f32, mode: Mode) -> Result<Option<Hit>, Error> {
    let model = vmo::View::parse(model.bytes(), vmo::Limits::default()).unwrap();
    intersect(&model, &ray, max, mode, false, &mut budget())
}
fn spawn(
    position: [f32; 3],
    rotation: [f32; 3],
    scale: f32,
    flags: u8,
    bound: [f32; 6],
) -> Vec<u8> {
    let mut out = vec![flags, 0];
    words(&mut out, [17]);
    words(&mut out, position.map(f32::to_bits));
    words(&mut out, rotation.map(f32::to_bits));
    words(&mut out, [scale.to_bits()]);
    if flags & 1 != 0 {
        words(&mut out, bound.map(f32::to_bits));
    }
    words(&mut out, [1]);
    out.push(b'x');
    out
}
fn instance_from(bytes: &[u8]) -> Result<Instance, Error> {
    let view = placement::DirBin::parse(bytes, placement::Limits::default()).unwrap();
    Instance::new(&view.records().next().unwrap())
}
fn bound() -> [f32; 6] {
    [-100., -100., -100., 100., 100., 100.]
}
#[test]
fn interior_two_sided_hit_keeps_parameter_distance() {
    let m = one(true);
    for ray in [down(1.), Ray::new([0.25, 0.25, -1.], [0., 0., 1.]).unwrap()] {
        let h = query(&m, ray, 2., Mode::Nearest).unwrap().unwrap();
        assert_eq!(h.distance_bits(), 1f32.to_bits());
        assert_eq!(h.group_index(), 0);
        assert_eq!(h.triangle_index(), 0);
    }
}
#[test]
fn direction_is_not_normalized() {
    let m = one(true);
    let ray = Ray::new([0.25, 0.25, 2.], [0., 0., -2.]).unwrap();
    assert_eq!(
        query(&m, ray, 2., Mode::Nearest)
            .unwrap()
            .unwrap()
            .distance_bits(),
        1f32.to_bits()
    );
}
#[test]
fn inclusive_triangle_vertices_and_edges() {
    let m = one(false);
    for [x, y] in [[0., 0.], [1., 0.], [0., 1.], [0.5, 0.5], [0.5, 0.]] {
        assert!(
            query(
                &m,
                Ray::new([x, y, 1.], [0., 0., -1.]).unwrap(),
                2.,
                Mode::Nearest
            )
            .unwrap()
            .is_some()
        );
    }
    assert!(
        query(
            &m,
            Ray::new([0.6, 0.6, 1.], [0., 0., -1.]).unwrap(),
            2.,
            Mode::Nearest
        )
        .unwrap()
        .is_none()
    );
}
#[test]
fn origin_and_exact_endpoint_are_excluded() {
    let m = one(false);
    for (z, max) in [(0., 1.), (1., 1.), (1., 0.), (1., -0.)] {
        assert!(query(&m, down(z), max, Mode::Nearest).unwrap().is_none());
    }
    assert!(
        query(
            &m,
            down(1.),
            f32::from_bits(1f32.to_bits() + 1),
            Mode::Nearest
        )
        .unwrap()
        .is_some()
    );
}
#[test]
fn determinant_strict_threshold_not_less_or_equal() {
    let ray = Ray::new([0., 0., 1.], [0., 0., -1.]).unwrap();
    let at = [[0., 0., 0.], [1., 0., 0.], [0., 1e-5, 0.]];
    assert!(intersect_triangle(at, &ray, 2.).unwrap().is_some());
    let below = [
        at[0],
        at[1],
        [0., f32::from_bits(1e-5_f32.to_bits() - 1), 0.],
    ];
    assert!(intersect_triangle(below, &ray, 2.).unwrap().is_none());
}
#[test]
fn parallel_and_degenerate_triangles_miss() {
    assert!(
        intersect_triangle(
            triangle(0.),
            &Ray::new([0.1, 0.1, 1.], [1., 0., 0.]).unwrap(),
            10.
        )
        .unwrap()
        .is_none()
    );
    assert!(
        intersect_triangle([[0.; 3]; 3], &down(1.), 10.)
            .unwrap()
            .is_none()
    );
}
#[test]
fn nearest_and_any_follow_leaf_order_not_sorted_distance() {
    let m = converted(
        &[group(&[triangle(0.), triangle(1.), triangle(2.)], bound())],
        3,
        false,
    );
    let near = query(&m, down(3.), 4., Mode::Nearest).unwrap().unwrap();
    assert_eq!(near.distance_bits(), 1f32.to_bits());
    assert_eq!(near.triangle_index(), 2);
    let any = query(&m, down(3.), 4., Mode::Any).unwrap().unwrap();
    assert_eq!(any.distance_bits(), 3f32.to_bits());
    assert_eq!(any.triangle_index(), 0);
}
#[test]
fn equal_distance_keeps_first_triangle() {
    let m = converted(&[group(&[triangle(0.), triangle(0.)], bound())], 2, false);
    assert_eq!(
        query(&m, down(1.), 2., Mode::Nearest)
            .unwrap()
            .unwrap()
            .triangle_index(),
        0
    );
}
#[test]
fn single_group_bypasses_declared_group_bound_entirely() {
    let m = converted(&[group(&[triangle(0.)], [100.; 6])], 1, false);
    assert!(query(&m, down(1.), 2., Mode::Nearest).unwrap().is_some());
    let many = converted(
        &[
            group(&[triangle(0.)], [100.; 6]),
            group(&[triangle(0.)], [100.; 6]),
        ],
        2,
        false,
    );
    assert!(query(&many, down(1.), 2., Mode::Nearest).unwrap().is_none());
}
#[test]
fn multiple_groups_visit_shared_budget_and_find_closest_group() {
    let m = converted(
        &[
            group(&[triangle(0.)], [-1., -1., -1., 2., 2., 0.]),
            group(&[triangle(2.)], [-1., -1., 2., 2., 2., 3.]),
        ],
        2,
        false,
    );
    let model = vmo::View::parse(m.bytes(), vmo::Limits::default()).unwrap();
    let mut work = budget();
    let hit = intersect(&model, &down(3.), 4., Mode::Nearest, false, &mut work)
        .unwrap()
        .unwrap();
    assert_eq!(hit.distance_bits(), 1f32.to_bits());
    assert_eq!(hit.group_index(), 1);
    assert!(work.stats().node_visits >= 2);
}
#[test]
fn ignored_m2_spends_no_tree_or_triangle_budget() {
    let m = one(true);
    let model = vmo::View::parse(m.bytes(), vmo::Limits::default()).unwrap();
    let mut work = Budget::new(Limits {
        max_node_visits: 0,
        max_triangle_tests: 0,
    })
    .unwrap();
    assert!(
        intersect(&model, &down(1.), 2., Mode::Nearest, true, &mut work)
            .unwrap()
            .is_none()
    );
    assert_eq!(work.stats().node_visits, 0);
    assert_eq!(work.stats().triangle_tests, 0);
    let m = one(false);
    let model = vmo::View::parse(m.bytes(), vmo::Limits::default()).unwrap();
    assert!(
        intersect(&model, &down(1.), 2., Mode::Nearest, true, &mut budget())
            .unwrap()
            .is_some()
    );
}
#[test]
fn empty_model_and_empty_group_are_misses() {
    let mut bytes = b"VMAP_4.EWMOD".to_vec();
    words(&mut bytes, [8, 0, 0]);
    let model = vmo::View::parse(&bytes, vmo::Limits::default()).unwrap();
    assert!(
        intersect(&model, &down(1.), 2., Mode::Nearest, false, &mut budget())
            .unwrap()
            .is_none()
    );
    let m = converted(
        &[group(&[], bound()), group(&[triangle(0.)], bound())],
        1,
        false,
    );
    assert!(query(&m, down(1.), 2., Mode::Nearest).unwrap().is_some());
}
#[test]
fn public_infinite_distance_allowed_negative_and_nan_rejected() {
    let m = one(true);
    assert!(
        query(&m, down(1.), f32::INFINITY, Mode::Nearest)
            .unwrap()
            .is_some()
    );
    for max in [f32::NAN, f32::NEG_INFINITY, -1.] {
        assert_eq!(
            query(&m, down(1.), max, Mode::Nearest).err(),
            Some(Error::InvalidDistance)
        );
    }
}
#[test]
fn signed_zero_direction_bits_survive_queries() {
    let ray = Ray::new([0.25, 0.25, 1.], [-0., 0., -1.]).unwrap();
    assert_eq!(ray.direction()[0].to_bits(), 0x8000_0000);
    assert_eq!(
        query(&one(true), ray, 2., Mode::Nearest)
            .unwrap()
            .unwrap()
            .distance_bits(),
        1f32.to_bits()
    );
}
#[test]
fn triangle_and_node_budget_exhaustion_are_errors_not_misses() {
    let m = one(true);
    let model = vmo::View::parse(m.bytes(), vmo::Limits::default()).unwrap();
    for (limits, error) in [
        (
            Limits {
                max_node_visits: 0,
                ..Limits::default()
            },
            Error::NodeLimit,
        ),
        (
            Limits {
                max_triangle_tests: 0,
                ..Limits::default()
            },
            Error::TriangleLimit,
        ),
    ] {
        let mut work = Budget::new(limits).unwrap();
        assert_eq!(
            intersect(&model, &down(1.), 2., Mode::Nearest, false, &mut work).err(),
            Some(error)
        );
    }
}
#[test]
fn same_budget_accumulates_across_public_calls() {
    let m = one(true);
    let model = vmo::View::parse(m.bytes(), vmo::Limits::default()).unwrap();
    let mut work = Budget::new(Limits {
        max_triangle_tests: 1,
        ..Limits::default()
    })
    .unwrap();
    assert!(
        intersect(&model, &down(1.), 2., Mode::Nearest, false, &mut work)
            .unwrap()
            .is_some()
    );
    assert_eq!(
        intersect(&model, &down(1.), 2., Mode::Nearest, false, &mut work).err(),
        Some(Error::TriangleLimit)
    );
    assert_eq!(work.stats().triangle_tests, 1);
}
#[test]
fn translated_scaled_instance_preserves_distance_units() {
    let m = one(true);
    let view = vmo::View::parse(m.bytes(), vmo::Limits::default()).unwrap();
    let instance = instance_from(&spawn([10., 20., 5.], [0.; 3], 2., 1, bound())).unwrap();
    let ray = Ray::new([10.5, 20.5, 9.], [0., 0., -1.]).unwrap();
    let hit = instance
        .intersect(&view, &ray, 5., Mode::Nearest, false, &mut budget())
        .unwrap()
        .unwrap();
    assert_eq!(hit.distance_bits(), 4f32.to_bits());
}
#[test]
fn negative_scale_preserves_native_negative_local_distance_miss() {
    let m = one(true);
    let view = vmo::View::parse(m.bytes(), vmo::Limits::default()).unwrap();
    let instance = instance_from(&spawn([0.; 3], [0.; 3], -1., 1, bound())).unwrap();
    for max in [2., f32::INFINITY] {
        assert!(
            instance
                .intersect(
                    &view,
                    &Ray::new([-0.25, -0.25, -1.], [0., 0., 1.]).unwrap(),
                    max,
                    Mode::Nearest,
                    false,
                    &mut budget()
                )
                .unwrap()
                .is_none()
        );
    }
}
#[test]
fn zero_and_reciprocal_overflow_scales_are_explicit_errors() {
    for scale in [0., -0., f32::from_bits(1), -f32::from_bits(1)] {
        assert_eq!(
            instance_from(&spawn([0.; 3], [0.; 3], scale, 1, bound())).err(),
            Some(Error::InvalidScale)
        );
    }
}
#[test]
fn missing_bounds_and_singular_inverse_are_explicit_errors() {
    assert_eq!(
        instance_from(&spawn([0.; 3], [0.; 3], 1., 0, bound())).err(),
        Some(Error::MissingBounds)
    );
    assert_eq!(invert([[0.; 3]; 3]).err(), Some(Error::SingularTransform));
    assert_eq!(
        invert([[2., 0., 0.], [0., 4., 0.], [0., 0., 8.]]).unwrap(),
        [[0.5, 0., 0.], [0., 0.25, 0.], [0., 0., 0.125]]
    );
}
#[test]
fn instance_box_miss_does_not_enter_bih_and_path_flag_is_not_filter() {
    let m = one(true);
    let view = vmo::View::parse(m.bytes(), vmo::Limits::default()).unwrap();
    let miss = instance_from(&spawn([0.; 3], [0.; 3], 1., 1, [100.; 6])).unwrap();
    let mut work = budget();
    assert!(
        miss.intersect(&view, &down(1.), 2., Mode::Nearest, false, &mut work)
            .unwrap()
            .is_none()
    );
    assert_eq!(work.stats().node_visits, 0);
    let path = instance_from(&spawn([0.; 3], [0.; 3], 1., 5, bound())).unwrap();
    assert!(
        path.intersect(&view, &down(1.), 2., Mode::Nearest, false, &mut budget())
            .unwrap()
            .is_some()
    );
}
#[test]
fn rotated_instance_uses_inverse_zyx_without_normalizing_direction() {
    let m = one(true);
    let view = vmo::View::parse(m.bytes(), vmo::Limits::default()).unwrap();
    let instance = instance_from(&spawn([0.; 3], [0., 90., 0.], 1., 1, bound())).unwrap();
    let ray = Ray::new([-0.25, 0.25, 2.], [0., 0., -2.]).unwrap();
    assert_eq!(
        instance
            .intersect(&view, &ray, 2., Mode::Nearest, false, &mut budget())
            .unwrap()
            .unwrap()
            .distance_bits(),
        1f32.to_bits()
    );
}
#[test]
fn nonfinite_derived_triangle_and_transform_never_become_silent_misses() {
    let huge = [[f32::MAX, 0., 0.], [-f32::MAX, 0., 0.], [0., 1., 0.]];
    assert_eq!(
        intersect_triangle(huge, &down(1.), 2.).err(),
        Some(Error::NonFiniteResult)
    );
    assert_eq!(
        instance_from(&spawn([0.; 3], [f32::MAX, 0., 0.], 1., 1, bound())).err(),
        Some(Error::NonFiniteResult)
    );
}

fn result_key(result: Result<Option<Hit>, Error>) -> Result<Option<(u32, u32, u32)>, Error> {
    result.map(|v| v.map(|h| (h.distance_bits(), h.group_index(), h.triangle_index())))
}

#[test]
fn owned_and_borrowed_model_paths_share_exact_hits_and_budget_counts() {
    let models = [
        one(true),
        one(false),
        converted(
            &[
                group(&[triangle(-2.), triangle(1.)], bound()),
                group(&[triangle(0.)], bound()),
            ],
            3,
            false,
        ),
    ];
    for source in models {
        let borrowed = vmo::View::parse(source.bytes(), vmo::Limits::default()).unwrap();
        let owned = vmo::Owned::parse(source.bytes().to_vec(), vmo::Limits::default()).unwrap();
        let mut a = budget();
        let mut b = budget();
        for mode in [Mode::Nearest, Mode::Any] {
            for ignore in [false, true] {
                for ray in [
                    down(4.),
                    down(-4.),
                    Ray::new([0.25, 0.25, 4.], [-0., 0., -2.]).unwrap(),
                    Ray::new([2., 2., 4.], [0., 0., -1.]).unwrap(),
                ] {
                    for maximum in [0., -0., 1., 4., 100., f32::INFINITY] {
                        assert_eq!(
                            result_key(intersect(&borrowed, &ray, maximum, mode, ignore, &mut a)),
                            result_key(intersect_owned(
                                &owned, &ray, maximum, mode, ignore, &mut b
                            ))
                        );
                        assert_eq!(a.stats(), b.stats());
                    }
                }
            }
        }
    }
}

#[test]
fn owned_instance_and_one_shot_paths_reuse_identical_transform_operations() {
    let source = one(false);
    let borrowed = vmo::View::parse(source.bytes(), vmo::Limits::default()).unwrap();
    let owned = vmo::Owned::parse(source.bytes().to_vec(), vmo::Limits::default()).unwrap();
    for rotation in [[0., 0., 0.], [15., 30., 45.], [-90., 45., 180.]] {
        for scale in [-2., 0.5, 1., 2.] {
            let wire = spawn([1., 2., 3.], rotation, scale, 1, bound());
            let records = placement::DirBin::parse(&wire, placement::Limits::default()).unwrap();
            let placement = records.records().next().unwrap();
            let instance = Instance::new(&placement).unwrap();
            let mut a = budget();
            let mut b = budget();
            for ray in [
                down(10.),
                Ray::new([1.25, 2.25, 10.], [0., -0., -1.]).unwrap(),
            ] {
                for mode in [Mode::Nearest, Mode::Any] {
                    let expected =
                        result_key(instance.intersect(&borrowed, &ray, 100., mode, false, &mut a));
                    assert_eq!(
                        expected,
                        result_key(
                            instance.intersect_owned(&owned, &ray, 100., mode, false, &mut b)
                        )
                    );
                    assert_eq!(a.stats(), b.stats());
                    assert_eq!(
                        expected,
                        result_key(instance_owned(
                            &owned,
                            &placement,
                            &ray,
                            100.,
                            mode,
                            false,
                            &mut budget()
                        ))
                    );
                }
            }
        }
    }
}

#[test]
fn owned_queries_preserve_public_validation_and_budget_failures() {
    let source = one(false);
    let borrowed = vmo::View::parse(source.bytes(), vmo::Limits::default()).unwrap();
    let owned = vmo::Owned::parse(source.bytes().to_vec(), vmo::Limits::default()).unwrap();
    for maximum in [-1., f32::NEG_INFINITY, f32::NAN] {
        assert_eq!(
            result_key(intersect_owned(
                &owned,
                &down(1.),
                maximum,
                Mode::Any,
                false,
                &mut budget()
            )),
            Err(Error::InvalidDistance)
        );
    }
    for limits in [
        Limits {
            max_node_visits: 0,
            ..Limits::default()
        },
        Limits {
            max_triangle_tests: 0,
            ..Limits::default()
        },
        Limits {
            max_node_visits: 1,
            max_triangle_tests: 1,
        },
    ] {
        let mut a = Budget::new(limits).unwrap();
        let mut b = Budget::new(limits).unwrap();
        for _ in 0..2 {
            assert_eq!(
                result_key(intersect(
                    &borrowed,
                    &down(1.),
                    2.,
                    Mode::Nearest,
                    false,
                    &mut a
                )),
                result_key(intersect_owned(
                    &owned,
                    &down(1.),
                    2.,
                    Mode::Nearest,
                    false,
                    &mut b
                ))
            );
            assert_eq!(a.stats(), b.stats());
        }
    }
}

#[test]
fn owned_empty_model_queries_do_not_create_descriptors_or_consume_work() {
    let mut bytes = b"VMAP_4.EWMOD".to_vec();
    words(&mut bytes, [8, 0, 0]);
    let owned = vmo::Owned::parse(bytes, vmo::Limits::default()).unwrap();
    let stats = owned.stats();
    let pointer = owned.bytes().as_ptr();
    let mut budget = Budget::new(Limits {
        max_node_visits: 0,
        max_triangle_tests: 0,
    })
    .unwrap();
    for _ in 0..100 {
        assert!(
            intersect_owned(&owned, &down(1.), 10., Mode::Nearest, false, &mut budget)
                .unwrap()
                .is_none()
        );
        assert_eq!(owned.stats(), stats);
        assert_eq!(owned.bytes().as_ptr(), pointer);
    }
    assert_eq!(budget.stats(), super::super::Stats::default());
}
