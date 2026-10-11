// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: supplied synthetic polygon/detail merge corpus.
use super::*;
use std::{fs, path::Path};
struct Input {
    bytes: Vec<u8>,
    at: usize,
}
impl Input {
    fn word(&mut self) -> u32 {
        let b = &self.bytes[self.at..self.at + 4];
        self.at += 4;
        u32::from_le_bytes(b.try_into().unwrap())
    }
    fn float(&mut self) -> f32 {
        f32::from_bits(self.word())
    }
    fn mesh(&mut self) -> TileMesh {
        let nv = self.word() as usize;
        let np = self.word() as usize;
        let max = self.word() as usize;
        let nvp = self.word() as u8;
        let border = self.word() as u16;
        let error = self.float();
        let min = [self.float(), self.float(), self.float()];
        let max_bounds = [self.float(), self.float(), self.float()];
        let cs = self.float();
        let ch = self.float();
        let vertices = (0..nv)
            .map(|_| [self.word() as u16, self.word() as u16, self.word() as u16])
            .collect();
        let mut polygons = Vec::new();
        for _ in 0..np {
            let mut p = Polygon::new(0, 0);
            for v in &mut p.vertices[..usize::from(nvp)] {
                *v = self.word() as u16;
            }
            for n in &mut p.neighbors[..usize::from(nvp)] {
                *n = self.word() as u16;
            }
            p.region = self.word() as u16;
            p.area = self.word() as u8;
            p.flags = self.word() as u16;
            polygons.push(p);
        }
        let nm = self.word();
        let dv = self.word();
        let dt = self.word();
        let submeshes = (0..nm)
            .map(|_| DetailSubmesh {
                vertex_base: self.word(),
                vertex_count: self.word(),
                triangle_base: self.word(),
                triangle_count: self.word(),
            })
            .collect();
        let detail_vertices = (0..dv)
            .map(|_| [self.float(), self.float(), self.float()])
            .collect();
        let detail_triangles = (0..dt)
            .map(|_| {
                [
                    self.word() as u8,
                    self.word() as u8,
                    self.word() as u8,
                    self.word() as u8,
                ]
            })
            .collect();
        let discarded_faces = self.word();
        TileMesh {
            header: MeshHeader {
                min,
                max: max_bounds,
                cell_size: cs,
                cell_height: ch,
                vertices_per_polygon: nvp,
                maximum_polygons: max,
                border_size: border,
                maximum_edge_error: error,
            },
            vertices,
            polygons,
            submeshes,
            detail_vertices,
            detail_triangles,
            discarded_faces,
            processing_work: 0,
            merge_work: 0,
        }
    }
}
fn words(out: &mut Vec<u8>, values: impl IntoIterator<Item = u32>) {
    for v in values {
        out.extend(v.to_le_bytes());
    }
}
fn serialize(m: &TileMesh) -> Vec<u8> {
    let mut out = Vec::new();
    let h = m.header;
    words(
        &mut out,
        [
            m.vertices.len() as u32,
            m.polygons.len() as u32,
            h.maximum_polygons as u32,
            u32::from(h.vertices_per_polygon),
            u32::from(h.border_size),
            h.maximum_edge_error.to_bits(),
        ],
    );
    words(
        &mut out,
        h.min
            .into_iter()
            .chain(h.max)
            .chain([h.cell_size, h.cell_height])
            .map(f32::to_bits),
    );
    for v in &m.vertices {
        words(&mut out, v.map(u32::from));
    }
    for p in &m.polygons {
        words(
            &mut out,
            p.vertices[..usize::from(h.vertices_per_polygon)]
                .iter()
                .chain(&p.neighbors[..usize::from(h.vertices_per_polygon)])
                .copied()
                .map(u32::from),
        );
        words(
            &mut out,
            [u32::from(p.region), u32::from(p.area), u32::from(p.flags)],
        );
    }
    words(
        &mut out,
        [
            m.submeshes.len() as u32,
            m.detail_vertices.len() as u32,
            m.detail_triangles.len() as u32,
        ],
    );
    for s in &m.submeshes {
        words(
            &mut out,
            [
                s.vertex_base,
                s.vertex_count,
                s.triangle_base,
                s.triangle_count,
            ],
        );
    }
    for v in &m.detail_vertices {
        words(&mut out, v.map(f32::to_bits));
    }
    for t in &m.detail_triangles {
        words(&mut out, t.map(u32::from));
    }
    out
}
#[test]
#[ignore = "explicit offline original-C++ merge acceptance; external synthetic inputs/outputs"]
fn write_recast_merge_corpus() {
    let src = std::env::var("RCE_RECAST_MERGE_INPUTS").unwrap();
    let dst = std::env::var("RCE_RECAST_MERGE_CORPUS").unwrap();
    let root = Path::new(&dst);
    fs::create_dir(root).unwrap();
    let mut index = String::new();
    for entry in fs::read_to_string(Path::new(&src).join("cases.txt"))
        .unwrap()
        .lines()
    {
        let source = Path::new(&src).join(format!("{entry}.bin"));
        let bytes = fs::read(source).unwrap();
        let mut input = Input { bytes, at: 0 };
        assert_eq!(input.word(), 0x31474d52);
        let n = input.word();
        let meshes = (0..n).map(|_| input.mesh()).collect::<Vec<_>>();
        assert_eq!(input.at, input.bytes.len());
        let source_vertices = meshes.iter().map(|m| m.vertices.len()).sum::<usize>();
        let m = TileMesh::merge(meshes, MergeLimits::default())
            .unwrap_or_else(|e| panic!("{entry}: {e}"));
        let folder = root.join(entry);
        fs::create_dir(&folder).unwrap();
        fs::write(folder.join("input.bin"), input.bytes).unwrap();
        fs::write(folder.join("expected.bin"), serialize(&m)).unwrap();
        index.push_str(&format!(
            "{entry}\t{n} {source_vertices} {} {} {} {} {} {} {} {} {}\n",
            m.vertices.len(),
            m.polygons.len(),
            m.submeshes.len(),
            m.detail_vertices.len(),
            m.detail_triangles.len(),
            portals(&m),
            links(&m),
            m.discarded_faces(),
            m.merge_work()
        ));
    }
    fs::write(root.join("cases.tsv"), index).unwrap();
}
