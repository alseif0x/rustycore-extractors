// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: synthetic ROOT rows and logical names only.

use super::*;
use crate::root::ContentFlags;

#[derive(Default)]
struct Fixture {
    blocks: Vec<Block>,
    entries: Vec<Entry>,
}

impl Fixture {
    fn group(&mut self, flags: u32, locale: u32, rows: &[(u32, u8, Option<u64>)]) {
        let start = self.entries.len();
        for &(file_data_id, key, file_name_hash) in rows {
            self.entries.push(Entry {
                file_data_id,
                content_key: [key; 16],
                file_name_hash,
                block_index: self.blocks.len(),
            });
        }
        self.blocks.push(Block {
            locale_flags: locale,
            content_flags: ContentFlags::Legacy(flags),
            entries: start..self.entries.len(),
        });
    }

    fn build(&self) -> Result<NameIndex, Error> {
        NameIndex::build(
            &self.blocks,
            &self.entries,
            Locale::EnUs,
            |_| true,
            Limits::default(),
        )
    }
}

fn hash(name: &[u8]) -> u64 {
    hash_name(name, MAX_NAME_BYTES).unwrap()
}

fn named_fixture() -> Fixture {
    let mut fixture = Fixture::default();
    fixture.group(0, 2, &[(7, 1, Some(hash(b"WORLD\\MAPS\\A\\A_0_0.ADT")))]);
    fixture
}

fn assert_route(index: &NameIndex, name: &[u8], ordinal: usize, id: u32, key: u8) {
    let route = index.resolve(name).unwrap().unwrap();
    assert_eq!(route.first_entry_ordinal, ordinal);
    assert_eq!(route.file_data_id, id);
    assert_eq!(route.content_key, [key; 16]);
}

#[test]
fn independent_original_lookup3_vectors_preserve_output_word_order() {
    // Coordinator compiled unchanged pinned dep/CascLib/src/jenkins/lookup3.c
    // with pc=pb=0. These literals were supplied independently of this module
    // and of its reused Rust Jenkins implementation (2026-10-11).
    for (name, expected) in [
        (b"A".as_slice(), 0x0101_4ba1_1078_6e8c),
        (b"ABCDEFGHIJK", 0x09ce_7a9e_9518_09da),
        (b"ABCDEFGHIJKL", 0x4dcc_6ecf_4f3d_c944),
        (b"ABCDEFGHIJKLM", 0xef91_ecec_9507_1727),
        (b"ABCDEFGHIJKLMNOPQRSTUVWX", 0x4a34_1603_228d_ae1c),
        (b"WORLD\\MAPS\\FOO\\FOO_3_4.ADT", 0xf33f_a794_0c0e_497e),
    ] {
        assert_eq!(hash(name), expected);
    }
    assert_eq!(hash(&[b'A'; 1024]), 0x7260_9df7_934f_abb8);
}

#[test]
fn normalization_is_ascii_case_and_slash_only() {
    let expected = hash_normalized(b"WORLD\\MAPS\\FOO\\FOO_3_4.ADT");
    for name in [
        b"World/Maps/Foo/Foo_3_4.adt".as_slice(),
        b"world\\maps\\foo\\foo_3_4.adt",
        b"WORLD\\MAPS\\FOO\\FOO_3_4.ADT",
    ] {
        assert_eq!(hash(name), expected);
    }
    for (name, normalized) in [
        (b"a//b/../c ".as_slice(), b"A\\\\B\\..\\C ".as_slice()),
        (b"a:b", b"A:B"),
        (b"a\x01\x7fb", b"A\x01\x7fB"),
    ] {
        assert_eq!(hash(name), hash_normalized(normalized));
    }
    assert_ne!(hash(b"A//B"), hash(b"A/B"));
    assert_ne!(hash(b"A/../B"), hash(b"B"));
}

#[test]
fn names_reject_empty_nul_and_non_ascii_bytes() {
    for name in [
        b"".as_slice(),
        b"a\0b",
        b"\0",
        b"\x80",
        b"\xff",
        "é".as_bytes(),
    ] {
        assert_eq!(hash_name(name, 1024), Err(Error::InvalidName));
    }
}

