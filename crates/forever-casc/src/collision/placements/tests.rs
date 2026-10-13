// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: private unit fixtures and complete synthetic LocalStores.
use super::super::{Facts, ModelHandle, NameBinding, WmoMetadata};
use super::*;
use crate::local_store::tests::{CollisionFile, CollisionFixture};
use forever_vmap::{objects, placement::DirBin};

fn put(bytes: &mut [u8], at: usize, value: u32) {
    bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
}
fn chunk(tag: &[u8; 4], body: &[u8]) -> Vec<u8> {
    let mut out: Vec<_> = tag.iter().rev().copied().collect();
    out.extend_from_slice(&(body.len() as u32).to_le_bytes());
    out.extend_from_slice(body);
    out
}
fn mddf(id: u32, unique: u32, fdid: bool) -> [u8; 36] {
    let mut b = [0; 36];
    put(&mut b, 0, id);
    put(&mut b, 4, unique);
    b[32..34].copy_from_slice(&1024u16.to_le_bytes());
    b[34..36].copy_from_slice(&(if fdid { 0x40u16 } else { 0 }).to_le_bytes());
    b
}
fn modf(id: u32, unique: u32, set: u16) -> [u8; 64] {
    let mut b = [0; 64];
    put(&mut b, 0, id);
    put(&mut b, 4, unique);
    b[56..58].copy_from_slice(&8u16.to_le_bytes());
    b[58..60].copy_from_slice(&set.to_le_bytes());
    b
}
fn model(class: Class, input: InputRef<'_>, status: Status) -> Model {
    let prepared = names::Prepared::new(input, class).unwrap();
    let origin = prepared
        .id
        .map_or_else(|| Origin::Name(prepared.canonical().to_vec()), Origin::Id);
    Model {
        class,
        name: prepared.name().to_vec(),
        origin,
        pending: false,
        status,
        raw: (status == Status::Collision).then(|| vec![0]),
        source: None,
        wmo: None,
    }
}
fn extraction(models: Vec<Model>, requests: &[usize]) -> Extraction {
    let count = models.len();
    Extraction {
        resident_bytes: 0,
        store_resident_bytes: 0,
        models,
        requests: requests.iter().map(|i| ModelHandle(*i)).collect(),
        facts: Facts {
            build: 70205,
            reads: 0,
            dependency_requests: 0,
            source_bytes: 0,
            raw_bytes: 0,
            models: count,
        },
    }
}
fn m2(id: u32, status: Status) -> Model {
    model(Class::M2, InputRef::FileDataId(id), status)
}
fn parse(bytes: &[u8]) -> Objects<'_> {
    Objects::parse_adt(bytes, objects::Limits::default()).unwrap()
}
fn batch<'a>(objects: &'a Objects<'a>) -> Batch<'a, 'a> {
    Batch {
        objects,
        map_id: 9,
        original_map_id: 9,
    }
}
fn output_ids(output: &BatchOutput) -> Vec<u32> {
    DirBin::parse(output.bytes(), placement::Limits::default())
        .unwrap()
        .records()
        .map(|p| p.id())
        .collect()
}
fn wmo_fixture(status: Status, overlap: bool) -> Extraction {
    let mut h = [0; 64];
    put(&mut h, 4, 1);
    h[60] = 4;
    let mut root_bytes = chunk(b"MOHD", &h);
    root_bytes.extend(chunk(b"GFID", &30u32.to_le_bytes()));
    root_bytes.extend(chunk(b"MODI", &20u32.to_le_bytes()));
    let mut spawns = vec![0; 80];
    for at in [0, 40] {
        put(&mut spawns, at + 28, 1f32.to_bits());
        put(&mut spawns, at + 32, 1f32.to_bits());
    }
    root_bytes.extend(chunk(b"MODD", &spawns));
    let mut sets = vec![0; if overlap { 64 } else { 32 }];
    put(&mut sets, 24, 2);
    if overlap {
        put(&mut sets, 52, 1);
        put(&mut sets, 56, 1);
    }
    root_bytes.extend(chunk(b"MODS", &sets));
    let mut group = vec![0; 68];
    group.extend(chunk(b"MODR", &[1, 0, 0, 0]));
    let group_bytes = chunk(b"MOGP", &group);
    let root = wmo::Root::parse(&root_bytes, wmo::Limits::default()).unwrap();
    let group = wmo::Group::parse(&group_bytes, wmo::Limits::default()).unwrap();
    let selection = wmo::doodads::resolve(
        &root,
        &[wmo::GroupInput {
            file_data_id: 30,
            group: &group,
        }],
        &[0],
        wmo::Limits::default(),
    )
    .unwrap();
    let mut parent = model(Class::Wmo, InputRef::FileDataId(10), status);
    parent.wmo = Some(WmoMetadata {
        root_bytes,
        bindings: vec![NameBinding {
            name_index: 0,
            model: ModelHandle(1),
        }],
        selection,
        groups: vec![],
        limits: wmo::Limits::default(),
    });
    extraction(vec![parent, m2(20, Status::Collision)], &[0])
}

