// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: shared startup-to-WDT/ADT collision execution.
//! Explicit addition to the existing per-batch runner. One collector retains
//! startup RAW, dependency handles, omissions and all cumulative budgets. Only
//! the current Objects request window is projected. One run is admitted per
//! collector, including across lookup transitions; every failure is sticky.
//!
//! WDTs all read before globals, then serial depth/physical-map and x/y order.
//! These are declared scheduling rules, not native parallel-order equivalence.
//! Sink callbacks only stage. Startup models and each subsequently inserted
//! model are delivered once, including omission states. No publication occurs.
//!
//! Run-owned source capacities, parser allowances, WDT/cache descriptors and
//! cache growth overlap share the collector's logical memory ceiling. A bridge
//! reservation covers its entire Session (including ID ceiling) and one Output.
//! Caller Maps/strings, Sink storage, stack and allocator overhead are excluded.
//! Existing cumulative collector ceilings may reject a complete large map; no
//! counter reset, truncated plan, implicit cache eviction or full-build claim.
use super::*;
use crate::collision::{Facts, gameobjects};

#[cfg(test)]
mod tests;

/// Only reductions of run component ceilings. The already established
/// collector owns all model, source-total, read, request and cache-entry limits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SharedLimits {
    pub max_maps: usize,
    pub max_tile_positions: u64,
    /// WDT/ADT per-file cap; model reads use the collector's established cap.
    /// All source reads additionally obey that collector cap.
    pub max_file_bytes: usize,
    pub max_wdt_memory_bytes: usize,
    pub max_cache_memory_bytes: usize,
    pub max_output_bytes: u64,
    pub wdt: plan::Limits,
    pub objects: objects::Limits,
    pub placements: placements::Limits,
    pub cache: cache::Limits,
}
impl Default for SharedLimits {
    fn default() -> Self {
        let defaults = Limits::default();
        Self {
            max_maps: defaults.max_maps,
            max_tile_positions: defaults.max_tile_positions,
            max_file_bytes: defaults.max_file_bytes,
            max_wdt_memory_bytes: defaults.max_wdt_memory_bytes,
            max_cache_memory_bytes: defaults.max_cache_memory_bytes,
            max_output_bytes: defaults.max_output_bytes,
            wdt: defaults.wdt,
            objects: defaults.objects,
            placements: defaults.placements,
            cache: defaults.cache,
        }
    }
}
impl SharedLimits {
    // Component-only use of existing validators/output accounting. The dummy
    // collision limits are never installed on or used to construct a collector.
    fn components(self) -> Limits {
        Limits {
            max_maps: self.max_maps,
            max_tile_positions: self.max_tile_positions,
            max_file_bytes: self.max_file_bytes,
            max_wdt_memory_bytes: self.max_wdt_memory_bytes,
            max_cache_memory_bytes: self.max_cache_memory_bytes,
            max_output_bytes: self.max_output_bytes,
            wdt: self.wdt,
            objects: self.objects,
            collision: super::super::Limits::default(),
            placements: self.placements,
            cache: self.cache,
        }
    }
    pub(crate) fn validate(self) -> Result<(), Error> {
        self.components().validate()?;
        placements::Session::new(self.placements).map_err(Error::Placements)?;
        Ok(())
    }
}
/// `run.model_reads/model_source_bytes` count only deltas during model requests,
/// excluding startup and WDT/ADT reads. Model callback/raw totals include the
/// initial inventory once. Collector snapshots cover all phases and sources.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SharedReport {
    pub run: Report,
    pub collector_before: Facts,
    pub collector_after: Facts,
    pub request_calls_before: usize,
    pub request_calls_after: usize,
}

pub fn extract_shared<L: LiquidLookup + ?Sized, S: Sink + ?Sized>(
    context: &mut gameobjects::Context<'_, '_, L>,
    maps: &plan::Maps<'_>,
    sink: &mut S,
    limits: SharedLimits,
) -> Result<SharedReport, Error> {
    let collector = context.collector_mut();
    let result = execute(collector, maps, sink, limits);
    if result.is_err() {
        collector.abort();
    }
    result
}