#[test]
fn name_byte_cap_includes_exact_linux_boundary_without_prefix_truncation() {
    let name = vec![b'A'; 1024];
    assert_eq!(hash(&name), hash_normalized(&name));
    assert_eq!(hash_name(&name, 1023), Err(Error::Limit));
    let mut longer = name;
    longer.push(b'B');
    assert_eq!(hash_name(&longer, 1024), Err(Error::Limit));
    assert_eq!(hash_name(b"A", 0), Err(Error::Limit));
    assert_eq!(hash_name(b"", 0), Err(Error::InvalidName));
    assert_eq!(hash_name(b"A", 1025), Err(Error::InvalidLimits));
}

#[test]
fn internal_name_hashes_reject_even_when_root_has_them() {
    for name in RESERVED_NAMES {
        let mut fixture = Fixture::default();
        fixture.group(0, 2, &[(7, 1, Some(hash_normalized(name)))]);
        let index = fixture.build().unwrap();
        assert_eq!(index.stats().names, 1);
        assert_eq!(hash_name(name, 1024), Err(Error::ReservedName));
        assert_eq!(index.resolve(name).err(), Some(Error::ReservedName));
        let lower: Vec<_> = name.iter().map(u8::to_ascii_lowercase).collect();
        assert_eq!(index.resolve(&lower).err(), Some(Error::ReservedName));
    }
}

#[test]
fn later_hash_resolves_first_ckey_not_the_hash_bearing_row() {
    let mut fixture = Fixture::default();
    fixture.group(0, 0, &[(7, 1, None)]);
    fixture.group(0, 2, &[(7, 2, Some(hash(b"LATE")))]);
    fixture.group(0, 2, &[(7, 3, Some(0))]);
    fixture.group(0, 2, &[(7, 4, None)]);
    let index = fixture.build().unwrap();
    assert_route(&index, b"late", 0, 7, 1);
    assert_eq!(index.stats().nodes, 1);
    assert_eq!(index.stats().names, 1);
    assert_eq!(index.stats().central_entries, 4);
}

#[test]
fn repeated_identical_hash_preserves_first_ckey() {
    let mut fixture = Fixture::default();
    fixture.group(0, 2, &[(7, 1, Some(hash(b"SAME")))]);
    fixture.group(0, 2, &[(7, 2, Some(hash(b"SAME")))]);
    assert_route(&fixture.build().unwrap(), b"SAME", 0, 7, 1);
}

#[test]
fn missing_central_key_does_not_create_a_node_or_mutate_its_hash() {
    let mut fixture = Fixture::default();
    fixture.group(0, 2, &[(7, 1, Some(hash(b"ABSENT")))]);
    fixture.group(0, 2, &[(7, 2, None)]);
    fixture.group(0, 2, &[(7, 3, Some(hash(b"PRESENT")))]);
    let mut visited = Vec::new();
    let index = NameIndex::build(
        &fixture.blocks,
        &fixture.entries,
        Locale::EnUs,
        |key| {
            visited.push(key[0]);
            key[0] != 1
        },
        Limits::default(),
    )
    .unwrap();
    assert_eq!(visited, [1, 2, 3]);
    assert!(index.resolve(b"ABSENT").unwrap().is_none());
    assert_route(&index, b"PRESENT", 1, 7, 2);
    assert_eq!(index.stats().central_entries, 2);
}

#[test]
fn selection_has_no_local_availability_input_or_later_row_fallback() {
    let mut fixture = Fixture::default();
    fixture.group(0, 2, &[(7, 1, Some(hash(b"FILE")))]);
    fixture.group(0, 2, &[(7, 2, Some(hash(b"FILE")))]);
    // Both keys are centrally present. A later integration may fail reading
    // key 1 locally; repeated pure lookup must still return key 1.
    let index = fixture.build().unwrap();
    for _ in 0..2 {
        assert_route(&index, b"FILE", 0, 7, 1);
    }
}

