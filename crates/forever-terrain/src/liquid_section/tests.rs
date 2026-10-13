// SPDX-License-Identifier: GPL-3.0-or-later
//! Original synthetic projected arrays only. No client files or upstream data.
use super::*;
use crate::{Adt, Limits, liquid};

#[derive(Clone)]
struct Input {
    show: Vec<u8>,
    entries: Vec<u16>,
    flags: Vec<u8>,
    heights: Vec<u32>,
}
impl Default for Input {
    fn default() -> Self {
        Self {
            show: vec![0; SHOW_COUNT],
            entries: vec![0; CELL_COUNT],
            flags: vec![0; CELL_COUNT],
            heights: vec![0; HEIGHT_COUNT],
        }
    }
}
impl Input {
    fn uniform(flag: u8, entry: u16, value: f32) -> Self {
        Self {
            show: vec![1; SHOW_COUNT],
            entries: vec![entry; CELL_COUNT],
            flags: vec![flag; CELL_COUNT],
            heights: vec![value.to_bits(); HEIGHT_COUNT],
        }
    }
    fn make(&self, packing: bool, limit: usize) -> Result<Option<LiquidSection>, Error> {
        LiquidSection::from_arrays(
            &self.show,
            &self.entries,
            &self.flags,
            &self.heights,
            Options {
                pack_integers: packing,
            },
            limit,
        )
    }
    fn section(&self, packing: bool) -> LiquidSection {
        self.make(packing, MAX_SECTION_BYTES).unwrap().unwrap()
    }
    fn set_height(&mut self, x: usize, y: usize, height: f32) {
        self.heights[y * 129 + x] = height.to_bits();
    }
    fn visible(&mut self, x: usize, y: usize) {
        self.show[y * 128 + x] = 1;
    }
}
fn u16_at(data: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes(data[offset..offset + 2].try_into().unwrap())
}
fn u32_at(data: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap())
}
fn height_at(section: &LiquidSection, x: usize, y: usize) -> u32 {
    let start = HEADER_BYTES
        + if section.flags() & NO_TYPE == 0 {
            TYPE_BYTES
        } else {
            0
        };
    u32_at(
        section.bytes(),
        start + (y * usize::from(section.width()) + x) * 4,
    )
}

#[test]
fn constants_options_and_fixed_error_codes() {
    assert_eq!(MAX_SECTION_BYTES, 67348);
    assert_eq!(HEADER_BYTES, 16);
    assert_eq!(TYPE_BYTES, 768);
    assert!(Options::default().pack_integers);
    for (error, code) in [
        (Error::InvalidShape, "liquid_section_invalid_shape"),
        (
            Error::InvalidVisibility,
            "liquid_section_invalid_visibility",
        ),
        (Error::NonFiniteHeight, "liquid_section_nonfinite_height"),
        (Error::Limit, "liquid_section_policy_limit"),
    ] {
        assert_eq!(error.code(), code);
        assert_eq!(error.to_string(), code);
    }
}

#[test]
fn uniform_zero_flag_omits_regardless_of_entry_visibility_height_and_cap() {
    for entry in [0, 1, u16::MAX] {
        for packing in [false, true] {
            for visible in [0, 1] {
                let mut input = Input::uniform(0, entry, -3500.0);
                input.show.fill(visible);
                input.set_height(128, 128, f32::MAX);
                assert!(input.make(packing, 0).unwrap().is_none());
            }
        }
    }
}

#[test]
fn uniform_flat_header_is_exact_little_endian_and_all_getters_match() {
    let input = Input::uniform(0x91, 0xabcd, 12.5);
    let section = input.section(true);
    let expected = [
        b'M', b'L', b'I', b'Q', 3, 0x91, 0xcd, 0xab, 0, 0, 129, 129, 0, 0, 0x48, 0x41,
    ];
    assert_eq!(section.bytes(), expected);
    assert_eq!(section.flags(), NO_TYPE | NO_HEIGHT);
    assert_eq!(section.liquid_flags(), 0x91);
    assert_eq!(section.liquid_type(), 0xabcd);
    assert_eq!((section.offset_x(), section.offset_y()), (0, 0));
    assert_eq!((section.width(), section.height()), (129, 129));
    assert_eq!(section.level_bits(), 12.5f32.to_bits());
    assert_eq!(section.into_bytes(), expected);
}

