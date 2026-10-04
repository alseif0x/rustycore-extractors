// SPDX-License-Identifier: GPL-3.0-or-later
//! Synthetic, in-memory WDC5 fixtures for the bounded regular-record decoder.
//! Every byte is generated here; no client file, key or server path is read.
//! A green test is a self-consistency check, not DB2/format parity evidence.
use super::CharacterTable;
use super::schema::Kind;
use super::{DecodedTable, RecordError, Value};

const HEADER: usize = 204;
const SECTION: usize = 40;
/// Offset of the first column entry for a single-section file: 204 header +
/// 40 section bytes + one 4-byte field-offset word per field.
const COLUMN_BASE: usize = 244;

fn put32(buf: &mut [u8], at: usize, value: u32) {
    buf[at..at + 4].copy_from_slice(&value.to_le_bytes());
}
fn put16(buf: &mut [u8], at: usize, value: u16) {
    buf[at..at + 2].copy_from_slice(&value.to_le_bytes());
}

#[derive(Clone, Copy)]
struct Col {
    offset: u16,
    compression: u32,
    extra: usize,
    a: u32,
    b: u32,
    c: u32,
    width: u32,
    count: usize,
}

fn default_cols(table: CharacterTable) -> Vec<Col> {
    let schema = table.schema();
    let mut bit = 0u32;
    schema
        .fields
        .iter()
        .map(|kind| {
            let width = kind.bits();
            let count = kind.count();
            let col = Col {
                offset: bit as u16,
                compression: 0,
                extra: 0,
                a: 0,
                b: width,
                c: count as u32,
                width,
                count,
            };
            bit += width * count as u32;
            col
        })
        .collect()
}

/// Direct-value columns need `offset + width*count` bits; packed modes read at
/// most 32 bits from the record start in these fixtures. Keep at least 8 bytes.
fn record_size(cols: &[Col]) -> usize {
    let mut bits = 0usize;
    for col in cols {
        if col.compression == 0 {
            bits = bits.max(col.offset as usize + col.width as usize * col.count);
        }
    }
    bits.div_ceil(8).max(8)
}

struct Spec {
    cols: Vec<Col>,
    records: Vec<Vec<u8>>,
    strings: Vec<u8>,
    ids: Vec<u32>,
    copies: Vec<(u32, u32)>,
    flags: u16,
    min_id: u32,
    max_id: u32,
    pallet: Vec<u8>,
    common: Vec<u8>,
    packed_data_offset: u32,
    h_record_count: Option<u32>,
    s_record_count: Option<u32>,
    h_string_size: Option<u32>,
    s_string_size: Option<u32>,
    id_table_size: Option<u32>,
    copy_count: Option<u32>,
    column_meta_size: Option<u32>,
    file_offset_delta: i64,
    extra_sections: u32,
    extra_tail: usize,
    truncate: Option<usize>,
    /// Header `ParentLookupCount` (`0` or `1`).
    parent_lookup_count: u32,
    /// `{ParentId, RecordIndex}` entries of the parent-lookup block.
    parent_entries: Vec<(u32, u32)>,
    /// Overrides the header's declared `NumEntries` when `parent_lookup_count == 1`.
    parent_count_override: Option<u32>,
    /// Overrides the section's declared `parent_lookup_data_size`.
    parent_lookup_size: Option<u32>,
    /// Extra bytes appended after the parent-lookup block.
    parent_tail: usize,
}

fn spec(table: CharacterTable) -> Spec {
    Spec {
        cols: default_cols(table),
        records: Vec::new(),
        strings: Vec::new(),
        ids: Vec::new(),
        copies: Vec::new(),
        flags: 0,
        min_id: 1,
        max_id: 100,
        pallet: Vec::new(),
        common: Vec::new(),
        packed_data_offset: 0,
        h_record_count: None,
        s_record_count: None,
        h_string_size: None,
        s_string_size: None,
        id_table_size: None,
        copy_count: None,
        column_meta_size: None,
        file_offset_delta: 0,
        extra_sections: 0,
        extra_tail: 0,
        truncate: None,
        parent_lookup_count: 0,
        parent_entries: Vec::new(),
        parent_count_override: None,
        parent_lookup_size: None,
        parent_tail: 0,
    }
}

fn classes(count: usize) -> Spec {
    let mut spec = spec(CharacterTable::Classes);
    let len = record_size(&spec.cols);
    spec.records = vec![vec![0u8; len]; count];
    spec
}

fn races(count: usize) -> Spec {
    let mut spec = spec(CharacterTable::Races);
    let len = record_size(&spec.cols);
    spec.records = vec![vec![0u8; len]; count];
    spec.ids = vec![0; count];
    spec.min_id = 0;
    spec.max_id = 100;
    spec
}

fn as_mode(spec: &mut Spec, field: usize, mode: u32, a: u32, b: u32, c: u32, extra: usize) {
    let col = &mut spec.cols[field];
    col.compression = mode;
    col.a = a;
    col.b = b;
    col.c = c;
    col.extra = extra;
}

fn field_offset(spec: &Spec, field: usize) -> u16 {
    spec.cols[field].offset
}

