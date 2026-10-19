// SPDX-License-Identifier: GPL-3.0-or-later
// Modified by the RustyCore Extractors project, 2026-10-19.
// Authored 2026-10-17: same-owner compatible Map/liquid preparation.
//! Available Map coverage is separate from complete liquid SourceFacts. One
//! compatible collector/cache/ledger survives startup and the whole callback.
//! These preparation functions perform no traversal. The separate consuming
//! inventory owner traverses the world; final VMaps/product/runtime are separate.
#[cfg(test)]
use super::placements;
use super::{Collector, Target, startup};
use crate::collision::{
    Facts,
    available_70374::{self as available, AvailableAccess},
    payload::{self, CallbackFailure, CollectorError},
    plan,
    spool::BackendProfile,
    world::{self as strict_world, LookupStats, Topology, input},
};
use forever_db2::availability::MaterializedVMapMaps;
use forever_vmap::wmo::liquid::LiquidLookup;

pub mod inventory;
#[cfg(test)]
mod tests;
pub type Error = startup::Error;
pub type Limits = strict_world::Limits;
pub const PROFILE_70374: &str = "linux70374-disabled-key-compatible-world-preparation-v1";
pub const PROFILE_70425: &str = "linux70425-disabled-key-compatible-world-preparation-v1";
pub const PROFILE_70450: &str = "linux70450-disabled-key-compatible-world-preparation-v1";
pub const PROFILE_GNU15_70374: &str =
    "linux70374-disabled-key-compatible-world-preparation-gnu15-native-order-v2";
pub const PROFILE_GNU15_70425: &str =
    "linux70425-disabled-key-compatible-world-preparation-gnu15-native-order-v2";
pub const PROFILE_GNU15_70450: &str =
    "linux70450-disabled-key-compatible-world-preparation-gnu15-native-order-v2";

