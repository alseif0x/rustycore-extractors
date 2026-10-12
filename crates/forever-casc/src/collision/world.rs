// SPDX-License-Identifier: GPL-3.0-or-later
// Modified 2026-10-12: typed shared preparation; original authored 2026-10-11: complete startup-to-world acquisition with one collector.
//! Consumes a successful empty-lookup GameObject startup, then acquires Map,
//! LiquidMaterial and LiquidType in pinned vmapexport.cpp:713-723 order. All
//! reads share the existing LocalStore, model cache and cumulative budgets.
//! Cached WMO output is never rebuilt when the liquid lookup changes. The
//! shared runner declares its own serial/global-preload ordering limitations.
//!
//! No files are published. Sink callbacks are provisional until this function
//! returns success; any failure consumes the context and cannot yield Completed.
//! Map provenance comes from the closed reader and its direct-prefix count,
//! rather than caller-supplied rows. Liquid replay preserves last direct values,
//! ordered copies and missing-source insertion, including zero/self/chains.
//!
//! Owned source buffers, materialized Map strings/vectors, liquid vectors and
//! graph capacities stay charged in the collector ledger while live. Before
//! each DB2 read is decoded, reserve 2 MiB + 32 * source length, plus 64 MiB for
//! schemas with text (Map and LiquidType). The text allowance covers the shared
//! cumulative UTF-8 policy, including Map copy cloning. The factor covers common
//! maps, column/pallet metadata, ordinal IDs, growing projection/copy vectors
//! and one decoded row on the supported toolchain. This is conservative admission
//! accounting, NOT a portable allocator bound or an RSS guarantee: std BTreeMap
//! nodes, temporary Vec growth and string allocator overhead are not instrumented.
//! Actual retained capacities replace the allowance after temporaries are gone.
//! Sink-owned memory is excluded and must be bounded separately by its caller.
use super::{SourceFacts, gameobjects, plan, run};
use forever_db2::{RecordError, terrain_sources as source};
use forever_vmap::wmo::liquid::LiquidLookup;
use std::mem::size_of;

mod input;
#[cfg(target_os = "linux")]
pub mod spooled;
#[cfg(test)]
mod tests;

