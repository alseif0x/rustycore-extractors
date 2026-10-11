// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original synthetic PE fixtures; no client or loader data.
use super::*;
const PE: usize = 128;
const OPT: usize = PE + 24;
const TABLE: usize = OPT + 240;
const BASE: u64 = 0x140000000;
const RAW: [usize; 4] = [1024, 1536, 2048, 2560];
const RVA: [u32; 4] = [0x1000, 0x2000, 0x3000, 0x4000];
fn put(data: &mut [u8], at: usize, value: u64, width: usize) {
    data[at..at + width].copy_from_slice(&value.to_le_bytes()[..width]);
}
fn dir(data: &mut [u8], index: usize, rva: u32, size: u32) {
    put(data, OPT + 112 + index * 8, rva.into(), 4);
    put(data, OPT + 116 + index * 8, size.into(), 4);
}
fn fixture() -> Vec<u8> {
    let mut d = vec![0; 3072];
    d[..2].copy_from_slice(b"MZ");
    d[PE..PE + 4].copy_from_slice(b"PE\0\0");
    for (at, value, width) in [
        (0x3c, PE as u64, 4),
        (PE + 4, 0x8664, 2),
        (PE + 6, 4, 2),
        (PE + 20, 240, 2),
        (PE + 22, 0x22, 2),
        (OPT, 0x20b, 2),
        (OPT + 24, BASE, 8),
        (OPT + 32, 4096, 4),
        (OPT + 36, 512, 4),
        (OPT + 56, 0x5000, 4),
        (OPT + 60, 1024, 4),
        (OPT + 68, 3, 2),
        (OPT + 108, 16, 4),
    ] {
        put(&mut d, at, value, width);
    }
    for (i, (name, size, flags)) in [
        (b".text\0\0\0", 128, 0x60000020),
        (b".rceid\0\0", 64, 0x40000040),
        (b".rdata\0\0", 512, 0xc0000040),
        (b".reloc\0\0", 64, 0x42000040),
    ]
    .into_iter()
    .enumerate()
    {
        let p = TABLE + i * 40;
        d[p..p + 8].copy_from_slice(name);
        for (delta, value) in [
            (8, size),
            (12, RVA[i] as u64),
            (16, 512),
            (20, RAW[i] as u64),
            (36, flags),
        ] {
            put(&mut d, p + delta, value, 4);
        }
    }
    d[RAW[0]..RAW[0] + 4].copy_from_slice(b"CODE");
    d[RAW[1]..RAW[1] + 32].copy_from_slice(&MARKER);
    d
}
fn sealed(mut data: Vec<u8>) -> Vec<u8> {
    let layout = locate(&data).unwrap();
    let hash = canonical_digest(&data, layout).unwrap();
    data[layout.digest_offset..layout.digest_offset + 32].copy_from_slice(&hash);
    data
}
fn payload(data: &[u8]) -> &[u8] {
    &data[RAW[1]..RAW[1] + 64]
}
fn reject(data: &[u8], code: &str) {
    assert_eq!(locate(data).unwrap_err().code(), code);
}
fn reloc(target: u32, kind: u16) -> Vec<u8> {
    let mut d = fixture();
    dir(&mut d, 5, RVA[3], 12);
    put(&mut d, RAW[3], (target & !4095).into(), 4);
    put(&mut d, RAW[3] + 4, 12, 4);
    put(
        &mut d,
        RAW[3] + 8,
        ((kind << 12) | (target as u16 & 4095)).into(),
        2,
    );
    d
}
fn tls(index: u32, callback: u32) -> Vec<u8> {
    let mut d = fixture();
    dir(&mut d, 9, RVA[2], 40);
    put(&mut d, RAW[2] + 16, BASE + u64::from(index), 8);
    put(&mut d, RAW[2] + 24, BASE + 0x30a0, 8);
    put(&mut d, RAW[2] + 0xa0, BASE + u64::from(callback), 8);
    d
}
fn import(target: u32) -> Vec<u8> {
    let mut d = fixture();
    dir(&mut d, 1, RVA[2], 40);
    for (off, value, width) in [
        (0, 0x3080, 4),
        (12, 0x30c0, 4),
        (16, target.into(), 4),
        (0x80, 0x8000000000000005, 8),
        (0xa0, 0x8000000000000005, 8),
    ] {
        put(&mut d, RAW[2] + off, value, width);
    }
    d[RAW[2] + 0xc0..RAW[2] + 0xc8].copy_from_slice(b"toy.dll\0");
    d
}

