// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original implementation from the pinned format facts.
// Sources: vmap4_extractor/wmo.h, wmo.cpp and vmapexport.cpp at ORACLE_COMMIT.
// No TrinityCore or G3D implementation is copied or imported.
//! Complete borrowed WMO framing and the default small-mode geometry projection.
//!
//! `Root::parse` + `Group::parse` -> `resolve_contexts` -> `project_geometry`.
//! `liquid` projects source liquid types against supplied table lookups; `raw`
//! converts complete supplied groups and verifies the raw WMO intermediate.
//! Doodad acquisition/outcome, VMap assembly and runtime remain separate.
//! No acquisition or I/O occurs in this API.
//!
//! Admission is deliberately stricter than the unchecked C++ reader: exact
//! framing/record extents, unique consumed chunks, exclusive alternative arrays,
//! finite geometry/liquid values, ordered bounds and all index references are
//! checked, even for triangles or groups later omitted. Unknown chunks remain
//! opaque. MOGP's declared container is authoritative, unlike the source's
//! unconditional 68-byte cursor adjustment. MVER is optional and not version
//! gated by the pin; when present its single u32 is retained.

pub mod doodads;
mod geometry;
mod group;
pub mod liquid;
pub mod raw;
mod root;

pub use geometry::{Geometry, GroupContext, GroupInput, SkipReason, resolve_contexts};
pub use group::{Group, GroupHeader, LiquidView, TriangleMaterial};
pub use root::{DoodadSet, DoodadSpawn, Root, RootHeader};

pub const MAX_INPUT_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_VERTICES: usize = 1_000_000;
pub const MAX_INDICES: usize = 3_000_000;
pub const MAX_CHUNKS: usize = 4096;
pub const MAX_GROUPS: usize = 1_000_000;
pub const MAX_METADATA_RECORDS: usize = 1_000_000;
pub const MAX_PROJECTION_BYTES: usize = 64 * 1024 * 1024;

/// Reducible ceilings, including zero. Projection budgets count live vector
/// capacities (including renumbering scratch), not allocator overhead or RSS.
/// Borrowed input bytes are governed separately by `max_input_bytes`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub max_input_bytes: usize,
    pub max_vertices: usize,
    pub max_indices: usize,
    pub max_chunks: usize,
    pub max_groups: usize,
    pub max_metadata_records: usize,
    pub max_projection_bytes: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_input_bytes: MAX_INPUT_BYTES,
            max_vertices: MAX_VERTICES,
            max_indices: MAX_INDICES,
            max_chunks: MAX_CHUNKS,
            max_groups: MAX_GROUPS,
            max_metadata_records: MAX_METADATA_RECORDS,
            max_projection_bytes: MAX_PROJECTION_BYTES,
        }
    }
}

impl Limits {
    fn validate(self) -> Result<(), Error> {
        if self.max_input_bytes > MAX_INPUT_BYTES
            || self.max_vertices > MAX_VERTICES
            || self.max_indices > MAX_INDICES
            || self.max_chunks > MAX_CHUNKS
            || self.max_groups > MAX_GROUPS
            || self.max_metadata_records > MAX_METADATA_RECORDS
            || self.max_projection_bytes > MAX_PROJECTION_BYTES
        {
            return Err(Error::InvalidLimits);
        }
        Ok(())
    }
    fn input(self, bytes: &[u8]) -> Result<(), Error> {
        self.validate()?;
        if bytes.len() > self.max_input_bytes {
            return Err(Error::Limit);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidLimits,
    Limit,
    Framing,
    MissingHeader,
    Duplicate,
    Unsupported,
    Shape,
    Bounds,
    NonFinite,
    InvertedBounds,
    Index,
    Name,
    Inventory,
    Allocation,
}

impl Error {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidLimits => "wmo_invalid_limits",
            Self::Limit => "wmo_policy_limit",
            Self::Framing => "wmo_framing",
            Self::MissingHeader => "wmo_missing_header",
            Self::Duplicate => "wmo_duplicate_chunk",
            Self::Unsupported => "wmo_unsupported",
            Self::Shape => "wmo_record_shape",
            Self::Bounds => "wmo_bounds",
            Self::NonFinite => "wmo_nonfinite",
            Self::InvertedBounds => "wmo_inverted_bounds",
            Self::Index => "wmo_invalid_index",
            Self::Name => "wmo_invalid_name",
            Self::Inventory => "wmo_group_inventory",
            Self::Allocation => "wmo_allocation",
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        out.write_str(self.code())
    }
}
impl std::error::Error for Error {}

/// A validated frame. `tag` is in readable order (the file stores it reversed).
/// Unknown payloads are preserved without interpreting their contents.
#[derive(Clone, Copy)]
pub struct Chunk<'a> {
    pub tag: [u8; 4],
    pub payload: &'a [u8],
    pub offset: usize,
}