#[test]
fn full_types_are_detected_at_last_pair_and_header_defaults_are_zero() {
    for change_entry in [false, true] {
        let mut input = Input::uniform(0, 0x1234, 0.0);
        if change_entry {
            input.entries[255] = 0xabcd;
        } else {
            input.flags[255] = 0x80;
        }
        let section = input.section(false);
        assert_eq!(section.flags(), NO_HEIGHT);
        assert_eq!(section.liquid_flags(), 0);
        assert_eq!(section.liquid_type(), 0);
        assert_eq!(section.bytes().len(), HEADER_BYTES + TYPE_BYTES);
        for (index, &entry) in input.entries.iter().enumerate() {
            assert_eq!(u16_at(section.bytes(), 16 + index * 2), entry);
        }
        assert_eq!(&section.bytes()[528..784], input.flags.as_slice());
    }
}

#[test]
fn opaque_flag_bits_are_preserved_in_uniform_and_full_type_forms() {
    for flag in [1, 0x20, 0x80, 0xff] {
        let mut input = Input::uniform(flag, 9, 1.0);
        assert_eq!(input.section(true).liquid_flags(), flag);
        input.flags[255] ^= 1;
        let section = input.section(true);
        assert_eq!(&section.bytes()[528..784], input.flags.as_slice());
        assert_eq!(section.liquid_flags(), 0);
    }
}

#[test]
fn empty_visibility_retains_wrapping_rectangle_but_writes_no_heights() {
    for full in [false, true] {
        for packing in [false, true] {
            let mut input = Input::uniform(1, 7, 123.0);
            input.show.fill(0);
            if full {
                input.entries[255] = 0;
            }
            let section = input.section(packing);
            assert_eq!((section.offset_x(), section.offset_y()), (128, 128));
            assert_eq!((section.width(), section.height()), (130, 130));
            assert_eq!(section.level_bits(), (-2000.0f32).to_bits());
            assert_eq!(section.flags(), NO_HEIGHT | if full { 0 } else { NO_TYPE });
            assert_eq!(
                section.bytes().len(),
                HEADER_BYTES + if full { TYPE_BYTES } else { 0 }
            );
        }
    }
}

#[test]
fn asymmetric_sparse_crop_is_row_major_and_invisible_vertices_are_replaced() {
    let mut input = Input::default();
    input.flags[0] = 1;
    input.entries[0] = 7;
    input.visible(3, 7);
    input.visible(5, 8);
    for y in 0..129 {
        for x in 0..129 {
            input.set_height(x, y, (y * 100 + x) as f32);
        }
    }
    let section = input.section(true);
    assert_eq!((section.offset_x(), section.offset_y()), (3, 7));
    assert_eq!((section.width(), section.height()), (4, 3));
    assert_eq!(section.flags(), 0);
    assert_eq!(section.level_bits(), (-2000.0f32).to_bits());
    assert_eq!(section.bytes().len(), HEADER_BYTES + TYPE_BYTES + 4 * 3 * 4);
    for y in 0..3 {
        for x in 0..4 {
            let value = if (x, y) == (0, 0) {
                703.0
            } else if (x, y) == (2, 1) {
                805.0
            } else {
                -2000.0
            };
            assert_eq!(height_at(&section, x, y), f32::to_bits(value));
        }
    }
}

