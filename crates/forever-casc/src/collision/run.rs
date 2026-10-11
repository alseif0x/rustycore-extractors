// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: bounded local collision acquisition and placement run.
//! Serial execution of the pinned WDT/ADT acquisition and immediate-parent
//! placement-cache contract. Every requested local read must complete; failures
//! are fatal, never native-style silent omission or a reason to try another ADT.
//! WDTs preload in physical order, globals emit in that order, then maps process
//! by depth/physical order and tiles by x/y. This declares a serial ID order,
//! not equivalence to native parallel scheduling or unordered doodad iteration.
//! Unlike native per-WDT construction, all WDT reads complete before globals
//! begin; this intentional all-or-error acquisition phase preserves output/ID
//! order, not the native interleaving of acquisition with global conversion.
//!
//! Sink callbacks are staging only. Errors can leave earlier staged callbacks;
//! there is no commit, publication, automatic cleanup or successful partial
//! Report. Models are reported with their actual status, including NoCollision.
//! The original `extract` entry point has no filename/acquisition cache across
//! batches; `extract_shared` explicitly retains its supplied collector. Parent caches contain
//! only successful own ADTs; an empty own ADT blocks fallback, and a fallback
//! never populates the child's cache. Globals are never inherited.
//! Model callbacks can repeat between batches; the caller owns cross-batch
//! destination alias checks and deduplication within its staging transaction.
//!
//! Budgets are separate component limits, not a process RSS bound: retained WDT
//! capacities/descriptors, parent cache capacities, one WDT/Object parser, one
//! collision collector, one placement call with lifetime IDs, one replay, and
//! LocalStore's own policy. Caller Maps/rows/lookups and Sink retention are
//! excluded. Components can overlap; the ceilings must not be read as one total.
use super::{Extractor, InputRef, Model, ModelRequest, Status, placements, plan};
use crate::local_store::{self, LocalStore};
use forever_vmap::{
    objects::{self, Objects, Source, Target},
    placement::{self, cache, ids},
    wmo::liquid::LiquidLookup,
};

mod shared;
pub use shared::{SharedLimits, SharedReport, extract_shared};

#[cfg(test)]
mod tests;

