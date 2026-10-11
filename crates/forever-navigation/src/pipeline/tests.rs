// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: source-to-final selection, lifecycle and bounded failures.
use super::*;
use crate::{
    inventory::{Input, Options},
    tilebuilder::Coordinate,
};
use forever_db2::terrain_sources::{DirectRecord, NavigationMapFields};
#[derive(Default)]
struct Terrain {
    bytes: Option<Vec<u8>>,
    reads: usize,
    verifies: usize,
    fail: usize,
}
impl terrain::load::Source for Terrain {
    fn read(
        &mut self,
        _: terrain::load::Artifact,
        out: &mut terrain::load::ReadBuffer<'_>,
    ) -> Result<terrain::load::Presence, terrain::load::SourceError> {
        self.reads += 1;
        if let Some(b) = &self.bytes {
            out.append(b)?;
            Ok(terrain::load::Presence::Present)
        } else {
            Ok(terrain::load::Presence::Missing)
        }
    }
    fn verify_unchanged(&mut self) -> Result<(), terrain::load::SourceError> {
        self.verifies += 1;
        if self.verifies == self.fail {
            Err(terrain::load::SourceError::InputChanged)
        } else {
            Ok(())
        }
    }
}
struct Absent;
impl store::Source for Absent {
    fn read(
        &mut self,
        _: store::Artifact<'_>,
        _: &mut store::ReadBuffer<'_>,
    ) -> Result<store::Presence, store::SourceError> {
        Ok(store::Presence::Missing)
    }
    fn verify_unchanged(&mut self) -> Result<(), store::SourceError> {
        Ok(())
    }
}
fn request() -> Request {
    Request {
        tile: tilebuilder::Request {
            map_id: 1,
            tile: Coordinate { row: 0, column: 0 },
            big_base_unit: true,
            slopes: Default::default(),
        },
        skip_liquids: false,
    }
}
fn plan(terrain: bool, collision: bool) -> inventory::Plan {
    let rows = [DirectRecord {
        id: 1,
        fields: NavigationMapFields {
            map_type: 0,
            instance_type: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags: 0,
        },
    }];
    let mut bytes = vec![b'0'; 4108];
    bytes[..4].copy_from_slice(b"MAPS");
    bytes[4..8].copy_from_slice(&10u32.to_le_bytes());
    bytes[8..12].copy_from_slice(&70205u32.to_le_bytes());
    bytes[4107] = b'1';
    let mut inputs = Vec::new();
    if terrain {
        inputs.push(Input::Terrain {
            map_id: 1,
            bytes: &bytes,
        });
    }
    if collision {
        inputs.push(Input::CollisionTile {
            map_id: 1,
            tile: request().tile.tile,
        });
    }
    inventory::Plan::from_rows(
        &rows,
        &inputs,
        70205,
        Options::default(),
        Default::default(),
    )
    .unwrap()
}
fn collision(p: &inventory::Plan) -> store::StaticStore<Absent> {
    store::StaticStore::new(
        Absent,
        &p.parent_rows().collect::<Vec<_>>(),
        store::Options {
            load_path_only_models: true,
            ..Default::default()
        },
        Default::default(),
    )
    .unwrap()
}
fn execute(
    t: &mut Terrain,
    p: &inventory::Plan,
    r: Request,
    l: Limits,
    off: &[offmesh::Source],
) -> Result<Outcome, Error> {
    run(
        p,
        &mut Sources {
            terrain: t,
            collision: &mut collision(p),
        },
        models::LiquidTypes::new(&[]).unwrap(),
        off,
        r,
        l,
    )
}
#[test]
fn missing_promised_terrain_rejects_before_collision() {
    assert!(matches!(
        execute(
            &mut Terrain::default(),
            &plan(true, false),
            request(),
            Limits::default(),
            &[]
        ),
        Err(Error::MissingTerrain)
    ));
}
#[test]
fn missing_promised_collision_is_strict() {
    assert!(matches!(
        execute(
            &mut Terrain::default(),
            &plan(false, true),
            request(),
            Limits::default(),
            &[]
        ),
        Err(Error::Store(store::Error::MissingTree))
    ));
}
#[test]
fn flat_terrain_optional_collision_and_bad_connections_are_empty() {
    let mut t = Terrain {
        bytes: Some(crate::terrain::tests::fixture(70205, 0, 0, false)),
        ..Default::default()
    };
    let off = [offmesh::Source {
        map_id: 1,
        row: 0,
        column: 0,
        from: [f32::NAN; 3],
        to: [0.; 3],
        radius: f32::NAN,
        connection_flags: 255,
        area: 255,
        flags: 65535,
    }];
    let p = plan(true, false);
    let mut c = collision(&p);
    let o = run(
        &p,
        &mut Sources {
            terrain: &mut t,
            collision: &mut c,
        },
        models::LiquidTypes::new(&[]).unwrap(),
        &off,
        request(),
        Limits::default(),
    )
    .unwrap();
    assert!(matches!(
        o,
        Outcome::Empty {
            reason: EmptyReason::NoVertices,
            ..
        }
    ));
    assert_eq!(c.stats().maps, 0);
    assert_eq!(c.stats().tiles, 0);
    assert_eq!(t.reads, 1);
}
#[test]
fn selected_bad_connections_reject_after_geometry() {
    let mut t = Terrain {
        bytes: Some(crate::terrain::tests::fixture(70205, 1, 0, false)),
        ..Default::default()
    };
    let off = [offmesh::Source {
        map_id: 1,
        row: 0,
        column: 0,
        from: [f32::NAN; 3],
        to: [0.; 3],
        radius: 1.,
        connection_flags: 1,
        area: 1,
        flags: 1,
    }];
    assert!(matches!(
        execute(
            &mut t,
            &plan(true, false),
            request(),
            Limits::default(),
            &off
        ),
        Err(Error::Geometry(crate::Error::NonFinite))
    ));
}
#[test]
fn changed_source_never_returns_empty_success() {
    let mut t = Terrain {
        bytes: Some(crate::terrain::tests::fixture(70205, 0, 0, false)),
        fail: 4,
        ..Default::default()
    };
    assert!(matches!(
        execute(
            &mut t,
            &plan(true, false),
            request(),
            Limits::default(),
            &[]
        ),
        Err(Error::Terrain(terrain::load::Error::Source(
            terrain::load::SourceError::InputChanged
        )))
    ));
}
#[test]
fn upward_limits_reject_before_io_even_when_empty() {
    let mut a = Limits::default();
    a.heap_bytes += 1;
    let mut b = Limits::default();
    b.builder.work += 1;
    let mut c = Limits::default();
    c.connections.output_records += 1;
    let mut d = Limits::default();
    d.save.work += 1;
    let mut e = Limits::default();
    e.terrain.mesh.vertices += 1;
    let mut f = Limits::default();
    f.output.file_bytes += 1;
    for limits in [a, b, c, d, e, f] {
        let mut t = Terrain::default();
        assert!(execute(&mut t, &plan(true, false), request(), limits, &[]).is_err());
        assert_eq!(t.reads, 0);
        assert_eq!(t.verifies, 0);
    }
}
#[test]
fn invalid_request_and_store_options_reject_before_io() {
    let p = plan(true, false);
    let mut t = Terrain::default();
    let mut r = request();
    r.tile.tile.row = 64;
    assert!(matches!(
        execute(&mut t, &p, r, Limits::default(), &[]),
        Err(Error::Selection)
    ));
    r = request();
    r.tile.slopes.maximum = f32::NAN;
    assert!(matches!(
        execute(&mut t, &p, r, Limits::default(), &[]),
        Err(Error::Builder(tilebuilder::Error::Slopes))
    ));
    let mut c = store::StaticStore::new(
        Absent,
        &p.parent_rows().collect::<Vec<_>>(),
        Default::default(),
        Default::default(),
    )
    .unwrap();
    assert!(matches!(
        run(
            &p,
            &mut Sources {
                terrain: &mut t,
                collision: &mut c
            },
            models::LiquidTypes::new(&[]).unwrap(),
            &[],
            request(),
            Limits::default()
        ),
        Err(Error::Options)
    ));
    assert_eq!(t.reads, 0);
    assert_eq!(t.verifies, 0);
}
#[test]
fn parent_mismatch_rejects_before_io() {
    let p = plan(true, false);
    let mut t = Terrain::default();
    let parents = [
        store::MapParent {
            map_id: 1,
            parent: Some(2),
            cosmetic_parent: None,
        },
        store::MapParent {
            map_id: 2,
            parent: None,
            cosmetic_parent: None,
        },
    ];
    let mut c = store::StaticStore::new(
        Absent,
        &parents,
        store::Options {
            load_path_only_models: true,
            ..Default::default()
        },
        Default::default(),
    )
    .unwrap();
    assert!(matches!(
        run(
            &p,
            &mut Sources {
                terrain: &mut t,
                collision: &mut c
            },
            models::LiquidTypes::new(&[]).unwrap(),
            &[],
            request(),
            Limits::default()
        ),
        Err(Error::Topology)
    ));
    assert_eq!(t.reads, 0);
    assert_eq!(t.verifies, 0);
}

