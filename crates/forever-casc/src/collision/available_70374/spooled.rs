// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-16: available70374 startup over the existing payload spool.
//! Distinct available GODI facts; shared descriptor/cache/list engine. Known
//! dependencies remain complete and terminal on failure. No world/publication.

use super::{TableFacts, allowance, header_policy, table_facts};
use crate::collision::{
    Facts, gameobjects,
    payload::{self, CallbackFailure, CollectorError, SpoolingExtraction, SpoolingExtractor},
    spool::Spool,
};
use crate::local_store::LocalStore;
use forever_db2::availability as db2;
use forever_vmap::{gameobject_models, wmo::liquid::LiquidLookup};

pub mod world;

#[cfg(test)]
mod tests;

pub const PROFILE: &str = "linux70374-disabled-key-available-spooled-startup-v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cause {
    Available(super::Error),
    Engine(gameobjects::spooled::Cause),
    Collector(CollectorError),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Error {
    cause: Cause,
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
impl Error {
    pub const fn cause(self) -> Cause {
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
            Cause::Available(e) => e.code(),
            Cause::Engine(e) => match e {
                gameobjects::spooled::Cause::Startup(e) => e.code(),
                gameobjects::spooled::Cause::Collector(e) => e.code(),
                gameobjects::spooled::Cause::Lists(e) => e.code(),
            },
            Cause::Collector(e) => e.code(),
        }
    }
    pub(in crate::collision) fn with_guard(mut self, guard: Option<CollectorError>) -> Self {
        if let Some(e) = guard
            && (changed(e) || !self.guard.is_some_and(changed))
        {
            self.guard = Some(e);
        }
        self
    }
}
impl From<super::Error> for Error {
    fn from(e: super::Error) -> Self {
        Self {
            cause: Cause::Available(e),
            guard: None,
        }
    }
}
impl From<gameobjects::spooled::Error> for Error {
    fn from(e: gameobjects::spooled::Error) -> Self {
        Self {
            cause: Cause::Engine(e.cause()),
            guard: e.guard(),
        }
    }
}
impl From<CollectorError> for Error {
    fn from(e: CollectorError) -> Self {
        Self {
            cause: Cause::Collector(e),
            guard: None,
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}
impl std::error::Error for Error {}
fn bridge(e: CallbackFailure<Error>) -> Error {
    match e {
        CallbackFailure::Collector(e) => e.into(),
        CallbackFailure::Callback { cause, guard } => cause.with_guard(guard),
    }
}

/// Available section coverage is retained independently of complete model facts.
/// No conversion into complete gameobjects::Data/SourceFacts is provided.
pub struct Data {
    table: TableFacts,
    engine: gameobjects::EngineData,
}
impl Data {
    pub const fn table(&self) -> &TableFacts {
        &self.table
    }
    pub fn outcomes(&self) -> &[gameobjects::RowOutcome] {
        &self.engine.outcomes
    }
    pub const fn lists(&self) -> &gameobject_models::Lists {
        &self.engine.lists
    }
    pub const fn stats(&self) -> gameobjects::Stats {
        self.engine.stats
    }
    pub const fn startup_facts(&self) -> Facts {
        self.engine.startup_facts
    }
}
pub type Startup<'store> = Context<'store, 'static, gameobjects::EmptyLookup>;
pub struct Context<'store, 'lookup, L: LiquidLookup + ?Sized> {
    data: Data,
    collector: SpoolingExtractor<'store, 'lookup, L>,
}
impl<'store, 'lookup, L: LiquidLookup + ?Sized> Context<'store, 'lookup, L> {
    pub fn data(&self) -> Result<&Data, Error> {
        self.collector.facts()?;
        Ok(&self.data)
    }
    pub fn collector_mut(&mut self) -> &mut SpoolingExtractor<'store, 'lookup, L> {
        &mut self.collector
    }
    pub fn with_lookup<'next, N: LiquidLookup + ?Sized>(
        self,
        lookup: &'next N,
    ) -> Result<Context<'store, 'next, N>, Error> {
        Ok(Context {
            data: self.data,
            collector: self.collector.with_lookup(lookup)?,
        })
    }
    pub(in crate::collision) fn try_with_lookup<'next, N: LiquidLookup + ?Sized>(
        self,
        lookup: &'next N,
    ) -> payload::LookupTransition<Context<'store, 'next, N>, Self> {
        match self.collector.try_with_lookup(lookup) {
            payload::LookupTransition::Ready(collector) => {
                payload::LookupTransition::Ready(Context {
                    data: self.data,
                    collector,
                })
            }
            payload::LookupTransition::Failed(collector, e) => payload::LookupTransition::Failed(
                Context {
                    data: self.data,
                    collector,
                },
                e,
            ),
        }
    }
    pub(in crate::collision) fn finish_with_topology(
        self,
        topology: crate::collision::world::Topology,
    ) -> Result<(CompletedStartup, crate::collision::world::Topology), Error> {
        let retained = self
            .data
            .engine
            .stats
            .resident_bytes
            .checked_add(topology.resident_bytes())
            .ok_or(super::Error::Limit)?;
        let extraction = self.collector.finish_with_retained_external(retained)?;
        Ok((
            CompletedStartup {
                data: self.data,
                extraction,
            },
            topology,
        ))
    }
    /// Data remains charged inside the extraction for every later payload reload.
    pub fn finish(self) -> Result<CompletedStartup, Error> {
        let extraction = self
            .collector
            .finish_with_retained_external(self.data.engine.stats.resident_bytes)?;
        Ok(CompletedStartup {
            data: self.data,
            extraction,
        })
    }
}
pub struct CompletedStartup {
    data: Data,
    extraction: SpoolingExtraction,
}
impl CompletedStartup {
    pub const fn profile(&self) -> &'static str {
        PROFILE
    }
    pub fn data(&self) -> Result<&Data, Error> {
        self.extraction.ready()?;
        Ok(&self.data)
    }
    pub const fn extraction(&self) -> &SpoolingExtraction {
        &self.extraction
    }
    pub fn extraction_mut(&mut self) -> &mut SpoolingExtraction {
        &mut self.extraction
    }
    /// Splitting never releases the retained Data charge from the spool owner.
    pub fn into_parts(self) -> (Data, SpoolingExtraction) {
        (self.data, self.extraction)
    }
}