#[test]
fn full_hash_is_distinct_from_canonical_digest() {
    let source = fixture();
    let d = sealed(source.clone());
    let hash = verify_pe(&d, payload(&d), RVA[1]).unwrap();
    assert_eq!(hash, integrity::digest(&d));
    assert_eq!(&payload(&d)[32..], integrity::digest(&source));
    assert_ne!(hash.as_slice(), &payload(&d)[32..]);
}
#[test]
fn valid_different_disk_seal_rejects_loaded_binding() {
    let original = sealed(fixture());
    let mut other = fixture();
    other[RAW[0]] ^= 1;
    let other = sealed(other);
    assert_eq!(
        verify_pe(&other, payload(&original), RVA[1])
            .unwrap_err()
            .code(),
        "pe_loaded_mismatch"
    );
    assert!(verify_pe(&original, payload(&original), RVA[1]).is_ok());
}
#[test]
fn body_tamper_with_same_slot_rejects() {
    let d = sealed(fixture());
    let mut changed = d.clone();
    changed[RAW[0]] ^= 1;
    assert_eq!(
        verify_pe(&changed, payload(&d), RVA[1]).unwrap_err().code(),
        "pe_digest_mismatch"
    );
}
#[test]
fn zero_loaded_literal_has_no_disk_fallback() {
    let d = sealed(fixture());
    assert_eq!(
        verify_pe(&d, payload(&fixture()), RVA[1])
            .unwrap_err()
            .code(),
        "pe_loaded_seal"
    );
}
#[test]
fn sealed_input_is_not_canonical_zero_slot() {
    let d = sealed(fixture());
    assert!(payload(&d)[32..].iter().any(|b| *b != 0));
    assert_ne!(
        canonical_digest(&d, locate(&d).unwrap()).unwrap(),
        integrity::digest(&d)
    );
}
#[test]
fn overlay_is_covered() {
    let mut source = fixture();
    source.extend_from_slice(b"authored overlay");
    let d = sealed(source);
    let mut changed = d.clone();
    changed.push(1);
    assert_eq!(
        verify_pe(&changed, payload(&d), RVA[1]).unwrap_err().code(),
        "pe_digest_mismatch"
    );
}
#[test]
fn raw_offset_and_loaded_rva_are_different() {
    let l = locate(&fixture()).unwrap();
    assert_eq!(l.digest_offset, RAW[1] + 32);
    assert_eq!(l.payload_rva, RVA[1]);
    assert_ne!(l.digest_offset as u32, l.payload_rva + 32);
}
#[test]
fn marker_elsewhere_is_not_a_second_slot() {
    let mut d = fixture();
    d[RAW[0]..RAW[0] + 32].copy_from_slice(&MARKER);
    let d = sealed(d);
    assert!(verify_pe(&d, payload(&d), RVA[1]).is_ok());
}
#[test]
fn duplicate_named_sections_reject() {
    let mut d = fixture();
    d[TABLE..TABLE + 8].copy_from_slice(&NAME);
    reject(&d, "pe_seal_count");
}
#[test]
fn virtual_only_seal_rejects() {
    let mut d = fixture();
    put(&mut d, TABLE + 40 + 16, 0, 4);
    put(&mut d, TABLE + 40 + 20, 0, 4);
    reject(&d, "pe_seal_initialized");
}
#[test]
fn incorrect_payload_sizes_reject() {
    for size in [0, 31, 63, 65, 512] {
        let mut d = fixture();
        put(&mut d, TABLE + 40 + 8, size, 4);
        assert!(locate(&d).is_err());
    }
}
#[test]
fn nonzero_padding_rejects() {
    let mut d = fixture();
    d[RAW[1] + 64..RAW[1] + 96].copy_from_slice(&MARKER);
    reject(&d, "pe_seal_padding");
}
#[test]
fn seal_permissions_reject() {
    for f in [0x80000000, 0x20000000, 0x02000000, 0x10000000, 0x80] {
        let mut d = fixture();
        put(&mut d, TABLE + 40 + 36, 0x40000040 | f, 4);
        reject(&d, "pe_seal_flags");
    }
}
#[test]
fn raw_and_virtual_overlaps_reject() {
    let mut d = fixture();
    put(&mut d, TABLE + 40 + 20, RAW[0] as u64, 4);
    reject(&d, "pe_raw_overlap");
    let mut d = fixture();
    put(&mut d, TABLE + 40 + 12, RVA[0].into(), 4);
    reject(&d, "pe_virtual_overlap");
}
#[test]
fn truncated_raw_data_rejects() {
    let d = fixture();
    reject(&d[..d.len() - 1], "pe_raw_bounds");
}
#[test]
fn header_alias_rejects() {
    let mut d = fixture();
    put(&mut d, TABLE + 40 + 20, 512, 4);
    reject(&d, "pe_raw_overlap");
}
#[test]
fn nonzero_checksum_requires_external_preparation() {
    let mut d = fixture();
    put(&mut d, OPT + 64, 123, 4);
    reject(&d, "pe_checksum_nonzero");
    put(&mut d, OPT + 64, 0, 4);
    assert!(locate(&d).is_ok());
}
#[test]
fn certificate_and_unsupported_directories_reject() {
    for i in [4, 7, 8, 10, 11, 13, 14, 15] {
        let mut d = fixture();
        dir(&mut d, i, RVA[2], 16);
        reject(&d, "pe_unsupported_directory");
    }
}
#[test]
fn valid_relocation_and_padding() {
    assert!(locate(&reloc(0x1020, 10)).is_ok());
}
#[test]
fn relocation_into_seal_rejects() {
    for t in [RVA[1], RVA[1] + 32, RVA[1] + 56] {
        reject(&reloc(t, 10), "pe_loader_writes_seal");
    }
}
#[test]
fn unsupported_relocation_rejects() {
    reject(&reloc(0x1020, 3), "pe_relocation_kind");
}
#[test]
fn malformed_relocation_block_rejects() {
    let mut d = reloc(0x1020, 10);
    put(&mut d, RAW[3] + 4, 11, 4);
    reject(&d, "pe_relocation_block");
}
#[test]
fn valid_tls_index_and_callback() {
    assert!(locate(&tls(0x3080, RVA[0])).is_ok());
}
#[test]
fn tls_index_into_seal_rejects() {
    reject(&tls(RVA[1] + 32, RVA[0]), "pe_loader_writes_seal");
}
#[test]
fn tls_callback_into_seal_rejects() {
    reject(&tls(0x3080, RVA[1]), "pe_tls_callback_code");
}
#[test]
fn ordinal_import_admitted() {
    assert!(locate(&import(0x30a0)).is_ok());
}
#[test]
fn import_write_into_seal_rejects() {
    reject(&import(RVA[1]), "pe_loader_writes_seal");
}
#[test]
fn data_directory_alias_into_seal_rejects() {
    let mut d = fixture();
    dir(&mut d, 3, RVA[1], 8);
    reject(&d, "pe_loader_writes_seal");
}
#[test]
fn machine_header_and_section_counts_reject() {
    for (off, value, width) in [
        (PE + 4, 0x14c, 2),
        (OPT, 0x10b, 2),
        (PE + 6, 0, 2),
        (PE + 6, 97, 2),
        (PE + 20, 224, 2),
    ] {
        let mut d = fixture();
        put(&mut d, off, value, width);
        assert!(locate(&d).is_err());
    }
}
#[test]
fn measured_payload_rva_requires_exact_section_start() {
    let d = sealed(fixture());
    for rva in [0, RVA[1] + 32, RAW[1] as u32, u32::MAX] {
        assert_eq!(
            verify_pe(&d, payload(&d), rva).unwrap_err().code(),
            "pe_loaded_rva"
        );
    }
}
#[test]
fn loaded_shape_and_marker_reject() {
    let d = sealed(fixture());
    for bytes in [&[][..], &[0; 63][..], &[0; 65][..]] {
        assert_eq!(
            verify_pe(&d, bytes, RVA[1]).unwrap_err().code(),
            "pe_loaded_payload"
        );
    }
    let mut loaded = payload(&d).to_vec();
    loaded[0] ^= 1;
    assert_eq!(
        verify_pe(&d, &loaded, RVA[1]).unwrap_err().code(),
        "pe_loaded_seal"
    );
}
#[test]
fn malformed_bounds_do_not_panic() {
    for len in 0..fixture().len() {
        let d = fixture();
        assert!(locate(&d[..len]).is_err());
    }
    let mut d = fixture();
    put(&mut d, 0x3c, u32::MAX.into(), 4);
    assert!(locate(&d).is_err());
    let mut d = tls(0x3080, RVA[0]);
    put(&mut d, RAW[2] + 24, u64::MAX, 8);
    assert!(locate(&d).is_err());
}

