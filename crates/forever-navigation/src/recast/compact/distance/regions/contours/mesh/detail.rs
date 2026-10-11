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

// Modified/ported 2026-10-11: checked original polygon detail construction.
use super::*;
mod height;
mod math;
mod sample;
#[cfg(test)]
mod tests;
mod tiles;
mod triangulate;
use height::Patch;
use math::*;
pub use tiles::{MergeLimits, MeshHeader, TileMesh};
type Point = [f32; 3];
type Tri = [usize; 3];
struct Triangulation {
    triangles: Vec<Tri>,
    discarded_faces: u32,
}
struct LocalDetail {
    vertices: Vec<Point>,
    triangles: Vec<Tri>,
    discarded_faces: u32,
}
#[derive(Clone, Copy, Debug)]
pub struct DetailOptions {
    /// World units; positive, finite. TileBuilder normally chooses cell_size*16.
    pub sample_distance: f32,
    /// World units; nonnegative, finite. TileBuilder normally chooses cell_height.
    pub maximum_error: f32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DetailSubmesh {
    pub vertex_base: u32,
    pub vertex_count: u32,
    pub triangle_base: u32,
    pub triangle_count: u32,
}
pub struct DetailMesh {
    mesh: PolyMesh,
    options: DetailOptions,
    submeshes: Vec<DetailSubmesh>,
    vertices: Vec<Point>,
    triangles: Vec<[u8; 4]>,
    discarded_faces: u32,
}
impl DetailMesh {
    pub fn polygon_mesh(&self) -> &PolyMesh {
        &self.mesh
    }
    pub const fn options(&self) -> DetailOptions {
        self.options
    }
    pub fn submeshes(&self) -> &[DetailSubmesh] {
        &self.submeshes
    }
    /// Native world coordinates, including the extra cell-height y offset.
    pub fn vertices(&self) -> &[Point] {
        &self.vertices
    }
    /// Three indices local to the submesh, followed by edge flags in bits 0,2,4.
    pub fn triangles(&self) -> &[[u8; 4]] {
        &self.triangles
    }
    /// Number of native dangling-face recovery warnings across all rebuilds.
    pub const fn discarded_faces(&self) -> u32 {
        self.discarded_faces
    }
    pub fn capacity_bytes(&self) -> usize {
        self.mesh.capacity_bytes()
            + self.submeshes.capacity() * 16
            + self.vertices.capacity() * 12
            + self.triangles.capacity() * 4
    }
}
impl PolyMesh {
    pub fn build_detail(mut self, options: DetailOptions) -> Result<DetailMesh, Error> {
        if !options.sample_distance.is_finite()
            || options.sample_distance <= 0.0
            || !options.maximum_error.is_finite()
            || options.maximum_error < 0.0
            || !(options.maximum_error * options.maximum_error).is_finite()
        {
            return Err(Error::Numeric);
        }
        let field = &self.contours.regions.field.compact;
        let mut b = Budget {
            limits: field.limits,
            work: field.work,
            bytes: self.capacity_bytes(),
        };
        let grid = self.contours.grid;
        let radius = integer(self.contours.options.maximum_error.ceil())?.max(1);
        if radius > 23169 {
            return Err(Error::Numeric);
        }
        let mut submeshes = Vec::new();
        let mut vertices = Vec::new();
        let mut triangles = Vec::new();
        let mut discarded_faces = 0u32;
        for p in &self.polygons {
            b.charge(1)?;
            let mut poly = Vec::new();
            for &id in p.vertices() {
                let v = self.vertices[usize::from(id)];
                b.push(
                    &mut poly,
                    [
                        finite(f32::from(v[0]) * grid.cell_size)?,
                        finite(f32::from(v[1]) * grid.cell_height)?,
                        finite(f32::from(v[2]) * grid.cell_size)?,
                    ],
                )?;
            }
            let patch = b.height_patch(
                field,
                &self.vertices,
                p,
                self.contours.regions.options.border_size,
            )?;
            let local = b.detail_polygon(&poly, options, radius, field, &patch)?;
            discarded_faces = discarded_faces
                .checked_add(local.discarded_faces)
                .ok_or(Error::Limit)?;
            let mut points = local.vertices;
            let tris = local.triangles;
            if points.len() > 127 || tris.len() > 255 {
                return Err(Error::DetailLimit);
            }
            for v in &mut points {
                b.charge(1)?;
                v[0] = finite(v[0] + grid.min[0])?;
                v[1] = finite(v[1] + finite(grid.min[1] + field.grid.cell_height)?)?;
                v[2] = finite(v[2] + grid.min[2])?;
            }
            for v in &mut poly {
                b.charge(1)?;
                for (value, origin) in v.iter_mut().zip(grid.min) {
                    *value = finite(*value + origin)?;
                }
            }
            b.push(
                &mut submeshes,
                DetailSubmesh {
                    vertex_base: u32::try_from(vertices.len()).map_err(|_| Error::Limit)?,
                    vertex_count: points.len() as u32,
                    triangle_base: u32::try_from(triangles.len()).map_err(|_| Error::Limit)?,
                    triangle_count: tris.len() as u32,
                },
            )?;
            for &v in &points {
                b.push(&mut vertices, v)?;
            }
            for t in &tris {
                if t.iter().any(|&i| i >= points.len()) {
                    return Err(Error::DetailTriangulation);
                }
                let mut flags = 0;
                for e in 0..3 {
                    for i in 0..poly.len() {
                        b.charge(1)?;
                        let j = (i + poly.len() - 1) % poly.len();
                        if segment2(points[t[e]], poly[j], poly[i]) < 0.001 * 0.001
                            && segment2(points[t[(e + 1) % 3]], poly[j], poly[i]) < 0.001 * 0.001
                        {
                            flags |= 1 << (e * 2);
                            break;
                        }
                    }
                }
                b.push(&mut triangles, [t[0] as u8, t[1] as u8, t[2] as u8, flags])?;
            }
            b.release(&poly);
            b.release(&patch.data);
            b.release(&points);
            b.release(&tris);
        }
        self.contours.regions.field.compact.work = b.work;
        Ok(DetailMesh {
            mesh: self,
            options,
            submeshes,
            vertices,
            triangles,
            discarded_faces,
        })
    }
}
fn integer(v: f32) -> Result<i32, Error> {
    if !v.is_finite() || f64::from(v) < f64::from(i32::MIN) || f64::from(v) > f64::from(i32::MAX) {
        Err(Error::Numeric)
    } else {
        Ok(v as i32)
    }
}

pub use tiles::detour;