pub fn extract_startup_fdid(
    store: &mut LocalStore,
    limits: gameobjects::Limits,
    spool: Spool,
) -> Result<Startup<'_>, Error> {
    gameobjects::spooled::validate_resources(limits, spool.resources())?;
    let mut collector = SpoolingExtractor::new(store, &super::EMPTY, limits.collision, spool)?;
    let result = extract_data(&mut collector, limits);
    let data = collector
        .finish_external_operation(result)
        .map_err(bridge)?;
    Ok(Context { data, collector })
}
fn extract_data(
    collector: &mut SpoolingExtractor<'_, '_, gameobjects::EmptyLookup>,
    limits: gameobjects::Limits,
) -> Result<Data, Error> {
    let file =
        collector.read_available_source(gameobjects::TABLE_FILE_DATA_ID, limits.max_db2_bytes)?;
    let source_resident = file
        .resident_bytes()
        .map_err(|e| super::Error::Collision(crate::collision::Error::from_available(e)))?;
    header_policy(&file, limits.max_direct_rows, limits.max_copy_records)?;
    let allowance = allowance(
        file.declared_bytes(),
        false,
        limits.max_decode_allowance_bytes,
    )?;
    collector.reserve_external(allowance)?;
    let snapshot = db2::read_gameobject_display_info_70374(&file).map_err(super::Error::Db2)?;
    let mut table = table_facts(
        &file,
        snapshot.sections(),
        snapshot.direct_records().len(),
        snapshot.copies().len(),
        allowance,
    )?;
    if table.declared_direct_rows > limits.max_direct_rows
        || table.declared_copy_records > limits.max_copy_records
    {
        return Err(super::Error::Limit.into());
    }
    let (outcomes, stats) = gameobjects::spooled::collect_direct(
        collector,
        snapshot.direct_records(),
        snapshot.copies().len(),
        allowance,
        limits,
    )?;
    drop(snapshot);
    drop(file);
    collector.release_external(allowance)?;
    collector.release_source(source_resident)?;
    let engine = gameobjects::spooled::finish_lists(collector, outcomes, stats, limits)?;
    table.materialized_rows = table.direct_rows;
    Ok(Data { table, engine })
}
