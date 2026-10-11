// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original checked ADT metadata and height projection.
//! Source facts at ORACLE_COMMIT: map_extractor/adt.h:47-54,87-132 and
//! System.cpp:449-461,482-575,632-644. No upstream implementation imported.
//!
//! Admitted MCNK shape: 128-byte metadata prefix, then consecutive complete
//! chunk frames. MCNR is opaque; no undeclared 13-byte padding or arbitrary
//! gaps are skipped. The pin's byte scanner does not justify a general gap
//! grammar. Unsupported nested framing fails without returning prior cells.
//!
//! Metadata retains every float bit. The separate prepacking projection
//! admits only finite base/MCVT inputs and finite f32 sums. No clamp, extrema,
//! quantization, liquid or MAPS output is produced.

use crate::{Chunks, Error, Limits, wdt::FILE_VERSION};

pub const CELLS_PER_AXIS: usize = 16;
pub const CELL_COUNT: usize = CELLS_PER_AXIS * CELLS_PER_AXIS;
pub const CELL_SIZE: usize = 8;
pub const GRID_SIZE: usize = CELLS_PER_AXIS * CELL_SIZE;
pub const MCNK_PREFIX_BYTES: usize = 128;
pub const MCVT_VALUES: usize = 145;
pub const HIGH_RESOLUTION_HOLES: u32 = 0x10000;

/// Every prefix byte is retained. The accessors interpret only reviewed fields.
#[derive(Clone, Copy)]
pub struct McnkHeader<'a> {
    raw: &'a [u8],
}

impl<'a> McnkHeader<'a> {
    pub const fn raw(self) -> &'a [u8] {
        self.raw
    }
    pub fn flags(self) -> u32 {
        word(self.raw, 0)
    }
    pub fn ix(self) -> u32 {
        word(self.raw, 4)
    }
    pub fn iy(self) -> u32 {
        word(self.raw, 8)
    }
    pub fn n_layers(self) -> u32 {
        word(self.raw, 12)
    }
    pub fn n_doodad_refs(self) -> u32 {
        word(self.raw, 16)
    }
    pub fn area_id(self) -> u32 {
        word(self.raw, 52)
    }
    pub fn raw_low_resolution_holes(self) -> u32 {
        word(self.raw, 60)
    }
    pub fn low_resolution_holes(self) -> u16 {
        self.raw_low_resolution_holes() as u16
    }
    /// zpos, xpos, ypos in the source declaration; ypos is vertical height.
    pub fn position_bits(self) -> [u32; 3] {
        [
            word(self.raw, 104),
            word(self.raw, 108),
            word(self.raw, 112),
        ]
    }
    pub fn height_base_bits(self) -> u32 {
        word(self.raw, 112)
    }
    pub fn mclq_offset(self) -> u32 {
        word(self.raw, 96)
    }
    pub fn mclq_size(self) -> u32 {
        word(self.raw, 100)
    }
    /// These source offsets are metadata only, never followed as pointers.
    pub fn low_resolution_offsets(self) -> Option<[u32; 2]> {
        (self.flags() & HIGH_RESOLUTION_HOLES == 0)
            .then(|| [word(self.raw, 20), word(self.raw, 24)])
    }
    pub fn high_resolution_holes(self) -> Option<[u8; 8]> {
        (self.flags() & HIGH_RESOLUTION_HOLES != 0).then(|| {
            let mut rows = [0; 8];
            rows.copy_from_slice(&self.raw[20..28]);
            rows
        })
    }
}

#[derive(Clone, Copy)]
pub struct Mcvt<'a> {
    raw: &'a [u8],
}
impl<'a> Mcvt<'a> {
    pub const fn raw(self) -> &'a [u8] {
        self.raw
    }
    pub fn value_bits(self, index: usize) -> Option<u32> {
        if index >= MCVT_VALUES {
            return None;
        }
        Some(word(self.raw, index * 4))
    }
    pub fn bits(self) -> impl ExactSizeIterator<Item = u32> + 'a {
        self.raw
            .as_chunks::<4>()
            .0
            .iter()
            .map(|bytes| u32::from_le_bytes(*bytes))
    }
}

