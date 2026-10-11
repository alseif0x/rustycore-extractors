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

// Modified/ported 2026-10-11: original hull and constrained Delaunay detail order.
use super::*;
const UNDEFINED: i32 = -1;
const HULL: i32 = -2;
#[derive(Clone, Copy)]
struct Edge {
    s: usize,
    t: usize,
    left: i32,
    right: i32,
}
fn update(edge: &mut Edge, s: usize, t: usize, face: i32) {
    if edge.s == s && edge.t == t && edge.left == UNDEFINED {
        edge.left = face;
    } else if edge.t == s && edge.s == t && edge.right == UNDEFINED {
        edge.right = face;
    }
}
fn overlap(a: Point, b: Point, c: Point, d: Point) -> bool {
    let a1 = cross2(a, b, d);
    let a2 = cross2(a, b, c);
    if a1 * a2 < 0.0 {
        let a3 = cross2(c, d, a);
        let a4 = a3 + a2 - a1;
        a3 * a4 < 0.0
    } else {
        false
    }
}
impl Budget {
    pub(super) fn hull_triangles(
        &mut self,
        vertices: &[Point],
        hull: &[usize],
        original: usize,
    ) -> Result<Vec<Tri>, Error> {
        let n = hull.len();
        if n < 3 {
            return Err(Error::DetailTriangulation);
        }
        let (mut start, mut left, mut right) = (0, 1, n - 1);
        let mut minimum = f32::MAX;
        for i in 0..n {
            self.charge(1)?;
            if hull[i] >= original {
                continue;
            }
            let pi = (i + n - 1) % n;
            let ni = (i + 1) % n;
            let (p, c, v) = (vertices[hull[pi]], vertices[hull[i]], vertices[hull[ni]]);
            let d = distance2(p, c) + distance2(c, v) + distance2(v, p);
            if d < minimum {
                start = i;
                left = ni;
                right = pi;
                minimum = d;
            }
        }
        let mut tris = Vec::new();
        self.push(&mut tris, [hull[start], hull[left], hull[right]])?;
        while (left + 1) % n != right {
            self.charge(1)?;
            let nl = (left + 1) % n;
            let nr = (right + n - 1) % n;
            let (cl, vl, cr, vr) = (
                vertices[hull[left]],
                vertices[hull[nl]],
                vertices[hull[right]],
                vertices[hull[nr]],
            );
            let dl = distance2(cl, vl) + distance2(vl, cr);
            let dr = distance2(cr, vr) + distance2(cl, vr);
            if dl < dr {
                self.push(&mut tris, [hull[left], hull[nl], hull[right]])?;
                left = nl;
            } else {
                self.push(&mut tris, [hull[left], hull[nr], hull[right]])?;
                right = nr;
            }
        }
        Ok(tris)
    }
    fn find_detail_edge(
        &mut self,
        edges: &[Edge],
        s: usize,
        t: usize,
    ) -> Result<Option<usize>, Error> {
        for (i, e) in edges.iter().enumerate() {
            self.charge(1)?;
            if (e.s == s && e.t == t) || (e.s == t && e.t == s) {
                return Ok(Some(i));
            }
        }
        Ok(None)
    }
    fn add_detail_edge(
        &mut self,
        edges: &mut Vec<Edge>,
        e: Edge,
        maximum: usize,
    ) -> Result<(), Error> {
        if edges.len() >= maximum {
            return Err(Error::DetailLimit);
        }
        if self.find_detail_edge(edges, e.s, e.t)?.is_none() {
            self.push(edges, e)?;
        }
        Ok(())
    }
    fn overlap_detail_edges(
        &mut self,
        points: &[Point],
        edges: &[Edge],
        s: usize,
        t: usize,
    ) -> Result<bool, Error> {
        for e in edges {
            self.charge(1)?;
            if e.s == s || e.s == t || e.t == s || e.t == t {
                continue;
            }
            if overlap(points[e.s], points[e.t], points[s], points[t]) {
                return Ok(true);
            }
        }
        Ok(false)
    }
    fn facet(
        &mut self,
        points: &[Point],
        edges: &mut Vec<Edge>,
        faces: &mut usize,
        at: usize,
    ) -> Result<(), Error> {
        let e = edges[at];
        let (s, t) = if e.left == UNDEFINED {
            (e.s, e.t)
        } else if e.right == UNDEFINED {
            (e.t, e.s)
        } else {
            return Ok(());
        };
        let mut selected = None;
        let mut center = [0.0; 3];
        let mut radius = -1.0;
        for u in 0..points.len() {
            self.charge(1)?;
            if u == s || u == t {
                continue;
            }
            if cross2(points[s], points[t], points[u]) > 1e-5 {
                if radius < 0.0 {
                    selected = Some(u);
                    (center, radius) = circle(points[s], points[t], points[u]);
                    continue;
                }
                let d = distance2(center, points[u]);
                if d > radius * (1.0 + 0.001) {
                    continue;
                }
                if d < radius * (1.0 - 0.001) {
                    selected = Some(u);
                    (center, radius) = circle(points[s], points[t], points[u]);
                } else {
                    if self.overlap_detail_edges(points, edges, s, u)?
                        || self.overlap_detail_edges(points, edges, t, u)?
                    {
                        continue;
                    }
                    selected = Some(u);
                    (center, radius) = circle(points[s], points[t], points[u]);
                }
            }
        }
        if let Some(pt) = selected {
            let face = i32::try_from(*faces).map_err(|_| Error::DetailLimit)?;
            update(&mut edges[at], s, t, face);
            for (a, b) in [(pt, s), (t, pt)] {
                if let Some(at) = self.find_detail_edge(edges, a, b)? {
                    update(&mut edges[at], a, b, face);
                } else {
                    self.add_detail_edge(
                        edges,
                        Edge {
                            s: a,
                            t: b,
                            left: face,
                            right: UNDEFINED,
                        },
                        points.len() * 10,
                    )?;
                }
            }
            *faces += 1;
        } else {
            update(&mut edges[at], s, t, HULL);
        }
        Ok(())
    }
    pub(super) fn delaunay(
        &mut self,
        points: &[Point],
        hull: &[usize],
    ) -> Result<Triangulation, Error> {
        let mut edges = Vec::new();
        for i in 0..hull.len() {
            self.add_detail_edge(
                &mut edges,
                Edge {
                    s: hull[(i + hull.len() - 1) % hull.len()],
                    t: hull[i],
                    left: HULL,
                    right: UNDEFINED,
                },
                points.len() * 10,
            )?;
        }
        let mut faces = 0;
        let mut at = 0;
        while at < edges.len() {
            self.charge(1)?;
            if edges[at].left == UNDEFINED {
                self.facet(points, &mut edges, &mut faces, at)?;
            }
            if edges[at].right == UNDEFINED {
                self.facet(points, &mut edges, &mut faces, at)?;
            }
            at += 1;
        }
        if faces == 0 {
            return Err(Error::DetailTriangulation);
        }
        let mut records = self.filled(faces, [-1i32; 3])?;
        for e in &edges {
            self.charge(1)?;
            if e.right >= 0 {
                let t = &mut records[e.right as usize];
                if t[0] == -1 {
                    t[0] = e.s as i32;
                    t[1] = e.t as i32;
                } else if t[0] == e.t as i32 {
                    t[2] = e.s as i32;
                } else if t[1] == e.s as i32 {
                    t[2] = e.t as i32;
                }
            }
            if e.left >= 0 {
                let t = &mut records[e.left as usize];
                if t[0] == -1 {
                    t[0] = e.t as i32;
                    t[1] = e.s as i32;
                } else if t[0] == e.s as i32 {
                    t[2] = e.t as i32;
                } else if t[1] == e.t as i32 {
                    t[2] = e.s as i32;
                }
            }
        }
        let discarded_faces = self.discard_detail_faces(&mut records)?;
        if records.is_empty() {
            return Err(Error::DetailTriangulation);
        }
        let mut tris = Vec::new();
        for t in &records {
            if t.iter().any(|&n| n < 0 || n as usize >= points.len()) {
                return Err(Error::DetailTriangulation);
            }
            self.push(&mut tris, [t[0] as usize, t[1] as usize, t[2] as usize])?;
        }
        self.release(&records);
        self.release(&edges);
        Ok(Triangulation {
            triangles: tris,
            discarded_faces,
        })
    }

    pub(super) fn discard_detail_faces(
        &mut self,
        records: &mut Vec<[i32; 3]>,
    ) -> Result<u32, Error> {
        let mut discarded = 0u32;
        let mut i = 0;
        while i < records.len() {
            self.charge(1)?;
            if records[i].contains(&-1) {
                discarded = discarded.checked_add(1).ok_or(Error::Limit)?;
                self.charge(1)?;
                records.swap_remove(i);
            } else {
                i += 1;
            }
        }
        Ok(discarded)
    }
}
