// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: complete local canonical navigation package discovery.
//! Windows strict immutable-directory profile for original producer names (four-digit
//! map IDs and 0..63 tiles). No CASC, network, implicit download or publication.
//! Enumerates admitted map directories completely, binds observations to held
//! directory identities, and runs the existing source-to-final pipeline.
#![cfg(windows)]
use super::{Input, Options, Plan};
use crate::{models, offmesh, pipeline, terrain, tilebuilder::Coordinate};
use forever_db2::terrain_sources::{DirectRecord, NavigationMapFields, SourceRecords};
use forever_vmap::store;
use std::{mem::size_of, path::Path};
mod guard;
pub mod run;
use crate::terrain::load::windows::hash;
#[cfg(test)]
mod tests;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Limit,
    Allocation,
    Path,
    Filename,
    Kind,
    Io,
    InputChanged,
    Inventory(super::Error),
    TerrainSource(terrain::load::SourceError),
    CollisionSource(store::SourceError),
    Store(store::Error),
    Pipeline(pipeline::Error),
}
impl Error {
    fn changed(self) -> bool {
        use pipeline::Error as P;
        use store::{Error as S, SourceError as SE};
        use terrain::load::{Error as T, SourceError as TE};
        matches!(
            self,
            Self::InputChanged
                | Self::TerrainSource(TE::InputChanged)
                | Self::CollisionSource(SE::InputChanged)
                | Self::Store(S::Source(SE::InputChanged) | S::Poisoned)
                | Self::Pipeline(
                    P::Terrain(T::Source(TE::InputChanged))
                        | P::Store(S::Source(SE::InputChanged) | S::Poisoned)
                        | P::Models(models::LoadedError::Store(
                            S::Source(SE::InputChanged) | S::Poisoned
                        ))
                )
        )
    }
    pub const fn code(self) -> &'static str {
        match self {
            Self::Limit => "navigation_discovery_limit",
            Self::Allocation => "navigation_discovery_allocation",
            Self::Path => "navigation_discovery_root_path",
            Self::Filename => "navigation_discovery_filename",
            Self::Kind => "navigation_discovery_file_kind",
            Self::Io => "navigation_discovery_io",
            Self::InputChanged => "navigation_discovery_input_changed",
            Self::Inventory(e) => e.code(),
            Self::TerrainSource(e) => match e {
                terrain::load::SourceError::Io => "navigation_terrain_source_io",
                terrain::load::SourceError::InputChanged => "navigation_terrain_input_changed",
                terrain::load::SourceError::Limit => "navigation_terrain_source_limit",
            },
            Self::CollisionSource(e) => store::Error::Source(e).code(),
            Self::Store(e) => e.code(),
            Self::Pipeline(e) => e.code(),
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}
impl std::error::Error for Error {}
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub entries: usize,
    pub tilelists: usize,
    pub descriptors: usize,
    pub root_bytes: usize,
    /// Snapshot names/entries and temporary file/descriptors Vec capacities.
    /// Includes old/new growth overlap and a fixed 256KiB filesystem allowance.
    /// Plan, caller DB2s, fresh pipeline/source/store capacities are separate.
    pub heap_bytes: usize,
    /// Total metadata calls for opening/discovery; renewed for each verification.
    pub metadata_calls: u64,
    pub plan: super::Limits,
    pub terrain_source: terrain::load::windows::FsLimits,
    pub collision_source: store::windows::FsLimits,
    pub store: store::Limits,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            entries: 1_000_000,
            tilelists: 4096,
            descriptors: 1_000_000,
            root_bytes: 4096,
            heap_bytes: 256 * 1024 * 1024,
            metadata_calls: 8_000_000,
            plan: Default::default(),
            terrain_source: Default::default(),
            collision_source: Default::default(),
            store: Default::default(),
        }
    }
}
impl Limits {
    fn validate(self) -> Result<(), Error> {
        let d = Self::default();
        if self.entries > d.entries
            || self.tilelists > d.tilelists
            || self.descriptors > d.descriptors
            || self.root_bytes > d.root_bytes
            || self.heap_bytes > d.heap_bytes
            || self.metadata_calls > d.metadata_calls
        {
            return Err(Error::Limit);
        }
        self.plan.validate().map_err(Error::Inventory)?;
        self.store.validate().map_err(Error::Store)?;
        let a = self.terrain_source;
        let b = d.terrain_source;
        if a.max_metadata_bytes > b.max_metadata_bytes
            || a.max_entries > b.max_entries
            || a.max_root_bytes > b.max_root_bytes
            || a.max_verify_steps > b.max_verify_steps
        {
            return Err(Error::Limit);
        }
        let a = self.collision_source;
        let b = d.collision_source;
        if a.max_metadata_bytes > b.max_metadata_bytes
            || a.max_entries > b.max_entries
            || a.max_root_bytes > b.max_root_bytes
            || a.max_verify_steps > b.max_verify_steps
        {
            return Err(Error::Limit);
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Statistics {
    pub entries: usize,
    pub tilelists: usize,
    pub tilelist_bytes: usize,
    pub descriptors: usize,
    pub metadata_calls: u64,
    pub resident_capacity: usize,
    pub peak_capacity: usize,
}
struct Budget {
    limits: Limits,
    stats: Statistics,
    used: u64,
}
impl Budget {
    fn new(limits: Limits) -> Result<Self, Error> {
        if limits.heap_bytes < guard::SCRATCH {
            return Err(Error::Limit);
        }
        Ok(Self {
            limits,
            used: 0,
            stats: Statistics {
                resident_capacity: guard::SCRATCH,
                peak_capacity: guard::SCRATCH,
                ..Default::default()
            },
        })
    }
    fn metadata(&mut self) -> Result<(), Error> {
        self.used = self.used.checked_add(1).ok_or(Error::Limit)?;
        if self.used > self.limits.metadata_calls {
            return Err(Error::Limit);
        }
        self.stats.metadata_calls = self
            .stats
            .metadata_calls
            .checked_add(1)
            .ok_or(Error::Limit)?;
        Ok(())
    }
    fn platform(&mut self, path: &Path, operations: u64) -> Result<(), Error> {
        let components = path.components().count() as u64;
        let units = components
            .checked_add(2)
            .and_then(|n| n.checked_mul(32))
            .and_then(|n| n.checked_mul(operations))
            .ok_or(Error::Limit)?;
        if self.used.checked_add(units).ok_or(Error::Limit)? > self.limits.metadata_calls {
            return Err(Error::Limit);
        }
        self.used += units;
        self.stats.metadata_calls = self
            .stats
            .metadata_calls
            .checked_add(units)
            .ok_or(Error::Limit)?;
        Ok(())
    }
    fn reserve<T>(&mut self, v: &mut Vec<T>, capacity: usize, ceiling: usize) -> Result<(), Error> {
        if capacity <= v.capacity() {
            return Ok(());
        }
        if capacity > ceiling {
            return Err(Error::Limit);
        }
        let bytes = capacity.checked_mul(size_of::<T>()).ok_or(Error::Limit)?;
        let reservation = self
            .stats
            .resident_capacity
            .checked_add(bytes)
            .ok_or(Error::Limit)?;
        if reservation > self.limits.heap_bytes {
            return Err(Error::Limit);
        }
        self.stats.peak_capacity = self.stats.peak_capacity.max(reservation);
        let before = v.capacity();
        v.try_reserve_exact(capacity - v.len())
            .map_err(|_| Error::Allocation)?;
        let actual = v
            .capacity()
            .checked_mul(size_of::<T>())
            .ok_or(Error::Limit)?;
        if self
            .stats
            .resident_capacity
            .checked_add(actual)
            .ok_or(Error::Limit)?
            > self.limits.heap_bytes
        {
            return Err(Error::Limit);
        }
        self.stats.peak_capacity = self
            .stats
            .peak_capacity
            .max(self.stats.resident_capacity + actual);
        self.stats.resident_capacity += actual - before * size_of::<T>();
        Ok(())
    }
    fn grow<T>(&mut self, v: &mut Vec<T>, ceiling: usize) -> Result<(), Error> {
        let needed = v.len().checked_add(1).ok_or(Error::Limit)?;
        self.reserve(
            v,
            needed.max(v.capacity().saturating_mul(2)).min(ceiling),
            ceiling,
        )?;
        if needed > v.capacity() || needed > ceiling {
            return Err(Error::Limit);
        }
        Ok(())
    }
    fn release<T>(&mut self, v: &Vec<T>) {
        self.stats.resident_capacity -= v.capacity() * size_of::<T>();
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TileListIdentity {
    pub map_id: u32,
    pub sha256: [u8; 32],
}
struct TileList {
    id: u32,
    bytes: Vec<u8>,
}
#[derive(Clone, Copy)]
enum Descriptor {
    Terrain(usize),
    Map(u32),
    Tile(u32, Coordinate),
}
pub struct Package {
    plan: Plan,
    guard: guard::Guard,
    lists: Vec<TileListIdentity>,
    budget: Budget,
    failed: bool,
}
impl Package {
    pub fn open(
        maps: &Path,
        vmaps: &Path,
        source: &SourceRecords<NavigationMapFields>,
        build: u32,
        options: Options,
        limits: Limits,
    ) -> Result<Self, Error> {
        Self::open_rows(maps, vmaps, source.direct_records(), build, options, limits)
    }
    /// Explicit numeric rows, equivalent to the closed source reader's direct
    /// records. Copies are intentionally not replayed by navigation startup.
    pub fn open_rows(
        maps: &Path,
        vmaps: &Path,
        rows: &[DirectRecord<NavigationMapFields>],
        build: u32,
        options: Options,
        limits: Limits,
    ) -> Result<Self, Error> {
        limits.validate()?;
        let mut budget = Budget::new(limits)?;
        let preflight = Plan::from_rows(rows, &[], build, Options::default(), limits.plan)
            .map_err(Error::Inventory)?;
        if let Some(id) = options.map {
            preflight
                .admits_inventory(id, options)
                .map_err(Error::Inventory)?;
        }
        let maps = guard::Root::new(maps, &mut budget)?;
        let vmaps = guard::Root::new(vmaps, &mut budget)?;
        let mut guard = guard::Guard::new(maps, vmaps);
        let mut files = Vec::new();
        let mut descriptors = Vec::new();
        let mut lists = Vec::new();
        scan_maps(
            &mut guard,
            &preflight,
            options,
            &mut budget,
            &mut files,
            &mut descriptors,
            &mut lists,
        )?;
        scan_vmaps(
            &mut guard,
            &preflight,
            options,
            &mut budget,
            &mut descriptors,
        )?;
        guard.canonical_order();
        guard.verify(&mut budget)?;
        drop(preflight);
        descriptors.sort_unstable_by_key(|d| match *d {
            Descriptor::Terrain(i) => (files[i].id, 0, 0, 0),
            Descriptor::Map(id) => (id, 1, 0, 0),
            Descriptor::Tile(id, t) => (id, 2, t.row, t.column),
        });
        let mut inputs = Vec::new();
        budget.reserve(&mut inputs, descriptors.len(), limits.descriptors)?;
        for d in &descriptors {
            inputs.push(match *d {
                Descriptor::Terrain(i) => Input::Terrain {
                    map_id: files[i].id,
                    bytes: &files[i].bytes,
                },
                Descriptor::Map(map_id) => Input::CollisionMap { map_id },
                Descriptor::Tile(map_id, tile) => Input::CollisionTile { map_id, tile },
            });
        }
        let plan = Plan::from_rows(rows, &inputs, build, options, limits.plan)
            .map_err(Error::Inventory)?;
        budget.release(&inputs);
        drop(inputs);
        budget.release(&descriptors);
        drop(descriptors);
        for f in &files {
            budget.release(&f.bytes);
        }
        budget.release(&files);
        drop(files);
        lists.sort_unstable_by_key(|l| l.map_id);
        budget.used = 0;
        guard.verify(&mut budget)?;
        Ok(Self {
            plan,
            guard,
            lists,
            budget,
            failed: false,
        })
    }
    pub fn plan(&self) -> &Plan {
        &self.plan
    }
    pub fn tilelists(&self) -> &[TileListIdentity] {
        &self.lists
    }
    pub const fn statistics(&self) -> Statistics {
        self.budget.stats
    }
    pub const fn failed(&self) -> bool {
        self.failed
    }
    pub fn verify_unchanged(&mut self) -> Result<(), Error> {
        if self.failed {
            return Err(Error::InputChanged);
        }
        self.budget.used = 0;
        let result = self.guard.verify(&mut self.budget);
        if matches!(result, Err(Error::InputChanged)) {
            self.failed = true;
        }
        result
    }
    /// Fresh rooted providers are bound between complete inventory verifications.
    /// Each call discards provider/store caches, so metadata residency does not
    /// grow across tiles. Complete observed package identities persist here.
    pub fn run_tile(
        &mut self,
        types: models::LiquidTypes<'_>,
        connections: &[offmesh::Source],
        request: pipeline::Request,
        limits: pipeline::Limits,
    ) -> Result<pipeline::Outcome, Error> {
        let result = self.run_tile_inner(types, connections, request, limits);
        if let Err(error) = result.as_ref()
            && error.changed()
        {
            self.failed = true;
        }
        result
    }
    fn run_tile_inner(
        &mut self,
        types: models::LiquidTypes<'_>,
        connections: &[offmesh::Source],
        request: pipeline::Request,
        limits: pipeline::Limits,
    ) -> Result<pipeline::Outcome, Error> {
        limits.validate().map_err(Error::Pipeline)?;
        self.verify_unchanged()?;
        let mut terrain = terrain::load::windows::RootedSource::new(
            self.guard.path(0),
            self.budget.limits.terrain_source,
        )
        .map_err(Error::TerrainSource)?;
        let collision = store::windows::RootedSource::new(
            self.guard.path(1),
            self.budget.limits.collision_source,
        )
        .map_err(Error::CollisionSource)?;
        let mut parents = Vec::new();
        let count = self.plan.parent_rows().count();
        if count > self.budget.limits.store.max_maps {
            return Err(Error::Store(store::Error::Limit));
        }
        self.budget
            .reserve(&mut parents, count, self.budget.limits.store.max_maps)?;
        parents.extend(self.plan.parent_rows());
        let store = store::StaticStore::new(
            collision,
            &parents,
            store::Options {
                load_path_only_models: true,
                ..Default::default()
            },
            self.budget.limits.store,
        );
        self.budget.release(&parents);
        drop(parents);
        let mut collision = store.map_err(Error::Store)?;
        self.verify_unchanged()?;
        let result = pipeline::run(
            &self.plan,
            &mut pipeline::Sources {
                terrain: &mut terrain,
                collision: &mut collision,
            },
            types,
            connections,
            request,
            limits,
        )
        .map_err(Error::Pipeline);
        self.finish_tile(result)
    }
    fn finish_tile<T>(&mut self, result: Result<T, Error>) -> Result<T, Error> {
        let after = self.verify_unchanged();
        if result.as_ref().is_err_and(|error| error.changed()) {
            self.failed = true;
            return result;
        }
        after?;
        result
    }
}
fn map_id(name: &[u8]) -> Result<u32, Error> {
    if name.len() != 4 || !name.iter().all(u8::is_ascii_digit) {
        return Err(Error::Filename);
    }
    Ok(name.iter().fold(0, |v, &d| v * 10 + u32::from(d - b'0')))
}
fn tile(name: &[u8], map: u32) -> Result<Coordinate, Error> {
    if name.len() != 17
        || name[4] != b'_'
        || name[7] != b'_'
        || &name[10..] != b".vmtile"
        || map_id(&name[..4])? != map
        || !name[5..7]
            .iter()
            .chain(&name[8..10])
            .all(u8::is_ascii_digit)
    {
        return Err(Error::Filename);
    }
    let t = Coordinate {
        row: (name[5] - b'0') * 10 + name[6] - b'0',
        column: (name[8] - b'0') * 10 + name[9] - b'0',
    };
    if t.row >= 64 || t.column >= 64 {
        return Err(Error::Filename);
    }
    Ok(t)
}
fn descriptor(d: Descriptor, ds: &mut Vec<Descriptor>, b: &mut Budget) -> Result<(), Error> {
    b.grow(ds, b.limits.descriptors)?;
    ds.push(d);
    b.stats.descriptors += 1;
    Ok(())
}
fn scan_maps(
    g: &mut guard::Guard,
    p: &Plan,
    o: Options,
    b: &mut Budget,
    files: &mut Vec<TileList>,
    ds: &mut Vec<Descriptor>,
    lists: &mut Vec<TileListIdentity>,
) -> Result<(), Error> {
    let entries = g.scan(0, b"", b)?;
    let result = (|| {
        for entry in &entries {
            let name = entry.name.as_slice();
            if !name.ends_with(b".tilelist") {
                continue;
            }
            let id = map_id(&name[..name.len() - 9])?;
            if !p.admits_inventory(id, o).map_err(Error::Inventory)? {
                continue;
            }
            if entry.directory {
                return Err(Error::Kind);
            }
            if files.len() >= b.limits.tilelists {
                return Err(Error::Limit);
            }
            let bytes = g.read_list(0, name, entry.identity, b)?;
            super::validate_tilelist(&bytes, p.build()).map_err(Error::Inventory)?;
            let mut hash = hash::Sha256::new();
            hash.update(&bytes);
            b.grow(lists, b.limits.tilelists)?;
            lists.push(TileListIdentity {
                map_id: id,
                sha256: hash.finish(),
            });
            b.grow(files, b.limits.tilelists)?;
            let index = files.len();
            files.push(TileList { id, bytes });
            descriptor(Descriptor::Terrain(index), ds, b)?;
            b.stats.tilelists += 1;
        }
        Ok(())
    })();
    guard::release_scan(entries, b);
    result
}
fn scan_vmaps(
    g: &mut guard::Guard,
    p: &Plan,
    o: Options,
    b: &mut Budget,
    ds: &mut Vec<Descriptor>,
) -> Result<(), Error> {
    let entries = g.scan(1, b"", b)?;
    let result = (|| {
        for entry in &entries {
            let name = entry.name.as_slice();
            if name.is_empty() || !name.iter().all(u8::is_ascii_digit) {
                continue;
            }
            let id = map_id(name)?;
            if !p.admits_inventory(id, o).map_err(Error::Inventory)? {
                continue;
            }
            if !entry.directory {
                return Err(Error::Kind);
            }
            descriptor(Descriptor::Map(id), ds, b)?;
            let children = g.scan(1, name, b)?;
            let result = (|| {
                for child in &children {
                    let file = child.name.as_slice();
                    if !file.ends_with(b".vmtile") {
                        continue;
                    }
                    if child.directory {
                        return Err(Error::Kind);
                    }
                    descriptor(Descriptor::Tile(id, tile(file, id)?), ds, b)?;
                }
                Ok(())
            })();
            guard::release_scan(children, b);
            result?;
        }
        Ok(())
    })();
    guard::release_scan(entries, b);
    result
}
