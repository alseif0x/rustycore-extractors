// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original AREA/MAPS assembly and bounded borrowed views.
// Pinned facts: TrinityCore 02245dcd245e7433e524577656177723d3e4992e,
// src/common/Collision/Maps/MapDefines.h:38-130, MapDefines.cpp:20-24;
// src/tools/map_extractor/System.cpp:725-757,868-955,958-1031.
// No upstream implementation imported; original authors retain their rights.
//! Pure MAPS v10 container assembly and strict structural verification.
//! Supported build tags are explicit, not evidence of client-data provenance.
//! There is no I/O, acquisition, coordinate discovery or runtime query here.
//! Verification is stronger than the reviewed C++/RustyCore reader admission;
//! it does not reconstruct packing decisions or authenticate historical input.

use crate::{
    HeightProjection,
    adt::CELL_COUNT,
    height::{self, Encoding, HeightSection},
    liquid_section::{self, LiquidSection},
};

pub const HEADER_BYTES: usize = 44;
pub const VERSION: u32 = 10;
/// Explicit assumption: build 70450 uses the same MAPS v10 wire layout as
/// 70374. This is not independent format evidence; exact build/version gates
/// remain mandatory and no conversion silently retags another build.
pub const BUILD_70450_FORMAT_ASSUMPTION: &str =
    "build 70450 MAPS output assumes the build 70374 MAPS v10 wire layout";
pub const AREA_HEADER_BYTES: usize = 8;
pub const AREA_FULL_BYTES: usize = AREA_HEADER_BYTES + CELL_COUNT * 2;
pub const HOLE_BYTES: usize = CELL_COUNT * 8;
pub const MAX_MAP_BYTES: usize = HEADER_BYTES
    + AREA_FULL_BYTES
    + height::MAX_SECTION_BYTES
    + liquid_section::MAX_SECTION_BYTES
    + HOLE_BYTES;
pub const NO_AREA: u16 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    UnsupportedBuild,
    BuildMismatch,
    Limit,
    Header,
    Layout,
    Area,
    Height,
    Liquid,
    Holes,
}
impl Error {
    pub const fn code(self) -> &'static str {
        match self {
            Self::UnsupportedBuild => "maps_unsupported_build",
            Self::BuildMismatch => "maps_build_mismatch",
            Self::Limit => "maps_policy_limit",
            Self::Header => "maps_invalid_header",
            Self::Layout => "maps_invalid_layout",
            Self::Area => "maps_invalid_area",
            Self::Height => "maps_invalid_height",
            Self::Liquid => "maps_invalid_liquid",
            Self::Holes => "maps_invalid_holes",
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        out.write_str(self.code())
    }
}
impl std::error::Error for Error {}

/// Canonical AREA bytes; fixed backing storage avoids allocation.
pub struct AreaSection {
    bytes: [u8; AREA_FULL_BYTES],
    len: usize,
}
impl AreaSection {
    pub fn from_ids(ids: &[u16; CELL_COUNT]) -> Self {
        let mut bytes = [0; AREA_FULL_BYTES];
        bytes[..4].copy_from_slice(b"AREA");
        let uniform = ids.iter().all(|&id| id == ids[0]);
        let len = if uniform {
            bytes[4..6].copy_from_slice(&NO_AREA.to_le_bytes());
            bytes[6..8].copy_from_slice(&ids[0].to_le_bytes());
            AREA_HEADER_BYTES
        } else {
            for (index, id) in ids.iter().enumerate() {
                bytes[8 + index * 2..10 + index * 2].copy_from_slice(&id.to_le_bytes());
            }
            AREA_FULL_BYTES
        };
        Self { bytes, len }
    }
    pub fn from_projection(projection: &HeightProjection) -> Self {
        Self::from_ids(projection.area_ids())
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes[..self.len]
    }
}

#[derive(Clone, Copy)]
pub struct MapSections<'a> {
    pub area: &'a AreaSection,
    pub height: &'a HeightSection,
    pub liquid: Option<&'a LiquidSection>,
    /// Presence is explicit: Some(all-zero) preserves the source sticky flag.
    pub holes: Option<&'a [u8; HOLE_BYTES]>,
}