struct Run<'a, 's, 'l, L: LiquidLookup + ?Sized, S: Sink + ?Sized> {
    collector: &'a mut Extractor<'s, 'l, L>,
    sink: &'a mut S,
    session: placements::Session,
    limits: Limits,
    report: Report,
    emitted: usize,
    cache_budget: CacheBudget,
}
fn execute<L: LiquidLookup + ?Sized, S: Sink + ?Sized>(
    collector: &mut Extractor<'_, '_, L>,
    maps: &plan::Maps<'_>,
    sink: &mut S,
    limits: SharedLimits,
) -> Result<SharedReport, Error> {
    collector.begin_shared_run().map_err(Error::Collision)?;
    limits.validate()?;
    let before = collector.facts();
    let calls_before = collector.request_calls();
    let count = maps.preload().len();
    let positions = (count as u64).checked_mul(4096).ok_or(Error::Limit)?;
    if count > limits.max_maps || positions > limits.max_tile_positions {
        return Err(Error::Limit);
    }
    let bridge = limits
        .placements
        .max_memory_bytes
        .min(collector.free_bytes().map_err(Error::Collision)?);
    if bridge < limits.placements.ids.max_memory_bytes {
        return Err(Error::Limit);
    }
    collector
        .reserve_external(bridge)
        .map_err(Error::Collision)?;
    let session = placements::Session::new(placements::Limits {
        max_memory_bytes: bridge,
        ..limits.placements
    })
    .map_err(Error::Placements)?;
    let requested = count
        .checked_mul(size_of::<MapState>())
        .ok_or(Error::Limit)?;
    if requested > limits.max_wdt_memory_bytes {
        return Err(Error::Limit);
    }
    collector
        .reserve_external(requested)
        .map_err(Error::Collision)?;
    let mut states = Vec::new();
    states
        .try_reserve_exact(count)
        .map_err(|_| Error::Allocation)?;
    let descriptors = capacity(&states)?;
    if descriptors > limits.max_wdt_memory_bytes {
        return Err(Error::Limit);
    }
    collector
        .reserve_external(descriptors.checked_sub(requested).ok_or(Error::State)?)
        .map_err(Error::Collision)?;
    let mut wdt_resident = descriptors;
    for map in maps.preload() {
        let cap = limits
            .max_file_bytes
            .min(limits.wdt.max_wdt_bytes)
            .min(limits.max_wdt_memory_bytes - wdt_resident);
        let (bytes, _) = collector
            .read_source(map.wdt_file_data_id(), cap)
            .map_err(Error::Collision)?;
        wdt_resident = wdt_resident
            .checked_add(bytes.capacity())
            .filter(|n| *n <= limits.max_wdt_memory_bytes)
            .ok_or(Error::Limit)?;
        states.push(MapState {
            id: map.id(),
            bytes,
            cache: Vec::new(),
        });
    }
    let report = Report {
        build: before.build,
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
    let mut run = Run {
        collector,
        sink,
        session,
        limits: limits.components(),
        report,
        emitted: 0,
        cache_budget: CacheBudget {
            live: 0,
            peak: 0,
            cap: limits.max_cache_memory_bytes,
        },
    };
    run.models()?;
    for (map, state) in maps.preload().zip(&states) {
        let wdt = run.wdt(&state.bytes)?;
        wdt.tiles(map).map_err(Error::Plan)?;
        run.project(
            wdt.globals(),
            Context {
                map_id: map.id(),
                original_map_id: map.id(),
                kind: Kind::Global,
                replayed: false,
            },
        )?;
        let resident = wdt.stats().resident_bytes;
        drop(wdt);
        run.collector
            .release_external(resident)
            .map_err(Error::Collision)?;
    }
    for map in maps.processing() {
        let index = states
            .iter()
            .position(|state| state.id == map.id())
            .ok_or(Error::State)?;
        let (prior, rest) = states.split_at_mut(index);
        let (current, after) = rest.split_first_mut().ok_or(Error::State)?;
        let wdt = run.wdt(&current.bytes)?;
        let mut tiles = wdt.tiles(map).map_err(Error::Plan)?;
        let parent = map.parent_map_id().and_then(|id| {
            prior
                .iter()
                .chain(after.iter())
                .find(|state| state.id == id)
        });
        let mut names = [0; plan::MAX_NAME_BYTES];
        let mut cursor = 0u16;
        while let Some(tile) = tiles.next(&mut names) {
            let cell = u16::from(tile.x) * 64 + u16::from(tile.y);
            while cursor < cell {
                run.replay(maps, map, parent, cursor)?;
                cursor += 1;
            }
            let target = match tile.target {
                plan::TileTarget::FileDataId(id) => InputRef::FileDataId(id),
                plan::TileTarget::Name(name) => InputRef::LogicalName(name),
            };
            let (bytes, _) = run
                .collector
                .read_source_ref(
                    target,
                    limits.max_file_bytes.min(limits.objects.max_input_bytes),
                )
                .map_err(Error::Collision)?;
            bump(&mut run.report.object_adts, 1)?;
            let objects = run.objects(&bytes)?;
            let output = run.project(
                &objects,
                Context {
                    map_id: map.id(),
                    original_map_id: map.id(),
                    kind: Kind::Tile {
                        x: tile.x,
                        y: tile.y,
                    },
                    replayed: false,
                },
            )?;
            bump(&mut run.report.own_tiles, 1)?;
            if map.is_parent() {
                let batch = output.batches().first().ok_or(Error::State)?;
                run.cache(&mut current.cache, cell, batch.bytes())?;
                bump(&mut run.report.cached_tiles, 1)?;
            }
            drop(output); // still covered by the lifetime bridge reservation
            let object_resident = objects.stats().resident_bytes;
            drop(objects);
            run.collector
                .release_external(object_resident)
                .map_err(Error::Collision)?;
            let source_capacity = bytes.capacity();
            drop(bytes);
            run.collector
                .release_source(source_capacity)
                .map_err(Error::Collision)?;
            cursor = cell + 1;
        }
        while cursor < 4096 {
            run.replay(maps, map, parent, cursor)?;
            cursor += 1;
        }
        let resident = wdt.stats().resident_bytes;
        drop(wdt);
        run.collector
            .release_external(resident)
            .map_err(Error::Collision)?;
    }
    run.collector.verify().map_err(Error::Collision)?;
    run.report.cache_resident_bytes = run.cache_budget.live;
    run.report.cache_peak_bytes = run.cache_budget.peak;
    run.report.ids = run.session.id_stats();
    drop(states);
    run.collector
        .release_source(wdt_resident - descriptors)
        .map_err(Error::Collision)?;
    run.collector
        .release_external(
            descriptors
                .checked_add(run.cache_budget.live)
                .ok_or(Error::Limit)?,
        )
        .map_err(Error::Collision)?;
    drop(run.session);
    run.collector
        .release_external(bridge)
        .map_err(Error::Collision)?;
    Ok(SharedReport {
        run: run.report,
        collector_before: before,
        collector_after: run.collector.facts(),
        request_calls_before: calls_before,
        request_calls_after: run.collector.request_calls(),
    })
}
impl<L: LiquidLookup + ?Sized, S: Sink + ?Sized> Run<'_, '_, '_, L, S> {
    fn wdt<'b>(&mut self, bytes: &'b [u8]) -> Result<plan::Wdt<'b>, Error> {
        let allowance = self
            .limits
            .wdt
            .max_memory_bytes
            .min(self.collector.free_bytes().map_err(Error::Collision)?);
        self.collector
            .reserve_external(allowance)
            .map_err(Error::Collision)?;
        let value = plan::Wdt::parse(
            bytes,
            plan::Limits {
                max_memory_bytes: allowance,
                ..self.limits.wdt
            },
        )
        .map_err(Error::Plan)?;
        let resident = value.stats().resident_bytes;
        self.collector
            .release_external(allowance.checked_sub(resident).ok_or(Error::State)?)
            .map_err(Error::Collision)?;
        Ok(value)
    }
    fn objects<'b>(&mut self, bytes: &'b [u8]) -> Result<Objects<'b>, Error> {
        let allowance = self
            .limits
            .objects
            .max_memory_bytes
            .min(self.collector.free_bytes().map_err(Error::Collision)?);
        self.collector
            .reserve_external(allowance)
            .map_err(Error::Collision)?;
        let value = Objects::parse_adt(
            bytes,
            objects::Limits {
                max_memory_bytes: allowance,
                ..self.limits.objects
            },
        )
        .map_err(Error::Objects)?;
        self.collector
            .release_external(
                allowance
                    .checked_sub(value.stats().resident_bytes)
                    .ok_or(Error::State)?,
            )
            .map_err(Error::Collision)?;
        Ok(value)
    }
    fn models(&mut self) -> Result<(), Error> {
        for model in &self.collector.models()[self.emitted..] {
            let bytes = model.raw().map_or(0, <[u8]>::len) as u64;
            output_bytes(&mut self.report, bytes, self.limits)?;
            self.sink.model(model).map_err(|_| Error::Sink)?;
            bump(&mut self.report.model_callbacks, 1)?;
            bump(&mut self.report.model_raw_bytes, bytes)?;
            bump(
                match model.status() {
                    Status::Collision => &mut self.report.collision_models,
                    Status::NoCollision => &mut self.report.no_collision_models,
                    Status::DoodadOnly => &mut self.report.doodad_only_models,
                },
                1,
            )?;
        }
        self.emitted = self.collector.models().len();
        Ok(())
    }
    fn project(
        &mut self,
        objects: &Objects<'_>,
        context: Context,
    ) -> Result<placements::Output, Error> {
        let start = self.collector.request_len();
        let before = self.collector.facts();
        for request in objects.requests() {
            let target = match request.target() {
                Target::FileDataId(id) => InputRef::FileDataId(id),
                Target::Name(name) => InputRef::LogicalName(name),
            };
            let request = match request.source() {
                Source::M2(_) => ModelRequest::M2(target),
                Source::Wmo(_) => ModelRequest::Wmo(target),
            };
            self.collector.extract(request).map_err(Error::Collision)?;
        }
        let after = self.collector.facts();
        bump(
            &mut self.report.model_reads,
            after.reads.checked_sub(before.reads).ok_or(Error::State)? as u64,
        )?;
        bump(
            &mut self.report.model_source_bytes,
            after
                .source_bytes
                .checked_sub(before.source_bytes)
                .ok_or(Error::State)? as u64,
        )?;
        self.models()?;
        let view = self
            .collector
            .request_view(start, self.collector.request_len())
            .map_err(Error::Collision)?;
        let output = self
            .session
            .project_view(
                view,
                &[placements::Batch {
                    objects,
                    map_id: context.map_id,
                    original_map_id: context.original_map_id,
                }],
            )
            .map_err(Error::Placements)?;
        let batch = output.batches().first().ok_or(Error::State)?;
        emit(
            self.sink,
            context,
            batch.bytes(),
            batch.record_count(),
            self.limits,
            &mut self.report,
        )?;
        Ok(output)
    }
    fn cache(
        &mut self,
        entries: &mut Vec<TileCache>,
        cell: u16,
        bytes: &[u8],
    ) -> Result<(), Error> {
        let allowance = self
            .limits
            .cache
            .max_memory_bytes
            .min(self.cache_budget.cap - self.cache_budget.live)
            .min(self.collector.free_bytes().map_err(Error::Collision)?);
        self.collector
            .reserve_external(allowance)
            .map_err(Error::Collision)?;
        let value = cache::Cached::from_bytes(
            bytes,
            cache::Limits {
                max_memory_bytes: allowance,
                ..self.limits.cache
            },
        )
        .map_err(Error::Placement)?;
        self.collector
            .release_external(
                allowance
                    .checked_sub(value.resident_bytes())
                    .ok_or(Error::State)?,
            )
            .map_err(Error::Collision)?;
        self.cache_budget.charge(value.resident_bytes())?;
        if entries.len() >= 4096 {
            return Err(Error::Limit);
        }
        if entries.len() == entries.capacity() {
            let old = capacity(entries)?;
            let next = entries.capacity().saturating_mul(2).clamp(8, 4096);
            let requested = next
                .checked_mul(size_of::<TileCache>())
                .ok_or(Error::Limit)?;
            self.cache_budget.check(requested)?;
            self.collector
                .reserve_external(requested)
                .map_err(Error::Collision)?;
            entries
                .try_reserve_exact(next - entries.len())
                .map_err(|_| Error::Allocation)?;
            let actual = capacity(entries)?;
            self.cache_budget.peak = self.cache_budget.peak.max(self.cache_budget.check(actual)?);
            self.collector
                .reserve_external(actual.checked_sub(requested).ok_or(Error::State)?)
                .map_err(Error::Collision)?;
            self.collector
                .release_external(old)
                .map_err(Error::Collision)?;
            self.cache_budget.live = self.cache_budget.live - old + actual;
        }
        entries.push(TileCache { cell, value });
        Ok(())
    }
    fn replay(
        &mut self,
        maps: &plan::Maps<'_>,
        map: plan::MapEntry<'_>,
        parent: Option<&MapState>,
        cell: u16,
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
        let old = cached.resident_bytes();
        let allowance = self
            .limits
            .cache
            .max_memory_bytes
            .checked_sub(old)
            .ok_or(Error::Limit)?
            .min(self.collector.free_bytes().map_err(Error::Collision)?);
        self.collector
            .reserve_external(allowance)
            .map_err(Error::Collision)?;
        let bytes = cached
            .replay(
                map.id(),
                parent.id,
                cache::Limits {
                    max_memory_bytes: old.checked_add(allowance).ok_or(Error::Limit)?,
                    ..self.limits.cache
                },
            )
            .map_err(Error::Placement)?;
        emit(
            self.sink,
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
            self.limits,
            &mut self.report,
        )?;
        drop(bytes);
        self.collector
            .release_external(allowance)
            .map_err(Error::Collision)?;
        bump(&mut self.report.replayed_tiles, 1)?;
        if cached.record_count() == 0 {
            bump(&mut self.report.empty_replays, 1)?;
        }
        Ok(())
    }
}
