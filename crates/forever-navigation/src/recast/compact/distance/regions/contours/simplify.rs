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

// Modified/ported 2026-10-11: native f32 deviation, ordered subdivision and flags.
use super::*;
fn distance(p: Vertex, a: Vertex, b: Vertex) -> f32 {
    let pqx = (b[0] - a[0]) as f32;
    let pqz = (b[2] - a[2]) as f32;
    let dx = (p[0] - a[0]) as f32;
    let dz = (p[2] - a[2]) as f32;
    let d = pqx * pqx + pqz * pqz;
    let mut t = pqx * dx + pqz * dz;
    if d > 0.0 {
        t /= d;
    }
    t = t.clamp(0.0, 1.0);
    let dx = (a[0] as f32 + t * pqx) - p[0] as f32;
    let dz = (a[2] as f32 + t * pqz) - p[2] as f32;
    dx * dx + dz * dz
}
fn lexical(a: Vertex, b: Vertex) -> bool {
    b[0] > a[0] || (b[0] == a[0] && b[2] > a[2])
}
fn indexed(p: Vertex, i: usize) -> Vertex {
    [p[0], p[1], p[2], i as i32]
}
impl ContourBuilder {
    pub(super) fn simplify(
        &mut self,
        points: &[Vertex],
        options: ContourOptions,
    ) -> Result<Vec<Vertex>, Error> {
        if points.is_empty() {
            return Err(Error::Contour);
        }
        let n = points.len();
        let mut out = Vec::new();
        self.charge(n as u64)?;
        if points.iter().any(|p| p[3] & REGION_MASK != 0) {
            for i in 0..n {
                self.charge(1)?;
                let p = points[i];
                let q = points[(i + 1) % n];
                if p[3] & REGION_MASK != q[3] & REGION_MASK
                    || p[3] & AREA_BORDER != q[3] & AREA_BORDER
                {
                    self.push(&mut out, indexed(p, i))?;
                }
            }
        }
        if out.is_empty() {
            let (mut lo, mut hi) = (0, 0);
            for i in 0..n {
                self.charge(1)?;
                if lexical(points[i], points[lo]) {
                    lo = i;
                }
                if lexical(points[hi], points[i]) {
                    hi = i;
                }
            }
            self.push(&mut out, indexed(points[lo], lo))?;
            self.push(&mut out, indexed(points[hi], hi))?;
        }
        let mut i = 0;
        while i < out.len() {
            self.charge(1)?;
            let mut a = out[i];
            let mut b = out[(i + 1) % out.len()];
            let (ai, bi) = (a[3] as usize, b[3] as usize);
            let (step, mut at, end) = if lexical(a, b) {
                (1, (ai + 1) % n, bi)
            } else {
                std::mem::swap(&mut a, &mut b);
                (n - 1, (bi + n - 1) % n, ai)
            };
            let (mut maximum, mut chosen) = (0.0, None);
            if points[at][3] & REGION_MASK == 0 || points[at][3] & AREA_BORDER != 0 {
                while at != end {
                    self.charge(1)?;
                    let d = distance(points[at], a, b);
                    if d > maximum {
                        maximum = d;
                        chosen = Some(at);
                    }
                    at = (at + step) % n;
                }
            }
            if let Some(chosen) =
                chosen.filter(|_| maximum > options.maximum_error * options.maximum_error)
            {
                self.insert(&mut out, i + 1, indexed(points[chosen], chosen))?;
            } else {
                i += 1;
            }
        }
        if options.maximum_edge_length > 0 && (options.tessellate_walls || options.tessellate_areas)
        {
            let mut i = 0;
            while i < out.len() {
                self.charge(1)?;
                let a = out[i];
                let b = out[(i + 1) % out.len()];
                let (ai, bi) = (a[3] as usize, b[3] as usize);
                let p = points[(ai + 1) % n];
                let tess = (options.tessellate_walls && p[3] & REGION_MASK == 0)
                    || (options.tessellate_areas && p[3] & AREA_BORDER != 0);
                let (dx, dz) = (b[0] - a[0], b[2] - a[2]);
                let edge = i32::from(options.maximum_edge_length);
                let count = if bi < ai { bi + n - ai } else { bi - ai };
                if tess && dx * dx + dz * dz > edge * edge && count > 1 {
                    let chosen =
                        (ai + if lexical(a, b) {
                            count / 2
                        } else {
                            count.div_ceil(2)
                        }) % n;
                    self.insert(&mut out, i + 1, indexed(points[chosen], chosen))?;
                } else {
                    i += 1;
                }
            }
        }
        for p in &mut out {
            self.charge(1)?;
            let b = p[3] as usize;
            let a = (b + 1) % n;
            p[3] = (points[a][3] & (REGION_MASK | AREA_BORDER)) | (points[b][3] & BORDER_VERTEX);
        }
        let mut i = 0;
        while i < out.len() {
            self.charge(1)?;
            let next = (i + 1) % out.len();
            if out[i][0] == out[next][0] && out[i][2] == out[next][2] {
                self.charge((out.len() - i) as u64)?;
                out.remove(i);
            }
            i += 1;
        }
        Ok(out)
    }
}
