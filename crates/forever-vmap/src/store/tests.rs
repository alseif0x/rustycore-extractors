// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: complete synthetic packages and transactional state tests.
use super::*;
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

#[derive(Default)]
struct State {
    files: BTreeMap<String, Vec<u8>>,
    reads: Vec<String>,
    verifies: usize,
    fail_verify: Option<usize>,
    read_error: Option<SourceError>,
    chunk: usize,
}
#[derive(Clone, Default)]
struct Fake(Rc<RefCell<State>>);
fn key(a: Artifact<'_>) -> String {
    match a {
        Artifact::Tree { map_id } => format!("{map_id}/tree"),
        Artifact::Tile { map_id, x, y } => format!("{map_id}/{x}/{y}/tile"),
        Artifact::Index { map_id, x, y } => format!("{map_id}/{x}/{y}/index"),
        Artifact::Model { name } => format!("model/{}", std::str::from_utf8(name).unwrap()),
    }
}
impl Source for Fake {
    fn read(&mut self, a: Artifact<'_>, out: &mut ReadBuffer<'_>) -> Result<Presence, SourceError> {
        let mut s = self.0.borrow_mut();
        let key = key(a);
        s.reads.push(key.clone());
        if let Some(e) = s.read_error.take() {
            return Err(e);
        }
        let Some(bytes) = s.files.get(&key) else {
            return Ok(Presence::Missing);
        };
        let chunk = if s.chunk == 0 {
            bytes.len().max(1)
        } else {
            s.chunk
        };
        for chunk in bytes.chunks(chunk) {
            out.append(chunk)?;
        }
        Ok(Presence::Present)
    }
    fn verify_unchanged(&mut self) -> Result<(), SourceError> {
        let mut s = self.0.borrow_mut();
        s.verifies += 1;
        if s.fail_verify == Some(s.verifies) {
            Err(SourceError::InputChanged)
        } else {
            Ok(())
        }
    }
}
fn word(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}
fn words(out: &mut Vec<u8>, v: impl IntoIterator<Item = u32>) {
    for v in v {
        word(out, v)
    }
}
fn row(id: u32, flags: u8, name: &[u8], bounds: [f32; 6]) -> Vec<u8> {
    let mut out = vec![flags, 7];
    word(&mut out, id);
    words(&mut out, [0u32; 6]);
    word(&mut out, 1f32.to_bits());
    words(&mut out, bounds.map(f32::to_bits));
    word(&mut out, name.len() as u32);
    out.extend_from_slice(name);
    out
}
fn bounds() -> [f32; 6] {
    [0., 0., -1., 1., 1., 1.]
}
fn m2() -> Vec<u8> {
    let mut out = crate::RAW_MAGIC.to_vec();
    words(&mut out, [3, 1, 0, 1, 0, 0]);
    words(&mut out, [-1f32, -1., -1., 2., 2., 2.].map(f32::to_bits));
    word(&mut out, 0);
    out.extend_from_slice(b"GRP ");
    words(&mut out, [8, 1, 3]);
    out.extend_from_slice(b"INDX");
    words(&mut out, [10, 3, 0, 1, 2]);
    out.extend_from_slice(b"VERT");
    words(&mut out, [40, 3]);
    words(
        &mut out,
        [0f32, 0., 0., 1., 0., 0., 0., 1., 0.].map(f32::to_bits),
    );
    let raw = crate::RawM2::parse(&out, crate::Limits::default()).unwrap();
    vmo::convert_m2(&raw, vmo::Limits::default())
        .unwrap()
        .into_bytes()
}
fn add_map(source: &Fake, id: u32, rows: &[u8]) {
    let input = placement::DirBin::parse(rows, placement::Limits::default()).unwrap();
    let placements: Vec<_> = input.records().collect();
    let map = assembly::build(id, &placements, assembly::Limits::default()).unwrap();
    let mut s = source.0.borrow_mut();
    s.files.insert(
        key(Artifact::Tree { map_id: id }),
        map.tree_bytes().to_vec(),
    );
    for tile in map.tiles() {
        let (x, y) = (tile.x() as u16, tile.y() as u8);
        s.files.insert(
            key(Artifact::Index { map_id: id, x, y }),
            tile.index_bytes().to_vec(),
        );
        if let Some(bytes) = tile.payload() {
            s.files
                .insert(key(Artifact::Tile { map_id: id, x, y }), bytes.to_vec());
        }
    }
}
fn parent(id: u32, p: Option<u32>) -> MapParent {
    MapParent {
        map_id: id,
        parent: p,
        cosmetic_parent: None,
    }
}
fn fixture() -> Fake {
    let source = Fake::default();
    add_map(&source, 1, &row(9, 1, b"model", bounds()));
    source
        .0
        .borrow_mut()
        .files
        .insert("model/model".into(), m2());
    source
}
fn store(source: Fake) -> StaticStore<Fake> {
    StaticStore::new(
        source,
        &[parent(1, None)],
        Options::default(),
        Limits::default(),
    )
    .unwrap()
}
#[test]
fn optional_absent_tree_is_verified_but_strict_stays_missing() {
    let f = Fake::default();
    let mut s = store(f.clone());
    assert_eq!(s.load_tile(1, 0, 0), Err(Error::MissingTree));
    assert_eq!(s.load_optional_tile(1, 0, 0), Ok(None));
    assert_eq!(s.stats().maps, 0);
    assert_eq!(f.0.borrow().verifies, 3);
}
#[test]
fn optional_empty_tree_is_absence() {
    let f = Fake::default();
    add_map(&f, 1, &[]);
    let mut s = store(f);
    assert_eq!(s.load_tile(1, 0, 0), Err(Error::EmptyTree));
    assert_eq!(s.load_optional_tile(1, 0, 0), Ok(None));
    assert_eq!(s.stats().tiles, 0);
}
#[test]
fn optional_missing_pair_initializes_only_tree() {
    let f = fixture();
    f.0.borrow_mut()
        .files
        .retain(|k, _| !k.ends_with("/tile") && !k.ends_with("/index"));
    let mut s = store(f);
    assert_eq!(s.load_tile(1, 0, 0), Err(Error::MissingIndex));
    assert_eq!(s.load_optional_tile(1, 0, 0), Ok(None));
    assert_eq!(s.stats().maps, 1);
    assert_eq!(s.stats().tiles, 0);
    assert_eq!(s.stats().active_instances, 0);
}
#[test]
fn optional_present_partial_pair_and_missing_model_stay_errors() {
    for (suffix, expected) in [
        ("/index", Error::MissingIndex),
        ("/tile", Error::MissingTile),
        ("model/model", Error::MissingModel),
    ] {
        let f = fixture();
        f.0.borrow_mut().files.retain(|k, _| !k.ends_with(suffix));
        let mut s = store(f);
        assert_eq!(s.load_optional_tile(1, 0, 0), Err(expected));
        assert_eq!(s.stats().maps, 0);
        assert_eq!(s.stats().active_instances, 0);
    }
}
#[test]
fn optional_absence_verification_poison_and_io_are_not_swallowed() {
    let f = Fake::default();
    f.0.borrow_mut().fail_verify = Some(2);
    let mut s = store(f);
    assert_eq!(
        s.load_optional_tile(1, 0, 0),
        Err(Error::Source(SourceError::InputChanged))
    );
    assert!(s.is_poisoned());
    assert_eq!(s.load_optional_tile(1, 0, 0), Err(Error::Poisoned));
    let f = Fake::default();
    f.0.borrow_mut().read_error = Some(SourceError::Io);
    assert_eq!(
        store(f).load_optional_tile(1, 0, 0),
        Err(Error::Source(SourceError::Io))
    );
}
fn ray() -> query::Ray {
    query::Ray::new([0.25, 0.25, 1.], [0., 0., -1.]).unwrap()
}
fn budget() -> query::Budget {
    query::Budget::new(query::Limits::default()).unwrap()
}
fn hit(s: &StaticStore<Fake>, map: u32) -> Result<Option<StoreHit>, Error> {
    s.intersect(map, &ray(), 2., query::Mode::Nearest, false, &mut budget())
}
fn logical(s: &StaticStore<Fake>) -> (usize, usize, usize, usize, usize, usize) {
    let s = s.stats();
    (
        s.maps,
        s.tiles,
        s.models,
        s.primitive_slots,
        s.active_instances,
        s.tile_references,
    )
}