pub struct Cell<'a> {
    offset: usize,
    header: McnkHeader<'a>,
    children: Chunks<'a>,
    mcvt: Option<Mcvt<'a>>,
}
impl<'a> Cell<'a> {
    /// MCNK frame offset relative to the full ADT input.
    pub const fn offset(&self) -> usize {
        self.offset
    }
    pub const fn header(&self) -> McnkHeader<'a> {
        self.header
    }
    /// Child offsets are relative to this cell's framed suffix, after its
    /// 8-byte outer frame and 128-byte prefix. No recursive scan occurs.
    pub const fn children(&self) -> &Chunks<'a> {
        &self.children
    }
    pub const fn mcvt(&self) -> Option<Mcvt<'a>> {
        self.mcvt
    }
}

pub struct Adt<'a> {
    chunks: Chunks<'a>,
    cells: Vec<Cell<'a>>,
    total_chunks: usize,
}
impl<'a> Adt<'a> {
    pub fn parse(bytes: &'a [u8], limits: Limits) -> Result<Self, Error> {
        let chunks = Chunks::parse(bytes, limits)?;
        let version = chunks.unique(*b"MVER")?.ok_or(Error::MissingChunk)?;
        if version.payload().len() != 4 {
            return Err(Error::InvalidChunkSize);
        }
        if word(version.payload(), 0) != FILE_VERSION {
            return Err(Error::UnsupportedVersion);
        }
        let mut remaining = limits
            .max_chunks
            .checked_sub(chunks.all().len())
            .ok_or(Error::Limit)?;
        let mut cells = Vec::new();
        cells
            .try_reserve_exact(
                chunks
                    .all()
                    .iter()
                    .filter(|chunk| chunk.fourcc() == *b"MCNK")
                    .count(),
            )
            .map_err(|_| Error::Limit)?;
        for chunk in chunks
            .all()
            .iter()
            .filter(|chunk| chunk.fourcc() == *b"MCNK")
        {
            let raw = chunk
                .payload()
                .get(..MCNK_PREFIX_BYTES)
                .ok_or(Error::InvalidChunkSize)?;
            let header = McnkHeader { raw };
            if header.ix() >= CELLS_PER_AXIS as u32 || header.iy() >= CELLS_PER_AXIS as u32 {
                return Err(Error::InvalidCellCoordinates);
            }
            let children = Chunks::parse(
                &chunk.payload()[MCNK_PREFIX_BYTES..],
                Limits {
                    max_chunks: remaining,
                    ..limits
                },
            )
            .map_err(|error| match error {
                Error::Limit => Error::Limit,
                _ => Error::UnsupportedNestedLayout,
            })?;
            remaining = remaining
                .checked_sub(children.all().len())
                .ok_or(Error::Limit)?;
            let mcvt = match children.unique(*b"MCVT")? {
                Some(chunk) => {
                    if chunk.payload().len() != MCVT_VALUES * 4 {
                        return Err(Error::InvalidChunkSize);
                    }
                    Some(Mcvt {
                        raw: chunk.payload(),
                    })
                }
                None => None,
            };
            cells.push(Cell {
                offset: chunk.offset(),
                header,
                children,
                mcvt,
            });
        }
        Ok(Self {
            chunks,
            cells,
            total_chunks: limits.max_chunks - remaining,
        })
    }
    pub const fn chunks(&self) -> &Chunks<'a> {
        &self.chunks
    }
    /// Physical order, including repeated coordinates. No cell sorting occurs.
    pub fn cells(&self) -> &[Cell<'a>] {
        &self.cells
    }
    /// Global frame budget includes top-level and all direct nested frames.
    pub const fn total_chunks(&self) -> usize {
        self.total_chunks
    }

    pub fn project_heights(&self) -> Result<HeightProjection, Error> {
        let mut projection = HeightProjection {
            v9_bits: zeroed((GRID_SIZE + 1) * (GRID_SIZE + 1))?,
            v8_bits: zeroed(GRID_SIZE * GRID_SIZE)?,
            area_ids: [0; CELL_COUNT],
            holes: [0; CELL_COUNT * 8],
            has_holes: false,
            coverage: [false; CELL_COUNT],
            visited_cells: self.cells.len(),
        };
        for cell in &self.cells {
            let header = cell.header;
            let ix = header.ix() as usize;
            let iy = header.iy() as usize;
            let base_bits = header.height_base_bits();
            let base = f32::from_bits(base_bits);
            if !base.is_finite() {
                return Err(Error::NonFiniteHeight);
            }
            projection.coverage[iy * CELLS_PER_AXIS + ix] = true;
            projection.area_ids[iy * CELLS_PER_AXIS + ix] = header.area_id() as u16;
            for y in 0..=CELL_SIZE {
                for x in 0..=CELL_SIZE {
                    let value = match cell.mcvt {
                        Some(mcvt) => height_sum(base, word(mcvt.raw, (17 * y + x) * 4))?,
                        None => base_bits,
                    };
                    projection.v9_bits
                        [(iy * CELL_SIZE + y) * (GRID_SIZE + 1) + ix * CELL_SIZE + x] = value;
                }
            }
            for y in 0..CELL_SIZE {
                for x in 0..CELL_SIZE {
                    let value = match cell.mcvt {
                        Some(mcvt) => height_sum(base, word(mcvt.raw, (17 * y + 9 + x) * 4))?,
                        None => base_bits,
                    };
                    projection.v8_bits[(iy * CELL_SIZE + y) * GRID_SIZE + ix * CELL_SIZE + x] =
                        value;
                }
            }
            let start = (iy * CELLS_PER_AXIS + ix) * 8;
            let rows = &mut projection.holes[start..start + 8];
            if let Some(high) = header.high_resolution_holes() {
                rows.copy_from_slice(&high);
            } else {
                let low = header.low_resolution_holes();
                for (y, row) in rows.iter_mut().enumerate() {
                    for x in 0..8 {
                        if low & (1 << ((y / 2) * 4 + x / 2)) != 0 {
                            *row |= 1 << x;
                        }
                    }
                }
            }
            // Native hasHoles is sticky across overwrites, not recomputed
            // from the final array when a later high-resolution cell clears it.
            if rows.iter().any(|&row| row != 0) {
                projection.has_holes = true;
            }
        }
        Ok(projection)
    }
}

