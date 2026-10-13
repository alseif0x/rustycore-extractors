// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: bounded supplied VMO/placement navigation geometry.
// Behavioral facts: pinned TerrainBuilder::loadVMap/loadVMapModel,
// transformVertices/copyIndices and PathGenerator's liquid callback.
//! The supplied `append` API uses caller-selected models and checked instance
//! transforms. `append_loaded` borrows the static store in original slot order;
//! `append_tile` connects its checked loading/ancestor binding/unload lifecycle.
//! Direct-row liquid-type lookup is supplied explicitly; no DB2 acquisition or
//! implicit filesystem/network discovery occurs in this module.

pub mod liquid;

use crate::{Error, Mesh, reserve, terrain::GRID_SIZE};
use forever_vmap::{placement::Placement, query::model::Instance, store, vmo};

#[derive(Clone, Copy)]
pub struct Model<'a> {
    pub geometry: &'a vmo::View<'a>,
    pub placement: Placement<'a>,
}

/// Sorted unique (ID, SoundBank) pairs, already reduced from direct LiquidType
/// rows with last-direct-ID-wins. Native navigation does not replay DB2 copies.
/// Unknown IDs produce flags zero and still emit area-zero WMO liquid triangles.
/// SoundBank >=31 is rejected to avoid the native signed-shift undefined domain.
#[derive(Clone, Copy)]
pub struct LiquidTypes<'a> {
    entries: &'a [(u32, u8)],
}
impl<'a> LiquidTypes<'a> {
    pub fn new(entries: &'a [(u32, u8)]) -> Result<Self, Error> {
        if entries.len() > 65_536 {
            return Err(Error::Limit);
        }
        if entries.iter().any(|e| e.1 >= 31) || entries.windows(2).any(|w| w[0].0 >= w[1].0) {
            return Err(Error::LiquidTypes);
        }
        Ok(Self { entries })
    }
    pub fn flags(self, id: u32) -> u8 {
        self.entries
            .binary_search_by_key(&id, |e| e.0)
            .map_or(0, |i| (1u32 << self.entries[i].1) as u8)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub mesh: crate::Limits,
    pub models: usize,
    pub groups: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            mesh: crate::Limits::default(),
            models: 4096,
            groups: 65_536,
        }
    }
}

/// Consume an existing terrain mesh and append all selected model instances.
/// Any transform/resource failure discards the entire result, including the
/// consumed prefix. WMO liquids are always projected here: the pinned original
/// does not consult its skipLiquid option in loadVMapModel.
pub fn append(
    prefix: Mesh,
    models: &[Model<'_>],
    types: LiquidTypes<'_>,
    limits: Limits,
) -> Result<Mesh, Error> {
    append_models(
        prefix,
        models.iter().map(|m| SelectedModel {
            geometry: Geometry::Borrowed(m.geometry),
            placement: m.placement,
            transform: None,
        }),
        types,
        limits,
    )
}

#[derive(Clone, Copy)]
enum Geometry<'a> {
    Borrowed(&'a vmo::View<'a>),
    Owned(&'a vmo::Owned),
}
impl<'a> Geometry<'a> {
    fn group_count(self) -> usize {
        match self {
            Self::Borrowed(v) => v.groups().len(),
            Self::Owned(v) => v.group_count(),
        }
    }
    fn group(self, i: usize) -> vmo::Group<'a> {
        match self {
            Self::Borrowed(v) => v.groups()[i],
            Self::Owned(v) => v.group(i).expect("owned group index"),
        }
    }
    fn flags(self) -> u32 {
        match self {
            Self::Borrowed(v) => v.flags(),
            Self::Owned(v) => v.flags(),
        }
    }
    fn groups(self) -> impl Iterator<Item = vmo::Group<'a>> {
        (0..self.group_count()).map(move |i| self.group(i))
    }
}
#[derive(Clone, Copy)]
struct SelectedModel<'a> {
    geometry: Geometry<'a>,
    placement: Placement<'a>,
    transform: Option<&'a Instance>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoadedError {
    Store(store::Error),
    Geometry(Error),
    Options,
}
impl LoadedError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Store(e) => e.code(),
            Self::Geometry(e) => e.code(),
            Self::Options => "navigation_loaded_model_options",
        }
    }
}
impl std::fmt::Display for LoadedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}
impl std::error::Error for LoadedError {}

