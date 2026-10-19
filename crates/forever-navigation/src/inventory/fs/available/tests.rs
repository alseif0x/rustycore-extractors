// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-19: synthetic rooted available-selected navigation checks.
use super::*;
use crate::{inventory::fs::run, models};
use forever_db2::{
    availability::{SectionCoverage, SectionState},
    terrain_sources::{DirectRecord, NavigationMapFields, RecordCopy},
};
use forever_terrain::{
    height::{self, HeightSection},
    maps::{self, AreaSection, MapSections},
};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "rce-navigation-available-fs-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        fs::create_dir(path.join("maps")).unwrap();
        fs::create_dir(path.join("vmaps")).unwrap();
        Self(path)
    }
    fn maps(&self) -> PathBuf {
        self.0.join("maps")
    }
    fn vmaps(&self) -> PathBuf {
        self.0.join("vmaps")
    }
    fn add_list(&self, id: u32, tiles: &[(u8, u8)]) {
        fs::write(
            self.maps().join(format!("{id:04}.tilelist")),
            tilelist(tiles),
        )
        .unwrap();
    }
    fn add_vmap_dir(&self, id: u32) -> PathBuf {
        let directory = self.vmaps().join(format!("{id:04}"));
        fs::create_dir(&directory).unwrap();
        directory
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
            parent_map_id: parent,
            cosmetic_parent_map_id: -1,
            flags: 0,
        },
    }
}

fn sections(rows: usize, copies: usize) -> Vec<SectionCoverage> {
    vec![
        SectionCoverage {
            ordinal: 0,
            declared_records: rows as u32,
            declared_copies: copies as u32,
            state: SectionState::Admitted,
        },
        SectionCoverage {
            ordinal: 1,
            declared_records: 2,
            declared_copies: 1,
            state: SectionState::UnknownTact,
        },
    ]
}

fn tilelist(tiles: &[(u8, u8)]) -> Vec<u8> {
    let mut bytes = vec![b'0'; 4108];
    bytes[..4].copy_from_slice(b"MAPS");
    bytes[4..8].copy_from_slice(&10u32.to_le_bytes());
    bytes[8..12].copy_from_slice(&70450u32.to_le_bytes());
    for &(row, column) in tiles {
        bytes[12 + 4095 - (usize::from(row) * 64 + usize::from(column))] = b'1';
    }
    bytes
}

fn complete_chain_rows() -> [DirectRecord<NavigationMapFields>; 3] {
    [row(1, -1), row(2, 1), row(3, 2)]
}

fn rooted_selected(t: &Temp) -> AvailableSelectedPackage {
    t.add_list(1, &[(0, 0)]);
    t.add_list(2, &[]); // Authoritative empty terrain inventory for an ancestor.
    t.add_list(3, &[(0, 0)]);
    t.add_vmap_dir(3);
    let rows = [
        row(3, 999), // Superseded direct row must not select its invalid parent.
        row(1, -1),
        row(2, 1),
        row(4, 999), // Unavailable, unrelated parent is outside selected closure.
        row(3, 2),
    ];
    let copies = [RecordCopy {
        new_id: 4,
        source_id: 1,
    }];
    let coverage = sections(rows.len(), copies.len());
    AvailableSelectedPackage::open_rows_for_test(
        &t.maps(),
        &t.vmaps(),
        &rows,
        &copies,
        &coverage,
        3,
        Limits::default(),
    )
    .unwrap()
}

