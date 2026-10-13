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

// Modified/ported 2026-10-11: original ordered ear clipping with strict completion.
use super::*;
const MASK: u32 = 0x0fffffff;
const EAR: u32 = 0x80000000;
fn prev(i: usize, n: usize) -> usize {
    if i == 0 { n - 1 } else { i - 1 }
}
fn next(i: usize, n: usize) -> usize {
    if i + 1 < n { i + 1 } else { 0 }
}
fn point(vertices: &[Vertex], indices: &[u32], i: usize) -> Vertex {
    vertices[(indices[i] & MASK) as usize]
}
fn cross(a: Vertex, b: Vertex, c: Vertex) -> i32 {
    (b[0] - a[0]) * (c[2] - a[2]) - (c[0] - a[0]) * (b[2] - a[2])
}
fn same(a: Vertex, b: Vertex) -> bool {
    a[0] == b[0] && a[2] == b[2]
}
fn proper(a: Vertex, b: Vertex, c: Vertex, d: Vertex) -> bool {
    let (x, y, z, w) = (
        cross(a, b, c),
        cross(a, b, d),
        cross(c, d, a),
        cross(c, d, b),
    );
    x != 0 && y != 0 && z != 0 && w != 0 && (x < 0) != (y < 0) && (z < 0) != (w < 0)
}
fn between(a: Vertex, b: Vertex, c: Vertex) -> bool {
    if cross(a, b, c) != 0 {
        return false;
    }
    let i = if a[0] != b[0] { 0 } else { 2 };
    (a[i] <= c[i] && c[i] <= b[i]) || (b[i] <= c[i] && c[i] <= a[i])
}
impl Budget {
    fn diagonal(
        &mut self,
        vertices: &[Vertex],
        indices: &[u32],
        i: usize,
        j: usize,
        loose: bool,
    ) -> Result<bool, Error> {
        self.charge(1)?;
        let n = indices.len();
        let (a, b, c, d) = (
            point(vertices, indices, i),
            point(vertices, indices, j),
            point(vertices, indices, next(i, n)),
            point(vertices, indices, prev(i, n)),
        );
        let inside = if cross(d, a, c) <= 0 {
            if loose {
                cross(a, b, d) <= 0 && cross(b, a, c) <= 0
            } else {
                cross(a, b, d) < 0 && cross(b, a, c) < 0
            }
        } else {
            !(cross(a, b, c) <= 0 && cross(b, a, d) <= 0)
        };
        if !inside {
            return Ok(false);
        }
        for k in 0..n {
            self.charge(1)?;
            let k1 = next(k, n);
            if k == i || k1 == i || k == j || k1 == j {
                continue;
            }
            let (c, d) = (point(vertices, indices, k), point(vertices, indices, k1));
            if same(a, c) || same(b, c) || same(a, d) || same(b, d) {
                continue;
            }
            if proper(a, b, c, d)
                || (!loose
                    && (between(a, b, c)
                        || between(a, b, d)
                        || between(c, d, a)
                        || between(c, d, b)))
            {
                return Ok(false);
            }
        }
        Ok(true)
    }
    pub(super) fn triangulate(&mut self, vertices: &[Vertex]) -> Result<Vec<[usize; 3]>, Error> {
        if vertices.len() < 3 {
            return Err(Error::Triangulation);
        }
        let mut indices = Vec::new();
        for i in 0..vertices.len() {
            self.push(&mut indices, i as u32)?;
        }
        let n = indices.len();
        for i in 0..n {
            let i1 = next(i, n);
            let i2 = next(i1, n);
            if self.diagonal(vertices, &indices, i, i2, false)? {
                indices[i1] |= EAR;
            }
        }
        let mut tris = Vec::new();
        while indices.len() > 3 {
            let n = indices.len();
            let mut best = None;
            for i in 0..n {
                self.charge(1)?;
                let i1 = next(i, n);
                if indices[i1] & EAR != 0 {
                    let a = point(vertices, &indices, i);
                    let b = point(vertices, &indices, next(i1, n));
                    let dx = b[0] - a[0];
                    let dz = b[2] - a[2];
                    let length = dx * dx + dz * dz;
                    if best.is_none_or(|(d, _)| length < d) {
                        best = Some((length, i));
                    }
                }
            }
            if best.is_none() {
                for i in 0..n {
                    let i2 = next(next(i, n), n);
                    if self.diagonal(vertices, &indices, i, i2, true)? {
                        // The original loose fallback scores next(i2), not i2.
                        let a = point(vertices, &indices, i);
                        let b = point(vertices, &indices, next(i2, n));
                        let dx = b[0] - a[0];
                        let dz = b[2] - a[2];
                        let length = dx * dx + dz * dz;
                        if best.is_none_or(|(d, _)| length < d) {
                            best = Some((length, i));
                        }
                    }
                }
            }
            let (_, i) = best.ok_or(Error::Triangulation)?;
            let mut i1 = next(i, n);
            let i2 = next(i1, n);
            self.push(
                &mut tris,
                [
                    (indices[i] & MASK) as usize,
                    (indices[i1] & MASK) as usize,
                    (indices[i2] & MASK) as usize,
                ],
            )?;
            self.charge((n - i1) as u64)?;
            indices.remove(i1);
            let n = indices.len();
            if i1 >= n {
                i1 = 0;
            }
            let i = prev(i1, n);
            if self.diagonal(vertices, &indices, prev(i, n), i1, false)? {
                indices[i] |= EAR;
            } else {
                indices[i] &= MASK;
            }
            if self.diagonal(vertices, &indices, i, next(i1, n), false)? {
                indices[i1] |= EAR;
            } else {
                indices[i1] &= MASK;
            }
        }
        self.push(
            &mut tris,
            [
                (indices[0] & MASK) as usize,
                (indices[1] & MASK) as usize,
                (indices[2] & MASK) as usize,
            ],
        )?;
        self.release(&indices);
        Ok(tris)
    }
}
