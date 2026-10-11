// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: bounded serial ID call sequences, no client identifiers.
use super::*;
#[test]
fn m2_repeated_key_consumes_counter_before_returning_first_value() {
    let mut a = IdAllocator::new(Limits::default()).unwrap();
    assert_eq!(a.assign(7, 0, false), Ok(u32::MAX - 1));
    assert_eq!(a.assign(7, 0, false), Ok(u32::MAX - 1));
    assert_eq!(a.assign(8, 0, false), Ok(u32::MAX - 3));
    assert_eq!(a.stats().entries, 2);
    assert_eq!(a.stats().m2_counter_consumed, 3);
}
#[test]
fn wmo_preserves_client_id_and_never_consumes_counter() {
    let mut a = IdAllocator::new(Limits::default()).unwrap();
    for id in [0, 1, u32::MAX] {
        assert_eq!(a.assign(id, 0, true), Ok(id));
        assert_eq!(a.assign(id, 0, true), Ok(id));
    }
    assert_eq!(a.stats().next_m2_id, Some(u32::MAX - 1));
    assert_eq!(a.stats().m2_counter_consumed, 0);
}
#[test]
fn cross_class_first_winner_is_shared_without_map_or_class_keys() {
    let mut a = IdAllocator::new(Limits::default()).unwrap();
    assert_eq!(a.assign(7, 3, true), Ok(7));
    assert_eq!(a.assign(7, 3, false), Ok(7));
    assert_eq!(a.assign(8, 0, false), Ok(u32::MAX - 2));
    let mut a = IdAllocator::new(Limits::default()).unwrap();
    assert_eq!(a.assign(7, 3, false), Ok(u32::MAX - 1));
    assert_eq!(a.assign(7, 3, true), Ok(u32::MAX - 1));
    assert_eq!(a.stats().m2_counter_consumed, 1);
}
#[test]
fn doodad_id_is_a_full_u16_part_of_the_key() {
    let mut a = IdAllocator::new(Limits::default()).unwrap();
    assert_eq!(a.assign(7, 0, false), Ok(u32::MAX - 1));
    assert_eq!(a.assign(7, 255, false), Ok(u32::MAX - 2));
    assert_eq!(a.assign(7, 256, false), Ok(u32::MAX - 3));
    assert_eq!(a.assign(7, 65535, false), Ok(u32::MAX - 4));
    assert_eq!(a.assign(7, 256, true), Ok(u32::MAX - 3));
}
#[test]
fn two_distinct_keys_cannot_silently_share_assigned_id() {
    let mut a = IdAllocator::new(Limits::default()).unwrap();
    assert_eq!(a.assign(u32::MAX - 1, 0, true), Ok(u32::MAX - 1));
    assert_eq!(a.assign(7, 0, false), Err(Error::AssignedIdCollision));
    let before = a.stats();
    assert_eq!(a.assign(9, 0, true), Err(Error::AssignedIdCollision));
    assert_eq!(a.stats(), before);
    let mut a = IdAllocator::new(Limits::default()).unwrap();
    a.assign(7, 0, false).unwrap();
    assert_eq!(
        a.assign(u32::MAX - 1, 0, true),
        Err(Error::AssignedIdCollision)
    );
}
#[test]
fn two_wmo_keys_for_same_client_with_different_doodads_are_rejected() {
    let mut a = IdAllocator::new(Limits::default()).unwrap();
    assert_eq!(a.assign(7, 0, true), Ok(7));
    assert_eq!(a.assign(7, 1, true), Err(Error::AssignedIdCollision));
}
#[test]
fn unused_proposed_m2_collision_does_not_override_existing_key() {
    let mut a = IdAllocator::new(Limits::default()).unwrap();
    a.assign(u32::MAX - 1, 0, true).unwrap();
    a.assign(7, 0, true).unwrap();
    assert_eq!(a.assign(7, 0, false), Ok(7));
    assert_eq!(a.assign(8, 0, false), Ok(u32::MAX - 2));
}
#[test]
fn last_counter_value_zero_is_usable_but_wrap_is_rejected_even_for_hit() {
    let mut a = IdAllocator::new(Limits::default()).unwrap();
    a.stats.next_m2_id = Some(1);
    assert_eq!(a.assign(7, 0, false), Ok(1));
    assert_eq!(a.assign(8, 0, false), Ok(0));
    assert_eq!(a.stats().next_m2_id, None);
    assert_eq!(a.assign(7, 0, false), Err(Error::CounterExhausted));
    let s = a.stats();
    assert_eq!(a.assign(9, 0, true), Err(Error::CounterExhausted));
    assert_eq!(a.stats(), s);
}
#[test]
fn wmo_can_still_assign_after_zero_was_used_before_an_exhausted_m2_call() {
    let mut a = IdAllocator::new(Limits::default()).unwrap();
    a.stats.next_m2_id = Some(0);
    a.assign(7, 0, false).unwrap();
    assert_eq!(a.assign(42, 0, true), Ok(42));
}
#[test]
fn full_entry_limit_allows_hits_but_rejects_new_key_and_poisons() {
    let mut a = IdAllocator::new(Limits {
        max_entries: 1,
        ..Limits::default()
    })
    .unwrap();
    a.assign(7, 0, false).unwrap();
    assert_eq!(a.assign(7, 0, false), Ok(u32::MAX - 1));
    assert_eq!(a.assign(8, 0, false), Err(Error::Limit));
    let s = a.stats();
    assert_eq!(a.assign(7, 0, false), Err(Error::Limit));
    assert_eq!(a.stats(), s);
}
#[test]
fn invalid_and_zero_limits_are_bounded_before_allocation() {
    for l in [
        Limits {
            max_entries: MAX_ENTRIES + 1,
            ..Limits::default()
        },
        Limits {
            max_memory_bytes: MAX_MEMORY_BYTES + 1,
            ..Limits::default()
        },
        Limits {
            max_probes: MAX_PROBES + 1,
            ..Limits::default()
        },
    ] {
        assert_eq!(IdAllocator::new(l).err(), Some(Error::InvalidLimits));
    }
    for l in [
        Limits {
            max_entries: 0,
            ..Limits::default()
        },
        Limits {
            max_memory_bytes: 0,
            ..Limits::default()
        },
    ] {
        let mut a = IdAllocator::new(l).unwrap();
        assert_eq!(a.stats().resident_bytes, 0);
        assert_eq!(a.assign(7, 0, false), Err(Error::Limit));
        assert_eq!(a.stats().resident_bytes, 0);
    }
}
#[test]
fn exact_memory_limit_covers_old_and_new_tables_during_rehash() {
    let mut a = IdAllocator::new(Limits::default()).unwrap();
    a.assign(7, 0, false).unwrap();
    a.assign(8, 0, false).unwrap();
    let peak = a.stats().peak_bytes;
    let resident = a.stats().resident_bytes;
    assert!(peak > resident);
    let mut exact = IdAllocator::new(Limits {
        max_memory_bytes: peak,
        ..Limits::default()
    })
    .unwrap();
    exact.assign(7, 0, false).unwrap();
    assert!(exact.assign(8, 0, false).is_ok());
    let mut short = IdAllocator::new(Limits {
        max_memory_bytes: peak - 1,
        ..Limits::default()
    })
    .unwrap();
    short.assign(7, 0, false).unwrap();
    assert_eq!(short.assign(8, 0, false), Err(Error::Limit));
    assert_eq!(short.stats().entries, 1);
}
#[test]
fn many_serial_keys_survive_growth_and_first_values_remain_stable() {
    let mut a = IdAllocator::new(Limits::default()).unwrap();
    for n in 0..4096 {
        assert_eq!(a.assign(n, 0, false), Ok(u32::MAX - 1 - n));
    }
    for n in (0..4096).rev() {
        assert_eq!(a.assign(n, 0, false), Ok(u32::MAX - 1 - n));
    }
    assert_eq!(a.stats().entries, 4096);
    assert_eq!(a.stats().m2_counter_consumed, 8192);
    assert!(a.stats().peak_bytes <= MAX_MEMORY_BYTES);
}
#[test]
fn reduced_probe_budget_fails_closed_instead_of_unbounded_collision_scan() {
    let mut zero = IdAllocator::new(Limits {
        max_probes: 0,
        ..Limits::default()
    })
    .unwrap();
    assert_eq!(zero.assign(7, 0, true), Err(Error::ProbeLimit));
    let s = zero.stats();
    assert_eq!(zero.assign(8, 0, true), Err(Error::ProbeLimit));
    assert_eq!(zero.stats(), s);
    // Select two known keys sharing the first two-slot key bucket. Numeric
    // output uniqueness is independent; the key scan reaches its explicit cap.
    let first = 7u32;
    let second = (8..1000)
        .find(|n| key_hash(*n, 0) & 1 == key_hash(first, 0) & 1)
        .unwrap();
    let mut a = IdAllocator::new(Limits {
        max_probes: 1,
        ..Limits::default()
    })
    .unwrap();
    a.assign(first, 0, true).unwrap();
    assert_eq!(a.assign(second, 0, true), Err(Error::ProbeLimit));
    assert_eq!(a.stats().entries, 1);
}
#[test]
fn table_capacities_match_resident_accounting() {
    let mut a = IdAllocator::new(Limits::default()).unwrap();
    for n in 0..30 {
        a.assign(n, 0, false).unwrap();
    }
    assert_eq!(
        a.stats().resident_bytes,
        a.keys.capacity() * size_of::<Option<Entry>>()
            + a.assigned.capacity() * size_of::<Option<u32>>()
    );
    assert!(a.stats().peak_bytes >= a.stats().resident_bytes);
}
