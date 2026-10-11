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

// Modified/ported 2026-10-11: original region patch seeding, DFS/BFS and height rings.
use super::*;
pub(super) struct Patch {
    pub min: [i32; 2],
    pub width: usize,
    pub height: usize,
    pub data: Vec<u16>,
}
impl Patch {
    fn index(&self, x: i32, z: i32) -> Option<usize> {
        let x = x - self.min[0];
        let z = z - self.min[1];
        if x < 0 || z < 0 || x as usize >= self.width || z as usize >= self.height {
            None
        } else {
            Some(x as usize + z as usize * self.width)
        }
    }
}
#[derive(Clone, Copy)]
struct Node {
    x: i32,
    z: i32,
    index: usize,
}
fn cell(field: &Compact, x: i32, z: i32) -> Result<usize, Error> {
    if x < 0 || z < 0 || x >= i32::from(field.grid.width) || z >= i32::from(field.grid.height) {
        return Err(Error::DetailHeight);
    }
    Ok(x as usize + z as usize * usize::from(field.grid.width))
}
fn store(patch: &mut Patch, index: usize, value: u16) -> Result<(), Error> {
    if value == NULL {
        return Err(Error::DetailHeight);
    }
    patch.data[index] = value;
    Ok(())
}
impl Budget {
    fn height_link(
        &mut self,
        field: &Compact,
        node: Node,
        dir: usize,
    ) -> Result<Option<Node>, Error> {
        self.charge(1)?;
        let layer = field
            .spans
            .get(node.index)
            .ok_or(Error::DetailHeight)?
            .connections[dir];
        if layer == NOT_CONNECTED {
            return Ok(None);
        }
        let x = node.x + [-1, 0, 1, 0][dir];
        let z = node.z + [0, 1, 0, -1][dir];
        let c = field.cells[cell(field, x, z)?];
        if layer >= c.count {
            return Err(Error::DetailHeight);
        }
        Ok(Some(Node {
            x,
            z,
            index: c.index as usize + usize::from(layer),
        }))
    }
    pub(super) fn height_patch(
        &mut self,
        field: &Compact,
        vertices: &[[u16; 3]],
        poly: &Polygon,
        border: u16,
    ) -> Result<Patch, Error> {
        let (mut xmin, mut xmax, mut zmin, mut zmax) = (
            i32::from(field.grid.width),
            0,
            i32::from(field.grid.height),
            0,
        );
        for &id in poly.vertices() {
            self.charge(1)?;
            let v = vertices[usize::from(id)];
            xmin = xmin.min(i32::from(v[0]));
            xmax = xmax.max(i32::from(v[0]));
            zmin = zmin.min(i32::from(v[2]));
            zmax = zmax.max(i32::from(v[2]));
        }
        xmin = (xmin - 1).max(0);
        xmax = (xmax + 1).min(i32::from(field.grid.width));
        zmin = (zmin - 1).max(0);
        zmax = (zmax + 1).min(i32::from(field.grid.height));
        let bs = i32::from(border);
        if xmin >= xmax
            || zmin >= zmax
            || xmax + bs > i32::from(field.grid.width)
            || zmax + bs > i32::from(field.grid.height)
        {
            return Err(Error::DetailHeight);
        }
        let (w, h) = ((xmax - xmin) as usize, (zmax - zmin) as usize);
        let mut patch = Patch {
            min: [xmin, zmin],
            width: w,
            height: h,
            data: self.filled(w * h, NULL)?,
        };
        let mut queue = Vec::new();
        let mut empty = true;
        if poly.region != 0 {
            for z in zmin..zmax {
                for x in xmin..xmax {
                    self.charge(1)?;
                    let c = field.cells[cell(field, x + bs, z + bs)?];
                    for i in c.index as usize..c.index as usize + usize::from(c.count) {
                        self.charge(1)?;
                        let span = field.spans[i];
                        if span.region != poly.region {
                            continue;
                        }
                        let at = patch.index(x, z).ok_or(Error::DetailHeight)?;
                        store(&mut patch, at, span.y)?;
                        empty = false;
                        let node = Node {
                            x: x + bs,
                            z: z + bs,
                            index: i,
                        };
                        for dir in 0..4 {
                            if let Some(n) = self.height_link(field, node, dir)?
                                && field.spans[n.index].region != poly.region
                            {
                                self.push(&mut queue, node)?;
                                break;
                            }
                        }
                        break;
                    }
                }
            }
        }
        if empty {
            self.seed(field, vertices, poly, bs, &mut patch, &mut queue)?;
        }
        let mut head = 0;
        while head < queue.len() {
            self.charge(1)?;
            let node = queue[head];
            head += 1;
            if head >= 256 {
                let remain = queue.len() - 256;
                self.charge(remain as u64)?;
                queue.copy_within(256.., 0);
                queue.truncate(remain);
                head = 0;
            }
            for dir in 0..4 {
                if let Some(n) = self.height_link(field, node, dir)? {
                    let Some(at) = patch.index(n.x - bs, n.z - bs) else {
                        continue;
                    };
                    if patch.data[at] != NULL {
                        continue;
                    }
                    store(&mut patch, at, field.spans[n.index].y)?;
                    self.push(&mut queue, n)?;
                }
            }
        }
        self.release(&queue);
        Ok(patch)
    }
    fn seed(
        &mut self,
        field: &Compact,
        vertices: &[[u16; 3]],
        poly: &Polygon,
        bs: i32,
        patch: &mut Patch,
        queue: &mut Vec<Node>,
    ) -> Result<(), Error> {
        let offsets = [
            [0, 0],
            [-1, -1],
            [0, -1],
            [1, -1],
            [1, 0],
            [1, 1],
            [0, 1],
            [-1, 1],
            [-1, 0],
        ];
        let mut start = None;
        let mut minimum = 65535i32;
        for &id in poly.vertices() {
            if minimum == 0 {
                break;
            }
            let v = vertices[usize::from(id)];
            for offset in offsets {
                self.charge(1)?;
                if minimum == 0 {
                    break;
                }
                let x = i32::from(v[0]) + offset[0];
                let z = i32::from(v[2]) + offset[1];
                if patch.index(x, z).is_none() {
                    continue;
                }
                let c = field.cells[cell(field, x + bs, z + bs)?];
                for i in c.index as usize..c.index as usize + usize::from(c.count) {
                    self.charge(1)?;
                    if minimum == 0 {
                        break;
                    }
                    let d = (i32::from(v[1]) - i32::from(field.spans[i].y)).abs();
                    if d < minimum {
                        minimum = d;
                        start = Some(Node {
                            x: x + bs,
                            z: z + bs,
                            index: i,
                        });
                    }
                }
            }
        }
        let start = start.ok_or(Error::DetailHeight)?;
        let mut center = [0i32; 2];
        for &id in poly.vertices() {
            self.charge(1)?;
            let v = vertices[usize::from(id)];
            center[0] += i32::from(v[0]);
            center[1] += i32::from(v[2]);
        }
        center[0] = center[0] / poly.len() as i32 + bs;
        center[1] = center[1] / poly.len() as i32 + bs;
        queue.clear();
        self.push(queue, start)?;
        self.charge(patch.data.len() as u64)?;
        patch.data.fill(0);
        let seed = loop {
            self.charge(1)?;
            let node = queue.pop().ok_or(Error::DetailHeight)?;
            if [node.x, node.z] == center {
                break node;
            }
            let direct = if node.x == center[0] {
                if center[1] > node.z { 1 } else { 3 }
            } else if center[0] > node.x {
                2
            } else {
                0
            };
            let mut dirs = [0, 1, 2, 3];
            dirs.swap(direct, 3);
            for dir in dirs {
                if let Some(n) = self.height_link(field, node, dir)? {
                    let Some(at) = patch.index(n.x - bs, n.z - bs) else {
                        continue;
                    };
                    if patch.data[at] != 0 {
                        continue;
                    }
                    patch.data[at] = 1;
                    self.push(queue, n)?;
                }
            }
        };
        queue.clear();
        self.push(queue, seed)?;
        self.charge(patch.data.len() as u64)?;
        patch.data.fill(NULL);
        let at = patch
            .index(seed.x - bs, seed.z - bs)
            .ok_or(Error::DetailHeight)?;
        store(patch, at, field.spans[seed.index].y)
    }
    pub(super) fn sample_height(
        &mut self,
        p: Point,
        inverse_cell: f32,
        cell_height: f32,
        radius: i32,
        patch: &Patch,
    ) -> Result<u16, Error> {
        let ix = integer((p[0] * inverse_cell + 0.01).floor())?
            .checked_sub(patch.min[0])
            .ok_or(Error::Numeric)?
            .clamp(0, patch.width as i32 - 1);
        let iz = integer((p[2] * inverse_cell + 0.01).floor())?
            .checked_sub(patch.min[1])
            .ok_or(Error::Numeric)?
            .clamp(0, patch.height as i32 - 1);
        let mut h = patch.data[ix as usize + iz as usize * patch.width];
        self.charge(1)?;
        if h == NULL {
            let (mut x, mut z, mut dx, mut dz) = (1, 0, 1, 0);
            let size = radius * 2 + 1;
            let max_iter = size * size - 1;
            let (mut next_ring, mut ring_iters) = (8, 16);
            let mut minimum = f32::MAX;
            for i in 0..max_iter {
                self.charge(1)?;
                let nx = ix + x;
                let nz = iz + z;
                if nx >= 0 && nz >= 0 && nx < patch.width as i32 && nz < patch.height as i32 {
                    let nh = patch.data[nx as usize + nz as usize * patch.width];
                    if nh != NULL {
                        let d = (f32::from(nh) * cell_height - p[1]).abs();
                        if d < minimum {
                            h = nh;
                            minimum = d;
                        }
                    }
                }
                if i + 1 == next_ring {
                    if h != NULL {
                        break;
                    }
                    next_ring += ring_iters;
                    ring_iters += 8;
                }
                if x == z || (x < 0 && x == -z) || (x > 0 && x == 1 - z) {
                    let old = dx;
                    dx = -dz;
                    dz = old;
                }
                x += dx;
                z += dz;
            }
        }
        if h == NULL {
            Err(Error::DetailHeight)
        } else {
            Ok(h)
        }
    }
}
