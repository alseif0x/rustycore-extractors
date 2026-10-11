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

// Modified/ported 2026-10-11: bounded initial compact-cell graph, with checked layers.
// Recast algorithm reference and attribution are recorded in the parent module.
use super::*;
mod area;
mod distance;
pub use distance::{
    Contour, ContourOptions, ContourSet, DetailMesh, DetailOptions, DetailSubmesh, MergeLimits,
    MeshHeader, PolyMesh, Polygon, TileMesh,
};
pub use distance::{DistanceField, RegionOptions, Regions};
#[cfg(test)]
mod tests;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Cell {
    pub index: u32,
    pub count: u8,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompactSpan {
    pub y: u16,
    pub region: u16,
    pub connections: [u8; 4],
    pub height: u8,
    pub area: u8,
}
pub struct Compact {
    grid: Grid,
    agent: Agent,
    cells: Vec<Cell>,
    spans: Vec<CompactSpan>,
    work: u64,
    limits: Limits,
}
impl Compact {
    pub const fn grid(&self) -> Grid {
        self.grid
    }
    pub const fn agent(&self) -> Agent {
        self.agent
    }
    pub fn cells(&self) -> &[Cell] {
        &self.cells
    }
    pub fn spans(&self) -> &[CompactSpan] {
        &self.spans
    }
    pub const fn work_used(&self) -> u64 {
        self.work
    }
    pub fn capacity_bytes(&self) -> usize {
        self.cells.capacity() * std::mem::size_of::<Cell>()
            + self.spans.capacity() * std::mem::size_of::<CompactSpan>()
    }

    /// The original post-compaction order before watershed region building.
    pub fn prepare(self, radius: u16) -> Result<DistanceField, Error> {
        self.erode(radius)?.median_filter()?.build_distance_field()
    }

    fn charge(&mut self, amount: u64) -> Result<(), Error> {
        self.work = self.work.checked_add(amount).ok_or(Error::Limit)?;
        if self.work > self.limits.work {
            Err(Error::Limit)
        } else {
            Ok(())
        }
    }

    fn scratch<T: Clone>(&mut self, value: T, other_bytes: usize) -> Result<Vec<T>, Error> {
        let count = self.spans.len();
        self.charge(count as u64)?;
        let resident = self
            .capacity_bytes()
            .checked_add(other_bytes)
            .ok_or(Error::Limit)?;
        let bytes = count
            .checked_mul(std::mem::size_of::<T>())
            .ok_or(Error::Limit)?;
        if resident.checked_add(bytes).ok_or(Error::Limit)? > self.limits.heap_bytes {
            return Err(Error::Limit);
        }
        let mut out = Vec::new();
        out.try_reserve_exact(count)
            .map_err(|_| Error::Allocation)?;
        if resident + out.capacity() * std::mem::size_of::<T>() > self.limits.heap_bytes {
            return Err(Error::Limit);
        }
        out.resize(count, value);
        Ok(out)
    }

    fn indices(&self, cell: usize) -> std::ops::Range<usize> {
        let c = self.cells[cell];
        c.index as usize..c.index as usize + usize::from(c.count)
    }

    /// Connections were checked during construction and never mutate afterward.
    fn link(
        &mut self,
        cell: usize,
        index: usize,
        direction: usize,
    ) -> Result<Option<(usize, usize)>, Error> {
        self.charge(1)?;
        let layer = self.spans[index].connections[direction];
        if layer == NOT_CONNECTED {
            return Ok(None);
        }
        let cell = neighbor(self.grid, cell, direction)
            .expect("constructed compact connection stays in grid");
        Ok(Some((
            cell,
            self.cells[cell].index as usize + usize::from(layer),
        )))
    }

