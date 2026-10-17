// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-17: compatible traversal derived from the strict disk runner.
//! Compatible same-owner serial world traversal with typed missing WDT/ADT.
//! One placement Session acquires physical requests, retains GNU15 order and
//! serial IDs, and stages successful OWN tiles including empty ones. Only an
//! immediate parent's OWN cache can replay; inherited tiles are never cached.
//! Every non-MissingFile error is fatal. No publication/final VMap is implied.
pub use super::spooled::Sink;
use super::{Context, Kind, SharedLimits};
use crate::collision::{
    Facts, InputRef,
    compatible::{Collector, startup},
    inventory_spool::{self as inventory, Inventory, Tile},
    payload::{self, CallbackFailure, CollectorError},
    placements::{self, compatible as bridge},
    plan,
};
use forever_vmap::{
    objects::{self, Objects},
    placement::{DirBin, ids},
    wmo::liquid::LiquidLookup,
};
pub const PROFILE: &str = "linux-private-compatible-disk-world-run-v1";
// Same existing two-inventory ceiling and component limits, no cap increase.
pub use super::inventory::{Limits, MAX_COMBINED_MEMORY_BYTES};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StageError<E> {
    Run(super::Error),
    Inventory(inventory::Error),
    Sink(E),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cause<E> {
    Run(super::Error),
    Collector(CollectorError),
    Inventory(inventory::Error),
    Bridge(bridge::Error<StageError<E>>),
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Guards {
    pub collector: Option<CollectorError>,
    pub wdt: Option<inventory::Error>,
    pub cache: Option<inventory::Error>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Error<E> {
    cause: Cause<E>,
    guards: Guards,
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
    pub const fn guards(self) -> Guards {
        self.guards
    }
    pub fn code(self) -> &'static str {
        if let Some(e) = self.guards.collector
            && changed(e)
        {
            return e.code();
        }
        if [self.guards.wdt, self.guards.cache]
            .contains(&Some(inventory::Error::Spool(payload::Error::Changed)))
        {
            return payload::Error::Changed.code();
        }
        match self.cause {
            Cause::Run(e) => e.code(),
            Cause::Collector(e) => e.code(),
            Cause::Inventory(e) => e.code(),
            Cause::Bridge(e) => {
                if e.guard().is_some_and(changed) {
                    return e.code();
                }
                match e.cause() {
                    bridge::Cause::Sink(StageError::Run(e)) => e.code(),
                    bridge::Cause::Sink(StageError::Inventory(e)) => e.code(),
                    bridge::Cause::Sink(StageError::Sink(_)) => "collision_disk_run_sink",
                    _ => e.code(),
                }
            }
        }
    }
    fn with_collector(mut self, guard: Option<CollectorError>) -> Self {
        if let Some(e) = guard
            && (changed(e) || !self.guards.collector.is_some_and(changed))
        {
            self.guards.collector = Some(e);
        }
        self
    }
}
impl<E> From<super::Error> for Error<E> {
    fn from(e: super::Error) -> Self {
        Self {
            cause: Cause::Run(e),
            guards: Guards::default(),
        }
    }
}
impl<E> From<CollectorError> for Error<E> {
    fn from(e: CollectorError) -> Self {
        Self {
            cause: Cause::Collector(e),
            guards: Guards::default(),
        }
    }
}
impl<E> From<inventory::Error> for Error<E> {
    fn from(e: inventory::Error) -> Self {
        Self {
            cause: Cause::Inventory(e),
            guards: Guards::default(),
        }
    }
}
impl<E> From<bridge::Error<StageError<E>>> for Error<E> {
    fn from(e: bridge::Error<StageError<E>>) -> Self {
        Self {
            cause: Cause::Bridge(e),
            guards: Guards::default(),
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
        CallbackFailure::Callback { cause, guard } => cause.with_collector(guard),
    }
}
fn wdt_error<E: Copy>(e: inventory::CallbackError<Error<E>>) -> Error<E> {
    let mut error = match e.cause {
        inventory::Cause::Access(e) => e.into(),
        inventory::Cause::Callback(e) => e,
    };
    if e.guard.is_some() {
        error.guards.wdt = e.guard;
    }
    error
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
    pub wdt_files: usize,
    pub missing_wdts: usize,
    pub missing_object_adts: u64,
    pub placements: bridge::Stats,
    pub placements_profile: &'static str,
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
    pub wdt: inventory::Stats,
    pub cache: inventory::Stats,
    /// Entire collector ceiling plus the two complete inventory heap ceilings.
    pub combined_reserved_bytes: usize,
    pub collector_before: Facts,
    pub collector_after: Facts,
    pub request_calls_before: usize,
    pub request_calls_after: usize,
}
struct Stage<'a, S: Sink> {
    sink: S,
    cache: &'a mut Inventory,
    context: Option<Context>,
    save_to: bool,
    output_limit: u64,
    raw: usize,
    work: Work,
}
impl<S: Sink> Stage<'_, S> {
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
}
impl<S: Sink> bridge::Sink for Stage<'_, S> {
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
            if self.save_to {
                self.cache
                    .insert_own_tile(
                        Tile {
                            map_id: context.map_id,
                            x,
                            y,
                        },
                        batch.bytes,
                    )
                    .map_err(StageError::Inventory)?;
                super::bump(&mut self.work.cached, 1).map_err(StageError::Run)?;
            }
        } else if self.save_to {
            return Err(StageError::Run(super::Error::State));
        }
        self.save_to = false;
        Ok(())
    }
}
/// Trusted world owner only. The exact owned Sink reservation is transferred
/// without a release gap; public callers cannot declare an unbacked charge.
pub(crate) struct ReservedSink<S> {
    pub sink: S,
    pub heap_allowance: usize,
    pub precharged: usize,
}
pub(crate) fn extract_precharged<L: LiquidLookup + ?Sized, S: Sink>(
    context: &mut startup::Context<'_, '_, L>,
    maps: &plan::Maps<'_>,
    wdts: &mut Inventory,
    caches: &mut Inventory,
    sink: ReservedSink<S>,
    limits: Limits,
) -> Result<Report, Error<S::Error>> {
    extract_collector_precharged(context.collector_mut(), maps, wdts, caches, sink, limits)
}
// Same execute/final inventory/source guards; no Context/source-fact cast.
fn extract_collector_precharged<L: LiquidLookup + ?Sized, S: Sink>(
    collector: &mut Collector<'_, '_, L>,
    maps: &plan::Maps<'_>,
    wdts: &mut Inventory,
    caches: &mut Inventory,
    sink: ReservedSink<S>,
    limits: Limits,
) -> Result<Report, Error<S::Error>> {
    let result = execute(collector, maps, wdts, caches, sink, limits);
    // execute has destroyed the owned Sink, including every early-error path.
    // Evaluate each independent inventory guard even when another already failed.
    let wdt_guard = wdts.verify_unchanged().err();
    let cache_guard = caches.verify_unchanged().err();
    let result = match result {
        Err(mut e) => {
            if wdt_guard.is_some() {
                e.guards.wdt = wdt_guard;
            }
            if cache_guard.is_some() {
                e.guards.cache = cache_guard;
            }
            Err(e)
        }
        Ok(report) => match wdt_guard.or(cache_guard) {
            Some(cause) => Err(Error {
                cause: Cause::Inventory(cause),
                guards: Guards {
                    collector: None,
                    wdt: wdt_guard,
                    cache: cache_guard,
                },
            }),
            None => Ok(report),
        },
    };
    collector
        .finish_external_operation(result)
        .map_err(finish_error)
}
fn execute<L: LiquidLookup + ?Sized, S: Sink>(
    collector: &mut Collector<'_, '_, L>,
    maps: &plan::Maps<'_>,
    wdts: &mut Inventory,
    caches: &mut Inventory,
    reservation: ReservedSink<S>,
    limits: Limits,
) -> Result<Report, Error<S::Error>> {
    // Preparation already admitted the one shared run; never begin/reset here.
    let ReservedSink {
        sink,
        heap_allowance: sink_heap_allowance,
        precharged,
    } = reservation;
    if precharged != 0 && precharged != sink_heap_allowance {
        return Err(super::Error::State.into());
    }
    let combined = admit_inventories::<_, S::Error>(collector, wdts, caches, limits)?;
    let limits = limits.run;
    let count = maps.preload().len();
    let positions = (count as u64)
        .checked_mul(4096)
        .ok_or(super::Error::Limit)?;
    if count > limits.max_maps || positions > limits.max_tile_positions {
        return Err(super::Error::Limit.into());
    }
    let before = collector.facts()?;
    let calls_before = collector.request_calls();
    let stage = Stage {
        sink,
        cache: caches,
        context: None,
        save_to: false,
        output_limit: limits.max_output_bytes,
        raw: before.raw_bytes,
        work: Work::default(),
    };
    let mut session = bridge::Session::new_precharged(
        collector,
        stage,
        sink_heap_allowance,
        precharged,
        limits.placements,
    )?;
    session.run_with_sink(|s| s.check_output(0))?;
    let active_bytes = count
        .checked_mul(size_of::<u32>())
        .ok_or(super::Error::Limit)?;
    session.run_reserve(active_bytes)?;
    let mut active = Vec::new();
    active
        .try_reserve_exact(count)
        .map_err(|_| super::Error::Allocation)?;
    let active_bytes_actual = active
        .capacity()
        .checked_mul(size_of::<u32>())
        .ok_or(super::Error::Limit)?;
    session.run_reserve(
        active_bytes_actual
            .checked_sub(active_bytes)
            .ok_or(super::Error::State)?,
    )?;
    for map in maps.preload() {
        let Some(bytes) = session.run_read_world_source(
            InputRef::FileDataId(map.wdt_file_data_id()),
            limits.max_file_bytes.min(limits.wdt.max_wdt_bytes),
        )?
        else {
            continue;
        };
        wdts.insert_wdt(map.id(), map.wdt_file_data_id(), &bytes)?;
        if active.len() == active.capacity() {
            return Err(super::Error::Limit.into());
        }
        active.push(map.id());
        let n = bytes.capacity();
        drop(bytes);
        session.run_release_source(n)?;
    }
    let mut object_adts = 0;
    let mut missing_object_adts = 0;
    let mut model_reads = 0;
    let mut model_source_bytes = 0;
    for map in maps.preload() {
        if !active.contains(&map.id()) {
            continue;
        }
        wdts.with_wdt(map.id(), map.wdt_file_data_id(), 0, |wdt| {
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
                false,
                &mut model_reads,
                &mut model_source_bytes,
            )
        })
        .map_err(wdt_error)?;
    }
    for map in maps.processing() {
        if !active.contains(&map.id()) {
            continue;
        }
        wdts.with_wdt(map.id(), map.wdt_file_data_id(), 0, |wdt| {
            let mut tiles = wdt.tiles(map).map_err(super::Error::Plan)?;
            let mut names = [0; plan::MAX_NAME_BYTES];
            let mut cursor = 0u16;
            while let Some(tile) = tiles.next(&mut names) {
                let cell = u16::from(tile.x) * 64 + u16::from(tile.y);
                replay_range(&mut session, maps, map.id(), cursor, cell, limits)?;
                let target = match tile.target {
                    plan::TileTarget::FileDataId(id) => InputRef::FileDataId(id),
                    plan::TileTarget::Name(name) => InputRef::LogicalName(name),
                };
                let Some(bytes) = session.run_read_world_source(
                    target,
                    limits.max_file_bytes.min(limits.objects.max_input_bytes),
                )?
                else {
                    super::bump(&mut missing_object_adts, 1)?;
                    // Typed own absence permits only this exact parent's OWN
                    // cache; no inherited cache and no recursive ancestor search.
                    replay_range(&mut session, maps, map.id(), cell, cell + 1, limits)?;
                    cursor = cell + 1;
                    continue;
                };
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
                    map.is_parent(),
                    &mut model_reads,
                    &mut model_source_bytes,
                )?;
                let n = objects.stats().resident_bytes;
                drop(objects);
                session.run_release(n)?;
                let n = bytes.capacity();
                drop(bytes);
                session.run_release_source(n)?;
                cursor = cell + 1;
            }
            replay_range(&mut session, maps, map.id(), cursor, 4096, limits)?;
            Ok(())
        })
        .map_err(wdt_error)?;
    }
    session.verify_unchanged()?;
    let work = session.sink().work;
    let cache_stats = session.sink().cache.stats()?;
    let wdt_stats = wdts.stats()?;
    let wdt_files = active.len();
    drop(active);
    session.run_release(active_bytes_actual)?;
    let placements_profile = session.profile();
    let placements = session.finish()?;
    let ids = placements.ids;
    let after = collector.facts()?;
    Ok(Report {
        maps: count,
        tile_positions: positions,
        object_adts,
        wdt_files,
        missing_wdts: count.checked_sub(wdt_files).ok_or(super::Error::State)?,
        missing_object_adts,
        placements,
        placements_profile,
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
        wdt_resident_bytes: wdt_stats.reserved_heap_bytes,
        cache_resident_bytes: cache_stats.reserved_heap_bytes,
        cache_peak_bytes: cache_stats.peak_admitted_heap_bytes,
        wdt: wdt_stats,
        cache: cache_stats,
        combined_reserved_bytes: combined,
        ids,
        collector_before: before,
        collector_after: after,
        request_calls_before: calls_before,
        request_calls_after: collector.request_calls(),
    })
}
fn parse_objects<'b, L: LiquidLookup + ?Sized, S: Sink>(
    s: &mut bridge::Session<'_, '_, '_, L, Stage<'_, S>>,
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
    s: &mut bridge::Session<'_, '_, '_, L, Stage<'_, S>>,
    objects: &Objects<'_>,
    context: Context,
    save_to: bool,
    reads: &mut u64,
    source_bytes: &mut u64,
) -> Result<(), Error<S::Error>> {
    let before = s.run_facts()?;
    s.run_with_sink(|stage| {
        stage.context = Some(context);
        stage.save_to = save_to;
        Ok(())
    })?;
    s.run_project_to_sink(
        &[placements::Batch {
            objects,
            map_id: context.map_id,
            original_map_id: context.original_map_id,
        }],
        |stage, facts| {
            stage.raw = facts.raw_bytes;
            stage.check_output(0)
        },
    )?;
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
        stage.check_output(0)
    })?;
    Ok(())
}

