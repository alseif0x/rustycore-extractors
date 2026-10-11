// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original bounded ROOT name materialization.
// Source facts: TrinityCore 02245dcd245e7433e524577656177723d3e4992e,
// dep/CascLib/src/CascRootFile_WoW.cpp:334-405,432-546;
// common/FileTree.cpp:93-125,201-233,370-419; common/Common.cpp:418-460.
//! Optional conservative name resolution, independent of FDID resolution.
//!
//! Eligible ROOT rows are visited in audio-pass then physical order. A node's
//! first centrally present CKey is retained even when its first nonzero name
//! hash arrives in a later row. Central presence must never mean local archive
//! availability. The callback is called exactly once for each eligible row.
//!
//! The native tree mutates node hashes and may rebuild its pointer-based map.
//! This profile rejects different nonzero hashes for one FDID and hashes shared
//! by different FDIDs globally. It therefore does not claim general name parity.
//! Zero/absent hashes never clear an earlier hash. FDID zero remains valid.
//! Nothing here authenticates bytes, reads files, loads listfiles or falls back
//! to textual FDIDs, CKeys, EKeys, INSTALL, another locale or another backend.
//!
//! All vectors use checked, fallible exact reservations. The memory ceiling
//! covers their simultaneous capacities, including construction scratch. It
//! excludes borrowed inputs, fixed stack values and allocator bookkeeping.
//! The returned index owns no names and has no references into the input ROOT.

use super::{Block, Entry};
use crate::{Locale, local_index::jenkins};
use std::mem::size_of;

pub const MAX_NAME_BYTES: usize = 1024;
const MAX_BLOCKS: usize = 65_536;
const MAX_ENTRIES: usize = 4_000_000;
const MAX_MEMORY_BYTES: usize = 256 * 1024 * 1024;
const RESERVED_NAMES: [&[u8]; 6] = [
    b"ENCODING",
    b"DOWNLOAD",
    b"INSTALL",
    b"PATCH",
    b"ROOT",
    b"SIZE",
];

/// All limits may only reduce these defaults; zero is a meaningful reduction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub max_blocks: usize,
    pub max_entries: usize,
    pub max_nodes: usize,
    pub max_names: usize,
    pub max_memory_bytes: usize,
    pub max_name_bytes: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_blocks: MAX_BLOCKS,
            max_entries: MAX_ENTRIES,
            max_nodes: MAX_ENTRIES,
            max_names: MAX_ENTRIES,
            max_memory_bytes: MAX_MEMORY_BYTES,
            max_name_bytes: MAX_NAME_BYTES,
        }
    }
}

