// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original bounded WMO raw writer and borrowed reader.
// Facts at ORACLE_COMMIT: wmo.cpp:175-192,369-541;
// vmapexport.cpp:255-311; TileAssembler.cpp:475-594; WorldModel.cpp:99-112.
//! Pure default-small-mode raw WMO conversion. No acquisition or doodad outcome
//! is inferred: `None` means no raw collision file, even if liquids/doodads exist.
//! The strict reader checks this format, not assembled VMAP/runtime semantics.

use super::{
    Error, Group, GroupInput, Limits, Root, bits3, bounds, finite,
    liquid::{self, Liquid, LiquidLookup},
    resolve_contexts, word,
};

pub const MAX_RAW_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_MEMORY_BYTES: usize = 256 * 1024 * 1024;

/// Only reductions are admitted. Memory covers live context/plan capacities,
/// each geometry projection including scratch, and the single output allocation.
/// Borrowed sources, caller-owned lookups and allocator overhead are excluded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConversionLimits {
    pub source: Limits,
    pub max_raw_bytes: usize,
    pub max_memory_bytes: usize,
}
impl Default for ConversionLimits {
    fn default() -> Self {
        Self {
            source: Limits::default(),
            max_raw_bytes: MAX_RAW_BYTES,
            max_memory_bytes: MAX_MEMORY_BYTES,
        }
    }
}
impl ConversionLimits {
    fn validate(self) -> Result<(), Error> {
        self.source.validate()?;
        if self.max_raw_bytes > MAX_RAW_BYTES || self.max_memory_bytes > MAX_MEMORY_BYTES {
            return Err(Error::InvalidLimits);
        }
        Ok(())
    }
    fn projection(self, resident: usize) -> Result<Limits, Error> {
        Ok(Limits {
            max_projection_bytes: self.source.max_projection_bytes.min(
                self.max_memory_bytes
                    .checked_sub(resident)
                    .ok_or(Error::Limit)?,
            ),
            ..self.source
        })
    }
}

/// Complete owned raw file. Intentionally no Debug implementation for bytes.
pub struct ConvertedWmo {
    bytes: Vec<u8>,
    groups: u32,
    triangles: u32,
}
impl ConvertedWmo {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
    pub const fn group_count(&self) -> u32 {
        self.groups
    }
    pub const fn triangle_count(&self) -> u32 {
        self.triangles
    }
}

struct Plan<'a> {
    liquid: Liquid<'a>,
    geometry_bytes: usize,
    emitted: bool,
}

