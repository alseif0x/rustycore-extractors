// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: complete offline terrain acquisition and conversion.
//! Pinned System.cpp:275-389,1085-1147 startup/selection/output semantics.
//! Reads Map, LiquidMaterial, LiquidObject and LiquidType in that order, then
//! WDT/root ADTs through one already admitted LocalStore. Names require prior
//! explicit opt-in; no fallback, truncated input or skipped present tile.
//!
//! Sink callbacks are PROVISIONAL until run returns Completed. On any failure
//! the caller must discard the whole staged selection, including earlier maps.
//! A Tiles selection never emits tile lists, even if it names all 4096 cells.
//! Sink-owned memory/output lifetime must be bounded separately by the caller.
//! No publication, path resolution, network, manifest or restart policy here.
//!
//! Tables charge retained Vec/String capacities and a conservative decode
//! allowance (2 MiB + 32*source bytes, plus 64 MiB for text). Planner/expansion
//! maxima and 4 MiB conversion scratch are reserved in the combined LocalStore
//! ledger. These are explicit conservative admission budgets, not measured
//! allocator overhead, all CPU operations or portable RSS/OOM guarantees.
use super::{plan, tilelist};
use crate::local_store::{self, LocalStore, VerifiedFile};
use forever_db2::{RecordError, terrain_sources as source};
use forever_terrain::{Wdt, conversion, liquid};

mod tables;
#[cfg(test)]
mod tests;

