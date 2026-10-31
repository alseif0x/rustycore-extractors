// SPDX-License-Identifier: GPL-3.0-or-later
// Authored synthetic-only integration. Existing fixture bodies are reused.
use super::*;
use crate::collision::world::tests as fixture;
use crate::local_store::tests::{CollisionFile, CollisionFixture};
use crate::{Locale, local_index, local_store, root};
use forever_db2::catalog;
use forever_vmap::wmo::raw;

fn put(bytes: &mut [u8], at: usize, n: u32) {
    bytes[at..at + 4].copy_from_slice(&n.to_le_bytes());
}
fn word(bytes: &[u8], at: usize) -> usize {
    u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()) as usize
}
fn files(maps: &[(u32, u32, i16)]) -> Vec<CollisionFile<'static>> {
    let mut files = fixture::files(maps);
    for (id, _, bytes) in &mut files {
        let name = match *id {
            gameobjects::TABLE_FILE_DATA_ID => Some("GameObjectDisplayInfo"),
            world::MAP_FILE_DATA_ID => Some("Map"),
            world::MATERIAL_FILE_DATA_ID => Some("LiquidMaterial"),
            world::TYPE_FILE_DATA_ID => Some("LiquidType"),
            _ => None,
        };
        if let Some(name) = name {
            put(
                bytes,
                152,
                catalog::resolve_for(catalog::WOWDBDEFS_70374_CATALOG_ID, name)
                    .unwrap()
                    .unwrap()
                    .metadata()
                    .table_hash
                    .unwrap(),
            );
        }
    }
    files
}
fn replace_godi(files: &mut [CollisionFile<'_>], rows: &[(u32, u32)], copies: &[(u32, u32)]) {
    let hash = word(&files[0].2, 152) as u32;
    files[0].2 = fixture::gameobject_db2(rows, copies);
    put(&mut files[0].2, 152, hash);
}
fn open(f: &CollisionFixture) -> LocalStore {
    let path = f.path().join(".build.info");
    let text = std::fs::read_to_string(&path).unwrap();
    std::fs::write(path, text.replace("1.60.1.70205", "1.60.1.70374")).unwrap();
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
fn files_70450(maps: &[(u32, u32, i16)]) -> Vec<CollisionFile<'static>> {
    let mut files = files(maps);
    for (id, _, bytes) in &mut files {
        let name = match *id {
            gameobjects::TABLE_FILE_DATA_ID => Some("GameObjectDisplayInfo"),
            world::MAP_FILE_DATA_ID => Some("Map"),
            world::MATERIAL_FILE_DATA_ID => Some("LiquidMaterial"),
            world::TYPE_FILE_DATA_ID => Some("LiquidType"),
            _ => None,
        };
        if let Some(name) = name {
            put(
                bytes,
                152,
                catalog::resolve_for(catalog::WOWDBDEFS_70450_CATALOG_ID, name)
                    .unwrap()
                    .unwrap()
                    .metadata()
                    .table_hash
                    .unwrap(),
            );
        }
    }
    files
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
fn root_absence_result(files: &[CollisionFile<'_>], sink: &mut Stage) -> Result<Completed, Error> {
    let fixture = CollisionFixture::for_build(files, 70450);
    let mut store = open_70450(&fixture);
    let startup =
        gameobjects::extract_startup_without_godi(&mut store, gameobjects::Limits::default())
            .unwrap();
    extract_all_with_root_absence_70450(startup, sink, world::Limits::default())
}
fn model_root_absence_result(
    files: &[CollisionFile<'_>],
    sink: &mut Stage,
) -> Result<Completed, Error> {
    let fixture = CollisionFixture::for_build(files, 70450);
    let mut store = open_70450(&fixture);
    let startup =
        gameobjects::extract_startup_without_godi(&mut store, gameobjects::Limits::default())
            .unwrap();
    extract_all_with_model_root_absence_70450(startup, sink, world::Limits::default())
}
fn unselected_wmo_result(
    files: &[CollisionFile<'_>],
    sink: &mut Stage,
) -> Result<Completed, Error> {
    let fixture = CollisionFixture::for_build(files, 70450);
    let mut store = open_70450(&fixture);
    let startup = gameobjects::extract_startup_without_godi_with_resources(
        &mut store,
        gameobjects::Limits::default(),
        crate::collision::resources::Profile::RequestHeavyV1,
    )
    .unwrap();
    extract_all_with_unselected_wmo_70450(startup, sink, world::Limits::default())
}
fn material_filtered_wmo_result(
    files: &[CollisionFile<'_>],
    sink: &mut Stage,
) -> Result<Completed, Error> {
    let fixture = CollisionFixture::for_build(files, 70450);
    let mut store = open_70450(&fixture);
    let startup = gameobjects::extract_startup_without_godi_with_resources(
        &mut store,
        gameobjects::Limits::default(),
        crate::collision::resources::Profile::RequestHeavyV1,
    )
    .unwrap();
    extract_all_with_material_filtered_wmo_70450(startup, sink, world::Limits::default())
}
fn dependency_heavy_material_filtered_wmo_result(
    files: &[CollisionFile<'_>],
    sink: &mut Stage,
) -> Result<Completed, Error> {
    let fixture = CollisionFixture::for_build(files, 70450);
    let mut store = open_70450(&fixture);
    let startup = gameobjects::extract_startup_without_godi_with_resources(
        &mut store,
        gameobjects::Limits::default(),
        crate::collision::resources::Profile::DependencyHeavyV1,
    )
    .unwrap();
    extract_all_with_dependency_heavy_material_filtered_wmo_70450(
        startup,
        sink,
        world::Limits::default(),
    )
}
fn source_heavy_material_filtered_wmo_result(
    files: &[CollisionFile<'_>],
    sink: &mut Stage,
) -> Result<Completed, Error> {
    let fixture = CollisionFixture::for_build(files, 70450);
    let mut store = open_70450(&fixture);
    let startup = gameobjects::extract_startup_without_godi_with_resources(
        &mut store,
        gameobjects::Limits::default(),
        crate::collision::resources::Profile::SourceHeavyV1,
    )
    .unwrap();
    extract_all_with_source_heavy_material_filtered_wmo_70450(
        startup,
        sink,
        world::Limits::default(),
    )
}
fn read_heavy_material_filtered_wmo_result(
    files: &[CollisionFile<'_>],
    sink: &mut Stage,
) -> Result<Completed, Error> {
    let fixture = CollisionFixture::for_build(files, 70450);
    let mut store = open_70450(&fixture);
    let startup = gameobjects::extract_startup_without_godi_with_resources(
        &mut store,
        gameobjects::Limits::default(),
        crate::collision::resources::Profile::ReadHeavyV1,
    )
    .unwrap();
    extract_all_with_read_heavy_material_filtered_wmo_70450(startup, sink, world::Limits::default())
}
fn model_heavy_material_filtered_wmo_result(
    files: &[CollisionFile<'_>],
    sink: &mut Stage,
) -> Result<Completed, Error> {
    let fixture = CollisionFixture::for_build(files, 70450);
    let mut store = open_70450(&fixture);
    let startup = gameobjects::extract_startup_without_godi_with_resources(
        &mut store,
        gameobjects::Limits::default(),
        crate::collision::resources::Profile::ModelHeavyV1,
    )
    .unwrap();
    extract_all_with_model_heavy_material_filtered_wmo_70450(
        startup,
        sink,
        world::Limits::default(),
    )
}
fn model_output_expanded_material_filtered_wmo_result(
    files: &[CollisionFile<'_>],
    sink: &mut Stage,
) -> Result<Completed, Error> {
    let fixture = CollisionFixture::for_build(files, 70450);
    let mut store = open_70450(&fixture);
    let startup = gameobjects::extract_startup_without_godi_with_resources(
        &mut store,
        gameobjects::Limits::default(),
        crate::collision::resources::Profile::ModelOutputExpandedV1,
    )
    .unwrap();
    extract_all_with_model_output_expanded_material_filtered_wmo_70450(
        startup,
        sink,
        world::Limits::default(),
    )
}
fn model_bytes_expanded_material_filtered_wmo_result(
    files: &[CollisionFile<'_>],
    sink: &mut Stage,
) -> Result<Completed, Error> {
    let fixture = CollisionFixture::for_build(files, 70450);
    let mut store = open_70450(&fixture);
    let startup = gameobjects::extract_startup_without_godi_with_resources(
        &mut store,
        gameobjects::Limits::default(),
        crate::collision::resources::Profile::ModelBytesExpandedV1,
    )
    .unwrap();
    extract_all_with_model_bytes_expanded_material_filtered_wmo_70450(
        startup,
        sink,
        world::Limits::default(),
    )
}
fn source_expanded_v2_material_filtered_wmo_result(
    files: &[CollisionFile<'_>],
    sink: &mut Stage,
) -> Result<Completed, Error> {
    let fixture = CollisionFixture::for_build(files, 70450);
    let mut store = open_70450(&fixture);
    let startup = gameobjects::extract_startup_without_godi_with_resources(
        &mut store,
        gameobjects::Limits::default(),
        crate::collision::resources::Profile::SourceExpandedV2,
    )
    .unwrap();
    extract_all_with_source_expanded_v2_material_filtered_wmo_70450(
        startup,
        sink,
        world::Limits::default(),
    )
}
fn source_expanded_material_filtered_wmo_result(
    files: &[CollisionFile<'_>],
    sink: &mut Stage,
) -> Result<Completed, Error> {
    let fixture = CollisionFixture::for_build(files, 70450);
    let mut store = open_70450(&fixture);
    let startup = gameobjects::extract_startup_without_godi_with_resources(
        &mut store,
        gameobjects::Limits::default(),
        crate::collision::resources::Profile::SourceExpandedV1,
    )
    .unwrap();
    extract_all_with_source_expanded_material_filtered_wmo_70450(
        startup,
        sink,
        world::Limits::default(),
    )
}
fn model_root_absence_request_capacity_result(
    files: &[CollisionFile<'_>],
    sink: &mut Stage,
) -> Result<Completed, Error> {
    let fixture = CollisionFixture::for_build(files, 70450);
    let mut store = open_70450(&fixture);
    let startup = gameobjects::extract_startup_without_godi_with_resources(
        &mut store,
        gameobjects::Limits::default(),
        crate::collision::resources::Profile::RequestHeavyV1,
    )
    .unwrap();
    extract_all_with_model_root_absence_request_capacity_70450(
        startup,
        sink,
        world::Limits::default(),
    )
}
// Second nonzero-TACT section is declared but its records/copies are arbitrary.
// These N-framed fixtures are whole-CKey verified; DB2 admission is nevertheless
// partial. This does not invent encoded-key authentication or encrypted I/O.
fn unknown(mut bytes: Vec<u8>, text: bool, copies: usize) -> Vec<u8> {
    let start = word(&bytes, 212);
    let rows = word(&bytes, 136);
    let size = word(&bytes, 144);
    if text {
        for i in 0..rows {
            for field in [0, 4] {
                let at = start + i * size + field;
                let n = word(&bytes, at);
                put(&mut bytes, at, (n + size) as u32);
            }
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
    put(&mut out, 280, copies as u32);
    out.extend(vec![0xa5; size]);
    out.extend(99u32.to_le_bytes());
    out.extend(vec![0xa5; copies * 8]);
    out
}
#[derive(Default)]
struct Stage {
    models: usize,
    placements: Vec<(run::Context, Vec<u8>)>,
    fail: bool,
}
impl run::Sink for Stage {
    fn model(&mut self, _: &Model) -> Result<(), run::SinkError> {
        if self.fail {
            return Err(run::SinkError::Rejected);
        }
        self.models += 1;
        Ok(())
    }
    fn placements(&mut self, context: run::Context, bytes: &[u8]) -> Result<(), run::SinkError> {
        if self.fail {
            return Err(run::SinkError::Rejected);
        }
        self.placements.push((context, bytes.to_vec()));
        Ok(())
    }
}
fn world_result(files: &[CollisionFile<'_>], limits: world::Limits) -> Result<Completed, Error> {
    let f = CollisionFixture::new(files);
    let mut store = open(&f);
    let startup = extract_startup_fdid(&mut store, gameobjects::Limits::default())?;
    extract(startup, &mut Stage::default(), limits)
}

#[test]
fn empty_startup_and_inactive_map_finish_with_distinct_facts() {
    let out = world_result(&files(&[(7, 0, -1)]), world::Limits::default()).unwrap();
    assert_eq!(out.profile(), PROFILE);
    assert_eq!(out.map_stats().rows, 1);
    assert_eq!(out.topology().direct_count(), 1);
    assert!(!out.topology().get(7).unwrap().active());
    assert_eq!(out.startup().extraction().facts().reads, 4);
    assert!(out.map().full_ckey_verified);
    assert!(!out.map().encoded_key_authenticated);
    assert!(out.map().all_sections_admitted());
    assert_eq!(out.startup().data().lists().temp_bytes(), b"VMAP04E\0");
}
#[test]
fn godi_direct_duplicates_zero_and_copies_preserve_original_engine_semantics() {
    let mut fs = files(&[]);
    replace_godi(&mut fs, &[(10, 50), (11, 0), (12, 50)], &[(20, 10)]);
    fs.push((50, None, fixture::m2(true)));
    let f = CollisionFixture::new(&fs);
    let mut store = open(&f);
    let out = extract_startup_fdid(&mut store, gameobjects::Limits::default())
        .unwrap()
        .finish()
        .unwrap();
    assert_eq!(
        out.data()
            .outcomes()
            .iter()
            .map(|r| (r.ordinal(), r.display_id()))
            .collect::<Vec<_>>(),
        [(0, 10), (1, 11), (2, 12)]
    );
    assert_eq!(out.data().stats().copy_records, 1);
    assert_eq!(out.data().stats().collision_rows, 2);
    assert_eq!(out.data().stats().zero_fdids, 1);
    assert_eq!(out.extraction().models().len(), 1);
    assert_eq!(out.extraction().facts().reads, 2);
}
#[test]
fn unknown_godi_section_is_disclosed_and_never_becomes_a_candidate() {
    let mut fs = files(&[]);
    replace_godi(&mut fs, &[(10, 0)], &[]);
    fs[0].2 = unknown(fs[0].2.clone(), false, 1);
    let f = CollisionFixture::new(&fs);
    let mut store = open(&f);
    let out = extract_startup_fdid(&mut store, gameobjects::Limits::default())
        .unwrap()
        .finish()
        .unwrap();
    assert_eq!(out.data().table().declared_direct_rows, 2);
    assert_eq!(out.data().table().direct_rows, 1);
    assert_eq!(out.data().table().declared_copy_records, 1);
    assert_eq!(out.data().stats().copy_records, 0);
    assert!(!out.data().table().all_sections_admitted());
    assert!(out.data().table().full_ckey_verified);
    assert!(!out.data().table().encoded_key_authenticated);
    assert_eq!(out.extraction().facts().reads, 1);
}
#[test]
fn unknown_map_section_retains_wdt_zero_rows_and_coverage() {
    let mut fs = files(&[(7, 0, -1)]);
    fs[1].2 = unknown(fs[1].2.clone(), true, 1);
    let out = world_result(&fs, world::Limits::default()).unwrap();
    assert_eq!(out.map().declared_direct_rows, 2);
    assert_eq!(out.map().direct_rows, 1);
    assert_eq!(out.map().copy_records, 0);
    assert!(!out.map().all_sections_admitted());
    assert_eq!(out.topology().all().len(), 1);
    assert_eq!(out.topology().all()[0].id(), 7);
}
#[test]
fn map_copy_replay_keeps_direct_prefix_and_never_uses_copy_as_source() {
    let mut fs = files(&[(7, 0, -1)]);
    let hash = word(&fs[1].2, 152) as u32;
    fs[1].2 = fixture::map_db2(&[(7, 0, -1)], &[(8, 7), (9, 8)]);
    put(&mut fs[1].2, 152, hash);
    let out = world_result(&fs, world::Limits::default()).unwrap();
    assert_eq!(out.map().copy_records, 2);
    assert_eq!(out.map().materialized_rows, 2);
    assert_eq!(out.topology().direct_count(), 1);
    assert_eq!(
        out.topology()
            .all()
            .iter()
            .map(|r| r.id())
            .collect::<Vec<_>>(),
        [7, 8]
    );
}
#[test]
fn negative_parents_and_inactive_direct_parent_remain_in_graph() {
    let mut fs = files(&[(7, 0, -2), (8, 100, 7)]);
    fs.push((100, None, fixture::wdt(None, None)));
    // Retaining an inactive parent in the DB2 graph does not admit fallback
    // through that parent during strict dependency execution.
    let f = CollisionFixture::new(&fs);
    let mut store = open(&f);
    let file = store
        .read_file_data_id_available_with_limit(
            world::MAP_FILE_DATA_ID,
            world::MAX_DB2_BYTES as u64,
        )
        .unwrap();
    let owner = db2::read_vmap_maps_70374(&file)
        .unwrap()
        .materialize()
        .unwrap();
    let maps = plan::Maps::from_materialized(
        owner.rows(),
        owner.direct_records().len(),
        plan::Limits::default(),
    )
    .unwrap();
    assert_eq!(maps.get(7).unwrap().parent_map_id(), None);
    assert!(!maps.get(7).unwrap().active());
    assert_eq!(maps.get(8).unwrap().parent_map_id(), Some(7));
    assert_eq!(maps.stats().active_maps, 1);
    assert_eq!(
        world_result(&fs, world::Limits::default()).err(),
        Some(Error::Run(run::Error::Plan(plan::Error::ParentUnavailable)))
    );
}
#[test]
fn duplicate_map_ids_are_terminal() {
    assert_eq!(
        world_result(&files(&[(7, 0, -1), (7, 0, -1)]), world::Limits::default()).err(),
        Some(Error::Plan(plan::Error::DuplicateId))
    );
}
#[test]
fn missing_parent_is_terminal_even_when_parent_might_be_omitted() {
    let mut fs = files(&[(7, 0, 99)]);
    fs[1].2 = unknown(fs[1].2.clone(), true, 0);
    assert_eq!(
        world_result(&fs, world::Limits::default()).err(),
        Some(Error::Plan(plan::Error::MissingParent))
    );
}
#[test]
fn cyclic_parent_graph_is_terminal() {
    assert_eq!(
        world_result(&files(&[(7, 0, 8), (8, 0, 7)]), world::Limits::default()).err(),
        Some(Error::Plan(plan::Error::Cycle))
    );
}
#[test]
fn copied_parent_is_terminal() {
    let mut fs = files(&[(7, 0, 8)]);
    let hash = word(&fs[1].2, 152) as u32;
    fs[1].2 = fixture::map_db2(&[(7, 0, 8)], &[(8, 7)]);
    put(&mut fs[1].2, 152, hash);
    assert_eq!(
        world_result(&fs, world::Limits::default()).err(),
        Some(Error::Plan(plan::Error::CopiedParent))
    );
}
#[test]
fn missing_known_godi_model_never_returns_startup() {
    let mut fs = files(&[]);
    replace_godi(&mut fs, &[(10, 999)], &[]);
    let f = CollisionFixture::new(&fs);
    let mut store = open(&f);
    assert_eq!(
        extract_startup_fdid(&mut store, gameobjects::Limits::default()).err(),
        Some(Error::Startup(gameobjects::Error::Collision(
            super::super::Error::Store(local_store::Error::MissingFile)
        )))
    );
}

#[test]
fn v48_godi_missing_root_is_reported_only_after_proof_and_keeps_physical_row() {
    let mut fs = files_70450(&[]);
    replace_godi(&mut fs, &[(10, 50), (11, 0), (12, 50)], &[(20, 10)]);
    let fixture = CollisionFixture::for_build(&fs, 70450);
    let mut store = open_70450(&fixture);
    let startup =
        extract_startup_fdid_70450_with_root_absence(&mut store, gameobjects::Limits::default())
            .unwrap();
    assert_eq!(startup.data().missing_root(), &[50]);
    assert_eq!(
        startup
            .data()
            .outcomes()
            .iter()
            .map(|row| (
                row.ordinal(),
                row.display_id(),
                row.file_data_id(),
                row.outcome()
            ))
            .collect::<Vec<_>>(),
        [
            (0, 10, 50, gameobjects::Outcome::MissingRoot),
            (1, 11, 0, gameobjects::Outcome::ZeroFileDataId),
            (2, 12, 50, gameobjects::Outcome::MissingRoot),
        ]
    );
    assert_eq!(startup.data().stats().candidate_requests, 2);
    assert_eq!(startup.data().stats().collision_rows, 0);
    assert!(
        startup
            .data()
            .lists()
            .temp_bytes()
            .starts_with(forever_vmap::gameobject_models::TEMP_MAGIC)
    );
    assert!(
        startup
            .data()
            .lists()
            .dtree_bytes()
            .starts_with(forever_vmap::gameobject_models::DTREE_MAGIC)
    );
    assert_eq!(startup.data().lists().stats().rows, 0);
    assert_eq!(
        startup.data().lists().stats().resident_bytes,
        forever_vmap::gameobject_models::TEMP_MAGIC.len()
            + forever_vmap::gameobject_models::DTREE_MAGIC.len()
    );
    assert_eq!(startup.finish().unwrap().extraction().models().len(), 0);
}

#[test]
fn v48_godi_present_root_without_a_central_route_stays_fatal() {
    let mut fs = files_70450(&[]);
    replace_godi(&mut fs, &[(10, 50)], &[]);
    fs.push((50, None, fixture::m2(true)));
    let fixture = CollisionFixture::with_central_absence(&fs, 70450, 50);
    let mut store = open_70450(&fixture);
    assert_eq!(
        extract_startup_fdid_70450_with_root_absence(&mut store, gameobjects::Limits::default(),)
            .err(),
        Some(Error::Startup(gameobjects::Error::Collision(
            super::super::Error::Store(local_store::Error::MissingFile)
        )))
    );
}

#[test]
fn v48_godi_corruption_is_not_omitted_when_root_lacks_a_row() {
    let mut fs = files_70450(&[]);
    replace_godi(&mut fs, &[(10, 50)], &[]);
    fs.push((50, None, fixture::m2(true)));
    let fixture = CollisionFixture::with_encoding_absence(&fs, 70450, 50);
    let mut store = open_70450(&fixture);
    assert!(matches!(
        extract_startup_fdid_70450_with_root_absence(&mut store, gameobjects::Limits::default(),),
        Err(Error::Startup(gameobjects::Error::Collision(
            super::super::Error::Store(local_store::Error::NoLocalEncoding(_))
        )))
    ));
}

#[cfg(target_os = "linux")]
#[test]
fn v49_encrypted_unavailable_rows_are_deduplicated_and_charged_once() {
    let mut fs = files_70450(&[]);
    replace_godi(&mut fs, &[(10, 50), (11, 50), (12, 0)], &[]);
    fs.push((50, None, fixture::m2(true)));
    let fixture = CollisionFixture::with_available_holes_for_build(
        &fs,
        &[(50, 0..fs.last().unwrap().2.len())],
        70450,
    );
    let mut store = open_70450(&fixture);
    let startup = extract_startup_fdid_70450_with_root_or_encrypted_unavailable(
        &mut store,
        gameobjects::Limits::default(),
    )
    .unwrap();
    assert_eq!(startup.data().encrypted_unavailable(), &[50]);
    assert_eq!(startup.data().encrypted_unavailable_rows(), 2);
    assert_eq!(startup.data().stats().candidate_requests, 2);
    assert!(
        startup.data().stats().resident_bytes
            >= std::mem::size_of_val(startup.data().outcomes())
                + startup.data().lists().stats().resident_bytes
                + std::mem::size_of_val(startup.data().encrypted_unavailable())
    );
    assert!(startup.finish().is_ok());
}

#[cfg(target_os = "linux")]
#[test]
fn v49_list_budget_boundary_does_not_double_admit_encrypted_fdids() {
    let mut fs = files_70450(&[]);
    replace_godi(&mut fs, &[(10, 50)], &[]);
    fs.push((50, None, fixture::m2(true)));
    let fixture = CollisionFixture::with_available_holes_for_build(
        &fs,
        &[(50, 0..fs.last().unwrap().2.len())],
        70450,
    );
    let mut store = open_70450(&fixture);
    let mut limits = gameobjects::Limits::default();
    limits.lists.max_memory_bytes = forever_vmap::gameobject_models::TEMP_MAGIC.len()
        + forever_vmap::gameobject_models::DTREE_MAGIC.len();
    let startup =
        extract_startup_fdid_70450_with_root_or_encrypted_unavailable(&mut store, limits).unwrap();
    assert_eq!(startup.data().encrypted_unavailable(), &[50]);
    assert!(startup.finish().is_ok());
}

#[cfg(target_os = "linux")]
#[test]
fn v49_encrypted_unavailable_candidate_limit_is_4096_rows() {
    let limits =
        crate::collision::resources::Profile::GameObjectRootAndEncryptedUnavailableExpandedV1
            .startup_limits();
    for count in [4096, 4097] {
        let mut fs = files_70450(&[]);
        let rows: Vec<_> = (0..count)
            .map(|i| (u32::try_from(i + 10).unwrap(), 50))
            .collect();
        replace_godi(&mut fs, &rows, &[]);
        fs.push((50, None, fixture::m2(true)));
        let fixture = CollisionFixture::with_available_holes_for_build(
            &fs,
            &[(50, 0..fs.last().unwrap().2.len())],
            70450,
        );
        let mut store = open_70450(&fixture);
        let result =
            extract_startup_fdid_70450_with_root_or_encrypted_unavailable(&mut store, limits);
        if count == 4096 {
            let startup = result.unwrap();
            assert_eq!(startup.data().stats().candidate_requests, 4096);
            assert_eq!(startup.data().encrypted_unavailable_rows(), 4096);
            assert_eq!(startup.data().encrypted_unavailable(), &[50]);
            startup.finish().unwrap();
        } else {
            assert!(matches!(
                result,
                Err(Error::Startup(gameobjects::Error::Limit))
            ));
        }
    }
}

#[test]
fn v47_godi_missing_root_remains_strict() {
    let mut fs = files_70450(&[]);
    replace_godi(&mut fs, &[(10, 50)], &[]);
    let fixture = CollisionFixture::for_build(&fs, 70450);
    let mut store = open_70450(&fixture);
    assert_eq!(
        extract_startup_fdid_70450(&mut store, gameobjects::Limits::default()).err(),
        Some(Error::Startup(gameobjects::Error::Collision(
            super::super::Error::Store(local_store::Error::MissingFile)
        )))
    );
}
#[test]
fn missing_known_liquid_remains_fatal() {
    let mut fs = files(&[]);
    fs.remove(2);
    assert_eq!(
        world_result(&fs, world::Limits::default()).err(),
        Some(Error::Collision(super::super::Error::Store(
            local_store::Error::MissingFile
        )))
    );
}
#[test]
fn missing_known_wdt_remains_fatal() {
    assert_eq!(
        world_result(&files(&[(7, 999, -1)]), world::Limits::default()).err(),
        Some(Error::Run(run::Error::Collision(
            super::super::Error::Store(local_store::Error::MissingFile)
        )))
    );
}
#[test]
fn corrupt_known_godi_schema_is_not_omitted() {
    let mut fs = files(&[]);
    put(&mut fs[0].2, 156, 0);
    let f = CollisionFixture::new(&fs);
    let mut store = open(&f);
    assert_eq!(
        extract_startup_fdid(&mut store, gameobjects::Limits::default()).err(),
        Some(Error::Db2(RecordError::Schema))
    );
}
#[test]
fn wrong_build_is_sticky_without_second_read() {
    let f = CollisionFixture::new(&files(&[]));
    let mut store = f.open(false);
    let mut c = Extractor::new(&mut store, &EMPTY, super::super::Limits::default()).unwrap();
    let error = c
        .read_available_source(gameobjects::TABLE_FILE_DATA_ID, gameobjects::MAX_DB2_BYTES)
        .err()
        .unwrap();
    assert_eq!(
        error,
        super::super::Error::Available(super::super::AvailableError::Build)
    );
    assert_eq!(
        c.read_source(gameobjects::TABLE_FILE_DATA_ID, gameobjects::MAX_DB2_BYTES)
            .err(),
        Some(error)
    );
    assert_eq!(c.facts().reads, 1);
    assert_eq!(c.finish().err(), Some(error));
}
#[test]
fn declared_source_read_is_charged_until_explicit_release() {
    let f = CollisionFixture::new(&files(&[]));
    let mut store = open(&f);
    let mut c = Extractor::new(&mut store, &EMPTY, super::super::Limits::default()).unwrap();
    let file = c
        .read_available_source(gameobjects::TABLE_FILE_DATA_ID, gameobjects::MAX_DB2_BYTES)
        .unwrap();
    let declared = file.declared_bytes() as usize;
    let resident = file.resident_bytes().unwrap();
    assert_eq!(c.facts().source_bytes, declared);
    assert_eq!(c.retained_bytes(), resident);
    drop(file);
    c.release_source(resident).unwrap();
    assert_eq!(c.retained_bytes(), 0);
    assert_eq!(c.finish().unwrap().facts().reads, 1);
}
#[test]
fn zero_source_budget_rejects_before_read_and_stays_sticky() {
    let f = CollisionFixture::new(&files(&[]));
    let mut store = open(&f);
    let mut c = Extractor::new(
        &mut store,
        &EMPTY,
        super::super::Limits {
            max_source_bytes: 0,
            ..super::super::Limits::default()
        },
    )
    .unwrap();
    assert_eq!(
        c.read_available_source(gameobjects::TABLE_FILE_DATA_ID, 1024)
            .err(),
        Some(super::super::Error::Limit)
    );
    assert_eq!(c.facts().reads, 0);
    assert_eq!(
        c.read_source(gameobjects::TABLE_FILE_DATA_ID, 1024).err(),
        Some(super::super::Error::Limit)
    );
}
#[test]
fn external_live_budget_participates_before_available_allocation() {
    let f = CollisionFixture::new(&files(&[]));
    let mut store = open(&f);
    let resident = store.stats().resident_bytes;
    let mut c = Extractor::new(
        &mut store,
        &EMPTY,
        super::super::Limits {
            max_memory_bytes: resident,
            ..super::super::Limits::default()
        },
    )
    .unwrap();
    assert!(
        c.read_available_source(gameobjects::TABLE_FILE_DATA_ID, 1024)
            .is_err()
    );
    assert_eq!(c.facts().source_bytes, 0);
    assert_eq!(c.facts().reads, 1);
}
#[test]
fn downward_declared_row_limit_counts_unknown_sections() {
    let mut fs = files(&[]);
    replace_godi(&mut fs, &[(10, 0)], &[]);
    fs[0].2 = unknown(fs[0].2.clone(), false, 0);
    let f = CollisionFixture::new(&fs);
    let mut store = open(&f);
    assert_eq!(
        extract_startup_fdid(
            &mut store,
            gameobjects::Limits {
                max_direct_rows: 1,
                ..gameobjects::Limits::default()
            }
        )
        .err(),
        Some(Error::Limit)
    );
}
#[test]
fn downward_copy_limit_counts_unknown_sections() {
    let mut fs = files(&[]);
    fs[0].2 = unknown(fs[0].2.clone(), false, 1);
    let f = CollisionFixture::new(&fs);
    let mut store = open(&f);
    assert_eq!(
        extract_startup_fdid(
            &mut store,
            gameobjects::Limits {
                max_copy_records: 0,
                ..gameobjects::Limits::default()
            }
        )
        .err(),
        Some(Error::Limit)
    );
}
#[test]
fn decode_allowance_rejects_without_models() {
    let f = CollisionFixture::new(&files(&[]));
    let mut store = open(&f);
    assert_eq!(
        extract_startup_fdid(
            &mut store,
            gameobjects::Limits {
                max_decode_allowance_bytes: 0,
                ..gameobjects::Limits::default()
            }
        )
        .err(),
        Some(Error::Limit)
    );
}
#[test]
fn whole_table_graph_topology_overlap_is_downward_bounded() {
    let fs = files(&[(7, 0, -1)]);
    assert_eq!(
        world_result(
            &fs,
            world::Limits {
                max_table_memory_bytes: 0,
                ..world::Limits::default()
            }
        )
        .err(),
        Some(Error::Limit)
    );
}
#[test]
fn changed_store_after_startup_rejects_finish() {
    let f = CollisionFixture::new(&files(&[]));
    let mut store = open(&f);
    let startup = extract_startup_fdid(&mut store, gameobjects::Limits::default()).unwrap();
    f.change_metadata();
    assert_eq!(
        startup.finish().err(),
        Some(Error::Collision(super::super::Error::Store(
            local_store::Error::InputChanged
        )))
    );
}
#[test]
fn shared_run_failure_poison_survives_available_lookup_transition() {
    let mut fs = files(&[]);
    replace_godi(&mut fs, &[(10, 50)], &[]);
    fs.push((50, None, fixture::m2(true)));
    let f = CollisionFixture::new(&fs);
    let mut store = open(&f);
    let mut startup = extract_startup_fdid(&mut store, gameobjects::Limits::default()).unwrap();
    let rows = [];
    let maps = plan::Maps::from_materialized(&rows, 0, plan::Limits::default()).unwrap();
    let mut sink = Stage {
        fail: true,
        ..Stage::default()
    };
    assert!(
        run::shared::extract_collector(
            startup.collector_mut(),
            &maps,
            &mut sink,
            run::SharedLimits::default()
        )
        .is_err()
    );
    assert!(startup.with_lookup(&EMPTY).is_err());
}
#[test]
fn cached_wmo_keeps_empty_lookup_and_new_wmo_uses_complete_liquids() {
    let mut fs = files(&[(1, 101, -1)]);
    replace_godi(&mut fs, &[(1, 10)], &[]);
    fs.extend([
        (10, None, fixture::wmo_root(11, None)),
        (11, None, fixture::wmo_group(true, false, true)),
        (20, None, fixture::wmo_root(21, None)),
        (21, None, fixture::wmo_group(true, false, true)),
        (101, None, fixture::wdt(Some(10), Some(201))),
        (201, None, fixture::wmo_adt(20, 8)),
    ]);
    let f = CollisionFixture::new(&fs);
    let mut store = open(&f);
    let startup = extract_startup_fdid(&mut store, gameobjects::Limits::default()).unwrap();
    let gameobjects::Outcome::Collision(handle) = startup.data().outcomes()[0].outcome() else {
        panic!()
    };
    let cached = startup.model(handle).unwrap().raw().unwrap().to_vec();
    let mut stage = Stage::default();
    let out = extract(startup, &mut stage, world::Limits::default()).unwrap();
    let models = out.startup().extraction().models();
    assert_eq!(models.len(), 2);
    assert_eq!(models[0].raw().unwrap(), cached);
    let flags = models
        .iter()
        .map(|m| {
            raw::RawWmo::parse(m.raw().unwrap(), raw::ConversionLimits::default())
                .unwrap()
                .groups()
                .next()
                .unwrap()
                .liquid_flags()
        })
        .collect::<Vec<_>>();
    assert_eq!(flags, [2, 0]);
    assert_eq!(out.startup().extraction().facts().reads, 10);
    assert_eq!(stage.placements.len(), 2);
    assert_eq!(out.report().run.model_callbacks, 2);
}
#[test]
fn complete_inputs_preserve_strict_world_outputs() {
    let mut fs = files(&[(7, 101, -1)]);
    replace_godi(&mut fs, &[(1, 50)], &[]);
    fs.extend([
        (50, None, fixture::m2(true)),
        (101, None, fixture::wdt(None, Some(201))),
        (201, None, fixture::m2_adt(50, 9)),
    ]);
    let f = CollisionFixture::new(&fs);
    let mut store = open(&f);
    let strict =
        gameobjects::extract_startup_fdid(&mut store, gameobjects::Limits::default()).unwrap();
    let mut a = Stage::default();
    let strict = world::extract(strict, &mut a, world::Limits::default()).unwrap();
    drop(store);
    let mut store = open(&f);
    let available = extract_startup_fdid(&mut store, gameobjects::Limits::default()).unwrap();
    let mut b = Stage::default();
    let available = extract(available, &mut b, world::Limits::default()).unwrap();
    assert_eq!(a.placements, b.placements);
    assert_eq!(a.models, b.models);
    assert_eq!(
        strict.startup().data().lists().dtree_bytes(),
        available.startup().data().lists().dtree_bytes()
    );
    assert_eq!(
        strict.startup().extraction().facts(),
        available.startup().extraction().facts()
    );
}

#[test]
fn joined_encrypted_unknown_godi_hole_preserves_direct_engine_and_crypto_facts() {
    let mut fs = files(&[]);
    replace_godi(&mut fs, &[(10, 50), (11, 50)], &[(20, 10)]);
    fs[0].2 = unknown(fs[0].2.clone(), false, 1);
    fs.push((50, None, fixture::m2(true)));
    let start = word(&fs[0].2, 252);
    let end = fs[0].2.len();
    let f = CollisionFixture::with_available_holes(
        &fs,
        &[(gameobjects::TABLE_FILE_DATA_ID, start..end)],
    );
    let mut store = open(&f);
    assert!(
        store
            .read_file_data_id(gameobjects::TABLE_FILE_DATA_ID)
            .is_err()
    );
    let out = extract_startup_fdid(&mut store, gameobjects::Limits::default())
        .unwrap()
        .finish()
        .unwrap();
    let facts = out.data().table();
    assert!(!facts.full_ckey_verified);
    assert!(facts.encoded_key_authenticated);
    assert_eq!(facts.available_bytes, start as u64);
    assert_eq!(facts.declared_bytes, end as u64);
    assert_eq!(facts.work.strict_candidate_attempts, 1);
    assert_eq!(facts.work.partial_candidate_attempts, 1);
    assert_eq!(facts.declared_direct_rows, 3);
    assert_eq!(facts.direct_rows, 2);
    assert_eq!(facts.copy_records, 1);
    assert_eq!(facts.declared_copy_records, 2);
    assert_eq!(out.extraction().facts().reads, 2);
    assert_eq!(out.extraction().models().len(), 1);
    assert_eq!(
        out.extraction().facts().source_bytes,
        end + fs.last().unwrap().2.len()
    );
}
#[test]
fn joined_encrypted_unknown_map_hole_keeps_strict_parent_graph_and_complete_liquids() {
    let mut fs = files(&[(7, 101, -1), (8, 100, 7)]);
    fs[1].2 = unknown(fs[1].2.clone(), true, 1);
    fs.extend([
        (101, None, fixture::wdt(None, None)),
        (100, None, fixture::wdt(None, None)),
    ]);
    let start = word(&fs[1].2, 252);
    let end = fs[1].2.len();
    let f = CollisionFixture::with_available_holes(&fs, &[(world::MAP_FILE_DATA_ID, start..end)]);
    let mut store = open(&f);
    let startup = extract_startup_fdid(&mut store, gameobjects::Limits::default()).unwrap();
    let out = extract(startup, &mut Stage::default(), world::Limits::default()).unwrap();
    assert!(!out.map().full_ckey_verified);
    assert!(out.map().encoded_key_authenticated);
    assert!(!out.map().all_sections_admitted());
    assert_eq!(out.topology().direct_count(), 2);
    assert_eq!(out.topology().get(8).unwrap().parent_map_id(), Some(7));
    assert!(out.topology().get(7).unwrap().active());
    assert_eq!(out.lookup_stats().materials, 1);
    assert_eq!(out.lookup_stats().types, 1);
    assert_eq!(out.startup().extraction().facts().reads, 6);
}
#[test]
fn joined_known_zero_tact_payload_hole_is_fatal_not_omitted() {
    let mut fs = files(&[]);
    replace_godi(&mut fs, &[(10, 0)], &[]);
    let start = word(&fs[0].2, 212);
    let end = fs[0].2.len();
    let f = CollisionFixture::with_available_holes(
        &fs,
        &[(gameobjects::TABLE_FILE_DATA_ID, start..end)],
    );
    let mut store = open(&f);
    assert_eq!(
        extract_startup_fdid(&mut store, gameobjects::Limits::default()).err(),
        Some(Error::Db2(RecordError::Bounds))
    );
}
#[test]
fn joined_partial_source_counts_declared_holes_before_read_budget() {
    let mut fs = files(&[]);
    fs[0].2 = unknown(fs[0].2.clone(), false, 0);
    let start = word(&fs[0].2, 252);
    let end = fs[0].2.len();
    let f = CollisionFixture::with_available_holes(
        &fs,
        &[(gameobjects::TABLE_FILE_DATA_ID, start..end)],
    );
    let mut store = open(&f);
    let mut c = Extractor::new(
        &mut store,
        &EMPTY,
        super::super::Limits {
            max_source_bytes: end - 1,
            ..super::super::Limits::default()
        },
    )
    .unwrap();
    assert!(
        c.read_available_source(gameobjects::TABLE_FILE_DATA_ID, end)
            .is_err()
    );
    assert_eq!(c.facts().source_bytes, 0);
    assert_eq!(c.facts().reads, 1);
}
#[test]
fn supplied_keys_cannot_activate_available_startup() {
    let f = CollisionFixture::new(&files(&[]));
    let store = open(&f);
    drop(store);
    let keys = crate::keys::Keyring::parse(
        b"forever-tact-keys\t1\n0102030405060708\t11111111111111111111111111111111\n",
    )
    .unwrap();
    let mut store = LocalStore::open_db2_70374_with_keys(
        f.path(),
        local_store::OpenOptions {
            locale: Locale::EnUs,
            index_layouts: [local_index::Layout::Contiguous; 16],
            root_layout: root::Layout::Header50893V2,
            limits: local_store::Limits::default(),
        },
        keys,
    )
    .unwrap();
    assert_eq!(
        extract_startup_fdid(&mut store, gameobjects::Limits::default()).err(),
        Some(Error::Collision(super::super::Error::Available(
            super::super::AvailableError::KeysPresent
        )))
    );
}

#[test]
fn available_store_errors_flatten_without_losing_typed_candidate_counts() {
    use super::super::Error as CollisionError;
    let failures = local_store::CandidateFailures {
        missing: 1,
        incomplete: 2,
        encrypted: 3,
        corrupt: 4,
        unsupported: 5,
    };
    for store in [
        local_store::Error::InputChanged,
        local_store::Error::MissingFile,
        local_store::Error::Integrity,
        local_store::Error::Limit,
        local_store::Error::NoLocalEncoding(failures),
    ] {
        let source = acquired::Error::Store(store);
        let mapped = CollisionError::from_available(source);
        assert_eq!(mapped, CollisionError::Store(store));
        assert_eq!(mapped.code(), source.code());
        assert_eq!(CollisionError::from(source), mapped);
    }
}
#[test]
fn availability_only_error_variants_retain_exact_codes() {
    use super::super::{AvailableError, Error as CollisionError};
    for (source, compact) in [
        (
            acquired::Error::Blte(crate::blte::Error::Checksum),
            AvailableError::Blte(crate::blte::Error::Checksum),
        ),
        (acquired::Error::KeysPresent, AvailableError::KeysPresent),
        (acquired::Error::Build, AvailableError::Build),
        (
            acquired::Error::AmbiguousEncoding,
            AvailableError::AmbiguousEncoding,
        ),
        (
            acquired::Error::UnsupportedArchive,
            AvailableError::UnsupportedArchive,
        ),
    ] {
        let mapped = CollisionError::from_available(source);
        assert_eq!(mapped, CollisionError::Available(compact));
        assert_eq!(mapped.code(), source.code());
        assert_eq!(compact.code(), source.code());
    }
}
#[test]
fn compact_available_error_keeps_copy_and_existing_store_size_ceiling() {
    use super::super::{AvailableError, Error as CollisionError};
    fn assert_copy<T: Copy>() {}
    assert_copy::<AvailableError>();
    assert_copy::<CollisionError>();
    assert!(std::mem::size_of::<AvailableError>() <= std::mem::size_of::<local_store::Error>());
    assert!(
        std::mem::size_of::<CollisionError>()
            <= std::mem::size_of::<local_store::Error>()
                + std::mem::align_of::<local_store::Error>()
    );
    // Clippy's result_large_err threshold must not be reached again by nesting
    // the already present Store payload inside another availability error.
    assert!(std::mem::size_of::<CollisionError>() < 128);
}

#[test]
fn root_absent_completion_charges_declaration_capacity_and_explicit_profile() {
    let mut sink = Stage::default();
    let out = root_absence_result(&files_70450(&[(7, 999, -1)]), &mut sink).unwrap();
    assert_eq!(out.profile(), PROFILE_70450_AVAILABLE_ROOT_ABSENT_GEOMETRY);
    assert_eq!(
        out.root_absent(),
        &[run::RootAbsent {
            map_id: 7,
            wdt_file_data_id: 999,
        }]
    );
    assert_eq!((out.report().run.maps, out.report().run.wdt_files), (1, 0));
    assert_eq!((sink.models, sink.placements.len()), (0, 0));

    let declaration_bytes = out.root_absent.capacity() * std::mem::size_of::<run::RootAbsent>();
    assert!(declaration_bytes > 0);
    let expected = out.startup().extraction().resident_bytes()
        + out.startup().data().stats().resident_bytes
        + out.topology().resident_bytes()
        + declaration_bytes;
    assert_eq!(out.resident_bytes(), expected);
}

#[test]
fn model_root_absent_completion_retains_wmo_and_charges_both_capacities() {
    let mut files = files_70450(&[(7, 100, -1)]);
    files.extend([
        (100, None, fixture::wdt(Some(10), None)),
        (10, None, fixture::wmo_root(30, Some(20))),
        (30, None, fixture::wmo_group(true, true, false)),
    ]);
    let mut sink = Stage::default();
    let out = model_root_absence_result(&files, &mut sink).unwrap();
    assert_eq!(
        out.profile(),
        PROFILE_70450_AVAILABLE_MODEL_ROOT_ABSENT_GEOMETRY
    );
    assert!(out.root_absent().is_empty());
    assert_eq!(out.missing_m2(), &[20]);
    assert_eq!((out.report().run.maps, out.report().run.wdt_files), (1, 1));
    assert_eq!(out.report().run.model_callbacks, 1);
    assert_eq!(out.report().run.ids.calls, 1);
    assert_eq!((sink.models, sink.placements.len()), (1, 1));
    assert_eq!(
        out.report().collector_after.dependency_requests
            - out.report().collector_before.dependency_requests,
        1
    );
    assert_eq!(
        out.report().request_calls_after - out.report().request_calls_before,
        1
    );

    let root_absent_bytes = out.root_absent.capacity() * std::mem::size_of::<run::RootAbsent>();
    let missing_m2_bytes = out.missing_m2.capacity() * std::mem::size_of::<u32>();
    assert!(root_absent_bytes > 0 && missing_m2_bytes > 0);
    let expected = out.startup().extraction().resident_bytes()
        + out.startup().data().stats().resident_bytes
        + out.topology().resident_bytes()
        + root_absent_bytes
        + missing_m2_bytes;
    assert_eq!(out.resident_bytes(), expected);
}

#[test]
fn request_capacity_completion_is_distinct_even_without_omissions() {
    let mut files = files_70450(&[(7, 100, -1)]);
    files.push((100, None, fixture::wdt(None, None)));
    let mut sink = Stage::default();
    let out = model_root_absence_request_capacity_result(&files, &mut sink).unwrap();
    assert_eq!(
        out.profile(),
        PROFILE_70450_AVAILABLE_MODEL_ROOT_ABSENT_REQUEST_CAPACITY_GEOMETRY
    );
    assert!(out.root_absent().is_empty());
    assert!(out.missing_m2().is_empty());
    assert_eq!((out.report().run.maps, out.report().run.wdt_files), (1, 1));
    assert_eq!((sink.models, sink.placements.len()), (0, 1));

    let mut sink = Stage::default();
    let previous = model_root_absence_result(&files, &mut sink).unwrap();
    assert_eq!(
        previous.profile(),
        PROFILE_70450_AVAILABLE_MODEL_ROOT_ABSENT_GEOMETRY
    );
    assert!(previous.missing_m2().is_empty());
}

#[test]
fn unselected_wmo_profile_reports_skipped_geometry_and_charges_capacity() {
    let mut files = files_70450(&[(7, 100, -1)]);
    let mut root = fixture::wmo_root(30, None);
    let mut mods: Vec<_> = b"MODS".iter().rev().copied().collect();
    mods.extend_from_slice(&32u32.to_le_bytes());
    mods.extend_from_slice(&[0; 32]);
    root.extend(mods);
    files.extend([
        (100, None, fixture::wdt(Some(10), None)),
        (10, None, root),
        (30, None, {
            let mut group = fixture::wmo_group(true, false, false);
            put(&mut group, 16, 0x80);
            group
        }),
    ]);
    let mut sink = Stage::default();
    let out = unselected_wmo_result(&files, &mut sink).unwrap();
    assert_eq!(
        out.profile(),
        PROFILE_70450_AVAILABLE_UNSELECTED_WMO_GEOMETRY
    );
    assert!(out.root_absent().is_empty());
    assert!(out.missing_m2().is_empty());
    assert!(out.missing_wmo().is_empty());
    assert!(out.empty_wmo().is_empty());
    assert_eq!(out.unselected_wmo(), &[10]);
    assert!(out.material_filtered_wmo().is_empty());
    assert_eq!(out.material_filtered_wmo.capacity(), 0);
    assert_eq!(sink.models, 1);
    assert_eq!(sink.placements.len(), 1);
    assert!(sink.placements[0].1.is_empty());
    assert_eq!(out.report().run.ids.calls, 0);
    let expected = out.startup().extraction().resident_bytes()
        + out.startup().data().stats().resident_bytes
        + out.topology().resident_bytes()
        + out.root_absent.capacity() * std::mem::size_of::<run::RootAbsent>()
        + out.missing_m2.capacity() * std::mem::size_of::<u32>()
        + out.missing_wmo.capacity() * std::mem::size_of::<u32>()
        + out.empty_wmo.capacity() * std::mem::size_of::<u32>()
        + out.unselected_wmo.capacity() * std::mem::size_of::<u32>();
    assert_eq!(out.resident_bytes(), expected);
}

#[test]
fn empty_wmo_profile_is_distinct_and_reports_empty_intrinsic_source() {
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
    let fixture = CollisionFixture::for_build(&files, 70450);
    let mut store = open_70450(&fixture);
    let startup = gameobjects::extract_startup_without_godi_with_resources(
        &mut store,
        gameobjects::Limits::default(),
        crate::collision::resources::Profile::RequestHeavyV1,
    )
    .unwrap();
    let mut sink = Stage::default();
    let out =
        extract_all_with_empty_wmo_70450(startup, &mut sink, world::Limits::default()).unwrap();
    assert_eq!(out.profile(), PROFILE_70450_AVAILABLE_EMPTY_WMO_GEOMETRY);
    assert!(out.root_absent().is_empty());
    assert!(out.missing_m2().is_empty());
    assert!(out.missing_wmo().is_empty());
    assert_eq!(out.empty_wmo(), &[10]);
    assert_eq!((sink.models, sink.placements.len()), (1, 1));
    assert!(sink.placements[0].1.is_empty());
    assert_eq!(out.report().run.ids.calls, 0);
    let empty_bytes = out.empty_wmo.capacity() * std::mem::size_of::<u32>();
    let root_absent_bytes = out.root_absent.capacity() * std::mem::size_of::<run::RootAbsent>();
    let missing_m2_bytes = out.missing_m2.capacity() * std::mem::size_of::<u32>();
    assert!(empty_bytes > 0);
    let expected = out.startup().extraction().resident_bytes()
        + out.startup().data().stats().resident_bytes
        + out.topology().resident_bytes()
        + root_absent_bytes
        + missing_m2_bytes
        + out.missing_wmo.capacity() * std::mem::size_of::<u32>()
        + empty_bytes;
    assert_eq!(out.resident_bytes(), expected);
}

#[test]
fn material_filtered_wmo_profile_reports_its_separate_id_capacity() {
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
    let out = material_filtered_wmo_result(&files, &mut sink).unwrap();
    assert_eq!(
        out.profile(),
        PROFILE_70450_AVAILABLE_MATERIAL_FILTERED_WMO_GEOMETRY
    );
    assert!(out.root_absent().is_empty());
    assert!(out.missing_m2().is_empty());
    assert!(out.missing_wmo().is_empty());
    assert!(out.empty_wmo().is_empty());
    assert!(out.unselected_wmo().is_empty());
    assert_eq!(out.material_filtered_wmo(), &[10]);
    assert_eq!(sink.models, 1);
    assert_eq!(sink.placements.len(), 1);
    assert!(sink.placements[0].1.is_empty());
    assert_eq!(out.report().run.ids.calls, 0);
    let material_filtered_bytes = out.material_filtered_wmo.capacity() * std::mem::size_of::<u32>();
    assert!(material_filtered_bytes > 0);
    let expected = out.startup().extraction().resident_bytes()
        + out.startup().data().stats().resident_bytes
        + out.topology().resident_bytes()
        + out.root_absent.capacity() * std::mem::size_of::<run::RootAbsent>()
        + out.missing_m2.capacity() * std::mem::size_of::<u32>()
        + out.missing_wmo.capacity() * std::mem::size_of::<u32>()
        + out.empty_wmo.capacity() * std::mem::size_of::<u32>()
        + out.unselected_wmo.capacity() * std::mem::size_of::<u32>()
        + material_filtered_bytes;
    assert_eq!(out.resident_bytes(), expected);
}

#[test]
fn dependency_heavy_material_filtered_wmo_profile_is_distinct_and_charges_capacity() {
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
    let out = dependency_heavy_material_filtered_wmo_result(&files, &mut sink).unwrap();
    assert_eq!(
        out.profile(),
        PROFILE_70450_AVAILABLE_DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_GEOMETRY
    );
    assert!(out.root_absent().is_empty());
    assert!(out.missing_m2().is_empty());
    assert!(out.missing_wmo().is_empty());
    assert!(out.empty_wmo().is_empty());
    assert!(out.unselected_wmo().is_empty());
    assert_eq!(out.material_filtered_wmo(), &[10]);
    assert_eq!(out.report().run.no_collision_models, 1);
    assert_eq!(out.report().run.ids.calls, 0);
    assert_eq!(sink.models, 1);
    assert_eq!(sink.placements.len(), 1);
    assert!(sink.placements[0].1.is_empty());

    let expected = out.startup().extraction().resident_bytes()
        + out.startup().data().stats().resident_bytes
        + out.topology().resident_bytes()
        + out.root_absent.capacity() * std::mem::size_of::<run::RootAbsent>()
        + out.missing_m2.capacity() * std::mem::size_of::<u32>()
        + out.missing_wmo.capacity() * std::mem::size_of::<u32>()
        + out.empty_wmo.capacity() * std::mem::size_of::<u32>()
        + out.unselected_wmo.capacity() * std::mem::size_of::<u32>()
        + out.material_filtered_wmo.capacity() * std::mem::size_of::<u32>();
    assert_eq!(out.resident_bytes(), expected);

    let mut v36_sink = Stage::default();
    let v36 = material_filtered_wmo_result(&files, &mut v36_sink).unwrap();
    assert_eq!(
        v36.profile(),
        PROFILE_70450_AVAILABLE_MATERIAL_FILTERED_WMO_GEOMETRY
    );
    assert_eq!(v36.material_filtered_wmo(), &[10]);
    assert_eq!(v36_sink.models, sink.models);
    assert_eq!(v36_sink.placements, sink.placements);
}

#[test]
fn source_heavy_material_filtered_wmo_profile_has_its_own_completion_identity() {
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
    let out = source_heavy_material_filtered_wmo_result(&files, &mut sink).unwrap();
    assert_eq!(
        out.profile(),
        PROFILE_70450_AVAILABLE_SOURCE_HEAVY_MATERIAL_FILTERED_WMO_GEOMETRY
    );
    assert_eq!(out.material_filtered_wmo(), &[10]);
    assert!(out.root_absent().is_empty());
    assert!(out.missing_m2().is_empty());
    assert!(out.missing_wmo().is_empty());
    assert!(out.empty_wmo().is_empty());
    assert!(out.unselected_wmo().is_empty());
    assert_eq!(out.report().run.no_collision_models, 1);
    assert_eq!(out.report().run.ids.calls, 0);
    assert_eq!(sink.models, 1);
    assert_eq!(sink.placements.len(), 1);
    assert!(sink.placements[0].1.is_empty());

    let expected = out.startup().extraction().resident_bytes()
        + out.startup().data().stats().resident_bytes
        + out.topology().resident_bytes()
        + out.root_absent.capacity() * std::mem::size_of::<run::RootAbsent>()
        + out.missing_m2.capacity() * std::mem::size_of::<u32>()
        + out.missing_wmo.capacity() * std::mem::size_of::<u32>()
        + out.empty_wmo.capacity() * std::mem::size_of::<u32>()
        + out.unselected_wmo.capacity() * std::mem::size_of::<u32>()
        + out.material_filtered_wmo.capacity() * std::mem::size_of::<u32>();
    assert_eq!(out.resident_bytes(), expected);

    let mut v37_sink = Stage::default();
    let v37 = dependency_heavy_material_filtered_wmo_result(&files, &mut v37_sink).unwrap();
    assert_eq!(
        v37.profile(),
        PROFILE_70450_AVAILABLE_DEPENDENCY_HEAVY_MATERIAL_FILTERED_WMO_GEOMETRY
    );
    assert_eq!(v37.material_filtered_wmo(), out.material_filtered_wmo());
    assert_eq!(v37_sink.models, sink.models);
    assert_eq!(v37_sink.placements, sink.placements);

    let mut v36_sink = Stage::default();
    let v36 = material_filtered_wmo_result(&files, &mut v36_sink).unwrap();
    assert_eq!(
        v36.profile(),
        PROFILE_70450_AVAILABLE_MATERIAL_FILTERED_WMO_GEOMETRY
    );
    assert_eq!(v36.material_filtered_wmo(), out.material_filtered_wmo());
    assert_eq!(v36_sink.models, sink.models);
    assert_eq!(v36_sink.placements, sink.placements);

    let mut v39_sink = Stage::default();
    let v39 = read_heavy_material_filtered_wmo_result(&files, &mut v39_sink).unwrap();
    assert_eq!(
        v39.profile(),
        PROFILE_70450_AVAILABLE_READ_HEAVY_MATERIAL_FILTERED_WMO_GEOMETRY
    );
    assert_eq!(v39.material_filtered_wmo(), out.material_filtered_wmo());
    assert_eq!(v39.root_absent(), out.root_absent());
    assert_eq!(v39.missing_m2(), out.missing_m2());
    assert_eq!(v39.missing_wmo(), out.missing_wmo());
    assert_eq!(v39.empty_wmo(), out.empty_wmo());
    assert_eq!(v39.unselected_wmo(), out.unselected_wmo());
    assert_eq!(v39.report().run, out.report().run);
    assert_eq!(v39_sink.models, sink.models);
    assert_eq!(v39_sink.placements, sink.placements);

    let mut v40_sink = Stage::default();
    let v40 = model_heavy_material_filtered_wmo_result(&files, &mut v40_sink).unwrap();
    assert_eq!(
        v40.profile(),
        PROFILE_70450_AVAILABLE_MODEL_HEAVY_MATERIAL_FILTERED_WMO_GEOMETRY
    );
    assert_eq!(v40.material_filtered_wmo(), out.material_filtered_wmo());
    assert_eq!(v40.root_absent(), out.root_absent());
    assert_eq!(v40.missing_m2(), out.missing_m2());
    assert_eq!(v40.missing_wmo(), out.missing_wmo());
    assert_eq!(v40.empty_wmo(), out.empty_wmo());
    assert_eq!(v40.unselected_wmo(), out.unselected_wmo());
    assert_eq!(v40.report().run, out.report().run);
    assert_eq!(v40_sink.models, sink.models);
    assert_eq!(v40_sink.placements, sink.placements);

    let mut v41_sink = Stage::default();
    let v41 = source_expanded_material_filtered_wmo_result(&files, &mut v41_sink).unwrap();
    assert_eq!(
        v41.profile(),
        PROFILE_70450_AVAILABLE_SOURCE_EXPANDED_MATERIAL_FILTERED_WMO_GEOMETRY
    );
    assert_eq!(v41.material_filtered_wmo(), out.material_filtered_wmo());
    assert_eq!(v41.root_absent(), out.root_absent());
    assert_eq!(v41.missing_m2(), out.missing_m2());
    assert_eq!(v41.missing_wmo(), out.missing_wmo());
    assert_eq!(v41.empty_wmo(), out.empty_wmo());
    assert_eq!(v41.unselected_wmo(), out.unselected_wmo());
    assert_eq!(v41.report().run, out.report().run);
    assert_eq!(v41_sink.models, sink.models);
    assert_eq!(v41_sink.placements, sink.placements);

    let mut v42_sink = Stage::default();
    let v42 = model_output_expanded_material_filtered_wmo_result(&files, &mut v42_sink).unwrap();
    assert_eq!(
        v42.profile(),
        PROFILE_70450_AVAILABLE_MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_GEOMETRY
    );
    assert_eq!(v42.material_filtered_wmo(), out.material_filtered_wmo());
    assert_eq!(v42.root_absent(), out.root_absent());
    assert_eq!(v42.missing_m2(), out.missing_m2());
    assert_eq!(v42.missing_wmo(), out.missing_wmo());
    assert_eq!(v42.empty_wmo(), out.empty_wmo());
    assert_eq!(v42.unselected_wmo(), out.unselected_wmo());
    assert_eq!(v42.report().run, out.report().run);
    assert_eq!(v42_sink.models, sink.models);
    assert_eq!(v42_sink.placements, sink.placements);

    let mut v43_sink = Stage::default();
    let v43 = model_bytes_expanded_material_filtered_wmo_result(&files, &mut v43_sink).unwrap();
    assert_eq!(
        v43.profile(),
        PROFILE_70450_AVAILABLE_MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_GEOMETRY
    );
    assert_eq!(v43.material_filtered_wmo(), out.material_filtered_wmo());
    assert_eq!(v43.root_absent(), out.root_absent());
    assert_eq!(v43.missing_m2(), out.missing_m2());
    assert_eq!(v43.missing_wmo(), out.missing_wmo());
    assert_eq!(v43.empty_wmo(), out.empty_wmo());
    assert_eq!(v43.unselected_wmo(), out.unselected_wmo());
    assert_eq!(v43.report().run, out.report().run);
    assert_eq!(v43_sink.models, sink.models);
    assert_eq!(v43_sink.placements, sink.placements);

    let fixture = CollisionFixture::for_build(&files, 70450);
    let mut store = open_70450(&fixture);
    let startup = gameobjects::extract_startup_without_godi_with_resources(
        &mut store,
        gameobjects::Limits::default(),
        crate::collision::resources::Profile::ModelBytesExpandedV1,
    )
    .unwrap();
    let mut alias_sink = Stage::default();
    let alias = extract_all_with_model_bytes_expanded_material_filtered_wmo_70450(
        startup,
        &mut alias_sink,
        world::Limits::default(),
    )
    .unwrap();
    assert_eq!(
        alias.profile(),
        PROFILE_70450_AVAILABLE_MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_GEOMETRY
    );
    assert_eq!(alias.material_filtered_wmo(), v43.material_filtered_wmo());
    assert_eq!(alias.report().run, v43.report().run);
    assert_eq!(alias_sink.models, v43_sink.models);
    assert_eq!(alias_sink.placements, v43_sink.placements);

    let mut v44_sink = Stage::default();
    let v44 = source_expanded_v2_material_filtered_wmo_result(&files, &mut v44_sink).unwrap();
    assert_eq!(
        v44.profile(),
        PROFILE_70450_AVAILABLE_SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_GEOMETRY
    );
    assert_eq!(v44.material_filtered_wmo(), out.material_filtered_wmo());
    assert_eq!(v44.root_absent(), out.root_absent());
    assert_eq!(v44.missing_m2(), out.missing_m2());
    assert_eq!(v44.missing_wmo(), out.missing_wmo());
    assert_eq!(v44.empty_wmo(), out.empty_wmo());
    assert_eq!(v44.unselected_wmo(), out.unselected_wmo());
    assert_eq!(v44.report().run, out.report().run);
    assert_eq!(v44_sink.models, sink.models);
    assert_eq!(v44_sink.placements, sink.placements);

    let fixture = CollisionFixture::for_build(&files, 70450);
    let mut store = open_70450(&fixture);
    let startup = gameobjects::extract_startup_without_godi_with_resources(
        &mut store,
        gameobjects::Limits::default(),
        crate::collision::resources::Profile::SourceExpandedV2,
    )
    .unwrap();
    let mut alias_sink = Stage::default();
    let alias = extract_all_with_source_expanded_v2_material_filtered_wmo_70450(
        startup,
        &mut alias_sink,
        world::Limits::default(),
    )
    .unwrap();
    assert_eq!(
        alias.profile(),
        PROFILE_70450_AVAILABLE_SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_GEOMETRY
    );
    assert_eq!(alias.material_filtered_wmo(), v44.material_filtered_wmo());
    assert_eq!(alias.report().run, v44.report().run);
    assert_eq!(alias_sink.models, v44_sink.models);
    assert_eq!(alias_sink.placements, v44_sink.placements);
}

#[cfg(test)]
mod model_bytes_expanded_completion_tests {
    use super::*;

    #[test]
    fn model_bytes_expanded_completion_requires_exact_build_scope_and_omissions() {
        use crate::collision::resources::Profile;
        use crate::collision::target::Target;

        let material_filtered =
            run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered;
        assert_eq!(
            completion_profile_with_omissions(
                Target::Build70450,
                plan::MapSelection::All,
                run::MissingWdtPolicy::DeclareRootAbsent,
                run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                material_filtered,
                Profile::ModelBytesExpandedV1,
            ),
            Ok(PROFILE_70450_AVAILABLE_MODEL_BYTES_EXPANDED_MATERIAL_FILTERED_WMO_GEOMETRY)
        );
        assert_eq!(
            completion_profile_with_omissions(
                Target::Build70450,
                plan::MapSelection::All,
                run::MissingWdtPolicy::DeclareRootAbsent,
                run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                material_filtered,
                Profile::ModelOutputExpandedV1,
            ),
            Ok(PROFILE_70450_AVAILABLE_MODEL_OUTPUT_EXPANDED_MATERIAL_FILTERED_WMO_GEOMETRY)
        );
        assert_eq!(
            completion_profile_with_omissions(
                Target::Build70450,
                plan::MapSelection::All,
                run::MissingWdtPolicy::DeclareRootAbsent,
                run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                material_filtered,
                Profile::SourceExpandedV2,
            ),
            Ok(PROFILE_70450_AVAILABLE_SOURCE_EXPANDED_V2_MATERIAL_FILTERED_WMO_GEOMETRY)
        );
        for (target, selection, missing_wdt, missing_model, missing_wmo, resource) in [
            (
                Target::Build70425,
                plan::MapSelection::All,
                run::MissingWdtPolicy::DeclareRootAbsent,
                run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                material_filtered,
                Profile::SourceExpandedV2,
            ),
            (
                Target::Build70450,
                plan::MapSelection::MapAndAncestors(7),
                run::MissingWdtPolicy::DeclareRootAbsent,
                run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                material_filtered,
                Profile::SourceExpandedV2,
            ),
            (
                Target::Build70450,
                plan::MapSelection::All,
                run::MissingWdtPolicy::Reject,
                run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                material_filtered,
                Profile::SourceExpandedV2,
            ),
            (
                Target::Build70450,
                plan::MapSelection::All,
                run::MissingWdtPolicy::DeclareRootAbsent,
                run::MissingModelPolicy::Reject,
                material_filtered,
                Profile::SourceExpandedV2,
            ),
            (
                Target::Build70450,
                plan::MapSelection::All,
                run::MissingWdtPolicy::DeclareRootAbsent,
                run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                run::MissingWmoPolicy::Reject,
                Profile::SourceExpandedV2,
            ),
            (
                Target::Build70450,
                plan::MapSelection::All,
                run::MissingWdtPolicy::DeclareRootAbsent,
                run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                material_filtered,
                Profile::DiskWorldV1,
            ),
        ] {
            assert_eq!(
                completion_profile_with_omissions(
                    target,
                    selection,
                    missing_wdt,
                    missing_model,
                    missing_wmo,
                    resource,
                ),
                Err(Error::State)
            );
        }
        for (target, selection, missing_wdt, missing_model, missing_wmo, resource) in [
            (
                Target::Build70425,
                plan::MapSelection::All,
                run::MissingWdtPolicy::DeclareRootAbsent,
                run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                material_filtered,
                Profile::ModelBytesExpandedV1,
            ),
            (
                Target::Build70450,
                plan::MapSelection::MapAndAncestors(7),
                run::MissingWdtPolicy::DeclareRootAbsent,
                run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                material_filtered,
                Profile::ModelBytesExpandedV1,
            ),
            (
                Target::Build70450,
                plan::MapSelection::All,
                run::MissingWdtPolicy::Reject,
                run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                material_filtered,
                Profile::ModelBytesExpandedV1,
            ),
            (
                Target::Build70450,
                plan::MapSelection::All,
                run::MissingWdtPolicy::DeclareRootAbsent,
                run::MissingModelPolicy::Reject,
                material_filtered,
                Profile::ModelBytesExpandedV1,
            ),
            (
                Target::Build70450,
                plan::MapSelection::All,
                run::MissingWdtPolicy::DeclareRootAbsent,
                run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                run::MissingWmoPolicy::Reject,
                Profile::ModelBytesExpandedV1,
            ),
            (
                Target::Build70450,
                plan::MapSelection::All,
                run::MissingWdtPolicy::DeclareRootAbsent,
                run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
                material_filtered,
                Profile::DiskWorldV1,
            ),
        ] {
            assert_eq!(
                completion_profile_with_omissions(
                    target,
                    selection,
                    missing_wdt,
                    missing_model,
                    missing_wmo,
                    resource,
                ),
                Err(Error::State)
            );
        }
    }
}