fn replay_range<L: LiquidLookup + ?Sized, S: Sink>(
    s: &mut bridge::Session<'_, '_, '_, L, Stage<'_, S>>,
    maps: &plan::Maps<'_>,
    map_id: u32,
    mut first: u16,
    end: u16,
    limits: SharedLimits,
) -> Result<(), Error<S::Error>> {
    if first > end || end > 4096 {
        return Err(super::Error::State.into());
    }
    if first == end {
        return Ok(());
    }
    // Preserve lazy inactive-parent failure at the first actually absent cell.
    let Some(parent) = maps.fallback_parent(map_id).map_err(super::Error::Plan)? else {
        return Ok(());
    };
    loop {
        let next = s.run_with_sink(|stage| {
            stage
                .cache
                .next_own_tile(parent.id(), first)
                .map_err(StageError::Inventory)
        })?;
        let Some(tile) = next else {
            return Ok(());
        };
        let cell = u16::from(tile.x) * 64 + u16::from(tile.y);
        if cell >= end {
            return Ok(());
        }
        // Returned Vec storage is owned by the stage: its full allowed output
        // capacity is separately charged to the collector before reload.
        let output_allowance = limits.cache.records.max_bytes;
        s.run_reserve(output_allowance)?;
        s.run_with_sink(|stage| {
            let bytes = stage
                .cache
                .replay_own_tile(tile, map_id)
                .map_err(StageError::Inventory)?;
            let records = DirBin::parse(&bytes, limits.cache.records)
                .map_err(|e| StageError::Run(super::Error::Placement(e)))?
                .record_count();
            stage.emit(
                Context {
                    map_id,
                    original_map_id: parent.id(),
                    kind: Kind::Tile {
                        x: tile.x,
                        y: tile.y,
                    },
                    replayed: true,
                },
                &bytes,
                records,
            )?;
            drop(bytes);
            super::bump(&mut stage.work.replays, 1).map_err(StageError::Run)?;
            if records == 0 {
                super::bump(&mut stage.work.empty_replays, 1).map_err(StageError::Run)?;
            }
            Ok(())
        })?;
        s.run_release(output_allowance)?;
        first = cell + 1;
        if first == end {
            return Ok(());
        }
    }
}

