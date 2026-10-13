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

// Modified/ported 2026-10-11: bounded original contour-to-polygon construction.
use super::*;
mod adjacency;
mod detail;
pub use detail::{DetailMesh, DetailOptions, DetailSubmesh, MergeLimits, MeshHeader, TileMesh};
mod merge;
mod remove;
#[cfg(test)]
mod tests;
mod triangulate;
const NULL: u16 = u16::MAX;
const MAX_NVP: usize = 6;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Polygon {
    vertices: [u16; MAX_NVP],
    neighbors: [u16; MAX_NVP],
    region: u16,
    area: u8,
    flags: u16,
}
impl Polygon {
    fn new(region: u16, area: u8) -> Self {
        Self {
            vertices: [NULL; MAX_NVP],
            neighbors: [NULL; MAX_NVP],
            region,
            area,
            flags: 0,
        }
    }
    fn len(&self) -> usize {
        self.vertices
            .iter()
            .position(|&v| v == NULL)
            .unwrap_or(MAX_NVP)
    }
    pub fn vertices(&self) -> &[u16] {
        &self.vertices[..self.len()]
    }
    /// NULL is a wall; 0x8000|direction is a portal; otherwise a polygon index.
    pub fn neighbors(&self) -> &[u16] {
        &self.neighbors[..self.len()]
    }
    pub const fn region(&self) -> u16 {
        self.region
    }
    pub const fn area(&self) -> u8 {
        self.area
    }
    /// Original rcBuildPolyMesh initializes flags to zero; navigation flags are later work.
    pub const fn flags(&self) -> u16 {
        self.flags
    }
}
pub struct PolyMesh {
    contours: ContourSet,
    vertices: Vec<[u16; 3]>,
    polygons: Vec<Polygon>,
    nvp: u8,
    maximum_polygons: usize,
}
impl PolyMesh {
    pub fn contours(&self) -> &ContourSet {
        &self.contours
    }
    pub fn vertices(&self) -> &[[u16; 3]] {
        &self.vertices
    }
    pub fn polygons(&self) -> &[Polygon] {
        &self.polygons
    }
    pub const fn vertices_per_polygon(&self) -> u8 {
        self.nvp
    }
    /// Native maxpolys metadata: sum of (contour vertices - 2), not Vec capacity.
    pub const fn maximum_polygons(&self) -> usize {
        self.maximum_polygons
    }
    pub fn capacity_bytes(&self) -> usize {
        self.contours.capacity_bytes()
            + self.vertices.capacity() * 6
            + self.polygons.capacity() * std::mem::size_of::<Polygon>()
    }
}
struct Budget {
    limits: Limits,
    work: u64,
    bytes: usize,
}
impl Budget {
    fn charge(&mut self, n: u64) -> Result<(), Error> {
        self.work = self.work.checked_add(n).ok_or(Error::Limit)?;
        if self.work > self.limits.work {
            Err(Error::Limit)
        } else {
            Ok(())
        }
    }
    fn reserve<T>(&mut self, v: &mut Vec<T>, extra: usize) -> Result<(), Error> {
        let want = v.len().checked_add(extra).ok_or(Error::Limit)?;
        if want <= v.capacity() {
            return Ok(());
        }
        let size = std::mem::size_of::<T>();
        let old = v.capacity() * size;
        let capacity = want.max(v.capacity().saturating_mul(2)).max(8);
        let bytes = capacity.checked_mul(size).ok_or(Error::Limit)?;
        if self.bytes.checked_add(bytes).ok_or(Error::Limit)? > self.limits.heap_bytes {
            return Err(Error::Limit);
        }
        self.charge(v.len() as u64)?;
        v.try_reserve_exact(capacity - v.len())
            .map_err(|_| Error::Allocation)?;
        let new = v.capacity() * size;
        if self.bytes.checked_add(new).ok_or(Error::Limit)? > self.limits.heap_bytes {
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
    fn filled<T: Clone>(&mut self, n: usize, value: T) -> Result<Vec<T>, Error> {
        let mut v = Vec::new();
        self.reserve(&mut v, n)?;
        self.charge(n as u64)?;
        v.resize(n, value);
        Ok(v)
    }
    fn release<T>(&mut self, v: &Vec<T>) {
        self.bytes -= v.capacity() * std::mem::size_of::<T>();
    }
    fn store(&mut self, data: &mut Data, p: Polygon) -> Result<(), Error> {
        if data.polygons.len() >= data.maximum_polygons || data.polygons.len() >= 0x8000 {
            return Err(Error::Mesh);
        }
        self.push(&mut data.polygons, p)
    }
}
struct Data {
    vertices: Vec<[u16; 3]>,
    polygons: Vec<Polygon>,
    nvp: usize,
    maximum_polygons: usize,
}
impl ContourSet {
    /// Builds the original polygon stage with an explicit Detour-compatible
    /// vertex count (3..=6). Does not build detail triangles or Detour bytes.
    pub fn build_mesh(mut self, vertices_per_polygon: u8) -> Result<PolyMesh, Error> {
        if !(3..=6).contains(&vertices_per_polygon) {
            return Err(Error::Mesh);
        }
        let field = &self.regions.field.compact;
        let mut b = Budget {
            limits: field.limits,
            work: field.work,
            bytes: self.capacity_bytes(),
        };
        let (mut maximum_vertices, mut maximum_polygons) = (0usize, 0usize);
        for c in &self.contours {
            b.charge(1)?;
            if c.vertices.len() < 3 {
                continue;
            }
            maximum_vertices = maximum_vertices
                .checked_add(c.vertices.len())
                .ok_or(Error::Limit)?;
            maximum_polygons = maximum_polygons
                .checked_add(c.vertices.len() - 2)
                .ok_or(Error::Limit)?;
            for v in &c.vertices {
                b.charge(1)?;
                if !(0..=1024).contains(&v[0])
                    || !(0..=1024).contains(&v[2])
                    || !(0..=65535).contains(&v[1])
                {
                    return Err(Error::Numeric);
                }
            }
        }
        if maximum_vertices >= 0xfffe {
            return Err(Error::Mesh);
        }
        let mut data = Data {
            vertices: Vec::new(),
            polygons: Vec::new(),
            nvp: usize::from(vertices_per_polygon),
            maximum_polygons,
        };
        let mut first = b.filled(4096, NULL)?;
        let mut next = Vec::new();
        let mut flags = b.filled(maximum_vertices, 0u8)?;
        for c in &self.contours {
            b.charge(1)?;
            if c.vertices.len() < 3 {
                continue;
            }
            let tris = b.triangulate(&c.vertices)?;
            let mut indices = Vec::new();
            for v in &c.vertices {
                let id = b.add_vertex(
                    &mut data.vertices,
                    &mut first,
                    &mut next,
                    [v[0] as u16, v[1] as u16, v[2] as u16],
                )?;
                b.push(&mut indices, id)?;
                if v[3] & BORDER_VERTEX != 0 {
                    flags[usize::from(id)] = 1;
                }
            }
            let mut polys = Vec::new();
            for t in &tris {
                b.charge(1)?;
                // Native tests contour indices, before welding by coordinates.
                if t[0] != t[1] && t[0] != t[2] && t[1] != t[2] {
                    let mut p = Polygon::new(c.region, c.area);
                    for j in 0..3 {
                        p.vertices[j] = indices[t[j]];
                    }
                    b.push(&mut polys, p)?;
                }
            }
            b.merge_polygons(&data.vertices, &mut polys, data.nvp)?;
            for &p in &polys {
                b.store(&mut data, p)?;
            }
            b.release(&polys);
            b.release(&indices);
            b.release(&tris);
        }
        b.release(&first);
        drop(first);
        b.release(&next);
        drop(next);
        let mut i = 0;
        while i < data.vertices.len() {
            b.charge(1)?;
            if flags[i] != 0 && b.can_remove(&data, i as u16)? {
                b.remove_vertex(&mut data, i as u16)?;
                b.charge((flags.len() - i) as u64)?;
                flags.remove(i);
            } else {
                i += 1;
            }
        }
        b.release(&flags);
        drop(flags);
        b.adjacency(&mut data)?;
        if self.regions.options.border_size > 0 {
            for p in &mut data.polygons {
                let n = p.len();
                for i in 0..n {
                    b.charge(1)?;
                    if p.neighbors[i] != NULL {
                        continue;
                    }
                    let a = data.vertices[usize::from(p.vertices[i])];
                    let v = data.vertices[usize::from(p.vertices[(i + 1) % n])];
                    p.neighbors[i] = if a[0] == 0 && v[0] == 0 {
                        0x8000
                    } else if a[2] == self.grid.height && v[2] == self.grid.height {
                        0x8001
                    } else if a[0] == self.grid.width && v[0] == self.grid.width {
                        0x8002
                    } else if a[2] == 0 && v[2] == 0 {
                        0x8003
                    } else {
                        NULL
                    };
                }
            }
        }
        self.regions.field.compact.work = b.work;
        Ok(PolyMesh {
            contours: self,
            vertices: data.vertices,
            polygons: data.polygons,
            nvp: vertices_per_polygon,
            maximum_polygons,
        })
    }
}
impl Budget {
    fn add_vertex(
        &mut self,
        vertices: &mut Vec<[u16; 3]>,
        first: &mut [u16],
        next: &mut Vec<u16>,
        v: [u16; 3],
    ) -> Result<u16, Error> {
        let bucket = (u32::from(v[0])
            .wrapping_mul(0x8da6b343)
            .wrapping_add(u32::from(v[2]).wrapping_mul(0xcb1ab31f))
            & 4095) as usize;
        let mut at = first[bucket];
        while at != NULL {
            self.charge(1)?;
            let p = vertices[usize::from(at)];
            if p[0] == v[0] && p[2] == v[2] && p[1].abs_diff(v[1]) <= 2 {
                return Ok(at);
            }
            at = next[usize::from(at)];
        }
        let id = u16::try_from(vertices.len()).map_err(|_| Error::Mesh)?;
        if id >= 0xfffe {
            return Err(Error::Mesh);
        }
        self.push(vertices, v)?;
        self.push(next, first[bucket])?;
        first[bucket] = id;
        Ok(id)
    }
}

pub use detail::detour;
