// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: synthetic polygon construction and strict admission.
use super::*;
mod corpus;
fn empty() -> ContourSet {
    let grid = Grid {
        width: 32,
        height: 32,
        min: [0.0; 3],
        max: [32.0, 100.0, 32.0],
        cell_size: 1.0,
        cell_height: 1.0,
    };
    Heightfield::new(grid, Limits::default())
        .unwrap()
        .compact(Agent {
            height: 1,
            climb: 0,
        })
        .unwrap()
        .build_distance_field()
        .unwrap()
        .build_regions(RegionOptions {
            border_size: 0,
            minimum_area: 0,
            merge_area: 0,
        })
        .unwrap()
        .build_contours(ContourOptions::default())
        .unwrap()
}
fn contour(points: &[[i32; 2]], region: u16, area: u8) -> Contour {
    let vertices: Vec<_> = points.iter().map(|v| [v[0], 1, v[1], 0]).collect();
    Contour {
        raw: vertices.clone(),
        vertices,
        region,
        area,
    }
}
fn square() -> Contour {
    contour(&[[0, 0], [0, 8], [8, 8], [8, 0]], 1, 11)
}
fn supplied(contours: Vec<Contour>) -> ContourSet {
    let mut c = empty();
    c.contours = contours;
    c
}
fn budget() -> Budget {
    Budget {
        limits: Limits::default(),
        work: 0,
        bytes: 0,
    }
}
#[test]
fn empty_mesh_preserves_native_metadata() {
    let m = empty().build_mesh(6).unwrap();
    assert!(m.vertices().is_empty());
    assert!(m.polygons().is_empty());
    assert_eq!(m.maximum_polygons(), 0);
}
#[test]
fn rectangle_merges_to_one_quad_or_two_triangles() {
    for nvp in [3, 4, 5, 6] {
        let m = supplied(vec![square()]).build_mesh(nvp).unwrap();
        assert_eq!(m.vertices().len(), 4);
        assert_eq!(m.maximum_polygons(), 2);
        assert_eq!(m.polygons().len(), if nvp == 3 { 2 } else { 1 });
        assert!(
            m.polygons()
                .iter()
                .all(|p| p.flags() == 0 && p.area() == 11 && p.region() == 1)
        );
    }
}
#[test]
fn triangle_adjacency_is_reciprocal() {
    let m = supplied(vec![square()]).build_mesh(3).unwrap();
    assert_eq!(
        m.polygons()[0]
            .neighbors()
            .iter()
            .filter(|&&n| n == 1)
            .count(),
        1
    );
    assert_eq!(
        m.polygons()[1]
            .neighbors()
            .iter()
            .filter(|&&n| n == 0)
            .count(),
        1
    );
}
#[test]
fn padding_emits_four_directional_portals() {
    let mut c = supplied(vec![square()]);
    c.grid.width = 8;
    c.grid.height = 8;
    c.regions.options.border_size = 1;
    let m = c.build_mesh(6).unwrap();
    let mut edges = m.polygons()[0].neighbors().to_vec();
    edges.sort();
    assert_eq!(edges, [0x8000, 0x8001, 0x8002, 0x8003]);
}
#[test]
fn y_tolerance_welds_at_two_and_keeps_three() {
    for (height, count) in [(3, 4), (4, 8)] {
        let a = square();
        let mut b = square();
        for v in &mut b.vertices {
            v[1] = height;
        }
        let m = supplied(vec![a, b]).build_mesh(6).unwrap();
        assert_eq!(m.vertices().len(), count);
    }
}
#[test]
fn weld_prefers_latest_matching_hash_entry() {
    let mut b = budget();
    let mut vertices = Vec::new();
    let mut first = b.filled(4096, NULL).unwrap();
    let mut next = Vec::new();
    assert_eq!(
        b.add_vertex(&mut vertices, &mut first, &mut next, [1, 1, 2])
            .unwrap(),
        0
    );
    assert_eq!(
        b.add_vertex(&mut vertices, &mut first, &mut next, [1, 4, 2])
            .unwrap(),
        1
    );
    assert_eq!(
        b.add_vertex(&mut vertices, &mut first, &mut next, [1, 2, 2])
            .unwrap(),
        1
    );
}
#[test]
fn unsupported_polygon_widths_reject() {
    for n in [0, 1, 2, 7, 255] {
        assert!(matches!(empty().build_mesh(n), Err(Error::Mesh)));
    }
}
#[test]
fn coordinate_wrap_rejects() {
    for (axis, value) in [(0, -1), (2, 1025), (1, 65536)] {
        let mut c = square();
        c.vertices[0][axis] = value;
        assert!(matches!(
            supplied(vec![c]).build_mesh(6),
            Err(Error::Numeric)
        ));
    }
}
#[test]
fn native_total_vertex_ceiling_rejects_before_triangulation() {
    let mut c = square();
    c.vertices.resize(65534, [0, 1, 0, 0]);
    assert!(matches!(supplied(vec![c]).build_mesh(6), Err(Error::Mesh)));
}
#[test]
fn mesh_work_and_heap_budgets_carry_forward() {
    let m = supplied(vec![square()]).build_mesh(6).unwrap();
    let total = m
        .contours()
        .regions()
        .distance_field()
        .compact()
        .work_used();
    let mut c = supplied(vec![square()]);
    c.regions.field.compact.limits.work = total;
    assert!(c.build_mesh(6).is_ok());
    let mut c = supplied(vec![square()]);
    c.regions.field.compact.limits.work = total - 1;
    assert!(matches!(c.build_mesh(6), Err(Error::Limit)));
    let mut c = supplied(vec![square()]);
    c.regions.field.compact.limits.heap_bytes = c.capacity_bytes();
    assert!(matches!(c.build_mesh(6), Err(Error::Limit)));
}
#[test]
fn marked_straight_border_vertex_is_removed() {
    let mut c = contour(&[[0, 0], [0, 4], [0, 8], [8, 8], [8, 0]], 1, 11);
    c.vertices[1][3] = BORDER_VERTEX;
    let m = supplied(vec![c]).build_mesh(6).unwrap();
    assert_eq!(m.vertices().len(), 4);
    assert_eq!(m.polygons().len(), 1);
    assert!(!m.vertices().contains(&[0, 1, 4]));
}
#[test]
fn isolated_triangle_tip_is_not_removed() {
    let mut c = contour(&[[0, 0], [0, 8], [8, 0]], 1, 11);
    c.vertices[0][3] = BORDER_VERTEX;
    let m = supplied(vec![c]).build_mesh(6).unwrap();
    assert_eq!(m.vertices().len(), 3);
}
#[test]
fn polygon_ids_cannot_collide_with_portal_bit() {
    let mut data = Data {
        vertices: Vec::new(),
        polygons: vec![Polygon::new(1, 11); 32768],
        nvp: 6,
        maximum_polygons: 65533,
    };
    assert!(matches!(
        budget().store(&mut data, Polygon::new(1, 11)),
        Err(Error::Mesh)
    ));
}
#[test]
fn adjacency_edge_list_cannot_collide_with_null_index() {
    let mut p = Polygon::new(1, 11);
    p.vertices = [0, 1, 2, 3, 4, 5];
    let mut data = Data {
        vertices: vec![[0, 0, 0]; 6],
        polygons: vec![p; 14000],
        nvp: 6,
        maximum_polygons: 14000,
    };
    assert!(matches!(budget().adjacency(&mut data), Err(Error::Mesh)));
}

