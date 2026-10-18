// SPDX-License-Identifier: GPL-3.0-or-later
//! Original synthetic section/container fixtures; no client-derived bytes.
use super::*;
use crate::{Adt, Limits, liquid};

fn set16(raw: &mut [u8], offset: usize, value: u16) {
    raw[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}
fn set32(raw: &mut [u8], offset: usize, value: u32) {
    raw[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}
fn area(full: bool) -> AreaSection {
    let mut ids = [0x1234; 256];
    if full {
        ids[255] = 0xabcd;
    }
    AreaSection::from_ids(&ids)
}
fn heights(encoding: Encoding, flight: bool) -> HeightSection {
    let mut v9 = vec![0; height::V9_COUNT];
    let v8 = vec![0; height::V8_COUNT];
    v9[0] = match encoding {
        Encoding::Flat => 0,
        Encoding::Uint8 => 1f32.to_bits(),
        Encoding::Uint16 => 8f32.to_bits(),
        Encoding::Float32 => 2048f32.to_bits(),
    };
    let bounds = height::FlightBounds {
        max: [1, -2, 3, -4, 5, -6, 7, -8, 9],
        min: [i16::MIN, i16::MAX, 0, 0, 0, 0, 0, 0, 0],
    };
    let section = HeightSection::from_bits(
        &v9,
        &v8,
        height::Options::default(),
        flight.then_some(&bounds),
        height::MAX_SECTION_BYTES,
    )
    .unwrap();
    assert_eq!(section.encoding(), encoding);
    section
}
fn liquids(full: bool, has_heights: bool, empty: bool) -> LiquidSection {
    let show = vec![u8::from(!empty); liquid::SHOW_COUNT];
    let mut entries = vec![7; 256];
    let flags = vec![0x81; 256];
    let mut values = vec![7f32.to_bits(); liquid::HEIGHT_COUNT];
    if full {
        entries[255] = 9;
    }
    if has_heights {
        values[0] = 8f32.to_bits();
    }
    LiquidSection::from_arrays(
        &show,
        &entries,
        &flags,
        &values,
        liquid_section::Options::default(),
        liquid_section::MAX_SECTION_BYTES,
    )
    .unwrap()
    .unwrap()
}
fn built(
    area: &AreaSection,
    height: &HeightSection,
    liquid: Option<&LiquidSection>,
    holes: Option<&[u8; HOLE_BYTES]>,
) -> Vec<u8> {
    assemble(
        70205,
        MapSections {
            area,
            height,
            liquid,
            holes,
        },
        MAX_MAP_BYTES,
    )
    .unwrap()
}
// An independent byte envelope enables malformed-section checks without
// manufacturing invalid typed production objects or bypassing their privacy.
fn envelope(area: &[u8], height: &[u8], liquid: Option<&[u8]>, holes: Option<&[u8]>) -> Vec<u8> {
    let mut out = vec![0; HEADER_BYTES];
    out[..4].copy_from_slice(b"MAPS");
    set32(&mut out, 4, 10);
    set32(&mut out, 8, 70205);
    for (index, piece) in [Some(area), Some(height), liquid, holes]
        .into_iter()
        .enumerate()
    {
        if let Some(bytes) = piece {
            let offset = out.len() as u32;
            set32(&mut out, 12 + index * 8, offset);
            set32(&mut out, 16 + index * 8, bytes.len() as u32);
            out.extend(bytes);
        }
    }
    out
}
fn check(bytes: &[u8]) -> Result<MapView<'_>, Error> {
    verify(bytes, 70205, MAX_MAP_BYTES)
}

#[test]
fn constants_and_error_reports_are_fixed() {
    assert_eq!(HEADER_BYTES, 44);
    assert_eq!(AREA_FULL_BYTES, 520);
    assert_eq!(HOLE_BYTES, 2048);
    assert_eq!(MAX_MAP_BYTES, 202112);
    for (error, code) in [
        (Error::UnsupportedBuild, "maps_unsupported_build"),
        (Error::BuildMismatch, "maps_build_mismatch"),
        (Error::Limit, "maps_policy_limit"),
        (Error::Header, "maps_invalid_header"),
        (Error::Layout, "maps_invalid_layout"),
        (Error::Area, "maps_invalid_area"),
        (Error::Height, "maps_invalid_height"),
        (Error::Liquid, "maps_invalid_liquid"),
        (Error::Holes, "maps_invalid_holes"),
    ] {
        assert_eq!(error.code(), code);
        assert_eq!(error.to_string(), code);
    }
}

#[test]
fn area_uniform_header_and_full_grid_are_exact() {
    let compact = AreaSection::from_ids(&[0xabcd; 256]);
    assert_eq!(compact.bytes(), b"AREA\x01\x00\xcd\xab");
    let mut ids = [0u16; 256];
    for (index, id) in ids.iter_mut().enumerate() {
        *id = index as u16 * 257;
    }
    let full = AreaSection::from_ids(&ids);
    assert_eq!(&full.bytes()[..8], b"AREA\0\0\0\0");
    assert_eq!(full.bytes().len(), 520);
    for (index, id) in ids.iter().enumerate() {
        assert_eq!(short(full.bytes(), 8 + index * 2), *id);
    }
    let view = verify_area(full.bytes()).unwrap();
    assert_eq!(view.flags(), 0);
    assert_eq!(view.grid_area(), 0);
    assert_eq!(view.id(255), Some(65535));
    assert_eq!(view.id(256), None);
    assert_eq!(view.id(usize::MAX), None);
}

#[test]
fn area_projection_adapter_retains_empty_projection_semantics() {
    let mut bytes = b"REVM".to_vec();
    bytes.extend(4u32.to_le_bytes());
    bytes.extend(18u32.to_le_bytes());
    let projection = Adt::parse(&bytes, Limits::default())
        .unwrap()
        .project_heights()
        .unwrap();
    assert_eq!(
        AreaSection::from_projection(&projection).bytes(),
        b"AREA\x01\0\0\0"
    );
}

#[test]
fn minimal_map_header_has_exact_offsets_and_no_optional_sections() {
    let area = area(false);
    let height = heights(Encoding::Flat, false);
    let bytes = built(&area, &height, None, None);
    assert_eq!(bytes.len(), 68);
    assert_eq!(&bytes[..4], b"MAPS");
    let expected = [10, 70205, 44, 8, 52, 16, 0, 0, 0, 0];
    for (index, value) in expected.into_iter().enumerate() {
        assert_eq!(word(&bytes, 4 + index * 4), value);
    }
    assert_eq!(&bytes[44..52], area.bytes());
    assert_eq!(&bytes[52..68], height.bytes());
    let view = check(&bytes).unwrap();
    assert_eq!(view.build(), 70205);
    assert_eq!(view.area().id(255), Some(0x1234));
    assert_eq!(view.height().encoding(), Encoding::Flat);
    assert!(view.height().v9_samples().is_empty());
    assert!(view.height().v8_samples().is_empty());
    assert!(view.liquid().is_none());
    assert!(view.holes().is_none());
    assert_eq!(view.bytes().as_ptr(), bytes.as_ptr());
    assert_eq!(view.area().bytes().as_ptr(), bytes[44..].as_ptr());
}

#[test]
fn both_supported_builds_and_exact_expected_build_are_enforced() {
    let area = area(false);
    let height = heights(Encoding::Flat, false);
    let sections = MapSections {
        area: &area,
        height: &height,
        liquid: None,
        holes: None,
    };
    for build in [70170, 70205] {
        let bytes = assemble(build, sections, MAX_MAP_BYTES).unwrap();
        assert_eq!(verify(&bytes, build, MAX_MAP_BYTES).unwrap().build(), build);
        let other = if build == 70170 { 70205 } else { 70170 };
        assert_eq!(
            verify(&bytes, other, MAX_MAP_BYTES).err(),
            Some(Error::BuildMismatch)
        );
    }
    for build in [0, 51943, 70171, u32::MAX] {
        assert_eq!(
            assemble(build, sections, MAX_MAP_BYTES).err(),
            Some(Error::UnsupportedBuild)
        );
        assert_eq!(verify(&[], build, 0).err(), Some(Error::UnsupportedBuild));
        let mut bytes = built(&area, &height, None, None);
        set32(&mut bytes, 8, build);
        assert_eq!(check(&bytes).err(), Some(Error::UnsupportedBuild));
    }
}

#[test]
fn all_height_encodings_flight_tails_and_unaligned_offsets_roundtrip() {
    let area = area(true);
    let liquid = liquids(false, false, false);
    let holes = [0x91; HOLE_BYTES];
    for (encoding, width) in [
        (Encoding::Flat, 0),
        (Encoding::Uint8, 1),
        (Encoding::Uint16, 2),
        (Encoding::Float32, 4),
    ] {
        for flight in [false, true] {
            let height = heights(encoding, flight);
            let bytes = built(&area, &height, Some(&liquid), Some(&holes));
            let view = check(&bytes).unwrap();
            assert_eq!(view.height().encoding(), encoding);
            assert_eq!(view.height().v9_samples().len(), height::V9_COUNT * width);
            assert_eq!(view.height().v8_samples().len(), height::V8_COUNT * width);
            assert_eq!(view.height().bytes(), height.bytes());
            let liquid_offset = word(&bytes, 28) as usize;
            assert_eq!(liquid_offset, 44 + 520 + height.bytes().len());
            if encoding == Encoding::Uint8 {
                assert_eq!(liquid_offset % 2, 1);
            }
            if encoding == Encoding::Uint16 {
                assert_eq!(liquid_offset % 4, 2);
            }
            let bounds = view.height().flight_bounds();
            assert_eq!(bounds.is_some(), flight);
            if let Some(raw) = bounds {
                assert_eq!(&raw[..4], &[1, 0, 254, 255]);
                assert_eq!(&raw[18..22], &[0, 128, 255, 127]);
            }
            assert_eq!(view.holes(), Some(&holes));
        }
    }
}

#[test]
fn optional_holes_and_liquid_are_independent_and_zero_holes_are_retained() {
    let area = area(false);
    let height = heights(Encoding::Flat, false);
    let liquid = liquids(false, false, true);
    let holes = [0; HOLE_BYTES];
    for with_liquid in [false, true] {
        for with_holes in [false, true] {
            let bytes = built(
                &area,
                &height,
                with_liquid.then_some(&liquid),
                with_holes.then_some(&holes),
            );
            let view = check(&bytes).unwrap();
            assert_eq!(view.liquid().is_some(), with_liquid);
            assert_eq!(view.holes().is_some(), with_holes);
            if with_holes {
                assert_eq!(view.holes(), Some(&holes));
                assert_eq!(
                    word(&bytes, 36) as usize,
                    68 + if with_liquid { 16 } else { 0 }
                );
                assert_eq!(word(&bytes, 40), 2048);
            } else {
                assert_eq!(word(&bytes, 36), 0);
                assert_eq!(word(&bytes, 40), 0);
            }
        }
    }
}

#[test]
fn liquid_views_cover_full_uniform_heights_flat_empty_and_opaque_flags() {
    let area = area(false);
    let height = heights(Encoding::Flat, false);
    for full in [false, true] {
        for (samples, empty) in [(false, false), (true, false), (false, true)] {
            let liquid = liquids(full, samples, empty);
            let bytes = built(&area, &height, Some(&liquid), None);
            let view = check(&bytes).unwrap().liquid().unwrap();
            assert_eq!(view.bytes(), liquid.bytes());
            assert_eq!(view.flags(), liquid.flags());
            assert_eq!(view.level_bits(), liquid.level_bits());
            assert_eq!(view.liquid_flags(), if full { 0 } else { 0x81 });
            assert_eq!(view.liquid_type(), if full { 0 } else { 7 });
            assert_eq!(view.entry_bytes().is_some(), full);
            assert_eq!(view.flag_bytes().is_some(), full);
            if full {
                assert_eq!(short(view.entry_bytes().unwrap(), 510), 9);
                assert!(view.flag_bytes().unwrap().iter().all(|&flag| flag == 0x81));
            }
            assert_eq!(
                view.height_bytes().len(),
                if samples { 16641 * 4 } else { 0 }
            );
            assert_eq!(
                (view.offset_x(), view.offset_y()),
                if empty { (128, 128) } else { (0, 0) }
            );
            assert_eq!(
                (view.width(), view.height()),
                if empty { (130, 130) } else { (129, 129) }
            );
        }
    }
}

#[test]
fn maximum_container_size_and_exact_caps_are_enforced() {
    let area = area(true);
    let height = heights(Encoding::Float32, true);
    let liquid = liquids(true, true, false);
    let holes = [0xff; HOLE_BYTES];
    let sections = MapSections {
        area: &area,
        height: &height,
        liquid: Some(&liquid),
        holes: Some(&holes),
    };
    let bytes = assemble(70205, sections, MAX_MAP_BYTES).unwrap();
    assert_eq!(bytes.len(), MAX_MAP_BYTES);
    assert_eq!(
        assemble(70205, sections, MAX_MAP_BYTES - 1).err(),
        Some(Error::Limit)
    );
    assert_eq!(
        verify(&bytes, 70205, MAX_MAP_BYTES - 1).err(),
        Some(Error::Limit)
    );
    assert!(verify(&bytes, 70205, usize::MAX).is_ok());
    let mut oversized = bytes;
    oversized.push(0);
    assert_eq!(
        verify(&oversized, 70205, usize::MAX).err(),
        Some(Error::Limit)
    );
}

#[test]
fn every_minimal_prefix_and_bad_header_is_rejected_without_panics() {
    let bytes = built(&area(false), &heights(Encoding::Flat, false), None, None);
    for length in 0..bytes.len() {
        assert!(check(&bytes[..length]).is_err());
    }
    for offset in [0, 3, 4, 7] {
        let mut bad = bytes.clone();
        bad[offset] ^= 0xff;
        assert_eq!(check(&bad).err(), Some(Error::Header));
    }
    assert_eq!(verify(&bytes, 70205, 67).err(), Some(Error::Limit));
}

#[test]
fn absent_pairs_required_sections_order_and_extents_are_strict() {
    let bytes = built(
        &area(true),
        &heights(Encoding::Uint8, false),
        Some(&liquids(false, false, false)),
        Some(&[0; HOLE_BYTES]),
    );
    for offset in [12, 20, 28, 36] {
        for (target, value) in [
            (offset, 0),
            (offset + 4, 0),
            (offset, word(&bytes, offset) + 1),
            (offset, u32::MAX),
            (offset + 4, u32::MAX),
        ] {
            let mut bad = bytes.clone();
            set32(&mut bad, target, value);
            assert_eq!(check(&bad).err(), Some(Error::Layout));
        }
    }
    for offset in [12, 20] {
        let mut bad = bytes.clone();
        set32(&mut bad, offset, 0);
        set32(&mut bad, offset + 4, 0);
        assert_eq!(check(&bad).err(), Some(Error::Layout));
    }
    let mut bad = bytes.clone();
    set32(&mut bad, 20, 44);
    assert_eq!(check(&bad).err(), Some(Error::Layout));
    let mut trailing = bytes;
    trailing.push(0);
    assert_eq!(check(&trailing).err(), Some(Error::Layout));
}

#[test]
fn unexpected_alignment_padding_is_rejected() {
    let mut bytes = built(
        &area(false),
        &heights(Encoding::Uint8, false),
        Some(&liquids(false, false, false)),
        None,
    );
    let offset = word(&bytes, 28) as usize;
    bytes.insert(offset, 0);
    set32(&mut bytes, 28, offset as u32 + 1);
    assert_eq!(check(&bytes).err(), Some(Error::Layout));
}

#[test]
fn area_flags_tag_sizes_and_canonical_uniform_choice_are_checked() {
    let height = heights(Encoding::Flat, false);
    for full in [false, true] {
        let original = area(full).bytes().to_vec();
        for mutation in 0..5 {
            let mut bad = original.clone();
            match mutation {
                0 => bad[0] = b'X',
                1 => set16(&mut bad, 4, 2),
                2 => {
                    bad.pop();
                }
                3 => bad.push(0),
                _ => set16(&mut bad, 4, if full { 1 } else { 0 }),
            }
            assert_eq!(
                check(&envelope(&bad, height.bytes(), None, None)).err(),
                Some(Error::Area)
            );
        }
    }
    let mut full = area(true).bytes().to_vec();
    set16(&mut full, 6, 7);
    assert_eq!(
        check(&envelope(&full, height.bytes(), None, None)).err(),
        Some(Error::Area)
    );
    set16(&mut full, 6, 0);
    set16(&mut full, 518, 0x1234);
    assert_eq!(
        check(&envelope(&full, height.bytes(), None, None)).err(),
        Some(Error::Area)
    );
}

#[test]
fn height_flags_tags_and_exact_sample_flight_sizes_are_checked() {
    let area = area(false);
    let original = heights(Encoding::Float32, false).bytes().to_vec();
    for flags in [3, 5, 6, 7, 16, 0x80000000, 0xff] {
        let mut bad = original.clone();
        set32(&mut bad, 4, flags);
        assert_eq!(
            check(&envelope(area.bytes(), &bad, None, None)).err(),
            Some(Error::Height)
        );
    }
    for encoding in [
        Encoding::Flat,
        Encoding::Uint8,
        Encoding::Uint16,
        Encoding::Float32,
    ] {
        for flight in [false, true] {
            let raw = heights(encoding, flight).bytes().to_vec();
            for mutation in 0..4 {
                let mut bad = raw.clone();
                match mutation {
                    0 => bad[0] = b'X',
                    1 => {
                        bad.pop();
                    }
                    2 => bad.push(0),
                    _ => {
                        let toggled = word(&bad, 4) ^ 8;
                        set32(&mut bad, 4, toggled);
                    }
                }
                assert_eq!(
                    check(&envelope(area.bytes(), &bad, None, None)).err(),
                    Some(Error::Height)
                );
            }
        }
    }
}

#[test]
fn height_extrema_and_float_samples_must_be_finite_and_ordered() {
    let area = area(false);
    let original = heights(Encoding::Float32, false).bytes().to_vec();
    for offset in [8, 12, 16, original.len() - 4] {
        for bits in [0x7fc00001, 0x7f800000, 0xff800000] {
            let mut bad = original.clone();
            set32(&mut bad, offset, bits);
            assert_eq!(
                check(&envelope(area.bytes(), &bad, None, None)).err(),
                Some(Error::Height)
            );
        }
    }
    let mut bad = original;
    set32(&mut bad, 8, 4096f32.to_bits());
    assert_eq!(
        check(&envelope(area.bytes(), &bad, None, None)).err(),
        Some(Error::Height)
    );
}

#[test]
fn verifier_does_not_claim_recomputed_height_packing_or_extrema() {
    let area = area(false);
    let mut raw = heights(Encoding::Flat, false).bytes().to_vec();
    set32(&mut raw, 12, 0.001f32.to_bits());
    assert!(check(&envelope(area.bytes(), &raw, None, None)).is_ok());
    let mut raw = heights(Encoding::Float32, false).bytes().to_vec();
    // Structural verification cannot reestablish original extrema/packing;
    // finite samples remain admitted without reconstructing the source scan.
    set32(&mut raw, 16, (-10f32).to_bits());
    assert!(check(&envelope(area.bytes(), &raw, None, None)).is_ok());
}

#[test]
fn liquid_flags_tags_lengths_and_type_canonicalization_are_checked() {
    let area = area(false);
    let height = heights(Encoding::Flat, false);
    let raw = liquids(true, false, false).bytes().to_vec();
    for mutation in 0..8 {
        let mut bad = raw.clone();
        match mutation {
            0 => bad[0] = b'X',
            1 => bad[4] |= 4,
            2 => bad[5] = 1,
            3 => set16(&mut bad, 6, 1),
            4 => {
                bad.pop();
            }
            5 => bad.push(0),
            6 => bad[4] ^= liquid_section::NO_TYPE,
            _ => set16(&mut bad, 16 + 255 * 2, 7),
        }
        assert_eq!(
            check(&envelope(area.bytes(), height.bytes(), Some(&bad), None)).err(),
            Some(Error::Liquid)
        );
    }
    let mut uniform = liquids(false, false, false).bytes().to_vec();
    uniform[5] = 0;
    assert_eq!(
        check(&envelope(
            area.bytes(),
            height.bytes(),
            Some(&uniform),
            None
        ))
        .err(),
        Some(Error::Liquid)
    );
}

#[test]
fn liquid_rectangles_require_normal_bounds_or_exact_empty_noheight_shape() {
    let area = area(false);
    let height = heights(Encoding::Flat, false);
    for has_heights in [false, true] {
        let original = liquids(false, has_heights, false).bytes().to_vec();
        for rect in [
            [0, 0, 0, 0],
            [0, 0, 1, 1],
            [128, 0, 2, 2],
            [0, 128, 2, 2],
            [127, 127, 3, 2],
            [0, 0, 130, 130],
            [128, 128, 130, 129],
        ] {
            let mut bad = original.clone();
            bad[8..12].copy_from_slice(&rect);
            assert_eq!(
                check(&envelope(area.bytes(), height.bytes(), Some(&bad), None)).err(),
                Some(Error::Liquid)
            );
        }
    }
    let mut empty = liquids(false, false, true).bytes().to_vec();
    empty[4] &= !liquid_section::NO_HEIGHT;
    assert_eq!(
        check(&envelope(area.bytes(), height.bytes(), Some(&empty), None)).err(),
        Some(Error::Liquid)
    );
    empty[4] |= liquid_section::NO_HEIGHT;
    set32(&mut empty, 12, 0);
    assert_eq!(
        check(&envelope(area.bytes(), height.bytes(), Some(&empty), None)).err(),
        Some(Error::Liquid)
    );
}

#[test]
fn liquid_level_and_emitted_samples_are_finite() {
    let area = area(false);
    let height = heights(Encoding::Flat, false);
    let raw = liquids(false, true, false).bytes().to_vec();
    for offset in [12, 16, raw.len() - 4] {
        for bits in [0x7fc00001, 0x7f800000, 0xff800000] {
            let mut bad = raw.clone();
            set32(&mut bad, offset, bits);
            assert_eq!(
                check(&envelope(area.bytes(), height.bytes(), Some(&bad), None)).err(),
                Some(Error::Liquid)
            );
        }
    }
}

#[test]
fn holes_have_exact_size_and_all_raw_bit_patterns_are_preserved() {
    let area = area(false);
    let height = heights(Encoding::Flat, false);
    for size in [1, 2047, 2049] {
        assert_eq!(
            check(&envelope(
                area.bytes(),
                height.bytes(),
                None,
                Some(&vec![0; size])
            ))
            .err(),
            Some(Error::Holes)
        );
    }
    let mut holes = [0; HOLE_BYTES];
    for (index, value) in holes.iter_mut().enumerate() {
        *value = index as u8;
    }
    let bytes = built(&area, &height, None, Some(&holes));
    assert_eq!(check(&bytes).unwrap().holes(), Some(&holes));
}

#[test]
fn assembly_and_verification_do_not_modify_inputs() {
    let area = area(true);
    let height = heights(Encoding::Uint16, true);
    let liquid = liquids(true, true, false);
    let holes = [0xa5; HOLE_BYTES];
    let before = (
        area.bytes().to_vec(),
        height.bytes().to_vec(),
        liquid.bytes().to_vec(),
        holes,
    );
    let bytes = built(&area, &height, Some(&liquid), Some(&holes));
    let bytes_before = bytes.clone();
    assert!(check(&bytes).is_ok());
    assert_eq!(area.bytes(), before.0);
    assert_eq!(height.bytes(), before.1);
    assert_eq!(liquid.bytes(), before.2);
    assert_eq!(holes, before.3);
    assert_eq!(bytes, bytes_before);
}

#[test]
fn current_70374_entrypoints_do_not_widen_legacy_gates() {
    let a = area(true);
    let h = heights(Encoding::Uint8, true);
    let l = liquids(true, true, false);
    let sections = MapSections {
        area: &a,
        height: &h,
        liquid: Some(&l),
        holes: None,
    };
    let bytes = assemble_70374(70374, sections, MAX_MAP_BYTES).unwrap();
    assert_eq!(
        verify_70374(&bytes, 70374, MAX_MAP_BYTES).unwrap().build(),
        70374
    );
    assert_eq!(
        assemble(70374, sections, MAX_MAP_BYTES).err(),
        Some(Error::UnsupportedBuild)
    );
    assert_eq!(
        verify(&bytes, 70374, MAX_MAP_BYTES).err(),
        Some(Error::UnsupportedBuild)
    );
    for build in [0, 70170, 70205, 70338, u32::MAX] {
        assert_eq!(
            assemble_70374(build, sections, MAX_MAP_BYTES).err(),
            Some(Error::UnsupportedBuild)
        );
        assert_eq!(
            verify_70374(&bytes, build, MAX_MAP_BYTES).err(),
            Some(Error::UnsupportedBuild)
        );
    }
    let mut legacy = bytes.clone();
    set32(&mut legacy, 8, 70205);
    assert!(verify(&legacy, 70205, MAX_MAP_BYTES).is_ok());
    assert_eq!(
        verify_70374(&legacy, 70374, MAX_MAP_BYTES).err(),
        Some(Error::UnsupportedBuild)
    );
    assert_eq!(
        verify_70374(&bytes, 70374, bytes.len() - 1).err(),
        Some(Error::Limit)
    );
    let mut trailing = bytes;
    trailing.push(0);
    assert_eq!(
        verify_70374(&trailing, 70374, MAX_MAP_BYTES).err(),
        Some(Error::Layout)
    );
}

#[test]
fn build70450_maps_v10_round_trip_and_target_gates_are_explicit() {
    assert_eq!(
        BUILD_70450_FORMAT_ASSUMPTION,
        "build 70450 MAPS output assumes the build 70374 MAPS v10 wire layout"
    );
    let area = area(true);
    let height = heights(Encoding::Uint8, true);
    let liquid = liquids(true, true, false);
    let sections = MapSections {
        area: &area,
        height: &height,
        liquid: Some(&liquid),
        holes: None,
    };
    let out_70450 = assemble_70450(70450, sections, MAX_MAP_BYTES).unwrap();
    let out_70374 = assemble_70374(70374, sections, MAX_MAP_BYTES).unwrap();
    let mut expected = out_70374.clone();
    expected[8..12].copy_from_slice(&70450u32.to_le_bytes());
    assert_eq!(out_70450, expected);
    assert_eq!(
        verify_70450(&out_70450, 70450, MAX_MAP_BYTES)
            .unwrap()
            .build(),
        70450
    );

    assert_eq!(
        assemble_70450(70374, sections, MAX_MAP_BYTES).err(),
        Some(Error::UnsupportedBuild)
    );
    assert_eq!(
        assemble_70374(70450, sections, MAX_MAP_BYTES).err(),
        Some(Error::UnsupportedBuild)
    );
    assert_eq!(
        verify_70450(&out_70374, 70450, MAX_MAP_BYTES).err(),
        Some(Error::UnsupportedBuild)
    );
    assert_eq!(
        verify_70374(&out_70450, 70374, MAX_MAP_BYTES).err(),
        Some(Error::UnsupportedBuild)
    );
    let mut wrong_format = out_70450;
    set32(&mut wrong_format, 4, VERSION + 1);
    assert_eq!(
        verify_70450(&wrong_format, 70450, MAX_MAP_BYTES).err(),
        Some(Error::Header)
    );
}
