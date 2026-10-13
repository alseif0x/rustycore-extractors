// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: authored/public nonsecret keys only; no client assets.
use super::*;

fn record(name: u64, value: u8) -> String {
    format!("{name:016x}\t{}\n", format!("{value:02x}").repeat(16))
}
fn fixture(records: &[(u64, u8)]) -> Vec<u8> {
    let mut bytes = HEADER.to_vec();
    for &(name, value) in records {
        bytes.extend_from_slice(record(name, value).as_bytes());
    }
    bytes
}
fn error(bytes: &[u8]) -> Error {
    Keyring::parse(bytes)
        .err()
        .expect("synthetic input must reject")
}
fn hex<const N: usize>(text: &str) -> [u8; N] {
    assert_eq!(text.len(), N * 2);
    let mut result = [0; N];
    for (output, pair) in result.iter_mut().zip(text.as_bytes().as_chunks::<2>().0) {
        *output = u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap();
    }
    result
}

#[test]
fn parser_retains_only_explicit_sorted_material_and_capacity() {
    let bytes = fixture(&[(0, 0), (0x0123456789abcdef, 0x12), (u64::MAX, 0xff)]);
    let ring = Keyring::parse(&bytes).unwrap();
    assert_eq!(ring.len(), 3);
    assert!(!ring.is_empty());
    assert_eq!(ring.key(0), Some(&[0; 16]));
    assert_eq!(ring.key(0x0123456789abcdef), Some(&[0x12; 16]));
    assert_eq!(ring.key(u64::MAX), Some(&[0xff; 16]));
    assert_eq!(ring.key(1), None);
    assert_eq!(ring.key(u64::MAX - 1), None);
    assert_eq!(
        ring.resident_bytes(),
        ring.entries.capacity() * std::mem::size_of::<Entry>()
    );
    assert!(ring.resident_bytes() >= ring.len() * std::mem::size_of::<Entry>());
}
#[test]
fn parser_decodes_key_bytes_without_numeric_endian_alias() {
    let bytes = [
        HEADER,
        b"0000000000000001\t000102030405060708090a0b0c0d0e0f\n",
    ]
    .concat();
    let ring = Keyring::parse(&bytes).unwrap();
    assert_eq!(ring.key(1), Some(&std::array::from_fn(|i| i as u8)));
}
#[test]
fn parser_admits_exact_entry_ceiling() {
    let mut bytes = HEADER.to_vec();
    for name in 0..MAX_ENTRIES {
        bytes.extend_from_slice(record(name as u64, (name % 256) as u8).as_bytes());
    }
    assert!(bytes.len() < MAX_BYTES);
    let ring = Keyring::parse(&bytes).unwrap();
    assert_eq!(ring.len(), MAX_ENTRIES);
    assert_eq!(ring.key((MAX_ENTRIES - 1) as u64), Some(&[255; 16]));
    assert!(ring.resident_bytes() <= MAX_ENTRIES * std::mem::size_of::<Entry>());
    bytes.extend_from_slice(record(MAX_ENTRIES as u64, 0).as_bytes());
    assert_eq!(error(&bytes), Error::Limit);
}
#[test]
fn parser_rejects_byte_ceiling_before_interpreting_input() {
    assert_eq!(error(&vec![b'x'; MAX_BYTES + 1]), Error::Limit);
    assert_eq!(error(&vec![b'x'; MAX_BYTES]), Error::Format);
}
#[test]
fn parser_rejects_empty_truncated_unknown_or_aliased_header() {
    for bytes in [
        b"".as_slice(),
        HEADER,
        b"forever-tact-keys\t2\n",
        b"forever-tact-keys\t01\n",
        b"Forever-tact-keys\t1\n",
        b"forever-tact-keys\t1\r\n",
        b"forever-tact-keys 1\n",
    ] {
        assert_eq!(error(bytes), Error::Format);
    }
    let valid = fixture(&[(1, 2)]);
    for prefix in 0..valid.len() {
        assert!(Keyring::parse(&valid[..prefix]).is_err());
    }
}
#[test]
fn parser_rejects_duplicate_or_descending_names() {
    assert_eq!(error(&fixture(&[(1, 2), (1, 2)])), Error::Order);
    assert_eq!(error(&fixture(&[(1, 2), (1, 3)])), Error::Order);
    assert_eq!(error(&fixture(&[(2, 2), (1, 3)])), Error::Order);
}
#[test]
fn parser_rejects_noncanonical_case_and_non_hex_values() {
    let valid = fixture(&[(0xabcdef, 0xab)]);
    for (index, byte) in [
        (HEADER.len(), b'A'),
        (HEADER.len() + 15, b'g'),
        (HEADER.len() + 17, b'A'),
        (HEADER.len() + 48, b'g'),
        (HEADER.len() + 16, b' '),
        (HEADER.len() + 49, b'\r'),
        (HEADER.len() + 17, 0xff),
        (HEADER.len() + 17, 0),
    ] {
        let mut bytes = valid.clone();
        bytes[index] = byte;
        assert_eq!(error(&bytes), Error::Format);
    }
}
#[test]
fn parser_rejects_whitespace_comments_blank_lines_and_extra_fields() {
    let valid = String::from_utf8(fixture(&[(1, 2)])).unwrap();
    for bytes in [
        format!("{valid}\n"),
        format!("{valid}# comment\n"),
        format!(" {valid}"),
        valid.replace('\n', "\r\n"),
        valid.replace("\t0202", "\t 0202"),
        valid.replace("\t0202", "\t\t0202"),
        valid.replace("0202\n", "0202 \n"),
    ] {
        assert!(Keyring::parse(bytes.as_bytes()).is_err());
    }
}
#[test]
fn parser_rejects_short_long_and_other_key_sizes() {
    for (name, value) in [
        ("1", "00".repeat(16)),
        ("00000000000000001", "00".repeat(16)),
        ("0000000000000001", "00".repeat(15)),
        ("0000000000000001", "00".repeat(17)),
        ("0000000000000001", "00".repeat(32)),
    ] {
        let bytes = [HEADER, format!("{name}\t{value}\n").as_bytes()].concat();
        assert_eq!(error(&bytes), Error::Format);
    }
}
#[test]
fn redacted_errors_have_only_static_codes() {
    for error in [Error::Limit, Error::Format, Error::Order, Error::Allocation] {
        assert_eq!(error.to_string(), error.code());
        assert!(error.code().starts_with("tact_keys_"));
        assert!(!format!("{error:?}").contains("000102030405060708090a0b0c0d0e0f"));
    }
}

