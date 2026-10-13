// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: pure complete VMO construction from validated RAW views.
use super::*;
use crate::wmo::raw::{RawGroup, RawLiquid};

enum Input<'a, 'b> {
    M2(&'b crate::RawM2<'a>),
    Wmo(RawGroup<'a>),
}
impl Input<'_, '_> {
    fn bounds(&self) -> [u32; 6] {
        match self {
            Self::M2(v) => v.bounds_bits(),
            Self::Wmo(v) => v.bounds_bits(),
        }
    }
    fn vertices(&self) -> usize {
        match self {
            Self::M2(v) => v.vertex_count(),
            Self::Wmo(v) => v.vertex_count(),
        }
    }
    fn triangles(&self) -> usize {
        match self {
            Self::M2(v) => v.triangle_count(),
            Self::Wmo(v) => v.triangle_count(),
        }
    }
    fn vertex(&self, i: usize) -> Result<[u32; 3], Error> {
        match self {
            Self::M2(v) => v.vertex_bits(i),
            Self::Wmo(v) => v.vertex_bits(i),
        }
        .ok_or(Error::Index)
    }
    fn triangle(&self, i: usize) -> Result<[u32; 3], Error> {
        let base = mul(i, 3)?;
        let get = |i| {
            match self {
                Self::M2(v) => v.index(i),
                Self::Wmo(v) => v.index(i),
            }
            .ok_or(Error::Index)
        };
        Ok([get(base)?, get(base + 1)?, get(base + 2)?])
    }
    fn flags(&self) -> u32 {
        match self {
            Self::M2(_) => 0,
            Self::Wmo(v) => v.flags(),
        }
    }
    fn id(&self) -> u32 {
        match self {
            Self::M2(_) => 0,
            Self::Wmo(v) => v.group_wmo_id(),
        }
    }
    fn liquid(&self) -> Option<RawLiquid<'_>> {
        match self {
            Self::M2(_) => None,
            Self::Wmo(v) => v.liquid(),
        }
    }
}

pub(super) fn m2(raw: &crate::RawM2<'_>, limits: Limits) -> Result<Model, Error> {
    limits.validate()?;
    let raw = crate::RawM2::parse(
        raw.bytes(),
        crate::Limits {
            max_input_bytes: limits.max_raw_bytes,
            max_vertices: limits.max_vertices,
            max_indices: mul(limits.max_triangles, 3)?,
        },
    )
    .map_err(Error::RawM2)?;
    convert(std::iter::once(Input::M2(&raw)), 1, 1, 0, limits)
}
pub(super) fn wmo(raw: &crate::wmo::raw::RawWmo<'_>, limits: Limits) -> Result<Model, Error> {
    limits.validate()?;
    let raw = crate::wmo::raw::RawWmo::parse(
        raw.bytes(),
        crate::wmo::raw::ConversionLimits {
            source: crate::wmo::Limits {
                max_input_bytes: limits.max_raw_bytes,
                max_groups: limits.max_groups,
                max_vertices: limits.max_vertices,
                max_indices: mul(limits.max_triangles, 3)?,
                ..crate::wmo::Limits::default()
            },
            max_raw_bytes: limits.max_raw_bytes,
            ..crate::wmo::raw::ConversionLimits::default()
        },
    )
    .map_err(Error::RawWmo)?;
    convert(
        raw.groups().map(Input::Wmo),
        raw.group_count() as usize,
        0,
        raw.root_wmo_id(),
        limits,
    )
}

struct Writer {
    bytes: Vec<u8>,
    memory: Memory,
    limits: Limits,
    work: u64,
}
impl Writer {
    fn ensure(&mut self, size: usize) -> Result<(), Error> {
        let needed = add(self.bytes.len(), size)?;
        if needed > self.limits.max_bytes {
            return Err(Error::Limit);
        }
        if needed > self.bytes.capacity() {
            let capacity = needed
                .max(self.bytes.capacity().saturating_mul(2))
                .min(self.limits.max_bytes);
            self.memory.reserve(&mut self.bytes, capacity)?;
            if self.bytes.capacity() > self.limits.max_bytes {
                return Err(Error::Limit);
            }
        }
        Ok(())
    }
    fn push(&mut self, value: u32) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }
    fn array(&mut self, tag: &[u8; 4], count: usize) -> Result<(), Error> {
        self.bytes.extend_from_slice(tag);
        self.push(u32::try_from(add(4, mul(count, 12)?)?).map_err(|_| Error::Limit)?);
        self.push(u32::try_from(count).map_err(|_| Error::Limit)?);
        Ok(())
    }
    fn tree(&mut self, primitive_bounds: &[bih::Bounds], leaf: u32) -> Result<bih::Bih, Error> {
        let tree = bih::build(
            primitive_bounds,
            leaf,
            self.limits.tree(self.memory.live, self.work)?,
        )?;
        self.memory.observe(tree.stats().peak_bytes)?;
        self.memory.live = add(self.memory.live, tree.stats().resident_bytes)?;
        self.work = self
            .work
            .checked_add(tree.stats().build_work)
            .ok_or(Error::Limit)?;
        Ok(tree)
    }
    fn drop_tree(&mut self, tree: bih::Bih) {
        self.memory.live -= tree.stats().resident_bytes;
    }
}

