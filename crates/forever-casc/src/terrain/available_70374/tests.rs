use super::*;
use crate::local_store::tests::{CollisionFile, CollisionFixture};
use crate::terrain::available_70450;
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
    files_for_catalog(catalog::WOWDBDEFS_70374_CATALOG_ID)
}
fn files_70450() -> Vec<CollisionFile<'static>> {
    files_for_catalog(catalog::WOWDBDEFS_70450_CATALOG_ID)
}
fn files_for_catalog(catalog_id: &str) -> Vec<CollisionFile<'static>> {
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
            let h = catalog::resolve_for(catalog_id, name)
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
fn open_70450(f: &CollisionFixture) -> LocalStore {
    LocalStore::open_db2_70450(
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
    outcomes: Vec<CompatibleMapOutcome>,
    fail_outcome: bool,
    change_on_outcome: Option<&'a CollisionFixture>,
}
impl Sink for Stage<'_> {
    fn compatible_map_outcome(
        &mut self,
        outcome: CompatibleMapOutcome,
    ) -> Result<(), run::SinkError> {
        if self.fail_outcome {
            return Err(run::SinkError);
        }
        self.outcomes.push(outcome);
        if let Some(f) = self.change_on_outcome {
            f.change_metadata();
        }
        Ok(())
    }
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

#[derive(Default)]
struct Stage70450 {
    maps: Vec<MapFacts>,
    sources: Vec<run::SourceFacts>,
    tiles: Vec<(conversion::Context, Vec<u8>)>,
    witnesses: usize,
    lists: Vec<(u32, Vec<u8>)>,
    outcomes: Vec<CompatibleMapOutcome>,
}
impl Sink for Stage70450 {
    fn compatible_map_outcome(
        &mut self,
        outcome: CompatibleMapOutcome,
    ) -> Result<(), run::SinkError> {
        self.outcomes.push(outcome);
        Ok(())
    }
    fn map_source(&mut self, facts: MapFacts) -> Result<(), run::SinkError> {
        self.maps.push(facts);
        Ok(())
    }
    fn source(&mut self, facts: run::SourceFacts) -> Result<(), run::SinkError> {
        self.sources.push(facts);
        Ok(())
    }
    fn native_witness(&mut self, witness: NativeWitness<'_>) -> Result<(), run::SinkError> {
        assert!(!witness.adt.is_empty());
        assert_eq!(witness.context.build, 70450);
        self.witnesses += 1;
        Ok(())
    }
    fn tile_list(&mut self, map_id: u32, list: &tilelist::TileList) -> Result<(), run::SinkError> {
        self.lists.push((map_id, list.bytes().to_vec()));
        Ok(())
    }
    fn tile(
        &mut self,
        context: conversion::Context,
        _: run::SourceFacts,
        tile: &conversion::ConvertedTile,
    ) -> Result<(), run::SinkError> {
        assert_eq!(context.build, 70450);
        self.tiles.push((context, tile.bytes().to_vec()));
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
fn available_70450_and_70374_runner_build_gates_reject_in_both_directions() {
    let fixture = CollisionFixture::for_build(&files(), 70374);
    let mut store = open(&fixture);
    assert_eq!(
        available_70450::run_selected(
            &mut store,
            &requests()[..1],
            true,
            Limits::default(),
            &mut Stage70450::default(),
        )
        .err()
        .unwrap()
        .error,
        Error::Build70450
    );

    let fixture = CollisionFixture::for_build(&files_70450(), 70450);
    let mut store = open_70450(&fixture);
    assert_eq!(
        run_selected(
            &mut store,
            &requests()[..1],
            true,
            Limits::default(),
            &mut Stage::default(),
        )
        .err()
        .unwrap()
        .error,
        Error::Build
    );
}

#[test]
fn available_70450_strict_and_partial_map_routes_preserve_authentication_coverage() {
    let request = [requests()[0]];
    let fixture = CollisionFixture::for_build(&files_70450(), 70450);
    let mut store = open_70450(&fixture);
    let mut strict_sink = Stage70450::default();
    let strict = available_70450::run_selected(
        &mut store,
        &request,
        true,
        Limits::default(),
        &mut strict_sink,
    )
    .unwrap();
    assert_eq!(strict.build(), 70450);
    assert_eq!(strict.profile(), PROFILE_70450);
    assert!(strict.map().full_ckey_verified);
    assert!(!strict.map().encoded_key_authenticated);
    assert_eq!(strict.map().available_bytes, strict.map().declared_bytes);
    assert_eq!(strict_sink.tiles.len(), 1);
    forever_terrain::maps::verify_70450(
        &strict_sink.tiles[0].1,
        70450,
        forever_terrain::maps::MAX_MAP_BYTES,
    )
    .unwrap();

    let mut partial_files = files_70450();
    partial_files[0].2 = unknown(partial_files[0].2.clone(), 2);
    let section_offset =
        u32::from_le_bytes(partial_files[0].2[252..256].try_into().unwrap()) as usize;
    let record_size = u32::from_le_bytes(partial_files[0].2[144..148].try_into().unwrap()) as usize;
    let encrypted_range = section_offset..section_offset + record_size;
    let fixture = CollisionFixture::with_available_holes_for_build(
        &partial_files,
        &[(MAP_ID, encrypted_range)],
        70450,
    );
    let mut store = open_70450(&fixture);
    let mut partial_sink = Stage70450::default();
    let partial = available_70450::run_selected(
        &mut store,
        &request,
        true,
        Limits::default(),
        &mut partial_sink,
    )
    .unwrap();
    let facts = partial.map();
    assert!(!facts.full_ckey_verified);
    assert!(facts.encoded_key_authenticated);
    assert!(facts.available_bytes < facts.declared_bytes);
    assert_eq!(
        facts.sections[1].unwrap().state,
        db2::SectionState::UnknownTact
    );
    assert_eq!(facts.sections[1].unwrap().declared_records, 1);
    assert_eq!(facts.sections[1].unwrap().declared_copies, 2);
    assert_eq!(partial_sink.tiles.len(), 1);
    forever_terrain::maps::verify_70450(
        &partial_sink.tiles[0].1,
        70450,
        forever_terrain::maps::MAX_MAP_BYTES,
    )
    .unwrap();
}

#[test]
fn available_70450_omitted_map_sections_still_count_against_row_and_copy_limits() {
    let mut f = files_70450();
    f[0].2 = unknown(f[0].2.clone(), 2);
    for direct_limit in [true, false] {
        let fixture = CollisionFixture::for_build(&f, 70450);
        let mut store = open_70450(&fixture);
        let mut limits = Limits::default();
        if direct_limit {
            limits.resources.max_direct_rows = 1;
        } else {
            limits.resources.max_copy_records = 1;
        }
        let mut sink = Stage70450::default();
        let failure =
            available_70450::run_selected(&mut store, &requests()[..1], true, limits, &mut sink)
                .err()
                .unwrap();
        assert_eq!(failure.error, limit());
        assert!(sink.tiles.is_empty());
    }
}

#[test]
fn available_70450_full_map_entrypoint_emits_only_70450_tilelist_and_maps() {
    let fixture = CollisionFixture::for_build(&files_70450(), 70450);
    let mut store = open_70450(&fixture);
    let selected_maps = [7];
    let mut sink = Stage70450::default();
    let completed = available_70450::run_full_maps(
        &mut store,
        plan::Selection::MapIds(&selected_maps),
        true,
        FullMapLimits::default(),
        &mut sink,
    )
    .unwrap();
    assert_eq!(completed.build(), 70450);
    assert_eq!(completed.profile(), available_70450::FULL_MAP_PROFILE);
    assert_eq!(sink.tiles.len(), 2);
    assert_eq!(sink.lists.len(), 1);
    tilelist::verify_70450(&sink.lists[0].1, 70450).unwrap();
    assert_eq!(
        tilelist::verify_70374(&sink.lists[0].1, 70450),
        Err(tilelist::Error::Build)
    );
    for (_, bytes) in &sink.tiles {
        forever_terrain::maps::verify_70450(bytes, 70450, forever_terrain::maps::MAX_MAP_BYTES)
            .unwrap();
    }

    let fixture = CollisionFixture::for_build(&files_70450(), 70450);
    let mut store = open_70450(&fixture);
    let mut sink = Stage70450::default();
    let compatible = available_70450::run_compatible_full_maps(
        &mut store,
        plan::Selection::MapIds(&selected_maps),
        true,
        FullMapLimits::default(),
        &mut sink,
    )
    .unwrap();
    assert_eq!(compatible.build(), 70450);
    assert_eq!(
        compatible.profile(),
        available_70450::COMPATIBLE_FULL_MAP_PROFILE
    );
    assert_eq!(compatible.outcomes().known_present_tiles, 2);
    assert_eq!(sink.lists.len(), 1);
    tilelist::verify_70450(&sink.lists[0].1, 70450).unwrap();
}

#[test]
fn available_map_coverage_invariant_rejects_inconsistent_ckey_completeness() {
    assert_eq!(validate_map_coverage(10, 9, false), Ok(()));
    assert_eq!(validate_map_coverage(10, 10, false), Ok(()));
    assert_eq!(validate_map_coverage(10, 10, true), Ok(()));
    assert_eq!(validate_map_coverage(10, 9, true), Err(Error::Coverage));
    assert_eq!(validate_map_coverage(10, 11, false), Err(Error::Coverage));
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

fn compatible_execute(
    f: &[CollisionFile<'_>],
    selection: plan::Selection<'_>,
    limits: FullMapLimits,
) -> (Result<CompletedCompatibleFullMaps, Failure>, Stage<'static>) {
    let fixture = CollisionFixture::new(f);
    let mut store = open(&fixture);
    let mut sink = Stage::default();
    (
        run_compatible_full_maps(&mut store, selection, true, limits, &mut sink),
        sink,
    )
}

#[test]
fn compatible_missing_wdt_then_loaded_map_preserves_serial_order_and_unknown_main() {
    let mut f = files();
    replace_maps(&mut f, &[(9, 100, -1), (7, 101, -1)], &[]);
    f.retain(|(id, _, _)| *id != 100);
    f.push((101, None, wdt(&[(0, 0, 201)], true)));
    let (r, s) = compatible_execute(&f, plan::Selection::AllMaps, FullMapLimits::default());
    let done = r.unwrap();
    assert_eq!(done.profile(), COMPATIBLE_FULL_MAP_PROFILE);
    assert_eq!(done.build(), 70374);
    assert_eq!(done.stats().read_attempts, 7);
    assert_eq!(done.stats().converted_tiles, 1);
    assert_eq!(done.stats().tile_lists, 2);
    assert_eq!(
        done.plan(),
        plan::CompatibleCompletion {
            scope: plan::Scope::AllMaps,
            maps: 2,
            selected_coordinates: 8192,
            known_present_tiles: 1,
            known_absent_tiles: 4095,
            unknown_coordinates: 4096,
            unavailable_wdts: 1,
        }
    );
    assert_eq!(
        done.outcomes(),
        CompatibleSummary {
            loaded_wdts: 1,
            unavailable_wdts: 1,
            known_present_tiles: 1,
            converted_tiles: 1,
            missing_root_adts: 0,
        }
    );
    assert_eq!(
        s.outcomes
            .iter()
            .map(|o| (o.map_id, o.materialized_ordinal))
            .collect::<Vec<_>>(),
        [(9, 0), (7, 1)]
    );
    assert_eq!(s.outcomes[0].wdt, WdtDisposition::Unavailable);
    assert_eq!(s.outcomes[0].known_present_tiles, None);
    assert_eq!(s.outcomes[1].wdt, WdtDisposition::Loaded);
    assert_eq!(s.outcomes[1].known_present_tiles, Some(1));
    assert!(s.lists[0].1[12..].iter().all(|b| *b == b'0'));
    assert!(tilelist::present_70374(&s.lists[1].1, 70374, 0, 0).unwrap());
    let (strict, strict_sink) =
        full_execute(&f, plan::Selection::AllMaps, FullMapLimits::default());
    assert!(matches!(
        strict.err().unwrap().error,
        Error::Run(run::Error::Store(local_store::Error::MissingFile))
    ));
    assert!(strict_sink.lists.is_empty() && strict_sink.tiles.is_empty());
}

#[test]
fn compatible_first_middle_last_missing_adt_never_sets_omitted_bit() {
    for missing in 0..3 {
        let mut f = files();
        f[4].2 = wdt(&[(3, 2, 201), (4, 2, 202), (5, 2, 203)], true);
        f.push((203, None, adt(3.0)));
        f.retain(|(id, _, _)| *id != 201 + missing as u32);
        let (r, s) = compatible_execute(&f, plan::Selection::AllMaps, FullMapLimits::default());
        let done = r.unwrap();
        assert_eq!(
            (done.stats().read_attempts, done.stats().converted_tiles),
            (8, 2)
        );
        assert_eq!(
            (
                done.plan().known_present_tiles,
                done.plan().known_absent_tiles,
                done.plan().unknown_coordinates
            ),
            (3, 4093, 0)
        );
        assert_eq!(
            (
                s.outcomes[0].known_present_tiles,
                s.outcomes[0].converted_tiles,
                s.outcomes[0].missing_root_adts
            ),
            (Some(3), 2, 1)
        );
        assert_eq!(done.outcomes().missing_root_adts, 1);
        for n in 0..3 {
            assert_eq!(
                tilelist::present_70374(&s.lists[0].1, 70374, 3 + n, 2).unwrap(),
                usize::from(n) != missing
            );
        }
        let (strict, strict_sink) =
            full_execute(&f, plan::Selection::AllMaps, FullMapLimits::default());
        assert!(matches!(
            strict.err().unwrap().error,
            Error::Run(run::Error::Store(local_store::Error::MissingFile))
        ));
        assert!(strict_sink.lists.is_empty());
    }
}

#[test]
fn compatible_all_unavailable_wdts_are_unknown_not_absent() {
    let mut f = files();
    replace_maps(&mut f, &[(9, 100, -1), (7, 101, -1), (11, 102, -1)], &[]);
    f.retain(|(id, _, _)| *id != 100);
    let (r, s) = compatible_execute(&f, plan::Selection::AllMaps, FullMapLimits::default());
    let done = r.unwrap();
    assert_eq!(
        (
            done.stats().read_attempts,
            done.stats().converted_tiles,
            done.stats().tile_lists
        ),
        (7, 0, 3)
    );
    assert_eq!(
        (
            done.plan().known_present_tiles,
            done.plan().known_absent_tiles,
            done.plan().unknown_coordinates
        ),
        (0, 0, 12288)
    );
    assert_eq!(done.stats().output_bytes, 3 * tilelist::BYTES as u64);
    assert_eq!(done.outcomes().unavailable_wdts, 3);
    assert!(s.outcomes.iter().all(|o| o.known_present_tiles.is_none()
        && o.converted_tiles == 0
        && o.missing_root_adts == 0));
    assert!(
        s.lists
            .iter()
            .all(|(_, list)| list[12..].iter().all(|b| *b == b'0'))
    );
}

#[test]
fn compatible_loaded_empty_main_and_empty_domain_keep_distinct_facts() {
    let mut f = files();
    f[4].2 = wdt(&[], true);
    let (r, s) = compatible_execute(&f, plan::Selection::AllMaps, FullMapLimits::default());
    let done = r.unwrap();
    assert_eq!(
        (
            done.plan().known_absent_tiles,
            done.plan().unknown_coordinates
        ),
        (4096, 0)
    );
    assert_eq!(s.outcomes[0].known_present_tiles, Some(0));
    assert_eq!(s.outcomes[0].wdt, WdtDisposition::Loaded);
    replace_maps(&mut f, &[], &[]);
    let (r, s) = compatible_execute(&f, plan::Selection::AllMaps, FullMapLimits::default());
    let done = r.unwrap();
    assert_eq!(done.plan().maps, 0);
    assert_eq!(done.stats().read_attempts, 4);
    assert_eq!(done.outcomes(), CompatibleSummary::default());
    assert!(s.lists.is_empty() && s.outcomes.is_empty());
}

#[test]
fn compatible_malformed_sources_converter_and_sink_failures_remain_fatal() {
    for which in 0..3 {
        let mut f = files();
        match which {
            0 => f[4].2 = vec![0; 12],
            1 => f[6].2 = vec![0; 12],
            _ => f[4].2 = wdt(&[(3, 2, 0)], true),
        }
        let (r, s) = compatible_execute(&f, plan::Selection::AllMaps, FullMapLimits::default());
        assert!(r.is_err());
        assert!(s.lists.is_empty() && s.outcomes.is_empty());
    }
    for outcome in [false, true] {
        let f = CollisionFixture::new(&files());
        let mut store = open(&f);
        let mut s = Stage {
            fail_list: !outcome,
            fail_outcome: outcome,
            ..Default::default()
        };
        let failure = run_compatible_full_maps(
            &mut store,
            plan::Selection::AllMaps,
            true,
            FullMapLimits::default(),
            &mut s,
        )
        .err()
        .unwrap();
        assert_eq!(failure.error, Error::Run(run::Error::Sink));
        assert!(s.outcomes.is_empty());
    }
}

#[test]
fn compatible_only_missing_file_is_omittable() {
    assert!(missing_file(store_error(local_store::Error::MissingFile)));
    for error in [
        local_store::Error::Input,
        local_store::Error::InputChanged,
        local_store::Error::Limit,
        local_store::Error::UnknownSize,
        local_store::Error::Integrity,
        local_store::Error::NoLocalEncoding(local_store::CandidateFailures::default()),
        local_store::Error::NoLocalEncoding(local_store::CandidateFailures {
            incomplete: 1,
            ..Default::default()
        }),
        local_store::Error::NoLocalEncoding(local_store::CandidateFailures {
            corrupt: 1,
            ..Default::default()
        }),
    ] {
        assert!(!missing_file(store_error(error)));
    }
    assert!(!missing_file(Error::Run(run::Error::Sink)));
    assert!(!missing_file(limit()));
}

#[test]
fn compatible_missing_reads_and_lists_respect_downward_budgets() {
    let mut f = files();
    f.retain(|(id, _, _)| *id != 100);
    for which in 0..5 {
        let mut limits = FullMapLimits::default();
        match which {
            0 => limits.resources.max_reads = 4,
            1 => limits.resources.max_output_bytes = tilelist::BYTES as u64 - 1,
            2 => limits.resources.plan.max_total_coordinates = 4095,
            3 => limits.resources.max_memory_bytes = 1,
            _ => limits.resources.max_source_bytes = 0,
        }
        let (r, s) = compatible_execute(&f, plan::Selection::AllMaps, limits);
        assert!(r.is_err());
        assert!(s.lists.is_empty() && s.outcomes.is_empty());
    }
    let mut limits = FullMapLimits::default();
    limits.resources.max_reads = 5;
    limits.resources.max_output_bytes = tilelist::BYTES as u64;
    let (r, _) = compatible_execute(&f, plan::Selection::AllMaps, limits);
    assert_eq!(r.unwrap().stats().read_attempts, 5);
    let (r, s) = compatible_execute(
        &files(),
        plan::Selection::Tiles(&requests()),
        FullMapLimits::default(),
    );
    assert_eq!(r.err().unwrap().error, Error::FullMapSelection);
    assert!(s.maps.is_empty());
}

#[test]
fn compatible_source_change_after_list_or_outcome_is_fatal() {
    for after_outcome in [false, true] {
        let mut f = files();
        f.retain(|(id, _, _)| *id != 100);
        let fixture = CollisionFixture::new(&f);
        let mut store = open(&fixture);
        let mut s = Stage {
            change_on_list: (!after_outcome).then_some(&fixture),
            change_on_outcome: after_outcome.then_some(&fixture),
            ..Default::default()
        };
        let failure = run_compatible_full_maps(
            &mut store,
            plan::Selection::AllMaps,
            true,
            FullMapLimits::default(),
            &mut s,
        )
        .err()
        .unwrap();
        assert_eq!(failure.error, store_error(local_store::Error::InputChanged));
        assert_eq!(s.outcomes.len(), usize::from(after_outcome));
    }
}

#[test]
fn compatible_outcome_callback_requires_explicit_sink_opt_in() {
    struct FullOnly;
    impl Sink for FullOnly {
        fn tile(
            &mut self,
            _: conversion::Context,
            _: run::SourceFacts,
            _: &conversion::ConvertedTile,
        ) -> Result<(), run::SinkError> {
            Ok(())
        }
        fn tile_list(&mut self, _: u32, _: &tilelist::TileList) -> Result<(), run::SinkError> {
            Ok(())
        }
    }
    let fixture = CollisionFixture::new(&files());
    let mut store = open(&fixture);
    let failure = run_compatible_full_maps(
        &mut store,
        plan::Selection::AllMaps,
        true,
        FullMapLimits::default(),
        &mut FullOnly,
    )
    .err()
    .unwrap();
    assert_eq!(failure.error, Error::Run(run::Error::Sink));
    assert_eq!(failure.stats.tile_lists, 1);
}

#[test]
fn compatible_all_missing_adts_keep_loaded_main_known() {
    let mut f = files();
    f.retain(|(id, _, _)| *id != 201 && *id != 202);
    let (r, s) = compatible_execute(&f, plan::Selection::AllMaps, FullMapLimits::default());
    let done = r.unwrap();
    assert_eq!(
        (done.stats().read_attempts, done.stats().converted_tiles),
        (7, 0)
    );
    assert_eq!(
        (
            done.plan().known_present_tiles,
            done.plan().known_absent_tiles,
            done.plan().unknown_coordinates
        ),
        (2, 4094, 0)
    );
    assert_eq!(
        (
            s.outcomes[0].known_present_tiles,
            s.outcomes[0].converted_tiles,
            s.outcomes[0].missing_root_adts
        ),
        (Some(2), 0, 2)
    );
    assert!(s.lists[0].1[12..].iter().all(|b| *b == b'0'));
}

#[test]
fn compatible_complete_inputs_match_strict_tile_and_list_bytes() {
    let f = files();
    let (strict, strict_sink) =
        full_execute(&f, plan::Selection::AllMaps, FullMapLimits::default());
    let (compatible, compatible_sink) =
        compatible_execute(&f, plan::Selection::AllMaps, FullMapLimits::default());
    let strict = strict.unwrap();
    let compatible = compatible.unwrap();
    assert_eq!(compatible_sink.tiles, strict_sink.tiles);
    assert_eq!(compatible_sink.lists, strict_sink.lists);
    assert_eq!(compatible.stats(), strict.stats());
    assert_eq!(
        compatible.plan().known_present_tiles,
        strict.plan().present_tiles
    );
    assert_eq!(
        compatible.plan().known_absent_tiles,
        strict.plan().absent_tiles
    );
    assert_eq!(compatible.plan().unknown_coordinates, 0);
}

#[test]
fn compatible_ckey_corruption_is_fatal_not_an_unavailable_omission() {
    for target in [100, 202] {
        let f = files();
        let fixture = CollisionFixture::new(&f);
        let payload = &f.iter().find(|(id, _, _)| *id == target).unwrap().2;
        let archive_path = fixture.path().join("Data/data/data.000");
        let mut archive = std::fs::read(&archive_path).unwrap();
        let start = archive
            .windows(payload.len())
            .position(|part| part == payload)
            .unwrap();
        archive[start] ^= 1; // Authored source bytes differ from the retained CKey.
        std::fs::write(archive_path, archive).unwrap();
        let mut store = open(&fixture); // Mutation predates the store snapshot.
        let mut sink = Stage::default();
        let failure = run_compatible_full_maps(
            &mut store,
            plan::Selection::AllMaps,
            true,
            FullMapLimits::default(),
            &mut sink,
        )
        .err()
        .unwrap();
        assert!(matches!(
            failure.error,
            Error::Run(run::Error::Store(local_store::Error::NoLocalEncoding(_)))
        ));
        assert!(sink.lists.is_empty() && sink.outcomes.is_empty());
    }
}