/// Append every currently active non-null model in original tree slot order.
/// Uses checked owned group views and cached instance transforms, with no model
/// reparse, selection Vec or I/O. Store limits bound slot scans; mesh limits
/// bound cumulative model/group/geometry visits. The store must have the pinned
/// navigation path-only policy enabled. Other loaded tiles are intentionally
/// visible, matching original getModelsOnMap rather than filtering by tile ID.
pub fn append_loaded<S: store::Source>(
    prefix: Mesh,
    store: &store::StaticStore<S>,
    map: u32,
    types: LiquidTypes<'_>,
    limits: Limits,
) -> Result<Mesh, LoadedError> {
    if !store.options().load_path_only_models {
        return Err(LoadedError::Options);
    }
    let models = store.loaded_models(map).map_err(LoadedError::Store)?;
    append_models(
        prefix,
        models.map(|m| SelectedModel {
            geometry: Geometry::Owned(m.geometry()),
            placement: m.placement(),
            transform: Some(m.transform()),
        }),
        types,
        limits,
    )
    .map_err(LoadedError::Geometry)
}

/// Original TerrainBuilder::loadVMap lifecycle: load a tile, append all active
/// map instances, unload that tile even if projection fails. Unload removes all
/// references to this tile, including preceding loads (the original contract).
/// Other tiles remain loaded. Store failures remain explicit, including missing
/// files; no read/corruption error is converted to an empty successful mesh.
/// A successful load can initialize a map even when later projection rejects.
pub fn append_tile<S: store::Source>(
    prefix: Mesh,
    store: &mut store::StaticStore<S>,
    map: u32,
    tile: crate::tilebuilder::Coordinate,
    types: LiquidTypes<'_>,
    limits: Limits,
) -> Result<Mesh, LoadedError> {
    if !store.options().load_path_only_models {
        return Err(LoadedError::Options);
    }
    if tile.row >= 64 || tile.column >= 64 {
        return Err(LoadedError::Geometry(Error::Coordinates));
    }
    store
        .load_tile(map, u16::from(tile.row), tile.column)
        .map_err(LoadedError::Store)?;
    let result = append_loaded(prefix, store, map, types, limits);
    store
        .unload_tile(map, u16::from(tile.row), tile.column)
        .map_err(LoadedError::Store)?;
    result
}