impl Limits {
    /// Pure validation, usable before an integration performs any input I/O.
    pub fn validate(self) -> Result<(), Error> {
        if self.max_blocks > MAX_BLOCKS
            || self.max_entries > MAX_ENTRIES
            || self.max_nodes > MAX_ENTRIES
            || self.max_names > MAX_ENTRIES
            || self.max_memory_bytes > MAX_MEMORY_BYTES
            || self.max_name_bytes > MAX_NAME_BYTES
        {
            return Err(Error::InvalidLimits);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidLimits,
    Limit,
    Structure,
    FileDataId,
    InvalidName,
    ReservedName,
    NameConflict,
    Allocation,
}

impl Error {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidLimits => "root_name_invalid_limits",
            Self::Limit => "root_name_policy_limit",
            Self::Structure => "root_name_invalid_structure",
            Self::FileDataId => "root_name_invalid_file_data_id",
            Self::InvalidName => "root_name_invalid_name",
            Self::ReservedName => "root_name_reserved_hash",
            Self::NameConflict => "root_name_conflict",
            Self::Allocation => "root_name_allocation",
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        out.write_str(self.code())
    }
}

impl std::error::Error for Error {}

/// The first admitted ROOT row of the node, not the later row carrying its name.
/// No Debug implementation: CKeys must not leak into ordinary diagnostics.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Route {
    pub first_entry_ordinal: usize,
    pub file_data_id: u32,
    pub content_key: [u8; 16],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stats {
    pub observed_entries: usize,
    pub eligible_entries: usize,
    pub central_entries: usize,
    pub nodes: usize,
    /// Nodes with a nonzero hash, including reserved hashes that cannot resolve.
    pub names: usize,
    /// Retained vector capacity in bytes, excluding fixed structs and inputs.
    pub resident_bytes: usize,
    /// Highest simultaneous scratch plus retained vector capacity during build.
    pub peak_bytes: usize,
}

struct NamedRoute {
    hash: u64,
    route: Route,
}

/// Fully constructed immutable index. Failed builds never expose partial state.
pub struct NameIndex {
    names: Vec<NamedRoute>,
    max_name_bytes: usize,
    stats: Stats,
}

impl NameIndex {
    pub fn build(
        blocks: &[Block],
        entries: &[Entry],
        locale: Locale,
        mut present: impl FnMut([u8; 16]) -> bool,
        limits: Limits,
    ) -> Result<Self, Error> {
        limits.validate()?;
        validate_structure(blocks, entries, limits)?;
        let eligible_entries = blocks
            .iter()
            .filter(|block| eligible(block, locale))
            .map(|block| block.entries.len())
            .sum();
        let mut budget = Budget::new(limits.max_memory_bytes);
        let mut order = budget.vector::<u32>(eligible_entries)?;
        for audio in [0, 1] {
            for block in blocks {
                if !eligible(block, locale) || block.content_flags.effective() >> 31 != audio {
                    continue;
                }
                for ordinal in block.entries.clone() {
                    if present(entries[ordinal].content_key) {
                        // Structure/count validation bounds all ordinals below 4M.
                        order.push(ordinal as u32);
                    }
                }
            }
        }
        // The explicit secondary keys reproduce processing order after grouping
        // by FDID, without stable-sort allocation or an unbudgeted hash map.
        order.sort_unstable_by_key(|&ordinal| {
            let entry = &entries[ordinal as usize];
            (
                entry.file_data_id,
                blocks[entry.block_index].content_flags.effective() >> 31,
                ordinal,
            )
        });

        let mut nodes = 0;
        let mut names = 0;
        let mut start = 0;
        while start < order.len() {
            let (end, hash) = node(&order, entries, start)?;
            nodes += 1;
            names += usize::from(hash != 0);
            if nodes > limits.max_nodes || names > limits.max_names {
                return Err(Error::Limit);
            }
            start = end;
        }

        let mut named = budget.vector::<NamedRoute>(names)?;
        start = 0;
        while start < order.len() {
            let (end, hash) = node(&order, entries, start)?;
            if hash != 0 {
                let first_entry_ordinal = order[start] as usize;
                let first = &entries[first_entry_ordinal];
                named.push(NamedRoute {
                    hash,
                    route: Route {
                        first_entry_ordinal,
                        file_data_id: first.file_data_id,
                        content_key: first.content_key,
                    },
                });
            }
            start = end;
        }
        named.sort_unstable_by_key(|value| value.hash);
        if named.windows(2).any(|pair| pair[0].hash == pair[1].hash) {
            return Err(Error::NameConflict);
        }
        let stats = Stats {
            observed_entries: entries.len(),
            eligible_entries,
            central_entries: order.len(),
            nodes,
            names,
            resident_bytes: allocation_bytes::<NamedRoute>(named.capacity())?,
            peak_bytes: budget.peak,
        };
        Ok(Self {
            names: named,
            max_name_bytes: limits.max_name_bytes,
            stats,
        })
    }

    /// Validate and hash the complete logical name. No acquisition or fallback.
    /// Reserved hashes reject even if ROOT contains a corresponding node.
    pub fn resolve(&self, name: &[u8]) -> Result<Option<Route>, Error> {
        let hash = hash_name(name, self.max_name_bytes)?;
        Ok(self
            .names
            .binary_search_by_key(&hash, |value| value.hash)
            .ok()
            .map(|index| self.names[index].route))
    }

    pub const fn stats(&self) -> Stats {
        self.stats
    }

    pub const fn resident_bytes(&self) -> usize {
        self.stats.resident_bytes
    }

