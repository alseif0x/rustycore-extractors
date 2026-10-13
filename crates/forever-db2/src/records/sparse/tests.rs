// SPDX-License-Identifier: GPL-3.0-or-later
//! Literal synthetic sparse fixtures, independently transcribed from pinned
//! ConversationLineMeta and SparseImpl. No client bytes are embedded or read.
use super::*;
use crate::CharacterTable;
use std::io::Cursor;

fn put32(data: &mut [u8], at: usize, value: u32) {
    data[at..at + 4].copy_from_slice(&value.to_le_bytes());
}
fn put16(data: &mut [u8], at: usize, value: u16) {
    data[at..at + 2].copy_from_slice(&value.to_le_bytes());
}
fn table() -> CharacterTable {
    CharacterTable::from_catalog("ConversationLine")
        .unwrap()
        .unwrap()
}
const UNUSED: [i16; 9] = [0, 0, 0, 0, 16, 16, 24, 24, 24];
const RAW: [u32; 9] = [
    u32::MAX,
    0x80000000,
    7,
    0x80000000,
    65535,
    32768,
    255,
    128,
    0,
];

fn fixture(
    ids: &[u32],
    rows: &[[u32; 9]],
    copies: &[(u32, u32)],
    unused: [i16; 9],
    columns: bool,
) -> Vec<u8> {
    assert_eq!(ids.len(), rows.len());
    let mut data = vec![0; 244];
    data[..4].copy_from_slice(b"WDC5");
    put32(&mut data, 4, 5);
    put32(&mut data, 136, ids.len() as u32);
    put32(&mut data, 140, 9);
    put32(&mut data, 144, 24);
    put32(&mut data, 156, 0xAA1E7988);
    put32(&mut data, 160, 50); // Catalog IDs below MinId are not repaired.
    put32(&mut data, 164, 100000);
    put16(&mut data, 172, 5);
    put16(&mut data, 174, 0xFFFF);
    put32(&mut data, 176, 9);
    put32(&mut data, 188, if columns { 216 } else { 0 });
    put32(&mut data, 200, 1);
    for (i, bits) in unused.into_iter().enumerate() {
        data.extend(bits.to_le_bytes());
        data.extend((65000u16 - i as u16).to_le_bytes()); // Deliberately ignored.
    }
    if columns {
        for _ in 0..9 {
            let at = data.len();
            data.resize(at + 24, 0);
            put16(&mut data, at, 60000); // Sparse does not use regular offsets.
        }
    }
    let payload = data.len();
    let mut entries = Vec::new();
    for row in rows {
        let start = data.len();
        for (value, bits) in row.iter().zip(unused) {
            let width = 4 - bits as usize / 8;
            data.extend_from_slice(&value.to_le_bytes()[..width]);
        }
        data.push(0xA5); // Catalog record can have trailing padding.
        entries.push((start as u32, (data.len() - start) as u16));
    }
    let catalog = data.len();
    for id in ids {
        data.extend(id.to_le_bytes());
    }
    for (new, source) in copies {
        data.extend(new.to_le_bytes());
        data.extend(source.to_le_bytes());
    }
    for (at, size) in entries {
        data.extend(at.to_le_bytes());
        data.extend(size.to_le_bytes());
    }
    for _ in ids {
        data.extend(u32::MAX.to_le_bytes());
    } // Unused conventional IDs.
    put32(&mut data, 212, payload as u32);
    put32(&mut data, 216, ids.len() as u32);
    put32(&mut data, 224, catalog as u32);
    put32(&mut data, 228, (ids.len() * 4) as u32);
    put32(&mut data, 236, ids.len() as u32);
    put32(&mut data, 240, copies.len() as u32);
    data
}
fn error(data: &[u8]) -> RecordError {
    DecodedTable::read(data, table())
        .err()
        .expect("fixture must fail")
}
fn golden() -> Vec<Vec<Value>> {
    vec![
        vec![Value::Unsigned(u32::MAX)],
        vec![Value::Unsigned(0x80000000)],
        vec![Value::Unsigned(7)],
        vec![Value::Signed(i32::MIN)],
        vec![Value::Unsigned(65535)],
        vec![Value::Unsigned(32768)],
        vec![Value::Unsigned(255)],
        vec![Value::Unsigned(128)],
        vec![Value::Unsigned(0)],
    ]
}