#[test]
fn complete_m2_request_projects_with_explicit_serial_id() {
    let e = extraction(vec![m2(10, Status::Collision)], &[0]);
    let bytes = chunk(b"MDDF", &mddf(10, 7, true));
    let objects = parse(&bytes);
    let out = build(&e, &[batch(&objects)], Limits::default()).unwrap();
    assert_eq!(out.batches().len(), 1);
    assert_eq!(output_ids(&out.batches()[0]), [u32::MAX - 1]);
    assert_eq!(out.stats().ids.calls, 1);
    assert_eq!(out.stats().source_requests, 1);
    let p = DirBin::parse(out.batches()[0].bytes(), placement::Limits::default())
        .unwrap()
        .records()
        .next()
        .unwrap();
    assert_eq!(p.name(), b"FILE0000000A.xxx");
    assert_eq!(p.flags(), 0);
}

#[test]
fn session_distinct_calls_share_serial_allocator_and_snapshot_stats() {
    let e = extraction(vec![m2(10, Status::Collision)], &[0]);
    let first_bytes = chunk(b"MDDF", &mddf(10, 7, true));
    let first_objects = parse(&first_bytes);
    let second_bytes = chunk(b"MDDF", &mddf(10, 8, true));
    let second_objects = parse(&second_bytes);
    let mut session = Session::new(Limits::default()).unwrap();
    let first = session.project(&e, &[batch(&first_objects)]).unwrap();
    let second = session.project(&e, &[batch(&second_objects)]).unwrap();
    assert_eq!(output_ids(&first.batches()[0]), [u32::MAX - 1]);
    assert_eq!(output_ids(&second.batches()[0]), [u32::MAX - 2]);
    assert_eq!(first.stats().ids.calls, 1);
    assert_eq!(second.stats().ids.calls, 2);
    assert_eq!(second.stats().records, 1);
    assert_eq!(session.id_stats(), second.stats().ids);
    assert_eq!(session.failed(), None);
}

#[test]
fn session_repeated_key_hits_still_consume_m2_counter_between_calls() {
    let e = extraction(vec![m2(10, Status::Collision)], &[0]);
    let bytes = chunk(b"MDDF", &mddf(10, 7, true));
    let objects = parse(&bytes);
    let other_bytes = chunk(b"MDDF", &mddf(10, 8, true));
    let other = parse(&other_bytes);
    let mut session = Session::new(Limits::default()).unwrap();
    let first = session.project(&e, &[batch(&objects)]).unwrap();
    let again = session.project(&e, &[batch(&objects)]).unwrap();
    let third = session.project(&e, &[batch(&other)]).unwrap();
    assert_eq!(first.batches()[0].bytes(), again.batches()[0].bytes());
    assert_eq!(output_ids(&third.batches()[0]), [u32::MAX - 3]);
    assert_eq!(session.id_stats().entries, 2);
    assert_eq!(session.id_stats().m2_counter_consumed, 3);
}

#[test]
fn session_no_collision_calls_do_not_assign_ids() {
    let none = extraction(vec![m2(10, Status::NoCollision)], &[0]);
    let positive = extraction(vec![m2(10, Status::Collision)], &[0]);
    let bytes = chunk(b"MDDF", &mddf(10, 7, true));
    let objects = parse(&bytes);
    let mut session = Session::new(Limits::default()).unwrap();
    for _ in 0..2 {
        let output = session.project(&none, &[batch(&objects)]).unwrap();
        assert_eq!(output.stats().records, 0);
        assert_eq!(output.stats().ids.calls, 0);
        assert!(output.batches()[0].bytes().is_empty());
    }
    let output = session.project(&positive, &[batch(&objects)]).unwrap();
    assert_eq!(output_ids(&output.batches()[0]), [u32::MAX - 1]);
}

#[test]
fn session_late_failure_discards_call_output_and_poison_blocks_all_further_work() {
    let valid = extraction(vec![m2(40, Status::Collision)], &[0]);
    let bytes = chunk(b"MDDF", &mddf(40, 7, true));
    let objects = parse(&bytes);
    let mut session = Session::new(Limits::default()).unwrap();
    let previous = session.project(&valid, &[batch(&objects)]).unwrap();
    let mut broken = wmo_fixture(Status::Collision, false);
    broken.models[0].wmo.as_mut().unwrap().root_bytes[0] = 0xff;
    broken.models.push(m2(40, Status::Collision));
    broken.requests = vec![ModelHandle(2), ModelHandle(0)];
    let later_bytes = chunk(b"MODF", &modf(10, 100, 0));
    let later = parse(&later_bytes);
    let error = session
        .project(&broken, &[batch(&objects), batch(&later)])
        .err()
        .unwrap();
    assert_eq!(session.failed(), Some(error));
    let stats = session.id_stats();
    assert_eq!(stats.calls, 2);
    assert_eq!(previous.stats().ids.calls, 1);
    assert_eq!(output_ids(&previous.batches()[0]), [u32::MAX - 1]);
    assert_eq!(
        session.project(&valid, &[batch(&objects)]).err(),
        Some(error)
    );
    assert_eq!(
        session.project(&extraction(vec![], &[]), &[]).err(),
        Some(error)
    );
    assert_eq!(session.id_stats(), stats);
}

