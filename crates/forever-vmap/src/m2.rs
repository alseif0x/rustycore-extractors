// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original M2 collision parser and raw writer.
// Facts: pinned vmap4_extractor/modelheaders.h:24-94 and model.cpp:37-158.

use crate::{
    Error, Limits, M2_HEADER_BYTES, MAX_RAW_BYTES, RAW_MAGIC, bounded_slice, bounds_at,
    finite_vertices, raw_size, vertex_at, word,
};

/// Validated collision geometry borrowed from complete source bytes.
///
/// The first bytewise MD20 occurrence is authoritative, matching the source
/// scanner. A rejected first candidate never falls through to a later one.
/// A full 304-byte header is required even for no collision. No version field
/// is checked by the pin. Prefixes, unrelated fields, padding and trailing bytes
/// are not interpreted as an MD21 container or validated as other M2 sections.
/// Arrays may overlap and may be unaligned; offsets are relative to MD20.
///
/// Safety policies stronger than the source: all used extents must be present,
/// indices must form valid triangles, emitted floats must be finite, and bounds
/// must be ordered. Bounds need not enclose the vertices (the source copies the
/// declared bounds). No Debug implementation exposes geometry accidentally.
pub struct M2Collision<'a> {
    header_offset: usize,
    bounds: [u32; 6],
    vertices: &'a [u8],
    indices: &'a [u8],
}

impl<'a> M2Collision<'a> {
    pub fn parse(bytes: &'a [u8], limits: Limits) -> Result<Option<Self>, Error> {
        limits.validate()?;
        if bytes.len() > limits.max_input_bytes {
            return Err(Error::Limit);
        }
        let start = bytes
            .windows(4)
            .position(|candidate| candidate == b"MD20")
            .ok_or(Error::MissingMd20)?;
        let source = bytes.get(start..).ok_or(Error::Bounds)?;
        let header = source.get(..M2_HEADER_BYTES).ok_or(Error::Truncated)?;
        let index_count = word(header, 216)? as usize;
        // model.cpp:63-84 does not access any array on this path. Unused counts,
        // offsets, normals, sphere radii and collision bounds impose no gate.
        if index_count == 0 {
            return Ok(None);
        }
        let vertex_count = word(header, 224)? as usize;
        limits.counts(vertex_count, index_count)?;
        let bounds = bounds_at(header, 188)?;
        let vertices = bounded_slice(source, word(header, 228)? as usize, vertex_count, 12)?;
        let indices = bounded_slice(source, word(header, 220)? as usize, index_count, 2)?;
        finite_vertices(vertices)?;
        for pair in indices.as_chunks::<2>().0 {
            let index = usize::from(u16::from_le_bytes([pair[0], pair[1]]));
            if index >= vertex_count {
                return Err(Error::Index);
            }
        }
        Ok(Some(Self {
            header_offset: start,
            bounds,
            vertices,
            indices,
        }))
    }

    pub const fn header_offset(&self) -> usize {
        self.header_offset
    }
    pub fn vertex_count(&self) -> usize {
        self.vertices.len() / 12
    }
    /// The source's nBoundingTriangles is an index count, not a triangle count.
    pub fn index_count(&self) -> usize {
        self.indices.len() / 2
    }
    pub fn triangle_count(&self) -> usize {
        self.index_count() / 3
    }
    pub const fn bounds_bits(&self) -> [u32; 6] {
        self.bounds
    }
    /// Vertex coordinates as emitted by the raw writer, including signed zero.
    pub fn vertex_bits(&self, index: usize) -> Option<[u32; 3]> {
        vertex_at(self.vertices, index)
    }
    /// Index order in the input M2, before the raw writer swaps triangle winding.
    pub fn source_index(&self, index: usize) -> Option<u16> {
        let offset = index.checked_mul(2)?;
        let pair = self.indices.get(offset..offset.checked_add(2)?)?;
        Some(u16::from_le_bytes([pair[0], pair[1]]))
    }
    /// Triangle in emitted raw order: original indices 0, 2, 1.
    pub fn raw_triangle(&self, index: usize) -> Option<[u32; 3]> {
        let first = index.checked_mul(3)?;
        Some([
            u32::from(self.source_index(first)?),
            u32::from(self.source_index(first.checked_add(2)?)?),
            u32::from(self.source_index(first.checked_add(1)?)?),
        ])
    }

    /// Writes the complete single-group M2 intermediate. No partial output.
    /// The cap may reduce [`MAX_RAW_BYTES`], never increase it.
    ///
    /// model.cpp first maps (x,y,z) to (x,z,-y), then maps that to
    /// (x,-(-y),z). For admitted finite IEEE f32 values these transformations
    /// restore every original bit, including signed zero. Copying the borrowed
    /// vertex bytes therefore preserves both transforms without arithmetic.
    pub fn write_raw(&self, max_bytes: usize) -> Result<Vec<u8>, Error> {
        if max_bytes > MAX_RAW_BYTES {
            return Err(Error::InvalidLimits);
        }
        let size = raw_size(self.vertex_count(), self.index_count())?;
        if size > max_bytes {
            return Err(Error::Limit);
        }
        let vertex_count = u32::try_from(self.vertex_count()).map_err(|_| Error::Bounds)?;
        let index_count = u32::try_from(self.index_count()).map_err(|_| Error::Bounds)?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(size)
            .map_err(|_| Error::Allocation)?;
        bytes.extend_from_slice(RAW_MAGIC);
        append(&mut bytes, vertex_count);
        append(&mut bytes, 1); // groups
        append(&mut bytes, 0); // RootWMOID
        append(&mut bytes, 1); // ModelFlags::IsM2
        append(&mut bytes, 0); // mogpFlags
        append(&mut bytes, 0); // groupWMOID
        for bits in self.bounds {
            append(&mut bytes, bits);
        }
        append(&mut bytes, 0); // liquidflags
        bytes.extend_from_slice(b"GRP ");
        append(&mut bytes, 8);
        append(&mut bytes, 1);
        append(&mut bytes, index_count);
        bytes.extend_from_slice(b"INDX");
        // Intentional pin format: declared u16 size, physical u32 indices.
        append(&mut bytes, 4 + 2 * index_count);
        append(&mut bytes, index_count);
        for triple in self.indices.as_chunks::<6>().0 {
            for at in [0, 4, 2] {
                append(
                    &mut bytes,
                    u32::from(u16::from_le_bytes([triple[at], triple[at + 1]])),
                );
            }
        }
        bytes.extend_from_slice(b"VERT");
        append(&mut bytes, 4 + 12 * vertex_count);
        append(&mut bytes, vertex_count);
        bytes.extend_from_slice(self.vertices);
        Ok(bytes)
    }
}

fn append(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}
