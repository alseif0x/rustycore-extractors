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

// Modified/ported 2026-10-11: original single-precision detail geometry operations.
use super::*;
pub(super) fn sub(a: Point, b: Point) -> Point {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
pub(super) fn dot2(a: Point, b: Point) -> f32 {
    a[0] * b[0] + a[2] * b[2]
}
pub(super) fn distance2(a: Point, b: Point) -> f32 {
    let x = b[0] - a[0];
    let z = b[2] - a[2];
    (x * x + z * z).sqrt()
}
pub(super) fn cross2(a: Point, b: Point, c: Point) -> f32 {
    (b[0] - a[0]) * (c[2] - a[2]) - (b[2] - a[2]) * (c[0] - a[0])
}
pub(super) fn circle(a: Point, b: Point, c: Point) -> (Point, f32) {
    let zero = [0.0; 3];
    let b = sub(b, a);
    let c = sub(c, a);
    let cp = cross2(zero, b, c);
    if cp.abs() > 1e-6 {
        let s0 = dot2(zero, zero);
        let s1 = dot2(b, b);
        let s2 = dot2(c, c);
        let x = (s0 * (b[2] - c[2]) + s1 * (c[2] - zero[2]) + s2 * (zero[2] - b[2])) / (2.0 * cp);
        let z = (s0 * (c[0] - b[0]) + s1 * (zero[0] - c[0]) + s2 * (b[0] - zero[0])) / (2.0 * cp);
        let p = [x, 0.0, z];
        let r = distance2(p, zero);
        ([p[0] + a[0], p[1] + a[1], p[2] + a[2]], r)
    } else {
        (a, 0.0)
    }
}
pub(super) fn segment2(pt: Point, p: Point, q: Point) -> f32 {
    let px = q[0] - p[0];
    let pz = q[2] - p[2];
    let x = pt[0] - p[0];
    let z = pt[2] - p[2];
    let d = px * px + pz * pz;
    let mut t = px * x + pz * z;
    if d > 0.0 {
        t /= d;
    }
    t = t.clamp(0.0, 1.0);
    let x = p[0] + t * px - pt[0];
    let z = p[2] + t * pz - pt[2];
    x * x + z * z
}
pub(super) fn segment3(pt: Point, p: Point, q: Point) -> f32 {
    let pq = sub(q, p);
    let d = sub(pt, p);
    let length = pq[0] * pq[0] + pq[1] * pq[1] + pq[2] * pq[2];
    let mut t = pq[0] * d[0] + pq[1] * d[1] + pq[2] * d[2];
    if length > 0.0 {
        t /= length;
    }
    t = t.clamp(0.0, 1.0);
    let x = p[0] + t * pq[0] - pt[0];
    let y = p[1] + t * pq[1] - pt[1];
    let z = p[2] + t * pq[2] - pt[2];
    x * x + y * y + z * z
}
pub(super) fn triangle_height(p: Point, a: Point, b: Point, c: Point) -> f32 {
    let v0 = sub(c, a);
    let v1 = sub(b, a);
    let v2 = sub(p, a);
    let d00 = dot2(v0, v0);
    let d01 = dot2(v0, v1);
    let d02 = dot2(v0, v2);
    let d11 = dot2(v1, v1);
    let d12 = dot2(v1, v2);
    let inv = 1.0 / (d00 * d11 - d01 * d01);
    let u = (d11 * d02 - d01 * d12) * inv;
    let v = (d00 * d12 - d01 * d02) * inv;
    if u >= -1e-4 && v >= -1e-4 && u + v <= 1.0 + 1e-4 {
        (a[1] + v0[1] * u + v1[1] * v - p[1]).abs()
    } else {
        f32::MAX
    }
}
impl Budget {
    pub(super) fn minimum_extent(&mut self, vertices: &[Point]) -> Result<f32, Error> {
        let mut minimum = f32::MAX;
        for i in 0..vertices.len() {
            let next = (i + 1) % vertices.len();
            let mut maximum = 0.0f32;
            for j in 0..vertices.len() {
                self.charge(1)?;
                if j == i || j == next {
                    continue;
                }
                let d = segment2(vertices[j], vertices[i], vertices[next]);
                if d > maximum {
                    maximum = d;
                }
            }
            if maximum < minimum {
                minimum = maximum;
            }
        }
        finite(minimum.sqrt())
    }
    pub(super) fn poly_distance(&mut self, vertices: &[Point], p: Point) -> Result<f32, Error> {
        let mut minimum = f32::MAX;
        let mut inside = false;
        for i in 0..vertices.len() {
            self.charge(1)?;
            let a = vertices[i];
            let b = vertices[(i + vertices.len() - 1) % vertices.len()];
            if (a[2] > p[2]) != (b[2] > p[2])
                && p[0] < (b[0] - a[0]) * (p[2] - a[2]) / (b[2] - a[2]) + a[0]
            {
                inside = !inside;
            }
            let d = segment2(p, b, a);
            if d < minimum {
                minimum = d;
            }
        }
        Ok(if inside { -minimum } else { minimum })
    }
    pub(super) fn mesh_distance(
        &mut self,
        p: Point,
        vertices: &[Point],
        tris: &[Tri],
    ) -> Result<f32, Error> {
        let mut minimum = f32::MAX;
        for t in tris {
            self.charge(1)?;
            let d = triangle_height(p, vertices[t[0]], vertices[t[1]], vertices[t[2]]);
            if d < minimum {
                minimum = d;
            }
        }
        Ok(if minimum == f32::MAX { -1.0 } else { minimum })
    }
}
