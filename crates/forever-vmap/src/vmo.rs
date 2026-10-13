// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original Rust implementation from pinned format and
// operation facts. No TrinityCore/G3D implementation is copied or imported.
//! Complete pure RAW M2/WMO to `VMAP_4.E` model conversion.
//!
//! Reference 02245dcd245e7433e524577656177723d3e4992e: TileAssembler.cpp
//! 374-400/475-594 and WorldModel.cpp 87-112/214-387/567-646. Triangle BIHs use
//! leaf size 3; group BIHs use 1. Vertex/index order and float bits are retained.
//! Group bounds come from RAW and need not contain the vertices. No additional
//! winding or coordinate transform occurs. Empty groups end after VERT and
//! discard any source liquid. A model without groups has only its 24-byte header.
//!
//! Conversion retains the existing RAW readers' stricter input domains. The
//! reader here also enforces exact sizes/EOF, finite ordered bounds, valid
//! indices, embedded BIH structure and geometric containment: mesh BIHs contain
//! their referenced triangles and the group BIH contains declared group bounds.
//! This does not prove canonical constructor bytes or authenticate raw origin.
//! Geometry/liquids remain borrowed; the view owns bounded group descriptors.
//! Only model flags 0/1 and safe liquid grids or flat 0/0 surfaces are reviewed.
//! Acquisition, I/O, runtime queries, map trees and placement assembly are absent.

use crate::bih;
mod convert;
mod owned;
pub use owned::Owned;
mod read;
#[cfg(test)]
mod tests;

pub const MAGIC: &[u8; 8] = b"VMAP_4.E";
pub const MAX_RAW_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_BYTES: usize = 128 * 1024 * 1024;
pub const MAX_MEMORY_BYTES: usize = 256 * 1024 * 1024;
pub const MAX_GROUPS: usize = 1_000_000;
pub const MAX_VERTICES: usize = 1_000_000;
pub const MAX_TRIANGLES: usize = 1_000_000;
pub const MAX_LIQUID_VALUES: usize = 1_000_000;
pub const MAX_WORK: u64 = 128_000_000;

