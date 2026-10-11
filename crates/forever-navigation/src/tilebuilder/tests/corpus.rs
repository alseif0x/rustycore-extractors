// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: external synthetic complete TileBuilder corpus.
use super::*;
use std::{env, fs, path::PathBuf};
fn words(bytes: &[u8]) -> Vec<u32> {
    bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| u32::from_le_bytes(*b))
        .collect()
}
fn save(path: PathBuf, w: &[u32]) {
    fs::write(
        path,
        w.iter().flat_map(|v| v.to_le_bytes()).collect::<Vec<_>>(),
    )
    .unwrap();
}
#[test]
#[ignore = "requires external authored inputs and exclusive output"]
fn write_tilebuilder_corpus() {
    let input = PathBuf::from(env::var_os("RCE_TILEBUILDER_INPUT").expect("input"));
    let output = PathBuf::from(env::var_os("RCE_TILEBUILDER_OUTPUT").expect("output"));
    fs::create_dir(&output).unwrap();
    let mut paths = fs::read_dir(input)
        .unwrap()
        .map(|p| p.unwrap().path())
        .collect::<Vec<_>>();
    paths.sort();
    for path in paths {
        let raw = fs::read(&path).unwrap();
        assert_eq!(raw.len() % 4, 0);
        let w = words(&raw);
        assert_eq!(w[0], 0x314c4254);
        let r = Request {
            map_id: w[1],
            tile: Coordinate {
                row: w[2] as u8,
                column: w[3] as u8,
            },
            big_base_unit: w[4] != 0,
            slopes: recast::Slopes {
                maximum: f32::from_bits(w[5]),
                non_steep: f32::from_bits(w[6]),
            },
        };
        let p = mmap::Params {
            origin: w[8..11]
                .try_into()
                .map(|a: [u32; 3]| a.map(f32::from_bits))
                .unwrap(),
            tile_width: f32::from_bits(w[11]),
            tile_height: f32::from_bits(w[12]),
            max_tiles: w[13] as i32,
            max_polys: w[14] as i32,
        };
        let (sv, st, lv, lt, nc) = (
            w[15] as usize,
            w[16] as usize,
            w[17] as usize,
            w[18] as usize,
            w[19] as usize,
        );
        let mut at = 20;
        let mut take = |n| {
            let s = &w[at..at + n];
            at += n;
            s
        };
        let solid_vertices = take(sv * 3)
            .as_chunks::<3>()
            .0
            .iter()
            .map(|a| a.map(f32::from_bits))
            .collect();
        let solid_triangles = take(st * 3).as_chunks::<3>().0.to_vec();
        let liquid_vertices = take(lv * 3)
            .as_chunks::<3>()
            .0
            .iter()
            .map(|a| a.map(f32::from_bits))
            .collect();
        let liquid_triangles = take(lt * 3).as_chunks::<3>().0.to_vec();
        let liquid_areas = take(lt).iter().map(|&v| v as u8).collect();
        let mut sources = Vec::new();
        for _ in 0..nc {
            let c = take(10);
            sources.push(offmesh::Source {
                map_id: r.map_id,
                row: u32::from(r.tile.row),
                column: u32::from(r.tile.column),
                from: [
                    f32::from_bits(c[2]),
                    f32::from_bits(c[0]),
                    f32::from_bits(c[1]),
                ],
                to: [
                    f32::from_bits(c[5]),
                    f32::from_bits(c[3]),
                    f32::from_bits(c[4]),
                ],
                radius: f32::from_bits(c[6]),
                connection_flags: c[7] as u8,
                area: c[8] as u8,
                flags: c[9] as u16,
            });
        }
        assert_eq!(at, w.len());
        let mesh = Mesh {
            solid_vertices,
            solid_triangles,
            liquid_vertices,
            liquid_triangles,
            liquid_areas,
        };
        let con = offmesh::select(
            offmesh::Target {
                map_id: r.map_id,
                row: r.tile.row,
                column: r.tile.column,
            },
            &sources,
            Default::default(),
        )
        .unwrap();
        let c = Config::new(r, &mesh).unwrap();
        let b = c.bounds();
        let co = c.contour_options();
        let re = c.region_options();
        let d = c.detail_options();
        let mut v = vec![
            c.base_unit.to_bits(),
            c.base_unit.to_bits(),
            c.cell_height.to_bits(),
            u32::from(c.tile_size),
            u32::from(c.border),
            u32::from(c.radius),
            u32::from(c.agent.height),
            u32::from(c.agent.climb),
            u32::from(co.maximum_edge_length),
            re.minimum_area,
            re.merge_area,
            6,
            co.maximum_error.to_bits(),
            d.sample_distance.to_bits(),
            d.maximum_error.to_bits(),
            r.slopes.maximum.to_bits(),
            r.slopes.non_steep.to_bits(),
            ((b.max[0] - b.min[0]) / c.base_unit + 0.5) as u32,
            ((b.max[2] - b.min[2]) / c.base_unit + 0.5) as u32,
        ];
        v.extend(b.min.map(f32::to_bits));
        v.extend(b.max.map(f32::to_bits));
        for y in 0..SUBTILES {
            for x in 0..SUBTILES {
                let g = c.grid(x, y).unwrap();
                v.extend(g.min.map(f32::to_bits));
                v.extend(g.max.map(f32::to_bits));
                v.push(u32::from(g.width));
            }
        }
        let dir = output.join(path.file_stem().unwrap());
        fs::create_dir(&dir).unwrap();
        save(dir.join("config.bin"), &v);
        if w[7] != 0 {
            let out = build(&mesh, con.values(), r, p, Limits::default()).unwrap();
            let (stats, bytes) = match out {
                Outcome::Empty(s) => (s, Vec::new()),
                Outcome::Tile(t) => {
                    let b = t.payload().bytes().to_vec();
                    (t.statistics(), b)
                }
            };
            fs::write(dir.join("payload.bin"), bytes).unwrap();
            save(
                dir.join("stats.bin"),
                &[
                    u32::from(stats.subtiles),
                    u32::from(stats.nonempty_subtiles),
                    stats.discarded_detail_faces,
                    stats.work as u32,
                    (stats.work >> 32) as u32,
                    stats.heap_reservation as u32,
                ],
            );
        }
    }
}
