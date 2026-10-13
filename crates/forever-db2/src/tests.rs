// SPDX-License-Identifier: GPL-3.0-or-later
use super::*;
use std::io::Cursor;

fn header(sections: u32) -> Vec<u8> {
    let mut bytes = vec![0; HEADER_SIZE];
    bytes[..4].copy_from_slice(b"WDC5");
    bytes[4..8].copy_from_slice(&5u32.to_le_bytes());
    bytes[200..204].copy_from_slice(&sections.to_le_bytes());
    bytes
}

#[test]
fn every_header_word_and_signed_index_has_the_packed_offset() {
    let mut bytes = header(0);
    for at in (136..=168).step_by(4).chain((176..=196).step_by(4)) {
        bytes[at..at + 4].copy_from_slice(&(0x8000_0000 | at as u32).to_le_bytes());
    }
    bytes[172..174].copy_from_slice(&0x8041u16.to_le_bytes());
    bytes[174..176].copy_from_slice(&(-1i16).to_le_bytes());
    bytes[184..188].copy_from_slice(&1u32.to_le_bytes());
    let h = Metadata::read(&mut Cursor::new(bytes)).unwrap().header;
    assert_eq!(h.record_count, 0x8000_0088);
    assert_eq!(h.field_count, 0x8000_008c);
    assert_eq!(h.record_size, 0x8000_0090);
    assert_eq!(h.string_table_size, 0x8000_0094);
    assert_eq!(h.table_hash, 0x8000_0098);
    assert_eq!(h.layout_hash, 0x8000_009c);
    assert_eq!(h.min_id, 0x8000_00a0);
    assert_eq!(h.max_id, 0x8000_00a4);
    assert_eq!(h.locale_mask, 0x8000_00a8);
    assert_eq!(h.flags, 0x8041);
    assert_eq!(h.index_field, -1);
    assert_eq!(h.total_field_count, 0x8000_00b0);
    assert_eq!(h.packed_data_offset, 0x8000_00b4);
    assert_eq!(h.parent_lookup_count, 1);
    assert_eq!(h.column_meta_size, 0x8000_00bc);
    assert_eq!(h.common_data_size, 0x8000_00c0);
    assert_eq!(h.pallet_data_size, 0x8000_00c4);
    assert_eq!(h.section_count, 0);
}

#[test]
fn preserves_section_order_all_words_and_nonzero_tact_declaration() {
    let mut bytes = header(2);
    let first = [
        0u64.to_le_bytes().to_vec(),
        (8..40u32).step_by(4).flat_map(u32::to_le_bytes).collect(),
    ]
    .concat();
    let mut second = first.clone();
    second[7] = 0x80;
    bytes.extend(&first);
    bytes.extend(&second);
    let parsed = Metadata::read(&mut Cursor::new(bytes)).unwrap();
    let s = parsed.sections[0];
    assert!(!s.tact_declared);
    assert!(parsed.sections[1].tact_declared);
    assert_eq!(s.file_offset, 8);
    assert_eq!(s.record_count, 12);
    assert_eq!(s.string_table_size, 16);
    assert_eq!(s.catalog_data_offset, 20);
    assert_eq!(s.id_table_size, 24);
    assert_eq!(s.parent_lookup_data_size, 28);
    assert_eq!(s.catalog_data_count, 32);
    assert_eq!(s.copy_table_count, 36);
}

#[test]
fn reads_no_record_or_column_data_even_when_declared_counts_are_huge() {
    let mut bytes = header(1);
    bytes[136..140].copy_from_slice(&u32::MAX.to_le_bytes());
    bytes[140..144].copy_from_slice(&u32::MAX.to_le_bytes());
    bytes.extend([0; SECTION_SIZE]);
    bytes.extend(b"DO NOT READ RECORD DATA");
    let mut reader = Cursor::new(bytes);
    Metadata::read(&mut reader).unwrap();
    assert_eq!(reader.position(), (HEADER_SIZE + SECTION_SIZE) as u64);
}

#[test]
fn rejects_every_truncated_header_and_section_prefix() {
    let mut bytes = header(2);
    bytes.extend([0; 2 * SECTION_SIZE]);
    for end in 0..bytes.len() {
        assert_eq!(
            Metadata::read(&mut Cursor::new(&bytes[..end])),
            Err(Error::Truncated)
        );
    }
    assert!(Metadata::read(&mut Cursor::new(bytes)).is_ok());
}

#[test]
fn wrong_signature_version_parent_count_and_allocation_limit_reject() {
    for (at, value, expected) in [
        (0, u32::from_le_bytes(*b"WDC4"), Error::UnsupportedSignature),
        (4, 4, Error::UnsupportedVersion),
        (184, 2, Error::ParentLookupCount),
        (200, MAX_SECTIONS + 1, Error::SectionLimit),
        (200, u32::MAX, Error::SectionLimit),
    ] {
        let mut bytes = header(0);
        bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
        assert_eq!(Metadata::read(&mut Cursor::new(bytes)), Err(expected));
    }
}

#[test]
fn exact_section_limit_is_admitted_without_reading_the_body() {
    let mut bytes = header(MAX_SECTIONS);
    bytes.resize(HEADER_SIZE + MAX_SECTIONS as usize * SECTION_SIZE, 0);
    assert_eq!(
        Metadata::read(&mut Cursor::new(bytes))
            .unwrap()
            .sections
            .len(),
        MAX_SECTIONS as usize
    );
}

#[test]
fn io_failure_is_redacted_and_distinct_from_truncation() {
    struct Broken;
    impl Read for Broken {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::other("/private/client/name/secret"))
        }
    }
    let error = Metadata::read(&mut Broken).unwrap_err();
    assert_eq!(error, Error::Io);
    assert_eq!(error.to_string(), "input_io");
}
