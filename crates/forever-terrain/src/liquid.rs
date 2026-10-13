// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original bounded liquid views and prepacking projection.
//! Pinned facts: map_extractor/adt.h:60-81,148-296 and
//! System.cpp:430-446,577-630,647-713,1056-1080 at crate::ORACLE_COMMIT.
//! No upstream implementation is imported. No DB2 acquisition, MLIQ packing,
//! terrain-height masking, I/O or runtime acceptance occurs here.

use crate::{
    Adt,
    adt::{CELL_COUNT, CELL_SIZE, CELLS_PER_AXIS, GRID_SIZE},
};

pub const MCLQ_PREFIX_BYTES: usize = 804;
pub const MH2O_HEADER_BYTES: usize = CELL_COUNT * 12;
pub const INSTANCE_BYTES: usize = 24;
pub const SHOW_COUNT: usize = GRID_SIZE * GRID_SIZE;
pub const HEIGHT_COUNT: usize = (GRID_SIZE + 1) * (GRID_SIZE + 1);
pub const WATER: u8 = 0x01;
pub const OCEAN: u8 = 0x02;
pub const MAGMA: u8 = 0x04;
pub const SLIME: u8 = 0x08;
pub const DARK_WATER: u8 = 0x10;
pub const MATERIAL_VISUAL_ONLY: u32 = 0x02;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Limit,
    Bounds,
    DuplicateChunk,
    InvalidRectangle,
    UnsupportedVertexFormat,
    NonFiniteHeight,
}
impl Error {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Limit => "liquid_policy_limit",
            Self::Bounds => "liquid_bounds",
            Self::DuplicateChunk => "liquid_duplicate_chunk",
            Self::InvalidRectangle => "liquid_invalid_rectangle",
            Self::UnsupportedVertexFormat => "liquid_unsupported_vertex_format",
            Self::NonFiniteHeight => "liquid_nonfinite_height",
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        out.write_str(self.code())
    }
}
impl std::error::Error for Error {}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LiquidType {
    pub sound_bank: u8,
    pub material_id: u8,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LiquidMaterial {
    /// Raw signed32 Flags bits; only VisualOnly is interpreted.
    pub flags: u32,
    /// Source stores GetUInt8("LVF") in int8; preserve that reinterpretation.
    pub vertex_format: i8,
}
/// A deterministic, already validated lookup. No DB2 loading or copy semantics
/// are inferred by this module. LiquidObject is not consulted by the pin.
pub trait LiquidLookup {
    fn liquid_type(&self, id: u32) -> Option<LiquidType>;
    fn material(&self, id: u32) -> Option<LiquidMaterial>;
}

/// Checked fixed MCLQ prefix; trailing declared bytes remain opaque. Views
/// borrow their input and deliberately do not implement Debug for raw data.
#[derive(Clone, Copy)]
pub struct Mclq<'a> {
    raw: &'a [u8],
}
impl<'a> Mclq<'a> {
    pub fn parse(payload: &'a [u8]) -> Result<Self, Error> {
        span(payload, 0, MCLQ_PREFIX_BYTES)?;
        Ok(Self { raw: payload })
    }
    pub const fn raw(self) -> &'a [u8] {
        self.raw
    }
    pub fn level_bits(self) -> [u32; 2] {
        [word(self.raw, 0), word(self.raw, 4)]
    }
    pub fn height_bits(self, x: usize, y: usize) -> Option<u32> {
        if x > CELL_SIZE || y > CELL_SIZE {
            return None;
        }
        Some(word(self.raw, 12 + (y * 9 + x) * 8))
    }
    pub fn light(self, x: usize, y: usize) -> Option<u32> {
        if x > CELL_SIZE || y > CELL_SIZE {
            return None;
        }
        Some(word(self.raw, 8 + (y * 9 + x) * 8))
    }
    pub fn tile_flags(self, x: usize, y: usize) -> Option<u8> {
        if x >= CELL_SIZE || y >= CELL_SIZE {
            return None;
        }
        Some(self.raw[656 + y * 8 + x])
    }
    /// Includes the declared 84 opaque bytes and any complete extra payload.
    pub fn opaque_tail(self) -> &'a [u8] {
        &self.raw[720..]
    }
}