impl<L: LiquidLookup + ?Sized> input::Access for Collector<'_, '_, L> {
    type Error = CollectorError;
    fn read_source(
        &mut self,
        id: u32,
        cap: usize,
    ) -> Result<(Vec<u8>, crate::collision::SourceFacts), Self::Error> {
        Collector::read_source(self, id, cap)
    }
    fn reserve_external(&mut self, bytes: usize) -> Result<(), Self::Error> {
        Collector::reserve_external(self, bytes)
    }
    fn release_external(&mut self, bytes: usize) -> Result<(), Self::Error> {
        Collector::release_external(self, bytes)
    }
    fn release_source(&mut self, bytes: usize) -> Result<(), Self::Error> {
        Collector::release_source(self, bytes)
    }
}
impl<L: LiquidLookup + ?Sized> AvailableAccess for Collector<'_, '_, L> {
    fn read_available_source_for_target(
        &mut self,
        id: u32,
        cap: usize,
        target: Target,
    ) -> Result<crate::local_store::availability::AvailableSource, CollectorError> {
        let result = self
            .inner
            .inner
            .read_available_source_target(id, cap, target);
        self.inner.checked(result)
    }
}
fn world_error(e: strict_world::Error) -> Error {
    available::Error::World(e).into()
}
fn input_error(e: input::Failure<CollectorError>) -> Error {
    match e {
        input::Failure::World(e) => world_error(e),
        input::Failure::Collector(e) => e.into(),
    }
}
fn prepare_error(e: available::PrepareFailure<CollectorError>) -> Error {
    match e {
        available::PrepareFailure::Available(e) => e.into(),
        available::PrepareFailure::Collector(e) => e.into(),
    }
}
fn bridge(e: CallbackFailure<Error>) -> Error {
    match e {
        CallbackFailure::Collector(e) => e.into(),
        CallbackFailure::Callback { cause, guard } => cause.with_guard(guard),
    }
}
fn guarded<T, L: LiquidLookup + ?Sized, S>(
    context: &mut startup::Context<'_, '_, L>,
    sink: &mut Option<S>,
    result: Result<T, Error>,
) -> Result<T, Error> {
    match context
        .collector_mut()
        .finish_external_operation(result)
        .map_err(bridge)
    {
        Ok(value) => Ok(value),
        Err(error) => {
            // A Sink destructor can mutate the source/spool: observe again
            // after the actual owned Sink has been destroyed, even if poisoned.
            drop(sink.take());
            context
                .collector_mut()
                .finish_external_operation::<T, Error>(Err(error))
                .map_err(bridge)
        }
    }
}
// FnOnce captures are owned caller state too. An uninvoked capture may mutate
// source/spool on Drop, so destroy it before the final guards on early failure.
fn drop_callback_and_guard<T, L: LiquidLookup + ?Sized, S, C>(
    context: &mut startup::Context<'_, '_, L>,
    sink: &mut Option<S>,
    call: &mut Option<C>,
    error: Error,
) -> Result<T, Error> {
    drop(call.take());
    guarded(context, sink, Err(error))
}
fn profile(target: Target, native: bool) -> &'static str {
    match (target, native) {
        (Target::Build70374, false) => PROFILE_70374,
        (Target::Build70425, false) => PROFILE_70425,
        (Target::Build70450, false) => PROFILE_70450,
        (Target::Build70374, true) => PROFILE_GNU15_70374,
        (Target::Build70425, true) => PROFILE_GNU15_70425,
        (Target::Build70450, true) => PROFILE_GNU15_70450,
    }
}
/// Owns authentic startup, available Map materialization, complete liquids,
/// topology and the provisional Sink. Retained capacities stay charged while
/// live, including the declared bound for ALL Sink-owned heap and growth.
/// The callback may span the entire serial world: it never resets the collector
/// or its model/negative cache, handles, target/order or source counters. One
/// serial placement Session within that callback can retain IDs across all maps.
/// No public constructor accepts caller Map rows, topology or source proofs.
pub struct Prepared<'s, S> {
    context: Option<startup::Startup<'s>>,
    owner: Option<MaterializedVMapMaps>,
    lookup: Option<strict_world::Lookups>,
    sink: Option<S>,
    topology: Option<Topology>,
    map: Option<available::TableFacts>,
    liquids: Option<[strict_world::TableFacts; 2]>,
    lookup_stats: LookupStats,
    map_stats: plan::Stats,
    retained: usize,
    sink_allowance: usize,
    limits: Limits,
    profile: &'static str,
    failed: Option<Error>,
    // A borrowed callback can own arbitrary IDs/staging. It cannot later be
    // relabelled as the authentic one-Session consuming traversal.
    callback_used: bool,
}
impl<'s, S> Prepared<'s, S> {
    fn ready(&self) -> Result<&startup::Startup<'s>, Error> {
        if let Some(e) = self.failed {
            return Err(e);
        }
        let context = self.context.as_ref().ok_or(available::Error::State)?;
        context.data()?;
        Ok(context)
    }
    pub const fn profile(&self) -> &'static str {
        self.profile
    }
    pub fn startup(&self) -> Result<&startup::Data, Error> {
        self.ready()?.data()
    }
    pub fn map(&self) -> Result<&available::TableFacts, Error> {
        self.ready()?;
        self.map
            .as_ref()
            .ok_or_else(|| available::Error::State.into())
    }
    pub fn liquids(&self) -> Result<&[strict_world::TableFacts; 2], Error> {
        self.ready()?;
        self.liquids
            .as_ref()
            .ok_or_else(|| available::Error::State.into())
    }
    pub fn topology(&self) -> Result<&Topology, Error> {
        self.ready()?;
        self.topology
            .as_ref()
            .ok_or_else(|| available::Error::State.into())
    }
    pub fn lookup_stats(&self) -> Result<LookupStats, Error> {
        self.ready()?;
        Ok(self.lookup_stats)
    }
    pub fn map_stats(&self) -> Result<plan::Stats, Error> {
        self.ready()?;
        Ok(self.map_stats)
    }
    pub fn facts(&mut self) -> Result<Facts, Error> {
        self.ready()?;
        self.context
            .as_mut()
            .ok_or(available::Error::State)?
            .collector_mut()
            .facts()
            .map_err(Error::from)
    }
    pub fn retained_bytes(&mut self) -> Result<usize, Error> {
        self.ready()?;
        self.context
            .as_mut()
            .ok_or(available::Error::State)?
            .collector_mut()
            .retained_bytes()
            .map_err(Error::from)
    }
    pub fn memory_ceiling(&mut self) -> Result<usize, Error> {
        self.ready()?;
        Ok(self
            .context
            .as_mut()
            .ok_or(available::Error::State)?
            .collector_mut()
            .memory_ceiling())
    }
    pub fn backend(&mut self) -> Result<BackendProfile, Error> {
        self.ready()?;
        Ok(self
            .context
            .as_mut()
            .ok_or(available::Error::State)?
            .collector_mut()
            .backend())
    }
    pub fn verify_unchanged(&mut self) -> Result<(), Error> {
        self.ready()?;
        let context = self.context.as_mut().ok_or(available::Error::State)?;
        let result = guarded(context, &mut self.sink, Ok(()));
        if let Err(e) = result {
            self.failed = Some(e);
        }
        result
    }
    /// One whole-run callback can own one serial placement Session across every
    /// map/tile. It borrows the internally acquired lookup and validated graph.
    /// Callback-owned heap/source buffers must remain within the precharged Sink
    /// allowance or an independently checked collector reservation (for example
    /// a placement Session). These counters are not a bound on arbitrary Rust
    /// callback allocations or RSS. All output remains provisional.
    /// Returning an error poisons this owner;
    /// failed transition/drop guards take priority and no retry is possible.
    /// After any borrowed callback, `finish_preparation` remains available but
    /// consuming inventory traversal rejects with world State: a callback may
    /// already have used its own ID Session. Traversal requires fresh preparation.
    pub fn with_context<T>(
        &mut self,
        call: impl FnOnce(
            &mut startup::Context<'s, '_, dyn LiquidLookup>,
            &plan::Maps<'_>,
            &Topology,
            &mut S,
        ) -> Result<T, Error>,
    ) -> Result<T, Error> {
        let mut call = Some(call);
        if let Err(error) = self.ready() {
            drop(call.take());
            if let Some(context) = self.context.as_mut() {
                let error = guarded::<(), _, _>(context, &mut self.sink, Err(error)).unwrap_err();
                self.failed = Some(error);
                return Err(error);
            }
            return Err(error);
        }
        self.callback_used = true;
        let mut original = self.context.take().ok_or(available::Error::State)?;
        let (Some(owner), Some(topology)) = (self.owner.as_ref(), self.topology.as_ref()) else {
            let error = drop_callback_and_guard::<(), _, _, _>(
                &mut original,
                &mut self.sink,
                &mut call,
                available::Error::State.into(),
            )
            .unwrap_err();
            self.failed = Some(error);
            return Err(error);
        };
        let result = (|| {
            let remaining = self
                .limits
                .max_table_memory_bytes
                .checked_sub(available::add(self.retained, topology.resident_bytes())?)
                .ok_or(available::Error::Limit)?;
            let budget = self.limits.maps.max_memory_bytes.min(remaining);
            original.collector_mut().reserve_external(budget)?;
            let maps = plan::Maps::from_materialized_selection(
                owner.rows(),
                owner.direct_records().len(),
                topology.selection(),
                plan::Limits {
                    max_memory_bytes: budget,
                    ..self.limits.maps
                },
            )
            .map_err(available::Error::Plan)?;
            let actual = maps.stats().resident_bytes;
            original
                .collector_mut()
                .release_external(budget.checked_sub(actual).ok_or(available::Error::State)?)?;
            Ok((maps, actual))
        })();
        let (maps, graph_bytes) = match guarded(&mut original, &mut self.sink, result) {
            Ok(value) => value,
            Err(e) => {
                let error = drop_callback_and_guard::<(), _, _, _>(
                    &mut original,
                    &mut self.sink,
                    &mut call,
                    e,
                )
                .unwrap_err();
                self.failed = Some(error);
                return Err(error);
            }
        };
        let Some(lookup) = self.lookup.as_ref() else {
            drop(maps);
            let error = drop_callback_and_guard::<(), _, _, _>(
                &mut original,
                &mut self.sink,
                &mut call,
                available::Error::State.into(),
            )
            .unwrap_err();
            self.failed = Some(error);
            return Err(error);
        };
        let lookup: &dyn LiquidLookup = lookup;
        let mut context = match original.try_with_lookup(lookup) {
            payload::LookupTransition::Ready(value) => value,
            payload::LookupTransition::Failed(mut original, e) => {
                let error = drop_callback_and_guard::<(), _, _, _>(
                    &mut original,
                    &mut self.sink,
                    &mut call,
                    e.into(),
                )
                .unwrap_err();
                self.failed = Some(error);
                return Err(error);
            }
        };
        let result = match (self.sink.as_mut(), call.take()) {
            (Some(sink), Some(call)) => call(&mut context, &maps, topology, sink),
            (_, uncalled) => {
                drop(uncalled);
                Err(available::Error::State.into())
            }
        };
        let value = match guarded(&mut context, &mut self.sink, result) {
            Ok(value) => value,
            Err(e) => {
                self.failed = Some(e);
                return Err(e);
            }
        };
        drop(maps);
        let result = context
            .collector_mut()
            .release_external(graph_bytes)
            .map_err(Error::from);
        if let Err(e) = guarded(&mut context, &mut self.sink, result) {
            drop(value);
            let error = guarded::<(), _, _>(&mut context, &mut self.sink, Err(e)).unwrap_err();
            self.failed = Some(error);
            return Err(error);
        }
        match context.try_with_lookup(&startup::EMPTY) {
            payload::LookupTransition::Ready(context) => {
                self.context = Some(context);
                Ok(value)
            }
            payload::LookupTransition::Failed(mut original, e) => {
                drop(value);
                let error =
                    guarded::<(), _, _>(&mut original, &mut self.sink, Err(e.into())).unwrap_err();
                self.failed = Some(error);
                Err(error)
            }
        }
    }
    /// Finish the authenticated preparation/model owner only. This returns
    /// genuine startup extraction and its topology, NOT a Completed world or a
    /// declaration that any traversal, cache replay or output was performed.
    /// All temporary table/lookup/Sink owners drop BEFORE their exact release.
    pub fn finish_preparation(mut self) -> Result<(startup::CompletedStartup, Topology), Error> {
        self.ready()?;
        let mut context = self.context.take().ok_or(available::Error::State)?;
        drop(self.sink.take());
        guarded(&mut context, &mut self.sink, Ok(()))?;
        drop(self.owner.take());
        drop(self.lookup.take());
        let result = context
            .collector_mut()
            .release_external(available::add(self.retained, self.sink_allowance)?)
            .map_err(Error::from);
        guarded(&mut context, &mut self.sink, result)?;
        let topology = self.topology.take().ok_or(available::Error::State)?;
        context.finish_with_topology(topology)
    }
}
impl<S> Drop for Prepared<'_, S> {
    fn drop(&mut self) {
        drop(self.sink.take());
        if let Some(context) = self.context.as_mut() {
            let _ = context
                .collector_mut()
                .finish_external_operation::<(), Error>(Err(available::Error::State.into()));
        }
    }
}
/// Prepare all available Map rows and complete liquids; perform no WDT/ADT IO.
/// Sink allowance bounds its actual capacities and growth for its whole lifetime.
pub fn prepare<'s, S>(
    context: startup::Startup<'s>,
    sink: S,
    sink_heap_allowance: usize,
    limits: Limits,
) -> Result<Prepared<'s, S>, Error> {
    prepare_selected(
        context,
        sink,
        sink_heap_allowance,
        plan::MapSelection::All,
        limits,
    )
}
pub fn prepare_selected<'s, S>(
    mut context: startup::Startup<'s>,
    sink: S,
    sink_heap_allowance: usize,
    selection: plan::MapSelection,
    limits: Limits,
) -> Result<Prepared<'s, S>, Error> {
    let mut sink = Some(sink);
    let target = context.collector_mut().target();
    let profile = profile(target, context.collector_mut().native_order());
    let result = (|| {
        strict_world::inventory::validate_resources(limits, context.collector_mut().resources())
            .map_err(world_error)?;
        if sink_heap_allowance > limits.run.placements.max_memory_bytes {
            return Err(world_error(strict_world::Error::Limit));
        }
        context
            .collector_mut()
            .reserve_external(sink_heap_allowance)?;
        context.collector_mut().begin_shared_run()?;
        available::prepare_available_target(context.collector_mut(), limits, target)
            .map_err(prepare_error)
    })();
    let available::Prepared {
        owner,
        mut map,
        lookup,
        liquids,
        retained,
    } = guarded(&mut context, &mut sink, result)?;
    let result = (|| {
        let budget = limits.maps.max_memory_bytes.min(
            limits
                .max_table_memory_bytes
                .checked_sub(retained)
                .ok_or(available::Error::Limit)?,
        );
        context.collector_mut().reserve_external(budget)?;
        let maps = plan::Maps::from_materialized_selection(
            owner.rows(),
            owner.direct_records().len(),
            selection,
            plan::Limits {
                max_memory_bytes: budget,
                ..limits.maps
            },
        )
        .map_err(available::Error::Plan)?;
        let actual = maps.stats().resident_bytes;
        context
            .collector_mut()
            .release_external(budget.checked_sub(actual).ok_or(available::Error::State)?)?;
        Ok(maps)
    })();
    let maps = guarded(&mut context, &mut sink, result)?;
    let map_stats = maps.stats();
    let result = (|| {
        let remaining = limits
            .max_table_memory_bytes
            .checked_sub(available::add(retained, map_stats.resident_bytes)?)
            .ok_or(available::Error::Limit)?;
        input::topology(&maps, context.collector_mut(), remaining).map_err(input_error)
    })();
    let topology = guarded(&mut context, &mut sink, result)?;
    let result = (|| {
        Ok(LookupStats {
            materials: lookup.materials.len(),
            types: lookup.types.len(),
            retained_bytes: available::add(
                available::capacity(&lookup.materials)?,
                available::capacity(&lookup.types)?,
            )?,
        })
    })();
    let lookup_stats = guarded(&mut context, &mut sink, result)?;
    drop(maps);
    let result = context
        .collector_mut()
        .release_external(map_stats.resident_bytes)
        .map_err(Error::from);
    guarded(&mut context, &mut sink, result)?;
    map.materialized_rows = map_stats.rows;
    Ok(Prepared {
        context: Some(context),
        owner: Some(owner),
        lookup: Some(lookup),
        sink,
        topology: Some(topology),
        map: Some(map),
        liquids: Some(liquids),
        lookup_stats,
        map_stats,
        retained,
        sink_allowance: sink_heap_allowance,
        limits,
        profile,
        failed: None,
        callback_used: false,
    })
}
