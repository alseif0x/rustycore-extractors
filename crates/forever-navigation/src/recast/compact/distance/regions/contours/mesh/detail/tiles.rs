// SPDX-License-Identifier: GPL-3.0-or-later
//
// Copyright (c) 2009-2010 Mikko Mononen memon@inside.org
//
// This software is provided 'as-is', without any express or implied
// warranty.  In no event will the authors be held liable for any damages
// arising from the use of this software.
// Permission is granted to anyone to use this software for any purpose,
// including commercial applications, and to alter it and redistribute it
// freely, subject to the following restrictions:
// 1. The origin of this software must not be misrepresented; you must not
//    claim that you wrote the original software. If you use this software
//    in a product, an acknowledgment in the product documentation would be
//    appreciated but is not required.
// 2. Altered source versions must be plainly marked as such, and must not be
//    misrepresented as being the original software.
// 3. This notice may not be removed or altered from any source distribution.
//

// Modified/ported 2026-10-11: original polygon/detail merging with bounded ownership.
use super::*;
#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MeshHeader {
    pub min: Point,
    pub max: Point,
    pub cell_size: f32,
    pub cell_height: f32,
    pub vertices_per_polygon: u8,
    /// Native metadata, not Rust capacity. Original merge leaves this zero.
    pub maximum_polygons: usize,
    /// Original merge leaves this zero even when boundary portals remain.
    pub border_size: u16,
    /// Original merge leaves this zero.
    pub maximum_edge_error: f32,
}

/// Separate assembly budget after per-subtile fields have been released.
/// Input vector capacities and all assembly scratch/results count toward heap.
/// Prior stage work is retained as provenance, not charged again to this budget.
#[derive(Clone, Copy, Debug)]
pub struct MergeLimits {
    pub subtiles: usize,
    pub heap_bytes: usize,
    pub work: u64,
}
impl Default for MergeLimits {
    fn default() -> Self {
        Self {
            subtiles: 4096,
            heap_bytes: 128 * 1024 * 1024,
            work: 128_000_000,
        }
    }
}
impl MergeLimits {
    fn validate(self) -> Result<(), Error> {
        let max = Self::default();
        if self.subtiles > max.subtiles || self.heap_bytes > max.heap_bytes || self.work > max.work
        {
            Err(Error::Limit)
        } else {
            Ok(())
        }
    }
}

/// Paired polygon/detail geometry, detached from compact fields and contours.
/// Can represent one subtile or an ordered merge; contains no Detour data.
pub struct TileMesh {
    header: MeshHeader,
    vertices: Vec<[u16; 3]>,
    polygons: Vec<Polygon>,
    submeshes: Vec<DetailSubmesh>,
    detail_vertices: Vec<Point>,
    detail_triangles: Vec<[u8; 4]>,
    discarded_faces: u32,
    processing_work: u64,
    merge_work: u64,
}
impl DetailMesh {
    /// Release the compact graph/contours after detail construction, without
    /// copying polygon or detail arrays. The paired ordering stays immutable.
    pub fn into_tile_mesh(self) -> TileMesh {
        let m = self.mesh;
        let c = m.contours;
        TileMesh {
            header: MeshHeader {
                min: c.grid.min,
                max: c.grid.max,
                cell_size: c.grid.cell_size,
                cell_height: c.grid.cell_height,
                vertices_per_polygon: m.nvp,
                maximum_polygons: m.maximum_polygons,
                border_size: c.regions.options.border_size,
                maximum_edge_error: c.options.maximum_error,
            },
            vertices: m.vertices,
            polygons: m.polygons,
            submeshes: self.submeshes,
            detail_vertices: self.vertices,
            detail_triangles: self.triangles,
            discarded_faces: self.discarded_faces,
            processing_work: c.regions.field.compact.work,
            merge_work: 0,
        }
    }
}
impl TileMesh {
    pub const fn header(&self) -> MeshHeader {
        self.header
    }
    pub fn vertices(&self) -> &[[u16; 3]] {
        &self.vertices
    }
    pub fn polygons(&self) -> &[Polygon] {
        &self.polygons
    }
    pub fn submeshes(&self) -> &[DetailSubmesh] {
        &self.submeshes
    }
    pub fn detail_vertices(&self) -> &[Point] {
        &self.detail_vertices
    }
    pub fn detail_triangles(&self) -> &[[u8; 4]] {
        &self.detail_triangles
    }
    /// Sum of detail-stage recovery warnings in the supplied meshes.
    pub const fn discarded_faces(&self) -> u32 {
        self.discarded_faces
    }
    /// Total logical work from all supplied stages, including this merge.
    pub const fn processing_work(&self) -> u64 {
        self.processing_work
    }
    /// Logical work of this merge only; zero on an unmerged subtile.
    pub const fn merge_work(&self) -> u64 {
        self.merge_work
    }
    pub fn capacity_bytes(&self) -> usize {
        self.vertices.capacity() * 6
            + self.polygons.capacity() * std::mem::size_of::<Polygon>()
            + self.submeshes.capacity() * 16
            + self.detail_vertices.capacity() * 12
            + self.detail_triangles.capacity() * 4
    }

