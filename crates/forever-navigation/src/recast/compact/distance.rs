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

// Modified/ported 2026-10-11: immutable distance result over a consumed compact graph.
use super::*;
mod regions;
pub use regions::{
    Contour, ContourOptions, ContourSet, DetailMesh, DetailOptions, DetailSubmesh, MergeLimits,
    MeshHeader, PolyMesh, Polygon, TileMesh,
};
pub use regions::{RegionOptions, Regions};

/// The graph and its blurred distances stay bound. Mutating area operations
/// require consuming this object with `into_compact`, which drops the distances.
pub struct DistanceField {
    compact: Compact,
    values: Vec<u16>,
    maximum: u16,
}
impl DistanceField {
    pub fn compact(&self) -> &Compact {
        &self.compact
    }
    pub fn values(&self) -> &[u16] {
        &self.values
    }
    /// Maximum of the unblurred distance field, as in original Recast.
    pub const fn maximum(&self) -> u16 {
        self.maximum
    }
    pub fn capacity_bytes(&self) -> usize {
        self.compact.capacity_bytes() + self.values.capacity() * 2
    }
    pub fn into_compact(self) -> Compact {
        self.compact
    }
}

impl Compact {
    pub fn build_distance_field(mut self) -> Result<DistanceField, Error> {
        let mut source = self.scratch(u16::MAX, 0)?;
        let mut result = self.scratch(0u16, source.capacity() * 2)?;
        for cell in 0..self.cells.len() {
            self.charge(1)?;
            for i in self.indices(cell) {
                self.charge(1)?;
                let mut count = 0;
                for direction in 0..4 {
                    if let Some((_, near)) = self.link(cell, i, direction)?
                        && self.spans[i].area == self.spans[near].area
                    {
                        count += 1;
                    }
                }
                if count != 4 {
                    source[i] = 0;
                }
            }
        }
        self.propagate(&mut source)?;
        self.charge(self.spans.len() as u64)?;
        let maximum = source.iter().copied().max().unwrap_or(0);
        for cell in 0..self.cells.len() {
            self.charge(1)?;
            for i in self.indices(cell) {
                self.charge(1)?;
                let value = u32::from(source[i]);
                if value <= 2 {
                    result[i] = source[i];
                    continue;
                }
                let mut sum = value;
                for direction in 0..4 {
                    if let Some((near_cell, near)) = self.link(cell, i, direction)? {
                        sum += u32::from(source[near]);
                        sum += match self.link(near_cell, near, (direction + 1) & 3)? {
                            Some((_, far)) => u32::from(source[far]),
                            None => value,
                        };
                    } else {
                        sum += value * 2;
                    }
                }
                result[i] = ((sum + 5) / 9) as u16;
            }
        }
        Ok(DistanceField {
            compact: self,
            values: result,
            maximum,
        })
    }
}

pub use regions::detour;
