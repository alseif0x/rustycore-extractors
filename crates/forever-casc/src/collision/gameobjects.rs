// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: complete offline GameObject startup using existing readers.
//! GameObject startup precedes liquid-table loading in the pinned extractor
//! (02245dcd245e7433e524577656177723d3e4992e, vmapexport.cpp:713-723).
//! This explicit-FDID profile reads the whole GameObjectDisplayInfo table, then
//! visits its direct rows in physical order. Zero FDIDs cause no acquisition;
//! copies are validated but never replayed. Only models with collision RAW enter
//! both complete lists. Texture, M2 NoCollision and WMO DoodadOnly remain
//! per-ordinal outcomes. Every acquisition/decoding/limit error rejects startup.
//! Native FILE-name-first routing, model publication and runtime are not implied.
//!
//! Startup keeps its collector tied to the same LocalStore. A consuming lookup
//! transition retains the existing cache, including geometry and omission
//! outcomes made with empty liquid tables. Later lookups affect only new models.
//! The caller's lookup storage is excluded from the collector's budget, as in
//! the collector API; caller-owned allocations must be budgeted separately.
//!
//! Source buffers, snapshot/outcome/projection vector capacities, both lists and
//! collector storage share the collector memory ceiling, including acquisition
//! scratch. Before DB2 decoding, 2 MiB + 32 * input length is reserved as a
//! conservative admission allowance for this fixed numeric schema: nine columns,
//! common maps (eight source bytes per entry), pallet vectors, ordinal IDs,
//! growing eight-byte projected rows, copy pairs and one fourteen-scalar row.
//! The source Vec stays charged throughout. This allowance is not an exact or
//! portable allocator bound: std BTreeMap nodes and temporary Vec growth are
//! not instrumented by the DB2 decoder. No end-to-end hard-heap or RSS claim is
//! made. On return the allowance becomes actual snapshot capacities; capacities
//! are released only after their owned buffers are dropped or transferred.

use super::{
    Class, Extraction, Extractor, Facts, GameObjectOutcome, Model, ModelHandle, SourceFacts, Status,
};
use crate::local_store::LocalStore;
use forever_db2::{RecordError, terrain_sources};
use forever_vmap::{gameobject_models, wmo::liquid::LiquidLookup};

#[cfg(test)]
mod tests;

