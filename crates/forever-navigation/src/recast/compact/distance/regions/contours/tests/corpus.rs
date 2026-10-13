// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: synthetic native contour pipeline corpus.
use super::*;
use std::{fs, path::Path};
struct Case {
    name: String,
    w: u16,
    h: u16,
    origin: [f32; 3],
    spans: Vec<SpanInput>,
    agent: Agent,
    radius: Option<u16>,
    median: bool,
    supplied_single_region: bool,
    regions: RegionOptions,
    contours: ContourOptions,
}
impl Case {
    fn new(name: impl Into<String>, w: u16, h: u16) -> Self {
        Self {
            name: name.into(),
            w,
            h,
            origin: [0.0; 3],
            spans: Vec::new(),
            agent: Agent {
                height: 1,
                climb: 0,
            },
            radius: None,
            median: false,
            supplied_single_region: false,
            regions: RegionOptions {
                border_size: 0,
                minimum_area: 0,
                merge_area: 2500,
            },
            contours: ContourOptions::default(),
        }
    }
    fn plane(name: impl Into<String>, w: u16, h: u16) -> Self {
        let mut c = Self::new(name, w, h);
        for z in 0..h {
            for x in 0..w {
                c.spans.push(span(x, z, 0, 11));
            }
        }
        c
    }
    fn grid(&self) -> Grid {
        Grid {
            width: self.w,
            height: self.h,
            min: self.origin,
            max: [f32::from(self.w), 2000.0, f32::from(self.h)],
            cell_size: 1.0,
            cell_height: 1.0,
        }
    }
}
fn span(x: u16, z: u16, y: u16, area: u8) -> SpanInput {
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
fn words(out: &mut Vec<u8>, values: impl IntoIterator<Item = u32>) {
    for v in values {
        out.extend(v.to_le_bytes());
    }
}
fn compact(out: &mut Vec<u8>, c: &Compact) {
    let g = c.grid();
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
            u32::from(c.agent().height),
            u32::from(c.agent().climb),
            c.spans().len() as u32,
        ],
    );
    for c in c.cells() {
        words(out, [c.index, u32::from(c.count)]);
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
    let g = c.grid();
    words(&mut input, [0x31544352, u32::from(c.w), u32::from(c.h)]);
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
            u32::from(c.regions.border_size),
            c.regions.minimum_area,
            c.regions.merge_area,
            c.contours.maximum_error.to_bits(),
            u32::from(c.contours.maximum_edge_length),
            u32::from(c.contours.tessellate_walls) | u32::from(c.contours.tessellate_areas) << 1,
            u32::from(c.supplied_single_region),
        ],
    );
    fs::write(folder.join("input.bin"), input).unwrap();
    let mut f = Heightfield::new(g, Limits::default())
        .unwrap()
        .add_spans(&c.spans, 0)
        .unwrap()
        .compact(c.agent)
        .unwrap();
    if let Some(radius) = c.radius {
        f = f.erode(radius).unwrap();
    }
    if c.median {
        f = f.median_filter().unwrap();
    }
    let d = f.build_distance_field().unwrap();
    let mut r = d.build_regions(c.regions).unwrap();
    if c.supplied_single_region {
        // Separate synthetic contour-only evidence. This deliberately supplies
        // region labels; it is not a watershed-generation acceptance claim.
        assert_eq!(c.regions.border_size, 0);
        for s in &mut r.field.compact.spans {
            if s.region != 0 {
                s.region = 1;
            }
        }
        r.maximum = 1;
    }
    let out = r
        .build_contours(c.contours)
        .unwrap_or_else(|e| panic!("{}: {e}", c.name));
    let mut expected = Vec::new();
    compact(&mut expected, out.regions().distance_field().compact());
    words(
        &mut expected,
        [
            u32::from(out.regions().maximum()),
            u32::from(out.regions().distance_field().maximum()),
        ],
    );
    let g = out.grid();
    words(
        &mut expected,
        g.min
            .into_iter()
            .chain(g.max)
            .chain([g.cell_size, g.cell_height])
            .map(f32::to_bits),
    );
    words(
        &mut expected,
        [
            u32::from(g.width),
            u32::from(g.height),
            u32::from(c.regions.border_size),
            c.contours.maximum_error.to_bits(),
            out.contours().len() as u32,
        ],
    );
    for contour in out.contours() {
        words(
            &mut expected,
            [
                u32::from(contour.region()),
                u32::from(contour.area()),
                contour.vertices().len() as u32,
                contour.raw_vertices().len() as u32,
            ],
        );
        words(
            &mut expected,
            contour
                .vertices()
                .iter()
                .chain(contour.raw_vertices())
                .flatten()
                .map(|&n| n as u32),
        );
    }
    fs::write(folder.join("expected.bin"), expected).unwrap();
    index.push_str(&format!(
        "{}\t{} {} {} {} {} {} {}\n",
        c.name,
        c.grid().cells(),
        out.regions().distance_field().values().len(),
        out.contours().len(),
        out.contours()
            .iter()
            .map(|c| c.vertices().len())
            .sum::<usize>(),
        out.contours()
            .iter()
            .map(|c| c.raw_vertices().len())
            .sum::<usize>(),
        out.contours()
            .iter()
            .filter(|c| c.vertices().is_empty())
            .count(),
        out.regions().distance_field().compact().work_used()
    ));
}
#[test]
#[ignore = "external synthetic original contour comparison; explicit output root required"]
fn write_recast_contours_corpus() {
    let root =
        std::env::var_os("RCE_RECAST_CONTOURS_CORPUS").expect("explicit private corpus root");
    let root = Path::new(&root);
    assert!(root.is_absolute());
    fs::create_dir(root).unwrap();
    let mut index = String::new();
    write(root, Case::new("empty", 3, 7), &mut index);
    write(root, Case::plane("isolated", 1, 1), &mut index);
    let mut signed = Case::plane("negative-zero-origin", 7, 9);
    signed.origin = [-0.0; 3];
    write(root, signed, &mut index);
    for (w, h) in [(7, 9), (17, 15), (32, 32)] {
        for border in [0, 1, 3] {
            for flags in 0..4 {
                for error in [0.0, 1.8, 4.0] {
                    let mut c =
                        Case::plane(format!("plane-{w}-{h}-b{border}-f{flags}-e{error}"), w, h);
                    c.regions.border_size = border;
                    c.contours = ContourOptions {
                        maximum_error: error,
                        maximum_edge_length: if flags % 2 == 0 { 0 } else { 3 },
                        tessellate_walls: flags & 1 != 0,
                        tessellate_areas: flags & 2 != 0,
                    };
                    write(root, c, &mut index);
                }
            }
        }
    }
    for pattern in 0..10 {
        for error in [0.0, 1.8, 4.0] {
            let mut c = Case::plane(format!("shape-{pattern}-e{error}"), 45, 35);
            c.contours.maximum_error = error;
            c.contours.maximum_edge_length = 11;
            c.contours.tessellate_areas = true;
            c.regions.border_size = if pattern % 3 == 0 { 2 } else { 0 };
            c.spans.retain(|s| {
                let (x, z) = (s.x, s.z);
                match pattern {
                    0 => !(x > 8 && x < 36 && z > 8 && z < 26),
                    1 => !(18..=26).contains(&x) || z == 17,
                    2 => x % 8 != 4 || z % 12 < 3,
                    3 => !((x == 8 || x == 36) && z > 4 && z < 30),
                    4 => (x < 12 && z < 12) || (x > 30 && z > 20),
                    5 => (x + z) % 7 != 0,
                    6 => x % 5 != 0 && z % 5 != 0,
                    _ => true,
                }
            });
            for s in &mut c.spans {
                if pattern == 7 {
                    s.span.area = if s.x < 22 { 8 } else { 11 };
                }
                if pattern == 8 {
                    s.span.area = if (s.x / 5 + s.z / 5) % 2 == 0 { 9 } else { 11 };
                }
                if pattern == 9 {
                    s.span.min = (s.x / 3 + s.z / 4) % 3;
                    s.span.max = s.span.min + 1;
                    c.agent.climb = 3;
                }
            }
            write(root, c, &mut index);
        }
    }
    for holes in [1, 2, 3, 8] {
        let mut c = Case::plane(format!("holes-{holes}"), 65, 33);
        c.spans.retain(|s| {
            !(s.z > 12 && s.z < 20 && (0..holes).any(|i| s.x > 3 + i * 7 && s.x < 8 + i * 7))
        });
        write(root, c, &mut index);
    }
    for holes in [2, 3, 8] {
        let mut c = Case::plane(format!("supplied-one-region-holes-{holes}"), 65, 33);
        c.supplied_single_region = true;
        c.contours.maximum_edge_length = 3;
        c.spans.retain(|s| {
            !(s.z > 12 && s.z < 20 && (0..holes).any(|i| s.x > 3 + i * 7 && s.x < 8 + i * 7))
        });
        write(root, c, &mut index);
    }
    for layers in [2, 5, 31, 63] {
        let mut c = Case::new(format!("layers-{layers}"), 9, 9);
        for z in 0..9 {
            for x in 0..9 {
                for layer in 0..layers {
                    c.spans
                        .push(span(x, z, layer * 4, if layer % 2 == 0 { 11 } else { 8 }));
                }
            }
        }
        write(root, c, &mut index);
    }
    for radius in [0, 1, 2] {
        let mut c = Case::plane(format!("subtile-90-r{radius}"), 90, 90);
        c.radius = Some(radius);
        c.median = true;
        c.regions = RegionOptions {
            border_size: 5,
            minimum_area: 3600,
            merge_area: 2500,
        };
        c.contours.maximum_edge_length = 81;
        write(root, c, &mut index);
    }
    fs::write(root.join("cases.tsv"), index).unwrap();
}
