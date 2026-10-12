// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: closed shared-world to collision-artifact integration.
//! The sole public constructor consumes an authenticated startup, acquires the
//! remaining world with the SAME collector, seals a private placement sink and
//! assembles the complete package. No caller-supplied stage/proof is accepted.
//! Pin 02245dcd245e7433e524577656177723d3e4992e: TileAssembler.cpp60-465;
//! MapTree.cpp200-223,292-367. Names include ALL placement records before ID
//! deduplication, plus successful startup list rows. Unused acquired dependencies
//! are not converted. Bound completion is restricted to unbounded M2 records.
//!
//! Parent-only tiles retain their child index and use the FIRST existing payload
//! along the real effective-parent chain; a present mismatch is terminal. This
//! is separate from extractor cache fallback. Only parent bit2 may differ.
//! Effective parent provenance cannot prove the native Parent/Cosmetic assertion.
//! Empty-map serialization is supported, not runtime LoadMapTile acceptance.
//!
//! Before acquisition, the collector ceiling is reduced by the full staging
//! allowance. Afterwards the ledger includes final world/store residency, stage,
//! indexes, placements, component scratch/output and retained ordered proofs.
//! Vec capacities and growth overlap are charged. Existing DB2 decode allowance
//! remains conservative, NOT an exact allocator/RSS guarantee. Bounded stack and
//! allocator overhead are excluded. Work counters separately cover caller RAW
//! passes, aggregate geometry, BIH constructor work and inherited proof checks;
//! they are not a machine-instruction count. No files are published here.
use super::{Class, Model, Status, gameobjects, run, world};
use forever_vmap::{RawM2, assembly, gameobject_models, placement, vmo, wmo::raw};
use std::{fmt, mem::size_of};
#[cfg(target_os = "linux")]
pub mod staged;
#[cfg(test)]
mod tests;