#[test]
fn session_preflight_error_is_sticky_without_consuming_ids() {
    let valid = extraction(vec![m2(10, Status::Collision)], &[0]);
    let bytes = chunk(b"MDDF", &mddf(10, 7, true));
    let objects = parse(&bytes);
    let mut session = Session::new(Limits::default()).unwrap();
    assert_eq!(
        session.project(&valid, &[]).err(),
        Some(Error::RequestCount)
    );
    assert_eq!(
        session.project(&valid, &[batch(&objects)]).err(),
        Some(Error::RequestCount)
    );
    assert_eq!(session.id_stats().calls, 0);
}

#[test]
fn session_output_map_record_and_memory_limits_reset_for_each_call() {
    let e = extraction(vec![m2(10, Status::Collision)], &[0]);
    let bytes = chunk(b"MDDF", &mddf(10, 7, true));
    let objects = parse(&bytes);
    let mut limits = Limits {
        max_batches: 1,
        max_maps: 1,
        max_models: 1,
        max_source_requests: 1,
        max_records: 1,
        max_output_bytes: 54,
        ids: ids::Limits {
            max_memory_bytes: 4096,
            ..ids::Limits::default()
        },
        ..Limits::default()
    };
    limits.max_memory_bytes = build(&e, &[batch(&objects)], limits)
        .unwrap()
        .stats()
        .charged_peak_bytes;
    let mut session = Session::new(limits).unwrap();
    let first = session.project(&e, &[batch(&objects)]).unwrap();
    let second = session
        .project(
            &e,
            &[Batch {
                objects: &objects,
                map_id: 10,
                original_map_id: 9,
            }],
        )
        .unwrap();
    assert_eq!(first.stats().output_bytes, 54);
    assert_eq!(second.stats().output_bytes, 54);
    assert_eq!(first.stats().maps, 1);
    assert_eq!(second.stats().maps, 1);
    assert_eq!(second.stats().charged_peak_bytes, limits.max_memory_bytes);
    assert_eq!(second.stats().ids.calls, 2);
}

#[test]
fn session_id_entry_limit_is_lifetime_wide_and_error_poisoned() {
    let e = extraction(vec![m2(10, Status::Collision)], &[0]);
    let bytes = chunk(b"MDDF", &mddf(10, 7, true));
    let objects = parse(&bytes);
    let other_bytes = chunk(b"MDDF", &mddf(10, 8, true));
    let other = parse(&other_bytes);
    let mut session = Session::new(Limits {
        ids: ids::Limits {
            max_entries: 1,
            ..ids::Limits::default()
        },
        ..Limits::default()
    })
    .unwrap();
    assert!(session.project(&e, &[batch(&objects)]).is_ok());
    let error = Some(Error::Ids(ids::Error::Limit));
    assert_eq!(session.project(&e, &[batch(&other)]).err(), error);
    let stats = session.id_stats();
    assert_eq!(stats.entries, 1);
    assert_eq!(stats.calls, 2);
    assert_eq!(session.project(&e, &[batch(&objects)]).err(), error);
    assert_eq!(session.id_stats(), stats);
}

#[test]
fn session_zero_caps_support_repeated_empty_calls_without_hidden_allocations() {
    let e = extraction(vec![], &[]);
    let mut session = Session::new(Limits {
        max_batches: 0,
        max_maps: 0,
        max_models: 0,
        max_source_requests: 0,
        max_records: 0,
        max_output_bytes: 0,
        max_memory_bytes: 0,
        ids: ids::Limits {
            max_entries: 0,
            max_memory_bytes: 0,
            ..ids::Limits::default()
        },
    })
    .unwrap();
    for _ in 0..2 {
        let output = session.project(&e, &[]).unwrap();
        assert!(output.batches().is_empty());
        assert_eq!(output.stats().charged_peak_bytes, 0);
        assert_eq!(output.stats().ids.calls, 0);
    }
}