/// Only the fixed index is checked at construction. Referenced instances,
/// attributes, bitmaps and height data are checked on access. This preserves
/// the source's inactive-cell and post-filter behavior without unsafe reads.
#[derive(Clone, Copy)]
pub struct Mh2o<'a> {
    raw: &'a [u8],
}
impl<'a> Mh2o<'a> {
    pub fn parse(payload: &'a [u8]) -> Result<Self, Error> {
        span(payload, 0, MH2O_HEADER_BYTES)?;
        Ok(Self { raw: payload })
    }
    pub const fn raw(self) -> &'a [u8] {
        self.raw
    }
    /// Coordinates follow conventional x=column, y=row order.
    pub fn cell(self, x: usize, y: usize) -> Option<LiquidCell<'a>> {
        if x >= CELLS_PER_AXIS || y >= CELLS_PER_AXIS {
            return None;
        }
        let offset = (y * CELLS_PER_AXIS + x) * 12;
        Some(LiquidCell {
            container: self,
            header: &self.raw[offset..offset + 12],
        })
    }
}
#[derive(Clone, Copy)]
pub struct LiquidCell<'a> {
    container: Mh2o<'a>,
    header: &'a [u8],
}
impl<'a> LiquidCell<'a> {
    pub const fn raw(self) -> &'a [u8] {
        self.header
    }
    pub fn instance_offset(self) -> u32 {
        word(self.header, 0)
    }
    pub fn declared_layers(self) -> u32 {
        word(self.header, 4)
    }
    pub fn attributes_offset(self) -> u32 {
        word(self.header, 8)
    }
    /// `used` is a presence test in this consumer. Further declared layers are
    /// neither allocated nor interpreted; this is not a multilayer decoder.
    pub fn first_instance(self) -> Result<Option<LiquidInstance<'a>>, Error> {
        if self.declared_layers() == 0 || self.instance_offset() == 0 {
            return Ok(None);
        }
        let raw = at(self.container.raw, self.instance_offset(), INSTANCE_BYTES)?;
        Ok(Some(LiquidInstance {
            container: self.container,
            raw,
        }))
    }
    pub fn attributes(self) -> Result<LiquidAttributes, Error> {
        if self.declared_layers() == 0 {
            return Ok(LiquidAttributes {
                fishable: 0,
                deep: 0,
            });
        }
        if self.attributes_offset() == 0 {
            return Ok(LiquidAttributes {
                fishable: u64::MAX,
                deep: u64::MAX,
            });
        }
        let raw = at(self.container.raw, self.attributes_offset(), 16)?;
        Ok(LiquidAttributes {
            fishable: long(raw, 0),
            deep: long(raw, 8),
        })
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LiquidAttributes {
    pub fishable: u64,
    pub deep: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rectangle {
    pub x: u8,
    pub y: u8,
    pub width: u8,
    pub height: u8,
}
#[derive(Clone, Copy)]
pub struct LiquidInstance<'a> {
    container: Mh2o<'a>,
    raw: &'a [u8],
}
impl<'a> LiquidInstance<'a> {
    pub const fn raw(self) -> &'a [u8] {
        self.raw
    }
    pub fn liquid_type(self) -> u16 {
        short(self.raw, 0)
    }
    pub fn vertex_format(self) -> u16 {
        short(self.raw, 2)
    }
    pub fn level_bits(self) -> [u32; 2] {
        [word(self.raw, 4), word(self.raw, 8)]
    }
    pub fn raw_rectangle(self) -> Rectangle {
        Rectangle {
            x: self.raw[12],
            y: self.raw[13],
            width: self.raw[14],
            height: self.raw[15],
        }
    }
    pub fn rectangle(self) -> Result<Rectangle, Error> {
        let r = if self.vertex_format() >= 42 {
            Rectangle {
                x: 0,
                y: 0,
                width: 8,
                height: 8,
            }
        } else {
            self.raw_rectangle()
        };
        if u16::from(r.x) + u16::from(r.width) > 8 || u16::from(r.y) + u16::from(r.height) > 8 {
            return Err(Error::InvalidRectangle);
        }
        Ok(r)
    }
    pub fn bitmap_offset(self) -> u32 {
        word(self.raw, 16)
    }
    pub fn vertex_offset(self) -> u32 {
        word(self.raw, 20)
    }
    pub fn exists_bitmap(self) -> Result<u64, Error> {
        if self.bitmap_offset() == 0 {
            return Ok(u64::MAX);
        }
        Ok(long(at(self.container.raw, self.bitmap_offset(), 8)?, 0))
    }
    fn heights(self, format: u16, count: usize) -> Result<Heights<'a>, Error> {
        if format > 5 {
            return Err(Error::UnsupportedVertexFormat);
        }
        if self.vertex_offset() == 0 || format == 2 {
            return Ok(Heights::Zero);
        }
        let stride = if format <= 3 { 4usize } else { 8 };
        let bytes = count.checked_mul(stride).ok_or(Error::Limit)?;
        let data = at(self.container.raw, self.vertex_offset(), bytes)?;
        Ok(Heights::Data { data, stride })
    }
}
enum Heights<'a> {
    Zero,
    Data { data: &'a [u8], stride: usize },
}
impl Heights<'_> {
    fn bits(&self, index: usize) -> Result<u32, Error> {
        let bits = match self {
            Self::Zero => 0,
            Self::Data { data, stride } => {
                word(data, index * stride + if *stride == 8 { 4 } else { 0 })
            }
        };
        finite(bits)
    }
}

