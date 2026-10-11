// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: complete source graph/file selection and strict failures.
use super::*;
use crate::terrain::tests::fixture;
use crate::{
    inventory::{Input, Options},
    terrain::{self, Portion},
    tilebuilder::Coordinate,
};
use forever_db2::terrain_sources::{DirectRecord, NavigationMapFields};
use std::collections::BTreeMap;
fn tile(row: u8, column: u8) -> Coordinate {
    Coordinate { row, column }
}
fn plan(rows: &[(u32, i16, i16)], target: Coordinate) -> inventory::Plan {
    let rows: Vec<_> = rows
        .iter()
        .map(|&(id, parent, cosmetic)| DirectRecord {
            id,
            fields: NavigationMapFields {
                map_type: 0,
                instance_type: 0,
                parent_map_id: parent,
                cosmetic_parent_map_id: cosmetic,
                flags: 0,
            },
        })
        .collect();
    let inputs: Vec<_> = rows
        .iter()
        .map(|r| Input::CollisionTile {
            map_id: r.id,
            tile: target,
        })
        .collect();
    inventory::Plan::from_rows(
        &rows,
        &inputs,
        70205,
        Options::default(),
        inventory::Limits::default(),
    )
    .unwrap()
}
#[derive(Default)]
struct Fake {
    files: BTreeMap<(u32, u8, u8), Vec<u8>>,
    reads: Vec<(u32, u8, u8)>,
    verifies: usize,
    fail_verify: usize,
    error: Option<SourceError>,
    missing_bytes: bool,
}
impl Source for Fake {
    fn read(&mut self, a: Artifact, out: &mut ReadBuffer<'_>) -> Result<Presence, SourceError> {
        let key = (a.map_id, a.tile.row, a.tile.column);
        self.reads.push(key);
        if let Some(e) = self.error {
            return Err(e);
        }
        if let Some(bytes) = self.files.get(&key) {
            for b in bytes.chunks(16384) {
                out.append(b)?;
            }
            Ok(Presence::Present)
        } else {
            if self.missing_bytes {
                out.append(b"x")?;
            }
            Ok(Presence::Missing)
        }
    }
    fn verify_unchanged(&mut self) -> Result<(), SourceError> {
        self.verifies += 1;
        if self.verifies == self.fail_verify {
            Err(SourceError::InputChanged)
        } else {
            Ok(())
        }
    }
}
fn insert(s: &mut Fake, id: u32, t: Coordinate, kind: u8) {
    s.files
        .insert((id, t.row, t.column), fixture(70205, kind, 0, false));
}
fn run(s: &mut Fake, p: &inventory::Plan, id: u32, t: Coordinate) -> Result<Loaded, Error> {
    neighborhood(s, p, id, t, false, Limits::default())
}
fn same(a: &Mesh, b: &Mesh) {
    assert_eq!(
        a.solid_vertices()
            .iter()
            .map(|p| p.map(f32::to_bits))
            .collect::<Vec<_>>(),
        b.solid_vertices()
            .iter()
            .map(|p| p.map(f32::to_bits))
            .collect::<Vec<_>>()
    );
    assert_eq!(a.solid_triangles(), b.solid_triangles());
    assert_eq!(
        a.liquid_vertices()
            .iter()
            .map(|p| p.map(f32::to_bits))
            .collect::<Vec<_>>(),
        b.liquid_vertices()
            .iter()
            .map(|p| p.map(f32::to_bits))
            .collect::<Vec<_>>()
    );
    assert_eq!(a.liquid_triangles(), b.liquid_triangles());
    assert_eq!(a.liquid_areas(), b.liquid_areas());
}
#[test]
fn own_center_precedes_ancestor_and_retains_target_coordinate() {
    let t = tile(24, 39);
    let p = plan(&[(1, -1, -1), (2, 1, -1)], t);
    let mut s = Fake::default();
    insert(&mut s, 1, t, 1);
    insert(&mut s, 2, t, 3);
    let loaded = run(&mut s, &p, 2, t).unwrap();
    assert_eq!(loaded.origins(), &[Some(2), None, None, None, None]);
    let raw = &s.files[&(2, 24, 39)];
    let expected = terrain::project(
        &[terrain::Region {
            map: maps::verify(raw, 70205, maps::MAX_MAP_BYTES).unwrap(),
            row: 24,
            column: 39,
            portion: Portion::Entire,
        }],
        false,
        crate::Limits::default(),
    )
    .unwrap();
    same(loaded.mesh(), &expected);
    assert_eq!(s.reads[0], (2, 24, 39));
    assert!(!s.reads.contains(&(1, 24, 39)));
}
#[test]
fn every_neighbor_resolves_independent_effective_parent_in_original_order() {
    let t = tile(24, 39);
    let p = plan(&[(1, -1, -1), (2, 1, -1), (3, -1, 2)], t);
    let mut s = Fake::default();
    for (id, t, k) in [
        (2, t, 3),
        (1, tile(24, 40), 1),
        (3, tile(24, 38), 2),
        (2, tile(25, 39), 3),
        (1, tile(23, 39), 2),
    ] {
        insert(&mut s, id, t, k);
    }
    let got = run(&mut s, &p, 3, t).unwrap();
    assert_eq!(
        got.origins(),
        &[Some(2), Some(1), Some(3), Some(2), Some(1)]
    );
    assert_eq!(
        s.reads,
        [
            (3, 24, 39),
            (2, 24, 39),
            (3, 24, 40),
            (2, 24, 40),
            (1, 24, 40),
            (3, 24, 38),
            (3, 25, 39),
            (2, 25, 39),
            (3, 23, 39),
            (2, 23, 39),
            (1, 23, 39)
        ]
    );
}
#[test]
fn missing_center_suppresses_all_neighbor_reads() {
    let t = tile(24, 39);
    let p = plan(&[(1, -1, -1), (2, 1, -1)], t);
    let mut s = Fake::default();
    insert(&mut s, 1, tile(24, 40), 3);
    let got = run(&mut s, &p, 2, t).unwrap();
    assert_eq!(s.reads, [(2, 24, 39), (1, 24, 39)]);
    assert!(!got.mesh().has_triangles());
    assert_eq!(got.origins(), &[None; 5]);
}
#[test]
fn present_flat_center_blocks_parent_fallback_and_neighbor_corruption() {
    let t = tile(24, 39);
    let p = plan(&[(1, -1, -1), (2, 1, -1)], t);
    let mut s = Fake::default();
    insert(&mut s, 2, t, 0);
    insert(&mut s, 1, t, 3);
    s.files.insert((2, 24, 40), b"corrupt".to_vec());
    let got = run(&mut s, &p, 2, t).unwrap();
    assert_eq!(s.reads, [(2, 24, 39)]);
    assert!(!got.mesh().has_triangles());
    assert_eq!(got.origins()[0], Some(2));
}
#[test]
fn dark_center_retains_unreferenced_vertices_and_suppresses_neighbors() {
    let t = tile(24, 39);
    let p = plan(&[(1, -1, -1)], t);
    let mut s = Fake::default();
    s.files.insert((1, 24, 39), fixture(70205, 3, 5, false));
    let got = run(&mut s, &p, 1, t).unwrap();
    assert!(!got.mesh().has_triangles());
    assert!(!got.mesh().solid_vertices().is_empty());
    assert_eq!(s.reads.len(), 1);
}
#[test]
fn present_corrupt_center_rejects_without_ancestor_retry() {
    let t = tile(24, 39);
    let p = plan(&[(1, -1, -1), (2, 1, -1)], t);
    let mut s = Fake::default();
    s.files.insert((2, 24, 39), b"bad".to_vec());
    insert(&mut s, 1, t, 3);
    assert!(matches!(run(&mut s, &p, 2, t), Err(Error::Map(_))));
    assert_eq!(s.reads, [(2, 24, 39)]);
}
#[test]
fn present_corrupt_neighbor_rejects_before_later_neighbors() {
    let t = tile(24, 39);
    let p = plan(&[(1, -1, -1)], t);
    let mut s = Fake::default();
    insert(&mut s, 1, t, 3);
    s.files.insert((1, 24, 40), b"bad".to_vec());
    assert!(matches!(run(&mut s, &p, 1, t), Err(Error::Map(_))));
    assert_eq!(s.reads, [(1, 24, 39), (1, 24, 40)]);
}
#[test]
fn source_error_and_missing_with_bytes_never_allow_fallback() {
    let t = tile(24, 39);
    let p = plan(&[(1, -1, -1), (2, 1, -1)], t);
    for e in [
        SourceError::Io,
        SourceError::InputChanged,
        SourceError::Limit,
    ] {
        let mut s = Fake {
            error: Some(e),
            ..Default::default()
        };
        assert!(matches!(run(&mut s,&p,2,t),Err(Error::Source(x))if x==e));
        assert_eq!(s.reads.len(), 1);
    }
    let mut s = Fake {
        missing_bytes: true,
        ..Default::default()
    };
    assert!(matches!(
        run(&mut s, &p, 2, t),
        Err(Error::Source(SourceError::Io))
    ));
    assert_eq!(s.reads.len(), 1);
}
#[test]
fn build_mismatch_rejects_present_file_without_fallback() {
    let t = tile(24, 39);
    let p = plan(&[(1, -1, -1)], t);
    let mut s = Fake::default();
    s.files.insert((1, 24, 39), fixture(70170, 3, 0, false));
    assert!(matches!(
        run(&mut s, &p, 1, t),
        Err(Error::Map(maps::Error::BuildMismatch))
    ));
}
#[test]
fn invalid_selection_or_increased_limits_reject_before_io() {
    let t = tile(24, 39);
    let p = plan(&[(1, -1, -1)], t);
    let mut s = Fake::default();
    for (id, t) in [(99, t), (1, tile(64, 0)), (1, tile(24, 40))] {
        assert!(matches!(run(&mut s, &p, id, t), Err(Error::Selection)));
    }
    assert!(matches!(
        neighborhood(
            &mut s,
            &p,
            1,
            t,
            false,
            Limits {
                reads: 5121,
                ..Default::default()
            }
        ),
        Err(Error::Limit)
    ));
    assert!(s.reads.is_empty());
    assert_eq!(s.verifies, 0);
}
#[test]
fn boundaries_do_not_request_out_of_domain_neighbors() {
    for t in [tile(0, 0), tile(63, 63)] {
        let p = plan(&[(1, -1, -1)], t);
        let mut s = Fake::default();
        insert(&mut s, 1, t, 3);
        let got = run(&mut s, &p, 1, t).unwrap();
        assert_eq!(got.statistics().reads, 3);
        assert!(s.reads.iter().all(|&(_, x, y)| x < 64 && y < 64));
    }
}
#[test]
fn verification_failure_discards_center_or_complete_neighborhood() {
    let t = tile(24, 39);
    let p = plan(&[(1, -1, -1)], t);
    for fail in 1..=3 {
        let mut s = Fake {
            fail_verify: fail,
            ..Default::default()
        };
        insert(&mut s, 1, t, 3);
        assert!(matches!(
            run(&mut s, &p, 1, t),
            Err(Error::Source(SourceError::InputChanged))
        ));
        assert_eq!(s.verifies, fail);
    }
}
#[test]
fn exact_read_and_capacity_budgets_and_downward_reductions() {
    let t = tile(24, 39);
    let p = plan(&[(1, -1, -1)], t);
    let mut s = Fake::default();
    insert(&mut s, 1, t, 3);
    let got = run(&mut s, &p, 1, t).unwrap();
    let stats = got.statistics();
    for l in [
        Limits {
            reads: stats.reads,
            read_bytes: stats.read_bytes,
            source_heap_bytes: stats.peak_source_capacity,
            ..Default::default()
        },
        Limits {
            reads: stats.reads - 1,
            ..Default::default()
        },
        Limits {
            read_bytes: stats.read_bytes - 1,
            ..Default::default()
        },
        Limits {
            source_heap_bytes: stats.peak_source_capacity - 1,
            ..Default::default()
        },
        Limits {
            file_bytes: stats.read_bytes - 1,
            ..Default::default()
        },
    ] {
        let mut s = Fake::default();
        insert(&mut s, 1, t, 3);
        let result = neighborhood(&mut s, &p, 1, t, false, l);
        assert_eq!(
            result.is_ok(),
            l.reads == stats.reads
                && l.read_bytes == stats.read_bytes
                && l.source_heap_bytes == stats.peak_source_capacity
        );
    }
}
#[test]
fn complete_1024_parent_chain_uses_exact_5120_read_ceiling() {
    let t = tile(24, 39);
    let rows: Vec<_> = (0..1024)
        .map(|id| (id, if id == 0 { -1 } else { (id - 1) as i16 }, -1))
        .collect();
    let p = plan(&rows, t);
    let mut s = Fake::default();
    insert(&mut s, 0, t, 3);
    let got = run(&mut s, &p, 1023, t).unwrap();
    assert_eq!(got.statistics().reads, 5120);
    assert_eq!(got.origins()[0], Some(0));
}

