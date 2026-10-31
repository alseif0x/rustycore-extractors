// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-19: synthetic-only selected/all-available collision geometry coverage.
use super::*;
use crate::collision::{
    Extractor, Limits as CollectorLimits, assemble::staged::models, gameobjects,
    world::tests as fixture,
};
use crate::local_store::{
    self,
    tests::{CollisionFile, CollisionFixture},
};
use crate::{Locale, local_index, root};
use forever_db2::catalog;

fn put(bytes: &mut [u8], at: usize, value: u32) {
    bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
}
fn word(bytes: &[u8], at: usize) -> usize {
    u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()) as usize
}
fn files_70450(maps: &[(u32, u32, i16)]) -> Vec<CollisionFile<'static>> {
    let mut files = fixture::files(maps);
    // The excluded profiles read only their explicit startup and liquid sources;
    // v47 has separate fixtures for executed GODI rows. LiquidObject stays poisoned.
    files.retain(|(id, _, _)| *id != gameobjects::TABLE_FILE_DATA_ID);
    files.push((
        gameobjects::TABLE_FILE_DATA_ID,
        None,
        fixture::gameobject_db2(&[], &[]),
    ));
    files.push((
        crate::terrain::run::OBJECT_ID,
        None,
        b"poisonous synthetic LiquidObject must not be read".to_vec(),
    ));
    for (id, _, bytes) in &mut files {
        let name = match *id {
            world::MAP_FILE_DATA_ID => "Map",
            world::MATERIAL_FILE_DATA_ID => "LiquidMaterial",
            world::TYPE_FILE_DATA_ID => "LiquidType",
            gameobjects::TABLE_FILE_DATA_ID => "GameObjectDisplayInfo",
            _ => continue,
        };
        let metadata = catalog::resolve_for(catalog::WOWDBDEFS_70450_CATALOG_ID, name)
            .unwrap()
            .unwrap()
            .metadata();
        put(bytes, 152, metadata.table_hash.unwrap());
    }
    files
}
fn open(fixture: &CollisionFixture) -> LocalStore {
    LocalStore::open_db2_70450(
        fixture.path(),
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
struct Stage {
    models: usize,
    placements: Vec<(run::Context, Vec<u8>)>,
}
impl run::Sink for Stage {
    fn model(&mut self, _: &Model) -> Result<(), run::SinkError> {
        self.models += 1;
        Ok(())
    }
    fn placements(&mut self, context: run::Context, bytes: &[u8]) -> Result<(), run::SinkError> {
        self.placements.push((context, bytes.to_vec()));
        Ok(())
    }
}
fn run_fixture(
    files: &[CollisionFile<'_>],
    map_id: u32,
    sink: &mut Stage,
) -> Result<Completed, Error> {
    let fixture = CollisionFixture::for_build(files, 70450);
    let mut store = open(&fixture);
    extract_map(&mut store, map_id, sink, Limits::default())
}
fn run_all_with_root_absence(
    files: &[CollisionFile<'_>],
    sink: &mut Stage,
) -> Result<Completed, Error> {
    let fixture = CollisionFixture::for_build(files, 70450);
    let mut store = open(&fixture);
    extract_all_with_root_absence(&mut store, sink, Limits::default())
}
fn run_all_with_model_root_absence(
    files: &[CollisionFile<'_>],
    sink: &mut Stage,
) -> Result<Completed, Error> {
    let fixture = CollisionFixture::for_build(files, 70450);
    let mut store = open(&fixture);
    extract_all_with_model_root_absence(&mut store, sink, Limits::default())
}
fn run_all_with_model_root_absence_request_capacity(
    files: &[CollisionFile<'_>],
    sink: &mut Stage,
) -> Result<Completed, Error> {
    let fixture = CollisionFixture::for_build(files, 70450);
    let mut store = open(&fixture);
    extract_all_with_model_root_absence_request_capacity(&mut store, sink, Limits::default())
}
fn run_all_with_vacated_wmo(
    files: &[CollisionFile<'_>],
    sink: &mut Stage,
) -> Result<Completed, Error> {
    let fixture = CollisionFixture::for_build(files, 70450);
    let mut store = open(&fixture);
    extract_all_with_vacated_wmo(&mut store, sink, Limits::default())
}
fn run_all_with_empty_wmo(
    files: &[CollisionFile<'_>],
    sink: &mut Stage,
) -> Result<Completed, Error> {
    let fixture = CollisionFixture::for_build(files, 70450);
    let mut store = open(&fixture);
    extract_all_with_empty_wmo(&mut store, sink, Limits::default())
}
fn run_all_with_material_filtered_wmo(
    files: &[CollisionFile<'_>],
    sink: &mut Stage,
) -> Result<Completed, Error> {
    let fixture = CollisionFixture::for_build(files, 70450);
    let mut store = open(&fixture);
    extract_all_with_material_filtered_wmo(&mut store, sink, Limits::default())
}
fn run_all_with_dependency_heavy_material_filtered_wmo(
    files: &[CollisionFile<'_>],
    sink: &mut Stage,
) -> Result<Completed, Error> {
    let fixture = CollisionFixture::for_build(files, 70450);
    let mut store = open(&fixture);
    extract_all_with_dependency_heavy_material_filtered_wmo(&mut store, sink, Limits::default())
}
fn run_all_with_source_heavy_material_filtered_wmo(
    files: &[CollisionFile<'_>],
    sink: &mut Stage,
) -> Result<Completed, Error> {
    let fixture = CollisionFixture::for_build(files, 70450);
    let mut store = open(&fixture);
    extract_all_with_source_heavy_material_filtered_wmo(&mut store, sink, Limits::default())
}
fn run_all_with_read_heavy_material_filtered_wmo(
    files: &[CollisionFile<'_>],
    sink: &mut Stage,
) -> Result<Completed, Error> {
    let fixture = CollisionFixture::for_build(files, 70450);
    let mut store = open(&fixture);
    extract_all_with_read_heavy_material_filtered_wmo(&mut store, sink, Limits::default())
}
fn run_all_with_model_heavy_material_filtered_wmo(
    files: &[CollisionFile<'_>],
    sink: &mut Stage,
) -> Result<Completed, Error> {
    let fixture = CollisionFixture::for_build(files, 70450);
    let mut store = open(&fixture);
    extract_all_with_model_heavy_material_filtered_wmo(&mut store, sink, Limits::default())
}
fn run_all_with_source_expanded_material_filtered_wmo(
    files: &[CollisionFile<'_>],
    sink: &mut Stage,
) -> Result<Completed, Error> {
    let fixture = CollisionFixture::for_build(files, 70450);
    let mut store = open(&fixture);
    extract_all_with_source_expanded_material_filtered_wmo(&mut store, sink, Limits::default())
}
fn run_all_with_model_output_expanded_material_filtered_wmo(
    files: &[CollisionFile<'_>],
    sink: &mut Stage,
) -> Result<Completed, Error> {
    let fixture = CollisionFixture::for_build(files, 70450);
    let mut store = open(&fixture);
    extract_all_with_model_output_expanded_material_filtered_wmo(
        &mut store,
        sink,
        Limits::default(),
    )
}
fn run_all_with_model_bytes_expanded_material_filtered_wmo(
    files: &[CollisionFile<'_>],
    sink: &mut Stage,
) -> Result<Completed, Error> {
    let fixture = CollisionFixture::for_build(files, 70450);
    let mut store = open(&fixture);
    extract_all_with_model_bytes_expanded_material_filtered_wmo(&mut store, sink, Limits::default())
}
fn run_all_with_source_expanded_v2_material_filtered_wmo(
    files: &[CollisionFile<'_>],
    sink: &mut Stage,
) -> Result<Completed, Error> {
    let fixture = CollisionFixture::for_build(files, 70450);
    let mut store = open(&fixture);
    extract_all_with_source_expanded_v2_material_filtered_wmo(&mut store, sink, Limits::default())
}
fn run_all_with_placement_expanded_material_filtered_wmo(
    files: &[CollisionFile<'_>],
    sink: &mut Stage,
) -> Result<Completed, Error> {
    let fixture = CollisionFixture::for_build(files, 70450);
    let mut store = open(&fixture);
    extract_all_with_placement_expanded_material_filtered_wmo(&mut store, sink, Limits::default())
}
fn with_tile_geometry() -> Vec<CollisionFile<'static>> {
    let mut files = files_70450(&[(7, 100, -1)]);
    files.extend([
        (100, None, fixture::wdt(None, Some(200))),
        (
            200,
            None,
            [fixture::wmo_adt(400, 2), fixture::m2_adt(300, 3)].concat(),
        ),
        (300, None, fixture::m2(true)),
        (400, None, fixture::wmo_root(401, None)),
        (401, None, fixture::wmo_group(true, false, false)),
    ]);
    files
}
// Mirrors the available-source fixture: the declared parent is confined to a
// second unknown section, so only the child row is materialized.
fn with_unavailable_text_section(mut bytes: Vec<u8>) -> Vec<u8> {
    let start = word(&bytes, 212);
    let rows = word(&bytes, 136);
    let size = word(&bytes, 144);
    for i in 0..rows {
        for field in [0, 4] {
            let at = start + i * size + field;
            let n = word(&bytes, at);
            put(&mut bytes, at, (n + size) as u32);
        }
    }
    let mut out = bytes[..244].to_vec();
    out.extend([0; 40]);
    out.extend(&bytes[244..start]);
    out.extend(0u32.to_le_bytes());
    out.extend(&bytes[start..]);
    put(&mut out, 136, (rows + 1) as u32);
    put(&mut out, 200, 2);
    put(&mut out, 212, (start + 44) as u32);
    let next = out.len() as u32;
    out[244..252].copy_from_slice(&1u64.to_le_bytes());
    put(&mut out, 252, next);
    put(&mut out, 256, 1);
    put(&mut out, 268, 4);
    out.extend(vec![0xa5; size]);
    out.extend(99u32.to_le_bytes());
    out
}

#[test]
fn p1_one_active_map_acquires_complete_m2_and_wmo_without_godi() {
    let mut sink = Stage::default();
    let out = run_fixture(&with_tile_geometry(), 7, &mut sink).unwrap();

    assert_eq!(out.profile(), PROFILE);
    assert_eq!(
        out.world().profile(),
        available_70374::PROFILE_70450_SELECTED_GEOMETRY
    );
    assert_eq!(out.world().map_stats().active_maps, 1);
    assert_eq!(out.selection(), plan::MapSelection::MapAndAncestors(7));
    assert_eq!(sink.models, 2);
    assert_eq!(sink.placements.len(), 2);
    assert!(sink.placements.iter().any(|(_, bytes)| !bytes.is_empty()));
    assert_eq!(out.world().startup().data().table().file_data_id(), 0);
    let startup = out.world().startup().data().stats();
    assert_eq!(startup.direct_rows, 0);
    assert_eq!(startup.copy_records, 0);
    assert_eq!(startup.candidate_requests, 0);
    assert!(out.world().startup().data().outcomes().is_empty());
    assert_eq!(out.world().startup().data().startup_facts().reads, 0);
    assert_eq!(out.report().collector_before.reads, 3); // Map + two liquids only.
    assert_eq!(out.report().request_calls_before, 0);
    assert_eq!(
        *out.coverage(),
        CoverageFacts {
            gameobject_scope: GameObjectScope::Excluded,
            gameobject_file_data_id: gameobjects::TABLE_FILE_DATA_ID,
            gameobject_block_reason: GameObjectBlockReason::ExcludedByProfile,
            gameobject_request_made: false,
            gameobject_startup_reads: 0,
            gameobject_sections: 0,
            gameobject_direct_rows: 0,
            gameobject_copy_records: 0,
            gameobject_outcomes: 0,
            model_scope: "placement-only-v1",
            liquid_object_scope: LiquidObjectScope::OutOfScope,
            liquid_object_request_made: false,
        }
    );
    assert_eq!(out.coverage().gameobject_scope.as_str(), "excluded");
    assert_eq!(
        out.coverage().gameobject_block_reason.as_str(),
        "excluded_by_profile"
    );
    assert!(!out.coverage().gameobject_request_made);
    assert_eq!(out.coverage().liquid_object_scope.as_str(), "out_of_scope");
    assert!(!out.coverage().liquid_object_request_made);
    assert!(out.gameobject_facts().is_none());
}

#[test]
fn v47_executes_godi_direct_rows_once_and_reports_physical_outcomes() {
    let mut files = files_70450(&[(7, 100, -1)]);
    let godi = files
        .iter_mut()
        .find(|(id, _, _)| *id == gameobjects::TABLE_FILE_DATA_ID)
        .unwrap();
    godi.2 = fixture::gameobject_db2(&[(10, 300), (11, 0), (12, 301)], &[(20, 10)]);
    godi.2[152..156].copy_from_slice(
        &catalog::resolve_for(catalog::WOWDBDEFS_70450_CATALOG_ID, "GameObjectDisplayInfo")
            .unwrap()
            .unwrap()
            .metadata()
            .table_hash
            .unwrap()
            .to_le_bytes(),
    );
    files.extend([
        (100, None, fixture::wdt(None, None)),
        (300, None, fixture::m2(true)),
        (301, None, fixture::m2(true)),
    ]);
    let fixture = CollisionFixture::for_build(&files, 70450);
    let mut store = open(&fixture);
    let mut sink = Stage::default();
    let out = extract_all_with_gameobject_expanded_material_filtered_wmo(
        &mut store,
        &mut sink,
        Limits::default(),
    )
    .unwrap();

    assert_eq!(
        out.profile(),
        AVAILABLE_GAMEOBJECT_EXPANDED_MATERIAL_FILTERED_WMO_PROFILE
    );
    assert_eq!(out.selection(), plan::MapSelection::All);
    assert_eq!(
        out.coverage().gameobject_scope,
        GameObjectScope::AvailableFdidDirectV1
    );
    assert_eq!(
        out.coverage().gameobject_file_data_id,
        gameobjects::TABLE_FILE_DATA_ID
    );
    assert_eq!(
        out.coverage().gameobject_block_reason,
        GameObjectBlockReason::None
    );
    assert!(out.coverage().gameobject_request_made);
    assert_eq!(out.coverage().gameobject_startup_reads, 3);
    assert_eq!(out.coverage().model_scope, "placement-and-gameobject-v1");
    assert_eq!(out.report().run.model_reads, 0);
    assert_eq!(out.report().run.ids.calls, 0);

    let facts = out.gameobject_facts().unwrap();
    assert_eq!(facts.direct_rows, 3);
    assert_eq!(facts.copy_records, 1);
    assert_eq!(facts.direct_outcomes, 3);
    assert_eq!(facts.zero_fdids, 1);
    assert_eq!(facts.collision_rows, 2);
    assert_eq!(facts.list_rows, 2);
    assert_eq!(facts.sections.len(), 1);
    assert_eq!(facts.outcomes[0].ordinal, 0);
    assert_eq!(facts.outcomes[1].ordinal, 1);
    assert_eq!(facts.outcomes[2].ordinal, 2);
    assert_eq!(
        facts.outcomes[0].outcome,
        available_70374::GameObjectOutcome::Collision
    );
    assert_eq!(
        facts.outcomes[1].outcome,
        available_70374::GameObjectOutcome::ZeroFileDataId
    );
    assert_eq!(
        facts.outcomes[2].outcome,
        available_70374::GameObjectOutcome::Collision
    );
    assert_eq!(out.gameobjects().outcomes().len(), 3);
    assert_eq!(out.gameobjects().startup_facts().reads, 3);
    assert_eq!(
        out.world().startup().data().table().file_data_id(),
        gameobjects::TABLE_FILE_DATA_ID
    );
    assert_eq!(sink.models, 2);
}

#[test]
fn p2_selected_child_closes_over_one_active_parent() {
    let mut files = files_70450(&[(7, 100, -1), (8, 101, 7)]);
    files.extend([
        (100, None, fixture::wdt(None, None)),
        (101, None, fixture::wdt(None, Some(200))),
        (200, None, fixture::m2_adt(300, 4)),
        (300, None, fixture::m2(true)),
    ]);
    let mut sink = Stage::default();
    let out = run_fixture(&files, 8, &mut sink).unwrap();

    assert_eq!(out.selection(), plan::MapSelection::MapAndAncestors(8));
    assert_eq!(out.world().map_stats().rows, 2);
    assert_eq!(out.world().map_stats().active_maps, 2);
    assert_eq!(out.report().run.maps, 2);
    assert_eq!(out.world().topology().selected_active().count(), 2);
    assert_eq!(out.report().run.object_adts, 1);
    assert_eq!(sink.models, 1);
    let map = out.world().map();
    assert_eq!(map.file_data_id(), world::MAP_FILE_DATA_ID);
    assert_eq!(map.declared_bytes, map.available_bytes);
    assert!(map.full_ckey_verified);
    assert!(map.full_ckey_verified || map.encoded_key_authenticated);
    assert_eq!(map.declared_direct_rows, 2);
    assert_eq!(map.declared_copy_records, 0);
    assert_eq!(map.direct_rows, 2);
    assert_eq!(map.copy_records, 0);
    assert_eq!(map.materialized_rows, 2);
    assert_eq!(map.sections().count(), 1);
    assert!(map.all_sections_admitted());
}

#[test]
fn selected_70450_world_rejects_a_godi_startup_instead_of_emptying_it() {
    let mut files = files_70450(&[(7, 100, -1)]);
    files
        .iter_mut()
        .find(|(id, _, _)| *id == gameobjects::TABLE_FILE_DATA_ID)
        .unwrap()
        .2 = fixture::gameobject_db2(&[], &[]);
    let fixture = CollisionFixture::for_build(&files, 70450);
    let mut store = open(&fixture);
    let startup =
        gameobjects::extract_startup_fdid(&mut store, gameobjects::Limits::default()).unwrap();
    assert_eq!(startup.data().startup_facts().reads, 1);
    let mut sink = Stage::default();
    let error = match available_70374::extract_selected_70450(
        startup,
        7,
        &mut sink,
        world::Limits::default(),
    ) {
        Ok(_) => panic!("a GODI-reading startup was silently reclassified as empty"),
        Err(error) => error,
    };
    assert_eq!(error.code(), "available_collision_state");
    assert_eq!((sink.models, sink.placements.len()), (0, 0));
}

#[test]
fn n1_parent_in_unavailable_map_section_is_missing_parent() {
    let mut files = files_70450(&[(8, 101, 99)]);
    let map = files
        .iter_mut()
        .find(|(id, _, _)| *id == world::MAP_FILE_DATA_ID)
        .unwrap();
    map.2 = with_unavailable_text_section(std::mem::take(&mut map.2));

    // Preserve the source facts that precede domain closure failure: the Map
    // declaration includes both rows, but only the admitted section materializes.
    let fixture = CollisionFixture::for_build(&files, 70450);
    let mut store = open(&fixture);
    let lookup = gameobjects::EmptyLookup;
    let mut collector = Extractor::new(&mut store, &lookup, CollectorLimits::default()).unwrap();
    let prepared = match available_70374::prepare_available_target(
        &mut collector,
        world::Limits::default(),
        crate::collision::target::Target::Build70450,
    ) {
        Ok(prepared) => prepared,
        Err(_) => panic!("available Map preparation should precede missing-parent closure"),
    };
    let table = &prepared.map;
    assert_eq!(table.file_data_id(), world::MAP_FILE_DATA_ID);
    assert!(table.declared_bytes > 0);
    assert!(table.available_bytes > 0);
    assert!(table.full_ckey_verified || table.encoded_key_authenticated);
    assert_eq!(table.declared_direct_rows, 2);
    assert_eq!(table.direct_rows, 1);
    assert_eq!(table.materialized_rows, 1);
    let sections: Vec<_> = table.sections().collect();
    assert_eq!(sections.len(), 2);
    assert_eq!(
        sections[0].state,
        forever_db2::availability::SectionState::Admitted
    );
    assert_eq!(
        sections[1].state,
        forever_db2::availability::SectionState::UnknownTact
    );
    drop(prepared);
    drop(collector);
    drop(store);

    let mut sink = Stage::default();
    let error = match run_fixture(&files, 8, &mut sink) {
        Err(error) => error,
        Ok(_) => panic!("unavailable parent unexpectedly completed"),
    };
    assert!(matches!(
        error,
        Error::Available(available_70374::Error::Plan(plan::Error::MissingParent))
    ));
    assert_eq!((sink.models, sink.placements.len()), (0, 0));
}

#[test]
fn all_70450_empty_active_domain_has_an_explicit_empty_completion() {
    let files = files_70450(&[(7, 0, -1)]);
    let fixture = CollisionFixture::for_build(&files, 70450);
    let mut store = open(&fixture);
    let mut sink = Stage::default();
    let out = extract_all(&mut store, &mut sink, Limits::default()).unwrap();

    assert_eq!(out.profile(), AVAILABLE_PROFILE);
    assert_eq!(
        out.world().profile(),
        available_70374::PROFILE_70450_AVAILABLE_GEOMETRY
    );
    assert_eq!(out.selection(), plan::MapSelection::All);
    assert_eq!(out.world().map_stats().rows, 1);
    assert_eq!(out.world().map_stats().direct_rows, 1);
    assert_eq!(out.world().map_stats().active_maps, 0);
    assert_eq!(out.report().run.maps, 0);
    assert_eq!(out.report().run.wdt_files, 0);
    assert_eq!((sink.models, sink.placements.len()), (0, 0));
}

#[test]
fn n2_zero_wdt_file_data_id_is_inactive_map() {
    let mut sink = Stage::default();
    let error = match run_fixture(&files_70450(&[(7, 0, -1)]), 7, &mut sink) {
        Err(error) => error,
        Ok(_) => panic!("inactive map unexpectedly completed"),
    };
    assert!(matches!(
        error,
        Error::Available(available_70374::Error::Plan(plan::Error::InactiveMap))
    ));
    assert_eq!((sink.models, sink.placements.len()), (0, 0));
}

#[test]
fn n3_missing_or_malformed_liquid_table_fails_before_sink_callbacks() {
    let mut missing = files_70450(&[(7, 100, -1)]);
    missing.retain(|(id, _, _)| *id != world::MATERIAL_FILE_DATA_ID);
    let mut malformed = files_70450(&[(7, 100, -1)]);
    malformed
        .iter_mut()
        .find(|(id, _, _)| *id == world::TYPE_FILE_DATA_ID)
        .unwrap()
        .2
        .truncate(1);

    for (files, code) in [
        (&missing, "file_unavailable"),
        (&malformed, "invalid_record_metadata"),
    ] {
        let mut sink = Stage::default();
        let error = match run_fixture(files, 7, &mut sink) {
            Ok(_) => panic!("missing or malformed liquid source unexpectedly passed"),
            Err(error) => error,
        };
        assert!(matches!(error, Error::Available(_)));
        assert_eq!(error.code(), code);
        assert_eq!((sink.models, sink.placements.len()), (0, 0));
    }
}

#[test]
fn n4_unknown_map_id_is_typed_unknown_map() {
    let mut sink = Stage::default();
    let error = match run_fixture(&files_70450(&[(7, 100, -1)]), 404, &mut sink) {
        Err(error) => error,
        Ok(_) => panic!("unknown map unexpectedly completed"),
    };
    assert!(matches!(
        error,
        Error::Available(available_70374::Error::Plan(plan::Error::UnknownMap))
    ));
    assert_eq!((sink.models, sink.placements.len()), (0, 0));
}

#[test]
fn requested_wmo_or_m2_missing_or_malformed_is_fatal() {
    let mut missing_m2 = with_tile_geometry();
    missing_m2.retain(|(id, _, _)| *id != 300);
    let mut missing_wmo = with_tile_geometry();
    missing_wmo.retain(|(id, _, _)| *id != 400);
    let mut malformed_wmo = with_tile_geometry();
    malformed_wmo
        .iter_mut()
        .find(|(id, _, _)| *id == 401)
        .unwrap()
        .2 = b"not a WMO group".to_vec();
    let mut malformed_m2 = with_tile_geometry();
    malformed_m2
        .iter_mut()
        .find(|(id, _, _)| *id == 300)
        .unwrap()
        .2 = b"not an M2".to_vec();

    for (files, code) in [
        (&missing_m2, "file_unavailable"),
        (&missing_wmo, "file_unavailable"),
        (&malformed_wmo, "wmo_framing"),
        (&malformed_m2, "vmap_missing_md20"),
    ] {
        let mut sink = Stage::default();
        // Earlier callbacks can only be provisional sink staging; failure
        // prevents a Completed domain from being returned.
        let error = match run_fixture(files, 7, &mut sink) {
            Ok(_) => panic!("missing or malformed model unexpectedly passed"),
            Err(error) => error,
        };
        assert!(matches!(error, Error::Available(_)));
        assert_eq!(error.code(), code);
    }
}

#[test]
fn global_vmo_model_profile_still_selects_temp_list_rows() {
    assert!(models::ModelScope::Global.includes_startup_temp_list());
    assert!(!models::ModelScope::PlacementOnly.includes_startup_temp_list());
    assert_eq!(models::ModelScope::PlacementOnly.as_str(), "placement_only");
    // The existing integration control exercises the global TempList union:
    // startup_duplicates_omissions_and_no_collision_preserve_list_union.
    // It remains part of this crate's full library-test acceptance.
}

#[test]
fn all_with_root_absence_keeps_new_profile_and_present_empty_map_without_omissions() {
    let mut files = files_70450(&[(7, 100, -1)]);
    files.push((100, None, fixture::wdt(None, None)));
    let mut sink = Stage::default();
    let out = run_all_with_root_absence(&files, &mut sink).unwrap();

    assert_eq!(out.profile(), AVAILABLE_ROOT_ABSENT_PROFILE);
    assert_eq!(
        out.world().profile(),
        available_70374::PROFILE_70450_AVAILABLE_ROOT_ABSENT_GEOMETRY
    );
    assert_eq!(out.selection(), plan::MapSelection::All);
    assert!(out.world().root_absent().is_empty());
    assert_eq!((out.report().run.maps, out.report().run.wdt_files), (1, 1));
    assert_eq!(out.report().run.placement_callbacks, 1);
    assert_eq!(sink.models, 0);
    assert_eq!(sink.placements.len(), 1);
    assert_eq!(sink.placements[0].0.kind, run::Kind::Global);
    assert!(sink.placements[0].1.is_empty());
    assert_eq!(out.world().startup().data().table().file_data_id(), 0);
    assert_eq!(out.world().startup().data().startup_facts().reads, 0);
}

#[test]
fn all_with_root_absence_declares_raw_missing_wdt_while_previous_route_rejects() {
    let files = files_70450(&[(7, 100, -1)]);
    let mut sink = Stage::default();
    let out = run_all_with_root_absence(&files, &mut sink).unwrap();

    assert_eq!(out.profile(), AVAILABLE_ROOT_ABSENT_PROFILE);
    assert_eq!(
        out.world().root_absent(),
        &[run::RootAbsent {
            map_id: 7,
            wdt_file_data_id: 100,
        }]
    );
    assert_eq!((out.report().run.maps, out.report().run.wdt_files), (1, 0));
    assert_eq!(out.report().run.tile_positions, 4096);
    assert_eq!(out.report().run.ids.calls, 0);
    assert_eq!((sink.models, sink.placements.len()), (0, 0));

    let fixture = CollisionFixture::for_build(&files, 70450);
    let mut store = open(&fixture);
    let mut sink = Stage::default();
    let error = match extract_all(&mut store, &mut sink, Limits::default()) {
        Ok(_) => panic!("previous strict route accepted a missing WDT"),
        Err(error) => error,
    };
    assert_eq!(error.code(), "file_unavailable");
    assert_eq!((sink.models, sink.placements.len()), (0, 0));
}

#[test]
fn model_root_absence_profile_is_distinct_even_without_omissions() {
    let mut files = files_70450(&[(7, 100, -1)]);
    files.push((100, None, fixture::wdt(None, None)));
    let mut sink = Stage::default();
    let out = run_all_with_model_root_absence(&files, &mut sink).unwrap();
    assert_eq!(out.profile(), AVAILABLE_MODEL_ROOT_ABSENT_PROFILE);
    assert_eq!(
        out.world().profile(),
        available_70374::PROFILE_70450_AVAILABLE_MODEL_ROOT_ABSENT_GEOMETRY
    );
    assert!(out.world().root_absent().is_empty());
    assert!(out.world().missing_m2().is_empty());
    assert_eq!((out.report().run.maps, out.report().run.wdt_files), (1, 1));
    assert_eq!((sink.models, sink.placements.len()), (0, 1));

    let mut sink = Stage::default();
    let previous = run_all_with_root_absence(&files, &mut sink).unwrap();
    assert_eq!(previous.profile(), AVAILABLE_ROOT_ABSENT_PROFILE);
    assert!(previous.world().missing_m2().is_empty());
}

#[test]
fn model_root_absence_retains_wmo_root_and_placement() {
    let mut files = files_70450(&[(7, 100, -1)]);
    files.extend([
        (100, None, fixture::wdt(Some(10), None)),
        (10, None, fixture::wmo_root(30, Some(20))),
        (30, None, fixture::wmo_group(true, true, false)),
    ]);
    let mut sink = Stage::default();
    let out = run_all_with_model_root_absence(&files, &mut sink).unwrap();
    assert_eq!(out.profile(), AVAILABLE_MODEL_ROOT_ABSENT_PROFILE);
    assert!(out.world().root_absent().is_empty());
    assert_eq!(out.world().missing_m2(), &[20]);
    assert_eq!(out.report().run.model_callbacks, 1);
    assert_eq!(out.report().run.ids.calls, 1);
    assert!((sink.models, sink.placements.len()) == (1, 1));
}

#[test]
fn vacated_wmo_geometry_profile_uses_request_heavy_and_emits_no_wmo_placement() {
    let mut files = files_70450(&[(7, 100, -1)]);
    files.extend([
        (100, None, fixture::wdt(Some(10), None)),
        (10, None, fixture::wmo_root(30, Some(20))),
        (30, None, fixture::wmo_group(false, true, false)),
    ]);
    let mut sink = Stage::default();
    let out = run_all_with_vacated_wmo(&files, &mut sink).unwrap();
    assert_eq!(out.profile(), AVAILABLE_VACATED_WMO_PROFILE);
    assert_eq!(out.world().profile(), AVAILABLE_VACATED_WMO_PROFILE);
    assert_eq!(out.world().missing_m2(), &[20]);
    assert_eq!(out.world().missing_wmo(), &[10]);
    assert!(out.world().empty_wmo().is_empty());
    assert_eq!(out.report().run.model_callbacks, 1);
    assert_eq!(out.report().run.no_collision_models, 1);
    assert_eq!(
        (
            out.report().run.placement_records,
            out.report().run.ids.calls
        ),
        (0, 0)
    );
    assert_eq!(sink.models, 1);
    assert!(sink.placements.iter().all(|(_, bytes)| bytes.is_empty()));
}

#[test]
fn empty_wmo_geometry_profile_reports_intrinsic_empty_fdids_separately() {
    let mut files = files_70450(&[(7, 100, -1)]);
    let mut root = fixture::wmo_root(30, None);
    let mut mods: Vec<_> = b"MODS".iter().rev().copied().collect();
    mods.extend_from_slice(&32u32.to_le_bytes());
    mods.extend_from_slice(&[0; 32]);
    root.extend(mods);
    files.extend([
        (100, None, fixture::wdt(Some(10), None)),
        (10, None, root),
        (30, None, fixture::wmo_group(false, false, false)),
    ]);
    let mut sink = Stage::default();
    let out = run_all_with_empty_wmo(&files, &mut sink).unwrap();
    assert_eq!(out.profile(), AVAILABLE_EMPTY_WMO_PROFILE);
    assert_eq!(out.world().profile(), AVAILABLE_EMPTY_WMO_PROFILE);
    assert!(out.world().root_absent().is_empty());
    assert!(out.world().missing_m2().is_empty());
    assert!(out.world().missing_wmo().is_empty());
    assert_eq!(out.world().empty_wmo(), &[10]);
    assert_eq!(out.report().run.model_callbacks, 1);
    assert_eq!(out.report().run.no_collision_models, 1);
    assert_eq!(
        (
            out.report().run.placement_records,
            out.report().run.ids.calls
        ),
        (0, 0)
    );
    assert_eq!(sink.models, 1);
    assert!(sink.placements.iter().all(|(_, bytes)| bytes.is_empty()));
}

#[test]
fn placement_expanded_material_filtered_wmo_profile_is_public_and_preserves_omissions() {
    let mut files = files_70450(&[(7, 100, -1)]);
    let mut root = fixture::wmo_root(30, None);
    let mut mods: Vec<_> = b"MODS".iter().rev().copied().collect();
    mods.extend_from_slice(&32u32.to_le_bytes());
    mods.extend_from_slice(&[0; 32]);
    root.extend(mods);
    let mut group = fixture::wmo_group(true, false, false);
    group[84..86].copy_from_slice(&0x26u16.to_le_bytes());
    files.extend([
        (100, None, fixture::wdt(Some(10), None)),
        (10, None, root),
        (30, None, group),
    ]);

    let mut baseline_sink = Stage::default();
    let baseline = run_all_with_material_filtered_wmo(&files, &mut baseline_sink).unwrap();
    let mut v44_sink = Stage::default();
    let v44 = run_all_with_source_expanded_v2_material_filtered_wmo(&files, &mut v44_sink).unwrap();
    let mut sink = Stage::default();
    let out = run_all_with_placement_expanded_material_filtered_wmo(&files, &mut sink).unwrap();

    assert_eq!(
        out.profile(),
        AVAILABLE_PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_PROFILE
    );
    assert_eq!(
        out.world().profile(),
        AVAILABLE_PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_PROFILE
    );
    assert_eq!(out.world().root_absent(), baseline.world().root_absent());
    assert_eq!(out.world().missing_m2(), baseline.world().missing_m2());
    assert_eq!(out.world().missing_wmo(), baseline.world().missing_wmo());
    assert_eq!(out.world().empty_wmo(), baseline.world().empty_wmo());
    assert_eq!(
        out.world().unselected_wmo(),
        baseline.world().unselected_wmo()
    );
    assert_eq!(out.world().material_filtered_wmo(), &[10]);
    assert_eq!(
        out.world().material_filtered_wmo(),
        v44.world().material_filtered_wmo()
    );
    assert_eq!(out.report().run, baseline.report().run);
    assert_eq!(out.report().run, v44.report().run);
    assert_eq!(sink.models, baseline_sink.models);
    assert_eq!(sink.models, v44_sink.models);
    assert_eq!(sink.placements, baseline_sink.placements);
    assert_eq!(sink.placements, v44_sink.placements);
}

#[test]
fn material_filtered_wmo_geometry_profile_is_public_and_disjoint() {
    let mut files = files_70450(&[(7, 100, -1)]);
    let mut root = fixture::wmo_root(30, None);
    let mut mods: Vec<_> = b"MODS".iter().rev().copied().collect();
    mods.extend_from_slice(&32u32.to_le_bytes());
    mods.extend_from_slice(&[0; 32]);
    root.extend(mods);
    let mut group = fixture::wmo_group(true, false, false);
    group[84..86].copy_from_slice(&0x26u16.to_le_bytes());
    files.extend([
        (100, None, fixture::wdt(Some(10), None)),
        (10, None, root),
        (30, None, group),
    ]);
    let mut sink = Stage::default();
    let out = run_all_with_material_filtered_wmo(&files, &mut sink).unwrap();
    assert_eq!(out.profile(), AVAILABLE_MATERIAL_FILTERED_WMO_PROFILE);
    assert_eq!(
        out.world().profile(),
        AVAILABLE_MATERIAL_FILTERED_WMO_PROFILE
    );
    assert!(out.world().root_absent().is_empty());
    assert!(out.world().missing_m2().is_empty());
    assert!(out.world().missing_wmo().is_empty());
    assert!(out.world().empty_wmo().is_empty());
    assert!(out.world().unselected_wmo().is_empty());
    assert_eq!(out.world().material_filtered_wmo(), &[10]);
    assert_eq!(out.report().run.no_collision_models, 1);
    assert_eq!(
        (
            out.report().run.placement_records,
            out.report().run.ids.calls
        ),
        (0, 0)
    );
    assert_eq!(sink.models, 1);
    assert!(sink.placements.iter().all(|(_, bytes)| bytes.is_empty()));
}

#[test]
fn dependency_heavy_material_filtered_wmo_profile_is_public_and_distinct() {
    let mut files = files_70450(&[(7, 100, -1)]);
    let mut root = fixture::wmo_root(30, None);
    let mut mods: Vec<_> = b"MODS".iter().rev().copied().collect();
    mods.extend_from_slice(&32u32.to_le_bytes());
    mods.extend_from_slice(&[0; 32]);
    root.extend(mods);
    let mut group = fixture::wmo_group(true, false, false);
    group[84..86].copy_from_slice(&0x26u16.to_le_bytes());
    files.extend([
        (100, None, fixture::wdt(Some(10), None)),
        (10, None, root),
        (30, None, group),
    ]);

    let mut sink = Stage::default();
    let out = run_all_with_dependency_heavy_material_filtered_wmo(&files, &mut sink).unwrap();
    assert_eq!(
        out.profile(),
        AVAILABLE_DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_PROFILE
    );
    assert_eq!(
        out.world().profile(),
        AVAILABLE_DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_PROFILE
    );
    assert!(out.world().root_absent().is_empty());
    assert!(out.world().missing_m2().is_empty());
    assert!(out.world().missing_wmo().is_empty());
    assert!(out.world().empty_wmo().is_empty());
    assert!(out.world().unselected_wmo().is_empty());
    assert_eq!(out.world().material_filtered_wmo(), &[10]);
    assert_eq!(out.report().run.no_collision_models, 1);
    assert_eq!(out.report().run.ids.calls, 0);
    assert_eq!(sink.models, 1);
    assert_eq!(sink.placements.len(), 1);
    assert!(sink.placements.iter().all(|(_, bytes)| bytes.is_empty()));

    let mut v36_sink = Stage::default();
    let v36_fixture = CollisionFixture::for_build(&files, 70450);
    let mut v36_store = open(&v36_fixture);
    let v36 =
        extract_all_with_material_filtered_wmo(&mut v36_store, &mut v36_sink, Limits::default())
            .unwrap();
    assert_eq!(v36.profile(), AVAILABLE_MATERIAL_FILTERED_WMO_PROFILE);
    assert_eq!(v36.world().material_filtered_wmo(), &[10]);
    assert_eq!(v36_sink.models, sink.models);
    assert_eq!(v36_sink.placements, sink.placements);
}

#[test]
fn source_heavy_material_filtered_wmo_profile_is_public_and_distinct() {
    let files = {
        let mut files = files_70450(&[(7, 100, -1)]);
        let mut root = fixture::wmo_root(30, None);
        let mut mods: Vec<_> = b"MODS".iter().rev().copied().collect();
        mods.extend_from_slice(&32u32.to_le_bytes());
        mods.extend_from_slice(&[0; 32]);
        root.extend(mods);
        let mut group = fixture::wmo_group(true, false, false);
        group[84..86].copy_from_slice(&0x26u16.to_le_bytes());
        files.extend([
            (100, None, fixture::wdt(Some(10), None)),
            (10, None, root),
            (30, None, group),
        ]);
        files
    };

    let mut sink = Stage::default();
    let out = run_all_with_source_heavy_material_filtered_wmo(&files, &mut sink).unwrap();
    assert_eq!(
        out.profile(),
        AVAILABLE_SOURCE_HEAVY_MATERIAL_FILTERED_WMO_PROFILE
    );
    assert_eq!(
        out.world().profile(),
        AVAILABLE_SOURCE_HEAVY_MATERIAL_FILTERED_WMO_PROFILE
    );
    assert!(out.world().root_absent().is_empty());
    assert!(out.world().missing_m2().is_empty());
    assert!(out.world().missing_wmo().is_empty());
    assert!(out.world().empty_wmo().is_empty());
    assert!(out.world().unselected_wmo().is_empty());
    assert_eq!(out.world().material_filtered_wmo(), &[10]);
    assert_eq!(out.report().run.no_collision_models, 1);
    assert_eq!(out.report().run.ids.calls, 0);
    assert_eq!(sink.models, 1);
    assert_eq!(sink.placements.len(), 1);
    assert!(sink.placements.iter().all(|(_, bytes)| bytes.is_empty()));

    let fixture = CollisionFixture::for_build(&files, 70450);
    let mut store = open(&fixture);
    let mut alias_sink = Stage::default();
    let alias = extract_all_with_source_heavy_material_filtered_wmo_70450(
        &mut store,
        &mut alias_sink,
        Limits::default(),
    )
    .unwrap();
    assert_eq!(
        alias.profile(),
        AVAILABLE_SOURCE_HEAVY_MATERIAL_FILTERED_WMO_PROFILE
    );
    assert_eq!(alias.world().material_filtered_wmo(), &[10]);
    assert_eq!(alias_sink.models, sink.models);
    assert_eq!(alias_sink.placements, sink.placements);

    let mut v37_sink = Stage::default();
    let v37 = run_all_with_dependency_heavy_material_filtered_wmo(&files, &mut v37_sink).unwrap();
    assert_eq!(
        v37.profile(),
        AVAILABLE_DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_PROFILE
    );
    assert_eq!(v37.world().material_filtered_wmo(), &[10]);
    assert_eq!(v37_sink.models, sink.models);
    assert_eq!(v37_sink.placements, sink.placements);

    let mut v36_sink = Stage::default();
    let v36 = run_all_with_material_filtered_wmo(&files, &mut v36_sink).unwrap();
    assert_eq!(v36.profile(), AVAILABLE_MATERIAL_FILTERED_WMO_PROFILE);
    assert_eq!(v36.world().material_filtered_wmo(), &[10]);
    assert_eq!(v36_sink.models, sink.models);
    assert_eq!(v36_sink.placements, sink.placements);

    let mut v39_sink = Stage::default();
    let v39 = run_all_with_read_heavy_material_filtered_wmo(&files, &mut v39_sink).unwrap();
    assert_eq!(
        v39.profile(),
        AVAILABLE_READ_HEAVY_MATERIAL_FILTERED_WMO_PROFILE
    );
    assert_eq!(
        v39.world().profile(),
        AVAILABLE_READ_HEAVY_MATERIAL_FILTERED_WMO_PROFILE
    );
    assert_eq!(
        v39.world().material_filtered_wmo(),
        out.world().material_filtered_wmo()
    );
    assert_eq!(v39.world().root_absent(), out.world().root_absent());
    assert_eq!(v39.world().missing_m2(), out.world().missing_m2());
    assert_eq!(v39.world().missing_wmo(), out.world().missing_wmo());
    assert_eq!(v39.world().empty_wmo(), out.world().empty_wmo());
    assert_eq!(v39.world().unselected_wmo(), out.world().unselected_wmo());
    assert_eq!(v39.report().run, out.report().run);
    assert_eq!(v39_sink.models, sink.models);
    assert_eq!(v39_sink.placements, sink.placements);

    let fixture = CollisionFixture::for_build(&files, 70450);
    let mut store = open(&fixture);
    let mut alias_sink = Stage::default();
    let alias = extract_all_with_read_heavy_material_filtered_wmo_70450(
        &mut store,
        &mut alias_sink,
        Limits::default(),
    )
    .unwrap();
    assert_eq!(
        alias.profile(),
        AVAILABLE_READ_HEAVY_MATERIAL_FILTERED_WMO_PROFILE
    );
    assert_eq!(
        alias.world().material_filtered_wmo(),
        v39.world().material_filtered_wmo()
    );
    assert_eq!(alias.report().run, v39.report().run);
    assert_eq!(alias_sink.models, v39_sink.models);
    assert_eq!(alias_sink.placements, v39_sink.placements);

    let mut v40_sink = Stage::default();
    let v40 = run_all_with_model_heavy_material_filtered_wmo(&files, &mut v40_sink).unwrap();
    assert_eq!(
        v40.profile(),
        AVAILABLE_MODEL_HEAVY_MATERIAL_FILTERED_WMO_PROFILE
    );
    assert_eq!(
        v40.world().profile(),
        AVAILABLE_MODEL_HEAVY_MATERIAL_FILTERED_WMO_PROFILE
    );
    assert_eq!(
        v40.world().material_filtered_wmo(),
        out.world().material_filtered_wmo()
    );
    assert_eq!(v40.world().root_absent(), out.world().root_absent());
    assert_eq!(v40.world().missing_m2(), out.world().missing_m2());
    assert_eq!(v40.world().missing_wmo(), out.world().missing_wmo());
    assert_eq!(v40.world().empty_wmo(), out.world().empty_wmo());
    assert_eq!(v40.world().unselected_wmo(), out.world().unselected_wmo());
    assert_eq!(v40.report().run, out.report().run);
    assert_eq!(v40_sink.models, sink.models);
    assert_eq!(v40_sink.placements, sink.placements);

    let mut v41_sink = Stage::default();
    let v41 = run_all_with_source_expanded_material_filtered_wmo(&files, &mut v41_sink).unwrap();
    assert_eq!(
        v41.profile(),
        AVAILABLE_SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_PROFILE
    );
    assert_eq!(
        v41.world().profile(),
        AVAILABLE_SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_PROFILE
    );
    assert_eq!(
        v41.world().material_filtered_wmo(),
        out.world().material_filtered_wmo()
    );
    assert_eq!(v41.world().root_absent(), out.world().root_absent());
    assert_eq!(v41.world().missing_m2(), out.world().missing_m2());
    assert_eq!(v41.world().missing_wmo(), out.world().missing_wmo());
    assert_eq!(v41.world().empty_wmo(), out.world().empty_wmo());
    assert_eq!(v41.world().unselected_wmo(), out.world().unselected_wmo());
    assert_eq!(v41.report().run, out.report().run);
    assert_eq!(v41_sink.models, sink.models);
    assert_eq!(v41_sink.placements, sink.placements);

    let fixture = CollisionFixture::for_build(&files, 70450);
    let mut store = open(&fixture);
    let mut alias_sink = Stage::default();
    let alias = extract_all_with_source_expanded_material_filtered_wmo_70450(
        &mut store,
        &mut alias_sink,
        Limits::default(),
    )
    .unwrap();
    assert_eq!(
        alias.profile(),
        AVAILABLE_SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_PROFILE
    );
    assert_eq!(
        alias.world().material_filtered_wmo(),
        v41.world().material_filtered_wmo()
    );
    assert_eq!(alias.report().run, v41.report().run);
    assert_eq!(alias_sink.models, v41_sink.models);
    assert_eq!(alias_sink.placements, v41_sink.placements);

    let mut v42_sink = Stage::default();
    let v42 =
        run_all_with_model_output_expanded_material_filtered_wmo(&files, &mut v42_sink).unwrap();
    assert_eq!(
        v42.profile(),
        AVAILABLE_MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_PROFILE
    );
    assert_eq!(
        v42.world().profile(),
        AVAILABLE_MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_PROFILE
    );
    assert_eq!(
        v42.world().material_filtered_wmo(),
        v41.world().material_filtered_wmo()
    );
    assert_eq!(v42.world().root_absent(), v41.world().root_absent());
    assert_eq!(v42.world().missing_m2(), v41.world().missing_m2());
    assert_eq!(v42.world().missing_wmo(), v41.world().missing_wmo());
    assert_eq!(v42.world().empty_wmo(), v41.world().empty_wmo());
    assert_eq!(v42.world().unselected_wmo(), v41.world().unselected_wmo());
    assert_eq!(v42.report().run, v41.report().run);
    assert_eq!(v42_sink.models, v41_sink.models);
    assert_eq!(v42_sink.placements, v41_sink.placements);

    let fixture = CollisionFixture::for_build(&files, 70450);
    let mut store = open(&fixture);
    let mut alias_sink = Stage::default();
    let alias = extract_all_with_model_output_expanded_material_filtered_wmo_70450(
        &mut store,
        &mut alias_sink,
        Limits::default(),
    )
    .unwrap();
    assert_eq!(
        alias.profile(),
        AVAILABLE_MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_PROFILE
    );
    assert_eq!(
        alias.world().material_filtered_wmo(),
        v42.world().material_filtered_wmo()
    );
    assert_eq!(alias.report().run, v42.report().run);
    assert_eq!(alias_sink.models, v42_sink.models);
    assert_eq!(alias_sink.placements, v42_sink.placements);

    let mut v43_sink = Stage::default();
    let v43 =
        run_all_with_model_bytes_expanded_material_filtered_wmo(&files, &mut v43_sink).unwrap();
    assert_eq!(
        v43.profile(),
        AVAILABLE_MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_PROFILE
    );
    assert_eq!(
        v43.world().profile(),
        AVAILABLE_MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_PROFILE
    );
    assert_eq!(
        v43.world().material_filtered_wmo(),
        v42.world().material_filtered_wmo()
    );
    assert_eq!(v43.world().root_absent(), v42.world().root_absent());
    assert_eq!(v43.world().missing_m2(), v42.world().missing_m2());
    assert_eq!(v43.world().missing_wmo(), v42.world().missing_wmo());
    assert_eq!(v43.world().empty_wmo(), v42.world().empty_wmo());
    assert_eq!(v43.world().unselected_wmo(), v42.world().unselected_wmo());
    assert_eq!(v43.report().run, v42.report().run);
    assert_eq!(v43_sink.models, v42_sink.models);
    assert_eq!(v43_sink.placements, v42_sink.placements);

    let fixture = CollisionFixture::for_build(&files, 70450);
    let mut store = open(&fixture);
    let mut alias_sink = Stage::default();
    let alias = extract_all_with_model_bytes_expanded_material_filtered_wmo_70450(
        &mut store,
        &mut alias_sink,
        Limits::default(),
    )
    .unwrap();
    assert_eq!(
        alias.profile(),
        AVAILABLE_MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_PROFILE
    );
    assert_eq!(
        alias.world().material_filtered_wmo(),
        v43.world().material_filtered_wmo()
    );
    assert_eq!(alias.report().run, v43.report().run);
    assert_eq!(alias_sink.models, v43_sink.models);
    assert_eq!(alias_sink.placements, v43_sink.placements);

    let mut v44_sink = Stage::default();
    let v44 = run_all_with_source_expanded_v2_material_filtered_wmo(&files, &mut v44_sink).unwrap();
    assert_eq!(
        v44.profile(),
        AVAILABLE_SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_PROFILE
    );
    assert_eq!(
        v44.world().profile(),
        AVAILABLE_SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_PROFILE
    );
    assert_eq!(
        v44.world().material_filtered_wmo(),
        v43.world().material_filtered_wmo()
    );
    assert_eq!(v44.world().root_absent(), v43.world().root_absent());
    assert_eq!(v44.world().missing_m2(), v43.world().missing_m2());
    assert_eq!(v44.world().missing_wmo(), v43.world().missing_wmo());
    assert_eq!(v44.world().empty_wmo(), v43.world().empty_wmo());
    assert_eq!(v44.world().unselected_wmo(), v43.world().unselected_wmo());
    assert_eq!(v44.report().run, v43.report().run);
    assert_eq!(v44_sink.models, v43_sink.models);
    assert_eq!(v44_sink.placements, v43_sink.placements);

    let fixture = CollisionFixture::for_build(&files, 70450);
    let mut store = open(&fixture);
    let mut alias_sink = Stage::default();
    let alias = extract_all_with_source_expanded_v2_material_filtered_wmo_70450(
        &mut store,
        &mut alias_sink,
        Limits::default(),
    )
    .unwrap();
    assert_eq!(
        alias.profile(),
        AVAILABLE_SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_PROFILE
    );
    assert_eq!(
        alias.world().material_filtered_wmo(),
        v44.world().material_filtered_wmo()
    );
    assert_eq!(alias.report().run, v44.report().run);
    assert_eq!(alias_sink.models, v44_sink.models);
    assert_eq!(alias_sink.placements, v44_sink.placements);

    let mut v45_sink = Stage::default();
    let v45 = run_all_with_placement_expanded_material_filtered_wmo(&files, &mut v45_sink).unwrap();
    assert_eq!(
        v45.profile(),
        AVAILABLE_PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_PROFILE
    );
    assert_eq!(
        v45.world().profile(),
        AVAILABLE_PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_PROFILE
    );
    assert_eq!(
        v45.world().material_filtered_wmo(),
        v44.world().material_filtered_wmo()
    );
    assert_eq!(v45.world().root_absent(), v44.world().root_absent());
    assert_eq!(v45.world().missing_m2(), v44.world().missing_m2());
    assert_eq!(v45.world().missing_wmo(), v44.world().missing_wmo());
    assert_eq!(v45.world().empty_wmo(), v44.world().empty_wmo());
    assert_eq!(v45.world().unselected_wmo(), v44.world().unselected_wmo());
    assert_eq!(v45.report().run, v44.report().run);
    assert_eq!(v45_sink.models, v44_sink.models);
    assert_eq!(v45_sink.placements, v44_sink.placements);

    let fixture = CollisionFixture::for_build(&files, 70450);
    let mut store = open(&fixture);
    let mut alias_sink = Stage::default();
    let alias = extract_all_with_placement_expanded_material_filtered_wmo_70450(
        &mut store,
        &mut alias_sink,
        Limits::default(),
    )
    .unwrap();
    assert_eq!(
        alias.profile(),
        AVAILABLE_PLACEMENT_EXPANDED_MATERIAL_FILTERED_WMO_PROFILE
    );
    assert_eq!(
        alias.world().material_filtered_wmo(),
        v45.world().material_filtered_wmo()
    );
    assert_eq!(alias.report().run, v45.report().run);
    assert_eq!(alias_sink.models, v45_sink.models);
    assert_eq!(alias_sink.placements, v45_sink.placements);
}

#[test]
fn previous_vacated_wmo_profile_still_rejects_intrinsic_empty_wmo() {
    let mut files = files_70450(&[(7, 100, -1)]);
    files.extend([
        (100, None, fixture::wdt(Some(10), None)),
        (10, None, fixture::wmo_root(30, None)),
        (30, None, fixture::wmo_group(false, false, false)),
    ]);
    let fixture = CollisionFixture::for_build(&files, 70450);
    let mut store = open(&fixture);
    let startup = gameobjects::extract_startup_without_godi_with_resources(
        &mut store,
        gameobjects::Limits::default(),
        crate::collision::resources::Profile::RequestHeavyV1,
    )
    .unwrap();
    let error = available_70374::extract_all_with_vacated_wmo_70450(
        startup,
        &mut Stage::default(),
        world::Limits::default(),
    );
    assert_eq!(
        error.err(),
        Some(available_70374::Error::Run(run::Error::Collision(
            crate::collision::Error::NoCollision
        )))
    );
}

#[test]
fn request_capacity_geometry_profile_is_distinct_without_omissions() {
    let mut files = files_70450(&[(7, 100, -1)]);
    files.push((100, None, fixture::wdt(None, None)));
    let mut sink = Stage::default();
    let out = run_all_with_model_root_absence_request_capacity(&files, &mut sink).unwrap();
    assert_eq!(
        out.profile(),
        AVAILABLE_MODEL_ROOT_ABSENT_REQUEST_CAPACITY_PROFILE
    );
    assert_eq!(
        out.world().profile(),
        available_70374::PROFILE_70450_AVAILABLE_MODEL_ROOT_ABSENT_REQUEST_CAPACITY_GEOMETRY
    );
    assert!(out.world().root_absent().is_empty());
    assert!(out.world().missing_m2().is_empty());
    assert_eq!((out.report().run.maps, out.report().run.wdt_files), (1, 1));
    assert_eq!((sink.models, sink.placements.len()), (0, 1));
}
