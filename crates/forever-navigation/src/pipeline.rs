// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: bounded single-tile source-to-final navigation composition.
// Modified 2026-10-19: add selected available-plan pipeline entry.
//! Executes terrain ancestor lookup, checked VMap selection, compaction,
//! connection selection, all 625 subtiles and the original fresh-navmesh save.
//! Returns an owned complete MMap or an explicit empty outcome. Publication,
//! acquisition of DB2/inventory and multi-tile runtime loading remain external.
use crate::{Mesh, detour, inventory, mmap, models, offmesh, terrain, tilebuilder};
use forever_vmap::store;

#[cfg(test)]
mod tests;

/// Exclusive tile ownership: the store must have no loaded tiles or active
/// instances. Empty initialized trees are allowed and the target is released.
pub struct Sources<'a, T, V: store::Source> {
    pub terrain: &'a mut T,
    pub collision: &'a mut store::StaticStore<V>,
}
#[derive(Clone, Copy, Debug)]
pub struct Request {
    pub tile: tilebuilder::Request,
    pub skip_liquids: bool,
}
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    /// Owned pipeline capacities, including borrowed mesh during TileBuilder.
    /// Terrain file buffers are included. Provider inventories/scratch,
    /// store residency and caller inputs have
    /// separate policies. This is not a bound on allocator overhead or RSS.
    pub heap_bytes: usize,
    pub terrain: terrain::load::Limits,
    pub models: models::Limits,
    pub compact: crate::Limits,
    pub connections: offmesh::Limits,
    pub builder: tilebuilder::Limits,
    pub save: detour::SaveLimits,
    pub output: mmap::Limits,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            heap_bytes: 512 * 1024 * 1024,
            terrain: Default::default(),
            models: Default::default(),
            compact: Default::default(),
            connections: Default::default(),
            builder: Default::default(),
            save: Default::default(),
            output: Default::default(),
        }
    }
}
impl Limits {
    pub(crate) fn validate(self) -> Result<(), Error> {
        let d = Self::default();
        self.terrain.validate().map_err(Error::Terrain)?;
        self.models.mesh.validate().map_err(Error::Geometry)?;
        self.compact.validate().map_err(Error::Geometry)?;
        self.output.validate().map_err(Error::Output)?;
        if self.heap_bytes > d.heap_bytes
            || self.models.models > d.models.models
            || self.models.groups > d.models.groups
            || self.connections.input_records > d.connections.input_records
            || self.connections.output_records > d.connections.output_records
            || self.connections.heap_bytes > d.connections.heap_bytes
            || self.builder.heap_bytes > d.builder.heap_bytes
            || self.builder.work > d.builder.work
            || self.save.work > d.save.work
        {
            return Err(Error::Limit);
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Selection,
    Topology,
    StoreBusy,
    Options,
    MissingTerrain,
    Limit,
    Terrain(terrain::load::Error),
    Store(store::Error),
    Models(models::LoadedError),
    Geometry(crate::Error),
    Builder(tilebuilder::Error),
    Save(detour::SaveError),
    Output(mmap::Error),
}
impl Error {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Selection => "navigation_pipeline_selection",
            Self::Topology => "navigation_pipeline_parent_mismatch",
            Self::StoreBusy => "navigation_pipeline_store_busy",
            Self::Options => "navigation_pipeline_store_options",
            Self::MissingTerrain => "navigation_pipeline_missing_terrain",
            Self::Limit => "navigation_pipeline_limit",
            Self::Terrain(e) => e.code(),
            Self::Store(e) => e.code(),
            Self::Models(e) => e.code(),
            Self::Geometry(e) => e.code(),
            Self::Builder(e) => e.code(),
            Self::Save(e) => e.code(),
            Self::Output(e) => e.code(),
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}
impl std::error::Error for Error {}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EmptyReason {
    NoVertices,
    NoReferencedVertices,
    NoPolygons,
}
#[derive(Clone, Copy, Debug, Default)]
pub struct Statistics {
    pub terrain_origins: [Option<u32>; 5],
    pub terrain: terrain::load::Statistics,
    pub collision: Option<store::LoadReport>,
    /// Solid vertices/triangles, liquid vertices/triangles after compaction.
    pub geometry: [usize; 4],
    pub connections: usize,
    pub builder: tilebuilder::Statistics,
    pub save: detour::SaveStatistics,
    pub output_bytes: usize,
}
pub enum Outcome {
    Empty {
        reason: EmptyReason,
        statistics: Statistics,
    },
    Tile {
        bytes: Vec<u8>,
        statistics: Statistics,
    },
}
fn no_vertices(mesh: &Mesh) -> bool {
    mesh.solid_vertices().is_empty() && mesh.liquid_vertices().is_empty()
}
fn verify<T: terrain::load::Source, V: store::Source>(
    sources: &mut Sources<'_, T, V>,
) -> Result<(), Error> {
    sources
        .terrain
        .verify_unchanged()
        .map_err(|e| Error::Terrain(terrain::load::Error::Source(e)))?;
    sources.collision.verify_source().map_err(Error::Store)
}
/// Inventory component promises are strict. A terrain-only tile may genuinely
/// lack collision; present partial collision packages still reject. Native WMO
/// liquids remain projected even when skip_liquids disables terrain liquids.
/// The final usesLiquids header is !skip_liquids, as in the original producer.
/// Malformed selected connections are inspected only after nonempty geometry.
pub fn run<T: terrain::load::Source, V: store::Source>(
    plan: &inventory::Plan,
    sources: &mut Sources<'_, T, V>,
    types: models::LiquidTypes<'_>,
    connections: &[offmesh::Source],
    request: Request,
    limits: Limits,
) -> Result<Outcome, Error> {
    run_inner(plan, None, sources, types, connections, request, limits)
}

/// Runs one output tile from a plan built by the Linux 70450 selected-map
/// available-section owner. Only its single output map is eligible; ancestors
/// remain available to terrain and collision readers as context.
#[cfg(target_os = "linux")]
pub fn run_available_selected<T: terrain::load::Source, V: store::Source>(
    plan: &inventory::AvailableSelectedPlan,
    sources: &mut Sources<'_, T, V>,
    types: models::LiquidTypes<'_>,
    connections: &[offmesh::Source],
    request: Request,
    limits: Limits,
) -> Result<Outcome, Error> {
    limits.validate()?;
    let selected_map_id = selected_output_map(plan.maps())?;
    run_inner(
        plan.inventory_plan(),
        Some(selected_map_id),
        sources,
        types,
        connections,
        request,
        limits,
    )
}

#[cfg(target_os = "linux")]
fn selected_output_map(maps: &[inventory::Map]) -> Result<u32, Error> {
    let mut outputs = maps.iter().filter(|map| map.is_output());
    let selected = outputs.next().ok_or(Error::Selection)?.map_id();
    if outputs.next().is_some() {
        return Err(Error::Selection);
    }
    Ok(selected)
}

fn run_inner<T: terrain::load::Source, V: store::Source>(
    plan: &inventory::Plan,
    selected_map_id: Option<u32>,
    sources: &mut Sources<'_, T, V>,
    types: models::LiquidTypes<'_>,
    connections: &[offmesh::Source],
    request: Request,
    mut limits: Limits,
) -> Result<Outcome, Error> {
    limits.validate()?;
    let r = request.tile;
    let map = plan
        .map(r.map_id)
        .filter(|m| {
            m.is_output()
                && selected_map_id.is_none_or(|selected| selected == m.map_id())
                && m.contains(r.tile)
        })
        .ok_or(Error::Selection)?;
    let params = map.params().ok_or(Error::Selection)?;
    tilebuilder::Config::new(r, &Mesh::default()).map_err(Error::Builder)?;
    if !sources.collision.options().load_path_only_models {
        return Err(Error::Options);
    }
    for row in plan.parent_rows() {
        if sources
            .collision
            .effective_parent(row.map_id)
            .map_err(Error::Store)?
            != row.parent
        {
            return Err(Error::Topology);
        }
    }
    let store_stats = sources.collision.stats();
    if store_stats.tiles != 0 || store_stats.active_instances != 0 {
        return Err(Error::StoreBusy);
    }
    verify(sources)?;
    // The loader can retain source buffers while projecting geometry. Reserve
    // their separate ceiling before passing the remaining capacity to meshes.
    limits.terrain.source_heap_bytes = limits.terrain.source_heap_bytes.min(limits.heap_bytes);
    limits.terrain.mesh.heap_bytes = limits
        .terrain
        .mesh
        .heap_bytes
        .min(limits.heap_bytes - limits.terrain.source_heap_bytes);
    limits.models.mesh.heap_bytes = limits.models.mesh.heap_bytes.min(limits.heap_bytes);
    limits.compact.heap_bytes = limits.compact.heap_bytes.min(limits.heap_bytes);
    let loaded = terrain::load::neighborhood(
        sources.terrain,
        plan,
        r.map_id,
        r.tile,
        request.skip_liquids,
        limits.terrain,
    )
    .map_err(Error::Terrain)?;
    let mut statistics = Statistics {
        terrain_origins: *loaded.origins(),
        terrain: loaded.statistics(),
        ..Default::default()
    };
    if map.terrain_contains(r.tile) && loaded.origins()[0].is_none() {
        return Err(Error::MissingTerrain);
    }
    let prefix = loaded.into_mesh();
    let collision = if map.collision_contains(r.tile) {
        Some(
            sources
                .collision
                .load_tile(r.map_id, u16::from(r.tile.row), r.tile.column)
                .map_err(Error::Store)?,
        )
    } else {
        sources
            .collision
            .load_optional_tile(r.map_id, u16::from(r.tile.row), r.tile.column)
            .map_err(Error::Store)?
    };
    statistics.collision = collision;
    let projected = if collision.is_some() {
        models::append_loaded(prefix, sources.collision, r.map_id, types, limits.models)
            .map_err(Error::Models)
    } else {
        Ok(prefix)
    };
    // Exclusive context allows releasing the target, including any initialized
    // tree from an optional miss. Projection errors cannot leave live instances.
    sources
        .collision
        .unload_map(r.map_id)
        .map_err(Error::Store)?;
    let mesh = projected?;
    verify(sources)?;
    if no_vertices(&mesh) {
        return Ok(Outcome::Empty {
            reason: EmptyReason::NoVertices,
            statistics,
        });
    }
    let mesh = mesh.compact(limits.compact).map_err(Error::Geometry)?;
    statistics.geometry = [
        mesh.solid_vertices().len(),
        mesh.solid_triangles().len(),
        mesh.liquid_vertices().len(),
        mesh.liquid_triangles().len(),
    ];
    if no_vertices(&mesh) {
        verify(sources)?;
        return Ok(Outcome::Empty {
            reason: EmptyReason::NoReferencedVertices,
            statistics,
        });
    }
    limits.connections.heap_bytes = limits.connections.heap_bytes.min(
        limits
            .heap_bytes
            .checked_sub(mesh.capacity_bytes())
            .ok_or(Error::Limit)?,
    );
    let selected = offmesh::select(
        offmesh::Target {
            map_id: r.map_id,
            row: r.tile.row,
            column: r.tile.column,
        },
        connections,
        limits.connections,
    )
    .map_err(Error::Geometry)?;
    statistics.connections = selected.values().len();
    let borrowed = mesh
        .capacity_bytes()
        .checked_add(selected.capacity_bytes())
        .ok_or(Error::Limit)?;
    limits.builder.heap_bytes = limits.builder.heap_bytes.min(
        limits
            .heap_bytes
            .checked_sub(borrowed)
            .ok_or(Error::Limit)?,
    );
    let built = tilebuilder::build(&mesh, selected.values(), r, params, limits.builder)
        .map_err(Error::Builder)?;
    drop(mesh);
    drop(selected);
    match built {
        tilebuilder::Outcome::Empty(s) => {
            statistics.builder = s;
            verify(sources)?;
            Ok(Outcome::Empty {
                reason: EmptyReason::NoPolygons,
                statistics,
            })
        }
        tilebuilder::Outcome::Tile(data) => {
            statistics.builder = data.statistics();
            let saved = data.into_saved(params, limits.save).map_err(Error::Save)?;
            statistics.save = saved.statistics();
            limits.output.heap_bytes = limits.output.heap_bytes.min(
                limits
                    .heap_bytes
                    .checked_sub(saved.capacity_bytes())
                    .ok_or(Error::Limit)?,
            );
            let bytes = mmap::encode_saved_tile(&saved, !request.skip_liquids, limits.output)
                .map_err(Error::Output)?;
            statistics.output_bytes = bytes.len();
            verify(sources)?;
            Ok(Outcome::Tile { bytes, statistics })
        }
    }
}
