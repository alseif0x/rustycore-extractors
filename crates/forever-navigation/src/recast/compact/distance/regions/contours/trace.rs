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

// Modified/ported 2026-10-11: checked native corners and boundary traversal.
use super::*;
impl ContourBuilder {
    fn corner(&mut self, e: Entry, dir: usize) -> Result<(i32, bool), Error> {
        let s = self.span(e);
        let mut height = i32::from(s.y);
        let mut regs = [0u32; 4];
        regs[0] = u32::from(s.region) | (u32::from(s.area) << 16);
        let next = (dir + 1) & 3;
        for (direction, slot, diagonal) in [(dir, 1, next), (next, 3, dir)] {
            if let Some(n) = self.link(e, direction)? {
                let s = self.span(n);
                height = height.max(i32::from(s.y));
                regs[slot] = u32::from(s.region) | (u32::from(s.area) << 16);
                if let Some(n) = self.link(n, diagonal)? {
                    let s = self.span(n);
                    height = height.max(i32::from(s.y));
                    regs[2] = u32::from(s.region) | (u32::from(s.area) << 16);
                }
            }
        }
        let border = (0..4).any(|a| {
            let (b, c, d) = ((a + 1) & 3, (a + 2) & 3, (a + 3) & 3);
            (regs[a] & regs[b] & u32::from(BORDER)) != 0
                && regs[a] == regs[b]
                && (regs[c] | regs[d]) & u32::from(BORDER) == 0
                && regs[c] >> 16 == regs[d] >> 16
                && regs.iter().all(|&r| r != 0)
        });
        self.charge(16)?;
        Ok((height, border))
    }
    pub(super) fn trace(&mut self, start: Entry, flags: &mut [u8]) -> Result<Vec<Vertex>, Error> {
        let mut points = Vec::new();
        let mut e = start;
        let start_dir = (0..4)
            .find(|&d| flags[e.index as usize] & (1 << d) != 0)
            .ok_or(Error::Contour)?;
        let mut dir = start_dir;
        let area = self.span(start).area;
        for _ in 0..39999 {
            self.charge(1)?;
            if flags[e.index as usize] & (1 << dir) != 0 {
                let w = u32::from(self.regions.field.compact.grid.width);
                let mut x = (e.cell % w) as i32;
                let mut z = (e.cell / w) as i32;
                let (y, border) = self.corner(e, dir)?;
                match dir {
                    0 => z += 1,
                    1 => {
                        x += 1;
                        z += 1;
                    }
                    2 => x += 1,
                    _ => {}
                }
                let mut r = 0;
                if let Some(n) = self.link(e, dir)? {
                    let s = self.span(n);
                    r = i32::from(s.region);
                    if s.area != area {
                        r |= AREA_BORDER;
                    }
                }
                if border {
                    r |= BORDER_VERTEX;
                }
                self.push(&mut points, [x, y, z, r])?;
                flags[e.index as usize] &= !(1 << dir);
                dir = (dir + 1) & 3;
            } else {
                e = self.link(e, dir)?.ok_or(Error::Contour)?;
                dir = (dir + 3) & 3;
            }
            if e.index == start.index && dir == start_dir {
                return Ok(points);
            }
        }
        Err(Error::Contour)
    }
}