pub const MAP_FILE_DATA_ID: u32 = 1_349_477;
pub const MATERIAL_FILE_DATA_ID: u32 = 1_132_538;
pub const TYPE_FILE_DATA_ID: u32 = 1_371_380;
pub const MAX_DB2_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_DIRECT_ROWS: usize = 100_000;
pub const MAX_COPY_RECORDS: usize = 100_000;
pub const MAX_LOOKUP_ROWS: usize = 300_000;
pub const MAX_TABLE_MEMORY_BYTES: usize = 256 * 1024 * 1024;
pub const MAX_DECODE_ALLOWANCE_BYTES: usize = 66 * 1024 * 1024 + 32 * MAX_DB2_BYTES;
const TEXT_ALLOWANCE: usize = 64 * 1024 * 1024;
static EMPTY: gameobjects::EmptyLookup = gameobjects::EmptyLookup;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub max_db2_bytes: usize,
    pub max_direct_rows: usize,
    pub max_copy_records: usize,
    /// Unique keys, including keys introduced by the RHS of a copy.
    pub max_lookup_rows: usize,
    /// Retained Map rows/strings, both lookup vectors, borrowed graph and
    /// owned topology snapshot together, including snapshot creation overlap.
    pub max_table_memory_bytes: usize,
    /// Conservative per-decode allowance; separate from retained table memory.
    pub max_decode_allowance_bytes: usize,
    pub maps: plan::Limits,
    pub run: run::SharedLimits,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_db2_bytes: MAX_DB2_BYTES,
            max_direct_rows: MAX_DIRECT_ROWS,
            max_copy_records: MAX_COPY_RECORDS,
            max_lookup_rows: MAX_LOOKUP_ROWS,
            max_table_memory_bytes: MAX_TABLE_MEMORY_BYTES,
            max_decode_allowance_bytes: MAX_DECODE_ALLOWANCE_BYTES,
            maps: plan::Limits::default(),
            run: run::SharedLimits::default(),
        }
    }
}
impl Limits {
    fn validate(self) -> Result<(), Error> {
        if self.max_db2_bytes > MAX_DB2_BYTES
            || self.max_direct_rows > MAX_DIRECT_ROWS
            || self.max_copy_records > MAX_COPY_RECORDS
            || self.max_lookup_rows > MAX_LOOKUP_ROWS
            || self.max_table_memory_bytes > MAX_TABLE_MEMORY_BYTES
            || self.max_decode_allowance_bytes > MAX_DECODE_ALLOWANCE_BYTES
        {
            return Err(Error::InvalidLimits);
        }
        self.maps.validate().map_err(Error::Plan)?;
        self.run.validate().map_err(Error::Run)
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
    Startup(gameobjects::Error),
    Plan(plan::Error),
    Run(run::Error),
}
impl Error {
    pub fn code(self) -> &'static str {
        match self {
            Self::InvalidLimits => "collision_world_invalid_limits",
            Self::Limit => "collision_world_policy_limit",
            Self::Allocation => "collision_world_allocation",
            Self::State => "collision_world_state",
            Self::Db2(e) => e.code(),
            Self::Collision(e) => e.code(),
            Self::Startup(e) => e.code(),
            Self::Plan(e) => e.code(),
            Self::Run(e) => e.code(),
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}
impl std::error::Error for Error {}

/// Source identity is retained without raw bytes or sensitive Debug output.
pub struct TableFacts {
    source: SourceFacts,
    pub direct_rows: usize,
    pub copy_records: usize,
    pub materialized_rows: usize,
    pub retained_bytes: usize,
    pub decode_allowance_bytes: usize,
}
impl TableFacts {
    pub const fn source(&self) -> &SourceFacts {
        &self.source
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LookupStats {
    pub materials: usize,
    pub types: usize,
    pub retained_bytes: usize,
}
/// Compact ownership of the already validated complete Map graph. No source
/// names or directory strings survive here. Physical rows include inactive and
/// appended copy rows; the direct prefix remains explicit. Construction is
/// private so callers cannot forge the graph's source/validation association.
/// Parents are the extractor's effective values; this does not retain the
/// original Parent/Cosmetic pair or prove the runtime pair-consistency rule.
pub struct Topology {
    rows: Vec<TopologyEntry>,
    by_id: Vec<usize>,
    direct_count: usize,
    resident_bytes: usize,
}
#[derive(Clone, Copy)]
pub struct TopologyEntry {
    id: u32,
    parent_map_id: Option<u32>,
    wdt_file_data_id: u32,
    physical_ordinal: usize,
    depth: usize,
    is_parent: bool,
}
impl TopologyEntry {
    pub const fn id(&self) -> u32 {
        self.id
    }
    pub const fn parent_map_id(&self) -> Option<u32> {
        self.parent_map_id
    }
    pub const fn wdt_file_data_id(&self) -> u32 {
        self.wdt_file_data_id
    }
    pub const fn active(&self) -> bool {
        self.wdt_file_data_id != 0
    }
    pub const fn physical_ordinal(&self) -> usize {
        self.physical_ordinal
    }
    pub const fn depth(&self) -> usize {
        self.depth
    }
    pub const fn is_parent(&self) -> bool {
        self.is_parent
    }
}
impl Topology {
    pub fn all(&self) -> &[TopologyEntry] {
        &self.rows
    }
    pub fn get(&self, id: u32) -> Option<&TopologyEntry> {
        let at = self
            .by_id
            .binary_search_by_key(&id, |&i| self.rows[i].id)
            .ok()?;
        Some(&self.rows[self.by_id[at]])
    }
    pub const fn direct_count(&self) -> usize {
        self.direct_count
    }
    pub const fn resident_bytes(&self) -> usize {
        self.resident_bytes
    }
    fn snapshot<L: LiquidLookup + ?Sized>(
        maps: &plan::Maps<'_>,
        collector: &mut super::Extractor<'_, '_, L>,
        max_bytes: usize,
    ) -> Result<Self, Error> {
        input::topology(maps, collector, max_bytes).map_err(native_failure)
    }
}
pub struct Completed {
    startup: gameobjects::Completed,
    report: run::SharedReport,
    tables: [TableFacts; 3],
    lookups: LookupStats,
    maps: plan::Stats,
    topology: Topology,
    resident_bytes: usize,
}
impl Completed {
    pub const fn startup(&self) -> &gameobjects::Completed {
        &self.startup
    }
    pub const fn report(&self) -> &run::SharedReport {
        &self.report
    }
    /// Fixed order: Map, LiquidMaterial, LiquidType.
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
    /// Exact retained capacities: final Extraction, startup Data and topology.
    /// Excludes the caller's Store, Sink, stack and allocator overhead.
    pub const fn resident_bytes(&self) -> usize {
        self.resident_bytes
    }
    pub const fn store_resident_bytes(&self) -> usize {
        self.startup.extraction().store_resident_bytes()
    }
}

/// Complete the shared world phase. The owned context cannot be recovered on
/// error. No partial Completed, source facts or success report is returned.
pub fn extract(
    mut startup: gameobjects::Startup<'_>,
    sink: &mut impl run::Sink,
    limits: Limits,
) -> Result<Completed, Error> {
    let prepared = (|| {
        limits.validate()?;
        prepare(&mut startup, limits)
    })();
    let prepared = match prepared {
        Ok(value) => value,
        Err(error) => {
            startup.collector_mut().abort();
            return Err(error);
        }
    };
    let Prepared {
        rows,
        direct_count,
        lookup,
        tables,
        retained,
    } = prepared;
    // The graph's construction scratch is reserved before allocation. Its own
    // implementation measures overlapping capacities under this ceiling.
    let graph = (|| {
        let graph_budget = limits.maps.max_memory_bytes.min(
            limits
                .max_table_memory_bytes
                .checked_sub(retained)
                .ok_or(Error::Limit)?,
        );
        startup
            .collector_mut()
            .reserve_external(graph_budget)
            .map_err(Error::Collision)?;
        let maps = plan::Maps::from_materialized(
            &rows,
            direct_count,
            plan::Limits {
                max_memory_bytes: graph_budget,
                ..limits.maps
            },
        )
        .map_err(Error::Plan)?;
        let resident = maps.stats().resident_bytes;
        if resident > graph_budget || add(retained, resident)? > limits.max_table_memory_bytes {
            return Err(Error::Limit);
        }
        startup
            .collector_mut()
            .release_external(graph_budget - resident)
            .map_err(Error::Collision)?;
        Ok(maps)
    })();
    let maps = match graph {
        Ok(value) => value,
        Err(error) => {
            startup.collector_mut().abort();
            return Err(error);
        }
    };
    let map_stats = maps.stats();
    // Preserve the complete graph before dropping the borrowed rows. The
    // snapshot overlaps all table/graph storage and remains charged until the
    // successful transfer into Completed immediately before finish.
    let topology = (|| {
        let remaining = limits
            .max_table_memory_bytes
            .checked_sub(add(retained, map_stats.resident_bytes)?)
            .ok_or(Error::Limit)?;
        Topology::snapshot(&maps, startup.collector_mut(), remaining)
    })();
    let topology = match topology {
        Ok(value) => value,
        Err(error) => {
            startup.collector_mut().abort();
            return Err(error);
        }
    };
    let lookup_stats = LookupStats {
        materials: lookup.materials.len(),
        types: lookup.types.len(),
        retained_bytes: add(capacity(&lookup.materials)?, capacity(&lookup.types)?)?,
    };
    let mut context = startup.with_lookup(&lookup).map_err(Error::Startup)?;
    let report = run::extract_shared(&mut context, &maps, sink, limits.run).map_err(Error::Run)?;
    // End the borrowed lookup lifetime before dropping its owned vectors. No
    // reconversion takes place, and no model/cache counters are reset.
    let mut context = context.with_lookup(&EMPTY).map_err(Error::Startup)?;
    drop(maps);
    drop(rows);
    drop(lookup);
    context
        .collector_mut()
        .release_external(add(retained, map_stats.resident_bytes)?)
        .map_err(Error::Collision)?;
    // Ownership transfer: topology stays alive in the returned object. It is
    // removed from the collector ledger only because finish rejects externals.
    context
        .collector_mut()
        .release_external(topology.resident_bytes())
        .map_err(Error::Collision)?;
    let startup = context.finish().map_err(Error::Startup)?;
    let resident_bytes = add(
        add(
            startup.extraction().resident_bytes(),
            startup.data().stats().resident_bytes,
        )?,
        topology.resident_bytes(),
    )?;
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
struct Prepared {
    rows: Vec<source::DirectRecord<source::VMapFields>>,
    direct_count: usize,
    lookup: Lookups,
    tables: [TableFacts; 3],
    retained: usize,
}
struct Lookups {
    materials: Vec<(u32, source::LiquidMaterialFields)>,
    types: Vec<(u32, source::LiquidTypeFields)>,
}
impl LiquidLookup for Lookups {
    fn material_id(&self, type_id: u32) -> Option<u8> {
        self.types
            .binary_search_by_key(&type_id, |r| r.0)
            .ok()
            .map(|i| self.types[i].1.material_id)
    }
    fn material_flags(&self, material_id: u8) -> Option<u32> {
        self.materials
            .binary_search_by_key(&u32::from(material_id), |r| r.0)
            .ok()
            .map(|i| self.materials[i].1.flags_bits)
    }
}
fn native_failure(e: input::Failure<super::Error>) -> Error {
    match e {
        input::Failure::World(e) => e,
        input::Failure::Collector(e) => Error::Collision(e),
    }
}
fn prepare(startup: &mut gameobjects::Startup<'_>, limits: Limits) -> Result<Prepared, Error> {
    input::prepare(startup.collector_mut(), limits).map_err(native_failure)
}
#[cfg(test)]
fn replay<T: Copy + Default>(
    startup: &mut gameobjects::Startup<'_>,
    snapshot: &source::SourceRecords<T>,
    limits: Limits,
    retained: usize,
) -> Result<Vec<(u32, T)>, Error> {
    input::replay(startup.collector_mut(), snapshot, limits, retained).map_err(native_failure)
}
fn capacity<T>(rows: &Vec<T>) -> Result<usize, Error> {
    rows.capacity()
        .checked_mul(size_of::<T>())
        .ok_or(Error::Limit)
}
fn add(a: usize, b: usize) -> Result<usize, Error> {
    a.checked_add(b).ok_or(Error::Limit)
}
fn word(bytes: &[u8], at: usize) -> Option<u32> {
    bytes
        .get(at..at + 4)
        .map(|b| u32::from_le_bytes(b.try_into().unwrap()))
}
