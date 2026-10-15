// SPDX-License-Identifier: GPL-3.0-or-later
// Authored synthetic-only available-world resident assembly controls.
use super::*;
use crate::collision::world::tests as fixture;
use crate::collision::{assemble as strict, gameobjects, world};
use crate::local_store::tests::{CollisionFile, CollisionFixture};
use crate::{Locale, local_index, local_store, root};
use forever_db2::catalog;

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
            gameobjects::TABLE_FILE_DATA_ID => "GameObjectDisplayInfo",
            world::MAP_FILE_DATA_ID => "Map",
            world::MATERIAL_FILE_DATA_ID => "LiquidMaterial",
            world::TYPE_FILE_DATA_ID => "LiquidType",
            _ => unreachable!(),
        };
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
    files
}
fn godi(files: &mut [CollisionFile<'_>], rows: &[(u32, u32)]) {
    let hash = word(&files[0].2, 152) as u32;
    files[0].2 = fixture::gameobject_db2(rows, &[]);
    put(&mut files[0].2, 152, hash);
}
fn open(f: &CollisionFixture) -> local_store::LocalStore {
    let path = f.path().join(".build.info");
    let text = std::fs::read_to_string(&path).unwrap();
    std::fs::write(path, text.replace("1.60.1.70205", "1.60.1.70374")).unwrap();
    local_store::LocalStore::open_db2_70374(
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
fn simple() -> Vec<CollisionFile<'static>> {
    let mut fs = files(&[(1, 101, -1)]);
    fs.extend([
        (10, None, fixture::m2(true)),
        (101, None, fixture::wdt(None, Some(201))),
        (201, None, fixture::m2_adt(10, 77)),
    ]);
    fs
}
fn perform(f: &CollisionFixture, limits: Limits) -> Result<Completed, Error> {
    let mut store = open(f);
    let startup = available::extract_startup_fdid(&mut store, gameobjects::Limits::default())
        .map_err(Error::World)?;
    extract(startup, limits)
}
fn names(out: &Completed) -> Vec<String> {
    out.artifacts().map(|a| a.name().to_string()).collect()
}
fn payload(out: &Completed) -> Vec<(String, Vec<u8>)> {
    out.artifacts()
        .map(|a| (a.name().to_string(), a.bytes().to_vec()))
        .collect()
}
fn unknown(mut bytes: Vec<u8>, text: bool) -> Vec<u8> {
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
    out.extend(vec![0xa5; size]);
    out.extend(99u32.to_le_bytes());
    out
}
fn staged(fs: &[CollisionFile<'_>]) -> (available::Completed, Stage) {
    let f = CollisionFixture::new(fs);
    let mut store = open(&f);
    let startup =
        available::extract_startup_fdid(&mut store, gameobjects::Limits::default()).unwrap();
    let mut stage = Stage::new(Limits::default());
    let world = available::extract(startup, &mut stage, world::Limits::default()).unwrap();
    (world, stage)
}
fn view(world: &available::Completed) -> WorldView<'_> {
    WorldView {
        source: world.startup().extraction().models(),
        lists: world.startup().data().lists(),
        topology: world.topology().assembly_topology(),
        resident_bytes: world.resident_bytes(),
        store_resident_bytes: world.store_resident_bytes(),
    }
}
fn record(id: u32, parent: bool) -> Vec<u8> {
    let name = b"FILE0000000A.xxx";
    let mut bytes = vec![0; 38];
    bytes[0] = if parent { 2 } else { 0 };
    put(&mut bytes, 2, id);
    put(&mut bytes, 30, 1f32.to_bits());
    put(&mut bytes, 34, name.len() as u32);
    bytes.extend(name);
    bytes
}
fn parent_files() -> Vec<CollisionFile<'static>> {
    let mut fs = files(&[(1, 101, -1), (2, 102, 1), (3, 103, 2)]);
    godi(&mut fs, &[(7, 10)]);
    fs.extend([
        (10, None, fixture::m2(true)),
        (101, None, fixture::wdt(None, None)),
        (102, None, fixture::wdt(None, None)),
        (103, None, fixture::wdt(None, None)),
    ]);
    fs
}

#[test]
fn empty_package_keeps_available_world_and_exact_dtree() {
    let out = perform(&CollisionFixture::new(&files(&[])), Limits::default()).unwrap();
    assert_eq!(out.profile(), PROFILE);
    assert_eq!(names(&out), ["GameObjectModels.dtree"]);
    assert_eq!(out.artifacts().next().unwrap().bytes(), b"VMAP_4.E");
    assert_eq!(out.stats().files, 1);
    assert_eq!(out.stats().output_bytes, 8);
    assert_eq!(out.world().startup().extraction().facts().reads, 4);
}
#[test]
fn complete_sources_match_every_strict_artifact_byte_and_name() {
    let f = CollisionFixture::new(&simple());
    let mut store = open(&f);
    let startup =
        gameobjects::extract_startup_fdid(&mut store, gameobjects::Limits::default()).unwrap();
    let old = strict::extract(startup, Limits::default()).unwrap();
    let expected = old
        .artifacts()
        .map(|a| (a.name().to_string(), a.bytes().to_vec()))
        .collect::<Vec<_>>();
    drop(store);
    let out = perform(&f, Limits::default()).unwrap();
    assert_eq!(payload(&out), expected);
    assert_eq!(out.stats(), old.stats());
    assert_eq!(
        out.world().startup().extraction().facts(),
        old.world().startup().extraction().facts()
    );
}
#[test]
fn artifact_order_layout_and_borrowed_model_names_remain_exact() {
    let out = perform(&CollisionFixture::new(&simple()), Limits::default()).unwrap();
    assert_eq!(
        names(&out),
        [
            "GameObjectModels.dtree",
            "FILE0000000A.xxx.vmo",
            "0001/0001.vmtree",
            "0001/0001_00_00.vmtile",
            "0001/0001_00_00.vmtileidx"
        ]
    );
    assert_eq!(out.models().next().unwrap().0, b"FILE0000000A.xxx");
}
#[test]
fn empty_active_maps_keep_numeric_order_and_ids_beyond_four_digits() {
    let mut fs = files(&[(70_000, 109, -1), (2, 102, -1)]);
    fs.extend([
        (109, None, fixture::wdt(None, None)),
        (102, None, fixture::wdt(None, None)),
    ]);
    let out = perform(&CollisionFixture::new(&fs), Limits::default()).unwrap();
    assert_eq!(
        names(&out),
        [
            "GameObjectModels.dtree",
            "0002/0002.vmtree",
            "70000/70000.vmtree"
        ]
    );
    assert_eq!(out.maps().len(), 2);
    assert!(out.maps().iter().all(|m| m.tiles().is_empty()));
}
#[test]
fn encrypted_unknown_godi_stays_partial_but_selects_known_startup_model() {
    let mut fs = files(&[]);
    godi(&mut fs, &[(7, 10)]);
    fs[0].2 = unknown(fs[0].2.clone(), false);
    fs.push((10, None, fixture::m2(true)));
    let start = word(&fs[0].2, 252);
    let end = fs[0].2.len();
    let f = CollisionFixture::with_available_holes(
        &fs,
        &[(gameobjects::TABLE_FILE_DATA_ID, start..end)],
    );
    let out = perform(&f, Limits::default()).unwrap();
    assert!(!out.world().startup().data().table().full_ckey_verified);
    assert!(
        out.world()
            .startup()
            .data()
            .table()
            .encoded_key_authenticated
    );
    assert!(!out.world().startup().data().table().all_sections_admitted());
    assert_eq!(out.stats().selected_models, 1);
    assert_eq!(out.stats().files, 2);
    assert_eq!(out.world().startup().data().stats().direct_rows, 1);
}
#[test]
fn encrypted_unknown_map_stays_partial_and_does_not_invent_tree() {
    let mut fs = simple();
    fs[1].2 = unknown(fs[1].2.clone(), true);
    let start = word(&fs[1].2, 252);
    let end = fs[1].2.len();
    let f = CollisionFixture::with_available_holes(&fs, &[(world::MAP_FILE_DATA_ID, start..end)]);
    let out = perform(&f, Limits::default()).unwrap();
    assert!(!out.world().map().full_ckey_verified);
    assert!(out.world().map().encoded_key_authenticated);
    assert_eq!(out.world().map().declared_direct_rows, 2);
    assert_eq!(out.world().map().direct_rows, 1);
    assert_eq!(out.stats().maps, 1);
    assert_eq!(out.maps()[0].map_id(), 1);
    assert!(!names(&out).iter().any(|n| n.starts_with("0099/")));
}
#[test]
fn known_section_hole_never_returns_assembled_owner() {
    let mut fs = files(&[]);
    godi(&mut fs, &[(7, 0)]);
    let start = word(&fs[0].2, 212);
    let end = fs[0].2.len();
    let f = CollisionFixture::with_available_holes(
        &fs,
        &[(gameobjects::TABLE_FILE_DATA_ID, start..end)],
    );
    assert!(perform(&f, Limits::default()).is_err());
}
#[test]
fn missing_known_wdt_is_fatal_without_empty_tree_substitute() {
    let f = CollisionFixture::new(&files(&[(1, 101, -1)]));
    assert!(perform(&f, Limits::default()).is_err());
}
#[test]
fn duplicate_id_records_select_full_model_union_before_dedup() {
    let mut fs = simple();
    fs.last_mut().unwrap().2.extend(fixture::m2_adt(11, 77));
    fs.push((11, None, fixture::m2(true)));
    let out = perform(&CollisionFixture::new(&fs), Limits::default()).unwrap();
    assert_eq!(out.maps()[0].stats().unique_spawns, 1);
    assert_eq!(out.stats().selected_models, 2);
    assert_eq!(out.models().count(), 2);
}
#[test]
fn cached_startup_wmo_survives_lookup_transition_and_final_conversion() {
    let mut fs = files(&[(1, 101, -1)]);
    godi(&mut fs, &[(7, 10)]);
    fs.extend([
        (10, None, fixture::wmo_root(11, None)),
        (11, None, fixture::wmo_group(true, false, true)),
        (20, None, fixture::wmo_root(21, None)),
        (21, None, fixture::wmo_group(true, false, true)),
        (101, None, fixture::wdt(Some(10), Some(201))),
        (201, None, fixture::wmo_adt(20, 8)),
    ]);
    let out = perform(&CollisionFixture::new(&fs), Limits::default()).unwrap();
    assert_eq!(out.world().startup().extraction().models().len(), 2);
    assert_eq!(out.stats().selected_models, 2);
    assert_eq!(out.world().startup().extraction().facts().reads, 10);
    assert_eq!(out.world().report().run.model_callbacks, 2);
}
#[test]
fn inherited_payload_uses_same_validated_admitted_parent_chain() {
    let (world, mut stage) = staged(&parent_files());
    stage.append(1, &record(7, false)).unwrap();
    stage.append(2, &record(7, true)).unwrap();
    stage.append(3, &record(7, true)).unwrap();
    let parts = super::super::build_parts(view(&world), stage, Limits::default()).unwrap();
    let out = Completed { world, parts };
    assert_eq!(out.stats().maps, 3);
    assert!(out.stats().ancestor_steps >= 3);
    assert!(out.maps()[0].tiles()[0].payload().is_some());
    assert!(out.maps()[1].tiles()[0].payload().is_none());
    assert!(out.maps()[2].tiles()[0].payload().is_none());
}
#[test]
fn inherited_payload_mismatch_and_missing_ancestor_remain_terminal() {
    let (world, mut stage) = staged(&parent_files());
    stage.append(1, &record(7, false)).unwrap();
    stage.append(2, &record(8, true)).unwrap();
    assert!(super::super::build_parts(view(&world), stage, Limits::default()).is_err());
    let (world, mut stage) = staged(&parent_files());
    stage.append(2, &record(7, true)).unwrap();
    assert_eq!(
        super::super::build_parts(view(&world), stage, Limits::default()).err(),
        Some(strict::Error::MissingAncestor)
    );
}
#[test]
fn stage_error_keeps_priority_over_world_sink_error() {
    let f = CollisionFixture::new(&simple());
    assert_eq!(
        perform(
            &f,
            Limits {
                max_records: 0,
                ..Limits::default()
            }
        )
        .err(),
        Some(Error::Assembly(strict::Error::Limit))
    );
}
#[test]
fn stage_partition_and_resident_output_accounting_remain_combined() {
    let f = CollisionFixture::new(&simple());
    assert!(
        perform(
            &f,
            Limits {
                max_memory_bytes: 1,
                max_stage_bytes: 1,
                ..Limits::default()
            }
        )
        .is_err()
    );
    let out = perform(&f, Limits::default()).unwrap();
    let minimum = out.world().resident_bytes()
        + out.world().store_resident_bytes()
        + out
            .maps()
            .iter()
            .map(|m| m.stats().resident_bytes)
            .sum::<usize>()
        + out
            .models()
            .map(|(_, m)| m.stats().resident_bytes)
            .sum::<usize>();
    assert!(out.stats().resident_bytes >= minimum);
    assert!(out.stats().peak_bytes >= out.stats().resident_bytes);
    assert_eq!(
        out.artifacts().map(|a| a.bytes().len()).sum::<usize>(),
        out.stats().output_bytes
    );
    assert_eq!(out.artifacts().count(), out.stats().files);
}
#[test]
fn exact_output_boundaries_and_downward_work_limits_remain_fatal() {
    let f = CollisionFixture::new(&simple());
    let first = perform(&f, Limits::default()).unwrap();
    let stats = first.stats();
    drop(first);
    assert!(
        perform(
            &f,
            Limits {
                max_files: stats.files,
                max_output_bytes: stats.output_bytes,
                ..Limits::default()
            }
        )
        .is_ok()
    );
    for limits in [
        Limits {
            max_files: stats.files - 1,
            ..Limits::default()
        },
        Limits {
            max_output_bytes: stats.output_bytes - 1,
            ..Limits::default()
        },
        Limits {
            max_raw_visit_bytes: 0,
            ..Limits::default()
        },
        Limits {
            max_bih_work: 0,
            ..Limits::default()
        },
        Limits {
            max_vertices: 2,
            ..Limits::default()
        },
    ] {
        assert!(perform(&f, limits).is_err());
    }
}
#[test]
fn changed_source_after_startup_rejects_before_assembly() {
    let f = CollisionFixture::new(&simple());
    let mut store = open(&f);
    let startup =
        available::extract_startup_fdid(&mut store, gameobjects::Limits::default()).unwrap();
    f.change_metadata();
    assert!(extract(startup, Limits::default()).is_err());
}