#[cfg(target_os = "linux")]
#[test]
#[ignore = "new private external output required in RCE_NAVIGATION_TERRAIN_LOAD_CORPUS"]
fn write_terrain_load_corpus() {
    use std::{fs, path::Path};
    let path = std::env::var_os("RCE_NAVIGATION_TERRAIN_LOAD_CORPUS").unwrap();
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
    let mut record = |name: String,
                      build: u32,
                      rows: Vec<(u32, i16, i16)>,
                      target: u32,
                      t: Coordinate,
                      skip: bool,
                      files: BTreeMap<(u32, u8, u8), Vec<u8>>| {
        let d = root.join(&name);
        fs::create_dir(&d).unwrap();
        fs::create_dir(d.join("maps")).unwrap();
        for (&(id, x, y), b) in &files {
            fs::write(d.join("maps").join(format!("{id:04}_{x:02}_{y:02}.map")), b).unwrap();
        }
        let mut text = format!(
            "{} {target} {} {} {} {build}\n",
            rows.len(),
            t.row,
            t.column,
            u8::from(skip)
        );
        for &(id, p, c) in &rows {
            text.push_str(&format!(
                "{id} {} {}\n",
                if p < 0 { u32::MAX } else { p as u32 },
                if c < 0 { u32::MAX } else { c as u32 }
            ));
        }
        fs::write(d.join("plan.txt"), text).unwrap();
        let sources: Vec<_> = rows
            .iter()
            .map(|&(id, p, c)| DirectRecord {
                id,
                fields: NavigationMapFields {
                    map_type: 0,
                    instance_type: 0,
                    parent_map_id: p,
                    cosmetic_parent_map_id: c,
                    flags: 0,
                },
            })
            .collect();
        let descriptors: Vec<_> = sources
            .iter()
            .map(|r| Input::CollisionTile {
                map_id: r.id,
                tile: t,
            })
            .collect();
        let plan = inventory::Plan::from_rows(
            &sources,
            &descriptors,
            build,
            Options::default(),
            inventory::Limits::default(),
        )
        .unwrap();
        let mut source =
            super::fs::RootedSource::new(&d.join("maps"), super::fs::FsLimits::default()).unwrap();
        let result = neighborhood(&mut source, &plan, target, t, skip, Limits::default()).unwrap();
        let mesh = result.mesh();
        let mut out = Vec::new();
        for (v, tris) in [
            (mesh.solid_vertices(), mesh.solid_triangles()),
            (mesh.liquid_vertices(), mesh.liquid_triangles()),
        ] {
            out.extend_from_slice(&((v.len() * 3) as u32).to_le_bytes());
            for p in v {
                for f in p {
                    out.extend_from_slice(&f.to_bits().to_le_bytes());
                }
            }
            out.extend_from_slice(&((tris.len() * 3) as u32).to_le_bytes());
            for tri in tris {
                for v in tri {
                    out.extend_from_slice(&v.to_le_bytes());
                }
            }
        }
        out.extend_from_slice(&(mesh.liquid_areas().len() as u32).to_le_bytes());
        out.extend_from_slice(mesh.liquid_areas());
        fs::write(d.join("expected.bin"), out).unwrap();
        let s = result.statistics();
        let origins = result
            .origins()
            .iter()
            .map(|id| id.map_or("-".into(), |id| id.to_string()))
            .collect::<Vec<_>>()
            .join(" ");
        fs::write(
            d.join("stats.txt"),
            format!(
                "{} {} {} {}\n{origins}\n",
                s.reads, s.read_bytes, s.peak_source_capacity, s.found_files
            ),
        )
        .unwrap();
        index.push_str(&format!(
            "{name}\t{} {} {} {} {}\n",
            mesh.solid_vertices().len(),
            mesh.solid_triangles().len(),
            mesh.liquid_vertices().len(),
            mesh.liquid_triangles().len(),
            mesh.liquid_areas().len()
        ));
    };
    for build in [70170, 70205] {
        for seed in 0..24 {
            for skip in [false, true] {
                let t = tile(24, 39);
                let mut files = BTreeMap::new();
                let rows = vec![
                    (1, -1, -1),
                    (2, 1, -1),
                    (3, if seed % 2 == 0 { 2 } else { -1 }, 2),
                ];
                if seed != 0 {
                    let id = 1 + seed % 3;
                    let (kind, liq) = match seed {
                        1 => (0, 0),
                        2 => (3, 5),
                        3 => (0, 1),
                        _ => (1 + (seed % 3) as u8, [0, 1, 4, 5, 10][seed as usize % 5]),
                    };
                    files.insert((id, 24, 39), fixture(build, kind, liq, seed % 4 == 0));
                    if id > 1 {
                        files.insert((1, 24, 39), fixture(build, 3, 0, false));
                    }
                }
                for (i, nt) in [tile(24, 40), tile(24, 38), tile(25, 39), tile(23, 39)]
                    .into_iter()
                    .enumerate()
                {
                    if !(seed as usize + i).is_multiple_of(5) {
                        let id = 1 + (seed as usize + i) % 3;
                        files.insert(
                            (id as u32, nt.row, nt.column),
                            fixture(
                                build,
                                1 + ((seed as usize + i) % 3) as u8,
                                if i % 2 == 0 { 10 } else { 0 },
                                i % 2 == 0,
                            ),
                        );
                        if id > 1 {
                            files.insert((1, nt.row, nt.column), fixture(build, 3, 0, false));
                        }
                    }
                }
                record(
                    format!("mixed-{build}-{seed:02}-{skip}"),
                    build,
                    rows,
                    3,
                    t,
                    skip,
                    files,
                );
            }
        }
        for t in [tile(0, 0), tile(0, 63), tile(63, 0), tile(63, 63)] {
            let mut files = BTreeMap::new();
            files.insert((0, t.row, t.column), fixture(build, 3, 0, false));
            record(
                format!("boundary-{build}-{}-{}", t.row, t.column),
                build,
                vec![(0, -1, -1)],
                0,
                t,
                false,
                files,
            );
        }
        for depth in [1, 4, 64] {
            let rows = (0..depth)
                .map(|id| (id, if id == 0 { -1 } else { (id - 1) as i16 }, -1))
                .collect();
            let mut files = BTreeMap::new();
            files.insert((0, 24, 39), fixture(build, 3, 10, true));
            files.insert((0, 24, 40), fixture(build, 2, 0, false));
            record(
                format!("deep-{build}-{depth}"),
                build,
                rows,
                depth - 1,
                tile(24, 39),
                false,
                files,
            );
        }
    }
    fs::write(root.join("cases.tsv"), index).unwrap();
}
