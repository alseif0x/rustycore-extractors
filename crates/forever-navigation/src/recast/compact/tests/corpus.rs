// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: synthetic original erosion, area median and distance corpus.
use super::*;
use std::{fs, path::Path};
struct Case {
    name: String,
    grid: Grid,
    agent: Agent,
    spans: Vec<SpanInput>,
    radii: Vec<u16>,
    medians: u32,
}
impl Case {
    fn new(name: impl Into<String>, w: u16, h: u16) -> Self {
        Self {
            name: name.into(),
            grid: grid(w, h),
            agent: Agent {
                height: 1,
                climb: 0,
            },
            spans: Vec::new(),
            radii: vec![1],
            medians: 1,
        }
    }
    fn plane(name: impl Into<String>, w: u16, h: u16) -> Self {
        let mut c = Self::new(name, w, h);
        for z in 0..h {
            for x in 0..w {
                c.spans.push(entry(x, z, 0, 11));
            }
        }
        c
    }
    fn compact(&self) -> Compact {
        Heightfield::new(self.grid, Limits::default())
            .unwrap()
            .add_spans(&self.spans, 0)
            .unwrap()
            .compact(self.agent)
            .unwrap()
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
    let g = c.grid;
    words(
        &mut input,
        [0x31434352, u32::from(g.width), u32::from(g.height)],
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
    words(&mut input, [c.radii.len() as u32]);
    words(&mut input, c.radii.iter().copied().map(u32::from));
    words(&mut input, [c.medians]);
    fs::write(folder.join("input.bin"), input).unwrap();
    let mut expected = Vec::new();
    let mut f = c.compact();
    snapshot(&mut expected, &f);
    for radius in &c.radii {
        f = f.erode(*radius).unwrap();
        snapshot(&mut expected, &f);
    }
    for _ in 0..c.medians {
        f = f.median_filter().unwrap();
        snapshot(&mut expected, &f);
    }
    let d = f.build_distance_field().unwrap();
    snapshot(&mut expected, d.compact());
    words(
        &mut expected,
        [u32::from(d.maximum()), d.values().len() as u32],
    );
    words(&mut expected, d.values().iter().copied().map(u32::from));
    let composed = c.radii.len() == 1 && c.medians == 1;
    if composed {
        let e = c.compact().prepare(c.radii[0]).unwrap();
        assert_eq!(e.values(), d.values(), "{}", c.name);
        assert_eq!(e.maximum(), d.maximum());
        assert_eq!(e.compact().spans(), d.compact().spans());
    }
    fs::write(folder.join("expected.bin"), expected).unwrap();
    index.push_str(&format!(
        "{}\t{} {} {} {} {} {} {}\n",
        c.name,
        g.cells(),
        c.spans.len(),
        d.values().len(),
        c.radii.len(),
        c.medians,
        d.maximum(),
        u8::from(composed)
    ));
}
fn next(seed: &mut u32) -> u32 {
    *seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
    *seed
}

#[test]
#[ignore = "external synthetic original-Recast compact comparison; explicit output root required"]
fn write_recast_compact_corpus() {
    let root = std::env::var_os("RCE_RECAST_COMPACT_CORPUS").expect("explicit private corpus root");
    let root = Path::new(&root);
    assert!(root.is_absolute());
    fs::create_dir(root).unwrap();
    let mut index = String::new();
    write(root, Case::new("empty", 3, 5), &mut index);
    for (w, h) in [(1, 1), (1, 13), (13, 1), (3, 3), (7, 9), (15, 13)] {
        for radius in [0, 1, 2, 3, 127] {
            let mut c = Case::plane(format!("plane-{w}-{h}-r{radius}"), w, h);
            c.radii = vec![radius];
            write(root, c, &mut index);
        }
    }
    for pattern in 0..8 {
        for medians in 0..=2 {
            let mut c = Case::plane(format!("areas-{pattern}-m{medians}"), 19, 17);
            c.radii = if pattern % 2 == 0 { vec![] } else { vec![1, 1] };
            c.medians = medians;
            for s in &mut c.spans {
                let (x, z) = (s.x, s.z);
                s.span.area = match pattern {
                    0 => {
                        if (x, z) == (9, 8) {
                            8
                        } else {
                            11
                        }
                    }
                    1 => {
                        if x == 9 {
                            0
                        } else {
                            11
                        }
                    }
                    2 => {
                        if x % 2 == 0 {
                            8
                        } else {
                            11
                        }
                    }
                    3 => {
                        if (x + z) % 2 == 0 {
                            9
                        } else {
                            11
                        }
                    }
                    4 => {
                        if x < 9 {
                            8
                        } else {
                            11
                        }
                    }
                    5 => {
                        if (x, z) == (9, 8) {
                            0
                        } else {
                            11
                        }
                    }
                    6 => {
                        if (x + z) % 3 == 0 {
                            63
                        } else {
                            1
                        }
                    }
                    _ => 0,
                };
            }
            write(root, c, &mut index);
        }
    }
    for layers in [2, 7, 63, 255] {
        let (w, h) = if layers == 255 { (1, 1) } else { (5, 7) };
        let mut c = Case::new(format!("layers-{layers}"), w, h);
        for z in 0..h {
            for x in 0..w {
                for layer in 0..layers {
                    c.spans
                        .push(entry(x, z, layer * 4, if layer % 2 == 0 { 11 } else { 9 }));
                }
            }
        }
        write(root, c, &mut index);
    }
    for height in [0, 1, 2, 3] {
        let mut c = Case::new(format!("asymmetric-{height}"), 9, 9);
        c.agent = Agent { height, climb: 20 };
        c.radii = vec![0];
        for z in 0..9 {
            for x in 0..9 {
                for layer in 0..4 {
                    c.spans.push(entry(
                        x,
                        z,
                        layer * 8 + (x + z) % 3,
                        if (x + z + layer) % 3 == 0 { 8 } else { 11 },
                    ));
                }
            }
        }
        write(root, c, &mut index);
    }
    let mut c = Case::new("height65535", 7, 7);
    c.agent.height = 0;
    for z in 0..7 {
        for x in 0..7 {
            c.spans.push(entry(x, z, 65534, 11));
        }
    }
    write(root, c, &mut index);
    for radius in [0, 1, 127] {
        let mut c = Case::plane(format!("saturated-259-r{radius}"), 259, 257);
        c.radii = vec![radius];
        write(root, c, &mut index);
    }
    let mut c = Case::plane("wide-distance-513", 513, 513);
    c.radii = vec![];
    c.medians = 0;
    write(root, c, &mut index);
    for seed in 0..96u32 {
        let mut rng = seed + 1;
        let mut c = Case::new(format!("varied-{seed:03}"), 23, 19);
        c.agent = Agent {
            height: (seed % 4) as u16,
            climb: (seed % 9) as u16,
        };
        c.radii = if seed % 5 == 0 {
            vec![]
        } else {
            vec![(seed % 5) as u16]
        };
        c.medians = seed % 3;
        for z in 0..19 {
            for x in 0..23 {
                let layers = next(&mut rng) % 4;
                for layer in 0..layers {
                    let y = (layer * 12 + next(&mut rng) % 5) as u16;
                    let area = [0, 8, 9, 10, 11, 63][(next(&mut rng) % 6) as usize];
                    c.spans.push(entry(x, z, y, area));
                }
            }
        }
        write(root, c, &mut index);
    }
    fs::write(root.join("cases.tsv"), index).unwrap();
}
