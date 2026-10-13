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

// Original synthetic v2 geometry. No native executable/loader bytes are copied.
const V2_FIELDS: [usize; 9] = [88, 112, 120, 256, 280, 288, 296, 304, 312];
const V2_POINTERS: [u32; 9] = [
    0x4000, 0x3300, 0x3310, 0x3200, 0x3308, 0x3318, 0x3320, 0x3328, 0x3330,
];
fn v2_offset(rva: u32) -> usize {
    for (address, raw, size) in [
        (0x1000, 1024, 512),
        (0x2000, 1536, 512),
        (0x3000, 2048, 2048),
        (0x4000, 4096, 512),
        (0x5000, 4608, 512),
    ] {
        if (address..address + size).contains(&rva) {
            return raw + (rva - address) as usize;
        }
    }
    panic!("synthetic RVA outside fixture");
}
fn v2_targets(tls: bool) -> Vec<u32> {
    let mut targets: Vec<_> = V2_FIELDS.into_iter().map(|f| 0x3000 + f as u32).collect();
    targets.extend(
        V2_FIELDS
            .into_iter()
            .zip(V2_POINTERS)
            .filter(|(field, _)| ![88, 256, 304].contains(field))
            .map(|(_, rva)| rva),
    );
    if tls {
        targets.extend([0x3400, 0x3408, 0x3410, 0x3418, 0x3440]);
    }
    targets
}
fn v2_relocations(data: &mut [u8], targets: &[u32]) {
    data[4608..5120].fill(0);
    dir(data, 5, 0x5000, targets.len() as u32 * 12);
    for (i, &target) in targets.iter().enumerate() {
        put(data, 4608 + i * 12, u64::from(target & !4095), 4);
        put(data, 4612 + i * 12, 12, 4);
        put(data, 4616 + i * 12, u64::from(0xa000 | (target & 4095)), 2);
    }
}
fn v2_fixture(tls: bool, imports: bool) -> Vec<u8> {
    let mut data = fixture();
    data.resize(5120, 0);
    put(&mut data, PE + 6, 5, 2);
    put(&mut data, OPT + 56, 0x6000, 4);
    put(&mut data, OPT + 70, 0x8160, 2);
    for (i, (name, rva, raw, virtual_size, size, flags)) in [
        (b".text\0\0\0", 0x1000, 1024, 512, 512, 0x60000020),
        (b".rceid\0\0", 0x2000, 1536, 64, 512, 0x40000040),
        (b".rdata\0\0", 0x3000, 2048, 2048, 2048, 0x40000040),
        (b".data\0\0\0", 0x4000, 4096, 512, 512, 0xc0000040),
        (b".reloc\0\0", 0x5000, 4608, 512, 512, 0x42000040),
    ]
    .into_iter()
    .enumerate()
    {
        let at = TABLE + i * 40;
        data[at..at + 40].fill(0);
        data[at..at + 8].copy_from_slice(name);
        for (delta, value) in [
            (8, virtual_size),
            (12, rva),
            (16, size),
            (20, raw),
            (36, flags),
        ] {
            put(&mut data, at + delta, value, 4);
        }
    }
    data[1536..1568].copy_from_slice(&MARKER_V2);
    dir(&mut data, 10, 0x3000, 320);
    put(&mut data, 2048, 320, 4);
    put(&mut data, 2048 + 144, 0x100, 4);
    for (field, rva) in V2_FIELDS.into_iter().zip(V2_POINTERS) {
        put(&mut data, 2048 + field, BASE + u64::from(rva), 8);
        if ![88, 256, 304].contains(&field) {
            put(&mut data, v2_offset(rva), BASE + 0x1000, 8);
        }
    }
    put(&mut data, 4096, 0x2b992ddfa232, 8);
    let at = v2_offset(0x3200);
    for (i, value) in [24, 0x80038003, 0x3218, 40, 0x3240, 24]
        .into_iter()
        .enumerate()
    {
        put(&mut data, at + i * 4, value, 4);
    }
    for i in 0..10 {
        put(&mut data, at + 24 + i * 4, 0x1004 + i as u64 * 4, 4);
    }
    for (i, (rva, size)) in [(0x1000, 16), (0x1010, 32), (0x1040, 16)]
        .into_iter()
        .enumerate()
    {
        put(&mut data, at + 64 + i * 8, rva, 4);
        put(&mut data, at + 68 + i * 8, size, 4);
    }
    if tls {
        dir(&mut data, 9, 0x3400, 40);
        for (i, rva) in [0x4020, 0x4024, 0x4010, 0x3440].into_iter().enumerate() {
            put(&mut data, v2_offset(0x3400) + i * 8, BASE + rva, 8);
        }
        put(&mut data, v2_offset(0x3440), BASE + 0x1000, 8);
    }
    if imports {
        dir(&mut data, 1, 0x3500, 40);
        dir(&mut data, 12, 0x4030, 16);
        for (delta, value) in [(0, 0x3540), (12, 0x3580), (16, 0x4030)] {
            put(&mut data, v2_offset(0x3500) + delta, value, 4);
        }
        put(&mut data, v2_offset(0x3540), 0x8000000000000005, 8);
        data[v2_offset(0x3580)..v2_offset(0x3580) + 8].copy_from_slice(b"toy.dll\0");
    }
    v2_relocations(&mut data, &v2_targets(tls));
    data
}
fn v2_reject(data: &[u8]) {
    assert!(locate_with_profile(data, Profile::MsvcV2).is_err());
}
fn v2_sealed(mut data: Vec<u8>) -> Vec<u8> {
    let layout = locate_with_profile(&data, Profile::MsvcV2).unwrap();
    let digest = canonical_digest(&data, layout).unwrap();
    data[layout.digest_offset..layout.digest_offset + 32].copy_from_slice(&digest);
    data
}

