// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: synthetic complete model/map manager query fixtures.
use super::*;
use crate::{
    assembly, bih, placement,
    store::{Artifact, Limits, MapParent, Options, Presence, ReadBuffer, SourceError},
};
use std::{cell::Cell, rc::Rc};

fn word(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}
fn floats(out: &mut Vec<u8>, values: &[f32]) {
    for value in values {
        word(out, value.to_bits());
    }
}
fn bound() -> [f32; 6] {
    [0., 0., 0., 4., 4., 0.]
}
fn tree() -> Vec<u8> {
    bih::build(
        &[bih::Bounds::from_bits(bound().map(f32::to_bits)).unwrap()],
        3,
        bih::Limits::default(),
    )
    .unwrap()
    .into_bytes()
}
fn model(m2: bool) -> Vec<u8> {
    let mut out = b"VMAP_4.EWMOD".to_vec();
    for n in [8, u32::from(m2), 777] {
        word(&mut out, n);
    }
    out.extend_from_slice(b"GMOD");
    word(&mut out, 1);
    floats(&mut out, &bound());
    for n in [0, 7] {
        word(&mut out, n);
    }
    out.extend_from_slice(b"VERT");
    word(&mut out, 40);
    word(&mut out, 3);
    floats(&mut out, &[0., 0., 0., 4., 0., 0., 0., 4., 0.]);
    out.extend_from_slice(b"TRIM");
    for n in [16, 1, 0, 1, 2] {
        word(&mut out, n);
    }
    out.extend_from_slice(b"MBIH");
    out.extend_from_slice(&tree());
    out.extend_from_slice(b"LIQU");
    word(&mut out, 0);
    out.extend_from_slice(b"GBIH");
    out.extend_from_slice(&tree());
    out
}
struct Fixture {
    tree: Vec<u8>,
    tile: Vec<u8>,
    index: Vec<u8>,
    model: Vec<u8>,
    changed: Rc<Cell<bool>>,
}
impl Source for Fixture {
    fn read(
        &mut self,
        artifact: Artifact<'_>,
        out: &mut ReadBuffer,
    ) -> Result<Presence, SourceError> {
        if self.changed.get() {
            return Err(SourceError::InputChanged);
        }
        let bytes = match artifact {
            Artifact::Tree { map_id: 1 } => &self.tree,
            Artifact::Tile {
                map_id: 1,
                x: 0,
                y: 0,
            } => &self.tile,
            Artifact::Index {
                map_id: 1,
                x: 0,
                y: 0,
            } => &self.index,
            Artifact::Model { name: b"plane" } => &self.model,
            _ => return Ok(Presence::Missing),
        };
        out.append(bytes)?;
        Ok(Presence::Present)
    }
    fn verify_unchanged(&mut self) -> Result<(), SourceError> {
        if self.changed.get() {
            Err(SourceError::InputChanged)
        } else {
            Ok(())
        }
    }
}
fn make(m2: bool, options: Options, load: bool) -> (StaticStore<Fixture>, Rc<Cell<bool>>) {
    let mut raw = vec![1, 0];
    word(&mut raw, 9);
    floats(&mut raw, &[0., 0., 0., 0., 0., 0., 1.]);
    floats(&mut raw, &bound());
    word(&mut raw, 5);
    raw.extend_from_slice(b"plane");
    let parsed = placement::DirBin::parse(&raw, placement::Limits::default()).unwrap();
    let map = assembly::build(
        1,
        &parsed.records().collect::<Vec<_>>(),
        assembly::Limits::default(),
    )
    .unwrap();
    let changed = Rc::new(Cell::new(false));
    let source = Fixture {
        tree: map.tree_bytes().to_vec(),
        tile: map.tiles()[0].payload().unwrap().to_vec(),
        index: map.tiles()[0].index_bytes().to_vec(),
        model: model(m2),
        changed: changed.clone(),
    };
    let mut store = StaticStore::new(
        source,
        &[MapParent {
            map_id: 1,
            parent: None,
            cosmetic_parent: None,
        }],
        options,
        Limits::default(),
    )
    .unwrap();
    if load {
        store.load_tile(1, 0, 0).unwrap();
    }
    (store, changed)
}
fn budget() -> Budget {
    Budget::new(query::Limits::default()).unwrap()
}
fn top() -> [f32; 3] {
    [1., 1., 10.]
}
fn bottom() -> [f32; 3] {
    [1., 1., -10.]
}

