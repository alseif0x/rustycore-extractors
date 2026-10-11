// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: synthetic detail geometry, heights and admission.
use super::*;
mod corpus;
fn options() -> DetailOptions {
    DetailOptions {
        sample_distance: 4.0,
        maximum_error: 0.5,
    }
}
fn mesh(pattern: u8, border: u16) -> PolyMesh {
    let grid = Grid {
        width: 17,
        height: 17,
        min: [10.0, 20.0, -10.0],
        max: [27.0, 120.0, 7.0],
        cell_size: 1.0,
        cell_height: 0.5,
    };
    let spans: Vec<_> = (0..17)
        .flat_map(|z| {
            (0..17).map(move |x| {
                let y = if pattern == 1 { (x + z) % 5 } else { 0 };
                SpanInput {
                    x,
                    z,
                    span: Span {
                        min: y,
                        max: y + 1,
                        area: 11,
                    },
                }
            })
        })
        .collect();
    let f = Heightfield::new(grid, Limits::default()).unwrap();
    let f = if pattern == 255 {
        f
    } else {
        f.add_spans(&spans, 0).unwrap()
    };
    f.compact(Agent {
        height: 1,
        climb: 8,
    })
    .unwrap()
    .build_distance_field()
    .unwrap()
    .build_regions(RegionOptions {
        border_size: border,
        minimum_area: 0,
        merge_area: 2500,
    })
    .unwrap()
    .build_contours(ContourOptions::default())
    .unwrap()
    .build_mesh(6)
    .unwrap()
}
fn budget() -> Budget {
    Budget {
        limits: Limits::default(),
        work: 0,
        bytes: 0,
    }
}
#[test]
fn flat_detail_keeps_world_origin_and_extra_y_cell() {
    let d = mesh(0, 0).build_detail(options()).unwrap();
    assert_eq!(d.submeshes.len(), 1);
    assert_eq!(d.vertices.len(), 4);
    assert_eq!(d.triangles.len(), 2);
    assert!(d.vertices.iter().all(|v| v[1] == 21.0));
    assert!(d.vertices.iter().any(|v| v[0] == 10.0 && v[2] == -10.0));
    assert_eq!(
        d.triangles.iter().map(|t| t[3].count_ones()).sum::<u32>(),
        4
    );
}
#[test]
fn empty_mesh_has_empty_detail() {
    let d = mesh(255, 0).build_detail(options()).unwrap();
    assert!(d.vertices.is_empty() && d.triangles.is_empty() && d.submeshes.is_empty());
}
#[test]
fn rough_field_adds_sample_vertices_and_triangles() {
    let d = mesh(1, 1)
        .build_detail(DetailOptions {
            sample_distance: 2.0,
            maximum_error: 0.1,
        })
        .unwrap();
    assert!(d.vertices.len() > d.mesh.polygons.iter().map(Polygon::len).sum::<usize>());
    assert!(d.triangles.len() > 2);
    for s in &d.submeshes {
        assert!(s.vertex_count <= 127 && s.triangle_count <= 255);
    }
}
#[test]
fn unavailable_sample_options_reject() {
    for distance in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        assert!(matches!(
            mesh(0, 0).build_detail(DetailOptions {
                sample_distance: distance,
                ..options()
            }),
            Err(Error::Numeric)
        ));
    }
    for error in [-1.0, f32::NAN, f32::INFINITY, f32::MAX] {
        assert!(matches!(
            mesh(0, 0).build_detail(DetailOptions {
                maximum_error: error,
                ..options()
            }),
            Err(Error::Numeric)
        ));
    }
}
#[test]
fn unrepresentable_sampling_ratios_reject() {
    assert!(matches!(
        mesh(0, 0).build_detail(DetailOptions {
            sample_distance: f32::MIN_POSITIVE,
            ..options()
        }),
        Err(Error::Numeric)
    ));
    let mut m = mesh(0, 0);
    m.contours.options.maximum_error = 23170.0;
    assert!(matches!(m.build_detail(options()), Err(Error::Numeric)));
}
#[test]
fn nearest_height_ring_wins_over_distant_better_height() {
    let mut patch = Patch {
        min: [0, 0],
        width: 5,
        height: 5,
        data: vec![NULL; 25],
    };
    patch.data[3 + 2 * 5] = 100;
    patch.data[4 + 2 * 5] = 7;
    assert_eq!(
        budget()
            .sample_height([2.0, 7.0, 2.0], 1.0, 1.0, 2, &patch)
            .unwrap(),
        100
    );
}
#[test]
fn equal_height_error_keeps_first_spiral_candidate() {
    let mut patch = Patch {
        min: [0, 0],
        width: 5,
        height: 5,
        data: vec![NULL; 25],
    };
    patch.data[3 + 2 * 5] = 5;
    patch.data[2 + 3 * 5] = 9;
    assert_eq!(
        budget()
            .sample_height([2.0, 7.0, 2.0], 1.0, 1.0, 2, &patch)
            .unwrap(),
        5
    );
}
#[test]
fn missing_sample_height_rejects_sentinel_output() {
    let patch = Patch {
        min: [0, 0],
        width: 3,
        height: 3,
        data: vec![NULL; 9],
    };
    assert!(matches!(
        budget().sample_height([1.0, 0.0, 1.0], 1.0, 1.0, 1, &patch),
        Err(Error::DetailHeight)
    ));
}
#[test]
fn mixed_and_missing_region_seeds_match_flat_heights() {
    let expected = mesh(0, 1).build_detail(options()).unwrap();
    for region in [0, 30000] {
        let mut m = mesh(0, 1);
        for p in &mut m.polygons {
            p.region = region;
        }
        let d = m.build_detail(options()).unwrap();
        assert_eq!(d.vertices, expected.vertices);
        assert_eq!(d.triangles, expected.triangles);
    }
}
#[test]
fn absent_compact_seeds_reject_without_assertion() {
    let mut m = mesh(0, 0);
    m.contours.regions.field.compact.spans.clear();
    m.contours
        .regions
        .field
        .compact
        .cells
        .fill(Cell { index: 0, count: 0 });
    assert!(matches!(
        m.build_detail(options()),
        Err(Error::DetailHeight)
    ));
}
#[test]
fn detail_work_and_heap_limits_are_inherited() {
    let d = mesh(0, 0).build_detail(options()).unwrap();
    let total = d.mesh.contours.regions.field.compact.work;
    let mut m = mesh(0, 0);
    m.contours.regions.field.compact.limits.work = total;
    assert!(m.build_detail(options()).is_ok());
    let mut m = mesh(0, 0);
    m.contours.regions.field.compact.limits.work = total - 1;
    assert!(matches!(m.build_detail(options()), Err(Error::Limit)));
    let mut m = mesh(0, 0);
    m.contours.regions.field.compact.limits.heap_bytes = m.capacity_bytes();
    assert!(matches!(m.build_detail(options()), Err(Error::Limit)));
}
#[test]
fn degenerate_triangle_returns_no_height_hit() {
    assert_eq!(
        triangle_height([1.0, 2.0, 0.0], [0.0; 3], [1.0, 0.0, 0.0], [2.0, 0.0, 0.0]),
        f32::MAX
    );
}