pub fn assemble(build: u32, sections: MapSections<'_>, max_bytes: usize) -> Result<Vec<u8>, Error> {
    supported_build(build)?;
    assemble_impl(build, sections, max_bytes, None)
}

/// Explicit70374 declaration; legacy assemble/verify gates remain unchanged.
pub fn assemble_70374(
    build: u32,
    sections: MapSections<'_>,
    max_bytes: usize,
) -> Result<Vec<u8>, Error> {
    assemble_impl(build, sections, max_bytes, Some(70374))
}

/// Explicit build70450 MAPS v10 assembly under the documented format
/// assumption. The build tag must match exactly.
pub fn assemble_70450(
    build: u32,
    sections: MapSections<'_>,
    max_bytes: usize,
) -> Result<Vec<u8>, Error> {
    assemble_impl(build, sections, max_bytes, Some(70450))
}

fn assemble_impl(
    build: u32,
    sections: MapSections<'_>,
    max_bytes: usize,
    current_build: Option<u32>,
) -> Result<Vec<u8>, Error> {
    check_build(build, current_build)?;
    let pieces = [
        Some(sections.area.bytes()),
        Some(sections.height.bytes()),
        sections.liquid.map(LiquidSection::bytes),
        sections.holes.map(|holes| holes.as_slice()),
    ];
    let mut size = HEADER_BYTES;
    let mut pairs = [(0u32, 0u32); 4];
    for (pair, piece) in pairs.iter_mut().zip(pieces) {
        if let Some(bytes) = piece {
            *pair = (
                u32::try_from(size).map_err(|_| Error::Limit)?,
                u32::try_from(bytes.len()).map_err(|_| Error::Limit)?,
            );
            size = size.checked_add(bytes.len()).ok_or(Error::Limit)?;
        }
    }
    if size > max_bytes || size > MAX_MAP_BYTES {
        return Err(Error::Limit);
    }
    let mut out = Vec::new();
    out.try_reserve_exact(size).map_err(|_| Error::Limit)?;
    out.extend_from_slice(b"MAPS");
    out.extend_from_slice(&VERSION.to_le_bytes());
    out.extend_from_slice(&build.to_le_bytes());
    for (offset, length) in pairs {
        out.extend_from_slice(&offset.to_le_bytes());
        out.extend_from_slice(&length.to_le_bytes());
    }
    for bytes in pieces.into_iter().flatten() {
        out.extend_from_slice(bytes);
    }
    // Typed section constructors establish content invariants. Independently
    // check their assembled placement before returning any complete bytes.
    verify_impl(&out, build, max_bytes, current_build)?;
    Ok(out)
}

#[derive(Clone, Copy)]
pub struct MapView<'a> {
    raw: &'a [u8],
    build: u32,
    area: AreaView<'a>,
    height: HeightView<'a>,
    liquid: Option<LiquidView<'a>>,
    holes: Option<&'a [u8; HOLE_BYTES]>,
}
impl<'a> MapView<'a> {
    pub const fn bytes(self) -> &'a [u8] {
        self.raw
    }
    pub const fn build(self) -> u32 {
        self.build
    }
    pub const fn area(self) -> AreaView<'a> {
        self.area
    }
    pub const fn height(self) -> HeightView<'a> {
        self.height
    }
    pub const fn liquid(self) -> Option<LiquidView<'a>> {
        self.liquid
    }
    pub const fn holes(self) -> Option<&'a [u8; HOLE_BYTES]> {
        self.holes
    }
}

