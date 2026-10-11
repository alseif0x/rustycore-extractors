// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: synthetic query topology, IEEE and bounded-work cases.
use super::*;
use crate::bih;

const LEAF: u32 = 0xc000_0000;
fn wire(bounds: [f32; 6], words: &[u32], objects: &[u32]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for value in bounds {
        bytes.extend(value.to_bits().to_le_bytes());
    }
    bytes.extend((words.len() as u32).to_le_bytes());
    for value in words {
        bytes.extend(value.to_le_bytes());
    }
    bytes.extend((objects.len() as u32).to_le_bytes());
    for value in objects {
        bytes.extend(value.to_le_bytes());
    }
    bytes
}
fn bounds() -> [f32; 6] {
    [0., 0., 0., 10., 10., 10.]
}
fn leaf() -> Vec<u8> {
    wire(bounds(), &[LEAF, 3, 0], &[2, 0, 1])
}
fn split(left: f32, right: f32) -> Vec<u8> {
    wire(
        bounds(),
        &[
            3,
            left.to_bits(),
            right.to_bits(),
            LEAF,
            1,
            0,
            LEAF | 1,
            1,
            0,
        ],
        &[0, 1],
    )
}
fn view(bytes: &[u8], count: usize) -> bih::View<'_> {
    bih::View::parse(bytes, count, bih::Limits::default()).unwrap()
}
fn budget() -> Budget {
    Budget::new(Limits::default()).unwrap()
}
fn xray() -> Ray {
    Ray::new([-1., 5., 5.], [1., 0., 0.]).unwrap()
}
fn misses(
    tree: &bih::View<'_>,
    ray: &Ray,
    distance: f32,
    limits: Limits,
) -> (Result<bool, Error>, Vec<u32>, Stats, f32) {
    let mut b = Budget::new(limits).unwrap();
    let mut seen = Vec::new();
    let mut distance = distance;
    let result = traverse::ray(
        tree,
        ray,
        &mut distance,
        Mode::Nearest,
        &mut b,
        |id, _, _| {
            seen.push(id);
            Ok(false)
        },
    );
    (result, seen, b.stats(), distance)
}

#[test]
fn ray_preserves_bits_and_does_not_normalize() {
    let ray = Ray::from_bits([0x8000_0000, 0, 0], [2f32.to_bits(), 0, 0x8000_0000]).unwrap();
    assert_eq!(ray.origin().map(f32::to_bits), [0x8000_0000, 0, 0]);
    assert_eq!(
        ray.direction().map(f32::to_bits),
        [2f32.to_bits(), 0, 0x8000_0000]
    );
    assert_eq!(
        ray.inverse_direction().map(f32::to_bits),
        [
            0.5f32.to_bits(),
            f32::INFINITY.to_bits(),
            f32::NEG_INFINITY.to_bits()
        ]
    );
}

#[test]
fn ray_rejects_nonfinite_and_all_signed_zero_directions() {
    for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        assert!(matches!(
            Ray::new([value, 0., 0.], [1., 0., 0.]),
            Err(Error::NonFinite)
        ));
        assert!(matches!(
            Ray::new([0.; 3], [value, 0., 0.]),
            Err(Error::NonFinite)
        ));
    }
    assert!(matches!(
        Ray::new([0.; 3], [0., -0., 0.]),
        Err(Error::ZeroDirection)
    ));
}

#[test]
fn finite_subnormal_direction_can_have_infinite_reciprocal() {
    let ray = Ray::new([0.; 3], [f32::from_bits(1), 0., -0.]).unwrap();
    assert_eq!(ray.inverse_direction()[0], f32::INFINITY);
}

#[test]
fn public_distance_accepts_positive_infinity_and_signed_zero() {
    for distance in [0., -0., 1., f32::MAX, f32::INFINITY] {
        assert_eq!(validate_distance(distance), Ok(()));
    }
    for distance in [-1., f32::NEG_INFINITY, f32::NAN] {
        assert_eq!(validate_distance(distance), Err(Error::InvalidDistance));
    }
}