/// Prepacking arrays in source [y][x] row-major order. Unvisited cells remain
/// zero; coverage distinguishes them from a visited zero-height cell.
pub struct HeightProjection {
    v9_bits: Vec<u32>,
    v8_bits: Vec<u32>,
    area_ids: [u16; CELL_COUNT],
    holes: [u8; CELL_COUNT * 8],
    has_holes: bool,
    coverage: [bool; CELL_COUNT],
    visited_cells: usize,
}
impl HeightProjection {
    pub fn v9_bits(&self) -> &[u32] {
        &self.v9_bits
    }
    pub fn v8_bits(&self) -> &[u32] {
        &self.v8_bits
    }
    /// Source ConvertADT assigns the raw u32 area ID to a u16 array.
    pub const fn area_ids(&self) -> &[u16; CELL_COUNT] {
        &self.area_ids
    }
    pub fn hole_bytes(&self) -> &[u8] {
        &self.holes
    }
    pub const fn has_holes(&self) -> bool {
        self.has_holes
    }
    pub const fn coverage(&self) -> &[bool; CELL_COUNT] {
        &self.coverage
    }
    pub const fn visited_cells(&self) -> usize {
        self.visited_cells
    }
    pub fn unique_cells(&self) -> usize {
        self.coverage.iter().filter(|&&value| value).count()
    }
    pub fn v9_at(&self, x: usize, y: usize) -> Option<u32> {
        if x > GRID_SIZE || y > GRID_SIZE {
            return None;
        }
        Some(self.v9_bits[y * (GRID_SIZE + 1) + x])
    }
    pub fn v8_at(&self, x: usize, y: usize) -> Option<u32> {
        if x >= GRID_SIZE || y >= GRID_SIZE {
            return None;
        }
        Some(self.v8_bits[y * GRID_SIZE + x])
    }
    pub fn holes_at(&self, x: usize, y: usize) -> Option<[u8; 8]> {
        if x >= CELLS_PER_AXIS || y >= CELLS_PER_AXIS {
            return None;
        }
        let start = (y * CELLS_PER_AXIS + x) * 8;
        let mut rows = [0; 8];
        rows.copy_from_slice(&self.holes[start..start + 8]);
        Some(rows)
    }
}

fn zeroed(count: usize) -> Result<Vec<u32>, Error> {
    let mut values = Vec::new();
    values.try_reserve_exact(count).map_err(|_| Error::Limit)?;
    values.resize(count, 0);
    Ok(values)
}
fn height_sum(base: f32, relative_bits: u32) -> Result<u32, Error> {
    let relative = f32::from_bits(relative_bits);
    if !relative.is_finite() {
        return Err(Error::NonFiniteHeight);
    }
    let height = base + relative;
    if !height.is_finite() {
        return Err(Error::NonFiniteHeight);
    }
    Ok(height.to_bits())
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