#[test]
fn midpoint_is_exact_and_reflection_preserves_z_bits() {
    assert_eq!(WORLD_MIDPOINT.to_bits(), 0x4685_5555);
    assert_eq!(
        reflect([WORLD_MIDPOINT, WORLD_MIDPOINT, -0.])[2].to_bits(),
        (-0f32).to_bits()
    );
    assert_eq!(
        &reflect([WORLD_MIDPOINT, WORLD_MIDPOINT, 0.])[..2],
        &[0., 0.]
    );
}
#[test]
fn actual_loaded_model_blocks_internal_and_world_segments() {
    let (store, _) = make(false, Options::default(), true);
    let mut b = budget();
    assert!(
        !store
            .line_of_sight_internal(1, top(), bottom(), false, &mut b)
            .unwrap()
    );
    assert!(
        !store
            .line_of_sight_world(1, reflect(top()), reflect(bottom()), false, &mut b)
            .unwrap()
    );
    assert!(
        store
            .line_of_sight_internal(1, [8., 8., 10.], [8., 8., -10.], false, &mut b)
            .unwrap()
    );
}
#[test]
fn strict_endpoints_and_both_directions_remain_visible_at_t_zero_or_maximum() {
    let (store, _) = make(false, Options::default(), true);
    let mut b = budget();
    assert!(
        store
            .line_of_sight_internal(1, top(), [1., 1., 0.], false, &mut b)
            .unwrap()
    );
    assert!(
        store
            .line_of_sight_internal(1, [1., 1., 0.], bottom(), false, &mut b)
            .unwrap()
    );
    assert!(
        !store
            .line_of_sight_internal(1, bottom(), top(), false, &mut b)
            .unwrap()
    );
}
#[test]
fn m2_ignore_is_forwarded_through_the_map_tree() {
    let (store, _) = make(true, Options::default(), true);
    let mut b = budget();
    assert!(
        !store
            .line_of_sight_internal(1, top(), bottom(), false, &mut b)
            .unwrap()
    );
    assert!(
        store
            .line_of_sight_internal(1, top(), bottom(), true, &mut b)
            .unwrap()
    );
}
#[test]
fn nearest_height_has_strict_range_and_world_coordinates() {
    let (store, _) = make(false, Options::default(), true);
    let mut b = budget();
    assert_eq!(
        store.height_internal(1, top(), 20., &mut b).unwrap(),
        Some(0.)
    );
    assert_eq!(
        store.height_world(1, reflect(top()), 20., &mut b).unwrap(),
        Some(0.)
    );
    assert_eq!(store.height_internal(1, top(), 10., &mut b).unwrap(), None);
    assert_eq!(
        store.height_internal(1, bottom(), 20., &mut b).unwrap(),
        None
    );
}
#[test]
fn positive_infinite_height_limit_is_admitted_on_safe_leaf_map_and_mesh() {
    let (store, _) = make(false, Options::default(), true);
    assert_eq!(
        store
            .height_internal(1, top(), f32::INFINITY, &mut budget())
            .unwrap(),
        Some(0.)
    );
}
#[test]
fn object_hit_negative_modify_recomputes_magnitude_and_clamps_at_origin() {
    let (store, _) = make(false, Options::default(), true);
    let mut b = budget();
    let hit = store
        .object_hit_internal(1, top(), bottom(), -2., &mut b)
        .unwrap();
    assert!(hit.hit());
    assert_eq!(hit.position(), [1., 1., 2.]);
    assert_eq!(
        store
            .object_hit_internal(1, top(), bottom(), -10., &mut b)
            .unwrap()
            .position(),
        top()
    );
    assert_eq!(
        store
            .object_hit_internal(1, top(), bottom(), -20., &mut b)
            .unwrap()
            .position(),
        top()
    );
}
#[test]
fn positive_modify_can_move_past_destination_without_clamping() {
    let (store, _) = make(false, Options::default(), true);
    let hit = store
        .object_hit_internal(1, top(), bottom(), 20., &mut budget())
        .unwrap();
    assert!(hit.hit());
    assert_eq!(hit.position(), [1., 1., -20.]);
}
#[test]
fn loaded_world_miss_rounds_destination_twice_but_unloaded_preserves_it() {
    let (store, _) = make(false, Options::default(), true);
    let (unloaded, _) = make(false, Options::default(), false);
    let start = [0.1, 0.2, 10.];
    let end = [0.1, 0.2, -10.];
    let result = store
        .object_hit_world(1, start, end, 0., &mut budget())
        .unwrap();
    assert!(!result.hit());
    assert_eq!(result.position(), reflect(reflect(end)));
    assert_ne!(result.position_bits(), end.map(f32::to_bits));
    assert_eq!(
        unloaded
            .object_hit_world(1, start, end, 0., &mut budget())
            .unwrap()
            .position_bits(),
        end.map(f32::to_bits)
    );
}
#[test]
fn tiny_internal_segments_use_no_tree_or_triangle_budget() {
    let (store, _) = make(false, Options::default(), true);
    let mut b = Budget::new(query::Limits {
        max_node_visits: 0,
        max_triangle_tests: 0,
    })
    .unwrap();
    assert!(
        store
            .line_of_sight_internal(1, [1., 1., 0.], [1., 1., 5e-11], false, &mut b)
            .unwrap()
    );
    assert!(
        !store
            .object_hit_internal(1, [1., 1., 0.], [1., 1., 5e-11], 0., &mut b)
            .unwrap()
            .hit()
    );
    assert_eq!(b.stats(), query::Stats::default());
}
#[test]
fn overflow_los_is_false_and_object_segment_is_error_without_traversal() {
    let (store, _) = make(false, Options::default(), true);
    let a = [0., 0., f32::MAX];
    let z = [0., 0., -f32::MAX];
    let mut b = budget();
    assert!(
        !store
            .line_of_sight_internal(1, a, z, false, &mut b)
            .unwrap()
    );
    assert!(matches!(
        store.object_hit_internal(1, a, z, 0., &mut b),
        Err(Error::Query(query::Error::InvalidDistance))
    ));
    assert_eq!(b.stats(), query::Stats::default());
}
#[test]
fn global_switches_gate_queries_independently() {
    let (no_los, _) = make(
        false,
        Options {
            enable_line_of_sight: false,
            ..Options::default()
        },
        true,
    );
    assert!(
        no_los
            .line_of_sight_internal(1, top(), bottom(), false, &mut budget())
            .unwrap()
    );
    assert_eq!(
        no_los
            .height_internal(1, top(), 20., &mut budget())
            .unwrap(),
        Some(0.)
    );
    assert!(
        !no_los
            .object_hit_internal(1, top(), bottom(), 0., &mut budget())
            .unwrap()
            .hit()
    );
    let (no_height, _) = make(
        false,
        Options {
            enable_height: false,
            ..Options::default()
        },
        true,
    );
    assert_eq!(
        no_height
            .height_internal(1, top(), 20., &mut budget())
            .unwrap(),
        None
    );
    assert!(
        !no_height
            .line_of_sight_internal(1, top(), bottom(), false, &mut budget())
            .unwrap()
    );
}
#[test]
fn invalid_values_unknown_maps_and_poison_cannot_bypass_shortcuts() {
    let (mut store, changed) = make(false, Options::default(), true);
    assert!(matches!(
        store.line_of_sight_internal(2, top(), top(), false, &mut budget()),
        Err(Error::UnknownMap)
    ));
    assert!(store.height_internal(1, top(), -1., &mut budget()).is_err());
    assert!(
        store
            .height_internal(1, top(), f32::NAN, &mut budget())
            .is_err()
    );
    assert!(
        store
            .line_of_sight_internal(1, [f32::NAN, 0., 0.], top(), false, &mut budget())
            .is_err()
    );
    assert!(
        store
            .object_hit_internal(1, top(), bottom(), f32::INFINITY, &mut budget())
            .is_err()
    );
    changed.set(true);
    assert!(store.verify_source().is_err());
    assert!(matches!(
        store.line_of_sight_internal(1, top(), top(), false, &mut budget()),
        Err(Error::Poisoned)
    ));
    assert!(matches!(
        store.height_internal(1, top(), 0., &mut budget()),
        Err(Error::Poisoned)
    ));
}
#[test]
fn exhausted_query_work_is_error_instead_of_clear_los() {
    let (store, _) = make(false, Options::default(), true);
    let mut b = Budget::new(query::Limits {
        max_node_visits: 0,
        max_triangle_tests: 0,
    })
    .unwrap();
    assert!(matches!(
        store.line_of_sight_internal(1, top(), bottom(), false, &mut b),
        Err(Error::Query(query::Error::NodeLimit))
    ));
}
#[test]
fn unloading_last_tile_changes_actual_geometry_queries() {
    let (mut store, _) = make(false, Options::default(), true);
    assert!(
        !store
            .line_of_sight_internal(1, top(), bottom(), false, &mut budget())
            .unwrap()
    );
    assert!(store.unload_tile(1, 0, 0).unwrap());
    assert!(
        store
            .line_of_sight_internal(1, top(), bottom(), false, &mut budget())
            .unwrap()
    );
    assert_eq!(
        store.height_internal(1, top(), 20., &mut budget()).unwrap(),
        None
    );
    store.load_tile(1, 0, 0).unwrap();
    assert!(
        !store
            .line_of_sight_internal(1, top(), bottom(), false, &mut budget())
            .unwrap()
    );
}
