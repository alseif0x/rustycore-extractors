// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original immutable offset descriptors; no unsafe/selfref.
use super::*;
use std::ops::Range;

#[cfg(test)]
mod tests;

struct TreeLayout {
    bytes: Range<usize>,
    layout: bih::Layout,
}
impl TreeLayout {
    fn new(bytes: &[u8], tree: &bih::View<'_>) -> Result<Self, Error> {
        Ok(Self {
            bytes: extent(bytes, tree.bytes())?,
            layout: bih::Layout::from_view(tree),
        })
    }
    fn bind<'a>(&self, bytes: &'a [u8]) -> bih::View<'a> {
        self.layout.bind(&bytes[self.bytes.clone()])
    }
}
struct LiquidLayout {
    dimensions: [u32; 2],
    corner: [u32; 3],
    type_id: u32,
    heights: Range<usize>,
    tiles: Range<usize>,
}
impl LiquidLayout {
    fn new(bytes: &[u8], liquid: Liquid<'_>) -> Result<Self, Error> {
        Ok(Self {
            dimensions: liquid.dimensions,
            corner: liquid.corner,
            type_id: liquid.type_id,
            heights: extent(bytes, liquid.heights)?,
            tiles: extent(bytes, liquid.tiles)?,
        })
    }
    fn bind<'a>(&self, bytes: &'a [u8]) -> Liquid<'a> {
        Liquid {
            dimensions: self.dimensions,
            corner: self.corner,
            type_id: self.type_id,
            heights: &bytes[self.heights.clone()],
            tiles: &bytes[self.tiles.clone()],
        }
    }
}
struct GroupLayout {
    bounds: [u32; 6],
    flags: u32,
    id: u32,
    vertices: Range<usize>,
    triangles: Range<usize>,
    mesh_bih: Option<TreeLayout>,
    liquid: Option<LiquidLayout>,
}
impl GroupLayout {
    fn new(bytes: &[u8], group: &Group<'_>) -> Result<Self, Error> {
        Ok(Self {
            bounds: group.bounds,
            flags: group.flags,
            id: group.id,
            vertices: extent(bytes, group.vertices)?,
            triangles: extent(bytes, group.triangles)?,
            mesh_bih: group
                .mesh_bih
                .as_ref()
                .map(|v| TreeLayout::new(bytes, v))
                .transpose()?,
            liquid: group
                .liquid
                .map(|v| LiquidLayout::new(bytes, v))
                .transpose()?,
        })
    }
    fn bind<'a>(&self, bytes: &'a [u8]) -> Group<'a> {
        Group {
            bounds: self.bounds,
            flags: self.flags,
            id: self.id,
            vertices: &bytes[self.vertices.clone()],
            triangles: &bytes[self.triangles.clone()],
            mesh_bih: self.mesh_bih.as_ref().map(|v| v.bind(bytes)),
            liquid: self.liquid.as_ref().map(|v| v.bind(bytes)),
        }
    }
}

// All nonempty slices originate in View::parse on this same backing allocation.
// Integer address differences merely record checked offsets; no pointer is
// dereferenced, reconstructed or retained. Empty borrowed sentinels use 0..0.
fn extent(bytes: &[u8], slice: &[u8]) -> Result<Range<usize>, Error> {
    if slice.is_empty() {
        return Ok(0..0);
    }
    let start = (slice.as_ptr() as usize)
        .checked_sub(bytes.as_ptr() as usize)
        .ok_or(Error::Framing)?;
    let end = add(start, slice.len())?;
    if bytes.get(start..end).is_none() {
        return Err(Error::Framing);
    }
    Ok(start..end)
}

/// Immutable complete VMO with prevalidated offset descriptors. Construction
/// runs the existing complete parser once (including geometric BIH checks).
/// Group/tree views are allocation-free and O(1); no model is reparsed per ray.
///
/// The memory cap charges input Vec capacity, parser scratch/descriptors and
/// new owned descriptors during their overlap. Stats' resident includes bytes
/// and owned descriptors; peak includes construction. Bounded stack, allocator
/// overhead and caller-owned data are excluded. No mutable bytes or descriptors
/// are exposed; returned borrows cannot outlive or mutate their owner.
pub struct Owned {
    bytes: Vec<u8>,
    flags: u32,
    root: u32,
    groups: Vec<GroupLayout>,
    group_bih: Option<TreeLayout>,
    stats: Stats,
}
impl Owned {
    pub fn parse(bytes: Vec<u8>, limits: Limits) -> Result<Self, Error> {
        limits.validate()?;
        let byte_capacity = bytes.capacity();
        let available = limits
            .max_memory_bytes
            .checked_sub(byte_capacity)
            .ok_or(Error::Limit)?;
        let view = View::parse(
            &bytes,
            Limits {
                max_memory_bytes: available,
                ..limits
            },
        )?;
        let mut memory = Memory::new(limits.max_memory_bytes);
        memory.live = add(byte_capacity, view.stats.resident_bytes)?;
        memory.observe(0)?;
        memory.peak = memory.peak.max(add(byte_capacity, view.stats.peak_bytes)?);
        let mut groups = Vec::new();
        memory.reserve(&mut groups, view.groups.len())?;
        for group in &view.groups {
            groups.push(GroupLayout::new(&bytes, group)?);
        }
        let group_bih = view
            .group_bih
            .as_ref()
            .map(|v| TreeLayout::new(&bytes, v))
            .transpose()?;
        let flags = view.flags;
        let root = view.root;
        let mut stats = view.stats;
        stats.resident_bytes = add(
            byte_capacity,
            mul(groups.capacity(), size_of::<GroupLayout>())?,
        )?;
        stats.peak_bytes = memory.peak;
        // Drop borrowed descriptor storage before moving the backing Vec. Its
        // retained contents are now represented only by private byte offsets.
        drop(view);
        Ok(Self {
            bytes,
            flags,
            root,
            groups,
            group_bih,
            stats,
        })
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
    pub const fn flags(&self) -> u32 {
        self.flags
    }
    pub const fn root_wmo_id(&self) -> u32 {
        self.root
    }
    pub fn group_count(&self) -> usize {
        self.groups.len()
    }
    pub fn group(&self, index: usize) -> Option<Group<'_>> {
        self.groups.get(index).map(|v| v.bind(&self.bytes))
    }
    pub fn group_bih(&self) -> Option<bih::View<'_>> {
        self.group_bih.as_ref().map(|v| v.bind(&self.bytes))
    }
    pub const fn stats(&self) -> Stats {
        self.stats
    }
    pub const fn resident_bytes(&self) -> usize {
        self.stats.resident_bytes
    }
    pub const fn validation_peak_bytes(&self) -> usize {
        self.stats.peak_bytes
    }
}