/// Convert the complete detailed GFID inventory, preserving order and aliases.
/// Geometry is projected twice (a pure operation), but liquid lookup decisions
/// are made once per source group, including skipped groups, and retained.
/// Source limits are revalidated even if callers parsed with larger bounds.
pub fn convert(
    root: &Root<'_>,
    groups: &[GroupInput<'_, '_>],
    lookup: &(impl LiquidLookup + ?Sized),
    limits: ConversionLimits,
) -> Result<Option<ConvertedWmo>, Error> {
    limits.validate()?;
    if groups.len() > limits.source.max_groups {
        return Err(Error::Limit);
    }
    Root::parse(root.bytes(), limits.source)?;
    for input in groups {
        Group::parse(input.group.bytes(), limits.source)?;
    }
    let contexts = resolve_contexts(root, groups, limits.projection(0)?)?;
    let mut resident = capacity_bytes(&contexts)?;
    let mut plans = allocate::<Plan<'_>>(contexts.len(), resident, limits.max_memory_bytes)?;
    resident = add(resident, capacity_bytes(&plans)?)?;
    let mut triangles = 0u32;
    let mut emitted = 0u32;
    let mut geometry_subtotal = 24usize;
    for context in &contexts {
        let liquid = liquid::project(root, context.group(), lookup)?;
        let geometry = context.project_geometry(limits.projection(resident)?)?;
        let geometry_bytes = if let Some(geometry) = &geometry {
            triangles = triangles
                .checked_add(as_u32(geometry.triangle_count())?)
                .ok_or(Error::Bounds)?;
            emitted = emitted.checked_add(1).ok_or(Error::Bounds)?;
            // 36-byte fixed group header plus three 12-byte array headers.
            add(
                72,
                add(
                    mul(geometry.batches().len(), 4)?,
                    add(
                        mul(geometry.indices().len(), 4)?,
                        mul(geometry.vertices_bits().len(), 12)?,
                    )?,
                )?,
            )?
        } else {
            0
        };
        geometry_subtotal = add(geometry_subtotal, geometry_bytes)?;
        // No file exists while all groups have zero collision triangles. Once
        // one does, geometry alone can prove the output cap impossible; stop
        // before projecting or consulting lookups for subsequent groups.
        if triangles != 0 && geometry_subtotal > limits.max_raw_bytes {
            return Err(Error::Limit);
        }
        plans.push(Plan {
            liquid,
            geometry_bytes,
            emitted: geometry.is_some(),
        });
    }
    // Native removes the raw file whenever no collision triangles were written.
    // Doodad-dependent extraction success is a separate, unimplemented decision.
    if triangles == 0 {
        return Ok(None);
    }
    let mut size = 24usize;
    for plan in &plans {
        if plan.emitted {
            size = add(size, add(plan.geometry_bytes, liquid_bytes(plan.liquid)?)?)?;
            if size > limits.max_raw_bytes {
                return Err(Error::Limit);
            }
        }
    }
    let mut output = allocate::<u8>(size, resident, limits.max_memory_bytes)?;
    resident = add(resident, capacity_bytes(&output)?)?;
    output.extend_from_slice(crate::RAW_MAGIC);
    for value in [triangles, emitted, root.header().root_wmo_id, 0] {
        push(&mut output, value);
    }
    for (context, plan) in contexts.iter().zip(&plans) {
        if !plan.emitted {
            continue;
        }
        let geometry = context
            .project_geometry(limits.projection(resident)?)?
            .ok_or(Error::Shape)?;
        for value in [geometry.flags(), geometry.group_wmo_id()] {
            push(&mut output, value);
        }
        for value in geometry.bounds_bits() {
            push(&mut output, value);
        }
        push(&mut output, plan.liquid.flags());
        array_header(&mut output, b"GRP ", geometry.batches().len(), 4)?;
        for &value in geometry.batches() {
            push(&mut output, value);
        }
        // Pinned anomaly: declared u16 width, physically u32 indices.
        array_header(&mut output, b"INDX", geometry.indices().len(), 2)?;
        for &value in geometry.indices() {
            push(&mut output, value);
        }
        array_header(&mut output, b"VERT", geometry.vertices_bits().len(), 12)?;
        for vertex in geometry.vertices_bits() {
            for &value in vertex {
                push(&mut output, value);
            }
        }
        write_liquid(&mut output, plan.liquid)?;
    }
    if output.len() != size {
        return Err(Error::Shape);
    }
    RawWmo::parse(&output, limits)?;
    Ok(Some(ConvertedWmo {
        bytes: output,
        groups: emitted,
        triangles,
    }))
}

