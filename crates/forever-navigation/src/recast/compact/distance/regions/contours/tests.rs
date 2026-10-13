// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: synthetic contour smoke checks and admission.
use super::*;
mod corpus;
fn plane(w: u16, h: u16, border: u16) -> Regions {
    let g = Grid {
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
    Heightfield::new(g, Limits::default())
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
        .build_regions(RegionOptions {
            border_size: border,
            minimum_area: 0,
            merge_area: 0,
        })
        .unwrap()
}
#[test]
fn flat_rectangle_has_four_simplified_corners() {
    let c = plane(7, 9, 0)
        .build_contours(ContourOptions::default())
        .unwrap();
    assert_eq!(c.contours().len(), 1);
    assert_eq!(c.contours()[0].vertices().len(), 4);
    assert_eq!(c.contours()[0].raw_vertices().len(), 32);
}
#[test]
fn border_padding_is_removed_from_header_and_vertices() {
    let c = plane(9, 9, 1)
        .build_contours(ContourOptions::default())
        .unwrap();
    assert_eq!(c.grid().width, 7);
    assert_eq!(c.grid().min, [1.0, 0.0, 1.0]);
    assert!(
        c.contours()
            .iter()
            .flat_map(|c| c.vertices())
            .all(|v| (0..=7).contains(&v[0]) && (0..=7).contains(&v[2]))
    );
}
#[test]
fn short_wall_limit_subdivides_edges() {
    let c = plane(9, 9, 0)
        .build_contours(ContourOptions {
            maximum_edge_length: 2,
            ..Default::default()
        })
        .unwrap();
    assert!(c.contours()[0].vertices().len() > 4);
}
#[test]
fn empty_regions_have_no_contours() {
    assert!(
        plane(1, 1, 0)
            .build_contours(ContourOptions::default())
            .unwrap()
            .contours()
            .is_empty()
    );
}
#[test]
fn invalid_numeric_and_border_domains_reject() {
    for o in [
        ContourOptions {
            maximum_error: f32::NAN,
            ..Default::default()
        },
        ContourOptions {
            maximum_error: -1.0,
            ..Default::default()
        },
        ContourOptions {
            maximum_edge_length: 46341,
            ..Default::default()
        },
    ] {
        assert!(matches!(
            plane(3, 3, 0).build_contours(o),
            Err(Error::Numeric)
        ));
    }
    assert!(matches!(
        plane(3, 3, 2).build_contours(ContourOptions::default()),
        Err(Error::Grid)
    ));
}

#[test]
fn zero_border_preserves_signed_zero_bounds() {
    let mut r = plane(7, 7, 0);
    r.field.compact.grid.min = [-0.0; 3];
    let c = r.build_contours(ContourOptions::default()).unwrap();
    assert_eq!(c.grid().min.map(f32::to_bits), [0x80000000; 3]);
}
#[test]
fn disabled_tessellation_keeps_long_edges() {
    let c = plane(9, 9, 0)
        .build_contours(ContourOptions {
            maximum_edge_length: 1,
            tessellate_walls: false,
            tessellate_areas: false,
            ..Default::default()
        })
        .unwrap();
    assert_eq!(c.contours()[0].vertices().len(), 4);
}
#[test]
fn zero_dimension_empty_contour_set_is_preserved() {
    let c = plane(4, 4, 2)
        .build_contours(ContourOptions::default())
        .unwrap();
    assert_eq!((c.grid().width, c.grid().height), (0, 0));
    assert!(c.contours().is_empty());
}
#[test]
fn contour_work_and_scratch_limits_are_enforced() {
    let total = plane(9, 9, 0)
        .build_contours(ContourOptions::default())
        .unwrap()
        .regions()
        .distance_field()
        .compact()
        .work_used();
    let mut r = plane(9, 9, 0);
    r.field.compact.limits.work = total;
    assert!(r.build_contours(ContourOptions::default()).is_ok());
    let mut r = plane(9, 9, 0);
    r.field.compact.limits.work = total - 1;
    assert!(matches!(
        r.build_contours(ContourOptions::default()),
        Err(Error::Limit)
    ));
    let mut r = plane(9, 9, 0);
    r.field.compact.limits.heap_bytes = r.capacity_bytes();
    assert!(matches!(
        r.build_contours(ContourOptions::default()),
        Err(Error::Limit)
    ));
}
fn rectangle(lo: i32, hi: i32, hole: bool) -> Contour {
    let mut vertices = vec![
        [lo, 1, lo, 0],
        [lo, 1, hi, 0],
        [hi, 1, hi, 0],
        [hi, 1, lo, 0],
    ];
    if hole {
        vertices.reverse();
    }
    Contour {
        raw: vertices.clone(),
        vertices,
        region: 1,
        area: 11,
    }
}
fn join(mut contours: Vec<Contour>) -> Result<Vec<Contour>, Error> {
    let regions = plane(15, 15, 0);
    let bytes = regions.capacity_bytes()
        + contours.capacity() * std::mem::size_of::<Contour>()
        + contours
            .iter()
            .map(|c| (c.vertices.capacity() + c.raw.capacity()) * 16)
            .sum::<usize>();
    let mut b = ContourBuilder { regions, bytes };
    b.merge_holes(&mut contours)?;
    Ok(contours)
}
#[test]
fn joined_hole_keeps_slot_raw_vertices_and_metadata() {
    let c = join(vec![rectangle(0, 10, false), rectangle(3, 7, true)]).unwrap();
    assert_eq!(c[0].vertices.len(), 10);
    assert!(c[1].vertices.is_empty());
    assert_eq!(c[1].raw.len(), 4);
    assert_eq!((c[1].region, c[1].area), (1, 11));
}
#[test]
fn missing_outline_rejects_incomplete_native_outcome() {
    assert!(matches!(
        join(vec![rectangle(3, 7, true)]),
        Err(Error::ContourHoles)
    ));
}
#[test]
fn multiple_outlines_with_holes_reject() {
    assert!(matches!(
        join(vec![
            rectangle(0, 10, false),
            rectangle(1, 9, false),
            rectangle(3, 7, true)
        ]),
        Err(Error::ContourHoles)
    ));
}
#[test]
fn unbridgeable_external_hole_rejects() {
    assert!(matches!(
        join(vec![rectangle(0, 2, false), rectangle(5, 7, true)]),
        Err(Error::ContourHoles)
    ));
}
#[test]
fn winding_accumulation_overflow_rejects() {
    let base = rectangle(0, 1024, false);
    let vertices = base.vertices.repeat(1025);
    let c = Contour {
        raw: Vec::new(),
        vertices,
        region: 1,
        area: 11,
    };
    assert!(matches!(join(vec![c]), Err(Error::Numeric)));
}
