// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: explicit external synthetic acceptance corpus.
use super::*;
use std::{fs, path::Path};

fn word(out: &mut Vec<u8>, value: usize) {
    out.extend_from_slice(&(value as u32).to_le_bytes());
}
fn encoded(mesh: &Mesh) -> Vec<u8> {
    let mut out = Vec::new();
    for (vertices, triangles) in [
        (&mesh.solid_vertices, &mesh.solid_triangles),
        (&mesh.liquid_vertices, &mesh.liquid_triangles),
    ] {
        word(&mut out, vertices.len() * 3);
        for v in vertices.iter().flatten() {
            out.extend_from_slice(&v.to_bits().to_le_bytes());
        }
        word(&mut out, triangles.len() * 3);
        for i in triangles.iter().flatten() {
            out.extend_from_slice(&i.to_le_bytes());
        }
    }
    word(&mut out, mesh.liquid_areas.len());
    out.extend(&mesh.liquid_areas);
    out
}
fn number(portion: Portion) -> u8 {
    match portion {
        Portion::Top => 1,
        Portion::Right => 2,
        Portion::Left => 3,
        Portion::Bottom => 4,
        Portion::Entire => 5,
    }
}
fn record(
    root: &Path,
    name: &str,
    regions: &[Region<'_>],
    skip: bool,
    compact: bool,
    neighborhood: bool,
    mesh: Mesh,
) -> String {
    let dir = root.join(name);
    fs::create_dir(&dir).unwrap();
    fs::create_dir(dir.join("maps")).unwrap();
    let mut plan = format!(
        "{} {} {} {}\n",
        regions.len(),
        u8::from(compact),
        u8::from(skip),
        u8::from(neighborhood)
    );
    for r in regions {
        fs::write(
            dir.join(format!("maps/0000_{:02}_{:02}.map", r.row, r.column)),
            r.map.bytes(),
        )
        .unwrap();
        plan.push_str(&format!("{} {} {}\n", r.row, r.column, number(r.portion)));
    }
    let mesh = if compact {
        mesh.compact(Limits::default()).unwrap()
    } else {
        mesh
    };
    let counts = format!(
        "{} {} {} {} {}",
        mesh.solid_vertices.len(),
        mesh.solid_triangles.len(),
        mesh.liquid_vertices.len(),
        mesh.liquid_triangles.len(),
        mesh.liquid_areas.len()
    );
    fs::write(dir.join("plan.txt"), plan).unwrap();
    fs::write(dir.join("expected.bin"), encoded(&mesh)).unwrap();
    format!("{name}\t{counts}\n")
}

#[test]
#[ignore = "explicit external synthetic corpus; set RCE_NAVIGATION_CORPUS to a new private directory"]
fn write_navigation_terrain_corpus() {
    let root = std::env::var_os("RCE_NAVIGATION_CORPUS").expect("explicit corpus path");
    let root = Path::new(&root);
    assert!(root.is_absolute());
    assert!(!root.exists());
    let parent = root.parent().unwrap().canonicalize().unwrap();
    assert!(parent.ancestors().all(|p| !p.join(".git").exists()));
    fs::create_dir(root).unwrap();
    let mut index = String::new();
    let portions = [
        Portion::Entire,
        Portion::Top,
        Portion::Right,
        Portion::Left,
        Portion::Bottom,
    ];
    for build in [70170, 70205] {
        for kind in 0..4 {
            for liquid in 0..12 {
                let seed = kind as usize * 12 + liquid as usize;
                let raw = fixture(build, kind, liquid, seed.is_multiple_of(3));
                let r = Region {
                    map: view(&raw, build),
                    row: [0, 24, 32, 63][seed % 4],
                    column: [63, 39, 32, 0][seed % 4],
                    portion: portions[seed % 5],
                };
                let skip = seed.is_multiple_of(7);
                let compact = seed.is_multiple_of(2);
                let mesh = project(&[r], skip, Limits::default()).unwrap();
                index.push_str(&record(
                    root,
                    &format!("b{build}-h{kind}-l{liquid}"),
                    &[r],
                    skip,
                    compact,
                    false,
                    mesh,
                ));
            }
        }
    }
    for flat in [false, true] {
        for compact in [false, true] {
            for skip in [false, true] {
                let raw = fixture(
                    70205,
                    if flat { 0 } else { 3 },
                    if flat { 0 } else { 10 },
                    true,
                );
                let neighbor = fixture(70205, 2, 4, false);
                let center = region(&raw, Portion::Entire);
                let v = view(&neighbor, 70205);
                let n = Neighbors {
                    column_next: Some(v),
                    column_previous: Some(v),
                    row_next: Some(v),
                    row_previous: Some(v),
                };
                let regions = [
                    center,
                    Region {
                        map: v,
                        row: 32,
                        column: 33,
                        portion: Portion::Left,
                    },
                    Region {
                        map: v,
                        row: 32,
                        column: 31,
                        portion: Portion::Right,
                    },
                    Region {
                        map: v,
                        row: 33,
                        column: 32,
                        portion: Portion::Top,
                    },
                    Region {
                        map: v,
                        row: 31,
                        column: 32,
                        portion: Portion::Bottom,
                    },
                ];
                let mesh = project_neighborhood(center, n, skip, Limits::default()).unwrap();
                index.push_str(&record(
                    root,
                    &format!("neighborhood-{flat}-{compact}-{skip}"),
                    &regions,
                    skip,
                    compact,
                    true,
                    mesh,
                ));
            }
        }
    }
    for compact in [false, true] {
        let raw = fixture(70205, 3, 10, true);
        let regions = [region(&raw, Portion::Entire); 5];
        let mesh = project(&regions, false, Limits::default()).unwrap();
        index.push_str(&record(
            root,
            &format!("five-entire-{compact}"),
            &regions,
            false,
            compact,
            false,
            mesh,
        ));
    }
    fs::write(root.join("cases.tsv"), index).unwrap();
}
