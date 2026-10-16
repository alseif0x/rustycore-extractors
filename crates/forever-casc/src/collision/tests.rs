// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: complete synthetic LocalStore-to-collision integration.
use super::*;
use crate::local_store::tests::{CollisionFile, CollisionFixture};

struct Lookup;
impl LiquidLookup for Lookup {
    fn material_id(&self, _: u32) -> Option<u8> {
        None
    }
    fn material_flags(&self, _: u8) -> Option<u32> {
        None
    }
}
fn put(bytes: &mut [u8], at: usize, value: u32) {
    bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
}
fn words(values: &[u32]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}
fn chunk(tag: &[u8; 4], body: &[u8]) -> Vec<u8> {
    let mut out: Vec<_> = tag.iter().rev().copied().collect();
    out.extend_from_slice(&(body.len() as u32).to_le_bytes());
    out.extend_from_slice(body);
    out
}
fn m2(faces: bool) -> Vec<u8> {
    let mut b = vec![0; 304];
    b[..4].copy_from_slice(b"MD20");
    if faces {
        put(&mut b, 216, 3);
        put(&mut b, 220, 304);
        put(&mut b, 224, 3);
        put(&mut b, 228, 310);
        for v in [0u16, 1, 2] {
            b.extend_from_slice(&v.to_le_bytes());
        }
        b.extend(words(&[
            0x8000_0000,
            0,
            0,
            0x3f80_0000,
            0,
            0,
            0,
            0x3f80_0000,
            0,
        ]));
    }
    b
}
fn root(ids: &[u32], doodads: &[u32], sets: bool) -> Vec<u8> {
    let mut h = [0; 64];
    put(&mut h, 4, ids.len() as u32);
    put(&mut h, 32, 777);
    h[60] = 4;
    let mut b = chunk(b"MOHD", &h);
    b.extend(chunk(b"GFID", &words(ids)));
    b.extend(chunk(b"MODI", &words(doodads)));
    let mut spawn = [0; 40];
    put(&mut spawn, 32, 1f32.to_bits());
    b.extend(chunk(b"MODD", &spawn));
    if sets {
        let mut set = [0; 32];
        put(&mut set, 24, 1);
        b.extend(chunk(b"MODS", &set));
    }
    b
}
fn group(faces: bool, skip: bool, refs: &[u16]) -> Vec<u8> {
    let mut h = vec![0; 68];
    put(&mut h, 56, 99);
    put(&mut h, 8, if skip { 0x80 } else { 0 });
    if faces {
        h.extend(chunk(b"MOPY", &[8, 0]));
        h.extend(chunk(b"MOVI", &[0, 0, 1, 0, 2, 0]));
        h.extend(chunk(
            b"MOVT",
            &words(&[0, 0, 0, 0x3f80_0000, 0, 0, 0, 0x3f80_0000, 0]),
        ));
    }
    h.extend(chunk(
        b"MODR",
        &refs
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect::<Vec<_>>(),
    ));
    chunk(b"MOGP", &h)
}
fn named_root(ids: &[u32], names: &[u8]) -> Vec<u8> {
    let mut bytes = root(ids, &[], true);
    let at = 72 + 8 + ids.len() * 4;
    bytes.splice(at..at + 8, chunk(b"MODN", names));
    bytes
}
fn request(id: u32) -> ModelRequest<'static> {
    ModelRequest::M2(InputRef::FileDataId(id))
}
fn wmo_request(id: u32) -> ModelRequest<'static> {
    ModelRequest::Wmo(InputRef::FileDataId(id))
}
fn run(files: &[CollisionFile<'_>], request: ModelRequest<'_>) -> Result<Extraction, Error> {
    let fixture = CollisionFixture::new(files);
    let mut store = fixture.open(false);
    let mut e = Extractor::new(&mut store, &Lookup, Limits::default())?;
    e.extract(request)?;
    e.finish()
}

#[test]
fn complete_store_m2_converts_raw_and_reuses_identical_requests() {
    let fixture = CollisionFixture::new(&[(10, None, m2(true))]);
    let mut store = fixture.open(false);
    let mut e = Extractor::new(&mut store, &Lookup, Limits::default()).unwrap();
    let first = e.extract(request(10)).unwrap();
    assert_eq!(first, e.extract(request(10)).unwrap());
    let result = e.finish().unwrap();
    assert_eq!(first.index(), 0);
    assert_eq!(result.requests(), &[first, first]);
    let model = result.model(first).unwrap();
    assert_eq!(model.status(), Status::Collision);
    assert_eq!(model.class(), Class::M2);
    assert_eq!(model.output_basename(), b"FILE0000000A.xxx");
    let parsed = RawM2::parse(model.raw().unwrap(), forever_vmap::Limits::default()).unwrap();
    assert_eq!(parsed.index_count(), 3);
    assert_eq!(model.source().unwrap().file_data_id(), 10);
    assert_eq!(result.facts().reads, 1);
    assert_eq!(result.facts().source_bytes, m2(true).len());
}
#[test]
fn m2_no_collision_is_explicit_and_has_no_raw() {
    let result = run(&[(10, None, m2(false))], request(10)).unwrap();
    assert_eq!(result.models()[0].status(), Status::NoCollision);
    assert!(result.models()[0].raw().is_none());
    assert_eq!(result.facts().raw_bytes, 0);
}
#[test]
fn wmo_reads_doodads_before_groups_and_preserves_owned_metadata() {
    let fixture = CollisionFixture::new(&[
        (10, None, root(&[30], &[20], true)),
        (20, None, m2(true)),
        (30, None, group(true, false, &[0])),
    ]);
    let mut store = fixture.open(false);
    let mut e = Extractor::new(&mut store, &Lookup, Limits::default()).unwrap();
    let handle = e.extract(wmo_request(10)).unwrap();
    let result = e.finish().unwrap();
    assert_eq!(result.facts().reads, 3);
    assert_eq!(result.models().len(), 2);
    let model = result.model(handle).unwrap();
    assert_eq!(model.status(), Status::Collision);
    let raw = raw::RawWmo::parse(model.raw().unwrap(), raw::ConversionLimits::default()).unwrap();
    assert_eq!(raw.triangle_count(), 1);
    let metadata = model.wmo_metadata().unwrap();
    assert_eq!(metadata.bindings()[0].name_index, 0);
    assert_eq!(metadata.bindings()[0].model.index(), 1);
    assert_eq!(metadata.selection().references(), &[0]);
    assert_eq!(metadata.selection().set_references(0).count(), 1);
    assert_eq!(
        metadata
            .root()
            .unwrap()
            .detailed_group_ids()
            .collect::<Vec<_>>(),
        [30]
    );
    assert_eq!(metadata.group_sources()[0].file_data_id(), 30);
}
#[test]
fn zero_geometry_wmo_retains_valid_doodads_without_a_raw_file() {
    let result = run(
        &[
            (10, None, root(&[30], &[20], true)),
            (20, None, m2(true)),
            (30, None, group(false, false, &[0])),
        ],
        wmo_request(10),
    )
    .unwrap();
    assert_eq!(result.models()[0].status(), Status::DoodadOnly);
    assert!(result.models()[0].raw().is_none());
    assert_eq!(
        result.models()[0]
            .wmo_metadata()
            .unwrap()
            .selection()
            .references(),
        &[0]
    );
}
#[test]
fn m2_without_geometry_never_validates_doodad_reference() {
    assert_eq!(
        run(
            &[
                (10, None, root(&[30], &[20], true)),
                (20, None, m2(false)),
                (30, None, group(false, false, &[0]))
            ],
            wmo_request(10)
        )
        .err(),
        Some(Error::NoCollision)
    );
}
#[test]
fn zero_geometry_wmo_requires_both_sets_and_references() {
    for sets in [false, true] {
        let refs = if sets { vec![] } else { vec![0] };
        assert_eq!(
            run(
                &[
                    (10, None, root(&[30], &[20], sets)),
                    (20, None, m2(true)),
                    (30, None, group(false, false, &refs))
                ],
                wmo_request(10)
            )
            .err(),
            Some(Error::NoCollision)
        );
    }
}
#[test]
fn unused_missing_doodad_fails_before_any_group_read() {
    let fixture = CollisionFixture::new(&[
        (10, None, root(&[30], &[20], false)),
        (30, None, group(true, false, &[])),
    ]);
    let mut store = fixture.open(false);
    let mut e = Extractor::new(&mut store, &Lookup, Limits::default()).unwrap();
    let error = e.extract(wmo_request(10)).unwrap_err();
    assert_eq!(error, Error::Store(local_store::Error::MissingFile));
    assert_eq!(e.reads, 2);
    assert_eq!(e.extract(request(30)), Err(error));
    assert_eq!(e.reads, 2);
    assert_eq!(e.finish().err(), Some(error));
}
#[test]
fn missing_or_malformed_group_prevents_partial_results() {
    for extra in [None, Some(b"broken".to_vec())] {
        let mut files = vec![
            (10, None, root(&[30, 31], &[], false)),
            (30, None, group(true, false, &[])),
        ];
        if let Some(bytes) = extra {
            files.push((31, None, bytes));
        }
        assert!(run(&files, wmo_request(10)).is_err());
    }
}
#[test]
fn skipped_group_is_still_acquired_and_invalid_source_is_terminal() {
    let files = [
        (10, None, root(&[30, 31], &[], false)),
        (30, None, group(true, false, &[])),
        (31, None, group(false, true, &[0])),
    ];
    let result = run(&files, wmo_request(10)).unwrap();
    assert_eq!(result.facts().reads, 3);
    assert_eq!(
        result.models()[0]
            .wmo_metadata()
            .unwrap()
            .group_sources()
            .len(),
        2
    );
    assert!(run(&files[..2], wmo_request(10)).is_err());
}
#[test]
fn duplicate_modi_targets_are_acquired_once_and_keep_each_name_index() {
    let result = run(
        &[
            (10, None, root(&[30], &[20, 20, 0], true)),
            (20, None, m2(true)),
            (30, None, group(true, false, &[0])),
        ],
        wmo_request(10),
    )
    .unwrap();
    let bindings = result.models()[0].wmo_metadata().unwrap().bindings();
    assert_eq!(bindings.len(), 2);
    assert_eq!((bindings[0].name_index, bindings[1].name_index), (0, 1));
    assert_eq!(bindings[0].model, bindings[1].model);
    assert_eq!(result.facts().reads, 3);
}
#[test]
fn dependency_request_cap_counts_cache_hits_but_not_zero_modi() {
    let fixture = CollisionFixture::new(&[
        (10, None, root(&[30], &[20, 0, 20], true)),
        (20, None, m2(true)),
        (30, None, group(true, false, &[0])),
    ]);
    let mut store = fixture.open(false);
    let mut e = Extractor::new(
        &mut store,
        &Lookup,
        Limits {
            max_dependency_requests: 2,
            ..Limits::default()
        },
    )
    .unwrap();
    e.extract(wmo_request(10)).unwrap();
    let result = e.finish().unwrap();
    assert_eq!(result.facts().dependency_requests, 2);
    assert_eq!(result.facts().reads, 3);
    for max in [0, 1] {
        let mut store = fixture.open(false);
        let mut e = Extractor::new(
            &mut store,
            &Lookup,
            Limits {
                max_dependency_requests: max,
                ..Limits::default()
            },
        )
        .unwrap();
        assert_eq!(e.extract(wmo_request(10)), Err(Error::Limit));
        assert_eq!(e.reads, 1 + max);
        assert_eq!(e.dependency_requests, max);
        assert_eq!(e.finish().err(), Some(Error::Limit));
    }
    let mut store = fixture.open(false);
    assert_eq!(
        Extractor::new(
            &mut store,
            &Lookup,
            Limits {
                max_dependency_requests: 4097,
                ..Limits::default()
            }
        )
        .err(),
        Some(Error::InvalidLimits)
    );
}
#[test]
fn modn_acquires_unused_names_and_retains_byte_offset_bindings() {
    let fixture = CollisionFixture::new(&[
        (10, None, named_root(&[30], b"A/one.mdx\0B/two.m2\0")),
        (20, Some(b"A/one.m2".as_slice()), m2(true)),
        (21, Some(b"B/two.m2".as_slice()), m2(true)),
        (30, None, group(true, false, &[0])),
    ]);
    let mut store = fixture.open(true);
    let mut e = Extractor::new(&mut store, &Lookup, Limits::default()).unwrap();
    e.extract(wmo_request(10)).unwrap();
    let result = e.finish().unwrap();
    assert_eq!(result.facts().reads, 4);
    let metadata = result.models()[0].wmo_metadata().unwrap();
    assert_eq!(
        metadata
            .bindings()
            .iter()
            .map(|b| b.name_index)
            .collect::<Vec<_>>(),
        [0, 10]
    );
    assert_eq!(metadata.selection().references(), &[0]);
    assert_eq!(result.models().len(), 3);
}
#[test]
fn empty_modn_name_is_not_silently_skipped() {
    assert_eq!(
        run(&[(10, None, named_root(&[], b"\0"))], wmo_request(10)).err(),
        Some(Error::Name)
    );
}
#[test]
fn self_doodad_is_a_terminal_cycle_before_reacquisition() {
    let fixture = CollisionFixture::new(&[(10, None, root(&[], &[10], true))]);
    let mut store = fixture.open(false);
    let mut e = Extractor::new(&mut store, &Lookup, Limits::default()).unwrap();
    assert_eq!(e.extract(wmo_request(10)), Err(Error::Cycle));
    assert_eq!(e.reads, 1);
    assert_eq!(e.finish().err(), Some(Error::Cycle));
}
#[test]
fn names_require_opt_in_and_m2_extension_rewrite_uses_full_path() {
    let fixture = CollisionFixture::new(&[(10, Some(b"Dir/Big Model.m2".as_slice()), m2(true))]);
    let req = ModelRequest::M2(InputRef::LogicalName(b"Dir/Big Model.MDX"));
    let mut store = fixture.open(false);
    let mut e = Extractor::new(&mut store, &Lookup, Limits::default()).unwrap();
    assert_eq!(
        e.extract(req),
        Err(Error::Store(local_store::Error::NamesDisabled))
    );
    let mut store = fixture.open(true);
    let mut e = Extractor::new(&mut store, &Lookup, Limits::default()).unwrap();
    let a = e.extract(req).unwrap();
    let b = e
        .extract(ModelRequest::M2(InputRef::LogicalName(
            b"DIR\\Big Model.m2",
        )))
        .unwrap();
    assert_eq!(a, b);
    let result = e.finish().unwrap();
    assert_eq!(result.facts().reads, 1);
    assert_eq!(result.models()[0].output_basename(), b"Big_Model.m2");
}
#[test]
fn distinct_source_paths_cannot_share_normalized_output_basename() {
    let fixture = CollisionFixture::new(&[
        (10, Some(b"A/Foo.m2".as_slice()), m2(true)),
        (20, Some(b"B/Foo.m2".as_slice()), m2(true)),
    ]);
    let mut store = fixture.open(true);
    let mut e = Extractor::new(&mut store, &Lookup, Limits::default()).unwrap();
    e.extract(ModelRequest::M2(InputRef::LogicalName(b"A/Foo.m2")))
        .unwrap();
    assert_eq!(
        e.extract(ModelRequest::M2(InputRef::LogicalName(b"B/Foo.m2"))),
        Err(Error::Alias)
    );
    assert_eq!(e.reads, 1);
    assert_eq!(e.finish().err(), Some(Error::Alias));
}
#[test]
fn same_numeric_origin_under_different_class_is_an_alias() {
    let fixture = CollisionFixture::new(&[(10, None, m2(true))]);
    let mut store = fixture.open(false);
    let mut e = Extractor::new(&mut store, &Lookup, Limits::default()).unwrap();
    e.extract(request(10)).unwrap();
    assert_eq!(e.extract(wmo_request(10)), Err(Error::Alias));
    assert_eq!(e.reads, 1);
}
#[test]
fn logical_file_text_does_not_fall_back_to_numeric_routing() {
    let fixture = CollisionFixture::new(&[(10, None, m2(true))]);
    let mut store = fixture.open(true);
    let mut e = Extractor::new(&mut store, &Lookup, Limits::default()).unwrap();
    assert_eq!(
        e.extract(ModelRequest::M2(InputRef::LogicalName(b"FILE0000000A.xxx"))),
        Err(Error::Store(local_store::Error::MissingFile))
    );
}
#[test]
fn group_name_heuristic_and_invalid_routes_fail_before_io() {
    let fixture = CollisionFixture::new(&[(10, None, m2(true))]);
    for req in [
        wmo_request(0),
        request(u32::MAX),
        ModelRequest::Wmo(InputRef::LogicalName(b"a_1234.wmo")),
        ModelRequest::M2(InputRef::LogicalName(b"abc")),
    ] {
        let mut store = fixture.open(false);
        let mut e = Extractor::new(&mut store, &Lookup, Limits::default()).unwrap();
        assert!(e.extract(req).is_err());
        assert_eq!(e.reads, 0);
    }
}
#[test]
fn metadata_mutation_after_conversion_is_rejected_at_finish() {
    let fixture = CollisionFixture::new(&[(10, None, m2(true))]);
    let mut store = fixture.open(false);
    let mut e = Extractor::new(&mut store, &Lookup, Limits::default()).unwrap();
    e.extract(request(10)).unwrap();
    fixture.change_metadata();
    assert_eq!(
        e.finish().err(),
        Some(Error::Store(local_store::Error::InputChanged))
    );
}
#[test]
fn metadata_mutation_poison_applies_even_to_cached_requests() {
    let fixture = CollisionFixture::new(&[(10, None, m2(true))]);
    let mut store = fixture.open(false);
    let mut e = Extractor::new(&mut store, &Lookup, Limits::default()).unwrap();
    e.extract(request(10)).unwrap();
    fixture.change_metadata();
    let error = Error::Store(local_store::Error::InputChanged);
    assert_eq!(e.extract(request(10)), Err(error));
    assert_eq!(e.extract(request(11)), Err(error));
    assert_eq!(e.reads, 1);
    assert_eq!(e.finish().err(), Some(error));
}
#[test]
fn incomplete_archive_is_terminal_and_never_yields_prior_outputs() {
    let fixture = CollisionFixture::new(&[(10, None, m2(true))]);
    let mut store = fixture.open(false);
    fixture.truncate_archive();
    let mut e = Extractor::new(&mut store, &Lookup, Limits::default()).unwrap();
    let error = e.extract(request(10)).unwrap_err();
    assert!(matches!(
        error,
        Error::Store(local_store::Error::NoLocalEncoding(_))
    ));
    assert_eq!(e.extract(request(10)), Err(error));
    assert_eq!(e.reads, 1);
    assert_eq!(e.finish().err(), Some(error));
}
#[test]
fn malformed_m2_is_not_reported_as_no_collision() {
    assert!(matches!(
        run(&[(10, None, b"MD20".to_vec())], request(10)),
        Err(Error::M2(_))
    ));
}
#[test]
fn reduced_reads_models_requests_source_and_raw_caps_are_enforced() {
    let fixture = CollisionFixture::new(&[(10, None, m2(true)), (20, None, m2(true))]);
    for limits in [
        Limits {
            max_reads: 0,
            ..Limits::default()
        },
        Limits {
            max_models: 0,
            ..Limits::default()
        },
        Limits {
            max_requests: 0,
            ..Limits::default()
        },
        Limits {
            max_file_bytes: 1,
            ..Limits::default()
        },
        Limits {
            max_source_bytes: 1,
            ..Limits::default()
        },
        Limits {
            max_raw_bytes: 0,
            ..Limits::default()
        },
    ] {
        let mut store = fixture.open(false);
        let mut e = Extractor::new(&mut store, &Lookup, limits).unwrap();
        assert!(e.extract(request(10)).is_err());
        assert!(e.finish().is_err());
    }
    for limits in [
        Limits {
            max_reads: 1,
            ..Limits::default()
        },
        Limits {
            max_models: 1,
            ..Limits::default()
        },
        Limits {
            max_requests: 1,
            ..Limits::default()
        },
        Limits {
            max_source_bytes: m2(true).len(),
            ..Limits::default()
        },
    ] {
        let mut store = fixture.open(false);
        let mut e = Extractor::new(&mut store, &Lookup, limits).unwrap();
        e.extract(request(10)).unwrap();
        assert!(e.extract(request(20)).is_err());
        assert!(e.finish().is_err());
    }
}
#[test]
fn aggregate_raw_cap_counts_retained_models_but_not_cache_hits() {
    let raw_len = M2Collision::parse(&m2(true), forever_vmap::Limits::default())
        .unwrap()
        .unwrap()
        .write_raw(forever_vmap::MAX_RAW_BYTES)
        .unwrap()
        .len();
    let fixture = CollisionFixture::new(&[(10, None, m2(true)), (20, None, m2(true))]);
    let mut store = fixture.open(false);
    let mut e = Extractor::new(
        &mut store,
        &Lookup,
        Limits {
            max_raw_bytes: raw_len,
            ..Limits::default()
        },
    )
    .unwrap();
    e.extract(request(10)).unwrap();
    e.extract(request(10)).unwrap();
    assert!(e.extract(request(20)).is_err());
}
#[test]
fn no_collision_m2_needs_no_output_budget() {
    let fixture = CollisionFixture::new(&[(10, None, m2(false))]);
    let mut store = fixture.open(false);
    let mut e = Extractor::new(
        &mut store,
        &Lookup,
        Limits {
            max_raw_bytes: 0,
            ..Limits::default()
        },
    )
    .unwrap();
    e.extract(request(10)).unwrap();
    assert_eq!(e.finish().unwrap().facts().raw_bytes, 0);
}
#[test]
fn live_heap_counter_matches_all_retained_capacities_after_wmo() {
    let fixture = CollisionFixture::new(&[
        (10, None, root(&[30], &[20], true)),
        (20, None, m2(true)),
        (30, None, group(true, false, &[0])),
    ]);
    let mut store = fixture.open(false);
    let mut e = Extractor::new(&mut store, &Lookup, Limits::default()).unwrap();
    e.extract(wmo_request(10)).unwrap();
    let mut retained = capacity(&e.models).unwrap() + capacity(&e.requests).unwrap();
    for model in &e.models {
        retained += model.name.capacity();
        if let Origin::Name(name) = &model.origin {
            retained += name.capacity();
        }
        retained += model.raw.as_ref().map_or(0, Vec::capacity);
        if let Some(wmo) = &model.wmo {
            retained += wmo.root_bytes.capacity()
                + capacity(&wmo.bindings).unwrap()
                + wmo.selection.resident_bytes()
                + capacity(&wmo.groups).unwrap();
        }
    }
    assert_eq!(e.live, retained);
}
#[test]
fn limits_validate_before_guards_and_external_memory_is_charged() {
    let fixture = CollisionFixture::new(&[(10, None, m2(true))]);
    let mut store = fixture.open(false);
    let resident = store.stats().resident_bytes;
    assert_eq!(
        Extractor::new(
            &mut store,
            &Lookup,
            Limits {
                max_memory_bytes: resident - 1,
                ..Limits::default()
            }
        )
        .err(),
        Some(Error::Limit)
    );
    let mut e = Extractor::new(
        &mut store,
        &Lookup,
        Limits {
            max_memory_bytes: resident,
            ..Limits::default()
        },
    )
    .unwrap();
    assert_eq!(e.extract(request(10)), Err(Error::Limit));
    drop(e);
    fixture.change_metadata();
    assert_eq!(
        Extractor::new(
            &mut store,
            &Lookup,
            Limits {
                max_reads: 4097,
                ..Limits::default()
            }
        )
        .err(),
        Some(Error::InvalidLimits)
    );
}
#[test]
fn external_live_budget_applies_to_both_store_routes_and_overflow() {
    let fixture = CollisionFixture::new(&[(10, Some(b"A.m2".as_slice()), m2(true))]);
    let mut store = fixture.open(true);
    let resident = store.stats().resident_bytes;
    let cap = resident + 4096;
    assert_eq!(
        store
            .read_file_data_id_with_external_budget(10, 4096, usize::MAX, cap)
            .err(),
        Some(local_store::Error::Limit)
    );
    assert_eq!(
        store
            .read_name_with_external_budget(b"A.m2", 4096, 4096, cap)
            .err(),
        Some(local_store::Error::Limit)
    );
    assert_eq!(
        store
            .read_file_data_id_with_external_budget(10, 4096, 4096, cap)
            .err(),
        Some(local_store::Error::Limit)
    );
    assert_eq!(
        store
            .read_file_data_id_with_external_budget(10, 4096, 1, cap)
            .unwrap()
            .bytes,
        m2(true)
    );
    assert_eq!(
        store
            .read_name_with_external_budget(b"A.m2", 4096, 1, cap)
            .unwrap()
            .bytes,
        m2(true)
    );
}

// Authored 2026-10-11: first-magic startup routing and persistent lookup state.
fn startup_root(ids: &[u32], doodads: &[u32], sets: bool) -> Vec<u8> {
    let mut bytes = chunk(b"MVER", &17u32.to_le_bytes());
    bytes.extend(root(ids, doodads, sets));
    bytes
}
fn gameobject_model(outcome: GameObjectOutcome) -> ModelHandle {
    match outcome {
        GameObjectOutcome::Model(handle) => handle,
        GameObjectOutcome::Texture => panic!("synthetic model unexpectedly classified as texture"),
    }
}

#[test]
fn gameobject_same_complete_m2_read_and_repeated_handles() {
    let mut md21 = b"MD21\x00\x00\x00\x00".to_vec();
    md21.extend(m2(true));
    let fixture = CollisionFixture::new(&[(10, None, m2(true)), (11, None, md21)]);
    let mut store = fixture.open(false);
    let mut e = Extractor::new(&mut store, &Lookup, Limits::default()).unwrap();
    let a = gameobject_model(e.extract_gameobject_fdid(10).unwrap());
    let b = gameobject_model(e.extract_gameobject_fdid(11).unwrap());
    assert_eq!(gameobject_model(e.extract_gameobject_fdid(10).unwrap()), a);
    assert_eq!(e.request_calls(), 3);
    assert_eq!(e.facts().reads, 2);
    assert_eq!(
        e.model(a).unwrap().source().unwrap().first_magic(),
        Some(*b"MD20")
    );
    assert_eq!(
        e.model(b).unwrap().source().unwrap().first_magic(),
        Some(*b"MD21")
    );
    assert_eq!(e.model(a).unwrap().raw(), e.model(b).unwrap().raw());
    let result = e.finish().unwrap();
    assert_eq!(result.requests(), &[a, b, a]);
}

#[test]
fn gameobject_wmo_dependencies_all_retain_source_magic() {
    let fixture = CollisionFixture::new(&[
        (10, None, startup_root(&[30], &[20], true)),
        (20, None, m2(true)),
        (30, None, group(true, false, &[0])),
    ]);
    let mut store = fixture.open(false);
    let mut e = Extractor::new(&mut store, &Lookup, Limits::default()).unwrap();
    let root = gameobject_model(e.extract_gameobject_fdid(10).unwrap());
    let model = e.model(root).unwrap();
    assert_eq!(model.class(), Class::Wmo);
    assert_eq!(model.source().unwrap().first_magic(), Some(*b"REVM"));
    let metadata = model.wmo_metadata().unwrap();
    assert_eq!(metadata.group_sources()[0].first_magic(), Some(*b"PGOM"));
    let dependency = metadata.bindings()[0].model;
    assert_eq!(
        e.model(dependency).unwrap().source().unwrap().first_magic(),
        Some(*b"MD20")
    );
    assert_eq!(
        gameobject_model(e.extract_gameobject_fdid(20).unwrap()),
        dependency
    );
    assert_eq!(e.facts().reads, 3);
    assert_eq!(e.facts().dependency_requests, 1);
    assert_eq!(e.finish().unwrap().requests(), &[root, dependency]);
}

#[test]
fn gameobject_no_collision_and_doodad_only_remain_explicit_models() {
    let fixture = CollisionFixture::new(&[
        (10, None, startup_root(&[30], &[20], true)),
        (20, None, m2(true)),
        (30, None, group(false, false, &[0])),
        (40, None, m2(false)),
    ]);
    let mut store = fixture.open(false);
    let mut e = Extractor::new(&mut store, &Lookup, Limits::default()).unwrap();
    let wmo = gameobject_model(e.extract_gameobject_fdid(10).unwrap());
    let m2 = gameobject_model(e.extract_gameobject_fdid(40).unwrap());
    assert_eq!(e.model(wmo).unwrap().status(), Status::DoodadOnly);
    assert_eq!(e.model(m2).unwrap().status(), Status::NoCollision);
    assert!(e.model(wmo).unwrap().raw().is_none());
    assert!(e.model(m2).unwrap().raw().is_none());
    e.finish().unwrap();
}

#[test]
fn gameobject_texture_uses_calls_reads_and_source_but_no_model_budget() {
    let fixture = CollisionFixture::new(&[(10, None, b"BLP2synthetic".to_vec())]);
    let mut store = fixture.open(false);
    let mut e = Extractor::new(
        &mut store,
        &Lookup,
        Limits {
            max_models: 0,
            max_requests: 2,
            ..Limits::default()
        },
    )
    .unwrap();
    for _ in 0..2 {
        assert_eq!(
            e.extract_gameobject_fdid(10),
            Ok(GameObjectOutcome::Texture)
        );
    }
    assert_eq!(e.request_calls(), 2);
    assert_eq!(e.facts().reads, 2);
    assert_eq!(e.facts().source_bytes, 26);
    assert_eq!(e.retained_bytes(), 0);
    let result = e.finish().unwrap();
    assert!(result.models().is_empty());
    assert!(result.requests().is_empty());
}

#[test]
fn gameobject_texture_exhausts_shared_request_budget_before_next_read() {
    let fixture = CollisionFixture::new(&[(10, None, b"BLP2".to_vec()), (20, None, m2(true))]);
    let mut store = fixture.open(false);
    let mut e = Extractor::new(
        &mut store,
        &Lookup,
        Limits {
            max_requests: 1,
            ..Limits::default()
        },
    )
    .unwrap();
    e.extract_gameobject_fdid(10).unwrap();
    assert_eq!(e.extract(request(20)), Err(Error::Limit));
    assert_eq!(e.facts().reads, 1);
    assert_eq!(e.extract_gameobject_fdid(10), Err(Error::Limit));
    assert_eq!(e.finish().err(), Some(Error::Limit));
}

#[test]
fn gameobject_texture_requires_complete_source_and_respects_source_cap() {
    let fixture = CollisionFixture::new(&[(10, None, b"BLP2not_a_prefix_read".to_vec())]);
    let mut store = fixture.open(false);
    let mut e = Extractor::new(
        &mut store,
        &Lookup,
        Limits {
            max_file_bytes: 4,
            ..Limits::default()
        },
    )
    .unwrap();
    assert!(e.extract_gameobject_fdid(10).is_err());
    assert_eq!(e.models.len(), 0);
    assert!(e.finish().is_err());
}

#[test]
fn gameobject_unknown_short_and_invalid_ids_poison() {
    for bytes in [b"????".as_slice(), b"MD2", b"R", b"MD20"].map(<[u8]>::to_vec) {
        let fixture = CollisionFixture::new(&[(10, None, bytes.clone())]);
        let mut store = fixture.open(false);
        let mut e = Extractor::new(&mut store, &Lookup, Limits::default()).unwrap();
        let error = e.extract_gameobject_fdid(10).unwrap_err();
        if bytes != b"MD20" {
            assert_eq!(error, Error::GameObjectMagic);
        } else {
            assert!(matches!(error, Error::M2(_)));
        }
        let reads = e.facts().reads;
        assert_eq!(e.extract_gameobject_fdid(10), Err(error));
        assert_eq!(e.extract(request(10)), Err(error));
        assert_eq!(e.facts().reads, reads);
        assert_eq!(e.finish().err(), Some(error));
    }
    for id in [0, u32::MAX] {
        let fixture = CollisionFixture::new(&[(10, None, m2(true))]);
        let mut store = fixture.open(false);
        let mut e = Extractor::new(&mut store, &Lookup, Limits::default()).unwrap();
        assert_eq!(e.extract_gameobject_fdid(id), Err(Error::Name));
        assert_eq!(e.facts().reads, 0);
    }
}

#[test]
fn gameobject_never_infers_first_magic_from_successful_cached_parser() {
    let mut prefixed = b"JUNK".to_vec();
    prefixed.extend(m2(true));
    let fixture = CollisionFixture::new(&[(10, None, prefixed)]);
    let mut store = fixture.open(false);
    let mut e = Extractor::new(&mut store, &Lookup, Limits::default()).unwrap();
    let handle = e.extract(request(10)).unwrap();
    assert_eq!(e.model(handle).unwrap().status(), Status::Collision);
    assert_eq!(e.extract_gameobject_fdid(10), Err(Error::GameObjectMagic));
    assert_eq!(e.facts().reads, 1);
    assert_eq!(e.finish().err(), Some(Error::GameObjectMagic));
}

#[test]
fn gameobject_cached_dependency_still_requires_its_original_first_magic() {
    let mut prefixed = b"JUNK".to_vec();
    prefixed.extend(m2(true));
    let fixture = CollisionFixture::new(&[
        (10, None, startup_root(&[30], &[20], true)),
        (20, None, prefixed),
        (30, None, group(true, false, &[0])),
    ]);
    let mut store = fixture.open(false);
    let mut e = Extractor::new(&mut store, &Lookup, Limits::default()).unwrap();
    e.extract_gameobject_fdid(10).unwrap();
    assert_eq!(e.extract_gameobject_fdid(20), Err(Error::GameObjectMagic));
    assert_eq!(e.facts().reads, 3);
}

#[test]
fn gameobject_cached_typed_wmo_without_mver_is_not_a_startup_wmo() {
    let fixture = CollisionFixture::new(&[
        (10, None, root(&[30], &[], false)),
        (30, None, group(true, false, &[])),
    ]);
    let mut store = fixture.open(false);
    let mut e = Extractor::new(&mut store, &Lookup, Limits::default()).unwrap();
    e.extract(wmo_request(10)).unwrap();
    assert_eq!(e.extract_gameobject_fdid(10), Err(Error::GameObjectMagic));
    assert_eq!(e.facts().reads, 2);
}

#[test]
fn gameobject_cached_texture_magic_overrides_typed_scanner_success() {
    let mut prefixed = b"BLP2".to_vec();
    prefixed.extend(m2(true));
    let fixture = CollisionFixture::new(&[(10, None, prefixed)]);
    let mut store = fixture.open(false);
    let mut e = Extractor::new(&mut store, &Lookup, Limits::default()).unwrap();
    let handle = e.extract(request(10)).unwrap();
    assert_eq!(
        e.extract_gameobject_fdid(10),
        Ok(GameObjectOutcome::Texture)
    );
    assert_eq!(e.facts().reads, 1);
    assert_eq!(e.finish().unwrap().requests(), &[handle]);
}

#[test]
fn gameobject_cached_origin_and_class_aliases_fail_before_rereading() {
    let fixture = CollisionFixture::new(&[(10, Some(b"FILE0000000A.xxx".as_slice()), m2(true))]);
    let mut store = fixture.open(true);
    let mut e = Extractor::new(&mut store, &Lookup, Limits::default()).unwrap();
    e.extract(ModelRequest::M2(InputRef::LogicalName(b"FILE0000000A.xxx")))
        .unwrap();
    assert_eq!(e.extract_gameobject_fdid(10), Err(Error::Alias));
    assert_eq!(e.facts().reads, 1);

    let mut source = b"REVM".to_vec();
    source.extend(m2(true));
    let fixture = CollisionFixture::new(&[(10, None, source)]);
    let mut store = fixture.open(false);
    let mut e = Extractor::new(&mut store, &Lookup, Limits::default()).unwrap();
    e.extract(request(10)).unwrap();
    assert_eq!(e.extract_gameobject_fdid(10), Err(Error::Alias));
    assert_eq!(e.facts().reads, 1);
}

#[test]
fn gameobject_cached_call_rechecks_input_guard_before_returning() {
    let fixture = CollisionFixture::new(&[(10, None, m2(true))]);
    let mut store = fixture.open(false);
    let mut e = Extractor::new(&mut store, &Lookup, Limits::default()).unwrap();
    e.extract_gameobject_fdid(10).unwrap();
    fixture.change_metadata();
    let error = e.extract_gameobject_fdid(10).unwrap_err();
    assert!(matches!(
        error,
        Error::Store(local_store::Error::InputChanged)
    ));
    assert_eq!(e.facts().reads, 1);
    assert_eq!(e.finish().err(), Some(error));
}

struct VisualLookup;
impl LiquidLookup for VisualLookup {
    fn material_id(&self, type_id: u32) -> Option<u8> {
        (type_id == 13).then_some(7)
    }
    fn material_flags(&self, material_id: u8) -> Option<u32> {
        (material_id == 7).then_some(2)
    }
}
fn raw_liquid_flags(model: &Model) -> u32 {
    raw::RawWmo::parse(model.raw().unwrap(), raw::ConversionLimits::default())
        .unwrap()
        .groups()
        .next()
        .unwrap()
        .liquid_flags()
}

#[test]
fn lookup_transition_preserves_cached_raw_handles_and_changes_only_new_models() {
    let mut liquid_group = group(true, false, &[]);
    put(&mut liquid_group, 8 + 52, 1);
    let fixture = CollisionFixture::new(&[
        (10, None, startup_root(&[30], &[], false)),
        (20, None, startup_root(&[40], &[], false)),
        (30, None, liquid_group.clone()),
        (40, None, liquid_group),
    ]);
    let mut store = fixture.open(false);
    let mut e = Extractor::new(&mut store, &Lookup, Limits::default()).unwrap();
    let old = gameobject_model(e.extract_gameobject_fdid(10).unwrap());
    let old_bytes = e.model(old).unwrap().raw().unwrap().to_vec();
    assert_eq!(raw_liquid_flags(e.model(old).unwrap()), 2);
    let old_live = e.retained_bytes();
    let mut e = e.with_lookup(&VisualLookup).unwrap();
    assert_eq!(e.retained_bytes(), old_live);
    assert_eq!(e.request_calls(), 1);
    assert_eq!(
        gameobject_model(e.extract_gameobject_fdid(10).unwrap()),
        old
    );
    assert_eq!(e.model(old).unwrap().raw(), Some(old_bytes.as_slice()));
    assert_eq!(e.facts().reads, 2);
    let new = gameobject_model(e.extract_gameobject_fdid(20).unwrap());
    assert_eq!(raw_liquid_flags(e.model(new).unwrap()), 0);
    assert_eq!(raw_liquid_flags(e.model(old).unwrap()), 2);
    assert_eq!(e.facts().reads, 4);
    assert_eq!(e.finish().unwrap().requests(), &[old, old, new]);
}

#[test]
fn lookup_transition_preserves_request_and_read_budgets() {
    let fixture = CollisionFixture::new(&[(10, None, m2(true)), (20, None, m2(false))]);
    let mut store = fixture.open(false);
    let mut e = Extractor::new(
        &mut store,
        &Lookup,
        Limits {
            max_requests: 1,
            ..Limits::default()
        },
    )
    .unwrap();
    e.extract(request(10)).unwrap();
    let mut e = e.with_lookup(&VisualLookup).unwrap();
    assert_eq!(e.extract(request(10)), Err(Error::Limit));
    assert_eq!(e.facts().reads, 1);
    drop(e);
    let mut e = Extractor::new(
        &mut store,
        &Lookup,
        Limits {
            max_reads: 1,
            ..Limits::default()
        },
    )
    .unwrap();
    e.extract(request(10)).unwrap();
    let mut e = e.with_lookup(&VisualLookup).unwrap();
    assert_eq!(e.extract_gameobject_fdid(20), Err(Error::Limit));
    assert_eq!(e.facts().reads, 1);
}

#[test]
fn lookup_transition_refuses_poison_and_changed_store() {
    let fixture = CollisionFixture::new(&[(10, None, b"????".to_vec())]);
    let mut store = fixture.open(false);
    let mut e = Extractor::new(&mut store, &Lookup, Limits::default()).unwrap();
    e.extract_gameobject_fdid(10).unwrap_err();
    assert_eq!(
        e.with_lookup(&VisualLookup).err(),
        Some(Error::GameObjectMagic)
    );
    let e = Extractor::new(&mut store, &Lookup, Limits::default()).unwrap();
    fixture.change_metadata();
    assert_eq!(
        e.with_lookup(&VisualLookup).err(),
        Some(Error::Store(local_store::Error::InputChanged))
    );
}

#[test]
fn startup_reservations_and_complete_source_remain_charged_across_lookup() {
    let fixture = CollisionFixture::new(&[(10, None, b"WDC5synthetic".to_vec())]);
    let mut store = fixture.open(false);
    let mut e = Extractor::new(&mut store, &Lookup, Limits::default()).unwrap();
    e.reserve_external(123).unwrap();
    let (source, facts) = e.read_source(10, 4096).unwrap();
    assert_eq!(facts.first_magic(), Some(*b"WDC5"));
    assert_eq!(e.facts().reads, 1);
    assert_eq!(e.facts().source_bytes, source.len());
    assert_eq!(e.request_calls(), 0);
    assert_eq!(e.retained_bytes(), 123 + source.capacity());
    let free = e.free_bytes().unwrap();
    let mut e = e.with_lookup(&VisualLookup).unwrap();
    assert_eq!(e.free_bytes().unwrap(), free);
    e.release_source(source.capacity()).unwrap();
    drop(source);
    e.release_external(123).unwrap();
    assert_eq!(e.retained_bytes(), 0);
    e.finish().unwrap();
}

#[test]
fn startup_reservation_failures_poison_and_cannot_be_released_twice() {
    let fixture = CollisionFixture::new(&[(10, None, m2(true))]);
    let mut store = fixture.open(false);
    let resident = store.stats().resident_bytes;
    let mut e = Extractor::new(
        &mut store,
        &Lookup,
        Limits {
            max_memory_bytes: resident + 32,
            ..Limits::default()
        },
    )
    .unwrap();
    e.reserve_external(32).unwrap();
    assert_eq!(e.reserve_external(1), Err(Error::Limit));
    assert_eq!(e.release_external(32), Err(Error::Limit));
    assert_eq!(e.extract(request(10)), Err(Error::Limit));
    assert_eq!(e.facts().reads, 0);
    drop(e);
    let mut e = Extractor::new(&mut store, &Lookup, Limits::default()).unwrap();
    e.reserve_external(1).unwrap();
    e.release_external(1).unwrap();
    assert_eq!(e.release_external(1), Err(Error::State));
    assert_eq!(e.finish().err(), Some(Error::State));
}

#[test]
fn startup_source_release_is_separate_from_external_and_finish_requires_transfer() {
    let fixture = CollisionFixture::new(&[(10, None, m2(true))]);
    let mut store = fixture.open(false);
    let mut e = Extractor::new(&mut store, &Lookup, Limits::default()).unwrap();
    e.reserve_external(1).unwrap();
    assert_eq!(e.release_source(1), Err(Error::State));
    drop(e);
    let mut e = Extractor::new(&mut store, &Lookup, Limits::default()).unwrap();
    e.reserve_external(1).unwrap();
    assert_eq!(e.finish().err(), Some(Error::State));
    let mut e = Extractor::new(&mut store, &Lookup, Limits::default()).unwrap();
    let (_source, _) = e.read_source(10, 4096).unwrap();
    assert_eq!(e.finish().err(), Some(Error::State));
}

#[test]
fn startup_source_caps_and_guard_failures_are_terminal() {
    let fixture = CollisionFixture::new(&[(10, None, m2(true))]);
    let mut store = fixture.open(false);
    let mut e = Extractor::new(&mut store, &Lookup, Limits::default()).unwrap();
    let error = e.read_source(10, 4).err().unwrap();
    assert_eq!(e.read_source(10, 4096).err(), Some(error));
    assert_eq!(e.facts().reads, 1);
    drop(e);
    let mut e = Extractor::new(&mut store, &Lookup, Limits::default()).unwrap();
    fixture.change_metadata();
    assert_eq!(
        e.verify(),
        Err(Error::Store(local_store::Error::InputChanged))
    );
    assert_eq!(
        e.read_source(10, 4096).err(),
        Some(Error::Store(local_store::Error::InputChanged))
    );
    assert_eq!(e.facts().reads, 0);
}

#[test]
fn final_extraction_reports_owned_capacities_and_store_snapshot_separately() {
    let fixture = CollisionFixture::new(&[(10, None, m2(true))]);
    let mut store = fixture.open(false);
    let store_bytes = store.stats().resident_bytes;
    let mut e = Extractor::new(&mut store, &Lookup, Limits::default()).unwrap();
    e.extract(request(10)).unwrap();
    let before = e.retained_bytes();
    e.reserve_external(321).unwrap();
    assert_eq!(e.retained_bytes(), before + 321);
    e.release_external(321).unwrap();
    let extraction = e.finish().unwrap();
    assert_eq!(extraction.resident_bytes(), before);
    assert_eq!(extraction.store_resident_bytes(), store_bytes);
    assert!(before >= extraction.models()[0].raw().unwrap().len());
    assert!(
        before
            >= extraction.models.capacity() * size_of::<Model>()
                + extraction.requests.capacity() * size_of::<ModelHandle>()
    );
}
#[test]
fn memory_partition_is_downward_and_survives_lookup_transition() {
    let fixture = CollisionFixture::new(&[(10, None, m2(true))]);
    let mut store = fixture.open(false);
    let store_bytes = store.stats().resident_bytes;
    let mut e = Extractor::new(&mut store, &Lookup, Limits::default()).unwrap();
    e.extract(request(10)).unwrap();
    let resident = e.retained_bytes();
    let facts = e.facts();
    e.constrain_memory(store_bytes + resident).unwrap();
    assert_eq!(e.memory_limit(), store_bytes + resident);
    assert_eq!(e.facts(), facts);
    let e = e.with_lookup(&Lookup).unwrap();
    assert_eq!(e.memory_limit(), store_bytes + resident);
    let extraction = e.finish().unwrap();
    assert_eq!(extraction.resident_bytes(), resident);
}
#[test]
fn memory_partition_includes_live_external_and_source_buffers() {
    let fixture = CollisionFixture::new(&[(10, None, m2(true))]);
    let mut store = fixture.open(false);
    let store_bytes = store.stats().resident_bytes;
    let mut e = Extractor::new(&mut store, &Lookup, Limits::default()).unwrap();
    e.reserve_external(32).unwrap();
    let (bytes, _) = e.read_source(10, 4096).unwrap();
    let ceiling = store_bytes + e.retained_bytes();
    e.constrain_memory(ceiling).unwrap();
    let old_limit = e.memory_limit();
    assert_eq!(e.constrain_memory(ceiling - 1), Err(Error::Limit));
    assert_eq!(e.memory_limit(), old_limit);
    assert_eq!(e.constrain_memory(old_limit), Err(Error::Limit));
    assert_eq!(e.finish().err(), Some(Error::Limit));
    drop(bytes);
}
#[test]
fn upward_partition_is_rejected_stickily_and_does_not_change_budget() {
    let fixture = CollisionFixture::new(&[(10, None, m2(true))]);
    let mut store = fixture.open(false);
    let mut e = Extractor::new(&mut store, &Lookup, Limits::default()).unwrap();
    let old = e.memory_limit();
    assert_eq!(e.constrain_memory(old + 1), Err(Error::InvalidLimits));
    assert_eq!(e.memory_limit(), old);
    assert_eq!(e.extract(request(10)), Err(Error::InvalidLimits));
    assert_eq!(e.facts().reads, 0);
    assert_eq!(e.finish().err(), Some(Error::InvalidLimits));
}
#[test]
fn partition_preserves_first_failure_and_checks_store_identity() {
    let fixture = CollisionFixture::new(&[(10, None, m2(true))]);
    let mut store = fixture.open(false);
    let mut e = Extractor::new(&mut store, &Lookup, Limits::default()).unwrap();
    assert_eq!(e.extract(request(0)), Err(Error::Name));
    assert_eq!(e.constrain_memory(0), Err(Error::Name));
    drop(e);
    let mut e = Extractor::new(&mut store, &Lookup, Limits::default()).unwrap();
    fixture.change_metadata();
    let old = e.memory_limit();
    assert_eq!(
        e.constrain_memory(old),
        Err(Error::Store(local_store::Error::InputChanged))
    );
    assert_eq!(e.memory_limit(), old);
    assert_eq!(
        e.finish().err(),
        Some(Error::Store(local_store::Error::InputChanged))
    );
}

#[cfg(target_os = "linux")]
#[path = "compatible_cases.rs"]
mod compatible_cases;