#[test]
fn no_face_delaunay_rejects_empty_triangulation() {
    let points = [[0.0; 3], [1.0, 0.0, 0.0], [2.0, 0.0, 0.0], [3.0, 0.0, 0.0]];
    assert!(matches!(
        budget().delaunay(&points, &[0, 1, 2, 3]),
        Err(Error::DetailTriangulation)
    ));
}
#[test]
fn dangling_faces_use_native_swap_removal_order() {
    let mut records = vec![
        [0, 1, 2],
        [0, 2, -1],
        [-1, 1, 2],
        [3, 4, 5],
        [6, 7, 8],
        [-1; 3],
    ];
    assert_eq!(budget().discard_detail_faces(&mut records).unwrap(), 3);
    assert_eq!(records, [[0, 1, 2], [6, 7, 8], [3, 4, 5]]);
    assert_eq!(budget().discard_detail_faces(&mut records).unwrap(), 0);
}
#[test]
fn disconnected_center_walk_rejects_last_cell_fallback() {
    let mut m = mesh(0, 0);
    for p in &mut m.polygons {
        p.region = 0;
    }
    for s in &mut m.contours.regions.field.compact.spans {
        s.connections = [NOT_CONNECTED; 4];
    }
    assert!(matches!(
        m.build_detail(options()),
        Err(Error::DetailHeight)
    ));
}
#[test]
fn compact_height_cannot_alias_unset_patch_marker() {
    let mut m = mesh(0, 0);
    for s in &mut m.contours.regions.field.compact.spans {
        s.y = NULL;
    }
    assert!(matches!(
        m.build_detail(options()),
        Err(Error::DetailHeight)
    ));
}
