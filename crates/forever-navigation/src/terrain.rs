// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: independent indexed geometry composition from MAPS v10.
// Behavioral reference: TrinityCore pin02245dcd245e7433e524577656177723d3e4992e
// TerrainBuilder::{loadMap,getHeightCoord,getHeightTriangle,isHole,cleanVertices}.
// Original C++ and its GPL-2.0-or-later notices remain in the external QA closure.
//! Solid/liquid triangulation, not a navmesh. Flat NoHeight terrain deliberately
//! contributes no solid geometry, matching the original navigation consumer.
//! Coordinates use filename order (row, column), unlike ADT (x, y) selections.

use crate::{Error, Limits, Mesh, reserve};
use forever_terrain::{
    height::{Encoding, V8_COUNT, V9_COUNT},
    liquid_section,
    maps::{HeightView, LiquidView, MapView},
};

pub mod load;

pub const GRID_SIZE: f32 = 533.3333;
const STEP: f32 = GRID_SIZE / 128.0;
const INVALID: f32 = -2000.0;
const MAX_LIQUID: f32 = 5000.0;
pub const WATER_AREA: u8 = 9;
pub const MAGMA_SLIME_AREA: u8 = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Portion {
    Entire,
    Top,
    Right,
    Left,
    Bottom,
}
impl Portion {
    fn squares(self) -> impl Iterator<Item = usize> {
        let (start, end, step) = match self {
            Self::Entire => (0, 16384, 1),
            Self::Top => (0, 128, 1),
            Self::Right => (127, 16384, 128),
            Self::Left => (0, 16257, 128),
            Self::Bottom => (16256, 16384, 1),
        };
        (start..end).step_by(step)
    }
}
#[derive(Clone, Copy)]
pub struct Region<'a> {
    pub map: MapView<'a>,
    pub row: u8,
    pub column: u8,
    pub portion: Portion,
}

/// Explicitly supplied neighbors; absence means the caller found no input.
/// This type does not perform ancestor lookup or interpret read failures as absence.
#[derive(Clone, Copy, Default)]
pub struct Neighbors<'a> {
    pub column_next: Option<MapView<'a>>,
    pub column_previous: Option<MapView<'a>>,
    pub row_next: Option<MapView<'a>>,
    pub row_previous: Option<MapView<'a>>,
}