#[cfg(target_os = "linux")]
#[test]
#[ignore = "explicit synthetic private input/output required in RCE_NAVIGATION_PIPELINE_INPUT/OUTPUT"]
fn accept_pipeline_corpus() {
    use std::{fs, path::Path};
    let input = std::env::var_os("RCE_NAVIGATION_PIPELINE_INPUT").unwrap();
    let input = Path::new(&input);
    let output = std::env::var_os("RCE_NAVIGATION_PIPELINE_OUTPUT").unwrap();
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
    for name in fs::read_to_string(input.join("cases.txt")).unwrap().lines() {
        let folder = input.join(name);
        let text = fs::read_to_string(folder.join("plan.txt")).unwrap();
        let mut words = text.split_whitespace();
        fn next<T: std::str::FromStr>(w: &mut std::str::SplitWhitespace<'_>) -> T
        where
            T::Err: std::fmt::Debug,
        {
            w.next().unwrap().parse().unwrap()
        }
        let n: usize = next(&mut words);
        assert!(n <= 64);
        let map_id = next(&mut words);
        let tile = Coordinate {
            row: next(&mut words),
            column: next(&mut words),
        };
        let skip: u8 = next(&mut words);
        let build = next(&mut words);
        let big: u8 = next(&mut words);
        let slopes = crate::recast::Slopes {
            maximum: next(&mut words),
            non_steep: next(&mut words),
        };
        let nt: usize = next(&mut words);
        let nc: usize = next(&mut words);
        let mut rows = Vec::new();
        let mut descriptors = Vec::new();
        let mut bytes = vec![b'0'; 4108];
        bytes[..4].copy_from_slice(b"MAPS");
        bytes[4..8].copy_from_slice(&10u32.to_le_bytes());
        bytes[8..12].copy_from_slice(&u32::to_le_bytes(build));
        bytes[12 + 4095 - (usize::from(tile.row) * 64 + usize::from(tile.column))] = b'1';
        for _ in 0..n {
            let id = next(&mut words);
            let parent: u32 = next(&mut words);
            let cosmetic: u32 = next(&mut words);
            let bits: u8 = next(&mut words);
            rows.push(DirectRecord {
                id,
                fields: NavigationMapFields {
                    map_type: 0,
                    instance_type: 0,
                    parent_map_id: if parent == u32::MAX {
                        -1
                    } else {
                        parent.try_into().unwrap()
                    },
                    cosmetic_parent_map_id: if cosmetic == u32::MAX {
                        -1
                    } else {
                        cosmetic.try_into().unwrap()
                    },
                    flags: 0,
                },
            });
            if bits & 1 != 0 {
                descriptors.push(Input::Terrain {
                    map_id: id,
                    bytes: &bytes,
                });
            }
            if bits & 2 != 0 {
                descriptors.push(Input::CollisionTile { map_id: id, tile });
            }
        }
        let types: Vec<(u32, u8)> = (0..nt)
            .map(|_| (next(&mut words), next(&mut words)))
            .collect();
        let direct_types: Vec<_> = types
            .iter()
            .map(|&(id, sound_bank)| DirectRecord {
                id,
                fields: forever_db2::terrain_sources::LiquidTypeFields {
                    sound_bank,
                    material_id: 255,
                },
            })
            .collect();
        let liquid_table =
            models::liquid::Table::from_rows(&direct_types, Default::default()).unwrap();
        let off: Vec<_> = (0..nc)
            .map(|_| offmesh::Source {
                map_id: next(&mut words),
                row: next(&mut words),
                column: next(&mut words),
                from: std::array::from_fn(|_| next(&mut words)),
                to: std::array::from_fn(|_| next(&mut words)),
                radius: next(&mut words),
                connection_flags: next(&mut words),
                area: next(&mut words),
                flags: next(&mut words),
            })
            .collect();
        assert!(words.next().is_none());
        let p = inventory::Plan::from_rows(
            &rows,
            &descriptors,
            build,
            Options::default(),
            Default::default(),
        )
        .unwrap();
        let mut t =
            terrain::load::fs::RootedSource::new(&folder.join("maps"), Default::default()).unwrap();
        let v = store::fs::RootedSource::new(&folder.join("vmaps"), Default::default()).unwrap();
        let mut c = store::StaticStore::new(
            v,
            &p.parent_rows().collect::<Vec<_>>(),
            store::Options {
                load_path_only_models: true,
                ..Default::default()
            },
            Default::default(),
        )
        .unwrap();
        let r = Request {
            tile: tilebuilder::Request {
                map_id,
                tile,
                big_base_unit: big != 0,
                slopes,
            },
            skip_liquids: skip != 0,
        };
        let out = output.join(name);
        fs::create_dir(&out).unwrap();
        let global = mmap::encode_global(
            map_id,
            p.map(map_id).unwrap().params().unwrap(),
            &off,
            Default::default(),
        )
        .unwrap();
        fs::write(out.join("global.mmap"), global).unwrap();
        let result = if std::env::var_os("RCE_NAVIGATION_DISCOVERY_BINDING").is_some() {
            let mut package = crate::inventory::fs::Package::open_rows(
                &folder.join("maps"),
                &folder.join("vmaps"),
                &rows,
                build,
                Options::default(),
                Default::default(),
            )
            .unwrap();
            assert_eq!(
                package.plan().map(map_id).unwrap().params(),
                p.map(map_id).unwrap().params()
            );
            let result = package.run_tile(liquid_table.types(), &off, r, Limits::default());
            package.verify_unchanged().unwrap();
            fs::write(
                out.join("discovery-stats.txt"),
                format!("{:?}\n", package.statistics()),
            )
            .unwrap();
            result
        } else {
            run(
                &p,
                &mut Sources {
                    terrain: &mut t,
                    collision: &mut c,
                },
                liquid_table.types(),
                &off,
                r,
                Limits::default(),
            )
            .map_err(crate::inventory::fs::Error::Pipeline)
        }
        .unwrap_or_else(|e| panic!("{name}: {e}"));
        let stats = match result {
            Outcome::Empty { reason, statistics } => {
                fs::write(out.join("empty.txt"), format!("{reason:?}\n")).unwrap();
                statistics
            }
            Outcome::Tile { bytes, statistics } => {
                let view = mmap::TileView::parse(&bytes, Default::default()).unwrap();
                assert_eq!(view.uses_liquids(), !r.skip_liquids);
                assert_eq!(statistics.builder.subtiles, 625);
                fs::write(
                    out.join(format!(
                        "{map_id:04}_{:02}_{:02}.mmtile",
                        tile.row, tile.column
                    )),
                    bytes,
                )
                .unwrap();
                statistics
            }
        };
        assert_eq!(c.stats().tiles, 0);
        assert_eq!(c.stats().models, 0);
        assert_eq!(c.stats().active_instances, 0);
        assert!(!c.contains_map(map_id));
        fs::write(out.join("stats.txt"), format!("{stats:?}\n")).unwrap();
        // Exercise store exclusivity and cleanup with complete admitted inputs.
        if p.map(map_id).unwrap().collision_contains(tile) {
            c.load_tile(map_id, u16::from(tile.row), tile.column)
                .unwrap();
            assert!(matches!(
                run(
                    &p,
                    &mut Sources {
                        terrain: &mut t,
                        collision: &mut c
                    },
                    liquid_table.types(),
                    &off,
                    r,
                    Limits::default()
                ),
                Err(Error::StoreBusy)
            ));
            c.unload_map(map_id).unwrap();
            let mut l = Limits::default();
            l.models.models = 0;
            assert!(matches!(
                run(
                    &p,
                    &mut Sources {
                        terrain: &mut t,
                        collision: &mut c
                    },
                    liquid_table.types(),
                    &off,
                    r,
                    l
                ),
                Err(Error::Models(models::LoadedError::Geometry(
                    crate::Error::Limit
                )))
            ));
            assert_eq!(c.stats().tiles, 0);
            assert_eq!(c.stats().active_instances, 0);
            assert!(!c.contains_map(map_id));
        }
    }
}

#[test]
fn zero_pipeline_heap_cannot_hide_terrain_buffer_capacity() {
    let mut t = Terrain {
        bytes: Some(crate::terrain::tests::fixture(70205, 0, 0, false)),
        ..Default::default()
    };
    let limits = Limits {
        heap_bytes: 0,
        ..Default::default()
    };
    assert!(execute(&mut t, &plan(true, false), request(), limits, &[]).is_err());
}