// eSTREAM Salsa20/20 Set1 vector0, 128-bit synthetic key, reproduced by the
// Crypto++ project's public TestVectors/salsa.txt (first17 lines):
// https://raw.githubusercontent.com/weidai11/cryptopp/master/TestVectors/salsa.txt
// Original source cited there: ecrypt/trunk/submissions/salsa20/full/
// verified.test-vectors?rev=161. These are mathematical test facts, not code.
const VECTORS: [(usize, &str); 4] = [
    (
        0,
        "4DFA5E481DA23EA09A31022050859936DA52FCEE218005164F267CB65F5CFD7F2B4F97E0FF16924A52DF269515110A07F9E460BC65EF95DA58F740B7D1DBB0AA",
    ),
    (
        192,
        "DA9C1581F429E0A00F7D67E23B730676783B262E8EB43A25F55FB90B3E753AEF8C6713EC66C51881111593CCB3E8CB8F8DE124080501EEEB389C4BCB6977CF95",
    ),
    (
        256,
        "7D5789631EB4554400E1E025935DFA7B3E9039D61BDC58A8697D36815BF1985CEFDF7AE112E5BB81E37ECF0616CE7147FC08A93A367E08631F23C03B00A8DA2F",
    ),
    (
        448,
        "B375703739DACED4DD4059FD71C3C47FC2F9939670FAD4A46066ADCC6A5645783308B90FFB72BE04A6B147CBE38CC0C3B9267C296A92A7C69873F9F263BE9703",
    ),
];
fn vector_key() -> [u8; 16] {
    let mut key = [0; 16];
    key[0] = 0x80;
    key
}
#[test]
fn published_128_bit_stream_vectors_at_four_block_positions() {
    let mut bytes = [0; 512];
    xor(&vector_key(), [0; 8], &mut bytes);
    for (offset, expected) in VECTORS {
        assert_eq!(bytes[offset..offset + 64], hex::<64>(expected));
    }
}
#[test]
fn block_boundaries_and_partial_last_blocks_preserve_exact_prefixes() {
    let mut complete = [0; 513];
    xor(&vector_key(), [0; 8], &mut complete);
    for length in [
        0, 1, 2, 31, 63, 64, 65, 127, 128, 129, 191, 192, 193, 255, 256, 257, 447, 448, 449, 511,
        512, 513,
    ] {
        let mut bytes = vec![0; length];
        xor(&vector_key(), [0; 8], &mut bytes);
        assert_eq!(bytes, complete[..length]);
    }
}
#[test]
fn xor_roundtrip_and_subslice_guards_for_nonzero_nonce_and_plaintext() {
    let key = std::array::from_fn(|index| index as u8);
    let nonce = std::array::from_fn(|index| (index + 10) as u8);
    for length in [0, 1, 63, 64, 65, 127, 128, 129, 1023, 1024, 1025] {
        let mut bytes: Vec<_> = (0..length + 2).map(|index| index as u8).collect();
        let original = bytes.clone();
        xor(&key, nonce, &mut bytes[1..length + 1]);
        assert_eq!(bytes[0], original[0]);
        assert_eq!(bytes[length + 1], original[length + 1]);
        if length != 0 {
            assert_ne!(bytes[1..length + 1], original[1..length + 1]);
        }
        xor(&key, nonce, &mut bytes[1..length + 1]);
        assert_eq!(bytes, original);
    }
}
#[test]
fn independent_calls_start_at_counter_zero_and_nonce_changes_stream() {
    let mut zero = [0; 65];
    let mut again = [0; 65];
    let mut changed_nonce = [0; 65];
    xor(&vector_key(), [0; 8], &mut zero);
    xor(&vector_key(), [0; 8], &mut again);
    xor(&vector_key(), [1; 8], &mut changed_nonce);
    assert_eq!(zero, again);
    assert_ne!(zero, changed_nonce);
    assert_eq!(zero[..64], hex::<64>(VECTORS[0].1));
}