#[test]
fn complete_package_load_queries_and_unloads() {
    let source = fixture();
    let mut s = store(source.clone());
    assert!(hit(&s, 1).unwrap().is_none());
    let r = s.load_tile(1, 0, 0).unwrap();
    assert_eq!(r.references_added, 1);
    assert_eq!(r.used_map_id, 1);
    assert_eq!(logical(&s), (1, 1, 1, 1, 1, 1));
    assert_eq!(s.model_references(b"model"), Some(1));
    let reads = source.0.borrow().reads.len();
    let h = hit(&s, 1).unwrap().unwrap();
    assert_eq!(
        (h.spawn_id(), h.slot(), h.hit().distance_bits()),
        (9, 0, 1f32.to_bits())
    );
    assert_eq!(source.0.borrow().reads.len(), reads);
    assert!(s.unload_tile(1, 0, 0).unwrap());
    assert_eq!(logical(&s), (0, 0, 0, 0, 0, 0));
    assert!(hit(&s, 1).unwrap().is_none());
    assert!(!s.unload_tile(1, 0, 0).unwrap());
}
#[test]
fn repeated_load_appends_references_and_one_unload_removes_all() {
    let source = fixture();
    let mut s = store(source.clone());
    s.load_tile(1, 0, 0).unwrap();
    let r = s.load_tile(1, 0, 0).unwrap();
    assert!(r.repeated);
    assert_eq!(r.new_models, 0);
    assert_eq!(logical(&s), (1, 1, 1, 1, 1, 2));
    assert_eq!(s.model_references(b"model"), Some(1));
    assert_eq!(
        source
            .0
            .borrow()
            .reads
            .iter()
            .filter(|v| *v == "model/model")
            .count(),
        1
    );
    s.unload_tile(1, 0, 0).unwrap();
    assert_eq!(logical(&s), (0, 0, 0, 0, 0, 0));
}
#[test]
fn same_model_is_shared_between_maps_and_released_after_last_slot() {
    let source = fixture();
    add_map(&source, 2, &row(12, 1, b"model", bounds()));
    let mut s = StaticStore::new(
        source.clone(),
        &[parent(1, None), parent(2, None)],
        Options::default(),
        Limits::default(),
    )
    .unwrap();
    s.load_tile(1, 0, 0).unwrap();
    s.load_tile(2, 0, 0).unwrap();
    assert_eq!(s.model_references(b"model"), Some(2));
    assert_eq!(
        source
            .0
            .borrow()
            .reads
            .iter()
            .filter(|v| *v == "model/model")
            .count(),
        1
    );
    s.unload_map(1).unwrap();
    assert_eq!(s.model_references(b"model"), Some(1));
    assert!(hit(&s, 2).unwrap().is_some());
    s.unload_map(2).unwrap();
    assert_eq!(s.stats().models, 0);
}
#[test]
fn freed_model_slots_are_reused_under_one_model_cap() {
    let mut s = StaticStore::new(
        fixture(),
        &[parent(1, None)],
        Options::default(),
        Limits {
            max_models: 1,
            ..Default::default()
        },
    )
    .unwrap();
    for _ in 0..20 {
        s.load_tile(1, 0, 0).unwrap();
        s.unload_map(1).unwrap();
        assert_eq!(s.core.models.len(), 1);
    }
}
#[test]
fn parent_payload_uses_child_index_and_does_not_load_parent_tree() {
    let source = fixture();
    add_map(&source, 2, &row(9, 3, b"model", bounds()));
    let mut s = StaticStore::new(
        source.clone(),
        &[parent(1, None), parent(2, Some(1))],
        Options::default(),
        Limits::default(),
    )
    .unwrap();
    assert_eq!(s.load_tile(2, 0, 0).unwrap().used_map_id, 1);
    assert!(hit(&s, 2).unwrap().is_some());
    assert!(!s.contains_map(1));
    assert!(!source.0.borrow().reads.iter().any(|r| r == "1/tree"));
}
#[test]
fn mixed_and_inherited_parent_flag_bindings_work_in_both_orders() {
    for first in [0, 1] {
        let source = Fake::default();
        let wide = [0., 0., -1., 600., 1., 1.];
        add_map(&source, 1, &row(10, 1, b"model", wide));
        add_map(
            &source,
            2,
            &[row(10, 3, b"model", wide), row(20, 1, b"model", bounds())].concat(),
        );
        source
            .0
            .borrow_mut()
            .files
            .insert("model/model".into(), m2());
        let mut s = StaticStore::new(
            source,
            &[parent(1, None), parent(2, Some(1))],
            Options::default(),
            Limits::default(),
        )
        .unwrap();
        s.load_tile(2, first, 0).unwrap();
        s.load_tile(2, 1 - first, 0).unwrap();
        let binding = s.core.maps[0].slots[0].binding.as_ref().unwrap();
        assert_eq!(binding.record[0], if first == 0 { 3 } else { 1 });
        assert_eq!(binding.references, 2);
        assert_eq!(s.stats().active_instances, 2);
    }
}
#[test]
fn first_present_corrupt_payload_never_falls_through() {
    let source = fixture();
    add_map(&source, 2, &row(9, 3, b"model", bounds()));
    add_map(&source, 3, &row(9, 3, b"model", bounds()));
    source
        .0
        .borrow_mut()
        .files
        .insert("2/0/0/tile".into(), vec![0]);
    let mut s = StaticStore::new(
        source.clone(),
        &[parent(1, None), parent(2, Some(1)), parent(3, Some(2))],
        Options::default(),
        Limits::default(),
    )
    .unwrap();
    assert!(matches!(s.load_tile(3, 0, 0), Err(Error::Assembly(_))));
    assert!(!source.0.borrow().reads.iter().any(|r| r == "1/0/0/tile"));
    assert_eq!(s.stats().maps, 0);
}
#[test]
fn missing_child_index_never_uses_parent_index() {
    let source = fixture();
    add_map(&source, 2, &row(9, 3, b"model", bounds()));
    source.0.borrow_mut().files.remove("2/0/0/index");
    let mut s = StaticStore::new(
        source,
        &[parent(1, None), parent(2, Some(1))],
        Options::default(),
        Limits::default(),
    )
    .unwrap();
    assert_eq!(s.load_tile(2, 0, 0), Err(Error::MissingIndex));
    assert_eq!(s.stats().models, 0);
}
#[test]
fn empty_tree_init_valid_but_tile_load_rejects_without_partial_state() {
    let source = Fake::default();
    add_map(&source, 1, &[]);
    let mut s = store(source);
    s.init_map(1).unwrap();
    assert_eq!(s.stats().maps, 1);
    assert!(hit(&s, 1).unwrap().is_none());
    assert_eq!(s.load_tile(1, 0, 0), Err(Error::EmptyTree));
    assert_eq!(s.stats().maps, 1);
    assert!(s.unload_map(1).unwrap());
}
#[test]
fn missing_late_model_rolls_back_all_new_models_and_map() {
    let source = fixture();
    add_map(
        &source,
        1,
        &[
            row(9, 1, b"model", bounds()),
            row(10, 1, b"missing", bounds()),
        ]
        .concat(),
    );
    let mut s = store(source);
    let resident = s.stats().resident_bytes;
    assert_eq!(s.load_tile(1, 0, 0), Err(Error::MissingModel));
    assert_eq!(logical(&s), (0, 0, 0, 0, 0, 0));
    assert_eq!(s.stats().resident_bytes, resident);
    assert!(!s.is_poisoned());
}
#[test]
fn invalid_late_model_preserves_previously_loaded_map() {
    let source = fixture();
    add_map(&source, 2, &row(20, 1, b"bad", bounds()));
    source
        .0
        .borrow_mut()
        .files
        .insert("model/bad".into(), vec![0]);
    let mut s = StaticStore::new(
        source,
        &[parent(1, None), parent(2, None)],
        Options::default(),
        Limits::default(),
    )
    .unwrap();
    s.load_tile(1, 0, 0).unwrap();
    let before = logical(&s);
    let bytes = s.stats().resident_bytes;
    assert!(matches!(s.load_tile(2, 0, 0), Err(Error::Model(_))));
    assert_eq!(logical(&s), before);
    assert_eq!(s.stats().resident_bytes, bytes);
    assert!(hit(&s, 1).unwrap().is_some());
}
#[test]
fn final_source_change_rolls_back_and_poison_is_sticky() {
    let source = fixture();
    source.0.borrow_mut().fail_verify = Some(2);
    let mut s = store(source);
    assert_eq!(
        s.load_tile(1, 0, 0),
        Err(Error::Source(SourceError::InputChanged))
    );
    assert_eq!(s.stats().maps, 0);
    assert!(s.is_poisoned());
    assert!(matches!(hit(&s, 1), Err(Error::Poisoned)));
    assert_eq!(s.unload_map(1), Err(Error::Poisoned));
    assert_eq!(s.load_tile(1, 0, 0), Err(Error::Poisoned));
}
#[test]
fn cached_queries_do_not_recheck_source_until_explicit_verify() {
    let source = fixture();
    let mut s = store(source.clone());
    s.load_tile(1, 0, 0).unwrap();
    {
        let mut st = source.0.borrow_mut();
        st.fail_verify = Some(st.verifies + 1);
    }
    assert!(hit(&s, 1).unwrap().is_some());
    assert!(s.verify_source().is_err());
    assert!(matches!(hit(&s, 1), Err(Error::Poisoned)));
}
#[test]
fn ordinary_io_failure_can_retry_without_poison() {
    let source = fixture();
    source.0.borrow_mut().read_error = Some(SourceError::Io);
    let mut s = store(source);
    assert_eq!(s.load_tile(1, 0, 0), Err(Error::Source(SourceError::Io)));
    assert!(!s.is_poisoned());
    s.load_tile(1, 0, 0).unwrap();
    assert!(hit(&s, 1).unwrap().is_some());
}
#[test]
fn input_changed_during_read_poisoned_without_commit() {
    let source = fixture();
    source.0.borrow_mut().read_error = Some(SourceError::InputChanged);
    let mut s = store(source);
    assert_eq!(
        s.load_tile(1, 0, 0),
        Err(Error::Source(SourceError::InputChanged))
    );
    assert!(s.is_poisoned());
    assert_eq!(s.stats().maps, 0);
}
#[test]
fn path_only_skip_avoids_model_acquisition_and_references() {
    let source = Fake::default();
    add_map(&source, 1, &row(9, 5, b"missing", bounds()));
    let mut s = store(source.clone());
    s.load_tile(1, 0, 0).unwrap();
    assert_eq!(logical(&s), (1, 1, 0, 1, 0, 0));
    assert!(hit(&s, 1).unwrap().is_none());
    assert!(
        !source
            .0
            .borrow()
            .reads
            .iter()
            .any(|r| r.starts_with("model/"))
    );
    let mut enabled = StaticStore::new(
        source,
        &[parent(1, None)],
        Options {
            load_path_only_models: true,
            ..Default::default()
        },
        Limits::default(),
    )
    .unwrap();
    assert_eq!(enabled.load_tile(1, 0, 0), Err(Error::MissingModel));
}
#[test]
fn path_only_enabled_queries_geometry() {
    let source = fixture();
    add_map(&source, 1, &row(9, 5, b"model", bounds()));
    let mut s = StaticStore::new(
        source,
        &[parent(1, None)],
        Options {
            load_path_only_models: true,
            ..Default::default()
        },
        Limits::default(),
    )
    .unwrap();
    s.load_tile(1, 0, 0).unwrap();
    assert!(hit(&s, 1).unwrap().is_some());
}
#[test]
fn conflicting_slot_identity_and_flags_fail_without_appending_refs() {
    for offset in [12 + 1, 12 + 2, 12 + 18] {
        let source = fixture();
        add_map(
            &source,
            1,
            &row(9, 1, b"model", [0., 0., -1., 600., 1., 1.]),
        );
        source.0.borrow_mut().files.get_mut("1/1/0/tile").unwrap()[offset] ^= 1;
        let mut s = store(source);
        s.load_tile(1, 0, 0).unwrap();
        let before = logical(&s);
        assert!(s.load_tile(1, 1, 0).is_err());
        assert_eq!(logical(&s), before);
    }
}
#[test]
fn swapped_index_ordinals_reject_even_inside_same_bih_leaf() {
    let source = fixture();
    add_map(
        &source,
        1,
        &[
            row(9, 1, b"model", bounds()),
            row(10, 1, b"model", bounds()),
        ]
        .concat(),
    );
    {
        let mut s = source.0.borrow_mut();
        let b = s.files.get_mut("1/0/0/index").unwrap();
        b[12..16].copy_from_slice(&1u32.to_le_bytes());
        b[16..20].copy_from_slice(&0u32.to_le_bytes());
    }
    let mut s = store(source);
    assert_eq!(s.load_tile(1, 0, 0), Err(Error::Association));
    assert_eq!(s.stats().maps, 0);
}
#[test]
fn payload_bounds_outside_intrinsic_tree_reject() {
    let source = fixture();
    source.0.borrow_mut().files.get_mut("1/0/0/tile").unwrap()[12 + 46..12 + 50]
        .copy_from_slice(&100f32.to_bits().to_le_bytes());
    let mut s = store(source);
    assert_eq!(s.load_tile(1, 0, 0), Err(Error::Association));
}
#[test]
fn malformed_index_count_duplicates_and_out_of_range_reject() {
    for value in [1, u32::MAX] {
        let source = fixture();
        source.0.borrow_mut().files.get_mut("1/0/0/index").unwrap()[12..16]
            .copy_from_slice(&value.to_le_bytes());
        assert!(store(source).load_tile(1, 0, 0).is_err());
    }
    let source = fixture();
    source
        .0
        .borrow_mut()
        .files
        .get_mut("1/0/0/index")
        .unwrap()
        .push(0);
    assert!(store(source).load_tile(1, 0, 0).is_err());
}
#[test]
fn singular_instance_rejects_before_model_acquisition() {
    let source = fixture();
    source.0.borrow_mut().files.get_mut("1/0/0/tile").unwrap()[42..46]
        .copy_from_slice(&0u32.to_le_bytes());
    let mut s = store(source.clone());
    assert_eq!(
        s.load_tile(1, 0, 0),
        Err(Error::Query(query::Error::InvalidScale))
    );
    assert!(
        !source
            .0
            .borrow()
            .reads
            .iter()
            .any(|r| r.starts_with("model/"))
    );
}
#[test]
fn unsafe_names_reject_before_model_lookup() {
    for name in [
        b"../x".as_slice(),
        b"a/b",
        b"a\\b",
        b".",
        b"..",
        b"a:b",
        b"\x01",
    ] {
        let source = Fake::default();
        add_map(&source, 1, &row(9, 1, name, bounds()));
        assert_eq!(store(source).load_tile(1, 0, 0), Err(Error::Name));
    }
}
#[test]
fn topology_duplicate_missing_cycle_pair_and_max_sentinel_reject() {
    let bad = [
        vec![parent(1, None), parent(1, None)],
        vec![parent(1, Some(2))],
        vec![parent(1, Some(1))],
        vec![parent(1, Some(2)), parent(2, Some(1))],
        vec![
            MapParent {
                map_id: 1,
                parent: Some(2),
                cosmetic_parent: Some(3),
            },
            parent(2, None),
            parent(3, None),
        ],
        vec![parent(1, Some(u32::MAX)), parent(u32::MAX, None)],
    ];
    for rows in bad {
        assert!(matches!(
            StaticStore::new(
                Fake::default(),
                &rows,
                Options::default(),
                Limits::default()
            ),
            Err(Error::Topology)
        ));
    }
}
#[test]
fn cosmetic_parent_and_high_bit_non_sentinel_are_admitted() {
    let rows = [
        MapParent {
            map_id: 1,
            parent: None,
            cosmetic_parent: Some(0x8000_0000),
        },
        parent(0x8000_0000, None),
    ];
    let s = StaticStore::new(
        Fake::default(),
        &rows,
        Options::default(),
        Limits::default(),
    )
    .unwrap();
    assert_eq!(s.core.parent(1).unwrap(), Some(0x8000_0000));
}
#[test]
fn disabled_and_unknown_map_fail_before_source_access() {
    let source = fixture();
    let mut s = StaticStore::new(
        source.clone(),
        &[parent(1, None)],
        Options {
            enable_line_of_sight: false,
            enable_height: false,
            ..Default::default()
        },
        Limits::default(),
    )
    .unwrap();
    assert_eq!(s.init_map(1), Err(Error::Disabled));
    assert_eq!(s.load_tile(1, 0, 0), Err(Error::Disabled));
    assert_eq!(s.load_tile(2, 0, 0), Err(Error::UnknownMap));
    assert!(matches!(hit(&s, 2), Err(Error::UnknownMap)));
    assert_eq!(s.load_tile(1, 32768, 0), Err(Error::Coordinates));
    assert!(source.0.borrow().reads.is_empty());
}
#[test]
fn reducible_file_work_read_slot_model_and_reference_caps() {
    for limits in [
        Limits {
            max_file_bytes: 0,
            ..Default::default()
        },
        Limits {
            max_load_work: 0,
            ..Default::default()
        },
        Limits {
            max_load_bytes: 0,
            ..Default::default()
        },
        Limits {
            max_slots: 0,
            ..Default::default()
        },
        Limits {
            max_models: 0,
            ..Default::default()
        },
        Limits {
            max_references: 0,
            ..Default::default()
        },
        Limits {
            max_tiles: 0,
            ..Default::default()
        },
    ] {
        let mut s =
            StaticStore::new(fixture(), &[parent(1, None)], Options::default(), limits).unwrap();
        assert!(s.load_tile(1, 0, 0).is_err());
        assert_eq!(s.stats().maps, 0);
    }
    assert!(matches!(
        StaticStore::new(
            Fake::default(),
            &[],
            Options::default(),
            Limits {
                max_file_bytes: MAX_FILE_BYTES + 1,
                ..Default::default()
            }
        ),
        Err(Error::InvalidLimits)
    ));
}
#[test]
fn repeated_reference_limit_rejects_atomically() {
    let mut s = StaticStore::new(
        fixture(),
        &[parent(1, None)],
        Options::default(),
        Limits {
            max_references: 1,
            ..Default::default()
        },
    )
    .unwrap();
    s.load_tile(1, 0, 0).unwrap();
    let before = logical(&s);
    assert_eq!(s.load_tile(1, 0, 0), Err(Error::Limit));
    assert_eq!(logical(&s), before);
}
#[test]
fn memory_peak_exact_boundary_and_one_less() {
    let mut s = store(fixture());
    s.load_tile(1, 0, 0).unwrap();
    let peak = s.stats().peak_bytes;
    let mut exact = StaticStore::new(
        fixture(),
        &[parent(1, None)],
        Options::default(),
        Limits {
            max_memory_bytes: peak,
            ..Default::default()
        },
    )
    .unwrap();
    exact.load_tile(1, 0, 0).unwrap();
    assert!(exact.stats().resident_bytes <= peak);
    assert_eq!(exact.stats().peak_bytes, peak);
    let mut short = StaticStore::new(
        fixture(),
        &[parent(1, None)],
        Options::default(),
        Limits {
            max_memory_bytes: peak - 1,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(short.load_tile(1, 0, 0).is_err());
    assert_eq!(short.stats().maps, 0);
}
#[test]
fn counters_overflow_rejects_before_read_or_commit() {
    let source = fixture();
    let mut s = store(source.clone());
    s.counters.source_reads = u64::MAX;
    assert_eq!(s.load_tile(1, 0, 0), Err(Error::Limit));
    assert!(source.0.borrow().reads.is_empty());
}
#[test]
fn query_budget_and_ignore_are_shared_with_map_traversal() {
    let mut s = store(fixture());
    s.load_tile(1, 0, 0).unwrap();
    let mut zero = query::Budget::new(query::Limits {
        max_node_visits: 0,
        ..Default::default()
    })
    .unwrap();
    assert!(matches!(
        s.intersect(1, &ray(), 2., query::Mode::Any, false, &mut zero),
        Err(Error::Query(query::Error::NodeLimit))
    ));
    let mut budget = budget();
    assert!(
        s.intersect(1, &ray(), 2., query::Mode::Any, true, &mut budget)
            .unwrap()
            .is_none()
    );
    assert_eq!(budget.stats().triangle_tests, 0);
    assert!(budget.stats().node_visits > 0);
}

#[test]
fn loaded_models_borrow_cached_geometry_without_reads_and_repeat_once() {
    let source = fixture();
    let mut s = store(source.clone());
    assert_eq!(s.loaded_models(1).unwrap().count(), 0);
    assert!(matches!(s.loaded_models(99), Err(Error::UnknownMap)));
    s.load_tile(1, 0, 0).unwrap();
    s.load_tile(1, 0, 0).unwrap();
    let before = source.0.borrow().reads.len();
    let mut it = s.loaded_models(1).unwrap();
    let copy = it.clone();
    let m = it.next().unwrap();
    assert_eq!((m.slot(), m.placement().id()), (0, 9));
    assert_eq!(m.placement().name(), b"model");
    assert_eq!(m.geometry().group_count(), 1);
    assert_eq!(m.transform().scale(), 1.);
    assert!(it.next().is_none());
    assert!(it.next().is_none());
    assert!(std::ptr::eq(
        copy.into_iter().next().unwrap().geometry(),
        m.geometry()
    ));
    assert_eq!(source.0.borrow().reads.len(), before);
    s.unload_tile(1, 0, 0).unwrap();
    assert_eq!(s.loaded_models(1).unwrap().count(), 0);
}
#[test]
fn loaded_models_skips_path_only_null_and_rejects_poisoned_store() {
    let source = fixture();
    add_map(&source, 1, &row(9, 5, b"missing", bounds()));
    let mut s = store(source.clone());
    s.load_tile(1, 0, 0).unwrap();
    assert_eq!(s.loaded_models(1).unwrap().count(), 0);
    let next = source.0.borrow().verifies + 1;
    source.0.borrow_mut().fail_verify = Some(next);
    assert!(s.verify_source().is_err());
    assert!(matches!(s.loaded_models(1), Err(Error::Poisoned)));
}

// Modified 2026-10-20: synthetic model-table growth regression.
#[test]
fn many_unique_models_grow_tables_only_when_full() {
    const MEMORY_CAP: usize = 512 * 1024; // Constrained budget exposes unnecessary table growth.
    let source = Fake::default();
    let model_bytes = m2();
    let mut rows = Vec::new();
    for id in 1..=24 {
        let name = format!("model-{id:02}");
        rows.push(row(id, 1, name.as_bytes(), bounds()));
        source
            .0
            .borrow_mut()
            .files
            .insert(format!("model/{name}"), model_bytes.clone());
    }
    add_map(&source, 1, &rows.concat());

    let mut store = StaticStore::new(
        source,
        &[parent(1, None)],
        Options::default(),
        Limits {
            max_memory_bytes: MEMORY_CAP,
            ..Default::default()
        },
    )
    .unwrap();
    let report = store.load_tile(1, 0, 0).unwrap();

    assert_eq!(report.new_models, 24);
    assert_eq!(store.stats().models, 24);
    assert!(store.stats().peak_bytes <= MEMORY_CAP);
}
