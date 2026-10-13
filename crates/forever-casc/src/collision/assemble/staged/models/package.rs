// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-12: closed supplied-world disk package assembly.
//! Consume the actual Prepared owner; no caller placements/models/proofs enter.
//! Complete maps and ordered proofs live in immutable bound private bundles.
//! Parent-only tiles use their CHILD index and the FIRST PRESENT payload in the
//! actual effective-parent chain. A present mismatch is terminal, not fallback.
//! One map build, or one copied child plus one borrowed ancestor, stays active.
//!
//! The original world/store/stage/VMO reservations, two complete new descriptor
//! ceilings, guard and actual map/tile capacities stay charged. Source buffers,
//! placements, component scratch/output, replacement overlap and declared caller
//! allowances are admitted before IO. Counters are logical, not RSS/all CPU.
//! Artifact visitation batches each complete map to avoid rereading a bundle for
//! every tile. No publication, resume, cleanup or live-client snapshot is added.
use super::{Prepared, payload, spool};
use crate::collision::{Class, assemble as resident, run};
use forever_vmap::{assembly, placement};
use std::mem::size_of;
#[cfg(all(test, target_os = "linux"))]
mod tests;
#[cfg(all(test, windows))]
mod windows_tests;
#[cfg(target_os = "linux")]
pub const PROFILE: &str = "linux-private-assembled-vmap-package-v1";
#[cfg(windows)]
pub const PROFILE: &str = "windows-private-assembled-vmap-package-v1";
pub const MAX_MEMORY_BYTES: usize = super::MAX_MEMORY_BYTES;
const HEADER: usize = 64;
const TILE_HEADER: usize = 48;
const GUARD_BYTES: usize = 512 * 1024;
const MAGIC: &[u8; 8] = b"RCEPKG1\0";
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub max_memory_bytes: usize,
    pub max_maps: usize,
    pub max_files: usize,
    pub max_models: usize,
    pub max_records: usize,
    pub max_output_bytes: usize,
    pub max_vertices: usize,
    pub max_triangles: usize,
    pub max_associations: usize,
    pub max_raw_visit_bytes: u64,
    pub max_bih_work: u64,
    pub max_ancestor_steps: usize,
    pub max_validation_bytes: u64,
    pub assembly: assembly::Limits,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_memory_bytes: MAX_MEMORY_BYTES,
            max_maps: resident::MAX_MAPS,
            max_files: resident::MAX_FILES,
            max_models: resident::MAX_MODELS,
            max_records: resident::MAX_RECORDS,
            max_output_bytes: resident::MAX_OUTPUT_BYTES,
            max_vertices: resident::MAX_GEOMETRY,
            max_triangles: resident::MAX_GEOMETRY,
            max_associations: resident::MAX_ASSOCIATIONS,
            max_raw_visit_bytes: resident::MAX_RAW_VISIT_BYTES,
            max_bih_work: resident::MAX_BIH_WORK,
            max_ancestor_steps: resident::MAX_ANCESTOR_STEPS,
            max_validation_bytes: resident::MAX_VALIDATION_BYTES,
            assembly: assembly::Limits::default(),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    InvalidLimits,
    Limit,
    Allocation,
    Binding,
    Class,
    Missing,
    MissingAncestor,
    State,
    Callback,
    Spool(payload::Error),
    Placement(placement::Error),
    Bounds(placement::bounds::Error),
    Assembly(assembly::Error),
}
impl Kind {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidLimits => "collision_package_invalid_limits",
            Self::Limit => "collision_package_limit",
            Self::Allocation => "collision_package_allocation",
            Self::Binding => "collision_package_binding",
            Self::Class => "collision_package_class",
            Self::Missing => "collision_package_missing",
            Self::MissingAncestor => "collision_package_missing_ancestor",
            Self::State => "collision_package_state",
            Self::Callback => "collision_package_callback",
            Self::Spool(e) => e.code(),
            Self::Placement(e) => e.code(),
            Self::Bounds(e) => e.code(),
            Self::Assembly(e) => e.code(),
        }
    }
}
impl From<payload::Error> for Kind {
    fn from(e: payload::Error) -> Self {
        Self::Spool(e)
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Guards {
    pub models: super::Guards,
    pub maps: Option<payload::Error>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cause<E = ()> {
    Models(super::Cause),
    Kind(Kind),
    Callback(E),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Error<E = ()> {
    pub cause: Cause<E>,
    pub guards: Guards,
}
impl<E: Copy> Error<E> {
    pub fn code(self) -> &'static str {
        if let Some(e) = self.guards.models.collector
            && super::super::collector_changed(e)
        {
            return e.code();
        }
        if self.guards.maps == Some(payload::Error::Changed)
            || self.guards.models.archive == Some(payload::Error::Changed)
            || self.guards.models.placements
                == Some(super::super::StageError::Spool(payload::Error::Changed))
        {
            return payload::Error::Changed.code();
        }
        match self.cause {
            Cause::Kind(e) => e.code(),
            Cause::Callback(_) => Kind::Callback.code(),
            Cause::Models(cause) => super::Error {
                cause,
                guards: self.guards.models,
            }
            .code(),
        }
    }
}
impl<E> From<Kind> for Error<E> {
    fn from(e: Kind) -> Self {
        Self {
            cause: Cause::Kind(e),
            guards: Guards::default(),
        }
    }
}
impl<E: Copy> From<super::Error<E>> for Error<E> {
    fn from(e: super::Error<E>) -> Self {
        let cause = match e.cause {
            super::Cause::Callback(e) => Cause::Callback(e),
            super::Cause::Phase(p) => Cause::Models(super::Cause::Phase(p)),
            super::Cause::Kind(k) => Cause::Models(super::Cause::Kind(k)),
            super::Cause::Collector(c) => Cause::Models(super::Cause::Collector(c)),
        };
        Self {
            cause,
            guards: Guards {
                models: e.guards,
                maps: None,
            },
        }
    }
}
fn cast<E: Copy>(e: Error) -> Error<E> {
    Error {
        cause: match e.cause {
            Cause::Models(c) => Cause::Models(c),
            Cause::Kind(k) => Cause::Kind(k),
            Cause::Callback(()) => Cause::Kind(Kind::Callback),
        },
        guards: e.guards,
    }
}
impl<E: Copy> std::fmt::Display for Error<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}
impl<E: Copy + std::fmt::Debug> std::error::Error for Error<E> {}
fn add(a: usize, b: usize) -> Result<usize, Kind> {
    a.checked_add(b).ok_or(Kind::Limit)
}
fn mul(a: usize, b: usize) -> Result<usize, Kind> {
    a.checked_mul(b).ok_or(Kind::Limit)
}
fn bump(n: &mut usize, amount: usize, cap: usize) -> Result<(), Kind> {
    *n = add(*n, amount)?;
    if *n > cap { Err(Kind::Limit) } else { Ok(()) }
}
fn bump64(n: &mut u64, amount: u64, cap: u64) -> Result<(), Kind> {
    *n = n.checked_add(amount).ok_or(Kind::Limit)?;
    if *n > cap { Err(Kind::Limit) } else { Ok(()) }
}
impl Limits {
    fn validate_resources(self, profile: crate::collision::resources::Profile) -> Result<(), Kind> {
        let cap = profile.package_limits();
        if !profile.supported()
            || self.max_models > cap.max_models
            || self.max_output_bytes > cap.max_output_bytes
            || self.max_vertices > cap.max_vertices
            || self.max_triangles > cap.max_triangles
            || self.max_raw_visit_bytes > cap.max_raw_visit_bytes
            || self.max_bih_work > cap.max_bih_work
            || self.max_validation_bytes > cap.max_validation_bytes
            || self.max_files > cap.max_files
            || self.max_records > cap.max_records
            || self.max_associations > cap.max_associations
            || self.max_ancestor_steps > cap.max_ancestor_steps
        {
            return Err(Kind::InvalidLimits);
        }
        let d = Self::default();
        let mut bounded = self;
        bounded.max_models = bounded.max_models.min(d.max_models);
        bounded.max_output_bytes = bounded.max_output_bytes.min(d.max_output_bytes);
        bounded.max_vertices = bounded.max_vertices.min(d.max_vertices);
        bounded.max_triangles = bounded.max_triangles.min(d.max_triangles);
        bounded.max_raw_visit_bytes = bounded.max_raw_visit_bytes.min(d.max_raw_visit_bytes);
        bounded.max_bih_work = bounded.max_bih_work.min(d.max_bih_work);
        bounded.max_validation_bytes = bounded.max_validation_bytes.min(d.max_validation_bytes);
        bounded.max_files = bounded.max_files.min(d.max_files);
        bounded.max_records = bounded.max_records.min(d.max_records);
        bounded.max_associations = bounded.max_associations.min(d.max_associations);
        bounded.max_ancestor_steps = bounded.max_ancestor_steps.min(d.max_ancestor_steps);
        bounded.validate()
    }
    fn validate(self) -> Result<(), Kind> {
        let d = Self::default();
        let c = self.assembly;
        let a = d.assembly;
        let b = c.bih;
        let db = a.bih;
        if self.max_memory_bytes > d.max_memory_bytes
            || self.max_maps > d.max_maps
            || self.max_files > d.max_files
            || self.max_models > d.max_models
            || self.max_records > d.max_records
            || self.max_output_bytes > d.max_output_bytes
            || self.max_vertices > d.max_vertices
            || self.max_triangles > d.max_triangles
            || self.max_associations > d.max_associations
            || self.max_raw_visit_bytes > d.max_raw_visit_bytes
            || self.max_bih_work > d.max_bih_work
            || self.max_ancestor_steps > d.max_ancestor_steps
            || self.max_validation_bytes > d.max_validation_bytes
            || c.max_input_bytes > a.max_input_bytes
            || c.max_tile_bytes > a.max_tile_bytes
            || c.max_spawns > a.max_spawns
            || c.max_associations > a.max_associations
            || c.max_tiles > a.max_tiles
            || c.max_output_bytes > a.max_output_bytes
            || c.max_memory_bytes > a.max_memory_bytes
            || c.max_name_bytes > a.max_name_bytes
            || b.max_primitives > db.max_primitives
            || b.max_tree_words > db.max_tree_words
            || b.max_bytes > db.max_bytes
            || b.max_memory_bytes > db.max_memory_bytes
            || b.max_work > db.max_work
            || b.max_depth > db.max_depth
        {
            return Err(Kind::InvalidLimits);
        }
        Ok(())
    }
}
#[derive(Clone, Copy)]
struct Span {
    start: usize,
    len: usize,
}
impl Span {
    fn get(self, bytes: &[u8]) -> Result<&[u8], Kind> {
        bytes
            .get(self.start..add(self.start, self.len)?)
            .ok_or(Kind::Binding)
    }
}
#[derive(Clone, Copy)]
struct Tile {
    x: u32,
    y: u32,
    payload: Span,
    index: Span,
}
impl Tile {
    fn header(self) -> [u8; TILE_HEADER] {
        let mut b = [0; TILE_HEADER];
        b[..4].copy_from_slice(&self.x.to_le_bytes());
        b[4..8].copy_from_slice(&self.y.to_le_bytes());
        for (at, value) in [
            (8, self.payload.start),
            (16, self.payload.len),
            (24, self.index.start),
            (32, self.index.len),
        ] {
            b[at..at + 8].copy_from_slice(&(value as u64).to_le_bytes());
        }
        b
    }
    fn scratch(self) -> Result<usize, Kind> {
        let len = self.index.len.checked_sub(12).ok_or(Kind::Binding)?;
        if len == 0 || len % 4 != 0 {
            return Err(Kind::Binding);
        }
        Ok(len)
    }
}
#[derive(Clone, Copy)]
struct Bound {
    id: u32,
    parent: Option<u32>,
    extent: usize,
    proof: Span,
    tree: Span,
    spawns: usize,
}
struct Map {
    bound: Bound,
    tiles: Vec<Tile>,
    record: spool::Record,
}
fn header(b: Bound, tiles: usize) -> [u8; HEADER] {
    let mut h = [0; HEADER];
    h[..8].copy_from_slice(MAGIC);
    h[8..12].copy_from_slice(&b.id.to_le_bytes());
    if let Some(id) = b.parent {
        h[12] = 1;
        h[16..20].copy_from_slice(&id.to_le_bytes());
    }
    for (at, n) in [
        (24, b.extent),
        (32, b.proof.len),
        (40, b.tree.len),
        (48, tiles),
        (56, b.spawns),
    ] {
        h[at..at + 8].copy_from_slice(&(n as u64).to_le_bytes());
    }
    h
}
impl Map {
    fn scratch(&self) -> Result<usize, Kind> {
        self.tiles
            .iter()
            .try_fold(0, |n, t| Ok(n.max(t.scratch()?)))
    }
    fn validation_bytes(&self) -> Result<usize, Kind> {
        self.tiles.iter().try_fold(self.bound.extent, |n, t| {
            let pair = if t.payload.len != 0 {
                add(add(self.bound.proof.len, t.payload.len)?, t.index.len)?
            } else {
                t.index.len
            };
            add(n, pair)
        })
    }
    fn tile(&self, x: u32, y: u32) -> Option<Tile> {
        self.tiles
            .binary_search_by_key(&(x, y), |t| (t.x, t.y))
            .ok()
            .map(|at| self.tiles[at])
    }
}
/// Borrowed complete checked map bundle. The ordered proof remains private;
/// callers receive only the exact native tree/payload/index artifact inventory.
pub struct MapView<'a> {
    map: &'a Map,
    bytes: &'a [u8],
}
impl<'a> MapView<'a> {
    fn check(map: &'a Map, bytes: &'a [u8], limits: assembly::Limits) -> Result<Self, Kind> {
        let b = map.bound;
        if bytes.len() != b.extent
            || bytes.get(..HEADER) != Some(header(b, map.tiles.len()).as_slice())
        {
            return Err(Kind::Binding);
        }
        let mut at = HEADER;
        for t in &map.tiles {
            if bytes.get(at..add(at, TILE_HEADER)?) != Some(t.header().as_slice()) {
                return Err(Kind::Binding);
            }
            at = add(at, TILE_HEADER)?;
        }
        if at != b.proof.start || add(b.proof.start, b.proof.len)? != b.tree.start {
            return Err(Kind::Binding);
        }
        let proof = placement::DirBin::parse(
            b.proof.get(bytes)?,
            placement::Limits {
                max_bytes: limits.max_input_bytes,
                max_records: limits.max_spawns,
                max_name_bytes: limits.max_name_bytes,
            },
        )
        .map_err(Kind::Placement)?;
        if proof.record_count() != b.spawns {
            return Err(Kind::Binding);
        }
        let mut previous = None;
        for p in proof.records() {
            if p.bounds_bits().is_none() || previous.is_some_and(|old| old >= p.id()) {
                return Err(Kind::Binding);
            }
            previous = Some(p.id());
        }
        assembly::Tree::parse(b.tree.get(bytes)?, b.spawns, limits).map_err(Kind::Assembly)?;
        at = add(b.tree.start, b.tree.len)?;
        let mut key = None;
        for t in &map.tiles {
            if key.is_some_and(|k| k >= (t.x, t.y))
                || t.x > 32767
                || t.y > 255
                || t.payload.start != at
                || add(t.payload.start, t.payload.len)? != t.index.start
            {
                return Err(Kind::Binding);
            }
            key = Some((t.x, t.y));
            let index = t.index.get(bytes)?;
            if t.payload.len != 0 {
                assembly::verify_associations(
                    t.payload.get(bytes)?,
                    index,
                    b.proof.get(bytes)?,
                    limits,
                )
                .map_err(Kind::Assembly)?;
            } else {
                assembly::IndexView::parse(index, b.spawns, limits).map_err(Kind::Assembly)?;
            }
            at = add(t.index.start, t.index.len)?;
        }
        if at != b.extent {
            return Err(Kind::Binding);
        }
        Ok(Self { map, bytes })
    }
    pub const fn map_id(&self) -> u32 {
        self.map.bound.id
    }
    pub fn artifacts(&self) -> impl Iterator<Item = resident::Artifact<'a>> + '_ {
        let map = self.map;
        let bytes = self.bytes;
        std::iter::once(resident::Artifact {
            name: resident::ArtifactName::Tree(map.bound.id),
            bytes: map.bound.tree.get(bytes).expect("checked tree"),
        })
        .chain(map.tiles.iter().flat_map(move |t| {
            (t.payload.len != 0)
                .then(|| resident::Artifact {
                    name: resident::ArtifactName::Tile {
                        map: map.bound.id,
                        x: t.x,
                        y: t.y,
                        index: false,
                    },
                    bytes: t.payload.get(bytes).expect("checked payload"),
                })
                .into_iter()
                .chain(std::iter::once(resident::Artifact {
                    name: resident::ArtifactName::Tile {
                        map: map.bound.id,
                        x: t.x,
                        y: t.y,
                        index: true,
                    },
                    bytes: t.index.get(bytes).expect("checked index"),
                }))
        }))
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    pub maps: usize,
    pub selected_models: usize,
    pub input_records: usize,
    pub unique_spawns: usize,
    pub vertices: usize,
    pub triangles: usize,
    pub associations: usize,
    pub files: usize,
    pub output_bytes: usize,
    pub proof_bytes: usize,
    pub parent_only_tiles: usize,
    pub payload_tiles: usize,
    pub raw_visit_bytes: u64,
    pub bih_work: u64,
    pub ancestor_steps: usize,
    /// All explicit complete-bundle and inherited-association validation passes.
    pub validation_bytes: u64,
    /// Comparable to the resident assembler's inherited-pair byte counter.
    pub association_validation_bytes: u64,
    pub bundle_validation_bytes: u64,
    pub model_validation_bytes: u64,
    pub reserved_bytes: usize,
    pub peak_admitted_bytes: usize,
}
/// Complete closed artifact inventory. Bodies remain in retained verified VMO
/// and map/proof repositories. Nothing is published by construction or Drop.
pub struct Completed {
    models: Prepared,
    repository: spool::Spool,
    maps: Vec<Map>,
    limits: Limits,
    base: usize,
    stats: Stats,
    failed: Option<Kind>,
}
impl Completed {
    fn live(&self) -> Result<usize, Kind> {
        self.maps.iter().try_fold(
            add(self.base, mul(self.maps.capacity(), size_of::<Map>())?)?,
            |n, m| add(n, mul(m.tiles.capacity(), size_of::<Tile>())?),
        )
    }
    fn admit(&mut self, extra: usize) -> Result<(), Kind> {
        let n = add(self.live()?, extra)?;
        if n > self.limits.max_memory_bytes {
            return Err(Kind::Limit);
        }
        self.stats.peak_admitted_bytes = self.stats.peak_admitted_bytes.max(n);
        Ok(())
    }
    fn guards(&mut self) -> Guards {
        Guards {
            models: self.models.guards(),
            maps: self.repository.verify_unchanged().err(),
        }
    }
    fn guarded<T, E: Copy>(&mut self, result: Result<T, Error<E>>) -> Result<T, Error<E>> {
        let mut guards = self.guards();
        let mut error = match result {
            Err(e) => e,
            Ok(value) => {
                if guards == Guards::default() && self.failed.is_none() {
                    return Ok(value);
                }
                drop(value);
                guards = self.guards();
                Error::from(self.failed.unwrap_or(Kind::State))
            }
        };
        if guards.models.collector.is_some() {
            error.guards.models.collector = guards.models.collector;
        }
        if guards.models.placements.is_some() {
            error.guards.models.placements = guards.models.placements;
        }
        if guards.models.archive.is_some() {
            error.guards.models.archive = guards.models.archive;
        }
        if guards.maps.is_some() {
            error.guards.maps = guards.maps;
        }
        self.failed.get_or_insert(Kind::State);
        Err(error)
    }
    pub fn verify_unchanged(&mut self) -> Result<(), Error> {
        let r = self.failed.map_or(Ok(()), |e| Err(e.into()));
        self.guarded(r)
    }
    pub const fn prepared(&self) -> &Prepared {
        &self.models
    }
    pub fn stats(&self) -> Result<Stats, Error> {
        self.failed.map_or(Ok(()), |e| Err(Error::from(e)))?;
        self.models.stats()?;
        Ok(self.stats)
    }
    fn map(&self, id: u32) -> Result<usize, Kind> {
        self.maps
            .binary_search_by_key(&id, |m| m.bound.id)
            .map_err(|_| Kind::Missing)
    }
    fn validation(&mut self, bytes: usize, bundle: bool) -> Result<(), Kind> {
        bump64(
            &mut self.stats.validation_bytes,
            bytes as u64,
            self.limits.max_validation_bytes,
        )?;
        let n = if bundle {
            &mut self.stats.bundle_validation_bytes
        } else {
            &mut self.stats.association_validation_bytes
        };
        *n = n.checked_add(bytes as u64).ok_or(Kind::Limit)?;
        Ok(())
    }
    /// Stable complete inventory; no file reads or public private-file ordinal.
    pub fn artifacts(
        &self,
    ) -> Result<impl Iterator<Item = (resident::ArtifactName<'_>, usize)> + '_, Error> {
        self.stats()?;
        let compact = self
            .models
            .stage
            .world
            .startup()
            .extraction()
            .phase_compact()
            .map_err(super::Error::from)?;
        let data = self
            .models
            .stage
            .world
            .startup()
            .data()
            .map_err(|_| Kind::State)?;
        let dtree = std::iter::once((
            resident::ArtifactName::Dtree,
            data.lists().dtree_bytes().len(),
        ));
        let models = self
            .models
            .index
            .iter()
            .filter(|e| e.selected)
            .map(move |e| {
                let d = compact
                    .descriptor(e.handle)
                    .expect("closed selected identity");
                (resident::ArtifactName::Model(d.output_basename()), e.bytes)
            });
        let maps = self.maps.iter().flat_map(|m| {
            std::iter::once((resident::ArtifactName::Tree(m.bound.id), m.bound.tree.len)).chain(
                m.tiles.iter().flat_map(move |t| {
                    (t.payload.len != 0)
                        .then_some((
                            resident::ArtifactName::Tile {
                                map: m.bound.id,
                                x: t.x,
                                y: t.y,
                                index: false,
                            },
                            t.payload.len,
                        ))
                        .into_iter()
                        .chain(std::iter::once((
                            resident::ArtifactName::Tile {
                                map: m.bound.id,
                                x: t.x,
                                y: t.y,
                                index: true,
                            },
                            t.index.len,
                        )))
                }),
            )
        });
        Ok(dtree.chain(models).chain(maps))
    }
    fn build_maps(&mut self) -> Result<(), Error> {
        let count = self.models.stage.stats().map_err(super::Error::from)?.maps;
        if count > self.limits.max_maps {
            return Err(Kind::Limit.into());
        }
        self.admit(mul(count, size_of::<Map>())?)?;
        self.maps
            .try_reserve_exact(count)
            .map_err(|_| Kind::Allocation)?;
        self.admit(0)?;
        for at in 0..self.models.stage.stage.entries.len() {
            let e = self.models.stage.stage.entries[at];
            if !matches!(e.context.kind, run::Kind::Global) {
                continue;
            }
            self.verify_unchanged()?;
            let id = e.context.map_id;
            let raw = self
                .models
                .stage
                .stage
                .read_map(id, 0)
                .map_err(|e| Error::from(super::Error::from(super::super::Error::from(e))))?;
            let view = placement::DirBin::parse(&raw, self.models.stage.stage.limits.records)
                .map_err(Kind::Placement)?;
            let mut placements = Vec::new();
            self.admit(mul(
                view.record_count(),
                size_of::<placement::Placement<'_>>(),
            )?)?;
            placements
                .try_reserve_exact(view.record_count())
                .map_err(|_| Kind::Allocation)?;
            let pbytes = mul(placements.capacity(), size_of::<placement::Placement<'_>>())?;
            self.admit(pbytes)?;
            let compact = self
                .models
                .stage
                .world
                .startup()
                .extraction()
                .phase_compact()
                .map_err(super::Error::from)?;
            for row in view.records() {
                let at = self
                    .models
                    .binding(row.name())
                    .map_err(|e| Error::from(super::Error::from(e)))?;
                let item = self.models.index[at];
                if !item.selected {
                    return Err(Kind::Binding.into());
                }
                if compact
                    .descriptor(item.handle)
                    .map_err(|e| super::Error::from(payload::CollectorError::Collision(e)))?
                    .class()
                    == Class::Wmo
                    && row.bounds_bits().is_none()
                {
                    return Err(Kind::Class.into());
                }
                placements.push(
                    placement::bounds::complete(
                        row,
                        item.bounds,
                        self.models.stage.stage.limits.records,
                    )
                    .map_err(Kind::Bounds)?,
                );
            }
            let mut component = self.limits.assembly;
            component.max_memory_bytes = component.max_memory_bytes.min(
                self.limits
                    .max_memory_bytes
                    .checked_sub(self.live()?)
                    .and_then(|n| n.checked_sub(pbytes))
                    .ok_or(Kind::Limit)?,
            );
            component.max_output_bytes = component
                .max_output_bytes
                .min(self.limits.max_output_bytes - self.stats.output_bytes);
            component.max_spawns = component.max_spawns.min(self.limits.max_records);
            component.max_tiles = component.max_tiles.min(
                self.limits
                    .max_files
                    .checked_sub(self.stats.files)
                    .and_then(|n| n.checked_sub(1))
                    .ok_or(Kind::Limit)?,
            );
            component.max_associations = component
                .max_associations
                .min(self.limits.max_associations - self.stats.associations);
            component.bih.max_work = component
                .bih
                .max_work
                .min(self.limits.max_bih_work - self.stats.bih_work);
            self.admit(add(pbytes, component.max_memory_bytes)?)?;
            let built = assembly::build(id, &placements, component).map_err(Kind::Assembly)?;
            let s = built.stats();
            self.admit(add(pbytes, s.peak_bytes)?)?;
            self.write_map(&built, pbytes)?;
            drop(built);
            drop(placements);
            drop(raw);
            self.verify_unchanged()?;
        }
        if self.maps.len() != count || self.maps.windows(2).any(|m| m[0].bound.id >= m[1].bound.id)
        {
            return Err(Kind::Binding.into());
        }
        Ok(())
    }
    fn write_map(&mut self, built: &assembly::AssembledMap, held: usize) -> Result<(), Error> {
        let id = built.map_id();
        let s = built.stats();
        if self.maps.len() >= self.maps.capacity() || self.maps.len() >= self.limits.max_maps {
            return Err(Kind::Limit.into());
        }
        if !self
            .models
            .stage
            .world
            .topology()
            .get(id)
            .is_some_and(|m| m.active())
        {
            return Err(Kind::Binding.into());
        }
        let mut tiles = Vec::new();
        self.admit(add(
            add(held, s.resident_bytes)?,
            mul(s.tiles, size_of::<Tile>())?,
        )?)?;
        tiles
            .try_reserve_exact(s.tiles)
            .map_err(|_| Kind::Allocation)?;
        self.admit(add(
            add(held, s.resident_bytes)?,
            mul(tiles.capacity(), size_of::<Tile>())?,
        )?)?;
        let proof = Span {
            start: add(HEADER, mul(s.tiles, TILE_HEADER)?)?,
            len: built.ordered_dir_bin().len(),
        };
        let tree = Span {
            start: add(proof.start, proof.len)?,
            len: built.tree_bytes().len(),
        };
        let mut extent = add(tree.start, tree.len)?;
        for t in built.tiles() {
            let payload = Span {
                start: extent,
                len: t.payload().map_or(0, <[u8]>::len),
            };
            extent = add(extent, payload.len)?;
            let index = Span {
                start: extent,
                len: t.index_bytes().len(),
            };
            extent = add(extent, index.len)?;
            tiles.push(Tile {
                x: t.x(),
                y: t.y(),
                payload,
                index,
            });
        }
        let parent = self
            .models
            .stage
            .world
            .topology()
            .get(id)
            .ok_or(Kind::Binding)?
            .parent_map_id();
        let bound = Bound {
            id,
            parent,
            extent,
            proof,
            tree,
            spawns: s.unique_spawns,
        };
        bump(
            &mut self.stats.files,
            add(1, add(s.tiles, s.payload_tiles)?)?,
            self.limits.max_files,
        )?;
        bump(
            &mut self.stats.output_bytes,
            s.output_bytes,
            self.limits.max_output_bytes,
        )?;
        bump(
            &mut self.stats.associations,
            s.associations,
            self.limits.max_associations,
        )?;
        bump64(
            &mut self.stats.bih_work,
            s.bih_work,
            self.limits.max_bih_work,
        )?;
        self.stats.unique_spawns = add(self.stats.unique_spawns, s.unique_spawns)?;
        self.stats.proof_bytes = add(self.stats.proof_bytes, s.proof_bytes)?;
        self.stats.payload_tiles = add(self.stats.payload_tiles, s.payload_tiles)?;
        self.stats.parent_only_tiles = add(self.stats.parent_only_tiles, s.parent_only_tiles)?;
        let h = header(bound, tiles.len());
        let record = self
            .repository
            .write(extent, |out| {
                out.bytes(&h)?;
                for t in &tiles {
                    out.bytes(&t.header())?;
                }
                out.bytes(built.ordered_dir_bin())?;
                out.bytes(built.tree_bytes())?;
                for t in built.tiles() {
                    if let Some(payload) = t.payload() {
                        out.bytes(payload)?;
                    }
                    out.bytes(t.index_bytes())?;
                }
                Ok(())
            })
            .map_err(Kind::Spool)?;
        self.maps.push(Map {
            bound,
            tiles,
            record,
        });
        self.stats.maps += 1;
        Ok(())
    }
    fn load<T, E: Copy>(
        &mut self,
        at: usize,
        retained: usize,
        callback_heap: usize,
        call: impl FnOnce(&MapView<'_>) -> Result<T, E>,
    ) -> Result<T, Error<E>> {
        let setup: Result<assembly::Limits, Error<E>> = (|| {
            self.verify_unchanged().map_err(cast)?;
            let m = self.maps.get(at).ok_or(Kind::Missing)?;
            let extent = m.bound.extent;
            let scratch = m.scratch()?;
            let validation = m.validation_bytes()?;
            self.admit(add(add(extent, scratch)?, add(retained, callback_heap)?)?)?;
            self.validation(validation, true)?;
            let mut limits = self.limits.assembly;
            limits.max_memory_bytes = limits.max_memory_bytes.min(scratch);
            Ok(limits)
        })();
        let limits = match setup {
            Ok(v) => v,
            Err(e) => {
                drop(call);
                return self.guarded(Err(e));
            }
        };
        let m = &self.maps[at];
        let mut outcome = None;
        let io = self
            .repository
            .with_record(m.record, m.bound.extent, |b| {
                outcome = Some(match MapView::check(m, b, limits) {
                    Ok(view) => call(&view).map_err(|e| Error {
                        cause: Cause::Callback(e),
                        guards: Guards::default(),
                    }),
                    Err(e) => Err(e.into()),
                });
                Ok(())
            })
            .map_err(Kind::Spool);
        let result = match (outcome, io) {
            (Some(Err(e)), _) => Err(e),
            (Some(Ok(value)), Ok(())) => Ok(value),
            (Some(Ok(value)), Err(e)) => {
                drop(value);
                Err(e.into())
            }
            (None, Err(e)) => Err(e.into()),
            (None, Ok(())) => Err(Kind::State.into()),
        };
        self.guarded(result)
    }
    /// One complete map read for all of its tree/tile/index artifacts. Caller
    /// allocations and retained callback state must fit the declared allowance.
    pub fn with_map<T, E: Copy>(
        &mut self,
        id: u32,
        callback_heap_allowance: usize,
        call: impl FnOnce(&MapView<'_>) -> Result<T, E>,
    ) -> Result<T, Error<E>> {
        let at = match self.map(id) {
            Ok(at) => at,
            Err(e) => {
                drop(call);
                return self.guarded(Err(e.into()));
            }
        };
        self.load(at, 0, callback_heap_allowance, call)
    }
    fn copy_map(&mut self, at: usize) -> Result<Vec<u8>, Error> {
        let extent = self.maps[at].bound.extent;
        self.load(at, 0, extent, |v| {
            let mut bytes = Vec::new();
            bytes
                .try_reserve_exact(extent)
                .map_err(|_| Kind::Allocation)?;
            if bytes.capacity() > extent {
                return Err(Kind::Limit);
            }
            bytes.extend_from_slice(v.bytes);
            Ok::<_, Kind>(bytes)
        })
        .map_err(|e| Error {
            cause: match e.cause {
                Cause::Callback(k) => Cause::Kind(k),
                Cause::Models(c) => Cause::Models(c),
                Cause::Kind(k) => Cause::Kind(k),
            },
            guards: e.guards,
        })
    }
    fn verify_parents(&mut self) -> Result<(), Error> {
        for at in 0..self.maps.len() {
            if self.maps[at].tiles.iter().all(|t| t.payload.len != 0) {
                continue;
            }
            let child = self.copy_map(at)?;
            for ti in 0..self.maps[at].tiles.len() {
                let t = self.maps[at].tiles[ti];
                if t.payload.len != 0 {
                    continue;
                }
                let mut parent = self.maps[at].bound.parent;
                let (ancestor, payload) = loop {
                    let id = parent.ok_or(Kind::MissingAncestor)?;
                    bump(
                        &mut self.stats.ancestor_steps,
                        1,
                        self.limits.max_ancestor_steps,
                    )?;
                    let entry = self
                        .models
                        .stage
                        .world
                        .topology()
                        .get(id)
                        .ok_or(Kind::Binding)?;
                    if let Ok(p) = self.map(id)
                        && let Some(tile) = self.maps[p].tile(t.x, t.y)
                        && tile.payload.len != 0
                    {
                        break (p, tile.payload);
                    }
                    parent = entry.parent_map_id();
                };
                let proof = self.maps[at].bound.proof.get(&child)?;
                let index = t.index.get(&child)?;
                let bytes = add(add(proof.len(), index.len())?, payload.len)?;
                self.validation(bytes, false)?;
                let mut limits = self.limits.assembly;
                limits.max_memory_bytes = limits.max_memory_bytes.min(t.scratch()?);
                let result = self.load(ancestor, child.capacity(), t.scratch()?, |v| {
                    assembly::verify_inherited_associations(
                        payload.get(v.bytes)?,
                        index,
                        proof,
                        limits,
                    )
                    .map_err(Kind::Assembly)
                });
                result.map_err(|e| Error {
                    cause: match e.cause {
                        Cause::Callback(k) => Cause::Kind(k),
                        Cause::Models(c) => Cause::Models(c),
                        Cause::Kind(k) => Cause::Kind(k),
                    },
                    guards: e.guards,
                })?;
            }
            drop(child);
            self.verify_unchanged()?;
        }
        Ok(())
    }
    pub fn with_model<T, E: Copy>(
        &mut self,
        name: &[u8],
        callback_heap_allowance: usize,
        call: impl FnOnce(&[u8]) -> Result<T, E>,
    ) -> Result<T, Error<E>> {
        let setup: Result<usize, Error<E>> = (|| {
            self.verify_unchanged().map_err(cast)?;
            let at = self
                .models
                .binding(name)
                .map_err(|e| Error::from(super::Error::from(e)).into_generic())?;
            let e = self.models.index[at];
            if !e.selected {
                return Err(Kind::Missing.into());
            }
            let extra = self
                .live()?
                .checked_sub(self.models.live().map_err(|_| Kind::Limit)?)
                .ok_or(Kind::State)?;
            // Reserve any gap between the older Prepared cap and a smaller
            // package cap, in addition to actual newly retained package memory.
            // This forces its reader to obey BOTH ledgers before any IO.
            let gap = self
                .models
                .limits
                .max_memory_bytes
                .saturating_sub(self.limits.max_memory_bytes);
            let callback = add(add(extra, callback_heap_allowance)?, gap)?;
            let extent = add(add(super::HEADER, name.len())?, e.bytes)?;
            let free = self
                .models
                .limits
                .max_memory_bytes
                .checked_sub(self.models.live().map_err(|_| Kind::Limit)?)
                .and_then(|n| n.checked_sub(extent))
                .and_then(|n| n.checked_sub(callback))
                .ok_or(Kind::Limit)?;
            let reader = self.models.limits.conversion.max_memory_bytes.min(free);
            self.admit(add(add(extent, reader)?, callback_heap_allowance)?)?;
            bump64(
                &mut self.stats.validation_bytes,
                e.bytes as u64,
                self.limits.max_validation_bytes,
            )?;
            self.stats.model_validation_bytes = self
                .stats
                .model_validation_bytes
                .checked_add(e.bytes as u64)
                .ok_or(Kind::Limit)?;
            Ok(callback)
        })();
        let callback = match setup {
            Ok(n) => n,
            Err(e) => {
                drop(call);
                return self.guarded(Err(e));
            }
        };
        let result = self
            .models
            .with_model(name, callback, |v| call(v.bytes()))
            .map_err(Error::from);
        self.guarded(result)
    }
    /// Visit the complete native inventory in stable order. Each selected VMO
    /// and each complete map bundle is reloaded once; all map artifacts borrow
    /// that single read. On failure the owned callback drops before final guards.
    pub fn visit<E: Copy>(
        &mut self,
        callback_heap_allowance: usize,
        mut call: impl FnMut(resident::Artifact<'_>) -> Result<(), E>,
    ) -> Result<(), Error<E>> {
        let result = (|| {
            self.verify_unchanged().map_err(cast)?;
            self.admit(callback_heap_allowance)?;
            let bytes = self
                .models
                .stage
                .world
                .startup()
                .data()
                .map_err(|_| Kind::State)?
                .lists()
                .dtree_bytes();
            call(resident::Artifact {
                name: resident::ArtifactName::Dtree,
                bytes,
            })
            .map_err(|e| Error {
                cause: Cause::Callback(e),
                guards: Guards::default(),
            })?;
            self.verify_unchanged().map_err(cast)?;
            for at in 0..self.models.index.len() {
                let entry = self.models.index[at];
                if !entry.selected {
                    continue;
                }
                // Copy the bounded source name to stack, avoiding a mutable
                // owner borrow while descriptor metadata is borrowed.
                let mut name = [0u8; super::NAME_BYTES];
                let compact = self
                    .models
                    .stage
                    .world
                    .startup()
                    .extraction()
                    .phase_compact()
                    .map_err(|e| Error::from(super::Error::from(e)).into_generic())?;
                let d = compact.descriptor(entry.handle).map_err(|e| {
                    Error::from(super::Error::from(payload::CollectorError::Collision(e)))
                        .into_generic()
                })?;
                let len = d.output_basename().len();
                if len > name.len() {
                    return Err(Kind::Binding.into());
                }
                name[..len].copy_from_slice(d.output_basename());
                self.with_model(&name[..len], callback_heap_allowance, |bytes| {
                    call(resident::Artifact {
                        name: resident::ArtifactName::Model(&name[..len]),
                        bytes,
                    })
                })?;
            }
            for at in 0..self.maps.len() {
                let id = self.maps[at].bound.id;
                self.with_map(id, callback_heap_allowance, |view| {
                    for artifact in view.artifacts() {
                        call(artifact)?;
                    }
                    Ok(())
                })?;
            }
            Ok(())
        })();
        drop(call);
        self.guarded(result)
    }
}
impl Error {
    fn into_generic<E: Copy>(self) -> Error<E> {
        cast(self)
    }
}
/// Sole constructor of a complete private artifact inventory. The supplied map
/// repository must be fresh. Failed assembly consumes all owners, never exposes
/// a usable prefix and may leave private files for caller-controlled retention.
pub fn assemble(
    mut models: Prepared,
    mut repository: spool::Spool,
    limits: Limits,
) -> Result<Completed, Error> {
    let admission: Result<(usize, Stats), Error> = (|| {
        if repository.resources() != models.repository.resources()
            || repository.backend() != models.repository.backend()
        {
            return Err(Kind::InvalidLimits.into());
        }
        limits.validate_resources(repository.resources())?;
        models.verify_unchanged()?;
        repository.verify_unchanged().map_err(Kind::Spool)?;
        if repository.stats().map_err(Kind::Spool)?.records != 0 {
            return Err(Kind::State.into());
        }
        let m = models.stats()?;
        let p = models.stage.stats().map_err(super::Error::from)?;
        if m.selected_models > limits.max_models
            || m.vertices > limits.max_vertices
            || m.triangles > limits.max_triangles
            || m.raw_visit_bytes > limits.max_raw_visit_bytes
            || m.bih_work > limits.max_bih_work
            || m.validation_bytes > limits.max_validation_bytes
            || p.records > limits.max_records
            || p.maps > limits.max_maps
        {
            return Err(Kind::Limit.into());
        }
        let base = add(
            models.live().map_err(|_| Kind::Limit)?,
            add(
                mul(repository.limits().max_descriptor_bytes, 2)?,
                GUARD_BYTES,
            )?,
        )?;
        if base > limits.max_memory_bytes {
            return Err(Kind::Limit.into());
        }
        let dtree = models
            .stage
            .world
            .startup()
            .data()
            .map_err(|_| Kind::State)?
            .lists()
            .dtree_bytes()
            .len();
        let files = add(1, m.selected_models)?;
        let bytes = add(dtree, m.output_bytes)?;
        if files > limits.max_files || bytes > limits.max_output_bytes {
            return Err(Kind::Limit.into());
        }
        Ok((
            base,
            Stats {
                selected_models: m.selected_models,
                input_records: p.records,
                vertices: m.vertices,
                triangles: m.triangles,
                files,
                output_bytes: bytes,
                raw_visit_bytes: m.raw_visit_bytes,
                bih_work: m.bih_work,
                validation_bytes: m.validation_bytes,
                model_validation_bytes: m.validation_bytes,
                reserved_bytes: base,
                peak_admitted_bytes: base,
                ..Default::default()
            },
        ))
    })();
    let (base, stats) = match admission {
        Ok(v) => v,
        Err(mut e) => {
            e.guards = Guards {
                models: models.guards(),
                maps: repository.verify_unchanged().err(),
            };
            return Err(e);
        }
    };
    let mut out = Completed {
        models,
        repository,
        maps: Vec::new(),
        limits,
        base,
        stats,
        failed: None,
    };
    let result = out.build_maps().and_then(|()| out.verify_parents());
    out.guarded(result)?;
    out.stats.reserved_bytes = out.live()?;
    let mut files = 0;
    let mut bytes = 0;
    for (_, len) in out.artifacts()? {
        files = add(files, 1)?;
        bytes = add(bytes, len)?;
    }
    if files != out.stats.files || bytes != out.stats.output_bytes {
        return out.guarded(Err(Kind::Binding.into()));
    }
    out.verify_unchanged()?;
    Ok(out)
}
