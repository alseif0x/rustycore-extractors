// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: checked store to navigation geometry/lifecycle fixtures.
use super::*;
use forever_vmap::{
    assembly,
    store::{self, Artifact, Presence, ReadBuffer, Source, SourceError},
};
use std::collections::BTreeMap;
#[derive(Default)]
struct Files {
    bytes: BTreeMap<String, Vec<u8>>,
}
fn key(a: Artifact<'_>) -> String {
    match a {
        Artifact::Tree { map_id } => format!("{map_id}/tree"),
        Artifact::Tile { map_id, x, y } => format!("{map_id}/{x}/{y}/tile"),
        Artifact::Index { map_id, x, y } => format!("{map_id}/{x}/{y}/index"),
        Artifact::Model { name } => format!("model/{}", std::str::from_utf8(name).unwrap()),
    }
}
impl Source for Files {
    fn read(&mut self, a: Artifact<'_>, o: &mut ReadBuffer<'_>) -> Result<Presence, SourceError> {
        match self.bytes.get(&key(a)) {
            Some(b) => {
                o.append(b)?;
                Ok(Presence::Present)
            }
            None => Ok(Presence::Missing),
        }
    }
    fn verify_unchanged(&mut self) -> Result<(), SourceError> {
        Ok(())
    }
}
fn package(path_only: bool) -> (store::StaticStore<Files>, Vec<u8>, Vec<u8>) {
    let raw = fixture(false, 2, Some((1, 1, 7)), false);
    let mut first = spawn([1., 1., 0.], [17., 31., -9.], 2., true);
    first[0] |= 4;
    for (i, v) in [0f32, 0., -100., 10., 10., 100.].into_iter().enumerate() {
        first[34 + i * 4..38 + i * 4].copy_from_slice(&v.to_bits().to_le_bytes());
    }
    let mut second = spawn([GRID_SIZE + 1., 1., 0.], [0., 0., 0.], 1., true);
    second[2..6].copy_from_slice(&72u32.to_le_bytes());
    for (i, v) in [GRID_SIZE + 1., 0., -100., GRID_SIZE + 10., 10., 100.]
        .into_iter()
        .enumerate()
    {
        second[34 + i * 4..38 + i * 4].copy_from_slice(&v.to_bits().to_le_bytes());
    }
    let rows = [placement(&first), placement(&second)];
    let built = assembly::build(1, &rows, assembly::Limits::default()).unwrap();
    let mut files = Files::default();
    files
        .bytes
        .insert("1/tree".into(), built.tree_bytes().to_vec());
    for t in built.tiles() {
        files.bytes.insert(
            format!("1/{}/{}/index", t.x(), t.y()),
            t.index_bytes().to_vec(),
        );
        if let Some(b) = t.payload() {
            files
                .bytes
                .insert(format!("1/{}/{}/tile", t.x(), t.y()), b.to_vec());
        }
    }
    files.bytes.insert("model/toy01".into(), raw.clone());
    let store = store::StaticStore::new(
        files,
        &[store::MapParent {
            map_id: 1,
            parent: None,
            cosmetic_parent: None,
        }],
        store::Options {
            load_path_only_models: path_only,
            ..Default::default()
        },
        store::Limits::default(),
    )
    .unwrap();
    (store, raw, [first, second].concat())
}
fn types() -> LiquidTypes<'static> {
    LiquidTypes::new(&[(7, 2)]).unwrap()
}
fn same(a: &Mesh, b: &Mesh) {
    assert_eq!(
        a.solid_vertices()
            .iter()
            .map(|v| v.map(f32::to_bits))
            .collect::<Vec<_>>(),
        b.solid_vertices()
            .iter()
            .map(|v| v.map(f32::to_bits))
            .collect::<Vec<_>>()
    );
    assert_eq!(a.solid_triangles(), b.solid_triangles());
    assert_eq!(
        a.liquid_vertices()
            .iter()
            .map(|v| v.map(f32::to_bits))
            .collect::<Vec<_>>(),
        b.liquid_vertices()
            .iter()
            .map(|v| v.map(f32::to_bits))
            .collect::<Vec<_>>()
    );
    assert_eq!(a.liquid_triangles(), b.liquid_triangles());
    assert_eq!(a.liquid_areas(), b.liquid_areas());
}
#[test]
fn slot_order_and_cached_owned_views_match_supplied_models() {
    let (mut s, raw, rows) = package(true);
    s.load_tile(1, 1, 0).unwrap();
    s.load_tile(1, 0, 0).unwrap();
    let before = s.stats();
    let v = view(&raw);
    let direct = placement::DirBin::parse(&rows, placement::Limits::default()).unwrap();
    let models: Vec<_> = direct
        .records()
        .map(|p| Model {
            geometry: &v,
            placement: p,
        })
        .collect();
    let expected = append(Mesh::default(), &models, types(), Limits::default()).unwrap();
    let actual = append_loaded(Mesh::default(), &s, 1, types(), Limits::default()).unwrap();
    same(&actual, &expected);
    assert_eq!(
        s.loaded_models(1)
            .unwrap()
            .map(|m| m.placement().id())
            .collect::<Vec<_>>(),
        [71, 72]
    );
    assert_eq!(s.stats(), before);
}
#[test]
fn tile_lifecycle_includes_other_loaded_tiles_and_removes_all_target_references() {
    let (mut s, _, _) = package(true);
    s.load_tile(1, 1, 0).unwrap();
    s.load_tile(1, 0, 0).unwrap();
    s.load_tile(1, 0, 0).unwrap();
    let expected = append_loaded(Mesh::default(), &s, 1, types(), Limits::default()).unwrap();
    let got = append_tile(
        Mesh::default(),
        &mut s,
        1,
        crate::tilebuilder::Coordinate { row: 0, column: 0 },
        types(),
        Limits::default(),
    )
    .unwrap();
    same(&got, &expected);
    assert_eq!(
        s.loaded_models(1)
            .unwrap()
            .map(|m| m.placement().id())
            .collect::<Vec<_>>(),
        [72]
    );
    assert_eq!(s.stats().tiles, 1);
    assert_eq!(s.stats().model_references, 1);
}
#[test]
fn projection_failure_unloads_successful_tile_without_removing_other_tile() {
    let (mut s, _, _) = package(true);
    s.load_tile(1, 1, 0).unwrap();
    let e = append_tile(
        Mesh::default(),
        &mut s,
        1,
        crate::tilebuilder::Coordinate { row: 0, column: 0 },
        types(),
        Limits {
            models: 0,
            ..Default::default()
        },
    );
    assert!(matches!(e, Err(LoadedError::Geometry(Error::Limit))));
    assert_eq!(
        s.loaded_models(1)
            .unwrap()
            .map(|m| m.placement().id())
            .collect::<Vec<_>>(),
        [72]
    );
}
#[test]
fn wrong_path_only_policy_and_coordinates_reject_before_loading() {
    let (mut s, _, _) = package(false);
    let before = s.stats();
    assert!(matches!(
        append_loaded(Mesh::default(), &s, 1, types(), Limits::default()),
        Err(LoadedError::Options)
    ));
    assert!(matches!(
        append_tile(
            Mesh::default(),
            &mut s,
            1,
            crate::tilebuilder::Coordinate { row: 0, column: 0 },
            types(),
            Limits::default()
        ),
        Err(LoadedError::Options)
    ));
    assert_eq!(s.stats(), before);
    let (mut s, _, _) = package(true);
    let before = s.stats();
    assert!(matches!(
        append_tile(
            Mesh::default(),
            &mut s,
            1,
            crate::tilebuilder::Coordinate { row: 64, column: 0 },
            types(),
            Limits::default()
        ),
        Err(LoadedError::Geometry(Error::Coordinates))
    ));
    assert_eq!(s.stats(), before);
}
#[test]
fn missing_tile_is_explicit_and_preserves_existing_loaded_geometry() {
    let (mut s, _, _) = package(true);
    s.load_tile(1, 1, 0).unwrap();
    assert!(matches!(
        append_tile(
            Mesh::default(),
            &mut s,
            1,
            crate::tilebuilder::Coordinate {
                row: 63,
                column: 63
            },
            types(),
            Limits::default()
        ),
        Err(LoadedError::Store(store::Error::MissingIndex))
    ));
    assert_eq!(s.loaded_models(1).unwrap().count(), 1);
}
#[test]
fn repeated_loaded_projection_is_bounded_and_empty_snapshot_keeps_prefix() {
    let (mut s, _, _) = package(true);
    assert!(
        !append_loaded(
            Mesh::default(),
            &s,
            1,
            types(),
            Limits {
                models: 0,
                ..Default::default()
            }
        )
        .unwrap()
        .has_triangles()
    );
    s.load_tile(1, 0, 0).unwrap();
    let prefix = append_loaded(Mesh::default(), &s, 1, types(), Limits::default()).unwrap();
    s.unload_tile(1, 0, 0).unwrap();
    let expected = prefix.solid_triangles().to_vec();
    let kept = append_loaded(prefix, &s, 1, types(), Limits::default()).unwrap();
    assert_eq!(kept.solid_triangles(), expected);
}

