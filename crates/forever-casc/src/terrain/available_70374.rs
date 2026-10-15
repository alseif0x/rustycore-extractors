// SPDX-License-Identifier: GPL-3.0-or-later
//! Explicit Linux70374 disabled-key selected terrain, at most two tiles/one map.
//! Map coverage can be partial; every requested tile must be present and its
//! WDT/root ADT and all liquid inputs must be complete CKey-verified sources.
//! All callbacks are provisional until CompletedSelected, including on closing
//! failure. The selected facade never emits lists; the separate full-map facade
//! emits lists only after complete maps. Neither admits omitted Map sections or
//! establishes whole-world, publication, restart or product acceptance.
use super::{plan, run, tilelist};
use crate::local_store::{self, LocalStore, VerifiedFile, availability as acquired};
use forever_db2::{RecordError, availability as db2, catalog, terrain_sources as source};
use forever_terrain::{Wdt, conversion, liquid};
use std::{collections::BTreeMap, mem::size_of};

#[cfg(test)]
mod tests;

pub const PROFILE: &str = "linux70374-selected-terrain-available-map-v1";
/// Whole requested maps in the admitted/materialized Map domain, not all client sections.
pub const FULL_MAP_PROFILE: &str = "linux70374-full-maps-available-map-v1";

/// Separate scalable ceilings, no larger than the existing full terrain runner.
/// Every nested override is downward-only. Selected Limits remain unchanged.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FullMapLimits {
    pub resources: run::Limits,
}
const MIB: usize = 1024 * 1024;
const SCRATCH: usize = 4 * MIB;

/// Downward limits only; read/source counters are logical, not physical I/O.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub resources: run::Limits,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            resources: run::Limits {
                max_reads: 7,
                max_source_bytes: 97 * MIB as u64,
                max_output_bytes: 2 * forever_terrain::maps::MAX_MAP_BYTES as u64,
                plan: plan::Limits {
                    max_maps: 1,
                    max_explicit_tiles: 2,
                    max_total_coordinates: 2,
                    ..plan::Limits::default()
                },
                ..run::Limits::default()
            },
        }
    }
}
impl Limits {
    fn validate(self) -> Result<(), Error> {
        let a = self.resources;
        let d = Self::default().resources;
        if a.max_db2_bytes > d.max_db2_bytes
            || a.max_wdt_bytes > d.max_wdt_bytes
            || a.max_direct_rows > d.max_direct_rows
            || a.max_copy_records > d.max_copy_records
            || a.max_lookup_rows > d.max_lookup_rows
            || a.max_table_bytes > d.max_table_bytes
            || a.max_decode_allowance > d.max_decode_allowance
            || a.max_reads > d.max_reads
            || a.max_source_bytes > d.max_source_bytes
            || a.max_output_bytes > d.max_output_bytes
            || a.max_memory_bytes > d.max_memory_bytes
            || a.plan.max_maps > d.plan.max_maps
            || a.plan.max_explicit_tiles > d.plan.max_explicit_tiles
            || a.plan.max_total_coordinates > d.plan.max_total_coordinates
            || a.conversion.max_input_bytes > d.conversion.max_input_bytes
            || a.conversion.max_chunks > d.conversion.max_chunks
            || a.conversion.max_output_bytes > d.conversion.max_output_bytes
        {
            return Err(Error::Run(run::Error::InvalidLimits));
        }
        a.plan
            .validate()
            .map_err(|e| Error::Run(run::Error::Plan(e)))
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Run(run::Error),
    Available(acquired::Error),
    Build,
    Catalog,
    AbsentTile,
    FileIdRouting,
    FullMapSelection,
}
impl Error {
    pub fn code(self) -> &'static str {
        match self {
            Self::Run(e) => e.code(),
            Self::Available(e) => e.code(),
            Self::Build => "terrain_70374_build",
            Self::Catalog => "terrain_70374_catalog",
            Self::AbsentTile => "terrain_70374_requested_tile_absent",
            Self::FileIdRouting => "terrain_70374_file_id_routing_required",
            Self::FullMapSelection => "terrain_70374_full_map_selection_required",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Failure {
    pub error: Error,
    pub step: run::Step,
    pub stats: run::Stats,
}
impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.error.code())
    }
}
impl std::error::Error for Failure {}