pub const MAX_MEMORY_BYTES: usize = 1024 * 1024 * 1024;
pub const MAX_STAGE_BYTES: usize = 256 * 1024 * 1024;
pub const MAX_OUTPUT_BYTES: usize = 512 * 1024 * 1024;
pub const MAX_FILES: usize = 65_536;
pub const MAX_MAPS: usize = 4096;
pub const MAX_MODELS: usize = 4096;
pub const MAX_RECORDS: usize = 1_000_000;
pub const MAX_GEOMETRY: usize = 16_000_000;
pub const MAX_ASSOCIATIONS: usize = 16_000_000;
pub const MAX_RAW_VISIT_BYTES: u64 = 1024 * 1024 * 1024;
pub const MAX_BIH_WORK: u64 = 128_000_000;
pub const MAX_ANCESTOR_STEPS: usize = 16_000_000;
pub const MAX_VALIDATION_BYTES: u64 = 4 * 1024 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub max_memory_bytes: usize,
    pub max_stage_bytes: usize,
    pub max_output_bytes: usize,
    pub max_files: usize,
    pub max_maps: usize,
    pub max_models: usize,
    pub max_records: usize,
    pub max_vertices: usize,
    pub max_triangles: usize,
    pub max_associations: usize,
    pub max_raw_visit_bytes: u64,
    pub max_bih_work: u64,
    pub max_ancestor_steps: usize,
    pub max_validation_bytes: u64,
    pub world: world::Limits,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_memory_bytes: MAX_MEMORY_BYTES,
            max_stage_bytes: MAX_STAGE_BYTES,
            max_output_bytes: MAX_OUTPUT_BYTES,
            max_files: MAX_FILES,
            max_maps: MAX_MAPS,
            max_models: MAX_MODELS,
            max_records: MAX_RECORDS,
            max_vertices: MAX_GEOMETRY,
            max_triangles: MAX_GEOMETRY,
            max_associations: MAX_ASSOCIATIONS,
            max_raw_visit_bytes: MAX_RAW_VISIT_BYTES,
            max_bih_work: MAX_BIH_WORK,
            max_ancestor_steps: MAX_ANCESTOR_STEPS,
            max_validation_bytes: MAX_VALIDATION_BYTES,
            world: world::Limits::default(),
        }
    }
}
impl Limits {
    fn validate(self) -> Result<(), Error> {
        if self.max_memory_bytes > MAX_MEMORY_BYTES
            || self.max_stage_bytes > MAX_STAGE_BYTES
            || self.max_output_bytes > MAX_OUTPUT_BYTES
            || self.max_files > MAX_FILES
            || self.max_maps > MAX_MAPS
            || self.max_models > MAX_MODELS
            || self.max_records > MAX_RECORDS
            || self.max_vertices > MAX_GEOMETRY
            || self.max_triangles > MAX_GEOMETRY
            || self.max_associations > MAX_ASSOCIATIONS
            || self.max_raw_visit_bytes > MAX_RAW_VISIT_BYTES
            || self.max_bih_work > MAX_BIH_WORK
            || self.max_ancestor_steps > MAX_ANCESTOR_STEPS
            || self.max_validation_bytes > MAX_VALIDATION_BYTES
        {
            return Err(Error::InvalidLimits);
        }
        if self.max_stage_bytes > self.max_memory_bytes {
            return Err(Error::Limit);
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidLimits,
    Limit,
    Allocation,
    Inventory,
    Binding,
    Class,
    MissingAncestor,
    Collision(super::Error),
    World(world::Error),
    Placement(placement::Error),
    Bounds(placement::bounds::Error),
    Assembly(assembly::Error),
    Vmo(vmo::Error),
    RawM2(forever_vmap::Error),
    RawWmo(forever_vmap::wmo::Error),
    Lists(gameobject_models::Error),
}
impl Error {
    pub fn code(self) -> &'static str {
        match self {
            Self::InvalidLimits => "collision_assembly_invalid_limits",
            Self::Limit => "collision_assembly_policy_limit",
            Self::Allocation => "collision_assembly_allocation",
            Self::Inventory => "collision_assembly_inventory",
            Self::Binding => "collision_assembly_model_binding",
            Self::Class => "collision_assembly_model_class",
            Self::MissingAncestor => "collision_assembly_missing_ancestor_payload",
            Self::Collision(e) => e.code(),
            Self::World(e) => e.code(),
            Self::Placement(e) => e.code(),
            Self::Bounds(e) => e.code(),
            Self::Assembly(e) => e.code(),
            Self::Vmo(e) => e.code(),
            Self::RawM2(e) => e.code(),
            Self::RawWmo(e) => e.code(),
            Self::Lists(e) => e.code(),
        }
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}
impl std::error::Error for Error {}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    pub maps: usize,
    pub selected_models: usize,
    pub input_records: usize,
    pub vertices: usize,
    pub triangles: usize,
    pub associations: usize,
    pub output_bytes: usize,
    pub files: usize,
    pub proof_bytes: usize,
    pub raw_visit_bytes: u64,
    pub bih_work: u64,
    pub ancestor_steps: usize,
    pub validation_bytes: u64,
    pub resident_bytes: usize,
    pub peak_bytes: usize,
    pub stage_peak_bytes: usize,
}
struct NamedModel {
    source: usize,
    model: vmo::Model,
}
/// Complete owned package. Names and startup dtree borrow their retained world;
/// ordered per-map proofs stay alive with the accepted artifacts.
pub struct Completed {
    world: world::Completed,
    maps: Vec<assembly::AssembledMap>,
    models: Vec<NamedModel>,
    stats: Stats,
}
impl Completed {
    pub const fn world(&self) -> &world::Completed {
        &self.world
    }
    pub fn maps(&self) -> &[assembly::AssembledMap] {
        &self.maps
    }
    pub const fn stats(&self) -> Stats {
        self.stats
    }
    pub fn models(&self) -> impl Iterator<Item = (&[u8], &vmo::Model)> {
        self.models.iter().map(|m| {
            (
                self.world.startup().extraction().models()[m.source].output_basename(),
                &m.model,
            )
        })
    }
    /// Stable inventory: dtree, byte-sorted model names, numeric map IDs, then
    /// each tree and tile payload/index in the accepted builder's tile order.
    pub fn artifacts(&self) -> impl Iterator<Item = Artifact<'_>> {
        let dtree = std::iter::once(Artifact {
            name: ArtifactName::Dtree,
            bytes: self.world.startup().data().lists().dtree_bytes(),
        });
        let models = self.models().map(|(name, m)| Artifact {
            name: ArtifactName::Model(name),
            bytes: m.bytes(),
        });
        let maps = self.maps.iter().flat_map(|map| {
            std::iter::once(Artifact {
                name: ArtifactName::Tree(map.map_id()),
                bytes: map.tree_bytes(),
            })
            .chain(map.tiles().iter().flat_map(move |tile| {
                tile.payload()
                    .map(|bytes| Artifact {
                        name: ArtifactName::Tile {
                            map: map.map_id(),
                            x: tile.x(),
                            y: tile.y(),
                            index: false,
                        },
                        bytes,
                    })
                    .into_iter()
                    .chain(std::iter::once(Artifact {
                        name: ArtifactName::Tile {
                            map: map.map_id(),
                            x: tile.x(),
                            y: tile.y(),
                            index: true,
                        },
                        bytes: tile.index_bytes(),
                    }))
            }))
        });
        dtree.chain(models).chain(maps)
    }
}
#[derive(Clone, Copy)]
pub enum ArtifactName<'a> {
    Dtree,
    Model(&'a [u8]),
    Tree(u32),
    Tile {
        map: u32,
        x: u32,
        y: u32,
        index: bool,
    },
}
impl fmt::Display for ArtifactName<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Dtree => f.write_str("GameObjectModels.dtree"),
            Self::Model(name) => write!(
                f,
                "{}.vmo",
                std::str::from_utf8(name).map_err(|_| fmt::Error)?
            ),
            Self::Tree(id) => write!(f, "{id:04}/{id:04}.vmtree"),
            Self::Tile { map, x, y, index } => write!(
                f,
                "{map:04}/{map:04}_{x:02}_{y:02}.{}",
                if *index { "vmtileidx" } else { "vmtile" }
            ),
        }
    }
}
pub struct Artifact<'a> {
    name: ArtifactName<'a>,
    bytes: &'a [u8],
}
impl<'a> Artifact<'a> {
    pub const fn name(&self) -> ArtifactName<'a> {
        self.name
    }
    pub const fn bytes(&self) -> &'a [u8] {
        self.bytes
    }
}