#[test]
fn selected_last_direct_wins_and_only_real_effective_ancestor_inventories_are_retained() {
    let t = Temp::new();
    let package = rooted_selected(&t);
    let plan = package.plan();
    assert_eq!(
        plan.profile(),
        inventory::SELECTED_AVAILABLE_70450_LINUX_PROFILE
    );
    assert_eq!(plan.build(), 70450);
    assert_eq!(plan.source_direct_rows(), 5);
    assert_eq!(plan.source_copy_pairs(), 1);
    assert!(!plan.all_sections_admitted());
    assert_eq!(plan.ancestors(3).unwrap().collect::<Vec<_>>(), [3, 2, 1]);
    assert_eq!(
        plan.maps()
            .iter()
            .map(|map| map.map_id())
            .collect::<Vec<_>>(),
        [1, 2, 3]
    );
    assert_eq!(
        plan.maps()
            .iter()
            .filter(|map| map.is_output())
            .map(|map| map.map_id())
            .collect::<Vec<_>>(),
        [3]
    );
    assert_eq!(plan.map(2).unwrap().tile_count(), 0);
    assert_eq!(plan.map(3).unwrap().tile_count(), 1);
    assert_eq!(
        package
            .tilelists()
            .iter()
            .map(|item| item.map_id)
            .collect::<Vec<_>>(),
        [1, 2, 3]
    );
}

#[test]
fn missing_active_ancestor_inventory_is_distinct_from_an_authoritative_empty_list() {
    let t = Temp::new();
    t.add_list(1, &[(0, 0)]);
    t.add_list(3, &[(0, 0)]);
    let rows = complete_chain_rows();
    let error = AvailableSelectedPackage::open_rows_for_test(
        &t.maps(),
        &t.vmaps(),
        &rows,
        &[],
        &sections(rows.len(), 0),
        3,
        Limits::default(),
    )
    .err()
    .unwrap();
    assert_eq!(
        error,
        Error::Inventory(inventory::Error::MissingAncestorInventory)
    );
}

#[test]
fn unknown_and_copy_only_selection_reject_before_root_access() {
    let t = Temp::new();
    let rows = [row(1, -1)];
    let coverage = sections(1, 1);
    assert_eq!(
        AvailableSelectedPackage::open_rows_for_test(
            &t.0.join("missing-maps"),
            &t.0.join("missing-vmaps"),
            &rows,
            &[],
            &coverage,
            2,
            Limits::default(),
        )
        .err()
        .unwrap(),
        Error::Inventory(inventory::Error::UnknownMap)
    );
    let copy = [RecordCopy {
        new_id: 2,
        source_id: 1,
    }];
    assert_eq!(
        AvailableSelectedPackage::open_rows_for_test(
            &t.0.join("missing-maps"),
            &t.0.join("missing-vmaps"),
            &rows,
            &copy,
            &coverage,
            2,
            Limits::default(),
        )
        .err()
        .unwrap(),
        Error::Inventory(inventory::Error::SelectedCopyOnly)
    );
}

#[test]
fn malformed_selected_list_and_symlink_tile_fail_closed() {
    let t = Temp::new();
    t.add_list(1, &[(0, 0)]);
    t.add_list(3, &[(0, 0)]);
    let rows = complete_chain_rows();
    let coverage = sections(rows.len(), 0);
    fs::write(t.maps().join("0002.tilelist"), b"truncated").unwrap();
    let error = AvailableSelectedPackage::open_rows_for_test(
        &t.maps(),
        &t.vmaps(),
        &rows,
        &[],
        &coverage,
        3,
        Limits::default(),
    )
    .err()
    .unwrap();
    assert_eq!(error, Error::Inventory(inventory::Error::TileList));

    fs::remove_file(t.maps().join("0002.tilelist")).unwrap();
    t.add_list(2, &[]);
    let vmap = t.add_vmap_dir(3);
    let target = t.0.join("target");
    fs::write(&target, []).unwrap();
    std::os::unix::fs::symlink(target, vmap.join("0003_00_00.vmtile")).unwrap();
    let error = AvailableSelectedPackage::open_rows_for_test(
        &t.maps(),
        &t.vmaps(),
        &rows,
        &[],
        &coverage,
        3,
        Limits::default(),
    )
    .err()
    .unwrap();
    assert_eq!(error, Error::Kind);
}

