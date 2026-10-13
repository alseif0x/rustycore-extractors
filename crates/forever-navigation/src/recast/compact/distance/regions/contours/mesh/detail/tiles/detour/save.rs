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

// Modified/ported 2026-10-11: fresh single-tile addTile state before original save.
//! Consumes builder output. This is the offline producer's fresh-navmesh save
//! state, not a mutable multi-tile runtime or a parser for arbitrary payloads.
use super::{Data, Point};
use crate::mmap::Params;
const NULL: u32 = u32::MAX;
const BASE: u64 = 1 << 52;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SaveError {
    Limit,
    Topology,
    Numeric,
    Params,
    Diagnostics,
}
impl SaveError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Limit => "detour_save_policy_limit",
            Self::Topology => "detour_save_invalid_topology",
            Self::Numeric => "detour_save_unrepresentable_numeric",
            Self::Params => "detour_save_invalid_params",
            Self::Diagnostics => "detour_save_builder_diagnostics",
        }
    }
}
impl std::fmt::Display for SaveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}
impl std::error::Error for SaveError {}
#[derive(Clone, Copy, Debug)]
pub struct SaveLimits {
    pub work: u64,
}
impl Default for SaveLimits {
    fn default() -> Self {
        Self { work: 128_000_000 }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SaveStatistics {
    pub links: usize,
    pub snapped_endpoints: usize,
    pub work: u64,
}
pub struct SavedData {
    bytes: Vec<u8>,
    statistics: SaveStatistics,
}
impl SavedData {
    pub fn capacity_bytes(&self) -> usize {
        self.bytes.capacity()
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
    pub const fn statistics(&self) -> SaveStatistics {
        self.statistics
    }
}
impl Data {
    /// Fresh tile index 0, salt 1, no neighbours, no restored reference. Reuses
    /// the owned byte allocation, fixed 128-candidate stack and no heap scratch.
    /// Builder detail-wrap/neighbor-marker diagnostics are rejected explicitly.
    pub fn prepare_for_save(
        self,
        params: Params,
        limits: SaveLimits,
    ) -> Result<SavedData, SaveError> {
        params.validate().map_err(|_| SaveError::Params)?;
        if limits.work > SaveLimits::default().work {
            return Err(SaveError::Limit);
        }
        if self.statistics.detail_base_wraps != 0 || self.statistics.neighbor_marker_collisions != 0
        {
            return Err(SaveError::Diagnostics);
        }
        let layout = Layout::new(&self.bytes)?;
        let mut state = State {
            bytes: self.bytes,
            l: layout,
            stats: SaveStatistics::default(),
            limit: limits.work,
        };
        state.validate()?;
        state.initialize()?;
        state.offmesh(false)?;
        state.offmesh(true)?;
        Ok(SavedData {
            bytes: state.bytes,
            statistics: state.stats,
        })
    }
}
fn word(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(b[at..at + 4].try_into().expect("checked layout"))
}
fn half(b: &[u8], at: usize) -> usize {
    u16::from_le_bytes(b[at..at + 2].try_into().expect("checked layout")) as usize
}
fn float(b: &[u8], at: usize) -> f32 {
    f32::from_bits(word(b, at))
}
fn point(b: &[u8], at: usize) -> Point {
    std::array::from_fn(|a| float(b, at + 4 * a))
}
fn put(b: &mut [u8], at: usize, v: u32) {
    b[at..at + 4].copy_from_slice(&v.to_le_bytes());
}
fn finite(v: f32) -> Result<f32, SaveError> {
    if v.is_finite() {
        Ok(v)
    } else {
        Err(SaveError::Numeric)
    }
}
fn checked_point(p: Point) -> Result<Point, SaveError> {
    for v in p {
        finite(v)?;
    }
    Ok(p)
}
#[derive(Clone, Copy)]
struct Layout {
    np: usize,
    nv: usize,
    nl: usize,
    ne: usize,
    nt: usize,
    nb: usize,
    no: usize,
    base: usize,
    polys: usize,
    links: usize,
    details: usize,
    extra: usize,
    tris: usize,
    bv: usize,
    off: usize,
}
impl Layout {
    fn new(b: &[u8]) -> Result<Self, SaveError> {
        if b.len() < 100
            || b.len() > 64 * 1024 * 1024
            || word(b, 0) != super::MAGIC
            || word(b, 4) != super::VERSION
        {
            return Err(SaveError::Topology);
        }
        let mut n = [0; 9];
        for (i, v) in n.iter_mut().enumerate() {
            *v = word(b, 24 + i * 4) as usize;
        }
        let [np, nv, nl, nd, ne, nt, nb, no, base] = n;
        if np == 0
            || np > 65536
            || nv == 0
            || nv > 65536
            || nl == 0
            || nd != base
            || base + no != np
        {
            return Err(SaveError::Topology);
        }
        let mut cursor = 100usize;
        let mut block = |count: usize, size: usize| -> Result<usize, SaveError> {
            let at = cursor;
            cursor = cursor
                .checked_add(count.checked_mul(size).ok_or(SaveError::Topology)?)
                .ok_or(SaveError::Topology)?;
            if cursor > b.len() {
                return Err(SaveError::Topology);
            }
            Ok(at)
        };
        block(nv, 12)?;
        let polys = block(np, 32)?;
        let links = block(nl, 16)?;
        let details = block(nd, 12)?;
        let extra = block(ne, 12)?;
        let tris = block(nt, 4)?;
        let bv = block(nb, 16)?;
        let off = block(no, 36)?;
        if cursor != b.len() {
            return Err(SaveError::Topology);
        }
        Ok(Self {
            np,
            nv,
            nl,
            ne,
            nt,
            nb,
            no,
            base,
            polys,
            links,
            details,
            extra,
            tris,
            bv,
            off,
        })
    }
}
struct State {
    bytes: Vec<u8>,
    l: Layout,
    stats: SaveStatistics,
    limit: u64,
}
impl State {
    fn charge(&mut self, n: u64) -> Result<(), SaveError> {
        self.stats.work = self.stats.work.checked_add(n).ok_or(SaveError::Limit)?;
        if self.stats.work > self.limit {
            return Err(SaveError::Limit);
        }
        Ok(())
    }
    fn poly(&self, i: usize) -> usize {
        self.l.polys + i * 32
    }
    fn vertex(&self, i: usize) -> Point {
        point(&self.bytes, 100 + i * 12)
    }
    fn corner(&self, p: usize, i: usize) -> Point {
        self.vertex(half(&self.bytes, self.poly(p) + 4 + i * 2))
    }
    fn detail_triangle(&self, p: usize, t: usize) -> ([u8; 4], [Point; 3]) {
        let d = self.l.details + p * 12;
        let at = self.l.tris + (word(&self.bytes, d + 4) as usize + t) * 4;
        let tri: [u8; 4] = self.bytes[at..at + 4].try_into().expect("checked triangle");
        let nv = self.bytes[self.poly(p) + 30] as usize;
        let v = std::array::from_fn(|j| {
            if (tri[j] as usize) < nv {
                self.corner(p, tri[j] as usize)
            } else {
                point(
                    &self.bytes,
                    self.l.extra + 12 * (word(&self.bytes, d) as usize + tri[j] as usize - nv),
                )
            }
        });
        (tri, v)
    }
    fn validate(&mut self) -> Result<(), SaveError> {
        self.charge((self.l.nv + self.l.ne + self.l.np + self.l.nb + self.l.no) as u64)?;
        for i in 0..self.l.nv {
            checked_point(self.vertex(i))?;
        }
        for i in 0..self.l.ne {
            checked_point(point(&self.bytes, self.l.extra + 12 * i))?;
        }
        for a in 0..3 {
            let lo = finite(float(&self.bytes, 72 + 4 * a))?;
            let hi = finite(float(&self.bytes, 84 + 4 * a))?;
            if lo > hi {
                return Err(SaveError::Topology);
            }
        }
        if finite(float(&self.bytes, 68))? < 0.0 || finite(float(&self.bytes, 96))? <= 0.0 {
            return Err(SaveError::Numeric);
        }
        for p in 0..self.l.np {
            let at = self.poly(p);
            let nv = self.bytes[at + 30] as usize;
            if (p < self.l.base && (!(3..=6).contains(&nv) || self.bytes[at + 31] >> 6 != 0))
                || (p >= self.l.base && (nv != 2 || self.bytes[at + 31] >> 6 != 1))
            {
                return Err(SaveError::Topology);
            }
            self.charge(nv as u64)?;
            for j in 0..nv {
                if half(&self.bytes, at + 4 + j * 2) >= self.l.nv {
                    return Err(SaveError::Topology);
                }
                let nei = half(&self.bytes, at + 16 + j * 2);
                if nei != 0 && nei & 0x8000 == 0 && nei > self.l.base {
                    return Err(SaveError::Topology);
                }
            }
            if p >= self.l.base {
                continue;
            }
            let d = self.l.details + p * 12;
            let vb = word(&self.bytes, d) as usize;
            let tb = word(&self.bytes, d + 4) as usize;
            let vc = self.bytes[d + 8] as usize;
            let tc = self.bytes[d + 9] as usize;
            if vb + vc > self.l.ne || tb + tc > self.l.nt {
                return Err(SaveError::Topology);
            }
            self.charge(tc as u64)?;
            for t in 0..tc {
                let at = self.l.tris + (tb + t) * 4;
                if self.bytes[at..at + 3]
                    .iter()
                    .any(|&i| i as usize >= nv + vc)
                {
                    return Err(SaveError::Topology);
                }
            }
        }
        for i in 0..self.l.nb {
            let at = self.l.bv + i * 16;
            let index = word(&self.bytes, at + 12) as i32;
            if (index >= 0 && index as usize >= self.l.base)
                || (index < 0 && (-(index as i64)) as usize > self.l.nb - i)
            {
                return Err(SaveError::Topology);
            }
        }
        for i in 0..self.l.no {
            let at = self.l.off + i * 36;
            checked_point(point(&self.bytes, at))?;
            checked_point(point(&self.bytes, at + 12))?;
            if finite(float(&self.bytes, at + 24))? < 0.0
                || half(&self.bytes, at + 28) != self.l.base + i
            {
                return Err(SaveError::Topology);
            }
        }
        Ok(())
    }
    fn link(&mut self, p: usize, target: usize, edge: u8) -> Result<(), SaveError> {
        self.charge(1)?;
        // allocLink returns NULL when exhausted; preserve its silent skip.
        if self.stats.links == self.l.nl {
            return Ok(());
        }
        let poly = self.poly(p);
        let at = self.l.links + self.stats.links * 16;
        self.bytes[at..at + 8].copy_from_slice(&(BASE | target as u64).to_le_bytes());
        let head = word(&self.bytes, poly);
        put(&mut self.bytes, at + 8, head);
        self.bytes[at + 12..at + 16].copy_from_slice(&[edge, 255, 0, 0]);
        put(&mut self.bytes, poly, self.stats.links as u32);
        self.stats.links += 1;
        Ok(())
    }
    fn initialize(&mut self) -> Result<(), SaveError> {
        self.charge((self.l.nl + self.l.np) as u64)?;
        for i in 0..self.l.nl {
            put(
                &mut self.bytes,
                self.l.links + i * 16 + 8,
                if i + 1 == self.l.nl {
                    NULL
                } else {
                    i as u32 + 1
                },
            );
        }
        for p in 0..self.l.np {
            let at = self.poly(p);
            put(&mut self.bytes, at, NULL);
            if p >= self.l.base {
                continue;
            }
            for j in (0..self.bytes[at + 30] as usize).rev() {
                let nei = half(&self.bytes, at + 16 + j * 2);
                if nei != 0 && nei & 0x8000 == 0 {
                    self.link(p, nei - 1, j as u8)?;
                }
            }
        }
        Ok(())
    }
    fn offmesh(&mut self, end: bool) -> Result<(), SaveError> {
        for i in 0..self.l.no {
            self.charge(1)?;
            let at = self.l.off + i * 36;
            let p = self.l.base + i;
            if end && (self.bytes[at + 31] != 255 || word(&self.bytes, self.poly(p)) == NULL) {
                continue;
            }
            let pos = point(&self.bytes, at + usize::from(end) * 12);
            let rad = float(&self.bytes, at + 24);
            let Some((land, nearest)) = self.nearest(pos, rad)? else {
                continue;
            };
            let dx = nearest[0] - pos[0];
            let dz = nearest[2] - pos[2];
            if finite(dx * dx + dz * dz)? > finite(rad * rad)? {
                continue;
            }
            let vi = half(&self.bytes, self.poly(p) + 4 + usize::from(end) * 2);
            for (a, v) in nearest.into_iter().enumerate() {
                put(&mut self.bytes, 100 + vi * 12 + a * 4, v.to_bits());
            }
            self.stats.snapped_endpoints += 1;
            self.link(p, land, u8::from(end))?;
            if !end || self.bytes[at + 30] & 1 != 0 {
                self.link(land, p, 255)?;
            }
        }
        Ok(())
    }
    fn nearest(&mut self, pos: Point, rad: f32) -> Result<Option<(usize, Point)>, SaveError> {
        let climb = float(&self.bytes, 68);
        let ext = [rad, climb, rad];
        let lo = checked_point(std::array::from_fn(|a| pos[a] - ext[a]))?;
        let hi = checked_point(std::array::from_fn(|a| pos[a] + ext[a]))?;
        let mut candidates = [0usize; 128];
        let mut count = 0;
        if self.l.nb != 0 {
            let mut qlo = [0u16; 3];
            let mut qhi = [0u16; 3];
            let fac = float(&self.bytes, 96);
            for a in 0..3 {
                let min = float(&self.bytes, 72 + 4 * a);
                let max = float(&self.bytes, 84 + 4 * a);
                let lower = finite(fac * (lo[a].clamp(min, max) - min))?;
                let upper = finite(fac * (hi[a].clamp(min, max) - min) + 1.0)?;
                // Reject undefined native float-to-ushort conversion, including
                // fractional values whose truncation exceeds ushort range.
                if !(0.0..65536.0).contains(&lower) || !(0.0..65536.0).contains(&upper) {
                    return Err(SaveError::Numeric);
                }
                qlo[a] = lower as u16 & 0xfffe;
                qhi[a] = upper as u16 | 1;
            }
            let mut i = 0;
            while i < self.l.nb {
                self.charge(1)?;
                let at = self.l.bv + i * 16;
                let index = word(&self.bytes, at + 12) as i32;
                let overlap = (0..3).all(|a| {
                    qlo[a] as usize <= half(&self.bytes, at + 6 + 2 * a)
                        && qhi[a] as usize >= half(&self.bytes, at + 2 * a)
                });
                if index >= 0 && overlap && count < 128 {
                    candidates[count] = index as usize;
                    count += 1;
                }
                i += if overlap || index >= 0 {
                    1
                } else {
                    (-index) as usize
                };
            }
        } else {
            for p in 0..self.l.base {
                self.charge(1)?;
                let nv = self.bytes[self.poly(p) + 30] as usize;
                let mut min = self.corner(p, 0);
                let mut max = min;
                for j in 1..nv {
                    let v = self.corner(p, j);
                    for a in 0..3 {
                        min[a] = min[a].min(v[a]);
                        max[a] = max[a].max(v[a]);
                    }
                }
                if (0..3).all(|a| lo[a] <= max[a] && hi[a] >= min[a]) && count < 128 {
                    candidates[count] = p;
                    count += 1;
                }
            }
        }
        let mut best = f32::MAX;
        let mut found = None;
        for &p in &candidates[..count] {
            self.charge(1)?;
            let (closest, over) = self.closest(p, pos)?;
            let d: Point = std::array::from_fn(|a| pos[a] - closest[a]);
            let dist = if over {
                let d = d[1].abs() - climb;
                if d > 0.0 { d * d } else { 0.0 }
            } else {
                d[0] * d[0] + d[1] * d[1] + d[2] * d[2]
            };
            finite(dist)?;
            if dist < best {
                best = dist;
                found = Some((p, closest));
            }
        }
        Ok(found)
    }
    fn closest(&mut self, p: usize, pos: Point) -> Result<(Point, bool), SaveError> {
        let nv = self.bytes[self.poly(p) + 30] as usize;
        let mut inside = false;
        for i in 0..nv {
            self.charge(1)?;
            let vi = self.corner(p, i);
            let vj = self.corner(p, if i == 0 { nv - 1 } else { i - 1 });
            if (vi[2] > pos[2]) != (vj[2] > pos[2])
                && pos[0] < finite((vj[0] - vi[0]) * (pos[2] - vi[2]) / (vj[2] - vi[2]) + vi[0])?
            {
                inside = !inside;
            }
        }
        let tc = self.bytes[self.l.details + p * 12 + 9] as usize;
        if inside {
            for t in 0..tc {
                self.charge(1)?;
                let (_, [a, b, c]) = self.detail_triangle(p, t);
                if let Some(h) = height(pos, a, b, c)? {
                    return Ok(([pos[0], h, pos[2]], true));
                }
            }
            let q = self.edges(p, pos, false)?;
            return Ok(([pos[0], q[1], pos[2]], true));
        }
        Ok((self.edges(p, pos, true)?, false))
    }
    fn edges(&mut self, p: usize, pos: Point, boundary: bool) -> Result<Point, SaveError> {
        let tc = self.bytes[self.l.details + p * 12 + 9] as usize;
        let mut best = f32::MAX;
        let mut result = None;
        for t in 0..tc {
            self.charge(1)?;
            let (tri, v) = self.detail_triangle(p, t);
            if boundary && tri[3] & 0x15 == 0 {
                continue;
            }
            for k in 0..3 {
                let j = if k == 0 { 2 } else { k - 1 };
                if (tri[3] >> (j * 2)) & 1 == 0 && (boundary || tri[j] < tri[k]) {
                    continue;
                }
                let (d, f) = segment(pos, v[j], v[k])?;
                if d < best {
                    best = d;
                    result = Some(checked_point(std::array::from_fn(|a| {
                        v[j][a] + (v[k][a] - v[j][a]) * f
                    }))?);
                }
            }
        }
        result.ok_or(SaveError::Topology)
    }
}
fn segment(pt: Point, p: Point, q: Point) -> Result<(f32, f32), SaveError> {
    let pqx = q[0] - p[0];
    let pqz = q[2] - p[2];
    let dx = pt[0] - p[0];
    let dz = pt[2] - p[2];
    let d = finite(pqx * pqx + pqz * pqz)?;
    let mut t = finite(pqx * dx + pqz * dz)?;
    if d > 0.0 {
        t /= d;
    }
    t = t.clamp(0.0, 1.0);
    let dx = p[0] + t * pqx - pt[0];
    let dz = p[2] + t * pqz - pt[2];
    Ok((finite(dx * dx + dz * dz)?, t))
}
fn height(p: Point, a: Point, b: Point, c: Point) -> Result<Option<f32>, SaveError> {
    let v0: Point = std::array::from_fn(|i| c[i] - a[i]);
    let v1: Point = std::array::from_fn(|i| b[i] - a[i]);
    let v2: Point = std::array::from_fn(|i| p[i] - a[i]);
    let mut denom = finite(v0[0] * v1[2] - v0[2] * v1[0])?;
    if denom.abs() < 1e-6 {
        return Ok(None);
    }
    let mut u = finite(v1[2] * v2[0] - v1[0] * v2[2])?;
    let mut v = finite(v0[0] * v2[2] - v0[2] * v2[0])?;
    if denom < 0.0 {
        denom = -denom;
        u = -u;
        v = -v;
    }
    if u >= 0.0 && v >= 0.0 && finite(u + v)? <= denom {
        return Ok(Some(finite(a[1] + (v0[1] * u + v1[1] * v) / denom)?));
    }
    Ok(None)
}
