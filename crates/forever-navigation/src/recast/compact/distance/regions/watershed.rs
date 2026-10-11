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

// Modified/ported 2026-10-11: ordered level buckets, staged expansion and flood.
use super::*;
impl Builder {
    pub(super) fn watershed(&mut self, border: u16) -> Result<u16, Error> {
        let mut next = 1u16;
        let g = self.field.compact.grid;
        let w = usize::from(g.width);
        let h = usize::from(g.height);
        if border > 0 {
            let bw = usize::from(border).min(w);
            let bh = usize::from(border).min(h);
            for (x0, x1, z0, z1) in [
                (0, bw, 0, h),
                (w - bw, w, 0, h),
                (0, w, 0, bh),
                (0, w, h - bh, h),
            ] {
                for z in z0..z1 {
                    for x in x0..x1 {
                        self.charge(1)?;
                        for i in self.indices(x + z * w) {
                            self.charge(1)?;
                            if self.field.compact.spans[i].area != 0 {
                                self.reg[i] = next | BORDER;
                            }
                        }
                    }
                }
                next += 1;
            }
        }
        let mut levels: [Vec<Entry>; 8] = std::array::from_fn(|_| Vec::new());
        let mut stack = Vec::new();
        let mut dirty = Vec::new();
        let mut level = (self.field.maximum + 1) & !1;
        let mut bucket = 7;
        while level > 0 {
            self.charge(1)?;
            level = level.saturating_sub(2);
            bucket = (bucket + 1) & 7;
            if bucket == 0 {
                for v in &mut levels {
                    v.clear();
                }
                for cell in 0..self.field.compact.cells.len() {
                    self.charge(1)?;
                    for i in self.indices(cell) {
                        self.charge(1)?;
                        if self.field.compact.spans[i].area == 0 || self.reg[i] != 0 {
                            continue;
                        }
                        let target = (i32::from(level >> 1) - i32::from(self.field.values[i] >> 1))
                            .max(0) as usize;
                        if target < 8 {
                            self.push(
                                &mut levels[target],
                                Entry {
                                    cell: cell as u32,
                                    index: i as u32,
                                },
                            )?;
                        }
                    }
                }
            } else {
                for j in 0..levels[bucket - 1].len() {
                    self.charge(1)?;
                    let e = levels[bucket - 1][j];
                    if e.index != NONE && self.reg[e.index as usize] == 0 {
                        self.push(&mut levels[bucket], e)?;
                    }
                }
            }
            self.expand(level, &mut levels[bucket], &mut dirty)?;
            for &e in &levels[bucket] {
                self.charge(1)?;
                if e.index == NONE || self.reg[e.index as usize] != 0 {
                    continue;
                }
                if next >= BORDER {
                    return Err(Error::Regions);
                }
                if self.flood(e, level, next, &mut stack)? {
                    next = next.checked_add(1).ok_or(Error::Regions)?;
                    if next >= BORDER {
                        return Err(Error::Regions);
                    }
                }
            }
        }
        stack.clear();
        for cell in 0..self.field.compact.cells.len() {
            self.charge(1)?;
            for i in self.indices(cell) {
                self.charge(1)?;
                if self.reg[i] == 0 && self.field.compact.spans[i].area != 0 {
                    self.push(
                        &mut stack,
                        Entry {
                            cell: cell as u32,
                            index: i as u32,
                        },
                    )?;
                }
            }
        }
        self.expand(0, &mut stack, &mut dirty)?;
        for v in &levels {
            self.release(v);
        }
        self.release(&stack);
        self.release(&dirty);
        Ok(next)
    }
    fn expand(
        &mut self,
        level: u16,
        stack: &mut [Entry],
        dirty: &mut Vec<Dirty>,
    ) -> Result<(), Error> {
        for e in stack.iter_mut() {
            self.charge(1)?;
            if e.index != NONE && self.reg[e.index as usize] != 0 {
                e.index = NONE;
            }
        }
        let mut iterations = 0;
        while !stack.is_empty() {
            self.charge(1)?;
            let mut failed = 0;
            dirty.clear();
            for e in stack.iter_mut() {
                self.charge(1)?;
                if e.index == NONE {
                    failed += 1;
                    continue;
                }
                let i = e.index as usize;
                let mut region = self.reg[i];
                let mut distance = u16::MAX;
                let area = self.area(*e);
                for dir in 0..4 {
                    if let Some(n) = self.link(*e, dir)? {
                        let ni = n.index as usize;
                        if self.area(n) != area {
                            continue;
                        }
                        if self.reg[ni] > 0
                            && self.reg[ni] & BORDER == 0
                            && u32::from(self.distance[ni]) + 2 < u32::from(distance)
                        {
                            region = self.reg[ni];
                            distance = self.distance[ni] + 2;
                        }
                    }
                }
                if region != 0 {
                    self.push(
                        dirty,
                        Dirty {
                            index: e.index,
                            region,
                            distance,
                        },
                    )?;
                    e.index = NONE;
                } else {
                    failed += 1;
                }
            }
            for d in dirty.iter() {
                self.charge(1)?;
                self.reg[d.index as usize] = d.region;
                self.distance[d.index as usize] = d.distance;
            }
            if failed == stack.len() {
                break;
            }
            if level > 0 {
                iterations += 1;
                if iterations >= 8 {
                    break;
                }
            }
        }
        Ok(())
    }
    fn flood(
        &mut self,
        start: Entry,
        level: u16,
        region: u16,
        stack: &mut Vec<Entry>,
    ) -> Result<bool, Error> {
        let area = self.area(start);
        stack.clear();
        self.push(stack, start)?;
        self.reg[start.index as usize] = region;
        self.distance[start.index as usize] = 0;
        let level = level.saturating_sub(2);
        let mut count = 0;
        while let Some(e) = stack.pop() {
            self.charge(1)?;
            let mut conflict = false;
            for dir in 0..4 {
                if let Some(n) = self.link(e, dir)? {
                    if self.area(n) != area {
                        continue;
                    }
                    let r = self.reg[n.index as usize];
                    if r & BORDER != 0 {
                        continue;
                    }
                    if r != 0 && r != region {
                        conflict = true;
                        break;
                    }
                    if let Some(diagonal) = self.link(n, (dir + 1) & 3)? {
                        if self.area(diagonal) != area {
                            continue;
                        }
                        let r = self.reg[diagonal.index as usize];
                        if r != 0 && r != region {
                            conflict = true;
                            break;
                        }
                    }
                }
            }
            if conflict {
                self.reg[e.index as usize] = 0;
                continue;
            }
            count += 1;
            for dir in 0..4 {
                if let Some(n) = self.link(e, dir)? {
                    let i = n.index as usize;
                    if self.area(n) == area && self.field.values[i] >= level && self.reg[i] == 0 {
                        self.reg[i] = region;
                        self.distance[i] = 0;
                        self.push(stack, n)?;
                    }
                }
            }
        }
        Ok(count > 0)
    }
}
