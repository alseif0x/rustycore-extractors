// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: synthetic paired subtile merging and admission.
use super::*;
mod corpus;
fn detail(x: f32, z: f32, height: u16, nvp: u8) -> DetailMesh {
    let g = Grid {
        width: 6,
        height: 6,
        min: [x - 1.0, 0.0, z - 1.0],
        max: [x + 5.0, 100.0, z + 5.0],
        cell_size: 1.0,
        cell_height: 0.5,
    };
    let spans: Vec<_> = (0..6)
        .flat_map(|z| {
            (0..6).map(move |x| SpanInput {
                x,
                z,
                span: Span {
                    min: height,
                    max: height + 1,
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
            climb: 4,
        })
        .unwrap()
        .build_distance_field()
        .unwrap()
        .build_regions(RegionOptions {
            border_size: 1,
            minimum_area: 0,
            merge_area: 0,
        })
        .unwrap()
        .build_contours(ContourOptions::default())
        .unwrap()
        .build_mesh(nvp)
        .unwrap()
        .build_detail(DetailOptions {
            sample_distance: 16.0,
            maximum_error: 0.5,
        })
        .unwrap()
}
fn tile(x: f32, z: f32, height: u16) -> TileMesh {
    detail(x, z, height, 6).into_tile_mesh()
}
fn merge(input: Vec<TileMesh>) -> TileMesh {
    TileMesh::merge(input, MergeLimits::default()).unwrap()
}
fn portals(m: &TileMesh) -> usize {
    m.polygons
        .iter()
        .flat_map(Polygon::neighbors)
        .filter(|&&n| (0x8000..=0x8003).contains(&n))
        .count()
}
fn links(m: &TileMesh) -> usize {
    m.polygons
        .iter()
        .flat_map(Polygon::neighbors)
        .filter(|&&n| n < 0x8000)
        .count()
}
#[test]
fn detach_moves_arrays_and_releases_compact_storage() {
    let d = detail(0.0, 0.0, 0, 6);
    let old = d.capacity_bytes();
    let v = d.mesh.vertices.as_ptr();
    let dv = d.vertices.as_ptr();
    let t = d.triangles.as_ptr();
    let work = d.mesh.contours.regions.field.compact.work;
    let m = d.into_tile_mesh();
    assert_eq!(m.vertices.as_ptr(), v);
    assert_eq!(m.detail_vertices.as_ptr(), dv);
    assert_eq!(m.detail_triangles.as_ptr(), t);
    assert!(m.capacity_bytes() < old);
    assert_eq!(m.processing_work(), work);
    assert_eq!(m.merge_work(), 0);
}
#[test]
fn empty_merge_keeps_native_zero_header() {
    let m = merge(vec![]);
    assert_eq!(m.header, MeshHeader::default());
    assert!(
        m.vertices.is_empty()
            && m.polygons.is_empty()
            && m.submeshes.is_empty()
            && m.detail_vertices.is_empty()
            && m.detail_triangles.is_empty()
    );
    assert_eq!(m.merge_work(), 0);
}
#[test]
fn single_merge_clears_native_unassigned_metadata() {
    let a = tile(0.0, 0.0, 0);
    let verts = a.vertices.clone();
    let detail = a.detail_vertices.clone();
    let polys = a.polygons.clone();
    assert!(
        a.header.maximum_polygons > 0
            && a.header.border_size > 0
            && a.header.maximum_edge_error > 0.0
    );
    let m = merge(vec![a]);
    assert_eq!(m.vertices, verts);
    assert_eq!(m.polygons, polys);
    assert_eq!(m.detail_vertices, detail);
    assert_eq!(m.header.maximum_polygons, 0);
    assert_eq!(m.header.border_size, 0);
    assert_eq!(m.header.maximum_edge_error.to_bits(), 0);
    assert_eq!(portals(&m), 4);
}
#[test]
fn adjacent_subtiles_weld_and_link_but_keep_detail_vertices() {
    let m = merge(vec![tile(0.0, 0.0, 0), tile(4.0, 0.0, 0)]);
    assert_eq!(m.vertices.len(), 6);
    assert_eq!(m.detail_vertices.len(), 8);
    assert_eq!(m.polygons.len(), 2);
    assert_eq!(links(&m), 2);
    assert_eq!(portals(&m), 6);
}
#[test]
fn four_subtiles_preserve_only_outer_portals() {
    let m = merge(vec![
        tile(0.0, 0.0, 0),
        tile(4.0, 0.0, 0),
        tile(0.0, 4.0, 0),
        tile(4.0, 4.0, 0),
    ]);
    assert_eq!(m.vertices.len(), 9);
    assert_eq!(links(&m), 8);
    assert_eq!(portals(&m), 8);
}
#[test]
fn interior_gap_is_wall_without_synthetic_link() {
    let m = merge(vec![tile(0.0, 0.0, 0), tile(8.0, 0.0, 0)]);
    assert_eq!(links(&m), 0);
    assert_eq!(portals(&m), 6);
    assert_eq!(
        m.polygons
            .iter()
            .flat_map(Polygon::neighbors)
            .filter(|&&n| n == NULL)
            .count(),
        2
    );
}
#[test]
fn welding_height_tolerance_keeps_first_encountered_height() {
    for (height, count) in [(2, 6), (3, 8)] {
        let m = merge(vec![tile(0.0, 0.0, 0), tile(4.0, 0.0, height)]);
        assert_eq!(m.vertices.len(), count);
        if height == 2 {
            assert!(m.vertices.iter().filter(|v| v[0] == 4).all(|v| v[1] == 1));
        }
    }
    let m = merge(vec![tile(4.0, 0.0, 2), tile(0.0, 0.0, 0)]);
    assert!(m.vertices.iter().filter(|v| v[0] == 4).all(|v| v[1] == 3));
}
#[test]
fn source_detail_indices_stay_local_while_bases_shift() {
    let a = tile(0.0, 0.0, 0);
    let b = tile(4.0, 0.0, 0);
    let bv = b.detail_vertices.clone();
    let bt = b.detail_triangles.clone();
    let vb = a.detail_vertices.len();
    let tb = a.detail_triangles.len();
    let m = merge(vec![a, b]);
    assert_eq!(&m.detail_vertices[vb..], bv);
    assert_eq!(&m.detail_triangles[tb..], bt);
    assert_eq!(m.submeshes[1].vertex_base, vb as u32);
    assert_eq!(m.submeshes[1].triangle_base, tb as u32);
}
#[test]
fn source_region_area_and_flags_are_not_renumbered() {
    let mut a = tile(0.0, 0.0, 0);
    let mut b = tile(4.0, 0.0, 0);
    a.polygons[0].region = 301;
    a.polygons[0].flags = 0xffff;
    b.polygons[0].region = 301;
    b.polygons[0].area = 63;
    b.polygons[0].flags = 0x1234;
    let m = merge(vec![a, b]);
    assert_eq!(m.polygons[0].region, 301);
    assert_eq!(m.polygons[1].region, 301);
    assert_eq!(m.polygons[0].flags, 0xffff);
    assert_eq!(m.polygons[1].flags, 0x1234);
    assert_eq!(m.polygons[1].area, 63);
}
#[test]
fn bounds_ties_take_right_signed_zero() {
    let a = tile(0.0, 0.0, 0);
    let mut b = tile(0.0, 0.0, 0);
    b.header.min[0] = -0.0;
    b.header.min[1] = -0.0;
    b.header.min[2] = -0.0;
    let m = merge(vec![a, b]);
    assert!(
        m.header
            .min
            .iter()
            .all(|v| v.to_bits() == (-0.0f32).to_bits())
    );
}
#[test]
fn empty_subtile_still_participates_in_bounds() {
    let mut a = tile(-4.0, 0.0, 0);
    a.vertices.clear();
    a.polygons.clear();
    a.submeshes.clear();
    a.detail_vertices.clear();
    a.detail_triangles.clear();
    let m = merge(vec![a, tile(0.0, 0.0, 0)]);
    assert_eq!(m.header.min[0], -4.0);
    assert_eq!(m.vertices.iter().map(|v| v[0]).min(), Some(4));
    assert_eq!(portals(&m), 3);
}
#[test]
fn differing_cell_units_vertical_origin_or_width_reject() {
    for kind in 0..4 {
        let a = tile(0.0, 0.0, 0);
        let mut b = tile(4.0, 0.0, 0);
        match kind {
            0 => b.header.cell_size = 0.5,
            1 => b.header.cell_height = 0.25,
            2 => b.header.min[1] = 1.0,
            _ => b.header.vertices_per_polygon = 3,
        };
        assert!(matches!(
            TileMesh::merge(vec![a, b], MergeLimits::default()),
            Err(Error::MergeContract)
        ));
    }
}
#[test]
fn offset_rounding_matches_original_half_up_rule() {
    assert_eq!(offset(0.499, 0.0, 1.0).unwrap(), 0);
    assert_eq!(offset(0.5, 0.0, 1.0).unwrap(), 1);
    assert_eq!(offset(1.5, 0.0, 1.0).unwrap(), 2);
    assert_eq!(offset(65535.49, 0.0, 1.0).unwrap(), 65535);
}
#[test]
fn unrepresentable_offsets_and_shifted_coordinates_reject() {
    for v in [65535.5, f32::INFINITY, f32::NAN] {
        assert!(matches!(offset(v, 0.0, 1.0), Err(Error::Numeric)));
    }
    for x in [65534.0, 65536.0] {
        assert!(matches!(
            TileMesh::merge(
                vec![tile(0.0, 0.0, 0), tile(x, 0.0, 0)],
                MergeLimits::default()
            ),
            Err(Error::Numeric)
        ));
    }
}
#[test]
fn aggregate_policy_limits_reject_without_partial_output() {
    assert!(matches!(
        TileMesh::merge(
            vec![tile(0.0, 0.0, 0)],
            MergeLimits {
                subtiles: 0,
                ..MergeLimits::default()
            }
        ),
        Err(Error::Limit)
    ));
    assert!(matches!(
        TileMesh::merge(
            vec![tile(0.0, 0.0, 0)],
            MergeLimits {
                heap_bytes: 0,
                ..MergeLimits::default()
            }
        ),
        Err(Error::Limit)
    ));
    assert!(matches!(
        TileMesh::merge(
            vec![],
            MergeLimits {
                work: 128_000_001,
                ..MergeLimits::default()
            }
        ),
        Err(Error::Limit)
    ));
    let work = merge(vec![tile(0.0, 0.0, 0)]).merge_work();
    assert!(
        TileMesh::merge(
            vec![tile(0.0, 0.0, 0)],
            MergeLimits {
                work,
                ..MergeLimits::default()
            }
        )
        .is_ok()
    );
    assert!(matches!(
        TileMesh::merge(
            vec![tile(0.0, 0.0, 0)],
            MergeLimits {
                work: work - 1,
                ..MergeLimits::default()
            }
        ),
        Err(Error::Limit)
    ));
}
#[test]
fn polygon_marker_collision_rejects_before_merging() {
    let mut a = tile(0.0, 0.0, 0);
    a.polygons.resize(0x8001, a.polygons[0]);
    assert!(matches!(
        TileMesh::merge(vec![a], MergeLimits::default()),
        Err(Error::Mesh)
    ));
}
#[test]
fn cumulative_work_and_recovery_diagnostics_survive_merge() {
    let mut a = tile(0.0, 0.0, 0);
    let mut b = tile(4.0, 0.0, 0);
    let prior = a.processing_work() + b.processing_work();
    a.discarded_faces = 2;
    b.discarded_faces = 4;
    let m = merge(vec![a, b]);
    assert_eq!(m.discarded_faces(), 6);
    assert_eq!(m.processing_work(), prior + m.merge_work());
}
#[test]
fn repeated_merges_preserve_pairing() {
    let left = merge(vec![tile(0.0, 0.0, 0), tile(4.0, 0.0, 0)]);
    let right = merge(vec![tile(8.0, 0.0, 0), tile(12.0, 0.0, 0)]);
    let m = merge(vec![left, right]);
    let direct = merge(vec![
        tile(0.0, 0.0, 0),
        tile(4.0, 0.0, 0),
        tile(8.0, 0.0, 0),
        tile(12.0, 0.0, 0),
    ]);
    assert_eq!(m.header, direct.header);
    assert_eq!(m.vertices, direct.vertices);
    assert_eq!(m.polygons, direct.polygons);
    assert_eq!(m.submeshes, direct.submeshes);
    assert_eq!(m.detail_vertices, direct.detail_vertices);
    assert_eq!(m.detail_triangles, direct.detail_triangles);
}