#[test]
fn build_is_byte_identical_to_one_new_session() {
    let e = wmo_fixture(Status::Collision, true);
    let bytes = chunk(b"MODF", &modf(10, 100, 1));
    let objects = parse(&bytes);
    let one = build(&e, &[batch(&objects)], Limits::default()).unwrap();
    let mut session = Session::new(Limits::default()).unwrap();
    let two = session.project(&e, &[batch(&objects)]).unwrap();
    assert_eq!(one.batches()[0].bytes(), two.batches()[0].bytes());
    assert_eq!(one.stats(), two.stats());
}
#[test]
fn no_collision_m2_skips_without_consuming_id() {
    let e = extraction(
        vec![m2(10, Status::NoCollision), m2(20, Status::Collision)],
        &[0, 1],
    );
    let mut rows = mddf(10, 7, true).to_vec();
    rows.extend(mddf(20, 8, true));
    let bytes = chunk(b"MDDF", &rows);
    let objects = parse(&bytes);
    let out = build(&e, &[batch(&objects)], Limits::default()).unwrap();
    assert_eq!(output_ids(&out.batches()[0]), [u32::MAX - 1]);
    assert_eq!(out.stats().ids.calls, 1);
    assert_eq!(out.stats().records, 1);
}
#[test]
fn repeated_placements_are_fresh_calls_and_share_first_key_value_across_batches() {
    let e = extraction(vec![m2(10, Status::Collision)], &[0, 0]);
    let bytes = chunk(b"MDDF", &mddf(10, 7, true));
    let objects = parse(&bytes);
    let out = build(
        &e,
        &[
            batch(&objects),
            Batch {
                objects: &objects,
                map_id: 11,
                original_map_id: 9,
            },
        ],
        Limits::default(),
    )
    .unwrap();
    assert_eq!(output_ids(&out.batches()[0]), output_ids(&out.batches()[1]));
    assert_eq!(out.stats().ids.m2_counter_consumed, 2);
    assert_eq!(out.stats().maps, 2);
    let second = &out.batches()[1];
    assert_eq!((second.map_id(), second.original_map_id()), (11, 9));
    assert_eq!(
        DirBin::parse(second.bytes(), placement::Limits::default())
            .unwrap()
            .records()
            .next()
            .unwrap()
            .flags(),
        2
    );
}
#[test]
fn wmo_direct_then_sorted_doodads_use_one_allocator() {
    let e = wmo_fixture(Status::Collision, false);
    let bytes = chunk(b"MODF", &modf(10, 100, 0));
    let objects = parse(&bytes);
    let out = build(&e, &[batch(&objects)], Limits::default()).unwrap();
    assert_eq!(
        output_ids(&out.batches()[0]),
        [100, u32::MAX - 1, u32::MAX - 2]
    );
    assert_eq!(out.stats().ids.calls, 3);
    assert_eq!(out.stats().ids.m2_counter_consumed, 2);
    let records: Vec<_> = DirBin::parse(out.batches()[0].bytes(), placement::Limits::default())
        .unwrap()
        .records()
        .collect();
    assert_eq!(records[0].flags(), 1);
    assert_eq!(records[1].flags(), 0);
    assert_eq!(records[1].name(), b"FILE00000014.xxx");
}
#[test]
fn doodad_only_wmo_expands_without_direct_wmo_assignment() {
    let e = wmo_fixture(Status::DoodadOnly, false);
    let bytes = chunk(b"MODF", &modf(10, 100, 0));
    let objects = parse(&bytes);
    let out = build(&e, &[batch(&objects)], Limits::default()).unwrap();
    assert_eq!(output_ids(&out.batches()[0]), [u32::MAX - 1, u32::MAX - 2]);
    assert_eq!(out.stats().ids.calls, 2);
}
#[test]
fn overlapping_sets_duplicate_occurrence_and_doodad_counter() {
    let e = wmo_fixture(Status::Collision, true);
    let bytes = chunk(b"MODF", &modf(10, 100, 1));
    let objects = parse(&bytes);
    let out = build(&e, &[batch(&objects)], Limits::default()).unwrap();
    assert_eq!(
        output_ids(&out.batches()[0]),
        [100, u32::MAX - 1, u32::MAX - 2, u32::MAX - 3]
    );
    assert_eq!(out.stats().ids.m2_counter_consumed, 3);
    let records: Vec<_> = DirBin::parse(out.batches()[0].bytes(), placement::Limits::default())
        .unwrap()
        .records()
        .collect();
    assert_eq!(records[2].position_bits(), records[3].position_bits());
}
#[test]
fn invalid_selected_set_still_emits_set_zero() {
    let e = wmo_fixture(Status::DoodadOnly, true);
    let bytes = chunk(b"MODF", &modf(10, 100, 65535));
    let objects = parse(&bytes);
    let out = build(&e, &[batch(&objects)], Limits::default()).unwrap();
    assert_eq!(out.stats().records, 2);
}
#[test]
fn wdt_global_context_applies_to_direct_and_child_placements() {
    let e = wmo_fixture(Status::Collision, false);
    let bytes = chunk(b"MODF", &modf(10, 100, 0));
    let objects = Objects::parse_wdt(&bytes, objects::Limits::default()).unwrap();
    let out = build(&e, &[batch(&objects)], Limits::default()).unwrap();
    for p in DirBin::parse(out.batches()[0].bytes(), placement::Limits::default())
        .unwrap()
        .records()
    {
        assert_eq!(p.position_bits(), [0x4685_5555, 0x4685_5555, 0]);
    }
}
#[test]
fn request_count_must_match_exactly_including_empty_batches() {
    let bytes = chunk(b"MDDF", &mddf(10, 7, true));
    let objects = parse(&bytes);
    for requests in [vec![], vec![0, 0]] {
        let e = extraction(vec![m2(10, Status::Collision)], &requests);
        assert_eq!(
            build(&e, &[batch(&objects)], Limits::default()).err(),
            Some(Error::RequestCount)
        );
    }
    let e = extraction(vec![m2(10, Status::Collision)], &[0]);
    assert_eq!(
        build(&e, &[], Limits::default()).err(),
        Some(Error::RequestCount)
    );
}
#[test]
fn matching_counts_do_not_hide_reordered_or_wrong_id_requests() {
    let mut rows = mddf(10, 7, true).to_vec();
    rows.extend(mddf(20, 8, true));
    let bytes = chunk(b"MDDF", &rows);
    let objects = parse(&bytes);
    for requests in [[1, 0], [0, 0]] {
        let e = extraction(
            vec![m2(10, Status::Collision), m2(20, Status::Collision)],
            &requests,
        );
        assert_eq!(
            build(&e, &[batch(&objects)], Limits::default()).err(),
            Some(Error::Identity)
        );
    }
}
#[test]
fn class_pending_handle_and_basename_mismatches_are_rejected() {
    let bytes = chunk(b"MDDF", &mddf(10, 7, true));
    let objects = parse(&bytes);
    for variant in 0..4 {
        let mut e = extraction(vec![m2(10, Status::Collision)], &[0]);
        match variant {
            0 => e.models[0].class = Class::Wmo,
            1 => e.models[0].pending = true,
            2 => e.requests[0] = ModelHandle(99),
            _ => e.models[0].name = b"wrong".to_vec(),
        };
        assert_eq!(
            build(&e, &[batch(&objects)], Limits::default()).err(),
            Some(Error::Identity)
        );
    }
}
#[test]
fn names_compare_canonical_origin_after_extension_rewrite_not_just_basename() {
    let mut bytes = chunk(b"MMDX", b"a/Foo.MDX\0");
    bytes.extend(chunk(b"MDDF", &mddf(0, 7, false)));
    let objects = parse(&bytes);
    let e = extraction(
        vec![model(
            Class::M2,
            InputRef::LogicalName(b"A\\Foo.M2"),
            Status::Collision,
        )],
        &[0],
    );
    assert!(build(&e, &[batch(&objects)], Limits::default()).is_ok());
    let e = extraction(
        vec![model(
            Class::M2,
            InputRef::LogicalName(b"B/Foo.M2"),
            Status::Collision,
        )],
        &[0],
    );
    assert_eq!(
        build(&e, &[batch(&objects)], Limits::default()).err(),
        Some(Error::Identity)
    );
}
#[test]
fn logical_file_basename_is_not_a_numeric_origin_alias() {
    let mut bytes = chunk(b"MMDX", b"FILE0000000A.xxx\0");
    bytes.extend(chunk(b"MDDF", &mddf(0, 7, false)));
    let objects = parse(&bytes);
    let e = extraction(vec![m2(10, Status::Collision)], &[0]);
    assert_eq!(
        build(&e, &[batch(&objects)], Limits::default()).err(),
        Some(Error::Identity)
    );
}
#[test]
fn malformed_completed_model_status_is_not_accepted() {
    let bytes = chunk(b"MDDF", &mddf(10, 7, true));
    let objects = parse(&bytes);
    let mut e = extraction(vec![m2(10, Status::Collision)], &[0]);
    e.models[0].raw = None;
    assert_eq!(
        build(&e, &[batch(&objects)], Limits::default()).err(),
        Some(Error::Model)
    );
}
#[test]
fn selected_bindings_must_exist_match_source_and_have_m2_collision() {
    let bytes = chunk(b"MODF", &modf(10, 100, 0));
    let objects = parse(&bytes);
    for variant in 0..4 {
        let mut e = wmo_fixture(Status::Collision, false);
        match variant {
            0 => e.models[0].wmo.as_mut().unwrap().bindings.clear(),
            1 => e.models[0].wmo.as_mut().unwrap().bindings[0].model = ModelHandle(99),
            2 => e.models[1].origin = Origin::Id(99),
            _ => {
                e.models[1].status = Status::NoCollision;
                e.models[1].raw = None;
            }
        }
        assert!(build(&e, &[batch(&objects)], Limits::default()).is_err());
    }
}
#[test]
fn late_projection_failure_discards_all_earlier_batch_outputs() {
    let mut e = wmo_fixture(Status::Collision, false);
    e.models[0].wmo.as_mut().unwrap().root_bytes[0] = 0xff;
    e.models.push(m2(40, Status::Collision));
    e.requests = vec![ModelHandle(2), ModelHandle(0)];
    let first = chunk(b"MDDF", &mddf(40, 7, true));
    let first = parse(&first);
    let second = chunk(b"MODF", &modf(10, 100, 0));
    let second = parse(&second);
    assert!(build(&e, &[batch(&first), batch(&second)], Limits::default()).is_err());
}
#[test]
fn allocator_numeric_collision_prevents_complete_output() {
    let mut e = wmo_fixture(Status::Collision, false);
    e.models.push(m2(40, Status::Collision));
    e.requests = vec![ModelHandle(0), ModelHandle(2)];
    let first = chunk(b"MODF", &modf(10, u32::MAX - 1, 0));
    let first = parse(&first);
    let second = chunk(b"MDDF", &mddf(40, 7, true));
    let second = parse(&second);
    assert_eq!(
        build(&e, &[batch(&first), batch(&second)], Limits::default()).err(),
        Some(Error::Ids(ids::Error::AssignedIdCollision))
    );
}
#[test]
fn input_record_byte_map_model_and_batch_caps_fail_closed() {
    let bytes = chunk(b"MDDF", &mddf(10, 7, true));
    let objects = parse(&bytes);
    let e = extraction(vec![m2(10, Status::Collision)], &[0]);
    for limits in [
        Limits {
            max_batches: 0,
            ..Limits::default()
        },
        Limits {
            max_maps: 0,
            ..Limits::default()
        },
        Limits {
            max_models: 0,
            ..Limits::default()
        },
        Limits {
            max_source_requests: 0,
            ..Limits::default()
        },
        Limits {
            max_records: 0,
            ..Limits::default()
        },
        Limits {
            max_output_bytes: 0,
            ..Limits::default()
        },
    ] {
        assert_eq!(
            build(&e, &[batch(&objects)], limits).err(),
            Some(Error::Limit)
        );
    }
}
#[test]
fn output_byte_cap_is_cumulative_across_separate_batches() {
    let bytes = chunk(b"MDDF", &mddf(10, 7, true));
    let objects = parse(&bytes);
    let e = extraction(vec![m2(10, Status::Collision)], &[0, 0]);
    let size = 38 + 16;
    assert!(
        build(
            &e,
            &[batch(&objects), batch(&objects)],
            Limits {
                max_output_bytes: size * 2,
                ..Limits::default()
            }
        )
        .is_ok()
    );
    assert_eq!(
        build(
            &e,
            &[batch(&objects), batch(&objects)],
            Limits {
                max_output_bytes: size * 2 - 1,
                ..Limits::default()
            }
        )
        .err(),
        Some(Error::Limit)
    );
}
#[test]
fn id_memory_ceiling_is_reserved_even_when_no_output_is_produced() {
    let e = extraction(vec![], &[]);
    assert_eq!(
        build(
            &e,
            &[],
            Limits {
                max_memory_bytes: 0,
                ..Limits::default()
            }
        )
        .err(),
        Some(Error::Limit)
    );
    let limits = Limits {
        max_memory_bytes: 0,
        ids: ids::Limits {
            max_memory_bytes: 0,
            ..ids::Limits::default()
        },
        ..Limits::default()
    };
    let out = build(&e, &[], limits).unwrap();
    assert!(out.batches().is_empty());
    assert_eq!(out.stats().charged_peak_bytes, 0);
}
#[test]
fn exact_own_memory_peak_succeeds_and_reduction_rejects() {
    let bytes = chunk(b"MDDF", &mddf(10, 7, true));
    let objects = parse(&bytes);
    let e = extraction(vec![m2(10, Status::Collision)], &[0]);
    let limits = Limits {
        ids: ids::Limits {
            max_memory_bytes: 4096,
            ..ids::Limits::default()
        },
        ..Limits::default()
    };
    let stats = build(&e, &[batch(&objects)], limits).unwrap().stats();
    assert!(stats.charged_peak_bytes > stats.resident_bytes);
    assert!(
        build(
            &e,
            &[batch(&objects)],
            Limits {
                max_memory_bytes: stats.charged_peak_bytes,
                ..limits
            }
        )
        .is_ok()
    );
    assert!(
        build(
            &e,
            &[batch(&objects)],
            Limits {
                max_memory_bytes: stats.charged_peak_bytes - 1,
                ..limits
            }
        )
        .is_err()
    );
}
#[test]
fn raised_bridge_and_id_limits_are_rejected() {
    let e = extraction(vec![], &[]);
    for limits in [
        Limits {
            max_batches: MAX_BATCHES + 1,
            ..Limits::default()
        },
        Limits {
            max_maps: MAX_MAPS + 1,
            ..Limits::default()
        },
        Limits {
            max_models: MAX_MODELS + 1,
            ..Limits::default()
        },
        Limits {
            max_source_requests: MAX_SOURCE_REQUESTS + 1,
            ..Limits::default()
        },
        Limits {
            max_records: MAX_RECORDS + 1,
            ..Limits::default()
        },
        Limits {
            max_output_bytes: MAX_OUTPUT_BYTES + 1,
            ..Limits::default()
        },
        Limits {
            max_memory_bytes: MAX_MEMORY_BYTES + 1,
            ..Limits::default()
        },
    ] {
        assert_eq!(build(&e, &[], limits).err(), Some(Error::InvalidLimits));
    }
    assert_eq!(
        build(
            &e,
            &[],
            Limits {
                ids: ids::Limits {
                    max_entries: ids::MAX_ENTRIES + 1,
                    ..ids::Limits::default()
                },
                ..Limits::default()
            }
        )
        .err(),
        Some(Error::Ids(ids::Error::InvalidLimits))
    );
}
#[test]
fn doodad_counter_starts_at_one_allows_65535_and_never_wraps() {
    let mut value = 0;
    assert_eq!(next_doodad(&mut value), Ok(1));
    value = 65534;
    assert_eq!(next_doodad(&mut value), Ok(65535));
    assert_eq!(next_doodad(&mut value), Err(Error::DoodadCount));
    assert_eq!(value, 65535);
}
#[test]
fn empty_output_batch_has_no_hidden_id_calls_and_uses_no_record_budget() {
    let e = extraction(vec![m2(10, Status::NoCollision)], &[0]);
    let bytes = chunk(b"MDDF", &mddf(10, 7, true));
    let objects = parse(&bytes);
    let out = build(
        &e,
        &[batch(&objects)],
        Limits {
            max_records: 0,
            max_output_bytes: 0,
            ..Limits::default()
        },
    )
    .unwrap();
    assert!(out.batches()[0].bytes().is_empty());
    assert_eq!(out.batches()[0].record_count(), 0);
    assert_eq!(out.stats().ids.calls, 0);
}
#[test]
fn distinct_map_limit_counts_maps_not_batches_or_original_map_context() {
    let e = extraction(vec![m2(10, Status::Collision)], &[0, 0]);
    let bytes = chunk(b"MDDF", &mddf(10, 7, true));
    let objects = parse(&bytes);
    let limits = Limits {
        max_maps: 1,
        ..Limits::default()
    };
    assert!(
        build(
            &e,
            &[
                batch(&objects),
                Batch {
                    objects: &objects,
                    map_id: 9,
                    original_map_id: 5
                }
            ],
            limits
        )
        .is_ok()
    );
    assert_eq!(
        build(
            &e,
            &[
                batch(&objects),
                Batch {
                    objects: &objects,
                    map_id: 10,
                    original_map_id: 9
                }
            ],
            limits
        )
        .err(),
        Some(Error::Limit)
    );
}