#[test]
fn audio_passes_then_physical_order_control_callback_and_first_key() {
    let mut fixture = Fixture::default();
    fixture.group(0x8000_0000, 2, &[(7, 1, Some(hash(b"FILE")))]);
    fixture.group(0, 0, &[(7, 2, Some(hash(b"FILE")))]);
    fixture.group(0, 2, &[(7, 3, Some(hash(b"FILE")))]);
    fixture.group(0x8000_0000, 0, &[(7, 4, Some(hash(b"FILE")))]);
    let mut visited = Vec::new();
    let index = NameIndex::build(
        &fixture.blocks,
        &fixture.entries,
        Locale::EnUs,
        |key| {
            visited.push(key[0]);
            true
        },
        Limits::default(),
    )
    .unwrap();
    assert_eq!(visited, [2, 3, 1, 4]);
    assert_route(&index, b"FILE", 1, 7, 2);
}

#[test]
fn locale_and_content_exclusions_precede_callback_and_conflict_detection() {
    let mut fixture = Fixture::default();
    fixture.group(0x100, 0, &[(7, 1, Some(hash(b"EXCLUDED1")))]);
    fixture.group(0x80, 0, &[(7, 2, Some(hash(b"EXCLUDED2")))]);
    fixture.group(0, 2, &[(7, 3, Some(hash(b"ENGLISH")))]);
    fixture.group(0, 0x80, &[(7, 4, Some(hash(b"SPANISH")))]);
    fixture.group(0, 0, &[(9, 5, Some(hash(b"NEUTRAL")))]);
    for (locale, wanted, rejected, key, ordinal) in [
        (
            Locale::EnUs,
            b"ENGLISH".as_slice(),
            b"SPANISH".as_slice(),
            3,
            2,
        ),
        (
            Locale::EsEs,
            b"SPANISH".as_slice(),
            b"ENGLISH".as_slice(),
            4,
            3,
        ),
    ] {
        let mut visited = Vec::new();
        let index = NameIndex::build(
            &fixture.blocks,
            &fixture.entries,
            locale,
            |value| {
                visited.push(value[0]);
                true
            },
            Limits::default(),
        )
        .unwrap();
        assert_eq!(visited, [key, 5]);
        assert_route(&index, wanted, ordinal, 7, key);
        assert_route(&index, b"NEUTRAL", 4, 9, 5);
        assert!(index.resolve(rejected).unwrap().is_none());
    }
}

#[test]
fn eses_does_not_fall_back_to_enus_and_architecture_is_not_preferred() {
    let mut fixture = Fixture::default();
    fixture.group(0, 2, &[(7, 1, Some(hash(b"ENGLISH")))]);
    let index = NameIndex::build(
        &fixture.blocks,
        &fixture.entries,
        Locale::EsEs,
        |_| true,
        Limits::default(),
    )
    .unwrap();
    assert!(index.resolve(b"ENGLISH").unwrap().is_none());

    let mut fixture = Fixture::default();
    fixture.group(0x40, 0, &[(7, 1, Some(hash(b"FILE")))]);
    fixture.group(0, 2, &[(7, 2, Some(hash(b"FILE")))]);
    assert_route(&fixture.build().unwrap(), b"FILE", 0, 7, 1);
}

#[test]
fn modern_content_flag_components_use_the_existing_effective_flags() {
    let mut fixture = Fixture::default();
    fixture.group(0, 2, &[(7, 1, Some(hash(b"FILE")))]);
    fixture.group(0, 2, &[(7, 2, Some(hash(b"FILE")))]);
    fixture.blocks[0].content_flags = ContentFlags::Modern {
        flags1: 0,
        flags2: 0x8000_0000,
        flags3: 0,
    };
    fixture.blocks[1].content_flags = ContentFlags::Modern {
        flags1: 0,
        flags2: 0,
        flags3: 3,
    };
    assert_route(&fixture.build().unwrap(), b"FILE", 1, 7, 2);
    fixture.blocks[1].content_flags = ContentFlags::Modern {
        flags1: 0,
        flags2: 0x100,
        flags3: 0,
    };
    assert_route(&fixture.build().unwrap(), b"FILE", 0, 7, 1);
}

