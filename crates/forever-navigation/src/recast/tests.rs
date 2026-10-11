// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: synthetic Recast voxel and adjacency checks.
use super::*;
mod corpus;

#[test]
fn slope_defaults_follow_native_configuration_not_its_stale_comment() {
    assert_eq!(Slopes::default().maximum, 55.0);
    assert_eq!(Slopes::default().non_steep, 55.0);
    let t = [[0.0; 3], [0.0, 1.7320508, 1.0], [1.0, 0.0, 0.0]];
    assert_eq!(solid_area(t, Slopes::default()).unwrap(), 0);
    assert_eq!(
        solid_area(
            t,
            Slopes {
                maximum: 70.0,
                non_steep: 55.0
            }
        )
        .unwrap(),
        10
    );
}

#[test]
fn capacity_budgets_include_growth_and_compaction_overlap() {
    let node = std::mem::size_of::<Node>();
    let cells = std::mem::size_of::<Cell>();
    let compact_span = std::mem::size_of::<CompactSpan>();
    let peak = 4 + node + cells + compact_span;
    let make = |heap_bytes| {
        Heightfield::new(
            grid(1, 1),
            Limits {
                spans: 1,
                heap_bytes,
                ..Default::default()
            },
        )
        .unwrap()
        .add_spans(&[span(0, 0, 0, 1, 11)], 0)
        .unwrap()
    };
    assert!(
        make(peak)
            .compact(Agent {
                height: 1,
                climb: 0
            })
            .is_ok()
    );
    assert!(matches!(
        make(peak - 1).compact(Agent {
            height: 1,
            climb: 0
        }),
        Err(Error::Limit)
    ));
    let values: Vec<_> = (0..64).map(|i| span(0, 0, 3 * i, 3 * i + 1, 11)).collect();
    let make = |heap_bytes| {
        Heightfield::new(
            grid(1, 1),
            Limits {
                spans: 128,
                heap_bytes,
                ..Default::default()
            },
        )
        .unwrap()
        .add_spans(&values, 0)
        .unwrap()
    };
    let peak = 4 + (64 + 128) * node;
    assert!(make(peak).add_spans(&[span(0, 0, 300, 301, 11)], 0).is_ok());
    assert!(matches!(
        make(peak - 1).add_spans(&[span(0, 0, 300, 301, 11)], 0),
        Err(Error::Limit)
    ));
}

#[test]
fn exact_cumulative_work_budget_is_admitted() {
    let run = |work| {
        Heightfield::new(
            grid(3, 3),
            Limits {
                work,
                ..Default::default()
            },
        )
        .unwrap()
        .add_spans(&[span(1, 1, 0, 1, 11)], 0)
        .unwrap()
        .filter_low_obstacles(1)
        .unwrap()
        .filter_ledges(Agent {
            height: 2,
            climb: 1,
        })
        .unwrap()
        .filter_clearance(2)
        .unwrap()
        .compact(Agent {
            height: 2,
            climb: 1,
        })
    };
    let total = run(Limits::default().work).unwrap().work_used();
    assert_eq!(run(total).unwrap().work_used(), total);
    assert!(matches!(run(total - 1), Err(Error::Limit)));
}

#[test]
fn derived_grid_and_raster_cast_overflows_reject() {
    for g in [
        Grid {
            cell_size: f32::from_bits(1),
            ..grid(1, 1)
        },
        Grid {
            min: [-f32::MAX, 0.0, 0.0],
            max: [f32::MAX, 1.0, 1.0],
            ..grid(1, 1)
        },
    ] {
        assert!(matches!(
            Heightfield::new(g, Limits::default()),
            Err(Error::Numeric)
        ));
    }
    let v = [[-1e20, 1.0, -1e20], [1e20, 1.0, 1e20], [0.0, 1.0, 1.0]];
    assert!(matches!(
        field(1, 1, &[]).rasterize(&v, &[[0, 1, 2]], &[1], 0),
        Err(Error::Numeric)
    ));
    let g = Grid {
        max: [1.0, f32::MAX, 1.0],
        cell_height: f32::MAX,
        ..grid(1, 1)
    };
    assert!(matches!(
        Heightfield::new(g, Limits::default())
            .unwrap()
            .compact(Agent {
                height: 2,
                climb: 0
            }),
        Err(Error::Numeric)
    ));
}

#[test]
fn invalid_span_area_coordinate_and_raster_area_reject() {
    for s in [
        span(1, 0, 0, 1, 1),
        span(0, 1, 0, 1, 1),
        span(0, 0, 0, 1, 64),
        span(0, 0, 2, 1, 1),
    ] {
        assert!(matches!(
            field(1, 1, &[]).add_spans(&[s], 0),
            Err(Error::Span)
        ));
    }
    assert!(matches!(
        field(1, 1, &[]).rasterize(&[[0.0; 3]], &[[0, 0, 0]], &[64], 0),
        Err(Error::Geometry)
    ));
    assert!(field(1, 1, &[]).spans(1, 0).is_none());
}