/// Fixed dimensions, all [y][x] row-major. Show bytes are exactly zero or one.
/// No extrema, invisible-tile clamp, rectangle packing or liquid-level query.
pub struct LiquidProjection {
    show: Vec<u8>,
    entries: Vec<u16>,
    flags: Vec<u8>,
    height_bits: Vec<u32>,
}
impl LiquidProjection {
    pub fn show(&self) -> &[u8] {
        &self.show
    }
    pub fn entries(&self) -> &[u16] {
        &self.entries
    }
    pub fn flags(&self) -> &[u8] {
        &self.flags
    }
    pub fn height_bits(&self) -> &[u32] {
        &self.height_bits
    }
}

pub fn project_liquids(
    adt: &Adt<'_>,
    lookup: &impl LiquidLookup,
    ignore_deep_water: bool,
) -> Result<LiquidProjection, Error> {
    let mut out = LiquidProjection {
        show: zeros(SHOW_COUNT)?,
        entries: zeros(CELL_COUNT)?,
        flags: zeros(CELL_COUNT)?,
        height_bits: zeros(HEIGHT_COUNT)?,
    };
    // Physical MCNK order. Duplicate coordinates deliberately accumulate show
    // and flags, overwrite entries and heights, and share last-written edges.
    for cell in adt.cells() {
        let header = cell.header();
        if header.mclq_size() <= 8 {
            continue;
        }
        let Some(chunk) = cell
            .children()
            .unique(*b"MCLQ")
            .map_err(|_| Error::DuplicateChunk)?
        else {
            continue;
        };
        let liquid = Mclq::parse(chunk.payload())?;
        let (ix, iy) = (header.ix() as usize, header.iy() as usize);
        let cell_index = iy * CELLS_PER_AXIS + ix;
        for y in 0..CELL_SIZE {
            for x in 0..CELL_SIZE {
                let flags = liquid.raw[656 + y * 8 + x];
                if flags != 0x0f {
                    out.show[(iy * 8 + y) * GRID_SIZE + ix * 8 + x] = 1;
                    if !ignore_deep_water && flags & 0x80 != 0 {
                        out.flags[cell_index] |= DARK_WATER;
                    }
                }
            }
        }
        for (source_bit, id, flag) in [(4, 1, WATER), (8, 2, OCEAN), (16, 3, MAGMA)] {
            if header.flags() & source_bit != 0 {
                out.entries[cell_index] = id;
                out.flags[cell_index] |= flag;
            }
        }
        for y in 0..=CELL_SIZE {
            for x in 0..=CELL_SIZE {
                out.height_bits[(iy * 8 + y) * (GRID_SIZE + 1) + ix * 8 + x] =
                    finite(word(liquid.raw, 12 + (y * 9 + x) * 8))?;
            }
        }
    }
    if let Some(chunk) = adt
        .chunks()
        .unique(*b"MH2O")
        .map_err(|_| Error::DuplicateChunk)?
    {
        let mh2o = Mh2o::parse(chunk.payload())?;
        for iy in 0..CELLS_PER_AXIS {
            for ix in 0..CELLS_PER_AXIS {
                // Both coordinates are bounded by the constant loops.
                let cell = mh2o.cell(ix, iy).ok_or(Error::InvalidRectangle)?;
                let Some(instance) = cell.first_instance()? else {
                    continue;
                };
                let format = resolve_format(instance, lookup);
                let id = if format == 2 {
                    2
                } else {
                    instance.liquid_type()
                };
                let cell_index = iy * CELLS_PER_AXIS + ix;
                // This write intentionally precedes missing-type/VisualOnly
                // filters and never clears preceding MCLQ flags or heights.
                out.entries[cell_index] = id;
                let Some(kind) = lookup.liquid_type(u32::from(id)) else {
                    continue;
                };
                if lookup
                    .material(u32::from(kind.material_id))
                    .is_some_and(|material| material.flags & MATERIAL_VISUAL_ONLY != 0)
                {
                    continue;
                }
                let attrs = cell.attributes()?;
                let r = instance.rectangle()?;
                let mut mask = instance.exists_bitmap()?;
                let x0 = ix * 8 + usize::from(r.x);
                let y0 = iy * 8 + usize::from(r.y);
                for y in 0..usize::from(r.height) {
                    for x in 0..usize::from(r.width) {
                        if mask & 1 != 0 {
                            out.show[(y0 + y) * GRID_SIZE + x0 + x] = 1;
                        }
                        mask >>= 1;
                    }
                }
                out.flags[cell_index] |= match kind.sound_bank {
                    0 => WATER,
                    1 => {
                        OCEAN
                            | if !ignore_deep_water && attrs.deep != 0 {
                                DARK_WATER
                            } else {
                                0
                            }
                    }
                    2 => MAGMA,
                    3 => SLIME,
                    _ => 0,
                };
                let width = usize::from(r.width) + 1;
                let height = usize::from(r.height) + 1;
                let samples =
                    instance.heights(format, width.checked_mul(height).ok_or(Error::Limit)?)?;
                for y in 0..height {
                    for x in 0..width {
                        out.height_bits[(y0 + y) * (GRID_SIZE + 1) + x0 + x] =
                            samples.bits(y * width + x)?;
                    }
                }
            }
        }
    }
    Ok(out)
}
fn resolve_format(instance: LiquidInstance<'_>, lookup: &impl LiquidLookup) -> u16 {
    if instance.vertex_format() < 42 {
        return instance.vertex_format();
    }
    if instance.liquid_type() == 2 {
        return 2;
    }
    lookup
        .liquid_type(u32::from(instance.liquid_type()))
        .and_then(|kind| lookup.material(u32::from(kind.material_id)))
        .map_or(u16::MAX, |material| material.vertex_format as u16)
}

