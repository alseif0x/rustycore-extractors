// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11. Synthetic metadata only, no acquisition or client data.
use super::*;

fn put(bytes: &mut [u8], at: usize, value: u32) {
    bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
}
fn frame(tag: &[u8; 4], payload: &[u8]) -> Vec<u8> {
    let mut bytes: Vec<_> = tag.iter().rev().copied().collect();
    bytes.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    bytes.extend_from_slice(payload);
    bytes
}
fn words(values: &[u32]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}
enum Names<'a> {
    Paths(&'a [u8]),
    Ids(&'a [u32]),
    Absent,
}
fn root_bytes(
    ids: &[u32],
    names: Names<'_>,
    spawns: &[u32],
    sets: &[(u32, u32)],
    group_names: &[u8],
) -> Vec<u8> {
    let mut h = [0; 64];
    put(&mut h, 4, ids.len() as u32);
    let mut bytes = frame(b"MOHD", &h);
    bytes.extend(frame(b"GFID", &words(ids)));
    bytes.extend(frame(b"MOGN", group_names));
    match names {
        Names::Paths(paths) => bytes.extend(frame(b"MODN", paths)),
        Names::Ids(ids) => bytes.extend(frame(b"MODI", &words(ids))),
        Names::Absent => {}
    }
    let mut spawn_bytes = vec![0; spawns.len() * 40];
    for (i, name) in spawns.iter().enumerate() {
        put(&mut spawn_bytes, i * 40, *name);
    }
    bytes.extend(frame(b"MODD", &spawn_bytes));
    let mut set_bytes = vec![0; sets.len() * 32];
    for (i, &(start, count)) in sets.iter().enumerate() {
        put(&mut set_bytes, i * 32 + 20, start);
        put(&mut set_bytes, i * 32 + 24, count);
    }
    bytes.extend(frame(b"MODS", &set_bytes));
    bytes
}
fn group_bytes(references: &[u16], flags: u32, name: u32) -> Vec<u8> {
    let mut h = vec![0; 68];
    put(&mut h, 0, name);
    put(&mut h, 8, flags);
    let refs: Vec<_> = references.iter().flat_map(|v| v.to_le_bytes()).collect();
    h.extend(frame(b"MODR", &refs));
    frame(b"MOGP", &h)
}
fn root(bytes: &[u8]) -> Root<'_> {
    Root::parse(bytes, Limits::default()).unwrap_or_else(|e| panic!("{e}"))
}
fn selected(
    rb: &[u8],
    group_bytes: &[Vec<u8>],
    valid: &[u32],
    limits: Limits,
) -> Result<Selection, Error> {
    let root = root(rb);
    let groups: Vec<_> = group_bytes
        .iter()
        .map(|bytes| Group::parse(bytes, Limits::default()).unwrap_or_else(|e| panic!("{e}")))
        .collect();
    let ids: Vec<_> = root.detailed_group_ids().collect();
    let inputs: Vec<_> = groups
        .iter()
        .enumerate()
        .map(|(i, group)| GroupInput {
            file_data_id: ids.get(i).copied().unwrap_or(999),
            group,
        })
        .collect();
    resolve(&root, &inputs, valid, limits)
}
fn must_select(rb: &[u8], groups: &[Vec<u8>], valid: &[u32]) -> Selection {
    selected(rb, groups, valid, Limits::default()).unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn modn_requests_all_physical_names_by_exact_offset_including_empty_and_unused() {
    let rb = root_bytes(&[], Names::Paths(b"a.m2\0\0bc.m2\0\xff\0"), &[6], &[], &[]);
    let root = root(&rb);
    let requested: Vec<_> = requests(&root)
        .map(|request| match request.target {
            Target::Name(name) => (request.name_index, name),
            Target::FileDataId(_) => panic!("unexpected ID target"),
        })
        .collect();
    assert_eq!(
        requested,
        [
            (0, b"a.m2".as_slice()),
            (5, b"".as_slice()),
            (6, b"bc.m2".as_slice()),
            (12, b"\xff".as_slice())
        ]
    );
    let mut exhausted = requests(&root);
    for _ in 0..4 {
        assert!(exhausted.next().is_some());
    }
    assert!(exhausted.next().is_none());
    assert!(exhausted.next().is_none());
}

#[test]
fn modi_requests_use_ordinals_omit_zero_and_preserve_aliases() {
    let rb = root_bytes(&[], Names::Ids(&[0, 9, 9, 0, u32::MAX]), &[], &[], &[]);
    let root = root(&rb);
    let requested: Vec<_> = requests(&root)
        .map(|request| match request.target {
            Target::FileDataId(id) => (request.name_index, id),
            Target::Name(_) => panic!("unexpected name target"),
        })
        .collect();
    assert_eq!(requested, [(1, 9), (2, 9), (4, u32::MAX)]);
}

#[test]
fn absent_and_empty_name_tables_produce_no_requests() {
    for names in [
        Names::Absent,
        Names::Paths(&[]),
        Names::Ids(&[]),
        Names::Ids(&[0, 0]),
    ] {
        let rb = root_bytes(&[], names, &[], &[], &[]);
        let root = root(&rb);
        assert_eq!(requests(&root).count(), 0);
    }
}

#[test]
fn accepted_name_indices_are_exact_requests_not_interior_or_terminator_offsets() {
    let rb = root_bytes(&[], Names::Paths(b"a.m2\0\0bc.m2\0\xff\0"), &[], &[], &[]);
    for unknown in [1, 4, 7, 11, 13, 14, u32::MAX] {
        assert_eq!(
            selected(&rb, &[], &[unknown], Limits::default()).err(),
            Some(Error::Name)
        );
    }
    // Caller order and duplicate successes have set semantics. Empty names
    // are structurally real requests; acquisition success is caller-owned.
    assert!(
        must_select(&rb, &[], &[12, 5, 6, 0, 6])
            .references()
            .is_empty()
    );
}

#[test]
fn zero_modi_slots_and_unknown_successes_are_rejected() {
    let rb = root_bytes(&[], Names::Ids(&[0, 9, 0]), &[], &[], &[]);
    for bad in [0, 2, 3, u32::MAX] {
        assert_eq!(
            selected(&rb, &[], &[bad], Limits::default()).err(),
            Some(Error::Name)
        );
    }
    assert!(must_select(&rb, &[], &[1, 1]).references().is_empty());
    let absent = root_bytes(&[], Names::Absent, &[], &[], &[]);
    assert_eq!(
        selected(&absent, &[], &[0], Limits::default()).err(),
        Some(Error::Name)
    );
}

#[test]
fn references_union_is_sorted_unique_and_uses_only_successful_names() {
    let rb = root_bytes(
        &[7, 7],
        Names::Ids(&[11, 12, 13]),
        &[0, 2, 1, 0],
        &[(0, 4)],
        &[],
    );
    let groups = [
        group_bytes(&[3, 1, 3, 2, 65535], 0, 0),
        group_bytes(&[2, 0, 1, 0, 4], 0, 0),
    ];
    let selection = must_select(&rb, &groups, &[1, 0, 1]);
    assert_eq!(selection.references(), [0, 2, 3]);
    assert_eq!(selection.set_count(), 1);
    assert_eq!(selection.outcome(false), Outcome::OkNoCollision);
    assert_eq!(selection.outcome(true), Outcome::Ok);
}

#[test]
fn m2_without_raw_collision_is_not_implicitly_an_accepted_doodad() {
    let rb = root_bytes(&[1], Names::Ids(&[123]), &[0], &[(0, 1)], &[]);
    let groups = [group_bytes(&[0], 0, 0)];
    let selection = must_select(&rb, &groups, &[]);
    assert!(selection.references().is_empty());
    assert_eq!(selection.outcome(false), Outcome::FailedNoCollision);
    assert_eq!(
        must_select(&rb, &groups, &[0]).outcome(false),
        Outcome::OkNoCollision
    );
}

#[test]
fn skipped_groups_do_not_contribute_even_when_their_references_are_valid() {
    let rb = root_bytes(
        &[1, 2, 3, 4],
        Names::Ids(&[11]),
        &[0, 0, 0, 0],
        &[(0, 4)],
        b"antiportal\0room\0",
    );
    let groups = [
        group_bytes(&[0], 0x80, 0),
        group_bytes(&[1], 0x0400_0000, 0),
        group_bytes(&[2], 0, 0),
        group_bytes(&[3], 0, 11),
    ];
    assert_eq!(must_select(&rb, &groups, &[0]).references(), [3]);
}

#[test]
fn unskipped_zero_geometry_groups_contribute_and_out_of_range_modr_is_ignored() {
    let rb = root_bytes(&[1], Names::Ids(&[11]), &[0], &[(0, 1)], &[]);
    let groups = [group_bytes(&[65535, 1, 0, 2], 0, 0)];
    assert_eq!(must_select(&rb, &groups, &[0]).references(), [0]);
}

#[test]
fn no_collision_outcomes_need_both_nonempty_sets_and_references_not_set_intersection() {
    let groups = [group_bytes(&[0], 0, 0)];
    let no_sets = root_bytes(&[1], Names::Ids(&[11]), &[0], &[], &[]);
    let selection = must_select(&no_sets, &groups, &[0]);
    assert_eq!(selection.references(), [0]);
    assert_eq!(selection.outcome(false), Outcome::FailedNoCollision);
    assert_eq!(selection.outcome(true), Outcome::Ok);
    let empty_range = root_bytes(&[1], Names::Ids(&[11]), &[0], &[(1, 0)], &[]);
    let selection = must_select(&empty_range, &groups, &[0]);
    assert_eq!(selection.outcome(false), Outcome::OkNoCollision);
    assert_eq!(selection.set_references(0).count(), 0);
    let no_refs = root_bytes(&[1], Names::Ids(&[11]), &[0], &[(0, 1)], &[]);
    assert_eq!(
        must_select(&no_refs, &groups, &[]).outcome(false),
        Outcome::FailedNoCollision
    );
}

#[test]
fn set_zero_then_requested_set_preserves_overlap_without_duplicate_storage() {
    let rb = root_bytes(
        &[1],
        Names::Ids(&[11]),
        &[0; 5],
        &[(0, 3), (2, 3), (4, 1), (0, 0)],
        &[],
    );
    let selection = must_select(&rb, &[group_bytes(&[4, 2, 0, 3, 1, 2], 0, 0)], &[0]);
    let refs = |set| {
        selection
            .set_references(set)
            .map(|r| (r.set_index, r.reference))
            .collect::<Vec<_>>()
    };
    assert_eq!(selection.references(), [0, 1, 2, 3, 4]);
    assert_eq!(refs(0), [(0, 0), (0, 1), (0, 2)]);
    assert_eq!(refs(1), [(0, 0), (0, 1), (0, 2), (1, 2), (1, 3), (1, 4)]);
    assert_eq!(refs(2), [(0, 0), (0, 1), (0, 2), (2, 4)]);
    assert_eq!(refs(3), refs(0));
    assert_eq!(refs(u16::MAX), refs(0));
    assert_eq!(selection.references().len(), 5);
}

#[test]
fn empty_set_table_is_an_empty_iterator_for_any_request() {
    let rb = root_bytes(&[], Names::Absent, &[], &[], &[]);
    let selection = must_select(&rb, &[], &[]);
    assert_eq!(selection.set_references(0).count(), 0);
    assert_eq!(selection.set_references(65535).count(), 0);
    assert_eq!(selection.outcome(false), Outcome::FailedNoCollision);
}

#[test]
fn maximum_u16_reference_remains_a_valid_spawn_ordinal() {
    let spawns = vec![0; 65536];
    let rb = root_bytes(&[1], Names::Ids(&[11]), &spawns, &[(65535, 1)], &[]);
    let selection = must_select(&rb, &[group_bytes(&[65535, 65535], 0, 0)], &[0]);
    assert_eq!(selection.references(), [65535]);
    assert_eq!(
        selection.set_references(0).collect::<Vec<_>>(),
        [SetReference {
            set_index: 0,
            reference: 65535
        }]
    );
}

#[test]
fn set_ranges_past_u16_reference_domain_are_empty_without_truncation() {
    let spawns = vec![0; 65537];
    let rb = root_bytes(&[1], Names::Ids(&[11]), &spawns, &[(65536, 1)], &[]);
    let selection = must_select(&rb, &[group_bytes(&[65535], 0, 0)], &[0]);
    assert_eq!(selection.references(), [65535]);
    assert_eq!(selection.set_references(0).count(), 0);
}

#[test]
fn selected_metadata_is_owned_after_all_source_buffers_are_dropped() {
    let selection = {
        let rb = root_bytes(&[1], Names::Ids(&[11]), &[0, 0], &[(0, 2)], &[]);
        must_select(&rb, &[group_bytes(&[1, 0], 0, 0)], &[0])
    };
    assert_eq!(selection.references(), [0, 1]);
    assert_eq!(selection.set_references(0).count(), 2);
    assert!(selection.resident_bytes() >= 2 * 2 + 8);
}

#[test]
fn input_inventory_must_be_complete_before_any_selection_can_succeed() {
    let rb = root_bytes(&[1], Names::Ids(&[11]), &[0], &[(0, 1)], &[]);
    assert_eq!(
        selected(&rb, &[], &[0], Limits::default()).err(),
        Some(Error::Inventory)
    );
    let root = root(&rb);
    let gb = group_bytes(&[0], 0, 0);
    let group = Group::parse(&gb, Limits::default()).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        resolve(
            &root,
            &[GroupInput {
                file_data_id: 2,
                group: &group
            }],
            &[0],
            Limits::default()
        )
        .err(),
        Some(Error::Inventory)
    );
}

