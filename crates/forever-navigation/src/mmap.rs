// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of the TrinityCore Project. See AUTHORS for Copyright information.
// This program is free software; you can redistribute it and/or modify it under
// the terms of the GNU General Public License as published by the Free Software
// Foundation; either version 2 of the License, or (at your option) any later version.
// This program is distributed in the hope that it will be useful, but WITHOUT
// ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
// FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.
// You should have received a copy of the GNU General Public License along with
// this program. If not, see <http://www.gnu.org/licenses/>.
// Modified/ported 2026-10-11: explicit Forever MMap 16 codecs and strict framing.
//! Versioned MMap containers. Parsing proves complete framing, not safe Detour
//! topology, source provenance, parent selection or suitability for pathfinding.
use crate::{detour, offmesh};

pub const MAGIC: u32 = 0x4d4d4150;
pub const VERSION: u32 = 16;
pub const GLOBAL_HEADER_BYTES: usize = 40;
pub const TILE_HEADER_BYTES: usize = 20;
pub const OFFMESH_BYTES: usize = 44;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Limit,
    Allocation,
    Size,
    Magic,
    Version,
    DetourVersion,
    Params,
    Map,
    Coordinates,
    Padding,
    Payload,
}
impl Error {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Limit => "mmap_policy_limit",
            Self::Allocation => "mmap_allocation_failed",
            Self::Size => "mmap_file_size",
            Self::Magic => "mmap_magic",
            Self::Version => "mmap_version",
            Self::DetourVersion => "mmap_detour_version",
            Self::Params => "mmap_invalid_params",
            Self::Map => "mmap_offmesh_map_mismatch",
            Self::Coordinates => "mmap_tile_coordinates",
            Self::Padding => "mmap_noncanonical_header",
            Self::Payload => "mmap_detour_framing",
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}
impl std::error::Error for Error {}