#[test]
fn mutation_latches_owner_and_cumulative_open_budgets_fail_closed() {
    let t = Temp::new();
    let mut package = rooted_selected(&t);
    let list_path = t.maps().join("0001.tilelist");
    let original = fs::read(&list_path).unwrap();
    fs::write(&list_path, tilelist(&[(1, 1)])).unwrap();
    assert_eq!(package.verify_unchanged(), Err(Error::InputChanged));
    assert!(package.failed());
    fs::write(&list_path, &original).unwrap();
    let after_restore = package.statistics();
    assert_eq!(package.verify_unchanged(), Err(Error::InputChanged));
    assert_eq!(package.statistics(), after_restore);

    let t = Temp::new();
    let rows = [row(1, -1)];
    let error = AvailableSelectedPackage::open_rows_for_test(
        &t.maps(),
        &t.vmaps(),
        &rows,
        &[],
        &sections(1, 0),
        1,
        Limits {
            heap_bytes: 64 * 1024,
            ..Limits::default()
        },
    )
    .err()
    .unwrap();
    assert_eq!(error, Error::Limit);
}

#[test]
fn metadata_descriptor_and_preparation_work_limits_fail_closed() {
    let t = Temp::new();
    t.add_list(1, &[(0, 0)]);
    let rows = [row(1, -1)];
    let cases = [
        (
            Limits {
                metadata_calls: 0,
                ..Limits::default()
            },
            Error::Limit,
        ),
        (
            Limits {
                descriptors: 0,
                ..Limits::default()
            },
            Error::Limit,
        ),
        (
            Limits {
                plan: inventory::Limits {
                    work: 0,
                    ..inventory::Limits::default()
                },
                ..Limits::default()
            },
            Error::Inventory(inventory::Error::Limit),
        ),
    ];
    for (limits, expected) in cases {
        let error = AvailableSelectedPackage::open_rows_for_test(
            &t.maps(),
            &t.vmaps(),
            &rows,
            &[],
            &sections(rows.len(), 0),
            1,
            limits,
        )
        .err()
        .unwrap();
        assert_eq!(error, expected);
    }
}

#[derive(Default)]
struct Recording {
    events: usize,
    fail_at: Option<usize>,
    on_tile: Option<Box<dyn FnMut()>>,
    tile_work: Vec<(bool, u64, u64)>,
}
impl run::Sink for Recording {
    type Error = &'static str;
    fn emit(&mut self, event: run::Event<'_>) -> Result<(), Self::Error> {
        self.events += 1;
        if let run::Event::Tile { outcome, .. } = event {
            let (nonempty, builder_work, save_work) = match outcome {
                crate::pipeline::Outcome::Empty { statistics, .. } => {
                    (false, statistics.builder.work, statistics.save.work)
                }
                crate::pipeline::Outcome::Tile { statistics, .. } => {
                    (true, statistics.builder.work, statistics.save.work)
                }
            };
            self.tile_work.push((nonempty, builder_work, save_work));
            if let Some(action) = &mut self.on_tile {
                action();
            }
        }
        if self.fail_at == Some(self.events) {
            Err("sink-failed")
        } else {
            Ok(())
        }
    }
}

fn runtime_package(t: &Temp, tile_columns: &[u8]) -> AvailableSelectedPackage {
    runtime_package_with_kind(t, tile_columns, 0)
}

fn runtime_package_with_kind(
    t: &Temp,
    tile_columns: &[u8],
    terrain_kind: u8,
) -> AvailableSelectedPackage {
    t.add_list(1, &[(0, 0)]);
    t.add_list(
        3,
        &tile_columns
            .iter()
            .map(|&column| (0, column))
            .collect::<Vec<_>>(),
    );
    let rows = [row(1, -1), row(3, 1)];
    let terrain = available_map_fixture_70450(terrain_kind);
    for &column in tile_columns {
        fs::write(t.maps().join(format!("0003_00_{column:02}.map")), &terrain).unwrap();
    }
    AvailableSelectedPackage::open_rows_for_test(
        &t.maps(),
        &t.vmaps(),
        &rows,
        &[],
        &sections(rows.len(), 0),
        3,
        Limits::default(),
    )
    .unwrap()
}

