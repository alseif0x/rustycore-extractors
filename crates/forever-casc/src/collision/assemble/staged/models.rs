// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-12: original closed selected-model VMO disk preparation.
//! Consumes the actual authenticated placement stage. Selection is every name
//! BEFORE placement ID deduplication plus every successful startup TempList row.
//! Unused acquired dependencies are excluded. One complete authenticated RAW is
//! parsed/converted at a time; validated VMO bytes live in a fresh private spool.
//! No caller placements, model bytes, proof, publication or final map assembly.
//!
//! Full placement-stage reservation, recorded store/compact/Data/topology,
//! immutable archive descriptors and catalog capacities stay disjoint. RAW,
//! conversion, reader and callback allowances are admitted before IO. Completed
//! RAW access separately reduces the unchanged collector budget by conversion
//! allowance. These are conservative logical budgets, not allocator/RSS or all
//! CPU counters. Existing v1 source/read/spool ceilings remain unchanged.
use super::{StageError, Staged};
use crate::collision::{
    Class, ModelHandle, Status,
    payload::{self, CallbackFailure, CollectorError},
    spool,
};
use forever_vmap::{RawM2, gameobject_models, vmo, wmo::raw};
use std::mem::size_of;
pub mod package;
#[cfg(all(test, target_os = "linux"))]
mod tests;
#[cfg(target_os = "linux")]
pub const PROFILE: &str = "linux-private-selected-vmo-stage-v1";
#[cfg(windows)]
pub const PROFILE: &str = "windows-private-selected-vmo-stage-v1";
pub const MAX_MEMORY_BYTES: usize = crate::collision::run::inventory::MAX_COMBINED_MEMORY_BYTES;
const MAGIC: &[u8; 8] = b"RCEVMO1\0";
const HEADER: usize = 92;
const NAME_BYTES: usize = 1024;
const GUARD_BYTES: usize = 512 * 1024;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub max_memory_bytes: usize,
    pub max_models: usize,
    pub max_output_bytes: usize,
    pub max_vertices: usize,
    pub max_triangles: usize,
    pub max_raw_visit_bytes: u64,
    pub max_bih_work: u64,
    pub max_validation_bytes: u64,
    pub conversion: vmo::Limits,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_memory_bytes: MAX_MEMORY_BYTES,
            max_models: super::super::MAX_MODELS,
            max_output_bytes: super::super::MAX_OUTPUT_BYTES,
            max_vertices: super::super::MAX_GEOMETRY,
            max_triangles: super::super::MAX_GEOMETRY,
            max_raw_visit_bytes: super::super::MAX_RAW_VISIT_BYTES,
            max_bih_work: super::super::MAX_BIH_WORK,
            max_validation_bytes: super::super::MAX_VALIDATION_BYTES,
            conversion: vmo::Limits::default(),
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
    State,
    Missing,
    Callback,
    Spool(payload::Error),
    Vmo(vmo::Error),
    RawM2(forever_vmap::Error),
    RawWmo(forever_vmap::wmo::Error),
    Lists(gameobject_models::Error),
    Placement(forever_vmap::placement::Error),
}
impl Kind {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidLimits => "collision_vmo_stage_invalid_limits",
            Self::Limit => "collision_vmo_stage_limit",
            Self::Allocation => "collision_vmo_stage_allocation",
            Self::Binding => "collision_vmo_stage_binding",
            Self::Class => "collision_vmo_stage_class",
            Self::State => "collision_vmo_stage_state",
            Self::Missing => "collision_vmo_stage_missing",
            Self::Callback => "collision_vmo_stage_callback",
            Self::Spool(e) => e.code(),
            Self::Vmo(e) => e.code(),
            Self::RawM2(e) => e.code(),
            Self::RawWmo(e) => e.code(),
            Self::Lists(e) => e.code(),
            Self::Placement(e) => e.code(),
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
    pub collector: Option<CollectorError>,
    pub placements: Option<StageError>,
    pub archive: Option<payload::Error>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cause<E = ()> {
    Phase(super::Cause),
    Kind(Kind),
    Collector(CollectorError),
    Callback(E),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Error<E = ()> {
    pub cause: Cause<E>,
    pub guards: Guards,
}
impl<E: Copy> Error<E> {
    pub fn code(self) -> &'static str {
        if let Some(e) = self.guards.collector
            && super::collector_changed(e)
        {
            return e.code();
        }
        if self.guards.placements == Some(StageError::Spool(payload::Error::Changed))
            || self.guards.archive == Some(payload::Error::Changed)
        {
            return payload::Error::Changed.code();
        }
        match self.cause {
            Cause::Kind(e) => e.code(),
            Cause::Collector(e) => e.code(),
            Cause::Callback(_) => Kind::Callback.code(),
            Cause::Phase(cause) => super::Error {
                cause,
                guards: super::Guards {
                    collector: self.guards.collector,
                    stage: self.guards.placements,
                    ..Default::default()
                },
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
impl<E> From<CollectorError> for Error<E> {
    fn from(e: CollectorError) -> Self {
        Self {
            cause: Cause::Collector(e),
            guards: Guards::default(),
        }
    }
}
impl<E> From<super::Error> for Error<E> {
    fn from(e: super::Error) -> Self {
        // This phase takes an already completed Staged owner. WDT/cache owners
        // were checked/dropped by acquisition and never enter this phase.
        debug_assert!(e.guards.wdt.is_none() && e.guards.cache.is_none());
        Self {
            cause: Cause::Phase(e.cause),
            guards: Guards {
                collector: e.guards.collector,
                placements: e.guards.stage,
                archive: None,
            },
        }
    }
}
impl<E: Copy> std::fmt::Display for Error<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}
impl<E: Copy + std::fmt::Debug> std::error::Error for Error<E> {}
#[derive(Clone, Copy)]
struct Entry {
    handle: ModelHandle,
    selected: bool,
    bounds: Option<[u32; 6]>,
    record: Option<spool::Record>,
    bytes: usize,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    pub selected_models: usize,
    pub maps_scanned: usize,
    pub placement_records: usize,
    pub list_rows: usize,
    pub vertices: usize,
    pub triangles: usize,
    pub output_bytes: usize,
    pub raw_visit_bytes: u64,
    pub bih_work: u64,
    pub validation_bytes: u64,
    /// Includes the FULL placement stage reservation and twice the artifact
    /// descriptor ceiling, independently of their actual retained capacities.
    pub reserved_bytes: usize,
    pub peak_admitted_bytes: usize,
}
/// Closed selected VMO inventory plus the actual placement/world owners. This
/// remains an intermediate phase and cannot enumerate final map artifacts.
pub struct Prepared {
    stage: Staged,
    repository: spool::Spool,
    index: Vec<Entry>,
    limits: Limits,
    base: usize,
    stats: Stats,
    failed: Option<Kind>,
}
fn add(a: usize, b: usize) -> Result<usize, Kind> {
    a.checked_add(b).ok_or(Kind::Limit)
}
fn mul(a: usize, b: usize) -> Result<usize, Kind> {
    a.checked_mul(b).ok_or(Kind::Limit)
}
fn bump(n: &mut usize, a: usize, cap: usize) -> Result<(), Kind> {
    *n = add(*n, a)?;
    if *n > cap { Err(Kind::Limit) } else { Ok(()) }
}
fn bump64(n: &mut u64, a: u64, cap: u64) -> Result<(), Kind> {
    *n = n.checked_add(a).ok_or(Kind::Limit)?;
    if *n > cap { Err(Kind::Limit) } else { Ok(()) }
}
impl Limits {
    fn validate_resources(self, profile: crate::collision::resources::Profile) -> Result<(), Kind> {
        let cap = profile.model_limits();
        if !profile.supported()
            || self.max_models > cap.max_models
            || self.max_output_bytes > cap.max_output_bytes
            || self.max_vertices > cap.max_vertices
            || self.max_triangles > cap.max_triangles
            || self.max_raw_visit_bytes > cap.max_raw_visit_bytes
            || self.max_bih_work > cap.max_bih_work
            || self.max_validation_bytes > cap.max_validation_bytes
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
        bounded.validate()
    }
    fn validate(self) -> Result<(), Kind> {
        let d = Self::default();
        let c = self.conversion;
        let v = vmo::Limits::default();
        let b = c.bih;
        let db = v.bih;
        if self.max_memory_bytes > d.max_memory_bytes
            || self.max_models > d.max_models
            || self.max_output_bytes > d.max_output_bytes
            || self.max_vertices > d.max_vertices
            || self.max_triangles > d.max_triangles
            || self.max_raw_visit_bytes > d.max_raw_visit_bytes
            || self.max_bih_work > d.max_bih_work
            || self.max_validation_bytes > d.max_validation_bytes
            || c.max_raw_bytes > v.max_raw_bytes
            || c.max_bytes > v.max_bytes
            || c.max_memory_bytes > v.max_memory_bytes
            || c.max_groups > v.max_groups
            || c.max_vertices > v.max_vertices
            || c.max_triangles > v.max_triangles
            || c.max_liquid_heights > v.max_liquid_heights
            || c.max_liquid_tiles > v.max_liquid_tiles
            || c.max_work > v.max_work
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
struct Identity {
    header: [u8; HEADER],
    name: [u8; NAME_BYTES],
    len: usize,
}
fn identity(extraction: &payload::SpoolingExtraction, e: Entry) -> Result<Identity, Error> {
    let descriptor = extraction.descriptor(e.handle)?;
    if descriptor.status() != Status::Collision || !e.selected {
        return Err(Kind::Binding.into());
    }
    let name = descriptor.output_basename();
    if name.is_empty() || name.len() > NAME_BYTES {
        return Err(Kind::Binding.into());
    }
    let source = descriptor.source().ok_or(Kind::Binding)?;
    let mut h = [0; HEADER];
    h[..8].copy_from_slice(MAGIC);
    h[8] = match descriptor.class() {
        Class::M2 => 1,
        Class::Wmo => 2,
    };
    h[9] = u8::from(e.bounds.is_some());
    h[12..20].copy_from_slice(&(e.handle.index() as u64).to_le_bytes());
    h[20..24].copy_from_slice(&source.file_data_id().to_le_bytes());
    h[24..40].copy_from_slice(&source.content_key());
    h[40..48].copy_from_slice(&(source.byte_count() as u64).to_le_bytes());
    if let Some(magic) = source.first_magic() {
        h[48] = 1;
        h[49..53].copy_from_slice(&magic);
    }
    h[56..60].copy_from_slice(&(name.len() as u32).to_le_bytes());
    h[60..68].copy_from_slice(&(e.bytes as u64).to_le_bytes());
    if let Some(bounds) = e.bounds {
        for (part, bits) in h[68..].as_chunks_mut::<4>().0.iter_mut().zip(bounds) {
            part.copy_from_slice(&bits.to_le_bytes());
        }
    }
    let mut n = [0; NAME_BYTES];
    n[..name.len()].copy_from_slice(name);
    Ok(Identity {
        header: h,
        name: n,
        len: name.len(),
    })
}
impl Prepared {
    fn live(&self) -> Result<usize, Kind> {
        add(self.base, mul(self.index.capacity(), size_of::<Entry>())?)
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
        let p = self.stage.guards();
        Guards {
            collector: p.collector,
            placements: p.stage,
            archive: self.repository.verify_unchanged().err(),
        }
    }
    fn guarded<T, E: Copy>(&mut self, result: Result<T, Error<E>>) -> Result<T, Error<E>> {
        let mut guards = self.guards();
        let mut error = match result {
            Err(e) => e,
            Ok(value) => {
                if guards.collector.is_none()
                    && guards.placements.is_none()
                    && guards.archive.is_none()
                    && self.failed.is_none()
                {
                    return Ok(value);
                }
                drop(value);
                guards = self.guards();
                Error::from(self.failed.unwrap_or(Kind::State))
            }
        };
        // Existing typed causes survive every independent observed change.
        if guards.collector.is_some() {
            error.guards.collector = guards.collector;
        }
        if guards.placements.is_some() {
            error.guards.placements = guards.placements;
        }
        if guards.archive.is_some() {
            error.guards.archive = guards.archive;
        }
        self.failed.get_or_insert(Kind::State);
        Err(error)
    }
    pub fn verify_unchanged(&mut self) -> Result<(), Error> {
        let r = self.failed.map_or(Ok(()), |e| Err(e.into()));
        self.guarded(r)
    }
    pub const fn stage(&self) -> &Staged {
        &self.stage
    }
    pub fn stats(&self) -> Result<Stats, Error> {
        self.failed.map_or(Ok(()), |e| Err(Error::from(e)))?;
        Ok(self.stats)
    }
    pub fn models(&self) -> Result<impl Iterator<Item = (&[u8], Class)> + '_, Error> {
        self.stats()?;
        let compact = self.stage.world.startup().extraction().phase_compact()?;
        Ok(self.index.iter().filter(|e| e.selected).map(move |e| {
            let d = compact.descriptor(e.handle).expect("closed source handle");
            (d.output_basename(), d.class())
        }))
    }
    fn binding(&self, name: &[u8]) -> Result<usize, Kind> {
        let view = self
            .stage
            .world
            .startup()
            .extraction()
            .phase_compact()
            .map_err(|_| Kind::State)?;
        let at = self
            .index
            .binary_search_by(|e| {
                view.descriptor(e.handle)
                    .expect("closed source handle")
                    .output_basename()
                    .cmp(name)
            })
            .map_err(|_| Kind::Binding)?;
        let descriptor = view
            .descriptor(self.index[at].handle)
            .map_err(|_| Kind::Binding)?;
        if descriptor.status() != Status::Collision {
            return Err(Kind::Binding);
        }
        Ok(at)
    }
    fn select(&mut self) -> Result<(), Error> {
        let count = self
            .stage
            .world
            .startup()
            .extraction()
            .phase_compact()?
            .model_count();
        if count > self.repository.resources().collector_limits().max_models {
            return Err(Kind::Limit.into());
        }
        self.admit(mul(count, size_of::<Entry>())?)?;
        self.index
            .try_reserve_exact(count)
            .map_err(|_| Kind::Allocation)?;
        self.admit(0)?;
        self.index.extend((0..count).map(|i| Entry {
            handle: ModelHandle(i),
            selected: false,
            bounds: None,
            record: None,
            bytes: 0,
        }));
        let source = self.stage.world.startup().extraction().phase_compact()?;
        self.index.sort_unstable_by(|a, b| {
            source
                .descriptor(a.handle)
                .expect("closed source handle")
                .output_basename()
                .cmp(
                    source
                        .descriptor(b.handle)
                        .expect("closed source handle")
                        .output_basename(),
                )
        });
        if self.index.windows(2).any(|w| {
            source
                .descriptor(w[0].handle)
                .expect("closed source handle")
                .output_basename()
                == source
                    .descriptor(w[1].handle)
                    .expect("closed source handle")
                    .output_basename()
        }) {
            return Err(Kind::Binding.into());
        }
        // One map buffer stays inside the fully reserved placement component.
        for at in 0..self.stage.stage.entries.len() {
            let entry = self.stage.stage.entries[at];
            if !matches!(entry.context.kind, crate::collision::run::Kind::Global) {
                continue;
            }
            self.stage.verify_unchanged()?;
            let bytes = self
                .stage
                .stage
                .read_map(entry.context.map_id, 0)
                .map_err(|e| Error::from(super::Error::from(e)))?;
            let view =
                forever_vmap::placement::DirBin::parse(&bytes, self.stage.stage.limits.records)
                    .map_err(Kind::Placement)?;
            for row in view.records() {
                let at = self.binding(row.name())?;
                self.index[at].selected = true;
                bump(
                    &mut self.stats.placement_records,
                    1,
                    super::super::MAX_RECORDS,
                )?;
            }
            drop(bytes);
            self.stage.verify_unchanged()?;
            bump(&mut self.stats.maps_scanned, 1, super::super::MAX_MAPS)?;
        }
        let data = self.stage.world.startup().data().map_err(|e| {
            Error::from(super::Error {
                cause: super::Cause::Startup(e),
                guards: Default::default(),
            })
        })?;
        let list = gameobject_models::TempList::parse(
            data.lists().temp_bytes(),
            gameobject_models::Limits::default(),
        )
        .map_err(Kind::Lists)?;
        for row in list.rows() {
            let at = self.binding(row.name())?;
            self.index[at].selected = true;
            bump(&mut self.stats.list_rows, 1, super::super::MAX_RECORDS)?;
        }
        self.stats.selected_models = self.index.iter().filter(|e| e.selected).count();
        if self.stats.selected_models > self.limits.max_models {
            return Err(Kind::Limit.into());
        }
        Ok(())
    }
    fn scan(&mut self) -> Result<(), Error> {
        // Aggregate geometry and bounds are admitted for EVERY selected model
        // before the first conversion or archive write, as in resident assembly.
        for at in 0..self.index.len() {
            if !self.index[at].selected {
                continue;
            }
            self.verify_unchanged()?;
            let e = self.index[at];
            let shape = self
                .stage
                .world
                .startup()
                .extraction()
                .phase_shape(e.handle)?;
            let bytes = shape.raw_bytes.ok_or(Kind::Binding)?;
            self.admit(shape.record_bytes)?;
            bump64(
                &mut self.stats.raw_visit_bytes,
                (bytes as u64)
                    .checked_mul(if shape.class == Class::M2 { 1 } else { 2 })
                    .ok_or(Kind::Limit)?,
                self.limits.max_raw_visit_bytes,
            )?;
            let result = self.stage.world.extraction_mut().with_payload_phase(
                e.handle,
                shape.record_bytes,
                0,
                |p| {
                    let bytes = p.raw().ok_or(Kind::Binding)?;
                    match shape.class {
                        Class::M2 => {
                            let v = RawM2::parse(bytes, forever_vmap::Limits::default())
                                .map_err(Kind::RawM2)?;
                            Ok((Some(v.bounds_bits()), v.vertex_count(), v.triangle_count()))
                        }
                        Class::Wmo => {
                            let v = raw::RawWmo::parse(bytes, raw::ConversionLimits::default())
                                .map_err(Kind::RawWmo)?;
                            let mut vertices = 0;
                            let mut triangles = 0;
                            for g in v.groups() {
                                vertices = add(vertices, g.vertex_count())?;
                                triangles = add(triangles, g.triangle_count())?;
                            }
                            Ok((None, vertices, triangles))
                        }
                    }
                },
            );
            let (bounds, vertices, triangles) = match result {
                Ok(v) => v,
                Err(CallbackFailure::Collector(e)) => return Err(e.into()),
                Err(CallbackFailure::Callback { cause, guard }) => {
                    return Err(Error {
                        cause: Cause::Kind(cause),
                        guards: Guards {
                            collector: guard,
                            ..Default::default()
                        },
                    });
                }
            };
            self.index[at].bounds = bounds;
            bump(&mut self.stats.vertices, vertices, self.limits.max_vertices)?;
            bump(
                &mut self.stats.triangles,
                triangles,
                self.limits.max_triangles,
            )?;
        }
        Ok(())
    }
    fn convert(&mut self) -> Result<(), Error> {
        for at in 0..self.index.len() {
            if !self.index[at].selected {
                continue;
            }
            self.verify_unchanged()?;
            let e = self.index[at];
            let shape = self
                .stage
                .world
                .startup()
                .extraction()
                .phase_shape(e.handle)?;
            let raw_bytes = shape.raw_bytes.ok_or(Kind::Binding)?;
            bump64(
                &mut self.stats.raw_visit_bytes,
                (raw_bytes as u64)
                    .checked_mul(if shape.class == Class::M2 { 2 } else { 3 })
                    .ok_or(Kind::Limit)?,
                self.limits.max_raw_visit_bytes,
            )?;
            let mut binding = identity(self.stage.world.startup().extraction(), e)?;
            let mut c = self.limits.conversion;
            let envelope = add(HEADER, binding.len)?;
            let disk_remaining = self
                .repository
                .limits()
                .max_disk_bytes
                .checked_sub(self.repository.stats().map_err(Kind::Spool)?.disk_bytes)
                .ok_or(Kind::Limit)?;
            c.max_bytes = c
                .max_bytes
                .min(self.limits.max_output_bytes - self.stats.output_bytes)
                .min(disk_remaining.checked_sub(envelope).ok_or(Kind::Limit)?)
                .min(
                    self.repository
                        .limits()
                        .max_active_bytes
                        .checked_sub(envelope)
                        .ok_or(Kind::Limit)?,
                );
            c.max_work = c
                .max_work
                .min(self.limits.max_bih_work - self.stats.bih_work);
            let free = self
                .limits
                .max_memory_bytes
                .checked_sub(self.live()?)
                .and_then(|n| n.checked_sub(shape.record_bytes))
                .ok_or(Kind::Limit)?;
            c.max_memory_bytes = c.max_memory_bytes.min(free);
            self.admit(add(shape.record_bytes, c.max_memory_bytes)?)?;
            let limits = self.limits;
            let stats = &mut self.stats;
            let repository = &mut self.repository;
            let result = self.stage.world.extraction_mut().with_payload_phase(
                e.handle,
                shape.record_bytes,
                c.max_memory_bytes,
                |p| {
                    let bytes = p.raw().ok_or(Kind::Binding)?;
                    let converted = match shape.class {
                        Class::M2 => {
                            let parsed = RawM2::parse(bytes, forever_vmap::Limits::default())
                                .map_err(Kind::RawM2)?;
                            if Some(parsed.bounds_bits()) != e.bounds {
                                return Err(Kind::Binding);
                            }
                            vmo::convert_m2(&parsed, c)
                        }
                        Class::Wmo => {
                            if e.bounds.is_some() {
                                return Err(Kind::Class);
                            }
                            let parsed =
                                raw::RawWmo::parse(bytes, raw::ConversionLimits::default())
                                    .map_err(Kind::RawWmo)?;
                            vmo::convert_wmo(&parsed, c)
                        }
                    }
                    .map_err(Kind::Vmo)?;
                    let s = converted.stats();
                    bump(
                        &mut stats.output_bytes,
                        converted.bytes().len(),
                        limits.max_output_bytes,
                    )?;
                    bump64(&mut stats.bih_work, s.build_work, limits.max_bih_work)?;
                    binding.header[60..68]
                        .copy_from_slice(&(converted.bytes().len() as u64).to_le_bytes());
                    let record = repository
                        .write(add(envelope, converted.bytes().len())?, |out| {
                            out.bytes(&binding.header)?;
                            out.bytes(&binding.name[..binding.len])?;
                            out.bytes(converted.bytes())
                        })
                        .map_err(Kind::Spool)?;
                    let len = converted.bytes().len();
                    drop(converted);
                    Ok((record, len))
                },
            );
            match result {
                Ok((record, len)) => {
                    self.index[at].record = Some(record);
                    self.index[at].bytes = len;
                }
                Err(CallbackFailure::Collector(e)) => return Err(e.into()),
                Err(CallbackFailure::Callback { cause, guard }) => {
                    return Err(Error {
                        cause: Cause::Kind(cause),
                        guards: Guards {
                            collector: guard,
                            ..Default::default()
                        },
                    });
                }
            }
        }
        Ok(())
    }
    /// Reload one complete strictly parsed VMO. Caller allocations must obey
    /// the declared allowance. Captures, reader/input buffers and rejected
    /// return values drop before all independent final phase/archive guards.
    pub fn with_model<T, E: Copy>(
        &mut self,
        name: &[u8],
        callback_heap_allowance: usize,
        call: impl FnOnce(&vmo::View<'_>) -> Result<T, E>,
    ) -> Result<T, Error<E>> {
        let setup = (|| {
            self.verify_unchanged().map_err(|e| Error {
                cause: match e.cause {
                    Cause::Phase(c) => Cause::Phase(c),
                    Cause::Kind(k) => Cause::Kind(k),
                    Cause::Collector(c) => Cause::Collector(c),
                    Cause::Callback(()) => Cause::Kind(Kind::Callback),
                },
                guards: e.guards,
            })?;
            let at = self.binding(name)?;
            let entry = self.index[at];
            if !entry.selected || entry.bytes == 0 {
                return Err(Kind::Missing.into());
            }
            let binding =
                identity(self.stage.world.startup().extraction(), entry).map_err(|e| Error {
                    cause: match e.cause {
                        Cause::Phase(c) => Cause::Phase(c),
                        Cause::Kind(k) => Cause::Kind(k),
                        Cause::Collector(c) => Cause::Collector(c),
                        Cause::Callback(()) => Cause::Kind(Kind::Callback),
                    },
                    guards: e.guards,
                })?;
            let extent = add(add(HEADER, binding.len)?, entry.bytes)?;
            let free = self
                .limits
                .max_memory_bytes
                .checked_sub(self.live()?)
                .and_then(|n| n.checked_sub(extent))
                .and_then(|n| n.checked_sub(callback_heap_allowance))
                .ok_or(Kind::Limit)?;
            let mut reader = self.limits.conversion;
            reader.max_memory_bytes = reader.max_memory_bytes.min(free);
            self.admit(add(
                add(extent, reader.max_memory_bytes)?,
                callback_heap_allowance,
            )?)?;
            bump64(
                &mut self.stats.validation_bytes,
                entry.bytes as u64,
                self.limits.max_validation_bytes,
            )?;
            Ok((entry, binding, extent, reader))
        })();
        let (entry, binding, extent, reader) = match setup {
            Ok(v) => v,
            Err(e) => {
                drop(call);
                return self.guarded(Err(e));
            }
        };
        let mut outcome = None;
        let io = self
            .repository
            .with_record(
                entry.record.expect("sealed selected record"),
                extent,
                |bytes| {
                    outcome = Some((|| {
                        if bytes.len() != extent
                            || bytes[..HEADER] != binding.header
                            || bytes[HEADER..HEADER + binding.len] != binding.name[..binding.len]
                        {
                            return Err(Error::from(Kind::Binding));
                        }
                        let view = vmo::View::parse(&bytes[HEADER + binding.len..], reader)
                            .map_err(Kind::Vmo)?;
                        let class = binding.header[8];
                        if (class == 1 && view.flags() != 1)
                            || (class == 2 && view.flags() != 0)
                            || (class == 1
                                && view.groups().first().map(|g| g.bounds_bits()) != entry.bounds)
                        {
                            return Err(Kind::Class.into());
                        }
                        call(&view).map_err(|e| Error {
                            cause: Cause::Callback(e),
                            guards: Default::default(),
                        })
                    })());
                    Ok(())
                },
            )
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
}
/// Sole constructor: no supplied model identities or selected-name inventory.
/// Errors consume the authenticated phase and return no usable partial archive.
pub fn prepare(
    mut stage: Staged,
    mut repository: spool::Spool,
    limits: Limits,
) -> Result<Prepared, Error> {
    let admission: Result<usize, Error> = (|| {
        if repository.resources() != stage.stage.repository.resources()
            || repository.backend() != stage.stage.repository.backend()
        {
            return Err(Kind::InvalidLimits.into());
        }
        limits.validate_resources(repository.resources())?;
        stage.verify_unchanged()?;
        repository.verify_unchanged().map_err(Kind::Spool)?;
        if repository.stats().map_err(Kind::Spool)?.records != 0 {
            return Err(Kind::State.into());
        }
        let base = add(
            add(
                stage.world.startup().extraction().phase_resident_bytes()?,
                stage.stage.limits.max_heap_bytes,
            )?,
            add(
                mul(repository.limits().max_descriptor_bytes, 2)?,
                GUARD_BYTES,
            )?,
        )?;
        if base > limits.max_memory_bytes {
            return Err(Kind::Limit.into());
        }
        Ok(base)
    })();
    let base = match admission {
        Ok(n) => n,
        Err(mut e) => {
            let g = stage.guards();
            e.guards = Guards {
                collector: g.collector,
                placements: g.stage,
                archive: repository.verify_unchanged().err(),
            };
            return Err(e);
        }
    };
    let mut out = Prepared {
        stage,
        repository,
        index: Vec::new(),
        limits,
        base,
        stats: Stats {
            reserved_bytes: base,
            peak_admitted_bytes: base,
            ..Default::default()
        },
        failed: None,
    };
    let result = out
        .select()
        .and_then(|()| out.scan())
        .and_then(|()| out.convert());
    out.guarded(result)?;
    out.stats.reserved_bytes = out.live()?;
    Ok(out)
}