// Source MLIQ allows more dimensions than the raw assembler can safely use.
// TileAssembler writes heights into storage inferred only from tile dimensions;
// require its full capacity, with no uninitialized suffix or overflow. Its
// fread(ptr, 0, 1) flags read also rejects a zero tile product. The source view
// keeps its broader domain; ignored/header-only liquid bypasses this gate.
fn consumer_shape(dimensions: [u32; 4], limits: Limits) -> Result<(usize, usize), Error> {
    if dimensions.iter().any(|&n| n > i32::MAX as u32) {
        return Err(Error::Shape);
    }
    let [vx, vy, tx, ty] = dimensions.map(|n| n as usize);
    let heights = mul(vx, vy)?;
    let tiles = mul(tx, ty)?;
    if tiles == 0 || heights != mul(add(tx, 1)?, add(ty, 1)?)? {
        return Err(Error::Shape);
    }
    if heights > limits.max_vertices || tiles > limits.max_metadata_records {
        return Err(Error::Limit);
    }
    Ok((heights, tiles))
}
fn liquid_bytes(liquid: Liquid<'_>) -> Result<usize, Error> {
    if liquid.flags() == 0 {
        return Ok(0);
    }
    let Some(view) = liquid.view() else {
        return Ok(12);
    };
    let (heights, tiles) = consumer_shape(view.dimensions(), Limits::default())?;
    add(42, add(mul(heights, 4)?, tiles)?)
}
fn write_liquid(output: &mut Vec<u8>, liquid: Liquid<'_>) -> Result<(), Error> {
    let size = liquid_bytes(liquid)?;
    if size == 0 {
        return Ok(());
    }
    output.extend_from_slice(b"LIQU");
    push(output, as_u32(size - 8)?);
    push(output, liquid.type_id());
    if let Some(view) = liquid.view() {
        // Exact source 30-byte packed header; the 8-byte source vertices are
        // reduced to their height words, followed by untouched tile bytes.
        output.extend_from_slice(&view.bytes()[..30]);
        for index in 0..view.vertex_count() {
            push(output, view.vertex(index).ok_or(Error::Shape)?.2);
        }
        output.extend_from_slice(view.tiles());
    }
    Ok(())
}

/// Borrowed strict WMO raw04E representation, fully checked before exposure.
/// Flags, totals, exact EOF and declared sizes (including INDX's anomaly) are
/// validated. It excludes M2 and unsafe MLIQ consumer dimensions. It is not an
/// assembled-world reader, and accepts u32 indices beyond the M2 u16 domain.
pub struct RawWmo<'a> {
    bytes: &'a [u8],
    groups: u32,
    triangles: u32,
    root_id: u32,
    limits: Limits,
}
impl<'a> RawWmo<'a> {
    pub fn parse(bytes: &'a [u8], limits: ConversionLimits) -> Result<Self, Error> {
        limits.validate()?;
        if bytes.len() > limits.max_raw_bytes {
            return Err(Error::Limit);
        }
        let mut cursor = Cursor { bytes, at: 0 };
        if cursor.take(8)? != crate::RAW_MAGIC {
            return Err(Error::Shape);
        }
        let triangles = cursor.word()?;
        let groups = cursor.word()?;
        let root_id = cursor.word()?;
        if cursor.word()? != 0 || triangles == 0 || groups == 0 {
            return Err(Error::Shape);
        }
        if groups as usize > limits.source.max_groups {
            return Err(Error::Limit);
        }
        let mut observed = 0u32;
        for _ in 0..groups {
            let group = read_group(&mut cursor, limits.source)?;
            observed = observed
                .checked_add(as_u32(group.triangle_count())?)
                .ok_or(Error::Bounds)?;
        }
        if observed != triangles || cursor.at != bytes.len() {
            return Err(Error::Shape);
        }
        Ok(Self {
            bytes,
            groups,
            triangles,
            root_id,
            limits: limits.source,
        })
    }
    pub const fn bytes(&self) -> &'a [u8] {
        self.bytes
    }
    pub const fn group_count(&self) -> u32 {
        self.groups
    }
    pub const fn triangle_count(&self) -> u32 {
        self.triangles
    }
    pub const fn root_wmo_id(&self) -> u32 {
        self.root_id
    }
    pub fn groups(&self) -> RawGroups<'a> {
        RawGroups {
            cursor: Cursor {
                bytes: self.bytes,
                at: 24,
            },
            remaining: self.groups,
            limits: self.limits,
        }
    }
}

pub struct RawGroups<'a> {
    cursor: Cursor<'a>,
    remaining: u32,
    limits: Limits,
}
impl<'a> Iterator for RawGroups<'a> {
    type Item = RawGroup<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }
        let group = read_group(&mut self.cursor, self.limits).ok()?;
        self.remaining -= 1;
        Some(group)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining as usize, Some(self.remaining as usize))
    }
}
impl ExactSizeIterator for RawGroups<'_> {}

