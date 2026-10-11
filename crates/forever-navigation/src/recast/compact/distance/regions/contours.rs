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

// Modified/ported 2026-10-11: checked contour tracing, simplification and hole assembly.
use super::*;
mod holes;
mod mesh;
pub use mesh::{
    DetailMesh, DetailOptions, DetailSubmesh, MergeLimits, MeshHeader, PolyMesh, Polygon, TileMesh,
};
mod simplify;
#[cfg(test)]
mod tests;
mod trace;
const BORDER_VERTEX: i32 = 0x10000;
const AREA_BORDER: i32 = 0x20000;
const REGION_MASK: i32 = 0xffff;
type Vertex = [i32; 4];

#[derive(Clone, Copy, Debug)]
pub struct ContourOptions {
    pub maximum_error: f32,
    pub maximum_edge_length: u16,
    pub tessellate_walls: bool,
    pub tessellate_areas: bool,
}
impl Default for ContourOptions {
    fn default() -> Self {
        Self {
            maximum_error: 1.8,
            maximum_edge_length: 0,
            tessellate_walls: true,
            tessellate_areas: false,
        }
    }
}
pub struct Contour {
    vertices: Vec<Vertex>,
    raw: Vec<Vertex>,
    region: u16,
    area: u8,
}
impl Contour {
    pub fn vertices(&self) -> &[Vertex] {
        &self.vertices
    }
    pub fn raw_vertices(&self) -> &[Vertex] {
        &self.raw
    }
    pub const fn region(&self) -> u16 {
        self.region
    }
    pub const fn area(&self) -> u8 {
        self.area
    }
}
pub struct ContourSet {
    regions: Regions,
    grid: Grid,
    options: ContourOptions,
    contours: Vec<Contour>,
}
impl ContourSet {
    pub fn regions(&self) -> &Regions {
        &self.regions
    }
    /// Padding removed from bounds and dimensions; contour x/z coordinates are
    /// relative to this grid. Zero dimensions are retained for empty outputs.
    pub const fn grid(&self) -> Grid {
        self.grid
    }
    pub const fn options(&self) -> ContourOptions {
        self.options
    }
    /// Merged hole entries remain in native order with empty simplified arrays.
    pub fn contours(&self) -> &[Contour] {
        &self.contours
    }
    pub fn capacity_bytes(&self) -> usize {
        self.regions.capacity_bytes()
            + self.contours.capacity() * std::mem::size_of::<Contour>()
            + self
                .contours
                .iter()
                .map(|c| (c.vertices.capacity() + c.raw.capacity()) * 16)
                .sum::<usize>()
    }
}
impl Regions {
    pub fn build_contours(self, options: ContourOptions) -> Result<ContourSet, Error> {
        if !options.maximum_error.is_finite()
            || options.maximum_error < 0.0
            || !(options.maximum_error * options.maximum_error).is_finite()
            || options.maximum_edge_length > 46340
        {
            return Err(Error::Numeric);
        }
        let mut grid = self.field.compact.grid;
        let border = self.options.border_size;
        if u32::from(border) * 2 > u32::from(grid.width.min(grid.height)) {
            return Err(Error::Grid);
        }
        grid.width -= border * 2;
        grid.height -= border * 2;
        if border > 0 {
            let pad = finite(f32::from(border) * grid.cell_size)?;
            for axis in [0, 2] {
                grid.min[axis] = finite(grid.min[axis] + pad)?;
                grid.max[axis] = finite(grid.max[axis] - pad)?;
                if grid.min[axis] > grid.max[axis] {
                    return Err(Error::Grid);
                }
            }
        }
        let bytes = self.capacity_bytes();
        let mut b = ContourBuilder {
            regions: self,
            bytes,
        };
        let mut flags = Vec::new();
        let n = b.regions.field.compact.spans.len();
        b.reserve(&mut flags, n)?;
        b.charge(n as u64)?;
        flags.resize(n, 0u8);
        for cell in 0..b.regions.field.compact.cells.len() {
            b.charge(1)?;
            for i in b.regions.field.compact.indices(cell) {
                b.charge(1)?;
                let e = Entry {
                    cell: cell as u32,
                    index: i as u32,
                };
                let r = b.span(e).region;
                if r == 0 || r & BORDER != 0 {
                    continue;
                }
                let mut res = 0;
                for dir in 0..4 {
                    if b.link(e, dir)?.map(|n| b.span(n).region).unwrap_or(0) == r {
                        res |= 1 << dir;
                    }
                }
                flags[i] = res ^ 15;
            }
        }
        let mut contours = Vec::new();
        for cell in 0..b.regions.field.compact.cells.len() {
            b.charge(1)?;
            for i in b.regions.field.compact.indices(cell) {
                b.charge(1)?;
                if flags[i] == 0 || flags[i] == 15 {
                    flags[i] = 0;
                    continue;
                }
                let e = Entry {
                    cell: cell as u32,
                    index: i as u32,
                };
                let s = b.span(e);
                if s.region == 0 || s.region & BORDER != 0 {
                    continue;
                }
                let mut raw = b.trace(e, &mut flags)?;
                let mut vertices = b.simplify(&raw, options)?;
                if vertices.len() >= 3 {
                    for v in raw.iter_mut().chain(vertices.iter_mut()) {
                        b.charge(1)?;
                        v[0] -= i32::from(border);
                        v[2] -= i32::from(border);
                    }
                    b.push(
                        &mut contours,
                        Contour {
                            vertices,
                            raw,
                            region: s.region,
                            area: s.area,
                        },
                    )?;
                } else {
                    b.release(&vertices);
                    b.release(&raw);
                }
            }
        }
        b.release(&flags);
        drop(flags);
        b.merge_holes(&mut contours)?;
        Ok(ContourSet {
            regions: b.regions,
            grid,
            options,
            contours,
        })
    }
}
struct ContourBuilder {
    regions: Regions,
    bytes: usize,
}
impl ContourBuilder {
    fn charge(&mut self, n: u64) -> Result<(), Error> {
        self.regions.field.compact.charge(n)
    }
    fn reserve<T>(&mut self, v: &mut Vec<T>, additional: usize) -> Result<(), Error> {
        let want = v.len().checked_add(additional).ok_or(Error::Limit)?;
        if want <= v.capacity() {
            return Ok(());
        }
        let size = std::mem::size_of::<T>();
        let old = v.capacity() * size;
        let cap = want.max(v.capacity().saturating_mul(2)).max(8);
        let bytes = cap.checked_mul(size).ok_or(Error::Limit)?;
        if self.bytes.checked_add(bytes).ok_or(Error::Limit)?
            > self.regions.field.compact.limits.heap_bytes
        {
            return Err(Error::Limit);
        }
        self.charge(v.len() as u64)?;
        v.try_reserve_exact(cap - v.len())
            .map_err(|_| Error::Allocation)?;
        let new = v.capacity() * size;
        if self.bytes.checked_add(new).ok_or(Error::Limit)?
            > self.regions.field.compact.limits.heap_bytes
        {
            return Err(Error::Limit);
        }
        self.bytes = self.bytes - old + new;
        Ok(())
    }
    fn push<T>(&mut self, v: &mut Vec<T>, value: T) -> Result<(), Error> {
        self.charge(1)?;
        self.reserve(v, 1)?;
        v.push(value);
        Ok(())
    }
    fn release<T>(&mut self, v: &Vec<T>) {
        self.bytes -= v.capacity() * std::mem::size_of::<T>();
    }
    fn insert(&mut self, v: &mut Vec<Vertex>, index: usize, p: Vertex) -> Result<(), Error> {
        self.charge((v.len() - index + 1) as u64)?;
        self.reserve(v, 1)?;
        v.insert(index, p);
        Ok(())
    }
    fn span(&self, e: Entry) -> CompactSpan {
        self.regions.field.compact.spans[e.index as usize]
    }
    fn link(&mut self, e: Entry, dir: usize) -> Result<Option<Entry>, Error> {
        Ok(self
            .regions
            .field
            .compact
            .link(e.cell as usize, e.index as usize, dir)?
            .map(|(cell, index)| Entry {
                cell: cell as u32,
                index: index as u32,
            }))
    }
}

pub use mesh::detour;
