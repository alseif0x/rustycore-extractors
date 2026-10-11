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

// Modified/ported 2026-10-11: original edge refinement, grid samples and jitter order.
use super::*;
#[derive(Clone, Copy)]
struct Sample {
    x: i32,
    z: i32,
    height: u16,
    added: bool,
}
fn jitter(index: usize, multiplier: u32) -> f32 {
    (((index as u32).wrapping_mul(multiplier) & 0xffff) as f32 / 65535.0 * 2.0) - 1.0
}
impl Budget {
    pub(super) fn detail_polygon(
        &mut self,
        input: &[Point],
        options: DetailOptions,
        radius: i32,
        field: &Compact,
        patch: &Patch,
    ) -> Result<LocalDetail, Error> {
        if input.len() < 3 || input.len() > MAX_NVP {
            return Err(Error::DetailTriangulation);
        }
        let distance = options.sample_distance;
        let error = options.maximum_error;
        let cs = field.grid.cell_size;
        let ch = field.grid.cell_height;
        let inverse = finite(1.0 / cs)?;
        let minimum = self.minimum_extent(input)?;
        let mut vertices = Vec::new();
        for &v in input {
            self.push(&mut vertices, v)?;
        }
        let mut hull = Vec::new();
        for i in 0..input.len() {
            self.charge(1)?;
            let j = (i + input.len() - 1) % input.len();
            let (mut a, mut b) = (input[j], input[i]);
            let mut swapped = false;
            if (a[0] - b[0]).abs() < 1e-6 {
                if a[2] > b[2] {
                    std::mem::swap(&mut a, &mut b);
                    swapped = true;
                }
            } else if a[0] > b[0] {
                std::mem::swap(&mut a, &mut b);
                swapped = true;
            }
            let delta = sub(b, a);
            let length = (delta[0] * delta[0] + delta[2] * delta[2]).sqrt();
            let mut nn = integer((length / distance).floor())?
                .checked_add(1)
                .ok_or(Error::Numeric)? as usize;
            if nn >= 32 {
                nn = 31;
            }
            if vertices.len() + nn >= 127 {
                nn = 126usize
                    .checked_sub(vertices.len())
                    .ok_or(Error::DetailLimit)?;
            }
            if nn == 0 {
                return Err(Error::DetailLimit);
            }
            let mut edge = [[0.0f32; 3]; 33];
            for (k, p) in edge.iter_mut().enumerate().take(nn + 1) {
                self.charge(1)?;
                let u = k as f32 / nn as f32;
                *p = [
                    finite(a[0] + delta[0] * u)?,
                    finite(a[1] + delta[1] * u)?,
                    finite(a[2] + delta[2] * u)?,
                ];
                p[1] = finite(f32::from(self.sample_height(*p, inverse, ch, radius, patch)?) * ch)?;
            }
            let mut indices = Vec::new();
            self.push(&mut indices, 0usize)?;
            self.push(&mut indices, nn)?;
            let mut k = 0;
            while k + 1 < indices.len() {
                self.charge(1)?;
                let (a, b) = (indices[k], indices[k + 1]);
                let mut maximum = 0.0;
                let mut chosen = None;
                for m in a + 1..b {
                    self.charge(1)?;
                    let deviation = segment3(edge[m], edge[a], edge[b]);
                    if deviation > maximum {
                        maximum = deviation;
                        chosen = Some(m);
                    }
                }
                if let Some(m) = chosen.filter(|_| maximum > error * error) {
                    self.reserve(&mut indices, 1)?;
                    self.charge((indices.len() - k) as u64)?;
                    indices.insert(k + 1, m);
                } else {
                    k += 1;
                }
            }
            self.push(&mut hull, j)?;
            for k in 1..indices.len() - 1 {
                let id = if swapped {
                    indices[indices.len() - 1 - k]
                } else {
                    indices[k]
                };
                self.push(&mut hull, vertices.len())?;
                self.push(&mut vertices, edge[id])?;
            }
            self.release(&indices);
        }
        let mut tris = self.hull_triangles(&vertices, &hull, input.len())?;
        if minimum < distance * 2.0 {
            self.release(&hull);
            return Ok(LocalDetail {
                vertices,
                triangles: tris,
                discarded_faces: 0,
            });
        }
        if tris.is_empty() {
            return Err(Error::DetailTriangulation);
        }
        let (mut min, mut max) = (input[0], input[0]);
        for &v in &input[1..] {
            self.charge(1)?;
            for axis in 0..3 {
                if v[axis] < min[axis] {
                    min[axis] = v[axis];
                }
                if v[axis] > max[axis] {
                    max[axis] = v[axis];
                }
            }
        }
        let x0 = integer((min[0] / distance).floor())?;
        let x1 = integer((max[0] / distance).ceil())?;
        let z0 = integer((min[2] / distance).floor())?;
        let z1 = integer((max[2] / distance).ceil())?;
        let rows = u64::try_from(i64::from(z1) - i64::from(z0)).map_err(|_| Error::Numeric)?;
        let columns = u64::try_from(i64::from(x1) - i64::from(x0)).map_err(|_| Error::Numeric)?;
        self.charge(rows.checked_mul(columns).ok_or(Error::Limit)?)?;
        let mut samples = Vec::new();
        for z in z0..z1 {
            for x in x0..x1 {
                let p = [
                    finite(x as f32 * distance)?,
                    finite((max[1] + min[1]) * 0.5)?,
                    finite(z as f32 * distance)?,
                ];
                // Original compares squared distance to a linear half-spacing.
                if self.poly_distance(input, p)? > -distance / 2.0 {
                    continue;
                }
                let height = self.sample_height(p, inverse, ch, radius, patch)?;
                self.push(
                    &mut samples,
                    Sample {
                        x,
                        z,
                        height,
                        added: false,
                    },
                )?;
            }
        }
        let mut discarded_faces = 0u32;
        for _ in 0..samples.len() {
            self.charge(1)?;
            if vertices.len() >= 127 {
                break;
            }
            let mut best_point = [0.0; 3];
            let mut best_distance = 0.0;
            let mut best = None;
            for (i, s) in samples.iter().enumerate() {
                self.charge(1)?;
                if s.added {
                    continue;
                }
                let p = [
                    finite(s.x as f32 * distance + jitter(i, 0x8da6b343) * cs * 0.1)?,
                    finite(f32::from(s.height) * ch)?,
                    finite(s.z as f32 * distance + jitter(i, 0xd8163841) * cs * 0.1)?,
                ];
                let d = self.mesh_distance(p, &vertices, &tris)?;
                if d < 0.0 {
                    continue;
                }
                if d > best_distance {
                    best_distance = d;
                    best = Some(i);
                    best_point = p;
                }
            }
            if best_distance <= error || best.is_none() {
                break;
            }
            samples[best.expect("selected sample")].added = true;
            self.push(&mut vertices, best_point)?;
            let next = self.delaunay(&vertices, &hull)?;
            discarded_faces = discarded_faces
                .checked_add(next.discarded_faces)
                .ok_or(Error::Limit)?;
            self.release(&tris);
            tris = next.triangles;
        }
        if tris.len() > 255 {
            return Err(Error::DetailLimit);
        }
        self.release(&samples);
        self.release(&hull);
        Ok(LocalDetail {
            vertices,
            triangles: tris,
            discarded_faces,
        })
    }
}