pub struct RawGroup<'a> {
    flags: u32,
    id: u32,
    bounds: [u32; 6],
    liquid_flags: u32,
    batches: &'a [u8],
    indices: &'a [u8],
    vertices: &'a [u8],
    liquid: Option<RawLiquid<'a>>,
}
impl<'a> RawGroup<'a> {
    pub const fn flags(&self) -> u32 {
        self.flags
    }
    pub const fn group_wmo_id(&self) -> u32 {
        self.id
    }
    pub const fn bounds_bits(&self) -> [u32; 6] {
        self.bounds
    }
    pub const fn liquid_flags(&self) -> u32 {
        self.liquid_flags
    }
    pub fn batch_count(&self) -> usize {
        self.batches.len() / 4
    }
    pub fn batch(&self, index: usize) -> Option<u32> {
        word(self.batches, index.checked_mul(4)?).ok()
    }
    pub fn index_count(&self) -> usize {
        self.indices.len() / 4
    }
    pub fn triangle_count(&self) -> usize {
        self.index_count() / 3
    }
    pub fn index(&self, index: usize) -> Option<u32> {
        word(self.indices, index.checked_mul(4)?).ok()
    }
    pub fn vertex_count(&self) -> usize {
        self.vertices.len() / 12
    }
    pub fn vertex_bits(&self, index: usize) -> Option<[u32; 3]> {
        bits3(self.vertices, index.checked_mul(12)?).ok()
    }
    pub const fn liquid(&self) -> Option<RawLiquid<'a>> {
        self.liquid
    }
}

#[derive(Clone, Copy)]
pub struct RawLiquid<'a> {
    type_id: u32,
    header: Option<&'a [u8]>,
    heights: &'a [u8],
    tiles: &'a [u8],
}
impl RawLiquid<'_> {
    pub const fn type_id(&self) -> u32 {
        self.type_id
    }
    pub fn dimensions(&self) -> Option<[u32; 4]> {
        let h = self.header?;
        Some([
            word(h, 0).ok()?,
            word(h, 4).ok()?,
            word(h, 8).ok()?,
            word(h, 12).ok()?,
        ])
    }
    pub fn position_bits(&self) -> Option<[u32; 3]> {
        bits3(self.header?, 16).ok()
    }
    pub fn material(&self) -> Option<i16> {
        super::short(self.header?, 28).ok().map(|v| v as i16)
    }
    pub fn height_count(&self) -> usize {
        self.heights.len() / 4
    }
    pub fn height_bits(&self, index: usize) -> Option<u32> {
        word(self.heights, index.checked_mul(4)?).ok()
    }
    pub const fn tiles(&self) -> &[u8] {
        self.tiles
    }
}

