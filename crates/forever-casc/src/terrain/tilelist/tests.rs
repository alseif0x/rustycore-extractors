// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: independent literal/native-order tile-list checks.
use super::*;
#[test]
fn header_and_reversed_bitset_order_are_exact() {
    for build in [70170, 70205] {
        let mut list = TileList::new(build).unwrap();
        for (x, y) in [(0, 0), (63, 63), (3, 2)] {
            list.set(x, y).unwrap();
        }
        assert_eq!(&list.bytes()[..8], b"MAPS\x0a\0\0\0");
        assert_eq!(&list.bytes()[8..12], &build.to_le_bytes());
        assert_eq!(list.bytes()[12], b'1');
        assert_eq!(list.bytes()[4107], b'1');
        assert_eq!(list.bytes()[3976], b'1');
        assert_eq!(list.bytes()[12..].iter().filter(|b| **b == b'1').count(), 3);
        assert!(present(list.bytes(), build, 3, 2).unwrap());
        assert!(!present(list.bytes(), build, 2, 3).unwrap());
    }
}
#[test]
fn all_coordinates_zero_full_idempotency_and_bounds() {
    let mut list = TileList::new(70205).unwrap();
    assert!(list.bytes()[12..].iter().all(|b| *b == b'0'));
    for y in 0..64 {
        for x in 0..64 {
            list.set(x, y).unwrap();
            list.set(x, y).unwrap();
        }
    }
    assert!(list.bytes()[12..].iter().all(|b| *b == b'1'));
    assert_eq!(list.set(64, 0), Err(Error::Coordinates));
    assert_eq!(list.set(0, 255), Err(Error::Coordinates));
    assert!(TileList::new(51943).is_err());
}
#[test]
fn malformed_lengths_builds_headers_and_non_ascii_bits_reject() {
    let list = TileList::new(70205).unwrap();
    for length in [0, 4, 12, BYTES - 1] {
        assert_eq!(verify(&list.bytes()[..length], 70205), Err(Error::Encoding));
    }
    assert_eq!(verify(list.bytes(), 70170), Err(Error::Encoding));
    let mut bytes = list.bytes().to_vec();
    bytes.push(0);
    assert_eq!(verify(&bytes, 70205), Err(Error::Encoding));
    for at in [0, 4, 8, 12, BYTES - 1] {
        let mut bytes = *list.bytes();
        bytes[at] = 0xff;
        assert_eq!(verify(&bytes, 70205), Err(Error::Encoding));
    }
}

#[test]
fn explicit_70374_codec_retains_legacy_gates_and_exact_layout() {
    assert!(TileList::new(70374).is_err());
    for build in [70170, 70205, 0] {
        assert!(TileList::new_70374(build).is_err());
    }
    let mut list = TileList::new_70374(70374).unwrap();
    let mut old = TileList::new(70205).unwrap();
    for (x, y) in [(0, 0), (63, 63), (3, 2), (1, 4)] {
        list.set(x, y).unwrap();
        old.set(x, y).unwrap();
    }
    let mut expected = *old.bytes();
    expected[8..12].copy_from_slice(&70374u32.to_le_bytes());
    assert_eq!(list.bytes(), &expected);
    assert_eq!(verify(list.bytes(), 70374), Err(Error::Build));
    assert_eq!(present(list.bytes(), 70374, 3, 2), Err(Error::Build));
    verify_70374(list.bytes(), 70374).unwrap();
    assert!(present_70374(list.bytes(), 70374, 3, 2).unwrap());
    assert!(!present_70374(list.bytes(), 70374, 2, 3).unwrap());
}
#[test]
fn explicit_70374_empty_full_lists_and_coordinate_bounds() {
    let mut list = TileList::new_70374(70374).unwrap();
    assert!(list.bytes()[12..].iter().all(|b| *b == b'0'));
    for y in 0..64 {
        for x in 0..64 {
            list.set(x, y).unwrap();
        }
    }
    assert!(list.bytes()[12..].iter().all(|b| *b == b'1'));
    assert_eq!(
        present_70374(list.bytes(), 70374, 64, 0),
        Err(Error::Coordinates)
    );
    assert_eq!(list.set(0, 64), Err(Error::Coordinates));
}
#[test]
fn explicit_70374_rejects_truncated_extra_header_and_non_ascii() {
    let list = TileList::new_70374(70374).unwrap();
    for n in [0, 12, BYTES - 1] {
        assert_eq!(
            verify_70374(&list.bytes()[..n], 70374),
            Err(Error::Encoding)
        );
    }
    let mut extra = list.bytes().to_vec();
    extra.push(b'0');
    assert_eq!(verify_70374(&extra, 70374), Err(Error::Encoding));
    for at in [0, 4, 8, 12, BYTES - 1] {
        let mut b = *list.bytes();
        b[at] = 0xff;
        assert_eq!(verify_70374(&b, 70374), Err(Error::Encoding));
    }
    assert_eq!(verify_70374(list.bytes(), 70205), Err(Error::Build));
}

#[test]
fn explicit_70450_tilelist_build_verify_round_trip_is_separate_from_70374() {
    for build in [70170, 70205, 70374, 70425, 0, u32::MAX] {
        assert_eq!(TileList::new_70450(build).err(), Some(Error::Build));
    }
    assert!(TileList::new(70450).is_err());
    assert!(TileList::new_70374(70450).is_err());
    let mut list = TileList::new_70450(70450).unwrap();
    for (x, y) in [(0, 0), (63, 63), (3, 2), (1, 4)] {
        list.set(x, y).unwrap();
    }
    let mut expected = *TileList::new_70374(70374).unwrap().bytes();
    expected[8..12].copy_from_slice(&70450u32.to_le_bytes());
    for (x, y) in [(0, 0), (63, 63), (3, 2), (1, 4)] {
        expected[index(x, y).unwrap()] = b'1';
    }
    assert_eq!(list.bytes(), &expected);
    verify_70450(list.bytes(), 70450).unwrap();
    assert!(present_70450(list.bytes(), 70450, 3, 2).unwrap());
    assert!(!present_70450(list.bytes(), 70450, 2, 3).unwrap());
    assert_eq!(verify_70450(list.bytes(), 70374), Err(Error::Build));
    assert_eq!(verify_70374(list.bytes(), 70450), Err(Error::Build));
}