#[test]
fn budgets_only_reduce_and_zero_is_admitted() {
    assert!(
        Budget::new(Limits {
            max_node_visits: 0,
            max_triangle_tests: 0
        })
        .is_ok()
    );
    assert!(matches!(
        Budget::new(Limits {
            max_node_visits: MAX_NODE_VISITS + 1,
            ..Limits::default()
        }),
        Err(Error::InvalidLimits)
    ));
    assert!(matches!(
        Budget::new(Limits {
            max_triangle_tests: MAX_TRIANGLE_TESTS + 1,
            ..Limits::default()
        }),
        Err(Error::InvalidLimits)
    ));
}

#[test]
fn native_min_max_keep_first_operand_on_ties_and_nan() {
    for function in [math::first_min as fn(f32, f32) -> f32, math::first_max] {
        assert_eq!(function(-0., 0.).to_bits(), (-0f32).to_bits());
        assert_eq!(function(0., -0.).to_bits(), 0);
        let nan = f32::from_bits(0x7fc0_0137);
        assert_eq!(function(nan, 1.).to_bits(), nan.to_bits());
        assert_eq!(function(1., nan), 1.);
    }
}

#[test]
fn direction_fuzziness_uses_double_overload_not_float_epsilon() {
    assert!(!math::fuzzy_nonzero(0.000_000_5));
    assert!(math::fuzzy_nonzero(0.000_000_6));
    assert!(math::fuzzy_nonzero(-0.000_001));
    assert!(!math::fuzzy_nonzero(-0.));
}

#[test]
fn box_candidate_returns_physical_length_even_for_nonunit_ray() {
    let ray = Ray::new([-2., 5., 5.], [2., 0., 0.]).unwrap();
    assert_eq!(math::box_intersection(&ray, bounds()), 2.);
    let inside = Ray::new([0., 5., 5.], [1., 0., 0.]).unwrap();
    assert_eq!(math::box_intersection(&inside, bounds()).to_bits(), 0);
}

#[test]
fn box_misses_backwards_and_parallel_outside_rays() {
    for ray in [
        Ray::new([-1., 5., 5.], [-1., 0., 0.]).unwrap(),
        Ray::new([-1., 5., 5.], [0., 1., 0.]).unwrap(),
        Ray::new([5., 11., 5.], [1., 0., 0.]).unwrap(),
    ] {
        assert_eq!(math::box_intersection(&ray, bounds()), f32::INFINITY);
    }
}

#[test]
fn signed_zero_candidate_plane_division_retains_native_misses() {
    let positive = Ray::new([11., 5., 5.], [0., 1., 0.]).unwrap();
    let negative = Ray::new([11., 5., 5.], [-0., 1., 0.]).unwrap();
    assert_eq!(math::box_intersection(&positive, bounds()), f32::INFINITY);
    // (-1)/(-0)=+inf, and the unused Z coordinate becomes 0*inf=NaN.
    // The Y coordinate becomes +inf, which rejects before the NaN matters.
    assert_eq!(math::box_intersection(&negative, bounds()), f32::INFINITY);
    let tiny = Ray::new([11., 5., 5.], [-0., f32::from_bits(1), 0.]).unwrap();
    assert_eq!(math::box_intersection(&tiny, bounds()), f32::INFINITY);
}

#[test]
fn overflowing_candidate_plane_retains_native_nan_result() {
    let ray = Ray::new([-f32::MAX, 0., 0.], [1., 0., 0.]).unwrap();
    let box_bounds = [f32::MAX, -1., -1., f32::MAX, 1., 1.];
    // Finite subtraction overflows to +inf; parallel coordinates become NaN.
    // Native instance rejection checks exactly +inf, not all nonfinite values.
    assert!(math::box_intersection(&ray, box_bounds).is_nan());
}

#[test]
fn leaf_visits_object_permutation_and_preserves_miss_distance() {
    let bytes = leaf();
    let (result, seen, stats, distance) =
        misses(&view(&bytes, 3), &xray(), 100., Limits::default());
    assert_eq!(result, Ok(false));
    assert_eq!(seen, [2, 0, 1]);
    assert_eq!(
        stats,
        Stats {
            node_visits: 1,
            triangle_tests: 0
        }
    );
    assert_eq!(distance, 100.);
}

