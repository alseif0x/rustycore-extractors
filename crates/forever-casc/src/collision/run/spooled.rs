// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-12: original shared disk-backed-model world runner.
//! Same spooled startup collector, serial IDs and immutable cached models.
//! WDT and own-tile parent caches remain bounded resident components; this is
//! not full-world memory/product acceptance. Sink writes remain provisional.
use super::{Context, Kind, SharedLimits, capacity};
use crate::collision::{
    Facts, InputRef, ModelRequest, gameobjects,
    payload::{self, CallbackFailure, CollectorError, SpoolingExtractor},
    placements::{self, spooled as bridge},
    plan,
};
use forever_vmap::{
    objects::{self, Objects, Source, Target},
    placement::{cache, ids},
    wmo::liquid::LiquidLookup,
};
#[cfg(test)]
mod tests;
pub const PROFILE: &str = "linux-private-spooled-shared-world-run-v1";
/// Owned implementations enforce their declared heap allowance. The runner
/// drops them before releasing it. No retained memory may escape that contract.
/// Caller Map rows/strings and immutable source residency remain separate.
pub trait Sink {
    type Error: Copy;
    fn placements(&mut self, context: Context, bytes: &[u8]) -> Result<(), Self::Error>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StageError<E> {
    Run(super::Error),
    Sink(E),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cause<E> {
    Run(super::Error),
    Collector(CollectorError),
    Bridge(bridge::Error<StageError<E>>),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Error<E> {
    cause: Cause<E>,
    guard: Option<CollectorError>,
}
fn changed(e: CollectorError) -> bool {
    matches!(
        e,
        CollectorError::Payload(payload::Error::Changed)
            | CollectorError::Collision(crate::collision::Error::Store(
                crate::local_store::Error::InputChanged
            ))
    )
}
impl<E: Copy> Error<E> {
    pub const fn cause(self) -> Cause<E> {
        self.cause
    }
    pub const fn guard(self) -> Option<CollectorError> {
        self.guard
    }
    pub fn code(self) -> &'static str {
        if let Some(e) = self.guard
            && changed(e)
        {
            return e.code();
        }
        match self.cause {
            Cause::Run(e) => e.code(),
            Cause::Collector(e) => e.code(),
            Cause::Bridge(e) => {
                if e.guard().is_some_and(changed) {
                    return e.code();
                }
                match e.cause() {
                    bridge::Cause::Sink(StageError::Run(e)) => e.code(),
                    bridge::Cause::Sink(StageError::Sink(_)) => "collision_spooled_run_sink",
                    _ => e.code(),
                }
            }
        }
    }
    fn with_guard(mut self, guard: Option<CollectorError>) -> Self {
        if let Some(e) = guard
            && (changed(e) || !self.guard.is_some_and(changed))
        {
            self.guard = Some(e);
        }
        self
    }
}
impl<E> From<super::Error> for Error<E> {
    fn from(e: super::Error) -> Self {
        Self {
            cause: Cause::Run(e),
            guard: None,
        }
    }
}
impl<E> From<CollectorError> for Error<E> {
    fn from(e: CollectorError) -> Self {
        Self {
            cause: Cause::Collector(e),
            guard: None,
        }
    }
}
impl<E> From<bridge::Error<StageError<E>>> for Error<E> {
    fn from(e: bridge::Error<StageError<E>>) -> Self {
        Self {
            cause: Cause::Bridge(e),
            guard: None,
        }
    }
}
impl<E: Copy> std::fmt::Display for Error<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}
impl<E: Copy + std::fmt::Debug> std::error::Error for Error<E> {}
fn finish_error<E: Copy>(e: CallbackFailure<Error<E>>) -> Error<E> {
    match e {
        CallbackFailure::Collector(e) => e.into(),
        CallbackFailure::Callback { cause, guard } => cause.with_guard(guard),
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Work {
    placements: u64,
    records: u64,
    bytes: u64,
    own: u64,
    replays: u64,
    empty_replays: u64,
    cached: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Report {
    pub maps: usize,
    pub tile_positions: u64,
    pub object_adts: u64,
    pub own_tiles: u64,
    pub replayed_tiles: u64,
    pub empty_replays: u64,
    pub cached_tiles: u64,
    pub placement_callbacks: u64,
    pub placement_records: u64,
    pub placement_bytes: u64,
    pub model_reads: u64,
    pub model_source_bytes: u64,
    pub model_raw_bytes: usize,
    pub wdt_resident_bytes: usize,
    pub cache_resident_bytes: usize,
    pub cache_peak_bytes: usize,
    pub ids: ids::Stats,
    pub collector_before: Facts,
    pub collector_after: Facts,
    pub request_calls_before: usize,
    pub request_calls_after: usize,
}
struct Tile {
    cell: u16,
    value: cache::Cached,
}
struct CachedMap {
    id: u32,
    tiles: Vec<Tile>,
}
struct Wdt {
    id: u32,
    bytes: Vec<u8>,
}
struct Stage<S: Sink> {
    sink: S,
    context: Option<Context>,
    save_to: Option<usize>,
    maps: Vec<CachedMap>,
    budget: super::CacheBudget,
    cache_limits: cache::Limits,
    output_limit: u64,
    raw: usize,
    work: Work,
}
impl<S: Sink> Stage<S> {
    fn initialize(&mut self, maps: &plan::Maps<'_>) -> Result<(), StageError<S::Error>> {
        let requested = maps
            .preload()
            .len()
            .checked_mul(size_of::<CachedMap>())
            .ok_or(super::Error::Limit)
            .map_err(StageError::Run)?;
        self.budget.check(requested).map_err(StageError::Run)?;
        self.maps
            .try_reserve_exact(maps.preload().len())
            .map_err(|_| StageError::Run(super::Error::Allocation))?;
        self.budget
            .charge(capacity(&self.maps).map_err(StageError::Run)?)
            .map_err(StageError::Run)?;
        for map in maps.preload() {
            self.maps.push(CachedMap {
                id: map.id(),
                tiles: Vec::new(),
            });
        }
        self.check_output(0)?;
        Ok(())
    }
    fn check_output(&self, additional: u64) -> Result<(), StageError<S::Error>> {
        if (self.raw as u64)
            .checked_add(self.work.bytes)
            .and_then(|n| n.checked_add(additional))
            .is_none_or(|n| n > self.output_limit)
        {
            return Err(StageError::Run(super::Error::Limit));
        }
        Ok(())
    }
    fn emit(
        &mut self,
        context: Context,
        bytes: &[u8],
        records: usize,
    ) -> Result<(), StageError<S::Error>> {
        self.check_output(bytes.len() as u64)?;
        self.sink
            .placements(context, bytes)
            .map_err(StageError::Sink)?;
        super::bump(&mut self.work.placements, 1).map_err(StageError::Run)?;
        super::bump(&mut self.work.records, records as u64).map_err(StageError::Run)?;
        super::bump(&mut self.work.bytes, bytes.len() as u64).map_err(StageError::Run)
    }
    fn save(&mut self, index: usize, cell: u16, bytes: &[u8]) -> Result<(), StageError<S::Error>> {
        let allowance = self
            .cache_limits
            .max_memory_bytes
            .min(self.budget.cap - self.budget.live);
        let value = cache::Cached::from_bytes(
            bytes,
            cache::Limits {
                max_memory_bytes: allowance,
                ..self.cache_limits
            },
        )
        .map_err(|e| StageError::Run(super::Error::Placement(e)))?;
        self.budget
            .charge(value.resident_bytes())
            .map_err(StageError::Run)?;
        let entries = &mut self
            .maps
            .get_mut(index)
            .ok_or(StageError::Run(super::Error::State))?
            .tiles;
        if entries.len() >= 4096 || entries.last().is_some_and(|t| t.cell >= cell) {
            return Err(StageError::Run(super::Error::State));
        }
        if entries.len() == entries.capacity() {
            let old = capacity(entries).map_err(StageError::Run)?;
            let next = entries.capacity().saturating_mul(2).clamp(8, 4096);
            let requested = next
                .checked_mul(size_of::<Tile>())
                .ok_or(StageError::Run(super::Error::Limit))?;
            self.budget.check(requested).map_err(StageError::Run)?;
            entries
                .try_reserve_exact(next - entries.len())
                .map_err(|_| StageError::Run(super::Error::Allocation))?;
            let actual = capacity(entries).map_err(StageError::Run)?;
            self.budget.peak = self
                .budget
                .peak
                .max(self.budget.check(actual).map_err(StageError::Run)?);
            self.budget.live = self.budget.live - old + actual;
        }
        entries.push(Tile { cell, value });
        super::bump(&mut self.work.cached, 1).map_err(StageError::Run)
    }
}
impl<S: Sink> bridge::Sink for Stage<S> {
    type Error = StageError<S::Error>;
    fn stage(&mut self, batch: bridge::Staged<'_>) -> Result<(), Self::Error> {
        let context = self
            .context
            .take()
            .ok_or(StageError::Run(super::Error::State))?;
        if context.map_id != batch.map_id
            || context.original_map_id != batch.original_map_id
            || context.replayed
        {
            return Err(StageError::Run(super::Error::State));
        }
        self.emit(context, batch.bytes, batch.records)?;
        if let Kind::Tile { x, y } = context.kind {
            super::bump(&mut self.work.own, 1).map_err(StageError::Run)?;
            if let Some(index) = self.save_to.take() {
                self.save(index, u16::from(x) * 64 + u16::from(y), batch.bytes)?;
            }
        } else if self.save_to.is_some() {
            return Err(StageError::Run(super::Error::State));
        }
        Ok(())
    }
}
/// One run per collector. Explicit allowances are downward-only: the cache
/// allowance includes map/tile descriptors and all cache byte/growth capacities;
/// the user Sink must enforce its own allowance. Both are fully precharged.
/// Large resident WDT/cache selections may reject; no truncation or eviction.
pub fn extract<L: LiquidLookup + ?Sized, S: Sink>(
    context: &mut gameobjects::spooled::Context<'_, '_, L>,
    maps: &plan::Maps<'_>,
    sink: S,
    cache_heap_allowance: usize,
    sink_heap_allowance: usize,
    limits: SharedLimits,
) -> Result<Report, Error<S::Error>> {
    extract_precharged(
        context,
        maps,
        sink,
        cache_heap_allowance,
        sink_heap_allowance,
        0,
        limits,
    )
}
// Consumes the same world-owned Sink reservation; there is no uncharged gap.
pub(crate) fn extract_precharged<L: LiquidLookup + ?Sized, S: Sink>(
    context: &mut gameobjects::spooled::Context<'_, '_, L>,
    maps: &plan::Maps<'_>,
    sink: S,
    cache_heap_allowance: usize,
    sink_heap_allowance: usize,
    precharged: usize,
    limits: SharedLimits,
) -> Result<Report, Error<S::Error>> {
    let collector = context.collector_mut();
    let result = execute(
        collector,
        maps,
        sink,
        cache_heap_allowance,
        sink_heap_allowance,
        precharged,
        limits,
    );
    collector
        .finish_external_operation(result)
        .map_err(finish_error)
}
fn execute<L: LiquidLookup + ?Sized, S: Sink>(
    collector: &mut SpoolingExtractor<'_, '_, L>,
    maps: &plan::Maps<'_>,
    sink: S,
    cache_heap_allowance: usize,
    sink_heap_allowance: usize,
    precharged: usize,
    limits: SharedLimits,
) -> Result<Report, Error<S::Error>> {
    collector.begin_shared_run()?;
    limits.validate()?;
    if precharged != 0 && precharged != sink_heap_allowance {
        return Err(super::Error::State.into());
    }
    let count = maps.preload().len();
    let positions = (count as u64)
        .checked_mul(4096)
        .ok_or(super::Error::Limit)?;
    if count > limits.max_maps
        || positions > limits.max_tile_positions
        || cache_heap_allowance > limits.max_cache_memory_bytes
    {
        return Err(super::Error::Limit.into());
    }
    let before = collector.facts()?;
    let calls_before = collector.request_calls();
    let heap = cache_heap_allowance
        .checked_add(sink_heap_allowance)
        .ok_or(super::Error::Limit)?;
    let stage = Stage {
        sink,
        context: None,
        save_to: None,
        maps: Vec::new(),
        budget: super::CacheBudget {
            live: 0,
            peak: 0,
            cap: cache_heap_allowance,
        },
        cache_limits: limits.cache,
        output_limit: limits.max_output_bytes,
        raw: before.raw_bytes,
        work: Work::default(),
    };
    let mut session =
        bridge::Session::new_precharged(collector, stage, heap, precharged, limits.placements)?;
    session.run_with_sink(|s| s.initialize(maps))?;
    let requested = count
        .checked_mul(size_of::<Wdt>())
        .ok_or(super::Error::Limit)?;
    if requested > limits.max_wdt_memory_bytes {
        return Err(super::Error::Limit.into());
    }
    session.run_reserve(requested)?;
    let mut wdts = Vec::new();
    wdts.try_reserve_exact(count)
        .map_err(|_| super::Error::Allocation)?;
    let descriptors = capacity(&wdts)?;
    if descriptors > limits.max_wdt_memory_bytes {
        return Err(super::Error::Limit.into());
    }
    session.run_reserve(
        descriptors
            .checked_sub(requested)
            .ok_or(super::Error::State)?,
    )?;
    let mut wdt_resident = descriptors;
    for map in maps.preload() {
        let cap = limits
            .max_file_bytes
            .min(limits.wdt.max_wdt_bytes)
            .min(limits.max_wdt_memory_bytes - wdt_resident);
        let bytes = session.run_read_source(InputRef::FileDataId(map.wdt_file_data_id()), cap)?;
        wdt_resident = wdt_resident
            .checked_add(bytes.capacity())
            .filter(|n| *n <= limits.max_wdt_memory_bytes)
            .ok_or(super::Error::Limit)?;
        wdts.push(Wdt {
            id: map.id(),
            bytes,
        });
    }
    let mut object_adts = 0;
    let mut model_reads = 0;
    let mut model_source_bytes = 0;
    for (map, state) in maps.preload().zip(&wdts) {
        let wdt = parse_wdt(&mut session, &state.bytes, limits)?;
        wdt.tiles(map).map_err(super::Error::Plan)?;
        project(
            &mut session,
            wdt.globals(),
            Context {
                map_id: map.id(),
                original_map_id: map.id(),
                kind: Kind::Global,
                replayed: false,
            },
            None,
            &mut model_reads,
            &mut model_source_bytes,
        )?;
        let resident = wdt.stats().resident_bytes;
        drop(wdt);
        session.run_release(resident)?;
    }
    for map in maps.processing() {
        let index = wdts
            .iter()
            .position(|s| s.id == map.id())
            .ok_or(super::Error::State)?;
        let wdt = parse_wdt(&mut session, &wdts[index].bytes, limits)?;
        let mut tiles = wdt.tiles(map).map_err(super::Error::Plan)?;
        let parent_index = map
            .parent_map_id()
            .and_then(|id| wdts.iter().position(|s| s.id == id));

        let mut names = [0; plan::MAX_NAME_BYTES];
        let mut cursor = 0u16;
        while let Some(tile) = tiles.next(&mut names) {
            let cell = u16::from(tile.x) * 64 + u16::from(tile.y);
            while cursor < cell {
                replay(&mut session, maps, map.id(), parent_index, cursor, limits)?;
                cursor += 1;
            }
            let target = match tile.target {
                plan::TileTarget::FileDataId(id) => InputRef::FileDataId(id),
                plan::TileTarget::Name(name) => InputRef::LogicalName(name),
            };
            let bytes = session.run_read_source(
                target,
                limits.max_file_bytes.min(limits.objects.max_input_bytes),
            )?;
            super::bump(&mut object_adts, 1)?;
            let objects = parse_objects(&mut session, &bytes, limits)?;
            project(
                &mut session,
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
                map.is_parent().then_some(index),
                &mut model_reads,
                &mut model_source_bytes,
            )?;
            let resident = objects.stats().resident_bytes;
            drop(objects);
            session.run_release(resident)?;
            let capacity = bytes.capacity();
            drop(bytes);
            session.run_release_source(capacity)?;
            cursor = cell + 1;
        }
        while cursor < 4096 {
            replay(&mut session, maps, map.id(), parent_index, cursor, limits)?;
            cursor += 1;
        }
        let resident = wdt.stats().resident_bytes;
        drop(wdt);
        session.run_release(resident)?;
    }
    session.verify_unchanged()?;
    let stage = session.sink();
    let work = stage.work;
    let cache_resident = stage.budget.live;
    let cache_peak = stage.budget.peak;
    drop(wdts);
    session.run_release_source(wdt_resident - descriptors)?;
    session.run_release(descriptors)?;
    let ids = session.finish()?.ids;
    let after = collector.facts()?;
    Ok(Report {
        maps: count,
        tile_positions: positions,
        object_adts,
        own_tiles: work.own,
        replayed_tiles: work.replays,
        empty_replays: work.empty_replays,
        cached_tiles: work.cached,
        placement_callbacks: work.placements,
        placement_records: work.records,
        placement_bytes: work.bytes,
        model_reads,
        model_source_bytes,
        model_raw_bytes: after.raw_bytes,
        wdt_resident_bytes: wdt_resident,
        cache_resident_bytes: cache_resident,
        cache_peak_bytes: cache_peak,
        ids,
        collector_before: before,
        collector_after: after,
        request_calls_before: calls_before,
        request_calls_after: collector.request_calls(),
    })
}

fn parse_wdt<'b, L: LiquidLookup + ?Sized, S: Sink>(
    s: &mut bridge::Session<'_, '_, '_, L, Stage<S>>,
    bytes: &'b [u8],
    limits: SharedLimits,
) -> Result<plan::Wdt<'b>, Error<S::Error>> {
    let allowance = limits.wdt.max_memory_bytes.min(s.run_free_bytes()?);
    s.run_reserve(allowance)?;
    let wdt = plan::Wdt::parse(
        bytes,
        plan::Limits {
            max_memory_bytes: allowance,
            ..limits.wdt
        },
    )
    .map_err(super::Error::Plan)?;
    s.run_release(
        allowance
            .checked_sub(wdt.stats().resident_bytes)
            .ok_or(super::Error::State)?,
    )?;
    Ok(wdt)
}
fn parse_objects<'b, L: LiquidLookup + ?Sized, S: Sink>(
    s: &mut bridge::Session<'_, '_, '_, L, Stage<S>>,
    bytes: &'b [u8],
    limits: SharedLimits,
) -> Result<Objects<'b>, Error<S::Error>> {
    let allowance = limits.objects.max_memory_bytes.min(s.run_free_bytes()?);
    s.run_reserve(allowance)?;
    let objects = Objects::parse_adt(
        bytes,
        objects::Limits {
            max_memory_bytes: allowance,
            ..limits.objects
        },
    )
    .map_err(super::Error::Objects)?;
    s.run_release(
        allowance
            .checked_sub(objects.stats().resident_bytes)
            .ok_or(super::Error::State)?,
    )?;
    Ok(objects)
}
fn project<L: LiquidLookup + ?Sized, S: Sink>(
    s: &mut bridge::Session<'_, '_, '_, L, Stage<S>>,
    objects: &Objects<'_>,
    context: Context,
    save_to: Option<usize>,
    reads: &mut u64,
    source_bytes: &mut u64,
) -> Result<(), Error<S::Error>> {
    let start = s.request_count()?;
    let before = s.run_facts()?;
    for r in objects.requests() {
        let target = match r.target() {
            Target::FileDataId(id) => InputRef::FileDataId(id),
            Target::Name(n) => InputRef::LogicalName(n),
        };
        s.extract(match r.source() {
            Source::M2(_) => ModelRequest::M2(target),
            Source::Wmo(_) => ModelRequest::Wmo(target),
        })?;
    }
    let after = s.run_facts()?;
    super::bump(
        reads,
        after
            .reads
            .checked_sub(before.reads)
            .ok_or(super::Error::State)? as u64,
    )?;
    super::bump(
        source_bytes,
        after
            .source_bytes
            .checked_sub(before.source_bytes)
            .ok_or(super::Error::State)? as u64,
    )?;
    s.run_with_sink(|stage| {
        stage.raw = after.raw_bytes;
        stage.check_output(0)?;
        stage.context = Some(context);
        stage.save_to = save_to;
        Ok(())
    })?;
    let end = s.request_count()?;
    s.project_to_sink(
        start..end,
        &[placements::Batch {
            objects,
            map_id: context.map_id,
            original_map_id: context.original_map_id,
        }],
    )?;
    Ok(())
}
fn replay<L: LiquidLookup + ?Sized, S: Sink>(
    s: &mut bridge::Session<'_, '_, '_, L, Stage<S>>,
    maps: &plan::Maps<'_>,
    map_id: u32,
    parent: Option<usize>,
    cell: u16,
    limits: SharedLimits,
) -> Result<(), Error<S::Error>> {
    // Preserve lazy admission: an inactive parent is irrelevant when every
    // tile is owned. Only an actually missing own tile requests fallback.
    let Some(parent_map) = maps.fallback_parent(map_id).map_err(super::Error::Plan)? else {
        return Ok(());
    };
    let parent = parent
        .filter(|&index| s.sink().maps[index].id == parent_map.id())
        .ok_or(super::Error::State)?;
    let entries = &s.sink().maps[parent].tiles;
    let Ok(at) = entries.binary_search_by_key(&cell, |t| t.cell) else {
        return Ok(());
    };
    let old = entries[at].value.resident_bytes();
    let allowance = limits
        .cache
        .max_memory_bytes
        .checked_sub(old)
        .ok_or(super::Error::Limit)?
        .min(s.run_free_bytes()?);
    s.run_reserve(allowance)?;
    s.run_with_sink(|stage| {
        let cached = &stage.maps[parent].tiles[at].value;
        let records = cached.record_count();
        let parent_id = stage.maps[parent].id;
        let bytes = cached
            .replay(
                map_id,
                parent_id,
                cache::Limits {
                    max_memory_bytes: old
                        .checked_add(allowance)
                        .ok_or(StageError::Run(super::Error::Limit))?,
                    ..limits.cache
                },
            )
            .map_err(|e| StageError::Run(super::Error::Placement(e)))?;
        let context = Context {
            map_id,
            original_map_id: parent_id,
            kind: Kind::Tile {
                x: (cell / 64) as u8,
                y: (cell % 64) as u8,
            },
            replayed: true,
        };
        stage.emit(context, &bytes, records)?;
        drop(bytes);
        super::bump(&mut stage.work.replays, 1).map_err(StageError::Run)?;
        if records == 0 {
            super::bump(&mut stage.work.empty_replays, 1).map_err(StageError::Run)?;
        }
        Ok(())
    })?;
    s.run_release(allowance)?;
    Ok(())
}
