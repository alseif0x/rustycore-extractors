// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: synthetic directory inventories and source graphs.
use super::*;
fn row(id: u32, parent: i16, cosmetic: i16) -> DirectRecord<NavigationMapFields> {
    DirectRecord {
        id,
        fields: NavigationMapFields {
            map_type: 0,
            instance_type: 0,
            parent_map_id: parent,
            cosmetic_parent_map_id: cosmetic,
            flags: 0,
        },
    }
}
fn list(build: u32, tiles: &[(u8, u8)]) -> Vec<u8> {
    let mut b = vec![b'0'; 4108];
    b[..4].copy_from_slice(b"MAPS");
    b[4..8].copy_from_slice(&10u32.to_le_bytes());
    b[8..12].copy_from_slice(&build.to_le_bytes());
    for &(r, c) in tiles {
        b[12 + 4095 - (r as usize * 64 + c as usize)] = b'1';
    }
    b
}
fn plan(
    rows: &[DirectRecord<NavigationMapFields>],
    inputs: &[Input<'_>],
    options: Options,
) -> Result<Plan, Error> {
    Plan::from_rows(rows, inputs, 70205, options, Limits::default())
}
#[test]
fn union_is_unique_sorted_and_keeps_empty_entries() {
    let rows = [row(1, -1, -1), row(2, -1, -1)];
    let a = list(70205, &[(63, 0), (0, 63), (7, 8)]);
    let b = list(70205, &[(7, 8), (0, 0)]);
    let inputs = [
        Input::CollisionMap { map_id: 2 },
        Input::Terrain {
            map_id: 1,
            bytes: &a,
        },
        Input::Terrain {
            map_id: 1,
            bytes: &b,
        },
        Input::CollisionTile {
            map_id: 1,
            tile: Coordinate { row: 63, column: 0 },
        },
    ];
    let p = plan(&rows, &inputs, Options::default()).unwrap();
    assert_eq!(p.maps.len(), 2);
    assert_eq!(
        p.maps[0].tiles().collect::<Vec<_>>(),
        [(0, 0), (0, 63), (7, 8), (63, 0)].map(|(row, column)| Coordinate { row, column })
    );
    assert_eq!(p.statistics().inventory_tiles, 4);
    assert!(!p.maps[1].is_output());
    assert_eq!(p.maps[0].params().unwrap().max_tiles, 4);
}
#[test]
fn component_promises_preserve_overlap_and_normalized_parent_rows() {
    let rows = [row(1, -1, -1), row(2, -1, 1)];
    let terrain = list(70205, &[(0, 0), (0, 1)]);
    let inputs = [
        Input::Terrain {
            map_id: 1,
            bytes: &terrain,
        },
        Input::Terrain {
            map_id: 2,
            bytes: &terrain,
        },
        Input::CollisionTile {
            map_id: 2,
            tile: Coordinate { row: 0, column: 1 },
        },
        Input::CollisionTile {
            map_id: 2,
            tile: Coordinate { row: 0, column: 2 },
        },
    ];
    let p = plan(&rows, &inputs, Options::default()).unwrap();
    let m = p.map(2).unwrap();
    assert_eq!(m.tile_count(), 3);
    for (column, t, c) in [
        (0, true, false),
        (1, true, true),
        (2, false, true),
        (3, false, false),
    ] {
        let at = Coordinate { row: 0, column };
        assert_eq!(m.terrain_contains(at), t);
        assert_eq!(m.collision_contains(at), c);
        assert_eq!(m.contains(at), t || c);
    }
    assert!(!m.terrain_contains(Coordinate { row: 64, column: 0 }));
    assert!(!m.collision_contains(Coordinate { row: 0, column: 64 }));
    let parents = p.parent_rows().collect::<Vec<_>>();
    assert_eq!(parents.len(), 2);
    assert_eq!(parents[0].parent, None);
    assert_eq!(parents[1].parent, Some(1));
    assert_eq!(parents[1].cosmetic_parent, None);
}
#[test]
fn ultimate_effective_parent_uses_own_union_not_child_union() {
    let rows = [row(9, 7, 1), row(1, -1, -1), row(7, -1, 1)];
    let root = list(70205, &[(63, 62)]);
    let child = list(70205, &[(1, 2), (5, 6)]);
    let p = plan(
        &rows,
        &[
            Input::Terrain {
                map_id: 1,
                bytes: &root,
            },
            Input::Terrain {
                map_id: 9,
                bytes: &child,
            },
        ],
        Options::default(),
    )
    .unwrap();
    let m = p.map(9).unwrap();
    assert_eq!(m.root_map_id(), 1);
    assert_eq!(m.tile_count(), 2);
    let expected = tilebuilder::global_params(&[Coordinate {
        row: 63,
        column: 62,
    }])
    .unwrap();
    assert_eq!(m.params(), Some(expected));
    assert_eq!(p.ancestors(9).unwrap().collect::<Vec<_>>(), [9, 7, 1]);
}
#[test]
fn direct_parent_zero_wins_over_cosmetic_and_last_duplicate_wins() {
    let mut rows = vec![row(0, -1, -1), row(9, 7, -1), row(7, -1, -1), row(9, 0, 7)];
    rows[1].fields.map_type = 3;
    let input = list(70205, &[(1, 2)]);
    let p = plan(
        &rows,
        &[
            Input::Terrain {
                map_id: 0,
                bytes: &input,
            },
            Input::Terrain {
                map_id: 9,
                bytes: &input,
            },
        ],
        Options {
            skip_junk: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(p.map(9).unwrap().root_map_id(), 0);
    assert_eq!(p.statistics.unique_source_maps, 3);
    assert_eq!(p.ancestors(9).unwrap().collect::<Vec<_>>(), [9, 0]);
}
#[test]
fn single_map_retains_ancestors_and_overrides_every_skip_flag() {
    let mut rows = [row(0, -1, -1), row(5, 0, -1), row(6, -1, -1)];
    rows[1].fields.map_type = 3;
    rows[0].fields.flags = 2;
    rows[1].fields.instance_type = 3;
    let input = list(70205, &[(0, 0)]);
    let p = plan(
        &rows,
        &[
            Input::Terrain {
                map_id: 0,
                bytes: &input,
            },
            Input::Terrain {
                map_id: 5,
                bytes: &input,
            },
            Input::Terrain {
                map_id: 6,
                bytes: &input,
            },
        ],
        Options {
            map: Some(5),
            skip_continents: true,
            skip_junk: true,
            skip_battlegrounds: true,
        },
    )
    .unwrap();
    assert_eq!(p.maps.iter().map(Map::map_id).collect::<Vec<_>>(), [0, 5]);
    assert!(!p.maps[0].is_output());
    assert!(p.maps[1].is_output());
    assert_eq!(p.statistics.output_tiles, 1);
}
#[test]
fn all_map_filters_match_original_numeric_predicates() {
    let mut rows = [
        row(0, -1, -1),
        row(4, -1, -1),
        row(5, -1, -1),
        row(6, -1, -1),
        row(7, -1, -1),
    ];
    rows[1].fields.flags = 2;
    rows[2].fields.map_type = 3;
    rows[3].fields.instance_type = 3;
    rows[4].fields.instance_type = -1;
    let inputs = rows
        .iter()
        .map(|r| Input::CollisionMap { map_id: r.id })
        .collect::<Vec<_>>();
    let p = plan(
        &rows,
        &inputs,
        Options {
            skip_continents: true,
            skip_junk: true,
            skip_battlegrounds: true,
            map: None,
        },
    )
    .unwrap();
    assert_eq!(p.maps.iter().map(Map::map_id).collect::<Vec<_>>(), [7]);
}
#[test]
fn missing_and_empty_ancestor_inventory_are_distinct_errors() {
    let rows = [row(1, -1, -1), row(5, 1, -1)];
    let input = list(70205, &[(1, 2)]);
    for (inputs, error) in [
        (
            vec![Input::Terrain {
                map_id: 5,
                bytes: &input,
            }],
            Error::MissingInventory,
        ),
        (
            vec![
                Input::Terrain {
                    map_id: 5,
                    bytes: &input,
                },
                Input::CollisionMap { map_id: 1 },
            ],
            Error::EmptyAncestor,
        ),
    ] {
        assert!(matches!(plan(&rows,&inputs,Options::default()),Err(e) if e==error));
    }
}
#[test]
fn graph_rejects_cycles_missing_parents_invalid_sentinels_and_depth() {
    for (rows, e) in [
        (vec![row(1, 2, -1), row(2, 1, -1)], Error::Cycle),
        (vec![row(1, 2, -1)], Error::Parent),
        (vec![row(1, -1, -2)], Error::Parent),
    ] {
        assert!(matches!(plan(&rows,&[],Options::default()),Err(got) if got==e));
    }
    let rows = [row(1, -1, -1), row(2, 1, -1)];
    assert!(matches!(
        Plan::from_rows(
            &rows,
            &[],
            70205,
            Options::default(),
            Limits {
                depth: 1,
                ..Default::default()
            }
        ),
        Err(Error::Depth)
    ));
}
#[test]
fn ignored_negative_direct_parent_uses_cosmetic() {
    let rows = [row(1, -1, -1), row(2, -32768, 1)];
    let p = plan(&rows, &[], Options::default()).unwrap();
    assert_eq!(p.ancestors(2).unwrap().collect::<Vec<_>>(), [2, 1]);
}
#[test]
fn tilelist_requires_full_matching_build_ascii_and_exact_length() {
    let rows = [row(1, -1, -1)];
    let original = list(70205, &[(0, 0)]);
    for b in [
        original[..4107].to_vec(),
        [original.as_slice(), &[0]].concat(),
        list(70170, &[(0, 0)]),
        {
            let mut b = original.clone();
            b[12] = b'2';
            b
        },
    ] {
        assert!(matches!(
            plan(
                &rows,
                &[Input::Terrain {
                    map_id: 1,
                    bytes: &b
                }],
                Options::default()
            ),
            Err(Error::TileList)
        ));
    }
    assert!(matches!(
        Plan::from_rows(&rows, &[], 51943, Options::default(), Limits::default()),
        Err(Error::Build)
    ));
}
#[test]
fn skipped_corrupt_tilelist_is_not_admitted_or_decoded() {
    let rows = [row(0, -1, -1)];
    let p = plan(
        &rows,
        &[Input::Terrain {
            map_id: 0,
            bytes: &[],
        }],
        Options {
            skip_continents: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(p.maps().is_empty());
}
#[test]
fn unknown_selection_descriptor_and_coordinates_reject() {
    let rows = [row(1, -1, -1)];
    assert!(matches!(
        plan(
            &rows,
            &[],
            Options {
                map: Some(2),
                ..Default::default()
            }
        ),
        Err(Error::UnknownMap)
    ));
    assert!(matches!(
        plan(
            &rows,
            &[Input::CollisionMap { map_id: 2 }],
            Options::default()
        ),
        Err(Error::UnknownMap)
    ));
    assert!(matches!(
        plan(
            &rows,
            &[Input::CollisionTile {
                map_id: 1,
                tile: Coordinate { row: 64, column: 0 }
            }],
            Options::default()
        ),
        Err(Error::Coordinates)
    ));
    assert!(matches!(
        plan(
            &rows,
            &[],
            Options {
                map: Some(1),
                ..Default::default()
            }
        ),
        Err(Error::MissingInventory)
    ));
}
#[test]
fn empty_complete_selection_and_max_source_id_are_explicit() {
    let p = plan(&[], &[], Options::default()).unwrap();
    assert!(p.maps().is_empty());
    let rows = [row(u32::MAX, -1, -1)];
    let b = list(70205, &[(63, 63)]);
    let p = plan(
        &rows,
        &[Input::Terrain {
            map_id: u32::MAX,
            bytes: &b,
        }],
        Options::default(),
    )
    .unwrap();
    assert_eq!(p.maps()[0].map_id(), u32::MAX);
    assert!(matches!(
        plan(
            &rows,
            &[],
            Options {
                map: Some(u32::MAX),
                ..Default::default()
            }
        ),
        Err(Error::Selection)
    ));
}
#[test]
fn limits_only_reduce_and_exact_work_heap_thresholds_hold() {
    let rows = [row(1, -1, -1)];
    let b = list(70205, &[(0, 0)]);
    let inputs = [Input::Terrain {
        map_id: 1,
        bytes: &b,
    }];
    let p = plan(&rows, &inputs, Options::default()).unwrap();
    let s = p.statistics();
    for limits in [
        Limits {
            work: s.work - 1,
            ..Default::default()
        },
        Limits {
            heap_bytes: s.heap_reservation - 1,
            ..Default::default()
        },
        Limits {
            rows: 0,
            ..Default::default()
        },
        Limits {
            maps: 0,
            ..Default::default()
        },
        Limits {
            descriptors: 0,
            ..Default::default()
        },
        Limits {
            depth: 1025,
            ..Default::default()
        },
    ] {
        assert!(matches!(
            Plan::from_rows(&rows, &inputs, 70205, Options::default(), limits),
            Err(Error::Limit)
        ));
    }
    assert!(
        Plan::from_rows(
            &rows,
            &inputs,
            70205,
            Options::default(),
            Limits {
                work: s.work,
                heap_bytes: s.heap_reservation,
                ..Default::default()
            }
        )
        .is_ok()
    );
}

#[test]
#[ignore = "requires explicit external synthetic directory inventory/output"]
fn write_inventory_corpus() {
    use std::{env, fs, path::PathBuf};
    let input = PathBuf::from(env::var_os("RCE_NAVIGATION_INVENTORY_INPUT").expect("input"));
    let output = PathBuf::from(env::var_os("RCE_NAVIGATION_INVENTORY_OUTPUT").expect("output"));
    fs::create_dir(&output).unwrap();
    // QA bridge enumerates canonical synthetic filenames into descriptors.
    // This is not a production filesystem discovery API or package verifier.
    for entry in fs::read_dir(input).unwrap() {
        let root = entry.unwrap().path();
        let out = output.join(root.file_name().unwrap());
        fs::create_dir(&out).unwrap();
        let bytes = fs::read(root.join("Map.db2")).unwrap();
        let source = forever_db2::terrain_sources::read_navigation_maps(&bytes).unwrap();
        let mut projection = vec![
            source.direct_records().len() as u32,
            source.copies().len() as u32,
        ];
        for r in source.direct_records() {
            let f = r.fields;
            projection.extend([
                r.id,
                u32::from(f.map_type),
                f.instance_type as i32 as u32,
                f.parent_map_id as i32 as u32,
                f.cosmetic_parent_map_id as i32 as u32,
                f.flags as u32,
            ]);
        }
        for c in source.copies() {
            projection.extend([c.new_id, c.source_id]);
        }
        fs::write(
            out.join("source.bin"),
            projection
                .iter()
                .flat_map(|v| v.to_le_bytes())
                .collect::<Vec<_>>(),
        )
        .unwrap();
        let options = fs::read(root.join("options.bin")).unwrap();
        let w = options
            .as_chunks::<4>()
            .0
            .iter()
            .map(|b| u32::from_le_bytes(*b))
            .collect::<Vec<_>>();
        assert_eq!(w.len(), 4);
        let options = Options {
            map: (w[0] != 0).then_some(w[1]),
            skip_continents: w[2] & 1 != 0,
            skip_junk: w[2] & 2 != 0,
            skip_battlegrounds: w[2] & 4 != 0,
        };
        let mut terrain = Vec::new();
        let mut inputs = Vec::new();
        for entry in fs::read_dir(root.join("maps")).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().unwrap() == "tilelist" {
                let id = path
                    .file_stem()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .parse::<u32>()
                    .unwrap();
                terrain.push((id, fs::read(path).unwrap()));
            }
        }
        for (id, b) in &terrain {
            inputs.push(Input::Terrain {
                map_id: *id,
                bytes: b,
            });
        }
        for entry in fs::read_dir(root.join("vmaps")).unwrap() {
            let path = entry.unwrap().path();
            let id = path
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .parse::<u32>()
                .unwrap();
            inputs.push(Input::CollisionMap { map_id: id });
            for entry in fs::read_dir(path).unwrap() {
                let path = entry.unwrap().path();
                if path.extension().unwrap() == "vmtile" {
                    let name = path.file_stem().unwrap().to_str().unwrap().to_owned();
                    let parts = name.split('_').collect::<Vec<_>>();
                    assert_eq!(parts.len(), 3);
                    assert_eq!(parts[0].parse::<u32>().unwrap(), id);
                    inputs.push(Input::CollisionTile {
                        map_id: id,
                        tile: Coordinate {
                            row: parts[1].parse().unwrap(),
                            column: parts[2].parse().unwrap(),
                        },
                    });
                }
            }
        }
        let plan = Plan::from_source(&source, &inputs, w[3], options, Limits::default()).unwrap();
        let mut inventory = vec![
            plan.maps().len() as u32,
            plan.statistics().inventory_tiles as u32,
        ];
        for m in plan.maps() {
            inventory.extend([m.map_id(), m.tile_count() as u32, u32::from(m.is_output())]);
            inventory.extend(
                m.tiles()
                    .map(|t| u32::from(t.row) << 16 | u32::from(t.column)),
            );
            if let Some(params) = m.params() {
                let b = mmap::encode_global(m.map_id(), params, &[], Default::default()).unwrap();
                fs::write(out.join(format!("{:04}.mmap", m.map_id())), b).unwrap();
            }
        }
        fs::write(
            out.join("inventory.bin"),
            inventory
                .iter()
                .flat_map(|v| v.to_le_bytes())
                .collect::<Vec<_>>(),
        )
        .unwrap();
        let s = plan.statistics();
        fs::write(
            out.join("stats.txt"),
            format!(
                "{} {} {} {} {} {} {} {}\n",
                s.direct_rows,
                s.unique_source_maps,
                s.discovered_maps,
                s.inventory_tiles,
                s.output_maps,
                s.output_tiles,
                s.work,
                s.heap_reservation
            ),
        )
        .unwrap();
    }
}

#[test]
fn all_ten_continent_ids_and_only_those_ids_are_filtered() {
    let ids = [0, 1, 530, 571, 870, 1116, 1220, 1642, 1643, 2222, 2];
    let rows = ids.map(|id| row(id, -1, -1));
    let inputs = ids.map(|id| Input::CollisionMap { map_id: id });
    let p = plan(
        &rows,
        &inputs,
        Options {
            skip_continents: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(p.maps().iter().map(Map::map_id).collect::<Vec<_>>(), [2]);
}
#[test]
fn complete_1024_node_parent_chain_is_admitted_and_1025_rejects() {
    let mut rows = (1..=1024)
        .map(|id| row(id, if id == 1 { -1 } else { (id - 1) as i16 }, -1))
        .collect::<Vec<_>>();
    let p = plan(&rows, &[], Options::default()).unwrap();
    assert_eq!(p.ancestors(1024).unwrap().count(), 1024);
    rows.push(row(1025, 1024, -1));
    assert!(matches!(
        plan(&rows, &[], Options::default()),
        Err(Error::Depth)
    ));
}