#[test]
fn nearest_and_any_preserve_leaf_order() {
    let bytes = leaf();
    for (mode, expected, distance) in [(Mode::Nearest, vec![2, 0, 1], 3.), (Mode::Any, vec![2], 7.)]
    {
        let mut seen = Vec::new();
        let mut max_distance = 100.;
        let mut b = budget();
        let hit = traverse::ray(
            &view(&bytes, 3),
            &xray(),
            &mut max_distance,
            mode,
            &mut b,
            |id, max, budget| {
                budget.charge_triangle()?;
                seen.push(id);
                let distance = [5., 3., 7.][id as usize];
                if distance < *max {
                    *max = distance;
                    Ok(true)
                } else {
                    Ok(false)
                }
            },
        )
        .unwrap();
        assert!(hit);
        assert_eq!(seen, expected);
        assert_eq!(max_distance, distance);
        assert_eq!(b.stats().triangle_tests, seen.len() as u64);
    }
}

#[test]
fn node_limit_fails_before_callback_or_charge() {
    let bytes = leaf();
    let (result, seen, stats, _) = misses(
        &view(&bytes, 3),
        &xray(),
        100.,
        Limits {
            max_node_visits: 0,
            ..Limits::default()
        },
    );
    assert_eq!(result, Err(Error::NodeLimit));
    assert!(seen.is_empty());
    assert_eq!(stats, Stats::default());
}

#[test]
fn root_interval_miss_needs_no_node_budget() {
    let bytes = leaf();
    let ray = Ray::new([11., 5., 5.], [1., 0., 0.]).unwrap();
    let (result, seen, stats, _) = misses(
        &view(&bytes, 3),
        &ray,
        100.,
        Limits {
            max_node_visits: 0,
            ..Limits::default()
        },
    );
    assert_eq!(result, Ok(false));
    assert!(seen.is_empty());
    assert_eq!(stats, Stats::default());
}

#[test]
fn shared_budget_is_not_reset_between_queries() {
    let bytes = leaf();
    let tree = view(&bytes, 3);
    let mut b = Budget::new(Limits {
        max_node_visits: 1,
        ..Limits::default()
    })
    .unwrap();
    assert_eq!(
        traverse::ray(
            &tree,
            &xray(),
            &mut 100.,
            Mode::Nearest,
            &mut b,
            |_, _, _| Ok(false)
        ),
        Ok(false)
    );
    assert_eq!(
        traverse::ray(
            &tree,
            &xray(),
            &mut 100.,
            Mode::Nearest,
            &mut b,
            |_, _, _| panic!("budget bypass")
        ),
        Err(Error::NodeLimit)
    );
    assert_eq!(b.stats().node_visits, 1);
}

#[test]
fn triangle_limit_is_checked_before_triangle_work() {
    let bytes = leaf();
    let mut b = Budget::new(Limits {
        max_triangle_tests: 0,
        ..Limits::default()
    })
    .unwrap();
    let result = traverse::ray(
        &view(&bytes, 3),
        &xray(),
        &mut 100.,
        Mode::Nearest,
        &mut b,
        |_, _, b| {
            b.charge_triangle()?;
            panic!("triangle budget bypass")
        },
    );
    assert_eq!(result, Err(Error::TriangleLimit));
    assert_eq!(
        b.stats(),
        Stats {
            node_visits: 1,
            triangle_tests: 0
        }
    );
}

#[test]
fn empty_tree_retains_native_empty_leaf_visit() {
    let bytes = wire([f32::from_bits(0x7fc0_0000); 6], &[LEAF, 0, 0], &[]);
    let (result, seen, stats, distance) =
        misses(&view(&bytes, 0), &xray(), f32::INFINITY, Limits::default());
    assert_eq!(result, Ok(false));
    assert!(seen.is_empty());
    assert_eq!(stats.node_visits, 1);
    assert_eq!(distance, f32::INFINITY);
}

#[test]
fn root_fuzzy_zero_axis_does_not_test_origin_inside_that_slab() {
    let bytes = leaf();
    let ray = Ray::new([-1., 100., 5.], [1., -0., 0.]).unwrap();
    let (result, seen, _, _) = misses(&view(&bytes, 3), &ray, 100., Limits::default());
    assert_eq!(result, Ok(false));
    assert_eq!(seen, [2, 0, 1]);
}