    /// Original rcMergePolyMeshes + rcMergePolyMeshDetails ordering. Inputs
    /// require common cell sizes, polygon width and vertical origin. Source
    /// neighbor IDs are rebuilt; detail triangle indices remain local.
    pub fn merge(subtiles: Vec<Self>, limits: MergeLimits) -> Result<Self, Error> {
        limits.validate()?;
        if subtiles.len() > limits.subtiles {
            return Err(Error::Limit);
        }
        let resident = subtiles.iter().try_fold(
            subtiles
                .capacity()
                .checked_mul(std::mem::size_of::<Self>())
                .ok_or(Error::Limit)?,
            |n, m| n.checked_add(m.capacity_bytes()).ok_or(Error::Limit),
        )?;
        if resident > limits.heap_bytes {
            return Err(Error::Limit);
        }
        let mut b = Budget {
            limits: Limits {
                heap_bytes: limits.heap_bytes,
                work: limits.work,
                ..Limits::default()
            },
            work: 0,
            bytes: resident,
        };
        let mut header = MeshHeader::default();
        let mut maximum_polygons = 0usize;
        let mut processing_work = 0u64;
        let mut discarded_faces = 0u32;
        if let Some(first) = subtiles.first() {
            header.min = first.header.min;
            header.max = first.header.max;
            header.cell_size = first.header.cell_size;
            header.cell_height = first.header.cell_height;
            header.vertices_per_polygon = first.header.vertices_per_polygon;
        }
        for m in &subtiles {
            b.charge(1)?;
            if m.header.vertices_per_polygon != header.vertices_per_polygon
                || m.header.cell_size != header.cell_size
                || m.header.cell_height != header.cell_height
                || m.header.min[1] != header.min[1]
                || !(3..=6).contains(&header.vertices_per_polygon)
            {
                return Err(Error::MergeContract);
            }
            // Native rcMin/rcMax take the right operand for equal values.
            for axis in 0..3 {
                if header.min[axis] >= m.header.min[axis] {
                    header.min[axis] = m.header.min[axis];
                }
                if header.max[axis] <= m.header.max[axis] {
                    header.max[axis] = m.header.max[axis];
                }
            }
            maximum_polygons = maximum_polygons
                .checked_add(m.polygons.len())
                .ok_or(Error::Limit)?;
            if maximum_polygons > 0x8000 {
                return Err(Error::Mesh);
            }
            processing_work = processing_work
                .checked_add(m.processing_work)
                .ok_or(Error::Limit)?;
            discarded_faces = discarded_faces
                .checked_add(m.discarded_faces)
                .ok_or(Error::Limit)?;
        }
        let mut data = Data {
            vertices: Vec::new(),
            polygons: Vec::new(),
            nvp: usize::from(header.vertices_per_polygon),
            maximum_polygons,
        };
        let mut submeshes = Vec::new();
        let mut detail_vertices = Vec::new();
        let mut detail_triangles = Vec::new();
        if !subtiles.is_empty() {
            let mut first = b.filled(4096, NULL)?;
            let mut next = Vec::new();
            for m in &subtiles {
                b.charge(1)?;
                let ox = offset(m.header.min[0], header.min[0], header.cell_size)?;
                let oz = offset(m.header.min[2], header.min[2], header.cell_size)?;
                let borders = [
                    ox == 0,
                    offset(header.max[2], m.header.max[2], header.cell_size)? == 0,
                    offset(header.max[0], m.header.max[0], header.cell_size)? == 0,
                    oz == 0,
                ];
                let mut remap = Vec::new();
                for &v in &m.vertices {
                    b.charge(1)?;
                    let shifted = [
                        v[0].checked_add(ox).ok_or(Error::Numeric)?,
                        v[1],
                        v[2].checked_add(oz).ok_or(Error::Numeric)?,
                    ];
                    let id = b.add_vertex(&mut data.vertices, &mut first, &mut next, shifted)?;
                    b.push(&mut remap, id)?;
                }
                for p in &m.polygons {
                    b.charge(1)?;
                    let mut out = Polygon::new(p.region, p.area);
                    out.flags = p.flags;
                    for (slot, &id) in p.vertices().iter().enumerate() {
                        b.charge(1)?;
                        out.vertices[slot] = remap[usize::from(id)];
                    }
                    for (slot, &n) in p.neighbors[..data.nvp].iter().enumerate() {
                        b.charge(1)?;
                        if n != NULL && n & 0x8000 != 0 {
                            let direction = usize::from(n & 0xf);
                            if borders.get(direction).copied().unwrap_or(false) {
                                out.neighbors[slot] = n;
                            }
                        }
                    }
                    b.store(&mut data, out)?;
                }
                b.release(&remap);
                drop(remap);
                let vb = u32::try_from(detail_vertices.len()).map_err(|_| Error::Limit)?;
                let tb = u32::try_from(detail_triangles.len()).map_err(|_| Error::Limit)?;
                for s in &m.submeshes {
                    b.push(
                        &mut submeshes,
                        DetailSubmesh {
                            vertex_base: vb.checked_add(s.vertex_base).ok_or(Error::Limit)?,
                            vertex_count: s.vertex_count,
                            triangle_base: tb.checked_add(s.triangle_base).ok_or(Error::Limit)?,
                            triangle_count: s.triangle_count,
                        },
                    )?;
                }
                for &v in &m.detail_vertices {
                    b.push(&mut detail_vertices, v)?;
                }
                for &t in &m.detail_triangles {
                    b.push(&mut detail_triangles, t)?;
                }
            }
            b.release(&first);
            drop(first);
            b.release(&next);
            drop(next);
            b.adjacency(&mut data)?;
        }
        processing_work = processing_work.checked_add(b.work).ok_or(Error::Limit)?;
        Ok(Self {
            header,
            vertices: data.vertices,
            polygons: data.polygons,
            submeshes,
            detail_vertices,
            detail_triangles,
            discarded_faces,
            processing_work,
            merge_work: b.work,
        })
    }
}
fn offset(max: f32, min: f32, cell: f32) -> Result<u16, Error> {
    let value = ((max - min) / cell + 0.5).floor();
    if !value.is_finite() || !(0.0..=65535.0).contains(&value) {
        Err(Error::Numeric)
    } else {
        Ok(value as u16)
    }
}

pub mod detour;
