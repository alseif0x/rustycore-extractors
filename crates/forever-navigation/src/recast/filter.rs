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

// Modified/ported 2026-10-11: consuming filters over the Rust span arena.
// Recast algorithm reference and attribution are recorded in the parent module.
use super::*;
impl Heightfield {
    pub fn filter_low_obstacles(mut self, climb: u16) -> Result<Self, Error> {
        for cell in 0..self.heads.len() {
            self.charge(1)?;
            let mut at = self.heads[cell];
            let mut previous = None;
            while at != NONE {
                self.charge(1)?;
                let node = self.nodes[at as usize];
                let walkable = node.span.area != 0;
                if let Some((was_walkable, upper, area)) = previous
                    && !walkable
                    && was_walkable
                    && node.span.max.abs_diff(upper) <= climb
                {
                    self.nodes[at as usize].span.area = area;
                }
                previous = Some((walkable, node.span.max, self.nodes[at as usize].span.area));
                at = node.next;
            }
        }
        Ok(self)
    }
    pub fn filter_clearance(mut self, height: u16) -> Result<Self, Error> {
        for cell in 0..self.heads.len() {
            self.charge(1)?;
            let mut at = self.heads[cell];
            while at != NONE {
                self.charge(1)?;
                let node = self.nodes[at as usize];
                let top = if node.next == NONE {
                    65535
                } else {
                    i32::from(self.nodes[node.next as usize].span.min)
                };
                if top - i32::from(node.span.max) <= i32::from(height) {
                    self.nodes[at as usize].span.area = 0;
                }
                at = node.next;
            }
        }
        Ok(self)
    }
    pub fn filter_ledges(mut self, agent: Agent) -> Result<Self, Error> {
        let height = i32::from(agent.height);
        let climb = i32::from(agent.climb);
        for cell in 0..self.heads.len() {
            self.charge(1)?;
            let mut at = self.heads[cell];
            while at != NONE {
                self.charge(1)?;
                let current = at;
                let node = self.nodes[at as usize];
                at = node.next;
                if node.span.area == 0 {
                    continue;
                }
                let bot = i32::from(node.span.max);
                let top = if node.next == NONE {
                    65535
                } else {
                    i32::from(self.nodes[node.next as usize].span.min)
                };
                let mut drop = 65535;
                let (mut amin, mut amax) = (bot, bot);
                for direction in 0..4 {
                    self.charge(1)?;
                    let Some(neighbor) = neighbor(self.grid, cell, direction) else {
                        drop = drop.min(-climb - bot);
                        continue;
                    };
                    let mut next = self.heads[neighbor];
                    let roof = if next == NONE {
                        65535
                    } else {
                        i32::from(self.nodes[next as usize].span.min)
                    };
                    if top.min(roof) - bot.max(-climb) > height {
                        drop = drop.min(-climb - bot);
                    }
                    while next != NONE {
                        self.charge(1)?;
                        let n = self.nodes[next as usize];
                        next = n.next;
                        let floor = i32::from(n.span.max);
                        let roof = if next == NONE {
                            65535
                        } else {
                            i32::from(self.nodes[next as usize].span.min)
                        };
                        if top.min(roof) - bot.max(floor) > height {
                            drop = drop.min(floor - bot);
                            if (floor - bot).abs() <= climb {
                                amin = amin.min(floor);
                                amax = amax.max(floor);
                            }
                        }
                    }
                }
                if drop < -climb || amax - amin > climb {
                    self.nodes[current as usize].span.area = 0;
                }
            }
        }
        Ok(self)
    }
}
