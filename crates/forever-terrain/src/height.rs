// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original bounded MHGT section packing and serialization.
// Format facts: TrinityCore 02245dcd245e7433e524577656177723d3e4992e,
// System.cpp:103-112,208-213,253-255,716-722,760-864,987-1012;
// MapDefines.h height flags/header and MapDefines.cpp:23. No upstream code imported.
//! Pure serialization of one height section, including optional flight bounds.
//!
//! This does not produce a complete MAPS file. The caller supplies all V9 and
//! V8 samples in source [y][x] order and explicitly supplies or omits flight
//! bounds. Finite samples are required. The fixed -2000 minimum follows the
//! pinned executable: its `-h` switch exits through Usage, while `-f` controls
//! integer packing. Areas, holes, liquids and flight-plane construction are
//! outside this module.

use crate::{HeightProjection, adt::GRID_SIZE};

pub const V9_COUNT: usize = (GRID_SIZE + 1) * (GRID_SIZE + 1);
pub const V8_COUNT: usize = GRID_SIZE * GRID_SIZE;
pub const HEADER_BYTES: usize = 16;
pub const FLIGHT_BOUNDS_BYTES: usize = 36;
pub const MAX_SECTION_BYTES: usize = HEADER_BYTES + (V9_COUNT + V8_COUNT) * 4 + FLIGHT_BOUNDS_BYTES;
pub const NO_HEIGHT: u32 = 0x1;
pub const HEIGHT_AS_INT16: u32 = 0x2;
pub const HEIGHT_AS_INT8: u32 = 0x4;
pub const HAS_FLIGHT_BOUNDS: u32 = 0x8;
pub const MIN_HEIGHT: f32 = -2000.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Options {
    /// Matches the effective boolean selected by the reference's `-f` switch.
    pub pack_integers: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            pack_integers: true,
        }
    }
}

/// Row-major MFBO values. Their ordering and values are preserved, including
/// max < min pairs; this section writer does not construct or validate planes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FlightBounds {
    pub max: [i16; 9],
    pub min: [i16; 9],
}