#[test]
fn v2_exact_observed_shape_and_optional_tls_imports() {
    for tls in [false, true] {
        for imports in [false, true] {
            let source = v2_fixture(tls, imports);
            let data = v2_sealed(source.clone());
            let full =
                verify_pe_with_profile(&data, payload(&data), RVA[1], Profile::MsvcV2).unwrap();
            assert_eq!(full, integrity::digest(&data));
            assert_eq!(&payload(&data)[32..], integrity::digest(&source));
            assert_ne!(full.as_slice(), &payload(&data)[32..]);
        }
    }
}
#[test]
fn v2_profile_and_markers_never_fallback() {
    reject(&v2_fixture(false, false), "pe_seal_marker");
    v2_reject(&fixture());
    let mut data = v2_fixture(false, false);
    data[1536..1568].copy_from_slice(&MARKER);
    reject(&data, "pe_unsupported_directory");
}
#[test]
fn v2_every_reserved_load_config_byte_is_zero() {
    let mut known = [false; 320];
    known[..4].fill(true);
    known[144..148].fill(true);
    for field in V2_FIELDS {
        known[field..field + 8].fill(true);
    }
    for (i, known) in known.into_iter().enumerate() {
        if !known {
            let mut data = v2_fixture(false, false);
            data[2048 + i] = 1;
            v2_reject(&data);
        }
    }
}
#[test]
fn v2_config_size_flags_and_directory_are_exact() {
    for (at, value, width) in [
        (2048, 319, 4),
        (2192, 0x500, 4),
        (OPT + 70, 0xc160, 2),
        (OPT + 70, 0x8100, 2),
    ] {
        let mut data = v2_fixture(false, false);
        put(&mut data, at, value, width);
        v2_reject(&data);
    }
    for size in [0, 319, 321, 512] {
        let mut data = v2_fixture(false, false);
        dir(&mut data, 10, if size == 0 { 0 } else { 0x3000 }, size);
        v2_reject(&data);
    }
}
#[test]
fn v2_each_required_fixup_is_exactly_once() {
    for tls in [false, true] {
        let targets = v2_targets(tls);
        for (i, &target) in targets.iter().enumerate() {
            let mut missing = targets.clone();
            missing.remove(i);
            let mut data = v2_fixture(tls, false);
            v2_relocations(&mut data, &missing);
            v2_reject(&data);
            missing.push(0x1060); // Same total count cannot compensate a missing typed fixup.
            v2_relocations(&mut data, &missing);
            v2_reject(&data);
            let mut duplicate = targets.clone();
            duplicate.push(target); // Deliberately a distinct relocation block.
            let mut data = v2_fixture(tls, false);
            v2_relocations(&mut data, &duplicate);
            v2_reject(&data);
            let mut data = v2_fixture(tls, false);
            put(&mut data, 4616 + i * 12, u64::from(target & 4095), 2);
            v2_reject(&data);
        }
    }
}
#[test]
fn v2_castguard_zero_and_cookie_magic_have_no_fixup() {
    for target in [0x3328, 0x4000] {
        let mut data = v2_fixture(false, false);
        let mut targets = v2_targets(false);
        targets.push(target);
        v2_relocations(&mut data, &targets);
        v2_reject(&data);
    }
    for (target, value) in [
        (0x3328, BASE + 0x1000),
        (0x4000, 0),
        (0x4000, 0x2b992ddfa233),
    ] {
        let mut data = v2_fixture(false, false);
        put(&mut data, v2_offset(target), value, 8);
        v2_reject(&data);
    }
}
#[test]
fn v2_va_underflow_unmapped_and_rva_values_reject() {
    for field in V2_FIELDS {
        for value in [0, BASE - 1, 0x3300, BASE + 0x9000, u64::MAX] {
            let mut data = v2_fixture(false, false);
            put(&mut data, 2048 + field, value, 8);
            v2_reject(&data);
        }
    }
}
#[test]
fn v2_slots_require_code_and_separate_read_only_storage() {
    for (field, slot) in V2_FIELDS.into_iter().zip(V2_POINTERS) {
        if [88, 256, 304].contains(&field) {
            continue;
        }
        for value in [0, BASE + 0x2000, BASE + 0x3000, BASE + 0x1200] {
            let mut data = v2_fixture(false, false);
            put(&mut data, v2_offset(slot), value, 8);
            v2_reject(&data);
        }
    }
    for field in [112, 120, 280, 288, 296, 304, 312] {
        let mut data = v2_fixture(false, false);
        let rva = if field == 112 { 0x3310 } else { 0x3300 };
        put(&mut data, 2048 + field, BASE + rva, 8);
        v2_reject(&data);
    }
    let mut data = v2_fixture(false, false);
    put(&mut data, TABLE + 2 * 40 + 36, 0xc0000040, 4);
    v2_reject(&data);
    let mut data = v2_fixture(false, false);
    put(&mut data, 2048 + 112, BASE + 0x3301, 8);
    v2_reject(&data);
}
#[test]
fn v2_volatile_signature_counts_and_contiguity_are_exact() {
    for (i, value) in [25, 1, 0x321c, 36, 0x3244, 32].into_iter().enumerate() {
        let mut data = v2_fixture(false, false);
        put(&mut data, v2_offset(0x3200) + i * 4, value, 4);
        v2_reject(&data);
    }
}
#[test]
fn v2_volatile_accesses_sorted_unique_and_ranges_wholly_code() {
    for (delta, value) in [
        (24, 0),
        (28, 0x1004),
        (28, 0x1000),
        (24, 0x1038),
        (64, 0x2000),
        (68, 0),
        (68, 0x201),
        (72, 0x1008),
        (80, 0x11f8),
        (84, 0x201),
    ] {
        let mut data = v2_fixture(false, false);
        put(&mut data, v2_offset(0x3200) + delta, value, 4);
        v2_reject(&data);
    }
    let mut data = v2_fixture(false, false);
    put(&mut data, TABLE + 2 * 40 + 8, 0x250, 4);
    v2_reject(&data);
}
const V2_CONTROLS: [(u32, u32); 8] = [
    (0x3000, 320),
    (0x3200, 88),
    (0x3400, 40),
    (0x3440, 16),
    (0x3500, 40),
    (0x3540, 16),
    (0x5000, 240),
    (0x2000, 64),
];
#[test]
fn v2_every_partial_relocation_against_controls_rejects() {
    for (start, size) in V2_CONTROLS {
        for target in start - 7..start + size {
            let mut data = v2_fixture(true, true);
            let mut targets = v2_targets(true);
            targets.push(target);
            v2_relocations(&mut data, &targets);
            v2_reject(&data);
        }
    }
}
#[test]
fn v2_iat_and_tls_index_cannot_overlap_control_boundaries() {
    for (start, size) in V2_CONTROLS {
        for target in start - 7..start + size {
            let mut data = v2_fixture(true, true);
            put(&mut data, v2_offset(0x3500) + 16, u64::from(target), 4);
            v2_reject(&data);
        }
        for target in start - 3..start + size {
            let mut data = v2_fixture(true, true);
            put(
                &mut data,
                v2_offset(0x3400) + 16,
                BASE + u64::from(target),
                8,
            );
            v2_reject(&data);
        }
    }
}
#[test]
fn v2_partial_and_other_writes_cannot_alias_cookie_or_slots() {
    for (start, size) in [(0x4000, 8), (0x3300, 56)] {
        for target in start - 7..start + size {
            let mut data = v2_fixture(true, true);
            let mut targets = v2_targets(true);
            targets.push(target);
            v2_relocations(&mut data, &targets);
            v2_reject(&data);
            let mut data = v2_fixture(true, true);
            put(&mut data, v2_offset(0x3500) + 16, u64::from(target), 4);
            v2_reject(&data);
        }
        for target in start - 3..start + size {
            let mut data = v2_fixture(true, true);
            put(
                &mut data,
                v2_offset(0x3400) + 16,
                BASE + u64::from(target),
                8,
            );
            v2_reject(&data);
        }
    }
}
#[test]
fn v2_duplicate_partial_relocations_and_cross_writes_reject() {
    for extra in [
        vec![0x1060, 0x1060],
        vec![0x1060, 0x1067],
        vec![0x4030],
        vec![0x4010],
    ] {
        let mut data = v2_fixture(true, true);
        let mut targets = v2_targets(true);
        targets.extend(extra);
        v2_relocations(&mut data, &targets);
        v2_reject(&data);
    }
    let mut data = v2_fixture(true, true);
    put(&mut data, v2_offset(0x3400) + 16, BASE + 0x4030, 8);
    v2_reject(&data);
    let mut data = v2_fixture(true, true);
    put(&mut data, v2_offset(0x3500), 0, 4);
    v2_reject(&data);
}
#[test]
fn v2_empty_tls_template_is_mapped_and_callbacks_require_code() {
    let mut data = v2_fixture(true, false);
    put(&mut data, v2_offset(0x3400) + 8, BASE + 0x4020, 8);
    assert!(locate_with_profile(&data, Profile::MsvcV2).is_ok());
    for value in [BASE + 0x9000, BASE + 0x100000000] {
        let mut data = v2_fixture(true, false);
        put(&mut data, v2_offset(0x3400), value, 8);
        put(&mut data, v2_offset(0x3400) + 8, value, 8);
        v2_reject(&data);
    }
    let mut data = v2_fixture(true, false);
    put(&mut data, v2_offset(0x3440), BASE + 0x3000, 8);
    v2_reject(&data);
    let mut data = v2_fixture(true, false);
    let mut targets = v2_targets(true);
    targets.push(0x3448);
    v2_relocations(&mut data, &targets);
    v2_reject(&data);
}
#[test]
fn v2_hash_marker_rva_and_loaded_binding_remain_strict() {
    let data = v2_sealed(v2_fixture(false, false));
    let loaded = payload(&data);
    for rva in [0, 0x2020, 1536] {
        assert_eq!(
            verify_pe_with_profile(&data, loaded, rva, Profile::MsvcV2)
                .unwrap_err()
                .code(),
            "pe_loaded_rva"
        );
    }
    for at in [1024, 4196, 5119] {
        let mut changed = data.clone();
        changed[at] ^= 1;
        assert_eq!(
            verify_pe_with_profile(&changed, loaded, RVA[1], Profile::MsvcV2)
                .unwrap_err()
                .code(),
            "pe_digest_mismatch"
        );
    }
    let mut wrong = loaded.to_vec();
    wrong[..32].copy_from_slice(&MARKER);
    assert_eq!(
        verify_pe_with_profile(&data, &wrong, RVA[1], Profile::MsvcV2)
            .unwrap_err()
            .code(),
        "pe_loaded_seal"
    );
    let mut another = v2_fixture(false, false);
    another[1024] ^= 1;
    let another = v2_sealed(another);
    assert_eq!(
        verify_pe_with_profile(&another, loaded, RVA[1], Profile::MsvcV2)
            .unwrap_err()
            .code(),
        "pe_loaded_mismatch"
    );
}

