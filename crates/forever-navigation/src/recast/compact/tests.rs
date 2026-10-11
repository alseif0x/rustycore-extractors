// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: synthetic area/distance semantics and resource boundaries.
use super::*;
mod corpus;

fn grid(width: u16, height: u16) -> Grid {
    Grid {
        width,
        height,
        min: [0.0; 3],
        max: [f32::from(width), 1000.0, f32::from(height)],
        cell_size: 1.0,
        cell_height: 1.0,
    }
}
fn entry(x: u16, z: u16, y: u16, area: u8) -> SpanInput {
    SpanInput {
        x,
        z,
        span: Span {
            min: y,
            max: y + 1,
            area,
        },
    }
}
fn field(w: u16, h: u16, spans: &[SpanInput]) -> Compact {
    Heightfield::new(grid(w, h), Limits::default())
        .unwrap()
        .add_spans(spans, 0)
        .unwrap()
        .compact(Agent {
            height: 1,
            climb: 0,
        })
        .unwrap()
}
fn plane(w: u16, h: u16) -> Compact {
    let spans: Vec<_> = (0..h)
        .flat_map(|z| (0..w).map(move |x| entry(x, z, 0, 11)))
        .collect();
    field(w, h, &spans)
}
fn areas(field: &Compact) -> Vec<u8> {
    field.spans().iter().map(|s| s.area).collect()
}

