// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-12: original typed private WDT/own-tile cache repository.
//! Complete supplied inputs only. Map/FDID and own-tile provenance are the
//! caller's responsibility; immutable envelopes bind subsequent reads to that
//! declared association. No acquisition, fallback, publication or resume occurs.
//! Every error is terminal. Borrowed input and returned callback/output storage
//! belong to the caller. Logical reservations are conservative, not RSS bounds.
use super::{payload, plan, spool};
use forever_vmap::placement::{self, DirBin, cache};

#[cfg(test)]
mod tests;
pub const PROFILE: &str = "linux-private-wdt-cache-inventory-v1";
pub const MAX_ENTRIES: usize = 4096;
pub const MAX_HEAP_BYTES: usize = 512 * 1024 * 1024;
pub const MAX_ACTIVE_BYTES: usize = 256 * 1024 * 1024;
const GUARD_ALLOWANCE: usize = 512 * 1024;
const MAGIC: &[u8; 8] = b"RCEINV1\0";
const HEADER: usize = 36;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub max_entries: usize,
    pub max_heap_bytes: usize,
    pub max_active_bytes: usize,
    pub wdt: plan::Limits,
    pub cache: cache::Limits,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_entries: MAX_ENTRIES,
            max_heap_bytes: MAX_HEAP_BYTES,
            max_active_bytes: MAX_ACTIVE_BYTES,
            wdt: plan::Limits::default(),
            cache: cache::Limits::default(),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidLimits,
    Limit,
    Allocation,
    Duplicate,
    Missing,
    Binding,
    Format,
    State,
    Callback,
    Spool(payload::Error),
    Wdt(plan::Error),
    Cache(placement::Error),
}
impl Error {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidLimits => "collision_inventory_invalid_limits",
            Self::Limit => "collision_inventory_limit",
            Self::Allocation => "collision_inventory_allocation",
            Self::Duplicate => "collision_inventory_duplicate",
            Self::Missing => "collision_inventory_missing",
            Self::Binding => "collision_inventory_binding",
            Self::Format => "collision_inventory_format",
            Self::State => "collision_inventory_state",
            Self::Callback => "collision_inventory_callback",
            Self::Spool(e) => e.code(),
            Self::Wdt(e) => e.code(),
            Self::Cache(e) => e.code(),
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}
impl std::error::Error for Error {}
impl From<payload::Error> for Error {
    fn from(e: payload::Error) -> Self {
        Self::Spool(e)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cause<E> {
    Access(Error),
    Callback(E),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CallbackError<E> {
    pub cause: Cause<E>,
    pub guard: Option<Error>,
}
impl<E> CallbackError<E> {
    pub fn code(&self) -> &'static str {
        if self.guard == Some(Error::Spool(payload::Error::Changed)) {
            return payload::Error::Changed.code();
        }
        match self.cause {
            Cause::Access(e) => e.code(),
            Cause::Callback(_) => Error::Callback.code(),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Tile {
    pub map_id: u32,
    pub x: u8,
    pub y: u8,
}
impl Tile {
    fn cell(self) -> Result<u32, Error> {
        if self.x >= 64 || self.y >= 64 {
            return Err(Error::Binding);
        }
        Ok(u32::from(self.x) * 64 + u32::from(self.y))
    }
}
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Key {
    Wdt(u32),
    Tile(Tile),
}
#[derive(Clone, Copy)]
struct Entry {
    key: Key,
    source: u32,
    records: usize,
    body_bytes: usize,
    record: spool::Record,
}
impl Entry {
    fn header(self) -> [u8; HEADER] {
        Self::header_for(self.key, self.source, self.records, self.body_bytes)
    }
    fn header_for(key: Key, source: u32, records: usize, body_bytes: usize) -> [u8; HEADER] {
        let (kind, map) = match key {
            Key::Wdt(id) => (1, id),
            Key::Tile(t) => (2, t.map_id),
        };
        let mut h = [0; HEADER];
        h[..8].copy_from_slice(MAGIC);
        h[8] = kind;
        h[12..16].copy_from_slice(&map.to_le_bytes());
        h[16..20].copy_from_slice(&source.to_le_bytes());
        h[20..28].copy_from_slice(&(records as u64).to_le_bytes());
        h[28..36].copy_from_slice(&(body_bytes as u64).to_le_bytes());
        h
    }
    fn body(self, bytes: &[u8]) -> Result<&[u8], Error> {
        if bytes.len() != add(HEADER, self.body_bytes)? || bytes[..HEADER] != self.header() {
            return Err(Error::Binding);
        }
        Ok(&bytes[HEADER..])
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stats {
    pub entries: usize,
    /// Includes twice the repository's descriptor ceiling and fixed guard work
    /// storage; this deliberately exceeds actual retained capacities.
    pub reserved_heap_bytes: usize,
    pub peak_admitted_heap_bytes: usize,
    pub peak_admitted_active_bytes: usize,
    pub repository: spool::Stats,
}
/// One owner, no public ordinal handles and no repository extraction method.
/// Sorted compact keys remain resident; complete bodies live only in the spool.
pub struct Inventory {
    repository: spool::Spool,
    entries: Vec<Entry>,
    limits: Limits,
    base_reservation: usize,
    heap_peak: usize,
    active_peak: usize,
    failed: Option<Error>,
}
fn add(a: usize, b: usize) -> Result<usize, Error> {
    a.checked_add(b).ok_or(Error::Limit)
}
fn mul(a: usize, b: usize) -> Result<usize, Error> {
    a.checked_mul(b).ok_or(Error::Limit)
}
impl Inventory {
    pub fn new(mut repository: spool::Spool, limits: Limits) -> Result<Self, Error> {
        if limits.max_entries > MAX_ENTRIES
            || limits.max_heap_bytes > MAX_HEAP_BYTES
            || limits.max_active_bytes > MAX_ACTIVE_BYTES
        {
            return Err(Error::InvalidLimits);
        }
        limits.wdt.validate().map_err(Error::Wdt)?;
        cache::Cached::from_bytes(&[], limits.cache).map_err(Error::Cache)?;
        repository.verify_unchanged()?;
        if repository.stats()?.records != 0 {
            return Err(Error::State);
        }
        // Two descriptor ceilings cover old/new repository allocation overlap.
        let base = add(
            mul(repository.limits().max_descriptor_bytes, 2)?,
            GUARD_ALLOWANCE,
        )?;
        if base > limits.max_heap_bytes {
            return Err(Error::Limit);
        }
        Ok(Self {
            repository,
            entries: Vec::new(),
            limits,
            base_reservation: base,
            heap_peak: base,
            active_peak: 0,
            failed: None,
        })
    }
    fn heap(&self) -> Result<usize, Error> {
        add(
            self.base_reservation,
            mul(self.entries.capacity(), size_of::<Entry>())?,
        )
    }
    fn admit(&mut self, active: usize) -> Result<(), Error> {
        let heap = add(self.heap()?, active)?;
        if active > self.limits.max_active_bytes || heap > self.limits.max_heap_bytes {
            return Err(Error::Limit);
        }
        self.heap_peak = self.heap_peak.max(heap);
        self.active_peak = self.active_peak.max(active);
        Ok(())
    }
    fn check(&mut self) -> Result<(), Error> {
        let guard = self.repository.verify_unchanged().map_err(Error::Spool);
        if guard == Err(Error::Spool(payload::Error::Changed))
            || self.failed == Some(Error::Spool(payload::Error::Changed))
        {
            self.failed = Some(Error::Spool(payload::Error::Changed));
        }
        guard.and(self.failed.map_or(Ok(()), Err))
    }
    fn finish<T>(&mut self, result: Result<T, Error>) -> Result<T, Error> {
        let checked = self.check().and(result);
        if let Err(e) = checked {
            self.failed.get_or_insert(e);
        }
        checked
    }
    pub fn verify_unchanged(&mut self) -> Result<(), Error> {
        self.finish(Ok(()))
    }
    pub const fn failed(&self) -> bool {
        self.failed.is_some()
    }
    pub fn stats(&self) -> Result<Stats, Error> {
        Ok(Stats {
            entries: self.entries.len(),
            reserved_heap_bytes: self.heap()?,
            peak_admitted_heap_bytes: self.heap_peak,
            peak_admitted_active_bytes: self.active_peak,
            repository: self.repository.stats()?,
        })
    }
    fn locate(&self, key: Key) -> Result<Entry, Error> {
        let at = self
            .entries
            .binary_search_by_key(&key, |e| e.key)
            .map_err(|_| Error::Missing)?;
        Ok(self.entries[at])
    }
    fn prepare(&mut self, key: Key, active: usize) -> Result<usize, Error> {
        self.check()?;
        let at = self
            .entries
            .binary_search_by_key(&key, |e| e.key)
            .err()
            .ok_or(Error::Duplicate)?;
        if self.entries.len() >= self.limits.max_entries {
            return Err(Error::Limit);
        }
        self.admit(active)?;
        if self.entries.len() == self.entries.capacity() {
            let new_bytes = mul(add(self.entries.len(), 1)?, size_of::<Entry>())?;
            let before = self.heap()?;
            if add(add(before, active)?, new_bytes)? > self.limits.max_heap_bytes {
                return Err(Error::Limit);
            }
            self.entries
                .try_reserve_exact(1)
                .map_err(|_| Error::Allocation)?;
            let actual = mul(self.entries.capacity(), size_of::<Entry>())?;
            let peak = add(add(before, active)?, actual)?;
            if peak > self.limits.max_heap_bytes {
                return Err(Error::Limit);
            }
            self.heap_peak = self.heap_peak.max(peak);
            self.admit(active)?;
        }
        Ok(at)
    }
    fn write(
        &mut self,
        key: Key,
        source: u32,
        count: usize,
        body: &[u8],
        at: usize,
    ) -> Result<(), Error> {
        let size = add(HEADER, body.len())?;
        if size > self.repository.limits().max_active_bytes {
            return Err(Error::Limit);
        }
        let header = Entry::header_for(key, source, count, body.len());
        let record = self.repository.write(size, |out| {
            out.bytes(&header)?;
            out.bytes(body)
        })?;
        self.entries.insert(
            at,
            Entry {
                key,
                source,
                records: count,
                body_bytes: body.len(),
                record,
            },
        );
        Ok(())
    }
    pub fn insert_wdt(
        &mut self,
        map_id: u32,
        file_data_id: u32,
        bytes: &[u8],
    ) -> Result<(), Error> {
        let result = (|| {
            if file_data_id == 0 {
                return Err(Error::Binding);
            }
            let active = add(add(HEADER, bytes.len())?, self.limits.wdt.max_memory_bytes)?;
            let at = self.prepare(Key::Wdt(map_id), active)?;
            let count = plan::Wdt::parse(bytes, self.limits.wdt)
                .map_err(Error::Wdt)?
                .stats()
                .global_records;
            self.write(Key::Wdt(map_id), file_data_id, count, bytes, at)
        })();
        self.finish(result)
    }
    /// Caller declares a successful OWN tile. Empty records are stored as a
    /// present cache; this primitive neither discovers nor inherits tiles.
    pub fn insert_own_tile(&mut self, tile: Tile, bytes: &[u8]) -> Result<(), Error> {
        let result = (|| {
            let cell = tile.cell()?;
            let active = add(
                add(HEADER, bytes.len())?,
                self.limits.cache.max_memory_bytes,
            )?;
            let at = self.prepare(Key::Tile(tile), active)?;
            let canonical =
                cache::Cached::from_bytes(bytes, self.limits.cache).map_err(Error::Cache)?;
            self.admit(add(add(HEADER, bytes.len())?, canonical.resident_bytes())?)?;
            self.write(
                Key::Tile(tile),
                cell,
                canonical.record_count(),
                canonical.bytes(),
                at,
            )
        })();
        self.finish(result)
    }
    /// The callback's complete retained/scratch allowance is caller-enforced.
    /// It is reserved before reload; returned owned data leaves this component.
    pub fn with_wdt<T, E: Copy>(
        &mut self,
        map_id: u32,
        file_data_id: u32,
        callback_heap_allowance: usize,
        call: impl FnOnce(&plan::Wdt<'_>) -> Result<T, E>,
    ) -> Result<T, CallbackError<E>> {
        let setup = (|| {
            self.check()?;
            let entry = self.locate(Key::Wdt(map_id))?;
            if entry.source != file_data_id {
                return Err(Error::Binding);
            }
            let size = add(HEADER, entry.body_bytes)?;
            self.admit(add(
                add(size, self.limits.wdt.max_memory_bytes)?,
                callback_heap_allowance,
            )?)?;
            Ok((entry, size))
        })();
        let (entry, size) = match setup {
            Ok(v) => v,
            Err(e) => {
                drop(call);
                return Err(CallbackError {
                    cause: Cause::Access(e),
                    guard: self.finish::<()>(Err(e)).err().filter(|guard| *guard != e),
                });
            }
        };
        let limits = self.limits.wdt;
        let mut outcome = None;
        let io = self
            .repository
            .with_record(entry.record, size, |bytes| {
                outcome = Some((|| {
                    let body = entry.body(bytes).map_err(Cause::Access)?;
                    let wdt =
                        plan::Wdt::parse(body, limits).map_err(|e| Cause::Access(Error::Wdt(e)))?;
                    if wdt.stats().global_records != entry.records {
                        return Err(Cause::Access(Error::Format));
                    }
                    call(&wdt).map_err(Cause::Callback)
                })());
                Ok(())
            })
            .map_err(Error::Spool);
        match io {
            Err(e) => {
                let cause = match outcome {
                    Some(Err(cause)) => cause,
                    other => {
                        drop(other);
                        Cause::Access(e)
                    }
                };
                let guard = self.finish::<()>(Err(e)).err();
                Err(CallbackError { cause, guard })
            }
            Ok(()) => match outcome.ok_or(Cause::Access(Error::State)).and_then(|v| v) {
                Ok(value) => match self.check() {
                    Ok(()) => Ok(value),
                    Err(e) => {
                        drop(value);
                        Err(CallbackError {
                            cause: Cause::Access(e),
                            guard: self.finish::<()>(Err(e)).err(),
                        })
                    }
                },
                Err(cause) => {
                    let marker = match cause {
                        Cause::Access(e) => e,
                        Cause::Callback(_) => Error::Callback,
                    };
                    let guard = self
                        .finish::<()>(Err(marker))
                        .err()
                        .filter(|e| *e != marker);
                    Err(CallbackError { cause, guard })
                }
            },
        }
    }
    /// Genuine absence is distinct from a present zero-record own-tile cache.
    pub fn contains_own_tile(&mut self, tile: Tile) -> Result<bool, Error> {
        let result = (|| {
            self.check()?;
            tile.cell()?;
            Ok(self
                .entries
                .binary_search_by_key(&Key::Tile(tile), |e| e.key)
                .is_ok())
        })();
        self.finish(result)
    }
    /// Returned bytes become caller-owned only after all repository checks.
    pub fn replay_own_tile(
        &mut self,
        tile: Tile,
        destination_map_id: u32,
    ) -> Result<Vec<u8>, Error> {
        let result = (|| {
            self.check()?;
            tile.cell()?;
            let entry = self.locate(Key::Tile(tile))?;
            let size = add(HEADER, entry.body_bytes)?;
            let scratch = mul(entry.body_bytes, 2)?;
            self.admit(add(size, scratch)?)?;
            let mut limits = self.limits.cache;
            limits.max_memory_bytes = limits.max_memory_bytes.min(scratch);
            let mut output = None;
            let io = self.repository.with_record(entry.record, size, |bytes| {
                output = Some((|| {
                    let body = entry.body(bytes)?;
                    let view = DirBin::parse(body, limits.records).map_err(Error::Cache)?;
                    if view.record_count() != entry.records
                        || view.records().any(|r| r.flags() & 2 != 0)
                    {
                        return Err(Error::Format);
                    }
                    let cached =
                        cache::Cached::from_dir_bin(&view, limits).map_err(Error::Cache)?;
                    cached
                        .replay(destination_map_id, tile.map_id, limits)
                        .map_err(Error::Cache)
                })());
                Ok(())
            });
            io?;
            output.ok_or(Error::State)?
        })();
        self.finish(result)
    }
}