#[cfg(target_os = "linux")]
#[test]
#[ignore = "explicit complete synthetic input/output roots required"]
fn write_loaded_corpus() {
    use std::{fs, path::Path};
    let input = std::env::var_os("RCE_NAVIGATION_LOADED_INPUT").unwrap();
    let output = std::env::var_os("RCE_NAVIGATION_LOADED_OUTPUT").unwrap();
    let input = Path::new(&input);
    let output = Path::new(&output);
    assert!(input.is_absolute() && output.is_absolute() && !output.exists());
    assert!(
        output
            .parent()
            .unwrap()
            .canonicalize()
            .unwrap()
            .ancestors()
            .all(|p| !p.join(".git").exists())
    );
    fs::create_dir(output).unwrap();
    let mut dirs: Vec<_> = fs::read_dir(input)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    dirs.sort();
    for dir in dirs {
        assert!(dir.is_dir());
        let plan = fs::read_to_string(dir.join("plan.txt")).unwrap();
        let numbers: Vec<u32> = plan
            .split_whitespace()
            .map(|v| v.parse().unwrap())
            .collect();
        let mut n = numbers.into_iter();
        let (np, no, nt) = (n.next().unwrap(), n.next().unwrap(), n.next().unwrap());
        assert!(np <= 64 && no <= 4096 && nt <= 65536);
        let mut parents = Vec::new();
        for _ in 0..np {
            let (map, p, c) = (n.next().unwrap(), n.next().unwrap(), n.next().unwrap());
            parents.push(store::MapParent {
                map_id: map,
                parent: (p != u32::MAX).then_some(p),
                cosmetic_parent: (c != u32::MAX).then_some(c),
            });
        }
        let mut entries = Vec::new();
        for _ in 0..nt {
            entries.push((n.next().unwrap(), u8::try_from(n.next().unwrap()).unwrap()));
        }
        let types = LiquidTypes::new(&entries).unwrap();
        let source =
            store::fs::RootedSource::new(&dir.join("vmaps"), store::fs::FsLimits::default())
                .unwrap();
        let mut store = store::StaticStore::new(
            source,
            &parents,
            store::Options {
                load_path_only_models: true,
                ..Default::default()
            },
            store::Limits::default(),
        )
        .unwrap();
        let out = output.join(dir.file_name().unwrap());
        fs::create_dir(&out).unwrap();
        for i in 0..no {
            let (kind, map, x, y) = (
                n.next().unwrap(),
                n.next().unwrap(),
                n.next().unwrap(),
                n.next().unwrap(),
            );
            assert!(x < 64 && y < 64);
            let mesh = match kind {
                0 => {
                    store.load_tile(map, x as u16, y as u8).unwrap();
                    append_loaded(Mesh::default(), &store, map, types, Limits::default()).unwrap()
                }
                1 => {
                    store.unload_tile(map, x as u16, y as u8).unwrap();
                    Mesh::default()
                }
                2 => {
                    store.unload_map(map).unwrap();
                    Mesh::default()
                }
                6 => append_tile(
                    Mesh::default(),
                    &mut store,
                    map,
                    crate::tilebuilder::Coordinate {
                        row: x as u8,
                        column: y as u8,
                    },
                    types,
                    Limits::default(),
                )
                .unwrap(),
                _ => panic!("invalid synthetic operation"),
            };
            let stats = store.map_stats(map).unwrap_or_default();
            let mut bytes = Vec::new();
            words(
                &mut bytes,
                [
                    kind,
                    map,
                    stats.primitive_slots as u32,
                    stats.active_instances as u32,
                ],
            );
            for (v, t) in [
                (mesh.solid_vertices(), mesh.solid_triangles()),
                (mesh.liquid_vertices(), mesh.liquid_triangles()),
            ] {
                words(&mut bytes, [(v.len() * 3) as u32]);
                words(&mut bytes, v.iter().flatten().map(|f| f.to_bits()));
                words(&mut bytes, [(t.len() * 3) as u32]);
                words(&mut bytes, t.iter().flatten().copied());
            }
            words(&mut bytes, [mesh.liquid_areas().len() as u32]);
            bytes.extend(mesh.liquid_areas());
            fs::write(out.join(format!("{i}.bin")), bytes).unwrap();
        }
        assert!(n.next().is_none());
        store.verify_source().unwrap();
    }
}