fn available_map_fixture_70450(kind: u8) -> Vec<u8> {
    let scale = match kind {
        0 => 0.0,
        1 => 0.0005,
        _ => unreachable!("available fixture kind must be flat or patterned"),
    };
    let v9: Vec<_> = (0..height::V9_COUNT)
        .map(|i| {
            let (row, column) = (i / 129, i % 129);
            let value = if kind == 1 && (44..=84).contains(&row) && (44..=84).contains(&column) {
                2.0
            } else {
                2.0 + (i % 213) as f32 * scale
            };
            value.to_bits()
        })
        .collect();
    let v8: Vec<_> = (0..height::V8_COUNT)
        .map(|i| {
            let (row, column) = (i / 128, i % 128);
            let value = if kind == 1 && (44..84).contains(&row) && (44..84).contains(&column) {
                2.0
            } else {
                2.0 + (i % 119) as f32 * scale
            };
            value.to_bits()
        })
        .collect();
    let height = HeightSection::from_bits(
        &v9,
        &v8,
        height::Options {
            pack_integers: true,
        },
        None,
        height::MAX_SECTION_BYTES,
    )
    .unwrap();
    let area = AreaSection::from_ids(&[42; 256]);
    let mut holes = [0xff; maps::HOLE_BYTES];
    // Leave one broad flat patch visible; all other MAPS terrain squares are holes.
    for row in 44..84 {
        for column in 44..84 {
            let index = ((row / 8) * 16 + column / 8) * 8 + row % 8;
            holes[index] &= !(1 << (column % 8));
        }
    }
    maps::assemble_70450(
        70450,
        MapSections {
            area: &area,
            height: &height,
            liquid: None,
            holes: Some(&holes),
        },
        maps::MAX_MAP_BYTES,
    )
    .unwrap()
}

fn run_selected_with_limits(
    package: &mut AvailableSelectedPackage,
    selection: run::Selection,
    limits: run::AvailableLimits,
    sink: &mut Recording,
) -> Result<run::AvailableCompleted, run::Failure<&'static str>> {
    run::execute_available_selected(
        package,
        models::LiquidTypes::new(&[]).unwrap(),
        &[],
        run::Options {
            selection,
            ..Default::default()
        },
        limits,
        sink,
    )
}

fn run_selected(
    package: &mut AvailableSelectedPackage,
    selection: run::Selection,
    sink: &mut Recording,
) -> Result<run::AvailableCompleted, run::Failure<&'static str>> {
    run_selected_with_limits(package, selection, run::AvailableLimits::default(), sink)
}

#[test]
fn selected_runner_rejects_all_and_invalid_map_before_verification_or_sink() {
    let t = Temp::new();
    let mut package = runtime_package(&t, &[0]);
    let before = package.statistics();
    for selection in [
        run::Selection::All,
        run::Selection::Map(1),
        run::Selection::Map(99),
    ] {
        let mut sink = Recording::default();
        let failure = run_selected(&mut package, selection, &mut sink).unwrap_err();
        assert!(matches!(
            failure.cause,
            run::Cause::Run(run::Error::Selection)
        ));
        assert_eq!(failure.progress, run::Progress::default());
        assert_eq!(package.statistics(), before);
        assert_eq!(sink.events, 0);
    }
}

#[test]
fn selected_runner_rejects_available_map_and_tile_caps_before_io_or_latch() {
    let t = Temp::new();
    let mut package = runtime_package(&t, &[0]);
    let defaults = run::AvailableLimits::default();
    let cases = [
        run::Limits {
            maps: 2,
            ..defaults.run
        },
        run::Limits {
            tiles: 4097,
            ..defaults.run
        },
    ];
    for run_limits in cases {
        let before = package.statistics();
        let mut sink = Recording::default();
        let failure = run_selected_with_limits(
            &mut package,
            run::Selection::Map(3),
            run::AvailableLimits {
                run: run_limits,
                ..defaults
            },
            &mut sink,
        )
        .unwrap_err();
        assert!(matches!(failure.cause, run::Cause::Run(run::Error::Limit)));
        assert_eq!(failure.progress, run::Progress::default());
        assert_eq!(package.statistics(), before);
        assert_eq!(sink.events, 0);
        assert!(!package.failed());
    }

    let mut sink = Recording::default();
    let completed = run_selected(&mut package, run::Selection::Map(3), &mut sink).unwrap();
    assert_eq!(completed.completion.progress.maps, 1);
    assert_eq!(completed.completion.progress.tiles, 1);
    assert_eq!(sink.events, 2);
    assert!(!package.failed());
}

