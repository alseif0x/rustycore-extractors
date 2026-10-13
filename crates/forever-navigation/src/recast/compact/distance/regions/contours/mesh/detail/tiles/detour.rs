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

// Modified/ported 2026-10-11: explicit little-endian Detour 7 / 64-bit-ref builder.
// The Forever flag projection is adapted from TileBuilder.cpp, part of the
// TrinityCore Project. See upstream AUTHORS for Copyright information.
// This program is free software; you can redistribute it and/or modify it
// under the terms of the GNU General Public License as published by the
// Free Software Foundation; either version 2 of the License, or (at your
// option) any later version.
// This program is distributed in the hope that it will be useful, but WITHOUT
// ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or
// FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for
// more details.
// You should have received a copy of the GNU General Public License along
// with this program. If not, see <http://www.gnu.org/licenses/>.
//! Original Detour tile payload construction from paired Recast geometry.
//! Does not wrap MMap headers, select product configuration or establish routing.
use super::TileMesh;
use crate::offmesh::Connection;
mod bv;
mod encode;
mod save;
pub use save::{SaveError, SaveLimits, SaveStatistics, SavedData};
#[cfg(test)]
mod tests;
type Point = [f32; 3];
pub const VERSION: u32 = 7;
pub const MAGIC: u32 = 0x444e4156;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Limit,
    Allocation,
    EmptyMesh,
    Numeric,
    Options,
    Area,
}
impl Error {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Limit => "detour_policy_limit",
            Self::Allocation => "detour_allocation_failed",
            Self::EmptyMesh => "detour_empty_mesh",
            Self::Numeric => "detour_unrepresentable_numeric",
            Self::Options => "detour_invalid_options",
            Self::Area => "detour_forever_area_out_of_range",
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}
impl std::error::Error for Error {}
#[derive(Clone, Copy, Debug)]
pub enum FlagPolicy {
    Preserve,
    Forever,
}
#[derive(Clone, Copy, Debug)]
pub struct Options {
    pub tile: [i32; 3],
    pub user_id: u32,
    /// Explicit dtNavMeshCreateParams bounds, in world units.
    pub min: Point,
    pub max: Point,
    pub walkable_height: f32,
    pub walkable_radius: f32,
    pub walkable_climb: f32,
    pub build_bv_tree: bool,
    pub flags: FlagPolicy,
}
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub connections: usize,
    pub output_bytes: usize,
    pub heap_bytes: usize,
    pub work: u64,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            connections: 65536,
            output_bytes: 64 * 1024 * 1024,
            heap_bytes: 128 * 1024 * 1024,
            work: 128_000_000,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Statistics {
    pub stored_connections: usize,
    pub culled_connections: usize,
    pub maximum_links: usize,
    pub extra_detail_vertices: usize,
    /// Original u16 detail cursor wraps, including an unused final wrap.
    pub detail_base_wraps: u32,
    /// Original one-based neighbor encoding that overlaps DT_EXT_LINK.
    pub neighbor_marker_collisions: u32,
    pub work: u64,
}
pub struct Data {
    bytes: Vec<u8>,
    statistics: Statistics,
}
impl Data {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
    pub const fn statistics(&self) -> Statistics {
        self.statistics
    }
}
struct Budget {
    limits: Limits,
    bytes: usize,
    work: u64,
}
impl Budget {
    fn charge(&mut self, n: u64) -> Result<(), Error> {
        self.work = self.work.checked_add(n).ok_or(Error::Limit)?;
        if self.work > self.limits.work {
            Err(Error::Limit)
        } else {
            Ok(())
        }
    }
    fn filled<T: Clone>(&mut self, n: usize, value: T) -> Result<Vec<T>, Error> {
        let size = n
            .checked_mul(std::mem::size_of::<T>())
            .ok_or(Error::Limit)?;
        if self.bytes.checked_add(size).ok_or(Error::Limit)? > self.limits.heap_bytes {
            return Err(Error::Limit);
        }
        self.charge(n as u64)?;
        let mut v = Vec::new();
        v.try_reserve_exact(n).map_err(|_| Error::Allocation)?;
        self.bytes = self
            .bytes
            .checked_add(v.capacity() * std::mem::size_of::<T>())
            .ok_or(Error::Limit)?;
        if self.bytes > self.limits.heap_bytes {
            return Err(Error::Limit);
        }
        v.resize(n, value);
        Ok(v)
    }
}
fn finite(v: f32) -> Result<f32, Error> {
    if v.is_finite() {
        Ok(v)
    } else {
        Err(Error::Numeric)
    }
}
/// Checked counterpart to TileBuilder's area flag projection. Areas >11 make
/// the pinned native signed shift undefined, and are rejected in Forever mode.
pub fn forever_flags(area: u8, existing: u16) -> Result<u16, Error> {
    let area = area & 63;
    match area {
        0 => Ok(existing),
        1..=7 => Ok(1),
        8..=11 => Ok(1 << (11 - area)),
        _ => Err(Error::Area),
    }
}
fn classify(p: Point, min: Point, max: Point) -> u8 {
    let code = u8::from(p[0] >= max[0])
        | (u8::from(p[2] >= max[2]) << 1)
        | (u8::from(p[0] < min[0]) << 2)
        | (u8::from(p[2] < min[2]) << 3);
    match code {
        1 => 0,
        3 => 1,
        2 => 2,
        6 => 3,
        4 => 4,
        12 => 5,
        8 => 6,
        9 => 7,
        _ => 255,
    }
}
/// Borrowed source residency is excluded; owned class/tree/output buffers are
/// bounded. The result preserves native defined truncations and reports them;
/// it is not a claim that every supplied mesh is suitable for runtime routes.
pub fn build(
    mesh: &TileMesh,
    connections: &[Connection],
    options: Options,
    limits: Limits,
) -> Result<Data, Error> {
    let max = Limits::default();
    if limits.connections > max.connections
        || limits.output_bytes > max.output_bytes
        || limits.heap_bytes > max.heap_bytes
        || limits.work > max.work
        || connections.len() > limits.connections
    {
        return Err(Error::Limit);
    }
    if mesh.vertices().is_empty() || mesh.polygons().is_empty() {
        return Err(Error::EmptyMesh);
    }
    if options.tile[2] < 0
        || !options
            .min
            .into_iter()
            .chain(options.max)
            .all(f32::is_finite)
        || (0..3).any(|a| options.min[a] > options.max[a])
        || [
            options.walkable_height,
            options.walkable_radius,
            options.walkable_climb,
        ]
        .into_iter()
        .any(|v| !v.is_finite() || v < 0.0)
    {
        return Err(Error::Options);
    }
    let h = mesh.header();
    let quantum = finite(1.0 / h.cell_size)?;
    let mut b = Budget {
        limits,
        bytes: 0,
        work: 0,
    };
    let mut classes = b.filled(connections.len(), [0u8; 2])?;
    let mut statistics = Statistics::default();
    let mut connection_links = 0usize;
    if !connections.is_empty() {
        let (mut low, mut high) = (f32::MAX, -f32::MAX);
        for p in mesh.detail_vertices() {
            b.charge(1)?;
            if low >= p[1] {
                low = p[1];
            }
            if high <= p[1] {
                high = p[1];
            }
        }
        let mut min = options.min;
        let mut max = options.max;
        min[1] = finite(low - options.walkable_climb)?;
        max[1] = finite(high + options.walkable_climb)?;
        for (c, class) in connections.iter().zip(&mut classes) {
            b.charge(1)?;
            if c.radius() < 0.0
                || !c.radius().is_finite()
                || !c.endpoints().into_iter().flatten().all(f32::is_finite)
            {
                return Err(Error::Options);
            }
            let p = c.endpoints();
            *class = [classify(p[0], min, max), classify(p[1], min, max)];
            if class[0] == 255 && (p[0][1] < min[1] || p[0][1] > max[1]) {
                class[0] = 0;
            }
            connection_links += usize::from(class[0] == 255) + usize::from(class[1] == 255);
            statistics.stored_connections += usize::from(class[0] == 255);
        }
    }
    statistics.culled_connections = connections.len() - statistics.stored_connections;
    let total_vertices = mesh.vertices().len() + statistics.stored_connections * 2;
    let total_polygons = mesh.polygons().len() + statistics.stored_connections;
    if total_vertices > 65536 || total_polygons > 65536 {
        return Err(Error::Limit);
    }
    let mut edges = 0usize;
    let mut portals = 0usize;
    for (p, s) in mesh.polygons().iter().zip(mesh.submeshes()) {
        b.charge(1)?;
        edges += p.vertices().len();
        statistics.extra_detail_vertices += s.vertex_count as usize - p.vertices().len();
        if matches!(options.flags, FlagPolicy::Forever) {
            forever_flags(p.area(), p.flags())?;
        }
        for &n in p.neighbors() {
            b.charge(1)?;
            if n & 0x8000 != 0 && n & 15 != 15 {
                portals += 1;
            }
            if n == 32767 {
                statistics.neighbor_marker_collisions += 1;
            }
        }
    }
    statistics.maximum_links = edges + portals * 2 + connection_links * 2;
    let layout = encode::Layout::new(
        mesh,
        total_vertices,
        total_polygons,
        &statistics,
        options.build_bv_tree,
    )?;
    if layout.bytes > limits.output_bytes {
        return Err(Error::Limit);
    }
    let mut bytes = b.filled(layout.bytes, 0u8)?;
    b.charge(layout.bytes as u64)?;
    encode::write(
        &mut bytes,
        &layout,
        mesh,
        connections,
        &classes,
        options,
        &mut statistics,
    )?;
    if options.build_bv_tree {
        bv::write(&mut b, &mut bytes, layout.tree, mesh, options.min, quantum)?;
    }
    statistics.work = b.work;
    Ok(Data { bytes, statistics })
}