#[test]
fn sparse_catalog_schema_matches_literal_transcription() {
    let schema = table().schema();
    assert_eq!(schema.layout, 0xAA1E7988);
    assert_eq!((schema.index, schema.parent), (-1, -1));
    assert_eq!(
        schema.fields,
        &[
            Kind::Unsigned(32, 1),
            Kind::Unsigned(32, 1),
            Kind::Unsigned(32, 1),
            Kind::Signed(32, 1),
            Kind::Unsigned(16, 1),
            Kind::Unsigned(16, 1),
            Kind::Unsigned(8, 1),
            Kind::Unsigned(8, 1),
            Kind::Unsigned(8, 1)
        ]
    );
}

#[test]
fn sparse_literal_golden_and_copies_preserve_physical_first_field() {
    for columns in [false, true] {
        let data = fixture(
            &[7],
            &[RAW],
            &[(90000, 7), (100000, 90000)],
            UNUSED,
            columns,
        );
        let decoded = DecodedTable::read(&data, table()).unwrap();
        assert_eq!(
            decoded.rows().keys().copied().collect::<Vec<_>>(),
            [7, 90000, 100000]
        );
        for row in decoded.rows().values() {
            assert_eq!(*row, golden());
        }
    }
}

#[test]
fn sparse_primitive_varints_mask_and_sign_extend_nonbyte_widths() {
    let mut unused = UNUSED;
    unused[0] = 20; // 12 meaningful bits, stored in two bytes.
    unused[1] = 31; // 1 meaningful bit.
    unused[3] = 20;
    let mut raw = RAW;
    raw[0] = 0xFABC;
    raw[1] = 0xFF;
    raw[3] = 0xF800;
    let data = fixture(&[7], &[raw], &[], unused, true);
    let decoded = DecodedTable::read(&data, table()).unwrap();
    assert_eq!(decoded.rows()[&7][0], [Value::Unsigned(0xABC)]);
    assert_eq!(decoded.rows()[&7][1], [Value::Unsigned(1)]);
    assert_eq!(decoded.rows()[&7][3], [Value::Signed(-2048)]);
}

#[test]
fn sparse_catalog_ids_are_authoritative_even_zero_and_below_minimum() {
    let data = fixture(&[0, 7], &[RAW, RAW], &[(9, 0), (0, 7)], UNUSED, true);
    let decoded = DecodedTable::read(&data, table()).unwrap();
    assert_eq!(decoded.rows().keys().copied().collect::<Vec<_>>(), [0, 7]);
    assert_eq!(decoded.rows()[&0], golden());
}

#[test]
fn sparse_copy_order_skips_missing_forward_and_out_of_extent_rows() {
    let mut second = RAW;
    second[0] = 22;
    let data = fixture(
        &[7, 8],
        &[RAW, second],
        &[
            (30, 31),
            (31, 7),
            (8, 31),
            (40, 0),
            (41, 77),
            (100001, 7),
            (42, 100001),
            (43, u32::MAX),
        ],
        UNUSED,
        true,
    );
    let decoded = DecodedTable::read(&data, table()).unwrap();
    assert_eq!(
        decoded.rows().keys().copied().collect::<Vec<_>>(),
        [7, 8, 31]
    );
    assert_eq!(decoded.rows()[&8], golden());
}

#[test]
fn sparse_direct_api_keeps_overwritten_values_and_rejects_late_errors() {
    let mut second = RAW;
    second[0] = 22;
    let data = fixture(&[7, 8], &[RAW, second], &[(8, 7)], UNUSED, true);
    let (decoded, direct) = DecodedTable::read_with_direct_u32_values(&data, table(), 0).unwrap();
    assert_eq!(direct, BTreeSet::from([22, u32::MAX]));
    assert_eq!(decoded.rows()[&8], golden());
    assert_eq!(
        DecodedTable::read_with_direct_u32_values(&data, table(), 4).err(),
        Some(RecordError::Schema)
    );
    let late = fixture(&[7], &[RAW], &[(8, 7), (8, 8)], UNUSED, true);
    assert_eq!(
        DecodedTable::read_with_direct_u32_values(&late, table(), 0).err(),
        Some(RecordError::Unsupported)
    );
}

