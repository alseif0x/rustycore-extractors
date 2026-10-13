// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: synthetic supplied-model/native navigation corpus.
use super::*;
use crate::{offmesh, terrain};
use forever_terrain::{height, maps};
use std::{fs, path::Path};

fn terrain_input() -> Vec<u8> {
    let v9: Vec<_> = (0..height::V9_COUNT)
        .map(|i| (i as f32 * 0.01).to_bits())
        .collect();
    let v8: Vec<_> = (0..height::V8_COUNT)
        .map(|i| (i as f32 * 0.03).to_bits())
        .collect();
    let h = height::HeightSection::from_bits(
        &v9,
        &v8,
        height::Options::default(),
        None,
        height::MAX_SECTION_BYTES,
    )
    .unwrap();
    let a = maps::AreaSection::from_ids(&[11; 256]);
    maps::assemble(
        70205,
        maps::MapSections {
            area: &a,
            height: &h,
            liquid: None,
            holes: None,
        },
        maps::MAX_MAP_BYTES,
    )
    .unwrap()
}
fn encoded(mesh: &Mesh, connections: &offmesh::Connections) -> Vec<u8> {
    let mut out = Vec::new();
    for (v, t) in [
        (mesh.solid_vertices(), mesh.solid_triangles()),
        (mesh.liquid_vertices(), mesh.liquid_triangles()),
    ] {
        words(&mut out, [(v.len() * 3) as u32]);
        words(&mut out, v.iter().flatten().map(|v| v.to_bits()));
        words(&mut out, [(t.len() * 3) as u32]);
        words(&mut out, t.iter().flatten().copied());
    }
    words(&mut out, [mesh.liquid_areas().len() as u32]);
    out.extend(mesh.liquid_areas());
    let c = connections.values();
    words(&mut out, [(c.len() * 6) as u32]);
    for c in c {
        words(
            &mut out,
            c.endpoints().into_iter().flatten().map(f32::to_bits),
        );
    }
    words(&mut out, [c.len() as u32]);
    words(&mut out, c.iter().map(|c| c.radius().to_bits()));
    words(&mut out, [c.len() as u32]);
    out.extend(c.iter().map(offmesh::Connection::direction));
    words(&mut out, [c.len() as u32]);
    out.extend(c.iter().map(offmesh::Connection::area));
    words(&mut out, [c.len() as u32]);
    for c in c {
        out.extend(c.flags().to_le_bytes());
    }
    out
}
fn connections(seed: usize) -> Vec<offmesh::Source> {
    (0..33)
        .map(|i| offmesh::Source {
            map_id: if i % 4 == 0 { 8 } else { 7 },
            row: if i % 7 == 0 { 39 } else { 24 },
            column: 39,
            from: [i as f32, -0.0, seed as f32],
            to: [-3.0, 5.5, 2.25],
            radius: if i % 2 == 0 { -0.0 } else { -2.5 },
            connection_flags: (i * 13) as u8,
            area: (i * 9) as u8,
            flags: (i * 2171) as u16,
        })
        .collect()
}
fn source_bytes(values: &[offmesh::Source]) -> Vec<u8> {
    let mut out = Vec::new();
    for s in values {
        words(&mut out, [s.map_id, s.row, s.column]);
        words(&mut out, s.from.into_iter().chain(s.to).map(f32::to_bits));
        words(&mut out, [s.radius.to_bits()]);
        out.extend([s.connection_flags, s.area]);
        out.extend(s.flags.to_le_bytes());
    }
    out
}
#[test]
#[ignore = "explicit new external directory required in RCE_NAVIGATION_MODEL_CORPUS"]
fn write_navigation_model_corpus() {
    let path = std::env::var_os("RCE_NAVIGATION_MODEL_CORPUS").unwrap();
    let root = Path::new(&path);
    assert!(root.is_absolute() && !root.exists());
    assert!(
        root.parent()
            .unwrap()
            .canonicalize()
            .unwrap()
            .ancestors()
            .all(|p| !p.join(".git").exists())
    );
    fs::create_dir(root).unwrap();
    let mut index = String::new();
    for seed in 0..96usize {
        let dir = root.join(format!("case-{seed:03}"));
        fs::create_dir(&dir).unwrap();
        let compact = seed.is_multiple_of(2);
        let prefix = seed.is_multiple_of(11);
        let count = if seed == 0 { 0 } else { seed % 4 + 1 };
        let type_entries = if seed.is_multiple_of(7) {
            vec![]
        } else {
            vec![(7, (seed % 9) as u8), (9, 30)]
        };
        let off = connections(seed);
        let target = offmesh::Target {
            map_id: 7,
            row: 24,
            column: 39,
        };
        let selected = offmesh::select(target, &off, offmesh::Limits::default()).unwrap();
        let mut plan = format!(
            "{count} {} {} 7 24 39 {} {}\n",
            u8::from(compact),
            u8::from(prefix),
            type_entries.len(),
            off.len()
        );
        for &(id, bank) in &type_entries {
            plan.push_str(&format!("{id} {bank}\n"));
        }
        fs::write(dir.join("plan.txt"), plan).unwrap();
        fs::write(dir.join("connections.bin"), source_bytes(&off)).unwrap();
        let mut geometry = Vec::new();
        let mut placements = Vec::new();
        for model in 0..count {
            let liquid = match (seed + model) % 6 {
                0 => None,
                1 => Some((0, 0, 7)),
                2 => Some((1, 1, 7)),
                3 => Some((2, 3, 7)),
                4 => Some((3, 2, 7)),
                _ => Some((8, 5, 9)),
            };
            let raw = fixture(
                (seed + model).is_multiple_of(2),
                (seed + model) % 4,
                liquid,
                seed == 23,
            );
            let angles = [
                [0.0; 3],
                [90.0, 0.0, 0.0],
                [0.0, 90.0, 0.0],
                [13.0, -29.0, 71.0],
                [-179.0, 360.0, 0.25],
                [0.001, -0.002, 0.003],
            ][(seed + model) % 6];
            let pos = if seed.is_multiple_of(3) {
                origin()
            } else {
                [12.25, 153.5, -33.75]
            };
            let scale = [1.0, -1.0, 0.125, -2.0, 4.5][(seed + model) % 5];
            let placement = spawn(pos, angles, scale, true);
            fs::write(dir.join(format!("model-{model}.vmo")), &raw).unwrap();
            fs::write(dir.join(format!("spawn-{model}.bin")), &placement).unwrap();
            geometry.push(raw);
            placements.push(placement);
        }
        let views: Vec<_> = geometry.iter().map(|b| view(b)).collect();
        let models: Vec<_> = views
            .iter()
            .zip(&placements)
            .map(|(v, p)| Model {
                geometry: v,
                placement: placement(p),
            })
            .collect();
        let mesh = if prefix {
            let raw = terrain_input();
            fs::create_dir(dir.join("maps")).unwrap();
            fs::write(dir.join("maps/0000_24_39.map"), &raw).unwrap();
            let map = maps::verify(&raw, 70205, maps::MAX_MAP_BYTES).unwrap();
            terrain::project(
                &[terrain::Region {
                    map,
                    row: 24,
                    column: 39,
                    portion: terrain::Portion::Top,
                }],
                true,
                crate::Limits::default(),
            )
            .unwrap()
        } else {
            Mesh::default()
        };
        let mesh = append(
            mesh,
            &models,
            LiquidTypes::new(&type_entries).unwrap(),
            Limits::default(),
        )
        .unwrap();
        let mesh = if compact {
            mesh.compact(crate::Limits::default()).unwrap()
        } else {
            mesh
        };
        let bytes = encoded(&mesh, &selected);
        fs::write(dir.join("expected.bin"), &bytes).unwrap();
        index.push_str(&format!(
            "case-{seed:03}\t{} {} {} {} {} {}\n",
            mesh.solid_vertices().len(),
            mesh.solid_triangles().len(),
            mesh.liquid_vertices().len(),
            mesh.liquid_triangles().len(),
            mesh.liquid_areas().len(),
            selected.values().len()
        ));
    }
    // The exact off-mesh ceiling is compared with the original selector too.
    let dir = root.join("offmesh-maximum");
    fs::create_dir(&dir).unwrap();
    let mut example = connections(0)[1];
    example.map_id = 7;
    example.row = 24;
    example.column = 39;
    let off = vec![example; 65536];
    let selected = offmesh::select(
        offmesh::Target {
            map_id: 7,
            row: 24,
            column: 39,
        },
        &off,
        offmesh::Limits::default(),
    )
    .unwrap();
    fs::write(dir.join("plan.txt"), "0 0 0 7 24 39 0 65536\n").unwrap();
    fs::write(dir.join("connections.bin"), source_bytes(&off)).unwrap();
    fs::write(
        dir.join("expected.bin"),
        encoded(&Mesh::default(), &selected),
    )
    .unwrap();
    index.push_str("offmesh-maximum\t0 0 0 0 0 65536\n");
    fs::write(root.join("cases.tsv"), index).unwrap();
}
