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

// Modified/ported 2026-10-11: bounded native hole joins, preserving defined indexed behavior.
use super::*;
#[derive(Clone, Copy)]
struct Hole {
    contour: usize,
    left: usize,
    x: i32,
    z: i32,
}
struct Group {
    outline: Option<usize>,
    holes: Vec<Hole>,
}
#[derive(Clone, Copy)]
struct Diagonal {
    vertex: usize,
    distance: i32,
}
fn cross(a: Vertex, b: Vertex, c: Vertex) -> i32 {
    (b[0] - a[0]) * (c[2] - a[2]) - (c[0] - a[0]) * (b[2] - a[2])
}
fn equal(a: Vertex, b: Vertex) -> bool {
    a[0] == b[0] && a[2] == b[2]
}
fn between(a: Vertex, b: Vertex, c: Vertex) -> bool {
    if cross(a, b, c) != 0 {
        return false;
    }
    let axis = if a[0] != b[0] { 0 } else { 2 };
    (a[axis] <= c[axis] && c[axis] <= b[axis]) || (a[axis] >= c[axis] && c[axis] >= b[axis])
}
fn intersect(a: Vertex, b: Vertex, c: Vertex, d: Vertex) -> bool {
    let (ab_c, ab_d, cd_a, cd_b) = (
        cross(a, b, c),
        cross(a, b, d),
        cross(c, d, a),
        cross(c, d, b),
    );
    if ab_c != 0
        && ab_d != 0
        && cd_a != 0
        && cd_b != 0
        && (ab_c < 0) != (ab_d < 0)
        && (cd_a < 0) != (cd_b < 0)
    {
        return true;
    }
    between(a, b, c) || between(a, b, d) || between(c, d, a) || between(c, d, b)
}
fn in_cone(vertices: &[Vertex], i: usize, p: Vertex) -> bool {
    let n = vertices.len();
    let v = vertices[i];
    let next = vertices[(i + 1) % n];
    let prev = vertices[(i + n - 1) % n];
    if cross(prev, v, next) <= 0 {
        cross(v, p, prev) < 0 && cross(p, v, next) < 0
    } else {
        !(cross(v, p, next) <= 0 && cross(p, v, prev) <= 0)
    }
}
impl ContourBuilder {
    fn winding(&mut self, vertices: &[Vertex]) -> Result<i8, Error> {
        let mut area = 0i32;
        for i in 0..vertices.len() {
            self.charge(1)?;
            let a = vertices[i];
            let b = vertices[(i + vertices.len() - 1) % vertices.len()];
            area = area
                .checked_add(a[0] * b[2] - b[0] * a[2])
                .ok_or(Error::Numeric)?;
        }
        let area = area.checked_add(1).ok_or(Error::Numeric)? / 2;
        Ok(if area < 0 { -1 } else { 1 })
    }
    fn intersects(
        &mut self,
        a: Vertex,
        b: Vertex,
        skip: Option<usize>,
        vertices: &[Vertex],
    ) -> Result<bool, Error> {
        for i in 0..vertices.len() {
            self.charge(1)?;
            let next = (i + 1) % vertices.len();
            if skip == Some(i) || skip == Some(next) {
                continue;
            }
            let c = vertices[i];
            let d = vertices[next];
            if equal(a, c) || equal(b, c) || equal(a, d) || equal(b, d) {
                continue;
            }
            if intersect(a, b, c, d) {
                return Ok(true);
            }
        }
        Ok(false)
    }
    pub(super) fn merge_holes(&mut self, contours: &mut [Contour]) -> Result<(), Error> {
        let mut winding = Vec::new();
        let mut holes = 0;
        for c in contours.iter() {
            let w = self.winding(&c.vertices)?;
            self.push(&mut winding, w)?;
            if w < 0 {
                holes += 1;
            }
        }
        if holes == 0 {
            self.release(&winding);
            return Ok(());
        }
        let mut groups = Vec::new();
        for _ in 0..=self.regions.maximum {
            self.push(
                &mut groups,
                Group {
                    outline: None,
                    holes: Vec::new(),
                },
            )?;
        }
        for (i, c) in contours.iter().enumerate() {
            self.charge(1)?;
            let g = &mut groups[usize::from(c.region)];
            if winding[i] > 0 {
                if g.outline.replace(i).is_some() {
                    return Err(Error::ContourHoles);
                }
            } else {
                let mut left = 0;
                for j in 1..c.vertices.len() {
                    self.charge(1)?;
                    let a = c.vertices[j];
                    let b = c.vertices[left];
                    if (a[0], a[2]) < (b[0], b[2]) {
                        left = j;
                    }
                }
                let v = c.vertices[left];
                self.push(
                    &mut g.holes,
                    Hole {
                        contour: i,
                        left,
                        x: v[0],
                        z: v[2],
                    },
                )?;
            }
        }
        self.release(&winding);
        drop(winding);
        for group in &mut groups {
            self.charge(1)?;
            if group.holes.is_empty() {
                continue;
            }
            let outline = group.outline.ok_or(Error::ContourHoles)?;
            // Explicit encounter-order tie-breaks match the observed pinned Linux
            // qsort profile without an uncharged stable-sort allocation.
            self.charge(
                (group.holes.len() as u64) * (u64::from(group.holes.len().ilog2()) + 1) * 4,
            )?;
            group.holes.sort_unstable_by_key(|h| (h.x, h.z, h.contour));
            let mut max_vertices = contours[outline].vertices.len();
            for h in &group.holes {
                self.charge(1)?;
                max_vertices = max_vertices
                    .checked_add(contours[h.contour].vertices.len())
                    .ok_or(Error::Limit)?;
            }
            let mut diagonals: Vec<Option<Diagonal>> = Vec::new();
            self.reserve(&mut diagonals, max_vertices)?;
            self.charge(max_vertices as u64)?;
            diagonals.resize(max_vertices, None);
            for i in 0..group.holes.len() {
                self.charge(1)?;
                let h = group.holes[i];
                let hole = h.contour;
                let count = contours[hole].vertices.len();
                let mut best = h.left;
                let mut chosen = None;
                for _ in 0..count {
                    self.charge(1)?;
                    let corner = contours[hole].vertices[best];
                    let mut ndiags = 0;
                    for (j, &p) in contours[outline].vertices.iter().enumerate() {
                        self.charge(1)?;
                        if in_cone(&contours[outline].vertices, j, corner) {
                            let dx = p[0] - corner[0];
                            let dz = p[2] - corner[2];
                            let slot = diagonals.get_mut(ndiags).ok_or(Error::ContourHoles)?;
                            *slot = Some(Diagonal {
                                vertex: j,
                                distance: dx * dx + dz * dz,
                            });
                            ndiags += 1;
                        }
                    }
                    if ndiags > 0 {
                        self.charge((ndiags as u64) * (u64::from(ndiags.ilog2()) + 1) * 4)?;
                        diagonals[..ndiags].sort_unstable_by_key(|d| {
                            let d = d.expect("candidate prefix initialized");
                            (d.distance, d.vertex)
                        });
                    }
                    for j in 0..ndiags {
                        self.charge(1)?;
                        let diag = diagonals[j].expect("candidate initialized");
                        let p = contours[outline].vertices[diag.vertex];
                        // The pinned implementation indexes diags[i], not diags[j].
                        // Retain previously initialized tail slots between attempts;
                        // reject any read that would depend on uninitialized storage.
                        let skip = diagonals
                            .get(i)
                            .and_then(|d| *d)
                            .ok_or(Error::ContourHoles)?
                            .vertex;
                        let mut hit =
                            self.intersects(p, corner, Some(skip), &contours[outline].vertices)?;
                        for h in &group.holes[i..] {
                            if hit {
                                break;
                            }
                            hit =
                                self.intersects(p, corner, None, &contours[h.contour].vertices)?;
                        }
                        if !hit {
                            chosen = Some(diag.vertex);
                            break;
                        }
                    }
                    if chosen.is_some() {
                        break;
                    }
                    best = (best + 1) % count;
                }
                let selected = chosen.ok_or(Error::ContourHoles)?;
                let a = &contours[outline].vertices;
                let b = &contours[hole].vertices;
                let mut joined = Vec::new();
                self.reserve(&mut joined, a.len() + b.len() + 2)?;
                for j in 0..=a.len() {
                    self.push(&mut joined, a[(selected + j) % a.len()])?;
                }
                for j in 0..=b.len() {
                    self.push(&mut joined, b[(best + j) % b.len()])?;
                }
                self.release(&contours[outline].vertices);
                contours[outline].vertices = joined;
                self.release(&contours[hole].vertices);
                contours[hole].vertices = Vec::new();
            }
            self.release(&diagonals);
        }
        for g in &groups {
            self.release(&g.holes);
        }
        self.release(&groups);
        Ok(())
    }
}
