// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: synthetic watershed smoke checks, extended by native corpus.
use super::*;
mod corpus;
fn field(w: u16, h: u16) -> DistanceField {
    let grid = Grid {
        width: w,
        height: h,
        min: [0.0; 3],
        max: [f32::from(w), 100.0, f32::from(h)],
        cell_size: 1.0,
        cell_height: 1.0,
    };
    let spans: Vec<_> = (0..h)
        .flat_map(|z| {
            (0..w).map(move |x| SpanInput {
                x,
                z,
                span: Span {
                    min: 0,
                    max: 1,
                    area: 11,
                },
            })
        })
        .collect();
    Heightfield::new(grid, Limits::default())
        .unwrap()
        .add_spans(&spans, 0)
        .unwrap()
        .compact(Agent {
            height: 1,
            climb: 0,
        })
        .unwrap()
        .build_distance_field()
        .unwrap()
}
fn options() -> RegionOptions {
    RegionOptions {
        border_size: 0,
        minimum_area: 0,
        merge_area: 0,
    }
}

fn supplied(w: u16, h: u16, spans: &[SpanInput]) -> DistanceField {
    let g = Grid {
        width: w,
        height: h,
        min: [0.0; 3],
        max: [f32::from(w), 100.0, f32::from(h)],
        cell_size: 1.0,
        cell_height: 1.0,
    };
    Heightfield::new(g, Limits::default())
        .unwrap()
        .add_spans(spans, 0)
        .unwrap()
        .compact(Agent {
            height: 1,
            climb: 0,
        })
        .unwrap()
        .build_distance_field()
        .unwrap()
}
fn span(x: u16, z: u16, y: u16) -> SpanInput {
    SpanInput {
        x,
        z,
        span: Span {
            min: y,
            max: y + 1,
            area: 11,
        },
    }
}