#[test]
fn selected_runner_applies_cumulative_output_limit_before_emission() {
    let t = Temp::new();
    let mut package = runtime_package(&t, &[0]);
    let before = package.statistics();
    let mut sink = Recording::default();
    let failure = run::execute_available_selected(
        &mut package,
        models::LiquidTypes::new(&[]).unwrap(),
        &[],
        run::Options {
            selection: run::Selection::Map(3),
            ..Default::default()
        },
        run::AvailableLimits {
            run: run::Limits {
                output_bytes: 0,
                ..run::AvailableLimits::default().run
            },
            ..Default::default()
        },
        &mut sink,
    )
    .unwrap_err();
    assert!(matches!(failure.cause, run::Cause::Run(run::Error::Limit)));
    assert_eq!(sink.events, 0);
    assert_ne!(package.statistics(), before);
    assert!(package.failed());
}

#[test]
fn selected_runner_cumulates_two_tile_outputs_and_keeps_sink_callbacks_provisional() {
    let t = Temp::new();
    let mut package = runtime_package(&t, &[0, 1]);
    let mut sink = Recording::default();
    let completed = run_selected(&mut package, run::Selection::Map(3), &mut sink).unwrap();
    assert_eq!(completed.completion.progress.maps, 1);
    assert_eq!(completed.completion.progress.tiles, 2);
    assert_eq!(sink.events, 3); // One global and two tile callbacks.

    let t = Temp::new();
    let mut package = runtime_package(&t, &[0]);
    let mut sink = Recording {
        fail_at: Some(1),
        ..Default::default()
    };
    let failure = run_selected(&mut package, run::Selection::Map(3), &mut sink).unwrap_err();
    assert!(matches!(failure.cause, run::Cause::Sink("sink-failed")));
    assert_eq!(failure.progress, run::Progress::default());
    assert_eq!(sink.events, 1);
    assert!(package.failed());

    let mut retry = Recording::default();
    let failure = run_selected(&mut package, run::Selection::Map(3), &mut retry).unwrap_err();
    assert!(matches!(
        failure.cause,
        run::Cause::Run(run::Error::Input(Error::Failed))
    ));
    assert_eq!(retry.events, 0);
}