fn append_models<'a>(
    prefix: Mesh,
    models: impl Iterator<Item = SelectedModel<'a>> + Clone,
    types: LiquidTypes<'_>,
    limits: Limits,
) -> Result<Mesh, Error> {
    limits.mesh.validate()?;
    if limits.models > 4096 || limits.groups > 65_536 {
        return Err(Error::Limit);
    }
    let mut counts = [
        prefix.solid_vertices.len(),
        prefix.solid_triangles.len(),
        prefix.liquid_vertices.len(),
        prefix.liquid_triangles.len(),
        prefix.liquid_areas.len(),
    ];
    let mut groups = 0usize;
    check_counts(counts, limits.mesh)?;
    let mut model_count = 0usize;
    for model in models.clone() {
        model_count += 1;
        if model_count > limits.models {
            return Err(Error::Limit);
        }
        groups = groups
            .checked_add(model.geometry.group_count())
            .ok_or(Error::Limit)?;
        if groups > limits.groups {
            return Err(Error::Limit);
        }
        for group in model.geometry.groups() {
            counts[0] = counts[0]
                .checked_add(group.vertex_count())
                .ok_or(Error::Limit)?;
            counts[1] = counts[1]
                .checked_add(group.triangle_count())
                .ok_or(Error::Limit)?;
            check_counts(counts, limits.mesh)?;
            if let Some(liquid) = group.liquid().filter(|l| !l.tiles().is_empty()) {
                counts[2] = counts[2]
                    .checked_add(liquid.height_count())
                    .ok_or(Error::Limit)?;
                check_counts(counts, limits.mesh)?;
                let n = liquid.tiles().iter().filter(|&&f| f & 15 != 15).count() * 2;
                counts[3] = counts[3].checked_add(n).ok_or(Error::Limit)?;
                counts[4] = counts[4].checked_add(n).ok_or(Error::Limit)?;
                check_counts(counts, limits.mesh)?;
            }
        }
    }
    let bytes = counts[..4].iter().sum::<usize>() * 12 + counts[4];
    if prefix
        .capacity_bytes()
        .checked_add(bytes)
        .ok_or(Error::Limit)?
        > limits.mesh.heap_bytes
    {
        return Err(Error::Limit);
    }
    let mut mesh = Mesh::default();
    reserve(&mut mesh.solid_vertices, counts[0])?;
    reserve(&mut mesh.solid_triangles, counts[1])?;
    reserve(&mut mesh.liquid_vertices, counts[2])?;
    reserve(&mut mesh.liquid_triangles, counts[3])?;
    reserve(&mut mesh.liquid_areas, counts[4])?;
    if prefix.capacity_bytes() + mesh.capacity_bytes() > limits.mesh.heap_bytes {
        return Err(Error::Limit);
    }
    mesh.solid_vertices
        .extend_from_slice(&prefix.solid_vertices);
    mesh.solid_triangles
        .extend_from_slice(&prefix.solid_triangles);
    mesh.liquid_vertices
        .extend_from_slice(&prefix.liquid_vertices);
    mesh.liquid_triangles
        .extend_from_slice(&prefix.liquid_triangles);
    mesh.liquid_areas.extend_from_slice(&prefix.liquid_areas);
    drop(prefix);
    for model in models {
        let created;
        let instance = match model.transform {
            Some(v) => v,
            None => {
                created = Instance::new(&model.placement).map_err(Error::Transform)?;
                &created
            }
        };
        let matrix = instance.inverse_rotation();
        let mut position = instance.position();
        position[0] -= 32.0 * GRID_SIZE;
        position[1] -= 32.0 * GRID_SIZE;
        let scale = instance.scale();
        for group in model.geometry.groups() {
            let base = mesh.solid_vertices.len() as u32;
            for i in 0..group.vertex_count() {
                let point = group
                    .vertex_bits(i)
                    .expect("verified VMO vertex")
                    .map(f32::from_bits);
                mesh.solid_vertices
                    .push(transform(point, matrix, scale, position)?);
            }
            for i in 0..group.triangle_count() {
                let mut tri = group.triangle(i).expect("verified VMO triangle");
                if model.geometry.flags() & 1 != 0 {
                    tri.reverse();
                }
                mesh.solid_triangles.push(tri.map(|i| i + base));
            }
            if let Some(liquid) = group.liquid().filter(|l| !l.tiles().is_empty()) {
                let [tx, ty] = liquid.dimensions();
                let [vx, vy] = [tx + 1, ty + 1];
                let corner = liquid.corner_bits().map(f32::from_bits);
                let flags = types.flags(liquid.type_id());
                let area = if flags & 3 != 0 {
                    9
                } else if flags & 12 != 0 {
                    8
                } else {
                    0
                };
                let base = mesh.liquid_vertices.len() as u32;
                for x in 0..vx {
                    for y in 0..vy {
                        let h = f32::from_bits(
                            liquid
                                .height_bits((y * vx + x) as usize)
                                .expect("verified VMO liquid height"),
                        );
                        let point = [
                            corner[0] + x as f32 * (GRID_SIZE / 128.0),
                            corner[1] + y as f32 * (GRID_SIZE / 128.0),
                            h,
                        ];
                        mesh.liquid_vertices
                            .push(transform(point, matrix, scale, position)?);
                    }
                }
                for x in 0..tx {
                    for y in 0..ty {
                        if liquid.tiles()[(x + y * tx) as usize] & 15 == 15 {
                            continue;
                        }
                        let i = base + x * vy + y;
                        mesh.liquid_triangles
                            .extend([[i + 1, i, i + vy + 1], [i + vy + 1, i, i + vy]]);
                        mesh.liquid_areas.extend([area; 2]);
                    }
                }
            }
        }
    }
    Ok(mesh)
}
fn check_counts(counts: [usize; 5], limits: crate::Limits) -> Result<(), Error> {
    if counts[0].checked_add(counts[2]).ok_or(Error::Limit)? > limits.vertices
        || counts[1].checked_add(counts[3]).ok_or(Error::Limit)? > limits.triangles
    {
        return Err(Error::Limit);
    }
    Ok(())
}
fn transform(
    point: [f32; 3],
    matrix: [[f32; 3]; 3],
    scale: f32,
    position: [f32; 3],
) -> Result<[f32; 3], Error> {
    let p: [f32; 3] = std::array::from_fn(|i| {
        let value =
            ((point[0] * matrix[0][i]) + (point[1] * matrix[1][i])) + (point[2] * matrix[2][i]);
        value * scale + position[i]
    });
    if !p.iter().all(|v| v.is_finite()) {
        return Err(Error::NonFinite);
    }
    Ok([-p[1], p[2], -p[0]])
}

#[cfg(test)]
mod tests;