#[test]
fn independent_python_hashlib_complete_fixture_vectors_match() {
    // Literal values from the separate original Python sealer and hashlib.
    for (source, canonical, full) in [
        (
            fixture(),
            "9fa1dfc1d154aaab94cbba7847a403b47776413b69502f82f14db53d1ec8d261",
            "0f1bba1756914e57477a53388464021e76a1f670c9b3234b7051d07b41b64289",
        ),
        (
            reloc(0x1020, 10),
            "cd18ffeaf6f9aa33b1eade9c1a62d4788af2ca2efd1ff5a181a12ab8a1a0e587",
            "f37e21c316a40f98c954c4df81f99acf100384e69e570c61e0ebe68901837782",
        ),
        (
            tls(0x3080, RVA[0]),
            "84617050a3a3d80437855036e62b4e98de90e39aa3b7bff6cbd1e6fcc5f029a4",
            "ac6b9daf46f794cbde50bc0c9fa1a26866ec25d3dce16eb1e0d2c508fdf4898d",
        ),
        (
            import(0x30a0),
            "d84452455909be60aa5c2bbca38c1e80333a724fbe0c505bc4bca61d2bb21ca4",
            "e8202592454b6ea52c5091feb32aaac29737bc3e3656185f36a1d2a0c597f123",
        ),
    ] {
        assert_eq!(integrity::hex(&integrity::digest(&source)), canonical);
        let bytes = sealed(source);
        assert_eq!(
            integrity::hex(&verify_pe(&bytes, payload(&bytes), RVA[1]).unwrap()),
            full
        );
    }
}