#[test]
fn v2_alternate_image_base_updates_only_va_fields() {
    let mut data = v2_fixture(true, true);
    let new_base = 0x180000000;
    let delta = new_base - BASE;
    put(&mut data, OPT + 24, new_base, 8);
    let mut addresses: Vec<_> = V2_FIELDS.into_iter().map(|field| 2048 + field).collect();
    addresses.extend(
        V2_FIELDS
            .into_iter()
            .zip(V2_POINTERS)
            .filter(|(field, _)| ![88, 256, 304].contains(field))
            .map(|(_, rva)| v2_offset(rva)),
    );
    addresses.extend((0..4).map(|i| v2_offset(0x3400) + i * 8));
    addresses.push(v2_offset(0x3440));
    for at in addresses {
        let value = integer(&data, at, 8).unwrap() + delta;
        put(&mut data, at, value, 8);
    }
    let data = v2_sealed(data);
    assert_eq!(
        verify_pe_with_profile(&data, payload(&data), RVA[1], Profile::MsvcV2).unwrap(),
        integrity::digest(&data)
    );
}
#[test]
fn v2_independent_python_complete_file_hash_vectors_match() {
    for (tls, imports, canonical, full) in [
        (
            false,
            false,
            "8b5385ee6e43affa9cc9a3958a41d488cd3e92135cd09cd6fe19d7ac63882192",
            "bedb8fd595b5d12254e1d5a30f5cb5b1402d3bae4a430538ed54a80dbd6eab70",
        ),
        (
            false,
            true,
            "e8f371bb047fc58bce966085693300d2936b40638f4d88736746164cda2ea34a",
            "9efc1a7e67ed17e4d760c5fc3f8262e9b59f7e7fef6b50c57f272630dca7c1e6",
        ),
        (
            true,
            false,
            "85f08894762e403b485990f9c079f4a3492704e6d5fa3e7c61fc8556433cc194",
            "89e4903524653857a748082de44dcd67a75ff1510622953b36fde5a2507d6c05",
        ),
        (
            true,
            true,
            "9e1339adaec5852cbe20d53383f8b21d6a0f9a5081912946344131f041883af8",
            "b05150f8fcc774460ce24fec1460c3629604c3a9855d11428b58ddcbfc458ff3",
        ),
    ] {
        let data = v2_fixture(tls, imports);
        assert_eq!(integrity::hex(&integrity::digest(&data)), canonical);
        let data = v2_sealed(data);
        assert_eq!(
            integrity::hex(
                &verify_pe_with_profile(&data, payload(&data), RVA[1], Profile::MsvcV2).unwrap()
            ),
            full
        );
    }
}
fn v2_grow_reloc(data: &mut Vec<u8>, size: usize) {
    let raw_size = aligned(size as u64, 512);
    data.resize(4608 + raw_size as usize, 0);
    dir(data, 5, 0x5000, size as u32);
    put(data, TABLE + 4 * 40 + 8, size as u64, 4);
    put(data, TABLE + 4 * 40 + 16, raw_size, 4);
    put(data, OPT + 56, aligned(0x5000 + raw_size, 4096), 4);
}
#[test]
fn v2_relocation_size_and_entry_caps_include_absolute_padding() {
    for count in [65536, 65537] {
        let mut data = v2_fixture(false, false);
        let remaining = count - v2_targets(false).len() * 2;
        let size = 180 + 8 + remaining * 2;
        v2_grow_reloc(&mut data, size);
        put(&mut data, 4608 + 180, 0x1000, 4);
        put(&mut data, 4608 + 184, (8 + remaining * 2) as u64, 4);
        if count == 65536 {
            assert!(locate_with_profile(&data, Profile::MsvcV2).is_ok());
        } else {
            assert_eq!(
                locate_with_profile(&data, Profile::MsvcV2)
                    .unwrap_err()
                    .code(),
                "pe_relocation_limit"
            );
        }
    }
    let mut data = v2_fixture(false, false);
    v2_grow_reloc(&mut data, 1024 * 1024 + 1);
    assert_eq!(
        locate_with_profile(&data, Profile::MsvcV2)
            .unwrap_err()
            .code(),
        "pe_relocation_limit"
    );
}
#[test]
fn v2_callback_terminator_must_be_inside_1024_slots() {
    let mut data = v2_fixture(true, false);
    let raw_size = aligned(0x100 + 1024 * 8, 512);
    data.resize(4608 + raw_size as usize, 0);
    put(&mut data, TABLE + 4 * 40 + 8, raw_size, 4);
    put(&mut data, TABLE + 4 * 40 + 16, raw_size, 4);
    put(&mut data, OPT + 56, aligned(0x5000 + raw_size, 4096), 4);
    put(&mut data, v2_offset(0x3400) + 24, BASE + 0x5100, 8);
    for i in 0..1024 {
        put(&mut data, 4608 + 0x100 + i * 8, BASE + 0x1000, 8);
    }
    assert_eq!(
        locate_with_profile(&data, Profile::MsvcV2)
            .unwrap_err()
            .code(),
        "pe_tls_callbacks_limit"
    );
}
#[test]
fn v2_truncation_and_unsupported_directories_reject() {
    let data = v2_fixture(true, true);
    for length in 0..data.len() {
        v2_reject(&data[..length]);
    }
    for index in [4, 7, 8, 11, 13, 14, 15] {
        let mut data = v2_fixture(false, false);
        dir(&mut data, index, 0x3700, 8);
        assert_eq!(
            locate_with_profile(&data, Profile::MsvcV2)
                .unwrap_err()
                .code(),
            "pe_unsupported_directory"
        );
    }
}
