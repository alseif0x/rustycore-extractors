// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: bounded synthetic, stage-by-stage original Recast corpus.
use super::*;
use std::{fs, path::Path};

struct Case {
    name: String,
    grid: Grid,
    slopes: Slopes,
    agent: Agent,
    merge: u16,
    filters: u32,
    spans: Vec<SpanInput>,
    mesh: crate::Mesh,
}
impl Case {
    fn new(name: impl Into<String>, w: u16, h: u16) -> Self {
        Self {
            name: name.into(),
            grid: grid(w, h),
            slopes: Slopes {
                maximum: 70.0,
                non_steep: 55.0,
            },
            agent: Agent {
                height: 2,
                climb: 1,
            },
            merge: 1,
            filters: 7,
            spans: Vec::new(),
            mesh: crate::Mesh::default(),
        }
    }
    fn triangle(&mut self, points: [[f32; 3]; 3], area: Option<u8>) {
        let (v, t) = if area.is_some() {
            (
                &mut self.mesh.liquid_vertices,
                &mut self.mesh.liquid_triangles,
            )
        } else {
            (
                &mut self.mesh.solid_vertices,
                &mut self.mesh.solid_triangles,
            )
        };
        let n = v.len() as u32;
        v.extend(points);
        t.push([n, n + 1, n + 2]);
        if let Some(a) = area {
            self.mesh.liquid_areas.push(a);
        }
    }
}
fn words(out: &mut Vec<u8>, values: impl IntoIterator<Item = u32>) {
    for v in values {
        out.extend(v.to_le_bytes());
    }
}
fn geometry(
    out: &mut Vec<u8>,
    vertices: &[[f32; 3]],
    triangles: &[[u32; 3]],
    areas: impl IntoIterator<Item = u8>,
) {
    words(out, [vertices.len() as u32, triangles.len() as u32]);
    words(out, vertices.iter().flatten().map(|v| v.to_bits()));
    words(out, triangles.iter().flatten().copied());
    words(out, areas.into_iter().map(u32::from));
}
fn field(out: &mut Vec<u8>, h: &Heightfield) {
    for z in 0..h.grid.height {
        for x in 0..h.grid.width {
            let s: Vec<_> = h.spans(x, z).unwrap().collect();
            words(out, [s.len() as u32]);
            for s in s {
                words(out, [u32::from(s.min), u32::from(s.max), u32::from(s.area)]);
            }
        }
    }
}
fn compact(out: &mut Vec<u8>, c: &Compact) {
    words(
        out,
        c.grid()
            .min
            .into_iter()
            .chain(c.grid().max)
            .chain([c.grid().cell_size, c.grid().cell_height])
            .map(f32::to_bits),
    );
    words(
        out,
        [
            u32::from(c.agent().height),
            u32::from(c.agent().climb),
            c.spans().len() as u32,
        ],
    );
    for s in c.cells() {
        words(out, [s.index, u32::from(s.count)]);
    }
    for s in c.spans() {
        words(out, [u32::from(s.y), u32::from(s.region)]);
        words(out, s.connections.map(u32::from));
        words(out, [u32::from(s.height), u32::from(s.area)]);
    }
}
fn write(root: &Path, c: Case, index: &mut String) {
    let folder = root.join(&c.name);
    fs::create_dir(&folder).unwrap();
    let mut input = Vec::new();
    let g = c.grid;
    words(
        &mut input,
        [0x31524352, u32::from(g.width), u32::from(g.height)],
    );
    words(
        &mut input,
        g.min
            .into_iter()
            .chain(g.max)
            .chain([
                g.cell_size,
                g.cell_height,
                c.slopes.maximum,
                c.slopes.non_steep,
            ])
            .map(f32::to_bits),
    );
    words(
        &mut input,
        [
            u32::from(c.agent.height),
            u32::from(c.agent.climb),
            u32::from(c.merge),
            c.filters,
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
    let m = &c.mesh;
    geometry(
        &mut input,
        m.solid_vertices(),
        m.solid_triangles(),
        std::iter::repeat_n(10, m.solid_triangles().len()),
    );
    geometry(
        &mut input,
        m.liquid_vertices(),
        m.liquid_triangles(),
        m.liquid_areas().iter().copied(),
    );
    fs::write(folder.join("input.bin"), input).unwrap();
    let mut expected = Vec::new();
    words(&mut expected, [m.solid_triangles().len() as u32]);
    for t in m.solid_triangles() {
        words(
            &mut expected,
            [u32::from(
                solid_area(t.map(|i| m.solid_vertices()[i as usize]), c.slopes).unwrap(),
            )],
        );
    }
    let mut h = Heightfield::new(g, Limits::default())
        .unwrap()
        .add_spans(&c.spans, c.merge)
        .unwrap();
    field(&mut expected, &h);
    h = h
        .rasterize_solid(m.solid_vertices(), m.solid_triangles(), c.slopes, c.merge)
        .unwrap();
    field(&mut expected, &h);
    if c.filters & 1 != 0 {
        h = h.filter_low_obstacles(c.agent.climb).unwrap();
    }
    field(&mut expected, &h);
    if c.filters & 2 != 0 {
        h = h.filter_ledges(c.agent).unwrap();
    }
    field(&mut expected, &h);
    if c.filters & 4 != 0 {
        h = h.filter_clearance(c.agent.height).unwrap();
    }
    field(&mut expected, &h);
    h = h
        .rasterize(
            m.liquid_vertices(),
            m.liquid_triangles(),
            m.liquid_areas(),
            c.merge,
        )
        .unwrap();
    field(&mut expected, &h);
    let raw_count = h.span_count();
    let h = h.compact(c.agent).unwrap();
    let mut encoded = Vec::new();
    compact(&mut encoded, &h);
    expected.extend(&encoded);
    let composed = c.spans.is_empty() && c.filters == 7 && c.merge == c.agent.climb;
    if composed {
        let mut via_build = Vec::new();
        compact(
            &mut via_build,
            &build(m, g, c.slopes, c.agent, Limits::default()).unwrap(),
        );
        assert_eq!(encoded, via_build, "{}", c.name);
    }
    fs::write(folder.join("expected.bin"), expected).unwrap();
    index.push_str(&format!(
        "{}\t{} {} {} {} {} {} {}\n",
        c.name,
        g.cells(),
        c.spans.len(),
        m.solid_triangles().len(),
        m.liquid_triangles().len(),
        raw_count,
        h.spans().len(),
        u8::from(composed)
    ));
}
fn next(seed: &mut u32) -> u32 {
    *seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
    *seed
}

#[test]
#[ignore = "external synthetic original-Recast comparison; explicit output root required"]
fn write_recast_heightfield_corpus() {
    let root = std::env::var_os("RCE_RECAST_CORPUS").expect("explicit private corpus root");
    let root = Path::new(&root);
    assert!(root.is_absolute());
    fs::create_dir(root).unwrap();
    let mut index = String::new();
    write(root, Case::new("empty", 1, 1), &mut index);
    for height in [0, 1, 2, 3] {
        let mut c = Case::new(format!("compact-edge-{height}"), 4, 1);
        c.filters = 0;
        c.agent = Agent { height, climb: 3 };
        c.spans = vec![
            span(0, 0, 0, 2, 11),
            span(0, 0, 4, 5, 0),
            span(1, 0, 0, 2, 9),
            span(1, 0, 4, 5, 8),
            span(3, 0, 65534, 65535, 0),
        ];
        write(root, c, &mut index);
    }
    for filter in [0, 4] {
        let mut c = Case::new(format!("height-65535-{filter}"), 1, 1);
        c.filters = filter;
        c.agent.height = 0;
        c.spans = vec![span(0, 0, 65534, 65535, 11)];
        write(root, c, &mut index);
    }
    for angle in [0.0, 55.0, 70.0, 90.0] {
        let mut c = Case::new(format!("threshold-{angle}"), 2, 2);
        c.slopes = Slopes {
            maximum: angle,
            non_steep: angle,
        };
        c.triangle([[0.0, 1.0, 0.0], [0.0, 1.0, 2.0], [2.0, 1.0, 0.0]], None);
        write(root, c, &mut index);
    }
    for filters in 0..8 {
        let mut c = Case::new(format!("span-filters-{filters}"), 5, 5);
        c.filters = filters;
        for z in 0..5 {
            for x in 0..5 {
                c.spans.extend([
                    span(x, z, 0, 2, 11),
                    span(x, z, 4, 5, 0),
                    span(x, z, 8, 9, 0),
                    span(x, z, 12, 13, 9),
                ]);
            }
        }
        c.spans.push(span(2, 2, 1, 11, 8));
        write(root, c, &mut index);
    }
    for merge in [0, 1, 8, 65535] {
        let mut c = Case::new(format!("merge-{merge}"), 3, 1);
        c.merge = merge;
        c.filters = 0;
        c.spans = vec![
            span(0, 0, 0, 10, 11),
            span(0, 0, 0, 2, 8),
            span(1, 0, 8, 10, 8),
            span(1, 0, 0, 2, 1),
            span(1, 0, 5, 8, 3),
            span(1, 0, 3, 4, 2),
            span(2, 0, 4, 5, 0),
        ];
        write(root, c, &mut index);
    }
    for layers in [1, 2, 31, 63, 255] {
        let mut c = Case::new(
            format!("layers-{layers}"),
            if layers == 255 { 1 } else { 2 },
            1,
        );
        c.filters = 0;
        c.agent = Agent {
            height: 1,
            climb: 0,
        };
        for x in 0..c.grid.width {
            for i in 0..layers {
                c.spans.push(span(x, 0, 3 * i, 3 * i + 1, 11));
            }
        }
        write(root, c, &mut index);
    }
    for height in [0, 1, 2, 3, 255, 65535] {
        for climb in [0, 1, 3] {
            let mut c = Case::new(format!("agent-{height}-{climb}"), 3, 3);
            c.agent = Agent { height, climb };
            for z in 0..3 {
                for x in 0..3 {
                    c.spans.extend([
                        span(x, z, 0, 1 + x + z, 11),
                        span(x, z, 7, 8, 8),
                        span(x, z, 65533, 65534, 9),
                    ]);
                }
            }
            write(root, c, &mut index);
        }
    }
    for slope in [0.0, 1.0, 1.428148, 2.747477, 10.0] {
        for reverse in [false, true] {
            let mut c = Case::new(format!("slope-{slope}-{reverse}"), 4, 4);
            let mut p = [
                [0.0, 1.0, 0.0],
                [0.0, 1.0 + slope * 4.0, 4.0],
                [4.0, 1.0, 0.0],
            ];
            if reverse {
                p.swap(1, 2);
            }
            c.triangle(p, None);
            write(root, c, &mut index);
        }
    }
    for (i, p) in [
        [[0.0; 3]; 3],
        [[1.0, 2.0, 1.0]; 3],
        [[-4.0, -1.0, -4.0], [2.0, 5.0, 2.0], [8.0, 12.0, -4.0]],
        [[4.0, 1.0, 0.0], [4.0, 3.0, 4.0], [4.0, 2.0, 2.0]],
        [[0.0, 100.0, 0.0], [0.0, 100.0, 4.0], [4.0, 100.0, 0.0]],
        [[0.0, -1.0, 0.0], [0.0, -1.0, 4.0], [4.0, -1.0, 0.0]],
    ]
    .into_iter()
    .enumerate()
    {
        let mut c = Case::new(format!("clip-{i}"), 4, 4);
        c.triangle(p, None);
        write(root, c, &mut index);
    }
    for area in [0, 8, 9, 63] {
        let mut c = Case::new(format!("liquid-after-filter-{area}"), 4, 4);
        let p = [[0.0, 1.0, 0.0], [0.0, 1.0, 4.0], [4.0, 1.0, 0.0]];
        c.triangle(p, None);
        c.triangle(p, Some(area));
        write(root, c, &mut index);
    }
    for seed in 0..96u32 {
        let mut rng = seed + 1;
        let mut c = Case::new(format!("random-{seed:03}"), 9, 7);
        c.grid.min = [-2.0, -5.0, -3.0];
        c.grid.cell_size = if seed % 2 == 0 { 0.5333333 } else { 0.2666666 };
        c.grid.cell_height = 0.25;
        c.grid.max = [
            c.grid.min[0] + 9.0 * c.grid.cell_size,
            50.0,
            c.grid.min[2] + 7.0 * c.grid.cell_size,
        ];
        c.filters = seed % 8;
        c.agent.height = (seed % 9) as u16;
        c.agent.climb = (seed % 5) as u16;
        c.merge = c.agent.climb;
        for t in 0..64 {
            let p = std::array::from_fn(|_| {
                [
                    (next(&mut rng) % 1600) as f32 / 100.0 - 8.0,
                    (next(&mut rng) % 5000) as f32 / 100.0 - 10.0,
                    (next(&mut rng) % 1600) as f32 / 100.0 - 8.0,
                ]
            });
            c.triangle(
                p,
                if t % 4 == 0 {
                    Some([0, 8, 9, 63][(next(&mut rng) % 4) as usize])
                } else {
                    None
                },
            );
        }
        write(root, c, &mut index);
    }
    fs::write(root.join("cases.tsv"), index).unwrap();
}
