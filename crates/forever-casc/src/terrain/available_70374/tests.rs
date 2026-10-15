use super::*;
use crate::local_store::tests::{CollisionFile, CollisionFixture};
use crate::terrain::run::tests::fixtures;
use crate::{Locale, local_index, root};
use fixtures::*;
use run::{MAP_ID, MATERIAL_ID, OBJECT_ID, TYPE_ID};

fn requests() -> [plan::TileRequest; 2] {
    [
        plan::TileRequest {
            map_id: 7,
            x: 3,
            y: 2,
        },
        plan::TileRequest {
            map_id: 7,
            x: 1,
            y: 4,
        },
    ]
}
fn files() -> Vec<CollisionFile<'static>> {
    let mut f = basic();
    f[5].2 = liquid_adt();
    for (id, _, bytes) in &mut f {
        let name = match *id {
            MAP_ID => Some("Map"),
            MATERIAL_ID => Some("LiquidMaterial"),
            OBJECT_ID => Some("LiquidObject"),
            TYPE_ID => Some("LiquidType"),
            _ => None,
        };
        if let Some(name) = name {
            let h = catalog::resolve_for(catalog::WOWDBDEFS_70374_CATALOG_ID, name)
                .unwrap()
                .unwrap()
                .metadata()
                .table_hash
                .unwrap();
            put(bytes, 152, h);
        }
    }
    f
}
fn open(f: &CollisionFixture) -> LocalStore {
    let path = f.path().join(".build.info");
    let text = std::fs::read_to_string(&path).unwrap();
    std::fs::write(&path, text.replace("1.60.1.70205", "1.60.1.70374")).unwrap();
    LocalStore::open_db2_70374(
        f.path(),
        local_store::OpenOptions {
            locale: Locale::EnUs,
            index_layouts: [local_index::Layout::Contiguous; 16],
            root_layout: root::Layout::Header50893V2,
            limits: local_store::Limits::default(),
        },
    )
    .unwrap()
}
#[derive(Default)]
struct Stage<'a> {
    maps: Vec<MapFacts>,
    sources: Vec<run::SourceFacts>,
    tiles: Vec<Vec<u8>>,
    witnesses: usize,
    lists: Vec<(u32, Vec<u8>)>,
    fail_list: bool,
    change_on_list: Option<&'a CollisionFixture>,
    fail: bool,
    change: Option<&'a CollisionFixture>,
}
impl Sink for Stage<'_> {
    fn map_source(&mut self, f: MapFacts) -> Result<(), run::SinkError> {
        self.maps.push(f);
        Ok(())
    }
    fn source(&mut self, f: run::SourceFacts) -> Result<(), run::SinkError> {
        self.sources.push(f);
        Ok(())
    }
    fn native_witness(&mut self, w: NativeWitness<'_>) -> Result<(), run::SinkError> {
        assert!(!w.adt.is_empty());
        assert_eq!(w.context.build, 70374);
        assert_eq!(w.source.bytes, w.adt.len());
        assert!(!w.materials.is_empty());
        assert!(!w.types.is_empty());
        self.witnesses += 1;
        Ok(())
    }
    fn tile_list(&mut self, id: u32, list: &tilelist::TileList) -> Result<(), run::SinkError> {
        if self.fail_list {
            return Err(run::SinkError);
        }
        self.lists.push((id, list.bytes().to_vec()));
        if let Some(f) = self.change_on_list {
            f.change_metadata();
        }
        Ok(())
    }
    fn tile(
        &mut self,
        c: conversion::Context,
        _: run::SourceFacts,
        t: &conversion::ConvertedTile,
    ) -> Result<(), run::SinkError> {
        assert_eq!(c.build, 70374);
        if self.fail {
            return Err(run::SinkError);
        }
        self.tiles.push(t.bytes().to_vec());
        if let Some(f) = self.change {
            f.change_metadata();
        }
        Ok(())
    }
}
fn execute(
    f: &[CollisionFile<'_>],
    q: &[plan::TileRequest],
    l: Limits,
) -> (Result<CompletedSelected, Failure>, Stage<'static>) {
    let fixture = CollisionFixture::new(f);
    let mut s = open(&fixture);
    let mut out = Stage::default();
    (run_selected(&mut s, q, true, l, &mut out), out)
}
fn unknown(mut b: Vec<u8>, copies: u32) -> Vec<u8> {
    // Authored second section. Shift metadata, preserve complete logical string
    // addresses including the omitted record slot, then add unknown TACT data.
    let old_start = u32::from_le_bytes(b[212..216].try_into().unwrap()) as usize;
    let record_size = u32::from_le_bytes(b[144..148].try_into().unwrap()) as usize;
    for at in [old_start, old_start + 4] {
        let n = u32::from_le_bytes(b[at..at + 4].try_into().unwrap());
        put(&mut b, at, n + record_size as u32);
    }
    let mut out = b[..244].to_vec();
    out.extend([0; 40]);
    out.extend(&b[244..old_start]);
    out.extend(0u32.to_le_bytes()); // encrypted-ID-list count, metadata before data
    out.extend(&b[old_start..]);
    put(&mut out, 136, 2);
    put(&mut out, 200, 2);
    put(&mut out, 212, (old_start + 44) as u32);
    let at = out.len() as u32;
    out[244..252].copy_from_slice(&1u64.to_le_bytes());
    put(&mut out, 252, at);
    put(&mut out, 256, 1);
    put(&mut out, 268, 4);
    put(&mut out, 280, copies);
    out.extend(vec![0xa5; record_size]);
    out.extend(99u32.to_le_bytes());
    for _ in 0..copies {
        out.extend([0xa5; 8]);
    }
    out
}
#[test]
fn selected_complete_sources_current_headers_and_fixed_order() {
    let (r, s) = execute(&files(), &requests(), Limits::default());
    let done = r.unwrap();
    assert_eq!(done.stats().read_attempts, 7);
    assert_eq!(done.stats().converted_tiles, 2);
    assert_eq!(done.stats().tile_lists, 0);
    assert_eq!(done.plan().absent_tiles, 0);
    assert_eq!(done.map().direct_rows, 1);
    assert!(done.map().full_ckey_verified);
    assert!(!done.map().encoded_key_authenticated);
    assert_eq!(
        s.sources.iter().map(|f| f.file_data_id).collect::<Vec<_>>(),
        [MATERIAL_ID, OBJECT_ID, TYPE_ID, 100, 201, 202]
    );
    assert_eq!(s.maps.len(), 1);
    assert_eq!(s.tiles.len(), 2);
    assert_eq!(s.witnesses, 2);
    assert!(s.lists.is_empty());
    for t in s.tiles {
        forever_terrain::maps::verify_70374(&t, 70374, 202112).unwrap();
        assert!(forever_terrain::maps::verify(&t, 70374, 202112).is_err());
    }
}
#[test]
fn unrelated_unknown_section_does_not_claim_complete_map() {
    let mut f = files();
    f[0].2 = unknown(f[0].2.clone(), 1);
    let (r, s) = execute(&f, &requests(), Limits::default());
    let done = r.unwrap();
    assert_eq!(s.tiles.len(), 2);
    assert_eq!(done.map().direct_rows, 1);
    assert_eq!(done.map().materialized_rows, 1);
    assert_eq!(
        done.map().sections[1].unwrap().state,
        db2::SectionState::UnknownTact
    );
    assert_eq!(done.map().sections[1].unwrap().declared_records, 1);
}
#[test]
fn omitted_declared_rows_and_copies_count_toward_downward_limits() {
    let mut f = files();
    f[0].2 = unknown(f[0].2.clone(), 2);
    for which in [0, 1] {
        let mut l = Limits::default();
        if which == 0 {
            l.resources.max_direct_rows = 1;
        } else {
            l.resources.max_copy_records = 1;
        }
        let (r, s) = execute(&f, &requests(), l);
        assert_eq!(r.err().unwrap().error, limit());
        assert!(s.tiles.is_empty());
    }
}
#[test]
fn absent_coordinate_rejects_before_first_adt() {
    let mut q = requests();
    q[1].x = 0;
    q[1].y = 0;
    let (r, s) = execute(&files(), &q, Limits::default());
    let fail = r.err().unwrap();
    assert_eq!(fail.error, Error::AbsentTile);
    assert_eq!(fail.stats.read_attempts, 5);
    assert!(s.tiles.is_empty());
}
#[test]
fn names_and_zero_or_reserved_root_ids_do_not_fall_back() {
    for id in [0, u32::MAX] {
        let mut f = files();
        f[4].2 = wdt(&[(3, 2, id), (1, 4, 202)], true);
        let (r, s) = execute(&f, &requests(), Limits::default());
        assert_eq!(r.err().unwrap().error, Error::FileIdRouting);
        assert!(s.tiles.is_empty());
    }
    let mut f = files();
    f[4].2 = wdt(&[(3, 2, 201), (1, 4, 202)], false);
    assert_eq!(
        execute(&f, &requests(), Limits::default())
            .0
            .err()
            .unwrap()
            .error,
        Error::FileIdRouting
    );
}
#[test]
fn missing_and_ambiguous_selected_map_fail_without_wdt() {
    for duplicates in [false, true] {
        let mut f = files();
        f[0].2 = map_db2(
            if duplicates {
                &[(7, 100, -1), (7, 100, -1)]
            } else {
                &[(8, 100, -1)]
            },
            &[],
        );
        put(
            &mut f[0].2,
            152,
            catalog::resolve_for(catalog::WOWDBDEFS_70374_CATALOG_ID, "Map")
                .unwrap()
                .unwrap()
                .metadata()
                .table_hash
                .unwrap(),
        );
        let (r, s) = execute(&f, &requests(), Limits::default());
        assert!(matches!(
            r.err().unwrap().error,
            Error::Run(run::Error::Plan(
                plan::Error::MissingMap | plan::Error::AmbiguousMap
            ))
        ));
        assert!(s.tiles.is_empty());
    }
}
#[test]
fn whole_selection_fails_after_second_adt_failure() {
    let mut f = files();
    f[6].2 = vec![0; 12];
    let (r, s) = execute(&f, &requests(), Limits::default());
    assert!(r.is_err());
    assert_eq!(s.tiles.len(), 1);
    assert_eq!(s.witnesses, 1); // provisional only, no Completed
}
#[test]
fn current_liquid_table_hash_mismatch_and_truncated_wdt_are_fatal() {
    let mut f = files();
    f[1].2[152] ^= 1;
    assert_eq!(
        execute(&f, &requests(), Limits::default())
            .0
            .err()
            .unwrap()
            .error,
        Error::Catalog
    );
    let mut f = files();
    f[4].2.truncate(11);
    assert!(execute(&f, &requests(), Limits::default()).0.is_err());
}
#[test]
fn downward_memory_and_read_output_limits_reject_whole_selection() {
    for which in 0..4 {
        let mut l = Limits::default();
        match which {
            0 => l.resources.max_memory_bytes = 0,
            1 => l.resources.max_reads = 4,
            2 => l.resources.max_source_bytes = 1,
            _ => l.resources.max_output_bytes = 0,
        }
        let (r, s) = execute(&files(), &requests(), l);
        assert!(r.is_err());
        assert!(s.tiles.is_empty());
    }
}
#[test]
fn malformed_requests_and_older_store_reject_before_map() {
    let mut q = requests();
    q[1] = q[0];
    let (r, s) = execute(&files(), &q, Limits::default());
    assert_eq!(r.err().unwrap().stats.read_attempts, 0);
    assert!(s.maps.is_empty());
    let f = CollisionFixture::new(&files());
    let mut s = f.open(false);
    let mut stage = Stage::default();
    let r = run_selected(&mut s, &requests(), true, Limits::default(), &mut stage);
    assert_eq!(r.err().unwrap().error, Error::Build);
}
#[test]
fn source_mutation_and_sink_errors_cannot_return_completion() {
    for change in [false, true] {
        let f = CollisionFixture::new(&files());
        let mut store = open(&f);
        let mut s = Stage {
            fail: !change,
            change: change.then_some(&f),
            ..Default::default()
        };
        assert!(run_selected(&mut store, &requests(), true, Limits::default(), &mut s).is_err());
    }
}

fn full_execute(
    f: &[CollisionFile<'_>],
    selection: plan::Selection<'_>,
    limits: FullMapLimits,
) -> (Result<CompletedFullMaps, Failure>, Stage<'static>) {
    let fixture = CollisionFixture::new(f);
    let mut store = open(&fixture);
    let mut sink = Stage::default();
    (
        run_full_maps(&mut store, selection, true, limits, &mut sink),
        sink,
    )
}
fn replace_maps(f: &mut [CollisionFile<'_>], rows: &[(u32, u32, i16)], copies: &[(u32, u32)]) {
    f[0].2 = map_db2(rows, copies);
    put(
        &mut f[0].2,
        152,
        catalog::resolve_for(catalog::WOWDBDEFS_70374_CATALOG_ID, "Map")
            .unwrap()
            .unwrap()
            .metadata()
            .table_hash
            .unwrap(),
    );
}
#[test]
fn full_maps_mixed_present_absent_exact_list_and_70374_outputs() {
    let (r, s) = full_execute(
        &files(),
        plan::Selection::MapIds(&[7]),
        FullMapLimits::default(),
    );
    let done = r.unwrap();
    assert_eq!(done.profile(), FULL_MAP_PROFILE);
    assert_eq!(done.plan().scope, plan::Scope::MapIds);
    assert_eq!(
        (
            done.plan().maps,
            done.plan().selected_coordinates,
            done.plan().present_tiles,
            done.plan().absent_tiles
        ),
        (1, 4096, 2, 4094)
    );
    assert_eq!(
        (
            done.stats().read_attempts,
            done.stats().converted_tiles,
            done.stats().tile_lists
        ),
        (7, 2, 1)
    );
    assert_eq!(s.lists.len(), 1);
    let (id, list) = &s.lists[0];
    assert_eq!(*id, 7);
    for y in 0..64 {
        for x in 0..64 {
            assert_eq!(
                tilelist::present_70374(list, 70374, x, y).unwrap(),
                [(3, 2), (1, 4)].contains(&(x, y))
            );
        }
    }
    for t in &s.tiles {
        forever_terrain::maps::verify_70374(t, 70374, 202112).unwrap();
    }
    assert_eq!(
        done.stats().output_bytes,
        s.tiles.iter().map(|t| t.len() as u64).sum::<u64>() + tilelist::BYTES as u64
    );
}
#[test]
fn full_maps_multiple_maps_startup_once_and_materialized_order() {
    let mut f = files();
    replace_maps(&mut f, &[(9, 100, -1), (7, 101, -1)], &[]);
    f.push((101, None, wdt(&[(0, 0, 201)], true)));
    let (r, s) = full_execute(
        &f,
        plan::Selection::MapIds(&[7, 9]),
        FullMapLimits::default(),
    );
    let done = r.unwrap();
    assert_eq!(
        (
            done.plan().maps,
            done.plan().selected_coordinates,
            done.stats().read_attempts
        ),
        (2, 8192, 9)
    );
    assert_eq!(
        s.lists.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
        [9, 7]
    );
    assert_eq!(s.maps.len(), 1);
    for id in [MATERIAL_ID, OBJECT_ID, TYPE_ID] {
        assert_eq!(s.sources.iter().filter(|v| v.file_data_id == id).count(), 1);
    }
}
#[test]
fn full_maps_empty_wdt_emits_one_zero_list() {
    let mut f = files();
    f[4].2 = wdt(&[], true);
    let (r, s) = full_execute(&f, plan::Selection::AllMaps, FullMapLimits::default());
    let done = r.unwrap();
    assert_eq!(done.plan().absent_tiles, 4096);
    assert_eq!(
        (
            done.stats().converted_tiles,
            done.stats().tile_lists,
            done.stats().read_attempts
        ),
        (0, 1, 5)
    );
    assert!(s.tiles.is_empty());
    assert!(s.lists[0].1[12..].iter().all(|b| *b == b'0'));
}
#[test]
fn full_maps_empty_admitted_domain_is_not_empty_world_claim() {
    let mut f = files();
    replace_maps(&mut f, &[], &[]);
    let (r, s) = full_execute(&f, plan::Selection::AllMaps, FullMapLimits::default());
    let done = r.unwrap();
    assert_eq!(done.plan().scope, plan::Scope::AllMaps);
    assert_eq!(
        (
            done.plan().maps,
            done.plan().selected_coordinates,
            done.stats().read_attempts
        ),
        (0, 0, 4)
    );
    assert!(s.tiles.is_empty() && s.lists.is_empty());
    assert_eq!(s.maps.len(), 1);
}
#[test]
fn full_maps_unknown_sections_remain_disclosed() {
    let mut f = files();
    f[0].2 = unknown(f[0].2.clone(), 1);
    let (r, s) = full_execute(&f, plan::Selection::AllMaps, FullMapLimits::default());
    let done = r.unwrap();
    assert_eq!(
        done.map().sections[1].unwrap().state,
        db2::SectionState::UnknownTact
    );
    assert_eq!(done.map().sections[1].unwrap().declared_records, 1);
    assert_eq!((done.map().materialized_rows, s.lists.len()), (1, 1));
}
#[test]
fn full_maps_copy_replay_order_and_zero_wdt_filter() {
    let mut f = files();
    replace_maps(&mut f, &[(9, 100, -1), (7, 0, -1)], &[(11, 9)]);
    let (r, s) = full_execute(&f, plan::Selection::AllMaps, FullMapLimits::default());
    let done = r.unwrap();
    assert_eq!(done.map().direct_rows, 2);
    assert_eq!(done.map().copy_records, 1);
    assert_eq!(done.map().materialized_rows, 2);
    assert_eq!(
        s.lists.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
        [9, 11]
    );
    assert_eq!(
        (done.stats().converted_tiles, done.stats().read_attempts),
        (4, 10)
    );
}
#[test]
fn full_maps_missing_duplicate_ambiguous_and_empty_explicit_ids_reject() {
    for ids in [&[][..], &[99][..], &[7, 7][..]] {
        let (r, s) = full_execute(
            &files(),
            plan::Selection::MapIds(ids),
            FullMapLimits::default(),
        );
        assert!(r.is_err());
        assert!(s.tiles.is_empty() && s.lists.is_empty());
    }
    let mut f = files();
    replace_maps(&mut f, &[(7, 100, -1), (7, 100, -1)], &[]);
    for selection in [plan::Selection::MapIds(&[7]), plan::Selection::AllMaps] {
        let (r, s) = full_execute(&f, selection, FullMapLimits::default());
        assert_eq!(
            r.err().unwrap().error,
            plan_error(plan::Error::AmbiguousMap)
        );
        assert!(s.lists.is_empty());
    }
}
#[test]
fn full_maps_rejects_tile_selection_before_acquisition() {
    let (r, s) = full_execute(
        &files(),
        plan::Selection::Tiles(&requests()),
        FullMapLimits::default(),
    );
    let failure = r.err().unwrap();
    assert_eq!(failure.error, Error::FullMapSelection);
    assert_eq!(failure.stats.read_attempts, 0);
    assert!(s.maps.is_empty());
}
#[test]
fn full_maps_preflights_present_root_ids_and_fd_id_policy() {
    for id in [0, u32::MAX] {
        let mut f = files();
        f[4].2 = wdt(&[(3, 2, 201), (1, 4, id)], true);
        let (r, s) = full_execute(&f, plan::Selection::AllMaps, FullMapLimits::default());
        let failure = r.err().unwrap();
        assert_eq!(failure.error, Error::FileIdRouting);
        assert_eq!(failure.stats.read_attempts, 5);
        assert!(s.tiles.is_empty() && s.lists.is_empty());
    }
    let mut f = files();
    f[4].2 = wdt(&[(3, 2, 201)], false);
    assert_eq!(
        full_execute(&f, plan::Selection::AllMaps, FullMapLimits::default())
            .0
            .err()
            .unwrap()
            .error,
        Error::FileIdRouting
    );
}
#[test]
fn full_maps_present_missing_or_corrupt_adt_never_emits_partial_list() {
    for corrupt in [false, true] {
        let mut f = files();
        if corrupt {
            f[6].2 = vec![0; 12];
        } else {
            f.remove(6);
        }
        let (r, s) = full_execute(&f, plan::Selection::AllMaps, FullMapLimits::default());
        assert!(r.is_err());
        assert_eq!(s.tiles.len(), 1);
        assert!(s.lists.is_empty());
    }
}
#[test]
fn full_maps_later_map_failure_leaves_earlier_list_only_provisional() {
    let mut f = files();
    replace_maps(&mut f, &[(7, 100, -1), (9, 101, -1)], &[]);
    let (r, s) = full_execute(&f, plan::Selection::AllMaps, FullMapLimits::default());
    let failure = r.err().unwrap();
    assert_eq!(failure.step, run::Step::Wdt { map: 9 });
    assert_eq!(s.lists.len(), 1);
    assert_eq!(failure.stats.tile_lists, 1);
    assert_eq!(s.tiles.len(), 2);
}
#[test]
fn full_maps_source_mutation_is_fatal_before_list_or_after_list() {
    for at_list in [false, true] {
        let fixture = CollisionFixture::new(&files());
        let mut store = open(&fixture);
        let mut sink = Stage::default();
        if at_list {
            sink.change_on_list = Some(&fixture);
        } else {
            sink.change = Some(&fixture);
        }
        let r = run_full_maps(
            &mut store,
            plan::Selection::AllMaps,
            true,
            FullMapLimits::default(),
            &mut sink,
        );
        assert!(r.is_err());
        assert_eq!(sink.lists.len(), usize::from(at_list));
        assert!(store.verify_unchanged().is_err());
    }
}
#[test]
fn full_maps_list_sink_failure_prevents_completion() {
    let fixture = CollisionFixture::new(&files());
    let mut store = open(&fixture);
    let mut sink = Stage {
        fail_list: true,
        ..Stage::default()
    };
    let failure = run_full_maps(
        &mut store,
        plan::Selection::AllMaps,
        true,
        FullMapLimits::default(),
        &mut sink,
    )
    .err()
    .unwrap();
    assert_eq!(failure.error, Error::Run(run::Error::Sink));
    assert_eq!(failure.step, run::Step::TileList { map: 7 });
    assert_eq!(sink.tiles.len(), 2);
    assert!(sink.lists.is_empty());
}
#[test]
fn full_maps_downward_read_output_coordinate_and_memory_limits_reject() {
    for which in 0..4 {
        let mut limits = FullMapLimits::default();
        match which {
            0 => limits.resources.max_reads = 6,
            1 => limits.resources.max_output_bytes = tilelist::BYTES as u64 + 87,
            2 => limits.resources.plan.max_total_coordinates = 4095,
            _ => limits.resources.max_memory_bytes = 1,
        }
        let (r, s) = full_execute(&files(), plan::Selection::AllMaps, limits);
        assert!(r.is_err());
        assert!(s.tiles.is_empty() && s.lists.is_empty());
    }
}
#[test]
fn full_maps_upward_nested_limits_reject_before_acquisition() {
    for which in 0..8 {
        let mut limits = FullMapLimits::default();
        match which {
            0 => limits.resources.max_reads += 1,
            1 => limits.resources.max_memory_bytes += 1,
            2 => limits.resources.plan.max_maps += 1,
            3 => limits.resources.plan.max_total_coordinates += 1,
            4 => limits.resources.plan.max_expansion_bytes += 1,
            5 => limits.resources.plan.max_source_rows += 1,
            6 => limits.resources.conversion.max_input_bytes += 1,
            _ => limits.resources.conversion.max_chunks += 1,
        }
        let (r, s) = full_execute(&files(), plan::Selection::AllMaps, limits);
        assert!(r.is_err());
        assert!(s.maps.is_empty() && s.lists.is_empty());
    }
}

#[test]
fn full_maps_selected_only_sink_default_rejects_required_list() {
    struct SelectedOnly;
    impl Sink for SelectedOnly {
        fn tile(
            &mut self,
            _: conversion::Context,
            _: run::SourceFacts,
            _: &conversion::ConvertedTile,
        ) -> Result<(), run::SinkError> {
            Ok(())
        }
    }
    let fixture = CollisionFixture::new(&files());
    let mut store = open(&fixture);
    let failure = run_full_maps(
        &mut store,
        plan::Selection::AllMaps,
        true,
        FullMapLimits::default(),
        &mut SelectedOnly,
    )
    .err()
    .unwrap();
    assert_eq!(failure.error, Error::Run(run::Error::Sink));
    assert_eq!(failure.stats.converted_tiles, 2);
    assert_eq!(failure.stats.tile_lists, 0);
}
#[test]
fn full_maps_final_source_closure_runs_even_on_invalid_limits() {
    let fixture = CollisionFixture::new(&files());
    let mut store = open(&fixture);
    fixture.change_metadata();
    let mut limits = FullMapLimits::default();
    limits.resources.max_reads += 1;
    let failure = run_full_maps(
        &mut store,
        plan::Selection::AllMaps,
        true,
        limits,
        &mut Stage::default(),
    )
    .err()
    .unwrap();
    assert!(matches!(failure.error, Error::Run(run::Error::Store(_))));
    assert_eq!(failure.stats.read_attempts, 0);
}
