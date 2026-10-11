// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: synthetic Detour payload and admission tests.
use super::*;
use crate::recast::{DetailSubmesh, MeshHeader, Polygon};
mod corpus;
fn mesh() -> TileMesh {
    TileMesh {
        header: MeshHeader {
            min: [0.0; 3],
            max: [4.0, 10.0, 4.0],
            cell_size: 1.0,
            cell_height: 0.5,
            vertices_per_polygon: 6,
            maximum_polygons: 2,
            border_size: 1,
            maximum_edge_error: 1.8,
        },
        vertices: vec![[0, 1, 0], [0, 1, 4], [4, 1, 4], [4, 1, 0]],
        polygons: vec![Polygon {
            vertices: [0, 1, 2, 3, 65535, 65535],
            neighbors: [0x8000, 0x8001, 0x8002, 0x8003, 65535, 65535],
            region: 7,
            area: 11,
            flags: 0,
        }],
        submeshes: vec![DetailSubmesh {
            vertex_base: 0,
            vertex_count: 4,
            triangle_base: 0,
            triangle_count: 2,
        }],
        detail_vertices: vec![
            [0.0, 1.0, 0.0],
            [0.0, 1.0, 4.0],
            [4.0, 1.0, 4.0],
            [4.0, 1.0, 0.0],
        ],
        detail_triangles: vec![[0, 1, 2, 5], [0, 2, 3, 20]],
        discarded_faces: 0,
        processing_work: 0,
        merge_work: 0,
    }
}
fn options(m: &TileMesh) -> Options {
    Options {
        tile: [0; 3],
        user_id: 0,
        min: m.header().min,
        max: m.header().max,
        walkable_height: 2.0,
        walkable_radius: 0.5,
        walkable_climb: 0.5,
        build_bv_tree: true,
        flags: FlagPolicy::Forever,
    }
}
fn generate(m: &TileMesh) -> Data {
    build(m, &[], options(m), Limits::default()).unwrap()
}
fn word(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap())
}
fn half(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes(bytes[at..at + 2].try_into().unwrap())
}
fn connections(
    ends: &[[Point; 2]],
    radius: f32,
    area: u8,
    flags: u16,
    direction: u8,
) -> crate::offmesh::Connections {
    let sources = ends
        .iter()
        .map(|p| crate::offmesh::Source {
            map_id: 1,
            row: 0,
            column: 0,
            from: [p[0][2], p[0][0], p[0][1]],
            to: [p[1][2], p[1][0], p[1][1]],
            radius,
            connection_flags: direction,
            area,
            flags,
        })
        .collect::<Vec<_>>();
    crate::offmesh::select(
        crate::offmesh::Target {
            map_id: 1,
            row: 0,
            column: 0,
        },
        &sources,
        crate::offmesh::Limits::default(),
    )
    .unwrap()
}
#[test]
fn flat_payload_uses_expected_64_bit_reference_layout() {
    let d = generate(&mesh());
    assert_eq!(d.bytes.len(), 424);
    assert_eq!(word(&d.bytes, 0), MAGIC);
    assert_eq!(word(&d.bytes, 4), 7);
    assert_eq!(word(&d.bytes, 32), 12);
    assert_eq!(word(&d.bytes, 48), 2);
    assert_eq!(word(&d.bytes, 96), 1.0f32.to_bits());
    assert_eq!(d.statistics.maximum_links, 12);
}
#[test]
fn portals_map_to_detour_cardinal_sides() {
    let d = generate(&mesh());
    assert_eq!(
        (0..4)
            .map(|i| half(&d.bytes, 148 + 16 + i * 2))
            .collect::<Vec<_>>(),
        [0x8004, 0x8002, 0x8000, 0x8006]
    );
    assert_eq!(half(&d.bytes, 148 + 28), 1);
}
#[test]
fn allocation_links_padding_and_spare_tree_node_stay_zero() {
    let d = generate(&mesh());
    assert!(d.bytes[180..372].iter().all(|&v| v == 0));
    assert_eq!(&d.bytes[382..384], &[0, 0]);
    assert!(d.bytes[408..424].iter().all(|&v| v == 0));
}
#[test]
fn bv_tree_can_be_omitted_explicitly() {
    let m = mesh();
    let mut o = options(&m);
    o.build_bv_tree = false;
    let d = build(&m, &[], o, Limits::default()).unwrap();
    assert_eq!(d.bytes.len(), 392);
    assert_eq!(word(&d.bytes, 48), 0);
}
#[test]
fn detail_corners_are_replaced_by_polygon_heights() {
    let d = generate(&mesh());
    assert_eq!(word(&d.bytes, 104), 0.5f32.to_bits());
    assert_eq!(word(&d.bytes, 40), 0);
    assert_eq!(half(&d.bytes, 392 + 2), 1); // BV bounds still use source detail height.
}
#[test]
fn explicit_bounds_tile_metadata_and_world_conversion() {
    let m = mesh();
    let mut o = options(&m);
    o.tile = [-7, 19, 2];
    o.user_id = 0xff00abcd;
    o.min = [-10.0, -20.0, -30.0];
    o.max = [10.0, 20.0, 30.0];
    let d = build(&m, &[], o, Limits::default()).unwrap();
    assert_eq!(word(&d.bytes, 8), (-7i32) as u32);
    assert_eq!(word(&d.bytes, 12), 19);
    assert_eq!(word(&d.bytes, 16), 2);
    assert_eq!(word(&d.bytes, 20), o.user_id);
    assert_eq!(word(&d.bytes, 100), (-10.0f32).to_bits());
    assert_eq!(word(&d.bytes, 104), (-19.5f32).to_bits());
}
#[test]
fn forever_flag_projection_covers_all_defined_areas() {
    for (a, f) in [(0, 99), (1, 1), (7, 1), (8, 8), (9, 4), (10, 2), (11, 1)] {
        assert_eq!(forever_flags(a, 99).unwrap(), f);
    }
    assert!(matches!(forever_flags(12, 0), Err(Error::Area)));
    assert_eq!(forever_flags(64, 99).unwrap(), 99);
}
#[test]
fn preserve_policy_keeps_flags_and_all_detour_area_values() {
    let mut m = mesh();
    m.polygons[0].area = 63;
    m.polygons[0].flags = 0xf135;
    assert!(matches!(
        build(&m, &[], options(&m), Limits::default()),
        Err(Error::Area)
    ));
    let mut o = options(&m);
    o.flags = FlagPolicy::Preserve;
    let d = build(&m, &[], o, Limits::default()).unwrap();
    assert_eq!(half(&d.bytes, 176), 0xf135);
    assert_eq!(d.bytes[179], 63);
}
#[test]
fn all_offmesh_sides_and_exact_edges_are_classified() {
    for (p, side) in [
        ([4.0, 0.0, 2.0], 0),
        ([4.0, 0.0, 4.0], 1),
        ([2.0, 0.0, 4.0], 2),
        ([-1.0, 0.0, 4.0], 3),
        ([-1.0, 0.0, 2.0], 4),
        ([-1.0, 0.0, -1.0], 5),
        ([2.0, 0.0, -1.0], 6),
        ([4.0, 0.0, -1.0], 7),
        ([0.0, 0.0, 0.0], 255),
    ] {
        assert_eq!(classify(p, [0.0; 3], [4.0; 3]), side);
    }
}
#[test]
fn culling_start_preserves_end_link_capacity_contribution() {
    let m = mesh();
    let c = connections(&[[[5.0, 1.0, 2.0], [2.0, 1.0, 2.0]]], 0.5, 11, 1, 1);
    let d = build(&m, c.values(), options(&m), Limits::default()).unwrap();
    assert_eq!(d.statistics.stored_connections, 0);
    assert_eq!(d.statistics.maximum_links, 14);
    assert_eq!(d.statistics.culled_connections, 1);
}
#[test]
fn start_height_limits_are_inclusive_and_end_height_is_not_culled() {
    let m = mesh();
    let c = connections(
        &[
            [[1.0, 0.5, 1.0], [2.0, 900.0, 2.0]],
            [[1.0, 1.5, 1.0], [2.0, 900.0, 2.0]],
            [[1.0, 1.501, 1.0], [2.0, 1.0, 2.0]],
        ],
        0.5,
        11,
        1,
        0,
    );
    let d = build(&m, c.values(), options(&m), Limits::default()).unwrap();
    assert_eq!(d.statistics.stored_connections, 2);
    assert_eq!(d.statistics.culled_connections, 1);
    assert_eq!(d.statistics.maximum_links, 22);
}
#[test]
fn offmesh_records_keep_order_flags_and_masked_area() {
    let m = mesh();
    let c = connections(
        &[
            [[1.0, 1.0, 1.0], [5.0, 1.0, 2.0]],
            [[2.0, 1.0, 1.0], [2.0, 1.0, 2.0]],
        ],
        0.5,
        255,
        0xabcd,
        1,
    );
    let d = build(&m, c.values(), options(&m), Limits::default()).unwrap();
    let at = d.bytes.len() - 72;
    assert_eq!(d.statistics.stored_connections, 2);
    assert_eq!(half(&d.bytes, at + 28), 1);
    assert_eq!(d.bytes[at + 30], 1);
    assert_eq!(d.bytes[at + 31], 0);
    assert_eq!(d.bytes[at + 36 + 31], 255);
    assert_eq!(word(&d.bytes, at + 32), 0);
    let poly = 100 + 8 * 12 + 32;
    assert_eq!(half(&d.bytes, poly + 28), 0xabcd);
    assert_eq!(d.bytes[poly + 31], 127);
}
#[test]
fn zero_radius_is_preserved_and_negative_radius_rejects() {
    let m = mesh();
    let c = connections(&[[[1.0, 1.0, 1.0], [2.0, 1.0, 2.0]]], -0.0, 11, 1, 0);
    let d = build(&m, c.values(), options(&m), Limits::default()).unwrap();
    assert_eq!(word(&d.bytes, d.bytes.len() - 12), (-0.0f32).to_bits());
    let c = connections(&[[[1.0, 1.0, 1.0], [2.0, 1.0, 2.0]]], -1.0, 11, 1, 0);
    assert!(matches!(
        build(&m, c.values(), options(&m), Limits::default()),
        Err(Error::Options)
    ));
}
#[test]
fn invalid_bounds_agent_and_layer_reject() {
    for mode in 0..5 {
        let m = mesh();
        let mut o = options(&m);
        match mode {
            0 => o.min[0] = f32::NAN,
            1 => o.max[0] = -1.0,
            2 => o.walkable_radius = -1.0,
            3 => o.walkable_climb = f32::INFINITY,
            _ => o.tile[2] = -1,
        };
        assert!(matches!(
            build(&m, &[], o, Limits::default()),
            Err(Error::Options)
        ));
    }
}
#[test]
fn empty_mesh_rejects_before_emitting_header() {
    let m = TileMesh::merge(vec![], crate::recast::MergeLimits::default()).unwrap();
    assert!(matches!(
        build(&m, &[], options(&m), Limits::default()),
        Err(Error::EmptyMesh)
    ));
}
#[test]
fn output_heap_and_work_limits_are_checked() {
    let m = mesh();
    let d = generate(&m);
    let limits = Limits {
        output_bytes: d.bytes.len(),
        work: d.statistics.work,
        ..Limits::default()
    };
    assert!(build(&m, &[], options(&m), limits).is_ok());
    assert!(matches!(
        build(
            &m,
            &[],
            options(&m),
            Limits {
                output_bytes: d.bytes.len() - 1,
                ..limits
            }
        ),
        Err(Error::Limit)
    ));
    assert!(matches!(
        build(
            &m,
            &[],
            options(&m),
            Limits {
                work: d.statistics.work - 1,
                ..limits
            }
        ),
        Err(Error::Limit)
    ));
    assert!(matches!(
        build(
            &m,
            &[],
            options(&m),
            Limits {
                heap_bytes: 0,
                ..limits
            }
        ),
        Err(Error::Limit)
    ));
    assert!(matches!(
        build(
            &m,
            &[],
            options(&m),
            Limits {
                connections: 65537,
                ..limits
            }
        ),
        Err(Error::Limit)
    ));
}
#[test]
fn quantization_rejects_native_float_to_int_undefined_domain() {
    let m = mesh();
    let mut o = options(&m);
    o.min[0] = -1e30;
    assert!(matches!(
        build(&m, &[], o, Limits::default()),
        Err(Error::Numeric)
    ));
    o.build_bv_tree = false;
    assert!(build(&m, &[], o, Limits::default()).is_ok());
}
#[test]
fn connections_cannot_wrap_stored_u16_vertex_indices() {
    let m = mesh();
    let ends = vec![[[1.0, 1.0, 1.0], [2.0, 1.0, 2.0]]; 32767];
    let c = connections(&ends, 0.5, 11, 1, 0);
    assert!(matches!(
        build(&m, c.values(), options(&m), Limits::default()),
        Err(Error::Limit)
    ));
}
#[test]
fn complete_public_recast_composition_reaches_detour_payload() {
    use crate::recast::{
        Agent, ContourOptions, DetailOptions, Grid, Heightfield, RegionOptions, Span, SpanInput,
    };
    let spans = (0..8)
        .flat_map(|z| {
            (0..8).map(move |x| SpanInput {
                x,
                z,
                span: Span {
                    min: 0,
                    max: 1,
                    area: 11,
                },
            })
        })
        .collect::<Vec<_>>();
    let m = Heightfield::new(
        Grid {
            width: 8,
            height: 8,
            min: [0.0; 3],
            max: [8.0, 20.0, 8.0],
            cell_size: 1.0,
            cell_height: 0.5,
        },
        crate::recast::Limits::default(),
    )
    .unwrap()
    .add_spans(&spans, 0)
    .unwrap()
    .compact(Agent {
        height: 1,
        climb: 1,
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
    .build_mesh(6)
    .unwrap()
    .build_detail(DetailOptions {
        sample_distance: 16.0,
        maximum_error: 0.5,
    })
    .unwrap()
    .into_tile_mesh();
    let m = TileMesh::merge(vec![m], crate::recast::MergeLimits::default()).unwrap();
    assert_eq!(word(generate(&m).bytes(), 0), MAGIC);
}

mod save;