#[test]
fn zero_radius_retains_areas_geometry_and_links() {
    let c = plane(5, 5);
    let cells = c.cells.clone();
    let spans = c.spans.clone();
    let c = c.erode(0).unwrap();
    assert_eq!(c.cells, cells);
    assert_eq!(c.spans, spans);
}
#[test]
fn erosion_removes_radius_shell_and_retains_exact_distance() {
    let c = plane(7, 7).erode(2).unwrap();
    for z in 0..7 {
        for x in 0..7 {
            assert_eq!(
                c.spans[z * 7 + x].area,
                if (2..5).contains(&x) && (2..5).contains(&z) {
                    11
                } else {
                    0
                }
            );
        }
    }
}
#[test]
fn erosion_has_diagonal_chamfer_cost_three() {
    let mut c = plane(9, 9);
    c.spans[4 * 9 + 4].area = 0;
    let c = c.erode(1).unwrap();
    assert_eq!(c.spans[4 * 9 + 3].area, 0);
    assert_eq!(c.spans[3 * 9 + 3].area, 11);
}
#[test]
fn erosion_does_not_treat_different_nonzero_areas_as_obstacles() {
    let mut c = plane(7, 7);
    c.spans[3 * 7 + 3].area = 9;
    let c = c.erode(1).unwrap();
    assert_eq!(c.spans[3 * 7 + 3].area, 9);
    assert_eq!(c.spans[3 * 7 + 2].area, 11);
}
#[test]
fn repeated_erosion_uses_new_null_areas() {
    let c = plane(9, 9).erode(1).unwrap().erode(1).unwrap();
    assert_eq!(areas(&c), areas(&plane(9, 9).erode(2).unwrap()));
}
#[test]
fn radius_wrap_domain_rejects() {
    for radius in [128, 255, 256, u16::MAX] {
        assert!(matches!(plane(1, 1).erode(radius), Err(Error::Radius)));
    }
    assert!(plane(1, 1).erode(127).is_ok());
}
#[test]
fn median_replaces_outlier_without_erasing_nulls_or_chaining() {
    let mut c = plane(5, 5);
    c.spans[12].area = 8;
    c.spans[0].area = 0;
    let before = c.spans.clone();
    let c = c.median_filter().unwrap();
    assert_eq!(c.spans[12].area, 11);
    assert_eq!(c.spans[0].area, 0);
    for (a, b) in c.spans.iter().zip(before) {
        assert_eq!(
            (a.y, a.height, a.region, a.connections),
            (b.y, b.height, b.region, b.connections)
        );
    }
}
#[test]
fn median_uses_center_for_missing_neighbors() {
    let mut c = plane(2, 1);
    c.spans[0].area = 8;
    assert_eq!(areas(&c.median_filter().unwrap()), [8, 11]);
}
#[test]
fn distance_starts_at_area_boundaries_not_only_obstacles() {
    let mut c = plane(9, 9);
    c.spans[40].area = 8;
    let d = c.build_distance_field().unwrap();
    assert_eq!(d.values()[40], 0);
    assert_eq!(d.values()[39], 0);
    assert_eq!(d.values()[30], 2);
}
#[test]
fn maximum_is_unblurred_and_blur_rounding_is_native() {
    let d = plane(7, 7).build_distance_field().unwrap();
    assert_eq!(d.maximum(), 6);
    assert_eq!(d.values()[24], 4); // (center 6 + eight neighbors * 4 + 5) / 9
    assert_eq!(d.values()[8], 2);
    assert_eq!(d.values()[0], 0);
}
#[test]
fn null_areas_can_have_positive_distance_to_their_area_boundary() {
    let d = plane(7, 7)
        .erode(127)
        .unwrap()
        .build_distance_field()
        .unwrap();
    assert!(d.compact().spans().iter().all(|s| s.area == 0));
    assert_eq!(d.maximum(), 6);
    assert_eq!(d.values()[24], 4); // (center 6 + eight neighbors * 4 + 5) / 9
}
#[test]
fn layers_and_missing_cells_stay_disconnected() {
    let spans = [entry(0, 0, 0, 11), entry(0, 0, 10, 8), entry(2, 0, 0, 11)];
    let d = field(3, 1, &spans).prepare(0).unwrap();
    assert_eq!(d.values(), [0, 0, 0]);
    assert!(d.compact().spans().iter().all(|s| s.connections == [63; 4]));
}
#[test]
fn empty_graph_prepares_and_rebuilds_without_stale_distances() {
    let d = field(3, 3, &[]).prepare(1).unwrap();
    assert!(d.values().is_empty());
    assert_eq!(d.maximum(), 0);
    let d = plane(7, 7).prepare(1).unwrap();
    let expected = d.values().to_vec();
    let d = d.into_compact().build_distance_field().unwrap();
    assert_eq!(d.values(), expected);
}
#[test]
fn composed_prepare_matches_ordered_calls() {
    let d = plane(9, 7).prepare(2).unwrap();
    let e = plane(9, 7)
        .erode(2)
        .unwrap()
        .median_filter()
        .unwrap()
        .build_distance_field()
        .unwrap();
    assert_eq!(d.values(), e.values());
    assert_eq!(d.maximum(), e.maximum());
    assert_eq!(d.compact().spans(), e.compact().spans());
    assert_eq!(d.compact().work_used(), e.compact().work_used());
}
#[test]
fn scratch_capacity_is_charged_with_resident_graph() {
    let mut c = plane(5, 5);
    let peak = c.capacity_bytes() + c.spans.len();
    c.limits.heap_bytes = peak;
    assert!(c.erode(1).is_ok());
    let mut c = plane(5, 5);
    c.limits.heap_bytes = peak - 1;
    assert!(matches!(c.erode(1), Err(Error::Limit)));
    let mut c = plane(5, 5);
    c.limits.heap_bytes = peak;
    assert!(c.median_filter().is_ok());
    let mut c = plane(5, 5);
    c.limits.heap_bytes = peak - 1;
    assert!(matches!(c.median_filter(), Err(Error::Limit)));
    let mut c = plane(5, 5);
    let peak = c.capacity_bytes() + 4 * c.spans.len();
    c.limits.heap_bytes = peak;
    let d = c.build_distance_field().unwrap();
    assert_eq!(d.capacity_bytes(), peak - 2 * 25);
    let mut c = plane(5, 5);
    c.limits.heap_bytes = peak - 1;
    assert!(matches!(c.build_distance_field(), Err(Error::Limit)));
}
#[test]
fn work_limits_carry_from_compaction_through_all_stages() {
    let expected = plane(5, 5).prepare(1).unwrap().compact().work_used();
    let mut c = plane(5, 5);
    assert!(c.work > 0);
    c.limits.work = expected;
    assert_eq!(c.prepare(1).unwrap().compact().work_used(), expected);
    let mut c = plane(5, 5);
    c.limits.work = expected - 1;
    assert!(matches!(c.prepare(1), Err(Error::Limit)));
    let mut c = plane(1, 1);
    c.limits.work = c.work;
    assert!(matches!(c.median_filter(), Err(Error::Limit)));
}
