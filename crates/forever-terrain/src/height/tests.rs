// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: independent synthetic MHGT boundary and byte fixtures.
use super::*;

fn samples(value: f32) -> (Vec<u32>, Vec<u32>) {
    (
        vec![value.to_bits(); V9_COUNT],
        vec![value.to_bits(); V8_COUNT],
    )
}

fn pack(v9: &[u32], v8: &[u32], enabled: bool) -> HeightSection {
    HeightSection::from_bits(
        v9,
        v8,
        Options {
            pack_integers: enabled,
        },
        None,
        MAX_SECTION_BYTES,
    )
    .unwrap()
}

fn payload_word(section: &HeightSection, ordinal: usize) -> u32 {
    let offset = HEADER_BYTES + ordinal * 4;
    u32::from_le_bytes(section.bytes()[offset..offset + 4].try_into().unwrap())
}

#[test]
fn flat_header_has_literal_tag_flags_and_float_bytes() {
    let (v9, v8) = samples(7.5);
    let expected = [
        b'M', b'H', b'G', b'T', 1, 0, 0, 0, 0, 0, 0xf0, 0x40, 0, 0, 0xf0, 0x40,
    ];
    for enabled in [false, true] {
        let section = pack(&v9, &v8, enabled);
        assert_eq!(section.bytes(), expected);
        assert_eq!(section.encoding(), Encoding::Flat);
        assert_eq!(section.flags(), NO_HEIGHT);
        assert_eq!(section.min_height_bits(), 0x40f00000);
        assert_eq!(section.max_height_bits(), 0x40f00000);
        assert_eq!(section.into_bytes(), expected);
    }
}

#[test]
fn zero_sign_follows_first_v8_sample_and_strict_comparisons() {
    for initial in [0.0_f32, -0.0_f32] {
        let (mut v9, mut v8) = samples(-initial);
        v8[0] = initial.to_bits();
        v9[V9_COUNT - 1] = initial.to_bits();
        let section = pack(&v9, &v8, true);
        assert_eq!(section.encoding(), Encoding::Flat);
        assert_eq!(section.min_height_bits(), initial.to_bits());
        assert_eq!(section.max_height_bits(), initial.to_bits());
    }
}

#[test]
fn unsigned_byte_payload_is_v9_then_v8_and_rounds_half_up() {
    let (mut v9, mut v8) = samples(0.0);
    for (index, value) in [0.0_f32, 0.5, 1.0].into_iter().enumerate() {
        v9[index] = value.to_bits();
    }
    v9[V9_COUNT - 1] = 0.5_f32.to_bits();
    v8[0] = 1.0_f32.to_bits();
    v8[V8_COUNT - 1] = 0.5_f32.to_bits();
    let section = pack(&v9, &v8, true);
    assert_eq!(section.encoding(), Encoding::Uint8);
    assert_eq!(section.flags(), HEIGHT_AS_INT8);
    assert_eq!(section.bytes().len(), HEADER_BYTES + V9_COUNT + V8_COUNT);
    let mut expected = vec![0; V9_COUNT + V8_COUNT];
    expected[..3].copy_from_slice(&[0, 128, 255]);
    expected[V9_COUNT - 1] = 128;
    expected[V9_COUNT] = 255;
    expected[V9_COUNT + V8_COUNT - 1] = 128;
    assert_eq!(&section.bytes()[HEADER_BYTES..], expected);
}

#[test]
fn unsigned_short_payload_is_little_endian_and_preserves_order() {
    let (mut v9, mut v8) = samples(0.0);
    for (index, value) in [0.0_f32, 0.5, 4.0, 8.0].into_iter().enumerate() {
        v9[index] = value.to_bits();
    }
    v9[V9_COUNT - 1] = 8.0_f32.to_bits();
    v8[0] = 4.0_f32.to_bits();
    v8[V8_COUNT - 1] = 0.5_f32.to_bits();
    let section = pack(&v9, &v8, true);
    assert_eq!(section.encoding(), Encoding::Uint16);
    assert_eq!(section.flags(), HEIGHT_AS_INT16);
    let mut expected = vec![0; (V9_COUNT + V8_COUNT) * 2];
    expected[..8].copy_from_slice(&[0, 0, 0, 16, 0, 128, 255, 255]);
    expected[V9_COUNT * 2 - 2..V9_COUNT * 2].copy_from_slice(&[255, 255]);
    expected[V9_COUNT * 2..V9_COUNT * 2 + 2].copy_from_slice(&[0, 128]);
    let end = expected.len();
    expected[end - 2..].copy_from_slice(&[0, 16]);
    assert_eq!(&section.bytes()[HEADER_BYTES..], expected);
}

