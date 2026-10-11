// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original polygon comparison on supplied synthetic contours.
use super::*;
use std::{fs, path::Path};
struct Input {
    data: Vec<u8>,
    at: usize,
}
impl Input {
    fn word(&mut self) -> u32 {
        let v = u32::from_le_bytes(self.data[self.at..self.at + 4].try_into().unwrap());
        self.at += 4;
        v
    }
    fn number(&mut self) -> f32 {
        f32::from_bits(self.word())
    }
    fn vertices(&mut self, n: usize) -> Vec<Vertex> {
        (0..n)
            .map(|_| {
                [
                    self.word() as i32,
                    self.word() as i32,
                    self.word() as i32,
                    self.word() as i32,
                ]
            })
            .collect()
    }
}
fn parse(path: &Path) -> (ContourSet, u8) {
    let mut r = Input {
        data: fs::read(path).unwrap(),
        at: 0,
    };
    assert_eq!(r.word(), 0x31504d52);
    let nvp = r.word() as u8;
    let mut c = empty();
    c.grid.width = r.word() as u16;
    c.grid.height = r.word() as u16;
    c.regions.options.border_size = r.word() as u16;
    c.options.maximum_error = r.number();
    c.grid.min = [r.number(), r.number(), r.number()];
    c.grid.max = [r.number(), r.number(), r.number()];
    c.grid.cell_size = r.number();
    c.grid.cell_height = r.number();
    let count = r.word();
    for _ in 0..count {
        let region = r.word() as u16;
        let area = r.word() as u8;
        let n = r.word() as usize;
        let nr = r.word() as usize;
        c.contours.push(Contour {
            vertices: r.vertices(n),
            raw: r.vertices(nr),
            region,
            area,
        });
    }
    assert_eq!(r.at, r.data.len());
    (c, nvp)
}
fn word(out: &mut Vec<u8>, v: u32) {
    out.extend(v.to_le_bytes());
}
fn output(mesh: &PolyMesh) -> Vec<u8> {
    let mut out = Vec::new();
    for v in [
        1,
        mesh.vertices.len() as u32,
        mesh.polygons.len() as u32,
        mesh.maximum_polygons as u32,
        u32::from(mesh.nvp),
        u32::from(mesh.contours.regions.options.border_size),
        mesh.contours.options.maximum_error.to_bits(),
    ] {
        word(&mut out, v);
    }
    let g = mesh.contours.grid;
    for v in g
        .min
        .into_iter()
        .chain(g.max)
        .chain([g.cell_size, g.cell_height])
    {
        word(&mut out, v.to_bits());
    }
    for v in &mesh.vertices {
        for &n in v {
            word(&mut out, u32::from(n));
        }
    }
    for p in &mesh.polygons {
        for &v in p.vertices[..usize::from(mesh.nvp)]
            .iter()
            .chain(&p.neighbors[..usize::from(mesh.nvp)])
        {
            word(&mut out, u32::from(v));
        }
        for v in [u32::from(p.region), u32::from(p.area), u32::from(p.flags)] {
            word(&mut out, v);
        }
    }
    out
}
#[test]
#[ignore = "external synthetic original polygon comparison; explicit roots required"]
fn write_recast_mesh_corpus() {
    let inputs =
        std::env::var_os("RCE_RECAST_MESH_INPUTS").expect("explicit synthetic source root");
    let root = std::env::var_os("RCE_RECAST_MESH_CORPUS").expect("explicit private output root");
    let (inputs, root) = (Path::new(&inputs), Path::new(&root));
    assert!(inputs.is_absolute() && root.is_absolute());
    fs::create_dir(root).unwrap();
    let mut paths: Vec<_> = fs::read_dir(inputs)
        .unwrap()
        .map(|p| p.unwrap().path())
        .collect();
    paths.sort();
    let mut index = String::new();
    for path in paths {
        assert_eq!(path.extension().unwrap(), "bin");
        let name = path.file_stem().unwrap().to_str().unwrap();
        let folder = root.join(name);
        fs::create_dir(&folder).unwrap();
        fs::copy(&path, folder.join("input.bin")).unwrap();
        let (c, nvp) = parse(&path);
        match c.build_mesh(nvp) {
            Ok(m) => {
                fs::write(folder.join("expected.bin"), output(&m)).unwrap();
                let portals = m
                    .polygons
                    .iter()
                    .flat_map(|p| p.neighbors())
                    .filter(|&&v| v != NULL && v & 0x8000 != 0)
                    .count();
                let links = m
                    .polygons
                    .iter()
                    .flat_map(|p| p.neighbors())
                    .filter(|&&v| v < 0x8000)
                    .count();
                let mixed = m.polygons.iter().filter(|p| p.region == 0).count();
                index.push_str(&format!(
                    "{name}\t1 {} {} {} {} {} {} {}\n",
                    m.vertices.len(),
                    m.polygons.len(),
                    m.maximum_polygons,
                    portals,
                    links,
                    mixed,
                    m.contours.regions.field.compact.work
                ));
            }
            Err(Error::Triangulation) => {
                fs::write(folder.join("expected.bin"), 2u32.to_le_bytes()).unwrap();
                index.push_str(&format!("{name}\t2 0 0 0 0 0 0 0\n"));
            }
            Err(e) => panic!("{name}: {e}"),
        }
    }
    fs::write(root.join("cases.tsv"), index).unwrap();
}
