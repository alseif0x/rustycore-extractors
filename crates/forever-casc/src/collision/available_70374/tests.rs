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