#[test]
fn last_visible_tile_includes_raw_outer_border_without_affecting_extrema() {
    let mut input = Input::default();
    input.flags.fill(1);
    input.visible(127, 127);
    input.set_height(127, 127, 7.0);
    input.set_height(128, 127, -5000.0);
    input.set_height(127, 128, 30000.0);
    input.set_height(128, 128, -0.0);
    let section = input.section(false);
    assert_eq!((section.offset_x(), section.offset_y()), (127, 127));
    assert_eq!((section.width(), section.height()), (2, 2));
    assert_eq!(section.level_bits(), (-2000.0f32).to_bits());
    for (index, expected) in [7.0f32, -5000.0, 30000.0, -0.0].into_iter().enumerate() {
        assert_eq!(
            height_at(&section, index % 2, index / 2),
            expected.to_bits()
        );
    }
}

#[test]
fn invisible_samples_outside_crop_still_affect_extrema_and_flat_selection() {
    let mut input = Input::uniform(1, 7, 5.0);
    input.show.fill(0);
    input.visible(64, 30);
    let section = input.section(true);
    assert_eq!(section.flags(), NO_TYPE);
    assert_eq!(section.level_bits(), (-2000.0f32).to_bits());
    assert_eq!(section.bytes().len(), HEADER_BYTES + 16);
    input.show.fill(1);
    assert_eq!(input.section(true).flags(), NO_TYPE | NO_HEIGHT);
}

#[test]
fn visible_heights_below_floor_are_never_clamped() {
    let mut input = Input::default();
    input.flags.fill(1);
    input.visible(0, 0);
    input.set_height(0, 0, -3000.0);
    // One visible height defines both extrema; invisible vertices raise
    // neither maximum nor an already lower minimum, so height is omitted.
    let flat = input.section(false);
    assert_eq!(flat.level_bits(), (-3000.0f32).to_bits());
    assert_eq!(flat.flags(), NO_TYPE | NO_HEIGHT);
    input.visible(1, 0);
    input.set_height(1, 0, -2500.0);
    let section = input.section(false);
    assert_eq!(section.flags(), NO_TYPE);
    assert_eq!(height_at(&section, 0, 0), (-3000.0f32).to_bits());
    assert_eq!(height_at(&section, 1, 0), (-2500.0f32).to_bits());
    assert_eq!(height_at(&section, 2, 0), (-2000.0f32).to_bits());
}

#[test]
fn positive_and_negative_source_sentinels_are_observable() {
    let above = Input::uniform(1, 0, 20001.0).section(true);
    assert_eq!(above.level_bits(), 20000.0f32.to_bits());
    assert_eq!(above.flags(), NO_TYPE);
    assert_eq!(height_at(&above, 0, 0), 20001.0f32.to_bits());
    let below = Input::uniform(1, 0, -20001.0).section(true);
    assert_eq!(below.level_bits(), (-20001.0f32).to_bits());
    assert_eq!(below.flags(), NO_TYPE);
    assert_eq!(height_at(&below, 0, 0), (-20001.0f32).to_bits());
}

#[test]
fn signed_zero_extrema_preserve_first_equal_value_in_row_order() {
    for first in [0u32, 0x80000000] {
        let mut input = Input::uniform(1, 0, f32::from_bits(first ^ 0x80000000));
        input.heights[0] = first;
        let section = input.section(true);
        assert_eq!(section.flags(), NO_TYPE | NO_HEIGHT);
        assert_eq!(section.level_bits(), first);
    }
}

#[test]
fn near_flat_threshold_is_strict_f32_and_respects_packing_switch() {
    let threshold = FLAT_DELTA_LIMIT.to_bits();
    for bits in [threshold - 1, threshold, threshold + 1] {
        for packing in [false, true] {
            let mut input = Input::uniform(1, 3, 0.0);
            input.heights[1] = bits;
            let section = input.section(packing);
            let expected_flat = packing && bits < threshold;
            assert_eq!(
                section.flags(),
                NO_TYPE | if expected_flat { NO_HEIGHT } else { 0 }
            );
            assert_eq!(section.level_bits(), 0);
            assert_eq!(
                section.bytes().len(),
                HEADER_BYTES + if expected_flat { 0 } else { HEIGHT_COUNT * 4 }
            );
        }
    }
}

