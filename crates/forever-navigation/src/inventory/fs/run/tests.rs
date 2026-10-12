// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: sequential selection, provisional sinks and source binding.
use super::*;
use forever_db2::terrain_sources::{DirectRecord, NavigationMapFields};
use std::{
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "rce-navigation-run-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        fs::create_dir(p.join("maps")).unwrap();
        fs::create_dir(p.join("vmaps")).unwrap();
        Self(p)
    }
    fn maps(&self) -> PathBuf {
        self.0.join("maps")
    }
    fn vmaps(&self) -> PathBuf {
        self.0.join("vmaps")
    }
    fn map(&self, id: u32, tiles: &[(u8, u8)]) {
        let mut b = vec![b'0'; 4108];
        b[..4].copy_from_slice(b"MAPS");
        b[4..8].copy_from_slice(&10u32.to_le_bytes());
        b[8..12].copy_from_slice(&70205u32.to_le_bytes());
        for &(r, c) in tiles {
            b[12 + 4095 - (usize::from(r) * 64 + usize::from(c))] = b'1';
            fs::write(
                self.maps().join(format!("{id:04}_{r:02}_{c:02}.map")),
                crate::terrain::tests::fixture(70205, 0, 0, false),
            )
            .unwrap();
        }
        fs::write(self.maps().join(format!("{id:04}.tilelist")), b).unwrap();
    }
    fn open(&self, rows: &[DirectRecord<NavigationMapFields>]) -> Package {
        Package::open_rows(
            &self.maps(),
            &self.vmaps(),
            rows,
            70205,
            super::super::Options::default(),
            Default::default(),
        )
        .unwrap()
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
fn row(id: u32, parent: i16) -> DirectRecord<NavigationMapFields> {
    DirectRecord {
        id,
        fields: NavigationMapFields {
            map_type: 0,
            instance_type: 0,
            flags: 0,
            parent_map_id: parent,
            cosmetic_parent_map_id: -1,
        },
    }
}
type Seen = (u32, Option<Coordinate>);
#[derive(Default)]
struct Recording {
    seen: Vec<Seen>,
    globals: Vec<Vec<u8>>,
    action: Option<Box<dyn FnMut(Seen)>>,
    fail_at: Option<usize>,
}
impl Sink for Recording {
    type Error = &'static str;
    fn emit(&mut self, event: Event<'_>) -> Result<(), Self::Error> {
        let item = match event {
            Event::Global { map_id, bytes } => {
                mmap::GlobalView::parse(bytes, map_id, Default::default()).unwrap();
                self.globals.push(bytes.to_vec());
                (map_id, None)
            }
            Event::Tile {
                map_id,
                tile,
                outcome,
            } => {
                assert!(matches!(
                    outcome,
                    pipeline::Outcome::Empty {
                        reason: pipeline::EmptyReason::NoVertices,
                        ..
                    }
                ));
                (map_id, Some(tile))
            }
        };
        self.seen.push(item);
        if let Some(action) = &mut self.action {
            action(item);
        }
        if self.fail_at == Some(self.seen.len()) {
            Err("sink-failed")
        } else {
            Ok(())
        }
    }
}
fn run(
    p: &mut Package,
    selection: Selection,
    limits: Limits,
    sink: &mut Recording,
) -> Result<Completed, Failure<&'static str>> {
    execute(
        p,
        models::LiquidTypes::new(&[]).unwrap(),
        &[],
        Options {
            selection,
            ..Default::default()
        },
        limits,
        sink,
    )
}
fn t(r: u8, c: u8) -> Coordinate {
    Coordinate { row: r, column: c }
}
#[test]
fn all_maps_are_sorted_tiles_row_major_and_empty_maps_emit_nothing() {
    let tmp = Temp::new();
    tmp.map(2, &[(4, 5), (0, 3), (0, 0)]);
    tmp.map(1, &[(63, 63)]);
    tmp.map(3, &[]);
    let mut p = tmp.open(&[row(3, -1), row(2, -1), row(1, -1)]);
    let resident = p.statistics().resident_capacity;
    let mut sink = Recording::default();
    let c = run(&mut p, Selection::All, Default::default(), &mut sink).unwrap();
    assert_eq!(
        sink.seen,
        vec![
            (1, None),
            (1, Some(t(63, 63))),
            (2, None),
            (2, Some(t(0, 0))),
            (2, Some(t(0, 3))),
            (2, Some(t(4, 5)))
        ]
    );
    assert_eq!(
        c.progress,
        Progress {
            maps: 2,
            tiles: 4,
            empty_tiles: 4,
            written_tiles: 0,
            output_bytes: 80
        }
    );
    assert_eq!(p.statistics().resident_capacity, resident);
    assert!(!p.failed());
}
#[test]
fn explicit_map_and_tile_preserve_whole_map_global_parameters() {
    let tmp = Temp::new();
    tmp.map(1, &[(4, 5), (0, 3)]);
    tmp.map(2, &[(9, 9)]);
    let mut p = tmp.open(&[row(1, -1), row(2, -1)]);
    let mut a = Recording::default();
    let mut b = Recording::default();
    assert_eq!(
        run(&mut p, Selection::Map(1), Default::default(), &mut a)
            .unwrap()
            .progress
            .tiles,
        2
    );
    assert_eq!(
        run(
            &mut p,
            Selection::Tile {
                map_id: 1,
                tile: t(4, 5)
            },
            Default::default(),
            &mut b
        )
        .unwrap()
        .progress
        .tiles,
        1
    );
    assert_eq!(a.globals, b.globals);
    assert_eq!(b.seen, vec![(1, None), (1, Some(t(4, 5)))]);
}
#[test]
fn child_selection_emits_no_ancestor_output_and_uses_ancestor_parameters() {
    let tmp = Temp::new();
    tmp.map(1, &[(4, 5)]);
    tmp.map(2, &[(0, 3)]);
    let mut p = Package::open_rows(
        &tmp.maps(),
        &tmp.vmaps(),
        &[row(1, -1), row(2, 1)],
        70205,
        super::super::Options {
            map: Some(2),
            ..Default::default()
        },
        Default::default(),
    )
    .unwrap();
    assert!(!p.plan.map(1).unwrap().is_output());
    let mut s = Recording::default();
    run(&mut p, Selection::All, Default::default(), &mut s).unwrap();
    assert_eq!(s.seen, vec![(2, None), (2, Some(t(0, 3)))]);
    assert_eq!(
        mmap::GlobalView::parse(&s.globals[0], 2, Default::default())
            .unwrap()
            .params(),
        tilebuilder::global_params(&[t(4, 5)]).unwrap()
    );
}
#[test]
fn empty_all_completes_without_callbacks() {
    let tmp = Temp::new();
    let mut p = tmp.open(&[row(1, -1)]);
    let mut s = Recording::default();
    assert_eq!(
        run(&mut p, Selection::All, Default::default(), &mut s)
            .unwrap()
            .progress,
        Progress::default()
    );
    assert!(s.seen.is_empty());
}
#[test]
fn invalid_selection_and_empty_explicit_map_reject_before_verification_or_sink() {
    let tmp = Temp::new();
    tmp.map(1, &[(0, 0)]);
    tmp.map(2, &[]);
    let mut p = tmp.open(&[row(1, -1), row(2, -1)]);
    for selection in [
        Selection::Map(3),
        Selection::Map(2),
        Selection::Tile {
            map_id: 1,
            tile: t(0, 1),
        },
        Selection::Tile {
            map_id: 1,
            tile: t(64, 0),
        },
    ] {
        let stats = p.statistics();
        let mut s = Recording::default();
        let f = run(&mut p, selection, Default::default(), &mut s).unwrap_err();
        assert!(matches!(f.cause, Cause::Run(Error::Selection)));
        assert_eq!(f.progress, Progress::default());
        assert_eq!(p.statistics(), stats);
        assert!(s.seen.is_empty());
    }
}
#[test]
fn limits_validate_before_input_checks_including_empty_selection() {
    let tmp = Temp::new();
    let mut p = tmp.open(&[row(1, -1)]);
    let mut global = mmap::Limits::default();
    global.records += 1;
    let mut pipeline = pipeline::Limits::default();
    pipeline.heap_bytes += 1;
    for l in [
        Limits {
            maps: 4097,
            ..Default::default()
        },
        Limits {
            tiles: 4096 * 4096 + 1,
            ..Default::default()
        },
        Limits {
            output_bytes: 1024 * 1024 * 1024 * 1024 + 1,
            ..Default::default()
        },
        Limits {
            global,
            ..Default::default()
        },
        Limits {
            pipeline,
            ..Default::default()
        },
    ] {
        let stats = p.statistics();
        let mut s = Recording::default();
        assert!(run(&mut p, Selection::All, l, &mut s).is_err());
        assert_eq!(p.statistics(), stats);
        assert!(s.seen.is_empty());
    }
}
#[test]
fn invalid_slopes_reject_before_io_even_when_nothing_selected() {
    let tmp = Temp::new();
    let mut p = tmp.open(&[row(1, -1)]);
    let stats = p.statistics();
    let mut s = Recording::default();
    assert!(
        execute(
            &mut p,
            models::LiquidTypes::new(&[]).unwrap(),
            &[],
            Options {
                slopes: recast::Slopes {
                    maximum: f32::NAN,
                    non_steep: 0.0
                },
                ..Default::default()
            },
            Default::default(),
            &mut s
        )
        .is_err()
    );
    assert_eq!(p.statistics(), stats);
    assert!(s.seen.is_empty());
}
#[test]
fn map_and_tile_count_limits_are_preflighted_without_output() {
    let tmp = Temp::new();
    tmp.map(1, &[(0, 0), (4, 4)]);
    let mut p = tmp.open(&[row(1, -1)]);
    for l in [
        Limits {
            maps: 0,
            ..Default::default()
        },
        Limits {
            tiles: 1,
            ..Default::default()
        },
    ] {
        let stats = p.statistics();
        let mut s = Recording::default();
        assert!(matches!(
            run(&mut p, Selection::All, l, &mut s).unwrap_err().cause,
            Cause::Run(Error::Limit)
        ));
        assert_eq!(p.statistics(), stats);
        assert!(s.seen.is_empty());
    }
    assert!(
        run(
            &mut p,
            Selection::All,
            Limits {
                maps: 1,
                tiles: 2,
                output_bytes: 40,
                ..Default::default()
            },
            &mut Recording::default()
        )
        .is_ok()
    );
}
#[test]
fn output_byte_ceiling_checks_before_callback_and_retains_provisional_progress() {
    let tmp = Temp::new();
    tmp.map(1, &[(0, 0)]);
    tmp.map(2, &[(0, 0)]);
    let mut p = tmp.open(&[row(1, -1), row(2, -1)]);
    let mut s = Recording::default();
    let f = run(
        &mut p,
        Selection::All,
        Limits {
            output_bytes: 79,
            ..Default::default()
        },
        &mut s,
    )
    .unwrap_err();
    assert!(matches!(f.cause, Cause::Run(Error::Limit)));
    assert_eq!(
        f.progress,
        Progress {
            maps: 1,
            tiles: 1,
            empty_tiles: 1,
            output_bytes: 40,
            ..Default::default()
        }
    );
    assert_eq!(s.seen.len(), 2);
    assert!(!p.failed());
}
#[test]
fn sink_error_is_preserved_and_only_acknowledged_callbacks_are_counted() {
    let tmp = Temp::new();
    tmp.map(1, &[(0, 0)]);
    let mut p = tmp.open(&[row(1, -1)]);
    for fail in [1, 2] {
        let mut s = Recording {
            fail_at: Some(fail),
            ..Default::default()
        };
        let f = run(&mut p, Selection::All, Default::default(), &mut s).unwrap_err();
        assert!(matches!(f.cause, Cause::Sink("sink-failed")));
        assert_eq!(f.progress.maps, fail - 1);
        assert_eq!(f.progress.tiles, 0);
        assert_eq!(f.progress.output_bytes, if fail == 1 { 0 } else { 40 });
        assert!(!p.failed());
    }
}
#[test]
fn preexisting_mutation_rejects_before_any_output_and_poison_is_permanent() {
    let tmp = Temp::new();
    tmp.map(1, &[(0, 0)]);
    let mut p = tmp.open(&[row(1, -1)]);
    fs::write(tmp.maps().join("0001_00_00.map"), b"changed").unwrap();
    let mut s = Recording::default();
    for _ in 0..2 {
        assert!(matches!(
            run(&mut p, Selection::All, Default::default(), &mut s)
                .unwrap_err()
                .cause,
            Cause::Run(Error::Input(super::super::Error::InputChanged))
        ));
        assert!(p.failed());
        assert!(s.seen.is_empty());
    }
}
#[test]
fn mutation_after_global_is_caught_against_original_artifact_identity() {
    let tmp = Temp::new();
    tmp.map(1, &[(0, 0)]);
    let mut p = tmp.open(&[row(1, -1)]);
    let path = tmp.maps().join("0001_00_00.map");
    let mut s = Recording {
        action: Some(Box::new(move |(_, tile)| {
            if tile.is_none() {
                fs::write(&path, b"changed").unwrap()
            }
        })),
        ..Default::default()
    };
    let f = run(&mut p, Selection::All, Default::default(), &mut s).unwrap_err();
    assert_eq!(f.progress.maps, 1);
    assert_eq!(f.progress.tiles, 0);
    assert!(p.failed());
    assert_eq!(s.seen.len(), 1);
}
#[test]
fn later_tile_mutation_rejects_whole_selection_after_earlier_provisional_tile() {
    let tmp = Temp::new();
    tmp.map(1, &[(0, 0), (4, 4)]);
    let mut p = tmp.open(&[row(1, -1)]);
    let path = tmp.maps().join("0001_04_04.map");
    let mut s = Recording {
        action: Some(Box::new(move |(_, tile)| {
            if tile == Some(t(0, 0)) {
                fs::write(&path, b"changed").unwrap()
            }
        })),
        ..Default::default()
    };
    let f = run(&mut p, Selection::All, Default::default(), &mut s).unwrap_err();
    assert_eq!(f.progress.tiles, 1);
    assert!(p.failed());
    assert_eq!(s.seen.len(), 2);
}
#[test]
fn unused_file_mutation_is_rejected_by_final_full_verification() {
    let tmp = Temp::new();
    tmp.map(1, &[(0, 0)]);
    let path = tmp.maps().join("unused");
    fs::write(&path, b"original").unwrap();
    let mut p = tmp.open(&[row(1, -1)]);
    let mut s = Recording {
        action: Some(Box::new(move |(_, tile)| {
            if tile.is_none() {
                fs::write(&path, b"changed").unwrap()
            }
        })),
        ..Default::default()
    };
    let f = run(&mut p, Selection::All, Default::default(), &mut s).unwrap_err();
    assert_eq!(f.progress.tiles, 1);
    assert_eq!(s.seen.len(), 2);
    assert!(p.failed());
    assert!(matches!(
        f.cause,
        Cause::Run(Error::Input(super::super::Error::InputChanged))
    ));
}
#[test]
fn new_entry_after_last_tile_rejects_final_verification() {
    let tmp = Temp::new();
    tmp.map(1, &[(0, 0)]);
    let path = tmp.vmaps().join("new-entry");
    let mut p = tmp.open(&[row(1, -1)]);
    let mut s = Recording {
        action: Some(Box::new(move |(_, tile)| {
            if tile.is_some() {
                fs::write(&path, b"new").unwrap();
                changed_mtime(path.parent().unwrap());
            }
        })),
        ..Default::default()
    };
    assert_eq!(
        run(&mut p, Selection::All, Default::default(), &mut s)
            .unwrap_err()
            .progress
            .tiles,
        1
    );
    assert!(p.failed());
}
#[test]
fn missing_promised_file_remains_error_without_fallback_or_success() {
    let tmp = Temp::new();
    tmp.map(1, &[(0, 0), (4, 4)]);
    fs::remove_file(tmp.maps().join("0001_04_04.map")).unwrap();
    let mut p = tmp.open(&[row(1, -1)]);
    let mut s = Recording::default();
    let f = run(&mut p, Selection::All, Default::default(), &mut s).unwrap_err();
    assert!(matches!(
        f.cause,
        Cause::Run(Error::Input(super::super::Error::Pipeline(
            pipeline::Error::MissingTerrain
        )))
    ));
    assert_eq!(f.progress.tiles, 1);
    assert!(!p.failed());
}
#[test]
fn bound_names_are_exact_and_reject_traversal_and_noncanonical_coordinates() {
    assert_eq!(
        terrain_name(terrain::load::Artifact {
            map_id: 1,
            tile: t(2, 3)
        })
        .unwrap()
        .bytes(),
        b"0001_02_03.map"
    );
    for a in [
        store::Artifact::Tree { map_id: 1 },
        store::Artifact::Tile {
            map_id: 1,
            x: 2,
            y: 3,
        },
        store::Artifact::Index {
            map_id: 1,
            x: 2,
            y: 3,
        },
        store::Artifact::Model { name: b"foo.m2" },
    ] {
        let expected = match a {
            store::Artifact::Tree { .. } => b"0001/0001.vmtree".as_slice(),
            store::Artifact::Tile { .. } => b"0001/0001_02_03.vmtile",
            store::Artifact::Index { .. } => b"0001/0001_02_03.vmtileidx",
            _ => b"foo.m2.vmo",
        };
        assert_eq!(collision_name(a).unwrap().bytes(), expected);
    }
    for name in [
        b"".as_slice(),
        b"..",
        b".",
        b"../x",
        b"a/b",
        b"a\\b",
        b"x\0y",
        b"\xff",
    ] {
        assert!(collision_name(store::Artifact::Model { name }).is_err());
    }
    assert!(collision_name(store::Artifact::Model { name: &[b'x'; 501] }).is_err());
    assert_eq!(
        collision_name(store::Artifact::Model { name: &[b'x'; 500] })
            .unwrap()
            .bytes()
            .len(),
        504
    );
    assert!(
        collision_name(store::Artifact::Tile {
            map_id: 1,
            x: 64,
            y: 0
        })
        .is_err()
    );
    assert!(collision_name(store::Artifact::Tree { map_id: 10000 }).is_err());
    assert!(
        terrain_name(terrain::load::Artifact {
            map_id: 10000,
            tile: t(0, 0)
        })
        .is_err()
    );
}
#[test]
fn full_inventory_cost_is_twice_extra_entries_independent_of_tile_count() {
    fn measure(extra: usize) -> u64 {
        let tmp = Temp::new();
        tmp.map(1, &[(0, 0), (4, 4), (8, 8), (63, 63)]);
        for i in 0..extra {
            fs::write(tmp.maps().join(format!("unused-{i}")), []).unwrap();
        }
        let mut p = tmp.open(&[row(1, -1)]);
        run(
            &mut p,
            Selection::All,
            Default::default(),
            &mut Recording::default(),
        )
        .unwrap()
        .metadata_calls
    }
    assert_eq!(measure(600) - measure(0), 1200);
}
#[test]
fn per_tile_metadata_limit_rejects_and_releases_temporary_parent_capacity() {
    let tmp = Temp::new();
    tmp.map(1, &[(0, 0)]);
    let mut p = tmp.open(&[row(1, -1)]);
    let resident = p.statistics().resident_capacity;
    p.budget.limits.metadata_calls = 20;
    let f = run(
        &mut p,
        Selection::All,
        Default::default(),
        &mut Recording::default(),
    )
    .unwrap_err();
    assert!(matches!(
        f.cause,
        Cause::Run(Error::Input(super::super::Error::Limit))
    ));
    assert_eq!(p.statistics().resident_capacity, resident);
    assert!(!p.failed());
}

