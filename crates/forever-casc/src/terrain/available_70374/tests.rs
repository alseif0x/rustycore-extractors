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
