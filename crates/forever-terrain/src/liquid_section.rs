// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original bounded MLIQ section serialization.
// Pinned facts: TrinityCore 02245dcd245e7433e524577656177723d3e4992e,
// src/tools/map_extractor/System.cpp:103-112,208-213,253-255,868-955,1014-1026;
// src/common/Collision/Maps/MapDefines.h:87-130 and MapDefines.cpp:24.
// No upstream implementation imported; original authors retain their rights.
//! Pure serialization of a projected liquid section, or explicit omission.
//! No ADT/DB2 loading, file I/O, MAPS container or runtime query occurs here.
//! All inputs are checked before omission or packing; input is never modified.

use crate::{
    adt::{CELL_COUNT, GRID_SIZE},
    liquid::{HEIGHT_COUNT, LiquidProjection, SHOW_COUNT},
};

pub const HEADER_BYTES: usize = 16;
pub const TYPE_BYTES: usize = CELL_COUNT * 3;
pub const MAX_SECTION_BYTES: usize = HEADER_BYTES + TYPE_BYTES + HEIGHT_COUNT * 4;
pub const NO_TYPE: u8 = 1;
pub const NO_HEIGHT: u8 = 2;
pub const INVISIBLE_HEIGHT: f32 = -2000.0;
pub const FLAT_DELTA_LIMIT: f32 = 0.001;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Options {
    /// The pinned executable's effective `-f` boolean. Liquid heights never
    /// become integers; this option only permits the near-flat omission rule.
    pub pack_integers: bool,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            pack_integers: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidShape,
    InvalidVisibility,
    NonFiniteHeight,
    Limit,
}
impl Error {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidShape => "liquid_section_invalid_shape",
            Self::InvalidVisibility => "liquid_section_invalid_visibility",
            Self::NonFiniteHeight => "liquid_section_nonfinite_height",
            Self::Limit => "liquid_section_policy_limit",
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        out.write_str(self.code())
    }
}
impl std::error::Error for Error {}

/// An immutable complete MLIQ section. Absence is represented by `None`,
/// never by an empty section or a fabricated zero header.
pub struct LiquidSection {
    bytes: Vec<u8>,
    flags: u8,
    liquid_flags: u8,
    liquid_type: u16,
    offset_x: u8,
    offset_y: u8,
    width: u8,
    height: u8,
    level_bits: u32,
}
impl LiquidSection {
    pub fn from_projection(
        projection: &LiquidProjection,
        options: Options,
        max_bytes: usize,
    ) -> Result<Option<Self>, Error> {
        Self::from_arrays(
            projection.show(),
            projection.entries(),
            projection.flags(),
            projection.height_bits(),
            options,
            max_bytes,
        )
    }