/// Separate facts: an available Map is never represented as VerifiedFile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MapFacts {
    pub declared_bytes: u64,
    pub available_bytes: u64,
    pub full_ckey_verified: bool,
    pub encoded_key_authenticated: bool,
    pub work: acquired::Work,
    pub direct_rows: usize,
    pub copy_records: usize,
    pub materialized_rows: usize,
    pub sections: [Option<db2::SectionCoverage>; 16],
}
/// Complete input witness, exposed only AFTER successful Rust conversion.
/// Borrowed bytes are CKey-verified; no available Map buffer/holes are exposed.
/// Caller copies and writes remain provisional until CompletedSelected and need
/// their own capacity/storage budget. No extra source acquisition occurs.
pub struct NativeWitness<'a> {
    pub adt: &'a [u8],
    pub materials: &'a [(u32, source::LiquidMaterialFields)],
    pub types: &'a [(u32, source::LiquidTypeFields)],
    pub context: conversion::Context,
    pub source: run::SourceFacts,
}
pub trait Sink {
    fn map_source(&mut self, _facts: MapFacts) -> Result<(), run::SinkError> {
        Ok(())
    }
    fn source(&mut self, _facts: run::SourceFacts) -> Result<(), run::SinkError> {
        Ok(())
    }
    fn native_witness(&mut self, _witness: NativeWitness<'_>) -> Result<(), run::SinkError> {
        Ok(())
    }
    /// Full-map lists are provisional until CompletedFullMaps. Default rejection
    /// prevents a selected-only sink silently dropping required whole-map output.
    fn tile_list(
        &mut self,
        _map_id: u32,
        _list: &tilelist::TileList,
    ) -> Result<(), run::SinkError> {
        Err(run::SinkError)
    }
    fn tile(
        &mut self,
        context: conversion::Context,
        source: run::SourceFacts,
        tile: &conversion::ConvertedTile,
    ) -> Result<(), run::SinkError>;
}
pub struct CompletedSelected {
    plan: plan::Completion,
    stats: run::Stats,
    map: MapFacts,
    liquids: [run::TableFacts; 3],
}
impl CompletedSelected {
    pub const fn build(&self) -> u32 {
        70374
    }
    pub const fn plan(&self) -> plan::Completion {
        self.plan
    }
    pub const fn stats(&self) -> run::Stats {
        self.stats
    }
    pub const fn map(&self) -> MapFacts {
        self.map
    }
    pub const fn liquids(&self) -> &[run::TableFacts; 3] {
        &self.liquids
    }
}

/// Completion of the entire chosen admitted Map domain. Earlier callbacks are
/// provisional on any failure, including final source closure. Global MapFacts
/// disclose omitted sections; AllMaps never means those sections were decoded.
pub struct CompletedFullMaps {
    completed: CompletedSelected,
}
impl CompletedFullMaps {
    pub const fn profile(&self) -> &'static str {
        FULL_MAP_PROFILE
    }
    pub const fn build(&self) -> u32 {
        70374
    }
    pub const fn plan(&self) -> plan::Completion {
        self.completed.plan
    }
    pub const fn stats(&self) -> run::Stats {
        self.completed.stats
    }
    pub const fn map(&self) -> MapFacts {
        self.completed.map
    }
    pub const fn liquids(&self) -> &[run::TableFacts; 3] {
        &self.completed.liquids
    }
}

