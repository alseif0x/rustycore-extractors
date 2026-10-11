// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11 from pinned format/operation facts; no imported code.
//! Direct MDDF/MODF placements and complete, unframed `dir_bin` streams.
//!
//! Source: ORACLE_COMMIT, adtfile.h:25-49, model.cpp:160-182,
//! wmo.cpp:584-629, vec3d.h:42-47 and ModelInstance.cpp:119-185.
//! Projection accepts IDs and already-normalized ASCII names from the caller.
//! Optional submodules provide serial ID allocation and doodad transforms.
//! This module performs no lookup, acquisition, placement filtering, assembly,
//! BIH construction or I/O. Admission requires
//! complete extents, finite used floats, ordered bounds and bounded ASCII names,
//! which is stricter than the source's unchecked records and strlen writes.
mod codec;
pub mod bounds;
pub mod cache;
pub mod doodads;
pub mod ids;
mod source;
#[cfg(test)]
mod tests;

pub use codec::{DirBin, Records, encode};
pub use source::{Mddf, Modf};

pub const MAX_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_RECORDS: usize = 1_000_000;
pub const MAX_NAME_BYTES: usize = 500;

/// Only reductions are accepted, including zero. Output accounts actual vector
/// capacity; borrowed source/codec views allocate nothing. Stack and allocator
/// overhead are outside this byte policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub max_bytes: usize,
    pub max_records: usize,
    pub max_name_bytes: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_bytes: MAX_BYTES,
            max_records: MAX_RECORDS,
            max_name_bytes: MAX_NAME_BYTES,
        }
    }
}
impl Limits {
    fn validate(self) -> Result<(), Error> {
        if self.max_bytes > MAX_BYTES
            || self.max_records > MAX_RECORDS
            || self.max_name_bytes > MAX_NAME_BYTES
        {
            return Err(Error::InvalidLimits);
        }
        Ok(())
    }
    fn one(self, bytes: usize) -> Result<(), Error> {
        self.validate()?;
        if bytes > self.max_bytes || self.max_records == 0 {
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
    Flags,
    NonFinite,
    InvertedBounds,
    Name,
    Allocation,
}
impl Error {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidLimits => "placement_invalid_limits",
            Self::Limit => "placement_policy_limit",
            Self::Framing => "placement_framing",
            Self::Flags => "placement_flags",
            Self::NonFinite => "placement_nonfinite",
            Self::InvertedBounds => "placement_inverted_bounds",
            Self::Name => "placement_name",
            Self::Allocation => "placement_allocation",
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}
impl std::error::Error for Error {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Context {
    pub map_id: u32,
    pub original_map_id: u32,
    pub global_wmo: bool,
}

/// A placement borrows only its supplied name; numeric fields are copied bits.
/// No Debug implementation exposes names or source-derived transforms.
#[derive(Clone, Copy)]
pub struct Placement<'a> {
    flags: u8,
    name_set: u8,
    id: u32,
    position: [u32; 3],
    rotation: [u32; 3],
    scale: u32,
    bounds: Option<[u32; 6]>,
    name: &'a [u8],
}
impl<'a> Placement<'a> {
    pub const fn flags(&self) -> u8 {
        self.flags
    }
    pub const fn name_set(&self) -> u8 {
        self.name_set
    }
    pub const fn id(&self) -> u32 {
        self.id
    }
    pub const fn position_bits(&self) -> [u32; 3] {
        self.position
    }
    pub const fn rotation_bits(&self) -> [u32; 3] {
        self.rotation
    }
    pub const fn scale_bits(&self) -> u32 {
        self.scale
    }
    pub const fn bounds_bits(&self) -> Option<[u32; 6]> {
        self.bounds
    }
    pub const fn name(&self) -> &'a [u8] {
        self.name
    }
    fn validate(&self, limits: Limits) -> Result<(), Error> {
        limits.one(self.encoded_len()?)?;
        if self.flags & !7 != 0 || (self.flags & 1 != 0) != self.bounds.is_some() {
            return Err(Error::Flags);
        }
        finite(&self.position)?;
        finite(&self.rotation)?;
        finite(&[self.scale])?;
        if let Some(bounds) = self.bounds {
            valid_bounds(&bounds)?;
        }
        valid_name(self.name, limits)?;
        Ok(())
    }
    fn encoded_len(&self) -> Result<usize, Error> {
        (if self.bounds.is_some() {
            62usize
        } else {
            38usize
        })
        .checked_add(self.name.len())
        .ok_or(Error::Limit)
    }
}

/// Direct Doodad::Extract projection. `global_wmo` has no effect on MDDF.
pub fn project_m2<'a>(
    source: &Mddf<'_>,
    assigned_id: u32,
    name: &'a [u8],
    context: Context,
    limits: Limits,
) -> Result<Placement<'a>, Error> {
    limits.validate()?;
    let result = Placement {
        flags: if context.map_id != context.original_map_id {
            2
        } else {
            0
        },
        name_set: 0,
        id: assigned_id,
        position: permute(source.position_bits()),
        rotation: source.rotation_bits(),
        scale: (f32::from(source.scale()) / 1024.0_f32).to_bits(),
        bounds: None,
        name,
    };
    result.validate(limits)?;
    Ok(result)
}
pub fn project_wmo<'a>(
    source: &Modf<'_>,
    assigned_id: u32,
    name: &'a [u8],
    context: Context,
    limits: Limits,
) -> Result<Placement<'a>, Error> {
    limits.validate()?;
    let mut position = permute(source.position_bits());
    let input_bounds = source.bounds_bits();
    let mut low = permute([input_bounds[0], input_bounds[1], input_bounds[2]]);
    let mut high = permute([input_bounds[3], input_bounds[4], input_bounds[5]]);
    if context.global_wmo {
        // Three independent f32 adds, including z + +0.0: do not omit the last
        // addition, which normalizes negative zero as in Vec3D::operator+=.
        // Preserve the pinned wmo.cpp decimal literal and its f32 rounding;
        // the resulting offset bits (0x46855555) have a synthetic golden.
        #[allow(clippy::excessive_precision)]
        let offsets = [533.33333_f32 * 32.0_f32, 533.33333_f32 * 32.0_f32, 0.0_f32];
        for index in 0..3 {
            position[index] = (f32::from_bits(position[index]) + offsets[index]).to_bits();
            low[index] = (f32::from_bits(low[index]) + offsets[index]).to_bits();
            high[index] = (f32::from_bits(high[index]) + offsets[index]).to_bits();
        }
    }
    let result = Placement {
        flags: 1
            | if context.map_id != context.original_map_id {
                2
            } else {
                0
            }
            | if source.flags() & 1 != 0 { 4 } else { 0 },
        name_set: source.name_set() as u8,
        id: assigned_id,
        position,
        rotation: source.rotation_bits(),
        scale: if source.flags() & 4 != 0 {
            (f32::from(source.scale()) / 1024.0_f32).to_bits()
        } else {
            1.0_f32.to_bits()
        },
        bounds: Some([low[0], low[1], low[2], high[0], high[1], high[2]]),
        name,
    };
    result.validate(limits)?;
    Ok(result)
}
fn permute(v: [u32; 3]) -> [u32; 3] {
    [v[2], v[0], v[1]]
}
fn finite(values: &[u32]) -> Result<(), Error> {
    if values.iter().any(|v| !f32::from_bits(*v).is_finite()) {
        Err(Error::NonFinite)
    } else {
        Ok(())
    }
}
fn valid_bounds(values: &[u32; 6]) -> Result<(), Error> {
    finite(values)?;
    for index in 0..3 {
        if f32::from_bits(values[index]) > f32::from_bits(values[index + 3]) {
            return Err(Error::InvertedBounds);
        }
    }
    Ok(())
}
fn valid_name(name: &[u8], limits: Limits) -> Result<(), Error> {
    if name.len() > limits.max_name_bytes {
        return Err(Error::Limit);
    }
    if name.is_empty() || name.iter().any(|v| *v == 0 || !v.is_ascii()) {
        return Err(Error::Name);
    }
    Ok(())
}
fn word(bytes: &[u8], at: usize) -> Result<u32, Error> {
    Ok(u32::from_le_bytes(
        bytes
            .get(at..at.checked_add(4).ok_or(Error::Framing)?)
            .ok_or(Error::Framing)?
            .try_into()
            .map_err(|_| Error::Framing)?,
    ))
}
fn short(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([bytes[at], bytes[at + 1]])
}
fn vector(bytes: &[u8], at: usize) -> Result<[u32; 3], Error> {
    Ok([word(bytes, at)?, word(bytes, at + 4)?, word(bytes, at + 8)?])
}