pub const MAP_ID: u32 = 1_349_477;
pub const MATERIAL_ID: u32 = 1_132_538;
pub const OBJECT_ID: u32 = 1_308_058;
pub const TYPE_ID: u32 = 1_371_380;
const MIB: usize = 1024 * 1024;
const CONVERSION_SCRATCH: usize = 4 * MIB;
const WDT_SCRATCH: usize = 256 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub max_db2_bytes: usize,
    pub max_wdt_bytes: usize,
    pub max_direct_rows: usize,
    pub max_copy_records: usize,
    pub max_lookup_rows: usize,
    pub max_table_bytes: usize,
    pub max_decode_allowance: usize,
    pub max_reads: u64,
    pub max_source_bytes: u64,
    pub max_output_bytes: u64,
    pub max_memory_bytes: usize,
    pub plan: plan::Limits,
    pub conversion: conversion::Limits,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_db2_bytes: 16 * MIB,
            max_wdt_bytes: MIB,
            max_direct_rows: 100_000,
            max_copy_records: 100_000,
            max_lookup_rows: 300_000,
            max_table_bytes: 256 * MIB,
            max_decode_allowance: 66 * MIB + 32 * 16 * MIB,
            max_reads: 1_000_000,
            max_source_bytes: 64 * 1024 * MIB as u64,
            max_output_bytes: 64 * 1024 * MIB as u64,
            max_memory_bytes: 1024 * MIB,
            plan: plan::Limits {
                max_maps: 4096,
                max_total_coordinates: 4096 * 4096,
                ..Default::default()
            },
            conversion: conversion::Limits::default(),
        }
    }
}
impl Limits {
    fn validate(self) -> Result<(), Error> {
        let d = Self::default();
        if self.max_db2_bytes > d.max_db2_bytes
            || self.max_wdt_bytes > d.max_wdt_bytes
            || self.max_direct_rows > d.max_direct_rows
            || self.max_copy_records > d.max_copy_records
            || self.max_lookup_rows > d.max_lookup_rows
            || self.max_table_bytes > d.max_table_bytes
            || self.max_decode_allowance > d.max_decode_allowance
            || self.max_reads > d.max_reads
            || self.max_source_bytes > d.max_source_bytes
            || self.max_output_bytes > d.max_output_bytes
            || self.max_memory_bytes > d.max_memory_bytes
            || self.plan.max_maps > d.plan.max_maps
            || self.plan.max_total_coordinates > d.plan.max_total_coordinates
            || self.conversion.max_input_bytes > d.conversion.max_input_bytes
            || self.conversion.max_chunks > d.conversion.max_chunks
            || self.conversion.max_output_bytes > d.conversion.max_output_bytes
        {
            return Err(Error::InvalidLimits);
        }
        self.plan.validate().map_err(Error::Plan)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidLimits,
    Limit,
    Allocation,
    Sink,
    Store(local_store::Error),
    Db2(RecordError),
    Plan(plan::Error),
    Terrain(forever_terrain::Error),
    Conversion(conversion::Error),
}
impl Error {
    pub fn code(self) -> &'static str {
        match self {
            Self::InvalidLimits => "terrain_run_invalid_limits",
            Self::Limit => "terrain_run_policy_limit",
            Self::Allocation => "terrain_run_allocation",
            Self::Sink => "terrain_run_sink",
            Self::Store(e) => e.code(),
            Self::Db2(e) => e.code(),
            Self::Plan(e) => e.code(),
            Self::Terrain(e) => e.code(),
            Self::Conversion(e) => e.code(),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    Admission,
    Map,
    Material,
    Object,
    Type,
    Wdt { map: u32 },
    Adt { map: u32, x: u8, y: u8 },
    TileList { map: u32 },
    Finish,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    pub read_attempts: u64,
    pub source_bytes: u64,
    pub output_bytes: u64,
    pub converted_tiles: u64,
    pub tile_lists: u64,
    pub max_external_bytes: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Failure {
    pub error: Error,
    pub step: Step,
    pub stats: Stats,
}
impl std::fmt::Display for Failure {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        out.write_str(self.error.code())
    }
}
impl std::error::Error for Failure {}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceFacts {
    pub step: Step,
    pub file_data_id: u32,
    pub content_key: [u8; 16],
    pub bytes: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TableFacts {
    pub source: SourceFacts,
    pub direct_rows: usize,
    pub copy_records: usize,
    pub materialized_rows: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SinkError;
pub trait Sink {
    /// Acquisition identity only: the subsequent parser can still reject it.
    fn source(&mut self, _facts: SourceFacts) -> Result<(), SinkError> {
        Ok(())
    }
    fn tile(
        &mut self,
        context: conversion::Context,
        source: SourceFacts,
        tile: &conversion::ConvertedTile,
    ) -> Result<(), SinkError>;
    fn tile_list(&mut self, map: u32, bytes: &[u8; tilelist::BYTES]) -> Result<(), SinkError>;
}
pub struct Completed {
    build: u32,
    plan: plan::Completion,
    stats: Stats,
    tables: [TableFacts; 4],
}
impl Completed {
    pub const fn build(&self) -> u32 {
        self.build
    }
    pub const fn plan(&self) -> plan::Completion {
        self.plan
    }
    pub const fn stats(&self) -> Stats {
        self.stats
    }
    pub const fn tables(&self) -> &[TableFacts; 4] {
        &self.tables
    }
}
pub fn run(
    store: &mut LocalStore,
    selection: plan::Selection<'_>,
    pack: bool,
    limits: Limits,
    sink: &mut impl Sink,
) -> Result<Completed, Failure> {
    let mut runner = Runner {
        store,
        sink,
        limits,
        stats: Stats::default(),
        step: Step::Admission,
        retained: 0,
    };
    runner.execute(selection, pack).map_err(|error| Failure {
        error,
        step: runner.step,
        stats: runner.stats,
    })
}
struct Runner<'a, S> {
    store: &'a mut LocalStore,
    sink: &'a mut S,
    limits: Limits,
    stats: Stats,
    step: Step,
    retained: usize,
}
impl<S: Sink> Runner<'_, S> {
    fn execute(&mut self, selection: plan::Selection<'_>, pack: bool) -> Result<Completed, Error> {
        self.limits.validate()?;
        self.store.verify_unchanged().map_err(Error::Store)?;
        let build = self.store.build().build;
        let tables = self.tables()?;
        self.step = Step::Admission;
        // Reserve both policies before their first allocation; caller data is
        // already charged in retained. Future reads reuse this same reservation.
        self.retained = add(
            self.retained,
            add(
                self.limits.plan.max_planner_bytes,
                self.limits.plan.max_expansion_bytes,
            )?,
        )?;
        self.memory(self.retained)?;
        let planned = plan::WdtPlan::from_materialized(&tables.maps, selection, self.limits.plan)
            .map_err(Error::Plan)?;
        let mut expansion = planned.expansion();
        for map in planned.maps() {
            self.step = Step::Wdt { map: map.map_id };
            let file = self.read(
                Some(map.file_data_id),
                &[],
                self.limits.max_wdt_bytes,
                add(self.retained, WDT_SCRATCH)?,
            )?;
            let wdt = Wdt::parse(
                &file.bytes,
                forever_terrain::Limits {
                    max_file_bytes: self.limits.max_wdt_bytes,
                    ..Default::default()
                },
            )
            .map_err(Error::Terrain)?;
            let tiles = expansion
                .expand_next(file.file_data_id, &wdt)
                .map_err(Error::Plan)?;
            drop(wdt);
            drop(file);
            let mut list = tilelist::TileList::new(build).map_err(|_| Error::InvalidLimits)?;
            for tile in tiles.tiles() {
                let (id, name) = match tile.source() {
                    plan::TileSource::Absent => continue,
                    plan::TileSource::FileDataId(id) => (Some(*id), &[][..]),
                    plan::TileSource::LogicalName(name) => (None, name.as_bytes()),
                };
                self.step = Step::Adt {
                    map: map.map_id,
                    x: tile.x(),
                    y: tile.y(),
                };
                let external = add(self.retained, CONVERSION_SCRATCH)?;
                let file = self.read(id, name, self.limits.conversion.max_input_bytes, external)?;
                let context = conversion::Context {
                    map_id: map.map_id,
                    tile_x: tile.x(),
                    tile_y: tile.y(),
                    build,
                    pack,
                };
                let output =
                    conversion::convert_adt(&file.bytes, &tables, context, self.limits.conversion)
                        .map_err(Error::Conversion)?;
                self.output(output.bytes().len())?;
                self.sink
                    .tile(context, self.facts(&file), &output)
                    .map_err(|_| Error::Sink)?;
                list.set(tile.x(), tile.y())
                    .map_err(|_| Error::InvalidLimits)?;
                self.stats.converted_tiles += 1;
            }
            if tiles.covers_whole_map() {
                self.step = Step::TileList { map: map.map_id };
                self.output(tilelist::BYTES)?;
                self.sink
                    .tile_list(map.map_id, list.bytes())
                    .map_err(|_| Error::Sink)?;
                self.stats.tile_lists += 1;
            }
        }
        self.step = Step::Finish;
        let plan = expansion.finish().map_err(Error::Plan)?;
        self.store.verify_unchanged().map_err(Error::Store)?;
        Ok(Completed {
            build,
            plan,
            stats: self.stats,
            tables: tables.facts,
        })
    }
    fn read(
        &mut self,
        id: Option<u32>,
        name: &[u8],
        cap: usize,
        external: usize,
    ) -> Result<VerifiedFile, Error> {
        self.memory(external)?;
        if self.stats.read_attempts >= self.limits.max_reads {
            return Err(Error::Limit);
        }
        let left = self
            .limits
            .max_source_bytes
            .checked_sub(self.stats.source_bytes)
            .ok_or(Error::Limit)?;
        self.stats.read_attempts += 1;
        let cap = (cap as u64).min(left);
        let file = match id {
            Some(id) => self.store.read_file_data_id_with_external_budget(
                id,
                cap,
                external,
                self.limits.max_memory_bytes,
            ),
            None => self.store.read_name_with_external_budget(
                name,
                cap,
                external,
                self.limits.max_memory_bytes,
            ),
        }
        .map_err(Error::Store)?;
        self.memory(add(external, file.bytes.capacity())?)?;
        self.stats.source_bytes += file.bytes.len() as u64;
        self.sink
            .source(self.facts(&file))
            .map_err(|_| Error::Sink)?;
        Ok(file)
    }
    fn facts(&self, file: &VerifiedFile) -> SourceFacts {
        SourceFacts {
            step: self.step,
            file_data_id: file.file_data_id,
            content_key: file.content_key,
            bytes: file.bytes.len(),
        }
    }
    fn memory(&mut self, external: usize) -> Result<(), Error> {
        if add(self.store.stats().resident_bytes, external)? > self.limits.max_memory_bytes {
            return Err(Error::Limit);
        }
        self.stats.max_external_bytes = self.stats.max_external_bytes.max(external);
        Ok(())
    }
    fn output(&mut self, bytes: usize) -> Result<(), Error> {
        let total = self
            .stats
            .output_bytes
            .checked_add(bytes as u64)
            .ok_or(Error::Limit)?;
        if total > self.limits.max_output_bytes {
            return Err(Error::Limit);
        }
        self.stats.output_bytes = total;
        Ok(())
    }
}
fn add(a: usize, b: usize) -> Result<usize, Error> {
    a.checked_add(b).ok_or(Error::Limit)
}
