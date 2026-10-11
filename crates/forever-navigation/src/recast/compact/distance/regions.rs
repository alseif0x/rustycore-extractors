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

// Modified/ported 2026-10-11: bounded original watershed region construction.
use super::*;
mod contours;
pub use contours::{
    Contour, ContourOptions, ContourSet, DetailMesh, DetailOptions, DetailSubmesh, MergeLimits,
    MeshHeader, PolyMesh, Polygon, TileMesh,
};
mod merge;
#[cfg(test)]
mod tests;
mod watershed;
const BORDER: u16 = 0x8000;

#[derive(Clone, Copy, Debug)]
pub struct RegionOptions {
    pub border_size: u16,
    /// Span count, already squared when the caller starts with a linear size.
    pub minimum_area: u32,
    pub merge_area: u32,
}
pub struct Regions {
    field: DistanceField,
    options: RegionOptions,
    maximum: u16,
    overlaps: Vec<u16>,
}
impl Regions {
    pub fn distance_field(&self) -> &DistanceField {
        &self.field
    }
    pub const fn options(&self) -> RegionOptions {
        self.options
    }
    /// The native upper ID bound, including native empty-region remapping.
    pub const fn maximum(&self) -> u16 {
        self.maximum
    }
    /// Native overlap diagnostics, in native order; not a polygon-safety proof.
    pub fn overlapping_regions(&self) -> &[u16] {
        &self.overlaps
    }
    pub fn capacity_bytes(&self) -> usize {
        self.field.capacity_bytes() + self.overlaps.capacity() * 2
    }
}

impl DistanceField {
    pub fn build_regions(self, options: RegionOptions) -> Result<Regions, Error> {
        if options.minimum_area > i32::MAX as u32 || options.merge_area > i32::MAX as u32 {
            return Err(Error::Numeric);
        }
        if self.maximum == u16::MAX {
            return Err(Error::Numeric);
        }
        let bytes = self.capacity_bytes();
        let mut b = Builder {
            field: self,
            bytes,
            reg: Vec::new(),
            distance: Vec::new(),
        };
        let n = b.field.values.len();
        let mut regs = Vec::new();
        b.reserve(&mut regs, n)?;
        b.charge(n as u64)?;
        regs.resize(n, 0);
        b.reg = regs;
        let mut distances = Vec::new();
        b.reserve(&mut distances, n)?;
        b.charge(n as u64)?;
        distances.resize(n, 0);
        b.distance = distances;
        let next = b.watershed(options.border_size)?;
        let (maximum, overlaps) = b.merge_and_filter(next, options)?;
        b.charge(n as u64)?;
        for (span, region) in b.field.compact.spans.iter_mut().zip(b.reg) {
            span.region = region;
        }
        Ok(Regions {
            field: b.field,
            options,
            maximum,
            overlaps,
        })
    }
}
#[derive(Clone, Copy)]
struct Entry {
    cell: u32,
    index: u32,
}
#[derive(Clone, Copy)]
struct Dirty {
    index: u32,
    region: u16,
    distance: u16,
}
struct Builder {
    field: DistanceField,
    bytes: usize,
    reg: Vec<u16>,
    distance: Vec<u16>,
}
impl Builder {
    fn charge(&mut self, n: u64) -> Result<(), Error> {
        self.field.compact.charge(n)
    }
    fn reserve<T>(&mut self, v: &mut Vec<T>, additional: usize) -> Result<(), Error> {
        let want = v.len().checked_add(additional).ok_or(Error::Limit)?;
        if want <= v.capacity() {
            return Ok(());
        }
        let size = std::mem::size_of::<T>();
        let old = v.capacity() * size;
        let cap = want.max(v.capacity().saturating_mul(2)).max(8);
        let bytes = cap.checked_mul(size).ok_or(Error::Limit)?;
        if self.bytes.checked_add(bytes).ok_or(Error::Limit)? > self.field.compact.limits.heap_bytes
        {
            return Err(Error::Limit);
        }
        self.charge(v.len() as u64)?;
        v.try_reserve_exact(cap - v.len())
            .map_err(|_| Error::Allocation)?;
        let new = v.capacity() * size;
        if self.bytes.checked_add(new).ok_or(Error::Limit)? > self.field.compact.limits.heap_bytes {
            return Err(Error::Limit);
        }
        self.bytes = self.bytes - old + new;
        Ok(())
    }
    fn push<T>(&mut self, v: &mut Vec<T>, value: T) -> Result<(), Error> {
        self.charge(1)?;
        self.reserve(v, 1)?;
        v.push(value);
        Ok(())
    }
    fn release<T>(&mut self, v: &Vec<T>) {
        self.bytes -= v.capacity() * std::mem::size_of::<T>();
    }
    fn link(&mut self, e: Entry, dir: usize) -> Result<Option<Entry>, Error> {
        Ok(self
            .field
            .compact
            .link(e.cell as usize, e.index as usize, dir)?
            .map(|(cell, index)| Entry {
                cell: cell as u32,
                index: index as u32,
            }))
    }
    fn indices(&self, cell: usize) -> std::ops::Range<usize> {
        self.field.compact.indices(cell)
    }
    fn area(&self, e: Entry) -> u8 {
        self.field.compact.spans[e.index as usize].area
    }
    fn edge_region(&mut self, e: Entry, dir: usize) -> Result<u16, Error> {
        Ok(self
            .link(e, dir)?
            .map(|n| self.reg[n.index as usize])
            .unwrap_or(0))
    }
}

pub use contours::detour;
