// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of the TrinityCore Project. See AUTHORS for Copyright information.
// This program is free software; you can redistribute it and/or modify it under
// the terms of the GNU General Public License as published by the Free Software
// Foundation; either version 2 of the License, or (at your option) any later version.
// This program is distributed in the hope that it will be useful, but WITHOUT
// ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
// FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.
// You should have received a copy of the GNU General Public License along with
// this program. If not, see <http://www.gnu.org/licenses/>.
// Modified/ported 2026-10-11: original supplied-geometry TileBuilder orchestration.
//! Fixed target configuration and complete ordered 25x25 subtile generation.
//! Source acquisition, instance selection, parent inventory and publication are external.
use crate::{Mesh, detour, mmap, offmesh, recast};
mod build;
pub use build::{Data, Limits, Outcome, Statistics, build};
#[cfg(test)]
mod tests;

pub const GRID_SIZE: f32 = 533.3333;
pub const SUBTILES: u16 = 25;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Coordinates,
    Inventory,
    Numeric,
    Slopes,
    Limit,
    Allocation,
    Subtile { index: u16, source: recast::Error },
    Merge(recast::Error),
    Detour(detour::Error),
    Params(mmap::Error),
    PayloadDiagnostics,
}
impl Error {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Coordinates => "tilebuilder_coordinates",
            Self::Inventory => "tilebuilder_invalid_tile_inventory",
            Self::Numeric => "tilebuilder_unrepresentable_numeric",
            Self::Slopes => "tilebuilder_invalid_slopes",
            Self::Limit => "tilebuilder_policy_limit",
            Self::Allocation => "tilebuilder_allocation_failed",
            Self::Subtile { .. } => "tilebuilder_subtile_failed",
            Self::Merge(_) => "tilebuilder_merge_failed",
            Self::Detour(_) => "tilebuilder_detour_failed",
            Self::Params(_) => "tilebuilder_invalid_navmesh_params",
            Self::PayloadDiagnostics => "tilebuilder_native_payload_diagnostics",
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Subtile { index, source } => write!(f, "{}: {index}: {source}", self.code()),
            Self::Merge(e) => write!(f, "{}: {e}", self.code()),
            Self::Detour(e) => write!(f, "{}: {e}", self.code()),
            Self::Params(e) => write!(f, "{}: {e}", self.code()),
            _ => f.write_str(self.code()),
        }
    }
}
impl std::error::Error for Error {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Coordinate {
    pub row: u8,
    pub column: u8,
}
impl Coordinate {
    fn validate(self) -> Result<(), Error> {
        if self.row >= 64 || self.column >= 64 {
            Err(Error::Coordinates)
        } else {
            Ok(())
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Request {
    pub map_id: u32,
    pub tile: Coordinate,
    pub big_base_unit: bool,
    pub slopes: recast::Slopes,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds {
    pub min: [f32; 3],
    pub max: [f32; 3],
}
fn horizontal(tile: Coordinate) -> Result<Bounds, Error> {
    tile.validate()?;
    let max = [
        (32 - i32::from(tile.column)) as f32 * GRID_SIZE,
        f32::MAX,
        (32 - i32::from(tile.row)) as f32 * GRID_SIZE,
    ];
    Ok(Bounds {
        min: [max[0] - GRID_SIZE, f32::MIN_POSITIVE, max[2] - GRID_SIZE],
        max,
    })
}
/// Complete caller-selected ancestor tile inventory. No implicit parent lookup.
/// Native discovery supplies unique grid coordinates; empty/duplicate lists reject.
pub fn global_params(tiles: &[Coordinate]) -> Result<mmap::Params, Error> {
    if tiles.is_empty() || tiles.len() > 4096 {
        return Err(Error::Inventory);
    }
    let mut seen = [0u64; 64];
    let mut corner = Coordinate { row: 0, column: 0 };
    for &t in tiles {
        t.validate()?;
        let bit = 1u64 << t.column;
        if seen[usize::from(t.row)] & bit != 0 {
            return Err(Error::Inventory);
        }
        seen[usize::from(t.row)] |= bit;
        corner.row = corner.row.max(t.row);
        corner.column = corner.column.max(t.column);
    }
    Ok(mmap::Params {
        origin: horizontal(corner)?.min,
        tile_width: GRID_SIZE,
        tile_height: GRID_SIZE,
        max_tiles: tiles.len() as i32,
        max_polys: i32::MIN,
    })
}
/// Height bounds scan liquid vertices before solids, as in original allVerts.
/// Unreferenced vertices participate. No-vertex height bounds are FLT_MIN/MAX.
pub fn bounds(tile: Coordinate, mesh: &Mesh) -> Result<Bounds, Error> {
    let mut b = horizontal(tile)?;
    let mut iter = mesh.liquid_vertices().iter().chain(mesh.solid_vertices());
    if let Some(v) = iter.next() {
        if !v.iter().all(|v| v.is_finite()) {
            return Err(Error::Numeric);
        }
        b.min[1] = v[1];
        b.max[1] = v[1];
        for v in iter {
            if !v.iter().all(|v| v.is_finite()) {
                return Err(Error::Numeric);
            }
            // rcMin/rcMax choose the right operand for equal signed zeros.
            b.min[1] = if b.min[1] < v[1] { b.min[1] } else { v[1] };
            b.max[1] = if b.max[1] > v[1] { b.max[1] } else { v[1] };
        }
    }
    Ok(b)
}

#[derive(Clone, Copy, Debug)]
pub struct Config {
    request: Request,
    bounds: Bounds,
    base_unit: f32,
    cell_height: f32,
    tile_size: u16,
    border: u16,
    radius: u16,
    agent: recast::Agent,
}
impl Config {
    pub fn new(request: Request, mesh: &Mesh) -> Result<Self, Error> {
        request.tile.validate()?;
        let s = request.slopes;
        if !s.maximum.is_finite()
            || !s.non_steep.is_finite()
            || s.non_steep < 0.0
            || s.maximum > 90.0
            || s.non_steep > s.maximum
        {
            return Err(Error::Slopes);
        }
        let (base_unit, tile_size, radius, height) = if request.big_base_unit {
            (0.5333333, 40, 1, 3)
        } else {
            (0.2666666, 80, 2, 6)
        };
        Ok(Self {
            request,
            bounds: bounds(request.tile, mesh)?,
            base_unit,
            cell_height: if request.map_id == 48 {
                base_unit * 2.0
            } else {
                base_unit
            },
            tile_size,
            border: radius + 3,
            radius: if request.map_id == 562 { 0 } else { radius },
            agent: recast::Agent {
                height,
                climb: height,
            },
        })
    }
    pub const fn bounds(self) -> Bounds {
        self.bounds
    }
    pub const fn base_unit(self) -> f32 {
        self.base_unit
    }
    pub const fn cell_height(self) -> f32 {
        self.cell_height
    }
    pub const fn tile_size(self) -> u16 {
        self.tile_size
    }
    pub const fn border(self) -> u16 {
        self.border
    }
    pub const fn radius(self) -> u16 {
        self.radius
    }
    pub const fn agent(self) -> recast::Agent {
        self.agent
    }
    pub fn contour_options(self) -> recast::ContourOptions {
        recast::ContourOptions {
            maximum_error: 1.8,
            maximum_edge_length: self.tile_size + 1,
            ..Default::default()
        }
    }
    pub const fn region_options(self) -> recast::RegionOptions {
        recast::RegionOptions {
            border_size: self.border,
            minimum_area: 3600,
            merge_area: 2500,
        }
    }
    pub fn detail_options(self) -> recast::DetailOptions {
        // Map 48 doubles ch after the original detail error was already set.
        recast::DetailOptions {
            sample_distance: self.base_unit * 16.0,
            maximum_error: self.base_unit,
        }
    }
    /// Original y-major / x-minor order, with padding applied after world bounds.
    pub fn grid(self, x: u16, y: u16) -> Result<recast::Grid, Error> {
        if x >= SUBTILES || y >= SUBTILES {
            return Err(Error::Coordinates);
        }
        let step = f32::from(self.tile_size) * self.base_unit;
        let pad = f32::from(self.border) * self.base_unit;
        let mut min = self.bounds.min;
        let mut max = self.bounds.max;
        min[0] = (self.bounds.min[0] + f32::from(x) * step) - pad;
        min[2] = (self.bounds.min[2] + f32::from(y) * step) - pad;
        max[0] = (self.bounds.min[0] + f32::from(x + 1) * step) + pad;
        max[2] = (self.bounds.min[2] + f32::from(y + 1) * step) + pad;
        Ok(recast::Grid {
            width: self.tile_size + self.border * 2,
            height: self.tile_size + self.border * 2,
            min,
            max,
            cell_size: self.base_unit,
            cell_height: self.cell_height,
        })
    }
    pub fn detour_options(self, params: mmap::Params) -> Result<detour::Options, Error> {
        params.validate().map_err(Error::Params)?;
        let mut tile = [0; 3];
        for (i, a) in [0, 2].into_iter().enumerate() {
            let v =
                (((self.bounds.min[a] + self.bounds.max[a]) / 2.0) - params.origin[a]) / GRID_SIZE;
            if !v.is_finite() || !(-2147483648.0..2147483648.0).contains(&v) {
                return Err(Error::Numeric);
            }
            tile[i] = v as i32;
        }
        Ok(detour::Options {
            tile,
            user_id: 0,
            min: self.bounds.min,
            max: self.bounds.max,
            walkable_height: self.base_unit * f32::from(self.agent.height),
            walkable_radius: self.base_unit * f32::from(self.radius),
            walkable_climb: self.base_unit * f32::from(self.agent.climb),
            build_bv_tree: true,
            flags: detour::FlagPolicy::Forever,
        })
    }
}