#[test]
fn highest_representable_span_and_clearance_are_preserved() {
    let h = field(1, 1, &[span(0, 0, 65534, 65535, 11)]);
    assert_eq!(column(&h, 0, 0)[0].max, 65535);
    assert_eq!(
        h.compact(Agent {
            height: 0,
            climb: 0
        })
        .unwrap()
        .spans()[0]
            .height,
        0
    );
    let h = field(1, 1, &[span(0, 0, 65534, 65535, 11)])
        .filter_clearance(0)
        .unwrap();
    assert_eq!(column(&h, 0, 0)[0].area, 0);
}

#[test]
fn compact_uses_equal_gap_and_first_reachable_layer() {
    let s = [
        span(0, 0, 0, 2, 11),
        span(0, 0, 4, 5, 0),
        span(1, 0, 0, 2, 9),
        span(1, 0, 4, 5, 8),
    ];
    let c = field(2, 1, &s)
        .compact(Agent {
            height: 2,
            climb: 3,
        })
        .unwrap();
    assert_eq!(c.spans()[0].connections[2], 0);
    let c = field(2, 1, &s)
        .compact(Agent {
            height: 3,
            climb: 3,
        })
        .unwrap();
    assert_eq!(c.spans()[0].connections[2], 63);
}

#[test]
fn build_rasterizes_liquids_after_border_filtering() {
    let m = crate::Mesh {
        solid_vertices: vec![[0.0, 1.0, 0.0], [0.0, 1.0, 1.0], [1.0, 1.0, 0.0]],
        solid_triangles: vec![[0, 1, 2]],
        liquid_vertices: vec![[0.0, 1.0, 0.0], [0.0, 1.0, 1.0], [1.0, 1.0, 0.0]],
        liquid_triangles: vec![[0, 1, 2]],
        liquid_areas: vec![9],
    };
    let c = build(
        &m,
        grid(1, 1),
        Slopes::default(),
        Agent {
            height: 2,
            climb: 1,
        },
        Limits::default(),
    )
    .unwrap();
    assert_eq!(c.spans().len(), 1);
    assert_eq!(c.spans()[0].area, 9);
}
fn grid(w: u16, h: u16) -> Grid {
    Grid {
        width: w,
        height: h,
        min: [0.0; 3],
        max: [f32::from(w), 100.0, f32::from(h)],
        cell_size: 1.0,
        cell_height: 1.0,
    }
}
fn span(x: u16, z: u16, min: u16, max: u16, area: u8) -> SpanInput {
    SpanInput {
        x,
        z,
        span: Span { min, max, area },
    }
}
fn field(w: u16, h: u16, spans: &[SpanInput]) -> Heightfield {
    Heightfield::new(grid(w, h), Limits::default())
        .unwrap()
        .add_spans(spans, 0)
        .unwrap()
}
fn column(h: &Heightfield, x: u16, z: u16) -> Vec<Span> {
    h.spans(x, z).unwrap().collect()
}

