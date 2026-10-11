// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11. Synthetic source bytes and independent liquid goldens.
use super::*;
use crate::wmo::Limits;

#[derive(Default)]
struct Lookup {
    types: Vec<(u32, u8)>,
    materials: Vec<(u8, u32)>,
}
impl LiquidLookup for Lookup {
    fn material_id(&self, id: u32) -> Option<u8> {
        self.types
            .iter()
            .rev()
            .find(|entry| entry.0 == id)
            .map(|entry| entry.1)
    }
    fn material_flags(&self, id: u8) -> Option<u32> {
        self.materials
            .iter()
            .rev()
            .find(|entry| entry.0 == id)
            .map(|entry| entry.1)
    }
}
fn put(bytes: &mut [u8], at: usize, value: u32) {
    bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
}
fn chunk(tag: &[u8; 4], payload: &[u8]) -> Vec<u8> {
    let mut bytes: Vec<_> = tag.iter().rev().copied().collect();
    bytes.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    bytes.extend_from_slice(payload);
    bytes
}
fn root_bytes(flags: u16) -> Vec<u8> {
    let mut bytes = [0; 64];
    bytes[60..62].copy_from_slice(&flags.to_le_bytes());
    chunk(b"MOHD", &bytes)
}
fn liquid_bytes(tiles: &[u8]) -> Vec<u8> {
    let mut bytes = vec![0; 30];
    put(&mut bytes, 0, 2);
    put(&mut bytes, 4, 1);
    put(&mut bytes, 8, tiles.len() as u32);
    put(&mut bytes, 12, 1);
    put(&mut bytes, 16, (-0.0f32).to_bits());
    put(&mut bytes, 20, (-3.5f32).to_bits());
    put(&mut bytes, 24, 7.0f32.to_bits());
    bytes[28..30].copy_from_slice(&i16::MIN.to_le_bytes());
    for (a, b, height) in [
        (0x1234u16, 0x5678u16, 0x8000_0000u32),
        (0, 65535, (-2.25f32).to_bits()),
    ] {
        bytes.extend_from_slice(&a.to_le_bytes());
        bytes.extend_from_slice(&b.to_le_bytes());
        bytes.extend_from_slice(&height.to_le_bytes());
    }
    bytes.extend_from_slice(tiles);
    bytes
}
fn group_bytes(raw_type: u32, flags: u32, liquid: Option<&[u8]>) -> Vec<u8> {
    let mut bytes = vec![0; 68];
    put(&mut bytes, 8, flags);
    put(&mut bytes, 52, raw_type);
    if let Some(raw) = liquid {
        bytes.extend(chunk(b"MLIQ", raw));
    }
    chunk(b"MOGP", &bytes)
}
fn facts(
    root_flags: u16,
    group_flags: u32,
    raw_type: u32,
    tiles: Option<&[u8]>,
    lookup: &impl LiquidLookup,
) -> (u32, u32, bool) {
    let rb = root_bytes(root_flags);
    let lb = tiles.map(liquid_bytes);
    let gb = group_bytes(raw_type, group_flags, lb.as_deref());
    let root = Root::parse(&rb, Limits::default()).unwrap_or_else(|e| panic!("{e}"));
    let group = Group::parse(&gb, Limits::default()).unwrap_or_else(|e| panic!("{e}"));
    let result = project(&root, &group, lookup).unwrap_or_else(|e| panic!("{e}"));
    (result.flags(), result.type_id(), result.view().is_some())
}

#[test]
fn direct_header_types_use_legacy_table_only_for_one_through_twenty() {
    let golden = [
        0, 13, 14, 19, 20, 13, 14, 19, 20, 13, 14, 19, 20, 13, 14, 19, 20, 13, 14, 19, 20, 21,
    ];
    for (raw, expected) in golden.into_iter().enumerate() {
        assert_eq!(
            facts(4, 0, raw as u32, None, &Lookup::default()),
            (if expected == 0 { 0 } else { 2 }, expected, false)
        );
    }
    for raw in [22, 255, 256, 0xffff, 0x8000_0000, u32::MAX] {
        assert_eq!(facts(4, 0, raw, None, &Lookup::default()), (2, raw, false));
    }
}

