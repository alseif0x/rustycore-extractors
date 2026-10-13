// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original bounded placement-cache byte replay.
//! Pinned model.cpp/wmo.cpp cache writers clear MOD_PARENT_SPAWN (2) and
//! preserve each serialized record's remaining bytes. ADTFile::initFromCache,
//! adtfile.cpp:199-221, restores that bit only when map != original map.
//! This module implements that byte operation on complete validated dir_bin
//! streams. It does not allocate IDs, transform floats, select a tile cache,
//! acquire data, fall back to another tile, or perform I/O.
use super::{DirBin, Error};

#[cfg(test)]
mod tests;

pub const MAX_MEMORY_BYTES: usize = 128 * 1024 * 1024;

/// Only reductions are admitted. `records` bounds the complete input/output
/// stream, record count and names on every operation. The memory ceiling covers
/// actual owned vector capacities: new cache on construction, or existing cache
/// plus newly replayed output. Borrowed input, stack/allocator overhead and
/// outputs retained from earlier calls are excluded; this is not an RSS cap.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub records: super::Limits,
    pub max_memory_bytes: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            records: super::Limits::default(),
            max_memory_bytes: MAX_MEMORY_BYTES,
        }
    }
}
impl Limits {
    fn validate(self) -> Result<(), Error> {
        self.records.validate()?;
        if self.max_memory_bytes > MAX_MEMORY_BYTES {
            return Err(Error::InvalidLimits);
        }
        Ok(())
    }
    fn memory(self, old: usize, new: usize) -> Result<(), Error> {
        if old.checked_add(new).ok_or(Error::Limit)? > self.max_memory_bytes {
            return Err(Error::Limit);
        }
        Ok(())
    }
}

/// Immutable complete record bytes with the parent flag cleared. IDs, names,
/// record order, repeated records, signed zeros and every other bit are retained.
/// No Debug implementation exposes source-derived values.
pub struct Cached {
    bytes: Vec<u8>,
    records: usize,
}
impl Cached {
    pub fn from_bytes(bytes: &[u8], limits: Limits) -> Result<Self, Error> {
        limits.validate()?;
        let parsed = DirBin::parse(bytes, limits.records)?;
        let output = copy_with_parent(&parsed, false, 0, limits)?;
        Ok(Self {
            bytes: output,
            records: parsed.record_count(),
        })
    }
    /// Revalidates under the supplied reduced limits; no input view is retained.
    pub fn from_dir_bin(view: &DirBin<'_>, limits: Limits) -> Result<Self, Error> {
        Self::from_bytes(view.bytes(), limits)
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub const fn record_count(&self) -> usize {
        self.records
    }
    pub fn resident_bytes(&self) -> usize {
        self.bytes.capacity()
    }
    /// Replay is independent of previous replays and never changes the cache.
    /// Errors return no output. Empty caches succeed with zero record/byte/memory
    /// caps; a nonempty cache still has to fit the reduced full-stream limits.
    pub fn replay(
        &self,
        map_id: u32,
        original_map_id: u32,
        limits: Limits,
    ) -> Result<Vec<u8>, Error> {
        limits.validate()?;
        let parsed = DirBin::parse(&self.bytes, limits.records)?;
        copy_with_parent(
            &parsed,
            map_id != original_map_id,
            self.resident_bytes(),
            limits,
        )
    }
}

fn copy_with_parent(
    parsed: &DirBin<'_>,
    parent: bool,
    old_capacity: usize,
    limits: Limits,
) -> Result<Vec<u8>, Error> {
    let input = parsed.bytes();
    limits.memory(old_capacity, input.len())?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(input.len())
        .map_err(|_| Error::Allocation)?;
    if bytes.capacity() > limits.records.max_bytes {
        return Err(Error::Limit);
    }
    limits.memory(old_capacity, bytes.capacity())?;
    bytes.extend_from_slice(input);
    let parent_flag = if parent { 2 } else { 0 };
    let mut at = 0usize;
    for record in parsed.records() {
        bytes[at] = (record.flags() & !2) | parent_flag;
        at = at.checked_add(record.encoded_len()?).ok_or(Error::Limit)?;
    }
    Ok(bytes)
}
