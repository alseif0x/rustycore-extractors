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

// Modified/ported 2026-10-11: Rust index-arena heightfield, checked public API and limits.
// Behavioral oracle: Recast at TrinityCore 02245dcd245e7433e524577656177723d3e4992e.
// Rust adaptation of the Recast algorithms; this is not the original software.
//! Conservative voxelization, the three ordered walkability filters and initial
//! compact-heightfield adjacency, erosion, median areas and distance fields.
//! Watershed regions, simplified contours, polygon and detail meshes are available.
//! Ordered subtile merging and Detour tile serialization are available.
//! Complete TileBuilder/MMap orchestration and real routing acceptance remain.

mod compact;
mod filter;
mod raster;
pub use compact::{Cell, Compact, CompactSpan, DistanceField, RegionOptions, Regions};
pub use compact::{
    Contour, ContourOptions, ContourSet, DetailMesh, DetailOptions, DetailSubmesh, MergeLimits,
    MeshHeader, PolyMesh, Polygon, TileMesh,
};
pub use raster::solid_area;
#[cfg(test)]
mod tests;

const NONE: u32 = u32::MAX;
pub const MAX_HEIGHT: u16 = u16::MAX;
pub const NOT_CONNECTED: u8 = 63;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Limit,
    Allocation,
    Grid,
    Geometry,
    Span,
    Numeric,
    Clip,
    Layers,
    Radius,
    Regions,
    RegionContour,
    Contour,
    ContourHoles,
    Mesh,
    Triangulation,
    MeshVertexRemoval,
    DetailHeight,
    DetailTriangulation,
    DetailLimit,
    MergeContract,
}
impl Error {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Limit => "recast_policy_limit",
            Self::Allocation => "recast_allocation_failed",
            Self::Grid => "recast_invalid_grid",
            Self::Geometry => "recast_invalid_geometry",
            Self::Span => "recast_invalid_span",
            Self::Numeric => "recast_unrepresentable_numeric",
            Self::Clip => "recast_clip_capacity",
            Self::Layers => "recast_unrepresentable_layers",
            Self::Radius => "recast_invalid_radius",
            Self::Regions => "recast_region_id_limit",
            Self::RegionContour => "recast_region_contour_incomplete",
            Self::Contour => "recast_contour_incomplete",
            Self::ContourHoles => "recast_contour_holes_unresolved",
            Self::Mesh => "recast_unrepresentable_polygon_mesh",
            Self::Triangulation => "recast_triangulation_incomplete",
            Self::MeshVertexRemoval => "recast_mesh_vertex_removal_incomplete",
            Self::DetailHeight => "recast_detail_height_unavailable",
            Self::DetailTriangulation => "recast_detail_triangulation_incomplete",
            Self::DetailLimit => "recast_detail_native_capacity",
            Self::MergeContract => "recast_merge_incompatible_meshes",
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
/// Downward-only ceilings. Heap charges Vec capacities, including old/new
/// growth and heightfield/compact overlap; excludes caller inputs and allocator
/// overhead. Work is cumulative across consuming stages, not a CPU timer.
pub struct Limits {
    pub cells: usize,
    pub spans: usize,
    pub vertices: usize,
    pub triangles: usize,
    pub heap_bytes: usize,
    pub work: u64,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            cells: 1_048_576,
            spans: 2_000_000,
            vertices: 2_000_000,
            triangles: 1_000_000,
            heap_bytes: 128 * 1024 * 1024,
            work: 128_000_000,
        }
    }
}
impl Limits {
    fn validate(self) -> Result<(), Error> {
        let m = Self::default();
        if self.cells > m.cells
            || self.spans > m.spans
            || self.vertices > m.vertices
            || self.triangles > m.triangles
            || self.heap_bytes > m.heap_bytes
            || self.work > m.work
        {
            Err(Error::Limit)
        } else {
            Ok(())
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
/// Explicit Recast grid. Each dimension is 1..=1024. Bounding extents need not
/// equal dimension * cell_size; no native subtile/configuration policy is chosen.
pub struct Grid {
    pub width: u16,
    pub height: u16,
    pub min: [f32; 3],
    pub max: [f32; 3],
    pub cell_size: f32,
    pub cell_height: f32,
}
impl Grid {
    fn validate(self) -> Result<(), Error> {
        if self.width == 0
            || self.height == 0
            || self.width > 1024
            || self.height > 1024
            || !self.min.into_iter().chain(self.max).all(f32::is_finite)
            || (0..3).any(|i| self.min[i] > self.max[i])
            || !self.cell_size.is_finite()
            || !self.cell_height.is_finite()
            || self.cell_size <= 0.0
            || self.cell_height <= 0.0
        {
            return Err(Error::Grid);
        }
        for n in [
            1.0 / self.cell_size,
            1.0 / self.cell_height,
            self.max[0] - self.min[0],
            self.max[1] - self.min[1],
            self.max[2] - self.min[2],
            self.min[0] + f32::from(self.width) * self.cell_size,
            self.min[2] + f32::from(self.height) * self.cell_size,
        ] {
            finite(n)?;
        }
        Ok(())
    }
    fn cells(self) -> usize {
        usize::from(self.width) * usize::from(self.height)
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Agent {
    pub height: u16,
    pub climb: u16,
}
#[derive(Clone, Copy, Debug)]
pub struct Slopes {
    pub maximum: f32,
    pub non_steep: f32,
}
impl Default for Slopes {
    fn default() -> Self {
        Self {
            maximum: 55.0,
            non_steep: 55.0,
        }
    }
}
impl Slopes {
    fn thresholds(self) -> Result<[f32; 2], Error> {
        if !self.maximum.is_finite()
            || !self.non_steep.is_finite()
            || self.non_steep < 0.0
            || self.maximum > 90.0
            || self.non_steep > self.maximum
        {
            return Err(Error::Geometry);
        }
        Ok(
            [self.maximum, self.non_steep]
                .map(|a| ((a / 180.0) * f32::from_bits(0x40490fdb)).cos()),
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    pub min: u16,
    pub max: u16,
    pub area: u8,
}
#[derive(Clone, Copy, Debug)]
pub struct SpanInput {
    pub x: u16,
    pub z: u16,
    pub span: Span,
}
#[derive(Clone, Copy)]
struct Node {
    span: Span,
    next: u32,
}
pub struct Heightfield {
    grid: Grid,
    heads: Vec<u32>,
    nodes: Vec<Node>,
    free: u32,
    live: usize,
    limits: Limits,
    work: u64,
}
pub struct Spans<'a> {
    field: &'a Heightfield,
    next: u32,
}
impl Iterator for Spans<'_> {
    type Item = Span;
    fn next(&mut self) -> Option<Span> {
        if self.next == NONE {
            return None;
        }
        let n = self.field.nodes[self.next as usize];
        self.next = n.next;
        Some(n.span)
    }
}

impl Heightfield {
    pub fn new(grid: Grid, limits: Limits) -> Result<Self, Error> {
        limits.validate()?;
        grid.validate()?;
        let n = grid.cells();
        if n > limits.cells || n * 4 > limits.heap_bytes {
            return Err(Error::Limit);
        }
        let mut heads = Vec::new();
        heads.try_reserve_exact(n).map_err(|_| Error::Allocation)?;
        if heads.capacity() * 4 > limits.heap_bytes {
            return Err(Error::Limit);
        }
        heads.resize(n, NONE);
        Ok(Self {
            grid,
            heads,
            nodes: Vec::new(),
            free: NONE,
            live: 0,
            limits,
            work: 0,
        })
    }
    pub const fn grid(&self) -> Grid {
        self.grid
    }
    pub const fn span_count(&self) -> usize {
        self.live
    }
    pub const fn work_used(&self) -> u64 {
        self.work
    }
    pub fn capacity_bytes(&self) -> usize {
        self.heads.capacity() * 4 + self.nodes.capacity() * std::mem::size_of::<Node>()
    }
    pub fn spans(&self, x: u16, z: u16) -> Option<Spans<'_>> {
        if x >= self.grid.width || z >= self.grid.height {
            return None;
        }
        Some(Spans {
            field: self,
            next: self.heads[usize::from(x) + usize::from(z) * usize::from(self.grid.width)],
        })
    }
    /// Explicit voxel spans; consuming operations never return a partial field.
    pub fn add_spans(mut self, spans: &[SpanInput], merge_threshold: u16) -> Result<Self, Error> {
        if spans.len() > self.limits.spans {
            return Err(Error::Limit);
        }
        self.charge(spans.len() as u64)?;
        if spans.iter().any(|s| {
            s.x >= self.grid.width
                || s.z >= self.grid.height
                || s.span.min >= s.span.max
                || s.span.area > 63
        }) {
            return Err(Error::Span);
        }
        for s in spans {
            self.insert(
                usize::from(s.x) + usize::from(s.z) * usize::from(self.grid.width),
                s.span,
                merge_threshold,
            )?;
        }
        Ok(self)
    }
    fn charge(&mut self, n: u64) -> Result<(), Error> {
        self.work = self.work.checked_add(n).ok_or(Error::Limit)?;
        if self.work > self.limits.work {
            Err(Error::Limit)
        } else {
            Ok(())
        }
    }
    fn allocate(&mut self, span: Span) -> Result<u32, Error> {
        let id = if self.free != NONE {
            let id = self.free;
            self.free = self.nodes[id as usize].next;
            id
        } else {
            if self.nodes.len() >= self.limits.spans {
                return Err(Error::Limit);
            }
            if self.nodes.len() == self.nodes.capacity() {
                let old_bytes = self.nodes.capacity() * std::mem::size_of::<Node>();
                let cap = (self.nodes.capacity().max(32) * 2).min(self.limits.spans);
                let size = cap * std::mem::size_of::<Node>();
                if self
                    .capacity_bytes()
                    .checked_add(size)
                    .ok_or(Error::Limit)?
                    > self.limits.heap_bytes
                {
                    return Err(Error::Limit);
                }
                self.nodes
                    .try_reserve_exact(cap - self.nodes.len())
                    .map_err(|_| Error::Allocation)?;
                if self.capacity_bytes() + old_bytes > self.limits.heap_bytes {
                    return Err(Error::Limit);
                }
            }
            let id = self.nodes.len() as u32;
            self.nodes.push(Node { span, next: NONE });
            id
        };
        self.nodes[id as usize] = Node { span, next: NONE };
        self.live += 1;
        Ok(id)
    }
    fn insert(&mut self, cell: usize, mut span: Span, threshold: u16) -> Result<(), Error> {
        self.charge(1)?;
        let id = self.allocate(span)?;
        let mut prev = NONE;
        let mut current = self.heads[cell];
        while current != NONE {
            self.charge(1)?;
            let n = self.nodes[current as usize];
            if n.span.min > span.max {
                break;
            }
            if n.span.max < span.min {
                prev = current;
                current = n.next;
                continue;
            }
            // Forever patch: compare upper endpoints BEFORE expanding the span.
            if span.max.abs_diff(n.span.max) <= threshold {
                span.area = span.area.max(n.span.area);
            }
            span.min = span.min.min(n.span.min);
            span.max = span.max.max(n.span.max);
            self.nodes[current as usize].next = self.free;
            self.free = current;
            self.live -= 1;
            if prev == NONE {
                self.heads[cell] = n.next;
            } else {
                self.nodes[prev as usize].next = n.next;
            }
            current = n.next;
        }
        self.nodes[id as usize] = Node {
            span,
            next: current,
        };
        if prev == NONE {
            self.heads[cell] = id;
        } else {
            self.nodes[prev as usize].next = id;
        }
        Ok(())
    }
}

/// Mirrors the pinned TileBuilder order: solid classification/rasterization,
/// low-obstacle/ledge/clearance filters, liquid rasterization, then compaction.
/// Grid/subtile selection is explicit. No map-specific configuration is inferred.
pub fn build(
    mesh: &crate::Mesh,
    grid: Grid,
    slopes: Slopes,
    agent: Agent,
    limits: Limits,
) -> Result<Compact, Error> {
    Heightfield::new(grid, limits)?
        .rasterize_solid(
            mesh.solid_vertices(),
            mesh.solid_triangles(),
            slopes,
            agent.climb,
        )?
        .filter_low_obstacles(agent.climb)?
        .filter_ledges(agent)?
        .filter_clearance(agent.height)?
        .rasterize(
            mesh.liquid_vertices(),
            mesh.liquid_triangles(),
            mesh.liquid_areas(),
            agent.climb,
        )?
        .compact(agent)
}
fn finite(value: f32) -> Result<f32, Error> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(Error::Numeric)
    }
}
fn integer(value: f32) -> Result<i32, Error> {
    if !value.is_finite() || !(-2147483648.0..2147483648.0).contains(&value) {
        Err(Error::Numeric)
    } else {
        Ok(value as i32)
    }
}
fn neighbor(grid: Grid, cell: usize, d: usize) -> Option<usize> {
    let w = usize::from(grid.width);
    let (x, z) = (cell % w, cell / w);
    match d {
        0 => (x > 0).then(|| cell - 1),
        1 => (z + 1 < usize::from(grid.height)).then(|| cell + w),
        2 => (x + 1 < w).then(|| cell + 1),
        3 => (z > 0).then(|| cell - w),
        _ => None,
    }
}

pub use compact::detour;