#[test]
fn legacy_header_increments_with_explicit_fifteen_exception_and_u32_wrap() {
    let golden = [
        13, 14, 19, 20, 13, 14, 19, 20, 13, 14, 19, 20, 13, 14, 19, 0, 13, 14, 19, 20, 21, 22,
    ];
    for (raw, expected) in golden.into_iter().enumerate() {
        assert_eq!(
            facts(0, 0, raw as u32, None, &Lookup::default()),
            (if expected == 0 { 0 } else { 2 }, expected, false)
        );
    }
    assert_eq!(
        facts(0, 0, u32::MAX, None, &Lookup::default()),
        (0, 0, false)
    );
    assert_eq!(
        facts(0, 0, u32::MAX - 1, None, &Lookup::default()),
        (2, u32::MAX, false)
    );
}

#[test]
fn ocean_group_flag_changes_only_first_legacy_class() {
    for raw in 0..=255u32 {
        let plain = facts(4, 0, raw, None, &Lookup::default());
        let ocean = facts(4, 0x80000, raw, None, &Lookup::default());
        let expected = if [1, 5, 9, 13, 17].contains(&raw) {
            14
        } else {
            plain.1
        };
        assert_eq!(ocean, (plain.0, expected, false));
        assert_eq!(facts(4, 0x40000, raw, None, &Lookup::default()), plain);
    }
}

#[test]
fn every_tile_byte_uses_low_nibble_and_only_non_fifteen_is_fallback() {
    let golden = [
        13, 14, 19, 20, 13, 14, 19, 20, 13, 14, 19, 20, 13, 14, 19, 0,
    ];
    for byte in 0..=255u8 {
        assert_eq!(
            facts(4, 0, 0, Some(&[byte]), &Lookup::default()),
            (1, golden[usize::from(byte & 15)], true)
        );
    }
    assert_eq!(
        facts(4, 0x80000, 0, Some(&[0]), &Lookup::default()),
        (1, 14, true)
    );
}

#[test]
fn fallback_selects_first_non_fifteen_and_does_not_add_header_flag() {
    assert_eq!(
        facts(4, 0, 0, Some(&[0xff, 0x8f, 0xa2, 0, 1]), &Lookup::default()),
        (1, 19, true)
    );
    assert_eq!(
        facts(0, 0, 15, Some(&[0xff, 3]), &Lookup::default()),
        (1, 20, true)
    );
    assert_eq!(
        facts(0, 0, u32::MAX, Some(&[0xff, 1]), &Lookup::default()),
        (1, 14, true)
    );
}

#[test]
fn nonzero_header_type_prevents_tile_fallback() {
    assert_eq!(
        facts(4, 0, 77, Some(&[2]), &Lookup::default()),
        (3, 77, true)
    );
    assert_eq!(
        facts(0, 0, 0, Some(&[2]), &Lookup::default()),
        (3, 13, true)
    );
}

#[test]
fn empty_or_all_fifteen_tiles_preserve_zero_with_surface_flag() {
    for tiles in [b"".as_slice(), &[15, 31, 0x8f, 0xff]] {
        assert_eq!(
            facts(4, 0, 0, Some(tiles), &Lookup::default()),
            (1, 0, true)
        );
    }
}

#[test]
fn missing_type_or_material_is_not_visual_only_and_other_flags_do_not_filter() {
    let missing_type = Lookup {
        types: vec![],
        materials: vec![(255, 2)],
    };
    let missing_material = Lookup {
        types: vec![(77, 255)],
        materials: vec![],
    };
    let not_visual = Lookup {
        types: vec![(77, 255)],
        materials: vec![(255, 0xffff_fffd)],
    };
    for lookup in [missing_type, missing_material, not_visual] {
        assert_eq!(facts(4, 0, 77, Some(&[2]), &lookup), (3, 77, true));
        assert_eq!(facts(4, 0, 77, None, &lookup), (2, 77, false));
    }
}

#[test]
fn visual_only_suppresses_all_flags_but_keeps_mapped_type() {
    for material_flags in [2, 3, 0x8000_0002, u32::MAX] {
        let lookup = Lookup {
            types: vec![(13, 255)],
            materials: vec![(255, material_flags)],
        };
        assert_eq!(facts(4, 0, 1, Some(&[2]), &lookup), (0, 13, false));
        assert_eq!(facts(4, 0, 1, None, &lookup), (0, 13, false));
        assert_eq!(facts(4, 0, 0, Some(&[0]), &lookup), (0, 13, false));
    }
}