#[test]
fn selected_runner_debits_builder_and_save_work_across_nonempty_tiles() {
    let t = Temp::new();
    let mut measured_package = runtime_package_with_kind(&t, &[0, 1], 1);
    let mut measured_sink = Recording::default();
    let measured = run_selected_with_limits(
        &mut measured_package,
        run::Selection::Map(3),
        run::AvailableLimits::default(),
        &mut measured_sink,
    )
    .unwrap();
    assert_eq!(measured.completion.progress.tiles, 2);
    assert_eq!(measured_sink.tile_work.len(), 2);
    assert!(
        measured_sink
            .tile_work
            .iter()
            .all(|(nonempty, builder, save)| *nonempty && *builder > 0 && *save > 0)
    );
    let builder_total = measured_sink
        .tile_work
        .iter()
        .fold(0u64, |total, (_, work, _)| {
            total.checked_add(*work).unwrap()
        });
    let save_total = measured_sink
        .tile_work
        .iter()
        .fold(0u64, |total, (_, _, work)| {
            total.checked_add(*work).unwrap()
        });
    assert_eq!(measured.builder_work, builder_total);
    assert_eq!(measured.save_work, save_total);

    let exact_limits = run::AvailableLimits {
        builder_work: builder_total,
        save_work: save_total,
        ..Default::default()
    };
    let t = Temp::new();
    let mut exact_package = runtime_package_with_kind(&t, &[0, 1], 1);
    let mut exact_sink = Recording::default();
    let exact = run_selected_with_limits(
        &mut exact_package,
        run::Selection::Map(3),
        exact_limits,
        &mut exact_sink,
    )
    .unwrap();
    assert_eq!(exact.completion.progress.tiles, 2);
    assert_eq!(exact.builder_work, builder_total);
    assert_eq!(exact.save_work, save_total);
    assert_eq!(exact_sink.tile_work, measured_sink.tile_work);

    let first_builder_work = measured_sink.tile_work[0].1;
    let builder_limits = run::AvailableLimits {
        builder_work: first_builder_work,
        save_work: save_total,
        ..Default::default()
    };
    let t = Temp::new();
    let mut builder_package = runtime_package_with_kind(&t, &[0, 1], 1);
    let mut builder_sink = Recording::default();
    let builder_failure = run_selected_with_limits(
        &mut builder_package,
        run::Selection::Map(3),
        builder_limits,
        &mut builder_sink,
    )
    .unwrap_err();
    let builder_error = match &builder_failure.cause {
        run::Cause::Run(run::Error::Input(Error::Pipeline(error))) => *error,
        other => panic!("unexpected builder budget failure cause: {other:?}"),
    };
    assert!(
        matches!(
            builder_error,
            crate::pipeline::Error::Builder(
                crate::tilebuilder::Error::Limit
                    | crate::tilebuilder::Error::Subtile {
                        source: crate::recast::Error::Limit,
                        ..
                    }
            )
        ),
        "unexpected builder budget error: {builder_error:?}"
    );
    assert_eq!(builder_failure.progress.tiles, 1);
    assert_eq!(builder_failure.progress.written_tiles, 1);
    assert_eq!(builder_sink.events, 2); // Global and first nonempty tile only.
    assert_eq!(builder_sink.tile_work.len(), 1);
    assert_eq!(builder_sink.tile_work[0], measured_sink.tile_work[0]);
    assert!(builder_package.failed());
    let builder_after_failure = builder_package.statistics();
    let mut builder_retry_sink = Recording::default();
    let builder_retry = run_selected_with_limits(
        &mut builder_package,
        run::Selection::Map(3),
        builder_limits,
        &mut builder_retry_sink,
    )
    .unwrap_err();
    let builder_retry_error = match &builder_retry.cause {
        run::Cause::Run(run::Error::Input(Error::Pipeline(error))) => *error,
        other => panic!("unexpected latched builder retry cause: {other:?}"),
    };
    assert_eq!(
        builder_retry_error, builder_error,
        "latched builder retry must preserve the original typed cause"
    );
    assert_eq!(builder_package.statistics(), builder_after_failure);
    assert_eq!(builder_retry_sink.events, 0);

    let first_save_work = measured_sink.tile_work[0].2;
    let save_limits = run::AvailableLimits {
        builder_work: builder_total,
        save_work: first_save_work,
        ..Default::default()
    };
    let t = Temp::new();
    let mut save_package = runtime_package_with_kind(&t, &[0, 1], 1);
    let mut save_sink = Recording::default();
    let save_failure = run_selected_with_limits(
        &mut save_package,
        run::Selection::Map(3),
        save_limits,
        &mut save_sink,
    )
    .unwrap_err();
    let save_error = match &save_failure.cause {
        run::Cause::Run(run::Error::Input(Error::Pipeline(error))) => *error,
        other => panic!("unexpected save budget failure cause: {other:?}"),
    };
    assert_eq!(
        save_error,
        crate::pipeline::Error::Save(crate::detour::SaveError::Limit),
        "unexpected save budget error"
    );
    assert_eq!(save_failure.progress.tiles, 1);
    assert_eq!(save_failure.progress.written_tiles, 1);
    assert_eq!(save_sink.events, 2); // Second tile fails before its callback.
    assert_eq!(save_sink.tile_work.len(), 1);
    assert_eq!(save_sink.tile_work[0], measured_sink.tile_work[0]);
    assert!(save_package.failed());
    let save_after_failure = save_package.statistics();
    let mut save_retry_sink = Recording::default();
    let save_retry = run_selected_with_limits(
        &mut save_package,
        run::Selection::Map(3),
        save_limits,
        &mut save_retry_sink,
    )
    .unwrap_err();
    let save_retry_error = match &save_retry.cause {
        run::Cause::Run(run::Error::Input(Error::Pipeline(error))) => *error,
        other => panic!("unexpected latched save retry cause: {other:?}"),
    };
    assert_eq!(
        save_retry_error, save_error,
        "latched save retry must preserve the original typed cause"
    );
    assert_eq!(save_package.statistics(), save_after_failure);
    assert_eq!(save_retry_sink.events, 0);
}

