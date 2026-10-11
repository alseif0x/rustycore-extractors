// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: only independently authored synthetic placements/wire.
use super::*;

fn row(id: u32, flags: u8, bounds: [f32; 6], name: &[u8]) -> Vec<u8> {
    let mut out = vec![flags, 7];
    push(&mut out, id);
    for v in [0., 0., 0., 0., 0., 0., 1.] {
        push(&mut out, f32::to_bits(v));
    }
    if flags & 1 != 0 {
        for v in bounds {
            push(&mut out, v.to_bits());
        }
    }
    push(&mut out, name.len() as u32);
    out.extend_from_slice(name);
    out
}
fn point(x: f32, y: f32) -> [f32; 6] {
    [x, y, 0., x, y, 0.]
}
fn own(id: u32) -> Vec<u8> {
    row(id, 1, point(0., 0.), b"model")
}
fn run(bytes: &[u8], limits: Limits) -> Result<AssembledMap, Error> {
    let input = placement::DirBin::parse(bytes, placement::Limits::default()).unwrap();
    let records: Vec<_> = input.records().collect();
    build(701, &records, limits)
}
fn index(values: &[u32]) -> Vec<u8> {
    let mut out = MAGIC.to_vec();
    push(&mut out, values.len() as u32);
    for v in values {
        push(&mut out, *v);
    }
    out
}
fn tile(rows: &[Vec<u8>]) -> Vec<u8> {
    let mut out = MAGIC.to_vec();
    push(&mut out, rows.len() as u32);
    for r in rows {
        out.extend_from_slice(r);
    }
    out
}
fn put(bytes: &mut [u8], at: usize, v: u32) {
    bytes[at..at + 4].copy_from_slice(&v.to_le_bytes());
}