#[test]
fn zero_lookup_can_filter_surface_but_header_zero_does_not_consult_it() {
    let lookup = Lookup {
        types: vec![(0, 0)],
        materials: vec![(0, 2)],
    };
    assert_eq!(facts(4, 0, 0, Some(&[0xff]), &lookup), (0, 0, false));
    assert_eq!(facts(4, 0, 0, Some(&[]), &lookup), (0, 0, false));
    assert_eq!(facts(4, 0, 0, None, &lookup), (0, 0, false));
    assert_eq!(facts(4, 0, 0, Some(&[1]), &lookup), (1, 14, true));
}

#[test]
fn view_retains_complete_signed_material_position_height_and_tile_bits() {
    let rb = root_bytes(4);
    let lb = liquid_bytes(&[0x8f, 3]);
    let gb = group_bytes(77, 0, Some(&lb));
    let root = Root::parse(&rb, Limits::default()).unwrap_or_else(|e| panic!("{e}"));
    let group = Group::parse(&gb, Limits::default()).unwrap_or_else(|e| panic!("{e}"));
    let liquid = project(&root, &group, &Lookup::default()).unwrap_or_else(|e| panic!("{e}"));
    let view = liquid.view().unwrap();
    assert_eq!(view.bytes(), lb);
    assert_eq!(view.dimensions(), [2, 1, 2, 1]);
    assert_eq!(view.material(), i16::MIN);
    assert_eq!(
        view.position_bits(),
        [0x8000_0000, (-3.5f32).to_bits(), 7.0f32.to_bits()]
    );
    assert_eq!(view.vertex(0), Some((0x1234, 0x5678, 0x8000_0000)));
    assert_eq!(view.vertex(1), Some((0, 65535, (-2.25f32).to_bits())));
    assert_eq!(view.tiles(), [0x8f, 3]);
    assert_eq!(liquid.flags(), 3);
}

#[test]
fn lookup_sequence_matches_header_and_final_surface_checks() {
    use std::cell::RefCell;
    #[derive(Default)]
    struct Trace(RefCell<Vec<u32>>);
    impl LiquidLookup for Trace {
        fn material_id(&self, id: u32) -> Option<u8> {
            self.0.borrow_mut().push(id);
            Some(255)
        }
        fn material_flags(&self, id: u8) -> Option<u32> {
            self.0.borrow_mut().push(0x8000_0000 | u32::from(id));
            Some(0)
        }
    }
    let trace = Trace::default();
    assert_eq!(facts(4, 0, 77, Some(&[2]), &trace), (3, 77, true));
    assert_eq!(*trace.0.borrow(), [77, 0x8000_00ff, 77, 0x8000_00ff]);
    trace.0.borrow_mut().clear();
    assert_eq!(facts(4, 0, 0, None, &trace), (0, 0, false));
    assert!(trace.0.borrow().is_empty());
    assert_eq!(facts(4, 0, 0, Some(&[0xff]), &trace), (1, 0, true));
    assert_eq!(*trace.0.borrow(), [0, 0x8000_00ff]);
}

#[test]
fn invalid_or_over_limit_surface_is_rejected_by_borrowed_parser() {
    let mut bad = liquid_bytes(&[1]);
    put(&mut bad, 34, f32::NAN.to_bits());
    let gb = group_bytes(77, 0, Some(&bad));
    assert_eq!(
        Group::parse(&gb, Limits::default()).err(),
        Some(Error::NonFinite)
    );
    let good = liquid_bytes(&[1]);
    let gb = group_bytes(77, 0, Some(&good));
    assert_eq!(
        Group::parse(
            &gb,
            Limits {
                max_vertices: 1,
                ..Limits::default()
            }
        )
        .err(),
        Some(Error::Limit)
    );
    assert_eq!(
        Group::parse(
            &gb,
            Limits {
                max_metadata_records: 0,
                ..Limits::default()
            }
        )
        .err(),
        Some(Error::Limit)
    );
}