#[derive(Clone, Copy)]
pub struct AreaView<'a> {
    raw: &'a [u8],
}
impl<'a> AreaView<'a> {
    pub const fn bytes(self) -> &'a [u8] {
        self.raw
    }
    pub fn flags(self) -> u16 {
        short(self.raw, 4)
    }
    pub fn grid_area(self) -> u16 {
        short(self.raw, 6)
    }
    pub fn id(self, index: usize) -> Option<u16> {
        if index >= CELL_COUNT {
            return None;
        }
        Some(if self.flags() == NO_AREA {
            self.grid_area()
        } else {
            short(self.raw, AREA_HEADER_BYTES + index * 2)
        })
    }
}

#[derive(Clone, Copy)]
pub struct HeightView<'a> {
    raw: &'a [u8],
    encoding: Encoding,
    sample_width: usize,
}
impl<'a> HeightView<'a> {
    pub const fn bytes(self) -> &'a [u8] {
        self.raw
    }
    pub fn flags(self) -> u32 {
        word(self.raw, 4)
    }
    pub const fn encoding(self) -> Encoding {
        self.encoding
    }
    pub fn min_height_bits(self) -> u32 {
        word(self.raw, 8)
    }
    pub fn max_height_bits(self) -> u32 {
        word(self.raw, 12)
    }
    /// Encoded bytes, not reconstructed heights. Flat sections have no samples.
    pub fn v9_samples(self) -> &'a [u8] {
        &self.raw[16..16 + height::V9_COUNT * self.sample_width]
    }
    pub fn v8_samples(self) -> &'a [u8] {
        let start = 16 + height::V9_COUNT * self.sample_width;
        &self.raw[start..start + height::V8_COUNT * self.sample_width]
    }
    /// Nine maxima then nine minima, raw signed-i16 LE; no plane construction.
    pub fn flight_bounds(self) -> Option<&'a [u8; height::FLIGHT_BOUNDS_BYTES]> {
        if self.flags() & height::HAS_FLIGHT_BOUNDS == 0 {
            return None;
        }
        self.raw[self.raw.len() - height::FLIGHT_BOUNDS_BYTES..]
            .try_into()
            .ok()
    }
}

#[derive(Clone, Copy)]
pub struct LiquidView<'a> {
    raw: &'a [u8],
}
impl<'a> LiquidView<'a> {
    pub const fn bytes(self) -> &'a [u8] {
        self.raw
    }
    pub fn flags(self) -> u8 {
        self.raw[4]
    }
    pub fn liquid_flags(self) -> u8 {
        self.raw[5]
    }
    pub fn liquid_type(self) -> u16 {
        short(self.raw, 6)
    }
    pub fn offset_x(self) -> u8 {
        self.raw[8]
    }
    pub fn offset_y(self) -> u8 {
        self.raw[9]
    }
    pub fn width(self) -> u8 {
        self.raw[10]
    }
    pub fn height(self) -> u8 {
        self.raw[11]
    }
    pub fn level_bits(self) -> u32 {
        word(self.raw, 12)
    }
    /// Full row-major LE type array, absent when the header stores one type.
    pub fn entry_bytes(self) -> Option<&'a [u8]> {
        (self.flags() & liquid_section::NO_TYPE == 0).then(|| &self.raw[16..528])
    }
    pub fn flag_bytes(self) -> Option<&'a [u8]> {
        (self.flags() & liquid_section::NO_TYPE == 0).then(|| &self.raw[528..784])
    }
    pub fn height_bytes(self) -> &'a [u8] {
        let start = if self.flags() & liquid_section::NO_TYPE != 0 {
            16
        } else {
            784
        };
        &self.raw[start..]
    }
}

/// Checks one complete, contiguous MAPS v10 object without allocation. It
/// neither repairs data nor guesses a build/layout. The build tag is a checked
/// declaration, not authentication of the original client or source tables.
pub fn verify(bytes: &[u8], expected_build: u32, max_bytes: usize) -> Result<MapView<'_>, Error> {
    supported_build(expected_build)?;
    verify_impl(bytes, expected_build, max_bytes, None)
}

/// Separate70374 verifier; build tags remain declarations, not source proofs.
pub fn verify_70374(
    bytes: &[u8],
    expected_build: u32,
    max_bytes: usize,
) -> Result<MapView<'_>, Error> {
    verify_impl(bytes, expected_build, max_bytes, Some(70374))
}

