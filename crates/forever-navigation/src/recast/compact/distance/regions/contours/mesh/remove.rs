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

// Modified/ported 2026-10-11: border-vertex removal and complete ordered hole repair.
use super::*;
#[derive(Clone, Copy)]
struct Touch {
    vertex: u16,
    count: usize,
}
#[derive(Clone, Copy)]
struct Boundary {
    a: u16,
    b: u16,
    region: u16,
    area: u8,
}
impl Budget {
    pub(super) fn can_remove(&mut self, data: &Data, rem: u16) -> Result<bool, Error> {
        let mut remaining = 0i32;
        for p in &data.polygons {
            let mut removed = 0;
            for &v in p.vertices() {
                self.charge(1)?;
                if v == rem {
                    removed += 1;
                }
            }
            if removed > 0 {
                remaining += p.len() as i32 - (removed + 1);
            }
        }
        if remaining <= 2 {
            return Ok(false);
        }
        let mut edges: Vec<Touch> = Vec::new();
        for p in &data.polygons {
            let n = p.len();
            for j in 0..n {
                self.charge(1)?;
                let (mut a, mut b) = (p.vertices[j], p.vertices[(j + n - 1) % n]);
                if a == rem || b == rem {
                    if b == rem {
                        std::mem::swap(&mut a, &mut b);
                    }
                    let mut exists = false;
                    for e in &mut edges {
                        self.charge(1)?;
                        if e.vertex == b {
                            e.count += 1;
                            exists = true;
                        }
                    }
                    if !exists {
                        self.push(
                            &mut edges,
                            Touch {
                                vertex: b,
                                count: 1,
                            },
                        )?;
                    }
                }
            }
        }
        self.charge(edges.len() as u64)?;
        let open = edges.iter().filter(|e| e.count < 2).count();
        self.release(&edges);
        Ok(open <= 2)
    }
    pub(super) fn remove_vertex(&mut self, data: &mut Data, rem: u16) -> Result<(), Error> {
        let mut edges: Vec<Boundary> = Vec::new();
        let mut i = 0;
        while i < data.polygons.len() {
            let p = data.polygons[i];
            let n = p.len();
            self.charge(n as u64)?;
            if p.vertices().contains(&rem) {
                for j in 0..n {
                    self.charge(1)?;
                    let (a, b) = (p.vertices[(j + n - 1) % n], p.vertices[j]);
                    if a != rem && b != rem {
                        self.push(
                            &mut edges,
                            Boundary {
                                a,
                                b,
                                region: p.region,
                                area: p.area,
                            },
                        )?;
                    }
                }
                self.charge(1)?;
                data.polygons.swap_remove(i);
            } else {
                i += 1;
            }
        }
        self.charge((data.vertices.len() - usize::from(rem)) as u64)?;
        data.vertices.remove(usize::from(rem));
        for p in &mut data.polygons {
            for v in &mut p.vertices {
                self.charge(1)?;
                if *v == NULL {
                    break;
                }
                if *v > rem {
                    *v -= 1;
                }
            }
        }
        for e in &mut edges {
            self.charge(1)?;
            if e.a > rem {
                e.a -= 1;
            }
            if e.b > rem {
                e.b -= 1;
            }
        }
        if edges.is_empty() {
            self.release(&edges);
            return Ok(());
        }
        let mut hole = Vec::new();
        let e = edges[0];
        self.push(&mut hole, [e.a, e.region, u16::from(e.area)])?;
        while !edges.is_empty() {
            let mut matched = false;
            let mut i = 0;
            while i < edges.len() {
                self.charge(1)?;
                let e = edges[i];
                let added = if hole[0][0] == e.b {
                    self.reserve(&mut hole, 1)?;
                    self.charge(hole.len() as u64 + 1)?;
                    hole.insert(0, [e.a, e.region, u16::from(e.area)]);
                    true
                } else if hole[hole.len() - 1][0] == e.a {
                    self.push(&mut hole, [e.b, e.region, u16::from(e.area)])?;
                    true
                } else {
                    false
                };
                if added {
                    self.charge(1)?;
                    edges.swap_remove(i);
                    matched = true;
                } else {
                    i += 1;
                }
            }
            if !matched {
                return Err(Error::MeshVertexRemoval);
            }
        }
        self.release(&edges);
        drop(edges);
        let mut vertices = Vec::new();
        for h in &hole {
            let v = data.vertices[usize::from(h[0])];
            self.push(
                &mut vertices,
                [i32::from(v[0]), i32::from(v[1]), i32::from(v[2]), 0],
            )?;
        }
        let tris = self.triangulate(&vertices)?;
        let mut polys = Vec::new();
        for t in &tris {
            self.charge(1)?;
            if t[0] != t[1] && t[0] != t[2] && t[1] != t[2] {
                let (a, b, c) = (hole[t[0]], hole[t[1]], hole[t[2]]);
                let region = if a[1] == b[1] && b[1] == c[1] {
                    a[1]
                } else {
                    0
                };
                let mut p = Polygon::new(region, a[2] as u8);
                p.vertices[..3].copy_from_slice(&[a[0], b[0], c[0]]);
                self.push(&mut polys, p)?;
            }
        }
        self.merge_polygons(&data.vertices, &mut polys, data.nvp)?;
        for &p in &polys {
            self.store(data, p)?;
        }
        self.release(&polys);
        self.release(&tris);
        self.release(&vertices);
        self.release(&hole);
        Ok(())
    }
}
