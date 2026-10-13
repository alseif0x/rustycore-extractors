// SPDX-License-Identifier: GPL-3.0-or-later
// Modified 2026-10-12: authenticated world with disjoint typed WDT/cache disk owners; derived from authored spooled world.
//! Complete Map/liquid reads and same closed preparation as the resident world.
//! Owned Sink reservation spans preparation, transitions and the shared runner.
//! Completed owns Data/topology with their reservations retained for RAW reloads.
//! No product publication, full-world, native-CASC or real-client acceptance.
use super::{LookupStats, Prepared, TableFacts, Topology, add, capacity, input, plan};
use crate::collision::{
    gameobjects,
    inventory_spool::{self as inventory, Inventory},
    payload::{self, CallbackFailure, CollectorError},
    run,
};
use forever_vmap::wmo::liquid::LiquidLookup;
#[cfg(test)]
mod tests;
pub const PROFILE: &str = "linux-private-authenticated-disk-world-v1";
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cause<E> {
    World(super::Error),
    Collector(CollectorError),
    Startup(gameobjects::spooled::Error),
    Run(run::inventory::Error<E>),
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
impl<E> From<super::Error> for Error<E> {
    fn from(e: super::Error) -> Self {
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
impl<E> From<gameobjects::spooled::Error> for Error<E> {
    fn from(e: gameobjects::spooled::Error) -> Self {
        Self {
            cause: Cause::Startup(e),
            guard: None,
            inventory_guards: [None; 2],
        }
    }
}
impl<E> From<run::inventory::Error<E>> for Error<E> {
    fn from(e: run::inventory::Error<E>) -> Self {
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
            max_combined_memory_bytes: run::inventory::MAX_COMBINED_MEMORY_BYTES,
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
impl<E> From<input::Failure<CollectorError>> for Error<E> {
    fn from(e: input::Failure<CollectorError>) -> Self {
        match e {
            input::Failure::World(e) => e.into(),
            input::Failure::Collector(e) => e.into(),
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
// A failed pure operation still owns its user Sink. Run both guards once more
// AFTER dropping that owned state, preserving the original typed cause.
fn guarded<T, L: LiquidLookup + ?Sized, S: run::inventory::Sink>(
    context: &mut gameobjects::spooled::Context<'_, '_, L>,
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
fn transition<
    's,
    'l,
    'n,
    L: LiquidLookup + ?Sized,
    N: LiquidLookup + ?Sized,
    S: run::inventory::Sink,
>(
    context: gameobjects::spooled::Context<'s, 'l, L>,
    lookup: &'n N,
    sink: &mut Option<S>,
    owners: &mut Inventories,
) -> Result<gameobjects::spooled::Context<'s, 'n, N>, Error<S::Error>> {
    match context.try_with_lookup(lookup) {
        payload::LookupTransition::Ready(value) => Ok(value),
        payload::LookupTransition::Failed(mut original, e) => {
            guarded(&mut original, sink, owners, Err(e.into()))
        }
    }
}
pub struct Completed {
    startup: gameobjects::spooled::Completed,
    report: run::inventory::Report,
    tables: [TableFacts; 3],
    lookups: LookupStats,
    maps: plan::Stats,
    topology: Topology,
    resident_bytes: usize,
}
impl Completed {
    pub const fn startup(&self) -> &gameobjects::spooled::Completed {
        &self.startup
    }
    pub const fn report(&self) -> &run::inventory::Report {
        &self.report
    }
    pub const fn tables(&self) -> &[TableFacts; 3] {
        &self.tables
    }
    pub const fn lookup_stats(&self) -> LookupStats {
        self.lookups
    }
    pub const fn map_stats(&self) -> plan::Stats {
        self.maps
    }
    pub const fn topology(&self) -> &Topology {
        &self.topology
    }
    /// Owned compact collector/repository, Data and topology capacities. Source
    /// residency, stack, allocator overhead and peak work storage are separate.
    pub const fn resident_bytes(&self) -> usize {
        self.resident_bytes
    }
    pub fn extraction_mut(&mut self) -> &mut payload::SpoolingExtraction {
        self.startup.extraction_mut()
    }
}
/// Consumes successful GameObject startup and a bounded owned provisional Sink.
/// The Sink's heap allowance is charged before any Map/liquid preparation and
/// transferred intact into the runner; no release gap or double reservation.
/// Failure exposes no Completed. WDT/cache and cumulative policy limits remain.
pub fn extract<S: run::inventory::Sink>(
    startup: gameobjects::spooled::Startup<'_>,
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
/// Complete graph validation with acquisition restricted to one active direct
/// map and its effective ancestors. Global GameObject startup is unchanged.
/// One serial Session spans this domain; its IDs are not full-world IDs.
#[allow(clippy::too_many_arguments)]
pub fn extract_selected<S: run::inventory::Sink>(
    mut startup: gameobjects::spooled::Startup<'_>,
    wdts: Inventory,
    caches: Inventory,
    sink: S,
    sink_heap_allowance: usize,
    selection: plan::MapSelection,
    limits: Limits,
) -> Result<Completed, Error<S::Error>> {
    let combined = limits.max_combined_memory_bytes;
    let limits = limits.world;
    let mut owners = Inventories { wdts, caches };
    let mut sink = Some(sink);
    let result = (|| {
        limits.validate_resources(startup.collector_mut().resources())?;
        run::inventory::admit_inventories::<_, S::Error>(
            startup.collector_mut(),
            &mut owners.wdts,
            &mut owners.caches,
            run::inventory::Limits {
                run: limits.run,
                max_combined_memory_bytes: combined,
            },
        )
        .map_err(Error::from)?;
        startup
            .collector_mut()
            .reserve_external(sink_heap_allowance)?;
        input::prepare(startup.collector_mut(), limits).map_err(Error::from)
    })();
    let Prepared {
        rows,
        direct_count,
        lookup,
        tables,
        retained,
    } = guarded(&mut startup, &mut sink, &mut owners, result)?;
    let result = (|| {
        let graph_budget = limits.maps.max_memory_bytes.min(
            limits
                .max_table_memory_bytes
                .checked_sub(retained)
                .ok_or(super::Error::Limit)?,
        );
        startup.collector_mut().reserve_external(graph_budget)?;
        let maps = plan::Maps::from_materialized_selection(
            &rows,
            direct_count,
            selection,
            plan::Limits {
                max_memory_bytes: graph_budget,
                ..limits.maps
            },
        )
        .map_err(super::Error::Plan)?;
        let resident = maps.stats().resident_bytes;
        if resident > graph_budget || add(retained, resident)? > limits.max_table_memory_bytes {
            return Err(super::Error::Limit.into());
        }
        startup
            .collector_mut()
            .release_external(graph_budget - resident)?;
        Ok(maps)
    })();
    let maps = guarded(&mut startup, &mut sink, &mut owners, result)?;
    let map_stats = maps.stats();
    let result = (|| {
        let remaining = limits
            .max_table_memory_bytes
            .checked_sub(add(retained, map_stats.resident_bytes)?)
            .ok_or(super::Error::Limit)?;
        input::topology(&maps, startup.collector_mut(), remaining).map_err(Error::from)
    })();
    let topology = guarded(&mut startup, &mut sink, &mut owners, result)?;
    let result = (|| {
        Ok(LookupStats {
            materials: lookup.materials.len(),
            types: lookup.types.len(),
            retained_bytes: add(capacity(&lookup.materials)?, capacity(&lookup.types)?)?,
        })
    })();
    let lookup_stats = guarded(&mut startup, &mut sink, &mut owners, result)?;
    let mut context = transition(startup, &lookup, &mut sink, &mut owners)?;
    let result = match sink.take() {
        Some(sink) => run::inventory::extract_precharged(
            &mut context,
            &maps,
            &mut owners.wdts,
            &mut owners.caches,
            run::inventory::ReservedSink {
                sink,
                heap_allowance: sink_heap_allowance,
                precharged: sink_heap_allowance,
            },
            run::inventory::Limits {
                run: limits.run,
                max_combined_memory_bytes: combined,
            },
        )
        .map_err(Error::from),
        None => Err(super::Error::State.into()),
    };
    let report = guarded(&mut context, &mut sink, &mut owners, result)?;
    let mut context = transition(context, &super::EMPTY, &mut sink, &mut owners)?;
    drop(maps);
    drop(rows);
    drop(lookup);
    let result = (|| {
        context
            .collector_mut()
            .release_external(add(retained, map_stats.resident_bytes)?)?;
        // Data + topology remain charged. The consuming finish validates the
        // exact sum and moves both owners with the completed payload repository.
        context
            .collector_mut()
            .retained_bytes()
            .map_err(Error::from)
    })();
    let resident_bytes = guarded(&mut context, &mut sink, &mut owners, result)?;
    let (startup, topology) = context
        .finish_with_topology(topology)
        .map_err(Error::from)?;
    Ok(Completed {
        startup,
        report,
        tables,
        lookups: lookup_stats,
        maps: map_stats,
        topology,
        resident_bytes,
    })
}