#[test]
fn caller_limits_reparse_sources_and_limit_success_input_before_allocation() {
    let rb = root_bytes(&[1], Names::Ids(&[11]), &[0, 0], &[(0, 2)], &[]);
    let groups = [group_bytes(&[0, 1], 0, 0)];
    assert_eq!(
        selected(
            &rb,
            &groups,
            &[0],
            Limits {
                max_input_bytes: rb.len() - 1,
                ..Limits::default()
            }
        )
        .err(),
        Some(Error::Limit)
    );
    assert_eq!(
        selected(
            &rb,
            &groups,
            &[0],
            Limits {
                max_metadata_records: 1,
                ..Limits::default()
            }
        )
        .err(),
        Some(Error::Limit)
    );
    assert_eq!(
        selected(
            &rb,
            &groups,
            &[0],
            Limits {
                max_groups: 0,
                ..Limits::default()
            }
        )
        .err(),
        Some(Error::Limit)
    );
    let smaller = root_bytes(&[], Names::Ids(&[11]), &[], &[], &[]);
    assert_eq!(
        selected(
            &smaller,
            &[],
            &[0, 0],
            Limits {
                max_metadata_records: 1,
                ..Limits::default()
            }
        )
        .err(),
        Some(Error::Limit)
    );
    assert_eq!(
        selected(
            &rb,
            &groups,
            &[0],
            Limits {
                max_projection_bytes: super::super::MAX_PROJECTION_BYTES + 1,
                ..Limits::default()
            }
        )
        .err(),
        Some(Error::InvalidLimits)
    );
}

