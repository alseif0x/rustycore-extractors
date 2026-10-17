// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-17: consuming compatible world and authentic retained facts.
//! Owns the prepared compatible collector, one serial run, and genuine startup
//! completion plus available Map/complete liquid facts and owned topology.
//! Outputs remain provisional. Final VMap/staging/product/runtime are separate.
use super::{Prepared, available, plan, startup, strict_world};
use crate::collision::{
    inventory_spool::{self as inventory, Inventory},
    payload::{self, CallbackFailure, CollectorError},
    run::compatible_inventory as run,
};
use forever_vmap::wmo::liquid::LiquidLookup;
#[cfg(test)]
mod tests;
pub const PROFILE_70374: &str = "linux70374-compatible-disk-world-v1";
pub const PROFILE_70425: &str = "linux70425-compatible-disk-world-v1";
pub const PROFILE_GNU15_70374: &str = "linux70374-compatible-disk-world-gnu15-native-order-v2";
pub const PROFILE_GNU15_70425: &str = "linux70425-compatible-disk-world-gnu15-native-order-v2";
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cause<E> {
    World(strict_world::Error),
    Collector(CollectorError),
    Startup(startup::Error),
    Run(run::Error<E>),
    Inventory(inventory::Error),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Error<E> {
    cause: Cause<E>,
    guard: Option<CollectorError>,
    inventory_guards: [Option<inventory::Error>; 2],
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
    pub const fn inventory_guards(self) -> [Option<inventory::Error>; 2] {
        self.inventory_guards
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
        if self
            .inventory_guards
            .contains(&Some(inventory::Error::Spool(payload::Error::Changed)))
        {
            return payload::Error::Changed.code();
        }
        match self.cause {
            Cause::World(e) => e.code(),
            Cause::Collector(e) => e.code(),
            Cause::Startup(e) => e.code(),
            Cause::Run(e) => e.code(),
            Cause::Inventory(e) => e.code(),
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
impl<E> From<strict_world::Error> for Error<E> {
    fn from(e: strict_world::Error) -> Self {
        Self {
            cause: Cause::World(e),
            guard: None,
            inventory_guards: [None; 2],
        }
    }
}
impl<E> From<CollectorError> for Error<E> {
    fn from(e: CollectorError) -> Self {
        Self {
            cause: Cause::Collector(e),
            guard: None,
            inventory_guards: [None; 2],
        }
    }
}
impl<E> From<startup::Error> for Error<E> {
    fn from(e: startup::Error) -> Self {
        // Preparation can consume its collector before outer inventories close.
        // Retain its typed guard, including a source change reported as the
        // collector cause, so a later inventory Changed cannot mask it.
        let source_changed = CollectorError::Collision(crate::collision::Error::Store(
            crate::local_store::Error::InputChanged,
        ));
        let guard = if e.guard() == Some(source_changed)
            || matches!(e.cause(), available::spooled::Cause::Collector(c) if c == source_changed)
        {
            Some(source_changed)
        } else {
            e.guard()
        };
        Self {
            cause: Cause::Startup(e),
            guard,
            inventory_guards: [None; 2],
        }
    }
}
impl<E> From<run::Error<E>> for Error<E> {
    fn from(e: run::Error<E>) -> Self {
        Self {
            cause: Cause::Run(e),
            guard: None,
            inventory_guards: [None; 2],
        }
    }
}
impl<E> From<inventory::Error> for Error<E> {
    fn from(e: inventory::Error) -> Self {
        Self {
            cause: Cause::Inventory(e),
            guard: None,
            inventory_guards: [None; 2],
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub world: super::Limits,
    pub max_combined_memory_bytes: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            world: super::Limits::default(),
            max_combined_memory_bytes: run::MAX_COMBINED_MEMORY_BYTES,
        }
    }
}
struct Inventories {
    wdts: Inventory,
    caches: Inventory,
}
fn check_inventories<T, E: Copy>(
    owners: &mut Inventories,
    result: Result<T, Error<E>>,
) -> Result<T, Error<E>> {
    let guards = [
        owners.wdts.verify_unchanged().err(),
        owners.caches.verify_unchanged().err(),
    ];
    match result {
        Err(mut e) => {
            for (i, g) in guards.into_iter().enumerate() {
                if g.is_some() {
                    e.inventory_guards[i] = g;
                }
            }
            Err(e)
        }
        Ok(value) => match guards.into_iter().flatten().next() {
            Some(cause) => {
                drop(value);
                Err(Error {
                    cause: Cause::Inventory(cause),
                    guard: None,
                    inventory_guards: guards,
                })
            }
            None => Ok(value),
        },
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
// A failed pure operation still owns its user Sink. Run both guards once more
// AFTER dropping that owned state, preserving the original typed cause.
fn guarded<T, L: LiquidLookup + ?Sized, S: run::Sink>(
    context: &mut startup::Context<'_, '_, L>,
    sink: &mut Option<S>,
    owners: &mut Inventories,
    result: Result<T, Error<S::Error>>,
) -> Result<T, Error<S::Error>> {
    let checked = context
        .collector_mut()
        .finish_external_operation(result)
        .map_err(finish_error);
    match check_inventories(owners, checked) {
        Ok(value) => Ok(value),
        Err(e) => {
            drop(sink.take());
            let checked = check_inventories(owners, Err(e));
            context
                .collector_mut()
                .finish_external_operation(checked)
                .map_err(finish_error)
        }
    }
}
/// Genuine successful traversal owner. No constructor accepts caller ancestry,
/// model handles or table/source proofs. All fields derive from this consumed run.
pub struct Completed {
    startup: startup::CompletedStartup,
    report: run::Report,
    map: available::TableFacts,
    liquids: [strict_world::TableFacts; 2],
    lookups: strict_world::LookupStats,
    maps: plan::Stats,
    topology: strict_world::Topology,
    resident_bytes: usize,
    profile: &'static str,
}
impl Completed {
    pub const fn profile(&self) -> &'static str {
        self.profile
    }
    pub fn startup_mut(&mut self) -> &mut startup::CompletedStartup {
        &mut self.startup
    }
    pub const fn report(&self) -> &run::Report {
        &self.report
    }
    pub const fn map(&self) -> &available::TableFacts {
        &self.map
    }
    pub const fn liquids(&self) -> &[strict_world::TableFacts; 2] {
        &self.liquids
    }
    pub const fn lookup_stats(&self) -> strict_world::LookupStats {
        self.lookups
    }
    pub const fn map_stats(&self) -> plan::Stats {
        self.maps
    }
    pub const fn topology(&self) -> &strict_world::Topology {
        &self.topology
    }
    /// Complete retained collector/cache + startup Data + topology capacities.
    pub const fn resident_bytes(&self) -> usize {
        self.resident_bytes
    }
    pub fn extraction_mut(&mut self) -> &mut crate::collision::compatible::Completed {
        self.startup.extraction_mut()
    }
    pub fn verify_unchanged(&mut self) -> Result<(), CollectorError> {
        self.startup.extraction_mut().verify_unchanged()
    }
}
fn profile(target: crate::collision::target::Target, native: bool) -> &'static str {
    match (target, native) {
        (crate::collision::target::Target::Build70374, false) => PROFILE_70374,
        (crate::collision::target::Target::Build70425, false) => PROFILE_70425,
        (crate::collision::target::Target::Build70374, true) => PROFILE_GNU15_70374,
        (crate::collision::target::Target::Build70425, true) => PROFILE_GNU15_70425,
    }
}
/// Consume genuine startup and two fresh inventories. All-map traversal and
/// one Session retain the same collector/negative cache/native order/IDs.
pub fn extract<S: run::Sink>(
    startup: startup::Startup<'_>,
    wdts: Inventory,
    caches: Inventory,
    sink: S,
    sink_heap_allowance: usize,
    limits: Limits,
) -> Result<Completed, Error<S::Error>> {
    extract_selected(
        startup,
        wdts,
        caches,
        sink,
        sink_heap_allowance,
        plan::MapSelection::All,
        limits,
    )
}
#[allow(clippy::too_many_arguments)]
pub fn extract_selected<S: run::Sink>(
    mut startup: startup::Startup<'_>,
    wdts: Inventory,
    caches: Inventory,
    sink: S,
    sink_heap_allowance: usize,
    selection: plan::MapSelection,
    limits: Limits,
) -> Result<Completed, Error<S::Error>> {
    let mut owners = Inventories { wdts, caches };
    let mut sink = Some(sink);
    let result = run::admit_inventories::<_, S::Error>(
        startup.collector_mut(),
        &mut owners.wdts,
        &mut owners.caches,
        run::Limits {
            run: limits.world.run,
            max_combined_memory_bytes: limits.max_combined_memory_bytes,
        },
    )
    .map_err(Error::from);
    if let Err(e) = guarded(&mut startup, &mut sink, &mut owners, result) {
        // Owners really drop before the last source/payload guard.
        drop(owners);
        return startup
            .collector_mut()
            .finish_external_operation(Err(e))
            .map_err(finish_error);
    }
    let Some(sink) = sink.take() else {
        return Err(strict_world::Error::State.into());
    };
    let prepared =
        super::prepare_selected(startup, sink, sink_heap_allowance, selection, limits.world)
            .map_err(Error::from);
    match prepared {
        Ok(prepared) => extract_prepared(
            prepared,
            owners.wdts,
            owners.caches,
            limits.max_combined_memory_bytes,
        ),
        Err(e) => check_inventories(&mut owners, Err(e)),
    }
}
/// Consume a fresh authenticated preparation (no prior borrowed callback), without reacquiring DB2,
/// releasing its Sink reservation, or beginning a second shared run.
pub fn extract_prepared<S: run::Sink>(
    mut prepared: Prepared<'_, S>,
    wdts: Inventory,
    caches: Inventory,
    max_combined_memory_bytes: usize,
) -> Result<Completed, Error<S::Error>> {
    let mut owners = Inventories { wdts, caches };
    let Some(mut original) = prepared.context.take() else {
        drop(prepared.sink.take());
        return check_inventories(&mut owners, Err(strict_world::Error::State.into()));
    };
    let mut sink = prepared.sink.take();
    // Execute in a closure so EVERY early return destroys temporary/borrowed
    // owners before the outer inventory + collector error/Drop closure below.
    let result = (|| {
        if let Some(e) = prepared.failed {
            return Err(e.into());
        }
        if prepared.callback_used {
            return Err(strict_world::Error::State.into());
        }
        original.data()?;
        run::admit_inventories::<_, S::Error>(
            original.collector_mut(),
            &mut owners.wdts,
            &mut owners.caches,
            run::Limits {
                run: prepared.limits.run,
                max_combined_memory_bytes,
            },
        )?;
        let owner = prepared.owner.as_ref().ok_or(strict_world::Error::State)?;
        let topology = prepared
            .topology
            .as_ref()
            .ok_or(strict_world::Error::State)?;
        let lookup = prepared.lookup.as_ref().ok_or(strict_world::Error::State)?;
        let remaining = prepared
            .limits
            .max_table_memory_bytes
            .checked_sub(
                available::add(prepared.retained, topology.resident_bytes())
                    .map_err(startup::Error::from)?,
            )
            .ok_or(strict_world::Error::Limit)?;
        let budget = prepared.limits.maps.max_memory_bytes.min(remaining);
        original.collector_mut().reserve_external(budget)?;
        let maps = plan::Maps::from_materialized_selection(
            owner.rows(),
            owner.direct_records().len(),
            topology.selection(),
            plan::Limits {
                max_memory_bytes: budget,
                ..prepared.limits.maps
            },
        )
        .map_err(strict_world::Error::Plan)?;
        let graph_bytes = maps.stats().resident_bytes;
        original.collector_mut().release_external(
            budget
                .checked_sub(graph_bytes)
                .ok_or(strict_world::Error::State)?,
        )?;
        Ok((maps, graph_bytes, lookup))
    })();
    let (maps, graph_bytes, lookup) = match guarded(&mut original, &mut sink, &mut owners, result) {
        Ok(value) => value,
        Err(e) => {
            drop(prepared);
            drop(owners);
            return original
                .collector_mut()
                .finish_external_operation(Err(e))
                .map_err(finish_error);
        }
    };
    let target = original.collector_mut().target();
    let native = original.collector_mut().native_order();
    let mut context = match original.try_with_lookup(lookup) {
        payload::LookupTransition::Ready(context) => context,
        payload::LookupTransition::Failed(mut original, e) => {
            let error = guarded::<(), _, _>(&mut original, &mut sink, &mut owners, Err(e.into()))
                .unwrap_err();
            drop(maps);
            drop(prepared);
            drop(owners);
            return original
                .collector_mut()
                .finish_external_operation(Err(error))
                .map_err(finish_error);
        }
    };
    let result = match sink.take() {
        Some(sink) => run::extract_precharged(
            &mut context,
            &maps,
            &mut owners.wdts,
            &mut owners.caches,
            run::ReservedSink {
                sink,
                heap_allowance: prepared.sink_allowance,
                precharged: prepared.sink_allowance,
            },
            run::Limits {
                run: prepared.limits.run,
                max_combined_memory_bytes,
            },
        )
        .map_err(Error::from),
        None => Err(strict_world::Error::State.into()),
    };
    let report = match guarded(&mut context, &mut sink, &mut owners, result) {
        Ok(report) => report,
        Err(e) => {
            drop(maps);
            // lookup borrow requires recovering/destroying context first; its
            // collector guard already ran after Session/Sink destruction.
            drop(owners);
            return context
                .collector_mut()
                .finish_external_operation(Err(e))
                .map_err(finish_error);
        }
    };
    let mut context = match context.try_with_lookup(&startup::EMPTY) {
        payload::LookupTransition::Ready(context) => context,
        payload::LookupTransition::Failed(mut original, e) => {
            let error = guarded::<(), _, _>(&mut original, &mut sink, &mut owners, Err(e.into()))
                .unwrap_err();
            drop(maps);
            drop(owners);
            return original
                .collector_mut()
                .finish_external_operation(Err(error))
                .map_err(finish_error);
        }
    };
    drop(maps);
    drop(prepared.owner.take());
    drop(prepared.lookup.take());
    let result = context
        .collector_mut()
        .release_external(
            available::add(prepared.retained, graph_bytes).map_err(startup::Error::from)?,
        )
        .map_err(Error::from);
    let result = guarded(&mut context, &mut sink, &mut owners, result);
    drop(owners);
    context
        .collector_mut()
        .finish_external_operation(result)
        .map_err(finish_error)?;
    let resident_bytes = context.collector_mut().retained_bytes()?;
    let topology = prepared.topology.take().ok_or(strict_world::Error::State)?;
    let (startup, topology) = context.finish_with_topology(topology)?;
    let map = prepared.map.take().ok_or(strict_world::Error::State)?;
    let liquids = prepared.liquids.take().ok_or(strict_world::Error::State)?;
    Ok(Completed {
        startup,
        report,
        map,
        liquids,
        lookups: prepared.lookup_stats,
        maps: prepared.map_stats,
        topology,
        resident_bytes,
        profile: profile(target, native),
    })
}