/// All fields only reduce policy ceilings. Vertex/triangle/liquid counts apply
/// per group; raw/output byte caps also bound their complete-model totals.
/// Heap accounting uses actual Vec capacities and old/new growth overlap,
/// including BIH scratch/output and final validation. Borrowed source bytes,
/// caller-retained outputs, bounded recursion stack and allocator overhead are
/// excluded; this is not an RSS bound. BIH limits apply to each individual tree;
/// max_work additionally bounds their cumulative constructor work per model.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub max_raw_bytes: usize,
    pub max_bytes: usize,
    pub max_memory_bytes: usize,
    pub max_groups: usize,
    pub max_vertices: usize,
    pub max_triangles: usize,
    pub max_liquid_heights: usize,
    pub max_liquid_tiles: usize,
    pub max_work: u64,
    pub bih: bih::Limits,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_raw_bytes: MAX_RAW_BYTES,
            max_bytes: MAX_BYTES,
            max_memory_bytes: MAX_MEMORY_BYTES,
            max_groups: MAX_GROUPS,
            max_vertices: MAX_VERTICES,
            max_triangles: MAX_TRIANGLES,
            max_liquid_heights: MAX_LIQUID_VALUES,
            max_liquid_tiles: MAX_LIQUID_VALUES,
            max_work: MAX_WORK,
            bih: bih::Limits::default(),
        }
    }
}
impl Limits {
    fn validate(self) -> Result<(), Error> {
        let b = self.bih;
        if self.max_raw_bytes > MAX_RAW_BYTES
            || self.max_bytes > MAX_BYTES
            || self.max_memory_bytes > MAX_MEMORY_BYTES
            || self.max_groups > MAX_GROUPS
            || self.max_vertices > MAX_VERTICES
            || self.max_triangles > MAX_TRIANGLES
            || self.max_liquid_heights > MAX_LIQUID_VALUES
            || self.max_liquid_tiles > MAX_LIQUID_VALUES
            || self.max_work > MAX_WORK
            || b.max_primitives > bih::MAX_PRIMITIVES
            || b.max_tree_words > bih::MAX_TREE_WORDS
            || b.max_bytes > bih::MAX_BYTES
            || b.max_memory_bytes > bih::MAX_MEMORY_BYTES
            || b.max_work > bih::MAX_WORK
            || b.max_depth > bih::MAX_DEPTH
        {
            return Err(Error::InvalidLimits);
        }
        Ok(())
    }
    fn tree(self, resident: usize, used_work: u64) -> Result<bih::Limits, Error> {
        Ok(bih::Limits {
            max_memory_bytes: self.bih.max_memory_bytes.min(
                self.max_memory_bytes
                    .checked_sub(resident)
                    .ok_or(Error::Limit)?,
            ),
            max_work: self
                .bih
                .max_work
                .min(self.max_work.checked_sub(used_work).ok_or(Error::Limit)?),
            ..self.bih
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidLimits,
    Limit,
    Allocation,
    Framing,
    Flags,
    NonFinite,
    Bounds,
    Index,
    Liquid,
    Containment,
    RawM2(crate::Error),
    RawWmo(crate::wmo::Error),
    Bih(bih::Error),
}
impl Error {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidLimits => "vmo_invalid_limits",
            Self::Limit => "vmo_policy_limit",
            Self::Allocation => "vmo_allocation",
            Self::Framing => "vmo_framing",
            Self::Flags => "vmo_model_flags",
            Self::NonFinite => "vmo_nonfinite",
            Self::Bounds => "vmo_bounds",
            Self::Index => "vmo_index",
            Self::Liquid => "vmo_liquid",
            Self::Containment => "vmo_bih_containment",
            Self::RawM2(_) => "vmo_raw_m2",
            Self::RawWmo(_) => "vmo_raw_wmo",
            Self::Bih(_) => "vmo_bih",
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        out.write_str(self.code())
    }
}
impl std::error::Error for Error {}
impl From<bih::Error> for Error {
    fn from(value: bih::Error) -> Self {
        Self::Bih(value)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    pub groups: usize,
    pub vertices: usize,
    pub triangles: usize,
    pub liquid_groups: usize,
    pub bytes: usize,
    pub resident_bytes: usize,
    pub peak_bytes: usize,
    pub build_work: u64,
}
/// Owns one complete validated serialized model; no geometry Debug output.
pub struct Model {
    bytes: Vec<u8>,
    stats: Stats,
}
impl Model {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
    pub const fn stats(&self) -> Stats {
        self.stats
    }
}
pub fn convert_m2(raw: &crate::RawM2<'_>, limits: Limits) -> Result<Model, Error> {
    convert::m2(raw, limits)
}
pub fn convert_wmo(raw: &crate::wmo::raw::RawWmo<'_>, limits: Limits) -> Result<Model, Error> {
    convert::wmo(raw, limits)
}

/// Geometry/liquid bytes are borrowed; group descriptors own a bounded Vec.
pub struct View<'a> {
    bytes: &'a [u8],
    flags: u32,
    root: u32,
    groups: Vec<Group<'a>>,
    group_bih: Option<bih::View<'a>>,
    stats: Stats,
}
impl<'a> View<'a> {
    pub fn parse(bytes: &'a [u8], limits: Limits) -> Result<Self, Error> {
        read::parse(bytes, limits)
    }
    pub const fn bytes(&self) -> &'a [u8] {
        self.bytes
    }
    pub const fn flags(&self) -> u32 {
        self.flags
    }
    pub const fn root_wmo_id(&self) -> u32 {
        self.root
    }
    pub fn groups(&self) -> &[Group<'a>] {
        &self.groups
    }
    pub fn group_bih(&self) -> Option<&bih::View<'a>> {
        self.group_bih.as_ref()
    }
    pub const fn stats(&self) -> Stats {
        self.stats
    }
}
#[derive(Clone, Copy)]
pub struct Group<'a> {
    bounds: [u32; 6],
    flags: u32,
    id: u32,
    vertices: &'a [u8],
    triangles: &'a [u8],
    mesh_bih: Option<bih::View<'a>>,
    liquid: Option<Liquid<'a>>,
}
impl<'a> Group<'a> {
    pub const fn bounds_bits(&self) -> [u32; 6] {
        self.bounds
    }
    pub const fn flags(&self) -> u32 {
        self.flags
    }
    pub const fn group_wmo_id(&self) -> u32 {
        self.id
    }
    pub fn vertex_count(&self) -> usize {
        self.vertices.len() / 12
    }
    pub fn triangle_count(&self) -> usize {
        self.triangles.len() / 12
    }
    pub fn vertex_bits(&self, index: usize) -> Option<[u32; 3]> {
        triple(self.vertices, index).ok()
    }
    pub fn triangle(&self, index: usize) -> Option<[u32; 3]> {
        triple(self.triangles, index).ok()
    }
    pub fn mesh_bih(&self) -> Option<&bih::View<'a>> {
        self.mesh_bih.as_ref()
    }
    pub const fn liquid(&self) -> Option<Liquid<'a>> {
        self.liquid
    }
}
#[derive(Clone, Copy)]
pub struct Liquid<'a> {
    dimensions: [u32; 2],
    corner: [u32; 3],
    type_id: u32,
    heights: &'a [u8],
    tiles: &'a [u8],
}
impl<'a> Liquid<'a> {
    pub const fn dimensions(&self) -> [u32; 2] {
        self.dimensions
    }
    pub const fn corner_bits(&self) -> [u32; 3] {
        self.corner
    }
    pub const fn type_id(&self) -> u32 {
        self.type_id
    }
    pub fn height_count(&self) -> usize {
        self.heights.len() / 4
    }
    pub fn height_bits(&self, index: usize) -> Option<u32> {
        word(self.heights, index.checked_mul(4)?).ok()
    }
    pub const fn tiles(&self) -> &'a [u8] {
        self.tiles
    }
}