    pub const fn peak_bytes(&self) -> usize {
        self.stats.peak_bytes
    }
}

/// Linux pinned name-hash profile: complete ASCII bytes, at most 1024, no NUL.
/// Names are logical references, not filesystem paths. No cleanup is performed.
/// Rejecting high-bit bytes avoids the source's signed-char table indexing;
/// rejecting overlong names avoids the source's MAX_PATH prefix truncation.
pub fn hash_name(name: &[u8], max_bytes: usize) -> Result<u64, Error> {
    if max_bytes > MAX_NAME_BYTES {
        return Err(Error::InvalidLimits);
    }
    if name.is_empty() {
        return Err(Error::InvalidName);
    }
    if name.len() > max_bytes {
        return Err(Error::Limit);
    }
    let mut normalized = [0; MAX_NAME_BYTES];
    for (destination, &byte) in normalized.iter_mut().zip(name) {
        if byte == 0 || !byte.is_ascii() {
            return Err(Error::InvalidName);
        }
        *destination = if byte == b'/' {
            b'\\'
        } else {
            byte.to_ascii_uppercase()
        };
    }
    let hash = hash_normalized(&normalized[..name.len()]);
    if RESERVED_NAMES
        .iter()
        .any(|value| hash_normalized(value) == hash)
    {
        return Err(Error::ReservedName);
    }
    Ok(hash)
}

fn hash_normalized(name: &[u8]) -> u64 {
    let (pc, pb) = jenkins::hashlittle2(name, 0, 0);
    (u64::from(pc) << 32) | u64::from(pb)
}

fn eligible(block: &Block, locale: Locale) -> bool {
    let mask = match locale {
        Locale::EnUs => 2,
        Locale::EsEs => 0x80,
    };
    block.content_flags.effective() & (0x100 | 0x80) == 0
        && (block.locale_flags == 0 || block.locale_flags & mask != 0)
}

fn validate_structure(blocks: &[Block], entries: &[Entry], limits: Limits) -> Result<(), Error> {
    if blocks.len() > limits.max_blocks || entries.len() > limits.max_entries {
        return Err(Error::Limit);
    }
    let mut next = 0;
    for (block_index, block) in blocks.iter().enumerate() {
        if block.entries.start != next
            || block.entries.end < next
            || block.entries.end > entries.len()
        {
            return Err(Error::Structure);
        }
        let mut previous = None;
        let mut has_hashes = None;
        for entry in &entries[block.entries.clone()] {
            if entry.block_index != block_index {
                return Err(Error::Structure);
            }
            if entry.file_data_id == u32::MAX {
                return Err(Error::FileDataId);
            }
            if previous.is_some_and(|id| id >= entry.file_data_id)
                || has_hashes.is_some_and(|has| has != entry.file_name_hash.is_some())
            {
                return Err(Error::Structure);
            }
            previous = Some(entry.file_data_id);
            has_hashes = Some(entry.file_name_hash.is_some());
        }
        next = block.entries.end;
    }
    if next != entries.len() {
        return Err(Error::Structure);
    }
    Ok(())
}

/// Return this FDID's group end and unique nonzero hash. Zero never erases.
fn node(order: &[u32], entries: &[Entry], start: usize) -> Result<(usize, u64), Error> {
    let id = entries[order[start] as usize].file_data_id;
    let end = start
        + order[start..].partition_point(|&ordinal| entries[ordinal as usize].file_data_id == id);
    let mut hash = 0;
    for &ordinal in &order[start..end] {
        let next_hash = entries[ordinal as usize].file_name_hash.unwrap_or(0);
        if next_hash != 0 {
            if hash != 0 && hash != next_hash {
                return Err(Error::NameConflict);
            }
            hash = next_hash;
        }
    }
    Ok((end, hash))
}

fn allocation_bytes<T>(count: usize) -> Result<usize, Error> {
    count.checked_mul(size_of::<T>()).ok_or(Error::Limit)
}

struct Budget {
    used: usize,
    peak: usize,
    limit: usize,
}

impl Budget {
    fn new(limit: usize) -> Self {
        Self {
            used: 0,
            peak: 0,
            limit,
        }
    }

    fn check(&self, bytes: usize) -> Result<usize, Error> {
        self.used
            .checked_add(bytes)
            .filter(|&sum| sum <= self.limit)
            .ok_or(Error::Limit)
    }

    fn vector<T>(&mut self, count: usize) -> Result<Vec<T>, Error> {
        self.check(allocation_bytes::<T>(count)?)?;
        let mut vector = Vec::new();
        vector
            .try_reserve_exact(count)
            .map_err(|_| Error::Allocation)?;
        self.used = self.check(allocation_bytes::<T>(vector.capacity())?)?;
        self.peak = self.peak.max(self.used);
        Ok(vector)
    }
}

#[cfg(test)]
mod tests;