/// Full disjoint ceiling admission before closed DB2 preparation or run IO.
/// Keeps owner limits and fresh-state validation identical in both callers.
pub(crate) fn admit_inventories<L: LiquidLookup + ?Sized, E>(
    collector: &Collector<'_, '_, L>,
    wdts: &mut Inventory,
    caches: &mut Inventory,
    limits: Limits,
) -> Result<usize, Error<E>> {
    limits.run.validate_resources(collector.resources())?;
    if wdts.resources() != collector.resources()
        || caches.resources() != collector.resources()
        || wdts.backend() != collector.backend()
        || caches.backend() != collector.backend()
    {
        return Err(super::Error::InvalidLimits.into());
    }
    if limits.max_combined_memory_bytes > MAX_COMBINED_MEMORY_BYTES {
        return Err(super::Error::InvalidLimits.into());
    }
    let combined = collector
        .memory_ceiling()
        .checked_add(wdts.limits().max_heap_bytes)
        .and_then(|n| n.checked_add(caches.limits().max_heap_bytes))
        .ok_or(super::Error::Limit)?;
    if combined > limits.max_combined_memory_bytes {
        return Err(super::Error::Limit.into());
    }
    if wdts.limits().max_heap_bytes > limits.run.max_wdt_memory_bytes
        || caches.limits().max_heap_bytes > limits.run.max_cache_memory_bytes
        || wdts.limits().wdt != limits.run.wdt
        || caches.limits().cache != limits.run.cache
    {
        return Err(super::Error::InvalidLimits.into());
    }
    if wdts.stats()?.entries != 0 || caches.stats()?.entries != 0 {
        return Err(super::Error::State.into());
    }
    wdts.verify_unchanged()?;
    caches.verify_unchanged()?;
    Ok(combined)
}