#[test]
fn split_front_order_follows_sign_bit() {
    let bytes = split(4., 6.);
    for (ray, expected) in [
        (xray(), vec![0, 1]),
        (Ray::new([11., 5., 5.], [-1., 0., 0.]).unwrap(), vec![1, 0]),
    ] {
        let (result, seen, stats, _) = misses(&view(&bytes, 2), &ray, 100., Limits::default());
        assert_eq!(result, Ok(false));
        assert_eq!(seen, expected);
        assert_eq!(stats.node_visits, 3);
    }
}

#[test]
fn nearest_hit_prunes_pending_back_node() {
    let bytes = split(4., 6.);
    let mut b = budget();
    let mut seen = Vec::new();
    let mut distance = 100.;
    assert!(
        traverse::ray(
            &view(&bytes, 2),
            &xray(),
            &mut distance,
            Mode::Nearest,
            &mut b,
            |id, d, _| {
                seen.push(id);
                *d = 3.;
                Ok(true)
            }
        )
        .unwrap()
    );
    assert_eq!(seen, [0]);
    assert_eq!(b.stats().node_visits, 2);
}

#[test]
fn split_gap_returns_miss_without_visiting_children() {
    let bytes = split(4., 6.);
    let ray = Ray::new([5., -1., 5.], [0., 1., 0.]).unwrap();
    let (result, seen, stats, _) = misses(&view(&bytes, 2), &ray, 100., Limits::default());
    assert_eq!(result, Ok(false));
    assert!(seen.is_empty());
    assert_eq!(stats.node_visits, 1);
}

#[test]
fn parallel_clip_nan_keeps_ternary_operands_and_zero_sign_order() {
    let bytes = split(5., 5.);
    for (zero, expected) in [(0., vec![0, 1]), (-0., vec![1, 0])] {
        let ray = Ray::new([5., -1., 5.], [zero, 1., 0.]).unwrap();
        let (result, seen, stats, _) = misses(&view(&bytes, 2), &ray, 100., Limits::default());
        assert_eq!(result, Ok(false));
        assert_eq!(seen, expected);
        assert_eq!(stats.node_visits, 3);
    }
}

#[test]
fn bvh2_narrows_or_excludes_interval_in_both_directions() {
    let bytes = wire(
        bounds(),
        &[0x2000_0003, 3f32.to_bits(), 7f32.to_bits(), LEAF, 1, 0],
        &[0],
    );
    for ray in [
        xray(),
        Ray::new([11., 5., 5.], [-1., 0., 0.]).unwrap(),
        Ray::new([5., -1., 5.], [0., 1., 0.]).unwrap(),
    ] {
        let (result, seen, stats, _) = misses(&view(&bytes, 1), &ray, 100., Limits::default());
        assert_eq!(result, Ok(false));
        assert_eq!(seen, [0]);
        assert_eq!(stats.node_visits, 2);
    }
    let outside = Ray::new([2., -1., 5.], [0., 1., 0.]).unwrap();
    let (_, seen, stats, _) = misses(&view(&bytes, 1), &outside, 100., Limits::default());
    assert!(seen.is_empty());
    assert_eq!(stats.node_visits, 1);
}

#[test]
fn finite_queries_skip_validated_dead_edges_even_when_they_alias_root() {
    for words in [
        [3, 4f32.to_bits(), f32::INFINITY.to_bits(), LEAF, 1, 0],
        [0, f32::NEG_INFINITY.to_bits(), 6f32.to_bits(), LEAF, 1, 0],
    ] {
        let bytes = wire(bounds(), &words, &[0]);
        for ray in [xray(), Ray::new([11., 5., 5.], [-1., 0., 0.]).unwrap()] {
            let (result, seen, stats, _) = misses(&view(&bytes, 1), &ray, 100., Limits::default());
            assert_eq!(result, Ok(false));
            assert_eq!(seen, [0]);
            assert_eq!(stats.node_visits, 2);
        }
    }
}

fn unsafe_unary() -> (Vec<u8>, Ray) {
    let bytes = wire(
        [0., -f32::MAX, 0., 10., f32::MAX, 10.],
        &[3, 5f32.to_bits(), f32::INFINITY.to_bits(), LEAF, 1, 0],
        &[0],
    );
    let ray = Ray::new([5., -f32::MAX, 5.], [0., 1., 0.]).unwrap();
    (bytes, ray)
}