#[test]
fn sparse_existing_self_copy_fails_but_missing_and_zero_self_copy_skip() {
    assert_eq!(
        error(&fixture(&[7], &[RAW], &[(7, 7)], UNUSED, true)),
        RecordError::Unsupported
    );
    assert!(
        DecodedTable::read(
            &fixture(&[0, 7], &[RAW, RAW], &[(0, 0), (8, 8)], UNUSED, true),
            table()
        )
        .is_ok()
    );
}

#[test]
fn sparse_extent_overflow_and_catalog_duplicate_or_out_of_range_fail() {
    assert_eq!(
        error(&fixture(&[7, 7], &[RAW, RAW], &[], UNUSED, true)),
        RecordError::Id
    );
    assert_eq!(
        error(&fixture(&[100001], &[RAW], &[], UNUSED, true)),
        RecordError::Id
    );
    let mut data = fixture(&[u32::MAX - 1], &[RAW], &[], UNUSED, true);
    put32(&mut data, 164, u32::MAX - 1);
    assert!(DecodedTable::read(&data, table()).is_ok());
    put32(&mut data, 164, u32::MAX);
    assert_eq!(error(&data), RecordError::Id);
}

#[test]
fn sparse_bounds_validate_full_catalog_and_both_id_tables() {
    let data = fixture(&[7], &[RAW], &[], UNUSED, true);
    for end in 244..data.len() {
        assert!(
            DecodedTable::read(&data[..end], table()).is_err(),
            "truncation {end}"
        );
    }
    let mut tail = data.clone();
    tail.push(0);
    assert_eq!(error(&tail), RecordError::Bounds);
    for (offset, value, expected) in [
        (212, 495, RecordError::Bounds),
        (224, 495, RecordError::Bounds),
        (216, 2, RecordError::Bounds),
        (236, 2, RecordError::Bounds),
        (228, 0, RecordError::Id),
    ] {
        let mut changed = data.clone();
        put32(&mut changed, offset, value);
        assert_eq!(error(&changed), expected, "header {offset}");
    }
    let entry = u32_at(&data, 224).unwrap() as usize + 4;
    for (at, size) in [
        (495u32, 24u16),
        (u32::MAX, 24),
        (496, 0),
        (496, 1),
        (496, u16::MAX),
    ] {
        let mut changed = data.clone();
        put32(&mut changed, entry, at);
        put16(&mut changed, entry + 4, size);
        assert_eq!(error(&changed), RecordError::Bounds);
    }
}

#[test]
fn sparse_unsupported_payloads_and_bad_primitive_widths_fail_closed() {
    let data = fixture(&[7], &[RAW], &[], UNUSED, true);
    for offset in [148, 192, 196, 204, 220, 232, 284, 288] {
        let mut changed = data.clone();
        put32(&mut changed, offset, 1);
        assert_eq!(error(&changed), RecordError::Unsupported, "offset {offset}");
    }
    for (field, unused) in [(0, 0xFFFF), (0, 32), (4, 0), (6, 16)] {
        let mut changed = data.clone();
        put16(&mut changed, 244 + field * 4, unused);
        assert_eq!(error(&changed), RecordError::Column);
    }
    let mut changed = data.clone();
    put32(&mut changed, 188, 215);
    assert_eq!(error(&changed), RecordError::Column);
    let mut changed = data.clone();
    put32(&mut changed, 184, 1);
    assert_eq!(error(&changed), RecordError::Schema);
    let mut changed = data;
    put32(&mut changed, 200, 2);
    assert_eq!(error(&changed), RecordError::Unsupported);
}

#[test]
fn sparse_schema_gate_still_precedes_any_decoding() {
    let data = fixture(&[7], &[RAW], &[], UNUSED, true);
    for (at, value) in [(156, 0), (140, 8), (176, 8)] {
        let mut changed = data.clone();
        put32(&mut changed, at, value);
        assert_eq!(error(&changed), RecordError::Schema);
    }
}

