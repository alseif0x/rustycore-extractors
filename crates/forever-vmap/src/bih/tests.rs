// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: independent synthetic BIH wire/branch/policy fixtures.
use super::*;

fn bounds(low: [f32; 3], high: [f32; 3]) -> Bounds {
    Bounds::from_bits([
        low[0].to_bits(),
        low[1].to_bits(),
        low[2].to_bits(),
        high[0].to_bits(),
        high[1].to_bits(),
        high[2].to_bits(),
    ])
    .unwrap()
}
fn point(x: f32, y: f32, z: f32) -> Bounds {
    bounds([x, y, z], [x, y, z])
}
fn wire(bounds: [u32; 6], tree: &[u32], objects: &[u32]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for value in bounds {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes.extend_from_slice(&(tree.len() as u32).to_le_bytes());
    for value in tree {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes.extend_from_slice(&(objects.len() as u32).to_le_bytes());
    for value in objects {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
}
fn view(tree: &Bih) -> View<'_> {
    View::parse(
        tree.bytes(),
        tree.stats().primitive_count,
        Limits::default(),
    )
    .unwrap()
}
fn nonempty_bounds() -> [u32; 6] {
    bounds([0.; 3], [10.; 3]).bits()
}
fn unary_chain(depth: usize) -> Vec<u8> {
    let mut tree = Vec::new();
    for index in 0..depth - 1 {
        tree.extend([BVH2 | ((index + 1) * 3) as u32, 0, 10.0_f32.to_bits()]);
    }
    tree.extend([LEAF, 1, 0]);
    wire(nonempty_bounds(), &tree, &[0])
}

#[test]
fn empty_fresh_build_is_exact_native_44_byte_sentinel() {
    let expected = wire([0x7fc0_0000; 6], &[0xc000_0000, 0, 0], &[]);
    for leaf in [1, 3] {
        let tree = build(&[], leaf, Limits::default()).unwrap();
        assert_eq!(tree.bytes(), expected);
        assert_eq!(tree.bytes().len(), 44);
        assert_eq!(view(&tree).objects().len(), 0);
        assert_eq!(tree.stats().max_depth, 1);
    }
}

#[test]
fn single_primitive_and_three_primitive_leaves_have_exact_wire() {
    let source = bounds([-1., -2., -3.], [4., 5., 6.]);
    let tree = build(&[source], 1, Limits::default()).unwrap();
    assert_eq!(tree.bytes(), wire(source.bits(), &[LEAF, 1, 0], &[0]));
    let tree = build(&[source; 3], 3, Limits::default()).unwrap();
    assert_eq!(tree.bytes(), wire(source.bits(), &[LEAF, 3, 0], &[0, 1, 2]));
}

#[test]
fn two_points_split_with_exact_nodes_and_right_swap_permutation() {
    let inputs = [point(0., 0., 0.), point(2., 0., 0.)];
    let expected_nodes = [3, 0, 2.0_f32.to_bits(), LEAF, 1, 0, LEAF | 1, 1, 0];
    let a = build(&inputs, 1, Limits::default()).unwrap();
    assert_eq!(
        a.bytes(),
        wire(
            bounds([0.; 3], [2., 0., 0.]).bits(),
            &expected_nodes,
            &[0, 1]
        )
    );
    let b = build(&[inputs[1], inputs[0]], 1, Limits::default()).unwrap();
    assert_eq!(
        b.bytes(),
        wire(
            bounds([0.; 3], [2., 0., 0.]).bits(),
            &expected_nodes,
            &[1, 0]
        )
    );
}

#[test]
fn primary_axis_ties_prefer_z_then_y_then_x() {
    for (end, axis) in [
        ([2., 2., 2.], 2),
        ([2., 2., 1.], 1),
        ([2., 1., 1.], 0),
        ([1., 2., 2.], 2),
    ] {
        let tree = build(
            &[point(0., 0., 0.), point(end[0], end[1], end[2])],
            1,
            Limits::default(),
        )
        .unwrap();
        assert_eq!(view(&tree).tree_words().next().unwrap() >> 30, axis);
    }
}

#[test]
fn merged_signed_zero_comes_from_last_box_but_leaf_bits_are_preserved() {
    let negative = Bounds::from_bits([0x8000_0000; 6]).unwrap();
    let positive = Bounds::from_bits([0; 6]).unwrap();
    for (source, expected) in [
        ([negative, positive], [0; 6]),
        ([positive, negative], [0x8000_0000; 6]),
    ] {
        let tree = build(&source, 3, Limits::default()).unwrap();
        assert_eq!(view(&tree).bounds_bits(), expected);
    }
    assert_eq!(
        build(&[negative], 1, Limits::default()).unwrap().bytes(),
        wire(negative.bits(), &[LEAF, 1, 0], &[0])
    );
}

#[test]
fn identical_points_stop_with_oversized_leaf_without_hanging() {
    let source = [point(1., 1., 1.); 5];
    let tree = build(&source, 1, Limits::default()).unwrap();
    assert_eq!(view(&tree).tree_words().collect::<Vec<_>>(), [LEAF, 5, 0]);
    assert_eq!(view(&tree).objects().collect::<Vec<_>>(), [0, 1, 2, 3, 4]);
    assert!(tree.stats().build_work < 100);
}

#[test]
fn finite_extreme_midpoint_and_extent_overflow_can_still_build_valid_trees() {
    for source in [
        vec![point(f32::MAX, 0., 0.); 4],
        vec![point(-f32::MAX, 0., 0.); 4],
        vec![point(-f32::MAX, 0., 0.), point(f32::MAX, 0., 0.)],
    ] {
        let tree = build(&source, 1, Limits::default()).unwrap();
        assert_eq!(view(&tree).objects().len(), source.len());
        assert!(tree.stats().build_work < 1000);
    }
}

#[test]
fn construction_exercises_bvh2_and_both_unary_directions() {
    let special = build(
        &[
            point(0., 4., 0.),
            point(1., 6., 0.),
            point(100., 0., 0.),
            point(100., 10., 0.),
        ],
        1,
        Limits::default(),
    )
    .unwrap();
    let words: Vec<_> = view(&special).tree_words().collect();
    assert!(
        words
            .as_chunks::<3>()
            .0
            .iter()
            .any(|node| node[0] & BVH2 != 0)
    );
    for (points, left) in [([0., 4., 5., 100.], true), ([0., 95., 96., 100.], false)] {
        let boxes = points.map(|x| point(x, 0., 0.));
        let tree = build(&boxes, 1, Limits::default()).unwrap();
        let words: Vec<_> = view(&tree).tree_words().collect();
        assert!(words.as_chunks::<3>().0.iter().any(|node| {
            node[0] >> 30 < 3
                && node[0] & BVH2 == 0
                && if left {
                    node[2] == f32::INFINITY.to_bits()
                } else {
                    node[1] == f32::NEG_INFINITY.to_bits()
                }
        }));
    }
}

#[test]
fn independent_builds_do_not_accumulate_prior_bounds_or_mutate_source() {
    let source = [point(2., 3., 4.), point(8., 9., 10.)];
    let original = source.map(Bounds::bits);
    let first = build(&source, 1, Limits::default()).unwrap();
    let second = build(&source, 1, Limits::default()).unwrap();
    assert_eq!(first.bytes(), second.bytes());
    assert_eq!(source.map(Bounds::bits), original);
    let small = build(&[point(0., 0., 0.)], 3, Limits::default()).unwrap();
    assert_eq!(view(&small).bounds_bits(), [0; 6]);
}

#[test]
fn input_rejects_nonfinite_or_inverted_bounds_but_accepts_signed_zero() {
    for bits in [
        f32::NAN.to_bits(),
        f32::INFINITY.to_bits(),
        f32::NEG_INFINITY.to_bits(),
    ] {
        for axis in 0..6 {
            let mut input = [0; 6];
            input[axis] = bits;
            assert_eq!(Bounds::from_bits(input).err(), Some(Error::NonFinite));
        }
    }
    let mut input = [0; 6];
    input[0] = 1.0_f32.to_bits();
    assert_eq!(Bounds::from_bits(input).err(), Some(Error::Bounds));
    assert!(Bounds::from_bits([0, 0, 0, 0x8000_0000, 0x8000_0000, 0x8000_0000]).is_ok());
}

#[test]
fn unsupported_leaf_sizes_are_rejected_even_for_empty_input() {
    for leaf in [0, 2, 4, u32::MAX] {
        assert_eq!(
            build(&[], leaf, Limits::default()).err(),
            Some(Error::LeafSize)
        );
    }
}

#[test]
fn work_budget_stops_retries_and_admits_exact_recorded_work() {
    let source = [point(0., 0., 0.); 8];
    let baseline = build(&source, 1, Limits::default()).unwrap();
    let limits = Limits {
        max_work: baseline.stats().build_work,
        ..Limits::default()
    };
    assert!(build(&source, 1, limits).is_ok());
    assert_eq!(
        build(
            &source,
            1,
            Limits {
                max_work: limits.max_work - 1,
                ..limits
            }
        )
        .err(),
        Some(Error::WorkLimit)
    );
}

#[test]
fn builder_caps_are_downward_and_account_encoding_validation_overlap() {
    let source = [point(0., 0., 0.), point(1., 1., 1.)];
    let baseline = build(&source, 1, Limits::default()).unwrap();
    let exact = Limits {
        max_primitives: 2,
        max_tree_words: baseline.stats().tree_words,
        max_bytes: baseline.bytes().len(),
        max_memory_bytes: baseline.stats().peak_bytes,
        max_work: baseline.stats().build_work,
        max_depth: baseline.stats().max_depth,
    };
    assert!(build(&source, 1, exact).is_ok());
    for limits in [
        Limits {
            max_primitives: 1,
            ..exact
        },
        Limits {
            max_tree_words: exact.max_tree_words - 1,
            ..exact
        },
        Limits {
            max_bytes: exact.max_bytes - 1,
            ..exact
        },
        Limits {
            max_depth: 1,
            ..exact
        },
    ] {
        assert!(build(&source, 1, limits).is_err());
    }
    // Keep the same tree-capacity growth policy while checking the exact heap
    // peak. Reducing max_tree_words can itself reduce allocation capacity.
    let memory_exact = Limits {
        max_memory_bytes: baseline.stats().peak_bytes,
        ..Limits::default()
    };
    assert!(build(&source, 1, memory_exact).is_ok());
    assert_eq!(
        build(
            &source,
            1,
            Limits {
                max_memory_bytes: memory_exact.max_memory_bytes - 1,
                ..memory_exact
            }
        )
        .err(),
        Some(Error::Limit)
    );
}

#[test]
fn native_geometric_spacing_witness_reaches_depth_65() {
    // Independently executed by the original C++ builder for both leaf sizes.
    // Only gap 62 creates the extra unary node inside the depth-63 frame;
    // the neighboring witnesses stop at depth 64 in the original builder.
    for gap in 58..=64 {
        let source: Vec<_> = (0..90)
            .map(|index| {
                let exponent = 120 - index - i32::from(index >= gap);
                let x = f32::from_bits(((exponent + 127) as u32) << 23);
                point(x, 0., 0.)
            })
            .collect();
        for leaf in [1, 3] {
            let tree = build(&source, leaf, Limits::default()).unwrap();
            assert_eq!(tree.stats().max_depth, if gap == 62 { 65 } else { 64 });
            assert_eq!(
                build(
                    &source,
                    leaf,
                    Limits {
                        max_depth: 64,
                        ..Limits::default()
                    }
                )
                .err(),
                if gap == 62 { Some(Error::Limit) } else { None }
            );
        }
    }
}

#[test]
fn viewer_accepts_dead_unary_offsets_outside_or_back_to_parent() {
    let root_bounds = nonempty_bounds();
    for tree in [
        [3, 5.0_f32.to_bits(), f32::INFINITY.to_bits(), LEAF, 1, 0],
        [
            0,
            f32::NEG_INFINITY.to_bits(),
            5.0_f32.to_bits(),
            LEAF,
            1,
            0,
        ],
    ] {
        let bytes = wire(root_bounds, &tree, &[0]);
        let parsed = View::parse(&bytes, 1, Limits::default()).unwrap();
        assert_eq!(parsed.stats().node_count, 2);
    }
}

#[test]
fn viewer_accepts_normal_gap_touch_overlap_and_bvh2_zero_width() {
    for (left, right) in [(3.0_f32, 7.0_f32), (5., 5.), (7., 3.)] {
        let bytes = wire(
            nonempty_bounds(),
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
            &[1, 0],
        );
        assert!(View::parse(&bytes, 2, Limits::default()).is_ok());
    }
    let bytes = wire(
        nonempty_bounds(),
        &[BVH2 | 3, 5.0_f32.to_bits(), 5.0_f32.to_bits(), LEAF, 1, 0],
        &[0],
    );
    assert!(View::parse(&bytes, 1, Limits::default()).is_ok());
}

#[test]
fn viewer_rejects_truncation_tail_incomplete_nodes_and_count_mismatch() {
    let good = wire(nonempty_bounds(), &[LEAF, 1, 0], &[0]);
    for length in 0..good.len() {
        assert!(View::parse(&good[..length], 1, Limits::default()).is_err());
    }
    let mut tail = good.clone();
    tail.push(0);
    assert_eq!(
        View::parse(&tail, 1, Limits::default()).err(),
        Some(Error::Framing)
    );
    for tree in [vec![], vec![LEAF], vec![LEAF, 1], vec![LEAF, 1, 0, 0]] {
        assert_eq!(
            View::parse(&wire(nonempty_bounds(), &tree, &[0]), 1, Limits::default()).err(),
            Some(Error::Framing)
        );
    }
    assert_eq!(
        View::parse(&good, 2, Limits::default()).err(),
        Some(Error::PrimitiveCount)
    );
}

#[test]
fn viewer_rejects_bad_leaf_ranges_and_reserved_word_or_flag() {
    for tree in [
        vec![LEAF, 0, 0],
        vec![LEAF | 1, 1, 0],
        vec![LEAF, 2, 0],
        vec![LEAF, 1, 9],
        vec![LEAF | BVH2, 1, 0],
    ] {
        assert!(View::parse(&wire(nonempty_bounds(), &tree, &[0]), 1, Limits::default()).is_err());
    }
}

#[test]
fn viewer_rejects_cycles_aliases_unaligned_and_unreachable_allocations() {
    for tree in [
        vec![BVH2, 0, 10.0_f32.to_bits()],
        vec![BVH2 | 4, 0, 10.0_f32.to_bits(), LEAF, 1, 0],
        vec![BVH2 | 9, 0, 10.0_f32.to_bits(), LEAF, 1, 0],
        vec![LEAF, 1, 0, LEAF, 1, 0],
        vec![
            3,
            5.0_f32.to_bits(),
            5.0_f32.to_bits(),
            BVH2 | 6,
            0,
            10.0_f32.to_bits(),
            LEAF,
            1,
            0,
        ],
    ] {
        assert!(View::parse(&wire(nonempty_bounds(), &tree, &[0]), 1, Limits::default()).is_err());
    }
}

#[test]
fn viewer_rejects_duplicate_or_out_of_range_object_permutations() {
    for ids in [[0, 0], [0, 2], [u32::MAX, 1]] {
        assert_eq!(
            View::parse(
                &wire(nonempty_bounds(), &[LEAF, 2, 0], &ids),
                2,
                Limits::default()
            )
            .err(),
            Some(Error::Permutation)
        );
    }
}

#[test]
fn viewer_rejects_leaf_range_overlap_gap_and_reordering() {
    for (first, second) in [(0, 0), (1, 0), (0, 2)] {
        let tree = [
            3,
            3.0_f32.to_bits(),
            7.0_f32.to_bits(),
            LEAF | first,
            1,
            0,
            LEAF | second,
            1,
            0,
        ];
        assert_eq!(
            View::parse(
                &wire(nonempty_bounds(), &tree, &[0, 1]),
                2,
                Limits::default()
            )
            .err(),
            Some(Error::Range)
        );
    }
}

#[test]
fn viewer_rejects_nan_wrong_infinity_both_dead_or_inverted_bvh2_clips() {
    for (header, left, right) in [
        (3, f32::NAN, 5.),
        (3, f32::INFINITY, 5.),
        (3, 5., f32::NEG_INFINITY),
        (3, f32::NEG_INFINITY, f32::INFINITY),
        (BVH2 | 3, 7., 3.),
        (BVH2 | 3, f32::NEG_INFINITY, 3.),
        (3, -1., 3.),
    ] {
        let tree = [header, left.to_bits(), right.to_bits(), LEAF, 1, 0];
        assert_eq!(
            View::parse(&wire(nonempty_bounds(), &tree, &[0]), 1, Limits::default()).err(),
            Some(Error::Clip)
        );
    }
}

#[test]
fn depth_65_is_admitted_but_66_and_reduced_depth_are_rejected() {
    let bytes = unary_chain(65);
    assert_eq!(
        View::parse(&bytes, 1, Limits::default())
            .unwrap()
            .stats()
            .max_depth,
        65
    );
    assert_eq!(
        View::parse(
            &bytes,
            1,
            Limits {
                max_depth: 64,
                ..Limits::default()
            }
        )
        .err(),
        Some(Error::Limit)
    );
    assert_eq!(
        View::parse(&unary_chain(66), 1, Limits::default()).err(),
        Some(Error::Limit)
    );
}

#[test]
fn empty_wire_requires_exact_nan_and_dummy_leaf_encoding() {
    for (bounds, tree) in [
        ([0; 6], [LEAF, 0, 0]),
        ([0xffc0_0000; 6], [LEAF, 0, 0]),
        ([EMPTY_BITS; 6], [LEAF, 0, 1]),
        ([EMPTY_BITS; 6], [LEAF | 1, 0, 0]),
    ] {
        assert_eq!(
            View::parse(&wire(bounds, &tree, &[]), 0, Limits::default()).err(),
            Some(Error::Empty)
        );
    }
}

#[test]
fn viewer_rejects_bad_nonempty_bounds_and_counts_before_allocation() {
    let mut b = nonempty_bounds();
    b[0] = f32::NAN.to_bits();
    assert_eq!(
        View::parse(&wire(b, &[LEAF, 1, 0], &[0]), 1, Limits::default()).err(),
        Some(Error::NonFinite)
    );
    b = nonempty_bounds();
    b[0] = 11.0_f32.to_bits();
    assert_eq!(
        View::parse(&wire(b, &[LEAF, 1, 0], &[0]), 1, Limits::default()).err(),
        Some(Error::Bounds)
    );
    let mut bytes = wire(nonempty_bounds(), &[LEAF, 1, 0], &[0]);
    bytes[24..28].copy_from_slice(&u32::MAX.to_le_bytes());
    assert_eq!(
        View::parse(&bytes, 1, Limits::default()).err(),
        Some(Error::Limit)
    );
}

#[test]
fn viewer_scratch_capacity_limits_are_exact_and_empty_allocates_nothing() {
    let bytes = wire(nonempty_bounds(), &[LEAF, 3, 0], &[2, 0, 1]);
    let peak = View::parse(&bytes, 3, Limits::default())
        .unwrap()
        .stats()
        .validation_peak_bytes;
    assert!(
        View::parse(
            &bytes,
            3,
            Limits {
                max_memory_bytes: peak,
                ..Limits::default()
            }
        )
        .is_ok()
    );
    assert_eq!(
        View::parse(
            &bytes,
            3,
            Limits {
                max_memory_bytes: peak - 1,
                ..Limits::default()
            }
        )
        .err(),
        Some(Error::Limit)
    );
    let empty = wire([EMPTY_BITS; 6], &[LEAF, 0, 0], &[]);
    assert!(
        View::parse(
            &empty,
            0,
            Limits {
                max_memory_bytes: 0,
                max_primitives: 0,
                max_work: 0,
                ..Limits::default()
            }
        )
        .is_ok()
    );
}

#[test]
fn raised_limits_reject_build_and_view_even_when_empty() {
    let bytes = wire([EMPTY_BITS; 6], &[LEAF, 0, 0], &[]);
    for limits in [
        Limits {
            max_primitives: MAX_PRIMITIVES + 1,
            ..Limits::default()
        },
        Limits {
            max_tree_words: MAX_TREE_WORDS + 1,
            ..Limits::default()
        },
        Limits {
            max_bytes: MAX_BYTES + 1,
            ..Limits::default()
        },
        Limits {
            max_memory_bytes: MAX_MEMORY_BYTES + 1,
            ..Limits::default()
        },
        Limits {
            max_work: MAX_WORK + 1,
            ..Limits::default()
        },
        Limits {
            max_depth: MAX_DEPTH + 1,
            ..Limits::default()
        },
    ] {
        assert_eq!(build(&[], 1, limits).err(), Some(Error::InvalidLimits));
        assert_eq!(
            View::parse(&bytes, 0, limits).err(),
            Some(Error::InvalidLimits)
        );
    }
}