#[test]
fn selected_runner_reserves_retained_owner_heap_at_the_exact_boundary() {
    let t = Temp::new();
    let mut package = runtime_package(&t, &[0]);
    let retained = package.retained_heap_bytes().unwrap();
    assert!(retained > 0);
    let before = package.statistics();
    let mut below_sink = Recording::default();
    let below = run_selected_with_limits(
        &mut package,
        run::Selection::Map(3),
        run::AvailableLimits {
            heap_bytes: retained - 1,
            ..Default::default()
        },
        &mut below_sink,
    )
    .unwrap_err();
    assert!(matches!(below.cause, run::Cause::Run(run::Error::Limit)));
    assert_eq!(package.statistics(), before);
    assert_eq!(below_sink.events, 0);
    assert!(!package.failed());

    let mut boundary_sink = Recording::default();
    let boundary = run_selected_with_limits(
        &mut package,
        run::Selection::Map(3),
        run::AvailableLimits {
            heap_bytes: retained,
            ..Default::default()
        },
        &mut boundary_sink,
    );
    assert!(boundary_sink.events >= 1); // Equality admits owner accounting and global output.
    assert!(boundary.is_ok() || package.failed());
}

#[test]
fn final_verification_failure_invalidates_callbacks_and_latches_owner() {
    let t = Temp::new();
    let mut package = runtime_package(&t, &[0]);
    let list_path = t.maps().join("0003.tilelist");
    let original = fs::read(&list_path).unwrap();
    let mut sink = Recording {
        on_tile: Some(Box::new(move || {
            fs::write(&list_path, tilelist(&[])).unwrap();
            fs::write(&list_path, &original).unwrap();
        })),
        ..Default::default()
    };
    let failure = run_selected(&mut package, run::Selection::Map(3), &mut sink).unwrap_err();
    assert!(matches!(
        failure.cause,
        run::Cause::Run(run::Error::Input(Error::InputChanged))
    ));
    assert_eq!(failure.progress.maps, 1);
    assert_eq!(failure.progress.tiles, 1);
    assert!(package.failed());
    let before_retry = package.statistics();
    let mut retry = Recording::default();
    let retry_failure = run_selected(&mut package, run::Selection::Map(3), &mut retry).unwrap_err();
    assert!(matches!(
        retry_failure.cause,
        run::Cause::Run(run::Error::Input(Error::InputChanged))
    ));
    assert_eq!(package.statistics(), before_retry);
    assert_eq!(retry.events, 0);
}

#[test]
fn complete_filesystem_package_api_remains_separate_and_available() {
    let t = Temp::new();
    t.add_list(1, &[(0, 0)]);
    t.add_list(2, &[]);
    t.add_list(3, &[(0, 0)]);
    let rows = complete_chain_rows();
    let complete = super::super::Package::open_rows(
        &t.maps(),
        &t.vmaps(),
        &rows,
        70450,
        Default::default(),
        Default::default(),
    )
    .unwrap();
    assert_eq!(complete.plan().build(), 70450);
    assert_eq!(complete.plan().map(3).unwrap().tile_count(), 1);
}
