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

// Modified/ported 2026-10-11: checked convex clipping and index-arena raster admission.
// Recast algorithm reference and attribution are recorded in the parent module.
use super::*;

/// Native comparisons deliberately leave degenerate/NaN normals at steep area
/// 10: neither the clear comparison nor the non-steep comparison succeeds.
pub fn solid_area(points: [[f32; 3]; 3], slopes: Slopes) -> Result<u8, Error> {
    if !points.iter().flatten().all(|v| v.is_finite()) {
        return Err(Error::Geometry);
    }
    Ok(area(points, slopes.thresholds()?))
}
fn area(p: [[f32; 3]; 3], threshold: [f32; 2]) -> u8 {
    let a: [f32; 3] = std::array::from_fn(|i| p[1][i] - p[0][i]);
    let b: [f32; 3] = std::array::from_fn(|i| p[2][i] - p[0][i]);
    let n = [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ];
    let scale = 1.0 / ((n[0] * n[0] + n[1] * n[1]) + n[2] * n[2]).sqrt();
    let y = n[1] * scale;
    let mut result = 10;
    if y <= threshold[0] {
        result = 0;
    }
    if y > threshold[1] {
        result = 11;
    }
    result
}

impl Heightfield {
    fn validate_geometry(
        &mut self,
        vertices: &[[f32; 3]],
        triangles: &[[u32; 3]],
    ) -> Result<(), Error> {
        if vertices.len() > self.limits.vertices || triangles.len() > self.limits.triangles {
            return Err(Error::Limit);
        }
        self.charge(vertices.len() as u64 + triangles.len() as u64 * 4)?;
        if vertices.iter().flatten().any(|v| !v.is_finite())
            || triangles
                .iter()
                .flatten()
                .any(|&v| v as usize >= vertices.len())
        {
            return Err(Error::Geometry);
        }
        Ok(())
    }
    pub fn rasterize(
        mut self,
        vertices: &[[f32; 3]],
        triangles: &[[u32; 3]],
        areas: &[u8],
        merge_threshold: u16,
    ) -> Result<Self, Error> {
        self.validate_geometry(vertices, triangles)?;
        if areas.len() != triangles.len() || areas.iter().any(|&a| a > 63) {
            return Err(Error::Geometry);
        }
        for (triangle, &a) in triangles.iter().zip(areas) {
            self.triangle(triangle.map(|v| vertices[v as usize]), a, merge_threshold)?;
        }
        Ok(self)
    }
    pub fn rasterize_solid(
        mut self,
        vertices: &[[f32; 3]],
        triangles: &[[u32; 3]],
        slopes: Slopes,
        merge_threshold: u16,
    ) -> Result<Self, Error> {
        let thresholds = slopes.thresholds()?;
        self.validate_geometry(vertices, triangles)?;
        for tri in triangles {
            let points = tri.map(|v| vertices[v as usize]);
            self.triangle(points, area(points, thresholds), merge_threshold)?;
        }
        Ok(self)
    }
    fn triangle(&mut self, points: [[f32; 3]; 3], area: u8, threshold: u16) -> Result<(), Error> {
        self.charge(1)?;
        let g = self.grid;
        let mut low = points[0];
        let mut high = points[0];
        for p in &points[1..] {
            for i in 0..3 {
                if p[i] < low[i] {
                    low[i] = p[i];
                }
                if p[i] > high[i] {
                    high[i] = p[i];
                }
            }
        }
        if (0..3).any(|i| g.min[i] > high[i] || g.max[i] < low[i]) {
            return Ok(());
        }
        let inv = 1.0 / g.cell_size;
        let invh = 1.0 / g.cell_height;
        let vertical = g.max[1] - g.min[1];
        let z0 = integer((low[2] - g.min[2]) * inv)?.clamp(0, i32::from(g.height) - 1);
        let z1 = integer((high[2] - g.min[2]) * inv)?.clamp(0, i32::from(g.height) - 1);
        let mut tail = Poly::triangle(points);
        for z in z0..=z1 {
            self.charge(1 + tail.len as u64)?;
            let plane = (g.min[2] + z as f32 * g.cell_size) + g.cell_size;
            let (mut row, rest) = tail.divide(finite(plane)?, 2)?;
            tail = rest;
            if row.len < 3 {
                continue;
            }
            let mut min = row.points[0][0];
            let mut max = min;
            for p in &row.points[1..row.len] {
                if min > p[0] {
                    min = p[0];
                }
                if max < p[0] {
                    max = p[0];
                }
            }
            let x0 = integer((min - g.min[0]) * inv)?.clamp(0, i32::from(g.width) - 1);
            let x1 = integer((max - g.min[0]) * inv)?.clamp(0, i32::from(g.width) - 1);
            for x in x0..=x1 {
                self.charge(1 + row.len as u64)?;
                let plane = (g.min[0] + x as f32 * g.cell_size) + g.cell_size;
                let (cell, rest) = row.divide(finite(plane)?, 0)?;
                row = rest;
                if cell.len < 3 {
                    continue;
                }
                let mut lo = cell.points[0][1];
                let mut hi = lo;
                for p in &cell.points[1..cell.len] {
                    lo = if lo < p[1] { lo } else { p[1] };
                    hi = if hi > p[1] { hi } else { p[1] };
                }
                lo = finite(lo - g.min[1])?;
                hi = finite(hi - g.min[1])?;
                if hi < 0.0 || lo > vertical {
                    continue;
                }
                if lo < 0.0 {
                    lo = 0.0;
                }
                if hi > vertical {
                    hi = vertical;
                }
                let lower = integer((lo * invh).floor())?.clamp(0, 65535);
                // Native lower+1 at 65535 overflows its u16 upper endpoint.
                if lower == 65535 {
                    return Err(Error::Numeric);
                }
                let upper = integer((hi * invh).ceil())?.clamp(lower + 1, 65535);
                self.insert(
                    x as usize + z as usize * usize::from(g.width),
                    Span {
                        min: lower as u16,
                        max: upper as u16,
                        area,
                    },
                    threshold,
                )?;
            }
        }
        Ok(())
    }
}
#[derive(Clone, Copy)]
struct Poly {
    points: [[f32; 3]; 7],
    len: usize,
}
impl Poly {
    fn empty() -> Self {
        Self {
            points: [[0.0; 3]; 7],
            len: 0,
        }
    }
    fn triangle(p: [[f32; 3]; 3]) -> Self {
        let mut v = Self::empty();
        v.points[..3].copy_from_slice(&p);
        v.len = 3;
        v
    }
    fn push(&mut self, p: [f32; 3]) -> Result<(), Error> {
        if self.len == 7 {
            return Err(Error::Clip);
        }
        if !p.iter().all(|v| v.is_finite()) {
            return Err(Error::Numeric);
        }
        self.points[self.len] = p;
        self.len += 1;
        Ok(())
    }
    fn divide(self, plane: f32, axis: usize) -> Result<(Self, Self), Error> {
        let mut inside = Self::empty();
        let mut outside = Self::empty();
        let mut distances = [0.0; 7];
        for (i, d) in distances.iter_mut().enumerate().take(self.len) {
            *d = finite(plane - self.points[i][axis])?;
        }
        for i in 0..self.len {
            let j = (i + self.len - 1) % self.len;
            let (a, b) = (distances[j], distances[i]);
            if (a >= 0.0) != (b >= 0.0) {
                let s = finite(a / finite(a - b)?)?;
                let p = std::array::from_fn(|k| {
                    self.points[j][k] + (self.points[i][k] - self.points[j][k]) * s
                });
                inside.push(p)?;
                outside.push(p)?;
                if b > 0.0 {
                    inside.push(self.points[i])?;
                } else if b < 0.0 {
                    outside.push(self.points[i])?;
                }
            } else {
                if b >= 0.0 {
                    inside.push(self.points[i])?;
                    if b != 0.0 {
                        continue;
                    }
                }
                outside.push(self.points[i])?;
            }
        }
        Ok((inside, outside))
    }
}
