// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11. Facts: pinned wmo.cpp:220-364 and wmo.h:110-166.
use super::{
    Chunks, Error, Limits, bits3, bounds, chunks, exact, finite, once, records, scan, short, word,
};

#[derive(Clone, Copy)]
pub struct GroupHeader {
    pub name_offset: i32,
    pub description_offset: i32,
    pub flags: u32,
    pub bounds_bits: [u32; 6],
    pub portal_start: u16,
    pub portal_count: u16,
    pub batches_a: u16,
    pub batches_b: u16,
    pub batches_c: u32,
    pub fog_indices: u32,
    pub source_liquid_type: u32,
    pub group_wmo_id: u32,
    pub flags2: u32,
    pub parent_or_first_child_split_group: i16,
    pub next_split_child_group: i16,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TriangleMaterial {
    pub flags: u16,
    pub material: u16,
}

/// Complete, structurally checked source MLIQ. No liquid-type mapping or
/// VisualOnly filtering is performed. Dimensions may be zero; their relation
/// is not invented. All present position/height f32 values must be finite.
#[derive(Clone, Copy)]
pub struct LiquidView<'a> {
    bytes: &'a [u8],
    vertices: &'a [u8],
    tiles: &'a [u8],
    dimensions: [u32; 4],
    position_bits: [u32; 3],
    material: i16,
}
impl<'a> LiquidView<'a> {
    fn parse(bytes: &'a [u8], limits: Limits) -> Result<Self, Error> {
        let header = bytes.get(..30).ok_or(Error::Shape)?;
        let dimensions = [
            word(header, 0)?,
            word(header, 4)?,
            word(header, 8)?,
            word(header, 12)?,
        ];
        if dimensions.iter().any(|v| *v > i32::MAX as u32) {
            return Err(Error::Shape);
        }
        let vertex_count = (dimensions[0] as usize)
            .checked_mul(dimensions[1] as usize)
            .ok_or(Error::Bounds)?;
        let tile_count = (dimensions[2] as usize)
            .checked_mul(dimensions[3] as usize)
            .ok_or(Error::Bounds)?;
        if vertex_count > limits.max_vertices || tile_count > limits.max_metadata_records {
            return Err(Error::Limit);
        }
        let tile_start = 30usize
            .checked_add(vertex_count.checked_mul(8).ok_or(Error::Bounds)?)
            .ok_or(Error::Bounds)?;
        exact(
            bytes,
            tile_start.checked_add(tile_count).ok_or(Error::Bounds)?,
        )?;
        let vertices = &bytes[30..tile_start];
        for vertex in vertices.as_chunks::<8>().0 {
            finite(word(vertex, 4)?)?;
        }
        Ok(Self {
            bytes,
            vertices,
            tiles: &bytes[tile_start..],
            dimensions,
            position_bits: bits3(header, 16)?,
            material: short(header, 28)? as i16,
        })
    }
    pub const fn bytes(&self) -> &'a [u8] {
        self.bytes
    }
    /// xverts, yverts, xtiles, ytiles; source fields are nonnegative i32.
    pub const fn dimensions(&self) -> [u32; 4] {
        self.dimensions
    }
    pub const fn position_bits(&self) -> [u32; 3] {
        self.position_bits
    }
    pub const fn material(&self) -> i16 {
        self.material
    }
    pub fn vertex_count(&self) -> usize {
        self.vertices.len() / 8
    }
    pub fn vertex(&self, index: usize) -> Option<(u16, u16, u32)> {
        let at = index.checked_mul(8)?;
        let bytes = self.vertices.get(at..at.checked_add(8)?)?;
        Some((
            short(bytes, 0).ok()?,
            short(bytes, 2).ok()?,
            word(bytes, 4).ok()?,
        ))
    }
    pub const fn tiles(&self) -> &'a [u8] {
        self.tiles
    }
}

pub struct Group<'a> {
    bytes: &'a [u8],
    children: &'a [u8],
    header: GroupHeader,
    version: Option<u32>,
    materials: &'a [u8],
    material_width: usize,
    indices: &'a [u8],
    index_width: usize,
    vertices: &'a [u8],
    batches: &'a [u8],
    references: &'a [u8],
    liquid: Option<LiquidView<'a>>,
    chunk_count: usize,
}

