// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original implementation from pinned format facts.
// No TrinityCore or G3D implementation is copied or imported.
//! Bounded M2/WMO collision, raw VMAP04E and placement formats.
//!
//! Pure APIs cover geometry, object chunks, serial ID allocation, doodad
//! selection and supplied placement projection. The separate BIH API
//! builds and validates bounded spatial trees from supplied primitive bounds.
//! Pure bounded queries operate on completely validated trees and model views;
//! [`store`] adds immutable final-package loading and static map queries, with
//! an optional Linux filesystem source. There is no CASC acquisition or server
//! provider installation here; full runtime integration remains separate.
//! Supply complete bytes from a separately verified acquisition layer.
//! The formats have no build tag.
//! A successful parse does not establish the originating client build.

pub mod assembly;
pub mod bih;
pub mod gameobject_models;
mod m2;
pub mod objects;
pub mod placement;
pub mod query;
mod raw;
pub mod store;
pub mod vmo;
pub mod wmo;

pub use m2::M2Collision;
pub use raw::RawM2;

pub const ORACLE_COMMIT: &str = "02245dcd245e7433e524577656177723d3e4992e";
pub const M2_HEADER_BYTES: usize = 304;
pub const MAX_INPUT_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_RAW_BYTES: usize = 32 * 1024 * 1024;
pub const MAX_VERTICES: usize = 1_000_000;
pub const MAX_INDICES: usize = 3_000_000;
pub const RAW_MAGIC: &[u8; 8] = b"VMAP04E\0";

/// Caller limits can reduce, including to zero, the fixed policy ceilings.
/// Raw input has an additional unconditional [`MAX_RAW_BYTES`] ceiling.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub max_input_bytes: usize,
    pub max_vertices: usize,
    pub max_indices: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_input_bytes: MAX_INPUT_BYTES,
            max_vertices: MAX_VERTICES,
            max_indices: MAX_INDICES,
        }
    }
}

impl Limits {
    fn validate(self) -> Result<(), Error> {
        if self.max_input_bytes > MAX_INPUT_BYTES
            || self.max_vertices > MAX_VERTICES
            || self.max_indices > MAX_INDICES
        {
            return Err(Error::InvalidLimits);
        }
        Ok(())
    }

    fn counts(self, vertices: usize, indices: usize) -> Result<(), Error> {
        if vertices > self.max_vertices || indices > self.max_indices {
            return Err(Error::Limit);
        }
        if vertices == 0 || indices == 0 || !indices.is_multiple_of(3) {
            return Err(Error::Geometry);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidLimits,
    Limit,
    MissingMd20,
    Truncated,
    Bounds,
    Geometry,
    Index,
    NonFinite,
    InvertedBounds,
    RawFormat,
    RawExtent,
    Allocation,
}

impl Error {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidLimits => "vmap_invalid_limits",
            Self::Limit => "vmap_policy_limit",
            Self::MissingMd20 => "vmap_missing_md20",
            Self::Truncated => "vmap_truncated",
            Self::Bounds => "vmap_array_bounds",
            Self::Geometry => "vmap_invalid_geometry",
            Self::Index => "vmap_invalid_index",
            Self::NonFinite => "vmap_nonfinite",
            Self::InvertedBounds => "vmap_inverted_bounds",
            Self::RawFormat => "vmap_raw_m2_format",
            Self::RawExtent => "vmap_raw_m2_extent",
            Self::Allocation => "vmap_allocation",
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        out.write_str(self.code())
    }
}

impl std::error::Error for Error {}

fn word(bytes: &[u8], offset: usize) -> Result<u32, Error> {
    let end = offset.checked_add(4).ok_or(Error::Bounds)?;
    let data = bytes.get(offset..end).ok_or(Error::Truncated)?;
    Ok(u32::from_le_bytes(
        data.try_into().map_err(|_| Error::Truncated)?,
    ))
}

fn bounded_slice(bytes: &[u8], offset: usize, count: usize, width: usize) -> Result<&[u8], Error> {
    let length = count.checked_mul(width).ok_or(Error::Bounds)?;
    let end = offset.checked_add(length).ok_or(Error::Bounds)?;
    bytes.get(offset..end).ok_or(Error::Bounds)
}

fn finite_vertices(bytes: &[u8]) -> Result<(), Error> {
    for bits in bytes.as_chunks::<4>().0 {
        let bits = u32::from_le_bytes(*bits);
        if !f32::from_bits(bits).is_finite() {
            return Err(Error::NonFinite);
        }
    }
    Ok(())
}

fn bounds_at(bytes: &[u8], offset: usize) -> Result<[u32; 6], Error> {
    let mut bits = [0; 6];
    for (index, value) in bits.iter_mut().enumerate() {
        *value = word(bytes, offset + index * 4)?;
        if !f32::from_bits(*value).is_finite() {
            return Err(Error::NonFinite);
        }
    }
    for index in 0..3 {
        if f32::from_bits(bits[index]) > f32::from_bits(bits[index + 3]) {
            return Err(Error::InvertedBounds);
        }
    }
    Ok(bits)
}

fn vertex_at(bytes: &[u8], index: usize) -> Option<[u32; 3]> {
    let start = index.checked_mul(12)?;
    let data = bytes.get(start..start.checked_add(12)?)?;
    Some([
        word(data, 0).ok()?,
        word(data, 4).ok()?,
        word(data, 8).ok()?,
    ])
}

fn raw_size(vertices: usize, indices: usize) -> Result<usize, Error> {
    100usize
        .checked_add(indices.checked_mul(4).ok_or(Error::Bounds)?)
        .and_then(|value| value.checked_add(vertices.checked_mul(12)?))
        .ok_or(Error::Bounds)
}

#[cfg(test)]
mod tests;