#[test]
fn changing_a_nodes_nonzero_hash_rejects_global_name_capability() {
    let mut fixture = Fixture::default();
    fixture.group(
        0,
        2,
        &[(7, 1, Some(hash(b"OLD"))), (8, 2, Some(hash(b"UNRELATED")))],
    );
    fixture.group(0, 2, &[(7, 3, Some(hash(b"NEW")))]);
    assert_eq!(fixture.build().err(), Some(Error::NameConflict));
}

#[test]
fn shared_hash_between_ids_rejects_even_if_their_ckeys_are_identical() {
    for second_key in [1, 2] {
        let mut fixture = Fixture::default();
        fixture.group(
            0,
            2,
            &[
                (7, 1, Some(hash(b"SHARED"))),
                (8, second_key, Some(hash(b"SHARED"))),
            ],
        );
        assert_eq!(fixture.build().err(), Some(Error::NameConflict));
    }
}

#[test]
fn initially_unnamed_node_collision_rejects_rebuild_dependent_selection() {
    let mut fixture = Fixture::default();
    fixture.group(0, 2, &[(7, 1, Some(0)), (8, 2, Some(hash(b"SHARED")))]);
    fixture.group(0, 2, &[(7, 3, Some(hash(b"SHARED")))]);
    assert_eq!(fixture.build().err(), Some(Error::NameConflict));
}

#[test]
fn shared_ckey_with_distinct_names_preserves_each_actual_node_id() {
    let mut fixture = Fixture::default();
    fixture.group(
        0,
        2,
        &[(7, 1, Some(hash(b"FIRST"))), (8, 1, Some(hash(b"SECOND")))],
    );
    let index = fixture.build().unwrap();
    assert_route(&index, b"FIRST", 0, 7, 1);
    assert_route(&index, b"SECOND", 1, 8, 1);
}

#[test]
fn zero_and_large_fdid_are_values_not_allocation_extents() {
    let mut fixture = Fixture::default();
    fixture.group(
        0,
        2,
        &[
            (0, 1, Some(hash(b"ZERO"))),
            (u32::MAX - 1, 2, Some(hash(b"LARGE"))),
        ],
    );
    let index = fixture.build().unwrap();
    assert_route(&index, b"ZERO", 0, 0, 1);
    assert_route(&index, b"LARGE", 1, u32::MAX - 1, 2);
    assert_eq!(index.stats().nodes, 2);
    assert!(index.peak_bytes() < 256);
}

#[test]
fn invalid_id_is_checked_even_in_excluded_or_centrally_absent_rows() {
    for flags in [0, 0x100] {
        let mut fixture = Fixture::default();
        fixture.group(flags, 2, &[(u32::MAX, 1, Some(0))]);
        let mut called = false;
        let result = NameIndex::build(
            &fixture.blocks,
            &fixture.entries,
            Locale::EnUs,
            |_| {
                called = true;
                false
            },
            Limits::default(),
        );
        assert_eq!(result.err(), Some(Error::FileDataId));
        assert!(!called);
    }
}

#[test]
fn textual_fdid_or_ckey_names_never_fall_back_to_another_lookup() {
    let fixture = named_fixture();
    let index = fixture.build().unwrap();
    for name in [
        b"FILE00000007.xxx".as_slice(),
        b"01010101010101010101010101010101",
    ] {
        assert!(index.resolve(name).unwrap().is_none());
    }
}

