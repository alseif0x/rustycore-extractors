// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: synthetic target configuration and transaction boundaries.
use super::*;
mod corpus;
fn request(map_id: u32, big: bool) -> Request {
    Request {
        map_id,
        tile: Coordinate {
            row: 31,
            column: 31,
        },
        big_base_unit: big,
        slopes: recast::Slopes::default(),
    }
}
fn cfg(map: u32, big: bool) -> Config {
    Config::new(request(map, big), &Mesh::default()).unwrap()
}
#[test]
fn default_cell_sizes_and_fixed_subtile_counts() {
    for (big, size, width, r, h, cs) in [
        (false, 80, 90, 2, 6, 0.2666666f32),
        (true, 40, 48, 1, 3, 0.5333333),
    ] {
        let c = cfg(1, big);
        assert_eq!(c.base_unit().to_bits(), cs.to_bits());
        assert_eq!(c.cell_height(), cs);
        assert_eq!(c.tile_size(), size);
        assert_eq!(c.grid(0, 0).unwrap().width, width);
        assert_eq!(c.radius(), r);
        assert_eq!(c.agent().height, h);
        assert_eq!(c.agent().climb, h);
        assert_eq!((GRID_SIZE / cs + 0.5) as u16 / size, 25);
    }
}
#[test]
fn arena_exception_does_not_recompute_border() {
    for big in [false, true] {
        let a = cfg(562, big);
        let d = cfg(1, big);
        assert_eq!(a.radius(), 0);
        assert_eq!(a.border(), d.border());
        assert_eq!(a.grid(0, 0).unwrap(), d.grid(0, 0).unwrap());
    }
}
#[test]
fn blackfathom_changes_cell_height_after_detail_and_agent_settings() {
    for big in [false, true] {
        let a = cfg(48, big);
        let d = cfg(1, big);
        assert_eq!(a.cell_height(), d.cell_height() * 2.0);
        assert_eq!(a.detail_options().maximum_error, d.base_unit());
        assert_eq!(a.detail_options().sample_distance, d.base_unit() * 16.0);
        let p = global_params(&[request(48, big).tile]).unwrap();
        assert_eq!(
            a.detour_options(p).unwrap().walkable_height,
            d.detour_options(p).unwrap().walkable_height
        );
    }
}
#[test]
fn thresholds_and_tessellation_match_target() {
    let c = cfg(1, false);
    let r = c.region_options();
    assert_eq!((r.minimum_area, r.merge_area), (3600, 2500));
    let co = c.contour_options();
    assert_eq!(co.maximum_error, 1.8);
    assert_eq!(co.maximum_edge_length, 81);
    assert!(co.tessellate_walls);
    assert!(!co.tessellate_areas);
}
#[test]
fn grid_order_and_float_operation_sequence_are_explicit() {
    let c = cfg(1, false);
    for y in 0..25 {
        for x in 0..25 {
            let g = c.grid(x, y).unwrap();
            let step = 80.0 * c.base_unit();
            let pad = 5.0 * c.base_unit();
            assert_eq!(g.min[0].to_bits(), (f32::from(x) * step - pad).to_bits());
            assert_eq!(
                g.max[2].to_bits(),
                (f32::from(y + 1) * step + pad).to_bits()
            );
        }
    }
    assert_eq!(c.grid(25, 0), Err(Error::Coordinates));
    assert_eq!(c.grid(0, 25), Err(Error::Coordinates));
}
#[test]
fn bounds_use_filename_coordinate_axes_and_all_vertices() {
    let mesh = Mesh {
        liquid_vertices: vec![[100.0, 4.0, -100.0]],
        solid_vertices: vec![[-1.0, -3.0, 9.0], [0.0, 20.0, 0.0]],
        ..Default::default()
    };
    let b = bounds(
        Coordinate {
            row: 30,
            column: 32,
        },
        &mesh,
    )
    .unwrap();
    assert_eq!(b.min, [-GRID_SIZE, -3.0, GRID_SIZE]);
    assert_eq!(b.max, [0.0, 20.0, GRID_SIZE * 2.0]);
}
#[test]
fn empty_bounds_keep_flt_min_not_negative_max() {
    let b = bounds(
        Coordinate {
            row: 31,
            column: 31,
        },
        &Mesh::default(),
    )
    .unwrap();
    assert_eq!(b.min, [0.0, f32::MIN_POSITIVE, 0.0]);
    assert_eq!(b.max, [GRID_SIZE, f32::MAX, GRID_SIZE]);
}
#[test]
fn height_bounds_retain_native_equal_zero_operand_order() {
    let mesh = Mesh {
        liquid_vertices: vec![[0.0, 0.0, 0.0]],
        solid_vertices: vec![[0.0, -0.0, 0.0]],
        ..Default::default()
    };
    let b = bounds(Coordinate { row: 0, column: 0 }, &mesh).unwrap();
    assert_eq!(b.min[1].to_bits(), (-0.0f32).to_bits());
    assert_eq!(b.max[1].to_bits(), (-0.0f32).to_bits());
}
#[test]
fn nonfinite_geometry_and_invalid_coordinate_reject_before_generation() {
    let m = Mesh {
        solid_vertices: vec![[f32::INFINITY, 0.0, 0.0]],
        ..Default::default()
    };
    assert_eq!(
        Config::new(request(1, false), &m).err(),
        Some(Error::Numeric)
    );
    let mut r = request(1, false);
    r.tile.row = 64;
    assert_eq!(
        Config::new(r, &Mesh::default()).err(),
        Some(Error::Coordinates)
    );
}
#[test]
fn slope_options_reject_invalid_domains_and_preserve_override() {
    for (max, non) in [(f32::NAN, 55.0), (91.0, 55.0), (55.0, -1.0), (54.0, 55.0)] {
        let mut r = request(1, false);
        r.slopes = recast::Slopes {
            maximum: max,
            non_steep: non,
        };
        assert_eq!(Config::new(r, &Mesh::default()).err(), Some(Error::Slopes));
    }
    let mut r = request(1, false);
    r.slopes = recast::Slopes {
        maximum: 70.0,
        non_steep: 55.0,
    };
    assert_eq!(
        Config::new(r, &Mesh::default())
            .unwrap()
            .request
            .slopes
            .maximum,
        70.0
    );
}
#[test]
fn complete_inventory_maxima_are_independent_of_input_order() {
    let a = [
        Coordinate { row: 1, column: 63 },
        Coordinate { row: 63, column: 1 },
    ];
    let p = global_params(&a).unwrap();
    assert_eq!(p, global_params(&[a[1], a[0]]).unwrap());
    assert_eq!(
        p.origin,
        [-32.0 * GRID_SIZE, f32::MIN_POSITIVE, -32.0 * GRID_SIZE]
    );
    assert_eq!(p.max_tiles, 2);
    assert_eq!(p.max_polys, i32::MIN);
}
#[test]
fn full_inventory_and_duplicate_empty_coordinate_guards() {
    let a = (0..64)
        .flat_map(|row| (0..64).map(move |column| Coordinate { row, column }))
        .collect::<Vec<_>>();
    assert_eq!(global_params(&a).unwrap().max_tiles, 4096);
    assert_eq!(global_params(&[]), Err(Error::Inventory));
    assert_eq!(global_params(&[a[0], a[0]]), Err(Error::Inventory));
    assert_eq!(
        global_params(&[Coordinate { row: 64, column: 0 }]),
        Err(Error::Coordinates)
    );
}
#[test]
fn detour_indices_truncate_center_relative_to_origin_with_fixed_grid_size() {
    let c = cfg(562, false);
    let mut p = global_params(&[request(1, false).tile]).unwrap();
    let a = c.detour_options(p).unwrap();
    assert_eq!(a.tile, [0; 3]);
    assert_eq!(a.walkable_radius, 0.0);
    assert_eq!(a.walkable_climb, c.base_unit() * 6.0);
    p.origin[0] = GRID_SIZE;
    p.origin[2] = -GRID_SIZE;
    assert_eq!(c.detour_options(p).unwrap().tile, [0, 1, 0]);
    p.origin[0] = f32::MAX;
    assert_eq!(c.detour_options(p).err(), Some(Error::Numeric));
}
#[test]
fn tile_limits_are_downward_and_failure_identifies_subtile() {
    let r = request(1, false);
    let p = global_params(&[r.tile]).unwrap();
    for l in [
        Limits {
            heap_bytes: 0,
            ..Default::default()
        },
        Limits {
            work: Limits::default().work + 1,
            ..Default::default()
        },
    ] {
        assert_eq!(
            build(&Mesh::default(), &[], r, p, l).err(),
            Some(Error::Limit)
        );
    }
    let e = build(
        &Mesh::default(),
        &[],
        r,
        p,
        Limits {
            work: 0,
            ..Default::default()
        },
    )
    .err()
    .unwrap();
    assert_eq!(
        e,
        Error::Subtile {
            index: 0,
            source: recast::Error::Limit
        }
    );
    assert!(e.to_string().contains("0: recast_policy_limit"));
}