#[test]
fn relocation_cannot_rewrite_a_later_relocation_block() {
    let mut d = reloc(0x400c, 10);
    dir(&mut d, 5, RVA[3], 24);
    put(&mut d, RAW[3] + 12, 0x1000, 4);
    put(&mut d, RAW[3] + 16, 12, 4);
    put(&mut d, RAW[3] + 20, 0xa020, 2);
    reject(&d, "pe_loader_control_write");
}

#[test]
fn relocation_cannot_rewrite_import_descriptors() {
    let mut d = import(0x30a0);
    let r = reloc(RVA[2], 10);
    dir(&mut d, 5, RVA[3], 12);
    d[RAW[3]..RAW[3] + 12].copy_from_slice(&r[RAW[3]..RAW[3] + 12]);
    reject(&d, "pe_loader_control_write");
}

#[test]
fn iat_writes_cannot_rewrite_relocations_or_import_descriptors() {
    reject(&import(RVA[2]), "pe_loader_control_write");
    let mut d = import(RVA[3]);
    let r = reloc(RVA[0] + 32, 10);
    dir(&mut d, 5, RVA[3], 12);
    d[RAW[3]..RAW[3] + 12].copy_from_slice(&r[RAW[3]..RAW[3] + 12]);
    reject(&d, "pe_loader_control_write");
}

#[test]
fn original_first_thunk_zero_admits_normal_in_place_slot_resolution() {
    let mut d = import(0x30a0);
    put(&mut d, RAW[2], 0, 4);
    let d = sealed(d);
    assert!(verify_pe(&d, payload(&d), RVA[1]).is_ok());
}