pub const MAX_MAPS: usize = 4096;
pub const MAX_TILE_POSITIONS: u64 = 4096 * 4096;
pub const MAX_FILE_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_WDT_MEMORY_BYTES: usize = 256 * 1024 * 1024;
pub const MAX_CACHE_MEMORY_BYTES: usize = 256 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub max_maps: usize,
    /// Complete 4096-position map plans count before acquisition; never truncate.
    pub max_tile_positions: u64,
    pub max_file_bytes: usize,
    pub max_wdt_memory_bytes: usize,
    pub max_cache_memory_bytes: usize,
    /// Cumulative model raw bytes plus placement bytes sent to the Sink.
    pub max_output_bytes: u64,
    pub wdt: plan::Limits,
    pub objects: objects::Limits,
    pub collision: super::Limits,
    pub placements: placements::Limits,
    pub cache: cache::Limits,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_maps: MAX_MAPS,
            max_tile_positions: MAX_TILE_POSITIONS,
            max_file_bytes: MAX_FILE_BYTES,
            max_wdt_memory_bytes: MAX_WDT_MEMORY_BYTES,
            max_cache_memory_bytes: MAX_CACHE_MEMORY_BYTES,
            max_output_bytes: u64::MAX,
            wdt: plan::Limits::default(),
            objects: objects::Limits::default(),
            collision: super::Limits::default(),
            placements: placements::Limits::default(),
            cache: cache::Limits::default(),
        }
    }
}
impl Limits {
    fn validate(self) -> Result<(), Error> {
        let o = objects::Limits::default();
        if self.max_maps > MAX_MAPS
            || self.max_tile_positions > MAX_TILE_POSITIONS
            || self.max_file_bytes > MAX_FILE_BYTES
            || self.max_wdt_memory_bytes > MAX_WDT_MEMORY_BYTES
            || self.max_cache_memory_bytes > MAX_CACHE_MEMORY_BYTES
            || self.objects.max_input_bytes > o.max_input_bytes
            || self.objects.max_chunks > o.max_chunks
            || self.objects.max_records > o.max_records
            || self.objects.max_names > o.max_names
            || self.objects.max_name_bytes > o.max_name_bytes
            || self.objects.max_memory_bytes > o.max_memory_bytes
        {
            return Err(Error::InvalidLimits);
        }
        self.wdt.validate().map_err(Error::Plan)?;
        self.collision.validate().map_err(Error::Collision)?;
        cache::Cached::from_bytes(&[], self.cache).map_err(Error::Placement)?;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SinkError {
    Rejected,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Global,
    Tile { x: u8, y: u8 },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Context {
    pub map_id: u32,
    pub original_map_id: u32,
    pub kind: Kind,
    pub replayed: bool,
}
/// Implementations stage borrowed data during each callback and must not
/// publish it until extract returns Ok and their own transaction is complete.
pub trait Sink {
    fn model(&mut self, model: &Model) -> Result<(), SinkError>;
    fn placements(&mut self, context: Context, bytes: &[u8]) -> Result<(), SinkError>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidLimits,
    Limit,
    Allocation,
    State,
    Sink,
    Store(local_store::Error),
    Plan(plan::Error),
    Objects(objects::Error),
    Collision(super::Error),
    Placements(placements::Error),
    Placement(placement::Error),
}
impl Error {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidLimits => "collision_run_invalid_limits",
            Self::Limit => "collision_run_policy_limit",
            Self::Allocation => "collision_run_allocation",
            Self::State => "collision_run_state",
            Self::Sink => "collision_run_sink",
            Self::Store(e) => e.code(),
            Self::Plan(e) => e.code(),
            Self::Objects(e) => e.code(),
            Self::Collision(e) => e.code(),
            Self::Placements(e) => e.code(),
            Self::Placement(e) => e.code(),
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}
impl std::error::Error for Error {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Report {
    pub build: u32,
    pub maps: u64,
    pub wdt_files: u64,
    pub object_adts: u64,
    pub tile_positions: u64,
    pub own_tiles: u64,
    pub replayed_tiles: u64,
    pub empty_replays: u64,
    pub cached_tiles: u64,
    pub model_callbacks: u64,
    pub collision_models: u64,
    pub no_collision_models: u64,
    pub doodad_only_models: u64,
    pub model_reads: u64,
    pub model_source_bytes: u64,
    pub model_raw_bytes: u64,
    pub placement_callbacks: u64,
    pub placement_records: u64,
    pub placement_bytes: u64,
    pub output_bytes: u64,
    pub wdt_resident_bytes: usize,
    pub cache_resident_bytes: usize,
    pub cache_peak_bytes: usize,
    pub ids: ids::Stats,
}
struct MapState {
    id: u32,
    bytes: Vec<u8>,
    // Ordered x/y own tiles only; sparse storage also retains empty Cached.
    cache: Vec<TileCache>,
}
struct TileCache {
    cell: u16,
    value: cache::Cached,
}
struct CacheBudget {
    live: usize,
    peak: usize,
    cap: usize,
}
impl CacheBudget {
    fn check(&self, extra: usize) -> Result<usize, Error> {
        self.live
            .checked_add(extra)
            .filter(|total| *total <= self.cap)
            .ok_or(Error::Limit)
    }
    fn charge(&mut self, extra: usize) -> Result<(), Error> {
        self.live = self.check(extra)?;
        self.peak = self.peak.max(self.live);
        Ok(())
    }
}

pub fn extract<L: LiquidLookup + ?Sized, S: Sink + ?Sized>(
    store: &mut LocalStore,
    maps: &plan::Maps<'_>,
    lookup: &L,
    sink: &mut S,
    limits: Limits,
) -> Result<Report, Error> {
    limits.validate()?;
    let mut session = placements::Session::new(limits.placements).map_err(Error::Placements)?;
    let count = maps.preload().len();
    let positions = (count as u64).checked_mul(4096).ok_or(Error::Limit)?;
    if count > limits.max_maps || positions > limits.max_tile_positions {
        return Err(Error::Limit);
    }
    store.verify_unchanged().map_err(Error::Store)?;
    let mut states = Vec::new();
    let descriptors = count
        .checked_mul(size_of::<MapState>())
        .ok_or(Error::Limit)?;
    if descriptors > limits.max_wdt_memory_bytes {
        return Err(Error::Limit);
    }
    states
        .try_reserve_exact(count)
        .map_err(|_| Error::Allocation)?;
    let mut wdt_resident = capacity(&states)?;
    if wdt_resident > limits.max_wdt_memory_bytes {
        return Err(Error::Limit);
    }
    // All required WDT files are acquired before any sink callback.
    for map in maps.preload() {
        let remaining = limits.max_wdt_memory_bytes - wdt_resident;
        let limit = limits
            .max_file_bytes
            .min(limits.wdt.max_wdt_bytes)
            .min(remaining);
        let file = store
            .read_file_data_id_with_limit(map.wdt_file_data_id(), limit as u64)
            .map_err(Error::Store)?;
        wdt_resident = wdt_resident
            .checked_add(file.bytes.capacity())
            .filter(|total| *total <= limits.max_wdt_memory_bytes)
            .ok_or(Error::Limit)?;
        states.push(MapState {
            id: map.id(),
            bytes: file.bytes,
            cache: Vec::new(),
        });
    }
    let mut report = Report {
        build: store.build().build,
        maps: count as u64,
        wdt_files: count as u64,
        object_adts: 0,
        tile_positions: positions,
        own_tiles: 0,
        replayed_tiles: 0,
        empty_replays: 0,
        cached_tiles: 0,
        model_callbacks: 0,
        collision_models: 0,
        no_collision_models: 0,
        doodad_only_models: 0,
        model_reads: 0,
        model_source_bytes: 0,
        model_raw_bytes: 0,
        placement_callbacks: 0,
        placement_records: 0,
        placement_bytes: 0,
        output_bytes: 0,
        wdt_resident_bytes: wdt_resident,
        cache_resident_bytes: 0,
        cache_peak_bytes: 0,
        ids: session.id_stats(),
    };
    for (map, state) in maps.preload().zip(&states) {
        let wdt = plan::Wdt::parse(&state.bytes, limits.wdt).map_err(Error::Plan)?;
        // Validate all logical tile names before processing this map's globals.
        wdt.tiles(map).map_err(Error::Plan)?;
        let context = Context {
            map_id: map.id(),
            original_map_id: map.id(),
            kind: Kind::Global,
            replayed: false,
        };
        project(
            store,
            wdt.globals(),
            lookup,
            sink,
            &mut session,
            context,
            limits,
            &mut report,
        )?;
    }
    let mut budget = CacheBudget {
        live: 0,
        peak: 0,
        cap: limits.max_cache_memory_bytes,
    };
    for map in maps.processing() {
        let index = states
            .iter()
            .position(|state| state.id == map.id())
            .ok_or(Error::State)?;
        // WDT source stays immutable; only this map's sparse cache is mutated.
        let (prior, current_and_after) = states.split_at_mut(index);
        let (current, after) = current_and_after.split_first_mut().ok_or(Error::State)?;
        let wdt = plan::Wdt::parse(&current.bytes, limits.wdt).map_err(Error::Plan)?;
        let mut tiles = wdt.tiles(map).map_err(Error::Plan)?;
        let parent = match map.parent_map_id() {
            Some(id) => prior
                .iter()
                .chain(after.iter())
                .find(|state| state.id == id),
            None => None,
        };
        let mut names = [0; plan::MAX_NAME_BYTES];
        let mut cursor = 0u16;
        while let Some(tile) = tiles.next(&mut names) {
            let cell = u16::from(tile.x) * 64 + u16::from(tile.y);
            while cursor < cell {
                replay(maps, map, parent, cursor, sink, limits, &mut report)?;
                cursor += 1;
            }
            let file = match tile.target {
                plan::TileTarget::FileDataId(id) => store.read_file_data_id_with_limit(
                    id,
                    limits.max_file_bytes.min(limits.objects.max_input_bytes) as u64,
                ),
                plan::TileTarget::Name(name) => store.read_name_with_limit(
                    name,
                    limits.max_file_bytes.min(limits.objects.max_input_bytes) as u64,
                ),
            }
            .map_err(Error::Store)?;
            bump(&mut report.object_adts, 1)?;
            let objects =
                Objects::parse_adt(&file.bytes, limits.objects).map_err(Error::Objects)?;
            let context = Context {
                map_id: map.id(),
                original_map_id: map.id(),
                kind: Kind::Tile {
                    x: tile.x,
                    y: tile.y,
                },
                replayed: false,
            };
            let output = project(
                store,
                &objects,
                lookup,
                sink,
                &mut session,
                context,
                limits,
                &mut report,
            )?;
            bump(&mut report.own_tiles, 1)?;
            if map.is_parent() {
                let bytes = output.batches().first().ok_or(Error::State)?.bytes();
                budget.check(bytes.len())?;
                let cached = cache::Cached::from_bytes(
                    bytes,
                    cache::Limits {
                        max_memory_bytes: limits
                            .cache
                            .max_memory_bytes
                            .min(budget.cap - budget.live),
                        ..limits.cache
                    },
                )
                .map_err(Error::Placement)?;
                budget.charge(cached.resident_bytes())?;
                push_cache(
                    &mut current.cache,
                    TileCache {
                        cell,
                        value: cached,
                    },
                    &mut budget,
                )?;
                bump(&mut report.cached_tiles, 1)?;
            }
            cursor = cell + 1;
        }
        while cursor < 4096 {
            replay(maps, map, parent, cursor, sink, limits, &mut report)?;
            cursor += 1;
        }
    }
    // Also observes a metadata change performed during the final sink callback.
    store.verify_unchanged().map_err(Error::Store)?;
    report.cache_resident_bytes = budget.live;
    report.cache_peak_bytes = budget.peak;
    report.ids = session.id_stats();
    Ok(report)
}

#[allow(clippy::too_many_arguments)]
fn project<L: LiquidLookup + ?Sized, S: Sink + ?Sized>(
    store: &mut LocalStore,
    objects: &Objects<'_>,
    lookup: &L,
    sink: &mut S,
    session: &mut placements::Session,
    context: Context,
    limits: Limits,
    report: &mut Report,
) -> Result<placements::Output, Error> {
    let mut extractor = Extractor::new(
        store,
        lookup,
        super::Limits {
            max_file_bytes: limits.collision.max_file_bytes.min(limits.max_file_bytes),
            ..limits.collision
        },
    )
    .map_err(Error::Collision)?;
    for request in objects.requests() {
        let target = match request.target() {
            Target::FileDataId(id) => InputRef::FileDataId(id),
            Target::Name(name) => InputRef::LogicalName(name),
        };
        let request = match request.source() {
            Source::M2(_) => ModelRequest::M2(target),
            Source::Wmo(_) => ModelRequest::Wmo(target),
        };
        extractor.extract(request).map_err(Error::Collision)?;
    }
    let extraction = extractor.finish().map_err(Error::Collision)?;
    bump(&mut report.model_reads, extraction.facts().reads as u64)?;
    bump(
        &mut report.model_source_bytes,
        extraction.facts().source_bytes as u64,
    )?;
    for model in extraction.models() {
        let size = model.raw().map_or(0, <[u8]>::len) as u64;
        output_bytes(report, size, limits)?;
        sink.model(model).map_err(|_| Error::Sink)?;
        bump(&mut report.model_callbacks, 1)?;
        bump(&mut report.model_raw_bytes, size)?;
        let count = match model.status() {
            Status::Collision => &mut report.collision_models,
            Status::NoCollision => &mut report.no_collision_models,
            Status::DoodadOnly => &mut report.doodad_only_models,
        };
        bump(count, 1)?;
    }
    let output = session
        .project(
            &extraction,
            &[placements::Batch {
                objects,
                map_id: context.map_id,
                original_map_id: context.original_map_id,
            }],
        )
        .map_err(Error::Placements)?;
    let batch = output.batches().first().ok_or(Error::State)?;
    emit(
        sink,
        context,
        batch.bytes(),
        batch.record_count(),
        limits,
        report,
    )?;
    Ok(output)
}

#[allow(clippy::too_many_arguments)]
fn replay<S: Sink + ?Sized>(
    maps: &plan::Maps<'_>,
    map: plan::MapEntry<'_>,
    parent: Option<&MapState>,
    cell: u16,
    sink: &mut S,
    limits: Limits,
    report: &mut Report,
) -> Result<(), Error> {
    let Some(parent_map) = maps.fallback_parent(map.id()).map_err(Error::Plan)? else {
        return Ok(());
    };
    let parent = parent
        .filter(|state| state.id == parent_map.id())
        .ok_or(Error::State)?;
    let Ok(at) = parent.cache.binary_search_by_key(&cell, |entry| entry.cell) else {
        return Ok(());
    };
    let cached = &parent.cache[at].value;
    let bytes = cached
        .replay(map.id(), parent.id, limits.cache)
        .map_err(Error::Placement)?;
    emit(
        sink,
        Context {
            map_id: map.id(),
            original_map_id: parent.id,
            kind: Kind::Tile {
                x: (cell / 64) as u8,
                y: (cell % 64) as u8,
            },
            replayed: true,
        },
        &bytes,
        cached.record_count(),
        limits,
        report,
    )?;
    bump(&mut report.replayed_tiles, 1)?;
    if cached.record_count() == 0 {
        bump(&mut report.empty_replays, 1)?;
    }
    Ok(())
}
fn emit<S: Sink + ?Sized>(
    sink: &mut S,
    context: Context,
    bytes: &[u8],
    records: usize,
    limits: Limits,
    report: &mut Report,
) -> Result<(), Error> {
    output_bytes(report, bytes.len() as u64, limits)?;
    sink.placements(context, bytes).map_err(|_| Error::Sink)?;
    bump(&mut report.placement_callbacks, 1)?;
    bump(&mut report.placement_records, records as u64)?;
    bump(&mut report.placement_bytes, bytes.len() as u64)
}
fn output_bytes(report: &mut Report, bytes: u64, limits: Limits) -> Result<(), Error> {
    let next = report.output_bytes.checked_add(bytes).ok_or(Error::Limit)?;
    if next > limits.max_output_bytes {
        return Err(Error::Limit);
    }
    report.output_bytes = next;
    Ok(())
}
fn bump(value: &mut u64, count: u64) -> Result<(), Error> {
    *value = value.checked_add(count).ok_or(Error::Limit)?;
    Ok(())
}
fn capacity<T>(values: &Vec<T>) -> Result<usize, Error> {
    values
        .capacity()
        .checked_mul(size_of::<T>())
        .ok_or(Error::Limit)
}
fn push_cache(
    values: &mut Vec<TileCache>,
    value: TileCache,
    budget: &mut CacheBudget,
) -> Result<(), Error> {
    if values.len() >= 4096 {
        return Err(Error::Limit);
    }
    if values.len() == values.capacity() {
        let old = capacity(values)?;
        let next = values.capacity().saturating_mul(2).clamp(8, 4096);
        budget.check(
            next.checked_mul(size_of::<TileCache>())
                .ok_or(Error::Limit)?,
        )?;
        values
            .try_reserve_exact(next - values.len())
            .map_err(|_| Error::Allocation)?;
        let actual = capacity(values)?;
        budget.peak = budget.peak.max(budget.check(actual)?);
        budget.live = budget.live - old + actual;
    }
    values.push(value);
    Ok(())
}