#[test]
fn empty_map_is_exact_native_fifty_six_bytes() {
    let map = build(0, &[], Limits::default()).unwrap();
    let mut expected = MAGIC.to_vec();
    expected.extend_from_slice(b"NODE");
    for v in [0x7fc0_0000; 6] {
        push(&mut expected, v);
    }
    for v in [3, 0xc000_0000, 0, 0, 0] {
        push(&mut expected, v);
    }
    assert_eq!(map.tree_bytes(), expected);
    assert_eq!(map.stats().output_bytes, 56);
    assert!(map.tiles().is_empty());
    assert!(map.ordered_dir_bin().is_empty());
    assert_eq!(
        Tree::parse(&expected, 0, Limits::default())
            .unwrap()
            .bih()
            .stats()
            .primitive_count,
        0
    );
}
#[test]
fn native_first_id_wins_then_sorted_with_maximum_and_zero_ids() {
    let first = row(9, 1, point(0., 0.), b"first");
    let other = row(9, 3, point(800., 800.), b"second");
    let raw = [own(u32::MAX), first.clone(), own(0), other].concat();
    let map = run(&raw, Limits::default()).unwrap();
    assert_eq!(
        map.ordered_dir_bin(),
        [own(0), first, own(u32::MAX)].concat()
    );
    assert_eq!(map.stats().duplicates, 1);
    assert_eq!(map.stats().unique_spawns, 3);
    assert_eq!(map.tiles().len(), 1);
    assert_eq!(
        IndexView::parse(map.tiles()[0].index_bytes(), 3, Limits::default())
            .unwrap()
            .indices()
            .collect::<Vec<_>>(),
        [0, 1, 2]
    );
}
#[test]
fn own_then_parent_order_is_not_global_id_order() {
    let parent = row(2, 3, point(0., 0.), b"parent");
    let raw = [parent.clone(), own(90)].concat();
    let map = run(&raw, Limits::default()).unwrap();
    let t = &map.tiles()[0];
    assert_eq!(t.payload().unwrap(), tile(&[own(90), parent]));
    assert_eq!(t.index_bytes(), index(&[1, 0]));
    verify_associations(
        t.payload().unwrap(),
        t.index_bytes(),
        map.ordered_dir_bin(),
        Limits::default(),
    )
    .unwrap();
}
#[test]
fn parent_only_tile_has_index_without_payload() {
    let raw = [own(8), row(1, 3, point(800., 800.), b"parent")].concat();
    let map = run(&raw, Limits::default()).unwrap();
    assert_eq!(map.tiles().len(), 2);
    assert_eq!((map.tiles()[1].x(), map.tiles()[1].y()), (1, 1));
    assert!(map.tiles()[1].payload().is_none());
    assert_eq!(map.tiles()[1].index_bytes(), index(&[0]));
    assert_eq!(
        (map.stats().payload_tiles, map.stats().parent_only_tiles),
        (1, 1)
    );
}
#[test]
fn inclusive_rectangles_are_x_major_without_clipping() {
    let raw = row(3, 1, [0., 0., 0., 600., 600., 1.], b"a");
    let map = run(&raw, Limits::default()).unwrap();
    assert_eq!(
        map.tiles()
            .iter()
            .map(|t| (t.x(), t.y()))
            .collect::<Vec<_>>(),
        [(0, 0), (0, 1), (1, 0), (1, 1)]
    );
    assert_eq!(map.stats().associations, 4);
    let far = run(&row(3, 1, point(80000., 80000.), b"a"), Limits::default()).unwrap();
    assert_eq!((far.tiles()[0].x(), far.tiles()[0].y()), (150, 150));
}
#[test]
fn exact_f32_tile_boundary_and_negative_fraction_truncation() {
    #[allow(clippy::excessive_precision)]
    let unit = 533.33333_f32;
    let raw = row(4, 1, [-0.25 * unit, -0.25 * unit, 0., unit, unit, 0.], b"a");
    let map = run(&raw, Limits::default()).unwrap();
    assert_eq!(map.tiles().len(), 4);
    assert_eq!((map.tiles()[0].x(), map.tiles()[0].y()), (0, 0));
    assert_eq!((map.tiles()[3].x(), map.tiles()[3].y()), (1, 1));
}
#[test]
fn injective_domain_edges_and_rejections_precede_casts() {
    #[allow(clippy::excessive_precision)]
    let unit = 533.33333_f32;
    let max = run(
        &row(1, 1, point(32767.5 * unit, 255.5 * unit), b"a"),
        Limits::default(),
    )
    .unwrap();
    assert_eq!((max.tiles()[0].x(), max.tiles()[0].y()), (32767, 255));
    for p in [
        point(-unit * 1.1, 0.),
        point(0., -unit * 1.1),
        point(32768.5 * unit, 0.),
        point(0., 256.5 * unit),
        point(f32::MAX, 0.),
    ] {
        assert_eq!(
            run(&row(1, 1, p, b"a"), Limits::default()).err(),
            Some(Error::Coordinates)
        );
    }
}
#[test]
fn all_inputs_including_discarded_duplicates_require_bounds_and_valid_range() {
    let raw = [own(1), row(1, 0, point(0., 0.), b"ignored")].concat();
    assert_eq!(
        run(&raw, Limits::default()).err(),
        Some(Error::MissingBounds)
    );
    let raw = [own(1), row(1, 1, point(-600., 0.), b"ignored")].concat();
    assert_eq!(run(&raw, Limits::default()).err(), Some(Error::Coordinates));
}
#[test]
fn association_and_individual_range_caps_preflight_expansion() {
    let raw = row(1, 1, [0., 0., 0., 600., 600., 0.], b"a");
    for limits in [
        Limits {
            max_associations: 3,
            ..Limits::default()
        },
        Limits {
            max_tiles: 3,
            ..Limits::default()
        },
    ] {
        assert_eq!(run(&raw, limits).err(), Some(Error::Limit));
    }
    let raw = [own(1), row(2, 1, point(600., 0.), b"b")].concat();
    assert_eq!(
        run(
            &raw,
            Limits {
                max_tiles: 1,
                ..Limits::default()
            }
        )
        .err(),
        Some(Error::Limit)
    );
}
#[test]
fn byte_record_and_name_caps_are_rechecked() {
    let raw = own(1);
    for limits in [
        Limits {
            max_input_bytes: raw.len() - 1,
            ..Limits::default()
        },
        Limits {
            max_spawns: 0,
            ..Limits::default()
        },
        Limits {
            max_name_bytes: 1,
            ..Limits::default()
        },
        Limits {
            max_tile_bytes: 12 + raw.len() - 1,
            ..Limits::default()
        },
    ] {
        assert!(run(&raw, limits).is_err());
    }
}
#[test]
fn total_output_excludes_proof_but_heap_retains_it() {
    let raw = [own(1), row(2, 3, point(600., 0.), b"p")].concat();
    let map = run(&raw, Limits::default()).unwrap();
    let total = map.tree_bytes().len()
        + map
            .tiles()
            .iter()
            .map(|t| t.index_bytes().len() + t.payload().map_or(0, <[u8]>::len))
            .sum::<usize>();
    assert_eq!(map.stats().output_bytes, total);
    assert_eq!(map.stats().proof_bytes, raw.len());
    assert!(map.stats().resident_bytes >= total + raw.len());
    assert!(
        run(
            &raw,
            Limits {
                max_output_bytes: total,
                ..Limits::default()
            }
        )
        .is_ok()
    );
    assert!(
        run(
            &raw,
            Limits {
                max_output_bytes: total - 1,
                ..Limits::default()
            }
        )
        .is_err()
    );
}
#[test]
fn heap_peak_and_bih_caps_cover_overlapping_work() {
    let raw = [own(9), own(2), row(7, 3, point(0., 0.), b"p")].concat();
    let stats = run(&raw, Limits::default()).unwrap().stats();
    assert!(stats.peak_bytes > stats.resident_bytes);
    assert!(
        run(
            &raw,
            Limits {
                max_memory_bytes: stats.peak_bytes,
                ..Limits::default()
            }
        )
        .is_ok()
    );
    assert!(
        run(
            &raw,
            Limits {
                max_memory_bytes: stats.peak_bytes - 1,
                ..Limits::default()
            }
        )
        .is_err()
    );
    assert!(
        run(
            &raw,
            Limits {
                bih: bih::Limits {
                    max_primitives: 2,
                    ..bih::Limits::default()
                },
                ..Limits::default()
            }
        )
        .is_err()
    );
}
#[test]
fn raised_limits_reject_even_empty_input() {
    for limits in [
        Limits {
            max_associations: MAX_ASSOCIATIONS + 1,
            ..Limits::default()
        },
        Limits {
            max_tiles: MAX_TILES + 1,
            ..Limits::default()
        },
        Limits {
            max_memory_bytes: MAX_MEMORY_BYTES + 1,
            ..Limits::default()
        },
        Limits {
            max_input_bytes: MAX_INPUT_BYTES + 1,
            ..Limits::default()
        },
        Limits {
            max_tile_bytes: MAX_TILE_BYTES + 1,
            ..Limits::default()
        },
        Limits {
            max_output_bytes: MAX_OUTPUT_BYTES + 1,
            ..Limits::default()
        },
    ] {
        assert_eq!(build(1, &[], limits).err(), Some(Error::InvalidLimits));
    }
}
#[test]
fn tree_reader_requires_magic_node_exact_eof_and_expected_count() {
    let map = run(&own(1), Limits::default()).unwrap();
    let bytes = map.tree_bytes();
    for n in 0..bytes.len() {
        assert!(Tree::parse(&bytes[..n], 1, Limits::default()).is_err());
    }
    let mut tail = bytes.to_vec();
    tail.push(0);
    assert!(Tree::parse(&tail, 1, Limits::default()).is_err());
    assert!(Tree::parse(bytes, 2, Limits::default()).is_err());
    for at in [0, 8] {
        let mut bad = bytes.to_vec();
        bad[at] = b'?';
        assert!(Tree::parse(&bad, 1, Limits::default()).is_err());
    }
}
#[test]
fn tile_reader_checks_full_records_bounds_count_and_order() {
    let good = tile(&[own(2), own(8), row(1, 3, point(0., 0.), b"p")]);
    assert_eq!(
        TileView::parse(&good, Limits::default()).unwrap().count(),
        3
    );
    for n in 0..good.len() {
        assert!(TileView::parse(&good[..n], Limits::default()).is_err());
    }
    let mut bad = good.clone();
    bad.push(0);
    assert!(TileView::parse(&bad, Limits::default()).is_err());
    let mut bad = good;
    put(&mut bad, 8, 2);
    assert!(TileView::parse(&bad, Limits::default()).is_err());
    for rows in [
        vec![own(8), own(2)],
        vec![own(1), own(1)],
        vec![row(1, 3, point(0., 0.), b"p"), own(2)],
        vec![row(1, 3, point(0., 0.), b"p")],
        vec![own(1), row(1, 3, point(0., 0.), b"p")],
        vec![row(1, 0, point(0., 0.), b"m")],
    ] {
        assert!(TileView::parse(&tile(&rows), Limits::default()).is_err());
    }
}
#[test]
fn index_reader_checks_extent_range_duplicates_and_small_tile_scratch() {
    let good = index(&[2, 0, 1]);
    assert_eq!(
        IndexView::parse(&good, 3, Limits::default())
            .unwrap()
            .indices()
            .collect::<Vec<_>>(),
        [2, 0, 1]
    );
    for n in 0..good.len() {
        assert!(IndexView::parse(&good[..n], 3, Limits::default()).is_err());
    }
    for bad in [index(&[]), index(&[3]), index(&[0, 0])] {
        assert!(IndexView::parse(&bad, 3, Limits::default()).is_err());
    }
    let mut tail = good;
    tail.push(0);
    assert!(IndexView::parse(&tail, 3, Limits::default()).is_err());
    let one = index(&[999_999]);
    let view = IndexView::parse(
        &one,
        MAX_SPAWNS,
        Limits {
            max_memory_bytes: 4,
            ..Limits::default()
        },
    )
    .unwrap();
    assert_eq!(view.validation_peak_bytes(), 4);
    assert!(
        IndexView::parse(
            &one,
            MAX_SPAWNS,
            Limits {
                max_memory_bytes: 3,
                ..Limits::default()
            }
        )
        .is_err()
    );
}
#[test]
fn association_verifier_checks_exact_data_not_just_ids() {
    let proof = [own(2), own(8)].concat();
    let payload = tile(&[own(2), own(8)]);
    let idx = index(&[0, 1]);
    verify_associations(&payload, &idx, &proof, Limits::default()).unwrap();
    assert!(verify_associations(&payload, &index(&[1, 0]), &proof, Limits::default()).is_err());
    let mut changed = payload.clone();
    changed[12 + 1] ^= 1;
    assert_eq!(
        verify_associations(&changed, &idx, &proof, Limits::default()),
        Err(Error::Association)
    );
    let mut changed = payload;
    put(&mut changed, 12 + 6, 0x8000_0000);
    assert_eq!(
        verify_associations(&changed, &idx, &proof, Limits::default()),
        Err(Error::Association)
    );
}
#[test]
fn association_verifier_validates_unused_proof_and_all_caps() {
    let payload = tile(&[own(2)]);
    let idx = index(&[0]);
    for proof in [
        [own(2), row(8, 0, point(0., 0.), b"m")].concat(),
        [own(2), own(2)].concat(),
        [own(2), own(1)].concat(),
        [own(2), row(8, 1, point(-800., 0.), b"m")].concat(),
    ] {
        assert!(verify_associations(&payload, &idx, &proof, Limits::default()).is_err());
    }
    assert!(
        verify_associations(
            &payload,
            &idx,
            &own(2),
            Limits {
                max_memory_bytes: 0,
                ..Limits::default()
            }
        )
        .is_err()
    );
}
#[test]
fn inherited_verifier_masks_only_parent_bit_and_never_reorders() {
    let ancestor = tile(&[own(2), row(9, 3, point(0., 0.), b"p")]);
    let child = [
        row(2, 3, point(0., 0.), b"model"),
        row(9, 3, point(0., 0.), b"p"),
    ]
    .concat();
    let idx = index(&[0, 1]);
    verify_inherited_associations(&ancestor, &idx, &child, Limits::default()).unwrap();
    assert!(verify_associations(&ancestor, &idx, &child, Limits::default()).is_err());
    let not_parent = [own(2), row(9, 3, point(0., 0.), b"p")].concat();
    assert!(
        verify_inherited_associations(&ancestor, &idx, &not_parent, Limits::default()).is_err()
    );
    let reordered = tile(&[
        row(9, 1, point(0., 0.), b"p"),
        row(2, 3, point(0., 0.), b"model"),
    ]);
    assert!(verify_inherited_associations(&reordered, &idx, &child, Limits::default()).is_err());
    assert!(
        verify_inherited_associations(&reordered, &index(&[1, 0]), &child, Limits::default())
            .is_err()
    );
    let mut bad = ancestor;
    bad[13] ^= 1;
    assert!(verify_inherited_associations(&bad, &idx, &child, Limits::default()).is_err());
}
#[test]
fn different_ids_can_repeat_the_same_model_and_coordinate() {
    let raw = [own(7), own(8), own(9), own(10)].concat();
    let map = run(&raw, Limits::default()).unwrap();
    assert_eq!(map.stats().unique_spawns, 4);
    assert_eq!(map.stats().associations, 4);
    assert_eq!(map.stats().bih_nodes, 1); // Oversized coincident leaf is native.
    assert!(map.stats().bih_work > 0);
}

#[test]
fn tile_ordinals_follow_sorted_ids_not_bih_permutation() {
    let raw = [
        row(1, 1, point(2400., 0.), b"a"),
        row(2, 1, point(1600., 0.), b"a"),
        row(3, 1, point(800., 0.), b"a"),
        row(4, 1, point(0., 0.), b"a"),
    ]
    .concat();
    let map = run(&raw, Limits::default()).unwrap();
    let tree = Tree::parse(map.tree_bytes(), 4, Limits::default()).unwrap();
    assert_ne!(tree.bih().objects().collect::<Vec<_>>(), [0, 1, 2, 3]);
    for (tile, ordinal) in map.tiles().iter().zip([3, 2, 1, 0]) {
        assert_eq!(tile.index_bytes(), index(&[ordinal]));
        verify_associations(
            tile.payload().unwrap(),
            tile.index_bytes(),
            map.ordered_dir_bin(),
            Limits::default(),
        )
        .unwrap();
    }
}