/// Separate build70450 verifier for exact MAPS v10 output under the documented
/// format assumption; build and version mismatches fail closed.
pub fn verify_70450(
    bytes: &[u8],
    expected_build: u32,
    max_bytes: usize,
) -> Result<MapView<'_>, Error> {
    verify_impl(bytes, expected_build, max_bytes, Some(70450))
}

fn verify_impl(
    bytes: &[u8],
    expected_build: u32,
    max_bytes: usize,
    current_build: Option<u32>,
) -> Result<MapView<'_>, Error> {
    check_build(expected_build, current_build)?;
    if bytes.len() > max_bytes || bytes.len() > MAX_MAP_BYTES {
        return Err(Error::Limit);
    }
    if bytes.len() < HEADER_BYTES || &bytes[..4] != b"MAPS" || word(bytes, 4) != VERSION {
        return Err(Error::Header);
    }
    let build = word(bytes, 8);
    check_build(build, current_build)?;
    if build != expected_build {
        return Err(Error::BuildMismatch);
    }
    let mut cursor = HEADER_BYTES;
    let area = next(bytes, &mut cursor, 12, false)?.ok_or(Error::Layout)?;
    let height = next(bytes, &mut cursor, 20, false)?.ok_or(Error::Layout)?;
    let liquid = next(bytes, &mut cursor, 28, true)?;
    let holes = next(bytes, &mut cursor, 36, true)?;
    if cursor != bytes.len() {
        return Err(Error::Layout);
    }
    let area = verify_area(area)?;
    let height = verify_height(height)?;
    let liquid = liquid.map(verify_liquid).transpose()?;
    let holes = holes
        .map(|raw| raw.try_into().map_err(|_| Error::Holes))
        .transpose()?;
    Ok(MapView {
        raw: bytes,
        build,
        area,
        height,
        liquid,
        holes,
    })
}

fn next<'a>(
    bytes: &'a [u8],
    cursor: &mut usize,
    header_offset: usize,
    optional: bool,
) -> Result<Option<&'a [u8]>, Error> {
    let offset = usize::try_from(word(bytes, header_offset)).map_err(|_| Error::Layout)?;
    let size = usize::try_from(word(bytes, header_offset + 4)).map_err(|_| Error::Layout)?;
    if offset == 0 && size == 0 && optional {
        return Ok(None);
    }
    if offset != *cursor || size == 0 {
        return Err(Error::Layout);
    }
    let end = offset.checked_add(size).ok_or(Error::Layout)?;
    let raw = bytes.get(offset..end).ok_or(Error::Layout)?;
    *cursor = end;
    Ok(Some(raw))
}

fn verify_area(raw: &[u8]) -> Result<AreaView<'_>, Error> {
    if raw.len() < AREA_HEADER_BYTES || &raw[..4] != b"AREA" {
        return Err(Error::Area);
    }
    match short(raw, 4) {
        NO_AREA if raw.len() == AREA_HEADER_BYTES => {}
        0 if raw.len() == AREA_FULL_BYTES && short(raw, 6) == 0 => {
            let first = short(raw, 8);
            if (1..CELL_COUNT).all(|index| short(raw, 8 + index * 2) == first) {
                return Err(Error::Area);
            }
        }
        _ => return Err(Error::Area),
    }
    Ok(AreaView { raw })
}