#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub records: usize,
    pub file_bytes: usize,
    pub heap_bytes: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            records: 65536,
            file_bytes: 64 * 1024 * 1024 + TILE_HEADER_BYTES,
            heap_bytes: 64 * 1024 * 1024 + TILE_HEADER_BYTES,
        }
    }
}
impl Limits {
    pub(crate) fn validate(self) -> Result<(), Error> {
        let max = Self::default();
        if self.records > max.records
            || self.file_bytes > max.file_bytes
            || self.heap_bytes > max.heap_bytes
        {
            return Err(Error::Limit);
        }
        Ok(())
    }
    fn bytes(self, size: usize) -> Result<(), Error> {
        self.validate()?;
        if size > self.file_bytes {
            return Err(Error::Limit);
        }
        Ok(())
    }
    fn allocate(self, size: usize) -> Result<Vec<u8>, Error> {
        self.bytes(size)?;
        if size > self.heap_bytes {
            return Err(Error::Limit);
        }
        let mut out = Vec::new();
        out.try_reserve_exact(size).map_err(|_| Error::Allocation)?;
        if out.capacity() > self.heap_bytes {
            return Err(Error::Limit);
        }
        out.resize(size, 0);
        Ok(out)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Params {
    pub origin: [f32; 3],
    pub tile_width: f32,
    pub tile_height: f32,
    pub max_tiles: i32,
    /// The original 64-bit-ref MapBuilder writes 1 << 31 (i32::MIN).
    /// Detour's fixed-bit build does not use this value for reference allocation.
    pub max_polys: i32,
}
impl Params {
    pub fn validate(self) -> Result<(), Error> {
        if !self.origin.into_iter().all(f32::is_finite)
            || ![self.tile_width, self.tile_height]
                .into_iter()
                .all(|v| v.is_finite() && v > 0.0)
            || !(1..=1 << 21).contains(&self.max_tiles)
            || (self.max_polys <= 0 && self.max_polys != i32::MIN)
        {
            return Err(Error::Params);
        }
        Ok(())
    }
}
fn word(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(b[at..at + 4].try_into().expect("checked framing"))
}
fn float(b: &[u8], at: usize) -> f32 {
    f32::from_bits(word(b, at))
}
fn put(b: &mut [u8], at: usize, v: u32) {
    b[at..at + 4].copy_from_slice(&v.to_le_bytes());
}
fn params(b: &[u8]) -> Params {
    Params {
        origin: [float(b, 8), float(b, 12), float(b, 16)],
        tile_width: float(b, 20),
        tile_height: float(b, 24),
        max_tiles: word(b, 28) as i32,
        max_polys: word(b, 32) as i32,
    }
}
fn source(b: &[u8]) -> offmesh::Source {
    offmesh::Source {
        map_id: word(b, 0),
        row: word(b, 4),
        column: word(b, 8),
        from: [float(b, 12), float(b, 16), float(b, 20)],
        to: [float(b, 24), float(b, 28), float(b, 32)],
        radius: float(b, 36),
        connection_flags: b[40],
        area: b[41],
        flags: u16::from_le_bytes([b[42], b[43]]),
    }
}

/// Writes the map's selected records in input order, including duplicates and
/// every raw bit of coordinates/radius/flags. No axis projection is applied.
/// Off-mesh numeric admission remains the responsibility of select/build.
pub fn encode_global(
    map_id: u32,
    params: Params,
    sources: &[offmesh::Source],
    limits: Limits,
) -> Result<Vec<u8>, Error> {
    limits.validate()?;
    params.validate()?;
    if sources.len() > limits.records {
        return Err(Error::Limit);
    }
    let selected = || sources.iter().filter(|s| s.map_id == map_id);
    let count = selected().count();
    let mut out = limits.allocate(GLOBAL_HEADER_BYTES + count * OFFMESH_BYTES)?;
    put(&mut out, 0, MAGIC);
    put(&mut out, 4, VERSION);
    for (i, v) in params.origin.into_iter().enumerate() {
        put(&mut out, 8 + i * 4, v.to_bits());
    }
    put(&mut out, 20, params.tile_width.to_bits());
    put(&mut out, 24, params.tile_height.to_bits());
    put(&mut out, 28, params.max_tiles as u32);
    put(&mut out, 32, params.max_polys as u32);
    put(&mut out, 36, count as u32);
    for (s, b) in selected().zip(
        out[GLOBAL_HEADER_BYTES..]
            .as_chunks_mut::<OFFMESH_BYTES>()
            .0
            .iter_mut(),
    ) {
        for (i, v) in [s.map_id, s.row, s.column].into_iter().enumerate() {
            put(b, i * 4, v);
        }
        for (i, v) in s.from.into_iter().chain(s.to).chain([s.radius]).enumerate() {
            put(b, 12 + i * 4, v.to_bits());
        }
        b[40] = s.connection_flags;
        b[41] = s.area;
        b[42..44].copy_from_slice(&s.flags.to_le_bytes());
    }
    Ok(out)
}

#[derive(Debug)]
pub struct GlobalView<'a> {
    bytes: &'a [u8],
    params: Params,
}
impl<'a> GlobalView<'a> {
    /// Strict complete file: legacy headers, missing records, trailing bytes and
    /// records belonging to another supplied map ID are rejected.
    pub fn parse(bytes: &'a [u8], map_id: u32, limits: Limits) -> Result<Self, Error> {
        limits.bytes(bytes.len())?;
        if bytes.len() < GLOBAL_HEADER_BYTES {
            return Err(Error::Size);
        }
        if word(bytes, 0) != MAGIC {
            return Err(Error::Magic);
        }
        if word(bytes, 4) != VERSION {
            return Err(Error::Version);
        }
        let count = word(bytes, 36) as usize;
        if count > limits.records {
            return Err(Error::Limit);
        }
        if bytes.len() != GLOBAL_HEADER_BYTES + count * OFFMESH_BYTES {
            return Err(Error::Size);
        }
        let params = params(bytes);
        params.validate()?;
        let view = Self { bytes, params };
        if view.sources().any(|s| s.map_id != map_id) {
            return Err(Error::Map);
        }
        Ok(view)
    }
    pub const fn params(&self) -> Params {
        self.params
    }
    pub fn sources(&self) -> impl ExactSizeIterator<Item = offmesh::Source> + '_ {
        self.bytes[GLOBAL_HEADER_BYTES..]
            .as_chunks::<OFFMESH_BYTES>()
            .0
            .iter()
            .map(|b| source(b))
    }
    pub const fn bytes(&self) -> &'a [u8] {
        self.bytes
    }
}

