// SPDX-License-Identifier: GPL-3.0-or-later
// Authored by RustyCore Extractors, 2026-10-17. SOURCE controls; not yet run.
use super::*;
fn one(parent: Option<u32>) -> Retained {
    Retained {
        maps: vec![Map {
            id: 2,
            wdt_file_data_id: 200,
            parent,
            observed: false,
            wdt: Wdt::Missing,
            global: None,
            cells: [0; 4096],
        }],
    }
}
#[test]
fn coverage_missing_wdt_has_no_tile_or_global_empty_claim() {
    let mut r = one(None);
    r.wdt(2, false).unwrap();
    r.finish().unwrap();
    assert_eq!(r.maps()[0].wdt(), Wdt::Missing);
    assert_eq!(r.maps()[0].global_records(), None);
    assert_eq!(r.maps()[0].tile(0, 0), None);
}
#[test]
fn coverage_present_empty_global_is_observed_not_missing() {
    let mut r = one(None);
    r.wdt(2, true).unwrap();
    r.global(2, 0).unwrap();
    r.finish().unwrap();
    assert_eq!(r.maps()[0].global_records(), Some(0));
    assert_eq!(r.maps()[0].tile(0, 0).unwrap().own(), Own::Unlisted);
}
#[test]
fn coverage_success_requires_all_wdt_attempts_and_globals() {
    let mut r = one(None);
    assert!(r.finish().is_err());
    r.wdt(2, true).unwrap();
    assert!(r.finish().is_err());
    r.global(2, 1).unwrap();
    r.finish().unwrap();
    assert!(r.global(2, 1).is_err());
    assert!(r.wdt(2, true).is_err());
}
#[test]
fn coverage_missing_own_keeps_parent_output_replay_distinct() {
    let mut r = one(Some(1));
    r.wdt(2, true).unwrap();
    r.global(2, 0).unwrap();
    r.own(2, 0, 1, Own::Missing).unwrap();
    r.replay(2, 1, 0, 1, 3).unwrap();
    let t = r.maps()[0].tile(0, 1).unwrap();
    assert_eq!(t.own(), Own::Missing);
    assert_eq!(t.replay(), Replay::Output);
    assert_eq!(t.replay_parent(), Some(1));
}
#[test]
fn coverage_unlisted_replay_is_not_an_own_adt_attempt() {
    let mut r = one(Some(1));
    r.wdt(2, true).unwrap();
    r.global(2, 0).unwrap();
    r.replay(2, 1, 0, 1, 0).unwrap();
    let t = r.maps()[0].tile(0, 1).unwrap();
    assert_eq!(t.own(), Own::Unlisted);
    assert_eq!(t.replay(), Replay::Empty);
}
#[test]
fn coverage_empty_and_nonempty_own_both_block_replay() {
    for own in [Own::Empty, Own::Output] {
        let mut r = one(Some(1));
        r.wdt(2, true).unwrap();
        r.global(2, 0).unwrap();
        r.own(2, 0, 1, own).unwrap();
        assert!(r.replay(2, 1, 0, 1, 9).is_err());
        let t = r.maps()[0].tile(0, 1).unwrap();
        assert_eq!(t.own(), own);
        assert_eq!(t.replay(), Replay::None);
        assert_eq!(t.replay_parent(), None);
    }
}
#[test]
fn coverage_replay_must_be_exact_immediate_parent_and_once() {
    let mut r = one(Some(1));
    r.wdt(2, true).unwrap();
    r.global(2, 0).unwrap();
    assert!(r.replay(2, 0, 0, 1, 2).is_err());
    r.replay(2, 1, 0, 1, 2).unwrap();
    assert!(r.replay(2, 1, 0, 1, 2).is_err());
    assert!(r.own(2, 0, 1, Own::Empty).is_err());
}
#[test]
fn coverage_unknown_map_and_outside_cells_reject_without_growth() {
    let mut r = one(None);
    let bytes = r.resident_bytes().unwrap();
    r.wdt(2, true).unwrap();
    r.global(2, 0).unwrap();
    assert!(r.own(3, 0, 1, Own::Empty).is_err());
    assert!(r.own(2, 64, 0, Own::Empty).is_err());
    assert_eq!(r.maps()[0].tile(64, 0), None);
    assert_eq!(r.resident_bytes().unwrap(), bytes);
}
#[test]
fn coverage_retained_charge_counts_actual_vector_capacity() {
    let mut r = one(None);
    r.maps.reserve_exact(3);
    assert_eq!(
        r.resident_bytes().unwrap(),
        r.maps.capacity() * size_of::<Map>()
    );
    assert!(r.resident_bytes().unwrap() > size_of::<Map>());
}