fn write_word(record: &mut [u8], bit_offset: u16, value: u32) {
    let at = bit_offset as usize / 8;
    record[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

fn write_byte(record: &mut [u8], bit_offset: u16, value: u8) {
    record[bit_offset as usize / 8] = value;
}

/// Writes an unsigned 64-bit (`FT_LONG`) direct value at a column bit offset.
fn write_u64(record: &mut [u8], bit_offset: u16, value: u64) {
    let at = bit_offset as usize / 8;
    record[at..at + 8].copy_from_slice(&value.to_le_bytes());
}

/// Writes the inline index byte of a character-table record (field 29).
fn write_inline_id(spec: &mut Spec, index: usize, value: u8) {
    let off = field_offset(spec, 29);
    write_byte(&mut spec.records[index], off, value);
}

/// Writes a 32-bit inline id at the column for `field` of record `row`.
fn write_inline_id_at(spec: &mut Spec, field: usize, value: u32, row: usize) {
    let off = field_offset(spec, field);
    write_word(&mut spec.records[row], off, value);
}

/// Decodes any reviewed table, returning its error code for a negative test.
fn record_error(table: CharacterTable, bytes: &[u8]) -> RecordError {
    match decode(table, bytes) {
        Err(error) => error,
        Ok(_) => panic!("expected error, decode succeeded"),
    }
}

fn assemble(table: CharacterTable, s: &Spec) -> Vec<u8> {
    let schema = table.schema();
    let fields = schema.fields.len();
    let record_size = record_size(&s.cols);
    let header_record_count = s.h_record_count.unwrap_or(s.records.len() as u32);
    let header_string_size = s.h_string_size.unwrap_or(s.strings.len() as u32);
    let mut out = vec![0u8; HEADER];
    out[..4].copy_from_slice(b"WDC5");
    put32(&mut out, 4, 5);
    put32(&mut out, 136, header_record_count);
    // `field_count` is the column count (FileFieldCount == schema.fields.len()).
    put32(&mut out, 140, fields as u32);
    put32(&mut out, 144, record_size as u32);
    put32(&mut out, 148, header_string_size);
    // Unknown table hashes (the four ChrClass* tables) are written as the
    // documented 0 "unknown" sentinel; `matches` does not gate on them.
    put32(&mut out, 152, schema.hash.unwrap_or(0));
    put32(&mut out, 156, schema.layout);
    put32(&mut out, 160, s.min_id);
    put32(&mut out, 164, s.max_id);
    put32(&mut out, 168, 0);
    put16(&mut out, 172, s.flags);
    out[174..176].copy_from_slice(&schema.index.to_le_bytes());
    // Reference DB2FileLoader.cpp:1771 gate:
    //   TotalFieldCount + (ParentIndexField >= TotalFieldCount ? 1 : 0)
    //   == Meta->FieldCount
    // with Meta->FieldCount == columns + 1 == fields.len() + 1 for the reviewed
    // extra-field-at-the-end parent variant. With ParentIndexField == fields.len()
    // (1) and TotalFieldCount == 1 this is `1 + 1 == 2`, so the fixture writes
    // total_field_count == fields.len() for that variant. No-parent tables keep
    // total == field == fields.len().
    put32(&mut out, 176, fields as u32);
    put32(&mut out, 180, s.packed_data_offset);
    put32(&mut out, 184, s.parent_lookup_count);
    let column_meta_size = s.column_meta_size.unwrap_or((fields * 24) as u32);
    put32(&mut out, 188, column_meta_size);
    put32(&mut out, 192, s.common.len() as u32);
    put32(&mut out, 196, s.pallet.len() as u32);
    put32(&mut out, 200, 1 + s.extra_sections);

    let section_at = out.len();
    let sections = 1 + s.extra_sections as usize;
    out.resize(section_at + SECTION * sections, 0);
    out.resize(out.len() + fields * 4, 0);
    for col in &s.cols {
        let base = out.len();
        out.resize(base + 24, 0);
        put16(&mut out, base, col.offset);
        put32(&mut out, base + 4, col.extra as u32);
        put32(&mut out, base + 8, col.compression);
        put32(&mut out, base + 12, col.a);
        put32(&mut out, base + 16, col.b);
        put32(&mut out, base + 20, col.c);
    }
    out.extend(&s.pallet);
    out.extend(&s.common);
    let metadata_end = out.len();
    for record in &s.records {
        assert_eq!(record.len(), record_size, "fixture record size");
        out.extend(record);
    }
    out.extend(&s.strings);
    if schema.index < 0 {
        for id in &s.ids {
            out.extend(id.to_le_bytes());
        }
    }
    for (new, source) in &s.copies {
        out.extend(new.to_le_bytes());
        out.extend(source.to_le_bytes());
    }
    // Parent-lookup block: 12-byte header (`NumEntries`, `MinId`, `MaxId`) then
    // `NumEntries` 8-byte `{ParentId, RecordIndex}` entries, after the copy table.
    if s.parent_lookup_count == 1 {
        let declared = s
            .parent_count_override
            .unwrap_or(s.parent_entries.len() as u32);
        out.extend(declared.to_le_bytes());
        out.extend(0u32.to_le_bytes());
        out.extend(0u32.to_le_bytes());
        for (parent_id, record_index) in &s.parent_entries {
            out.extend(parent_id.to_le_bytes());
            out.extend(record_index.to_le_bytes());
        }
    }
    out.resize(out.len() + s.parent_tail, 0);

    let default_id_table = if schema.index < 0 {
        header_record_count as usize * 4
    } else {
        0
    };
    let file_offset = (metadata_end as i64 + s.file_offset_delta) as u32;
    let section_record_count = s.s_record_count.unwrap_or(header_record_count);
    let section_string_size = s.s_string_size.unwrap_or(header_string_size);
    let section_id_table = s.id_table_size.unwrap_or(default_id_table as u32);
    let section_copy_count = s.copy_count.unwrap_or(s.copies.len() as u32);
    let default_parent_size = if s.parent_lookup_count == 1 {
        let declared = s
            .parent_count_override
            .unwrap_or(s.parent_entries.len() as u32);
        12 + declared as usize * 8
    } else {
        0
    };
    let section_parent_size = s.parent_lookup_size.unwrap_or(default_parent_size as u32);
    put32(&mut out, section_at + 8, file_offset);
    put32(&mut out, section_at + 12, section_record_count);
    put32(&mut out, section_at + 16, section_string_size);
    put32(&mut out, section_at + 20, 0);
    put32(&mut out, section_at + 24, section_id_table);
    put32(&mut out, section_at + 28, section_parent_size);
    put32(&mut out, section_at + 32, 0);
    put32(&mut out, section_at + 36, section_copy_count);

    out.resize(out.len() + s.extra_tail, 0);
    if let Some(len) = s.truncate {
        out.truncate(len);
    }
    out
}

fn decode(table: CharacterTable, bytes: &[u8]) -> Result<DecodedTable, RecordError> {
    DecodedTable::read(bytes, table)
}

fn build_classes(spec: &Spec) -> Vec<u8> {
    assemble(CharacterTable::Classes, spec)
}

fn build_races(spec: &Spec) -> Vec<u8> {
    assemble(CharacterTable::Races, spec)
}

fn decode_classes(spec: &Spec) -> Result<DecodedTable, RecordError> {
    let bytes = build_classes(spec);
    DecodedTable::read(&bytes, CharacterTable::Classes)
}

fn decode_races(spec: &Spec) -> Result<DecodedTable, RecordError> {
    let bytes = build_races(spec);
    DecodedTable::read(&bytes, CharacterTable::Races)
}

/// A decode-valid spec for any reviewed id-table-only table: zero records of
/// the right size plus a separate u32 id table. `ChrClass*` tables all have
/// IndexField -1 and a u32 id table, so the generic builder applies to them.
fn idtable_spec(table: CharacterTable, count: usize) -> Spec {
    let mut spec = spec(table);
    let len = record_size(&spec.cols);
    spec.records = vec![vec![0u8; len]; count];
    spec.ids = vec![0; count];
    spec.min_id = 0;
    spec.max_id = 100;
    spec
}

fn build_idtable(table: CharacterTable, spec: &Spec) -> Vec<u8> {
    assemble(table, spec)
}

/// Decode-valid spec for `ChrClassesXPowerTypes`: two zeroed records, a separate
/// u32 id table (IndexField -1) and an optional parent-lookup block.
fn xpower(count: usize) -> Spec {
    let mut spec = spec(CharacterTable::ClassXPowerTypes);
    let len = record_size(&spec.cols);
    spec.records = vec![vec![0u8; len]; count];
    spec.ids = vec![0; count];
    spec.min_id = 0;
    spec.max_id = 100;
    spec
}

fn decode_xpower(spec: &Spec) -> Result<DecodedTable, RecordError> {
    let bytes = assemble(CharacterTable::ClassXPowerTypes, spec);
    DecodedTable::read(&bytes, CharacterTable::ClassXPowerTypes)
}

fn expect_err_xpower(bytes: &[u8]) -> RecordError {
    match decode(CharacterTable::ClassXPowerTypes, bytes) {
        Err(error) => error,
        Ok(_) => panic!("expected error, decode succeeded"),
    }
}

/// A minimal header-only WDC5 file with `section_count == 0` (204 bytes). Used
/// to exercise the zero-section "empty table" path. `record_size` and
/// `min_id <= max_id` stay valid so only the section count distinguishes it.
fn build_zero_section_header(table: CharacterTable, record_count: u32) -> Vec<u8> {
    let schema = table.schema();
    let fields = schema.fields.len();
    let mut out = vec![0u8; HEADER];
    out[..4].copy_from_slice(b"WDC5");
    put32(&mut out, 4, 5);
    put32(&mut out, 136, record_count);
    put32(&mut out, 140, fields as u32);
    put32(&mut out, 144, 8);
    put32(&mut out, 148, 0);
    put32(&mut out, 152, schema.hash.unwrap_or(0));
    put32(&mut out, 156, schema.layout);
    put32(&mut out, 160, 0);
    put32(&mut out, 164, 100);
    put32(&mut out, 168, 0);
    put16(&mut out, 172, 0);
    out[174..176].copy_from_slice(&schema.index.to_le_bytes());
    put32(&mut out, 176, fields as u32);
    put32(&mut out, 180, 0);
    put32(&mut out, 184, 0);
    put32(&mut out, 188, (fields * 24) as u32);
    put32(&mut out, 192, 0);
    put32(&mut out, 196, 0);
    put32(&mut out, 200, 0);
    out
}

fn decode_idtable(table: CharacterTable, spec: &Spec) -> Result<DecodedTable, RecordError> {
    let bytes = build_idtable(table, spec);
    DecodedTable::read(&bytes, table)
}

fn expect_err_idtable(table: CharacterTable, bytes: &[u8]) -> RecordError {
    match decode(table, bytes) {
        Err(error) => error,
        Ok(_) => panic!("expected error, decode succeeded"),
    }
}

fn expect_err_classes(bytes: &[u8]) -> RecordError {
    match decode(CharacterTable::Classes, bytes) {
        Err(error) => error,
        Ok(_) => panic!("expected error, decode succeeded"),
    }
}

fn expect_err_races(bytes: &[u8]) -> RecordError {
    match decode(CharacterTable::Races, bytes) {
        Err(error) => error,
        Ok(_) => panic!("expected error, decode succeeded"),
    }
}

// ---------------------------------------------------------------------------
// Positive paths
// ---------------------------------------------------------------------------

#[test]
fn mode_zero_direct_unsigned_and_empty_strings() {
    let mut spec = classes(1);
    let off = field_offset(&spec, 10);
    write_word(&mut spec.records[0], off, 0xDEAD_BEEF);
    let out = decode_classes(&spec).unwrap();
    let row = out.rows().values().next().unwrap();
    assert_eq!(row[10][0], Value::Unsigned(0xDEAD_BEEF));
    assert_eq!(row[0][0], Value::Text(String::new()));
}

#[test]
fn mode_one_packed_unsigned() {
    let mut spec = classes(1);
    let a = u32::from(field_offset(&spec, 10));
    as_mode(&mut spec, 10, 1, a, 32, 0, 0);
    write_word(&mut spec.records[0], a as u16, 0x1122_3344);
    let out = decode_classes(&spec).unwrap();
    let row = out.rows().values().next().unwrap();
    assert_eq!(row[10][0], Value::Unsigned(0x1122_3344));
}

#[test]
fn mode_two_constant_default_and_id_common() {
    let mut spec = classes(1);
    as_mode(&mut spec, 10, 2, 0xABCD, 0, 0, 0);
    let out = decode_classes(&spec).unwrap();
    let row = out.rows().values().next().unwrap();
    assert_eq!(row[10][0], Value::Unsigned(0xABCD));

    let mut spec = classes(1);
    spec.min_id = 0;
    spec.max_id = 100;
    write_inline_id(&mut spec, 0, 7);
    spec.common = [7u32.to_le_bytes(), 0x1234u32.to_le_bytes()].concat();
    as_mode(&mut spec, 10, 2, 0xFFFF, 0, 0, 8);
    let out = decode_classes(&spec).unwrap();
    assert_eq!(out.rows()[&7][10][0], Value::Unsigned(0x1234));
}

#[test]
fn mode_three_scalar_pallet() {
    let mut spec = classes(1);
    let a = u32::from(field_offset(&spec, 10));
    spec.pallet = 0xCAFE_BABEu32.to_le_bytes().to_vec();
    as_mode(&mut spec, 10, 3, a, 8, 0, 4);
    let out = decode_classes(&spec).unwrap();
    let row = out.rows().values().next().unwrap();
    assert_eq!(row[10][0], Value::Unsigned(0xCAFE_BABE));
}

#[test]
fn mode_four_array_pallet() {
    let mut spec = races(1);
    spec.ids[0] = 5;
    let a = u32::from(field_offset(&spec, 23));
    spec.pallet = [1u32, 2, 3]
        .into_iter()
        .flat_map(u32::to_le_bytes)
        .collect();
    as_mode(&mut spec, 23, 4, a, 8, 3, 12);
    let out = decode_races(&spec).unwrap();
    let expected = vec![Value::Signed(1), Value::Signed(2), Value::Signed(3)];
    assert_eq!(out.rows()[&5][23], expected);
}

#[test]
fn mode_five_signed_packed_sign_extends() {
    let mut spec = classes(1);
    let a = u32::from(field_offset(&spec, 14));
    as_mode(&mut spec, 14, 5, a, 8, 0, 0);
    write_byte(&mut spec.records[0], a as u16, 0xFE);
    let out = decode_classes(&spec).unwrap();
    let row = out.rows().values().next().unwrap();
    assert_eq!(row[14][0], Value::Signed(-2));
}

#[test]
fn signed_eight_bit_direct_value_sign_extends() {
    let mut spec = classes(1);
    // ChrClasses Signed(8,1) at index 32 (29-30 are Unsigned(8,1), 31-32 are
    // Signed(8,1)).
    let off = field_offset(&spec, 32);
    write_byte(&mut spec.records[0], off, 0x80);
    let out = decode_classes(&spec).unwrap();
    let row = out.rows().values().next().unwrap();
    assert_eq!(row[32][0], Value::Signed(-128));
}

#[test]
fn sixteen_bit_unsigned_is_masked_to_width() {
    let mut spec = classes(1);
    // ChrClasses Unsigned(16,1) is at index 28 (27 and 28 are both FT_SHORT).
    let off = field_offset(&spec, 28);
    let at = off as usize / 8;
    spec.records[0][at..at + 2].copy_from_slice(&0xFFFFu16.to_le_bytes());
    let out = decode_classes(&spec).unwrap();
    let row = out.rows().values().next().unwrap();
    assert_eq!(row[28][0], Value::Unsigned(0xFFFF));
}

#[test]
fn float_bits_are_returned_verbatim() {
    let mut spec = races(1);
    spec.ids[0] = 5;
    let off = field_offset(&spec, 31);
    write_word(&mut spec.records[0], off, 0x3F80_0000);
    let out = decode_races(&spec).unwrap();
    assert_eq!(out.rows()[&5][31][0], Value::FloatBits(0x3F80_0000));
}

#[test]
fn utf8_and_empty_strings_are_decoded() {
    let mut spec = classes(1);
    spec.strings = "héllo\0".as_bytes().to_vec();
    let relative = record_size(&spec.cols) as u32;
    write_word(&mut spec.records[0], 0, relative);
    let out = decode_classes(&spec).unwrap();
    let row = out.rows().values().next().unwrap();
    assert_eq!(row[0][0], Value::Text("héllo".to_owned()));
    assert_eq!(row[1][0], Value::Text(String::new()));
}

#[test]
fn string_offsets_are_relative_to_each_record_not_the_string_table() {
    // Legacy evidence: vendor/trinitycore-legacy/src/common/DataStores/
    // DB2FileLoader.cpp:800-806 resolves a string as
    // record_start + fieldOffset + stored_offset. The alternative
    // "strings_start + stored_offset" is out of range for record 0 here, so this
    // test fails if the formula is changed away from the per-record base.
    let mut spec = classes(2);
    spec.min_id = 0;
    spec.max_id = 100;
    write_inline_id(&mut spec, 0, 1);
    write_inline_id(&mut spec, 1, 2);
    let size = record_size(&spec.cols) as u32;
    let field = 9;
    let off = field_offset(&spec, field);
    let base = u32::from(off) / 8;
    // Record 0 points at the start of the table; record 1 points 5 bytes in.
    let stored0 = 2 * size - base;
    let stored1 = size - base + 5;
    write_word(&mut spec.records[0], off, stored0);
    write_word(&mut spec.records[1], off, stored1);
    spec.strings = b"ZERO\0ONE\0".to_vec();
    let out = decode_classes(&spec).unwrap();
    assert_eq!(out.rows()[&1][field][0], Value::Text("ZERO".to_owned()));
    assert_eq!(out.rows()[&2][field][0], Value::Text("ONE".to_owned()));
}

#[test]
fn inline_id_is_decoded() {
    let mut spec = classes(1);
    spec.min_id = 0;
    spec.max_id = 100;
    write_inline_id(&mut spec, 0, 5);
    let out = decode_classes(&spec).unwrap();
    let keys = out.rows().keys().copied().collect::<Vec<_>>();
    assert_eq!(keys, vec![5]);
}

#[test]
fn id_table_id_is_decoded() {
    let mut spec = races(1);
    spec.ids[0] = 42;
    let out = decode_races(&spec).unwrap();
    let keys = out.rows().keys().copied().collect::<Vec<_>>();
    assert_eq!(keys, vec![42]);
}

#[test]
fn id_below_min_is_raw_inline_and_repaired_from_table() {
    // Inline index field: RecordGetId returns the stored value, so an id below
    // MinId is used as-is (DB2FileLoader.cpp:747-760).
    let mut spec = classes(1);
    spec.min_id = 10;
    spec.max_id = 100;
    write_inline_id(&mut spec, 0, 3);
    let out = decode_classes(&spec).unwrap();
    let keys = out.rows().keys().copied().collect::<Vec<_>>();
    assert_eq!(keys, vec![3]);

    // Separate id table: DB2FileLoader.cpp:1958-1967 repairs `id <= MinId` to
    // `MinId + i`, with `i` the single-section record ordinal.
    let mut spec = races(1);
    spec.min_id = 10;
    spec.max_id = 100;
    spec.ids[0] = 3;
    let out = decode_races(&spec).unwrap();
    let keys = out.rows().keys().copied().collect::<Vec<_>>();
    assert_eq!(keys, vec![10]);
}

#[test]
fn inline_index_is_not_repaired_when_below_min_id() {
    // Negative guard for the inline path: a raw inline id below MinId is not
    // rewritten, so the row is keyed by the raw value (1), not MinId (50).
    let mut spec = classes(1);
    spec.min_id = 50;
    spec.max_id = 100;
    write_inline_id(&mut spec, 0, 1);
    let out = decode_classes(&spec).unwrap();
    assert_eq!(out.rows().keys().copied().collect::<Vec<_>>(), vec![1]);
}

#[test]
fn copy_skips_source_zero() {
    let mut spec = classes(1);
    spec.copies = vec![(50, 0)];
    let out = decode_classes(&spec).unwrap();
    assert_eq!(out.rows().len(), 1);
    assert!(!out.rows().contains_key(&50));
}

#[test]
fn copy_overwrites_destination_and_rewrites_inline_id() {
    let mut spec = classes(3);
    spec.min_id = 0;
    spec.max_id = 1000;
    let off = field_offset(&spec, 10);
    write_inline_id(&mut spec, 0, 1);
    write_word(&mut spec.records[0], off, 0xAAAA);
    write_inline_id(&mut spec, 1, 2);
    write_word(&mut spec.records[1], off, 0xBBBB);
    write_inline_id(&mut spec, 2, 3);
    // Source row 1 exists and both ordinals are below record_count (3).
    spec.copies = vec![(2, 1)];
    let out = decode_classes(&spec).unwrap();
    assert_eq!(out.rows()[&2][10][0], Value::Unsigned(0xAAAA));
    assert_eq!(out.rows()[&2][29][0], Value::Unsigned(2));
}

#[test]
fn copy_table_exact_policy_limit_is_accepted() {
    let mut spec = classes(1);
    spec.copies = vec![(0, 0); 100_000];
    let out = decode_classes(&spec).unwrap();
    assert_eq!(out.rows().len(), 1);
}

#[test]
fn value_count_exact_policy_boundary_is_accepted() {
    // ChrClasses' inline index field (29) is FT_BYTE, so only 2^8 = 256 distinct
    // ids fit; a 43-field value at the record_count policy frontier cannot carry
    // that many distinct inline rows. The value-count limit is table independent,
    // so exercise the exact boundary with ChrRaces (51 fields, separate u32 id
    // table): floor(1_000_000 / 51) = 19_607 records is admitted, 19_608 exceeds
    // it. Distinct ids keep the accepted decode within the id-table path.
    let mut spec = races(19_607);
    spec.min_id = 0;
    spec.max_id = 100_000;
    spec.ids = (1..=19_607u32).collect();
    let out = decode_races(&spec).unwrap();
    assert_eq!(out.rows().len(), 19_607);
    assert_eq!(out.rows().keys().next().copied(), Some(1));

    let over = races(19_608);
    let bytes = build_races(&over);
    assert_eq!(expect_err_races(&bytes), RecordError::Limit);
}

// ---------------------------------------------------------------------------
// Negative paths: exact RecordError::code for every frontier
// ---------------------------------------------------------------------------

#[test]
fn truncation_frontiers_map_to_metadata_or_bounds() {
    let spec = classes(1);
    let full = build_classes(&spec);
    let metadata_end = COLUMN_BASE + spec.cols.len() * 4 + spec.cols.len() * 24;
    assert_eq!(metadata_end, 1448);
    for len in [0usize, 1, 100, 203, 204, 205, 243] {
        let err = expect_err_classes(&full[..len]);
        assert_eq!(err, RecordError::Metadata, "len {len}");
    }
    for len in [
        244usize,
        300,
        metadata_end - 1,
        metadata_end,
        full.len() - 1,
    ] {
        let err = expect_err_classes(&full[..len]);
        assert_eq!(err, RecordError::Bounds, "len {len}");
    }
    assert!(decode_classes(&spec).is_ok());
}

#[test]
fn zero_record_size_and_reversed_id_range_are_bounds() {
    let mut bytes = build_classes(&classes(1));
    put32(&mut bytes, 144, 0);
    assert_eq!(expect_err_classes(&bytes), RecordError::Bounds);

    let mut bytes = build_classes(&classes(1));
    put32(&mut bytes, 160, 50);
    put32(&mut bytes, 164, 10);
    assert_eq!(expect_err_classes(&bytes), RecordError::Bounds);
}

#[test]
fn header_section_record_and_string_count_mismatch_are_bounds() {
    let mut bytes = build_classes(&classes(1));
    put32(&mut bytes, 204 + 12, 2);
    assert_eq!(expect_err_classes(&bytes), RecordError::Bounds);

    let mut bytes = build_classes(&classes(1));
    put32(&mut bytes, 204 + 16, 5);
    assert_eq!(expect_err_classes(&bytes), RecordError::Bounds);
}

#[test]
fn extra_and_missing_tail_bytes_are_bounds() {
    let mut bytes = build_classes(&classes(1));
    bytes.push(0);
    assert_eq!(expect_err_classes(&bytes), RecordError::Bounds);

    let mut spec = classes(1);
    spec.file_offset_delta = 4;
    let bytes = build_classes(&spec);
    assert_eq!(expect_err_classes(&bytes), RecordError::Bounds);
}

#[test]
fn sparse_flag_bit_is_unsupported_and_other_flag_bits_are_accepted() {
    // Reference DB2FileLoader.cpp:1852 discriminates only on bit 0
    // (`if (!(_header.Flags & 0x1))` regular else sparse); no other bit is
    // validated. Bit 0 must be rejected, and any value without bit 0 must
    // still decode normally.
    for flag in [1u16, 3, 0x8001, u16::MAX] {
        let mut bytes = build_classes(&classes(1));
        put16(&mut bytes, 172, flag);
        let err = expect_err_classes(&bytes);
        assert_eq!(err, RecordError::Unsupported, "flag {flag:#06x}");
    }
    for flag in [2u16, 4, 0x0004, 0x8000, 0xFFFE] {
        let mut bytes = build_classes(&classes(1));
        put16(&mut bytes, 172, flag);
        let out = decode(CharacterTable::Classes, &bytes)
            .unwrap_or_else(|error| panic!("flag {flag:#06x} rejected as {}", error.code()));
        assert_eq!(out.rows().len(), 1, "flag {flag:#06x}");
    }
}

#[test]
fn nonzero_nonsparse_flags_decode_like_the_reviewed_schema() {
    // Real 70205 ChrRaces metadata carries Flags == 4; the reviewed schema must
    // accept and decode it rather than returning Unsupported.
    let mut spec = races(1);
    spec.flags = 4;
    spec.ids[0] = 42;
    let out = decode_races(&spec).unwrap();
    assert_eq!(out.rows().keys().copied().collect::<Vec<_>>(), vec![42]);
}

#[test]
fn matches_ignores_the_header_index_field() {
    // The reference never consults _header.IndexField (DB2FileLoader.cpp:1746
    // only endian-converts it); inline vs id-table comes from the reviewed
    // meta. Real 70205 ChrRaces has header IndexField == 0 while the reviewed
    // meta index is -1, so a Races-like file with header index_field 0 must
    // still match and decode through the separate id table.
    let mut spec = races(1);
    spec.ids[0] = 7;
    let mut bytes = build_races(&spec);
    put16(&mut bytes, 174, 0);
    let header = crate::Metadata::read(&mut std::io::Cursor::new(&bytes))
        .unwrap()
        .header;
    assert_eq!(header.index_field, 0);
    assert!(CharacterTable::Races.matches(&header));
    let out = decode(CharacterTable::Races, &bytes).unwrap();
    assert_eq!(out.rows().keys().copied().collect::<Vec<_>>(), vec![7]);

    // ChrClasses uses inline field 29; a header index_field of 0 (instead of
    // 29) must not change matching or the inline decode path.
    let mut spec = classes(1);
    spec.min_id = 0;
    spec.max_id = 100;
    write_inline_id(&mut spec, 0, 5);
    let mut bytes = build_classes(&spec);
    put16(&mut bytes, 174, 0);
    let header = crate::Metadata::read(&mut std::io::Cursor::new(&bytes))
        .unwrap()
        .header;
    assert_eq!(header.index_field, 0);
    assert!(CharacterTable::Classes.matches(&header));
    let out = decode(CharacterTable::Classes, &bytes).unwrap();
    assert_eq!(out.rows().keys().copied().collect::<Vec<_>>(), vec![5]);
}

#[test]
fn multi_section_tact_parent_and_catalog_variants_are_unsupported() {
    let mut spec = classes(1);
    spec.extra_sections = 1;
    let bytes = build_classes(&spec);
    assert_eq!(expect_err_classes(&bytes), RecordError::Unsupported);

    let mut bytes = build_classes(&classes(1));
    put32(&mut bytes, 204, 7);
    assert_eq!(expect_err_classes(&bytes), RecordError::Unsupported);

    // A parent-lookup count above 1 is rejected while the header is parsed:
    // lib.rs Header::parse returns Error::ParentLookupCount (mapped to
    // RecordError::Metadata) before this decoder can classify it as a variant,
    // mirroring DB2FileLoader.cpp:1767.
    let mut bytes = build_classes(&classes(1));
    put32(&mut bytes, 184, 2);
    assert_eq!(expect_err_classes(&bytes), RecordError::Metadata);

    // A parent-lookup count of 1 on a table whose reviewed metadata has no
    // parent index field is a schema mismatch (DB2FileLoader.cpp:1773).
    let mut bytes = build_classes(&classes(1));
    put32(&mut bytes, 184, 1);
    assert_eq!(expect_err_classes(&bytes), RecordError::Schema);

    let mut bytes = build_classes(&classes(1));
    put32(&mut bytes, 204 + 32, 1);
    assert_eq!(expect_err_classes(&bytes), RecordError::Unsupported);

    // A section that declares parent-lookup data while the header declares no
    // parent block (and the reviewed meta has no parent field) cannot be
    // reconciled: the trailing bytes have no accounted reader, so it is a
    // record_data_bounds mismatch rather than an unsupported variant.
    let mut bytes = build_classes(&classes(1));
    put32(&mut bytes, 204 + 28, 1);
    assert_eq!(expect_err_classes(&bytes), RecordError::Bounds);
}

#[test]
fn schema_identity_mismatches_are_schema_errors() {
    let schema = CharacterTable::Classes.schema();
    let mut bytes = build_classes(&classes(1));
    put32(&mut bytes, 152, schema.hash.unwrap() ^ 1);
    assert_eq!(expect_err_classes(&bytes), RecordError::Schema);

    let mut bytes = build_classes(&classes(1));
    put32(&mut bytes, 156, schema.layout ^ 1);
    assert_eq!(expect_err_classes(&bytes), RecordError::Schema);

    let mut bytes = build_classes(&classes(1));
    put32(&mut bytes, 140, 42);
    assert_eq!(expect_err_classes(&bytes), RecordError::Schema);

    let mut bytes = build_classes(&classes(1));
    put32(&mut bytes, 176, 42);
    assert_eq!(expect_err_classes(&bytes), RecordError::Schema);
}

#[test]
fn column_meta_size_compression_and_stride_are_column_errors() {
    let mut bytes = build_classes(&classes(1));
    put32(&mut bytes, 188, (43 * 24 + 1) as u32);
    assert_eq!(expect_err_classes(&bytes), RecordError::Column);

    let mut bytes = build_classes(&classes(1));
    // Column entries begin after the 204-byte header, 40-byte section and the
    // 4-byte-per-field offset table: 244 + 43*4; entry compression is at +8.
    put32(&mut bytes, 244 + 43 * 4 + 8, 6);
    assert_eq!(expect_err_classes(&bytes), RecordError::Column);

    let mut spec = classes(1);
    as_mode(&mut spec, 10, 3, 0, 8, 0, 6);
    let bytes = build_classes(&spec);
    assert_eq!(expect_err_classes(&bytes), RecordError::Column);
}

#[test]
fn duplicate_and_out_of_range_inline_ids_are_id_errors() {
    let mut spec = classes(2);
    spec.min_id = 0;
    spec.max_id = 100;
    let off = field_offset(&spec, 29);
    write_byte(&mut spec.records[0], off, 5);
    write_byte(&mut spec.records[1], off, 5);
    let bytes = build_classes(&spec);
    assert_eq!(expect_err_classes(&bytes), RecordError::Id);

    let mut spec = classes(1);
    spec.min_id = 0;
    spec.max_id = 100;
    write_inline_id(&mut spec, 0, 200);
    let bytes = build_classes(&spec);
    assert_eq!(expect_err_classes(&bytes), RecordError::Id);
}

#[test]
fn id_table_size_mismatch_is_id_error() {
    let mut spec = races(1);
    spec.id_table_size = Some(0);
    let bytes = build_races(&spec);
    assert_eq!(expect_err_races(&bytes), RecordError::Id);
}

#[test]
fn copy_with_new_row_id_at_or_above_record_count_is_skipped() {
    // AutoProduceRecordCopies requires NewRowId < records; a larger id makes the
    // entry a silent skip, not an error (DB2FileLoader.cpp:613-634).
    let mut spec = classes(1);
    spec.min_id = 0;
    spec.max_id = 1_000_000;
    write_inline_id(&mut spec, 0, 1);
    spec.copies = vec![(70_000, 1)];
    let out = decode_classes(&spec).unwrap();
    assert_eq!(out.rows().len(), 1);
    assert!(!out.rows().contains_key(&70_000));
}

#[test]
fn string_range_nul_and_utf8_violations_are_string_errors() {
    let mut spec = classes(1);
    spec.strings = b"A\0".to_vec();
    let relative = record_size(&spec.cols) as u32;
    write_word(&mut spec.records[0], 0, relative + 5);
    let bytes = build_classes(&spec);
    assert_eq!(expect_err_classes(&bytes), RecordError::String);

    let mut spec = classes(1);
    spec.strings = b"Hello".to_vec();
    let relative = record_size(&spec.cols) as u32;
    write_word(&mut spec.records[0], 0, relative);
    let bytes = build_classes(&spec);
    assert_eq!(expect_err_classes(&bytes), RecordError::String);

    let mut spec = classes(1);
    spec.strings = vec![0xFF, 0x00];
    let relative = record_size(&spec.cols) as u32;
    write_word(&mut spec.records[0], 0, relative);
    let bytes = build_classes(&spec);
    assert_eq!(expect_err_classes(&bytes), RecordError::String);
}

#[test]
fn policy_limits_are_limit_errors() {
    let oversized = vec![0u8; 16 * 1024 * 1024 + 1];
    assert_eq!(expect_err_classes(&oversized), RecordError::Limit);

    let mut spec = classes(0);
    spec.h_record_count = Some(100_001);
    spec.s_record_count = Some(100_001);
    let bytes = build_classes(&spec);
    assert_eq!(expect_err_classes(&bytes), RecordError::Limit);

    let mut spec = classes(0);
    spec.h_record_count = Some(100_000);
    spec.s_record_count = Some(100_000);
    let bytes = build_classes(&spec);
    assert_eq!(expect_err_classes(&bytes), RecordError::Limit);

    let mut spec = classes(1);
    spec.copy_count = Some(100_001);
    let bytes = build_classes(&spec);
    assert_eq!(expect_err_classes(&bytes), RecordError::Limit);
}

// ---------------------------------------------------------------------------
// Schema transcription self-check
// ---------------------------------------------------------------------------

#[test]
fn schema_field_counts_match_the_reviewed_reference() {
    // Verified 2026-10-04 against the pinned checkout
    // /home/joe/projects/dream/wow/rustycore/target/forever-cpp-reference at
    // 02245dcd245e7433e524577656177723d3e4992e:
    //   DB2Metadata.h:3455 `struct ChrClassesMeta` Fields[43],
    //   DB2Metadata.h:4182 `struct ChrRacesMeta`  Fields[51].
    // These assertions pin the transcription; a green test is not reader/runtime
    // parity, only that this vendored schema list matches the cited fields.
    let classes = CharacterTable::Classes.schema().fields.len();
    let races = CharacterTable::Races.schema().fields.len();
    assert_eq!(classes, 43, "ChrClasses transcribed field count");
    assert_eq!(races, 51, "ChrRaces transcribed field count");
}

#[test]
fn classes_index_field_widths_match_the_reviewed_reference() {
    // Verified 2026-10-04 against DB2Metadata.h:3455 (ChrClassesMeta) at
    // 02245dcd245e7433e524577656177723d3e4992e: the INT block 18-26 is nine
    // signed 32-bit fields, 27-28 are two unsigned FT_SHORT, and 29 -- the meta
    // IndexField -- is one unsigned FT_BYTE (8 bits). An earlier transcription
    // added a tenth Signed(32,1) at index 27, which shifted the index field to
    // 16 bits and read 6/9 real ids past max_id (invalid_record_ids).
    let schema = CharacterTable::Classes.schema();
    assert_eq!(schema.fields.len(), 43, "ChrClasses field count");
    assert_eq!(schema.index, 29, "ChrClasses meta index field");
    assert_eq!(schema.fields[27].bits(), 16, "field 27 width");
    assert_eq!(schema.fields[28].bits(), 16, "field 28 width");
    assert_eq!(schema.fields[29].bits(), 8, "field 29 (index) width");
    assert!(matches!(schema.fields[27], Kind::Unsigned(16, 1)));
    assert!(matches!(schema.fields[28], Kind::Unsigned(16, 1)));
    assert!(matches!(schema.fields[29], Kind::Unsigned(8, 1)));
}

// ---------------------------------------------------------------------------
// ChrClass* id-table tables (No parent lookup, IndexField -1)
// ---------------------------------------------------------------------------

/// The four reviewed `ChrClass*` tables with their pinned identity. Their table
/// hashes are NOT transcribed because `DB2Meta` does not carry one, so the only
/// trustworthy identity is (layout_hash, field_count, index=-1, file_id).
const NEW_TABLES: [(CharacterTable, u32, u32, u32, i16, usize); 4] = [
    (
        CharacterTable::ClassTitle,
        1267497,
        0xC086F95D,
        0xC086F95D,
        -1,
        3,
    ),
    (
        CharacterTable::ClassUIDisplay,
        1316515,
        0x50F5AD66,
        0x50F5AD66,
        -1,
        3,
    ),
    (
        CharacterTable::ClassUIChrModelInfo,
        3754770,
        0x26E5E322,
        0x26E5E322,
        -1,
        5,
    ),
    (
        CharacterTable::ClassVillain,
        1267498,
        0x46041630,
        0x46041630,
        -1,
        3,
    ),
];

#[test]
fn chr_class_table_identity_and_transcription_match_the_reference() {
    // Verified 2026-10-04 against the pinned checkout at
    // 02245dcd245e7433e524577656177723d3e4992e: DB2Metadata.h:3369/3390/3413/
    // 3434 declare FileDataId, IndexField = -1 and LayoutHash for each
    // `ChrClass*` meta; FieldCount == FileFieldCount for all four.
    for (table, file_id, layout, _, index, fields) in NEW_TABLES {
        let schema = table.schema();
        assert_eq!(table.file_id(), file_id, "{} file_id", table.name());
        assert_eq!(schema.layout, layout, "{} layout", table.name());
        assert_eq!(schema.index, index, "{} index", table.name());
        assert_eq!(schema.fields.len(), fields, "{} field count", table.name());
        // No `DB2Meta` member supplies a table hash, so none is transcribed.
        assert!(
            schema.hash.is_none(),
            "{} must not invent a hash",
            table.name()
        );
    }
}

#[test]
fn named_and_name_round_trip_for_every_reviewed_table() {
    for (table, _, _, _, _, _) in NEW_TABLES {
        assert_eq!(CharacterTable::named(table.name()), Some(table));
    }
    // The two legacy tables keep working and unrelated names stay rejected.
    assert_eq!(
        CharacterTable::named("ChrClasses"),
        Some(CharacterTable::Classes)
    );
    assert_eq!(
        CharacterTable::named("ChrRaces"),
        Some(CharacterTable::Races)
    );
    assert_eq!(CharacterTable::named("ChrClassTitles"), None);
    assert_eq!(CharacterTable::named("Synthetic"), None);
}

#[test]
fn chr_class_matches_gates_on_known_identity_not_an_unknown_table_hash() {
    // DB2Meta has no table hash and DB2FileLoader.cpp never checks
    // `_header.TableHash` (it validates LayoutHash at :1763 and the field count
    // at :1771), so an arbitrary header table hash must not block a `ChrClass*`
    // file whose layout/field/total identity is right.
    for (table, _, layout, _, _, fields) in NEW_TABLES {
        let mut spec = idtable_spec(table, 1);
        spec.ids[0] = 7;
        let mut bytes = build_idtable(table, &spec);
        let header = crate::Metadata::read(&mut std::io::Cursor::new(&bytes))
            .unwrap()
            .header;
        assert_eq!(header.layout_hash, layout, "{} layout", table.name());
        assert_eq!(
            header.field_count as usize,
            fields,
            "{} fields",
            table.name()
        );
        assert!(table.matches(&header), "{} identity", table.name());
        let out = decode(table, &bytes).unwrap();
        assert_eq!(out.rows().keys().copied().collect::<Vec<_>>(), vec![7]);

        // Any table-hash word is accepted for these tables.
        put32(&mut bytes, 152, 0xDEAD_BEEF);
        let header = crate::Metadata::read(&mut std::io::Cursor::new(&bytes))
            .unwrap()
            .header;
        assert!(table.matches(&header), "{} arbitrary hash", table.name());
    }
}

#[test]
fn chr_class_matches_rejects_layout_field_and_total_mismatches() {
    for (table, _, _, _, _, _) in NEW_TABLES {
        // Wrong layout hash.
        let mut bytes = build_idtable(table, &idtable_spec(table, 1));
        put32(&mut bytes, 156, 0);
        let header = crate::Metadata::read(&mut std::io::Cursor::new(&bytes))
            .unwrap()
            .header;
        assert!(!table.matches(&header), "{} layout mismatch", table.name());
        assert_eq!(
            expect_err_idtable(table, &bytes),
            RecordError::Schema,
            "{} layout",
            table.name()
        );

        // Wrong header field count.
        let mut bytes = build_idtable(table, &idtable_spec(table, 1));
        put32(&mut bytes, 140, 2);
        let header = crate::Metadata::read(&mut std::io::Cursor::new(&bytes))
            .unwrap()
            .header;
        assert!(!table.matches(&header), "{} field_count", table.name());
        assert_eq!(
            expect_err_idtable(table, &bytes),
            RecordError::Schema,
            "{} field_count",
            table.name()
        );

        // Total field count that disagrees with the header field count.
        let mut bytes = build_idtable(table, &idtable_spec(table, 1));
        let fields = table.schema().fields.len() as u32;
        put32(&mut bytes, 176, fields + 1);
        let header = crate::Metadata::read(&mut std::io::Cursor::new(&bytes))
            .unwrap()
            .header;
        assert!(
            !table.matches(&header),
            "{} total_field_count",
            table.name()
        );
        assert_eq!(
            expect_err_idtable(table, &bytes),
            RecordError::Schema,
            "{} total_field_count",
            table.name()
        );
    }
}

#[test]
fn classes_and_races_still_gate_on_their_evidence_pinned_table_hash() {
    // The transcription change must not weaken the two tables whose real-header
    // evidence pinned a table hash.
    assert!(CharacterTable::Classes.schema().hash.is_some());
    assert!(CharacterTable::Races.schema().hash.is_some());
    let mut bytes = build_classes(&classes(1));
    put32(&mut bytes, 152, 0);
    let header = crate::Metadata::read(&mut std::io::Cursor::new(&bytes))
        .unwrap()
        .header;
    assert!(!CharacterTable::Classes.matches(&header));
    assert_eq!(expect_err_classes(&bytes), RecordError::Schema);
}

#[test]
fn chr_class_title_decodes_strings_and_signed_byte() {
    let table = CharacterTable::ClassTitle;
    let mut spec = idtable_spec(table, 1);
    spec.ids[0] = 100;
    // Field 1 is a string: point record 0 at "Title\0" in the string table.
    // The stored word is relative to the record start plus the column's own
    // byte offset (DB2FileLoader.cpp:800-806), so subtract that byte offset.
    spec.strings = b"Title\0".to_vec();
    let string_off = field_offset(&spec, 1);
    let relative = record_size(&spec.cols) as u32 - u32::from(string_off) / 8;
    write_word(&mut spec.records[0], string_off, relative);
    // Field 2 is Signed(8,1); 0x9C sign-extends to -100.
    let signed_off = field_offset(&spec, 2);
    write_byte(&mut spec.records[0], signed_off, 0x9C);
    let out = decode_idtable(table, &spec).unwrap();
    let row = &out.rows()[&100];
    assert_eq!(row[0][0], Value::Text(String::new()));
    assert_eq!(row[1][0], Value::Text("Title".to_owned()));
    assert_eq!(row[2][0], Value::Signed(-100));
}

#[test]
fn chr_class_ui_display_decodes_signed_byte_and_unsigned_ints() {
    let table = CharacterTable::ClassUIDisplay;
    let mut spec = idtable_spec(table, 1);
    spec.ids[0] = 42;
    let off0 = field_offset(&spec, 0);
    let off1 = field_offset(&spec, 1);
    let off2 = field_offset(&spec, 2);
    write_byte(&mut spec.records[0], off0, 0x80);
    write_word(&mut spec.records[0], off1, 0xFFFF_FFFF);
    write_word(&mut spec.records[0], off2, 0x0000_0001);
    let out = decode_idtable(table, &spec).unwrap();
    let row = &out.rows()[&42];
    assert_eq!(row[0][0], Value::Signed(-128));
    assert_eq!(row[1][0], Value::Unsigned(0xFFFF_FFFF));
    assert_eq!(row[2][0], Value::Unsigned(1));
}

#[test]
fn chr_class_ui_chr_model_info_decodes_float_arrays_and_signed_fields() {
    let table = CharacterTable::ClassUIChrModelInfo;
    let mut spec = idtable_spec(table, 1);
    spec.ids[0] = 9;
    // Field 0 is Float(3): three 32-bit raw float words.
    let f0 = field_offset(&spec, 0);
    let at = f0 as usize / 8;
    for (array, bits) in [0x3F80_0000u32, 0x4000_0000, 0x4040_0000]
        .into_iter()
        .enumerate()
    {
        spec.records[0][at + array * 4..at + array * 4 + 4].copy_from_slice(&bits.to_le_bytes());
    }
    let off1 = field_offset(&spec, 1);
    let off2 = field_offset(&spec, 2);
    let off3 = field_offset(&spec, 3);
    let off4 = field_offset(&spec, 4);
    write_word(&mut spec.records[0], off1, 0xFFFF_FFFE);
    write_byte(&mut spec.records[0], off2, 0x7F);
    write_word(&mut spec.records[0], off3, 0x3F00_0000);
    write_word(&mut spec.records[0], off4, 0x0000_0005);
    let out = decode_idtable(table, &spec).unwrap();
    let row = &out.rows()[&9];
    assert_eq!(
        row[0],
        vec![
            Value::FloatBits(0x3F80_0000),
            Value::FloatBits(0x4000_0000),
            Value::FloatBits(0x4040_0000),
        ]
    );
    assert_eq!(row[1][0], Value::Signed(-2));
    assert_eq!(row[2][0], Value::Signed(127));
    assert_eq!(row[3][0], Value::FloatBits(0x3F00_0000));
    assert_eq!(row[4][0], Value::Signed(5));
}

#[test]
fn chr_class_villain_decodes_string_and_two_signed_bytes() {
    let table = CharacterTable::ClassVillain;
    let mut spec = idtable_spec(table, 1);
    spec.ids[0] = 3;
    spec.strings = b"Villain\0".to_vec();
    let relative = record_size(&spec.cols) as u32;
    let off0 = field_offset(&spec, 0);
    let off1 = field_offset(&spec, 1);
    let off2 = field_offset(&spec, 2);
    write_word(&mut spec.records[0], off0, relative);
    write_byte(&mut spec.records[0], off1, 0xFF);
    write_byte(&mut spec.records[0], off2, 0x01);
    let out = decode_idtable(table, &spec).unwrap();
    let row = &out.rows()[&3];
    assert_eq!(row[0][0], Value::Text("Villain".to_owned()));
    assert_eq!(row[1][0], Value::Signed(-1));
    assert_eq!(row[2][0], Value::Signed(1));
}

#[test]
fn chr_class_id_table_repair_matches_the_separate_id_path() {
    // The four ChrClass* tables use IndexField -1, so they share the separate
    // u32 id-table path: DB2FileLoader.cpp:1958-1967 repairs `id <= MinId` to
    // `MinId + i` for the single-section ordinal `i`.
    let table = CharacterTable::ClassUIDisplay;
    let mut spec = idtable_spec(table, 1);
    spec.min_id = 50;
    spec.max_id = 100;
    spec.ids[0] = 3;
    let out = decode_idtable(table, &spec).unwrap();
    assert_eq!(out.rows().keys().copied().collect::<Vec<_>>(), vec![50]);
}

#[test]
fn empty_table_with_zero_sections_decodes_to_zero_rows() {
    // DB2FileLoader.cpp:1778-1779 admits a zero-section header and
    // DB2FileLoader.cpp:1877 then assigns an empty record map. A reviewed table
    // header with section_count == 0 and record_count == 0 must decode as a
    // valid empty table, not Unsupported.
    for (table, _, _, _, _, _) in NEW_TABLES {
        let bytes = build_zero_section_header(table, 0);
        let meta = crate::Metadata::read(&mut std::io::Cursor::new(&bytes)).unwrap();
        assert_eq!(meta.sections.len(), 0, "{} sections", table.name());
        let out = decode(table, &bytes)
            .unwrap_or_else(|error| panic!("{} rejected as {}", table.name(), error.code()));
        assert_eq!(out.rows().len(), 0, "{} rows", table.name());
    }
}

#[test]
fn zero_sections_with_declared_records_is_rejected() {
    // A zero-section header that still declares records has no offsets to read,
    // so the declared records cannot be honored; reject rather than silently
    // dropping them.
    for (table, _, _, _, _, _) in NEW_TABLES {
        let bytes = build_zero_section_header(table, 1);
        assert_eq!(
            expect_err_idtable(table, &bytes),
            RecordError::Unsupported,
            "{} zero sections with records",
            table.name()
        );
    }
}

// ---------------------------------------------------------------------------
// ChrClassesXPowerTypes (IndexField -1, ParentIndexField 1, FileFieldCount 1)
// ---------------------------------------------------------------------------

#[test]
fn xpower_schema_has_one_file_field_and_an_appended_parent() {
    // Verified 2026-10-04 against DB2Metadata.h::ChrClassesXPowerTypesMeta at
    // 02245dcd245e7433e524577656177723d3e4992e: FileDataId 1121420,
    // IndexField -1, ParentIndexField 1, FieldCount 2, FileFieldCount 1,
    // LayoutHash 0x70DA1F8C. `fields` is the single decoded column; `parent`
    // indexes the total array (>= fields.len() => extra field at the end).
    let schema = CharacterTable::ClassXPowerTypes.schema();
    assert_eq!(CharacterTable::ClassXPowerTypes.file_id(), 1_121_420);
    assert_eq!(schema.layout, 0x70DA1F8C);
    assert_eq!(schema.index, -1);
    assert_eq!(schema.parent, 1);
    assert_eq!(schema.fields.len(), 1, "one file field");
    assert!(matches!(schema.fields[0], Kind::Signed(8, 1)));
    assert!(schema.parent as usize >= schema.fields.len());
    assert!(schema.hash.is_none(), "must not invent a table hash");
}

#[test]
fn xpower_header_gate_uses_total_plus_one_for_the_appended_parent() {
    // DB2FileLoader.cpp:1771: TotalFieldCount + (ParentIndexField >=
    // TotalFieldCount ? 1 : 0) == Meta->FieldCount. For this table the reviewed
    // Meta->FieldCount is 2 (1 column + appended parent). A conforming header
    // therefore carries field_count = 1 and total_field_count = 1.
    let mut spec = xpower(1);
    spec.ids[0] = 11;
    spec.min_id = 0;
    spec.max_id = 100;
    spec.parent_lookup_count = 1;
    spec.parent_entries = vec![(0, 0)];
    let bytes = assemble(CharacterTable::ClassXPowerTypes, &spec);
    let header = crate::Metadata::read(&mut std::io::Cursor::new(&bytes))
        .unwrap()
        .header;
    assert_eq!(header.field_count, 1, "file field count");
    assert_eq!(header.total_field_count, 1, "total field count");
    assert!(CharacterTable::ClassXPowerTypes.matches(&header));
    let out = decode_xpower(&spec).unwrap();
    assert_eq!(out.rows().keys().copied().collect::<Vec<_>>(), vec![11]);
}

#[test]
fn xpower_decodes_two_entries_and_fills_parent_ids() {
    // Two records keyed by the separate id table; parent-lookup entries
    // `{ParentId, RecordIndex}` fill the appended field of the addressed
    // ordinal. Reference DB2FileLoader.cpp:1877 fills the extra field from the
    // parent block at the record ordinal.
    let mut spec = xpower(2);
    spec.ids = vec![11, 22];
    spec.min_id = 0;
    spec.max_id = 100;
    spec.parent_lookup_count = 1;
    spec.parent_entries = vec![(0, 0), (1, 1)];
    let out = decode_xpower(&spec).unwrap();
    let keys = out.rows().keys().copied().collect::<Vec<_>>();
    assert_eq!(keys, vec![11, 22]);
    // Column 0 is the single signed byte (zero in these fixtures); index 1 is
    // the appended parent field.
    assert_eq!(out.rows()[&11].len(), 2);
    assert_eq!(out.rows()[&11][0][0], Value::Signed(0));
    assert_eq!(out.rows()[&11][1][0], Value::Signed(0));
    assert_eq!(out.rows()[&22][1][0], Value::Signed(1));
    assert_eq!(spec.parent_entries.len(), 2, "two parent entries");
}

#[test]
fn xpower_zero_parent_entries_still_gets_the_default_parent_field() {
    // A parent block with NumEntries = 0 is admitted; every row keeps the
    // default appended parent field of signed zero.
    let mut spec = xpower(2);
    spec.ids = vec![11, 22];
    spec.min_id = 0;
    spec.max_id = 100;
    spec.parent_lookup_count = 1;
    spec.parent_entries = Vec::new();
    let out = decode_xpower(&spec).unwrap();
    assert_eq!(out.rows()[&11].len(), 2);
    assert_eq!(out.rows()[&11][1][0], Value::Signed(0));
    assert_eq!(out.rows()[&22][1][0], Value::Signed(0));
}

#[test]
fn xpower_duplicate_parent_entry_last_wins() {
    // Two parent entries addressing the same record ordinal: the later entry is
    // applied last and wins.
    let mut spec = xpower(1);
    spec.ids[0] = 11;
    spec.min_id = 0;
    spec.max_id = 100;
    spec.parent_lookup_count = 1;
    spec.parent_entries = vec![(7, 0), (9, 0)];
    let out = decode_xpower(&spec).unwrap();
    assert_eq!(out.rows()[&11][1][0], Value::Signed(9));
}

#[test]
fn xpower_parent_record_index_at_or_above_record_count_is_id_error() {
    // A parent entry whose RecordIndex addresses no decoded ordinal is an
    // invalid_record_ids error, not a silent skip.
    let mut spec = xpower(1);
    spec.ids[0] = 11;
    spec.min_id = 0;
    spec.max_id = 100;
    spec.parent_lookup_count = 1;
    spec.parent_entries = vec![(5, 1)];
    let bytes = assemble(CharacterTable::ClassXPowerTypes, &spec);
    assert_eq!(expect_err_xpower(&bytes), RecordError::Id);
}

#[test]
fn xpower_parent_lookup_size_inconsistency_is_bounds() {
    // The section's declared parent_lookup_data_size must equal
    // 12 + NumEntries * 8; a mismatch is record_data_bounds.
    let mut spec = xpower(1);
    spec.ids[0] = 11;
    spec.min_id = 0;
    spec.max_id = 100;
    spec.parent_lookup_count = 1;
    spec.parent_entries = vec![(5, 0)];
    spec.parent_lookup_size = Some(0);
    let bytes = assemble(CharacterTable::ClassXPowerTypes, &spec);
    assert_eq!(expect_err_xpower(&bytes), RecordError::Bounds);
}

#[test]
fn xpower_extra_bytes_after_the_parent_block_are_bounds() {
    let mut spec = xpower(1);
    spec.ids[0] = 11;
    spec.min_id = 0;
    spec.max_id = 100;
    spec.parent_lookup_count = 1;
    spec.parent_entries = vec![(5, 0)];
    spec.parent_tail = 1;
    let bytes = assemble(CharacterTable::ClassXPowerTypes, &spec);
    assert_eq!(expect_err_xpower(&bytes), RecordError::Bounds);
}

#[test]
fn xpower_parent_count_zero_but_schema_expects_parent_is_schema() {
    // The reviewed schema has a parent index field, so a header that declares no
    // parent-lookup block is a schema mismatch (the `==` in
    // `(parent_lookup_count == 1) != (schema.parent >= 0)`), reported as
    // unsupported_table_schema before any data bounds are checked.
    let mut spec = xpower(1);
    spec.ids[0] = 11;
    spec.min_id = 0;
    spec.max_id = 100;
    spec.parent_lookup_count = 0;
    spec.parent_entries = Vec::new();
    let bytes = assemble(CharacterTable::ClassXPowerTypes, &spec);
    assert_eq!(expect_err_xpower(&bytes), RecordError::Schema);
}

#[test]
fn xpower_wrong_layout_and_field_counts_are_schema() {
    let mut spec = xpower(1);
    spec.ids[0] = 11;
    spec.min_id = 0;
    spec.max_id = 100;
    spec.parent_lookup_count = 1;
    spec.parent_entries = vec![(5, 0)];

    // Wrong layout hash.
    let mut bytes = assemble(CharacterTable::ClassXPowerTypes, &spec);
    put32(&mut bytes, 156, 0);
    assert_eq!(expect_err_xpower(&bytes), RecordError::Schema);

    // Header field_count must equal the single file field.
    let mut bytes = assemble(CharacterTable::ClassXPowerTypes, &spec);
    put32(&mut bytes, 140, 2);
    assert_eq!(expect_err_xpower(&bytes), RecordError::Schema);

    // Header total_field_count must satisfy total + 1 == FieldCount (2), i.e.
    // total == 1; a total of 2 fails the gate.
    let mut bytes = assemble(CharacterTable::ClassXPowerTypes, &spec);
    put32(&mut bytes, 176, 2);
    assert_eq!(expect_err_xpower(&bytes), RecordError::Schema);
}

// ---------------------------------------------------------------------------
// ChrSpecialization (IndexField 3, ParentIndexField 4 in the data block)
// ---------------------------------------------------------------------------

/// Decode-valid spec for `ChrSpecialization`: zeroed records plus an optional
/// parent-lookup block. `IndexField 3` is inline, so there is no separate id
/// table; `ParentIndexField 4` is INSIDE the file field count.
fn specialization(count: usize) -> Spec {
    let mut spec = spec(CharacterTable::Specialization);
    let len = record_size(&spec.cols);
    spec.records = vec![vec![0u8; len]; count];
    spec.min_id = 0;
    spec.max_id = 100;
    spec
}

fn decode_specialization(spec: &Spec) -> Result<DecodedTable, RecordError> {
    let bytes = assemble(CharacterTable::Specialization, spec);
    DecodedTable::read(&bytes, CharacterTable::Specialization)
}

fn expect_err_specialization(bytes: &[u8]) -> RecordError {
    match decode(CharacterTable::Specialization, bytes) {
        Err(error) => error,
        Ok(_) => panic!("expected error, decode succeeded"),
    }
}

#[test]
fn specialization_schema_has_an_in_data_block_parent() {
    // Verified 2026-10-04 against DB2Metadata.h::ChrSpecializationMeta at
    // 02245dcd245e7433e524577656177723d3e4992e: FileDataId 1343390,
    // IndexField 3, ParentIndexField 4, FieldCount == FileFieldCount == 13,
    // LayoutHash 0xDAB4CA4B. Field 4 is a SIGNED 8-bit FT_BYTE column that is
    // also the parent index, so `parent < fields.len()` (in data block).
    let schema = CharacterTable::Specialization.schema();
    assert_eq!(CharacterTable::Specialization.file_id(), 1_343_390);
    assert_eq!(schema.layout, 0xDAB4CA4B);
    assert_eq!(schema.index, 3);
    assert_eq!(schema.parent, 4);
    assert!(schema.parent >= 0);
    assert!((schema.parent as usize) < schema.fields.len());
    assert_eq!(schema.fields.len(), 13);
    for field in [0, 1, 2] {
        assert!(
            matches!(schema.fields[field], Kind::String),
            "field {field}"
        );
    }
    assert!(matches!(schema.fields[3], Kind::Unsigned(32, 1)));
    assert!(matches!(schema.fields[4], Kind::Signed(8, 1)));
    assert!(matches!(schema.fields[5], Kind::Signed(8, 1)));
    assert!(matches!(schema.fields[6], Kind::Signed(8, 1)));
    assert!(matches!(schema.fields[7], Kind::Signed(8, 1)));
    assert!(matches!(schema.fields[8], Kind::Signed(32, 1)));
    assert!(matches!(schema.fields[9], Kind::Signed(32, 1)));
    assert!(matches!(schema.fields[10], Kind::Signed(8, 1)));
    assert!(matches!(schema.fields[11], Kind::Signed(32, 1)));
    assert!(matches!(schema.fields[12], Kind::Signed(32, 2)));
    assert!(schema.hash.is_none(), "must not invent a table hash");
    assert_eq!(
        CharacterTable::named("ChrSpecialization"),
        Some(CharacterTable::Specialization)
    );
}

#[test]
fn specialization_header_gate_uses_total_equal_field_for_in_data_parent() {
    // DB2FileLoader.cpp:1771: TotalFieldCount + (ParentIndexField >=
    // TotalFieldCount ? 1 : 0) == Meta->FieldCount. For an in-data-block parent
    // there is no appended field, so the header carries field_count ==
    // total_field_count == 13.
    let mut spec = specialization(1);
    spec.parent_lookup_count = 1;
    spec.parent_entries = vec![(0, 0)];
    write_inline_id_at(&mut spec, 3, 11, 0);
    let bytes = assemble(CharacterTable::Specialization, &spec);
    let header = crate::Metadata::read(&mut std::io::Cursor::new(&bytes))
        .unwrap()
        .header;
    assert_eq!(header.field_count, 13, "file field count");
    assert_eq!(header.total_field_count, 13, "total field count");
    assert!(CharacterTable::Specialization.matches(&header));
}

#[test]
fn specialization_parent_overwrites_the_in_data_block_byte_column() {
    // A parent-lookup entry fills the EXISTING signed 8-bit column 4 (not an
    // appended field). Reference DB2FileLoader.cpp:650-712 writes with the
    // field's own width and sign-extends. ParentId 200 -> -56 at 8 bits.
    let mut spec = specialization(1);
    spec.parent_lookup_count = 1;
    spec.parent_entries = vec![(200, 0)];
    write_inline_id_at(&mut spec, 3, 11, 0);
    let off = field_offset(&spec, 4);
    write_byte(&mut spec.records[0], off, 0x7F);
    let out = decode_specialization(&spec).unwrap();
    let row = &out.rows()[&11];
    assert_eq!(row.len(), 13, "no appended parent field");
    assert_eq!(row[4][0], Value::Signed(-56));
    // The parent did not disturb the inline id column.
    assert_eq!(row[3][0], Value::Unsigned(11));
}

#[test]
fn specialization_parent_above_byte_width_is_id_error() {
    // A ParentId above 255 cannot fit the signed 8-bit FT_BYTE parent column,
    // so the reference ASSERT maps to invalid_record_ids.
    let mut spec = specialization(1);
    spec.parent_lookup_count = 1;
    spec.parent_entries = vec![(256, 0)];
    write_inline_id_at(&mut spec, 3, 11, 0);
    let bytes = assemble(CharacterTable::Specialization, &spec);
    assert_eq!(expect_err_specialization(&bytes), RecordError::Id);
}

#[test]
fn specialization_parent_byte_boundary_is_accepted() {
    // ParentId 255 fits the byte (sign-extends to -1); 256 is the first reject.
    let mut spec = specialization(1);
    spec.parent_lookup_count = 1;
    spec.parent_entries = vec![(255, 0)];
    write_inline_id_at(&mut spec, 3, 11, 0);
    let out = decode_specialization(&spec).unwrap();
    assert_eq!(out.rows()[&11][4][0], Value::Signed(-1));
}

#[test]
fn specialization_parent_count_zero_but_schema_expects_parent_is_schema() {
    // The reviewed schema has a parent index field, so a header that declares no
    // parent-lookup block is a schema mismatch.
    let mut spec = specialization(1);
    spec.parent_lookup_count = 0;
    write_inline_id_at(&mut spec, 3, 11, 0);
    let bytes = assemble(CharacterTable::Specialization, &spec);
    assert_eq!(expect_err_specialization(&bytes), RecordError::Schema);
}

// ---------------------------------------------------------------------------
// Forever ChrCustomization* / ChrCustItemGeoModify (No parent, IndexField -1)
// ---------------------------------------------------------------------------

/// The reviewed Forever tables with their pinned identity. Their table
/// hashes are NOT transcribed because `DB2Meta` does not carry one, so the only
/// trustworthy identity is (layout_hash, field_count, index=-1, file_id). All
/// declare `ParentIndexField = -1` and `FieldCount == FileFieldCount`.
const FOREVER_TABLES: [(CharacterTable, u32, u32, usize); 8] = [
    (
        CharacterTable::CustomizationCategory,
        3526439,
        0x1605B68A,
        8,
    ),
    (CharacterTable::CustItemGeoModify, 3952714, 0x2986E1F3, 3),
    (CharacterTable::CustomizationGeoset, 3456171, 0xBF55F2FB, 3),
    (
        CharacterTable::CustomizationMaterial,
        3459652,
        0xBE9767E9,
        2,
    ),
    (CharacterTable::CustomizationBoneSet, 3460181, 0xCC2CFF6E, 2),
    (
        CharacterTable::CustomizationElement,
        3512765,
        0x6483C37E,
        13,
    ),
    (
        CharacterTable::CustomizationDisplayInfo,
        3593032,
        0xB24CE54F,
        5,
    ),
    (CharacterTable::CustomizationReq, 3450453, 0xCA154412, 9),
];

#[test]
fn forever_table_identity_and_transcription_match_the_reference() {
    // Verified against DB2Metadata.h at 02245dcd: each of the five tables
    // declares FileDataId, IndexField = -1, ParentIndexField = -1 and a
    // LayoutHash; FieldCount == FileFieldCount for all five.
    for (table, file_id, layout, fields) in FOREVER_TABLES {
        let schema = table.schema();
        assert_eq!(table.file_id(), file_id, "{} file_id", table.name());
        assert_eq!(schema.layout, layout, "{} layout", table.name());
        assert_eq!(schema.index, -1, "{} index", table.name());
        assert_eq!(schema.parent, -1, "{} parent", table.name());
        assert_eq!(schema.fields.len(), fields, "{} field count", table.name());
        // No `DB2Meta` member supplies a table hash, so none is transcribed.
        assert!(
            schema.hash.is_none(),
            "{} must not invent a hash",
            table.name()
        );
        assert_eq!(CharacterTable::named(table.name()), Some(table));
    }
}

#[test]
fn forever_table_matches_gates_on_known_identity_not_a_table_hash() {
    // No table hash is transcribed, so any header `TableHash` word must be
    // accepted while layout/field/total identity is right (the reference checks
    // LayoutHash at DB2FileLoader.cpp:1763 and the field count at :1771).
    for (table, _, layout, fields) in FOREVER_TABLES {
        let mut spec = idtable_spec(table, 1);
        spec.ids[0] = 7;
        let mut bytes = build_idtable(table, &spec);
        let header = crate::Metadata::read(&mut std::io::Cursor::new(&bytes))
            .unwrap()
            .header;
        assert_eq!(header.layout_hash, layout, "{} layout", table.name());
        assert_eq!(
            header.field_count as usize,
            fields,
            "{} fields",
            table.name()
        );
        assert_eq!(
            header.total_field_count as usize,
            fields,
            "{} total fields",
            table.name()
        );
        assert!(table.matches(&header), "{} identity", table.name());
        let out = decode(table, &bytes).unwrap();
        assert_eq!(out.rows().keys().copied().collect::<Vec<_>>(), vec![7]);

        put32(&mut bytes, 152, 0xDEAD_BEEF);
        let header = crate::Metadata::read(&mut std::io::Cursor::new(&bytes))
            .unwrap()
            .header;
        assert!(table.matches(&header), "{} arbitrary hash", table.name());
    }
}

#[test]
fn forever_table_matches_rejects_layout_field_and_total_mismatches() {
    for (table, _, _, _) in FOREVER_TABLES {
        let mut bytes = build_idtable(table, &idtable_spec(table, 1));
        put32(&mut bytes, 156, 0);
        let header = crate::Metadata::read(&mut std::io::Cursor::new(&bytes))
            .unwrap()
            .header;
        assert!(!table.matches(&header), "{} layout mismatch", table.name());
        assert_eq!(
            expect_err_idtable(table, &bytes),
            RecordError::Schema,
            "{} layout",
            table.name()
        );

        let mut bytes = build_idtable(table, &idtable_spec(table, 1));
        put32(&mut bytes, 140, 42);
        let header = crate::Metadata::read(&mut std::io::Cursor::new(&bytes))
            .unwrap()
            .header;
        assert!(!table.matches(&header), "{} field_count", table.name());
        assert_eq!(
            expect_err_idtable(table, &bytes),
            RecordError::Schema,
            "{} field_count",
            table.name()
        );

        let mut bytes = build_idtable(table, &idtable_spec(table, 1));
        let fields = table.schema().fields.len() as u32;
        put32(&mut bytes, 176, fields + 1);
        let header = crate::Metadata::read(&mut std::io::Cursor::new(&bytes))
            .unwrap()
            .header;
        assert!(
            !table.matches(&header),
            "{} total_field_count",
            table.name()
        );
        assert_eq!(
            expect_err_idtable(table, &bytes),
            RecordError::Schema,
            "{} total_field_count",
            table.name()
        );
    }
}

#[test]
fn chr_customization_category_decodes_string_signed_ints_and_float() {
    // ChrCustomizationCategory: [String, Signed(32,1)x6, Float(1)].
    let table = CharacterTable::CustomizationCategory;
    let mut spec = idtable_spec(table, 1);
    spec.ids[0] = 100;
    spec.strings = b"Category\0".to_vec();
    let string_off = field_offset(&spec, 0);
    let relative = record_size(&spec.cols) as u32 - u32::from(string_off) / 8;
    write_word(&mut spec.records[0], string_off, relative);
    let signed = [-1i32, 0, 7, -2_000_000_000, 2_000_000_000, 42];
    for (field, value) in signed.into_iter().enumerate() {
        let off = field_offset(&spec, field + 1);
        write_word(&mut spec.records[0], off, value as u32);
    }
    let off_float = field_offset(&spec, 7);
    write_word(&mut spec.records[0], off_float, 0x3F80_0000);
    let out = decode_idtable(table, &spec).unwrap();
    let row = &out.rows()[&100];
    assert_eq!(row[0][0], Value::Text("Category".to_owned()));
    for (field, value) in signed.into_iter().enumerate() {
        assert_eq!(
            row[field + 1][0],
            Value::Signed(value),
            "field {}",
            field + 1
        );
    }
    assert_eq!(row[7][0], Value::FloatBits(0x3F80_0000));
}

#[test]
fn chr_cust_item_geo_modify_decodes_signed_byte_and_two_signed_ints() {
    let table = CharacterTable::CustItemGeoModify;
    let mut spec = idtable_spec(table, 1);
    spec.ids[0] = 7;
    let off0 = field_offset(&spec, 0);
    let off1 = field_offset(&spec, 1);
    let off2 = field_offset(&spec, 2);
    write_byte(&mut spec.records[0], off0, 0x9C);
    write_word(&mut spec.records[0], off1, (-5i32) as u32);
    write_word(&mut spec.records[0], off2, 123_456);
    let out = decode_idtable(table, &spec).unwrap();
    let row = &out.rows()[&7];
    assert_eq!(row[0][0], Value::Signed(-100));
    assert_eq!(row[1][0], Value::Signed(-5));
    assert_eq!(row[2][0], Value::Signed(123_456));
}

#[test]
fn chr_customization_geoset_decodes_signed_byte_and_two_signed_ints() {
    let table = CharacterTable::CustomizationGeoset;
    let mut spec = idtable_spec(table, 1);
    spec.ids[0] = 3;
    let off0 = field_offset(&spec, 0);
    let off1 = field_offset(&spec, 1);
    let off2 = field_offset(&spec, 2);
    write_byte(&mut spec.records[0], off0, 0x7F);
    write_word(&mut spec.records[0], off1, (-1i32) as u32);
    write_word(&mut spec.records[0], off2, 8);
    let out = decode_idtable(table, &spec).unwrap();
    let row = &out.rows()[&3];
    assert_eq!(row[0][0], Value::Signed(127));
    assert_eq!(row[1][0], Value::Signed(-1));
    assert_eq!(row[2][0], Value::Signed(8));
}

#[test]
fn chr_customization_material_decodes_two_signed_ints() {
    let table = CharacterTable::CustomizationMaterial;
    let mut spec = idtable_spec(table, 1);
    spec.ids[0] = 42;
    let off0 = field_offset(&spec, 0);
    let off1 = field_offset(&spec, 1);
    write_word(&mut spec.records[0], off0, (-7i32) as u32);
    write_word(&mut spec.records[0], off1, 999);
    let out = decode_idtable(table, &spec).unwrap();
    let row = &out.rows()[&42];
    assert_eq!(row[0][0], Value::Signed(-7));
    assert_eq!(row[1][0], Value::Signed(999));
}

#[test]
fn chr_customization_bone_set_decodes_two_signed_ints() {
    let table = CharacterTable::CustomizationBoneSet;
    let mut spec = idtable_spec(table, 1);
    spec.ids[0] = 5;
    let off0 = field_offset(&spec, 0);
    let off1 = field_offset(&spec, 1);
    write_word(&mut spec.records[0], off0, 11);
    write_word(&mut spec.records[0], off1, (-42i32) as u32);
    let out = decode_idtable(table, &spec).unwrap();
    let row = &out.rows()[&5];
    assert_eq!(row[0][0], Value::Signed(11));
    assert_eq!(row[1][0], Value::Signed(-42));
}

#[test]
fn chr_customization_element_transcription_is_thirteen_signed_ints() {
    // ChrCustomizationElement: 13 x Signed(32,1), no parent, IndexField -1.
    let schema = CharacterTable::CustomizationElement.schema();
    assert_eq!(schema.layout, 0x6483C37E);
    assert_eq!(schema.index, -1);
    assert_eq!(schema.parent, -1);
    assert_eq!(schema.fields.len(), 13);
    for (index, kind) in schema.fields.iter().enumerate() {
        assert!(
            matches!(kind, Kind::Signed(32, 1)),
            "field {index} is not Signed(32,1)"
        );
    }
}

#[test]
fn chr_customization_element_decodes_thirteen_signed_ints() {
    let table = CharacterTable::CustomizationElement;
    let mut spec = idtable_spec(table, 1);
    spec.ids[0] = 77;
    let values = [
        -1i32,
        0,
        7,
        -2_000_000_000,
        2_000_000_000,
        42,
        -123_456,
        123_456,
        i32::MIN,
        i32::MAX,
        9,
        -9,
        0,
    ];
    for (field, value) in values.into_iter().enumerate() {
        let off = field_offset(&spec, field);
        write_word(&mut spec.records[0], off, value as u32);
    }
    let out = decode_idtable(table, &spec).unwrap();
    let row = &out.rows()[&77];
    assert_eq!(row.len(), 13);
    for (field, value) in values.into_iter().enumerate() {
        assert_eq!(row[field][0], Value::Signed(value), "field {field}");
    }
}

#[test]
fn chr_customization_display_info_transcription_is_two_signed_and_three_floats() {
    // ChrCustomizationDisplayInfo: Signed(32,1), Signed(32,1), Float(1) x3.
    let schema = CharacterTable::CustomizationDisplayInfo.schema();
    assert_eq!(schema.layout, 0xB24CE54F);
    assert_eq!(schema.index, -1);
    assert_eq!(schema.parent, -1);
    assert_eq!(schema.fields.len(), 5);
    assert!(matches!(schema.fields[0], Kind::Signed(32, 1)));
    assert!(matches!(schema.fields[1], Kind::Signed(32, 1)));
    assert!(matches!(schema.fields[2], Kind::Float(1)));
    assert!(matches!(schema.fields[3], Kind::Float(1)));
    assert!(matches!(schema.fields[4], Kind::Float(1)));
}

#[test]
fn chr_customization_display_info_decodes_signed_ints_and_floats() {
    let table = CharacterTable::CustomizationDisplayInfo;
    let mut spec = idtable_spec(table, 1);
    spec.ids[0] = 12;
    let off0 = field_offset(&spec, 0);
    let off1 = field_offset(&spec, 1);
    let off2 = field_offset(&spec, 2);
    let off3 = field_offset(&spec, 3);
    let off4 = field_offset(&spec, 4);
    write_word(&mut spec.records[0], off0, (-7i32) as u32);
    write_word(&mut spec.records[0], off1, 1_000);
    write_word(&mut spec.records[0], off2, 0x3F80_0000);
    write_word(&mut spec.records[0], off3, 0x4000_0000);
    write_word(&mut spec.records[0], off4, 0x4040_0000);
    let out = decode_idtable(table, &spec).unwrap();
    let row = &out.rows()[&12];
    assert_eq!(row[0][0], Value::Signed(-7));
    assert_eq!(row[1][0], Value::Signed(1_000));
    assert_eq!(row[2][0], Value::FloatBits(0x3F80_0000));
    assert_eq!(row[3][0], Value::FloatBits(0x4000_0000));
    assert_eq!(row[4][0], Value::FloatBits(0x4040_0000));
}

#[test]
fn chr_customization_req_transcription_has_a_signed_array_of_two() {
    // ChrCustomizationReq: String, Signed(32,1) x7, Signed(32,2).
    let schema = CharacterTable::CustomizationReq.schema();
    assert_eq!(schema.layout, 0xCA154412);
    assert_eq!(schema.index, -1);
    assert_eq!(schema.parent, -1);
    assert_eq!(schema.fields.len(), 9);
    assert!(matches!(schema.fields[0], Kind::String));
    for index in 1..8 {
        assert!(
            matches!(schema.fields[index], Kind::Signed(32, 1)),
            "field {index} is not Signed(32,1)"
        );
    }
    // The last field is the reviewed array variant: two signed 32-bit values.
    assert!(matches!(schema.fields[8], Kind::Signed(32, 2)));
}

#[test]
fn chr_customization_req_decodes_string_signed_ints_and_array_of_two() {
    let table = CharacterTable::CustomizationReq;
    let mut spec = idtable_spec(table, 1);
    spec.ids[0] = 345;
    spec.max_id = 1000;
    spec.strings = b"Req\0".to_vec();
    let string_off = field_offset(&spec, 0);
    let relative = record_size(&spec.cols) as u32 - u32::from(string_off) / 8;
    write_word(&mut spec.records[0], string_off, relative);
    let scalars = [-1i32, 0, 7, -2_000_000_000, 2_000_000_000, 42, -8];
    for (field, value) in scalars.into_iter().enumerate() {
        let off = field_offset(&spec, field + 1);
        write_word(&mut spec.records[0], off, value as u32);
    }
    // Field 8 is Signed(32,2): two consecutive 32-bit words per row.
    let array_off = field_offset(&spec, 8);
    let at = array_off as usize / 8;
    spec.records[0][at..at + 4].copy_from_slice(&(-11i32).to_le_bytes());
    spec.records[0][at + 4..at + 8].copy_from_slice(&2_000_000_001u32.to_le_bytes());
    let out = decode_idtable(table, &spec).unwrap();
    let row = &out.rows()[&345];
    assert_eq!(row[0][0], Value::Text("Req".to_owned()));
    for (field, value) in scalars.into_iter().enumerate() {
        assert_eq!(
            row[field + 1][0],
            Value::Signed(value),
            "field {}",
            field + 1
        );
    }
    // Exactly two values decoded for the array field.
    assert_eq!(row[8].len(), 2);
    assert_eq!(row[8][0], Value::Signed(-11));
    assert_eq!(row[8][1], Value::Signed(2_000_000_001));
}

#[test]
fn forever_tables_empty_zero_section_and_declared_records_variants() {
    for (table, _, _, _) in FOREVER_TABLES {
        let bytes = build_zero_section_header(table, 0);
        let out = decode(table, &bytes)
            .unwrap_or_else(|error| panic!("{} rejected as {}", table.name(), error.code()));
        assert_eq!(out.rows().len(), 0, "{} rows", table.name());

        let bytes = build_zero_section_header(table, 1);
        assert_eq!(
            expect_err_idtable(table, &bytes),
            RecordError::Unsupported,
            "{} zero sections with records",
            table.name()
        );
    }
}

// ---------------------------------------------------------------------------
// Forever inline-index tables (No parent, IndexField >= 0)
// ---------------------------------------------------------------------------

/// The reviewed Forever tables whose id is read from an inline index field.
/// Identity is (file_id, layout, index, field_count); table hashes are NOT
/// transcribed because `DB2Meta` does not carry one. All declare
/// `ParentIndexField = -1` and `FieldCount == FileFieldCount`.
const FOREVER_INLINE_TABLES: [(CharacterTable, u32, u32, i16, usize); 3] = [
    (CharacterTable::Curve, 892585, 0xD362535A, 0, 3),
    (
        CharacterTable::ContentRestrictionRuleSet,
        4281307,
        0x5774D75A,
        0,
        3,
    ),
    (CharacterTable::DataTagInfo, 6652252, 0xDB46F772, 1, 3),
];

/// Decode-valid spec for a reviewed inline-index table: zero records of the
/// right size. There is no separate id table (`assemble` only emits one when
/// `schema.index < 0`), and the id is read from the inline column.
fn inline_spec(table: CharacterTable, count: usize) -> Spec {
    let mut spec = spec(table);
    let len = record_size(&spec.cols);
    spec.records = vec![vec![0u8; len]; count];
    spec.min_id = 0;
    spec.max_id = 100;
    spec
}

fn decode_inline(table: CharacterTable, spec: &Spec) -> Result<DecodedTable, RecordError> {
    let bytes = assemble(table, spec);
    DecodedTable::read(&bytes, table)
}

fn expect_err_inline(table: CharacterTable, spec: &Spec) -> RecordError {
    match decode_inline(table, spec) {
        Err(error) => error,
        Ok(_) => panic!("expected error, decode succeeded"),
    }
}

#[test]
fn forever_inline_table_identity_and_transcription_match_the_reference() {
    // Verified against DB2Metadata.h: `Curve` (892585, layout 0xD362535A,
    // IndexField 0), `ContentRestrictionRuleSet` (4281307, 0x5774D75A,
    // IndexField 0) and `DataTagInfo` (6652252, 0xDB46F772, IndexField 1); all
    // declare ParentIndexField -1 and FieldCount == FileFieldCount.
    for (table, file_id, layout, index, fields) in FOREVER_INLINE_TABLES {
        let schema = table.schema();
        assert_eq!(table.file_id(), file_id, "{} file_id", table.name());
        assert_eq!(schema.layout, layout, "{} layout", table.name());
        assert_eq!(schema.index, index, "{} index", table.name());
        assert_eq!(schema.parent, -1, "{} parent", table.name());
        assert_eq!(schema.fields.len(), fields, "{} field count", table.name());
        assert!(
            schema.index >= 0,
            "{} must use an inline index field",
            table.name()
        );
        assert!(
            schema.hash.is_none(),
            "{} must not invent a hash",
            table.name()
        );
        assert_eq!(CharacterTable::named(table.name()), Some(table));
    }
}

#[test]
fn curve_transcription_matches_field_kinds_and_widths() {
    // Curve: [Unsigned(32,1), Unsigned(8,1), Signed(32,1)]. The second field is
    // an UNSIGNED FT_BYTE, not signed.
    let schema = CharacterTable::Curve.schema();
    assert!(matches!(schema.fields[0], Kind::Unsigned(32, 1)));
    assert!(matches!(schema.fields[1], Kind::Unsigned(8, 1)));
    assert!(matches!(schema.fields[2], Kind::Signed(32, 1)));
}

#[test]
fn content_restriction_rule_set_transcription_is_unsigned_then_two_signed_ints() {
    // Field 0 is an UNSIGNED FT_INT in the fixed meta (IsSigned = false); fields
    // 1 and 2 are signed 32-bit fields.
    let schema = CharacterTable::ContentRestrictionRuleSet.schema();
    assert!(matches!(schema.fields[0], Kind::Unsigned(32, 1)));
    assert!(matches!(schema.fields[1], Kind::Signed(32, 1)));
    assert!(matches!(schema.fields[2], Kind::Signed(32, 1)));
}

#[test]
fn data_tag_info_transcription_is_string_and_two_signed_ints() {
    let schema = CharacterTable::DataTagInfo.schema();
    assert!(matches!(schema.fields[0], Kind::String));
    assert!(matches!(schema.fields[1], Kind::Unsigned(32, 1)));
    assert!(matches!(schema.fields[2], Kind::Signed(32, 1)));
}

#[test]
fn forever_inline_tables_match_gates_on_known_identity_not_a_table_hash() {
    // No table hash is transcribed, so any header `TableHash` word is accepted
    // while layout/field/total identity is right (DB2FileLoader.cpp:1763/:1771).
    // The header's own `IndexField` is not a gate (DB2FileLoader.cpp:1746 only
    // endian-converts it; inline vs id-table comes from the reviewed meta).
    for (table, _, layout, _, fields) in FOREVER_INLINE_TABLES {
        let mut spec = inline_spec(table, 1);
        write_inline_id_at(&mut spec, table.schema().index as usize, 7, 0);
        let mut bytes = assemble(table, &spec);
        let header = crate::Metadata::read(&mut std::io::Cursor::new(&bytes))
            .unwrap()
            .header;
        assert_eq!(header.layout_hash, layout, "{} layout", table.name());
        assert_eq!(
            header.field_count as usize,
            fields,
            "{} fields",
            table.name()
        );
        assert_eq!(
            header.total_field_count as usize,
            fields,
            "{} total fields",
            table.name()
        );
        assert!(table.matches(&header), "{} identity", table.name());

        put32(&mut bytes, 152, 0xDEAD_BEEF);
        let header = crate::Metadata::read(&mut std::io::Cursor::new(&bytes))
            .unwrap()
            .header;
        assert!(table.matches(&header), "{} arbitrary hash", table.name());
    }
}

#[test]
fn forever_inline_tables_reject_layout_field_and_total_mismatches() {
    for (table, _, _, _, _) in FOREVER_INLINE_TABLES {
        let mut bytes = assemble(table, &inline_spec(table, 1));
        put32(&mut bytes, 156, 0);
        assert_eq!(
            record_error(table, &bytes),
            RecordError::Schema,
            "{} layout",
            table.name()
        );

        let mut bytes = assemble(table, &inline_spec(table, 1));
        put32(&mut bytes, 140, 42);
        assert_eq!(
            record_error(table, &bytes),
            RecordError::Schema,
            "{} field_count",
            table.name()
        );

        let mut bytes = assemble(table, &inline_spec(table, 1));
        let fields = table.schema().fields.len() as u32;
        put32(&mut bytes, 176, fields + 1);
        assert_eq!(
            record_error(table, &bytes),
            RecordError::Schema,
            "{} total_field_count",
            table.name()
        );
    }
}

#[test]
fn curve_inline_id_is_decoded_from_field_zero() {
    // Route the row id through the inline index field (field 0, Unsigned(32,1))
    // rather than a separate id table. Field 1 is the unsigned FT_BYTE id data.
    let table = CharacterTable::Curve;
    let mut spec = inline_spec(table, 1);
    spec.max_id = 100_000;
    let off0 = field_offset(&spec, 0);
    let off1 = field_offset(&spec, 1);
    let off2 = field_offset(&spec, 2);
    write_word(&mut spec.records[0], off0, 500);
    write_byte(&mut spec.records[0], off1, 0xFF);
    write_word(&mut spec.records[0], off2, (-3i32) as u32);
    let out = decode_inline(table, &spec).unwrap();
    let keys = out.rows().keys().copied().collect::<Vec<_>>();
    assert_eq!(keys, vec![500]);
    let row = &out.rows()[&500];
    assert_eq!(row[0][0], Value::Unsigned(500));
    // Unsigned(8,1): 0xFF stays 255, not -1.
    assert_eq!(row[1][0], Value::Unsigned(255));
    assert_eq!(row[2][0], Value::Signed(-3));
}

#[test]
fn content_restriction_rule_set_inline_id_is_decoded_from_field_zero() {
    let table = CharacterTable::ContentRestrictionRuleSet;
    let mut spec = inline_spec(table, 1);
    spec.max_id = 100_000;
    let off0 = field_offset(&spec, 0);
    let off1 = field_offset(&spec, 1);
    let off2 = field_offset(&spec, 2);
    write_word(&mut spec.records[0], off0, 12345);
    write_word(&mut spec.records[0], off1, (-7i32) as u32);
    write_word(&mut spec.records[0], off2, 9);
    let out = decode_inline(table, &spec).unwrap();
    let keys = out.rows().keys().copied().collect::<Vec<_>>();
    assert_eq!(keys, vec![12345]);
    let row = &out.rows()[&12345];
    assert_eq!(row[0][0], Value::Unsigned(12345));
    assert_eq!(row[1][0], Value::Signed(-7));
    assert_eq!(row[2][0], Value::Signed(9));
}

#[test]
fn data_tag_info_inline_id_is_decoded_from_field_one() {
    // The reviewed inline index for DataTagInfo is field 1, so a non-index
    // field (0) may hold an arbitrary value without affecting the row key.
    let table = CharacterTable::DataTagInfo;
    let mut spec = inline_spec(table, 1);
    spec.max_id = 100_000;
    spec.strings = b"Tag\0".to_vec();
    let off0 = field_offset(&spec, 0);
    let off1 = field_offset(&spec, 1);
    let off2 = field_offset(&spec, 2);
    let relative = record_size(&spec.cols) as u32 - u32::from(off0) / 8;
    write_word(&mut spec.records[0], off0, relative);
    write_word(&mut spec.records[0], off1, 8080);
    write_word(&mut spec.records[0], off2, (-42i32) as u32);
    let out = decode_inline(table, &spec).unwrap();
    let keys = out.rows().keys().copied().collect::<Vec<_>>();
    assert_eq!(keys, vec![8080]);
    let row = &out.rows()[&8080];
    assert_eq!(row[0][0], Value::Text("Tag".to_owned()));
    assert_eq!(row[1][0], Value::Unsigned(8080));
    assert_eq!(row[2][0], Value::Signed(-42));
}

#[test]
fn inline_index_is_not_repaired_below_min_id_for_the_forever_tables() {
    // Inline semantics: a raw inline id below MinId is used as-is, never
    // repaired (DB2FileLoader.cpp:747-760). Exercise each reviewed inline table.
    let (table, raw, min) = (CharacterTable::Curve, 4u32, 50u32);
    let mut spec = inline_spec(table, 1);
    spec.min_id = min;
    spec.max_id = 100;
    let off = field_offset(&spec, 0);
    write_word(&mut spec.records[0], off, raw);
    let out = decode_inline(table, &spec).unwrap();
    assert_eq!(out.rows().keys().copied().collect::<Vec<_>>(), vec![raw]);

    let (table, raw, min) = (CharacterTable::ContentRestrictionRuleSet, 4u32, 50u32);
    let mut spec = inline_spec(table, 1);
    spec.min_id = min;
    spec.max_id = 100;
    let off = field_offset(&spec, 0);
    write_word(&mut spec.records[0], off, raw);
    let out = decode_inline(table, &spec).unwrap();
    assert_eq!(out.rows().keys().copied().collect::<Vec<_>>(), vec![raw]);

    let (table, raw, min) = (CharacterTable::DataTagInfo, 4u32, 50u32);
    let mut spec = inline_spec(table, 1);
    spec.min_id = min;
    spec.max_id = 100;
    let off = field_offset(&spec, 1);
    write_word(&mut spec.records[0], off, raw);
    let out = decode_inline(table, &spec).unwrap();
    assert_eq!(out.rows().keys().copied().collect::<Vec<_>>(), vec![raw]);
}

#[test]
fn forever_inline_tables_reject_out_of_range_inline_ids() {
    for (table, _, _, index, _) in FOREVER_INLINE_TABLES {
        let mut spec = inline_spec(table, 1);
        spec.min_id = 0;
        spec.max_id = 100;
        write_inline_id_at(&mut spec, index as usize, 200, 0);
        assert_eq!(
            expect_err_inline(table, &spec),
            RecordError::Id,
            "{} out-of-range id",
            table.name()
        );
    }
}

#[test]
fn forever_inline_copy_table_rewrites_the_inline_id() {
    // AutoProduceRecordCopies (DB2FileLoader.cpp:613-634) plus the inline-index
    // rewrite: a copied row's inline index column is set to the new row id.
    for (table, _, _, index, _) in FOREVER_INLINE_TABLES {
        let index = index as usize;
        let mut spec = inline_spec(table, 3);
        spec.min_id = 0;
        spec.max_id = 1000;
        write_inline_id_at(&mut spec, index, 1, 0);
        write_inline_id_at(&mut spec, index, 2, 1);
        write_inline_id_at(&mut spec, index, 3, 2);
        // Copy the row keyed 1 into new row id 2; the destination inline index
        // column is rewritten to 2 (DB2FileLoader.cpp:613-634).
        spec.copies = vec![(2, 1)];
        let out = decode_inline(table, &spec).unwrap();
        assert!(out.rows().contains_key(&2), "{} copy row", table.name());
        assert_eq!(
            out.rows()[&2][index][0],
            Value::Unsigned(2),
            "{} rewritten inline id",
            table.name()
        );
    }
}

#[test]
fn forever_inline_tables_empty_zero_section_and_declared_records_variants() {
    for (table, _, _, _, _) in FOREVER_INLINE_TABLES {
        let bytes = build_zero_section_header(table, 0);
        let out = decode(table, &bytes)
            .unwrap_or_else(|error| panic!("{} rejected as {}", table.name(), error.code()));
        assert_eq!(out.rows().len(), 0, "{} rows", table.name());

        let bytes = build_zero_section_header(table, 1);
        assert_eq!(
            record_error(table, &bytes),
            RecordError::Unsupported,
            "{} zero sections with records",
            table.name()
        );
    }
}

// ---------------------------------------------------------------------------
// Forever Anim* byte tables (No parent, IndexField -1, 1 or 2 byte fields)
// ---------------------------------------------------------------------------

/// The reviewed Forever `Anim*` tables, the first with a one-field file field
/// count. Identity is (file_id, layout, field_count); table hashes are NOT
/// transcribed because `DB2Meta` does not carry one. All declare
/// `IndexField = -1`, `ParentIndexField = -1` and `FieldCount == FileFieldCount`.
const FOREVER_BYTE_TABLES: [(CharacterTable, u32, u32, usize); 3] = [
    (CharacterTable::AnimKitBoneSetAlias, 1266937, 0x19A6986C, 2),
    (CharacterTable::AnimKitPriority, 1266540, 0xCCF889D8, 1),
    (CharacterTable::AnimReplacementSet, 1284817, 0x7C047695, 1),
];

#[test]
fn forever_byte_table_identity_and_transcription_match_the_reference() {
    // Verified against DB2Metadata.h: each declares FileDataId, IndexField = -1,
    // ParentIndexField = -1 and a LayoutHash; FieldCount == FileFieldCount.
    for (table, file_id, layout, fields) in FOREVER_BYTE_TABLES {
        let schema = table.schema();
        assert_eq!(table.file_id(), file_id, "{} file_id", table.name());
        assert_eq!(schema.layout, layout, "{} layout", table.name());
        assert_eq!(schema.index, -1, "{} index", table.name());
        assert_eq!(schema.parent, -1, "{} parent", table.name());
        assert_eq!(schema.fields.len(), fields, "{} field count", table.name());
        assert!(
            schema.hash.is_none(),
            "{} must not invent a hash",
            table.name()
        );
        assert_eq!(CharacterTable::named(table.name()), Some(table));
    }
}

#[test]
fn anim_kit_bone_set_alias_transcription_is_two_unsigned_bytes() {
    let schema = CharacterTable::AnimKitBoneSetAlias.schema();
    assert_eq!(schema.fields.len(), 2);
    assert!(matches!(schema.fields[0], Kind::Unsigned(8, 1)));
    assert!(matches!(schema.fields[1], Kind::Unsigned(8, 1)));
}

#[test]
fn anim_kit_priority_and_replacement_set_are_single_unsigned_bytes() {
    // The first reviewed one-field file field counts: each has exactly one
    // unsigned 8-bit column and a separate u32 id table.
    for table in [
        CharacterTable::AnimKitPriority,
        CharacterTable::AnimReplacementSet,
    ] {
        let schema = table.schema();
        assert_eq!(schema.fields.len(), 1, "{} field count", table.name());
        assert!(
            matches!(schema.fields[0], Kind::Unsigned(8, 1)),
            "{} single field",
            table.name()
        );
    }
}

#[test]
fn forever_byte_table_matches_gates_on_known_identity_not_a_table_hash() {
    // No table hash is transcribed, so any header `TableHash` word must be
    // accepted while layout/field/total identity is right (DB2FileLoader.cpp
    // validates LayoutHash at :1763 and the field count at :1771).
    for (table, _, layout, fields) in FOREVER_BYTE_TABLES {
        let mut spec = idtable_spec(table, 1);
        spec.ids[0] = 7;
        let mut bytes = build_idtable(table, &spec);
        let header = crate::Metadata::read(&mut std::io::Cursor::new(&bytes))
            .unwrap()
            .header;
        assert_eq!(header.layout_hash, layout, "{} layout", table.name());
        assert_eq!(
            header.field_count as usize,
            fields,
            "{} fields",
            table.name()
        );
        assert_eq!(
            header.total_field_count as usize,
            fields,
            "{} total fields",
            table.name()
        );
        assert!(table.matches(&header), "{} identity", table.name());
        let out = decode_idtable(table, &spec).unwrap();
        assert_eq!(out.rows().keys().copied().collect::<Vec<_>>(), vec![7]);

        put32(&mut bytes, 152, 0xDEAD_BEEF);
        let header = crate::Metadata::read(&mut std::io::Cursor::new(&bytes))
            .unwrap()
            .header;
        assert!(table.matches(&header), "{} arbitrary hash", table.name());
    }
}

#[test]
fn forever_byte_table_matches_rejects_layout_field_and_total_mismatches() {
    for (table, _, _, _) in FOREVER_BYTE_TABLES {
        let mut bytes = build_idtable(table, &idtable_spec(table, 1));
        put32(&mut bytes, 156, 0);
        assert_eq!(
            expect_err_idtable(table, &bytes),
            RecordError::Schema,
            "{} layout",
            table.name()
        );

        let mut bytes = build_idtable(table, &idtable_spec(table, 1));
        put32(&mut bytes, 140, 42);
        assert_eq!(
            expect_err_idtable(table, &bytes),
            RecordError::Schema,
            "{} field_count",
            table.name()
        );

        let mut bytes = build_idtable(table, &idtable_spec(table, 1));
        let fields = table.schema().fields.len() as u32;
        put32(&mut bytes, 176, fields + 1);
        assert_eq!(
            expect_err_idtable(table, &bytes),
            RecordError::Schema,
            "{} total_field_count",
            table.name()
        );
    }
}

#[test]
fn anim_kit_bone_set_alias_decodes_two_unsigned_bytes() {
    let table = CharacterTable::AnimKitBoneSetAlias;
    let mut spec = idtable_spec(table, 1);
    spec.ids[0] = 3;
    let off0 = field_offset(&spec, 0);
    let off1 = field_offset(&spec, 1);
    write_byte(&mut spec.records[0], off0, 0xFF);
    write_byte(&mut spec.records[0], off1, 0x01);
    let out = decode_idtable(table, &spec).unwrap();
    let row = &out.rows()[&3];
    // Unsigned(8,1): 0xFF stays 255, not -1.
    assert_eq!(row[0][0], Value::Unsigned(255));
    assert_eq!(row[1][0], Value::Unsigned(1));
}

#[test]
fn anim_kit_priority_decodes_one_unsigned_byte() {
    // A one-field table: the decoded row has exactly one field group.
    let table = CharacterTable::AnimKitPriority;
    let mut spec = idtable_spec(table, 1);
    spec.ids[0] = 9;
    let off = field_offset(&spec, 0);
    write_byte(&mut spec.records[0], off, 0x80);
    let out = decode_idtable(table, &spec).unwrap();
    let row = &out.rows()[&9];
    assert_eq!(row.len(), 1);
    assert_eq!(row[0][0], Value::Unsigned(128));
}

#[test]
fn anim_replacement_set_decodes_one_unsigned_byte() {
    let table = CharacterTable::AnimReplacementSet;
    let mut spec = idtable_spec(table, 1);
    spec.ids[0] = 42;
    let off = field_offset(&spec, 0);
    write_byte(&mut spec.records[0], off, 0x7F);
    let out = decode_idtable(table, &spec).unwrap();
    let row = &out.rows()[&42];
    assert_eq!(row.len(), 1);
    assert_eq!(row[0][0], Value::Unsigned(127));
}

#[test]
fn forever_byte_table_id_table_repair_matches_the_separate_id_path() {
    // IndexField -1 uses the separate u32 id-table path: DB2FileLoader.cpp:1958
    // repairs `id <= MinId` to `MinId + i` for the single-section ordinal.
    for (table, _, _, _) in FOREVER_BYTE_TABLES {
        let mut spec = idtable_spec(table, 1);
        spec.min_id = 50;
        spec.max_id = 100;
        spec.ids[0] = 3;
        let out = decode_idtable(table, &spec).unwrap();
        assert_eq!(
            out.rows().keys().copied().collect::<Vec<_>>(),
            vec![50],
            "{} repair",
            table.name()
        );
    }
}

#[test]
fn forever_byte_tables_empty_zero_section_and_declared_records_variants() {
    for (table, _, _, _) in FOREVER_BYTE_TABLES {
        let bytes = build_zero_section_header(table, 0);
        let out = decode(table, &bytes)
            .unwrap_or_else(|error| panic!("{} rejected as {}", table.name(), error.code()));
        assert_eq!(out.rows().len(), 0, "{} rows", table.name());

        let bytes = build_zero_section_header(table, 1);
        assert_eq!(
            expect_err_idtable(table, &bytes),
            RecordError::Unsupported,
            "{} zero sections with records",
            table.name()
        );
    }
}

// ---------------------------------------------------------------------------
// FT_LONG (unsigned 64-bit) tables: ItemExtendedCost / TransmogOutfitEntry /
// BankTab (No parent; first reviewed tables with a 64-bit field)
// ---------------------------------------------------------------------------

/// The three reviewed Forever `FT_LONG` tables with their pinned identity. No
/// table hash is transcribed (`DB2Meta` carries none), so the trustworthy
/// identity is (layout_hash, field_count, index, file_id).
const FOREVER_LONG_TABLES: [(CharacterTable, u32, u32, i16, usize); 3] = [
    (CharacterTable::ItemExtendedCost, 801681, 0x22331DAB, -1, 11),
    (
        CharacterTable::TransmogOutfitEntry,
        7320164,
        0x91D51304,
        2,
        9,
    ),
    (CharacterTable::BankTab, 5242411, 0x5B2F9B36, -1, 9),
];

#[test]
fn forever_long_table_identity_and_transcription_match_the_reference() {
    // Verified against DB2Metadata.h at 02245dcd: each table declares
    // FileDataId, IndexField, ParentIndexField = -1 and a LayoutHash;
    // FieldCount == FileFieldCount for all three.
    for (table, file_id, layout, index, fields) in FOREVER_LONG_TABLES {
        let schema = table.schema();
        assert_eq!(table.file_id(), file_id, "{} file_id", table.name());
        assert_eq!(schema.layout, layout, "{} layout", table.name());
        assert_eq!(schema.index, index, "{} index", table.name());
        assert_eq!(schema.parent, -1, "{} parent", table.name());
        assert_eq!(schema.fields.len(), fields, "{} field count", table.name());
        assert!(
            schema.hash.is_none(),
            "{} must not invent a hash",
            table.name()
        );
        assert_eq!(CharacterTable::named(table.name()), Some(table));
    }
}

#[test]
fn forever_long_tables_transcribe_their_64_bit_leading_field() {
    // Every reviewed FT_LONG table starts with exactly one Unsigned(64,1) field
    // whose `bits()` reports width 64.
    for (table, _, _, _, _) in FOREVER_LONG_TABLES {
        let schema = table.schema();
        assert!(
            matches!(schema.fields[0], Kind::Unsigned(64, 1)),
            "{} leading field",
            table.name()
        );
        assert_eq!(schema.fields[0].bits(), 64, "{} width", table.name());
        assert_eq!(schema.fields[0].count(), 1, "{} count", table.name());
    }
    // The transcribed per-table field shapes.
    let item = CharacterTable::ItemExtendedCost.schema();
    assert!(matches!(item.fields[7], Kind::Signed(32, 5)));
    assert!(matches!(item.fields[8], Kind::Unsigned(16, 5)));
    assert!(matches!(item.fields[9], Kind::Unsigned(16, 5)));
    assert!(matches!(item.fields[10], Kind::Unsigned(32, 5)));

    let transmog = CharacterTable::TransmogOutfitEntry.schema();
    assert!(matches!(transmog.fields[1], Kind::String));
    assert!(matches!(transmog.fields[2], Kind::Unsigned(32, 1)));
    assert!(matches!(transmog.fields[7], Kind::Float(1)));

    let bank = CharacterTable::BankTab.schema();
    assert!(matches!(bank.fields[1], Kind::Unsigned(8, 1)));
    assert!(matches!(bank.fields[2], Kind::Signed(8, 1)));
    for field in 3..9 {
        assert!(
            matches!(bank.fields[field], Kind::Signed(32, 1)),
            "bank {field}"
        );
    }
}

#[test]
fn item_extended_cost_decodes_u64_and_arrays() {
    // ItemExtendedCost is an id-table table (IndexField -1). Field 0 is the
    // 64-bit FT_LONG; field 7 is a Signed(32,5) array and fields 8/9/10 are
    // Unsigned(16,5)/Unsigned(16,5)/Unsigned(32,5) arrays.
    let table = CharacterTable::ItemExtendedCost;
    let mut spec = idtable_spec(table, 1);
    spec.ids[0] = 12;
    // A value above u32::MAX, so truncation to 32 bits would be caught.
    let big = 0x1_0000_0001u64;
    let off = field_offset(&spec, 0);
    write_u64(&mut spec.records[0], off, big);
    for (array, value) in [-1i32, 0, 7, -2_000_000_000, 2_000_000_000]
        .into_iter()
        .enumerate()
    {
        let off = field_offset(&spec, 7);
        let at = off as usize / 8 + array * 4;
        spec.records[0][at..at + 4].copy_from_slice(&(value as u32).to_le_bytes());
    }
    let out = decode_idtable(table, &spec).unwrap();
    let row = &out.rows()[&12];
    assert_eq!(row[0][0], Value::Unsigned64(big));
    assert_eq!(
        row[7],
        vec![
            Value::Signed(-1),
            Value::Signed(0),
            Value::Signed(7),
            Value::Signed(-2_000_000_000),
            Value::Signed(2_000_000_000),
        ]
    );
    // Unsigned 16-bit array at field 8.
    let off = field_offset(&spec, 8);
    let at = off as usize / 8;
    spec.records[0][at..at + 2].copy_from_slice(&0xFFFFu16.to_le_bytes());
    let out = decode_idtable(table, &spec).unwrap();
    assert_eq!(out.rows()[&12][8][0], Value::Unsigned(0xFFFF));
}

#[test]
fn transmog_outfit_entry_uses_inline_field_two_id() {
    // TransmogOutfitEntry declares IndexField 2, so no separate id table is
    // present and the row key is the inline unsigned 32-bit field 2.
    let table = CharacterTable::TransmogOutfitEntry;
    let mut spec = spec(table);
    let len = record_size(&spec.cols);
    spec.records = vec![vec![0u8; len]];
    spec.min_id = 0;
    spec.max_id = 100;
    write_inline_id_at(&mut spec, 2, 42, 0);
    let off = field_offset(&spec, 0);
    write_u64(&mut spec.records[0], off, 0xFFFF_FFFF_FFFF_FFFF);
    let out = decode_idtable(table, &spec).unwrap();
    let keys = out.rows().keys().copied().collect::<Vec<_>>();
    assert_eq!(keys, vec![42]);
    assert_eq!(
        out.rows()[&42][0][0],
        Value::Unsigned64(0xFFFF_FFFF_FFFF_FFFF)
    );
    // The inline id field itself decodes as an ordinary unsigned 32-bit value.
    assert_eq!(out.rows()[&42][2][0], Value::Unsigned(42));
}

#[test]
fn bank_tab_decodes_u64_byte_signed_and_six_int_fields() {
    // BankTab is an id-table table (IndexField -1): leading Unsigned(64,1), then
    // Unsigned(8,1), Signed(8,1) and six Signed(32,1) fields.
    let table = CharacterTable::BankTab;
    let mut spec = idtable_spec(table, 1);
    spec.ids[0] = 3;
    let off0 = field_offset(&spec, 0);
    write_u64(&mut spec.records[0], off0, 0x1_0000_0001);
    let off1 = field_offset(&spec, 1);
    write_byte(&mut spec.records[0], off1, 0xFF);
    let off2 = field_offset(&spec, 2);
    write_byte(&mut spec.records[0], off2, 0x80);
    let values = [-1i32, 0, 7, -2_000_000_000, 2_000_000_000, 42];
    for (field, value) in values.into_iter().enumerate() {
        let off = field_offset(&spec, field + 3);
        write_word(&mut spec.records[0], off, value as u32);
    }
    let out = decode_idtable(table, &spec).unwrap();
    let row = &out.rows()[&3];
    assert_eq!(row[0][0], Value::Unsigned64(0x1_0000_0001));
    assert_eq!(row[1][0], Value::Unsigned(255));
    assert_eq!(row[2][0], Value::Signed(-128));
    for (field, value) in values.into_iter().enumerate() {
        assert_eq!(
            row[field + 3][0],
            Value::Signed(value),
            "bank field {field}"
        );
    }
}

#[test]
fn forever_long_table_matches_gates_on_known_identity_not_a_table_hash() {
    // No table hash is transcribed, so any header `TableHash` word must be
    // accepted while layout/field/total identity is right.
    for (table, _, layout, _, fields) in FOREVER_LONG_TABLES {
        let mut spec = if table.schema().index < 0 {
            idtable_spec(table, 1)
        } else {
            let mut spec = spec(table);
            let len = record_size(&spec.cols);
            spec.records = vec![vec![0u8; len]];
            spec.min_id = 0;
            spec.max_id = 100;
            spec
        };
        if table.schema().index < 0 {
            spec.ids[0] = 7;
        } else {
            write_inline_id_at(&mut spec, table.schema().index as usize, 7, 0);
        }
        let mut bytes = build_idtable(table, &spec);
        let header = crate::Metadata::read(&mut std::io::Cursor::new(&bytes))
            .unwrap()
            .header;
        assert_eq!(header.layout_hash, layout, "{} layout", table.name());
        assert_eq!(
            header.field_count as usize,
            fields,
            "{} fields",
            table.name()
        );
        assert!(table.matches(&header), "{} identity", table.name());
        let out = decode(table, &bytes).unwrap();
        assert_eq!(out.rows().keys().copied().collect::<Vec<_>>(), vec![7]);

        put32(&mut bytes, 152, 0xDEAD_BEEF);
        let header = crate::Metadata::read(&mut std::io::Cursor::new(&bytes))
            .unwrap()
            .header;
        assert!(table.matches(&header), "{} arbitrary hash", table.name());
    }
}

#[test]
fn forever_long_tables_empty_zero_section_variant() {
    for (table, _, _, _, _) in FOREVER_LONG_TABLES {
        let bytes = build_zero_section_header(table, 0);
        let out = decode(table, &bytes)
            .unwrap_or_else(|error| panic!("{} rejected as {}", table.name(), error.code()));
        assert_eq!(out.rows().len(), 0, "{} rows", table.name());
    }
}
