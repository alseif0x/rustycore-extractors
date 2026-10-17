// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-16: available-table startup with explicit model omissions.
//! Required GODI reads remain fatal. Available TableFacts never become complete
//! SourceFacts. One same compatible collector survives lookup and later calls.
use super::{Collector, Completed, GameObjectOutcome, Omission, Target};
use crate::collision::{
    Facts, add,
    available_70374::{self as available, TableFacts},
    gameobjects,
    payload::{self, CallbackFailure},
    spool::Spool,
};
use crate::local_store::LocalStore;
use forever_db2::availability as db2;
use forever_vmap::{gameobject_models, wmo::liquid::LiquidLookup};

pub type Error = available::spooled::Error;
pub(super) static EMPTY: gameobjects::EmptyLookup = gameobjects::EmptyLookup;
pub const PROFILE_70374: &str = "linux70374-disabled-key-compatible-spooled-startup-v1";
pub const PROFILE_70425: &str = "linux70425-disabled-key-compatible-spooled-startup-v1";
pub const PROFILE_GNU15_70374: &str =
    "linux70374-disabled-key-compatible-spooled-startup-gnu15-native-order-v2";
pub const PROFILE_GNU15_70425: &str =
    "linux70425-disabled-key-compatible-spooled-startup-gnu15-native-order-v2";
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Disposition {
    Normal(gameobjects::Outcome),
    Omitted(Omission),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Row {
    pub ordinal: usize,
    pub display_id: u32,
    pub file_data_id: u32,
    pub disposition: Disposition,
}
pub struct Data {
    table: TableFacts,
    rows: Vec<Row>,
    engine: gameobjects::EngineData,
}
impl Data {
    pub const fn table(&self) -> &TableFacts {
        &self.table
    }
    pub fn rows(&self) -> &[Row] {
        &self.rows
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
pub type Startup<'s> = Context<'s, 'static, gameobjects::EmptyLookup>;
pub struct Context<'s, 'l, L: LiquidLookup + ?Sized> {
    data: Data,
    collector: Collector<'s, 'l, L>,
}
impl<'s, 'l, L: LiquidLookup + ?Sized> Context<'s, 'l, L> {
    pub fn profile(&self) -> &'static str {
        profile(self.collector.target(), self.collector.native_order())
    }

    pub fn data(&self) -> Result<&Data, Error> {
        self.collector.facts()?;
        Ok(&self.data)
    }
    pub fn collector_mut(&mut self) -> &mut Collector<'s, 'l, L> {
        &mut self.collector
    }
    pub fn with_lookup<'n, N: LiquidLookup + ?Sized>(
        self,
        lookup: &'n N,
    ) -> Result<Context<'s, 'n, N>, Error> {
        Ok(Context {
            data: self.data,
            collector: self.collector.with_lookup(lookup)?,
        })
    }
    pub(in crate::collision) fn try_with_lookup<'n, N: LiquidLookup + ?Sized>(
        self,
        lookup: &'n N,
    ) -> payload::LookupTransition<Context<'s, 'n, N>, Self> {
        let Self { data, collector } = self;
        match collector.try_with_lookup(lookup) {
            payload::LookupTransition::Ready(collector) => {
                payload::LookupTransition::Ready(Context { data, collector })
            }
            payload::LookupTransition::Failed(collector, error) => {
                payload::LookupTransition::Failed(Context { data, collector }, error)
            }
        }
    }
    pub(in crate::collision) fn finish_with_topology(
        self,
        topology: crate::collision::world::Topology,
    ) -> Result<(CompletedStartup, crate::collision::world::Topology), Error> {
        let retained = add(
            self.data.engine.stats.resident_bytes,
            topology.resident_bytes(),
        )
        .map_err(available::Error::Collision)?;
        let extraction = self.collector.finish_with_external(retained)?;
        Ok((
            CompletedStartup {
                data: self.data,
                extraction,
            },
            topology,
        ))
    }
    pub fn finish(self) -> Result<CompletedStartup, Error> {
        let extraction = self
            .collector
            .finish_with_external(self.data.engine.stats.resident_bytes)?;
        Ok(CompletedStartup {
            data: self.data,
            extraction,
        })
    }
}
pub struct CompletedStartup {
    data: Data,
    extraction: Completed,
}
impl CompletedStartup {
    pub fn profile(&self) -> &'static str {
        profile(self.extraction.target, self.extraction.state.native_order)
    }

    pub fn data(&mut self) -> Result<&Data, Error> {
        self.extraction.verify_unchanged()?;
        Ok(&self.data)
    }
    pub fn extraction_mut(&mut self) -> &mut Completed {
        &mut self.extraction
    }
}
pub fn extract_startup_fdid_70374(
    store: &mut LocalStore,
    limits: gameobjects::Limits,
    spool: Spool,
) -> Result<Startup<'_>, Error> {
    extract(store, limits, spool, Target::Build70374, false)
}
pub fn extract_startup_fdid_70425(
    store: &mut LocalStore,
    limits: gameobjects::Limits,
    spool: Spool,
) -> Result<Startup<'_>, Error> {
    extract(store, limits, spool, Target::Build70425, false)
}
pub fn extract_startup_fdid_native_gnu15_70374(
    store: &mut LocalStore,
    limits: gameobjects::Limits,
    spool: Spool,
) -> Result<Startup<'_>, Error> {
    extract(store, limits, spool, Target::Build70374, true)
}
pub fn extract_startup_fdid_native_gnu15_70425(
    store: &mut LocalStore,
    limits: gameobjects::Limits,
    spool: Spool,
) -> Result<Startup<'_>, Error> {
    extract(store, limits, spool, Target::Build70425, true)
}
fn profile(target: Target, native_order: bool) -> &'static str {
    match (target, native_order) {
        (Target::Build70374, false) => PROFILE_70374,
        (Target::Build70425, false) => PROFILE_70425,
        (Target::Build70374, true) => PROFILE_GNU15_70374,
        (Target::Build70425, true) => PROFILE_GNU15_70425,
    }
}
fn bridge(e: CallbackFailure<Error>) -> Error {
    match e {
        CallbackFailure::Collector(e) => e.into(),
        CallbackFailure::Callback { cause, guard } => cause.with_guard(guard),
    }
}
fn extract(
    store: &mut LocalStore,
    limits: gameobjects::Limits,
    spool: Spool,
    target: Target,
    native_order: bool,
) -> Result<Startup<'_>, Error> {
    gameobjects::spooled::validate_resources(limits, spool.resources())?;
    let mut collector = Collector::new_with_native_order(
        store,
        &EMPTY,
        target,
        limits.collision,
        spool,
        native_order,
    )?;
    let result = extract_data(&mut collector, limits, target);
    let data = collector
        .inner
        .finish_external_operation(result)
        .map_err(bridge)?;
    Ok(Context { data, collector })
}
fn reserve_rows<T>(
    collector: &mut Collector<'_, '_, gameobjects::EmptyLookup>,
    count: usize,
) -> Result<Vec<T>, Error> {
    let requested = count
        .checked_mul(size_of::<T>())
        .ok_or(available::Error::Limit)?;
    collector.inner.reserve_external(requested)?;
    let mut out = Vec::new();
    out.try_reserve_exact(count)
        .map_err(|_| available::Error::Limit)?;
    let actual = out
        .capacity()
        .checked_mul(size_of::<T>())
        .ok_or(available::Error::Limit)?;
    collector.inner.reserve_external(
        actual
            .checked_sub(requested)
            .ok_or(available::Error::State)?,
    )?;
    Ok(out)
}
fn extract_data(
    collector: &mut Collector<'_, '_, gameobjects::EmptyLookup>,
    limits: gameobjects::Limits,
    target: Target,
) -> Result<Data, Error> {
    let result = collector.inner.inner.read_available_source_target(
        gameobjects::TABLE_FILE_DATA_ID,
        limits.max_db2_bytes,
        target,
    );
    let file = collector.inner.checked(result)?;
    let source_resident = file
        .resident_bytes()
        .map_err(|e| available::Error::Collision(crate::collision::Error::from_available(e)))?;
    available::header_policy(&file, limits.max_direct_rows, limits.max_copy_records)?;
    let allowance = available::allowance(
        file.declared_bytes(),
        false,
        limits.max_decode_allowance_bytes,
    )?;
    collector.inner.reserve_external(allowance)?;
    let snapshot = match target {
        Target::Build70374 => db2::read_gameobject_display_info_70374(&file),
        Target::Build70425 => db2::read_gameobject_display_info_70425(&file),
    }
    .map_err(available::Error::Db2)?;
    let mut table = available::table_facts(
        &file,
        snapshot.sections(),
        snapshot.direct_records().len(),
        snapshot.copies().len(),
        allowance,
    )?;
    if table.declared_direct_rows > limits.max_direct_rows
        || table.declared_copy_records > limits.max_copy_records
    {
        return Err(available::Error::Limit.into());
    }
    let direct = snapshot.direct_records();
    let candidates = direct.iter().filter(|r| r.fields.file_data_id != 0).count();
    if direct.len() > limits.max_direct_rows
        || snapshot.copies().len() > limits.max_copy_records
        || candidates > limits.max_candidate_requests
    {
        return Err(available::Error::Limit.into());
    }
    let mut rows = reserve_rows(collector, direct.len())?;
    let mut normal = reserve_rows(collector, direct.len())?;
    let mut stats = gameobjects::Stats {
        direct_rows: direct.len(),
        copy_records: snapshot.copies().len(),
        candidate_requests: candidates,
        db2_decode_allowance_bytes: allowance,
        ..gameobjects::Stats::default()
    };
    for (ordinal, row) in direct.iter().enumerate() {
        let disposition = if row.fields.file_data_id == 0 {
            stats.zero_fdids += 1;
            Disposition::Normal(gameobjects::Outcome::ZeroFileDataId)
        } else {
            match collector.extract_gameobject_fdid(row.fields.file_data_id)? {
                GameObjectOutcome::Omitted(reason) => Disposition::Omitted(reason),
                GameObjectOutcome::Texture => {
                    stats.textures += 1;
                    Disposition::Normal(gameobjects::Outcome::Texture)
                }
                GameObjectOutcome::Model(h) => {
                    let outcome =
                        gameobjects::spooled::model_outcome(&collector.inner, h, &mut stats)?;
                    Disposition::Normal(outcome)
                }
            }
        };
        rows.push(Row {
            ordinal,
            display_id: row.id,
            file_data_id: row.fields.file_data_id,
            disposition,
        });
        if let Disposition::Normal(outcome) = disposition {
            normal.push(gameobjects::RowOutcome::new(
                ordinal,
                row.id,
                row.fields.file_data_id,
                outcome,
            ));
        }
    }
    drop(snapshot);
    drop(file);
    collector.inner.release_external(allowance)?;
    collector.inner.release_source(source_resident)?;
    let mut engine =
        gameobjects::spooled::finish_lists(&mut collector.inner, normal, stats, limits)?;
    engine.stats.resident_bytes = add(
        engine.stats.resident_bytes,
        super::capacity(&rows).map_err(available::Error::Collision)?,
    )
    .map_err(available::Error::Collision)?;
    table.materialized_rows = table.direct_rows;
    Ok(Data {
        table,
        rows,
        engine,
    })
}
