// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original bounded reader of the pinned M2 writer subset.
// Facts: model.cpp:88-152; TileAssembler.cpp:475-594 reads this representation.

use crate::{
    Error, Limits, MAX_RAW_BYTES, RAW_MAGIC, bounded_slice, bounds_at, finite_vertices, raw_size,
    vertex_at, word,
};

/// Borrowed, fully validated single-M2 raw04E intermediate.
///
/// This is the exact subset emitted by [`crate::M2Collision::write_raw`], not a
/// WMO/general raw reader. Unknown flags, extra groups, liquid, altered default
/// fields, empty geometry, corrected INDX sizes, padding and trailing bytes are
/// rejected. Unlike the permissive C++ raw reader, each known size is checked.
/// In particular INDX's declared size intentionally uses two bytes per index
/// while its physical payload uses four. All physical offsets use that payload.
pub struct RawM2<'a> {
    bytes: &'a [u8],
    bounds: [u32; 6],
    vertices: &'a [u8],
    indices: &'a [u8],
}

impl<'a> RawM2<'a> {
    pub fn parse(bytes: &'a [u8], limits: Limits) -> Result<Self, Error> {
        limits.validate()?;
        if bytes.len() > limits.max_input_bytes || bytes.len() > MAX_RAW_BYTES {
            return Err(Error::Limit);
        }
        if bytes.len() < 100 {
            return Err(Error::Truncated);
        }
        if bytes.get(..8) != Some(RAW_MAGIC.as_slice())
            || word(bytes, 12)? != 1
            || word(bytes, 16)? != 0
            || word(bytes, 20)? != 1
            || word(bytes, 24)? != 0
            || word(bytes, 28)? != 0
            || word(bytes, 56)? != 0
            || bytes.get(60..64) != Some(b"GRP ".as_slice())
            || word(bytes, 64)? != 8
            || word(bytes, 68)? != 1
            || bytes.get(76..80) != Some(b"INDX".as_slice())
        {
            return Err(Error::RawFormat);
        }
        let vertex_count = word(bytes, 8)? as usize;
        let index_count = word(bytes, 84)? as usize;
        limits.counts(vertex_count, index_count)?;
        if raw_size(vertex_count, index_count)? != bytes.len() {
            return Err(Error::RawExtent);
        }
        let declared_indices = 4usize
            .checked_add(index_count.checked_mul(2).ok_or(Error::Bounds)?)
            .ok_or(Error::Bounds)?;
        if word(bytes, 72)? as usize != index_count || word(bytes, 80)? as usize != declared_indices
        {
            return Err(Error::RawFormat);
        }
        let indices = bounded_slice(bytes, 88, index_count, 4)?;
        let vertex_header = 88usize.checked_add(indices.len()).ok_or(Error::Bounds)?;
        let declared_vertices = 4usize
            .checked_add(vertex_count.checked_mul(12).ok_or(Error::Bounds)?)
            .ok_or(Error::Bounds)?;
        if bytes.get(vertex_header..vertex_header + 4) != Some(b"VERT".as_slice())
            || word(bytes, vertex_header + 4)? as usize != declared_vertices
            || word(bytes, vertex_header + 8)? as usize != vertex_count
        {
            return Err(Error::RawFormat);
        }
        let vertices = bounded_slice(bytes, vertex_header + 12, vertex_count, 12)?;
        let bounds = bounds_at(bytes, 32)?;
        finite_vertices(vertices)?;
        for index in indices.as_chunks::<4>().0 {
            let index = word(index, 0)? as usize;
            // M2 source indices are u16 even though the raw representation is u32.
            if index > usize::from(u16::MAX) || index >= vertex_count {
                return Err(Error::Index);
            }
        }
        Ok(Self {
            bytes,
            bounds,
            vertices,
            indices,
        })
    }

    pub const fn bytes(&self) -> &'a [u8] {
        self.bytes
    }
    pub fn vertex_count(&self) -> usize {
        self.vertices.len() / 12
    }
    pub fn index_count(&self) -> usize {
        self.indices.len() / 4
    }
    pub fn triangle_count(&self) -> usize {
        self.index_count() / 3
    }
    pub const fn bounds_bits(&self) -> [u32; 6] {
        self.bounds
    }
    pub fn vertex_bits(&self, index: usize) -> Option<[u32; 3]> {
        vertex_at(self.vertices, index)
    }
    pub fn index(&self, index: usize) -> Option<u32> {
        let start = index.checked_mul(4)?;
        word(self.indices, start).ok()
    }
}