#[test]
fn sorted_spans_merge_touching_intervals() {
    let h = field(
        1,
        1,
        &[
            span(0, 0, 8, 10, 8),
            span(0, 0, 0, 2, 1),
            span(0, 0, 5, 8, 3),
            span(0, 0, 3, 4, 2),
        ],
    );
    assert_eq!(
        column(&h, 0, 0),
        [
            Span {
                min: 0,
                max: 2,
                area: 1
            },
            Span {
                min: 3,
                max: 4,
                area: 2
            },
            Span {
                min: 5,
                max: 10,
                area: 3
            }
        ]
    );
    assert_eq!(h.span_count(), 3);
}
#[test]
fn forever_area_merge_compares_endpoints_before_expanding() {
    let h = field(1, 1, &[span(0, 0, 0, 10, 11), span(0, 0, 0, 2, 8)]);
    assert_eq!(column(&h, 0, 0)[0].area, 8);
    let h = Heightfield::new(grid(1, 1), Limits::default())
        .unwrap()
        .add_spans(&[span(0, 0, 0, 10, 11), span(0, 0, 0, 2, 8)], 8)
        .unwrap();
    assert_eq!(column(&h, 0, 0)[0].area, 11);
}
#[test]
fn deleted_arena_entries_are_reused() {
    let h = Heightfield::new(
        grid(1, 1),
        Limits {
            spans: 2,
            ..Default::default()
        },
    )
    .unwrap();
    let h = h
        .add_spans(&[span(0, 0, 0, 1, 1), span(0, 0, 0, 2, 2)], 0)
        .unwrap();
    let h = h.add_spans(&[span(0, 0, 0, 3, 3)], 0).unwrap();
    assert_eq!(h.nodes.len(), 2);
    assert_eq!(h.span_count(), 1);
}
#[test]
fn slope_winding_and_degenerate_comparison_semantics() {
    let t = [[0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]];
    assert_eq!(solid_area(t, Slopes::default()).unwrap(), 11);
    assert_eq!(
        solid_area([t[0], t[2], t[1]], Slopes::default()).unwrap(),
        0
    );
    assert_eq!(solid_area([[0.0; 3]; 3], Slopes::default()).unwrap(), 10);
    assert_eq!(
        solid_area(
            [[0.0; 3], [0.0, 3.0, 1.0], [1.0, 0.0, 0.0]],
            Slopes::default()
        )
        .unwrap(),
        0
    );
}
#[test]
fn raster_flat_triangle_has_positive_thickness() {
    let v = [[0.0, 2.0, 0.0], [0.0, 2.0, 2.0], [2.0, 2.0, 0.0]];
    let h = Heightfield::new(grid(2, 2), Limits::default())
        .unwrap()
        .rasterize(&v, &[[0, 1, 2]], &[11], 0)
        .unwrap();
    assert!(h.span_count() > 0);
    assert!(
        column(&h, 0, 0)
            .iter()
            .all(|s| s.min == 2 && s.max == 3 && s.area == 11)
    );
}
#[test]
fn triangles_outside_box_emit_no_spans() {
    let v = [[0.0, 200.0, 0.0], [0.0, 200.0, 2.0], [2.0, 200.0, 0.0]];
    let h = Heightfield::new(grid(2, 2), Limits::default())
        .unwrap()
        .rasterize(&v, &[[0, 1, 2]], &[1], 0)
        .unwrap();
    assert_eq!(h.span_count(), 0);
}
#[test]
fn low_obstacles_do_not_propagate_through_multiple_blockers() {
    let h = field(
        1,
        1,
        &[
            span(0, 0, 0, 2, 11),
            span(0, 0, 3, 4, 0),
            span(0, 0, 5, 6, 0),
        ],
    )
    .filter_low_obstacles(4)
    .unwrap();
    assert_eq!(
        column(&h, 0, 0).iter().map(|s| s.area).collect::<Vec<_>>(),
        [11, 11, 0]
    );
}
#[test]
fn clearance_uses_inclusive_height_rejection() {
    let h = field(1, 1, &[span(0, 0, 0, 2, 11), span(0, 0, 4, 5, 11)])
        .filter_clearance(2)
        .unwrap();
    assert_eq!(
        column(&h, 0, 0).iter().map(|s| s.area).collect::<Vec<_>>(),
        [0, 11]
    );
}
#[test]
fn ledge_filter_removes_border_and_keeps_supported_center() {
    let s: Vec<_> = (0..3)
        .flat_map(|z| (0..3).map(move |x| span(x, z, 0, 1, 11)))
        .collect();
    let h = field(3, 3, &s)
        .filter_ledges(Agent {
            height: 2,
            climb: 1,
        })
        .unwrap();
    for z in 0..3 {
        for x in 0..3 {
            assert_eq!(
                column(&h, x, z)[0].area,
                if x == 1 && z == 1 { 11 } else { 0 }
            );
        }
    }
}
#[test]
fn compact_empty_and_null_only_cells_keep_distinct_indices() {
    let h = field(3, 1, &[span(0, 0, 0, 1, 11), span(2, 0, 0, 1, 0)]);
    let c = h
        .compact(Agent {
            height: 2,
            climb: 1,
        })
        .unwrap();
    assert_eq!(
        c.cells(),
        &[
            Cell { index: 0, count: 1 },
            Cell { index: 0, count: 0 },
            Cell { index: 1, count: 0 }
        ]
    );
    assert_eq!(c.spans()[0].height, 255);
    assert_eq!(c.grid().max[1], 102.0);
}
#[test]
fn compact_connects_first_eligible_neighbor_in_each_direction() {
    let s: Vec<_> = (0..2)
        .flat_map(|z| (0..2).map(move |x| span(x, z, 0, 1, 11)))
        .collect();
    let c = field(2, 2, &s)
        .compact(Agent {
            height: 2,
            climb: 0,
        })
        .unwrap();
    assert_eq!(c.spans()[0].connections, [63, 0, 0, 63]);
    assert_eq!(c.spans()[3].connections, [0, 63, 63, 0]);
}
#[test]
fn compact_height_saturates_and_region_starts_zero() {
    let c = field(1, 1, &[span(0, 0, 0, 1, 8), span(0, 0, 1000, 1001, 9)])
        .compact(Agent {
            height: 1,
            climb: 1,
        })
        .unwrap();
    assert_eq!(c.spans()[0].height, 255);
    assert!(c.spans().iter().all(|s| s.region == 0));
}
#[test]
fn compact_rejects_cell_count_overflow_and_accessible_layer_63() {
    let s: Vec<_> = (0..256).map(|i| span(0, 0, i * 3, i * 3 + 1, 11)).collect();
    assert!(matches!(
        field(1, 1, &s).compact(Agent {
            height: 1,
            climb: 0
        }),
        Err(Error::Layers)
    ));
    let s: Vec<_> = (0..64)
        .flat_map(|i| {
            [
                span(0, 0, i * 3, i * 3 + 1, 11),
                span(1, 0, i * 3, i * 3 + 1, 11),
            ]
        })
        .collect();
    assert!(matches!(
        field(2, 1, &s).compact(Agent {
            height: 1,
            climb: 0
        }),
        Err(Error::Layers)
    ));
    let s: Vec<_> = (0..255).map(|i| span(0, 0, i * 3, i * 3 + 1, 11)).collect();
    assert_eq!(
        field(1, 1, &s)
            .compact(Agent {
                height: 1,
                climb: 0
            })
            .unwrap()
            .spans()
            .len(),
        255
    );
}
#[test]
fn malformed_numeric_grid_geometry_and_spans_reject() {
    for g in [
        Grid {
            width: 0,
            ..grid(1, 1)
        },
        Grid {
            width: 1025,
            ..grid(1, 1)
        },
        Grid {
            cell_height: 0.0,
            ..grid(1, 1)
        },
        Grid {
            cell_size: f32::NAN,
            ..grid(1, 1)
        },
    ] {
        assert!(Heightfield::new(g, Limits::default()).is_err());
    }
    assert!(matches!(
        field(1, 1, &[]).add_spans(&[span(0, 0, 2, 2, 1)], 0),
        Err(Error::Span)
    ));
    assert!(matches!(
        field(1, 1, &[]).rasterize(&[[0.0; 3]], &[[0, 1, 0]], &[1], 0),
        Err(Error::Geometry)
    ));
    assert!(matches!(
        field(1, 1, &[]).rasterize(&[[0.0; 3]], &[[0, 0, 0]], &[], 0),
        Err(Error::Geometry)
    ));
    assert!(matches!(
        field(1, 1, &[]).rasterize(&[[f32::NAN; 3]], &[], &[], 0),
        Err(Error::Geometry)
    ));
    assert!(
        solid_area(
            [[0.0; 3]; 3],
            Slopes {
                maximum: 55.0,
                non_steep: 70.0
            }
        )
        .is_err()
    );
}
#[test]
fn span_upper_wrap_domain_is_rejected() {
    let g = Grid {
        max: [1.0, 65536.0, 1.0],
        ..grid(1, 1)
    };
    let v = [
        [0.0, 65535.0, 0.0],
        [0.0, 65535.0, 1.0],
        [1.0, 65535.0, 0.0],
    ];
    assert!(matches!(
        Heightfield::new(g, Limits::default())
            .unwrap()
            .rasterize(&v, &[[0, 1, 2]], &[11], 0),
        Err(Error::Numeric)
    ));
}
#[test]
fn downward_limits_and_cumulative_work_fail_closed() {
    for limits in [
        Limits {
            cells: 0,
            ..Default::default()
        },
        Limits {
            heap_bytes: 0,
            ..Default::default()
        },
        Limits {
            spans: usize::MAX,
            ..Default::default()
        },
        Limits {
            work: u64::MAX,
            ..Default::default()
        },
    ] {
        assert!(Heightfield::new(grid(1, 1), limits).is_err());
    }
    let h = Heightfield::new(
        grid(1, 1),
        Limits {
            work: 2,
            ..Default::default()
        },
    )
    .unwrap()
    .add_spans(&[span(0, 0, 0, 1, 11)], 0)
    .unwrap();
    assert!(matches!(h.filter_clearance(1), Err(Error::Limit)));
    let h = Heightfield::new(
        grid(1, 1),
        Limits {
            spans: 1,
            ..Default::default()
        },
    )
    .unwrap()
    .add_spans(&[span(0, 0, 0, 1, 11)], 0)
    .unwrap();
    assert!(matches!(
        h.add_spans(&[span(0, 0, 0, 2, 11)], 0),
        Err(Error::Limit)
    ));
}