#[test]
fn exact_minimum_area_retains_region() {
    let r = field(7, 7)
        .build_regions(RegionOptions {
            minimum_area: 49,
            ..options()
        })
        .unwrap();
    assert!(
        r.distance_field()
            .compact()
            .spans()
            .iter()
            .all(|s| s.region == 1)
    );
}
#[test]
fn wide_border_uses_last_paint_and_preserves_native_empty_ids() {
    let r = field(7, 7)
        .build_regions(RegionOptions {
            border_size: 65535,
            ..options()
        })
        .unwrap();
    assert_eq!(r.maximum(), 5);
    assert!(
        r.distance_field()
            .compact()
            .spans()
            .iter()
            .all(|s| s.region == BORDER | 4)
    );
}
#[test]
fn graph_areas_and_distance_values_stay_bound() {
    let d = field(9, 9);
    let before = d.compact.spans.clone();
    let cells = d.compact.cells.clone();
    let values = d.values.clone();
    let maximum = d.maximum;
    let r = d.build_regions(options()).unwrap();
    assert_eq!(r.distance_field().values(), values);
    assert_eq!(r.distance_field().maximum(), maximum);
    assert_eq!(r.distance_field().compact().cells(), cells);
    for (a, b) in before.iter().zip(r.distance_field().compact().spans()) {
        assert_eq!(
            (a.y, a.height, a.area, a.connections),
            (b.y, b.height, b.area, b.connections)
        );
    }
}
#[test]
fn overlapping_floor_diagnostic_retains_id_and_order() {
    let spans: Vec<_> = (0..3)
        .flat_map(|z| (0..3).flat_map(move |x| [span(x, z, 0), span(x, z, 10)]))
        .collect();
    let d = supplied(3, 3, &spans);
    let n = d.values.len();
    let bytes = d.capacity_bytes() + 4 * n;
    let mut b = Builder {
        field: d,
        bytes,
        reg: vec![1; n],
        distance: vec![0; n],
    };
    let (maximum, overlaps) = b.merge_and_filter(2, options()).unwrap();
    assert_eq!(maximum, 2);
    assert_eq!(overlaps, [1]);
    assert!(b.reg.iter().all(|&r| r == 1));
}
#[test]
fn region_stage_work_budget_is_cumulative_and_exact() {
    let total = field(9, 9)
        .build_regions(options())
        .unwrap()
        .distance_field()
        .compact()
        .work_used();
    let mut d = field(9, 9);
    d.compact.limits.work = total;
    assert_eq!(
        d.build_regions(options())
            .unwrap()
            .distance_field()
            .compact()
            .work_used(),
        total
    );
    let mut d = field(9, 9);
    d.compact.limits.work = total - 1;
    assert!(matches!(d.build_regions(options()), Err(Error::Limit)));
    let mut d = field(9, 9);
    d.compact.limits.heap_bytes = d.capacity_bytes();
    assert!(matches!(d.build_regions(options()), Err(Error::Limit)));
}
#[test]
fn growth_charges_old_and_new_region_capacity_together() {
    for shortage in [0, 1] {
        let d = field(1, 1);
        let bytes = d.capacity_bytes();
        let mut b = Builder {
            field: d,
            bytes,
            reg: Vec::new(),
            distance: Vec::new(),
        };
        b.field.compact.limits.heap_bytes = bytes + 16 * 2 + 32 * 2 - shortage;
        let mut v: Vec<u16> = Vec::new();
        b.reserve(&mut v, 16).unwrap();
        v.resize(16, 0);
        let result = b.reserve(&mut v, 16);
        if shortage == 0 {
            assert!(result.is_ok());
            assert_eq!(b.bytes, bytes + 64);
        } else {
            assert_eq!(result, Err(Error::Limit));
        }
    }
}
#[test]
fn high_region_ids_reject_before_border_flag_collision() {
    let spans: Vec<_> = (0..768)
        .flat_map(|z| {
            (0..768)
                .filter(move |&x| x % 4 != 3 && z % 4 != 3)
                .map(move |x| span(x, z, 0))
        })
        .collect();
    assert!(matches!(
        supplied(768, 768, &spans).build_regions(options()),
        Err(Error::Regions)
    ));
}
#[test]
fn incomplete_native_contour_guard_rejects_large_serpentine() {
    let mut spans = Vec::new();
    for z in 0..160 {
        for x in 0..1024 {
            if z % 4 != 3 || if (z / 4) % 2 == 0 { x >= 1021 } else { x < 3 } {
                spans.push(span(x, z, 0));
            }
        }
    }
    assert!(matches!(
        supplied(1024, 160, &spans).build_regions(options()),
        Err(Error::RegionContour)
    ));
}
#[test]
fn flat_plane_has_one_populated_region_and_native_unused_id() {
    let r = field(7, 7).build_regions(options()).unwrap();
    assert!(r.overlapping_regions().is_empty());
    assert_eq!(r.maximum(), 2);
    assert!(
        r.distance_field()
            .compact()
            .spans()
            .iter()
            .all(|s| s.region == 1)
    );
}
#[test]
fn zero_distance_does_not_start_a_region() {
    let r = field(1, 1).build_regions(options()).unwrap();
    assert_eq!(r.maximum(), 1);
    assert_eq!(r.distance_field().compact().spans()[0].region, 0);
}
#[test]
fn minimum_area_removes_whole_island() {
    let r = field(7, 7)
        .build_regions(RegionOptions {
            minimum_area: 50,
            ..options()
        })
        .unwrap();
    assert!(
        r.distance_field()
            .compact()
            .spans()
            .iter()
            .all(|s| s.region == 0)
    );
}
#[test]
fn painted_border_protects_small_regions() {
    let r = field(7, 7)
        .build_regions(RegionOptions {
            border_size: 1,
            minimum_area: 1000,
            ..options()
        })
        .unwrap();
    assert_ne!(r.distance_field().compact().spans()[24].region, 0);
    assert!(r.distance_field().compact().spans()[0].region & BORDER != 0);
}
#[test]
fn invalid_unsigned_native_threshold_rejects() {
    assert!(matches!(
        field(1, 1).build_regions(RegionOptions {
            minimum_area: u32::MAX,
            ..options()
        }),
        Err(Error::Numeric)
    ));
}