#[test]
fn forged_ranges_must_partition_the_complete_entry_slice() {
    let fixture = named_fixture();
    for range in [
        1..1,
        0..0,
        std::ops::Range { start: 1, end: 0 },
        0..2,
        0..usize::MAX,
    ] {
        let mut blocks = fixture.blocks.clone();
        blocks[0].entries = range;
        assert_eq!(
            NameIndex::build(
                &blocks,
                &fixture.entries,
                Locale::EnUs,
                |_| true,
                Limits::default()
            )
            .err(),
            Some(Error::Structure)
        );
    }
    assert_eq!(
        NameIndex::build(
            &[],
            &fixture.entries,
            Locale::EnUs,
            |_| true,
            Limits::default()
        )
        .err(),
        Some(Error::Structure)
    );
    let mut blocks = fixture.blocks.clone();
    blocks.push(blocks[0].clone()); // overlapping ownership of one row
    assert_eq!(
        NameIndex::build(
            &blocks,
            &fixture.entries,
            Locale::EnUs,
            |_| true,
            Limits::default()
        )
        .err(),
        Some(Error::Structure)
    );
}

#[test]
fn forged_block_index_or_physical_id_order_rejects() {
    let mut fixture = named_fixture();
    fixture.entries[0].block_index = usize::MAX;
    assert_eq!(fixture.build().err(), Some(Error::Structure));
    for second_id in [7, 6] {
        let mut fixture = Fixture::default();
        fixture.group(0x100, 0, &[(7, 1, Some(0)), (second_id, 2, Some(0))]);
        assert_eq!(fixture.build().err(), Some(Error::Structure));
    }
}

#[test]
fn forged_partial_hash_array_rejects_but_zero_and_absent_blocks_are_valid() {
    let mut fixture = Fixture::default();
    fixture.group(0, 0, &[(1, 1, None), (2, 2, Some(0))]);
    assert_eq!(fixture.build().err(), Some(Error::Structure));
    let mut fixture = Fixture::default();
    fixture.group(0, 0, &[(1, 1, None)]);
    fixture.group(0, 0, &[(2, 2, Some(0))]);
    let index = fixture.build().unwrap();
    assert_eq!(index.stats().nodes, 2);
    assert_eq!(index.stats().names, 0);
}

#[test]
fn empty_root_and_empty_blocks_need_no_heap_capacity() {
    let zero = Limits {
        max_blocks: 0,
        max_entries: 0,
        max_nodes: 0,
        max_names: 0,
        max_memory_bytes: 0,
        max_name_bytes: 0,
    };
    let index = NameIndex::build(&[], &[], Locale::EnUs, |_| panic!("no rows"), zero).unwrap();
    assert_eq!(index.resident_bytes(), 0);
    assert_eq!(index.peak_bytes(), 0);
    assert_eq!(index.resolve(b"A").err(), Some(Error::Limit));
    let mut fixture = Fixture::default();
    fixture.group(0, 0, &[]);
    fixture.group(0, 2, &[]);
    let index = fixture.build().unwrap();
    assert_eq!(index.stats().observed_entries, 0);
    assert_eq!(index.peak_bytes(), 0);
}

#[test]
fn count_limits_reject_without_truncating_nodes_or_names() {
    let fixture = named_fixture();
    for limits in [
        Limits {
            max_blocks: 0,
            ..Limits::default()
        },
        Limits {
            max_entries: 0,
            ..Limits::default()
        },
        Limits {
            max_nodes: 0,
            ..Limits::default()
        },
        Limits {
            max_names: 0,
            ..Limits::default()
        },
    ] {
        assert_eq!(
            NameIndex::build(
                &fixture.blocks,
                &fixture.entries,
                Locale::EnUs,
                |_| true,
                limits
            )
            .err(),
            Some(Error::Limit)
        );
    }
}

#[test]
fn all_centrally_absent_rows_use_scratch_but_no_resident_nodes() {
    let fixture = named_fixture();
    let index = NameIndex::build(
        &fixture.blocks,
        &fixture.entries,
        Locale::EnUs,
        |_| false,
        Limits {
            max_nodes: 0,
            max_names: 0,
            ..Limits::default()
        },
    )
    .unwrap();
    assert_eq!(index.stats().eligible_entries, 1);
    assert_eq!(index.stats().central_entries, 0);
    assert_eq!(index.stats().nodes, 0);
    assert_eq!(index.resident_bytes(), 0);
    assert_eq!(index.peak_bytes(), size_of::<u32>());
}

