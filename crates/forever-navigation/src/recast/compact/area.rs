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

// Modified/ported 2026-10-11: consuming bounded erosion and area median filter.
use super::*;

impl Compact {
    /// Mark spans nearer than `radius` to an obstacle as area zero, preserving
    /// cells, geometry and connections. Radius 0..=127 avoids native u8 wrap.
    pub fn erode(mut self, radius: u16) -> Result<Self, Error> {
        if radius > 127 {
            return Err(Error::Radius);
        }
        let mut dist = self.scratch(u8::MAX, 0)?;
        for cell in 0..self.cells.len() {
            self.charge(1)?;
            for i in self.indices(cell) {
                self.charge(1)?;
                if self.spans[i].area == 0 {
                    dist[i] = 0;
                    continue;
                }
                let mut count = 0;
                for direction in 0..4 {
                    if let Some((_, near)) = self.link(cell, i, direction)?
                        && self.spans[near].area != 0
                    {
                        count += 1;
                    }
                }
                if count != 4 {
                    dist[i] = 0;
                }
            }
        }
        self.propagate(&mut dist)?;
        self.charge(self.spans.len() as u64)?;
        for (span, d) in self.spans.iter_mut().zip(dist) {
            if u16::from(d) < radius * 2 {
                span.area = 0;
            }
        }
        Ok(self)
    }

    /// A 3x3 median over the original areas, retaining null spans and graph.
    pub fn median_filter(mut self) -> Result<Self, Error> {
        let mut areas = self.scratch(0u8, 0)?;
        for cell in 0..self.cells.len() {
            self.charge(1)?;
            for i in self.indices(cell) {
                self.charge(1)?;
                if self.spans[i].area == 0 {
                    continue;
                }
                let mut samples = [self.spans[i].area; 9];
                self.charge(81)?; // fixed conservative bound for nine-value sorting
                for direction in 0..4 {
                    if let Some((near_cell, near)) = self.link(cell, i, direction)? {
                        if self.spans[near].area != 0 {
                            samples[direction * 2] = self.spans[near].area;
                        }
                        if let Some((_, far)) = self.link(near_cell, near, (direction + 1) & 3)?
                            && self.spans[far].area != 0
                        {
                            samples[direction * 2 + 1] = self.spans[far].area;
                        }
                    }
                }
                samples.sort_unstable();
                areas[i] = samples[4];
            }
        }
        self.charge(self.spans.len() as u64)?;
        for (span, area) in self.spans.iter_mut().zip(areas) {
            span.area = area;
        }
        Ok(self)
    }
}