#[test]
fn float_payload_keeps_raw_finite_bits_and_v9_then_v8() {
    let (mut v9, mut v8) = samples(0.0);
    v9[..5].copy_from_slice(&[
        (-0.0_f32).to_bits(),
        1,
        0x80000001,
        0x3eaaaaab,
        2048.0_f32.to_bits(),
    ]);
    v8[0] = 129.125_f32.to_bits();
    v8[V8_COUNT - 1] = 0.25_f32.to_bits();
    let section = pack(&v9, &v8, true);
    assert_eq!(section.encoding(), Encoding::Float32);
    assert_eq!(section.flags(), 0);
    let expected = v9
        .iter()
        .chain(&v8)
        .flat_map(|bits| bits.to_le_bytes())
        .collect::<Vec<_>>();
    assert_eq!(&section.bytes()[HEADER_BYTES..], expected);
}

#[test]
fn all_thresholds_are_strict_f32_comparisons() {
    let (mut v9, v8) = samples(0.0);
    for (threshold, below, at) in [
        (0.005_f32, Encoding::Flat, Encoding::Uint8),
        (2.0_f32, Encoding::Uint8, Encoding::Uint16),
        (2048.0_f32, Encoding::Uint16, Encoding::Float32),
    ] {
        for (bits, expected) in [
            (threshold.to_bits() - 1, below),
            (threshold.to_bits(), at),
            (threshold.to_bits() + 1, at),
        ] {
            v9[0] = bits;
            let section = pack(&v9, &v8, true);
            assert_eq!(
                section.encoding(),
                expected,
                "threshold {threshold:?}, bits {bits:x}"
            );
            assert_eq!(section.max_height_bits(), bits);
            assert_eq!(section.min_height_bits(), 0);
        }
    }
}

#[test]
fn packing_disabled_retains_small_variations_as_float() {
    let (mut v9, v8) = samples(0.0);
    for difference in [f32::from_bits(1), 0.001, 0.005, 1.0, 2.0, 2047.0] {
        v9[0] = difference.to_bits();
        let section = pack(&v9, &v8, false);
        assert_eq!(section.encoding(), Encoding::Float32);
        assert_eq!(payload_word(&section, 0), difference.to_bits());
    }
}

#[test]
fn subnormal_variation_is_flat_only_when_packing_enabled() {
    let (mut v9, v8) = samples(0.0);
    v9[0] = 1;
    let section = pack(&v9, &v8, true);
    assert_eq!(section.encoding(), Encoding::Flat);
    assert_eq!(section.min_height_bits(), 0);
    assert_eq!(section.max_height_bits(), 1);
}

#[test]
fn minimum_clamp_applies_to_both_arrays_before_serialization() {
    let (mut v9, mut v8) = samples(-2000.0);
    v9[0] = (-2000.25_f32).to_bits();
    v9[1] = (-1999.0_f32).to_bits();
    v8[0] = (-f32::MAX).to_bits();
    let section = pack(&v9, &v8, true);
    assert_eq!(section.encoding(), Encoding::Uint8);
    assert_eq!(section.min_height_bits(), (-2000.0_f32).to_bits());
    assert_eq!(section.max_height_bits(), (-1999.0_f32).to_bits());
    assert_eq!(&section.bytes()[16..18], &[0, 255]);
    assert_eq!(section.bytes()[HEADER_BYTES + V9_COUNT], 0);
    let raw = pack(&v9, &v8, false);
    assert_eq!(raw.encoding(), Encoding::Float32);
    assert_eq!(payload_word(&raw, 0), (-2000.0_f32).to_bits());
    assert_eq!(payload_word(&raw, V9_COUNT), (-2000.0_f32).to_bits());
}

#[test]
fn all_below_clamp_is_flat_even_below_negative_sentinel() {
    for value in [-2000.0_f32, -20000.0, -20001.0, -f32::MAX] {
        let (v9, v8) = samples(value);
        for enabled in [false, true] {
            let section = pack(&v9, &v8, enabled);
            assert_eq!(section.encoding(), Encoding::Flat);
            assert_eq!(section.min_height_bits(), (-2000.0_f32).to_bits());
            assert_eq!(section.max_height_bits(), (-2000.0_f32).to_bits());
        }
    }
}

