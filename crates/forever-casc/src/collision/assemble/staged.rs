// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-12: original closed authenticated world placement disk phase.
//! This is a private intermediate phase, not a final VMap package. The sole
//! constructor consumes the authenticated startup; no caller placements or
//! association proof can construct Staged. Original callback contexts/counts
//! and bytes are bound in immutable envelopes. All records, including duplicate
//! IDs and empty successful callbacks, survive. A complete map is reloaded at a
//! time in global-then-x/y order, matching the resident assembler's append order.
//!
//! A disjoint full stage heap reservation is subtracted from the combined world
//! ceiling BEFORE acquisition. Stage growth overlap and complete map reloads
//! stay inside it. The collector's own ceiling is also reduced, so completed RAW
//! reloads cannot reuse that reservation. Source/parser/projection memory stays
//! with the original collector. Borrowed input and declared callback allocations
//! are caller-owned; counters are conservative logical admission, not RSS.
//! Same-mount/disk/read/reload limits of v1 repositories remain unchanged.
use crate::collision::{gameobjects, inventory_spool as inventory, payload, run, spool, world};
use forever_vmap::placement::{self, DirBin};
use std::mem::size_of;
#[cfg(test)]
mod tests;
pub const PROFILE: &str = "linux-private-authenticated-placement-stage-v1";
pub const MAX_HEAP_BYTES: usize = super::MAX_STAGE_BYTES;
pub const MAX_CALLBACKS: usize = 4096;
const GUARD_BYTES: usize = 512 * 1024;
const MAGIC: &[u8; 8] = b"RCEPLC1\0";
const HEADER: usize = 48;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub world: world::inventory::Limits,
    pub max_heap_bytes: usize,
    pub max_callbacks: usize,
    pub max_records: usize,
    pub records: placement::Limits,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            world: world::inventory::Limits::default(),
            max_heap_bytes: MAX_HEAP_BYTES,
            max_callbacks: MAX_CALLBACKS,
            max_records: super::MAX_RECORDS,
            records: placement::Limits::default(),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StageError {
    InvalidLimits,
    Limit,
    Allocation,
    State,
    Binding,
    Missing,
    Duplicate,
    Callback,
    Spool(payload::Error),
    Placement(placement::Error),
}
impl StageError {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidLimits => "collision_placement_stage_invalid_limits",
            Self::Limit => "collision_placement_stage_limit",
            Self::Allocation => "collision_placement_stage_allocation",
            Self::State => "collision_placement_stage_state",
            Self::Binding => "collision_placement_stage_binding",
            Self::Missing => "collision_placement_stage_missing",
            Self::Duplicate => "collision_placement_stage_duplicate",
            Self::Callback => "collision_placement_stage_callback",
            Self::Spool(e) => e.code(),
            Self::Placement(e) => e.code(),
        }
    }
}
impl std::fmt::Display for StageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}
impl std::error::Error for StageError {}
impl From<payload::Error> for StageError {
    fn from(e: payload::Error) -> Self {
        Self::Spool(e)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cause {
    Stage(StageError),
    World(world::inventory::Cause<StageError>),
    Collector(payload::CollectorError),
    Startup(gameobjects::spooled::Error),
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Guards {
    pub collector: Option<payload::CollectorError>,
    pub wdt: Option<inventory::Error>,
    pub cache: Option<inventory::Error>,
    pub stage: Option<StageError>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Error {
    pub cause: Cause,
    pub guards: Guards,
}
fn collector_changed(e: payload::CollectorError) -> bool {
    matches!(
        e,
        payload::CollectorError::Payload(payload::Error::Changed)
            | payload::CollectorError::Collision(crate::collision::Error::Store(
                crate::local_store::Error::InputChanged
            ))
    )
}
impl Error {
    pub fn code(self) -> &'static str {
        if let Some(e) = self.guards.collector
            && collector_changed(e)
        {
            return e.code();
        }
        if let Cause::World(e) = self.cause
            && matches!(
                world_code(e),
                "local_store_input_changed" | "collision_payload_changed"
            )
        {
            return world_code(e);
        }
        if [self.guards.wdt, self.guards.cache]
            .contains(&Some(inventory::Error::Spool(payload::Error::Changed)))
            || self.guards.stage == Some(StageError::Spool(payload::Error::Changed))
        {
            return payload::Error::Changed.code();
        }
        if let Some(e) = self.guards.stage {
            return e.code();
        }
        match self.cause {
            Cause::Stage(e) => e.code(),
            Cause::Collector(e) => e.code(),
            Cause::World(e) => world_code(e),
            Cause::Startup(e) => e.code(),
        }
    }
}
impl From<StageError> for Error {
    fn from(e: StageError) -> Self {
        Self {
            cause: Cause::Stage(e),
            guards: Guards::default(),
        }
    }
}
impl From<payload::CollectorError> for Error {
    fn from(e: payload::CollectorError) -> Self {
        Self {
            cause: Cause::Collector(e),
            guards: Guards::default(),
        }
    }
}
// Preserve each world's typed cause and every outer guard exactly once.
// Nested run/bridge guards remain in the typed cause, with no heap allocation.
fn world_code(cause: world::inventory::Cause<StageError>) -> &'static str {
    match cause {
        world::inventory::Cause::World(e) => e.code(),
        world::inventory::Cause::Collector(e) => e.code(),
        world::inventory::Cause::Startup(e) => e.code(),
        world::inventory::Cause::Run(e) => e.code(),
        world::inventory::Cause::Inventory(e) => e.code(),
    }
}
impl From<world::inventory::Error<StageError>> for Error {
    fn from(e: world::inventory::Error<StageError>) -> Self {
        let [wdt, cache] = e.inventory_guards();
        Self {
            cause: Cause::World(e.cause()),
            guards: Guards {
                collector: e.guard(),
                wdt,
                cache,
                stage: None,
            },
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}
impl std::error::Error for Error {}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccessCause<E> {
    Access(Cause),
    Callback(E),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AccessError<E> {
    pub cause: AccessCause<E>,
    pub guards: Guards,
}
impl<E: Copy> AccessError<E> {
    pub fn code(self) -> &'static str {
        let cause = match self.cause {
            AccessCause::Access(cause) => cause,
            AccessCause::Callback(_) => Cause::Stage(StageError::Callback),
        };
        Error {
            cause,
            guards: self.guards,
        }
        .code()
    }
}
#[derive(Clone, Copy)]
struct Entry {
    context: run::Context,
    sequence: usize,
    records: usize,
    bytes: usize,
    record: spool::Record,
}
fn cell(c: run::Context) -> Result<u16, StageError> {
    if c.replayed != (c.map_id != c.original_map_id) {
        return Err(StageError::Binding);
    }
    match c.kind {
        run::Kind::Global if !c.replayed => Ok(0),
        run::Kind::Tile { x, y } if x < 64 && y < 64 => Ok(1 + u16::from(x) * 64 + u16::from(y)),
        _ => Err(StageError::Binding),
    }
}
impl Entry {
    fn key(self) -> (u32, u16) {
        (
            self.context.map_id,
            cell(self.context).expect("admitted context"),
        )
    }
    fn header(self) -> [u8; HEADER] {
        header(self.context, self.sequence, self.records, self.bytes)
    }
    fn body(self, b: &[u8], limits: placement::Limits) -> Result<&[u8], StageError> {
        if b.len() != add(HEADER, self.bytes)? || b[..HEADER] != self.header() {
            return Err(StageError::Binding);
        }
        let body = &b[HEADER..];
        let view = DirBin::parse(body, limits).map_err(StageError::Placement)?;
        if view.record_count() != self.records
            || view
                .records()
                .any(|r| (r.flags() & 2 != 0) != self.context.replayed)
        {
            return Err(StageError::Binding);
        }
        Ok(body)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stats {
    pub maps: usize,
    pub callbacks: usize,
    pub records: usize,
    pub placement_bytes: usize,
    pub reserved_heap_bytes: usize,
    pub peak_admitted_heap_bytes: usize,
    pub repository: spool::Stats,
}
struct Stage {
    repository: spool::Spool,
    entries: Vec<Entry>,
    limits: Limits,
    base: usize,
    peak: usize,
    records: usize,
    bytes: usize,
    failed: Option<StageError>,
}
fn add(a: usize, b: usize) -> Result<usize, StageError> {
    a.checked_add(b).ok_or(StageError::Limit)
}
fn mul(a: usize, b: usize) -> Result<usize, StageError> {
    a.checked_mul(b).ok_or(StageError::Limit)
}
impl Stage {
    fn new(repository: spool::Spool, limits: Limits) -> Result<Self, StageError> {
        if limits.max_heap_bytes > MAX_HEAP_BYTES
            || limits.max_callbacks > MAX_CALLBACKS
            || limits.max_records > super::MAX_RECORDS
        {
            return Err(StageError::InvalidLimits);
        }
        DirBin::parse(&[], limits.records).map_err(StageError::Placement)?;
        // Twice the repository descriptor ceiling covers replacement growth;
        // fixed guard work and typed entries are disjoint from that repository.
        let base = add(
            mul(repository.limits().max_descriptor_bytes, 2)?,
            GUARD_BYTES,
        )?;
        let mut stage = Self {
            repository,
            entries: Vec::new(),
            limits,
            base,
            peak: base,
            records: 0,
            bytes: 0,
            failed: None,
        };
        let result = (|| {
            stage.check()?;
            if stage.repository.stats()?.records != 0 {
                return Err(StageError::State);
            }
            stage.admit(0)
        })();
        stage.finish(result)?;
        Ok(stage)
    }
    fn heap(&self) -> Result<usize, StageError> {
        add(self.base, mul(self.entries.capacity(), size_of::<Entry>())?)
    }
    fn admit(&mut self, active: usize) -> Result<(), StageError> {
        let total = add(self.heap()?, active)?;
        if total > self.limits.max_heap_bytes {
            return Err(StageError::Limit);
        }
        self.peak = self.peak.max(total);
        Ok(())
    }
    fn check(&mut self) -> Result<(), StageError> {
        let guard = self
            .repository
            .verify_unchanged()
            .map_err(StageError::Spool);
        if guard == Err(StageError::Spool(payload::Error::Changed)) {
            self.failed = guard.err();
        }
        guard.and(self.failed.map_or(Ok(()), Err))
    }
    fn finish<T>(&mut self, result: Result<T, StageError>) -> Result<T, StageError> {
        let guard = self.check();
        match (guard, result) {
            (Err(e), Ok(value)) => {
                drop(value);
                let final_guard = self.check();
                Err(final_guard.err().unwrap_or(e))
            }
            (Err(e), Err(_)) => Err(e),
            (Ok(()), Err(e)) => {
                self.failed = Some(e);
                Err(e)
            }
            (Ok(()), Ok(value)) => Ok(value),
        }
    }
    fn append(&mut self, context: run::Context, bytes: &[u8]) -> Result<(), StageError> {
        let result = (|| {
            self.check()?;
            let key = (context.map_id, cell(context)?);
            let at = self
                .entries
                .binary_search_by_key(&key, |e| e.key())
                .err()
                .ok_or(StageError::Duplicate)?;
            if self.entries.len() >= self.limits.max_callbacks {
                return Err(StageError::Limit);
            }
            let parsed =
                DirBin::parse(bytes, self.limits.records).map_err(StageError::Placement)?;
            if parsed
                .records()
                .any(|r| (r.flags() & 2 != 0) != context.replayed)
            {
                return Err(StageError::Binding);
            }
            let count = parsed.record_count();
            let records = add(self.records, count)?;
            if records > self.limits.max_records {
                return Err(StageError::Limit);
            }
            let total_bytes = add(self.bytes, bytes.len())?;
            let extent = add(HEADER, bytes.len())?;
            if extent > self.repository.limits().max_active_bytes {
                return Err(StageError::Limit);
            }
            self.admit(0)?; // borrowed bytes stay charged to the source collector
            if self.entries.len() == self.entries.capacity() {
                // Allocate replacement explicitly; account old/new overlap.
                let n = add(self.entries.len(), 1)?;
                self.admit(mul(n, size_of::<Entry>())?)?;
                let mut next = Vec::new();
                next.try_reserve_exact(n)
                    .map_err(|_| StageError::Allocation)?;
                self.admit(mul(next.capacity(), size_of::<Entry>())?)?;
                next.append(&mut self.entries);
                self.entries = next;
            }
            // Record ordinal is opaque and never leaves this private owner.
            let sequence = self.entries.len();
            let h = header(context, sequence, count, bytes.len());
            let record = self.repository.write(extent, |out| {
                out.bytes(&h)?;
                out.bytes(bytes)
            })?;
            self.entries.insert(
                at,
                Entry {
                    context,
                    sequence,
                    records: count,
                    bytes: bytes.len(),
                    record,
                },
            );
            self.records = records;
            self.bytes = total_bytes;
            Ok(())
        })();
        self.finish(result)
    }
    fn range(&self, map: u32) -> Result<std::ops::Range<usize>, StageError> {
        let start = self.entries.partition_point(|e| e.context.map_id < map);
        let end = self.entries.partition_point(|e| e.context.map_id <= map);
        if start == end {
            return Err(StageError::Missing);
        }
        Ok(start..end)
    }
    fn read_map(
        &mut self,
        map: u32,
        callback_heap_allowance: usize,
    ) -> Result<Vec<u8>, StageError> {
        let result = (|| {
            self.check()?;
            let range = self.range(map)?;
            let mut size = 0;
            let mut largest = 0;
            let mut records = 0;
            for e in &self.entries[range.clone()] {
                size = add(size, e.bytes)?;
                largest = largest.max(add(HEADER, e.bytes)?);
                records = add(records, e.records)?;
            }
            if size > self.limits.records.max_bytes || records > self.limits.records.max_records {
                return Err(StageError::Limit);
            }
            self.admit(add(add(size, largest)?, callback_heap_allowance)?)?;
            let mut bytes = Vec::new();
            bytes
                .try_reserve_exact(size)
                .map_err(|_| StageError::Allocation)?;
            self.admit(add(
                add(bytes.capacity(), largest)?,
                callback_heap_allowance,
            )?)?;
            for at in range {
                let entry = self.entries[at];
                let mut parse_error = None;
                self.repository
                    .with_record(entry.record, add(HEADER, entry.bytes)?, |b| {
                        match entry.body(b, self.limits.records) {
                            Ok(body) => bytes.extend_from_slice(body),
                            Err(e) => parse_error = Some(e),
                        }
                        Ok(())
                    })?;
                if let Some(e) = parse_error {
                    return Err(e);
                }
            }
            let view = DirBin::parse(&bytes, self.limits.records).map_err(StageError::Placement)?;
            if bytes.len() != size || view.record_count() != records {
                return Err(StageError::Binding);
            }
            Ok(bytes)
        })();
        self.finish(result)
    }
    fn stats(&self) -> Result<Stats, StageError> {
        Ok(Stats {
            maps: self
                .entries
                .iter()
                .filter(|e| matches!(e.context.kind, run::Kind::Global))
                .count(),
            callbacks: self.entries.len(),
            records: self.records,
            placement_bytes: self.bytes,
            reserved_heap_bytes: self.heap()?,
            peak_admitted_heap_bytes: self.peak,
            repository: self.repository.stats()?,
        })
    }
    fn validate_topology(&mut self, topology: &world::Topology) -> Result<(), StageError> {
        let result = (|| {
            self.check()?;
            let active = topology.all().iter().filter(|m| m.active()).count();
            if active != self.stats()?.maps {
                return Err(StageError::Binding);
            }
            for e in &self.entries {
                let map = topology
                    .get(e.context.map_id)
                    .filter(|m| m.active())
                    .ok_or(StageError::Binding)?;
                if e.context.replayed && map.parent_map_id() != Some(e.context.original_map_id) {
                    return Err(StageError::Binding);
                }
                if self.entries[self.range(e.context.map_id)?.start].key().1 != 0 {
                    return Err(StageError::Binding);
                }
            }
            Ok(())
        })();
        self.finish(result)
    }
}
fn header(context: run::Context, sequence: usize, records: usize, bytes: usize) -> [u8; HEADER] {
    // Same immutable envelope used on reload; no forged public handle exists.
    let mut h = [0; HEADER];
    h[..8].copy_from_slice(MAGIC);
    h[8..12].copy_from_slice(&context.map_id.to_le_bytes());
    h[12..16].copy_from_slice(&context.original_map_id.to_le_bytes());
    if let run::Kind::Tile { x, y } = context.kind {
        h[16] = 1;
        h[17] = x;
        h[18] = y;
    }
    h[19] = u8::from(context.replayed);
    h[20..28].copy_from_slice(&(sequence as u64).to_le_bytes());
    h[28..36].copy_from_slice(&(records as u64).to_le_bytes());
    h[36..44].copy_from_slice(&(bytes as u64).to_le_bytes());
    h
}
struct Capture<'a>(&'a mut Stage);
impl run::inventory::Sink for Capture<'_> {
    type Error = StageError;
    fn placements(&mut self, c: run::Context, b: &[u8]) -> Result<(), StageError> {
        self.0.append(c, b)
    }
}
/// Closed acquired world plus private placement inventory. This intermediate
/// owner deliberately cannot enumerate final VMap artifacts or be published.
pub struct Staged {
    world: world::inventory::Completed,
    stage: Stage,
}
impl Staged {
    pub const fn world(&self) -> &world::inventory::Completed {
        &self.world
    }
    pub fn stats(&self) -> Result<Stats, Error> {
        self.world.startup().data().map_err(|e| Error {
            cause: Cause::Startup(e),
            guards: Guards::default(),
        })?;
        self.stage.failed.map_or(Ok(()), |e| Err(Error::from(e)))?;
        self.stage.stats().map_err(Error::from)
    }
    pub fn maps(&self) -> Result<impl Iterator<Item = u32> + '_, Error> {
        self.stats()?;
        Ok(self
            .stage
            .entries
            .iter()
            .filter(|e| matches!(e.context.kind, run::Kind::Global))
            .map(|e| e.context.map_id))
    }
    fn guards(&mut self) -> Guards {
        Guards {
            collector: self.world.extraction_mut().verify_unchanged().err(),
            stage: self.stage.check().err(),
            ..Guards::default()
        }
    }
    pub fn verify_unchanged(&mut self) -> Result<(), Error> {
        let guards = self.guards();
        let cause = guards
            .collector
            .map(Cause::Collector)
            .or_else(|| guards.stage.map(Cause::Stage));
        match cause {
            None => Ok(()),
            Some(cause) => Err(Error { cause, guards }),
        }
    }
    /// The complete map buffer stays internal, is dropped before final guards,
    /// and never aliases callbacks. Callback allocations must obey the declared
    /// allowance; a returned value becomes caller-owned after successful guards.
    /// Every access/callback failure permanently invalidates this phase owner.
    pub fn with_map<T, E: Copy>(
        &mut self,
        map: u32,
        callback_heap_allowance: usize,
        call: impl FnOnce(&DirBin<'_>) -> Result<T, E>,
    ) -> Result<T, AccessError<E>> {
        let setup = self.verify_unchanged().and_then(|()| {
            self.stage
                .read_map(map, callback_heap_allowance)
                .map_err(Error::from)
        });
        let bytes = match setup {
            Ok(b) => b,
            Err(e) => {
                drop(call);
                return Err(AccessError {
                    cause: AccessCause::Access(e.cause),
                    guards: self.guards(),
                });
            }
        };
        let result = {
            let view =
                DirBin::parse(&bytes, self.stage.limits.records).expect("complete verified map");
            call(&view)
        };
        drop(bytes);
        let mut guards = self.guards();
        let cause = match result {
            Err(e) => AccessCause::Callback(e),
            Ok(value) => {
                if guards.collector.is_none() && guards.stage.is_none() {
                    return Ok(value);
                }
                drop(value);
                guards = self.guards();
                AccessCause::Access(
                    guards
                        .collector
                        .map(Cause::Collector)
                        .unwrap_or_else(|| Cause::Stage(guards.stage.unwrap_or(StageError::State))),
                )
            }
        };
        if self.stage.failed.is_none() {
            self.stage.failed = Some(StageError::Callback);
        }
        // Recheck after callback captures, failed values and internal buffer Drop.
        guards = self.guards();
        Err(AccessError { cause, guards })
    }
}
fn input_guards(
    startup: &mut gameobjects::spooled::Startup<'_>,
    wdts: &mut inventory::Inventory,
    caches: &mut inventory::Inventory,
    repository: &mut spool::Spool,
) -> Guards {
    Guards {
        collector: startup.collector_mut().verify_unchanged().err(),
        wdt: wdts.verify_unchanged().err(),
        cache: caches.verify_unchanged().err(),
        stage: repository.verify_unchanged().err().map(StageError::Spool),
    }
}
/// Acquire through the existing authenticated disk world using a private Sink.
/// Full stage and inventory reservations plus the collector's reduced ceiling
/// must fit the original combined limit; this never increases a v1 component.
/// All output remains a private intermediate, even on complete success.
pub fn extract(
    mut startup: gameobjects::spooled::Startup<'_>,
    mut wdts: inventory::Inventory,
    mut caches: inventory::Inventory,
    mut repository: spool::Spool,
    limits: Limits,
) -> Result<Staged, Error> {
    let admission: Result<usize, Error> = (|| {
        if limits.max_heap_bytes > MAX_HEAP_BYTES
            || limits.max_callbacks > MAX_CALLBACKS
            || limits.max_records > super::MAX_RECORDS
            || limits.world.max_combined_memory_bytes > run::inventory::MAX_COMBINED_MEMORY_BYTES
        {
            return Err(StageError::InvalidLimits.into());
        }
        DirBin::parse(&[], limits.records).map_err(StageError::Placement)?;
        let combined = limits
            .world
            .max_combined_memory_bytes
            .checked_sub(limits.max_heap_bytes)
            .ok_or(StageError::Limit)?;
        let inventories = add(wdts.limits().max_heap_bytes, caches.limits().max_heap_bytes)?;
        let available = combined.checked_sub(inventories).ok_or(StageError::Limit)?;
        let collector = startup.collector_mut();
        collector.constrain_memory_for_stage(available)?;
        Ok(combined)
    })();
    let combined = match admission {
        Ok(v) => v,
        Err(mut e) => {
            e.guards = input_guards(&mut startup, &mut wdts, &mut caches, &mut repository);
            return Err(e);
        }
    };
    let mut stage = match Stage::new(repository, limits) {
        Ok(s) => s,
        Err(cause) => {
            let guards = Guards {
                collector: startup.collector_mut().verify_unchanged().err(),
                wdt: wdts.verify_unchanged().err(),
                cache: caches.verify_unchanged().err(),
                stage: None,
            };
            return Err(Error {
                cause: Cause::Stage(cause),
                guards,
            });
        }
    };
    let world = world::inventory::extract(
        startup,
        wdts,
        caches,
        Capture(&mut stage),
        0,
        world::inventory::Limits {
            max_combined_memory_bytes: combined,
            ..limits.world
        },
    );
    let mut world = match world {
        Ok(w) => w,
        Err(e) => {
            let mut error = Error::from(e);
            error.guards.stage = stage.check().err();
            return Err(error);
        }
    };
    let validated = stage.validate_topology(world.topology());
    let guards = Guards {
        collector: world.extraction_mut().verify_unchanged().err(),
        stage: stage.check().err(),
        ..Guards::default()
    };
    if let Err(e) = validated {
        return Err(Error {
            cause: Cause::Stage(e),
            guards,
        });
    }
    if let Some(cause) = guards
        .collector
        .map(Cause::Collector)
        .or_else(|| guards.stage.map(Cause::Stage))
    {
        return Err(Error { cause, guards });
    }
    Ok(Staged { world, stage })
}