/// Acquires and assembles atomically in memory. An error consumes the startup;
/// no earlier world, provisional callback, model or assembled map is returned.
pub fn extract(mut startup: gameobjects::Startup<'_>, limits: Limits) -> Result<Completed, Error> {
    limits.validate()?;
    let ceiling = limits
        .max_memory_bytes
        .checked_sub(limits.max_stage_bytes)
        .ok_or(Error::Limit)?;
    let collector = startup.collector_mut();
    collector
        .constrain_memory(ceiling.min(collector.memory_limit()))
        .map_err(Error::Collision)?;
    let mut stage = Stage::new(limits);
    let result = world::extract(startup, &mut stage, limits.world);
    if let Some(error) = stage.error {
        return Err(error);
    }
    let world = result.map_err(Error::World)?;
    build(world, stage, limits)
}

struct MapStage {
    id: u32,
    bytes: Vec<u8>,
}
struct Stage {
    maps: Vec<MapStage>,
    limits: Limits,
    records: usize,
    memory: Ledger,
    error: Option<Error>,
}
impl Stage {
    fn new(limits: Limits) -> Self {
        Self {
            maps: Vec::new(),
            limits,
            records: 0,
            memory: Ledger::new(0, limits.max_stage_bytes).expect("zero ledger"),
            error: None,
        }
    }
    fn append(&mut self, id: u32, bytes: &[u8]) -> Result<(), Error> {
        let parsed = placement::DirBin::parse(bytes, placement::Limits::default())
            .map_err(Error::Placement)?;
        let records = add(self.records, parsed.record_count())?;
        if records > self.limits.max_records {
            return Err(Error::Limit);
        }
        let at = match self.maps.binary_search_by_key(&id, |m| m.id) {
            Ok(at) => at,
            Err(at) => {
                if self.maps.len() >= self.limits.max_maps {
                    return Err(Error::Limit);
                }
                grow(&mut self.maps, 1, &mut self.memory)?;
                self.maps.insert(
                    at,
                    MapStage {
                        id,
                        bytes: Vec::new(),
                    },
                );
                at
            }
        };
        let target = &mut self.maps[at].bytes;
        if add(target.len(), bytes.len())? > placement::MAX_BYTES {
            return Err(Error::Limit);
        }
        grow(target, bytes.len(), &mut self.memory)?;
        target.extend_from_slice(bytes);
        self.records = records;
        Ok(())
    }
}
impl run::Sink for Stage {
    fn model(&mut self, _: &Model) -> Result<(), run::SinkError> {
        Ok(())
    }
    fn placements(&mut self, context: run::Context, bytes: &[u8]) -> Result<(), run::SinkError> {
        if self.error.is_some() {
            return Err(run::SinkError::Rejected);
        }
        if let Err(error) = self.append(context.map_id, bytes) {
            self.error = Some(error);
            return Err(run::SinkError::Rejected);
        }
        Ok(())
    }
}
struct Ledger {
    live: usize,
    peak: usize,
    cap: usize,
}
impl Ledger {
    fn new(live: usize, cap: usize) -> Result<Self, Error> {
        if live > cap {
            return Err(Error::Limit);
        }
        Ok(Self {
            live,
            peak: live,
            cap,
        })
    }
    fn free(&self) -> usize {
        self.cap - self.live
    }
    fn check(&mut self, extra: usize) -> Result<(), Error> {
        let total = add(self.live, extra)?;
        if total > self.cap {
            return Err(Error::Limit);
        }
        self.peak = self.peak.max(total);
        Ok(())
    }
    fn charge(&mut self, bytes: usize) -> Result<(), Error> {
        self.check(bytes)?;
        self.live += bytes;
        Ok(())
    }
    fn release(&mut self, bytes: usize) {
        self.live = self.live.checked_sub(bytes).expect("owned ledger balance");
    }
}
fn add(a: usize, b: usize) -> Result<usize, Error> {
    a.checked_add(b).ok_or(Error::Limit)
}
fn capacity<T>(v: &Vec<T>) -> Result<usize, Error> {
    v.capacity().checked_mul(size_of::<T>()).ok_or(Error::Limit)
}
fn reserve<T>(count: usize, memory: &mut Ledger) -> Result<Vec<T>, Error> {
    memory.check(count.checked_mul(size_of::<T>()).ok_or(Error::Limit)?)?;
    let mut out = Vec::new();
    out.try_reserve_exact(count)
        .map_err(|_| Error::Allocation)?;
    memory.charge(capacity(&out)?)?;
    Ok(out)
}
fn grow<T>(v: &mut Vec<T>, extra: usize, memory: &mut Ledger) -> Result<(), Error> {
    let needed = add(v.len(), extra)?;
    if needed <= v.capacity() {
        return Ok(());
    }
    let target = needed.max(v.capacity().saturating_mul(2));
    let mut replacement = reserve(target, memory)?;
    replacement.append(v);
    let old = std::mem::replace(v, replacement);
    let oldbytes = capacity(&old)?;
    drop(old);
    memory.release(oldbytes);
    Ok(())
}
#[derive(Clone, Copy)]
struct ModelIndex {
    source: usize,
    selected: bool,
    bounds: Option<[u32; 6]>,
}
fn binding(index: &[ModelIndex], models: &[Model], name: &[u8]) -> Result<usize, Error> {
    let at = index
        .binary_search_by(|item| models[item.source].output_basename().cmp(name))
        .map_err(|_| Error::Binding)?;
    let m = &models[index[at].source];
    if m.status() != Status::Collision || m.raw().is_none() {
        return Err(Error::Binding);
    }
    Ok(at)
}
fn bump(value: &mut usize, amount: usize, cap: usize) -> Result<(), Error> {
    *value = add(*value, amount)?;
    if *value > cap {
        Err(Error::Limit)
    } else {
        Ok(())
    }
}
fn bump64(value: &mut u64, amount: u64, cap: u64) -> Result<(), Error> {
    *value = value.checked_add(amount).ok_or(Error::Limit)?;
    if *value > cap {
        Err(Error::Limit)
    } else {
        Ok(())
    }
}
fn visit(stats: &mut Stats, bytes: usize, limits: Limits) -> Result<(), Error> {
    bump64(
        &mut stats.raw_visit_bytes,
        bytes as u64,
        limits.max_raw_visit_bytes,
    )
}