    fn propagate<T: DistanceValue>(&mut self, values: &mut [T]) -> Result<(), Error> {
        // Reversing cell order does NOT reverse each column's span order.
        for pass in 0..2 {
            for step in 0..self.cells.len() {
                self.charge(1)?;
                let cell = if pass == 0 {
                    step
                } else {
                    self.cells.len() - 1 - step
                };
                for index in self.indices(cell) {
                    self.charge(1)?;
                    let dirs = if pass == 0 {
                        [(0, 3), (3, 2)]
                    } else {
                        [(2, 1), (1, 0)]
                    };
                    for (direction, diagonal) in dirs {
                        if let Some((near_cell, near)) = self.link(cell, index, direction)? {
                            values[index] =
                                T::from_value(values[index].value().min(values[near].value() + 2));
                            if let Some((_, far)) = self.link(near_cell, near, diagonal)? {
                                values[index] = T::from_value(
                                    values[index].value().min(values[far].value() + 3),
                                );
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
trait DistanceValue: Copy {
    fn value(self) -> u32;
    fn from_value(value: u32) -> Self;
}
impl DistanceValue for u8 {
    fn value(self) -> u32 {
        u32::from(self)
    }
    fn from_value(value: u32) -> Self {
        value as u8
    }
}
impl DistanceValue for u16 {
    fn value(self) -> u32 {
        u32::from(self)
    }
    fn from_value(value: u32) -> Self {
        value as u16
    }
}
impl Heightfield {
    pub fn compact(mut self, agent: Agent) -> Result<Compact, Error> {
        let mut count = 0usize;
        for cell in 0..self.heads.len() {
            self.charge(1)?;
            let mut at = self.heads[cell];
            let mut local = 0;
            while at != NONE {
                self.charge(1)?;
                let n = self.nodes[at as usize];
                if n.span.area != 0 {
                    local += 1;
                }
                at = n.next;
            }
            if local > 255 {
                return Err(Error::Layers);
            }
            count += local;
        }
        let bytes = self.heads.len() * std::mem::size_of::<Cell>()
            + count * std::mem::size_of::<CompactSpan>();
        if self.capacity_bytes() + bytes > self.limits.heap_bytes {
            return Err(Error::Limit);
        }
        let mut grid = self.grid;
        grid.max[1] = finite(grid.max[1] + f32::from(agent.height) * grid.cell_height)?;
        let mut out = Compact {
            grid,
            agent,
            cells: Vec::new(),
            spans: Vec::new(),
            work: 0,
            limits: self.limits,
        };
        out.cells
            .try_reserve_exact(self.heads.len())
            .map_err(|_| Error::Allocation)?;
        out.spans
            .try_reserve_exact(count)
            .map_err(|_| Error::Allocation)?;
        if self.capacity_bytes() + out.capacity_bytes() > self.limits.heap_bytes {
            return Err(Error::Limit);
        }
        out.cells.resize(self.heads.len(), Cell::default());
        for cell in 0..self.heads.len() {
            self.charge(1)?;
            let mut at = self.heads[cell];
            if at == NONE {
                continue;
            }
            let start = out.spans.len();
            while at != NONE {
                self.charge(1)?;
                let n = self.nodes[at as usize];
                at = n.next;
                if n.span.area == 0 {
                    continue;
                }
                let top = if at == NONE {
                    65535
                } else {
                    i32::from(self.nodes[at as usize].span.min)
                };
                out.spans.push(CompactSpan {
                    y: n.span.max,
                    region: 0,
                    connections: [63; 4],
                    height: (top - i32::from(n.span.max)).clamp(0, 255) as u8,
                    area: n.span.area,
                });
            }
            out.cells[cell] = Cell {
                index: start as u32,
                count: (out.spans.len() - start) as u8,
            };
        }
        for cell in 0..out.cells.len() {
            self.charge(1)?;
            let c = out.cells[cell];
            for index in c.index as usize..c.index as usize + usize::from(c.count) {
                let s = out.spans[index];
                for direction in 0..4 {
                    self.charge(1)?;
                    let Some(ncell) = neighbor(grid, cell, direction) else {
                        continue;
                    };
                    let nc = out.cells[ncell];
                    for layer in 0..usize::from(nc.count) {
                        self.charge(1)?;
                        let n = out.spans[nc.index as usize + layer];
                        let floor = s.y.max(n.y);
                        let ceiling = (u32::from(s.y) + u32::from(s.height))
                            .min(u32::from(n.y) + u32::from(n.height));
                        if ceiling as i32 - i32::from(floor) >= i32::from(agent.height)
                            && n.y.abs_diff(s.y) <= agent.climb
                        {
                            if layer >= 63 {
                                return Err(Error::Layers);
                            }
                            out.spans[index].connections[direction] = layer as u8;
                            break;
                        }
                    }
                }
            }
        }
        out.work = self.work;
        Ok(out)
    }
}

pub use distance::detour;
