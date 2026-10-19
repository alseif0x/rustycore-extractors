// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-19: synthetic-only selected collision geometry coverage.
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
    // Poisonous present tables make any accidental GODI/LiquidObject request
    // fail; the selected geometry route must not read either source.
    files.retain(|(id, _, _)| *id != gameobjects::TABLE_FILE_DATA_ID);
    files.push((
        gameobjects::TABLE_FILE_DATA_ID,
        None,
        b"poisonous synthetic GODI must not be read".to_vec(),
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
