// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: wholly authored complete VMO and placement fixtures.
use super::*;
use forever_vmap::{bih, placement};

fn words(out: &mut Vec<u8>, values: impl IntoIterator<Item = u32>) {
    for v in values {
        out.extend_from_slice(&v.to_le_bytes());
    }
}
fn array(out: &mut Vec<u8>, tag: &[u8; 4], values: &[[u32; 3]]) {
    out.extend(tag);
    words(out, [(values.len() * 12 + 4) as u32, values.len() as u32]);
    words(out, values.iter().flatten().copied());
}
fn fixture(m2: bool, groups: usize, liquid: Option<(u32, u32, u32)>, empty: bool) -> Vec<u8> {
    let mut out = vmo::MAGIC.to_vec();
    out.extend(b"WMOD");
    words(&mut out, [8, u32::from(m2), 19]);
    if groups == 0 {
        return out;
    }
    out.extend(b"GMOD");
    words(&mut out, [groups as u32]);
    let bounds = [-20.0f32, -20.0, -20.0, 20.0, 20.0, 20.0].map(f32::to_bits);
    for group in 0..groups {
        words(&mut out, bounds);
        words(
            &mut out,
            [if group % 2 == 0 { 0x2000 } else { 0 }, group as u32],
        );
        let vertices: Vec<_> = if empty {
            vec![]
        } else {
            vec![
                [-0.0f32, 1.0, 2.0],
                [3.0, 1.0, 2.0],
                [0.0, 4.0, 2.0],
                [1.0, 1.0, 6.0],
                [9.0, 8.0, 7.0],
            ]
        }
        .into_iter()
        .map(|v| v.map(f32::to_bits))
        .collect();
        array(&mut out, b"VERT", &vertices);
        if empty {
            continue;
        }
        let tris = [[0u32, 1, 2], [2, 1, 3]];
        array(&mut out, b"TRIM", &tris);
        let primitives: Vec<_> = tris
            .iter()
            .map(|tri| {
                let mut b = [0f32; 6];
                for axis in 0..3 {
                    let vals = tri.map(|i| f32::from_bits(vertices[i as usize][axis]));
                    b[axis] = vals.into_iter().reduce(f32::min).unwrap();
                    b[axis + 3] = vals.into_iter().reduce(f32::max).unwrap();
                }
                bih::Bounds::from_bits(b.map(f32::to_bits)).unwrap()
            })
            .collect();
        out.extend(b"MBIH");
        out.extend(
            bih::build(&primitives, 3, bih::Limits::default())
                .unwrap()
                .bytes(),
        );
        out.extend(b"LIQU");
        if let Some((x, y, kind)) = liquid {
            let mut data = Vec::new();
            words(
                &mut data,
                [
                    x,
                    y,
                    (-2.0f32).to_bits(),
                    3.0f32.to_bits(),
                    999.0f32.to_bits(),
                    kind,
                ],
            );
            words(
                &mut data,
                (0..(x + 1) * (y + 1)).map(|i| (-2.0 + i as f32 * 0.25).to_bits()),
            );
            data.extend((0..x * y).map(|i| [0, 15, 16, 31, 2, 255][i as usize % 6]));
            words(&mut out, [data.len() as u32]);
            out.extend(data);
        } else {
            words(&mut out, [0]);
        }
    }
    out.extend(b"GBIH");
    out.extend(
        bih::build(
            &vec![bih::Bounds::from_bits(bounds).unwrap(); groups],
            1,
            bih::Limits::default(),
        )
        .unwrap()
        .bytes(),
    );
    out
}
fn spawn(position: [f32; 3], angles: [f32; 3], scale: f32, bounds: bool) -> Vec<u8> {
    let mut out = vec![u8::from(bounds), 0];
    words(&mut out, [71]);
    words(&mut out, position.map(f32::to_bits));
    words(&mut out, angles.map(f32::to_bits));
    words(&mut out, [scale.to_bits()]);
    if bounds {
        words(
            &mut out,
            [-1000.0f32, -1000.0, -1000.0, 1000.0, 1000.0, 1000.0].map(f32::to_bits),
        );
    }
    words(&mut out, [5]);
    out.extend(b"toy01");
    out
}
fn placement(raw: &[u8]) -> Placement<'_> {
    placement::DirBin::parse(raw, placement::Limits::default())
        .unwrap()
        .records()
        .next()
        .unwrap()
}
fn view(raw: &[u8]) -> vmo::View<'_> {
    vmo::View::parse(raw, vmo::Limits::default()).unwrap()
}
fn origin() -> [f32; 3] {
    [32.0 * GRID_SIZE, 32.0 * GRID_SIZE, 0.0]
}
fn run(raw: &[u8], pos: &[u8], types: &[(u32, u8)]) -> Mesh {
    let v = view(raw);
    append(
        Mesh::default(),
        &[Model {
            geometry: &v,
            placement: placement(pos),
        }],
        LiquidTypes::new(types).unwrap(),
        Limits::default(),
    )
    .unwrap()
}
#[test]
fn identity_transform_reorders_and_mirrors_horizontal_axes() {
    let m = run(
        &fixture(false, 1, None, false),
        &spawn(origin(), [0.0; 3], 1.0, true),
        &[],
    );
    assert_eq!(m.solid_vertices()[1], [-1.0, 2.0, -3.0]);
    assert_eq!(m.solid_triangles(), &[[0, 1, 2], [2, 1, 3]]);
}
#[test]
fn m2_winding_reverses_and_wmo_does_not() {
    let m = run(
        &fixture(true, 1, None, false),
        &spawn(origin(), [0.0; 3], 1.0, true),
        &[],
    );
    assert_eq!(m.solid_triangles(), &[[2, 1, 0], [3, 1, 2]]);
}
#[test]
fn all_groups_and_unreferenced_vertices_survive_until_compaction() {
    let m = run(
        &fixture(false, 3, None, false),
        &spawn(origin(), [0.0; 3], 1.0, true),
        &[],
    );
    assert_eq!(m.solid_vertices().len(), 15);
    assert_eq!(m.solid_triangles()[2], [5, 6, 7]);
    let m = m.compact(crate::Limits::default()).unwrap();
    assert_eq!(m.solid_vertices().len(), 12);
}
#[test]
fn empty_models_groups_and_flat_liquid_add_no_surface() {
    let pos = spawn(origin(), [0.0; 3], 1.0, true);
    for raw in [
        fixture(false, 0, None, false),
        fixture(false, 3, None, true),
    ] {
        let m = run(&raw, &pos, &[]);
        assert!(!m.has_triangles());
    }
    let m = run(&fixture(false, 1, Some((0, 0, 9)), false), &pos, &[]);
    assert!(m.liquid_vertices().is_empty());
}
#[test]
fn rectangular_liquid_transposes_vertices_flags_and_triangles() {
    let m = run(
        &fixture(false, 1, Some((2, 3, 7)), false),
        &spawn(origin(), [0.0; 3], 1.0, true),
        &[(7, 0)],
    );
    assert_eq!(m.liquid_vertices().len(), 12);
    assert_eq!(m.liquid_vertices()[0], [-3.0, -2.0, 2.0]);
    assert_eq!(m.liquid_vertices()[1][1], -1.25);
    assert_eq!(m.liquid_vertices()[4][1], -1.75);
    assert_eq!(
        m.liquid_triangles(),
        &[
            [1, 0, 5],
            [5, 0, 4],
            [2, 1, 6],
            [6, 1, 5],
            [3, 2, 7],
            [7, 2, 6]
        ]
    );
    assert_eq!(m.liquid_areas(), &[9; 6]);
}
#[test]
fn vmap_liquid_missing_and_dark_types_keep_area_zero() {
    let raw = fixture(false, 1, Some((1, 1, 19)), false);
    let pos = spawn(origin(), [0.0; 3], 1.0, true);
    for (bank, area) in [
        (0, 9),
        (1, 9),
        (2, 8),
        (3, 8),
        (4, 0),
        (5, 0),
        (7, 0),
        (8, 0),
        (30, 0),
    ] {
        let m = run(&raw, &pos, &[(19, bank)]);
        assert_eq!(m.liquid_areas(), &[area; 2]);
    }
    assert_eq!(run(&raw, &pos, &[]).liquid_areas(), &[0; 2]);
}
#[test]
fn liquid_type_lookup_requires_sorted_unique_safe_shift_domain() {
    for types in [
        &[(7, 0), (7, 1)][..],
        &[(7, 0), (6, 1)][..],
        &[(7, 31)][..],
        &[(7, 255)][..],
    ] {
        assert!(matches!(LiquidTypes::new(types), Err(Error::LiquidTypes)));
    }
    assert!(matches!(
        LiquidTypes::new(&vec![(0, 0); 65537]),
        Err(Error::Limit)
    ));
    let types = LiquidTypes::new(&[(0, 0), (u32::MAX, 3)]).unwrap();
    assert_eq!(types.flags(0), 1);
    assert_eq!(types.flags(u32::MAX), 8);
    assert_eq!(types.flags(1), 0);
}
#[test]
fn transform_rotations_scales_and_negative_scales_remain_finite() {
    let raw = fixture(false, 1, Some((3, 2, 7)), false);
    for scale in [-2.0, 0.5, 1.0, 4.0] {
        for angles in [[0.0; 3], [90.0, 0.0, 0.0], [13.0, -29.0, 71.0]] {
            let m = run(
                &raw,
                &spawn([17.0, 19.0, 23.0], angles, scale, true),
                &[(7, 1)],
            );
            assert!(
                m.solid_vertices()
                    .iter()
                    .flatten()
                    .chain(m.liquid_vertices().iter().flatten())
                    .all(|v| v.is_finite())
            );
        }
    }
}
#[test]
fn missing_bounds_zero_tiny_scale_and_output_overflow_are_errors() {
    let raw = fixture(false, 1, None, false);
    let v = view(&raw);
    let types = LiquidTypes::new(&[]).unwrap();
    for p in [
        spawn(origin(), [0.0; 3], 1.0, false),
        spawn(origin(), [0.0; 3], 0.0, true),
        spawn(origin(), [0.0; 3], f32::from_bits(1), true),
    ] {
        assert!(matches!(
            append(
                Mesh::default(),
                &[Model {
                    geometry: &v,
                    placement: placement(&p)
                }],
                types,
                Limits::default()
            ),
            Err(Error::Transform(_))
        ));
    }
    let p = spawn(origin(), [0.0; 3], f32::MAX, true);
    assert!(matches!(
        append(
            Mesh::default(),
            &[Model {
                geometry: &v,
                placement: placement(&p)
            }],
            types,
            Limits::default()
        ),
        Err(Error::NonFinite)
    ));
}
#[test]
fn prefixes_are_preserved_with_correct_both_mesh_offsets() {
    let raw = fixture(false, 1, Some((1, 1, 7)), false);
    let pos = spawn(origin(), [0.0; 3], 1.0, true);
    let v = view(&raw);
    let models = [Model {
        geometry: &v,
        placement: placement(&pos),
    }];
    let types = LiquidTypes::new(&[(7, 0)]).unwrap();
    let first = append(Mesh::default(), &models, types, Limits::default()).unwrap();
    let second = append(first, &models, types, Limits::default()).unwrap();
    assert_eq!(second.solid_triangles()[2], [5, 6, 7]);
    assert_eq!(second.liquid_triangles()[2], [5, 4, 7]);
    assert_eq!(second.liquid_areas(), &[9; 4]);
}
#[test]
fn model_group_geometry_and_memory_limits_only_reduce() {
    let raw = fixture(false, 1, Some((1, 1, 7)), false);
    let pos = spawn(origin(), [0.0; 3], 1.0, true);
    let v = view(&raw);
    let models = [Model {
        geometry: &v,
        placement: placement(&pos),
    }];
    let types = LiquidTypes::new(&[]).unwrap();
    for l in [
        Limits {
            models: 0,
            ..Default::default()
        },
        Limits {
            groups: 0,
            ..Default::default()
        },
        Limits {
            models: 4097,
            ..Default::default()
        },
        Limits {
            groups: 65537,
            ..Default::default()
        },
        Limits {
            mesh: crate::Limits {
                vertices: 1,
                ..Default::default()
            },
            ..Default::default()
        },
        Limits {
            mesh: crate::Limits {
                triangles: 1,
                ..Default::default()
            },
            ..Default::default()
        },
        Limits {
            mesh: crate::Limits {
                heap_bytes: 1,
                ..Default::default()
            },
            ..Default::default()
        },
    ] {
        assert!(matches!(
            append(Mesh::default(), &models, types, l),
            Err(Error::Limit)
        ));
    }
    let m = append(Mesh::default(), &models, types, Limits::default()).unwrap();
    let cap = m.capacity_bytes();
    assert!(matches!(
        append(
            m,
            &models,
            types,
            Limits {
                mesh: crate::Limits {
                    heap_bytes: cap * 3 - 1,
                    ..Default::default()
                },
                ..Default::default()
            }
        ),
        Err(Error::Limit)
    ));
}
#[test]
fn exact_append_counts_and_peak_memory_are_admitted() {
    let raw = fixture(false, 1, Some((1, 1, 7)), false);
    let pos = spawn(origin(), [0.0; 3], 1.0, true);
    let v = view(&raw);
    let models = [Model {
        geometry: &v,
        placement: placement(&pos),
    }];
    let types = LiquidTypes::new(&[]).unwrap();
    let cap = (5 + 2 + 4 + 2) * 12 + 2;
    let l = Limits {
        mesh: crate::Limits {
            vertices: 9,
            triangles: 4,
            heap_bytes: cap,
        },
        models: 1,
        groups: 1,
    };
    let first = append(Mesh::default(), &models, types, l).unwrap();
    assert_eq!(first.capacity_bytes(), cap);
    let l = Limits {
        mesh: crate::Limits {
            vertices: 18,
            triangles: 8,
            heap_bytes: cap * 3,
        },
        models: 1,
        groups: 1,
    };
    let second = append(first, &models, types, l).unwrap();
    assert_eq!(second.capacity_bytes(), cap * 2);
}

mod corpus;

#[test]
fn exact_model_and_cumulative_group_ceilings_are_admitted() {
    let p = spawn(origin(), [0.0; 3], 1.0, true);
    let raw = fixture(false, 16, None, true);
    let v = view(&raw);
    let models = vec![
        Model {
            geometry: &v,
            placement: placement(&p)
        };
        4096
    ];
    let m = append(
        Mesh::default(),
        &models,
        LiquidTypes::new(&[]).unwrap(),
        Limits::default(),
    )
    .unwrap();
    assert!(!m.has_triangles());
    assert!(matches!(
        append(
            Mesh::default(),
            &models,
            LiquidTypes::new(&[]).unwrap(),
            Limits {
                groups: 65535,
                ..Default::default()
            }
        ),
        Err(Error::Limit)
    ));
    let too_many = vec![models[0]; 4097];
    assert!(matches!(
        append(
            Mesh::default(),
            &too_many,
            LiquidTypes::new(&[]).unwrap(),
            Limits::default()
        ),
        Err(Error::Limit)
    ));
}

mod loaded;
