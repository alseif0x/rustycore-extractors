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

// Modified/ported 2026-10-11: region adjacency, filtering, merging and native remap.
use super::*;
struct Region {
    count: u32,
    id: u16,
    area: u8,
    remap: bool,
    visited: bool,
    overlap: bool,
    connections: Vec<u16>,
    floors: Vec<u16>,
}
impl Region {
    fn new(id: u16) -> Self {
        Self {
            count: 0,
            id,
            area: 0,
            remap: false,
            visited: false,
            overlap: false,
            connections: Vec::new(),
            floors: Vec::new(),
        }
    }
}
impl Builder {
    pub(super) fn merge_and_filter(
        &mut self,
        next: u16,
        options: RegionOptions,
    ) -> Result<(u16, Vec<u16>), Error> {
        let mut regions = Vec::new();
        self.reserve(&mut regions, usize::from(next) + 1)?;
        for id in 0..=next {
            self.push(&mut regions, Region::new(id))?;
        }
        let nreg = regions.len();
        for cell in 0..self.field.compact.cells.len() {
            self.charge(1)?;
            let indices = self.indices(cell);
            for i in indices.clone() {
                self.charge(1)?;
                let id = usize::from(self.reg[i]);
                if id == 0 || id >= nreg {
                    continue;
                }
                regions[id].count += 1;
                for j in indices.clone() {
                    self.charge(1)?;
                    if i == j {
                        continue;
                    }
                    let floor = self.reg[j];
                    if floor == 0 || usize::from(floor) >= nreg {
                        continue;
                    }
                    if usize::from(floor) == id {
                        regions[id].overlap = true;
                    }
                    self.unique_floor(&mut regions[id], floor)?;
                }
                if !regions[id].connections.is_empty() {
                    continue;
                }
                let e = Entry {
                    cell: cell as u32,
                    index: i as u32,
                };
                regions[id].area = self.area(e);
                for dir in 0..4 {
                    if self.edge_region(e, dir)? != self.reg[i] {
                        regions[id].connections = self.contour(e, dir)?;
                        break;
                    }
                }
            }
        }
        let mut stack = Vec::new();
        let mut trace = Vec::new();
        for i in 0..nreg {
            self.charge(1)?;
            let r = &regions[i];
            if r.id == 0 || r.id & BORDER != 0 || r.count == 0 || r.visited {
                continue;
            }
            let mut border = false;
            let mut count = 0u32;
            stack.clear();
            trace.clear();
            regions[i].visited = true;
            self.push(&mut stack, i as u16)?;
            while let Some(index) = stack.pop() {
                self.charge(1)?;
                let index = usize::from(index);
                count += regions[index].count;
                self.push(&mut trace, index as u16)?;
                for j in 0..regions[index].connections.len() {
                    self.charge(1)?;
                    let id = regions[index].connections[j];
                    if id & BORDER != 0 {
                        border = true;
                        continue;
                    }
                    let r = &mut regions[usize::from(id)];
                    if r.visited || r.id == 0 || r.id & BORDER != 0 {
                        continue;
                    }
                    self.push(&mut stack, r.id)?;
                    r.visited = true;
                }
            }
            if count < options.minimum_area && !border {
                for &id in &trace {
                    self.charge(1)?;
                    regions[usize::from(id)].count = 0;
                    regions[usize::from(id)].id = 0;
                }
            }
        }
        loop {
            self.charge(1)?;
            let mut merged = false;
            for i in 0..nreg {
                self.charge(1)?;
                let r = &regions[i];
                if r.id == 0 || r.id & BORDER != 0 || r.overlap || r.count == 0 {
                    continue;
                }
                self.charge(r.connections.len() as u64)?;
                if r.count > options.merge_area && r.connections.contains(&0) {
                    continue;
                }
                let mut smallest = 0x0fff_ffff;
                let mut target = r.id;
                for &id in &r.connections {
                    self.charge(1)?;
                    if id & BORDER != 0 {
                        continue;
                    }
                    let candidate = &regions[usize::from(id)];
                    if candidate.id == 0 || candidate.id & BORDER != 0 || candidate.overlap {
                        continue;
                    }
                    if candidate.count < smallest
                        && self.can_merge(r, candidate)?
                        && self.can_merge(candidate, r)?
                    {
                        smallest = candidate.count;
                        target = candidate.id;
                    }
                }
                let old = regions[i].id;
                if target != old && self.merge(&mut regions, usize::from(target), i)? {
                    for r in &mut regions {
                        self.charge(1)?;
                        if r.id == 0 || r.id & BORDER != 0 {
                            continue;
                        }
                        if r.id == old {
                            r.id = target;
                        }
                        self.replace(r, old, target)?;
                    }
                    merged = true;
                }
            }
            if !merged {
                break;
            }
        }
        for r in &mut regions {
            self.charge(1)?;
            r.remap = r.id != 0 && r.id & BORDER == 0;
        }
        let mut maximum = 0u16;
        for i in 0..nreg {
            self.charge(1)?;
            if !regions[i].remap {
                continue;
            }
            let old = regions[i].id;
            maximum = maximum.checked_add(1).ok_or(Error::Regions)?;
            for r in &mut regions[i..] {
                self.charge(1)?;
                if r.id == old {
                    r.id = maximum;
                    r.remap = false;
                }
            }
        }
        self.charge(self.reg.len() as u64)?;
        for r in &mut self.reg {
            if *r & BORDER == 0 {
                *r = regions[usize::from(*r)].id;
            }
        }
        let mut overlaps = Vec::new();
        for r in &regions {
            self.charge(1)?;
            if r.overlap {
                self.push(&mut overlaps, r.id)?;
            }
        }
        for r in &regions {
            self.release(&r.connections);
            self.release(&r.floors);
        }
        self.release(&regions);
        self.release(&stack);
        self.release(&trace);
        Ok((maximum, overlaps))
    }
    fn unique_floor(&mut self, r: &mut Region, id: u16) -> Result<(), Error> {
        self.charge(r.floors.len() as u64)?;
        if !r.floors.contains(&id) {
            self.push(&mut r.floors, id)?;
        }
        Ok(())
    }
    fn dedup(&mut self, connections: &mut Vec<u16>) -> Result<(), Error> {
        let mut i = 0;
        while i < connections.len() && connections.len() > 1 {
            self.charge(1)?;
            let next = (i + 1) % connections.len();
            if connections[i] == connections[next] {
                self.charge((connections.len() - i) as u64)?;
                connections.remove(i);
            } else {
                i += 1;
            }
        }
        Ok(())
    }
    fn replace(&mut self, r: &mut Region, old: u16, new: u16) -> Result<(), Error> {
        let mut changed = false;
        for id in &mut r.connections {
            self.charge(1)?;
            if *id == old {
                *id = new;
                changed = true;
            }
        }
        for id in &mut r.floors {
            self.charge(1)?;
            if *id == old {
                *id = new;
            }
        }
        if changed {
            self.dedup(&mut r.connections)?;
        }
        Ok(())
    }
    fn can_merge(&mut self, a: &Region, b: &Region) -> Result<bool, Error> {
        if a.area != b.area {
            return Ok(false);
        }
        self.charge((a.connections.len() + a.floors.len()) as u64)?;
        Ok(
            a.connections.iter().filter(|&&id| id == b.id).count() <= 1
                && !a.floors.contains(&b.id),
        )
    }
    fn merge(
        &mut self,
        regions: &mut [Region],
        target: usize,
        source: usize,
    ) -> Result<bool, Error> {
        let (a, b) = if target < source {
            let (left, right) = regions.split_at_mut(source);
            (&mut left[target], &mut right[0])
        } else {
            let (left, right) = regions.split_at_mut(target);
            (&mut right[0], &mut left[source])
        };
        self.charge((a.connections.len() + b.connections.len()) as u64)?;
        let Some(ia) = a.connections.iter().position(|&id| id == b.id) else {
            return Ok(false);
        };
        let Some(ib) = b.connections.iter().position(|&id| id == a.id) else {
            return Ok(false);
        };
        let mut connections = Vec::new();
        self.reserve(
            &mut connections,
            a.connections.len() + b.connections.len() - 2,
        )?;
        for i in 0..a.connections.len() - 1 {
            self.push(
                &mut connections,
                a.connections[(ia + 1 + i) % a.connections.len()],
            )?;
        }
        for i in 0..b.connections.len() - 1 {
            self.push(
                &mut connections,
                b.connections[(ib + 1 + i) % b.connections.len()],
            )?;
        }
        self.dedup(&mut connections)?;
        for &id in &b.floors {
            self.unique_floor(a, id)?;
        }
        self.release(&a.connections);
        a.connections = connections;
        a.count += b.count;
        b.count = 0;
        b.connections.clear();
        Ok(true)
    }
    fn contour(&mut self, start: Entry, start_dir: usize) -> Result<Vec<u16>, Error> {
        let mut out = Vec::new();
        let mut e = start;
        let mut dir = start_dir;
        let mut current = self.edge_region(e, dir)?;
        self.push(&mut out, current)?;
        let mut closed = false;
        for _ in 0..39999 {
            self.charge(1)?;
            let edge = self.edge_region(e, dir)?;
            if edge != self.reg[e.index as usize] {
                if edge != current {
                    current = edge;
                    self.push(&mut out, current)?;
                }
                dir = (dir + 1) & 3;
            } else {
                e = self.link(e, dir)?.ok_or(Error::RegionContour)?;
                dir = (dir + 3) & 3;
            }
            if e.index == start.index && dir == start_dir {
                closed = true;
                break;
            }
        }
        if !closed {
            return Err(Error::RegionContour);
        }
        self.dedup(&mut out)?;
        Ok(out)
    }
}