#[test]
fn memory_budget_charges_simultaneous_scratch_and_retained_capacities() {
    let fixture = named_fixture();
    let exact = size_of::<u32>() + size_of::<NamedRoute>();
    let index = NameIndex::build(
        &fixture.blocks,
        &fixture.entries,
        Locale::EnUs,
        |_| true,
        Limits {
            max_memory_bytes: exact,
            ..Limits::default()
        },
    )
    .unwrap();
    assert_eq!(index.resident_bytes(), size_of::<NamedRoute>());
    assert_eq!(index.peak_bytes(), exact);
    for cap in [exact - 1, size_of::<NamedRoute>(), 0] {
        assert_eq!(
            NameIndex::build(
                &fixture.blocks,
                &fixture.entries,
                Locale::EnUs,
                |_| true,
                Limits {
                    max_memory_bytes: cap,
                    ..Limits::default()
                },
            )
            .err(),
            Some(Error::Limit)
        );
    }
}

#[test]
fn scratch_reservation_budget_is_checked_before_central_callback() {
    let fixture = named_fixture();
    let mut called = false;
    let result = NameIndex::build(
        &fixture.blocks,
        &fixture.entries,
        Locale::EnUs,
        |_| {
            called = true;
            true
        },
        Limits {
            max_memory_bytes: size_of::<u32>() - 1,
            ..Limits::default()
        },
    );
    assert_eq!(result.err(), Some(Error::Limit));
    assert!(!called);
}

#[test]
fn every_limit_is_only_reducible_and_validate_is_pure() {
    let base = Limits::default();
    for limits in [
        Limits {
            max_blocks: base.max_blocks + 1,
            ..base
        },
        Limits {
            max_entries: base.max_entries + 1,
            ..base
        },
        Limits {
            max_nodes: base.max_nodes + 1,
            ..base
        },
        Limits {
            max_names: base.max_names + 1,
            ..base
        },
        Limits {
            max_memory_bytes: base.max_memory_bytes + 1,
            ..base
        },
        Limits {
            max_name_bytes: base.max_name_bytes + 1,
            ..base
        },
    ] {
        assert_eq!(limits.validate(), Err(Error::InvalidLimits));
        assert_eq!(
            NameIndex::build(&[], &[], Locale::EnUs, |_| panic!("invalid limits"), limits).err(),
            Some(Error::InvalidLimits)
        );
    }
}

#[test]
fn name_length_policy_is_retained_for_each_query() {
    let mut fixture = Fixture::default();
    fixture.group(0, 2, &[(1, 1, Some(hash(b"LONG")))]);
    let index = NameIndex::build(
        &fixture.blocks,
        &fixture.entries,
        Locale::EnUs,
        |_| true,
        Limits {
            max_name_bytes: 3,
            ..Limits::default()
        },
    )
    .unwrap();
    assert_eq!(index.resolve(b"LONG").err(), Some(Error::Limit));
    assert!(index.resolve(b"A").unwrap().is_none());
}

#[test]
fn index_owns_routes_after_inputs_are_dropped() {
    let index = {
        let fixture = named_fixture();
        fixture.build().unwrap()
    };
    assert_route(&index, b"world/maps/a/a_0_0.adt", 0, 7, 1);
}

#[test]
fn diagnostics_are_static_and_stats_contain_only_counts() {
    for error in [
        Error::InvalidLimits,
        Error::Limit,
        Error::Structure,
        Error::FileDataId,
        Error::InvalidName,
        Error::ReservedName,
        Error::NameConflict,
        Error::Allocation,
    ] {
        assert_eq!(error.to_string(), error.code());
        assert!(error.code().starts_with("root_name_"));
    }
    let stats = named_fixture().build().unwrap().stats();
    assert_eq!(
        (
            stats.observed_entries,
            stats.eligible_entries,
            stats.central_entries
        ),
        (1, 1, 1)
    );
    assert_eq!((stats.nodes, stats.names), (1, 1));
}