/// Matches the extraction caller's order: tile_y first, tile_x second.
/// The pinned IsDeepWaterIgnored formal `x` receives the row (System.cpp:1117).
pub const fn ignore_deep_water(map_id: u32, tile_y: u32, tile_x: u32) -> bool {
    match map_id {
        0 => {
            (tile_y >= 39 && tile_y <= 40 && tile_x >= 24 && tile_x <= 26)
                || (tile_y >= 41 && tile_y <= 46 && tile_x >= 18 && tile_x <= 26)
        }
        1 => tile_y == 43 && (tile_x == 39 || tile_x == 40),
        _ => false,
    }
}
fn finite(bits: u32) -> Result<u32, Error> {
    if f32::from_bits(bits).is_finite() {
        Ok(bits)
    } else {
        Err(Error::NonFiniteHeight)
    }
}
fn zeros<T: Default + Clone>(count: usize) -> Result<Vec<T>, Error> {
    let mut out = Vec::new();
    out.try_reserve_exact(count).map_err(|_| Error::Limit)?;
    out.resize(count, T::default());
    Ok(out)
}
fn at(bytes: &[u8], offset: u32, length: usize) -> Result<&[u8], Error> {
    span(
        bytes,
        usize::try_from(offset).map_err(|_| Error::Limit)?,
        length,
    )
}
fn span(bytes: &[u8], offset: usize, length: usize) -> Result<&[u8], Error> {
    let end = offset.checked_add(length).ok_or(Error::Bounds)?;
    bytes.get(offset..end).ok_or(Error::Bounds)
}
fn short(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}
fn word(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}
fn long(bytes: &[u8], offset: usize) -> u64 {
    u64::from(word(bytes, offset)) | (u64::from(word(bytes, offset + 4)) << 32)
}

#[cfg(test)]
mod tests;