#[test]
fn infinite_effective_interval_cannot_execute_a_phantom_child() {
    // Finite endpoints overflow root intervalMax to +inf. On a clip with +0
    // direction, tf=NaN, so the absent back edge can be scheduled. This unsafe
    // native case is Rust-only; no C++ malformed/out-of-range access is invoked.
    let (bytes, ray) = unsafe_unary();
    let (result, seen, stats, _) = misses(&view(&bytes, 1), &ray, f32::INFINITY, Limits::default());
    assert_eq!(result, Err(Error::UnsafeTraversal));
    assert_eq!(seen, [0]);
    assert_eq!(stats.node_visits, 2);
}

#[test]
fn pending_phantom_is_safe_if_pruned_by_a_finite_hit_or_any_mode() {
    let (bytes, ray) = unsafe_unary();
    for mode in [Mode::Nearest, Mode::Any] {
        let mut b = budget();
        let mut distance = f32::INFINITY;
        assert!(
            traverse::ray(
                &view(&bytes, 1),
                &ray,
                &mut distance,
                mode,
                &mut b,
                |_, d, _| {
                    *d = 1.;
                    Ok(true)
                }
            )
            .unwrap()
        );
        assert_eq!(distance, 1.);
        assert_eq!(b.stats().node_visits, 2);
    }
}

#[test]
fn depth_65_constructor_tree_queries_all_primitives_with_checked_budget() {
    let boxes: Vec<_> = (0..90)
        .map(|i| {
            let x = 2.0f32.powi(120 - i - i32::from(i >= 62));
            bih::Bounds::from_bits([x.to_bits(), 0, 0, x.to_bits(), 0, 0]).unwrap()
        })
        .collect();
    let built = bih::build(&boxes, 1, bih::Limits::default()).unwrap();
    let tree = view(built.bytes(), boxes.len());
    assert_eq!(tree.stats().max_depth, 65);
    let ray = Ray::new([0.; 3], [1., 0., 0.]).unwrap();
    let (result, mut seen, stats, _) = misses(&tree, &ray, f32::INFINITY, Limits::default());
    assert_eq!(result, Ok(false));
    seen.sort_unstable();
    assert_eq!(seen, (0..90).collect::<Vec<_>>());
    assert_eq!(stats.node_visits, tree.stats().node_count as u64);
    let (result, _, spent, _) = misses(
        &tree,
        &ray,
        f32::INFINITY,
        Limits {
            max_node_visits: stats.node_visits - 1,
            ..Limits::default()
        },
    );
    assert_eq!(result, Err(Error::NodeLimit));
    assert_eq!(spent.node_visits, stats.node_visits - 1);
}

#[test]
fn private_negative_instance_distance_retains_native_miss() {
    let bytes = leaf();
    for distance in [-1., f32::NEG_INFINITY] {
        let (result, seen, stats, after) =
            misses(&view(&bytes, 3), &xray(), distance, Limits::default());
        assert_eq!(result, Ok(false));
        assert!(seen.is_empty());
        assert_eq!(stats.node_visits, 0);
        assert_eq!(after, distance);
    }
}

#[test]
fn callback_error_is_terminal_and_no_partial_hit_is_returned() {
    let bytes = leaf();
    let mut b = budget();
    let mut seen = Vec::new();
    let error = traverse::ray(
        &view(&bytes, 3),
        &xray(),
        &mut 100.,
        Mode::Nearest,
        &mut b,
        |id, _, _| {
            seen.push(id);
            Err(Error::InvalidModel)
        },
    );
    assert_eq!(error, Err(Error::InvalidModel));
    assert_eq!(seen, [2]);
}

#[test]
fn checked_bih_accessors_reject_overflow_and_out_of_range() {
    let bytes = leaf();
    let tree = view(&bytes, 3);
    assert_eq!(tree.query_word(0), Some(LEAF));
    assert_eq!(tree.query_object(0), Some(2));
    for index in [3, usize::MAX] {
        assert_eq!(tree.query_word(index), None);
        assert_eq!(tree.query_object(index), None);
    }
}
