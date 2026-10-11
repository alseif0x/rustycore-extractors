// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: native watershed positives from wholly synthetic spans.
use super::*;
use std::{fs, path::Path};
struct Case {
    name: String,
    w: u16,
    h: u16,
    spans: Vec<SpanInput>,
    agent: Agent,
    radius: Option<u16>,
    median: bool,
    options: RegionOptions,
}
impl Case {
    fn new(name: impl Into<String>, w: u16, h: u16) -> Self {
        Self {
            name: name.into(),
            w,
            h,
            spans: Vec::new(),
            agent: Agent {
                height: 1,
                climb: 0,
            },
            radius: None,
            median: false,
            options: options(),
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
            min: [0.0; 3],
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
fn snapshot(out: &mut Vec<u8>, c: &Compact) {
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
    words(&mut input, [0x31475252, u32::from(c.w), u32::from(c.h)]);
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
            u32::from(c.options.border_size),
            c.options.minimum_area,
            c.options.merge_area,
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
    let mut expected = Vec::new();
    snapshot(&mut expected, d.compact());
    words(
        &mut expected,
        [u32::from(d.maximum()), d.values().len() as u32],
    );
    words(&mut expected, d.values().iter().copied().map(u32::from));
    let r = d
        .build_regions(c.options)
        .unwrap_or_else(|e| panic!("{}: {e}", c.name));
    snapshot(&mut expected, r.distance_field().compact());
    words(
        &mut expected,
        [
            u32::from(r.options().border_size),
            u32::from(r.maximum()),
            r.overlapping_regions().len() as u32,
        ],
    );
    fs::write(folder.join("expected.bin"), expected).unwrap();
    let spans = r.distance_field().compact().spans();
    let active = spans
        .iter()
        .filter(|s| s.region > 0 && s.region & BORDER == 0)
        .count();
    let borders = spans.iter().filter(|s| s.region & BORDER != 0).count();
    index.push_str(&format!(
        "{}\t{} {} {} {} {} {} {}\n",
        c.name,
        g.cells(),
        spans.len(),
        r.maximum(),
        active,
        borders,
        r.overlapping_regions().len(),
        r.distance_field().compact().work_used()
    ));
}
fn next(seed: &mut u32) -> u32 {
    *seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
    *seed
}
#[test]
#[ignore = "external synthetic original-watershed comparison; explicit output root required"]
fn write_recast_regions_corpus() {
    let root = std::env::var_os("RCE_RECAST_REGIONS_CORPUS").expect("explicit private corpus root");
    let root = Path::new(&root);
    assert!(root.is_absolute());
    fs::create_dir(root).unwrap();
    let mut index = String::new();
    write(root, Case::new("empty", 3, 7), &mut index);
    for (w, h) in [(1, 1), (1, 13), (3, 3), (7, 9), (17, 15)] {
        for border in [0, 1, 3, 65535] {
            for minimum in [0, 40, 3600] {
                let mut c = Case::plane(format!("plane-{w}-{h}-b{border}-n{minimum}"), w, h);
                c.options = RegionOptions {
                    border_size: border,
                    minimum_area: minimum,
                    merge_area: 2500,
                };
                write(root, c, &mut index);
            }
        }
    }
    for pattern in 0..10 {
        for merge in [0, 20, 400, 2500] {
            let mut c = Case::plane(format!("shape-{pattern}-m{merge}"), 45, 35);
            c.options.merge_area = merge;
            c.options.border_size = if pattern % 3 == 0 { 2 } else { 0 };
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
                    s.span.min = s.x / 4;
                    s.span.max = s.span.min + 1;
                    c.agent.climb = 1;
                }
            }
            write(root, c, &mut index);
        }
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
    for seed in 0..72u32 {
        let mut rng = seed + 1;
        let mut c = Case::plane(format!("varied-{seed:03}"), 31, 29);
        c.radius = Some((seed % 3) as u16);
        c.median = seed % 2 == 0;
        c.options = RegionOptions {
            border_size: (seed % 5) as u16,
            minimum_area: (seed % 4) * 12,
            merge_area: (seed % 7) * 30,
        };
        c.spans.retain(|_| !next(&mut rng).is_multiple_of(9));
        for s in &mut c.spans {
            if next(&mut rng).is_multiple_of(13) {
                s.span.area = 9;
            }
        }
        write(root, c, &mut index);
    }
    for radius in [0, 1, 2] {
        let mut c = Case::plane(format!("subtile-90-r{radius}"), 90, 90);
        c.radius = Some(radius);
        c.median = true;
        c.options = RegionOptions {
            border_size: 5,
            minimum_area: 3600,
            merge_area: 2500,
        };
        write(root, c, &mut index);
    }
    let mut c = Case::plane("distance-levels-129", 129, 129);
    c.options.border_size = 4;
    write(root, c, &mut index);
    for bridge in 0..8 {
        let mut c = Case::new(format!("overlap-bridge-{bridge}"), 9, 9);
        c.agent.climb = 10;
        for z in 0..9 {
            for x in 0..9 {
                if bridge == 0 || !((x == 1 && z == 4) || (bridge > 1 && x == 7 && z == 4)) {
                    c.spans.push(span(x, z, 0, 11));
                }
                if (1..8).contains(&x) && (1..8).contains(&z) {
                    c.spans
                        .push(span(x, z, 10, if bridge == 3 { 9 } else { 11 }));
                }
            }
        }
        if bridge >= 4 {
            c.options.border_size = 1;
            c.options.minimum_area = if bridge == 5 { 500 } else { 0 };
            c.options.merge_area = if bridge == 6 { 2500 } else { 0 };
        }
        write(root, c, &mut index);
    }
    for minimum in [0, 100] {
        let mut c = Case::new(format!("small-overlap-n{minimum}"), 5, 3);
        c.agent.climb = 10;
        c.options.minimum_area = minimum;
        for z in 0..3 {
            for x in 0..5 {
                if (x, z) != (1, 1) {
                    c.spans.push(span(x, z, 0, 11));
                }
                if (1..4).contains(&x) && z == 1 {
                    c.spans.push(span(x, z, 10, 11));
                }
            }
        }
        write(root, c, &mut index);
    }
    fs::write(root.join("cases.tsv"), index).unwrap();
}