/// Complete each requested map or the full admitted/materialized Map domain.
/// Tiles selections are rejected; absence in MAIN is normal for whole maps.
/// Known-present missing/invalid dependencies fail the whole selection, never a
/// successful prefix. Startup sources are loaded once and ADTs remain serial.
pub fn run_full_maps(
    store: &mut LocalStore,
    selection: plan::Selection<'_>,
    pack: bool,
    limits: FullMapLimits,
    sink: &mut impl Sink,
) -> Result<CompletedFullMaps, Failure> {
    let mut r = Runner {
        store,
        sink,
        limits: limits.resources,
        stats: run::Stats::default(),
        step: run::Step::Admission,
        retained: 0,
    };
    let result = limits
        .resources
        .validate()
        .map_err(Error::Run)
        .and_then(|()| {
            if matches!(selection, plan::Selection::Tiles(_)) {
                return Err(Error::FullMapSelection);
            }
            r.execute(selection, pack, true)
        });
    let closing = r.store.verify_unchanged().map_err(store_error);
    match (result, closing) {
        (_, Err(error)) | (Err(error), _) => Err(Failure {
            error,
            step: r.step,
            stats: r.stats,
        }),
        (Ok(completed), Ok(())) => Ok(CompletedFullMaps { completed }),
    }
}