impl<'a> Group<'a> {
    pub fn parse(bytes: &'a [u8], limits: Limits) -> Result<Self, Error> {
        limits.input(bytes)?;
        let (mut mogp, mut version) = (None, None);
        let mut count = 0;
        scan(bytes, &mut count, limits, |chunk| match &chunk.tag {
            b"MOGP" => once(&mut mogp, chunk.payload),
            b"MVER" => once(&mut version, chunk.payload),
            b"MOPY" | b"MPY2" | b"MOVI" | b"MOVX" | b"MOVT" | b"MOBA" | b"MODR" | b"MLIQ" => {
                Err(Error::Unsupported)
            }
            _ => Ok(()),
        })?;
        let mogp = mogp.ok_or(Error::MissingHeader)?;
        let h = mogp.get(..68).ok_or(Error::Shape)?;
        let header = GroupHeader {
            name_offset: word(h, 0)? as i32,
            description_offset: word(h, 4)? as i32,
            flags: word(h, 8)?,
            bounds_bits: bounds(h, 12)?,
            portal_start: short(h, 36)?,
            portal_count: short(h, 38)?,
            batches_a: short(h, 40)?,
            batches_b: short(h, 42)?,
            batches_c: word(h, 44)?,
            fog_indices: word(h, 48)?,
            source_liquid_type: word(h, 52)?,
            group_wmo_id: word(h, 56)?,
            flags2: word(h, 60)?,
            parent_or_first_child_split_group: short(h, 64)? as i16,
            next_split_child_group: short(h, 66)? as i16,
        };
        let children = &mogp[68..];
        let (mut mopy, mut mpy2, mut movi, mut movx, mut movt) = (None, None, None, None, None);
        let (mut moba, mut modr, mut mliq) = (None, None, None);
        scan(children, &mut count, limits, |chunk| {
            let slot = match &chunk.tag {
                b"MOPY" => &mut mopy,
                b"MPY2" => &mut mpy2,
                b"MOVI" => &mut movi,
                b"MOVX" => &mut movx,
                b"MOVT" => &mut movt,
                b"MOBA" => &mut moba,
                b"MODR" => &mut modr,
                b"MLIQ" => &mut mliq,
                b"MOGP" | b"MVER" => return Err(Error::Unsupported),
                _ => return Ok(()),
            };
            once(slot, chunk.payload)
        })?;
        if (mopy.is_some() && mpy2.is_some()) || (movi.is_some() && movx.is_some()) {
            return Err(Error::Unsupported);
        }
        let material_width = if mpy2.is_some() { 4 } else { 2 };
        let index_width = if movx.is_some() { 4 } else { 2 };
        let materials = mpy2.or(mopy).unwrap_or(&[]);
        let indices = movx.or(movi).unwrap_or(&[]);
        let vertices = movt.unwrap_or(&[]);
        let batches = moba.unwrap_or(&[]);
        let references = modr.unwrap_or(&[]);
        let triangles = records(materials, material_width, limits.max_indices / 3)?;
        let index_count = records(indices, index_width, limits.max_indices)?;
        let vertex_count = records(vertices, 12, limits.max_vertices)?;
        if triangles.checked_mul(3).ok_or(Error::Bounds)? != index_count {
            return Err(Error::Shape);
        }
        records(batches, 24, limits.max_metadata_records)?;
        records(references, 2, limits.max_metadata_records)?;
        for vertex in vertices.as_chunks::<12>().0 {
            bits3(vertex, 0)?;
        }
        let result = Self {
            bytes,
            children,
            header,
            version: version
                .map(|v| {
                    exact(v, 4)?;
                    word(v, 0)
                })
                .transpose()?,
            materials,
            material_width,
            indices,
            index_width,
            vertices,
            batches,
            references,
            liquid: mliq.map(|v| LiquidView::parse(v, limits)).transpose()?,
            chunk_count: count,
        };
        for index in 0..index_count {
            if result.source_index(index).ok_or(Error::Index)? as usize >= vertex_count {
                return Err(Error::Index);
            }
        }
        Ok(result)
    }
    pub const fn bytes(&self) -> &'a [u8] {
        self.bytes
    }
    pub const fn header(&self) -> GroupHeader {
        self.header
    }
    pub const fn version(&self) -> Option<u32> {
        self.version
    }
    pub const fn chunk_count(&self) -> usize {
        self.chunk_count
    }
    pub fn outer_chunks(&self) -> Chunks<'a> {
        chunks(self.bytes)
    }
    /// Offsets are relative to the first child byte after the 68-byte prefix.
    pub fn chunks(&self) -> Chunks<'a> {
        chunks(self.children)
    }
    pub fn vertex_count(&self) -> usize {
        self.vertices.len() / 12
    }
    pub fn index_count(&self) -> usize {
        self.indices.len() / self.index_width
    }
    pub fn triangle_count(&self) -> usize {
        self.materials.len() / self.material_width
    }
    pub fn vertex_bits(&self, index: usize) -> Option<[u32; 3]> {
        let at = index.checked_mul(12)?;
        bits3(self.vertices.get(at..at.checked_add(12)?)?, 0).ok()
    }
    pub fn source_index(&self, index: usize) -> Option<u32> {
        let at = index.checked_mul(self.index_width)?;
        if self.index_width == 4 {
            word(self.indices, at).ok()
        } else {
            short(self.indices, at).ok().map(u32::from)
        }
    }
    pub fn triangle_material(&self, index: usize) -> Option<TriangleMaterial> {
        let at = index.checked_mul(self.material_width)?;
        let bytes = self
            .materials
            .get(at..at.checked_add(self.material_width)?)?;
        if self.material_width == 4 {
            Some(TriangleMaterial {
                flags: short(bytes, 0).ok()?,
                material: short(bytes, 2).ok()?,
            })
        } else {
            Some(TriangleMaterial {
                flags: u16::from(bytes[0]),
                material: u16::from(bytes[1]),
            })
        }
    }
    pub fn batch_count(&self) -> usize {
        self.batches.len() / 24
    }
    /// Exact source small/large-mode GRP value: uint16 MOBA[8 + 12*i].
    /// No extra interpretation of the opaque batch fields is imposed.
    pub fn batch_value(&self, index: usize) -> Option<u32> {
        short(self.batches, index.checked_mul(24)?.checked_add(16)?)
            .ok()
            .map(u32::from)
    }
    pub const fn batch_bytes(&self) -> &'a [u8] {
        self.batches
    }
    pub fn doodad_reference_count(&self) -> usize {
        self.references.len() / 2
    }
    pub fn doodad_reference(&self, index: usize) -> Option<u16> {
        short(self.references, index.checked_mul(2)?).ok()
    }
    pub const fn liquid(&self) -> Option<LiquidView<'a>> {
        self.liquid
    }
}
