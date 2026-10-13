// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: bounded navigation geometry from checked MAPS inputs.
//! Pure Rust navigation geometry, Recast stages and Detour tile payloads.
//! Real acquisition, complete TileBuilder/MMap orchestration and world routing remain.

pub mod inventory;
pub mod mmap;
pub mod models;
pub mod offmesh;
pub mod pipeline;
pub mod recast;
pub mod terrain;
pub mod tilebuilder;

pub const ORACLE_COMMIT: &str = "02245dcd245e7433e524577656177723d3e4992e";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Limit,
    Allocation,
    Coordinates,
    BuildMismatch,
    NonFinite,
    LiquidTypes,
    Transform(forever_vmap::query::Error),
}
impl Error {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Limit => "navigation_policy_limit",
            Self::Allocation => "navigation_allocation_failed",
            Self::Coordinates => "navigation_tile_coordinates",
            Self::BuildMismatch => "navigation_build_mismatch",
            Self::NonFinite => "navigation_nonfinite_geometry",
            Self::LiquidTypes => "navigation_invalid_liquid_types",
            Self::Transform(_) => "navigation_invalid_instance_transform",
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}
impl std::error::Error for Error {}

/// Logical capacity admission; not allocator overhead, RSS or caller residency.
/// Overrides can only reduce the built-in ceilings. Caller inputs are borrowed.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub vertices: usize,
    pub triangles: usize,
    pub heap_bytes: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            vertices: 250_000,
            triangles: 500_000,
            heap_bytes: 24 * 1024 * 1024,
        }
    }
}
impl Limits {
    fn validate(self) -> Result<(), Error> {
        let max = Self::default();
        if self.vertices > max.vertices
            || self.triangles > max.triangles
            || self.heap_bytes > max.heap_bytes
        {
            return Err(Error::Limit);
        }
        Ok(())
    }
}

/// Separate indexed solid and liquid meshes in Recast (y, height, x) coordinates.
/// Vectors cannot be mutated externally; all indices and liquid areas are paired.
#[derive(Debug, Default)]
pub struct Mesh {
    solid_vertices: Vec<[f32; 3]>,
    solid_triangles: Vec<[u32; 3]>,
    liquid_vertices: Vec<[f32; 3]>,
    liquid_triangles: Vec<[u32; 3]>,
    liquid_areas: Vec<u8>,
}
impl Mesh {
    pub fn solid_vertices(&self) -> &[[f32; 3]] {
        &self.solid_vertices
    }
    pub fn solid_triangles(&self) -> &[[u32; 3]] {
        &self.solid_triangles
    }
    pub fn liquid_vertices(&self) -> &[[f32; 3]] {
        &self.liquid_vertices
    }
    pub fn liquid_triangles(&self) -> &[[u32; 3]] {
        &self.liquid_triangles
    }
    pub fn liquid_areas(&self) -> &[u8] {
        &self.liquid_areas
    }
    pub fn has_triangles(&self) -> bool {
        !self.solid_triangles.is_empty() || !self.liquid_triangles.is_empty()
    }
    pub fn capacity_bytes(&self) -> usize {
        12 * (self.solid_vertices.capacity()
            + self.solid_triangles.capacity()
            + self.liquid_vertices.capacity()
            + self.liquid_triangles.capacity())
            + self.liquid_areas.capacity()
    }

    /// Drop unreferenced vertices and renumber by first index encounter. Equal
    /// positions remain separate when they had different source vertex indices.
    /// Consumes the mesh so errors cannot expose partially rewritten indices.
    pub fn compact(mut self, limits: Limits) -> Result<Self, Error> {
        limits.validate()?;
        if self.solid_vertices.len() + self.liquid_vertices.len() > limits.vertices
            || self.solid_triangles.len() + self.liquid_triangles.len() > limits.triangles
        {
            return Err(Error::Limit);
        }
        let peak = self.solid_vertices.len().max(self.liquid_vertices.len());
        if self
            .capacity_bytes()
            .checked_add(peak * 16)
            .ok_or(Error::Limit)?
            > limits.heap_bytes
        {
            return Err(Error::Limit);
        }
        let resident = self.capacity_bytes();
        compact_vertices(
            &mut self.solid_vertices,
            &mut self.solid_triangles,
            resident,
            limits,
        )?;
        let resident = self.capacity_bytes();
        compact_vertices(
            &mut self.liquid_vertices,
            &mut self.liquid_triangles,
            resident,
            limits,
        )?;
        Ok(self)
    }
}
fn reserve<T>(v: &mut Vec<T>, n: usize) -> Result<(), Error> {
    v.try_reserve_exact(n).map_err(|_| Error::Allocation)
}
fn compact_vertices(
    vertices: &mut Vec<[f32; 3]>,
    triangles: &mut [[u32; 3]],
    resident: usize,
    limits: Limits,
) -> Result<(), Error> {
    let mut remap = Vec::new();
    let mut out = Vec::new();
    reserve(&mut remap, vertices.len())?;
    reserve(&mut out, vertices.len())?;
    if resident + remap.capacity() * 4 + out.capacity() * 12 > limits.heap_bytes {
        return Err(Error::Limit);
    }
    remap.resize(vertices.len(), u32::MAX);
    for index in triangles.iter_mut().flatten() {
        let source = *index as usize;
        if remap[source] == u32::MAX {
            remap[source] = out.len() as u32;
            out.push(vertices[source]);
        }
        *index = remap[source];
    }
    *vertices = out;
    Ok(())
}

pub use recast::detour;