#[test]
fn clamp_boundary_uses_comparison_without_rounding() {
    let (mut v9, v8) = samples(-2000.0);
    v9[0] = f32::from_bits((-2000.0_f32).to_bits() - 1).to_bits();
    v9[1] = f32::from_bits((-2000.0_f32).to_bits() + 1).to_bits();
    let section = pack(&v9, &v8, false);
    assert_eq!(payload_word(&section, 0), v9[0]);
    assert_eq!(payload_word(&section, 1), (-2000.0_f32).to_bits());
}

#[test]
fn positive_extrema_sentinel_is_not_replaced_by_infinity_or_first_value() {
    let (v9, v8) = samples(20001.0);
    let section = pack(&v9, &v8, true);
    assert_eq!(section.encoding(), Encoding::Uint8);
    assert_eq!(section.min_height_bits(), 20000.0_f32.to_bits());
    assert_eq!(section.max_height_bits(), 20001.0_f32.to_bits());
    assert!(
        section.bytes()[HEADER_BYTES..]
            .iter()
            .all(|&value| value == 255)
    );
    let (v9, v8) = samples(25000.0);
    let section = pack(&v9, &v8, true);
    assert_eq!(section.encoding(), Encoding::Float32);
    assert_eq!(section.min_height_bits(), 20000.0_f32.to_bits());
}

#[test]
fn finite_extrema_remain_supported_in_float_output() {
    let (mut v9, mut v8) = samples(0.0);
    v9[0] = f32::MAX.to_bits();
    v8[0] = (-f32::MAX).to_bits();
    let section = pack(&v9, &v8, true);
    assert_eq!(section.encoding(), Encoding::Float32);
    assert_eq!(payload_word(&section, 0), f32::MAX.to_bits());
    assert_eq!(payload_word(&section, V9_COUNT), (-2000.0_f32).to_bits());
    assert_eq!(
        section.bytes().len(),
        MAX_SECTION_BYTES - FLIGHT_BOUNDS_BYTES
    );
}

#[test]
fn extrema_visit_every_sample_including_final_v9_and_v8() {
    let (mut v9, mut v8) = samples(0.0);
    v9[V9_COUNT - 1] = 23.0_f32.to_bits();
    v8[V8_COUNT - 1] = (-23.0_f32).to_bits();
    let section = pack(&v9, &v8, true);
    assert_eq!(section.min_height_bits(), (-23.0_f32).to_bits());
    assert_eq!(section.max_height_bits(), 23.0_f32.to_bits());
}

#[test]
fn nonfinite_samples_are_rejected_before_flat_or_clamp_shortcuts() {
    for bits in [
        f32::INFINITY.to_bits(),
        f32::NEG_INFINITY.to_bits(),
        0x7fc12345,
        0xff800001,
    ] {
        for center in [false, true] {
            let (mut v9, mut v8) = samples(-3000.0);
            if center {
                v8[V8_COUNT - 1] = bits;
            } else {
                v9[V9_COUNT - 1] = bits;
            }
            let result =
                HeightSection::from_bits(&v9, &v8, Options::default(), None, MAX_SECTION_BYTES);
            assert_eq!(result.unwrap_err(), Error::NonFiniteHeight);
        }
    }
}

#[test]
fn exact_sample_dimensions_are_required() {
    for (v9_count, v8_count) in [
        (0, 0),
        (V9_COUNT - 1, V8_COUNT),
        (V9_COUNT + 1, V8_COUNT),
        (V9_COUNT, V8_COUNT - 1),
        (V9_COUNT, V8_COUNT + 1),
    ] {
        let result = HeightSection::from_bits(
            &vec![0; v9_count],
            &vec![0; v8_count],
            Options::default(),
            None,
            MAX_SECTION_BYTES,
        );
        assert_eq!(result.unwrap_err(), Error::InvalidSampleCount);
    }
}

fn bounds() -> FlightBounds {
    FlightBounds {
        max: [i16::MIN, -256, -1, 0, 1, 255, 256, 8192, i16::MAX],
        min: [1, 2, 3, 4, 5, 6, 7, 8, 9],
    }
}