// These integration cases create full synthetic on-disk CASC stores and run
// the public acquisition/conversion APIs. They do not fabricate completed raw
// output or placement selection metadata.
struct SyntheticLookup;
impl wmo::liquid::LiquidLookup for SyntheticLookup {
    fn material_id(&self, _: u32) -> Option<u8> {
        None
    }
    fn material_flags(&self, _: u8) -> Option<u32> {
        None
    }
}
fn source_m2(faces: bool) -> Vec<u8> {
    let mut bytes = vec![0; 304];
    bytes[..4].copy_from_slice(b"MD20");
    if faces {
        put(&mut bytes, 216, 3);
        put(&mut bytes, 220, 304);
        put(&mut bytes, 224, 3);
        put(&mut bytes, 228, 310);
        bytes.extend_from_slice(&[0, 0, 1, 0, 2, 0]);
        for word in [0u32, 0, 0, 1f32.to_bits(), 0, 0, 0, 1f32.to_bits(), 0] {
            bytes.extend_from_slice(&word.to_le_bytes());
        }
    }
    bytes
}
fn source_wmo_files(faces: bool) -> Vec<CollisionFile<'static>> {
    let mut header = [0; 64];
    put(&mut header, 4, 1);
    put(&mut header, 32, 777);
    header[60] = 4;
    let mut root = chunk(b"MOHD", &header);
    root.extend(chunk(b"GFID", &30u32.to_le_bytes()));
    root.extend(chunk(b"MODI", &20u32.to_le_bytes()));
    let mut spawn = [0; 40];
    put(&mut spawn, 28, 1f32.to_bits());
    put(&mut spawn, 32, 1f32.to_bits());
    root.extend(chunk(b"MODD", &spawn));
    let mut set = [0; 32];
    put(&mut set, 24, 1);
    root.extend(chunk(b"MODS", &set));
    let mut group = vec![0; 68];
    put(&mut group, 56, 99);
    group.extend(chunk(b"MODR", &[0, 0]));
    if faces {
        group.extend(chunk(b"MOPY", &[8, 0]));
        group.extend(chunk(b"MOVI", &[0, 0, 1, 0, 2, 0]));
        let mut vertices = Vec::new();
        for word in [0u32, 0, 0, 1f32.to_bits(), 0, 0, 0, 1f32.to_bits(), 0] {
            vertices.extend_from_slice(&word.to_le_bytes());
        }
        group.extend(chunk(b"MOVT", &vertices));
    }
    vec![
        (10, None, root),
        (20, None, source_m2(true)),
        (30, None, chunk(b"MOGP", &group)),
    ]
}
fn acquire_synthetic(fixture: &CollisionFixture, objects: &Objects<'_>) -> Extraction {
    let mut store = fixture.open(false);
    let mut extractor = super::super::Extractor::new(
        &mut store,
        &SyntheticLookup,
        super::super::Limits::default(),
    )
    .unwrap();
    for request in objects.requests() {
        let input = match request.target() {
            Target::FileDataId(id) => InputRef::FileDataId(id),
            Target::Name(name) => InputRef::LogicalName(name),
        };
        let request = match request.source() {
            Source::M2(_) => super::super::ModelRequest::M2(input),
            Source::Wmo(_) => super::super::ModelRequest::Wmo(input),
        };
        extractor.extract(request).unwrap();
    }
    extractor.finish().unwrap()
}
#[test]
fn integrated_store_m2_collision_absence_and_repeated_requests_reach_dir_bin() {
    let fixture =
        CollisionFixture::new(&[(10, None, source_m2(true)), (20, None, source_m2(false))]);
    let mut records = mddf(10, 7, true).to_vec();
    records.extend(mddf(20, 8, true));
    records.extend(mddf(10, 7, true));
    let bytes = chunk(b"MDDF", &records);
    let objects = parse(&bytes);
    let extraction = acquire_synthetic(&fixture, &objects);
    assert_eq!(extraction.facts().reads, 2);
    assert_eq!(extraction.requests().len(), 3);
    assert_eq!(extraction.models()[1].status(), Status::NoCollision);
    let raw = extraction.models()[0].raw().unwrap();
    assert_eq!(
        forever_vmap::RawM2::parse(raw, forever_vmap::Limits::default())
            .unwrap()
            .triangle_count(),
        1
    );
    let output = build(&extraction, &[batch(&objects)], Limits::default()).unwrap();
    assert_eq!(
        output_ids(&output.batches()[0]),
        [u32::MAX - 1, u32::MAX - 1]
    );
    assert_eq!(output.stats().ids.m2_counter_consumed, 2);
    assert_eq!(output.stats().source_requests, 3);
    assert_eq!(output.stats().records, 2);
}
#[test]
fn integrated_store_wmo_group_and_child_conversion_reach_direct_and_doodad_records() {
    let fixture = CollisionFixture::new(&source_wmo_files(true));
    let bytes = chunk(b"MODF", &modf(10, 100, 0));
    let objects = parse(&bytes);
    let extraction = acquire_synthetic(&fixture, &objects);
    assert_eq!(extraction.facts().reads, 3);
    assert_eq!(extraction.models().len(), 2);
    assert_eq!(extraction.models()[0].status(), Status::Collision);
    let raw = extraction.models()[0].raw().unwrap();
    assert_eq!(
        wmo::raw::RawWmo::parse(raw, wmo::raw::ConversionLimits::default())
            .unwrap()
            .triangle_count(),
        1
    );
    let output = build(&extraction, &[batch(&objects)], Limits::default()).unwrap();
    assert_eq!(output_ids(&output.batches()[0]), [100, u32::MAX - 1]);
    let decoded = DirBin::parse(output.batches()[0].bytes(), placement::Limits::default()).unwrap();
    assert_eq!(decoded.record_count(), 2);
    assert_eq!(
        decoded.records().nth(1).unwrap().name(),
        b"FILE00000014.xxx"
    );
}
#[test]
fn integrated_store_wmo_without_own_geometry_still_projects_acquired_doodad() {
    let fixture = CollisionFixture::new(&source_wmo_files(false));
    let bytes = chunk(b"MODF", &modf(10, 100, 0));
    let objects = parse(&bytes);
    let extraction = acquire_synthetic(&fixture, &objects);
    assert_eq!(extraction.models()[0].status(), Status::DoodadOnly);
    assert!(extraction.models()[0].raw().is_none());
    assert!(extraction.models()[1].raw().is_some());
    let output = build(&extraction, &[batch(&objects)], Limits::default()).unwrap();
    assert_eq!(output_ids(&output.batches()[0]), [u32::MAX - 1]);
    assert_eq!(output.stats().ids.calls, 1);
    assert_eq!(
        DirBin::parse(output.batches()[0].bytes(), placement::Limits::default())
            .unwrap()
            .records()
            .next()
            .unwrap()
            .flags(),
        0
    );
}
#[test]
fn integrated_store_failure_cannot_finish_and_completed_requests_still_require_exact_identity() {
    let fixture = CollisionFixture::new(&[(10, None, source_m2(true))]);
    let bytes = chunk(b"MDDF", &mddf(10, 7, true));
    let objects = parse(&bytes);
    let completed = acquire_synthetic(&fixture, &objects);
    let extra_bytes = chunk(b"MDDF", &[mddf(10, 7, true), mddf(10, 8, true)].concat());
    let extra = parse(&extra_bytes);
    assert_eq!(
        build(&completed, &[batch(&extra)], Limits::default()).err(),
        Some(Error::RequestCount)
    );
    let wrong_bytes = chunk(b"MDDF", &mddf(11, 7, true));
    let wrong = parse(&wrong_bytes);
    assert_eq!(
        build(&completed, &[batch(&wrong)], Limits::default()).err(),
        Some(Error::Identity)
    );
    let mut store = fixture.open(false);
    fixture.truncate_archive();
    let mut extractor = super::super::Extractor::new(
        &mut store,
        &SyntheticLookup,
        super::super::Limits::default(),
    )
    .unwrap();
    let failure = extractor
        .extract(super::super::ModelRequest::M2(InputRef::FileDataId(10)))
        .unwrap_err();
    assert!(matches!(
        failure,
        super::super::Error::Store(crate::local_store::Error::NoLocalEncoding(_))
    ));
    assert_eq!(extractor.finish().err(), Some(failure));
}