#[test]
fn finite_subnormals_and_extremes_keep_bits_when_written() {
    let mut input = Input::uniform(1, 0, 0.0);
    let samples = [
        0x80000000,
        1,
        0x80000001,
        f32::MAX.to_bits(),
        (-f32::MAX).to_bits(),
    ];
    input.heights[..samples.len()].copy_from_slice(&samples);
    let section = input.section(true);
    // The finite extrema's subtraction overflows to +Inf; the threshold
    // comparison is false and all finite original heights remain writable.
    assert_eq!(section.flags(), NO_TYPE);
    assert_eq!(section.level_bits(), (-f32::MAX).to_bits());
    for (x, &bits) in samples.iter().enumerate() {
        assert_eq!(height_at(&section, x, 0), bits);
    }
}

#[test]
fn outer_border_does_not_prevent_flat_omission() {
    let mut input = Input::uniform(1, 7, 5.0);
    for y in 0..129 {
        input.set_height(128, y, -5000.0);
    }
    for x in 0..129 {
        input.set_height(x, 128, f32::MAX);
    }
    let section = input.section(false);
    assert_eq!(section.flags(), NO_TYPE | NO_HEIGHT);
    assert_eq!(section.level_bits(), 5f32.to_bits());
    assert_eq!(section.bytes().len(), HEADER_BYTES);
}

#[test]
fn maximum_full_section_and_exact_budget_are_enforced() {
    let mut input = Input::uniform(1, 7, 1.0);
    input.heights[0] = 2f32.to_bits();
    input.entries[255] = 65535;
    let section = input.section(true);
    assert_eq!(section.bytes().len(), MAX_SECTION_BYTES);
    assert_eq!(section.flags(), 0);
    assert_eq!((section.width(), section.height()), (129, 129));
    assert!(input.make(true, MAX_SECTION_BYTES).is_ok());
    assert_eq!(
        input.make(true, MAX_SECTION_BYTES - 1).err(),
        Some(Error::Limit)
    );
    assert_eq!(input.make(true, 0).err(), Some(Error::Limit));
    let flat = Input::uniform(1, 0, 0.0);
    assert!(flat.make(true, 16).is_ok());
    assert_eq!(flat.make(true, 15).err(), Some(Error::Limit));
}

#[test]
fn all_input_shapes_are_exact_before_omission() {
    for field in 0..4 {
        for delta in [-1i32, 1] {
            let mut input = Input::default();
            match field {
                0 => input.show.resize((SHOW_COUNT as i32 + delta) as usize, 0),
                1 => input
                    .entries
                    .resize((CELL_COUNT as i32 + delta) as usize, 0),
                2 => input.flags.resize((CELL_COUNT as i32 + delta) as usize, 0),
                _ => input
                    .heights
                    .resize((HEIGHT_COUNT as i32 + delta) as usize, 0),
            }
            assert_eq!(input.make(true, 0).err(), Some(Error::InvalidShape));
        }
    }
}

#[test]
fn visibility_must_be_boolean_even_for_omitted_or_flat_sections() {
    for flag in [0, 1] {
        for value in [2, 0x80, 0xff] {
            let mut input = Input::uniform(flag, 7, 0.0);
            input.show[SHOW_COUNT - 1] = value;
            assert_eq!(
                input.make(true, MAX_SECTION_BYTES).err(),
                Some(Error::InvalidVisibility)
            );
        }
    }
}