#[test]
fn sparse_unreviewed_kinds_inline_ids_and_all_parents_stay_excluded_even_empty() {
    for name in [
        "Map",
        "Curve",
        "AnimationData",
        "SkillRaceClassInfo",
        "CollectableSourceEncounterSparse",
        "CollectableSourceQuestSparse",
        "CollectableSourceVendorSparse",
        "LiquidType",
        "ItemExtendedCost",
    ] {
        let table = CharacterTable::from_catalog(name).unwrap().unwrap();
        let (hash, layout, _, fields) = table.schema_identity();
        let mut data = vec![0; 204 + 4 * fields];
        data[..4].copy_from_slice(b"WDC5");
        put32(&mut data, 4, 5);
        put32(&mut data, 140, fields as u32);
        put32(&mut data, 176, fields as u32);
        put32(&mut data, 152, hash);
        put32(&mut data, 156, layout);
        put16(&mut data, 172, 7);
        assert!(DecodedTable::read(&data, table).is_err(), "{name}");
    }
}

#[test]
fn sparse_empty_variants_require_complete_primitive_metadata_and_exact_end() {
    let section_empty = fixture(&[], &[], &[], UNUSED, true);
    assert!(
        DecodedTable::read(&section_empty, table())
            .unwrap()
            .rows()
            .is_empty()
    );
    let mut data = section_empty[..204].to_vec();
    put32(&mut data, 188, 0);
    put32(&mut data, 200, 0);
    for unused in UNUSED {
        data.extend(unused.to_le_bytes());
        data.extend(0u16.to_le_bytes());
    }
    assert!(
        DecodedTable::read(&data, table())
            .unwrap()
            .rows()
            .is_empty()
    );
    assert_eq!(error(&data[..204]), RecordError::Bounds);
    let mut tail = data.clone();
    tail.push(0);
    assert_eq!(error(&tail), RecordError::Bounds);
    put32(&mut data, 188, 216);
    assert_eq!(error(&data), RecordError::Unsupported);
}

#[test]
fn sparse_scalar_budget_charges_new_copy_rows_not_overwrites_or_skips() {
    let with_copies = fixture(
        &[7],
        &[RAW],
        &[(8, 7), (8, 7), (9, 0), (10, 99)],
        UNUSED,
        true,
    );
    let metadata = Metadata::read(&mut Cursor::new(&with_copies)).unwrap();
    let schema = table().schema();
    assert!(read_with_limit(&with_copies, &metadata, &schema, None, 18).is_ok());
    assert_eq!(
        read_with_limit(&with_copies, &metadata, &schema, None, 17).err(),
        Some(RecordError::Limit)
    );
    assert_eq!(
        read_with_limit(&with_copies, &metadata, &schema, None, 8).err(),
        Some(RecordError::Limit)
    );
    for (at, count) in [(136, 100001), (240, 100001)] {
        let mut data = fixture(&[7], &[RAW], &[], UNUSED, true);
        put32(&mut data, at, count);
        assert_eq!(error(&data), RecordError::Limit);
    }
}

#[test]
fn sparse_header_record_size_and_alias_ranges_do_not_replace_catalog_semantics() {
    let mut data = fixture(&[7, 8], &[RAW, [0; 9]], &[], UNUSED, true);
    put32(&mut data, 144, 0);
    put32(&mut data, 180, u32::MAX);
    let entries = u32_at(&data, 224).unwrap() as usize + 8;
    put32(&mut data, entries + 6, 496); // Two catalog entries may share bytes.
    let (decoded, direct) = DecodedTable::read_with_direct_u32_values(&data, table(), 0).unwrap();
    assert_eq!(decoded.rows()[&7], decoded.rows()[&8]);
    assert_eq!(direct, BTreeSet::from([u32::MAX]));
    let zeros = fixture(&[7, 8], &[[0; 9], [0; 9]], &[], UNUSED, true);
    assert_eq!(
        DecodedTable::read_with_direct_u32_values(&zeros, table(), 0)
            .unwrap()
            .1,
        BTreeSet::from([0])
    );
}

#[test]
fn sparse_each_excluded_value_kind_fails_before_reading_any_payload() {
    let data = fixture(&[7], &[RAW], &[], UNUSED, true);
    let meta = Metadata::read(&mut Cursor::new(&data)).unwrap();
    for fields in [
        &[Kind::Unsigned(64, 1)][..],
        &[Kind::Signed(64, 1)][..],
        &[Kind::StringArray(0)][..],
        &[Kind::Unsigned(32, 0)][..],
        &[Kind::Float(0)][..],
    ] {
        let schema = Schema {
            fields,
            ..table().schema()
        };
        assert_eq!(
            read_with_limit(&data, &meta, &schema, None, 9).err(),
            Some(RecordError::Unsupported)
        );
    }
}
