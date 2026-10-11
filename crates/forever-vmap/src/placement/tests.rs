// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: independent literal synthetic placement transcripts.
use super::*;

fn put(bytes: &mut [u8], at: usize, value: u32) {
    bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
}
fn put16(bytes: &mut [u8], at: usize, value: u16) {
    bytes[at..at + 2].copy_from_slice(&value.to_le_bytes());
}
fn words(bytes: &mut Vec<u8>, values: &[u32]) {
    for value in values {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
}
fn mddf() -> [u8; 36] {
    let mut bytes = [0; 36];
    put(&mut bytes, 0, 0x1020_3040);
    put(&mut bytes, 4, 0x5060_7080);
    for (at, value) in [
        (8, 1f32),
        (12, 2.0),
        (16, 3.0),
        (20, 4.0),
        (24, 5.0),
        (28, 6.0),
    ] {
        put(&mut bytes, at, value.to_bits());
    }
    put16(&mut bytes, 32, 512);
    put16(&mut bytes, 34, 0x40);
    bytes
}
fn modf() -> [u8; 64] {
    let mut bytes = [0; 64];
    bytes[..32].copy_from_slice(&mddf()[..32]);
    for (at, value) in [
        (32, -1f32),
        (36, -2.0),
        (40, -3.0),
        (44, 7.0),
        (48, 8.0),
        (52, 9.0),
    ] {
        put(&mut bytes, at, value.to_bits());
    }
    put16(&mut bytes, 56, 0x0d);
    put16(&mut bytes, 58, 0x1234);
    put16(&mut bytes, 60, 0x12ff);
    put16(&mut bytes, 62, 1536);
    bytes
}
fn context() -> Context {
    Context {
        map_id: 9,
        original_map_id: 9,
        global_wmo: false,
    }
}
fn m2(name: &[u8]) -> Placement<'_> {
    project_m2(
        &Mddf::parse(&mddf(), Limits::default()).unwrap(),
        0x1122_3344,
        name,
        context(),
        Limits::default(),
    )
    .unwrap()
}
fn wmo(name: &[u8]) -> Placement<'_> {
    project_wmo(
        &Modf::parse(&modf(), Limits::default()).unwrap(),
        0x1122_3344,
        name,
        context(),
        Limits::default(),
    )
    .unwrap()
}
fn wire(record: Placement<'_>) -> Vec<u8> {
    encode(&[record], Limits::default()).unwrap()
}
fn same(a: Placement<'_>, b: Placement<'_>) {
    assert_eq!(a.flags(), b.flags());
    assert_eq!(a.name_set(), b.name_set());
    assert_eq!(a.id(), b.id());
    assert_eq!(a.position_bits(), b.position_bits());
    assert_eq!(a.rotation_bits(), b.rotation_bits());
    assert_eq!(a.scale_bits(), b.scale_bits());
    assert_eq!(a.bounds_bits(), b.bounds_bits());
    assert_eq!(a.name(), b.name());
}

