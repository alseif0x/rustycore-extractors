// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: bounded static-map storage from pinned operation facts.
//! Owns complete immutable final artifacts, validated layouts and cached
//! instance transforms. Reference 02245dcd245e7433e524577656177723d3e4992e,
//! MapTree.cpp200-400 and VMapManager.cpp109-164/262-313.
//!
//! Repeated loads append tile references; one unload removes ALL references
//! for that tile. Missing models, inconsistent bindings and any failed load
//! reject atomically, deliberately stricter than native partial/tolerant loads.
//! Bindings may differ only in parent bit 2; the first binding retains its bits.
//! Parent payload lookup never changes the child's index or retries a present
//! corrupt payload. Intrinsic tree/index/first-binding/clip checks do not invent
//! the assembler-only ordered-dir-bin proof or authenticate caller topology.
//!
//! The source supplies one immutable package. InputChanged poisons the store;
//! other failed loads preserve its previous logical state. Queries only inspect
//! owned snapshots, with no source I/O or per-query parsing. Explicit source
//! verification is separate. Source implementation scratch/identity tracking,
//! bounded stack and allocator metadata are outside the capacity ledger.
#[cfg(target_os = "linux")]
pub mod fs;
mod load;
mod loaded;
mod manager;
mod memory;
mod source;
#[cfg(test)]
mod tests;
#[cfg(windows)]
pub mod windows;

use crate::{assembly, placement, query, vmo};
pub use loaded::{LoadedModel, LoadedModels};
pub use manager::{INVALID_HEIGHT, ObjectHit, WORLD_MIDPOINT};
use memory::{Memory, Work, add};
pub use source::{Artifact, Presence, ReadBuffer, Source, SourceError};