#[test]
fn live_memory_cap_includes_context_success_scratch_refs_and_set_ranges() {
    let rb = root_bytes(&[1], Names::Ids(&[11]), &[0, 0, 0], &[(0, 3)], &[]);
    let groups = [group_bytes(&[0, 2], 0, 0)];
    let expected = std::mem::size_of::<super::super::GroupContext<'_, '_>>() + 4 + 3 + 4 + 8;
    let limits = Limits {
        max_projection_bytes: expected,
        ..Limits::default()
    };
    let selection = selected(&rb, &groups, &[0], limits).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(selection.resident_bytes(), 4 + 8);
    assert_eq!(
        selected(
            &rb,
            &groups,
            &[0],
            Limits {
                max_projection_bytes: expected - 1,
                ..limits
            }
        )
        .err(),
        Some(Error::Limit)
    );
    let empty = root_bytes(&[], Names::Absent, &[], &[], &[]);
    assert_eq!(
        selected(
            &empty,
            &[],
            &[],
            Limits {
                max_projection_bytes: 0,
                ..Limits::default()
            }
        )
        .unwrap_or_else(|e| panic!("{e}"))
        .resident_bytes(),
        0
    );
}

#[test]
fn many_unordered_successes_are_validated_and_matched_without_request_duplicates() {
    let ids: Vec<_> = (1..=10000u32).collect();
    let spawns: Vec<_> = (0..10000u32).collect();
    let valid: Vec<_> = (0..10000u32).rev().collect();
    let refs: Vec<_> = (0..10000u16).rev().collect();
    let rb = root_bytes(&[1], Names::Ids(&ids), &spawns, &[(0, 10000)], &[]);
    let selection = must_select(&rb, &[group_bytes(&refs, 0, 0)], &valid);
    assert_eq!(selection.references().len(), 10000);
    assert_eq!(selection.references()[0], 0);
    assert_eq!(selection.references()[9999], 9999);
}