#[test]
fn packed_source_offsets_and_fdids_are_independent_of_assigned_ids() {
    let b = mddf();
    let source = Mddf::parse(&b, Limits::default()).unwrap();
    assert_eq!(source.source_id(), 0x1020_3040);
    assert_eq!(source.unique_id(), 0x5060_7080);
    assert_eq!(source.scale(), 512);
    assert_eq!(source.flags(), 0x40);
    assert!(source.uses_file_data_id());
    let b = modf();
    let source = Modf::parse(&b, Limits::default()).unwrap();
    assert_eq!(source.source_id(), 0x1020_3040);
    assert_eq!(source.unique_id(), 0x5060_7080);
    assert_eq!(source.flags(), 13);
    assert!(source.uses_file_data_id());
    assert_eq!(source.doodad_set(), 0x1234);
    assert_eq!(source.name_set(), 0x12ff);
    assert_eq!(source.scale(), 1536);
}
#[test]
fn every_mddf_truncation_and_extra_byte_is_rejected() {
    let mut bytes = mddf().to_vec();
    for length in 0..36 {
        assert_eq!(
            Mddf::parse(&bytes[..length], Limits::default()).err(),
            Some(Error::Framing)
        );
    }
    bytes.push(0);
    assert_eq!(
        Mddf::parse(&bytes, Limits::default()).err(),
        Some(Error::Framing)
    );
}
#[test]
fn every_modf_truncation_and_extra_byte_is_rejected() {
    let mut bytes = modf().to_vec();
    for length in 0..64 {
        assert_eq!(
            Modf::parse(&bytes[..length], Limits::default()).err(),
            Some(Error::Framing)
        );
    }
    bytes.push(0);
    assert_eq!(
        Modf::parse(&bytes, Limits::default()).err(),
        Some(Error::Framing)
    );
}
#[test]
fn source_transforms_reject_each_nonfinite_used_component() {
    for bad in [0x7f80_0000, 0xff80_0000, 0x7fc0_1234] {
        for at in (8..32).step_by(4) {
            let mut b = mddf();
            put(&mut b, at, bad);
            assert_eq!(
                Mddf::parse(&b, Limits::default()).err(),
                Some(Error::NonFinite)
            );
        }
        for at in (8..56).step_by(4) {
            let mut b = modf();
            put(&mut b, at, bad);
            assert_eq!(
                Modf::parse(&b, Limits::default()).err(),
                Some(Error::NonFinite)
            );
        }
    }
}
#[test]
fn source_bounds_require_order_on_each_axis_but_allow_equal_zero() {
    for axis in 0..3 {
        let mut b = modf();
        put(&mut b, 32 + axis * 4, 100f32.to_bits());
        assert_eq!(
            Modf::parse(&b, Limits::default()).err(),
            Some(Error::InvertedBounds)
        );
    }
    let mut b = modf();
    b[32..56].fill(0);
    put(&mut b, 32, 0x8000_0000);
    assert!(Modf::parse(&b, Limits::default()).is_ok());
}
#[test]
fn unknown_source_flags_are_opaque_and_only_known_bits_project() {
    let mut b = mddf();
    put16(&mut b, 34, 0xffff);
    let parsed = Mddf::parse(&b, Limits::default()).unwrap();
    assert!(parsed.uses_file_data_id());
    assert_eq!(
        project_m2(&parsed, 1, b"x", context(), Limits::default())
            .unwrap()
            .flags(),
        0
    );
    let mut b = modf();
    put16(&mut b, 56, 0xffff);
    let parsed = Modf::parse(&b, Limits::default()).unwrap();
    assert!(parsed.uses_file_data_id());
    assert_eq!(
        project_wmo(&parsed, 1, b"x", context(), Limits::default())
            .unwrap()
            .flags(),
        5
    );
    put16(&mut b, 56, 0xfff2);
    let parsed = Modf::parse(&b, Limits::default()).unwrap();
    assert!(!parsed.uses_file_data_id());
    assert_eq!(
        project_wmo(&parsed, 1, b"x", context(), Limits::default())
            .unwrap()
            .flags(),
        1
    );
}
#[test]
fn m2_complete_literal_dir_bin_golden() {
    let actual = wire(m2(b"A.m2"));
    let mut expected = vec![0, 0];
    words(
        &mut expected,
        &[
            0x1122_3344,
            0x4040_0000,
            0x3f80_0000,
            0x4000_0000,
            0x4080_0000,
            0x40a0_0000,
            0x40c0_0000,
            0x3f00_0000,
            4,
        ],
    );
    expected.extend_from_slice(b"A.m2");
    assert_eq!(actual, expected);
    assert_eq!(actual.len(), 42);
    let parsed = DirBin::parse(&actual, Limits::default()).unwrap();
    same(parsed.records().next().unwrap(), m2(b"A.m2"));
}
#[test]
fn wmo_complete_literal_dir_bin_golden() {
    let actual = wire(wmo(b"B.wmo"));
    let mut expected = vec![5, 255];
    words(
        &mut expected,
        &[
            0x1122_3344,
            0x4040_0000,
            0x3f80_0000,
            0x4000_0000,
            0x4080_0000,
            0x40a0_0000,
            0x40c0_0000,
            0x3fc0_0000,
            0xc040_0000,
            0xbf80_0000,
            0xc000_0000,
            0x4110_0000,
            0x40e0_0000,
            0x4100_0000,
            5,
        ],
    );
    expected.extend_from_slice(b"B.wmo");
    assert_eq!(actual, expected);
    assert_eq!(actual.len(), 67);
    same(
        DirBin::parse(&actual, Limits::default())
            .unwrap()
            .records()
            .next()
            .unwrap(),
        wmo(b"B.wmo"),
    );
}
#[test]
fn assigned_zero_and_maximum_ids_are_preserved_without_allocation() {
    let b = mddf();
    let source = Mddf::parse(&b, Limits::default()).unwrap();
    for id in [0, u32::MAX] {
        let p = project_m2(&source, id, b"x", context(), Limits::default()).unwrap();
        assert_eq!(p.id(), id);
        assert_eq!(word(&wire(p), 2).unwrap(), id);
    }
}
#[test]
fn parent_flags_depend_only_on_map_difference() {
    let b = mddf();
    let s = Mddf::parse(&b, Limits::default()).unwrap();
    let mut ctx = context();
    ctx.original_map_id = 8;
    ctx.global_wmo = true;
    let p = project_m2(&s, 1, b"x", ctx, Limits::default()).unwrap();
    assert_eq!(p.flags(), 2);
    assert_eq!(p.position_bits(), m2(b"x").position_bits());
    assert_eq!(p.name_set(), 0);
    assert!(p.bounds_bits().is_none());
    let b = modf();
    let s = Modf::parse(&b, Limits::default()).unwrap();
    ctx.global_wmo = false;
    assert_eq!(
        project_wmo(&s, 1, b"x", ctx, Limits::default())
            .unwrap()
            .flags(),
        7
    );
}
#[test]
fn source_scale_zero_and_maximum_are_exact_f32() {
    for (scale, expected) in [
        (0, 0u32),
        (1, 0x3a80_0000),
        (1024, 0x3f80_0000),
        (65535, 0x427f_ff00),
    ] {
        let mut b = mddf();
        put16(&mut b, 32, scale);
        let s = Mddf::parse(&b, Limits::default()).unwrap();
        assert_eq!(
            project_m2(&s, 0, b"x", context(), Limits::default())
                .unwrap()
                .scale_bits(),
            expected
        );
        let mut b = modf();
        put16(&mut b, 62, scale);
        let s = Modf::parse(&b, Limits::default()).unwrap();
        assert_eq!(
            project_wmo(&s, 0, b"x", context(), Limits::default())
                .unwrap()
                .scale_bits(),
            expected
        );
    }
}
#[test]
fn wmo_scale_flag_controls_default_one_independently_of_scale_value() {
    for flags in [0, 1, 8, 0xfff3] {
        for scale in [0, 65535] {
            let mut b = modf();
            put16(&mut b, 56, flags);
            put16(&mut b, 62, scale);
            let s = Modf::parse(&b, Limits::default()).unwrap();
            assert_eq!(
                project_wmo(&s, 0, b"x", context(), Limits::default())
                    .unwrap()
                    .scale_bits(),
                0x3f80_0000
            );
        }
    }
}
#[test]
fn wmo_name_set_truncates_to_one_byte() {
    for (input, expected) in [(0, 0), (255, 255), (256, 0), (65535, 255)] {
        let mut b = modf();
        put16(&mut b, 60, input);
        let s = Modf::parse(&b, Limits::default()).unwrap();
        let p = project_wmo(&s, 0, b"x", context(), Limits::default()).unwrap();
        assert_eq!(p.name_set(), expected);
        assert_eq!(wire(p)[1], expected);
    }
}
#[test]
fn non_global_permutation_preserves_signed_zero_and_subnormal_bits() {
    let mut b = mddf();
    for (at, bits) in [
        (8, 0x8000_0000),
        (12, 1),
        (16, 0),
        (20, 0x8000_0000),
        (24, 1),
        (28, 0x8000_0001),
    ] {
        put(&mut b, at, bits);
    }
    let s = Mddf::parse(&b, Limits::default()).unwrap();
    let p = project_m2(&s, 0, b"x", context(), Limits::default()).unwrap();
    assert_eq!(p.position_bits(), [0, 0x8000_0000, 1]);
    assert_eq!(p.rotation_bits(), [0x8000_0000, 1, 0x8000_0001]);
}
#[test]
fn global_wmo_adds_f32_offsets_to_position_and_both_bounds() {
    let mut b = modf();
    b[8..56].fill(0);
    for at in [12, 36, 48] {
        put(&mut b, at, 0x8000_0000);
    }
    put(&mut b, 20, 0x8000_0000);
    let s = Modf::parse(&b, Limits::default()).unwrap();
    let mut ctx = context();
    ctx.global_wmo = true;
    let p = project_wmo(&s, 0, b"x", ctx, Limits::default()).unwrap();
    assert_eq!(p.position_bits(), [0x4685_5555, 0x4685_5555, 0]);
    assert_eq!(
        p.bounds_bits(),
        Some([0x4685_5555, 0x4685_5555, 0, 0x4685_5555, 0x4685_5555, 0])
    );
    assert_eq!(p.rotation_bits()[0], 0x8000_0000);
}
#[test]
fn global_addition_rounds_after_float32_add_and_retains_other_rotation_bits() {
    let mut b = modf();
    put(&mut b, 16, 0.000_976_562_5_f32.to_bits());
    let s = Modf::parse(&b, Limits::default()).unwrap();
    let mut ctx = context();
    ctx.global_wmo = true;
    let p = project_wmo(&s, 1, b"x", ctx, Limits::default()).unwrap();
    assert_eq!(p.position_bits()[0], 0x4685_5556);
    assert_eq!(p.rotation_bits(), s.rotation_bits());
}
#[test]
fn names_are_borrowed_unmodified_and_not_renormalized() {
    let name = b"FILEfoo.M2";
    let p = m2(name);
    assert_eq!(p.name(), name);
    assert_eq!(p.name().as_ptr(), name.as_ptr());
    let name = b"literal Mixed NAME";
    assert_eq!(m2(name).name(), name);
}
#[test]
fn name500_is_admitted_501_and_empty_nul_nonascii_are_rejected() {
    let b = mddf();
    let s = Mddf::parse(&b, Limits::default()).unwrap();
    let long = [b'A'; 500];
    assert_eq!(
        wire(project_m2(&s, 0, &long, context(), Limits::default()).unwrap()).len(),
        538
    );
    assert_eq!(
        project_m2(&s, 0, &[b'A'; 501], context(), Limits::default()).err(),
        Some(Error::Limit)
    );
    for name in [b"".as_slice(), b"a\0b", &[0xff]] {
        assert_eq!(
            project_m2(&s, 0, name, context(), Limits::default()).err(),
            Some(Error::Name)
        );
    }
}
#[test]
fn empty_dir_bin_and_empty_encoding_are_complete() {
    let limits = Limits {
        max_bytes: 0,
        max_records: 0,
        max_name_bytes: 0,
    };
    let parsed = DirBin::parse(&[], limits).unwrap();
    assert_eq!(parsed.record_count(), 0);
    assert!(parsed.records().next().is_none());
    assert_eq!(encode(&[], limits).unwrap(), []);
}
#[test]
fn concatenated_unaligned_records_preserve_order_and_exact_bytes() {
    let records = [m2(b"x"), wmo(b"yz"), m2(b"abc")];
    let bytes = encode(&records, Limits::default()).unwrap();
    assert_eq!(bytes.len(), 39 + 64 + 41);
    let parsed = DirBin::parse(&bytes, Limits::default()).unwrap();
    assert_eq!(parsed.record_count(), 3);
    assert_eq!(parsed.bytes(), bytes);
    let mut iter = parsed.records();
    assert_eq!(iter.len(), 3);
    for original in records {
        same(iter.next().unwrap(), original);
    }
    assert_eq!(iter.len(), 0);
    assert!(iter.next().is_none());
    assert!(iter.next().is_none());
    assert_eq!(
        encode(&parsed.records().collect::<Vec<_>>(), Limits::default()).unwrap(),
        bytes
    );
}
#[test]
fn each_nonempty_partial_record_and_extra_tail_is_rejected() {
    for record in [m2(b"abc"), wmo(b"def")] {
        let mut bytes = wire(record);
        for length in 1..bytes.len() {
            assert!(
                DirBin::parse(&bytes[..length], Limits::default()).is_err(),
                "length {length}"
            );
        }
        bytes.push(0);
        assert_eq!(
            DirBin::parse(&bytes, Limits::default()).err(),
            Some(Error::Framing)
        );
    }
}
#[test]
fn full_validation_rejects_late_malformed_record_without_a_prefix_result() {
    let mut bytes = wire(m2(b"x"));
    bytes.extend(wire(wmo(b"y")));
    *bytes.last_mut().unwrap() = 0;
    assert_eq!(
        DirBin::parse(&bytes, Limits::default()).err(),
        Some(Error::Name)
    );
}
#[test]
fn codec_accepts_all_known_flags_without_inferring_model_class() {
    for flags in 0..8 {
        let mut p = m2(b"x");
        p.flags = flags;
        p.name_set = 17;
        p.bounds = if flags & 1 != 0 { Some([0; 6]) } else { None };
        let bytes = wire(p);
        let decoded = DirBin::parse(&bytes, Limits::default())
            .unwrap()
            .records()
            .next()
            .unwrap();
        same(decoded, p);
    }
}
#[test]
fn unknown_codec_flag_bits_and_inconsistent_writer_bounds_are_rejected() {
    for flags in [8, 0x80, 255] {
        let mut bytes = wire(m2(b"x"));
        bytes[0] = flags;
        assert_eq!(
            DirBin::parse(&bytes, Limits::default()).err(),
            Some(Error::Flags)
        );
    }
    let mut p = m2(b"x");
    p.flags = 1;
    assert_eq!(encode(&[p], Limits::default()).err(), Some(Error::Flags));
    p.flags = 0;
    p.bounds = Some([0; 6]);
    assert_eq!(encode(&[p], Limits::default()).err(), Some(Error::Flags));
}
#[test]
fn codec_validates_finite_transforms_and_bounds_without_positive_scale_gate() {
    for at in [6, 10, 14, 18, 22, 26, 30, 34, 38, 42, 46, 50, 54] {
        let mut bytes = wire(wmo(b"x"));
        put(&mut bytes, at, 0x7fc0_0001);
        assert_eq!(
            DirBin::parse(&bytes, Limits::default()).err(),
            Some(Error::NonFinite)
        );
    }
    let mut bytes = wire(wmo(b"x"));
    put(&mut bytes, 34, 100f32.to_bits());
    assert_eq!(
        DirBin::parse(&bytes, Limits::default()).err(),
        Some(Error::InvertedBounds)
    );
    for bits in [0, 0x8000_0000, (-1f32).to_bits()] {
        let mut p = m2(b"x");
        p.scale = bits;
        let bytes = wire(p);
        assert_eq!(
            DirBin::parse(&bytes, Limits::default())
                .unwrap()
                .records()
                .next()
                .unwrap()
                .scale_bits(),
            bits
        );
    }
}
#[test]
fn declared_name_length_is_bounded_before_extent_or_allocation() {
    let mut bytes = wire(m2(b"x"));
    put(&mut bytes, 34, u32::MAX);
    assert_eq!(
        DirBin::parse(&bytes, Limits::default()).err(),
        Some(Error::Limit)
    );
    put(&mut bytes, 34, 2);
    assert_eq!(
        DirBin::parse(&bytes, Limits::default()).err(),
        Some(Error::Framing)
    );
    put(&mut bytes, 34, 0);
    bytes.truncate(38);
    assert_eq!(
        DirBin::parse(&bytes, Limits::default()).err(),
        Some(Error::Name)
    );
}
#[test]
fn byte_record_name_caps_are_reducible_and_exact_boundaries_work() {
    let p = m2(b"abc");
    let bytes = wire(p);
    let exact = Limits {
        max_bytes: 41,
        max_records: 1,
        max_name_bytes: 3,
    };
    assert_eq!(encode(&[p], exact).unwrap(), bytes);
    assert!(DirBin::parse(&bytes, exact).is_ok());
    for limits in [
        Limits {
            max_bytes: 40,
            ..exact
        },
        Limits {
            max_records: 0,
            ..exact
        },
        Limits {
            max_name_bytes: 2,
            ..exact
        },
    ] {
        assert_eq!(encode(&[p], limits).err(), Some(Error::Limit));
        assert_eq!(DirBin::parse(&bytes, limits).err(), Some(Error::Limit));
    }
    assert_eq!(encode(&[p, p], exact).err(), Some(Error::Limit));
    let two = encode(&[p, p], Limits::default()).unwrap();
    assert_eq!(
        DirBin::parse(
            &two,
            Limits {
                max_records: 1,
                ..Limits::default()
            }
        )
        .err(),
        Some(Error::Limit)
    );
}
#[test]
fn raised_limits_are_invalid_before_parsing_or_writing() {
    for limits in [
        Limits {
            max_bytes: MAX_BYTES + 1,
            ..Limits::default()
        },
        Limits {
            max_records: MAX_RECORDS + 1,
            ..Limits::default()
        },
        Limits {
            max_name_bytes: MAX_NAME_BYTES + 1,
            ..Limits::default()
        },
    ] {
        assert_eq!(Mddf::parse(&[], limits).err(), Some(Error::InvalidLimits));
        assert_eq!(Modf::parse(&[], limits).err(), Some(Error::InvalidLimits));
        assert_eq!(DirBin::parse(&[], limits).err(), Some(Error::InvalidLimits));
        assert_eq!(encode(&[], limits).err(), Some(Error::InvalidLimits));
    }
}
#[test]
fn source_and_projection_respect_reduced_input_and_record_budgets() {
    let b = mddf();
    assert_eq!(
        Mddf::parse(
            &b,
            Limits {
                max_bytes: 35,
                ..Limits::default()
            }
        )
        .err(),
        Some(Error::Limit)
    );
    assert_eq!(
        Mddf::parse(
            &b,
            Limits {
                max_records: 0,
                ..Limits::default()
            }
        )
        .err(),
        Some(Error::Limit)
    );
    let b = modf();
    assert_eq!(
        Modf::parse(
            &b,
            Limits {
                max_bytes: 63,
                ..Limits::default()
            }
        )
        .err(),
        Some(Error::Limit)
    );
    let s = Modf::parse(&b, Limits::default()).unwrap();
    assert_eq!(
        project_wmo(
            &s,
            0,
            b"x",
            context(),
            Limits {
                max_bytes: 62,
                ..Limits::default()
            }
        )
        .err(),
        Some(Error::Limit)
    );
    assert!(
        project_wmo(
            &s,
            0,
            b"x",
            context(),
            Limits {
                max_bytes: 63,
                ..Limits::default()
            }
        )
        .is_ok()
    );
    let mut malformed = m2(b"x");
    malformed.rotation[0] = 0x7f80_0000;
    assert_eq!(
        encode(&[m2(b"a"), malformed], Limits::default()).err(),
        Some(Error::NonFinite)
    );
}