pub const MAX_FILE_BYTES: usize = 128 * 1024 * 1024;
pub const MAX_MEMORY_BYTES: usize = 1024 * 1024 * 1024;
pub const MAX_MAPS: usize = 4096;
pub const MAX_TILES: usize = 65_536;
pub const MAX_MODELS: usize = 65_536;
pub const MAX_SLOTS: usize = 1_000_000;
pub const MAX_REFERENCES: usize = 16_000_000;
pub const MAX_PARENT_DEPTH: usize = 1024;
pub const MAX_LOAD_BYTES: usize = 1024 * 1024 * 1024;
pub const MAX_LOAD_WORK: u64 = 128_000_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub max_file_bytes: usize,
    pub max_memory_bytes: usize,
    pub max_maps: usize,
    pub max_tiles: usize,
    pub max_models: usize,
    pub max_slots: usize,
    pub max_references: usize,
    pub max_parent_depth: usize,
    pub max_load_bytes: usize,
    pub max_load_work: u64,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_file_bytes: MAX_FILE_BYTES,
            max_memory_bytes: MAX_MEMORY_BYTES,
            max_maps: MAX_MAPS,
            max_tiles: MAX_TILES,
            max_models: MAX_MODELS,
            max_slots: MAX_SLOTS,
            max_references: MAX_REFERENCES,
            max_parent_depth: MAX_PARENT_DEPTH,
            max_load_bytes: MAX_LOAD_BYTES,
            max_load_work: MAX_LOAD_WORK,
        }
    }
}
impl Limits {
    pub fn validate(self) -> Result<(), Error> {
        if self.max_file_bytes > MAX_FILE_BYTES
            || self.max_memory_bytes > MAX_MEMORY_BYTES
            || self.max_maps > MAX_MAPS
            || self.max_tiles > MAX_TILES
            || self.max_models > MAX_MODELS
            || self.max_slots > MAX_SLOTS
            || self.max_references > MAX_REFERENCES
            || self.max_parent_depth > MAX_PARENT_DEPTH
            || self.max_load_bytes > MAX_LOAD_BYTES
            || self.max_load_work > MAX_LOAD_WORK
        {
            return Err(Error::InvalidLimits);
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Options {
    pub load_path_only_models: bool,
    pub enable_line_of_sight: bool,
    pub enable_height: bool,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            load_path_only_models: false,
            enable_line_of_sight: true,
            enable_height: true,
        }
    }
}
/// Original parent fields must be supplied, not reconstructed from a single
/// effective extractor parent. None represents the source's -1 sentinel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MapParent {
    pub map_id: u32,
    pub parent: Option<u32>,
    pub cosmetic_parent: Option<u32>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidLimits,
    Limit,
    WorkLimit,
    Allocation,
    Topology,
    UnknownMap,
    Coordinates,
    Disabled,
    Poisoned,
    MissingTree,
    MissingTile,
    MissingIndex,
    MissingModel,
    EmptyTree,
    Association,
    Name,
    Source(SourceError),
    Assembly(assembly::Error),
    Model(vmo::Error),
    Query(query::Error),
}
impl Error {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidLimits => "store_invalid_limits",
            Self::Limit => "store_policy_limit",
            Self::WorkLimit => "store_work_limit",
            Self::Allocation => "store_allocation",
            Self::Topology => "store_parent_topology",
            Self::UnknownMap => "store_unknown_map",
            Self::Coordinates => "store_tile_coordinates",
            Self::Disabled => "store_disabled",
            Self::Poisoned => "store_input_changed",
            Self::MissingTree => "store_missing_tree",
            Self::MissingTile => "store_missing_tile",
            Self::MissingIndex => "store_missing_index",
            Self::MissingModel => "store_missing_model",
            Self::EmptyTree => "store_empty_tree",
            Self::Association => "store_association",
            Self::Name => "store_model_name",
            Self::Source(_) => "store_source",
            Self::Assembly(_) => "store_assembly",
            Self::Model(_) => "store_model",
            Self::Query(_) => "store_query",
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}
impl std::error::Error for Error {}
impl From<SourceError> for Error {
    fn from(e: SourceError) -> Self {
        Self::Source(e)
    }
}
impl From<assembly::Error> for Error {
    fn from(e: assembly::Error) -> Self {
        Self::Assembly(e)
    }
}
impl From<vmo::Error> for Error {
    fn from(e: vmo::Error) -> Self {
        Self::Model(e)
    }
}
impl From<query::Error> for Error {
    fn from(e: query::Error) -> Self {
        Self::Query(e)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    pub maps: usize,
    pub tiles: usize,
    pub models: usize,
    pub primitive_slots: usize,
    pub active_instances: usize,
    pub model_references: usize,
    pub tile_references: usize,
    pub resident_bytes: usize,
    pub peak_bytes: usize,
    pub source_reads: u64,
    pub source_bytes: u64,
    pub load_work: u64,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MapStats {
    pub primitive_slots: usize,
    pub active_instances: usize,
    pub tiles: usize,
    pub tile_references: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LoadReport {
    pub used_map_id: u32,
    pub records: usize,
    pub references_added: usize,
    pub new_models: usize,
    pub repeated: bool,
}
#[derive(Clone, Copy)]
pub struct StoreHit {
    spawn_id: u32,
    slot: u32,
    hit: query::model::Hit,
}
impl StoreHit {
    pub const fn spawn_id(self) -> u32 {
        self.spawn_id
    }
    pub const fn slot(self) -> u32 {
        self.slot
    }
    pub const fn hit(self) -> query::model::Hit {
        self.hit
    }
}
struct Binding {
    record: Vec<u8>,
    instance: query::model::Instance,
    model: Option<usize>,
    references: usize,
}
impl Binding {
    fn id(&self) -> u32 {
        u32::from_le_bytes(self.record[2..6].try_into().expect("bound record"))
    }
}
struct Slot {
    bounds: [f32; 6],
    binding: Option<Binding>,
}
struct Tile {
    x: u16,
    y: u8,
    references: Vec<u32>,
    loads: u64,
}
struct Map {
    id: u32,
    tree: assembly::OwnedTree,
    slots: Vec<Slot>,
    tiles: Vec<Tile>,
}
struct Model {
    name: Vec<u8>,
    model: vmo::Owned,
    references: usize,
}
struct Core {
    parents: Vec<MapParent>,
    maps: Vec<Map>,             // sorted map ID
    models: Vec<Option<Model>>, // stable slots, reused after last release
    model_index: Vec<usize>,    // sorted exact name, handles into models
}
impl Core {
    fn parent(&self, id: u32) -> Result<Option<u32>, Error> {
        let p = self
            .parents
            .binary_search_by_key(&id, |p| p.map_id)
            .map_err(|_| Error::UnknownMap)?;
        Ok(self.parents[p].parent.or(self.parents[p].cosmetic_parent))
    }
    fn map(&self, id: u32) -> Result<usize, usize> {
        self.maps.binary_search_by_key(&id, |m| m.id)
    }
    fn model(&self, name: &[u8]) -> Option<usize> {
        self.model_index
            .binary_search_by(|h| {
                self.models[*h]
                    .as_ref()
                    .expect("indexed model")
                    .name
                    .as_slice()
                    .cmp(name)
            })
            .ok()
            .map(|at| self.model_index[at])
    }
    fn stats(&self) -> Stats {
        let mut s = Stats {
            maps: self.maps.len(),
            resident_bytes: self.parents.capacity() * size_of::<MapParent>()
                + self.maps.capacity() * size_of::<Map>()
                + self.models.capacity() * size_of::<Option<Model>>()
                + self.model_index.capacity() * size_of::<usize>(),
            ..Stats::default()
        };
        for m in &self.maps {
            s.primitive_slots += m.slots.len();
            s.tiles += m.tiles.len();
            s.resident_bytes += m.tree.resident_bytes()
                + m.slots.capacity() * size_of::<Slot>()
                + m.tiles.capacity() * size_of::<Tile>();
            for slot in &m.slots {
                if let Some(b) = &slot.binding {
                    s.resident_bytes += b.record.capacity();
                    s.active_instances += usize::from(b.references != 0);
                }
            }
            for tile in &m.tiles {
                s.tile_references += tile.references.len();
                s.resident_bytes += tile.references.capacity() * size_of::<u32>();
            }
        }
        for m in self.models.iter().flatten() {
            s.models += 1;
            s.model_references += m.references;
            s.resident_bytes += m.name.capacity() + m.model.resident_bytes();
        }
        s
    }
}

pub struct StaticStore<S> {
    source: S,
    core: Core,
    options: Options,
    limits: Limits,
    poisoned: bool,
    counters: Stats,
}
impl<S: Source> StaticStore<S> {
    pub fn new(
        source: S,
        parents: &[MapParent],
        options: Options,
        limits: Limits,
    ) -> Result<Self, Error> {
        limits.validate()?;
        if parents.len() > limits.max_maps {
            return Err(Error::Limit);
        }
        let mut memory = Memory::new(0, limits.max_memory_bytes);
        let mut rows = Vec::new();
        memory.reserve(&mut rows, parents.len())?;
        rows.extend_from_slice(parents);
        rows.sort_unstable_by_key(|p| p.map_id);
        if rows.windows(2).any(|p| p[0].map_id == p[1].map_id) {
            return Err(Error::Topology);
        }
        for p in &rows {
            if p.parent == Some(u32::MAX) || p.cosmetic_parent == Some(u32::MAX) {
                return Err(Error::Topology);
            }
            if p.parent.zip(p.cosmetic_parent).is_some_and(|(a, b)| a != b) {
                return Err(Error::Topology);
            }
            let mut next = p.parent.or(p.cosmetic_parent);
            let mut depth = 0;
            while let Some(id) = next {
                depth += 1;
                if id == p.map_id || depth > limits.max_parent_depth {
                    return Err(Error::Topology);
                }
                let at = rows
                    .binary_search_by_key(&id, |r| r.map_id)
                    .map_err(|_| Error::Topology)?;
                next = rows[at].parent.or(rows[at].cosmetic_parent);
            }
        }
        Ok(Self {
            source,
            core: Core {
                parents: rows,
                maps: Vec::new(),
                models: Vec::new(),
                model_index: Vec::new(),
            },
            options,
            limits,
            poisoned: false,
            counters: Stats {
                peak_bytes: memory.peak,
                ..Stats::default()
            },
        })
    }
    pub const fn options(&self) -> Options {
        self.options
    }
    pub const fn limits(&self) -> Limits {
        self.limits
    }
    pub const fn is_poisoned(&self) -> bool {
        self.poisoned
    }
    pub fn contains_map(&self, id: u32) -> bool {
        self.core.map(id).is_ok()
    }
    pub fn is_declared(&self, id: u32) -> bool {
        self.core.parent(id).is_ok()
    }
    pub(crate) fn query_ready(&self, id: u32) -> Result<(), Error> {
        self.healthy()?;
        self.core.parent(id)?;
        Ok(())
    }
    pub fn stats(&self) -> Stats {
        Stats {
            peak_bytes: self.counters.peak_bytes,
            source_reads: self.counters.source_reads,
            source_bytes: self.counters.source_bytes,
            load_work: self.counters.load_work,
            ..self.core.stats()
        }
    }
    pub fn map_stats(&self, id: u32) -> Option<MapStats> {
        let m = &self.core.maps[self.core.map(id).ok()?];
        Some(MapStats {
            primitive_slots: m.slots.len(),
            active_instances: m
                .slots
                .iter()
                .filter(|s| s.binding.as_ref().is_some_and(|b| b.references != 0))
                .count(),
            tiles: m.tiles.len(),
            tile_references: m.tiles.iter().map(|t| t.references.len()).sum(),
        })
    }
    pub fn model_references(&self, name: &[u8]) -> Option<usize> {
        Some(
            self.core.models[self.core.model(name)?]
                .as_ref()?
                .references,
        )
    }
    fn healthy(&self) -> Result<(), Error> {
        if self.poisoned {
            Err(Error::Poisoned)
        } else {
            Ok(())
        }
    }
    fn accept_error<T>(&mut self, result: Result<T, Error>) -> Result<T, Error> {
        if matches!(result, Err(Error::Source(SourceError::InputChanged))) {
            self.poisoned = true;
        }
        result
    }
    pub fn verify_source(&mut self) -> Result<(), Error> {
        self.healthy()?;
        let result = self.source.verify_unchanged().map_err(Error::Source);
        self.accept_error(result)
    }
    pub fn init_map(&mut self, map: u32) -> Result<(), Error> {
        self.load(map, None, false).map(|_| ())
    }
    pub fn load_tile(&mut self, map: u32, x: u16, y: u8) -> Result<LoadReport, Error> {
        if x > 32767 {
            return Err(Error::Coordinates);
        }
        self.load(map, Some((x, y)), false)?
            .ok_or(Error::Association)
    }
    /// Optional component lookup. Genuine absent tree, empty tree, or simultaneous
    /// own-payload/index absence returns None after verification. Present partial
    /// packages (missing index/model/ancestor payload), corruption and source
    /// errors remain fatal. Strict load_tile behavior is unchanged.
    pub fn load_optional_tile(
        &mut self,
        map: u32,
        x: u16,
        y: u8,
    ) -> Result<Option<LoadReport>, Error> {
        if x > 32767 {
            return Err(Error::Coordinates);
        }
        match self.load(map, Some((x, y)), true) {
            Err(Error::MissingTree | Error::EmptyTree) => {
                self.verify_source()?;
                Ok(None)
            }
            r => r,
        }
    }
    pub fn effective_parent(&self, map: u32) -> Result<Option<u32>, Error> {
        self.healthy()?;
        self.core.parent(map)
    }
    fn load(
        &mut self,
        map: u32,
        tile: Option<(u16, u8)>,
        optional: bool,
    ) -> Result<Option<LoadReport>, Error> {
        self.healthy()?;
        self.core.parent(map)?;
        if !self.options.enable_line_of_sight && !self.options.enable_height {
            return Err(Error::Disabled);
        }
        if self
            .counters
            .source_reads
            .checked_add(self.limits.max_load_work)
            .is_none()
            || self
                .counters
                .source_bytes
                .checked_add(self.limits.max_load_bytes as u64)
                .is_none()
            || self
                .counters
                .load_work
                .checked_add(self.limits.max_load_work)
                .is_none()
        {
            return Err(Error::Limit);
        }
        self.verify_source()?;
        let before = self.core.stats();
        let mut memory = Memory::new(before.resident_bytes, self.limits.max_memory_bytes);
        let mut work = Work::new(self.limits);
        let result = load::prepare(
            &mut self.source,
            &self.core,
            map,
            tile,
            optional,
            self.options,
            self.limits,
            &mut memory,
            &mut work,
        )
        .and_then(|plan| {
            self.source.verify_unchanged()?;
            Ok(plan.commit(&mut self.core))
        });
        self.counters.peak_bytes = self.counters.peak_bytes.max(memory.peak);
        self.counters.source_reads += work.reads;
        self.counters.source_bytes += work.bytes as u64;
        self.counters.load_work += work.used;
        self.accept_error(result)
    }
    pub fn unload_tile(&mut self, map: u32, x: u16, y: u8) -> Result<bool, Error> {
        self.healthy()?;
        self.core.parent(map)?;
        if x > 32767 {
            return Err(Error::Coordinates);
        }
        let Ok(mi) = self.core.map(map) else {
            return Ok(false);
        };
        let Some(ti) = self.core.maps[mi]
            .tiles
            .iter()
            .position(|t| (t.x, t.y) == (x, y))
        else {
            return Ok(false);
        };
        let tile = self.core.maps[mi].tiles.remove(ti);
        for ordinal in tile.references {
            let b = self.core.maps[mi].slots[ordinal as usize]
                .binding
                .as_mut()
                .expect("tile binding");
            b.references -= 1;
            if b.references == 0 {
                let handle = b.model.take().expect("active model");
                self.release_model(handle);
            }
        }
        if self.core.maps[mi].tiles.is_empty() {
            self.core.maps.remove(mi);
        }
        Ok(true)
    }
    pub fn unload_map(&mut self, map: u32) -> Result<bool, Error> {
        self.healthy()?;
        self.core.parent(map)?;
        let Ok(mi) = self.core.map(map) else {
            return Ok(false);
        };
        let removed = self.core.maps.remove(mi);
        for s in removed.slots {
            if let Some(b) = s.binding
                && let Some(handle) = b.model
            {
                self.release_model(handle);
            }
        }
        Ok(true)
    }
    fn release_model(&mut self, handle: usize) {
        let m = self.core.models[handle].as_mut().expect("referenced model");
        m.references -= 1;
        if m.references == 0 {
            let at = self
                .core
                .model_index
                .binary_search_by(|h| {
                    self.core.models[*h]
                        .as_ref()
                        .expect("indexed model")
                        .name
                        .cmp(&self.core.models[handle].as_ref().expect("model").name)
                })
                .expect("model index");
            self.core.model_index.remove(at);
            self.core.models[handle] = None;
        }
    }
    pub fn intersect(
        &self,
        map: u32,
        ray: &query::Ray,
        maximum: f32,
        mode: query::Mode,
        ignore_m2: bool,
        budget: &mut query::Budget,
    ) -> Result<Option<StoreHit>, Error> {
        self.query_ready(map)?;
        query::validate_distance(maximum)?;
        let Ok(mi) = self.core.map(map) else {
            return Ok(None);
        };
        let map = &self.core.maps[mi];
        if map.slots.is_empty() {
            return Ok(None);
        }
        let mut distance = maximum;
        let mut hit = None;
        query::traverse::ray(
            &map.tree.bih(),
            ray,
            &mut distance,
            mode,
            budget,
            |ordinal, distance, budget| {
                let slot = map
                    .slots
                    .get(ordinal as usize)
                    .ok_or(query::Error::InvalidTree)?;
                let Some(binding) = &slot.binding else {
                    return Ok(false);
                };
                let Some(handle) = binding.model else {
                    return Ok(false);
                };
                let model = self
                    .core
                    .models
                    .get(handle)
                    .and_then(Option::as_ref)
                    .ok_or(query::Error::InvalidModel)?;
                if let Some(found) = binding.instance.intersect_owned(
                    &model.model,
                    ray,
                    *distance,
                    mode,
                    ignore_m2,
                    budget,
                )? {
                    *distance = found.distance();
                    hit = Some(StoreHit {
                        spawn_id: binding.id(),
                        slot: ordinal,
                        hit: found,
                    });
                    Ok(true)
                } else {
                    Ok(false)
                }
            },
        )?;
        Ok(hit)
    }
}