#[test]
fn flight_payload_requires_exact_size_and_signed_little_endian_order() {
    let flight = bounds();
    let payload = flight
        .max
        .iter()
        .chain(&flight.min)
        .flat_map(|v| v.to_le_bytes())
        .collect::<Vec<_>>();
    assert_eq!(&payload[..6], &[0, 128, 0, 255, 255, 255]);
    assert_eq!(FlightBounds::parse(&payload).unwrap(), flight);
    for length in [0, 18, 35, 37, 44] {
        assert_eq!(
            FlightBounds::parse(&vec![0; length]).unwrap_err(),
            Error::InvalidFlightBounds
        );
    }
}

#[test]
fn flight_bounds_are_appended_after_all_four_encoding_variants() {
    let flight = bounds();
    let payload = flight
        .max
        .iter()
        .chain(&flight.min)
        .flat_map(|v| v.to_le_bytes())
        .collect::<Vec<_>>();
    for difference in [0.0_f32, 1.0, 2.0, 2048.0] {
        let (mut v9, v8) = samples(0.0);
        v9[0] = difference.to_bits();
        let plain = pack(&v9, &v8, true);
        let section = HeightSection::from_bits(
            &v9,
            &v8,
            Options::default(),
            Some(&flight),
            MAX_SECTION_BYTES,
        )
        .unwrap();
        assert_eq!(section.encoding(), plain.encoding());
        assert_eq!(section.flags(), plain.flags() | HAS_FLIGHT_BOUNDS);
        assert_eq!(
            section.bytes().len(),
            plain.bytes().len() + FLIGHT_BOUNDS_BYTES
        );
        assert_eq!(
            &section.bytes()[HEADER_BYTES..plain.bytes().len()],
            &plain.bytes()[HEADER_BYTES..]
        );
        assert_eq!(&section.bytes()[plain.bytes().len()..], payload);
    }
}

#[test]
fn output_limit_is_inclusive_for_each_encoding_and_flight_tail() {
    let flight = bounds();
    for difference in [0.0_f32, 1.0, 2.0, 2048.0] {
        for flight in [None, Some(&flight)] {
            let (mut v9, v8) = samples(0.0);
            v9[0] = difference.to_bits();
            let section =
                HeightSection::from_bits(&v9, &v8, Options::default(), flight, MAX_SECTION_BYTES)
                    .unwrap();
            let size = section.bytes().len();
            assert!(HeightSection::from_bits(&v9, &v8, Options::default(), flight, size).is_ok());
            assert_eq!(
                HeightSection::from_bits(&v9, &v8, Options::default(), flight, size - 1)
                    .unwrap_err(),
                Error::Limit
            );
        }
    }
}

#[test]
fn input_arrays_are_unchanged_after_clamp_and_rejection() {
    let (v9, v8) = samples(-f32::MAX);
    let before = (v9.clone(), v8.clone());
    let _ = pack(&v9, &v8, true);
    assert_eq!((&v9, &v8), (&before.0, &before.1));
    assert_eq!(
        HeightSection::from_bits(&v9, &v8, Options::default(), None, 0).unwrap_err(),
        Error::Limit
    );
    assert_eq!((&v9, &v8), (&before.0, &before.1));
}

#[test]
fn checked_projection_can_be_serialized_without_full_tile_claim() {
    let bytes = [b'R', b'E', b'V', b'M', 4, 0, 0, 0, 18, 0, 0, 0];
    let adt = crate::Adt::parse(&bytes, crate::Limits::default()).unwrap();
    let projection = adt.project_heights().unwrap();
    assert_eq!(projection.unique_cells(), 0);
    let section =
        HeightSection::from_projection(&projection, Options::default(), None, HEADER_BYTES)
            .unwrap();
    assert_eq!(section.encoding(), Encoding::Flat);
    assert_eq!(section.min_height_bits(), 0);
    assert_eq!(section.max_height_bits(), 0);
}

#[test]
fn error_codes_are_stable() {
    for (error, code) in [
        (Error::InvalidSampleCount, "height_invalid_sample_count"),
        (Error::NonFiniteHeight, "height_nonfinite_sample"),
        (Error::InvalidFlightBounds, "height_invalid_flight_bounds"),
        (
            Error::QuantizationOutOfRange,
            "height_quantization_out_of_range",
        ),
        (Error::Limit, "height_policy_limit"),
    ] {
        assert_eq!(error.code(), code);
        assert_eq!(error.to_string(), code);
    }
}