#[test]
fn borrowed_window_excludes_prior_requests_without_changing_model_handles() {
    let e = extraction(
        vec![m2(20, Status::Collision), m2(10, Status::Collision)],
        &[0, 1, 0],
    );
    let bytes = chunk(b"MDDF", &mddf(10, 7, true));
    let objects = parse(&bytes);
    assert_eq!(
        build(&e, &[batch(&objects)], Limits::default()).err(),
        Some(Error::RequestCount)
    );
    let view = ExtractionView {
        models: &e.models,
        requests: &e.requests[1..2],
    };
    let mut session = Session::new(Limits::default()).unwrap();
    let output = session.project_view(view, &[batch(&objects)]).unwrap();
    assert_eq!(output_ids(&output.batches()[0]), [u32::MAX - 1]);
    assert_eq!(e.requests.len(), 3);
}
#[test]
fn borrowed_window_keeps_dependency_lookup_and_rejects_wrong_binding_stickily() {
    let mut e = wmo_fixture(Status::Collision, false);
    e.requests = vec![ModelHandle(1), ModelHandle(0), ModelHandle(1)];
    let bytes = chunk(b"MODF", &modf(10, 7, 0));
    let objects = parse(&bytes);
    let view = ExtractionView {
        models: &e.models,
        requests: &e.requests[1..2],
    };
    let output = Session::new(Limits::default())
        .unwrap()
        .project_view(view, &[batch(&objects)])
        .unwrap();
    assert_eq!(output.stats().records, 3);
    let wrong = ExtractionView {
        models: &e.models,
        requests: &e.requests[..1],
    };
    let mut session = Session::new(Limits::default()).unwrap();
    assert_eq!(
        session.project_view(wrong, &[batch(&objects)]).err(),
        Some(Error::Identity)
    );
    assert_eq!(
        session.project_view(view, &[batch(&objects)]).err(),
        Some(Error::Identity)
    );
}