#[test]
#[ignore = "complete synthetic private packages required in RCE_NAVIGATION_BATCH_INPUT/OUTPUT"]
fn accept_batch_corpus() {
    let input = std::env::var_os("RCE_NAVIGATION_BATCH_INPUT").unwrap();
    let input = Path::new(&input);
    let output = std::env::var_os("RCE_NAVIGATION_BATCH_OUTPUT").unwrap();
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
    struct Files {
        root: PathBuf,
        order: Vec<String>,
    }
    impl Sink for Files {
        type Error = std::io::Error;
        fn emit(&mut self, event: Event<'_>) -> Result<(), Self::Error> {
            match event {
                Event::Global { map_id, bytes } => {
                    fs::write(self.root.join(format!("{map_id:04}.mmap")), bytes)?;
                    self.order.push(format!("global {map_id}"));
                }
                Event::Tile {
                    map_id,
                    tile,
                    outcome,
                } => {
                    let name = format!("{map_id:04}_{:02}_{:02}", tile.row, tile.column);
                    let stats = match outcome {
                        pipeline::Outcome::Tile { bytes, statistics } => {
                            mmap::TileView::parse(bytes, Default::default()).unwrap();
                            fs::write(self.root.join(format!("{name}.mmtile")), bytes)?;
                            statistics
                        }
                        pipeline::Outcome::Empty { reason, statistics } => {
                            fs::write(
                                self.root.join(format!("{name}.empty")),
                                format!("{reason:?}\n"),
                            )?;
                            statistics
                        }
                    };
                    fs::write(
                        self.root.join(format!("{name}.stats")),
                        format!("{stats:?}\n"),
                    )?;
                    self.order
                        .push(format!("tile {map_id} {} {}", tile.row, tile.column));
                }
            }
            Ok(())
        }
    }
    for name in fs::read_to_string(input.join("cases.txt")).unwrap().lines() {
        let folder = input.join(name);
        let plan = fs::read_to_string(folder.join("plan.txt")).unwrap();
        let mut words = plan.split_whitespace();
        fn next<T: std::str::FromStr>(w: &mut std::str::SplitWhitespace<'_>) -> T
        where
            T::Err: std::fmt::Debug,
        {
            w.next().unwrap().parse().unwrap()
        }
        let n: usize = next(&mut words);
        let _map: u32 = next(&mut words);
        let _row: u8 = next(&mut words);
        let _column: u8 = next(&mut words);
        let skip: u8 = next(&mut words);
        let build: u32 = next(&mut words);
        let big: u8 = next(&mut words);
        let slopes = recast::Slopes {
            maximum: next(&mut words),
            non_steep: next(&mut words),
        };
        let nt: usize = next(&mut words);
        let nc: usize = next(&mut words);
        assert!(n <= 64 && nt <= 65536 && nc <= 65536 && skip <= 1 && big <= 1);
        let source = forever_db2::terrain_sources::read_navigation_maps(
            &fs::read(folder.join("Map.db2")).unwrap(),
        )
        .unwrap();
        assert_eq!(source.direct_records().len(), n);
        for r in source.direct_records() {
            let id: u32 = next(&mut words);
            let parent: u32 = next(&mut words);
            let cosmetic: u32 = next(&mut words);
            let _bits: u8 = next(&mut words);
            assert_eq!(r.id, id);
            assert_eq!(i32::from(r.fields.parent_map_id) as u32, parent);
            assert_eq!(i32::from(r.fields.cosmetic_parent_map_id) as u32, cosmetic);
        }
        let liquids: Vec<_> = (0..nt)
            .map(|_| DirectRecord {
                id: next(&mut words),
                fields: forever_db2::terrain_sources::LiquidTypeFields {
                    sound_bank: next(&mut words),
                    material_id: 255,
                },
            })
            .collect();
        let types = models::liquid::Table::from_rows(&liquids, Default::default()).unwrap();
        let connections: Vec<_> = (0..nc)
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
        let b = fs::read(folder.join("options.bin")).unwrap();
        assert_eq!(b.len(), 16);
        let w: Vec<_> = b
            .as_chunks::<4>()
            .0
            .iter()
            .map(|b| u32::from_le_bytes(*b))
            .collect();
        assert_eq!(w[3], build);
        assert_eq!(w[2], 0);
        let mut package = Package::open(
            &folder.join("maps"),
            &folder.join("vmaps"),
            &source,
            build,
            super::super::Options {
                map: (w[0] != 0).then_some(w[1]),
                ..Default::default()
            },
            Default::default(),
        )
        .unwrap();
        let out = output.join(name);
        fs::create_dir(&out).unwrap();
        fs::write(out.join("INCOMPLETE_RUN"), b"provisional\n").unwrap();
        let mut sink = Files {
            root: out.clone(),
            order: Vec::new(),
        };
        let result = execute(
            &mut package,
            types.types(),
            &connections,
            Options {
                skip_liquids: skip != 0,
                big_base_unit: big != 0,
                slopes,
                selection: Selection::All,
            },
            Default::default(),
            &mut sink,
        )
        .unwrap_or_else(|e| panic!("{name}: {e:?}"));
        fs::write(out.join("order.txt"), sink.order.join("\n") + "\n").unwrap();
        fs::write(
            out.join("completed.txt"),
            format!(
                "{} {} {} {} {} {}\n",
                result.progress.maps,
                result.progress.tiles,
                result.progress.written_tiles,
                result.progress.empty_tiles,
                result.progress.output_bytes,
                result.metadata_calls
            ),
        )
        .unwrap();
        fs::remove_file(out.join("INCOMPLETE_RUN")).unwrap();
    }
}

#[test]
fn root_replacement_between_global_and_tile_permanently_poisons() {
    let tmp = Temp::new();
    tmp.map(1, &[(0, 0)]);
    let mut p = tmp.open(&[row(1, -1)]);
    let maps = tmp.maps();
    let old = tmp.0.join("old-maps");
    let mut sink = Recording {
        action: Some(Box::new(move |(_, tile)| {
            if tile.is_none() {
                fs::rename(&maps, &old).unwrap();
                fs::create_dir(&maps).unwrap();
            }
        })),
        ..Default::default()
    };
    let f = run(&mut p, Selection::All, Default::default(), &mut sink).unwrap_err();
    assert_eq!(f.code(), "navigation_discovery_input_changed");
    assert!(p.failed());
    assert_eq!(f.progress.tiles, 0);
}
#[test]
fn bound_collision_file_and_directory_identities_use_original_enumeration() {
    let tmp = Temp::new();
    tmp.map(1, &[(0, 0)]);
    fs::create_dir(tmp.vmaps().join("0001")).unwrap();
    let path = tmp.vmaps().join("foo.vmo");
    fs::write(&path, b"original").unwrap();
    let mut p = tmp.open(&[row(1, -1)]);
    let name = collision_name(store::Artifact::Model { name: b"foo" }).unwrap();
    p.budget.used = 0;
    p.guard.artifact(1, name.bytes(), &mut p.budget).unwrap();
    let missing = collision_name(store::Artifact::Tree { map_id: 1 }).unwrap();
    p.guard.artifact(1, missing.bytes(), &mut p.budget).unwrap();
    fs::write(&path, b"changed").unwrap();
    assert_eq!(
        p.guard.artifact(1, name.bytes(), &mut p.budget),
        Err(super::super::Error::InputChanged)
    );
    fs::write(tmp.vmaps().join("0001/new"), []).unwrap();
    changed_mtime(&tmp.vmaps().join("0001"));
    assert_eq!(
        p.guard.artifact(1, missing.bytes(), &mut p.budget),
        Err(super::super::Error::InputChanged)
    );
}
#[test]
fn global_offmesh_keeps_direct_order_and_unselected_maps_are_filtered() {
    let tmp = Temp::new();
    tmp.map(1, &[(0, 0)]);
    let mut p = tmp.open(&[row(1, -1)]);
    let source = offmesh::Source {
        map_id: 1,
        row: 99,
        column: 99,
        from: [1., 2., 3.],
        to: [4., 5., 6.],
        radius: f32::NAN,
        connection_flags: 255,
        area: 255,
        flags: 65535,
    };
    let mut other = source;
    other.map_id = 2;
    let sources = [source, other, source];
    let mut sink = Recording::default();
    // Flat-empty geometry does not inspect unselected malformed connections;
    // the original global codec preserves those raw source records.
    let c = execute(
        &mut p,
        models::LiquidTypes::new(&[]).unwrap(),
        &sources,
        Options::default(),
        Limits::default(),
        &mut sink,
    )
    .unwrap();
    assert_eq!(c.progress.output_bytes, 40 + 2 * 44);
    assert_eq!(
        sink.globals[0],
        mmap::encode_global(
            1,
            p.plan.map(1).unwrap().params().unwrap(),
            &sources,
            Default::default()
        )
        .unwrap()
    );
    let stats = p.statistics();
    let mut sink = Recording::default();
    let mut limits = Limits::default();
    limits.global.records = 2;
    assert!(
        execute(
            &mut p,
            models::LiquidTypes::new(&[]).unwrap(),
            &sources,
            Options::default(),
            limits,
            &mut sink
        )
        .is_err()
    );
    assert_eq!(p.statistics(), stats);
    assert!(sink.seen.is_empty());
}
#[test]
fn cumulative_output_arithmetic_checks_exact_ceiling_and_overflow() {
    let progress = Progress {
        output_bytes: 40,
        ..Default::default()
    };
    let limits = Limits {
        output_bytes: 60,
        ..Default::default()
    };
    assert_eq!(bytes(progress, 20, limits), Ok(60));
    assert_eq!(bytes(progress, 21, limits), Err(Error::Limit));
    assert_eq!(
        bytes(
            Progress {
                output_bytes: u64::MAX,
                ..Default::default()
            },
            1,
            Limits::default()
        ),
        Err(Error::Limit)
    );
}

mod world;

// Mutation fixtures require an observable directory metadata change. This does
// not strengthen production metadata-only verification into content hashing.
fn changed_mtime(path: &Path) {
    let time = fs::metadata(path).unwrap().modified().unwrap();
    fs::File::open(path)
        .unwrap()
        .set_times(fs::FileTimes::new().set_modified(time + std::time::Duration::from_secs(1)))
        .unwrap();
}