fn build(world: world::Completed, stage: Stage, limits: Limits) -> Result<Completed, Error> {
    // Only this module can call this function or construct a sealed stage.
    limits.validate()?;
    if stage.error.is_some() {
        return Err(Error::Inventory);
    }
    let active = world
        .topology()
        .all()
        .iter()
        .filter(|entry| entry.active())
        .count();
    if active != stage.maps.len() || active > limits.max_maps {
        return Err(Error::Inventory);
    }
    for map in &stage.maps {
        if !world
            .topology()
            .get(map.id)
            .is_some_and(|entry| entry.active())
        {
            return Err(Error::Inventory);
        }
    }
    let base = add(world.resident_bytes(), world.store_resident_bytes())?;
    let mut memory = Ledger::new(add(base, stage.memory.live)?, limits.max_memory_bytes)?;
    let mut stats = Stats {
        maps: active,
        input_records: stage.records,
        stage_peak_bytes: stage.memory.peak,
        ..Stats::default()
    };
    let source = world.startup().extraction().models();
    if source.len() > MAX_MODELS {
        return Err(Error::Limit);
    }
    let mut index = reserve(source.len(), &mut memory)?;
    index.extend((0..source.len()).map(|source| ModelIndex {
        source,
        selected: false,
        bounds: None,
    }));
    index.sort_unstable_by(|a, b| {
        source[a.source]
            .output_basename()
            .cmp(source[b.source].output_basename())
    });
    if index
        .windows(2)
        .any(|w| source[w[0].source].output_basename() == source[w[1].source].output_basename())
    {
        return Err(Error::Binding);
    }
    // Every record participates in the union, not just the first spawn per ID.
    for map in &stage.maps {
        let view = placement::DirBin::parse(&map.bytes, placement::Limits::default())
            .map_err(Error::Placement)?;
        for record in view.records() {
            let at = binding(&index, source, record.name())?;
            index[at].selected = true;
        }
    }
    let list = gameobject_models::TempList::parse(
        world.startup().data().lists().temp_bytes(),
        gameobject_models::Limits::default(),
    )
    .map_err(Error::Lists)?;
    for row in list.rows() {
        let at = binding(&index, source, row.name())?;
        index[at].selected = true;
    }
    stats.selected_models = index.iter().filter(|m| m.selected).count();
    if stats.selected_models > limits.max_models {
        return Err(Error::Limit);
    }
    // Count aggregate geometry BEFORE conversion; component caps are per group.
    for item in index.iter_mut().filter(|item| item.selected) {
        let model = &source[item.source];
        let bytes = model.raw().ok_or(Error::Binding)?;
        visit(&mut stats, bytes.len(), limits)?;
        match model.class() {
            Class::M2 => {
                let parsed =
                    RawM2::parse(bytes, forever_vmap::Limits::default()).map_err(Error::RawM2)?;
                item.bounds = Some(parsed.bounds_bits());
                bump(
                    &mut stats.vertices,
                    parsed.vertex_count(),
                    limits.max_vertices,
                )?;
                bump(
                    &mut stats.triangles,
                    parsed.triangle_count(),
                    limits.max_triangles,
                )?;
            }
            Class::Wmo => {
                let parsed = raw::RawWmo::parse(bytes, raw::ConversionLimits::default())
                    .map_err(Error::RawWmo)?;
                visit(&mut stats, bytes.len(), limits)?; // the borrowed group iterator revalidates
                for group in parsed.groups() {
                    bump(
                        &mut stats.vertices,
                        group.vertex_count(),
                        limits.max_vertices,
                    )?;
                    bump(
                        &mut stats.triangles,
                        group.triangle_count(),
                        limits.max_triangles,
                    )?;
                }
            }
        }
    }
    let mut maps = reserve(active, &mut memory)?;
    let mut models = reserve(stats.selected_models, &mut memory)?;
    bump(&mut stats.files, 1, limits.max_files)?;
    bump(
        &mut stats.output_bytes,
        world.startup().data().lists().dtree_bytes().len(),
        limits.max_output_bytes,
    )?;
    for map in &stage.maps {
        let view = placement::DirBin::parse(&map.bytes, placement::Limits::default())
            .map_err(Error::Placement)?;
        let mut placements = reserve(view.record_count(), &mut memory)?;
        for record in view.records() {
            let item = &index[binding(&index, source, record.name())?];
            if source[item.source].class() == Class::Wmo && record.bounds_bits().is_none() {
                return Err(Error::Class);
            }
            placements.push(
                placement::bounds::complete(record, item.bounds, placement::Limits::default())
                    .map_err(Error::Bounds)?,
            );
        }
        let mut component = assembly::Limits::default();
        component.max_memory_bytes = component.max_memory_bytes.min(memory.free());
        component.max_output_bytes = component
            .max_output_bytes
            .min(limits.max_output_bytes - stats.output_bytes);
        component.max_associations = component
            .max_associations
            .min(limits.max_associations - stats.associations);
        component.max_spawns = component.max_spawns.min(limits.max_records);
        component.max_tiles = component.max_tiles.min(limits.max_files - stats.files);
        component.bih.max_work = component
            .bih
            .max_work
            .min(limits.max_bih_work - stats.bih_work);
        let assembled = assembly::build(map.id, &placements, component).map_err(Error::Assembly)?;
        let s = assembled.stats();
        memory.check(s.peak_bytes)?;
        memory.charge(s.resident_bytes)?;
        bump(
            &mut stats.files,
            add(1, add(s.tiles, s.payload_tiles)?)?,
            limits.max_files,
        )?;
        bump(
            &mut stats.output_bytes,
            s.output_bytes,
            limits.max_output_bytes,
        )?;
        bump(
            &mut stats.associations,
            s.associations,
            limits.max_associations,
        )?;
        bump64(&mut stats.bih_work, s.bih_work, limits.max_bih_work)?;
        stats.proof_bytes = add(stats.proof_bytes, s.proof_bytes)?;
        maps.push(assembled);
        let bytes = capacity(&placements)?;
        drop(placements);
        memory.release(bytes);
    }
    // Component builders already validated their owned payload/index pairs.
    // Only cross-map pairs need the additional, explicitly charged proof scan.
    verify_parents(world.topology(), &maps, limits, &mut stats, &mut memory)?;
    for item in index.iter().filter(|item| item.selected) {
        let source_model = &source[item.source];
        let bytes = source_model.raw().ok_or(Error::Binding)?;
        visit(&mut stats, bytes.len(), limits)?; // second strict admission
        visit(&mut stats, bytes.len(), limits)?; // component strict RAW re-admission
        if source_model.class() == Class::Wmo {
            // convert_wmo also iterates groups(), whose read_group validates
            // geometry again after the component's own RawWmo::parse.
            visit(&mut stats, bytes.len(), limits)?;
        }
        let mut component = vmo::Limits::default();
        component.max_memory_bytes = component.max_memory_bytes.min(memory.free());
        component.max_bytes = component
            .max_bytes
            .min(limits.max_output_bytes - stats.output_bytes);
        component.max_work = component.max_work.min(limits.max_bih_work - stats.bih_work);
        let converted = match source_model.class() {
            Class::M2 => vmo::convert_m2(
                &RawM2::parse(bytes, forever_vmap::Limits::default()).map_err(Error::RawM2)?,
                component,
            ),
            Class::Wmo => vmo::convert_wmo(
                &raw::RawWmo::parse(bytes, raw::ConversionLimits::default())
                    .map_err(Error::RawWmo)?,
                component,
            ),
        }
        .map_err(Error::Vmo)?;
        let s = converted.stats();
        memory.check(s.peak_bytes)?;
        memory.charge(s.resident_bytes)?;
        bump(&mut stats.files, 1, limits.max_files)?;
        bump(&mut stats.output_bytes, s.bytes, limits.max_output_bytes)?;
        bump64(&mut stats.bih_work, s.build_work, limits.max_bih_work)?;
        models.push(NamedModel {
            source: item.source,
            model: converted,
        });
    }
    let index_bytes = capacity(&index)?;
    drop(index);
    memory.release(index_bytes);
    let stage_bytes = stage.memory.live;
    drop(stage);
    memory.release(stage_bytes);
    stats.resident_bytes = memory.live;
    stats.peak_bytes = memory.peak;
    Ok(Completed {
        world,
        maps,
        models,
        stats,
    })
}
fn verify_parents(
    topology: &world::Topology,
    maps: &[assembly::AssembledMap],
    limits: Limits,
    stats: &mut Stats,
    memory: &mut Ledger,
) -> Result<(), Error> {
    for map in maps {
        for tile in map.tiles().iter().filter(|tile| tile.payload().is_none()) {
            let mut parent = topology
                .get(map.map_id())
                .ok_or(Error::Inventory)?
                .parent_map_id();
            let payload = loop {
                let id = parent.ok_or(Error::MissingAncestor)?;
                bump(&mut stats.ancestor_steps, 1, limits.max_ancestor_steps)?;
                let entry = topology.get(id).ok_or(Error::Inventory)?;
                if let Ok(at) = maps.binary_search_by_key(&id, |map| map.map_id())
                    && let Ok(t) = maps[at]
                        .tiles()
                        .binary_search_by_key(&(tile.x(), tile.y()), |t| (t.x(), t.y()))
                    && let Some(payload) = maps[at].tiles()[t].payload()
                {
                    break payload;
                }
                parent = entry.parent_map_id();
            };
            let bytes = add(
                add(map.ordered_dir_bin().len(), tile.index_bytes().len())?,
                payload.len(),
            )?;
            bump64(
                &mut stats.validation_bytes,
                bytes as u64,
                limits.max_validation_bytes,
            )?;
            // IndexView reserves one u32 per association; all other views borrow.
            // Restrict its allowance to the observed scratch as well: any Vec
            // overcapacity must fail inside the reader, not exceed this peak.
            let count = tile.index_bytes().len().saturating_sub(12) / 4;
            let scratch = count.checked_mul(size_of::<u32>()).ok_or(Error::Limit)?;
            memory.check(scratch)?;
            let component = assembly::Limits {
                max_memory_bytes: memory.free().min(scratch).min(assembly::MAX_MEMORY_BYTES),
                ..assembly::Limits::default()
            };
            assembly::verify_inherited_associations(
                payload,
                tile.index_bytes(),
                map.ordered_dir_bin(),
                component,
            )
            .map_err(Error::Assembly)?;
        }
    }
    Ok(())
}
