// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: synthetic full supplied-span native detail comparison.
use super::*;
use std::{fs, path::Path};
struct Case {
    name: String,
    grid: Grid,
    spans: Vec<SpanInput>,
    agent: Agent,
    radius: Option<u16>,
    median: bool,
    region_options: RegionOptions,
    contour_options: ContourOptions,
    nvp: u8,
    region_mode: u32,
    detail: DetailOptions,
}
impl Case {
    fn plane(name: impl Into<String>, w: u16, h: u16) -> Self {
        let spans = (0..h)
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
        Self {
            name: name.into(),
            grid: Grid {
                width: w,
                height: h,
                min: [0.0; 3],
                max: [f32::from(w), 2000.0, f32::from(h)],
                cell_size: 1.0,
                cell_height: 0.5,
            },
            spans,
            agent: Agent {
                height: 1,
                climb: 8,
            },
            radius: None,
            median: false,
            region_options: RegionOptions {
                border_size: 0,
                minimum_area: 0,
                merge_area: 2500,
            },
            contour_options: ContourOptions::default(),
            nvp: 6,
            region_mode: 0,
            detail: options(),
        }
    }
    fn scaled(&mut self, cs: f32, ch: f32, origin: Point) {
        self.grid.cell_size = cs;
        self.grid.cell_height = ch;
        self.grid.min = origin;
        self.grid.max = [
            origin[0] + f32::from(self.grid.width) * cs,
            origin[1] + 2000.0 * ch,
            origin[2] + f32::from(self.grid.height) * cs,
        ];
    }
}
fn words(out: &mut Vec<u8>, values: impl IntoIterator<Item = u32>) {
    for v in values {
        out.extend(v.to_le_bytes());
    }
}
fn compact(out: &mut Vec<u8>, c: &Compact) {
    let g = c.grid;
    words(
        out,
        g.min
            .into_iter()
            .chain(g.max)
            .chain([g.cell_size, g.cell_height])
            .map(f32::to_bits),
    );
    words(
        out,
        [
            u32::from(c.agent.height),
            u32::from(c.agent.climb),
            c.spans.len() as u32,
        ],
    );
    for c in &c.cells {
        words(out, [c.index, u32::from(c.count)]);
    }
    for s in &c.spans {
        words(out, [u32::from(s.y), u32::from(s.region)]);
        words(out, s.connections.map(u32::from));
        words(out, [u32::from(s.height), u32::from(s.area)]);
    }
}
fn mesh(out: &mut Vec<u8>, m: &PolyMesh) {
    words(
        out,
        [
            m.vertices.len() as u32,
            m.polygons.len() as u32,
            m.maximum_polygons as u32,
            u32::from(m.nvp),
            u32::from(m.contours.regions.options.border_size),
            m.contours.options.maximum_error.to_bits(),
        ],
    );
    let g = m.contours.grid;
    words(
        out,
        g.min
            .into_iter()
            .chain(g.max)
            .chain([g.cell_size, g.cell_height])
            .map(f32::to_bits),
    );
    for v in &m.vertices {
        words(out, v.map(u32::from));
    }
    for p in &m.polygons {
        words(
            out,
            p.vertices[..usize::from(m.nvp)]
                .iter()
                .chain(&p.neighbors[..usize::from(m.nvp)])
                .copied()
                .map(u32::from),
        );
        words(
            out,
            [u32::from(p.region), u32::from(p.area), u32::from(p.flags)],
        );
    }
}
fn write(root: &Path, c: Case, index: &mut String) {
    let folder = root.join(&c.name);
    fs::create_dir(&folder).unwrap();
    let mut input = Vec::new();
    let g = c.grid;
    words(
        &mut input,
        [0x31544452, u32::from(g.width), u32::from(g.height)],
    );
    words(
        &mut input,
        g.min
            .into_iter()
            .chain(g.max)
            .chain([g.cell_size, g.cell_height])
            .map(f32::to_bits),
    );
    words(
        &mut input,
        [
            u32::from(c.agent.height),
            u32::from(c.agent.climb),
            c.spans.len() as u32,
        ],
    );
    for s in &c.spans {
        words(
            &mut input,
            [
                u32::from(s.x),
                u32::from(s.z),
                u32::from(s.span.min),
                u32::from(s.span.max),
                u32::from(s.span.area),
            ],
        );
    }
    words(
        &mut input,
        [
            c.radius.map(u32::from).unwrap_or(u32::MAX),
            u32::from(c.median),
            u32::from(c.region_options.border_size),
            c.region_options.minimum_area,
            c.region_options.merge_area,
            c.contour_options.maximum_error.to_bits(),
            u32::from(c.contour_options.maximum_edge_length),
            u32::from(c.contour_options.tessellate_walls)
                | (u32::from(c.contour_options.tessellate_areas) << 1),
            u32::from(c.nvp),
            c.region_mode,
            c.detail.sample_distance.to_bits(),
            c.detail.maximum_error.to_bits(),
        ],
    );
    fs::write(folder.join("input.bin"), input).unwrap();
    let mut field = Heightfield::new(c.grid, Limits::default())
        .unwrap()
        .add_spans(&c.spans, 0)
        .unwrap()
        .compact(c.agent)
        .unwrap();
    if let Some(radius) = c.radius {
        field = field.erode(radius).unwrap();
    }
    if c.median {
        field = field.median_filter().unwrap();
    }
    let mut mesh = field
        .build_distance_field()
        .unwrap()
        .build_regions(c.region_options)
        .unwrap()
        .build_contours(c.contour_options)
        .unwrap()
        .build_mesh(c.nvp)
        .unwrap();
    if c.region_mode != 0 {
        for p in &mut mesh.polygons {
            p.region = if c.region_mode == 1 { 0 } else { 30000 };
        }
    }
    let d = mesh
        .build_detail(c.detail)
        .unwrap_or_else(|e| panic!("{}: {e}", c.name));
    let m = &d.mesh;
    let r = &m.contours.regions;
    let mut expected = Vec::new();
    words(&mut expected, [1]);
    compact(&mut expected, &r.field.compact);
    words(
        &mut expected,
        [u32::from(r.maximum), u32::from(r.field.maximum)],
    );
    self::mesh(&mut expected, m);
    words(
        &mut expected,
        [
            d.submeshes.len() as u32,
            d.vertices.len() as u32,
            d.triangles.len() as u32,
        ],
    );
    for s in &d.submeshes {
        words(
            &mut expected,
            [
                s.vertex_base,
                s.vertex_count,
                s.triangle_base,
                s.triangle_count,
            ],
        );
    }
    for &v in &d.vertices {
        words(&mut expected, v.map(f32::to_bits));
    }
    for &t in &d.triangles {
        words(&mut expected, t.map(u32::from));
    }
    words(&mut expected, [d.discarded_faces()]);
    fs::write(folder.join("expected.bin"), expected).unwrap();
    let initial = m.polygons.iter().map(Polygon::len).sum::<usize>();
    index.push_str(&format!(
        "{}\t1 {} {} {} {} {} {} {} {} {} {} {} {} {}\n",
        c.name,
        g.cells(),
        r.field.compact.spans.len(),
        m.vertices.len(),
        m.polygons.len(),
        d.submeshes.len(),
        d.vertices.len(),
        d.triangles.len(),
        d.vertices.len() - initial,
        d.submeshes
            .iter()
            .map(|s| s.vertex_count)
            .max()
            .unwrap_or(0),
        d.submeshes
            .iter()
            .map(|s| s.triangle_count)
            .max()
            .unwrap_or(0),
        c.region_mode,
        r.field.compact.work,
        d.discarded_faces()
    ));
}
fn patterned(name: String, pattern: u16) -> Case {
    let mut c = Case::plane(name, 19, 17);
    c.spans.retain(|s| match pattern {
        4 => !((6..12).contains(&s.x) && (5..11).contains(&s.z)),
        7 => s.x < 3 || s.z < 3,
        8 => (s.x < 7 && s.z < 7) || (s.x > 11 && s.z > 9),
        _ => true,
    });
    for s in &mut c.spans {
        let (x, z) = (s.x, s.z);
        s.span.min = match pattern {
            0 => (x + z) / 4,
            1 => x / 2,
            2 => (x + z) % 5,
            3 => (x * x + z * z) % 7,
            6 => (x / 3 + z / 3) % 3,
            9 if (5..14).contains(&x) && (4..13).contains(&z) => 6,
            _ => 0,
        };
        s.span.max = s.span.min + 1;
        if pattern == 5 {
            s.span.area = if x < 9 { 8 } else { 11 };
        }
        if pattern == 6 {
            s.span.area = if (x / 4 + z / 4) % 2 == 0 { 9 } else { 11 };
        }
    }
    c
}
#[test]
#[ignore = "external synthetic original detail pipeline; explicit output root required"]
fn write_recast_detail_corpus() {
    let root = std::env::var_os("RCE_RECAST_DETAIL_CORPUS").expect("explicit private corpus root");
    let root = Path::new(&root);
    assert!(root.is_absolute());
    fs::create_dir(root).unwrap();
    let mut index = String::new();
    let mut c = Case::plane("empty", 3, 7);
    c.spans.clear();
    write(root, c, &mut index);
    write(root, Case::plane("isolated", 1, 1), &mut index);
    for (w, h) in [(7, 9), (17, 15), (32, 32)] {
        for border in [0, 1, 3] {
            for nvp in [3, 6] {
                for distance in [1.0, 3.0, 16.0] {
                    let mut c =
                        Case::plane(format!("plane-{w}-{h}-b{border}-n{nvp}-d{distance}"), w, h);
                    c.scaled(
                        if w == 7 {
                            0.25
                        } else if w == 17 {
                            0.53333336
                        } else {
                            1.0
                        },
                        0.26666668,
                        [0.0; 3],
                    );
                    c.region_options.border_size = border;
                    c.nvp = nvp;
                    c.detail = DetailOptions {
                        sample_distance: c.grid.cell_size * distance,
                        maximum_error: c.grid.cell_height,
                    };
                    write(root, c, &mut index);
                }
            }
        }
    }
    for pattern in 0..10 {
        for distance in [1.0, 3.0, 16.0] {
            for error in [0.25, 1.0] {
                let mut c = patterned(format!("shape-{pattern}-d{distance}-e{error}"), pattern);
                c.region_options.border_size = if pattern % 3 == 0 { 1 } else { 0 };
                c.detail = DetailOptions {
                    sample_distance: distance,
                    maximum_error: c.grid.cell_height * error,
                };
                write(root, c, &mut index);
            }
        }
    }
    for layers in [2, 5, 31, 63] {
        for nvp in [3, 6] {
            let mut c = Case::plane(format!("layers-{layers}-n{nvp}"), 9, 9);
            c.spans.clear();
            c.nvp = nvp;
            for z in 0..9 {
                for x in 0..9 {
                    for layer in 0..layers {
                        let y = layer * 20 + (x + z) % 3;
                        c.spans.push(SpanInput {
                            x,
                            z,
                            span: Span {
                                min: y,
                                max: y + 1,
                                area: 11,
                            },
                        });
                    }
                }
            }
            c.detail = DetailOptions {
                sample_distance: 3.0,
                maximum_error: 0.125,
            };
            write(root, c, &mut index);
        }
    }
    for radius in [0, 1, 2] {
        for pattern in [0, 1] {
            for factor in [8.0, 16.0] {
                let mut c = Case::plane(format!("subtile-r{radius}-p{pattern}-d{factor}"), 90, 90);
                c.scaled(0.53333336, 0.26666668, [0.0; 3]);
                c.radius = Some(radius);
                c.median = true;
                c.region_options = RegionOptions {
                    border_size: 5,
                    minimum_area: 3600,
                    merge_area: 2500,
                };
                c.contour_options.maximum_edge_length = 81;
                if pattern == 1 {
                    for s in &mut c.spans {
                        s.span.min = (s.x / 8 + s.z / 9) % 5;
                        s.span.max = s.span.min + 1;
                    }
                }
                c.detail = DetailOptions {
                    sample_distance: c.grid.cell_size * factor,
                    maximum_error: c.grid.cell_height,
                };
                write(root, c, &mut index);
            }
        }
    }
    for mode in [1, 2] {
        for pattern in [0, 5] {
            for border in [0, 2] {
                let mut c = patterned(format!("seed-mode{mode}-p{pattern}-b{border}"), pattern);
                c.region_mode = mode;
                c.region_options.border_size = border;
                write(root, c, &mut index);
            }
        }
    }
    let mut c = Case::plane("signed-zero-origin", 9, 9);
    c.scaled(1.0, 0.5, [-0.0; 3]);
    write(root, c, &mut index);
    let mut c = Case::plane("world-origin", 17, 19);
    c.scaled(0.53333336, 0.26666668, [16000.0, -500.0, 4000.0]);
    write(root, c, &mut index);
    let mut c = Case::plane("zero-padded-dimensions", 4, 4);
    c.region_options.border_size = 2;
    write(root, c, &mut index);
    for pattern in [2, 3] {
        let mut c = patterned(format!("dense-{pattern}"), pattern);
        c.detail = DetailOptions {
            sample_distance: 0.5,
            maximum_error: 0.0,
        };
        write(root, c, &mut index);
    }
    fs::write(root.join("cases.tsv"), index).unwrap();
}