fn add(a: usize, b: usize) -> Result<usize, Error> {
    a.checked_add(b).ok_or(Error::Limit)
}
fn mul(a: usize, b: usize) -> Result<usize, Error> {
    a.checked_mul(b).ok_or(Error::Limit)
}
fn word(bytes: &[u8], at: usize) -> Result<u32, Error> {
    Ok(u32::from_le_bytes(
        bytes
            .get(at..add(at, 4)?)
            .ok_or(Error::Framing)?
            .try_into()
            .map_err(|_| Error::Framing)?,
    ))
}
fn triple(bytes: &[u8], index: usize) -> Result<[u32; 3], Error> {
    let at = mul(index, 12)?;
    Ok([
        word(bytes, at)?,
        word(bytes, add(at, 4)?)?,
        word(bytes, add(at, 8)?)?,
    ])
}
fn finite(bits: u32) -> Result<(), Error> {
    if f32::from_bits(bits).is_finite() {
        Ok(())
    } else {
        Err(Error::NonFinite)
    }
}
fn bounds(bits: [u32; 6]) -> Result<bih::Bounds, Error> {
    bih::Bounds::from_bits(bits).map_err(Error::Bih)
}
fn triangle_bounds(points: [[u32; 3]; 3]) -> Result<bih::Bounds, Error> {
    let mut output = [0; 6];
    for axis in 0..3 {
        let mut low = points[0][axis];
        let mut high = low;
        for point in &points[1..] {
            // G3D p1.min(p2).min(p3): NEW argument wins equal signed zero.
            let new = point[axis];
            if f32::from_bits(new) <= f32::from_bits(low) {
                low = new;
            }
            if f32::from_bits(new) >= f32::from_bits(high) {
                high = new;
            }
        }
        output[axis] = low;
        output[axis + 3] = high;
    }
    bounds(output)
}
struct Memory {
    live: usize,
    peak: usize,
    cap: usize,
}
impl Memory {
    fn new(cap: usize) -> Self {
        Self {
            live: 0,
            peak: 0,
            cap,
        }
    }
    fn observe(&mut self, temporary: usize) -> Result<(), Error> {
        let total = add(self.live, temporary)?;
        if total > self.cap {
            return Err(Error::Limit);
        }
        self.peak = self.peak.max(total);
        Ok(())
    }
    fn reserve<T>(&mut self, v: &mut Vec<T>, capacity: usize) -> Result<(), Error> {
        if capacity <= v.capacity() {
            return Ok(());
        }
        self.observe(mul(capacity, size_of::<T>())?)?;
        let old = mul(v.capacity(), size_of::<T>())?;
        v.try_reserve_exact(capacity - v.len())
            .map_err(|_| Error::Allocation)?;
        let new = mul(v.capacity(), size_of::<T>())?;
        self.observe(new)?;
        self.live = self.live - old + new;
        Ok(())
    }
    fn release<T>(&mut self, v: Vec<T>) {
        self.live -= v.capacity() * size_of::<T>();
    }
}
