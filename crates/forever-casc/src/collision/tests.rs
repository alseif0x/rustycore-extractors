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
fn named_wmo_doodad_is_not_omitted_by_the_fdid_policy() {
    let fixture = CollisionFixture::new(&[
        (10, None, named_root(&[30], b"A/missing.m2\0")),
        (30, None, group(true, false, &[0])),
    ]);
    let mut store = fixture.open(true);
    let mut e = Extractor::new(&mut store, &Lookup, Limits::default()).unwrap();
    let mut missing_m2 = Vec::new();
    let mut missing_wmo_ids = Vec::new();
    let mut omissions = OmissionOptions {
        missing_model: run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
        missing_wmo: run::MissingWmoPolicy::Reject,
        missing_m2: &mut missing_m2,
        missing_m2_limit: placements::MAX_MODELS,
        missing_wmo_ids: &mut missing_wmo_ids,
        missing_wmo_limit: 0,
        empty_wmo_ids: &mut Vec::new(),
        empty_wmo_limit: 0,
        unselected_wmo_ids: &mut Vec::new(),
        unselected_wmo_limit: 0,
        material_filtered_wmo_ids: &mut Vec::new(),
        material_filtered_wmo_limit: 0,
    };
    let error = match e.extract_placement_with_omissions(wmo_request(10), &mut omissions) {
        Ok(_) => panic!("named doodad was omitted without an FDID proof"),
        Err(error) => error,
    };
    assert_eq!(error, Error::Store(local_store::Error::MissingFile));
    assert!(missing_m2.is_empty());
    assert_eq!(e.request_calls, 1);
    assert_eq!(e.dependency_requests, 1);
    assert_eq!(e.facts().reads, 2);
    assert_eq!(e.finish().err(), Some(error));
}
#[cfg(target_os = "linux")]
#[test]
fn material_filtered_list_respects_explicit_capacity_without_growth() {
    let fixture = CollisionFixture::new(&[
        (10, None, root(&[30], &[], true)),
        (30, None, {
            let mut bytes = vec![0; 68];
            put(&mut bytes, 56, 99);
            bytes.extend(chunk(b"MOPY", &[0x26, 0]));
            bytes.extend(chunk(b"MOVI", &[0, 0, 1, 0, 2, 0]));
            bytes.extend(chunk(
                b"MOVT",
                &words(&[0, 0, 0, 0x3f80_0000, 0, 0, 0, 0x3f80_0000, 0]),
            ));
            chunk(b"MOGP", &bytes)
        }),
    ]);
    let mut store = fixture.open(false);
    let mut extractor = Extractor::new(&mut store, &Lookup, Limits::default()).unwrap();
    let mut material_filtered = Vec::with_capacity(1);
    let mut missing_m2 = Vec::new();
    let mut missing_wmo = Vec::new();
    let mut empty_wmo = Vec::new();
    let mut unselected_wmo = Vec::new();
    let mut omissions = OmissionOptions {
        missing_model: run::MissingModelPolicy::Reject,
        missing_wmo: run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
        missing_m2: &mut missing_m2,
        missing_m2_limit: 0,
        missing_wmo_ids: &mut missing_wmo,
        missing_wmo_limit: 0,
        empty_wmo_ids: &mut empty_wmo,
        empty_wmo_limit: 0,
        unselected_wmo_ids: &mut unselected_wmo,
        unselected_wmo_limit: 0,
        material_filtered_wmo_ids: &mut material_filtered,
        material_filtered_wmo_limit: 1,
    };
    extractor
        .extract_placement_with_omissions(wmo_request(10), &mut omissions)
        .unwrap();
    assert_eq!(material_filtered, [10]);
    assert_eq!(material_filtered.capacity(), 1);
    assert_eq!(
        extractor.finish().unwrap().models()[0].status(),
        Status::Vacated
    );

    let mut files = vec![
        (10, None, root(&[30], &[], true)),
        (11, None, root(&[31], &[], true)),
        (30, None, {
            let mut bytes = vec![0; 68];
            put(&mut bytes, 56, 99);
            bytes.extend(chunk(b"MOPY", &[0x26, 0]));
            bytes.extend(chunk(b"MOVI", &[0, 0, 1, 0, 2, 0]));
            bytes.extend(chunk(
                b"MOVT",
                &words(&[0, 0, 0, 0x3f80_0000, 0, 0, 0, 0x3f80_0000, 0]),
            ));
            chunk(b"MOGP", &bytes)
        }),
        (31, None, {
            let mut bytes = vec![0; 68];
            put(&mut bytes, 56, 99);
            bytes.extend(chunk(b"MOPY", &[0x26, 0]));
            bytes.extend(chunk(b"MOVI", &[0, 0, 1, 0, 2, 0]));
            bytes.extend(chunk(
                b"MOVT",
                &words(&[0, 0, 0, 0x3f80_0000, 0, 0, 0, 0x3f80_0000, 0]),
            ));
            chunk(b"MOGP", &bytes)
        }),
    ];
    let fixture = CollisionFixture::new(&files);
    let mut store = fixture.open(false);
    let mut extractor = Extractor::new(&mut store, &Lookup, Limits::default()).unwrap();
    let mut material_filtered = Vec::with_capacity(1);
    for id in [10, 11] {
        let mut omissions = OmissionOptions {
            missing_model: run::MissingModelPolicy::Reject,
            missing_wmo: run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmptyOrUnselectedOrMaterialFiltered,
            missing_m2: &mut Vec::new(),
            missing_m2_limit: 0,
            missing_wmo_ids: &mut Vec::new(),
            missing_wmo_limit: 0,
            empty_wmo_ids: &mut Vec::new(),
            empty_wmo_limit: 0,
            unselected_wmo_ids: &mut Vec::new(),
            unselected_wmo_limit: 0,
            material_filtered_wmo_ids: &mut material_filtered,
            material_filtered_wmo_limit: 1,
        };
        let result = extractor.extract_placement_with_omissions(wmo_request(id), &mut omissions);
        if id == 10 {
            result.unwrap();
            assert_eq!(material_filtered, [10]);
            assert_eq!(material_filtered.capacity(), 1);
        } else {
            assert_eq!(result, Err(Error::Limit));
            assert_eq!(material_filtered, [10]);
            assert_eq!(material_filtered.capacity(), 1);
        }
    }
    assert_eq!(extractor.finish().err(), Some(Error::Limit));
    files.clear();
}

