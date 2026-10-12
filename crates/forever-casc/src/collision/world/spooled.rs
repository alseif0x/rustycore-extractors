// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-12: authenticated world startup with one disk-backed collector.
//! Complete Map/liquid reads and same closed preparation as the resident world.
//! Owned Sink reservation spans preparation, transitions and the shared runner.
//! Completed owns Data/topology with their reservations retained for RAW reloads.
//! No product publication, full-world, native-CASC or real-client acceptance.
use super::{Limits, LookupStats, Prepared, TableFacts, Topology, add, capacity, input, plan};
use crate::collision::{
    gameobjects,
    payload::{self, CallbackFailure, CollectorError, SpoolingExtractor},
    run,
};
use forever_vmap::wmo::liquid::LiquidLookup;
#[cfg(test)]
mod tests;
pub const PROFILE: &str = "linux-private-authenticated-spooled-world-v1";
impl<L: LiquidLookup + ?Sized> input::Access for SpoolingExtractor<'_, '_, L> {
    type Error = CollectorError;
    fn read_source(
        &mut self,
        id: u32,
        cap: usize,
    ) -> Result<(Vec<u8>, crate::collision::SourceFacts), Self::Error> {
        SpoolingExtractor::read_source(self, id, cap)
    }
    fn reserve_external(&mut self, n: usize) -> Result<(), Self::Error> {
        SpoolingExtractor::reserve_external(self, n)
    }
    fn release_external(&mut self, n: usize) -> Result<(), Self::Error> {
        SpoolingExtractor::release_external(self, n)
    }
    fn release_source(&mut self, n: usize) -> Result<(), Self::Error> {
        SpoolingExtractor::release_source(self, n)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cause<E> {
    World(super::Error),
    Collector(CollectorError),
    Startup(gameobjects::spooled::Error),
    Run(run::spooled::Error<E>),
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
            Cause::World(e) => e.code(),
            Cause::Collector(e) => e.code(),
            Cause::Startup(e) => e.code(),
            Cause::Run(e) => e.code(),
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
impl<E> From<gameobjects::spooled::Error> for Error<E> {
    fn from(e: gameobjects::spooled::Error) -> Self {
        Self {
            cause: Cause::Startup(e),
            guard: None,
        }
    }
}
impl<E> From<run::spooled::Error<E>> for Error<E> {
    fn from(e: run::spooled::Error<E>) -> Self {
        Self {
            cause: Cause::Run(e),
            guard: None,
        }
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
fn guarded<T, L: LiquidLookup + ?Sized, S: run::spooled::Sink>(
    context: &mut gameobjects::spooled::Context<'_, '_, L>,
    sink: &mut Option<S>,
    result: Result<T, Error<S::Error>>,
) -> Result<T, Error<S::Error>> {
    match context
        .collector_mut()
        .finish_external_operation(result)
        .map_err(finish_error)
    {
        Ok(value) => Ok(value),
        Err(e) => {
            drop(sink.take());
            context
                .collector_mut()
                .finish_external_operation(Err(e))
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
    S: run::spooled::Sink,
>(
    context: gameobjects::spooled::Context<'s, 'l, L>,
    lookup: &'n N,
    sink: &mut Option<S>,
) -> Result<gameobjects::spooled::Context<'s, 'n, N>, Error<S::Error>> {
    match context.try_with_lookup(lookup) {
        payload::LookupTransition::Ready(value) => Ok(value),
        payload::LookupTransition::Failed(mut original, e) => {
            guarded(&mut original, sink, Err(e.into()))
        }
    }
}
pub struct Completed {
    startup: gameobjects::spooled::Completed,
    report: run::spooled::Report,
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
    pub const fn report(&self) -> &run::spooled::Report {
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
pub fn extract<S: run::spooled::Sink>(
    mut startup: gameobjects::spooled::Startup<'_>,
    sink: S,
    cache_heap_allowance: usize,
    sink_heap_allowance: usize,
    limits: Limits,
) -> Result<Completed, Error<S::Error>> {
    let mut sink = Some(sink);
    let result = (|| {
        limits.validate()?;
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
    } = guarded(&mut startup, &mut sink, result)?;
    let result = (|| {
        let graph_budget = limits.maps.max_memory_bytes.min(
            limits
                .max_table_memory_bytes
                .checked_sub(retained)
                .ok_or(super::Error::Limit)?,
        );
        startup.collector_mut().reserve_external(graph_budget)?;
        let maps = plan::Maps::from_materialized(
            &rows,
            direct_count,
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
    let maps = guarded(&mut startup, &mut sink, result)?;
    let map_stats = maps.stats();
    let result = (|| {
        let remaining = limits
            .max_table_memory_bytes
            .checked_sub(add(retained, map_stats.resident_bytes)?)
            .ok_or(super::Error::Limit)?;
        input::topology(&maps, startup.collector_mut(), remaining).map_err(Error::from)
    })();
    let topology = guarded(&mut startup, &mut sink, result)?;
    let result = (|| {
        Ok(LookupStats {
            materials: lookup.materials.len(),
            types: lookup.types.len(),
            retained_bytes: add(capacity(&lookup.materials)?, capacity(&lookup.types)?)?,
        })
    })();
    let lookup_stats = guarded(&mut startup, &mut sink, result)?;
    let mut context = transition(startup, &lookup, &mut sink)?;
    let result = match sink.take() {
        Some(sink) => run::spooled::extract_precharged(
            &mut context,
            &maps,
            sink,
            cache_heap_allowance,
            sink_heap_allowance,
            sink_heap_allowance,
            limits.run,
        )
        .map_err(Error::from),
        None => Err(super::Error::State.into()),
    };
    let report = guarded(&mut context, &mut sink, result)?;
    let mut context = transition(context, &super::EMPTY, &mut sink)?;
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
    let resident_bytes = guarded(&mut context, &mut sink, result)?;
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