#[test]
fn reversed_contour_rejects_native_partial_triangulation() {
    let mut c = square();
    c.vertices.reverse();
    assert!(matches!(
        supplied(vec![c]).build_mesh(6),
        Err(Error::Triangulation)
    ));
}

#[test]
fn public_field_pipeline_builds_polygon_mesh() {
    let grid = Grid {
        width: 9,
        height: 9,
        min: [0.0; 3],
        max: [9.0, 100.0, 9.0],
        cell_size: 1.0,
        cell_height: 1.0,
    };
    let spans: Vec<_> = (0..9)
        .flat_map(|z| {
            (0..9).map(move |x| SpanInput {
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
    let mesh = Heightfield::new(grid, Limits::default())
        .unwrap()
        .add_spans(&spans, 0)
        .unwrap()
        .compact(Agent {
            height: 1,
            climb: 0,
        })
        .unwrap()
        .prepare(1)
        .unwrap()
        .build_regions(RegionOptions {
            border_size: 1,
            minimum_area: 0,
            merge_area: 2500,
        })
        .unwrap()
        .build_contours(ContourOptions::default())
        .unwrap()
        .build_mesh(6)
        .unwrap();
    assert_eq!(mesh.vertices().len(), 4);
    assert_eq!(mesh.polygons().len(), 1);
    assert_eq!(
        mesh.contours()
            .regions()
            .distance_field()
            .compact()
            .spans()
            .len(),
        81
    );
}

#[test]
fn disconnected_removal_boundaries_reject_incomplete_repair() {
    let vertices = vec![
        [16, 1, 16],
        [12, 1, 12],
        [12, 1, 20],
        [20, 1, 20],
        [20, 1, 12],
        [8, 1, 8],
        [8, 1, 24],
        [24, 1, 24],
        [24, 1, 8],
    ];
    let mut polygons = Vec::new();
    for start in [1, 5] {
        for i in 0..4 {
            let mut p = Polygon::new(1, 11);
            p.vertices[..3].copy_from_slice(&[0, start + i, start + (i + 1) % 4]);
            polygons.push(p);
        }
    }
    let mut data = Data {
        vertices,
        polygons,
        nvp: 6,
        maximum_polygons: 8,
    };
    let mut b = budget();
    assert!(b.can_remove(&data, 0).unwrap());
    assert!(matches!(
        b.remove_vertex(&mut data, 0),
        Err(Error::MeshVertexRemoval)
    ));
}
