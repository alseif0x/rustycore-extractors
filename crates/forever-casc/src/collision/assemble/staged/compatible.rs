// SPDX-License-Identifier: GPL-3.0-or-later
// Modified by the RustyCore Extractors project, 2026-10-17.
// Authored 2026-10-17: genuine compatible world placement-stage owner.
//! Separate compatible acquisition, sharing the private RCEPLC1 stage kernel.
//! No caller placements/completed report can manufacture this owner. Available
//! Map coverage remains distinct from complete liquid/model SourceFacts. No VMO,
//! final VMap, package, product or real/native acceptance is implied.
// Keep failure reporting allocation-free and retain every typed guard result.
#![expect(
    clippy::result_large_err,
    reason = "Preserve allocation-free Copy errors with typed causes and independent source/spool guards"
)]

use super::{Guards, Stage, StageError, Stats, add};
use crate::collision::{
    compatible::{startup, world::inventory as world},
    inventory_spool::{self as inventory, Inventory},
    payload::{self, CollectorError},
    plan, run, spool,
};
use forever_vmap::placement::{self, DirBin};
#[cfg(test)]
mod tests;
pub const PROFILE: &str = "linux-private-compatible-authentic-placement-stage-v1";
pub const COVERAGE_PROFILE: &str = "linux-private-compatible-authentic-placement-stage-coverage-v1";
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub world: world::Limits,
    pub max_heap_bytes: usize,
    pub max_callbacks: usize,
    pub max_records: usize,
    pub records: placement::Limits,
}
impl Default for Limits {
    fn default() -> Self {
        let old = super::Limits::default();
        Self {
            world: world::Limits::default(),
            max_heap_bytes: old.max_heap_bytes,
            max_callbacks: old.max_callbacks,
            max_records: old.max_records,
            records: old.records,
        }
    }
}
impl Limits {
    // Numeric limits only: no cast of strict/available/genuine owner or facts.
    fn stage(self) -> super::Limits {
        super::Limits {
            world: crate::collision::world::inventory::Limits {
                world: self.world.world,
                max_combined_memory_bytes: self.world.max_combined_memory_bytes,
            },
            max_heap_bytes: self.max_heap_bytes,
            max_callbacks: self.max_callbacks,
            max_records: self.max_records,
            records: self.records,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cause {
    Stage(StageError),
    World(world::Error<StageError>),
    Collector(CollectorError),
    Startup(startup::Error),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Error {
    pub cause: Cause,
    pub guards: Guards,
}
fn source(e: Option<CollectorError>) -> bool {
    e == Some(CollectorError::Collision(crate::collision::Error::Store(
        crate::local_store::Error::InputChanged,
    )))
}
fn startup_source(e: startup::Error) -> bool {
    source(e.guard())
        || matches!(e.cause(), crate::collision::available_70374::spooled::Cause::Collector(c) if source(Some(c)))
}
fn run_source(e: run::compatible_inventory::Error<StageError>) -> bool {
    if source(e.guards().collector) {
        return true;
    }
    match e.cause() {
        run::compatible_inventory::Cause::Collector(c) => source(Some(c)),
        run::compatible_inventory::Cause::Bridge(e) => {
            source(e.guard())
                || matches!(e.cause(), crate::collision::placements::compatible::Cause::Collector(c) if source(Some(c)))
        }
        _ => false,
    }
}
fn world_source(e: world::Error<StageError>) -> bool {
    if source(e.guard()) {
        return true;
    }
    match e.cause() {
        world::Cause::Collector(c) => source(Some(c)),
        world::Cause::Startup(e) => startup_source(e),
        world::Cause::Run(e) => run_source(e),
        _ => false,
    }
}
fn collector_spool(e: Option<CollectorError>) -> bool {
    e == Some(CollectorError::Payload(payload::Error::Changed))
}
fn startup_spool(e: startup::Error) -> bool {
    collector_spool(e.guard())
        || matches!(e.cause(), crate::collision::available_70374::spooled::Cause::Collector(c) if collector_spool(Some(c)))
}
fn run_spool(e: run::compatible_inventory::Error<StageError>) -> bool {
    let g = e.guards();
    if collector_spool(g.collector)
        || [g.wdt, g.cache].contains(&Some(inventory::Error::Spool(payload::Error::Changed)))
    {
        return true;
    }
    match e.cause() {
        run::compatible_inventory::Cause::Collector(c) => collector_spool(Some(c)),
        run::compatible_inventory::Cause::Inventory(e) => {
            e == inventory::Error::Spool(payload::Error::Changed)
        }
        run::compatible_inventory::Cause::Bridge(e) => {
            collector_spool(e.guard())
                || match e.cause() {
                    crate::collision::placements::compatible::Cause::Collector(c) => {
                        collector_spool(Some(c))
                    }
                    crate::collision::placements::compatible::Cause::Sink(
                        run::compatible_inventory::StageError::Inventory(e),
                    ) => e == inventory::Error::Spool(payload::Error::Changed),
                    crate::collision::placements::compatible::Cause::Sink(
                        run::compatible_inventory::StageError::Sink(e),
                    ) => e == StageError::Spool(payload::Error::Changed),
                    _ => false,
                }
        }
        _ => false,
    }
}
fn world_spool(e: world::Error<StageError>) -> bool {
    if collector_spool(e.guard())
        || e.inventory_guards()
            .contains(&Some(inventory::Error::Spool(payload::Error::Changed)))
    {
        return true;
    }
    match e.cause() {
        world::Cause::Collector(c) => collector_spool(Some(c)),
        world::Cause::Startup(e) => startup_spool(e),
        world::Cause::Run(e) => run_spool(e),
        world::Cause::Inventory(e) => e == inventory::Error::Spool(payload::Error::Changed),
        _ => false,
    }
}
impl Error {
    pub fn code(self) -> &'static str {
        // Typed observed source change wins even if a later outer spool changes.
        if source(self.guards.collector)
            || match self.cause {
                Cause::Collector(e) => source(Some(e)),
                Cause::Startup(e) => startup_source(e),
                Cause::World(e) => world_source(e),
                Cause::Stage(_) => false,
            }
        {
            return crate::local_store::Error::InputChanged.code();
        }
        if match self.cause {
            Cause::Collector(e) => collector_spool(Some(e)),
            Cause::Startup(e) => startup_spool(e),
            Cause::World(e) => world_spool(e),
            Cause::Stage(e) => e == StageError::Spool(payload::Error::Changed),
        } || self.guards.collector == Some(CollectorError::Payload(payload::Error::Changed))
            || [self.guards.wdt, self.guards.cache]
                .contains(&Some(inventory::Error::Spool(payload::Error::Changed)))
            || self.guards.stage == Some(StageError::Spool(payload::Error::Changed))
        {
            return payload::Error::Changed.code();
        }
        if let Some(e) = self.guards.stage {
            return e.code();
        }
        match self.cause {
            Cause::Stage(e) => e.code(),
            Cause::World(e) => e.code(),
            Cause::Collector(e) => e.code(),
            Cause::Startup(e) => e.code(),
        }
    }
}
impl From<StageError> for Error {
    fn from(e: StageError) -> Self {
        Self {
            cause: Cause::Stage(e),
            guards: Guards::default(),
        }
    }
}
impl From<CollectorError> for Error {
    fn from(e: CollectorError) -> Self {
        Self {
            cause: Cause::Collector(e),
            guards: Guards::default(),
        }
    }
}
impl From<startup::Error> for Error {
    fn from(e: startup::Error) -> Self {
        Self {
            cause: Cause::Startup(e),
            guards: Guards {
                collector: e.guard(),
                ..Default::default()
            },
        }
    }
}
impl From<world::Error<StageError>> for Error {
    fn from(e: world::Error<StageError>) -> Self {
        let [wdt, cache] = e.inventory_guards();
        Self {
            cause: Cause::World(e),
            guards: Guards {
                collector: e.guard(),
                wdt,
                cache,
                stage: None,
            },
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
pub enum AccessCause<E> {
    Access(Cause),
    Callback(E),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AccessError<E> {
    pub cause: AccessCause<E>,
    pub guards: Guards,
}
impl<E: Copy> AccessError<E> {
    pub fn code(self) -> &'static str {
        Error {
            cause: match self.cause {
                AccessCause::Access(e) => e,
                AccessCause::Callback(_) => Cause::Stage(StageError::Callback),
            },
            guards: self.guards,
        }
        .code()
    }
}
// Only this private Capture is ever installed in the genuine consuming world.
// Full stage heap was already partitioned; callback allowance is therefore0.
struct Capture<'a> {
    stage: &'a mut Stage,
    sealed: &'a mut bool,
}
impl run::compatible_inventory::Sink for Capture<'_> {
    type Error = StageError;
    fn placements(&mut self, context: run::Context, bytes: &[u8]) -> Result<(), StageError> {
        self.stage.append(context, bytes)
    }
}
fn seal(capture: &mut Capture<'_>, active: &mut [u32]) -> Result<(), StageError> {
    let result = (|| {
        capture.stage.check()?;
        if *capture.sealed {
            return Err(StageError::State);
        }
        // This hook runs AFTER all acquisition/placement/replay/ID calls. The
        // same already-charged Vec is no longer traversed after the hook; sort
        // only this ephemeral domain index in place, with no new capacity.
        active.sort_unstable();
        if active.windows(2).any(|v| v[0] == v[1]) {
            return Err(StageError::Binding);
        }
        let globals = capture
            .stage
            .entries
            .iter()
            .filter(|e| matches!(e.context.kind, run::Kind::Global))
            .map(|e| e.context.map_id);
        if !globals.eq(active.iter().copied()) {
            return Err(StageError::Binding);
        }
        Ok(())
    })();
    capture.stage.finish(result)?;
    *capture.sealed = true;
    Ok(())
}
fn validate(stage: &mut Stage, sealed: bool, world: &world::Completed) -> Result<(), StageError> {
    let result = (|| {
        stage.check()?;
        let r = world.report();
        if !sealed
            || stage.stats()?.maps != r.wdt_files
            || r.maps != world.topology().selected_active().count()
            || r.maps.checked_sub(r.wdt_files) != Some(r.missing_wdts)
        {
            return Err(StageError::Binding);
        }
        for entry in &stage.entries {
            let map = world
                .topology()
                .get(entry.context.map_id)
                .filter(|map| map.active() && world.topology().contains_selected(map.id()))
                .ok_or(StageError::Binding)?;
            if entry.context.replayed && map.parent_map_id() != Some(entry.context.original_map_id)
            {
                return Err(StageError::Binding);
            }
            if stage.entries[stage.range(entry.context.map_id)?.start]
                .key()
                .1
                != 0
            {
                return Err(StageError::Binding);
            }
        }
        Ok(())
    })();
    stage.finish(result)
}
/// Genuine compatible world plus the SAME privately captured placement archive.
/// No constructor accepts a report, completed owner or caller placement bytes.
pub struct Staged {
    world: world::Completed,
    stage: Stage,
}
impl Staged {
    pub const fn profile(&self) -> &'static str {
        if self.world.has_coverage() {
            COVERAGE_PROFILE
        } else {
            PROFILE
        }
    }
    pub const fn world(&self) -> &world::Completed {
        &self.world
    }
    pub fn stats(&self) -> Result<Stats, Error> {
        self.world.startup_view().data_view()?;
        self.stage.failed.map_or(Ok(()), |e| Err(Error::from(e)))?;
        self.stage.stats().map_err(Error::from)
    }
    pub fn maps(&self) -> Result<impl Iterator<Item = u32> + '_, Error> {
        self.stats()?;
        Ok(self
            .stage
            .entries
            .iter()
            .filter(|e| matches!(e.context.kind, run::Kind::Global))
            .map(|e| e.context.map_id))
    }
    fn guards(&mut self) -> Guards {
        Guards {
            collector: self.world.verify_unchanged().err(),
            stage: self.stage.check().err(),
            ..Default::default()
        }
    }
    pub fn verify_unchanged(&mut self) -> Result<(), Error> {
        let guards = self.guards();
        match guards
            .collector
            .map(Cause::Collector)
            .or_else(|| guards.stage.map(Cause::Stage))
        {
            None => Ok(()),
            Some(cause) => Err(Error { cause, guards }),
        }
    }
    /// Borrow genuine traversal coverage, retaining all placement/source guards.
    pub fn with_coverage<T, E: Copy>(
        &mut self,
        callback_heap: usize,
        call: impl FnOnce(run::compatible_inventory::coverage::View<'_>) -> Result<T, E>,
    ) -> Result<T, AccessError<E>> {
        if let Err(e) = self.verify_unchanged() {
            drop(call);
            return Err(AccessError {
                cause: AccessCause::Access(e.cause),
                guards: self.guards(),
            });
        }
        // Stage storage is already partitioned from the collector ceiling; the
        // collector's retained ledger admits the extra caller heap exactly once.
        let result = self.world.with_coverage(callback_heap, call);
        let result = match result {
            Ok(value) => Ok(value),
            Err(run::compatible_inventory::coverage::Error {
                cause: run::compatible_inventory::coverage::Cause::Collector(e),
                ..
            }) => Err(AccessCause::Access(Cause::Collector(e))),
            Err(run::compatible_inventory::coverage::Error {
                cause: run::compatible_inventory::coverage::Cause::Callback(cause),
                guard,
            }) => {
                if let Some(e) = guard {
                    return self.coverage_failure(AccessCause::Callback(cause), Some(e));
                }
                Err(AccessCause::Callback(cause))
            }
        };
        let mut guards = self.guards();
        let cause = match result {
            Ok(value) => {
                if guards == Guards::default() {
                    return Ok(value);
                }
                drop(value);
                guards = self.guards();
                AccessCause::Access(
                    guards
                        .collector
                        .map(Cause::Collector)
                        .unwrap_or_else(|| Cause::Stage(guards.stage.unwrap_or(StageError::State))),
                )
            }
            Err(e) => e,
        };
        self.stage.failed.get_or_insert(StageError::Callback);
        // Include any Drop mutation before the final independent guards.
        let final_guards = self.guards();
        if final_guards.collector.is_some() {
            guards.collector = final_guards.collector;
        }
        if final_guards.stage.is_some() {
            guards.stage = final_guards.stage;
        }
        Err(AccessError { cause, guards })
    }
    fn coverage_failure<T, E: Copy>(
        &mut self,
        cause: AccessCause<E>,
        collector: Option<CollectorError>,
    ) -> Result<T, AccessError<E>> {
        self.stage.failed.get_or_insert(StageError::Callback);
        let mut guards = self.guards();
        if source(collector) || guards.collector.is_none() {
            guards.collector = collector;
        }
        Err(AccessError { cause, guards })
    }
    /// One authenticated complete map, internal buffer and callback captures
    /// dropped before the final independent source+stage guards. Error poisons.
    pub fn with_map<T, E: Copy>(
        &mut self,
        map: u32,
        callback_heap_allowance: usize,
        call: impl FnOnce(&DirBin<'_>) -> Result<T, E>,
    ) -> Result<T, AccessError<E>> {
        let setup = self.verify_unchanged().and_then(|()| {
            self.stage
                .read_map(map, callback_heap_allowance)
                .map_err(Error::from)
        });
        let bytes = match setup {
            Ok(bytes) => bytes,
            Err(e) => {
                drop(call);
                return Err(AccessError {
                    cause: AccessCause::Access(e.cause),
                    guards: self.guards(),
                });
            }
        };
        let result = {
            let view =
                DirBin::parse(&bytes, self.stage.limits.records).expect("complete verified map");
            call(&view)
        };
        drop(bytes);
        let mut guards = self.guards();
        let cause = match result {
            Err(e) => AccessCause::Callback(e),
            Ok(value) => {
                if guards.collector.is_none() && guards.stage.is_none() {
                    return Ok(value);
                }
                drop(value);
                guards = self.guards();
                AccessCause::Access(
                    guards
                        .collector
                        .map(Cause::Collector)
                        .unwrap_or_else(|| Cause::Stage(guards.stage.unwrap_or(StageError::State))),
                )
            }
        };
        if self.stage.failed.is_none() {
            self.stage.failed = Some(StageError::Callback);
        }
        guards = self.guards();
        Err(AccessError { cause, guards })
    }
}
fn input_guards(
    startup: &mut startup::Startup<'_>,
    wdts: &mut Inventory,
    caches: &mut Inventory,
    repository: &mut spool::Spool,
) -> Guards {
    Guards {
        collector: startup.collector_mut().verify_unchanged().err(),
        wdt: wdts.verify_unchanged().err(),
        cache: caches.verify_unchanged().err(),
        stage: repository.verify_unchanged().err().map(StageError::Spool),
    }
}
pub fn extract(
    startup: startup::Startup<'_>,
    wdts: Inventory,
    caches: Inventory,
    repository: spool::Spool,
    limits: Limits,
) -> Result<Staged, Error> {
    extract_selected(
        startup,
        wdts,
        caches,
        repository,
        plan::MapSelection::All,
        limits,
    )
}
/// Original selector: no coverage allocation or additional retained charge.
pub fn extract_selected(
    startup: startup::Startup<'_>,
    wdts: Inventory,
    caches: Inventory,
    repository: spool::Spool,
    selection: plan::MapSelection,
    limits: Limits,
) -> Result<Staged, Error> {
    extract_selected_inner(startup, wdts, caches, repository, selection, limits, false)
}
/// Explicit genuine coverage capture under the existing collector ceiling.
pub fn extract_selected_with_coverage(
    startup: startup::Startup<'_>,
    wdts: Inventory,
    caches: Inventory,
    repository: spool::Spool,
    selection: plan::MapSelection,
    limits: Limits,
) -> Result<Staged, Error> {
    extract_selected_inner(startup, wdts, caches, repository, selection, limits, true)
}
#[allow(clippy::too_many_arguments)]
fn extract_selected_inner(
    mut startup: startup::Startup<'_>,
    mut wdts: Inventory,
    mut caches: Inventory,
    mut repository: spool::Spool,
    selection: plan::MapSelection,
    limits: Limits,
    capture_coverage: bool,
) -> Result<Staged, Error> {
    let admitted: Result<usize, Error> = (|| {
        if repository.resources() != startup.collector_mut().resources()
            || wdts.resources() != repository.resources()
            || caches.resources() != repository.resources()
            || repository.backend() != startup.collector_mut().backend()
            || wdts.backend() != repository.backend()
            || caches.backend() != repository.backend()
            || limits.max_heap_bytes > super::MAX_HEAP_BYTES
            || limits.max_callbacks > repository.resources().stage_limits().max_callbacks
            || limits.max_records > repository.resources().stage_limits().max_records
            || limits.world.max_combined_memory_bytes
                > run::compatible_inventory::MAX_COMBINED_MEMORY_BYTES
        {
            return Err(StageError::InvalidLimits.into());
        }
        DirBin::parse(&[], limits.records).map_err(StageError::Placement)?;
        let combined = limits
            .world
            .max_combined_memory_bytes
            .checked_sub(limits.max_heap_bytes)
            .ok_or(StageError::Limit)?;
        let available = combined
            .checked_sub(add(
                wdts.limits().max_heap_bytes,
                caches.limits().max_heap_bytes,
            )?)
            .ok_or(StageError::Limit)?;
        startup
            .collector_mut()
            .constrain_memory_for_stage(available)?;
        Ok(combined)
    })();
    let combined = match admitted {
        Ok(value) => value,
        Err(mut error) => {
            error.guards = input_guards(&mut startup, &mut wdts, &mut caches, &mut repository);
            return Err(error);
        }
    };
    let mut stage = match Stage::new(repository, limits.stage()) {
        Ok(stage) => stage,
        Err(e) => {
            return Err(Error {
                cause: Cause::Stage(e),
                guards: Guards {
                    collector: startup.collector_mut().verify_unchanged().err(),
                    wdt: wdts.verify_unchanged().err(),
                    cache: caches.verify_unchanged().err(),
                    stage: None,
                },
            });
        }
    };
    let mut sealed = false;
    let extract = if capture_coverage {
        world::extract_selected_sealed_with_coverage
    } else {
        world::extract_selected_sealed
    };
    let result = extract(
        startup,
        wdts,
        caches,
        Capture {
            stage: &mut stage,
            sealed: &mut sealed,
        },
        0,
        selection,
        world::Limits {
            max_combined_memory_bytes: combined,
            ..limits.world
        },
        seal,
    );
    let mut world = match result {
        Ok(world) => world,
        Err(e) => {
            let mut error = Error::from(e);
            error.guards.stage = stage.check().err();
            return Err(error);
        }
    };
    let result = validate(&mut stage, sealed, &world);
    let guards = Guards {
        collector: world.verify_unchanged().err(),
        stage: stage.check().err(),
        ..Default::default()
    };
    if let Err(e) = result {
        return Err(Error {
            cause: Cause::Stage(e),
            guards,
        });
    }
    if let Some(cause) = guards
        .collector
        .map(Cause::Collector)
        .or_else(|| guards.stage.map(Cause::Stage))
    {
        return Err(Error { cause, guards });
    }
    Ok(Staged { world, stage })
}

// Authored 2026-10-17: the second and final private model Phase implementation.
// Its private fields stay in this module; models receives only closed borrows.
impl super::models::Phase for Staged {
    type Cause = Cause;
    type Error = Error;
    fn archive(&self) -> &Stage {
        &self.stage
    }
    fn archive_mut(&mut self) -> &mut Stage {
        &mut self.stage
    }
    fn phase_guards(&mut self) -> Guards {
        self.guards()
    }
    fn verify_unchanged(&mut self) -> Result<(), Self::Error> {
        Staged::verify_unchanged(self)
    }
    fn parts(e: Self::Error) -> (Self::Cause, Guards) {
        debug_assert!(e.guards.wdt.is_none() && e.guards.cache.is_none());
        (e.cause, e.guards)
    }
    fn stage_error(e: StageError) -> Self::Error {
        e.into()
    }
    fn compact(&self) -> Result<payload::CompactView<'_>, CollectorError> {
        self.world.startup_view().model_view()?.phase_compact()
    }
    fn shape(
        &self,
        h: crate::collision::ModelHandle,
    ) -> Result<payload::PayloadShape, CollectorError> {
        self.world.startup_view().model_view()?.payload_shape(h)
    }
    fn resident_bytes(&self) -> Result<usize, CollectorError> {
        self.world
            .startup_view()
            .model_view()?
            .phase_resident_bytes()
    }
    fn lists(&self) -> Result<&forever_vmap::gameobject_models::Lists, Self::Error> {
        self.world
            .startup_view()
            .data_view()
            .map(|d| d.lists())
            .map_err(Error::from)
    }
    fn with_raw_phase<T, E: Copy>(
        &mut self,
        h: crate::collision::ModelHandle,
        source: usize,
        callback: usize,
        call: impl FnOnce(Option<&[u8]>) -> Result<T, E>,
    ) -> Result<T, payload::CallbackFailure<E>> {
        self.world
            .extraction_mut()
            .with_payload_phase(h, source, callback, |p| call(p.raw()))
    }
}
// Preserve typed nested source priority when an outer VMO archive also changes.
// A code-string comparison would collapse distinct phase causes and is avoided.
pub(super) fn model_source_changed(cause: Cause, collector: Option<CollectorError>) -> bool {
    source(collector)
        || match cause {
            Cause::Collector(e) => source(Some(e)),
            Cause::Startup(e) => startup_source(e),
            Cause::World(e) => world_source(e),
            Cause::Stage(_) => false,
        }
}