/// Created only over fully framed bytes. No Debug implementation exposes data.
pub struct Chunks<'a> {
    bytes: &'a [u8],
    at: usize,
}
impl<'a> Iterator for Chunks<'a> {
    type Item = Chunk<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.at == self.bytes.len() {
            return None;
        }
        let (chunk, end) = frame(self.bytes, self.at).ok()?;
        self.at = end;
        Some(chunk)
    }
}

fn chunks(bytes: &[u8]) -> Chunks<'_> {
    Chunks { bytes, at: 0 }
}
fn frame(bytes: &[u8], at: usize) -> Result<(Chunk<'_>, usize), Error> {
    let head = bytes
        .get(at..at.checked_add(8).ok_or(Error::Bounds)?)
        .ok_or(Error::Framing)?;
    let start = at + 8;
    let end = start
        .checked_add(word(head, 4)? as usize)
        .ok_or(Error::Bounds)?;
    let payload = bytes.get(start..end).ok_or(Error::Framing)?;
    Ok((
        Chunk {
            tag: [head[3], head[2], head[1], head[0]],
            payload,
            offset: at,
        },
        end,
    ))
}
fn scan<'a>(
    bytes: &'a [u8],
    count: &mut usize,
    limits: Limits,
    mut accept: impl FnMut(Chunk<'a>) -> Result<(), Error>,
) -> Result<(), Error> {
    let mut at = 0;
    while at < bytes.len() {
        if *count >= limits.max_chunks {
            return Err(Error::Limit);
        }
        let (chunk, end) = frame(bytes, at)?;
        *count += 1;
        accept(chunk)?;
        at = end;
    }
    Ok(())
}
fn once<'a>(slot: &mut Option<&'a [u8]>, bytes: &'a [u8]) -> Result<(), Error> {
    if slot.replace(bytes).is_some() {
        return Err(Error::Duplicate);
    }
    Ok(())
}
fn exact(bytes: &[u8], length: usize) -> Result<(), Error> {
    if bytes.len() != length {
        return Err(Error::Shape);
    }
    Ok(())
}
fn records(bytes: &[u8], width: usize, cap: usize) -> Result<usize, Error> {
    if !bytes.len().is_multiple_of(width) {
        return Err(Error::Shape);
    }
    let count = bytes.len() / width;
    if count > cap {
        return Err(Error::Limit);
    }
    Ok(count)
}
fn word(bytes: &[u8], at: usize) -> Result<u32, Error> {
    Ok(u32::from_le_bytes(
        bytes
            .get(at..at.checked_add(4).ok_or(Error::Bounds)?)
            .ok_or(Error::Bounds)?
            .try_into()
            .map_err(|_| Error::Bounds)?,
    ))
}
fn short(bytes: &[u8], at: usize) -> Result<u16, Error> {
    Ok(u16::from_le_bytes(
        bytes
            .get(at..at.checked_add(2).ok_or(Error::Bounds)?)
            .ok_or(Error::Bounds)?
            .try_into()
            .map_err(|_| Error::Bounds)?,
    ))
}
fn finite(bits: u32) -> Result<u32, Error> {
    if !f32::from_bits(bits).is_finite() {
        return Err(Error::NonFinite);
    }
    Ok(bits)
}
fn bits3(bytes: &[u8], at: usize) -> Result<[u32; 3], Error> {
    Ok([
        finite(word(bytes, at)?)?,
        finite(word(bytes, at + 4)?)?,
        finite(word(bytes, at + 8)?)?,
    ])
}
fn bounds(bytes: &[u8], at: usize) -> Result<[u32; 6], Error> {
    let lo = bits3(bytes, at)?;
    let hi = bits3(bytes, at + 12)?;
    for axis in 0..3 {
        if f32::from_bits(lo[axis]) > f32::from_bits(hi[axis]) {
            return Err(Error::InvertedBounds);
        }
    }
    Ok([lo[0], lo[1], lo[2], hi[0], hi[1], hi[2]])
}
fn cstring(bytes: &[u8], at: usize) -> Result<&[u8], Error> {
    let tail = bytes.get(at..).ok_or(Error::Name)?;
    let end = tail.iter().position(|b| *b == 0).ok_or(Error::Name)?;
    Ok(&tail[..end])
}
fn reserve<T>(count: usize, remaining: &mut usize) -> Result<Vec<T>, Error> {
    let needed = count
        .checked_mul(std::mem::size_of::<T>())
        .ok_or(Error::Bounds)?;
    if needed > *remaining {
        return Err(Error::Limit);
    }
    let mut values = Vec::new();
    values
        .try_reserve_exact(count)
        .map_err(|_| Error::Allocation)?;
    let capacity = values
        .capacity()
        .checked_mul(std::mem::size_of::<T>())
        .ok_or(Error::Bounds)?;
    *remaining = remaining.checked_sub(capacity).ok_or(Error::Limit)?;
    Ok(values)
}

#[cfg(test)]
mod tests;