#[test]
fn direct_wmo_doodad_limit_is_enforced_before_third_push() {
    let fixture = CollisionFixture::new(&[
        (10, None, root(&[30], &[20], true)),
        (11, None, root(&[31], &[19], true)),
        (12, None, root(&[32], &[18], true)),
        (30, None, group(true, false, &[0])),
        (31, None, group(true, false, &[0])),
        (32, None, group(true, false, &[0])),
    ]);
    let mut store = fixture.open(false);
    let mut e = Extractor::new(&mut store, &Lookup, Limits::default()).unwrap();
    let mut missing_m2 = Vec::with_capacity(2);
    let mut missing_wmo_ids = Vec::new();
    for id in [10, 11] {
        let mut empty_wmo_ids = Vec::new();
        let mut omissions = OmissionOptions {
            missing_model: run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
            missing_wmo: run::MissingWmoPolicy::Reject,
            missing_m2: &mut missing_m2,
            missing_m2_limit: 2,
            missing_wmo_ids: &mut missing_wmo_ids,
            missing_wmo_limit: 0,
            empty_wmo_ids: &mut empty_wmo_ids,
            empty_wmo_limit: 0,
            unselected_wmo_ids: &mut Vec::new(),
            unselected_wmo_limit: 0,
            material_filtered_wmo_ids: &mut Vec::new(),
            material_filtered_wmo_limit: 0,
        };
        e.extract_placement_with_omissions(wmo_request(id), &mut omissions)
            .unwrap();
    }
    assert_eq!(missing_m2, [20, 19]);
    assert_eq!(missing_m2.capacity(), 2);

    let mut omissions = OmissionOptions {
        missing_model: run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
        missing_wmo: run::MissingWmoPolicy::Reject,
        missing_m2: &mut missing_m2,
        missing_m2_limit: 2,
        missing_wmo_ids: &mut missing_wmo_ids,
        missing_wmo_limit: 0,
        empty_wmo_ids: &mut Vec::new(),
        empty_wmo_limit: 0,
        unselected_wmo_ids: &mut Vec::new(),
        unselected_wmo_limit: 0,
        material_filtered_wmo_ids: &mut Vec::new(),
        material_filtered_wmo_limit: 0,
    };
    let error = match e.extract_placement_with_omissions(wmo_request(12), &mut omissions) {
        Ok(_) => panic!("third omission exceeded the explicit limit"),
        Err(error) => error,
    };
    assert_eq!(error, Error::Limit);
    assert_eq!(missing_m2, [20, 19]);
    assert_eq!(missing_m2.capacity(), 2);
    assert_eq!(e.dependency_requests, 3);
    assert_eq!(e.request_calls, 3);
    assert_eq!(e.finish().err(), Some(Error::Limit));
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
        GameObjectOutcome::EncryptedUnavailable => {
            panic!("synthetic model unexpectedly classified as encrypted-unavailable")
        }
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
fn wdt_root_absence_rejects_zero_before_any_source_read() {
    let fixture = CollisionFixture::new(&[]);
    let mut store = fixture.open(false);
    let mut e = Extractor::new(&mut store, &Lookup, Limits::default()).unwrap();
    let error = match e.read_wdt_with_root_absence(0, 4096) {
        Ok(_) => panic!("WDT0 became a declarable ROOT absence"),
        Err(error) => error,
    };
    assert_eq!(error, Error::Name);
    assert_eq!(e.facts().reads, 0);
    assert_eq!(e.finish().err(), Some(Error::Name));
}

#[test]
fn wdt_root_absence_returns_no_source_facts_and_root_present_is_sticky_fatal() {
    let fixture = CollisionFixture::new(&[]);
    let mut store = fixture.open(false);
    let mut e = Extractor::new(&mut store, &Lookup, Limits::default()).unwrap();
    match e.read_wdt_with_root_absence(999, 4096).unwrap() {
        WdtRead::RootAbsent(999) => {}
        _ => panic!("missing ROOT produced facts or a present WDT"),
    }
    assert_eq!(e.external_sources, 0);
    assert_eq!(e.retained_bytes(), 0);
    e.finish().unwrap();

    let files = [(10, None, m2(true))];
    let fixture = CollisionFixture::with_ineligible_root(&files, 70205, 10);
    let mut store = fixture.open(false);
    let mut e = Extractor::new(&mut store, &Lookup, Limits::default()).unwrap();
    let error = match e.read_wdt_with_root_absence(10, 4096) {
        Ok(_) => panic!("ROOT-present ineligible WDT was omitted"),
        Err(error) => error,
    };
    assert_eq!(error, Error::Store(local_store::Error::MissingFile));
    let reads = e.facts().reads;
    let second = match e.read_wdt_with_root_absence(10, 4096) {
        Ok(_) => panic!("sticky failure allowed another read"),
        Err(error) => error,
    };
    assert_eq!(second, error);
    assert_eq!(e.facts().reads, reads);
    assert_eq!(e.finish().err(), Some(error));
}

#[test]
fn wdt_root_absence_guard_change_is_fatal_and_sticky() {
    let fixture = CollisionFixture::new(&[]);
    let mut store = fixture.open(false);
    let mut e = Extractor::new(&mut store, &Lookup, Limits::default()).unwrap();
    fixture.change_metadata();
    let error = match e.read_wdt_with_root_absence(999, 4096) {
        Ok(_) => panic!("changed guards allowed a ROOT absence"),
        Err(error) => error,
    };
    assert_eq!(error, Error::Store(local_store::Error::InputChanged));
    let reads = e.facts().reads;
    let second = match e.read_wdt_with_root_absence(1000, 4096) {
        Ok(_) => panic!("sticky guard failure allowed another read"),
        Err(error) => error,
    };
    assert_eq!(second, error);
    assert_eq!(e.facts().reads, reads);
    assert_eq!(e.finish().err(), Some(error));
}

#[test]
fn product_diagnostic_observer_receives_the_first_typed_synthetic_source_failure() {
    use std::{cell::RefCell, rc::Rc};

    #[derive(Clone, Copy)]
    struct Observed {
        request: local_store::ProductDiagnosticRequest,
        cause: local_store::Error,
        guards_observed: bool,
    }
    struct Observer {
        rows: Rc<RefCell<Vec<Observed>>>,
        resources: Rc<RefCell<Vec<local_store::ProductDiagnosticResource>>>,
        phase: local_store::ProductDiagnosticPhase,
        ordinal: u32,
    }
    impl local_store::ProductDiagnosticObserver for Observer {
        fn set_phase(&mut self, phase: local_store::ProductDiagnosticPhase) {
            if self.phase != phase {
                self.phase = phase;
                self.ordinal = 0;
            }
        }

        fn begin_request(
            &mut self,
            request_kind: local_store::ProductDiagnosticRequestKind,
            file_data_id: Option<u32>,
        ) -> Option<local_store::ProductDiagnosticRequest> {
            let request = local_store::ProductDiagnosticRequest {
                phase: self.phase,
                request_kind,
                file_data_id,
                ordinal: self.ordinal,
            };
            self.ordinal = self.ordinal.saturating_add(1);
            Some(request)
        }
        fn tolerated_absence(&mut self, _: local_store::ProductDiagnosticRequest) {}
        fn resource_limit(&mut self, resource: local_store::ProductDiagnosticResource) {
            self.resources.borrow_mut().push(resource);
        }

        fn source_failure(
            &mut self,
            request: local_store::ProductDiagnosticRequest,
            cause: local_store::Error,
            guards_observed: bool,
        ) {
            let mut rows = self.rows.borrow_mut();
            if rows.is_empty() {
                rows.push(Observed {
                    request,
                    cause,
                    guards_observed,
                });
            }
        }

        fn finish(&mut self, _: Option<&'static str>, _: bool) {}
    }

    let fixture = CollisionFixture::new(&[(10, None, m2(true))]);
    let mut store = fixture.open(false);
    fixture.truncate_archive();
    let rows = Rc::new(RefCell::new(Vec::new()));
    let resources = Rc::new(RefCell::new(Vec::new()));
    store.set_product_diagnostic_observer(Some(Box::new(Observer {
        rows: rows.clone(),
        resources: resources.clone(),
        phase: local_store::ProductDiagnosticPhase::Startup,
        ordinal: 0,
    })));
    store.set_product_diagnostic_phase(local_store::ProductDiagnosticPhase::Startup);
    let mut extractor = Extractor::new(&mut store, &Lookup, Limits::default()).unwrap();
    let error = extractor.extract(request(10)).unwrap_err();
    let Error::Store(local_store::Error::NoLocalEncoding(failures)) = error else {
        panic!("expected typed synthetic encoding failure")
    };
    let observations = rows.borrow();
    assert_eq!(observations.len(), 1);
    assert_eq!(
        observations[0].request.phase,
        local_store::ProductDiagnosticPhase::Startup
    );
    assert_eq!(
        observations[0].request.request_kind,
        local_store::ProductDiagnosticRequestKind::Other
    );
    assert_eq!(observations[0].request.file_data_id, Some(10));
    assert_eq!(observations[0].request.ordinal, 0);
    assert_eq!(
        observations[0].cause,
        local_store::Error::NoLocalEncoding(failures)
    );
    assert!(!observations[0].guards_observed);
    assert_eq!(resources.borrow().len(), 1);
}

#[test]
fn abort_forwards_the_current_resource_snapshot_to_the_observer() {
    use std::{cell::RefCell, rc::Rc};

    struct Observer(Rc<RefCell<Vec<local_store::ProductDiagnosticResource>>>);
    impl local_store::ProductDiagnosticObserver for Observer {
        fn set_phase(&mut self, _: local_store::ProductDiagnosticPhase) {}
        fn begin_request(
            &mut self,
            _: local_store::ProductDiagnosticRequestKind,
            _: Option<u32>,
        ) -> Option<local_store::ProductDiagnosticRequest> {
            None
        }
        fn tolerated_absence(&mut self, _: local_store::ProductDiagnosticRequest) {}
        fn resource_limit(&mut self, resource: local_store::ProductDiagnosticResource) {
            self.0.borrow_mut().push(resource);
        }
        fn source_failure(
            &mut self,
            _: local_store::ProductDiagnosticRequest,
            _: local_store::Error,
            _: bool,
        ) {
        }
        fn finish(&mut self, _: Option<&'static str>, _: bool) {}
    }

    let fixture = CollisionFixture::new(&[(10, None, m2(true))]);
    let mut store = fixture.open(false);
    let store_resident_bytes = store.stats().resident_bytes;
    let resources = Rc::new(RefCell::new(Vec::new()));
    store.set_product_diagnostic_observer(Some(Box::new(Observer(resources.clone()))));
    let limits = Limits {
        max_requests: 1,
        ..Limits::default()
    };
    let mut extractor = Extractor::new(&mut store, &Lookup, limits).unwrap();
    extractor.extract(request(10)).unwrap();
    extractor.reserve_external(3).unwrap();
    let (source, _) = extractor.read_source(10, 4096).unwrap();
    let external_source_bytes = source.capacity();
    assert_eq!(extractor.extract(request(10)), Err(Error::Limit));
    let facts = extractor.facts();
    let live = extractor.retained_bytes();
    let memory_limit = extractor.memory_limit();
    let expected = local_store::ProductDiagnosticResource {
        request_calls: 1,
        reads: facts.reads,
        dependency_requests: facts.dependency_requests,
        source_bytes: facts.source_bytes,
        raw_bytes: facts.raw_bytes,
        models: facts.models,
        resident_bytes: live,
        external_bytes: 3,
        external_source_bytes,
        store_resident_bytes,
        memory_limit,
    };
    assert_eq!(*resources.borrow(), [expected]);
    assert_eq!(extractor.abort(), Error::Limit);
    assert_eq!(*resources.borrow(), [expected]);
    assert_eq!(extractor.facts().reads, facts.reads);
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
#[test]
fn request_heavy_rejects_disk_world_before_acquisition() {
    let fixture = CollisionFixture::new(&[(10, None, m2(true))]);
    let mut store = fixture.open(false);
    assert_eq!(
        Extractor::new_with_resources(
            &mut store,
            &Lookup,
            Limits::default(),
            resources::Profile::DiskWorldV1,
        )
        .err(),
        Some(Error::InvalidLimits)
    );
}

#[test]
fn historical_request_cap_still_rejects_request_4097() {
    let fixture = CollisionFixture::new(&[(10, None, m2(true))]);
    let mut store = fixture.open(false);
    #[cfg(target_os = "linux")]
    run::SharedLimits::default()
        .validate_resources(resources::Profile::RequestHeavyV1)
        .unwrap();
    let mut e = Extractor::new(&mut store, &Lookup, Limits::default()).unwrap();
    for _ in 0..4096 {
        e.extract(request(10)).unwrap();
    }
    assert_eq!(e.extract(request(10)), Err(Error::Limit));
    assert_eq!(e.facts().reads, 1);
    assert_eq!(e.request_calls, 4096);
    assert_eq!(e.finish().err(), Some(Error::Limit));
}

#[cfg(target_os = "linux")]
#[test]
fn request_heavy_reuses_one_source_beyond_4096_and_enforces_reduced_cap() {
    let fixture = CollisionFixture::new(&[(10, None, m2(true))]);
    let mut store = fixture.open(false);
    let mut e = Extractor::new_with_resources(
        &mut store,
        &Lookup,
        resources::Profile::RequestHeavyV1.collector_limits(),
        resources::Profile::RequestHeavyV1,
    )
    .unwrap();
    for _ in 0..4097 {
        e.extract(request(10)).unwrap();
    }
    assert_eq!(e.request_calls, 4097);
    assert_eq!(e.facts().reads, 1);
    assert_eq!(e.models.len(), 1);
    let extraction = e.finish().unwrap();
    assert_eq!(extraction.requests().len(), 4097);
    assert_eq!(extraction.models().len(), 1);
    assert_eq!(extraction.facts().reads, 1);

    let mut store = fixture.open(false);
    let mut e = Extractor::new_with_resources(
        &mut store,
        &Lookup,
        Limits {
            max_requests: 4097,
            ..Limits::default()
        },
        resources::Profile::RequestHeavyV1,
    )
    .unwrap();
    for _ in 0..4097 {
        e.extract(request(10)).unwrap();
    }
    assert_eq!(e.extract(request(10)), Err(Error::Limit));
    assert_eq!(e.facts().reads, 1);
    assert_eq!(e.request_calls, 4097);
    assert_eq!(e.finish().err(), Some(Error::Limit));
}

#[cfg(target_os = "linux")]
#[test]
fn dependency_heavy_constructor_accepts_only_its_declared_collector_capacity() {
    let fixture = CollisionFixture::new(&[(10, None, m2(true))]);
    let mut store = fixture.open(false);
    let profile = resources::Profile::DependencyHeavyV1;
    let mut extractor =
        Extractor::new_with_resources(&mut store, &Lookup, profile.collector_limits(), profile)
            .unwrap();
    extractor.extract(request(10)).unwrap();
    let completed = extractor.finish().unwrap();
    assert_eq!(completed.facts().reads, 1);
    assert_eq!(completed.requests().len(), 1);

    for limits in [
        Limits {
            max_requests: 16_000_001,
            ..Limits::default()
        },
        Limits {
            max_dependency_requests: 16_000_001,
            ..Limits::default()
        },
    ] {
        let mut store = fixture.open(false);
        assert_eq!(
            Extractor::new_with_resources(&mut store, &Lookup, limits, profile).err(),
            Some(Error::InvalidLimits)
        );
    }
}

#[cfg(target_os = "linux")]
#[test]
fn model_heavy_accepts_4097_distinct_model_identities_and_rejects_other_profiles() {
    const MODEL_COUNT: usize = 4097;
    let files: Vec<CollisionFile<'static>> = (0..MODEL_COUNT)
        .map(|index| (100 + index as u32, None, m2(false)))
        .collect();
    let fixture = CollisionFixture::new(&files);
    let profile = resources::Profile::ModelHeavyV1;
    let mut store = fixture.open(false);
    let mut extractor =
        Extractor::new_with_resources(&mut store, &Lookup, profile.collector_limits(), profile)
            .unwrap();
    for id in 100..100 + MODEL_COUNT as u32 {
        extractor.extract(request(id)).unwrap();
    }
    assert_eq!(extractor.facts().models, MODEL_COUNT);
    assert_eq!(extractor.facts().reads, MODEL_COUNT);
    let extraction = extractor.finish().unwrap();
    assert_eq!(extraction.models().len(), MODEL_COUNT);

    let mut limits = resources::Profile::ReadHeavyV1.collector_limits();
    limits.max_models = MODEL_COUNT;
    assert_eq!(
        limits.validate_resources(resources::Profile::ReadHeavyV1),
        Err(Error::InvalidLimits)
    );
    assert_eq!(
        Limits {
            max_models: MODEL_COUNT,
            ..Limits::default()
        }
        .validate(),
        Err(Error::InvalidLimits)
    );
    let mut excessive = profile.collector_limits();
    excessive.max_models = 65_537;
    assert_eq!(
        excessive.validate_resources(profile),
        Err(Error::InvalidLimits)
    );
}

#[cfg(target_os = "linux")]
#[test]
fn dependency_heavy_source_budget_stays_charged_after_release() {
    let fixture = CollisionFixture::new(&[(10, None, vec![b'x'; 8]), (20, None, vec![b'y'; 8])]);
    let profile = resources::Profile::DependencyHeavyV1;
    let mut limits = profile.collector_limits();
    limits.max_source_bytes = 8;
    let mut store = fixture.open(false);
    let mut extractor =
        Extractor::new_with_resources(&mut store, &Lookup, limits, profile).unwrap();

    let (source, _) = extractor.read_source(10, 8).unwrap();
    assert_eq!(extractor.facts().source_bytes, 8);
    extractor.release_source(source.capacity()).unwrap();
    drop(source);

    assert_eq!(extractor.read_source(20, 8).err(), Some(Error::Limit));
    assert_eq!(extractor.facts().source_bytes, 8);
    assert_eq!(extractor.finish().err(), Some(Error::Limit));
}

#[cfg(target_os = "linux")]
#[test]
fn source_expanded_accepts_model_capacity_and_expanded_source_cap() {
    let fixture = CollisionFixture::new(&[(10, None, m2(true))]);
    let profile = resources::Profile::SourceExpandedV1;
    let mut limits = profile.collector_limits();
    limits.max_source_bytes = 1024 * 1024 * 1024;
    limits.max_models = 65_536;
    let mut store = fixture.open(false);
    let mut extractor =
        Extractor::new_with_resources(&mut store, &Lookup, limits, profile).unwrap();
    extractor.extract(request(10)).unwrap();
    let completed = extractor.finish().unwrap();
    assert_eq!(completed.facts().reads, 1);
    assert_eq!(completed.requests().len(), 1);

    for limits in [
        Limits {
            max_source_bytes: 1024 * 1024 * 1024 + 1,
            ..profile.collector_limits()
        },
        Limits {
            max_models: 65_537,
            ..profile.collector_limits()
        },
        Limits {
            max_reads: 1_000_001,
            ..profile.collector_limits()
        },
        Limits {
            max_requests: 16_000_001,
            ..profile.collector_limits()
        },
        Limits {
            max_dependency_requests: 16_000_001,
            ..profile.collector_limits()
        },
        Limits {
            max_raw_bytes: 256 * 1024 * 1024 + 1,
            ..profile.collector_limits()
        },
    ] {
        let mut store = fixture.open(false);
        assert_eq!(
            Extractor::new_with_resources(&mut store, &Lookup, limits, profile).err(),
            Some(Error::InvalidLimits)
        );
    }

    let mut downward = profile.collector_limits();
    downward.max_source_bytes = 128 * 1024 * 1024;
    downward.max_models = 4096;
    downward.max_reads = 4096;
    downward.max_requests = 4096;
    downward.max_dependency_requests = 4096;
    downward.max_raw_bytes = 128 * 1024 * 1024;
    let mut store = fixture.open(false);
    Extractor::new_with_resources(&mut store, &Lookup, downward, profile).unwrap();
}

#[cfg(target_os = "linux")]
#[test]
fn model_output_expanded_constructor_uses_source_expanded_casc_caps() {
    let fixture = CollisionFixture::new(&[(10, None, m2(true))]);
    let profile = resources::Profile::ModelOutputExpandedV1;
    let limits = profile.collector_limits();
    let mut store = fixture.open(false);
    let mut extractor =
        Extractor::new_with_resources(&mut store, &Lookup, limits, profile).unwrap();
    extractor.extract(request(10)).unwrap();
    let completed = extractor.finish().unwrap();
    assert_eq!(completed.facts().reads, 1);
    assert_eq!(completed.requests().len(), 1);

    let mut downward = limits;
    downward.max_source_bytes = 128 * 1024 * 1024;
    downward.max_models = 4096;
    downward.max_reads = 4096;
    downward.max_requests = 4096;
    downward.max_dependency_requests = 4096;
    downward.max_raw_bytes = 128 * 1024 * 1024;
    let mut store = fixture.open(false);
    Extractor::new_with_resources(&mut store, &Lookup, downward, profile).unwrap();
}

#[cfg(target_os = "linux")]
#[test]
fn source_expanded_reduced_source_budget_is_terminal_without_refund() {
    let fixture = CollisionFixture::new(&[(10, None, vec![b'x'; 8]), (20, None, vec![b'y'; 8])]);
    let profile = resources::Profile::SourceExpandedV1;
    let mut limits = profile.collector_limits();
    limits.max_source_bytes = 16;
    let mut store = fixture.open(false);
    let mut extractor =
        Extractor::new_with_resources(&mut store, &Lookup, limits, profile).unwrap();

    let (source, _) = extractor.read_source(10, 8).unwrap();
    assert_eq!(extractor.facts().source_bytes, 8);
    extractor.release_source(source.capacity()).unwrap();
    drop(source);

    let (source, _) = extractor.read_source(20, 8).unwrap();
    assert_eq!(extractor.facts().source_bytes, 16);
    extractor.release_source(source.capacity()).unwrap();
    drop(source);

    assert_eq!(extractor.read_source(10, 1).err(), Some(Error::Limit));
    assert_eq!(extractor.facts().source_bytes, 16);
    assert_eq!(extractor.finish().err(), Some(Error::Limit));
}

#[cfg(target_os = "linux")]
#[test]
fn source_expanded_v2_constructor_and_reduced_source_budget_accept_exact_total() {
    let fixture = CollisionFixture::new(&[(10, None, m2(true))]);
    let profile = resources::Profile::SourceExpandedV2;
    let mut store = fixture.open(false);
    let mut extractor =
        Extractor::new_with_resources(&mut store, &Lookup, profile.collector_limits(), profile)
            .unwrap();
    extractor.extract(request(10)).unwrap();
    let completed = extractor.finish().unwrap();
    assert_eq!(completed.facts().reads, 1);
    assert_eq!(completed.requests().len(), 1);

    let fixture = CollisionFixture::new(&[(10, None, vec![b'x'; 8]), (20, None, vec![b'y'; 8])]);
    let mut limits = profile.collector_limits();
    limits.max_source_bytes = 16;
    let mut store = fixture.open(false);
    let mut extractor =
        Extractor::new_with_resources(&mut store, &Lookup, limits, profile).unwrap();

    for (id, total) in [(10, 8), (20, 16)] {
        let (source, _) = extractor.read_source(id, 8).unwrap();
        assert_eq!(extractor.facts().source_bytes, total);
        extractor.release_source(source.capacity()).unwrap();
        drop(source);
    }
    let completed = extractor.finish().unwrap();
    assert_eq!(completed.facts().reads, 2);
    assert_eq!(completed.facts().source_bytes, 16);

    for limits in [
        Limits {
            max_source_bytes: 2 * 1024 * 1024 * 1024 + 1,
            ..profile.collector_limits()
        },
        Limits {
            max_models: 65_537,
            ..profile.collector_limits()
        },
        Limits {
            max_reads: 1_000_001,
            ..profile.collector_limits()
        },
        Limits {
            max_requests: 16_000_001,
            ..profile.collector_limits()
        },
        Limits {
            max_dependency_requests: 16_000_001,
            ..profile.collector_limits()
        },
        Limits {
            max_raw_bytes: 256 * 1024 * 1024 + 1,
            ..profile.collector_limits()
        },
    ] {
        let mut store = fixture.open(false);
        assert_eq!(
            Extractor::new_with_resources(&mut store, &Lookup, limits, profile).err(),
            Some(Error::InvalidLimits)
        );
    }
}

#[cfg(target_os = "linux")]
#[test]
fn source_expanded_v2_reduced_source_budget_rejects_first_excess_byte() {
    let fixture = CollisionFixture::new(&[
        (10, None, vec![b'x'; 8]),
        (20, None, vec![b'y'; 8]),
        (30, None, vec![b'z'; 1]),
    ]);
    let profile = resources::Profile::SourceExpandedV2;
    let mut limits = profile.collector_limits();
    limits.max_source_bytes = 16;
    let mut store = fixture.open(false);
    let mut extractor =
        Extractor::new_with_resources(&mut store, &Lookup, limits, profile).unwrap();

    for id in [10, 20] {
        let (source, _) = extractor.read_source(id, 8).unwrap();
        extractor.release_source(source.capacity()).unwrap();
        drop(source);
    }
    assert_eq!(extractor.facts().source_bytes, 16);
    assert_eq!(extractor.read_source(30, 1).err(), Some(Error::Limit));
    assert_eq!(extractor.facts().source_bytes, 16);
    assert_eq!(extractor.finish().err(), Some(Error::Limit));
}

#[cfg(target_os = "linux")]
#[test]
fn source_heavy_reduced_source_budget_accepts_exact_total() {
    let fixture = CollisionFixture::new(&[(10, None, vec![b'x'; 8]), (20, None, vec![b'y'; 8])]);
    let profile = resources::Profile::SourceHeavyV1;
    let mut limits = profile.collector_limits();
    limits.max_source_bytes = 16;
    let mut store = fixture.open(false);
    let mut extractor =
        Extractor::new_with_resources(&mut store, &Lookup, limits, profile).unwrap();

    for (id, total) in [(10, 8), (20, 16)] {
        let (source, _) = extractor.read_source(id, 8).unwrap();
        assert_eq!(extractor.facts().source_bytes, total);
        extractor.release_source(source.capacity()).unwrap();
        drop(source);
    }

    let completed = extractor.finish().unwrap();
    assert_eq!(completed.facts().reads, 2);
    assert_eq!(completed.facts().source_bytes, 16);
}

#[cfg(target_os = "linux")]
#[test]
fn source_heavy_reduced_source_budget_rejects_additional_byte() {
    let fixture = CollisionFixture::new(&[
        (10, None, vec![b'x'; 8]),
        (20, None, vec![b'y'; 8]),
        (30, None, vec![b'z'; 1]),
    ]);
    let profile = resources::Profile::SourceHeavyV1;
    let mut limits = profile.collector_limits();
    limits.max_source_bytes = 16;
    let mut store = fixture.open(false);
    let mut extractor =
        Extractor::new_with_resources(&mut store, &Lookup, limits, profile).unwrap();

    for id in [10, 20] {
        let (source, _) = extractor.read_source(id, 8).unwrap();
        extractor.release_source(source.capacity()).unwrap();
        drop(source);
    }
    assert_eq!(extractor.facts().source_bytes, 16);
    assert_eq!(extractor.read_source(30, 1).err(), Some(Error::Limit));
    assert_eq!(extractor.facts().source_bytes, 16);
    assert_eq!(extractor.finish().err(), Some(Error::Limit));
}

#[cfg(target_os = "linux")]
#[test]
fn source_heavy_constructor_accepts_only_its_declared_capacities() {
    let fixture = CollisionFixture::new(&[(10, None, m2(true))]);
    let profile = resources::Profile::SourceHeavyV1;
    let mut store = fixture.open(false);
    let mut extractor =
        Extractor::new_with_resources(&mut store, &Lookup, profile.collector_limits(), profile)
            .unwrap();
    extractor.extract(request(10)).unwrap();
    let completed = extractor.finish().unwrap();
    assert_eq!(completed.facts().reads, 1);
    assert_eq!(completed.requests().len(), 1);

    for limits in [
        Limits {
            max_source_bytes: 512 * 1024 * 1024 + 1,
            ..profile.collector_limits()
        },
        Limits {
            max_requests: 16_000_001,
            ..profile.collector_limits()
        },
        Limits {
            max_dependency_requests: 16_000_001,
            ..profile.collector_limits()
        },
        Limits {
            max_reads: 4097,
            ..profile.collector_limits()
        },
    ] {
        let mut store = fixture.open(false);
        assert_eq!(
            Extractor::new_with_resources(&mut store, &Lookup, limits, profile).err(),
            Some(Error::InvalidLimits)
        );
    }

    let mut downward = profile.collector_limits();
    downward.max_source_bytes = 128 * 1024 * 1024;
    downward.max_requests = 4096;
    downward.max_dependency_requests = 4096;
    downward.max_raw_bytes = 128 * 1024 * 1024;
    let mut store = fixture.open(false);
    Extractor::new_with_resources(&mut store, &Lookup, downward, profile).unwrap();
}

#[cfg(target_os = "linux")]
#[test]
fn read_heavy_constructor_accepts_only_its_declared_read_capacity() {
    let fixture = CollisionFixture::new(&[(10, None, m2(true))]);
    let profile = resources::Profile::ReadHeavyV1;
    let reads = 4097;
    let mut store = fixture.open(false);
    let mut extractor = Extractor::new_with_resources(
        &mut store,
        &Lookup,
        Limits {
            max_reads: reads,
            ..profile.collector_limits()
        },
        profile,
    )
    .unwrap();
    for _ in 0..reads {
        let (source, _) = extractor.read_source(10, 4096).unwrap();
        extractor.release_source(source.capacity()).unwrap();
        drop(source);
    }
    assert_eq!(extractor.facts().reads, reads);

    let mut terminal = Extractor::new_with_resources(
        &mut store,
        &Lookup,
        Limits {
            max_reads: reads,
            ..profile.collector_limits()
        },
        profile,
    )
    .unwrap();
    for _ in 0..reads {
        let (source, _) = terminal.read_source(10, 4096).unwrap();
        terminal.release_source(source.capacity()).unwrap();
        drop(source);
    }
    assert_eq!(terminal.read_source(10, 4096).err(), Some(Error::Limit));
    assert_eq!(terminal.facts().reads, reads);
    assert_eq!(terminal.finish().err(), Some(Error::Limit));

    let mut store = fixture.open(false);
    assert_eq!(
        Extractor::new_with_resources(
            &mut store,
            &Lookup,
            Limits {
                max_reads: 1_000_001,
                ..profile.collector_limits()
            },
            profile,
        )
        .err(),
        Some(Error::InvalidLimits)
    );

    let mut reduced = profile.collector_limits();
    reduced.max_reads = reads;
    reduced.max_source_bytes = 128 * 1024 * 1024;
    reduced.max_requests = 4096;
    reduced.max_dependency_requests = 4096;
    reduced.max_raw_bytes = 128 * 1024 * 1024;
    let mut store = fixture.open(false);
    Extractor::new_with_resources(&mut store, &Lookup, reduced, profile).unwrap();
}

#[cfg(target_os = "linux")]
#[test]
fn dependency_heavy_counts_repeated_wmo_doodads_beyond_request_heavy_cap() {
    const DOODAD_REQUESTS: usize = 4097;
    let doodads = vec![20; DOODAD_REQUESTS];
    let fixture = CollisionFixture::new(&[
        (10, None, root(&[30], &doodads, true)),
        (20, None, m2(true)),
        (30, None, group(true, false, &[0])),
    ]);

    let mut store = fixture.open(false);
    let mut dependency_limits = Limits {
        max_dependency_requests: DOODAD_REQUESTS,
        ..Limits::default()
    };
    dependency_limits.max_requests = 2;
    let mut extractor = Extractor::new_with_resources(
        &mut store,
        &Lookup,
        dependency_limits,
        resources::Profile::DependencyHeavyV1,
    )
    .unwrap();
    extractor.extract(wmo_request(10)).unwrap();
    assert_eq!(extractor.dependency_requests, DOODAD_REQUESTS);
    assert_eq!(extractor.request_calls(), 1);
    assert_eq!(extractor.reads, 3);
    assert_eq!(extractor.models.len(), 2);
    let completed = extractor.finish().unwrap();
    assert_eq!(completed.facts().dependency_requests, DOODAD_REQUESTS);
    assert_eq!(completed.facts().reads, 3);
    assert_eq!(completed.requests().len(), 1);
    assert_eq!(completed.models().len(), 2);
    let wmo = completed.model(completed.requests()[0]).unwrap();
    let bindings = wmo.wmo_metadata().unwrap().bindings();
    assert_eq!(bindings.len(), DOODAD_REQUESTS);
    let m2_handle = bindings[0].model;
    assert!(bindings.iter().all(|binding| binding.model == m2_handle));
    assert_eq!(completed.model(m2_handle).unwrap().class(), Class::M2);

    let mut store = fixture.open(false);
    let mut request_heavy_limits = Limits {
        max_dependency_requests: 4096,
        ..Limits::default()
    };
    request_heavy_limits.max_requests = 2;
    let mut extractor = Extractor::new_with_resources(
        &mut store,
        &Lookup,
        request_heavy_limits,
        resources::Profile::RequestHeavyV1,
    )
    .unwrap();
    assert_eq!(extractor.extract(wmo_request(10)), Err(Error::Limit));
    assert_eq!(extractor.dependency_requests, 4096);
    assert_eq!(extractor.request_calls(), 1);
    assert_eq!(extractor.reads, 2);
    assert_eq!(extractor.models.len(), 2);
    assert_eq!(extractor.finish().err(), Some(Error::Limit));

    let mut store = fixture.open(false);
    let mut reduced_limits = Limits {
        max_dependency_requests: 2048,
        ..Limits::default()
    };
    reduced_limits.max_requests = 2;
    let mut extractor = Extractor::new_with_resources(
        &mut store,
        &Lookup,
        reduced_limits,
        resources::Profile::DependencyHeavyV1,
    )
    .unwrap();
    assert_eq!(extractor.extract(wmo_request(10)), Err(Error::Limit));
    assert_eq!(extractor.dependency_requests, 2048);
    assert_eq!(extractor.request_calls(), 1);
    assert_eq!(extractor.reads, 2);
    assert_eq!(extractor.models.len(), 2);
    assert_eq!(extractor.finish().err(), Some(Error::Limit));
}

#[cfg(target_os = "linux")]
#[test]
fn request_heavy_request_growth_remains_memory_bounded() {
    let fixture = CollisionFixture::new(&[(10, None, m2(true))]);
    let mut store = fixture.open(false);
    let store_bytes = store.stats().resident_bytes;
    let mut e = Extractor::new_with_resources(
        &mut store,
        &Lookup,
        resources::Profile::RequestHeavyV1.collector_limits(),
        resources::Profile::RequestHeavyV1,
    )
    .unwrap();
    e.extract(request(10)).unwrap();
    let ceiling = store_bytes + e.retained_bytes();
    e.constrain_memory(ceiling).unwrap();
    assert_eq!(e.extract(request(10)), Err(Error::Limit));
    assert_eq!(e.facts().reads, 1);
    assert_eq!(e.finish().err(), Some(Error::Limit));
}

#[cfg(target_os = "linux")]
#[path = "compatible_cases.rs"]
mod compatible_cases;