    /// All arrays use source [y][x] row-major order. The 256 liquid flag bytes
    /// are preserved opaquely. Visibility must be exactly zero or one and all
    /// 16,641 heights finite, including heights an omitted section would hide.
    pub fn from_arrays(
        show: &[u8],
        entries: &[u16],
        liquid_flags: &[u8],
        height_bits: &[u32],
        options: Options,
        max_bytes: usize,
    ) -> Result<Option<Self>, Error> {
        if show.len() != SHOW_COUNT
            || entries.len() != CELL_COUNT
            || liquid_flags.len() != CELL_COUNT
            || height_bits.len() != HEIGHT_COUNT
        {
            return Err(Error::InvalidShape);
        }
        if show.iter().any(|&value| value > 1) {
            return Err(Error::InvalidVisibility);
        }
        if height_bits
            .iter()
            .any(|&bits| !f32::from_bits(bits).is_finite())
        {
            return Err(Error::NonFiniteHeight);
        }
        let first_type = entries[0];
        let first_flag = liquid_flags[0];
        let full_type = entries
            .iter()
            .zip(liquid_flags)
            .any(|(&entry, &flag)| entry != first_type || flag != first_flag);
        // Uniform NoWater omits the section even for a nonzero type ID or
        // visible tiles. Visibility does not participate in this source gate.
        if !full_type && first_flag == 0 {
            return Ok(None);
        }

        let (mut min_x, mut min_y) = (GRID_SIZE, GRID_SIZE);
        let (mut max_x, mut max_y) = (0, 0);
        let mut minimum = 20000.0_f32;
        let mut maximum = -20000.0_f32;
        // Only the 128x128 top-left samples affect extrema. Invisible ones
        // become -2000 for the emitted crop; the outer 129th border is intact.
        for y in 0..GRID_SIZE {
            for x in 0..GRID_SIZE {
                if show[y * GRID_SIZE + x] != 0 {
                    min_x = min_x.min(x);
                    min_y = min_y.min(y);
                    max_x = max_x.max(x);
                    max_y = max_y.max(y);
                    let height = f32::from_bits(height_bits[y * (GRID_SIZE + 1) + x]);
                    if height > maximum {
                        maximum = height;
                    }
                    if height < minimum {
                        minimum = height;
                    }
                } else if minimum > INVISIBLE_HEIGHT {
                    minimum = INVISIBLE_HEIGHT;
                }
            }
        }
        let empty = min_x > max_x || min_y > max_y;
        let no_height = empty
            || maximum == minimum
            || (options.pack_integers && maximum - minimum < FLAT_DELTA_LIMIT);
        let flags = if full_type { 0 } else { NO_TYPE } | if no_height { NO_HEIGHT } else { 0 };
        let (header_flag, header_type) = if full_type {
            (0, 0)
        } else {
            (first_flag, first_type)
        };
        // Preserve the source's int-to-uint8 conversion for an empty crop:
        // min=128, max=0, extent=-126 -> 130. NoHeight prevents indexing it.
        let width = (max_x as i32 - min_x as i32 + 2) as u8;
        let height = (max_y as i32 - min_y as i32 + 2) as u8;
        let sample_bytes = if no_height {
            0
        } else {
            usize::from(width)
                .checked_mul(usize::from(height))
                .and_then(|count| count.checked_mul(4))
                .ok_or(Error::Limit)?
        };
        let size = HEADER_BYTES
            .checked_add(if full_type { TYPE_BYTES } else { 0 })
            .and_then(|size| size.checked_add(sample_bytes))
            .ok_or(Error::Limit)?;
        if size > max_bytes || size > MAX_SECTION_BYTES {
            return Err(Error::Limit);
        }
        let mut bytes = Vec::new();
        bytes.try_reserve_exact(size).map_err(|_| Error::Limit)?;
        bytes.extend_from_slice(b"MLIQ");
        bytes.extend_from_slice(&[flags, header_flag]);
        bytes.extend_from_slice(&header_type.to_le_bytes());
        bytes.extend_from_slice(&[min_x as u8, min_y as u8, width, height]);
        bytes.extend_from_slice(&minimum.to_le_bytes());
        if full_type {
            for entry in entries {
                bytes.extend_from_slice(&entry.to_le_bytes());
            }
            bytes.extend_from_slice(liquid_flags);
        }
        if !no_height {
            for y in min_y..=max_y + 1 {
                for x in min_x..=max_x + 1 {
                    let bits = if x < GRID_SIZE && y < GRID_SIZE && show[y * GRID_SIZE + x] == 0 {
                        INVISIBLE_HEIGHT.to_bits()
                    } else {
                        height_bits[y * (GRID_SIZE + 1) + x]
                    };
                    bytes.extend_from_slice(&bits.to_le_bytes());
                }
            }
        }
        Ok(Some(Self {
            bytes,
            flags,
            liquid_flags: header_flag,
            liquid_type: header_type,
            offset_x: min_x as u8,
            offset_y: min_y as u8,
            width,
            height,
            level_bits: minimum.to_bits(),
        }))
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
    pub const fn flags(&self) -> u8 {
        self.flags
    }
    pub const fn liquid_flags(&self) -> u8 {
        self.liquid_flags
    }
    pub const fn liquid_type(&self) -> u16 {
        self.liquid_type
    }
    pub const fn offset_x(&self) -> u8 {
        self.offset_x
    }
    pub const fn offset_y(&self) -> u8 {
        self.offset_y
    }
    pub const fn width(&self) -> u8 {
        self.width
    }
    pub const fn height(&self) -> u8 {
        self.height
    }
    pub const fn level_bits(&self) -> u32 {
        self.level_bits
    }
}

#[cfg(test)]
mod tests;