fn convert<'a, 'b>(
    inputs: impl Iterator<Item = Input<'a, 'b>>,
    count: usize,
    flags: u32,
    root: u32,
    limits: Limits,
) -> Result<Model, Error>
where
    'a: 'b,
{
    if count > limits.max_groups {
        return Err(Error::Limit);
    }
    let mut writer = Writer {
        bytes: Vec::new(),
        memory: Memory::new(limits.max_memory_bytes),
        limits,
        work: 0,
    };
    let mut group_bounds = Vec::new();
    writer.memory.reserve(&mut group_bounds, count)?;
    writer.ensure(if count == 0 { 24 } else { 32 })?;
    writer.bytes.extend_from_slice(MAGIC);
    writer.bytes.extend_from_slice(b"WMOD");
    for value in [8, flags, root] {
        writer.push(value);
    }
    if count != 0 {
        writer.bytes.extend_from_slice(b"GMOD");
        writer.push(count as u32);
    }
    for input in inputs {
        if group_bounds.len() == count {
            return Err(Error::Framing);
        }
        let bound = bounds(input.bounds())?;
        group_bounds.push(bound);
        // Recheck per-component reductions even for liquid later omitted by VERT0.
        let liquid_size = liquid_size(input.liquid(), limits)?;
        if input.vertices() > limits.max_vertices || input.triangles() > limits.max_triangles {
            return Err(Error::Limit);
        }
        if input.vertices() == 0 {
            writer.ensure(44)?;
            for value in input.bounds() {
                writer.push(value);
            }
            writer.push(input.flags());
            writer.push(input.id());
            writer.array(b"VERT", 0)?;
            continue;
        }
        let mut triangle_boxes = Vec::new();
        writer
            .memory
            .reserve(&mut triangle_boxes, input.triangles())?;
        for index in 0..input.triangles() {
            let indices = input.triangle(index)?;
            triangle_boxes.push(triangle_bounds([
                input.vertex(indices[0] as usize)?,
                input.vertex(indices[1] as usize)?,
                input.vertex(indices[2] as usize)?,
            ])?);
        }
        let tree = writer.tree(&triangle_boxes, 3)?;
        writer.memory.release(triangle_boxes);
        let size = add(
            68,
            add(
                mul(input.vertices(), 12)?,
                add(
                    mul(input.triangles(), 12)?,
                    add(tree.bytes().len(), liquid_size)?,
                )?,
            )?,
        )?;
        // 32 fixed, VERT12, TRIM12, MBIH4, LIQU8 = 68 bytes.
        writer.ensure(size)?;
        for value in input.bounds() {
            writer.push(value);
        }
        writer.push(input.flags());
        writer.push(input.id());
        writer.array(b"VERT", input.vertices())?;
        for index in 0..input.vertices() {
            for value in input.vertex(index)? {
                writer.push(value);
            }
        }
        writer.array(b"TRIM", input.triangles())?;
        for index in 0..input.triangles() {
            for value in input.triangle(index)? {
                writer.push(value);
            }
        }
        writer.bytes.extend_from_slice(b"MBIH");
        writer.bytes.extend_from_slice(tree.bytes());
        writer.drop_tree(tree);
        writer.bytes.extend_from_slice(b"LIQU");
        writer.push(liquid_size as u32);
        if let Some(liquid) = input.liquid() {
            write_liquid(&mut writer, liquid, input.bounds()[5])?;
        }
    }
    if group_bounds.len() != count {
        return Err(Error::Framing);
    }
    if count != 0 {
        let tree = writer.tree(&group_bounds, 1)?;
        writer.ensure(add(4, tree.bytes().len())?)?;
        writer.bytes.extend_from_slice(b"GBIH");
        writer.bytes.extend_from_slice(tree.bytes());
        writer.drop_tree(tree);
    }
    writer.memory.release(group_bounds);
    let view = View::parse(
        &writer.bytes,
        Limits {
            max_memory_bytes: limits
                .max_memory_bytes
                .checked_sub(writer.memory.live)
                .ok_or(Error::Limit)?,
            ..limits
        },
    )?;
    writer.memory.observe(view.stats().peak_bytes)?;
    let stats = Stats {
        resident_bytes: writer.bytes.capacity(),
        peak_bytes: writer.memory.peak,
        build_work: writer.work,
        ..view.stats()
    };
    drop(view);
    Ok(Model {
        bytes: writer.bytes,
        stats,
    })
}

fn liquid_size(liquid: Option<RawLiquid<'_>>, limits: Limits) -> Result<usize, Error> {
    let Some(liquid) = liquid else { return Ok(0) };
    if liquid.dimensions().is_none() {
        if limits.max_liquid_heights == 0 {
            return Err(Error::Limit);
        }
        return Ok(28);
    }
    if liquid.height_count() > limits.max_liquid_heights
        || liquid.tiles().len() > limits.max_liquid_tiles
    {
        return Err(Error::Limit);
    }
    add(
        24,
        add(mul(liquid.height_count(), 4)?, liquid.tiles().len())?,
    )
}
fn write_liquid(writer: &mut Writer, liquid: RawLiquid<'_>, flat_height: u32) -> Result<(), Error> {
    let dims = liquid.dimensions();
    for value in dims.map(|v| [v[2], v[3]]).unwrap_or([0, 0]) {
        writer.push(value);
    }
    for value in liquid.position_bits().unwrap_or([0; 3]) {
        writer.push(value);
    }
    writer.push(liquid.type_id());
    if dims.is_none() {
        writer.push(flat_height);
    } else {
        for index in 0..liquid.height_count() {
            writer.push(liquid.height_bits(index).ok_or(Error::Liquid)?);
        }
        writer.bytes.extend_from_slice(liquid.tiles());
    }
    Ok(())
}