/// Container framing only: does not validate polygon indices, adjacency, detail
/// topology or BV traversal. Never use this check as an untrusted FFI safety gate.
fn payload_frame(b: &[u8]) -> Result<(), Error> {
    if b.len() < 100 || word(b, 0) != detour::MAGIC {
        return Err(Error::Payload);
    }
    if word(b, 4) != detour::VERSION {
        return Err(Error::DetourVersion);
    }
    // All array strides are already multiples of four in this ABI.
    let mut len = 100usize;
    for (at, width) in [
        (28, 12),
        (24, 32),
        (32, 16),
        (36, 12),
        (40, 12),
        (44, 4),
        (48, 16),
        (52, 36),
    ] {
        let count = word(b, at);
        if count > i32::MAX as u32 {
            return Err(Error::Payload);
        }
        len = len
            .checked_add((count as usize).checked_mul(width).ok_or(Error::Payload)?)
            .ok_or(Error::Payload)?;
    }
    if len != b.len() || word(b, 24) == 0 || word(b, 28) == 0 {
        return Err(Error::Payload);
    }
    Ok(())
}
fn encode_payload(payload: &[u8], uses_liquids: bool, limits: Limits) -> Result<Vec<u8>, Error> {
    limits.bytes(
        payload
            .len()
            .checked_add(TILE_HEADER_BYTES)
            .ok_or(Error::Limit)?,
    )?;
    payload_frame(payload)?;
    let mut out = limits.allocate(TILE_HEADER_BYTES + payload.len())?;
    put(&mut out, 0, MAGIC);
    put(&mut out, 4, detour::VERSION);
    put(&mut out, 8, VERSION);
    put(&mut out, 12, payload.len() as u32);
    out[16] = u8::from(uses_liquids);
    out[TILE_HEADER_BYTES..].copy_from_slice(payload);
    Ok(out)
}
/// Frames pristine builder bytes. Use encode_saved_tile for original producer
/// output after fresh-navmesh addTile mutation.
pub fn encode_tile(
    data: &detour::Data,
    uses_liquids: bool,
    limits: Limits,
) -> Result<Vec<u8>, Error> {
    encode_payload(data.bytes(), uses_liquids, limits)
}

/// Frames final offline producer bytes, including links and endpoint snapping.
pub fn encode_saved_tile(
    data: &detour::SavedData,
    uses_liquids: bool,
    limits: Limits,
) -> Result<Vec<u8>, Error> {
    encode_payload(data.bytes(), uses_liquids, limits)
}

#[derive(Debug)]
pub struct TileView<'a> {
    bytes: &'a [u8],
}
impl<'a> TileView<'a> {
    /// Checks version, canonical header and exact outer/inner block lengths.
    /// This does not certify Detour topology or arbitrary-input runtime safety.
    pub fn parse(bytes: &'a [u8], limits: Limits) -> Result<Self, Error> {
        limits.bytes(bytes.len())?;
        if bytes.len() < TILE_HEADER_BYTES {
            return Err(Error::Size);
        }
        if word(bytes, 0) != MAGIC {
            return Err(Error::Magic);
        }
        if word(bytes, 8) != VERSION {
            return Err(Error::Version);
        }
        if word(bytes, 4) != detour::VERSION {
            return Err(Error::DetourVersion);
        }
        if bytes.len() - TILE_HEADER_BYTES != word(bytes, 12) as usize {
            return Err(Error::Size);
        }
        if bytes[16] > 1 || bytes[17..20] != [0; 3] {
            return Err(Error::Padding);
        }
        payload_frame(&bytes[TILE_HEADER_BYTES..])?;
        Ok(Self { bytes })
    }
    pub const fn bytes(&self) -> &'a [u8] {
        self.bytes
    }
    pub fn payload(&self) -> &'a [u8] {
        &self.bytes[TILE_HEADER_BYTES..]
    }
    pub fn uses_liquids(&self) -> bool {
        self.bytes[16] != 0
    }
}

pub fn global_path(map_id: u32) -> String {
    format!("mmaps/{map_id:04}.mmap")
}
pub fn tile_path(map_id: u32, row: u8, column: u8) -> Result<String, Error> {
    if row >= 64 || column >= 64 {
        return Err(Error::Coordinates);
    }
    Ok(format!("mmaps/{map_id:04}_{row:02}_{column:02}.mmtile"))
}

#[cfg(test)]
mod tests;