#[test]
fn every_nonfinite_input_is_rejected_even_if_omitted_invisible_or_border() {
    for bits in [0x7fc01234, 0x7f800000, 0xff800000] {
        for flag in [0, 1] {
            for index in [0, 1, HEIGHT_COUNT - 1] {
                let mut input = Input::uniform(flag, 7, 0.0);
                input.show[0] = 0;
                input.heights[index] = bits;
                assert_eq!(
                    input.make(true, MAX_SECTION_BYTES).err(),
                    Some(Error::NonFiniteHeight)
                );
            }
        }
    }
}

#[test]
fn successful_and_failed_packing_leave_all_inputs_unchanged() {
    let mut input = Input::uniform(1, 7, 1.0);
    input.show[0] = 0;
    input.entries[2] = 8;
    let before = input.clone();
    assert!(input.make(true, MAX_SECTION_BYTES).is_ok());
    assert_eq!(input.make(true, 0).err(), Some(Error::Limit));
    assert_eq!(input.show, before.show);
    assert_eq!(input.entries, before.entries);
    assert_eq!(input.flags, before.flags);
    assert_eq!(input.heights, before.heights);
}

#[test]
fn projection_adapter_preserves_public_array_contract() {
    struct NoLookup;
    impl liquid::LiquidLookup for NoLookup {
        fn liquid_type(&self, _: u32) -> Option<liquid::LiquidType> {
            None
        }
        fn material(&self, _: u32) -> Option<liquid::LiquidMaterial> {
            None
        }
    }
    let mut bytes = b"REVM".to_vec();
    bytes.extend(4u32.to_le_bytes());
    bytes.extend(18u32.to_le_bytes());
    let adt = Adt::parse(&bytes, Limits::default()).unwrap();
    let projection = liquid::project_liquids(&adt, &NoLookup, false).unwrap();
    assert!(
        LiquidSection::from_projection(&projection, Options::default(), 0)
            .unwrap()
            .is_none()
    );
}

#[test]
fn nonempty_projection_adapter_matches_array_entry_point() {
    struct NoLookup;
    impl liquid::LiquidLookup for NoLookup {
        fn liquid_type(&self, _: u32) -> Option<liquid::LiquidType> {
            None
        }
        fn material(&self, _: u32) -> Option<liquid::LiquidMaterial> {
            None
        }
    }
    fn frame(tag: &[u8; 4], payload: &[u8]) -> Vec<u8> {
        let mut bytes = tag.iter().rev().copied().collect::<Vec<_>>();
        bytes.extend((payload.len() as u32).to_le_bytes());
        bytes.extend(payload);
        bytes
    }
    let mut old = vec![0; 804];
    for index in 0..81 {
        old[12 + index * 8..16 + index * 8].copy_from_slice(&5f32.to_le_bytes());
    }
    let mut cell = vec![0; 128];
    cell[..4].copy_from_slice(&4u32.to_le_bytes());
    cell[100..104].copy_from_slice(&812u32.to_le_bytes());
    cell.extend(frame(b"MCLQ", &old));
    let mut bytes = frame(b"MVER", &18u32.to_le_bytes());
    bytes.extend(frame(b"MCNK", &cell));
    let adt = Adt::parse(&bytes, Limits::default()).unwrap();
    let projection = liquid::project_liquids(&adt, &NoLookup, false).unwrap();
    let section =
        LiquidSection::from_projection(&projection, Options::default(), MAX_SECTION_BYTES)
            .unwrap()
            .unwrap();
    let arrays = LiquidSection::from_arrays(
        projection.show(),
        projection.entries(),
        projection.flags(),
        projection.height_bits(),
        Options::default(),
        MAX_SECTION_BYTES,
    )
    .unwrap()
    .unwrap();
    assert_eq!(section.bytes(), arrays.bytes());
    assert_eq!((section.offset_x(), section.offset_y()), (0, 0));
    assert_eq!((section.width(), section.height()), (9, 9));
    assert_eq!(section.flags(), 0);
    assert_eq!(height_at(&section, 7, 7), 5f32.to_bits());
    assert_eq!(height_at(&section, 8, 8), (-2000.0f32).to_bits());
}