/// Project one to five explicit regions in order. Every supplied file has
/// already passed the MAPS verifier. Whole result is discarded on any error.
pub fn project(regions: &[Region<'_>], skip_liquids: bool, limits: Limits) -> Result<Mesh, Error> {
    execute(regions, skip_liquids, limits, false)
}

/// Center first, then adjacent strips in original order. If the center has no
/// usable triangles, none of its neighbors are processed. No I/O or fallback.
pub fn project_neighborhood(
    center: Region<'_>,
    neighbors: Neighbors<'_>,
    skip_liquids: bool,
    limits: Limits,
) -> Result<Mesh, Error> {
    if center.portion != Portion::Entire {
        return Err(Error::Coordinates);
    }
    let options = [
        (
            neighbors.column_next,
            Some(center.row),
            center.column.checked_add(1),
            Portion::Left,
        ),
        (
            neighbors.column_previous,
            Some(center.row),
            center.column.checked_sub(1),
            Portion::Right,
        ),
        (
            neighbors.row_next,
            center.row.checked_add(1),
            Some(center.column),
            Portion::Top,
        ),
        (
            neighbors.row_previous,
            center.row.checked_sub(1),
            Some(center.column),
            Portion::Bottom,
        ),
    ];
    let mut regions = [center; 5];
    let mut count = 1;
    for (map, row, column, portion) in options {
        if let Some(map) = map {
            regions[count] = Region {
                map,
                row: row.ok_or(Error::Coordinates)?,
                column: column.ok_or(Error::Coordinates)?,
                portion,
            };
            count += 1;
        }
    }
    execute(&regions[..count], skip_liquids, limits, true)
}

fn execute(
    regions: &[Region<'_>],
    skip: bool,
    limits: Limits,
    stop_empty_center: bool,
) -> Result<Mesh, Error> {
    limits.validate()?;
    if regions.is_empty() || regions.len() > 5 {
        return Err(Error::Limit);
    }
    let mut counts = [0usize; 5]; // solid vertices/triangles, liquid vertices/triangles/areas
    for r in regions {
        if r.row >= 64 || r.column >= 64 {
            return Err(Error::Coordinates);
        }
        if r.map.build() != regions[0].map.build() {
            return Err(Error::BuildMismatch);
        }
        let squares = r.portion.squares().count();
        if r.map.height().encoding() != Encoding::Flat {
            counts[0] += V9_COUNT + V8_COUNT;
            counts[1] += squares * 4;
        }
        if r.map.liquid().is_some() && !skip {
            counts[2] += V9_COUNT;
            counts[3] += squares * 2;
            counts[4] += squares * 2;
        }
    }
    if counts[0] + counts[2] > limits.vertices || counts[1] + counts[3] > limits.triangles {
        return Err(Error::Limit);
    }
    let scratch_count = if counts[2] == 0 { 0 } else { V9_COUNT };
    let bytes = counts[..4].iter().sum::<usize>() * 12 + counts[4] + scratch_count * 4;
    if bytes > limits.heap_bytes {
        return Err(Error::Limit);
    }
    let mut mesh = Mesh::default();
    reserve(&mut mesh.solid_vertices, counts[0])?;
    reserve(&mut mesh.solid_triangles, counts[1])?;
    reserve(&mut mesh.liquid_vertices, counts[2])?;
    reserve(&mut mesh.liquid_triangles, counts[3])?;
    reserve(&mut mesh.liquid_areas, counts[4])?;
    let mut original_heights = Vec::new();
    reserve(&mut original_heights, scratch_count)?;
    if mesh.capacity_bytes() + original_heights.capacity() * 4 > limits.heap_bytes {
        return Err(Error::Limit);
    }
    for (i, region) in regions.iter().enumerate() {
        append(&mut mesh, *region, skip, &mut original_heights)?;
        if i == 0 && stop_empty_center && !mesh.has_triangles() {
            break;
        }
    }
    Ok(mesh)
}

fn append(
    mesh: &mut Mesh,
    region: Region<'_>,
    skip: bool,
    original: &mut Vec<f32>,
) -> Result<(), Error> {
    let height = region.map.height();
    let terrain = height.encoding() != Encoding::Flat;
    let liquid = region.map.liquid().filter(|_| !skip);
    let solid_base = mesh.solid_vertices.len() as u32;
    let liquid_base = mesh.liquid_vertices.len() as u32;
    let row_offset = (f32::from(region.row) - 32.0) * GRID_SIZE;
    let col_offset = (f32::from(region.column) - 32.0) * GRID_SIZE;
    if terrain {
        for (samples, width) in [(height.v9_samples(), 129), (height.v8_samples(), 128)] {
            for i in 0..width * width {
                let h = sample(height, samples, i);
                let half = if width == 128 { STEP / 2.0 } else { 0.0 };
                // Do not introduce an extra addition for V9 (signed-zero bits).
                let mut x = col_offset + (i % width) as f32 * STEP;
                let mut z = row_offset + (i / width) as f32 * STEP;
                if width == 128 {
                    x += half;
                    z += half;
                }
                push_vertex(&mut mesh.solid_vertices, [-x, h, -z])?;
            }
        }
    }
    original.clear();
    if let Some(liq) = liquid {
        for i in 0..V9_COUNT {
            let (row, col) = (i / 129, i % 129);
            let h = liquid_height(liq, row, col);
            push_vertex(
                &mut mesh.liquid_vertices,
                [
                    -(col_offset + col as f32 * STEP),
                    h,
                    -(row_offset + row as f32 * STEP),
                ],
            )?;
            original.push(h);
        }
    }
    for square in region.portion.squares() {
        let corner = (square + square / 128) as u32;
        let center = (V9_COUNT + square) as u32;
        let solids = [
            [center, corner + 1, corner],
            [center, corner + 130, corner + 1],
            [corner + 129, center, corner],
            [corner + 129, corner + 130, center],
        ];
        let liquids = [
            [corner + 130, corner + 1, corner],
            [corner + 129, corner + 130, corner],
        ];
        for half in 0..2 {
            let flags = liquid.map_or(0, |l| liquid_flags(l, square));
            let area = if flags & 3 != 0 {
                WATER_AREA
            } else {
                MAGMA_SLIME_AREA
            };
            let dark = flags & 16 != 0;
            let mut use_liquid = liquid.is_some() && !dark && flags & 15 != 0;
            let mut use_terrain = terrain && !dark;
            let lt = liquids[half].map(|i| i + liquid_base);
            let st = [
                solids[half * 2].map(|i| i + solid_base),
                solids[half * 2 + 1].map(|i| i + solid_base),
            ];
            if use_liquid {
                let mut sum = 0.0;
                let mut count = 0;
                for i in liquids[half] {
                    let h = original[i as usize];
                    if h != INVALID && h < MAX_LIQUID {
                        sum += h;
                        count += 1;
                    }
                }
                if count == 0 {
                    use_liquid = false;
                } else if count < 3 {
                    let average = sum / count as f32;
                    if !average.is_finite() {
                        return Err(Error::NonFinite);
                    }
                    for i in lt {
                        let h = &mut mesh.liquid_vertices[i as usize][1];
                        // Exactly 5000 participates in neither validity nor repair.
                        if *h == INVALID || *h > MAX_LIQUID {
                            *h = average;
                        }
                    }
                }
            }
            if use_terrain && hole(region.map, square) {
                use_terrain = false;
            }
            if use_terrain && use_liquid {
                let (mut lmin, mut lmax, mut tmin, mut tmax) =
                    (MAX_LIQUID, INVALID, MAX_LIQUID, INVALID);
                for i in lt {
                    let h = mesh.liquid_vertices[i as usize][1];
                    if lmin > h {
                        lmin = h;
                    }
                    if lmax < h {
                        lmax = h;
                    }
                }
                for i in st.into_iter().flatten() {
                    let h = mesh.solid_vertices[i as usize][1];
                    if tmin > h {
                        tmin = h;
                    }
                    if tmax < h {
                        tmax = h;
                    }
                }
                if lmin > tmax {
                    use_terrain = false;
                }
                if tmin > lmax {
                    use_liquid = false;
                }
            }
            if use_liquid {
                mesh.liquid_triangles.push(lt);
                mesh.liquid_areas.push(area);
            }
            if use_terrain {
                mesh.solid_triangles.extend(st);
            }
        }
    }
    Ok(())
}
fn push_vertex(vertices: &mut Vec<[f32; 3]>, value: [f32; 3]) -> Result<(), Error> {
    if !value.iter().all(|v| v.is_finite()) {
        return Err(Error::NonFinite);
    }
    vertices.push(value);
    Ok(())
}
fn sample(view: HeightView<'_>, bytes: &[u8], i: usize) -> f32 {
    let min = f32::from_bits(view.min_height_bits());
    let max = f32::from_bits(view.max_height_bits());
    match view.encoding() {
        Encoding::Uint8 => f32::from(bytes[i]) * ((max - min) / 255.0) + min,
        Encoding::Uint16 => {
            f32::from(u16::from_le_bytes(
                bytes[i * 2..i * 2 + 2].try_into().unwrap(),
            )) * ((max - min) / 65535.0)
                + min
        }
        Encoding::Float32 => f32::from_le_bytes(bytes[i * 4..i * 4 + 4].try_into().unwrap()),
        Encoding::Flat => unreachable!("flat has no samples"),
    }
}
fn liquid_height(liquid: LiquidView<'_>, row: usize, col: usize) -> f32 {
    if liquid.flags() & liquid_section::NO_HEIGHT != 0 {
        return f32::from_bits(liquid.level_bits());
    }
    let (x, y, w, h) = (
        liquid.offset_x() as usize,
        liquid.offset_y() as usize,
        liquid.width() as usize,
        liquid.height() as usize,
    );
    if row < y || row >= y + h || col < x || col >= x + w {
        return INVALID;
    }
    let i = ((row - y) * w + col - x) * 4;
    f32::from_le_bytes(liquid.height_bytes()[i..i + 4].try_into().unwrap())
}
fn liquid_flags(liquid: LiquidView<'_>, square: usize) -> u8 {
    liquid.flag_bytes().map_or(liquid.liquid_flags(), |f| {
        f[(square / 128 / 8) * 16 + square % 128 / 8]
    })
}
fn hole(map: MapView<'_>, square: usize) -> bool {
    let (row, col) = (square / 128, square % 128);
    map.holes()
        .is_some_and(|h| h[((row / 8) * 16 + col / 8) * 8 + row % 8] & (1 << (col % 8)) != 0)
}

#[cfg(test)]
pub(crate) mod tests;