fn read_group<'a>(cursor: &mut Cursor<'a>, limits: Limits) -> Result<RawGroup<'a>, Error> {
    let flags = cursor.word()?;
    let id = cursor.word()?;
    let bounds = bounds(cursor.take(24)?, 0)?;
    let liquid_flags = cursor.word()?;
    if liquid_flags > 3 {
        return Err(Error::Shape);
    }
    let batches = cursor.array(b"GRP ", 4, 4, limits.max_metadata_records)?;
    for value in batches.as_chunks::<4>().0 {
        if word(value, 0)? > u16::MAX as u32 {
            return Err(Error::Shape);
        }
    }
    let indices = cursor.array(b"INDX", 2, 4, limits.max_indices)?;
    if !(indices.len() / 4).is_multiple_of(3) {
        return Err(Error::Shape);
    }
    let vertices = cursor.array(b"VERT", 12, 12, limits.max_vertices)?;
    if indices.is_empty() != vertices.is_empty() {
        return Err(Error::Shape);
    }
    for vertex in vertices.as_chunks::<12>().0 {
        bits3(vertex, 0)?;
    }
    for value in indices.as_chunks::<4>().0 {
        if word(value, 0)? as usize >= vertices.len() / 12 {
            return Err(Error::Index);
        }
    }
    let liquid = if liquid_flags == 0 {
        None
    } else {
        if cursor.take(4)? != b"LIQU" {
            return Err(Error::Shape);
        }
        let declared = cursor.word()? as usize;
        let start = cursor.at;
        let type_id = cursor.word()?;
        let (header, heights, tiles) = if liquid_flags & 1 != 0 {
            let header = cursor.take(30)?;
            let dimensions = [
                word(header, 0)?,
                word(header, 4)?,
                word(header, 8)?,
                word(header, 12)?,
            ];
            let (heights, tiles) = consumer_shape(dimensions, limits)?;
            bits3(header, 16)?;
            let heights = cursor.take(mul(heights, 4)?)?;
            for value in heights.as_chunks::<4>().0 {
                finite(word(value, 0)?)?;
            }
            (Some(header), heights, cursor.take(tiles)?)
        } else {
            (None, &[][..], &[][..])
        };
        if cursor.at - start != declared {
            return Err(Error::Shape);
        }
        Some(RawLiquid {
            type_id,
            header,
            heights,
            tiles,
        })
    };
    Ok(RawGroup {
        flags,
        id,
        bounds,
        liquid_flags,
        batches,
        indices,
        vertices,
        liquid,
    })
}

struct Cursor<'a> {
    bytes: &'a [u8],
    at: usize,
}
impl<'a> Cursor<'a> {
    fn take(&mut self, size: usize) -> Result<&'a [u8], Error> {
        let end = add(self.at, size)?;
        let bytes = self.bytes.get(self.at..end).ok_or(Error::Bounds)?;
        self.at = end;
        Ok(bytes)
    }
    fn word(&mut self) -> Result<u32, Error> {
        word(self.take(4)?, 0)
    }
    fn array(
        &mut self,
        tag: &[u8; 4],
        declared_width: usize,
        physical_width: usize,
        cap: usize,
    ) -> Result<&'a [u8], Error> {
        if self.take(4)? != tag {
            return Err(Error::Shape);
        }
        let declared = self.word()? as usize;
        let count = self.word()? as usize;
        if count > cap {
            return Err(Error::Limit);
        }
        if declared != add(4, mul(count, declared_width)?)? {
            return Err(Error::Shape);
        }
        self.take(mul(count, physical_width)?)
    }
}
fn array_header(
    output: &mut Vec<u8>,
    tag: &[u8; 4],
    count: usize,
    width: usize,
) -> Result<(), Error> {
    output.extend_from_slice(tag);
    push(output, as_u32(add(4, mul(count, width)?)?)?);
    push(output, as_u32(count)?);
    Ok(())
}
fn push(output: &mut Vec<u8>, value: u32) {
    output.extend_from_slice(&value.to_le_bytes());
}
fn add(a: usize, b: usize) -> Result<usize, Error> {
    a.checked_add(b).ok_or(Error::Bounds)
}
fn mul(a: usize, b: usize) -> Result<usize, Error> {
    a.checked_mul(b).ok_or(Error::Bounds)
}
fn as_u32(value: usize) -> Result<u32, Error> {
    u32::try_from(value).map_err(|_| Error::Bounds)
}
fn capacity_bytes<T>(values: &Vec<T>) -> Result<usize, Error> {
    mul(values.capacity(), size_of::<T>())
}
fn allocate<T>(count: usize, resident: usize, limit: usize) -> Result<Vec<T>, Error> {
    if add(resident, mul(count, size_of::<T>())?)? > limit {
        return Err(Error::Limit);
    }
    let mut values = Vec::new();
    values
        .try_reserve_exact(count)
        .map_err(|_| Error::Allocation)?;
    if add(resident, capacity_bytes(&values)?)? > limit {
        return Err(Error::Limit);
    }
    Ok(values)
}

#[cfg(test)]
mod tests;
