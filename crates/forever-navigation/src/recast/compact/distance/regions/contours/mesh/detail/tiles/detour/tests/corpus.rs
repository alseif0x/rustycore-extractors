// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: external synthetic supplied-mesh Detour acceptance corpus.
use super::*;
use std::{fs, path::Path};
struct Input {
    bytes: Vec<u8>,
    at: usize,
}
impl Input {
    fn word(&mut self) -> u32 {
        let n = u32::from_le_bytes(self.bytes[self.at..self.at + 4].try_into().unwrap());
        self.at += 4;
        n
    }
    fn float(&mut self) -> f32 {
        f32::from_bits(self.word())
    }
    fn point(&mut self) -> Point {
        [self.float(), self.float(), self.float()]
    }
    fn mesh(&mut self) -> TileMesh {
        let nv = self.word();
        let np = self.word();
        let maximum_polygons = self.word() as usize;
        let vertices_per_polygon = self.word() as u8;
        let border_size = self.word() as u16;
        let maximum_edge_error = self.float();
        let min = self.point();
        let max = self.point();
        let cell_size = self.float();
        let cell_height = self.float();
        let vertices = (0..nv)
            .map(|_| [self.word() as u16, self.word() as u16, self.word() as u16])
            .collect();
        let mut polygons = Vec::new();
        for _ in 0..np {
            let mut p = Polygon::new(0, 0);
            for v in &mut p.vertices[..usize::from(vertices_per_polygon)] {
                *v = self.word() as u16;
            }
            for v in &mut p.neighbors[..usize::from(vertices_per_polygon)] {
                *v = self.word() as u16;
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
        let detail_vertices = (0..dv).map(|_| self.point()).collect();
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
        TileMesh {
            header: MeshHeader {
                min,
                max,
                cell_size,
                cell_height,
                vertices_per_polygon,
                maximum_polygons,
                border_size,
                maximum_edge_error,
            },
            vertices,
            polygons,
            submeshes,
            detail_vertices,
            detail_triangles,
            discarded_faces: 0,
            processing_work: 0,
            merge_work: 0,
        }
    }
}
#[test]
#[ignore = "explicit offline original-C++ Detour acceptance with external synthetic inputs"]
fn write_detour_corpus() {
    let src = std::env::var("RCE_DETOUR_INPUTS").unwrap();
    let dst = std::env::var("RCE_DETOUR_CORPUS").unwrap();
    let root = Path::new(&dst);
    fs::create_dir(root).unwrap();
    let mut index = String::new();
    for name in fs::read_to_string(Path::new(&src).join("cases.txt"))
        .unwrap()
        .lines()
    {
        let mut input = Input {
            bytes: fs::read(Path::new(&src).join(format!("{name}.bin"))).unwrap(),
            at: 0,
        };
        assert_eq!(input.word(), 0x31544444);
        let mesh = input.mesh();
        let tile = [
            input.word() as i32,
            input.word() as i32,
            input.word() as i32,
        ];
        let user_id = input.word();
        let min = input.point();
        let max = input.point();
        let walkable_height = input.float();
        let walkable_radius = input.float();
        let walkable_climb = input.float();
        let build_bv_tree = input.word() != 0;
        let flags = if input.word() == 0 {
            FlagPolicy::Preserve
        } else {
            FlagPolicy::Forever
        };
        let n = input.word();
        let mut sources = Vec::new();
        for _ in 0..n {
            let from = input.point();
            let to = input.point();
            sources.push(crate::offmesh::Source {
                map_id: 1,
                row: 0,
                column: 0,
                from: [from[2], from[0], from[1]],
                to: [to[2], to[0], to[1]],
                radius: input.float(),
                connection_flags: input.word() as u8,
                area: input.word() as u8,
                flags: input.word() as u16,
            });
        }
        let probe = input.word();
        assert_eq!(input.at, input.bytes.len());
        let connections = crate::offmesh::select(
            crate::offmesh::Target {
                map_id: 1,
                row: 0,
                column: 0,
            },
            &sources,
            crate::offmesh::Limits::default(),
        )
        .unwrap();
        let options = Options {
            tile,
            user_id,
            min,
            max,
            walkable_height,
            walkable_radius,
            walkable_climb,
            build_bv_tree,
            flags,
        };
        let (success, data, s) =
            match build(&mesh, connections.values(), options, Limits::default()) {
                Ok(d) => (1, d.bytes, d.statistics),
                Err(Error::EmptyMesh) => (0, Vec::new(), Statistics::default()),
                Err(e) => panic!("{name}: {e}"),
            };
        let folder = root.join(name);
        fs::create_dir(&folder).unwrap();
        fs::write(folder.join("input.bin"), input.bytes).unwrap();
        fs::write(folder.join("expected.bin"), &data).unwrap();
        index.push_str(&format!(
            "{name}\t{success} {} {} {} {} {} {} {} {} {} {} {probe}\n",
            mesh.polygons().len(),
            mesh.vertices().len(),
            data.len(),
            s.stored_connections,
            s.culled_connections,
            s.maximum_links,
            s.extra_detail_vertices,
            s.detail_base_wraps,
            s.neighbor_marker_collisions,
            s.work
        ));
    }
    fs::write(root.join("cases.tsv"), index).unwrap();
}