pub const TABLE_FILE_DATA_ID: u32 = 1_266_277;
pub const MAX_DB2_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_DIRECT_ROWS: usize = 1_000_000 / 14;
pub const MAX_COPY_RECORDS: usize = 100_000;
pub const MAX_CANDIDATE_REQUESTS: usize = 4096;
pub const MAX_DECODE_ALLOWANCE_BYTES: usize = 2 * 1024 * 1024 + 32 * MAX_DB2_BYTES;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub max_db2_bytes: usize,
    pub max_direct_rows: usize,
    pub max_copy_records: usize,
    /// Every nonzero direct FDID counts, including cache hits and textures.
    pub max_candidate_requests: usize,
    /// Conservative decoder admission allowance, not a tracked allocator cap.
    pub max_decode_allowance_bytes: usize,
    /// Includes the DB2 read and source bytes, not only subsequent models.
    pub collision: super::Limits,
    /// Repeated rows count again toward RAW and vertex visit limits.
    pub lists: gameobject_models::Limits,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_db2_bytes: MAX_DB2_BYTES,
            max_direct_rows: MAX_DIRECT_ROWS,
            max_copy_records: MAX_COPY_RECORDS,
            max_candidate_requests: MAX_CANDIDATE_REQUESTS,
            max_decode_allowance_bytes: MAX_DECODE_ALLOWANCE_BYTES,
            collision: super::Limits::default(),
            lists: gameobject_models::Limits::default(),
        }
    }
}
impl Limits {
    fn validate(self) -> Result<(), Error> {
        let defaults = Self::default();
        let l = self.lists;
        let d = defaults.lists;
        if self.max_db2_bytes > MAX_DB2_BYTES
            || self.max_direct_rows > MAX_DIRECT_ROWS
            || self.max_copy_records > MAX_COPY_RECORDS
            || self.max_candidate_requests > MAX_CANDIDATE_REQUESTS
            || self.max_decode_allowance_bytes > MAX_DECODE_ALLOWANCE_BYTES
            || l.max_list_bytes > d.max_list_bytes
            || l.max_raw_bytes > d.max_raw_bytes
            || l.max_raw_visit_bytes > d.max_raw_visit_bytes
            || l.max_rows > d.max_rows
            || l.max_name_bytes > d.max_name_bytes
            || l.max_vertex_visits > d.max_vertex_visits
            || l.max_memory_bytes > d.max_memory_bytes
        {
            return Err(Error::InvalidLimits);
        }
        self.collision.validate().map_err(Error::Collision)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidLimits,
    Limit,
    Allocation,
    State,
    Db2(RecordError),
    Collision(super::Error),
    Lists(gameobject_models::Error),
}
impl Error {
    pub fn code(self) -> &'static str {
        match self {
            Self::InvalidLimits => "gameobject_startup_invalid_limits",
            Self::Limit => "gameobject_startup_policy_limit",
            Self::Allocation => "gameobject_startup_allocation",
            Self::State => "gameobject_startup_state",
            Self::Db2(e) => e.code(),
            Self::Collision(e) => e.code(),
            Self::Lists(e) => e.code(),
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}
impl std::error::Error for Error {}

/// No implicit loading of liquid DB2s is performed in this startup phase.
pub struct EmptyLookup;
impl LiquidLookup for EmptyLookup {
    fn material_id(&self, _: u32) -> Option<u8> {
        None
    }
    fn material_flags(&self, _: u8) -> Option<u32> {
        None
    }
}
static EMPTY_LOOKUP: EmptyLookup = EmptyLookup;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    ZeroFileDataId,
    Texture,
    Collision(ModelHandle),
    M2NoCollision(ModelHandle),
    WmoDoodadOnly(ModelHandle),
}
pub struct RowOutcome {
    ordinal: usize,
    display_id: u32,
    file_data_id: u32,
    outcome: Outcome,
}
impl RowOutcome {
    pub const fn ordinal(&self) -> usize {
        self.ordinal
    }
    pub const fn display_id(&self) -> u32 {
        self.display_id
    }
    pub const fn file_data_id(&self) -> u32 {
        self.file_data_id
    }
    pub const fn outcome(&self) -> Outcome {
        self.outcome
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    pub direct_rows: usize,
    pub copy_records: usize,
    pub candidate_requests: usize,
    pub zero_fdids: usize,
    pub textures: usize,
    pub m2_no_collision: usize,
    pub wmo_doodad_only: usize,
    pub collision_rows: usize,
    /// Actual capacities of the returned outcome vector and both lists.
    pub resident_bytes: usize,
    /// Collector storage plus this Data's heap at successful startup; excludes
    /// the LocalStore resident index and caller-owned later lookup storage.
    pub collector_retained_bytes: usize,
    pub db2_decode_allowance_bytes: usize,
}
pub struct Data {
    source: SourceFacts,
    outcomes: Vec<RowOutcome>,
    lists: gameobject_models::Lists,
    stats: Stats,
    startup_facts: Facts,
}
impl Data {
    pub const fn source(&self) -> &SourceFacts {
        &self.source
    }
    pub fn outcomes(&self) -> &[RowOutcome] {
        &self.outcomes
    }
    pub const fn lists(&self) -> &gameobject_models::Lists {
        &self.lists
    }
    pub const fn stats(&self) -> Stats {
        self.stats
    }
    /// Snapshot at successful startup, before any subsequent extraction phase.
    pub const fn startup_facts(&self) -> Facts {
        self.startup_facts
    }
}
pub type Startup<'store> = Context<'store, 'static, EmptyLookup>;
pub struct Context<'store, 'lookup, L: LiquidLookup + ?Sized> {
    data: Data,
    collector: Extractor<'store, 'lookup, L>,
}
impl<'store, 'lookup, L: LiquidLookup + ?Sized> Context<'store, 'lookup, L> {
    pub const fn data(&self) -> &Data {
        &self.data
    }
    pub fn model(&self, handle: ModelHandle) -> Option<&Model> {
        self.collector.model(handle)
    }
    /// Later extraction retains startup output in the same memory budget.
    pub fn collector_mut(&mut self) -> &mut Extractor<'store, 'lookup, L> {
        &mut self.collector
    }
    pub fn with_lookup<'next, N: LiquidLookup + ?Sized>(
        self,
        lookup: &'next N,
    ) -> Result<Context<'store, 'next, N>, Error> {
        Ok(Context {
            data: self.data,
            collector: self
                .collector
                .with_lookup(lookup)
                .map_err(Error::Collision)?,
        })
    }
    /// A later collector error or changed input snapshot rejects this transfer.
    pub fn finish(mut self) -> Result<Completed, Error> {
        self.collector
            .release_external(self.data.stats.resident_bytes)
            .map_err(Error::Collision)?;
        Ok(Completed {
            data: self.data,
            extraction: self.collector.finish().map_err(Error::Collision)?,
        })
    }
}
pub struct Completed {
    data: Data,
    extraction: Extraction,
}
impl Completed {
    pub const fn data(&self) -> &Data {
        &self.data
    }
    pub const fn extraction(&self) -> &Extraction {
        &self.extraction
    }
    pub fn into_parts(self) -> (Data, Extraction) {
        (self.data, self.extraction)
    }
}

pub fn extract_startup_fdid(store: &mut LocalStore, limits: Limits) -> Result<Startup<'_>, Error> {
    limits.validate()?;
    let mut collector =
        Extractor::new(store, &EMPTY_LOOKUP, limits.collision).map_err(Error::Collision)?;
    let (bytes, source) = collector
        .read_source(TABLE_FILE_DATA_ID, limits.max_db2_bytes)
        .map_err(Error::Collision)?;
    // Header counts only establish early policy rejection. The closed decoder
    // still validates every field/extent before any model read or result.
    if word(&bytes, 136).is_some_and(|count| count as usize > limits.max_direct_rows)
        || (word(&bytes, 200) == Some(1)
            && word(&bytes, 240).is_some_and(|count| count as usize > limits.max_copy_records))
    {
        return Err(Error::Limit);
    }
    let allowance = bytes
        .len()
        .checked_mul(32)
        .and_then(|n| n.checked_add(2 * 1024 * 1024))
        .ok_or(Error::Limit)?;
    if allowance > limits.max_decode_allowance_bytes {
        return Err(Error::Limit);
    }
    collector
        .reserve_external(allowance)
        .map_err(Error::Collision)?;
    let snapshot = terrain_sources::read_gameobject_display_info(&bytes).map_err(Error::Db2)?;
    let snapshot_bytes = snapshot.resident_bytes().map_err(Error::Db2)?;
    if snapshot_bytes > allowance {
        return Err(Error::Limit);
    }
    collector
        .release_external(allowance - snapshot_bytes)
        .map_err(Error::Collision)?;
    let source_capacity = bytes.capacity();
    drop(bytes);
    collector
        .release_source(source_capacity)
        .map_err(Error::Collision)?;
    let (direct, copies) = snapshot.into_parts();
    let copy_records = copies.len();
    let copy_capacity = capacity(&copies)?;
    drop(copies);
    collector
        .release_external(copy_capacity)
        .map_err(Error::Collision)?;
    if direct.len() > limits.max_direct_rows || copy_records > limits.max_copy_records {
        return Err(Error::Limit);
    }
    let candidates = direct
        .iter()
        .filter(|row| row.fields.file_data_id != 0)
        .count();
    if candidates > limits.max_candidate_requests {
        return Err(Error::Limit);
    }
    let mut outcomes = reserved_vec(&mut collector, direct.len())?;
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
            match collector
                .extract_gameobject_fdid(row.fields.file_data_id)
                .map_err(Error::Collision)?
            {
                GameObjectOutcome::Texture => {
                    stats.textures += 1;
                    Outcome::Texture
                }
                GameObjectOutcome::Model(handle) => {
                    let model = collector.model(handle).ok_or(Error::State)?;
                    match (model.class(), model.status(), model.raw().is_some()) {
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
                        _ => return Err(Error::State),
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
    let direct_capacity = capacity(&direct)?;
    drop(direct);
    collector
        .release_external(direct_capacity)
        .map_err(Error::Collision)?;

    // Charge descriptor storage before borrowing models, then delegate both
    // list allocations and bounds scratch under the remaining admitted budget.
    let mut rows = reserved_vec(&mut collector, stats.collision_rows)?;
    let row_capacity = capacity(&rows)?;
    let list_allowance = limits
        .lists
        .max_memory_bytes
        .min(collector.free_bytes().map_err(Error::Collision)?);
    collector
        .reserve_external(list_allowance)
        .map_err(Error::Collision)?;
    for outcome in &outcomes {
        if let Outcome::Collision(handle) = outcome.outcome {
            let model = collector.model(handle).ok_or(Error::State)?;
            let raw = model.raw().ok_or(Error::State)?;
            rows.push(gameobject_models::ResolvedRow {
                display_id: outcome.display_id,
                name: model.output_basename(),
                model: Some(match model.class() {
                    Class::M2 => gameobject_models::RawModel::M2(raw),
                    Class::Wmo => gameobject_models::RawModel::Wmo(raw),
                }),
            });
        }
    }
    let lists = gameobject_models::build(
        &rows,
        gameobject_models::Limits {
            max_memory_bytes: list_allowance,
            ..limits.lists
        },
    )
    .map_err(Error::Lists)?;
    drop(rows);
    collector
        .release_external(row_capacity)
        .map_err(Error::Collision)?;
    let lists_capacity = lists.stats().resident_bytes;
    if lists_capacity > list_allowance {
        return Err(Error::State);
    }
    collector
        .release_external(list_allowance - lists_capacity)
        .map_err(Error::Collision)?;
    stats.resident_bytes = capacity(&outcomes)?
        .checked_add(lists_capacity)
        .ok_or(Error::Limit)?;
    stats.collector_retained_bytes = collector.retained_bytes();
    collector.verify().map_err(Error::Collision)?;
    let startup_facts = collector.facts();
    Ok(Context {
        data: Data {
            source,
            outcomes,
            lists,
            stats,
            startup_facts,
        },
        collector,
    })
}

fn word(bytes: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        bytes.get(offset..offset.checked_add(4)?)?.try_into().ok()?,
    ))
}
fn capacity<T>(vec: &Vec<T>) -> Result<usize, Error> {
    vec.capacity()
        .checked_mul(size_of::<T>())
        .ok_or(Error::Limit)
}
fn reserved_vec<T, L: LiquidLookup + ?Sized>(
    collector: &mut Extractor<'_, '_, L>,
    count: usize,
) -> Result<Vec<T>, Error> {
    let requested = count.checked_mul(size_of::<T>()).ok_or(Error::Limit)?;
    collector
        .reserve_external(requested)
        .map_err(Error::Collision)?;
    let mut vec = Vec::new();
    vec.try_reserve_exact(count)
        .map_err(|_| Error::Allocation)?;
    let actual = capacity(&vec)?;
    collector
        .reserve_external(actual.checked_sub(requested).ok_or(Error::State)?)
        .map_err(Error::Collision)?;
    Ok(vec)
}