impl FlightBounds {
    /// Parses the payload only, without its eight-byte ADT chunk frame.
    pub fn parse(payload: &[u8]) -> Result<Self, Error> {
        if payload.len() != FLIGHT_BOUNDS_BYTES {
            return Err(Error::InvalidFlightBounds);
        }
        let mut values = [0; 18];
        for (value, bytes) in values.iter_mut().zip(payload.as_chunks::<2>().0) {
            *value = i16::from_le_bytes(*bytes);
        }
        let mut max = [0; 9];
        let mut min = [0; 9];
        max.copy_from_slice(&values[..9]);
        min.copy_from_slice(&values[9..]);
        Ok(Self { max, min })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Encoding {
    Flat,
    Uint8,
    Uint16,
    Float32,
}

impl Encoding {
    const fn width(self) -> usize {
        match self {
            Self::Flat => 0,
            Self::Uint8 => 1,
            Self::Uint16 => 2,
            Self::Float32 => 4,
        }
    }

    const fn flag(self) -> u32 {
        match self {
            Self::Flat => NO_HEIGHT,
            Self::Uint8 => HEIGHT_AS_INT8,
            Self::Uint16 => HEIGHT_AS_INT16,
            Self::Float32 => 0,
        }
    }
}

#[derive(Debug)]
pub struct HeightSection {
    bytes: Vec<u8>,
    encoding: Encoding,
    flags: u32,
    min_height_bits: u32,
    max_height_bits: u32,
}

impl HeightSection {
    pub fn from_projection(
        projection: &HeightProjection,
        options: Options,
        flight_bounds: Option<&FlightBounds>,
        max_bytes: usize,
    ) -> Result<Self, Error> {
        Self::from_bits(
            projection.v9_bits(),
            projection.v8_bits(),
            options,
            flight_bounds,
            max_bytes,
        )
    }

    /// The fixed sample counts are validated before scanning or reserving.
    /// No input is modified and no partial result escapes on error.
    pub fn from_bits(
        v9_bits: &[u32],
        v8_bits: &[u32],
        options: Options,
        flight_bounds: Option<&FlightBounds>,
        max_bytes: usize,
    ) -> Result<Self, Error> {
        if v9_bits.len() != V9_COUNT || v8_bits.len() != V8_COUNT {
            return Err(Error::InvalidSampleCount);
        }
        // Source sentinels and visitation order are observable, including
        // signed zero and heights entirely above +20000. Do not use f32::min.
        let mut minimum = 20000.0_f32;
        let mut maximum = -20000.0_f32;
        for &bits in v8_bits.iter().chain(v9_bits) {
            let height = f32::from_bits(bits);
            if !height.is_finite() {
                return Err(Error::NonFiniteHeight);
            }
            if height < minimum {
                minimum = height;
            }
            if height > maximum {
                maximum = height;
            }
        }
        minimum = clamped(minimum);
        maximum = clamped(maximum);
        let difference = maximum - minimum;
        let encoding = if minimum == maximum || (options.pack_integers && difference < 0.005) {
            Encoding::Flat
        } else if options.pack_integers && difference < 2.0 {
            Encoding::Uint8
        } else if options.pack_integers && difference < 2048.0 {
            Encoding::Uint16
        } else {
            Encoding::Float32
        };
        let size = HEADER_BYTES
            + (V9_COUNT + V8_COUNT) * encoding.width()
            + usize::from(flight_bounds.is_some()) * FLIGHT_BOUNDS_BYTES;
        if size > max_bytes {
            return Err(Error::Limit);
        }
        let mut bytes = Vec::new();
        bytes.try_reserve_exact(size).map_err(|_| Error::Limit)?;
        let flags = encoding.flag()
            | if flight_bounds.is_some() {
                HAS_FLIGHT_BOUNDS
            } else {
                0
            };
        bytes.extend_from_slice(b"MHGT");
        bytes.extend_from_slice(&flags.to_le_bytes());
        bytes.extend_from_slice(&minimum.to_le_bytes());
        bytes.extend_from_slice(&maximum.to_le_bytes());

        // Serialization order differs from the extrema scan: V9 then V8.
        let samples = v9_bits
            .iter()
            .chain(v8_bits)
            .map(|&bits| clamped(f32::from_bits(bits)));
        match encoding {
            Encoding::Flat => {}
            Encoding::Uint8 | Encoding::Uint16 => {
                let step = match encoding {
                    Encoding::Uint8 => 255.0_f32 / difference,
                    _ => 65535.0_f32 / difference,
                };
                for height in samples {
                    // Keep each f32 operation separate. In particular, this
                    // is not a fused multiply-add or an f64 calculation.
                    let scaled = (height - minimum) * step;
                    let rounded = scaled + 0.5_f32;
                    let upper = if encoding == Encoding::Uint8 {
                        256.0
                    } else {
                        65536.0
                    };
                    if !rounded.is_finite() || !(0.0..upper).contains(&rounded) {
                        return Err(Error::QuantizationOutOfRange);
                    }
                    if encoding == Encoding::Uint8 {
                        bytes.push(rounded as u8);
                    } else {
                        bytes.extend_from_slice(&(rounded as u16).to_le_bytes());
                    }
                }
            }
            Encoding::Float32 => {
                for height in samples {
                    bytes.extend_from_slice(&height.to_le_bytes());
                }
            }
        }
        if let Some(bounds) = flight_bounds {
            for value in bounds.max.iter().chain(&bounds.min) {
                bytes.extend_from_slice(&value.to_le_bytes());
            }
        }
        Ok(Self {
            bytes,
            encoding,
            flags,
            min_height_bits: minimum.to_bits(),
            max_height_bits: maximum.to_bits(),
        })
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    pub const fn encoding(&self) -> Encoding {
        self.encoding
    }

    pub const fn flags(&self) -> u32 {
        self.flags
    }

    pub const fn min_height_bits(&self) -> u32 {
        self.min_height_bits
    }

    pub const fn max_height_bits(&self) -> u32 {
        self.max_height_bits
    }
}

fn clamped(value: f32) -> f32 {
    if value < MIN_HEIGHT {
        MIN_HEIGHT
    } else {
        value
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidSampleCount,
    NonFiniteHeight,
    InvalidFlightBounds,
    QuantizationOutOfRange,
    Limit,
}

impl Error {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidSampleCount => "height_invalid_sample_count",
            Self::NonFiniteHeight => "height_nonfinite_sample",
            Self::InvalidFlightBounds => "height_invalid_flight_bounds",
            Self::QuantizationOutOfRange => "height_quantization_out_of_range",
            Self::Limit => "height_policy_limit",
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for Error {}

#[cfg(test)]
mod tests;
