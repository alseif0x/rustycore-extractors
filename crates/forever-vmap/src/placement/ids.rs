// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: bounded serial GenerateUniqueObjectId contract.
//! Pin vmapexport.cpp:177-186: the key is (clientId, u16 doodadId), without
//! map or class. M2 consumes the descending counter before even a cache hit;
//! WMO proposes clientId. The first insertion for a key wins across classes.
//! This reproduces only a supplied serial call order, not thread scheduling.
//! Stronger admission rejects counter wrap and duplicate assigned numeric IDs
//! for distinct keys. Every error poisons the allocator, with no further work.
#[cfg(test)]
mod tests;
pub const MAX_ENTRIES: usize = 1_000_000;
pub const MAX_MEMORY_BYTES: usize = 128 * 1024 * 1024;
pub const MAX_PROBES: usize = 128;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub max_entries: usize,
    pub max_memory_bytes: usize,
    pub max_probes: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_entries: MAX_ENTRIES,
            max_memory_bytes: MAX_MEMORY_BYTES,
            max_probes: MAX_PROBES,
        }
    }
}
impl Limits {
    fn validate(self) -> Result<(), Error> {
        if self.max_entries > MAX_ENTRIES
            || self.max_memory_bytes > MAX_MEMORY_BYTES
            || self.max_probes > MAX_PROBES
        {
            Err(Error::InvalidLimits)
        } else {
            Ok(())
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidLimits,
    Limit,
    Allocation,
    CounterExhausted,
    AssignedIdCollision,
    ProbeLimit,
}
impl Error {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidLimits => "placement_ids_invalid_limits",
            Self::Limit => "placement_ids_policy_limit",
            Self::Allocation => "placement_ids_allocation",
            Self::CounterExhausted => "placement_ids_counter_exhausted",
            Self::AssignedIdCollision => "placement_ids_collision",
            Self::ProbeLimit => "placement_ids_probe_limit",
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}
impl std::error::Error for Error {}
#[derive(Clone, Copy)]
struct Entry {
    client: u32,
    doodad: u16,
    assigned: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stats {
    pub entries: usize,
    pub calls: u64,
    pub m2_counter_consumed: u64,
    pub next_m2_id: Option<u32>,
    pub resident_bytes: usize,
    pub peak_bytes: usize,
}
pub struct IdAllocator {
    keys: Vec<Option<Entry>>,
    assigned: Vec<Option<u32>>,
    limits: Limits,
    stats: Stats,
    failed: Option<Error>,
}
impl IdAllocator {
    pub fn new(limits: Limits) -> Result<Self, Error> {
        limits.validate()?;
        Ok(Self {
            keys: Vec::new(),
            assigned: Vec::new(),
            limits,
            stats: Stats {
                entries: 0,
                calls: 0,
                m2_counter_consumed: 0,
                next_m2_id: Some(u32::MAX - 1),
                resident_bytes: 0,
                peak_bytes: 0,
            },
            failed: None,
        })
    }
    pub const fn stats(&self) -> Stats {
        self.stats
    }
    pub fn assign(&mut self, client_id: u32, doodad_id: u16, is_wmo: bool) -> Result<u32, Error> {
        if let Some(error) = self.failed {
            return Err(error);
        }
        let result = self.assign_inner(client_id, doodad_id, is_wmo);
        if let Err(error) = result {
            self.failed = Some(error);
        }
        result
    }
    fn assign_inner(&mut self, client: u32, doodad: u16, is_wmo: bool) -> Result<u32, Error> {
        self.stats.calls = self.stats.calls.checked_add(1).ok_or(Error::Limit)?;
        let proposed = if is_wmo {
            client
        } else {
            let id = self.stats.next_m2_id.ok_or(Error::CounterExhausted)?;
            self.stats.next_m2_id = id.checked_sub(1);
            self.stats.m2_counter_consumed += 1;
            id
        };
        if let Some(entry) = find_key(&self.keys, client, doodad, self.limits.max_probes)? {
            return Ok(entry.assigned);
        }
        if self.stats.entries >= self.limits.max_entries {
            return Err(Error::Limit);
        }
        if contains_id(&self.assigned, proposed, self.limits.max_probes)? {
            return Err(Error::AssignedIdCollision);
        }
        if self.keys.is_empty() || (self.stats.entries + 1) > self.keys.len() / 2 {
            self.grow()?;
        }
        let key_at = key_slot(&self.keys, client, doodad, self.limits.max_probes)?;
        let id_at = id_slot(&self.assigned, proposed, self.limits.max_probes)?;
        self.keys[key_at] = Some(Entry {
            client,
            doodad,
            assigned: proposed,
        });
        self.assigned[id_at] = Some(proposed);
        self.stats.entries += 1;
        Ok(proposed)
    }
    fn grow(&mut self) -> Result<(), Error> {
        let slots = if self.keys.is_empty() {
            2
        } else {
            self.keys.len().checked_mul(2).ok_or(Error::Limit)?
        };
        let bytes = slots
            .checked_mul(size_of::<Option<Entry>>() + size_of::<Option<u32>>())
            .ok_or(Error::Limit)?;
        if self
            .stats
            .resident_bytes
            .checked_add(bytes)
            .ok_or(Error::Limit)?
            > self.limits.max_memory_bytes
        {
            return Err(Error::Limit);
        }
        let mut keys = Vec::new();
        let mut assigned = Vec::new();
        keys.try_reserve_exact(slots)
            .map_err(|_| Error::Allocation)?;
        let actual_keys = keys
            .capacity()
            .checked_mul(size_of::<Option<Entry>>())
            .ok_or(Error::Limit)?;
        if self
            .stats
            .resident_bytes
            .checked_add(actual_keys)
            .and_then(|v| v.checked_add(slots * size_of::<Option<u32>>()))
            .ok_or(Error::Limit)?
            > self.limits.max_memory_bytes
        {
            return Err(Error::Limit);
        }
        assigned
            .try_reserve_exact(slots)
            .map_err(|_| Error::Allocation)?;
        let actual = actual_keys
            .checked_add(
                assigned
                    .capacity()
                    .checked_mul(size_of::<Option<u32>>())
                    .ok_or(Error::Limit)?,
            )
            .ok_or(Error::Limit)?;
        let peak = self
            .stats
            .resident_bytes
            .checked_add(actual)
            .ok_or(Error::Limit)?;
        if peak > self.limits.max_memory_bytes {
            return Err(Error::Limit);
        }
        self.stats.peak_bytes = self.stats.peak_bytes.max(peak);
        keys.resize(slots, None);
        assigned.resize(slots, None);
        for entry in self.keys.iter().flatten().copied() {
            let at = key_slot(&keys, entry.client, entry.doodad, self.limits.max_probes)?;
            keys[at] = Some(entry);
            let at = id_slot(&assigned, entry.assigned, self.limits.max_probes)?;
            assigned[at] = Some(entry.assigned);
        }
        self.keys = keys;
        self.assigned = assigned;
        self.stats.resident_bytes = actual;
        Ok(())
    }
}
fn hash(mut value: u64) -> usize {
    value ^= value >> 30;
    value = value.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94d0_49bb_1331_11eb);
    (value ^ (value >> 31)) as usize
}
fn key_hash(client: u32, doodad: u16) -> usize {
    hash((u64::from(client) << 16) | u64::from(doodad))
}
fn key_slot(
    keys: &[Option<Entry>],
    client: u32,
    doodad: u16,
    probes: usize,
) -> Result<usize, Error> {
    for step in 0..keys.len().min(probes) {
        let at = key_hash(client, doodad).wrapping_add(step) & (keys.len() - 1);
        if keys[at].is_none_or(|e| e.client == client && e.doodad == doodad) {
            return Ok(at);
        }
    }
    Err(Error::ProbeLimit)
}
fn find_key(
    keys: &[Option<Entry>],
    client: u32,
    doodad: u16,
    probes: usize,
) -> Result<Option<Entry>, Error> {
    if keys.is_empty() {
        return Ok(None);
    }
    Ok(keys[key_slot(keys, client, doodad, probes)?])
}
fn id_slot(ids: &[Option<u32>], id: u32, probes: usize) -> Result<usize, Error> {
    for step in 0..ids.len().min(probes) {
        let at = hash(u64::from(id)).wrapping_add(step) & (ids.len() - 1);
        if ids[at].is_none_or(|v| v == id) {
            return Ok(at);
        }
    }
    Err(Error::ProbeLimit)
}
fn contains_id(ids: &[Option<u32>], id: u32, probes: usize) -> Result<bool, Error> {
    if ids.is_empty() {
        return Ok(false);
    }
    Ok(ids[id_slot(ids, id, probes)?].is_some())
}
