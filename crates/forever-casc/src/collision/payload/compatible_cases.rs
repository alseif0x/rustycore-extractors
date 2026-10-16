// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-16. Synthetic codec cases; unrun in this proposal.
use super::*;
fn model(id: u32) -> Model {
    let p = crate::collision::names::Prepared::new(InputRef::FileDataId(id), Class::M2).unwrap();
    Model {
        class: Class::M2,
        name: p.name().to_vec(),
        origin: Origin::Id(id),
        pending: false,
        status: Status::Collision,
        raw: None,
        source: Some(SourceFacts {
            id,
            key: [0; 16],
            bytes: 304,
            first_magic: Some(*b"MD20"),
        }),
        wmo: None,
    }
}
fn frame(bindings: &[(u32, u32, u64)], refs: &[u16]) -> Vec<u8> {
    let root = root(&[30], &[20, 21], true);
    let mut b = vec![0; HEADER];
    b[..8].copy_from_slice(COMPATIBLE_MAGIC);
    b[8..12].copy_from_slice(&2u32.to_le_bytes());
    for (slot, value) in b[12..].as_chunks_mut::<8>().0.iter_mut().zip([
        0,
        root.len(),
        bindings.len(),
        refs.len(),
        1,
    ]) {
        slot.copy_from_slice(&(value as u64).to_le_bytes());
    }
    b.extend(root);
    for (name, tag, h) in bindings {
        b.extend(name.to_le_bytes());
        b.extend(tag.to_le_bytes());
        b.extend(h.to_le_bytes());
    }
    for r in refs {
        b.extend(r.to_le_bytes());
    }
    b.extend(30u32.to_le_bytes());
    b.extend([0; 16]);
    b.extend(12u64.to_le_bytes());
    b.push(1);
    b.extend(*b"PGOM");
    b
}
fn accepted(b: &[u8], models: &[Model]) -> bool {
    Payload::parse_profile(b, wmo::Limits::default(), true)
        .and_then(|p| CompatiblePayload::validate(p, models))
        .is_ok()
}
#[test]
fn compatible_descriptor_origin_preserves_file_data_id_route() {
    let model = model(20);
    let descriptor = Descriptor { model: &model };
    assert!(matches!(descriptor.origin(), InputRef::FileDataId(20)));
    assert_eq!(descriptor.source().unwrap().file_data_id(), 20);
    assert_eq!(descriptor.output_basename(), model.name);
}
#[test]
fn compatible_descriptor_origin_borrows_canonical_name_without_using_source_id() {
    let prepared = crate::collision::names::Prepared::new(
        InputRef::LogicalName(b"World\\Fixture\\Child.M2"),
        Class::M2,
    )
    .unwrap();
    let mut model = model(20);
    model.origin = Origin::Name(prepared.canonical().to_vec());
    model.name = prepared.name().to_vec();
    let descriptor = Descriptor { model: &model };
    let InputRef::LogicalName(name) = descriptor.origin() else {
        panic!("named origin must remain a logical name");
    };
    let Origin::Name(retained) = &model.origin else {
        unreachable!();
    };
    assert_eq!(name, prepared.canonical());
    assert!(std::ptr::eq(name.as_ptr(), retained.as_ptr()));
    assert_eq!(descriptor.source().unwrap().file_data_id(), 20);
    assert_eq!(descriptor.output_basename(), prepared.name());
}
#[test]
fn compatible_payload_positive_missing_binding_preserves_full_order() {
    let b = frame(&[(0, 1, 0), (1, 0, 1)], &[]);
    let models = [model(20), model(21)];
    let p = CompatiblePayload::validate(
        Payload::parse_profile(&b, wmo::Limits::default(), true).unwrap(),
        &models,
    )
    .unwrap();
    let w = p.wmo().unwrap();
    let bindings: Vec<_> = w.bindings().collect();
    assert_eq!(bindings.len(), 2);
    assert_eq!(
        bindings[0].disposition,
        crate::collision::compatible::Disposition::Missing
    );
    assert_eq!(
        bindings[1].disposition,
        crate::collision::compatible::Disposition::Present(ModelHandle(1))
    );
}
#[test]
fn compatible_payload_rejects_missing_duplicate_extra_and_out_of_order_bindings() {
    for bindings in [
        vec![(0, 0, 0)],
        vec![(0, 0, 0), (0, 0, 1)],
        vec![(0, 0, 0), (1, 0, 1), (2, 1, 0)],
        vec![(1, 0, 1), (0, 0, 0)],
    ] {
        assert!(!accepted(&frame(&bindings, &[]), &[model(20), model(21)]));
    }
}
#[test]
fn compatible_payload_rejects_wrong_origin_class_and_dangling_handles() {
    let b = frame(&[(0, 0, 0), (1, 0, 1)], &[]);
    assert!(!accepted(&b, &[model(22), model(21)]));
    let mut wrong = model(20);
    wrong.class = Class::Wmo;
    assert!(!accepted(&b, &[wrong, model(21)]));
    assert!(!accepted(&b, &[model(20)]));
}
#[test]
fn compatible_payload_rejects_selected_missing_and_no_collision_models() {
    assert!(!accepted(
        &frame(&[(0, 1, 0), (1, 0, 1)], &[0]),
        &[model(20), model(21)]
    ));
    let mut empty = model(20);
    empty.status = Status::NoCollision;
    assert!(!accepted(
        &frame(&[(0, 0, 0), (1, 0, 1)], &[0]),
        &[empty, model(21)]
    ));
}
#[test]
fn compatible_payload_profile_never_enters_strict_reader() {
    let mut b = frame(&[(0, 0, 0), (1, 0, 1)], &[]);
    assert!(Payload::parse(&b, wmo::Limits::default()).is_err());
    b[..8].copy_from_slice(MAGIC);
    assert!(Payload::parse_profile(&b, wmo::Limits::default(), true).is_err());
}
#[test]
fn compatible_payload_rejects_noncanonical_missing_tag_group_and_eof() {
    assert!(!accepted(
        &frame(&[(0, 1, 1), (1, 0, 1)], &[]),
        &[model(20), model(21)]
    ));
    assert!(!accepted(
        &frame(&[(0, 2, 0), (1, 0, 1)], &[]),
        &[model(20), model(21)]
    ));
    let mut b = frame(&[(0, 0, 0), (1, 0, 1)], &[]);
    let at = b.len() - GROUP;
    b[at..at + 4].copy_from_slice(&31u32.to_le_bytes());
    assert!(!accepted(&b, &[model(20), model(21)]));
    let mut b = frame(&[(0, 0, 0), (1, 0, 1)], &[]);
    b.push(0);
    assert!(!accepted(&b, &[model(20), model(21)]));
    b.truncate(b.len() - 2);
    assert!(!accepted(&b, &[model(20), model(21)]));
}
#[test]
fn compatible_payload_metadata_caps_remain_downward_only() {
    let b = frame(&[(0, 0, 0), (1, 0, 1)], &[]);
    assert!(
        Payload::parse_profile(
            &b,
            wmo::Limits {
                max_metadata_records: 1,
                ..wmo::Limits::default()
            },
            true
        )
        .is_err()
    );
}

#[test]
fn compatible_payload_missing_binding_rejects_invalid_sentinel_identity() {
    let mut b = frame(&[(0, 1, 0), (1, 0, 1)], &[]);
    // Authored root: MOHD72 + GFID12 + MODI header8 precede its first ID.
    b[HEADER + 92..HEADER + 96].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(!accepted(&b, &[model(20), model(21)]));
}
