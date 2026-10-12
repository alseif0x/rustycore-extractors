// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original explicit spooled GameObject startup consumer.
//! Same physical direct rows and same collector, complete lists, one guarded
//! complete-record reload per listed row. No product/world/assembly publication.
//! DB2 allowance remains conservative; global/active accounting is not RSS.

use super::{
    Data, EMPTY_LOOKUP, EmptyLookup, Limits, Outcome, RowOutcome, Stats, TABLE_FILE_DATA_ID,
};
use crate::collision::{
    Class, GameObjectOutcome, Status,
    payload::{self, CallbackFailure, CollectorError, SpoolingExtraction, SpoolingExtractor},
    spool::Spool,
};
use crate::local_store::LocalStore;
use forever_db2::terrain_sources;
use forever_vmap::{
    gameobject_models::{self, streaming},
    wmo::liquid::LiquidLookup,
};

pub const PROFILE: &str = "linux-private-spooled-gameobject-startup-v1";
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cause {
    Startup(super::Error),
    Collector(CollectorError),
    Lists(streaming::Error),
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
    /// Original operation error is retained even when a final guard also fails.
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
            Cause::Startup(e) => e.code(),
            Cause::Collector(e) => e.code(),
            Cause::Lists(e) => e.code(),
        }
    }
    fn with_guard(mut self, guard: Option<CollectorError>) -> Self {
        if let Some(e) = guard
            && (changed(e) || !self.guard.is_some_and(changed))
        {
            self.guard = Some(e)
        }
        self
    }
}
impl From<super::Error> for Error {
    fn from(e: super::Error) -> Self {
        Self {
            cause: Cause::Startup(e),
            guard: None,
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
impl From<streaming::Error> for Error {
    fn from(e: streaming::Error) -> Self {
        Self {
            cause: Cause::Lists(e),
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

pub type Startup<'s> = Context<'s, 'static, EmptyLookup>;
pub struct Context<'s, 'l, L: LiquidLookup + ?Sized> {
    data: Data,
    collector: SpoolingExtractor<'s, 'l, L>,
}
impl<'s, 'l, L: LiquidLookup + ?Sized> Context<'s, 'l, L> {
    pub fn data(&self) -> Result<&Data, Error> {
        self.collector.facts()?;
        Ok(&self.data)
    }
    pub fn collector_mut(&mut self) -> &mut SpoolingExtractor<'s, 'l, L> {
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
    pub(crate) fn try_with_lookup<'n, N: LiquidLookup + ?Sized>(
        self,
        lookup: &'n N,
    ) -> payload::LookupTransition<Context<'s, 'n, N>, Self> {
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
    // The owned, unforgeable world topology travels with its exact reservation.
    // Completed payload reloads conservatively retain BOTH Data and topology.
    pub(crate) fn finish_with_topology(
        self,
        topology: crate::collision::world::Topology,
    ) -> Result<(Completed, crate::collision::world::Topology), Error> {
        let retained = self
            .data
            .stats
            .resident_bytes
            .checked_add(topology.resident_bytes())
            .ok_or(super::Error::Limit)?;
        let extraction = self.collector.finish_with_retained_external(retained)?;
        Ok((
            Completed {
                data: self.data,
                extraction,
            },
            topology,
        ))
    }
    pub fn finish(self) -> Result<Completed, Error> {
        let extraction = self
            .collector
            .finish_with_retained_external(self.data.stats.resident_bytes)?;
        Ok(Completed {
            data: self.data,
            extraction,
        })
    }
}
pub struct Completed {
    data: Data,
    extraction: SpoolingExtraction,
}
impl Completed {
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
    /// The extraction conservatively retains Data's reservation after splitting,
    /// even if the caller subsequently drops Data. No uncharged reload window.
    pub fn into_parts(self) -> (Data, SpoolingExtraction) {
        (self.data, self.extraction)
    }
}
pub fn extract_startup_fdid(
    store: &mut LocalStore,
    limits: Limits,
    spool: Spool,
) -> Result<Startup<'_>, Error> {
    limits.validate_resources(spool.resources())?;
    let mut collector = SpoolingExtractor::new(store, &EMPTY_LOOKUP, limits.collision, spool)?;
    let result = extract_data(&mut collector, limits);
    let data = collector
        .finish_external_operation(result)
        .map_err(bridge)?;
    Ok(Context { data, collector })
}
fn extract_data(
    collector: &mut SpoolingExtractor<'_, '_, EmptyLookup>,
    limits: Limits,
) -> Result<Data, Error> {
    let (bytes, source) = collector.read_source(TABLE_FILE_DATA_ID, limits.max_db2_bytes)?;
    if super::word(&bytes, 136).is_some_and(|n| n as usize > limits.max_direct_rows)
        || (super::word(&bytes, 200) == Some(1)
            && super::word(&bytes, 240).is_some_and(|n| n as usize > limits.max_copy_records))
    {
        return Err(super::Error::Limit.into());
    }
    let allowance = bytes
        .len()
        .checked_mul(32)
        .and_then(|n| n.checked_add(2 * 1024 * 1024))
        .ok_or(super::Error::Limit)?;
    if allowance > limits.max_decode_allowance_bytes {
        return Err(super::Error::Limit.into());
    }
    collector.reserve_external(allowance)?;
    let snapshot =
        terrain_sources::read_gameobject_display_info(&bytes).map_err(super::Error::Db2)?;
    let snapshot_bytes = snapshot.resident_bytes().map_err(super::Error::Db2)?;
    if snapshot_bytes > allowance {
        return Err(super::Error::Limit.into());
    }
    collector.release_external(allowance - snapshot_bytes)?;
    let source_capacity = bytes.capacity();
    drop(bytes);
    collector.release_source(source_capacity)?;
    let (direct, copies) = snapshot.into_parts();
    let copy_records = copies.len();
    let copy_capacity = super::capacity(&copies)?;
    drop(copies);
    collector.release_external(copy_capacity)?;
    if direct.len() > limits.max_direct_rows || copy_records > limits.max_copy_records {
        return Err(super::Error::Limit.into());
    }
    let candidates = direct.iter().filter(|r| r.fields.file_data_id != 0).count();
    if candidates > limits.max_candidate_requests {
        return Err(super::Error::Limit.into());
    }
    let mut outcomes = reserved_vec(collector, direct.len())?;
    let mut stats = Stats {
        direct_rows: direct.len(),
        copy_records,
        candidate_requests: candidates,
        db2_decode_allowance_bytes: allowance,
        ..Stats::default()
    };
    for (ordinal, row) in direct.iter().enumerate() {
        let outcome = if row.fields.file_data_id == 0 {
            stats.zero_fdids += 1;
            Outcome::ZeroFileDataId
        } else {
            match collector.extract_gameobject_fdid(row.fields.file_data_id)? {
                GameObjectOutcome::Texture => {
                    stats.textures += 1;
                    Outcome::Texture
                }
                GameObjectOutcome::Model(handle) => {
                    let descriptor = collector.descriptor(handle)?;
                    let shape = collector.payload_shape(handle)?;
                    match (
                        descriptor.class(),
                        descriptor.status(),
                        shape.raw_bytes.is_some(),
                    ) {
                        (_, Status::Collision, true) => {
                            stats.collision_rows += 1;
                            Outcome::Collision(handle)
                        }
                        (Class::M2, Status::NoCollision, false) => {
                            stats.m2_no_collision += 1;
                            Outcome::M2NoCollision(handle)
                        }
                        (Class::Wmo, Status::DoodadOnly, false) => {
                            stats.wmo_doodad_only += 1;
                            Outcome::WmoDoodadOnly(handle)
                        }
                        _ => return Err(super::Error::State.into()),
                    }
                }
            }
        };
        outcomes.push(RowOutcome {
            ordinal,
            display_id: row.id,
            file_data_id: row.fields.file_data_id,
            outcome,
        });
    }
    let direct_capacity = super::capacity(&direct)?;
    drop(direct);
    collector.release_external(direct_capacity)?;
    let lists = build_lists(collector, &outcomes, stats.collision_rows, limits.lists)?;
    stats.resident_bytes = super::capacity(&outcomes)?
        .checked_add(lists.stats().resident_bytes)
        .ok_or(super::Error::Limit)?;
    stats.collector_retained_bytes = collector.retained_bytes()?;
    collector.verify_unchanged()?;
    Ok(Data {
        source,
        outcomes,
        lists,
        stats,
        startup_facts: collector.facts()?,
    })
}
fn class(class: Class) -> streaming::Class {
    match class {
        Class::M2 => streaming::Class::M2,
        Class::Wmo => streaming::Class::Wmo,
    }
}
fn build_lists<L: LiquidLookup + ?Sized>(
    collector: &mut SpoolingExtractor<'_, '_, L>,
    outcomes: &[RowOutcome],
    count: usize,
    limits: gameobject_models::Limits,
) -> Result<gameobject_models::Lists, Error> {
    let (mut reload_bytes, mut largest) = (0usize, 0usize);
    for row in outcomes {
        if let Outcome::Collision(handle) = row.outcome {
            let shape = collector.payload_shape(handle)?;
            reload_bytes = reload_bytes
                .checked_add(shape.record_bytes)
                .ok_or(super::Error::Limit)?;
            largest = largest.max(shape.record_bytes);
        }
    }
    // Admission only: actual attempts/bytes are consumed exactly once by reload.
    collector.admit_reloads(count, reload_bytes)?;
    let mut metadata = reserved_vec(collector, count)?;
    let metadata_capacity = super::capacity(&metadata)?;
    let free = collector.free_bytes()?;
    let allowance = limits
        .max_memory_bytes
        .min(free.checked_sub(largest).ok_or(super::Error::Limit)?);
    collector.reserve_external(allowance)?;
    // Reserve before borrowing compact names. No RAW/root/geometry borrows enter
    // the metadata vector, and it is dropped before any mutable payload reload.
    for row in outcomes {
        if let Outcome::Collision(handle) = row.outcome {
            let descriptor = collector.descriptor(handle)?;
            let shape = collector.payload_shape(handle)?;
            metadata.push(streaming::Metadata {
                display_id: row.display_id,
                name: descriptor.output_basename(),
                model: Some(streaming::ModelInfo {
                    class: class(shape.class),
                    raw_bytes: shape.raw_bytes.ok_or(super::Error::State)?,
                }),
            });
        }
    }
    if metadata.len() != count {
        return Err(super::Error::State.into());
    }
    let list_limits = gameobject_models::Limits {
        max_memory_bytes: allowance,
        ..limits
    };
    let profile = collector.resources().list_profile();
    let plan = streaming::Plan::preflight_with_profile(&metadata, list_limits, profile)?;
    let mut builder = streaming::Builder::new_with_profile(plan, &metadata, list_limits, profile)?;
    if builder.resident_bytes()? > allowance {
        return Err(super::Error::State.into());
    }
    drop(metadata);
    collector.release_external(metadata_capacity)?;
    for row in outcomes {
        if let Outcome::Collision(handle) = row.outcome {
            let shape = collector.payload_shape(handle)?;
            collector
                .with_payload_result(handle, |p| {
                    let raw = p.raw().ok_or(super::Error::State)?;
                    if Some(raw.len()) != shape.raw_bytes {
                        return Err(super::Error::State.into());
                    }
                    let raw = match shape.class {
                        Class::M2 => gameobject_models::RawModel::M2(raw),
                        Class::Wmo => gameobject_models::RawModel::Wmo(raw),
                    };
                    builder.push(raw).map_err(Error::from)
                })
                .map_err(bridge)?;
        }
    }
    let lists = builder.finish()?;
    let actual = lists.stats().resident_bytes;
    collector.release_external(allowance.checked_sub(actual).ok_or(super::Error::State)?)?;
    Ok(lists)
}
fn reserved_vec<T, L: LiquidLookup + ?Sized>(
    collector: &mut SpoolingExtractor<'_, '_, L>,
    count: usize,
) -> Result<Vec<T>, Error> {
    let requested = count
        .checked_mul(size_of::<T>())
        .ok_or(super::Error::Limit)?;
    collector.reserve_external(requested)?;
    let mut vec = Vec::new();
    vec.try_reserve_exact(count)
        .map_err(|_| super::Error::Allocation)?;
    let actual = super::capacity(&vec)?;
    collector.reserve_external(actual.checked_sub(requested).ok_or(super::Error::State)?)?;
    Ok(vec)
}
#[cfg(test)]
mod tests;