pub fn run_selected(
    store: &mut LocalStore,
    requests: &[plan::TileRequest],
    pack: bool,
    limits: Limits,
    sink: &mut impl Sink,
) -> Result<CompletedSelected, Failure> {
    let mut r = Runner {
        store,
        sink,
        limits: limits.resources,
        stats: run::Stats::default(),
        step: run::Step::Admission,
        retained: 0,
    };
    let result = limits
        .validate()
        .and_then(|()| r.execute(plan::Selection::Tiles(requests), pack, false));
    // Always attempt final source closure, including all early/partial failures.
    let closing = r
        .store
        .verify_unchanged()
        .map_err(|e| Error::Run(run::Error::Store(e)));
    match (result, closing) {
        (_, Err(error)) | (Err(error), _) => Err(Failure {
            error,
            step: r.step,
            stats: r.stats,
        }),
        (Ok(done), Ok(())) => Ok(done),
    }
}
struct Runner<'a, S> {
    store: &'a mut LocalStore,
    sink: &'a mut S,
    limits: run::Limits,
    stats: run::Stats,
    step: run::Step,
    retained: usize,
}
struct Tables {
    maps: db2::MaterializedMaps,
    materials: Vec<(u32, source::LiquidMaterialFields)>,
    types: Vec<(u32, source::LiquidTypeFields)>,
    map_facts: MapFacts,
    facts: [run::TableFacts; 3],
}
impl liquid::LiquidLookup for Tables {
    fn liquid_type(&self, id: u32) -> Option<liquid::LiquidType> {
        let v = self.types[self.types.binary_search_by_key(&id, |r| r.0).ok()?].1;
        Some(liquid::LiquidType {
            sound_bank: v.sound_bank,
            material_id: v.material_id,
        })
    }
    fn material(&self, id: u32) -> Option<liquid::LiquidMaterial> {
        let v = self.materials[self.materials.binary_search_by_key(&id, |r| r.0).ok()?].1;
        Some(liquid::LiquidMaterial {
            flags: v.flags_bits,
            vertex_format: v.lvf,
        })
    }
}
impl<S: Sink> Runner<'_, S> {
    fn execute(
        &mut self,
        selection: plan::Selection<'_>,
        pack: bool,
        full_maps: bool,
    ) -> Result<CompletedSelected, Error> {
        self.store.verify_unchanged().map_err(store_error)?;
        if self.store.build().build != 70374 {
            return Err(Error::Build);
        }
        if !full_maps {
            let plan::Selection::Tiles(requests) = selection else {
                return Err(Error::FullMapSelection);
            };
            // Shape-only admission before acquisition, then planner binds actual rows.
            if requests.is_empty() || requests.len() > self.limits.plan.max_explicit_tiles {
                return Err(Error::Run(run::Error::Plan(if requests.is_empty() {
                    plan::Error::EmptySelection
                } else {
                    plan::Error::Limit
                })));
            }
            if requests.iter().any(|t| t.x >= 64 || t.y >= 64) {
                return Err(Error::Run(run::Error::Plan(
                    plan::Error::InvalidCoordinates,
                )));
            }
            if requests.iter().any(|t| t.map_id != requests[0].map_id) {
                return Err(Error::Run(run::Error::Plan(plan::Error::Limit)));
            }
            if requests.len() == 2 && requests[0] == requests[1] {
                return Err(Error::Run(run::Error::Plan(plan::Error::DuplicateRequest)));
            }
        }
        catalog_gate()?;
        let tables = self.tables()?;
        self.retained = add(
            self.retained,
            add(
                self.limits.plan.max_planner_bytes,
                self.limits.plan.max_expansion_bytes,
            )?,
        )?;
        self.memory(self.retained)?;
        let planned =
            plan::WdtPlan::from_materialized(tables.maps.rows(), selection, self.limits.plan)
                .map_err(plan_error)?;
        let mut expansion = planned.expansion();
        for map in planned.maps() {
            self.step = run::Step::Wdt { map: map.map_id };
            let file = self.read(
                map.file_data_id,
                self.limits.max_wdt_bytes,
                add(self.retained, 256 * 1024)?,
            )?;
            let wdt = Wdt::parse(
                &file.bytes,
                forever_terrain::Limits {
                    max_file_bytes: self.limits.max_wdt_bytes,
                    ..Default::default()
                },
            )
            .map_err(|e| Error::Run(run::Error::Terrain(e)))?;
            if !wdt
                .mphd()
                .is_some_and(|h| h.flags & forever_terrain::wdt::FILE_ID_REFERENCES != 0)
            {
                return Err(Error::FileIdRouting);
            }
            let tiles = expansion
                .expand_next(file.file_data_id, &wdt)
                .map_err(plan_error)?;
            if !full_maps && tiles.absent_tiles() != 0 {
                return Err(Error::AbsentTile);
            }
            // Preflight all selected roots before converting any tile.
            if tiles.tiles().iter().filter(|t|t.present()).any(|t|!matches!(t.source(),plan::TileSource::FileDataId(id) if *id!=0 && *id!=u32::MAX)) {return Err(Error::FileIdRouting);}
            let mut list = if full_maps {
                // Reserve every known remaining read and minimum framing output
                // before converting this map; actual lengths charge cumulatively.
                let reads = self
                    .stats
                    .read_attempts
                    .checked_add(tiles.present_tiles() as u64)
                    .ok_or(limit())?;
                let minimum = (tiles.present_tiles() as u64)
                    .checked_mul(44)
                    .and_then(|n| n.checked_add(tilelist::BYTES as u64))
                    .and_then(|n| n.checked_add(self.stats.output_bytes))
                    .ok_or(limit())?;
                if reads > self.limits.max_reads || minimum > self.limits.max_output_bytes {
                    return Err(limit());
                }
                self.memory(add(
                    self.retained,
                    add(self.limits.max_wdt_bytes, 256 * 1024)?,
                )?)?;
                Some(tilelist::TileList::new_70374(70374).map_err(|_| Error::Build)?)
            } else {
                None
            };
            drop(wdt);
            drop(file);
            for tile in tiles.tiles().iter().filter(|tile| tile.present()) {
                let plan::TileSource::FileDataId(id) = tile.source() else {
                    return Err(Error::FileIdRouting);
                };
                self.step = run::Step::Adt {
                    map: map.map_id,
                    x: tile.x(),
                    y: tile.y(),
                };
                let file = self.read(
                    *id,
                    self.limits.conversion.max_input_bytes,
                    add(self.retained, SCRATCH)?,
                )?;
                let context = conversion::Context {
                    map_id: map.map_id,
                    tile_x: tile.x(),
                    tile_y: tile.y(),
                    build: 70374,
                    pack,
                };
                let output = conversion::convert_adt_70374(
                    &file.bytes,
                    &tables,
                    context,
                    self.limits.conversion,
                )
                .map_err(|e| Error::Run(run::Error::Conversion(e)))?;
                self.output(output.bytes().len())?;
                self.sink
                    .tile(context, self.facts(&file), &output)
                    .map_err(|_| Error::Run(run::Error::Sink))?;
                self.sink
                    .native_witness(NativeWitness {
                        adt: &file.bytes,
                        materials: &tables.materials,
                        types: &tables.types,
                        context,
                        source: self.facts(&file),
                    })
                    .map_err(|_| Error::Run(run::Error::Sink))?;
                self.stats.converted_tiles += 1;
                self.store.verify_unchanged().map_err(store_error)?;
                if let Some(list) = &mut list {
                    list.set(tile.x(), tile.y())
                        .map_err(|_| Error::FileIdRouting)?;
                }
            }
            if let Some(list) = list {
                self.step = run::Step::TileList { map: map.map_id };
                self.output(tilelist::BYTES)?;
                self.sink
                    .tile_list(map.map_id, &list)
                    .map_err(|_| Error::Run(run::Error::Sink))?;
                self.stats.tile_lists += 1;
                self.store.verify_unchanged().map_err(store_error)?;
            }
        }
        self.step = run::Step::Finish;
        let plan = expansion.finish().map_err(plan_error)?;
        Ok(CompletedSelected {
            plan,
            stats: self.stats,
            map: tables.map_facts,
            liquids: tables.facts,
        })
    }
    fn tables(&mut self) -> Result<Tables, Error> {
        self.step = run::Step::Map;
        let cap = self.begin_read(self.limits.max_db2_bytes, self.retained)?;
        let file = self
            .store
            .read_file_data_id_available_with_external_budget(
                run::MAP_ID,
                cap,
                self.retained,
                self.limits.max_memory_bytes,
            )
            .map_err(Error::Available)?;
        self.stats.source_bytes = self
            .stats
            .source_bytes
            .checked_add(file.declared_bytes())
            .ok_or(limit())?;
        let allowance = self.allowance(
            usize::try_from(file.declared_bytes()).map_err(|_| limit())?,
            true,
        )?;
        self.memory(add(
            self.retained,
            add(file.resident_bytes().map_err(Error::Available)?, allowance)?,
        )?)?;
        let rows = db2::read_map_70374(&file).map_err(db2_error)?;
        let (declared_rows, declared_copies) =
            rows.sections()
                .iter()
                .try_fold((0usize, 0usize), |(n, c), s| {
                    Ok::<_, Error>((
                        add(n, s.declared_records as usize)?,
                        add(c, s.declared_copies as usize)?,
                    ))
                })?;
        if declared_rows > self.limits.max_direct_rows
            || declared_copies > self.limits.max_copy_records
        {
            return Err(limit());
        }
        let mut facts = MapFacts {
            declared_bytes: file.declared_bytes(),
            available_bytes: file.available_bytes(),
            full_ckey_verified: file.full_ckey_verified(),
            encoded_key_authenticated: file.encoded_key_authenticated(),
            work: file.work(),
            direct_rows: rows.direct_records().len(),
            copy_records: rows.copies().len(),
            materialized_rows: 0,
            sections: [None; 16],
        };
        for (slot, s) in facts.sections.iter_mut().zip(rows.sections()) {
            *slot = Some(*s);
        }
        let maps = rows.materialize().map_err(db2_error)?;
        facts.materialized_rows = maps.rows().len();
        self.store.verify_unchanged().map_err(store_error)?;
        drop(file);
        self.retain(maps.resident_bytes().map_err(db2_error)?)?;
        self.sink
            .map_source(facts)
            .map_err(|_| Error::Run(run::Error::Sink))?;
        let file = self.table_file(run::Step::Material, run::MATERIAL_ID, false)?;
        let rows = source::read_liquid_materials(&file.bytes).map_err(db2_error)?;
        let mut mat = self.table_facts(&file, &rows)?;
        let tree = rows.materialize().map_err(db2_error)?;
        let materials = compact(tree, self.limits.max_lookup_rows)?;
        mat.materialized_rows = materials.len();
        self.retain(capacity(&materials)?)?;
        drop(file);
        let file = self.table_file(run::Step::Object, run::OBJECT_ID, false)?;
        let rows = source::read_liquid_objects(&file.bytes).map_err(db2_error)?;
        let mut obj = self.table_facts(&file, &rows)?;
        let objects = rows.materialize().map_err(db2_error)?;
        if objects.len() > self.limits.max_lookup_rows {
            return Err(limit());
        }
        obj.materialized_rows = objects.len();
        drop(objects);
        drop(file);
        let file = self.table_file(run::Step::Type, run::TYPE_ID, true)?;
        let rows = source::read_liquid_types(&file.bytes).map_err(db2_error)?;
        let mut ty = self.table_facts(&file, &rows)?;
        let types = compact(
            rows.materialize().map_err(db2_error)?,
            self.limits.max_lookup_rows,
        )?;
        ty.materialized_rows = types.len();
        self.retain(capacity(&types)?)?;
        self.store.verify_unchanged().map_err(store_error)?;
        Ok(Tables {
            maps,
            materials,
            types,
            map_facts: facts,
            facts: [mat, obj, ty],
        })
    }
    fn allowance(&self, bytes: usize, text: bool) -> Result<usize, Error> {
        let n = add(
            2 * MIB + if text { 64 * MIB } else { 0 },
            bytes.checked_mul(32).ok_or(limit())?,
        )?;
        if n > self.limits.max_decode_allowance {
            return Err(limit());
        }
        Ok(n)
    }
    fn table_file(&mut self, step: run::Step, id: u32, text: bool) -> Result<VerifiedFile, Error> {
        self.step = step;
        let f = self.read(id, self.limits.max_db2_bytes, self.retained)?;
        let name = match id {
            run::MATERIAL_ID => "LiquidMaterial",
            run::OBJECT_ID => "LiquidObject",
            run::TYPE_ID => "LiquidType",
            _ => return Err(Error::Catalog),
        };
        let metadata = catalog::resolve_for(catalog::WOWDBDEFS_70374_CATALOG_ID, name)
            .map_err(|_| Error::Catalog)?
            .ok_or(Error::Catalog)?
            .metadata();
        let raw: &[u8; forever_db2::HEADER_SIZE] = f
            .bytes
            .get(..forever_db2::HEADER_SIZE)
            .ok_or(Error::Catalog)?
            .try_into()
            .map_err(|_| Error::Catalog)?;
        let header = forever_db2::Header::parse(raw).map_err(|_| Error::Catalog)?;
        if metadata
            .table_hash
            .is_some_and(|hash| header.table_hash != hash)
        {
            return Err(Error::Catalog);
        }
        self.memory(add(
            self.retained,
            add(f.bytes.capacity(), self.allowance(f.bytes.len(), text)?)?,
        )?)?;
        Ok(f)
    }
    fn table_facts<T>(
        &self,
        f: &VerifiedFile,
        rows: &source::SourceRecords<T>,
    ) -> Result<run::TableFacts, Error> {
        if rows.direct_records().len() > self.limits.max_direct_rows
            || rows.copies().len() > self.limits.max_copy_records
        {
            return Err(limit());
        }
        Ok(run::TableFacts {
            source: self.facts(f),
            direct_rows: rows.direct_records().len(),
            copy_records: rows.copies().len(),
            materialized_rows: 0,
        })
    }
    fn begin_read(&mut self, cap: usize, external: usize) -> Result<u64, Error> {
        self.memory(external)?;
        if self.stats.read_attempts >= self.limits.max_reads {
            return Err(limit());
        }
        let left = self
            .limits
            .max_source_bytes
            .checked_sub(self.stats.source_bytes)
            .ok_or(limit())?;
        self.stats.read_attempts += 1;
        Ok((cap as u64).min(left))
    }
    fn read(&mut self, id: u32, cap: usize, external: usize) -> Result<VerifiedFile, Error> {
        let cap = self.begin_read(cap, external)?;
        let f = self
            .store
            .read_file_data_id_with_external_budget(id, cap, external, self.limits.max_memory_bytes)
            .map_err(store_error)?;
        self.memory(add(external, f.bytes.capacity())?)?;
        self.stats.source_bytes = self
            .stats
            .source_bytes
            .checked_add(f.bytes.len() as u64)
            .ok_or(limit())?;
        self.sink
            .source(self.facts(&f))
            .map_err(|_| Error::Run(run::Error::Sink))?;
        self.store.verify_unchanged().map_err(store_error)?;
        Ok(f)
    }
    fn facts(&self, f: &VerifiedFile) -> run::SourceFacts {
        run::SourceFacts {
            step: self.step,
            file_data_id: f.file_data_id,
            content_key: f.content_key,
            bytes: f.bytes.len(),
        }
    }
    fn memory(&mut self, external: usize) -> Result<(), Error> {
        if add(self.store.stats().resident_bytes, external)? > self.limits.max_memory_bytes {
            return Err(limit());
        }
        self.stats.max_external_bytes = self.stats.max_external_bytes.max(external);
        Ok(())
    }
    fn retain(&mut self, n: usize) -> Result<(), Error> {
        let total = add(self.retained, n)?;
        if total > self.limits.max_table_bytes {
            return Err(limit());
        }
        self.memory(total)?;
        self.retained = total;
        Ok(())
    }
    fn output(&mut self, n: usize) -> Result<(), Error> {
        let total = self
            .stats
            .output_bytes
            .checked_add(n as u64)
            .ok_or(limit())?;
        if total > self.limits.max_output_bytes {
            return Err(limit());
        }
        self.stats.output_bytes = total;
        Ok(())
    }
}
fn catalog_gate() -> Result<(), Error> {
    for (name, id) in [
        ("LiquidMaterial", run::MATERIAL_ID),
        ("LiquidObject", run::OBJECT_ID),
        ("LiquidType", run::TYPE_ID),
    ] {
        let current = catalog::resolve_for(catalog::WOWDBDEFS_70374_CATALOG_ID, name)
            .map_err(|_| Error::Catalog)?
            .ok_or(Error::Catalog)?;
        let old = catalog::resolve_for(catalog::WOWDBDEFS_70338_CATALOG_ID, name)
            .map_err(|_| Error::Catalog)?
            .ok_or(Error::Catalog)?;
        let a = current.metadata();
        let b = old.metadata();
        if a.file_data_id != id
            || !a.shape_compatible()
            || a.layout_hash != b.layout_hash
            || a.index != b.index
            || a.parent_index != b.parent_index
            || a.field_count != b.field_count
            || a.file_field_count != b.file_field_count
            || a.fields != b.fields
        {
            return Err(Error::Catalog);
        }
    }
    Ok(())
}
fn compact<T>(tree: BTreeMap<u32, T>, cap: usize) -> Result<Vec<(u32, T)>, Error> {
    if tree.len() > cap {
        return Err(limit());
    }
    let mut v = Vec::new();
    v.try_reserve_exact(tree.len())
        .map_err(|_| Error::Run(run::Error::Allocation))?;
    v.extend(tree);
    Ok(v)
}
fn capacity<T>(v: &Vec<T>) -> Result<usize, Error> {
    v.capacity().checked_mul(size_of::<T>()).ok_or(limit())
}
fn add(a: usize, b: usize) -> Result<usize, Error> {
    a.checked_add(b).ok_or(limit())
}
fn limit() -> Error {
    Error::Run(run::Error::Limit)
}
fn store_error(e: local_store::Error) -> Error {
    Error::Run(run::Error::Store(e))
}
fn db2_error(e: RecordError) -> Error {
    Error::Run(run::Error::Db2(e))
}
fn plan_error(e: plan::Error) -> Error {
    Error::Run(run::Error::Plan(e))
}