fn verify_height(raw: &[u8]) -> Result<HeightView<'_>, Error> {
    if raw.len() < height::HEADER_BYTES || &raw[..4] != b"MHGT" {
        return Err(Error::Height);
    }
    let flags = word(raw, 4);
    let (encoding, width) = match flags & !height::HAS_FLIGHT_BOUNDS {
        0 => (Encoding::Float32, 4usize),
        height::NO_HEIGHT => (Encoding::Flat, 0),
        height::HEIGHT_AS_INT16 => (Encoding::Uint16, 2),
        height::HEIGHT_AS_INT8 => (Encoding::Uint8, 1),
        _ => return Err(Error::Height),
    };
    let sample_bytes = (height::V9_COUNT + height::V8_COUNT) * width;
    let flight_bytes = if flags & height::HAS_FLIGHT_BOUNDS != 0 {
        height::FLIGHT_BOUNDS_BYTES
    } else {
        0
    };
    if raw.len() != height::HEADER_BYTES + sample_bytes + flight_bytes {
        return Err(Error::Height);
    }
    let min = f32::from_bits(word(raw, 8));
    let max = f32::from_bits(word(raw, 12));
    if !min.is_finite() || !max.is_finite() || min > max {
        return Err(Error::Height);
    }
    if encoding == Encoding::Float32 && !finite_words(&raw[16..16 + sample_bytes]) {
        return Err(Error::Height);
    }
    Ok(HeightView {
        raw,
        encoding,
        sample_width: width,
    })
}

fn verify_liquid(raw: &[u8]) -> Result<LiquidView<'_>, Error> {
    if raw.len() < liquid_section::HEADER_BYTES || &raw[..4] != b"MLIQ" {
        return Err(Error::Liquid);
    }
    let flags = raw[4];
    if flags & !(liquid_section::NO_TYPE | liquid_section::NO_HEIGHT) != 0
        || !f32::from_bits(word(raw, 12)).is_finite()
    {
        return Err(Error::Liquid);
    }
    let no_type = flags & liquid_section::NO_TYPE != 0;
    let no_height = flags & liquid_section::NO_HEIGHT != 0;
    let (x, y, width, height) = (raw[8], raw[9], raw[10], raw[11]);
    let empty = (x, y, width, height) == (128, 128, 130, 130);
    let ordinary = x < 128
        && y < 128
        && width >= 2
        && height >= 2
        && u16::from(x) + u16::from(width) <= 129
        && u16::from(y) + u16::from(height) <= 129;
    if !(ordinary || empty && no_height) {
        return Err(Error::Liquid);
    }
    if empty && word(raw, 12) != liquid_section::INVISIBLE_HEIGHT.to_bits() {
        return Err(Error::Liquid);
    }
    let type_bytes = if no_type {
        0
    } else {
        liquid_section::TYPE_BYTES
    };
    let sample_bytes = if no_height {
        0
    } else {
        usize::from(width) * usize::from(height) * 4
    };
    if raw.len() != liquid_section::HEADER_BYTES + type_bytes + sample_bytes {
        return Err(Error::Liquid);
    }
    if no_type {
        // A uniform zero flag would have omitted this section entirely.
        if raw[5] == 0 {
            return Err(Error::Liquid);
        }
    } else {
        if raw[5] != 0 || short(raw, 6) != 0 {
            return Err(Error::Liquid);
        }
        let first_entry = short(raw, 16);
        let first_flag = raw[528];
        if (1..CELL_COUNT).all(|index| {
            short(raw, 16 + index * 2) == first_entry && raw[528 + index] == first_flag
        }) {
            return Err(Error::Liquid);
        }
    }
    if !finite_words(&raw[16 + type_bytes..]) {
        return Err(Error::Liquid);
    }
    Ok(LiquidView { raw })
}

fn check_build(build: u32, current_build: Option<u32>) -> Result<(), Error> {
    if let Some(expected) = current_build {
        if build == expected {
            return Ok(());
        }
        return Err(Error::UnsupportedBuild);
    }
    if matches!(build, 70170 | 70205) {
        Ok(())
    } else {
        Err(Error::UnsupportedBuild)
    }
}
fn supported_build(build: u32) -> Result<(), Error> {
    if matches!(build, 70170 | 70205) {
        Ok(())
    } else {
        Err(Error::UnsupportedBuild)
    }
}
fn finite_words(raw: &[u8]) -> bool {
    raw.as_chunks::<4>()
        .0
        .iter()
        .all(|bits| f32::from_le_bytes(*bits).is_finite())
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

#[cfg(test)]
mod tests;
