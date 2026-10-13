// SPDX-License-Identifier: GPL-3.0-or-later
// Authored selected-domain final facts; complete package rejection is CLI QA.
use super::*;
fn sample() -> (Plan, Facts) {
    let mut plan = super::tests::db2_plan();
    plan.map_domain = Some(7);
    plan.terrain = Selection::Map(7);
    plan.components = Component::Db2.bit() | Component::Terrain.bit() | Component::Collision.bit();
    plan.db2 = vec![crate::plan::Entry {
        name: "Map".into(),
        id: 1349477,
    }];
    plan.collision_storage = CollisionStorage::LeasedDiskV1(crate::plan::DiskBudget {
        raw_bytes: 1024,
        wdt_bytes: 1024,
        cache_bytes: 1024,
        placement_bytes: 1024,
        vmo_bytes: 1024,
        map_bytes: 1024,
        output_bytes: 1024,
    });
    plan.validate().unwrap();
    let bytes =
        crate::map_domain::tests::table(&[(9, 999, -1, -1), (7, 101, 1, -1), (1, 100, -1, -1)]);
    let facts = Facts {
        domain: Some(crate::map_domain::Domain::derive(&bytes, 7, false).unwrap()),
        domain_terrain_lists: vec![1, 7],
        domain_collision_maps: vec![1, 7],
        trees: [(1, 0), (7, 0)].into_iter().collect(),
        ..Default::default()
    };
    (plan, facts)
}
#[test]
fn complete_domain_accepts_empty_geometry_maps() {
    let (p, f) = sample();
    assert_eq!(f.finish_domain(&p), Ok(()));
}
#[test]
fn missing_or_foreign_terrain_lists_reject() {
    for lists in [vec![7], vec![1], vec![1, 9], vec![1, 7, 9]] {
        let (p, mut f) = sample();
        f.domain_terrain_lists = lists;
        assert_eq!(
            f.finish_domain(&p).unwrap_err(),
            Error {
                code: "product_map_domain_terrain",
                component: Some(Component::Terrain)
            }
        );
    }
}
#[test]
fn foreign_terrain_tile_map_rejects_even_with_exact_lists() {
    let (p, mut f) = sample();
    f.domain_terrain_maps = vec![9];
    assert_eq!(
        f.finish_domain(&p).unwrap_err().code,
        "product_map_domain_terrain"
    );
}
#[test]
fn every_required_collision_tree_is_mandatory() {
    for removed in [1, 7] {
        let (p, mut f) = sample();
        f.trees.remove(&removed);
        assert_eq!(
            f.finish_domain(&p).unwrap_err().code,
            "product_map_domain_collision"
        );
    }
}
#[test]
fn foreign_collision_tree_and_folder_reject() {
    for tree in [false, true] {
        let (p, mut f) = sample();
        if tree {
            f.trees.insert(9, 0);
        } else {
            f.domain_collision_maps.push(9);
        }
        assert_eq!(
            f.finish_domain(&p).unwrap_err().code,
            "product_map_domain_collision"
        );
    }
}
#[test]
fn missing_source_is_not_an_empty_domain() {
    let (p, mut f) = sample();
    f.domain = None;
    assert_eq!(
        f.finish_domain(&p).unwrap_err().code,
        "product_map_domain_source"
    );
}
#[test]
fn requested_map_must_match_the_derived_source_domain() {
    let (mut plan, facts) = sample();
    plan.map_domain = Some(8);
    assert_eq!(
        facts.finish_domain(&plan).unwrap_err().code,
        "product_map_domain_source"
    );
}
#[test]
fn navigation_requires_exact_own_candidate_global_union() {
    for terrain in [vec![], vec![1], vec![7], vec![1, 7]] {
        for payloads in [vec![], vec![1], vec![7]] {
            for globals in [vec![], vec![1], vec![7], vec![1, 7], vec![9]] {
                let (mut plan, mut facts) = sample();
                plan.components |= Component::Navigation.bit();
                facts.domain_terrain_maps = terrain.clone();
                facts.domain_collision_payload_maps = payloads.clone();
                facts.domain_navigation_globals = globals.clone();
                let expected: [bool; 2] =
                    [1, 7].map(|id| terrain.contains(&id) || payloads.contains(&id));
                let matches = globals.len() == expected.iter().filter(|&&v| v).count()
                    && [1, 7]
                        .into_iter()
                        .zip(expected)
                        .all(|(id, own)| globals.contains(&id) == own);
                let result = facts.finish_domain(&plan);
                if matches {
                    assert_eq!(result, Ok(()));
                } else {
                    assert_eq!(result.unwrap_err().code, "product_map_domain_navigation");
                }
            }
        }
    }
}
#[test]
fn inherited_indexes_do_not_invent_requested_navigation_global() {
    let (mut plan, mut facts) = sample();
    plan.components |= Component::Navigation.bit();
    facts.domain_collision_payload_maps = vec![1];
    facts.domain_navigation_globals = vec![1];
    assert_eq!(facts.finish_domain(&plan), Ok(()));
    facts.domain_navigation_globals.push(7);
    assert_eq!(
        facts.finish_domain(&plan).unwrap_err().code,
        "product_map_domain_navigation"
    );
}
#[test]
fn fact_maps_have_bounded_sorted_capacity_and_reuse() {
    let mut maps = Vec::new();
    for id in (0..1024).rev() {
        domain_map(&mut maps, id).unwrap();
    }
    assert_eq!(maps, (0..1024).collect::<Vec<_>>());
    domain_map(&mut maps, 123).unwrap();
    assert_eq!(
        domain_map(&mut maps, 1024).unwrap_err().code,
        "product_map_domain_limit"
    );
    assert_eq!(maps.len(), 1024);
}
#[test]
fn legacy_final_facts_do_not_infer_any_domain() {
    let (mut p, f) = sample();
    p.map_domain = None;
    assert_eq!(Facts::default().finish_domain(&p), Ok(()));
    drop(f);
}
