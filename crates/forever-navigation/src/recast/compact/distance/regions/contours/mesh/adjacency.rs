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

// Modified/ported 2026-10-11: native linked edge matching with checked u16 IDs.
// Original edge construction is based on Eric Lengyel's edges.php algorithm.
use super::*;
#[derive(Clone, Copy)]
struct Edge {
    end: u16,
    polygons: [u16; 2],
    slots: [usize; 2],
    next: u16,
}
impl Budget {
    pub(super) fn adjacency(&mut self, data: &mut Data) -> Result<(), Error> {
        let mut first = self.filled(data.vertices.len(), NULL)?;
        let mut edges: Vec<Edge> = Vec::new();
        for (i, p) in data.polygons.iter().enumerate() {
            let n = p.len();
            for j in 0..n {
                self.charge(1)?;
                let (v0, v1) = (p.vertices[j], p.vertices[(j + 1) % n]);
                if v0 < v1 {
                    if edges.len() >= usize::from(NULL) {
                        return Err(Error::Mesh);
                    }
                    let id = edges.len() as u16;
                    self.push(
                        &mut edges,
                        Edge {
                            end: v1,
                            polygons: [i as u16; 2],
                            slots: [j, 0],
                            next: first[usize::from(v0)],
                        },
                    )?;
                    first[usize::from(v0)] = id;
                }
            }
        }
        for (i, p) in data.polygons.iter().enumerate() {
            let n = p.len();
            for j in 0..n {
                self.charge(1)?;
                let (v0, v1) = (p.vertices[j], p.vertices[(j + 1) % n]);
                if v0 > v1 {
                    let mut at = first[usize::from(v1)];
                    while at != NULL {
                        self.charge(1)?;
                        let e = &mut edges[usize::from(at)];
                        if e.end == v0 && e.polygons[0] == e.polygons[1] {
                            e.polygons[1] = i as u16;
                            e.slots[1] = j;
                            break;
                        }
                        at = e.next;
                    }
                }
            }
        }
        for e in &edges {
            self.charge(1)?;
            if e.polygons[0] != e.polygons[1] {
                data.polygons[usize::from(e.polygons[0])].neighbors[e.slots[0]] = e.polygons[1];
                data.polygons[usize::from(e.polygons[1])].neighbors[e.slots[1]] = e.polygons[0];
            }
        }
        self.release(&edges);
        self.release(&first);
        Ok(())
    }
}
