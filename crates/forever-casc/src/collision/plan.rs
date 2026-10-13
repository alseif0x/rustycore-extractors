// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: bounded collision map graph and object-ADT request plan.
//! Source facts: pin 02245dcd245e7433e524577656177723d3e4992e,
//! vmapexport.cpp:323-390,421-469 and wdtfile.cpp:56-157/wdtfile.h:29-67.
//! This module plans requests; it performs no acquisition, cache replay, ID
//! assignment or placement conversion. WDT globals belong to their own map and
//! are not inherited. In a future executor, a successfully parsed own ADT with
//! no objects still blocks parent fallback. Acquisition errors must not be
//! reinterpreted here as missing tiles.
//!
//! All materialized rows participate in the graph before WDT-zero filtering.
//! Parent resolution uses only the direct prefix, matching the native index's
//! domain. Duplicate IDs, missing/copy-only parents and cycles are rejected
//! instead of reproducing unchecked indexing or the native operator[] bug.
//! The rows and direct-prefix count are caller-supplied provenance, not an
//! authenticated token: retain SourceRecords::direct_records().len() before
//! materialize. Likewise, the caller must bind each WDT to its map/FDID.
//! Preload order is physical; processing order is depth then physical ordinal.
//! This deterministic order does not assert native parallel scheduling parity.
use forever_db2::terrain_sources::{DirectRecord, VMapFields};
use forever_vmap::objects::{self, Objects};
use std::mem::size_of;

#[cfg(test)]
mod tests;

pub const MAX_ROWS: usize = 200_000;
pub const MAX_MAPS: usize = 4096;
pub const MAX_DEPTH: usize = 1024;
pub const MAX_NAME_BYTES: usize = 1024;
pub const MAX_MEMORY_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_WDT_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_WDT_CHUNKS: usize = 4096;
pub const MAX_DOMAIN_MAPS: usize = 1024;
pub const MAX_DOMAIN_BYTES: usize = 64 * 1024;
/// Conservative retained selection allowance per member, in addition to the
/// already charged graph/order capacities. This is logical admission, not RSS.
pub const DOMAIN_MEMBER_BYTES: usize = 64;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MapSelection {
    #[default]
    All,
    MapAndAncestors(u32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub max_rows: usize,
    pub max_maps: usize,
    pub max_depth: usize,
    pub max_name_bytes: usize,
    pub max_memory_bytes: usize,
    pub max_wdt_bytes: usize,
    pub max_wdt_chunks: usize,
    pub max_domain_maps: usize,
    pub max_domain_bytes: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_rows: MAX_ROWS,
            max_maps: MAX_MAPS,
            max_depth: MAX_DEPTH,
            max_name_bytes: MAX_NAME_BYTES,
            max_memory_bytes: MAX_MEMORY_BYTES,
            max_wdt_bytes: MAX_WDT_BYTES,
            max_wdt_chunks: MAX_WDT_CHUNKS,
            max_domain_maps: MAX_DOMAIN_MAPS,
            max_domain_bytes: MAX_DOMAIN_BYTES,
        }
    }
}
impl Limits {
    pub fn validate(self) -> Result<(), Error> {
        if self.max_rows > MAX_ROWS
            || self.max_maps > MAX_MAPS
            || self.max_depth > MAX_DEPTH
            || self.max_name_bytes > MAX_NAME_BYTES
            || self.max_memory_bytes > MAX_MEMORY_BYTES
            || self.max_wdt_bytes > MAX_WDT_BYTES
            || self.max_wdt_chunks > MAX_WDT_CHUNKS
            || self.max_domain_maps > MAX_DOMAIN_MAPS
            || self.max_domain_bytes > MAX_DOMAIN_BYTES
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
    Allocation,
    DirectCount,
    DuplicateId,
    MissingParent,
    CopiedParent,
    CopiedSelection,
    Cycle,
    Depth,
    UnknownMap,
    ParentUnavailable,
    InactiveMap,
    MissingFileIds,
    Name,
    Objects(objects::Error),
}
impl Error {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidLimits => "collision_plan_invalid_limits",
            Self::Limit => "collision_plan_policy_limit",
            Self::Allocation => "collision_plan_allocation",
            Self::DirectCount => "collision_plan_direct_count",
            Self::DuplicateId => "collision_plan_duplicate_id",
            Self::MissingParent => "collision_plan_missing_parent",
            Self::CopiedParent => "collision_plan_copied_parent",
            Self::CopiedSelection => "collision_plan_copied_selection",
            Self::Cycle => "collision_plan_parent_cycle",
            Self::Depth => "collision_plan_parent_depth",
            Self::UnknownMap => "collision_plan_unknown_map",
            Self::ParentUnavailable => "collision_plan_parent_unavailable",
            Self::InactiveMap => "collision_plan_inactive_map",
            Self::MissingFileIds => "collision_plan_missing_maid",
            Self::Name => "collision_plan_logical_name",
            Self::Objects(error) => error.code(),
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}
impl std::error::Error for Error {}

#[derive(Clone, Copy, Default)]
struct Node {
    parent: Option<usize>,
    depth: usize,
    is_parent: bool,
}
/// Views retain no names or source bytes beyond the borrowed input lifetime.
#[derive(Clone, Copy)]
pub struct MapEntry<'a> {
    row: &'a DirectRecord<VMapFields>,
    node: Node,
    ordinal: usize,
    name_limit: usize,
}
impl<'a> MapEntry<'a> {
    pub const fn id(self) -> u32 {
        self.row.id
    }
    /// The pinned signed DB2 value is passed to a uint32 WDT constructor. Only
    /// zero is inactive; high-bit values preserve their complete FDID bits.
    pub const fn wdt_file_data_id(self) -> u32 {
        self.row.fields.wdt_file_data_id as u32
    }
    pub const fn active(self) -> bool {
        self.row.fields.wdt_file_data_id != 0
    }
    pub const fn parent_map_id(self) -> Option<u32> {
        if self.row.fields.parent_map_id < 0 {
            None
        } else {
            Some(self.row.fields.parent_map_id as u32)
        }
    }
    pub fn name(self) -> &'a str {
        &self.row.fields.name
    }
    pub fn directory(self) -> &'a str {
        &self.row.fields.directory
    }
    pub const fn depth(self) -> usize {
        self.node.depth
    }
    pub const fn is_parent(self) -> bool {
        self.node.is_parent
    }
    pub const fn physical_ordinal(self) -> usize {
        self.ordinal
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stats {
    pub rows: usize,
    pub direct_rows: usize,
    pub active_maps: usize,
    pub max_depth: usize,
    pub resident_bytes: usize,
    pub peak_bytes: usize,
}
/// Own allocations include graph/index/order capacities and traversal scratch.
/// Borrowed rows, their strings and the caller's WDT/name buffers are excluded.
pub struct Maps<'a> {
    rows: &'a [DirectRecord<VMapFields>],
    nodes: Vec<Node>,
    by_id: Vec<usize>,
    preload: Vec<usize>,
    processing: Vec<usize>,
    name_limit: usize,
    selection: MapSelection,
    domain_bytes: usize,
    stats: Stats,
}
impl<'a> Maps<'a> {
    pub fn from_materialized(
        rows: &'a [DirectRecord<VMapFields>],
        direct_count: usize,
        limits: Limits,
    ) -> Result<Self, Error> {
        Self::from_materialized_selection(rows, direct_count, MapSelection::All, limits)
    }
    /// Validate the complete source graph before admitting a selected domain.
    /// Only acquisition/processing orders are narrowed; all/get retain source
    /// provenance. IDs generated by consumers belong to this serial domain.
    pub fn from_materialized_selection(
        rows: &'a [DirectRecord<VMapFields>],
        direct_count: usize,
        selection: MapSelection,
        limits: Limits,
    ) -> Result<Self, Error> {
        limits.validate()?;
        if direct_count > rows.len() {
            return Err(Error::DirectCount);
        }
        if rows.len() > limits.max_rows {
            return Err(Error::Limit);
        }
        let active = rows
            .iter()
            .filter(|row| row.fields.wdt_file_data_id != 0)
            .count();
        if active > limits.max_maps {
            return Err(Error::Limit);
        }
        let mut memory = Memory::new(limits.max_memory_bytes);
        let mut nodes = reserve(rows.len(), &mut memory)?;
        nodes.resize(rows.len(), Node::default());
        let mut by_id = reserve(rows.len(), &mut memory)?;
        by_id.extend(0..rows.len());
        by_id.sort_unstable_by_key(|&ordinal| rows[ordinal].id);
        if by_id
            .windows(2)
            .any(|pair| rows[pair[0]].id == rows[pair[1]].id)
        {
            return Err(Error::DuplicateId);
        }
        for (ordinal, row) in rows.iter().enumerate() {
            if row.fields.parent_map_id < 0 {
                continue;
            }
            let id = row.fields.parent_map_id as u32;
            let index = by_id
                .binary_search_by_key(&id, |&index| rows[index].id)
                .map_err(|_| Error::MissingParent)?;
            let parent = by_id[index];
            if parent >= direct_count {
                return Err(Error::CopiedParent);
            }
            nodes[ordinal].parent = Some(parent);
            nodes[parent].is_parent = true;
        }
        let mut states = reserve(rows.len(), &mut memory)?;
        states.resize(rows.len(), 0u8);
        let path_cap = rows.len().min(limits.max_depth + 1);
        let mut path = reserve(path_cap, &mut memory)?;
        for start in 0..rows.len() {
            let mut current = Some(start);
            while let Some(ordinal) = current {
                match states[ordinal] {
                    2 => break,
                    1 => return Err(Error::Cycle),
                    _ => {}
                }
                if path.len() == path_cap {
                    return Err(Error::Depth);
                }
                states[ordinal] = 1;
                path.push(ordinal);
                current = nodes[ordinal].parent;
            }
            while let Some(ordinal) = path.pop() {
                let depth = match nodes[ordinal].parent {
                    Some(parent) => nodes[parent].depth.checked_add(1).ok_or(Error::Depth)?,
                    None => 0,
                };
                if depth > limits.max_depth {
                    return Err(Error::Depth);
                }
                nodes[ordinal].depth = depth;
                states[ordinal] = 2;
            }
        }
        memory.live -= capacity(&states)? + capacity(&path)?;
        drop(states);
        drop(path);
        let (selected_count, domain_bytes, selected_start) = match selection {
            MapSelection::All => (active, 0, None),
            MapSelection::MapAndAncestors(id) => {
                let index = by_id
                    .binary_search_by_key(&id, |&i| rows[i].id)
                    .map_err(|_| Error::UnknownMap)?;
                let start = by_id[index];
                let mut count = 0usize;
                let mut current = Some(start);
                while let Some(ordinal) = current {
                    if ordinal >= direct_count {
                        return Err(Error::CopiedSelection);
                    }
                    if rows[ordinal].fields.wdt_file_data_id == 0 {
                        return Err(Error::InactiveMap);
                    }
                    count = count.checked_add(1).ok_or(Error::Limit)?;
                    if count > limits.max_domain_maps {
                        return Err(Error::Limit);
                    }
                    current = nodes[ordinal].parent;
                }
                let bytes = count.checked_mul(DOMAIN_MEMBER_BYTES).ok_or(Error::Limit)?;
                if bytes > limits.max_domain_bytes {
                    return Err(Error::Limit);
                }
                memory.live = memory.check(bytes)?;
                memory.peak = memory.peak.max(memory.live);
                (count, bytes, Some(start))
            }
        };
        let mut preload = reserve(selected_count, &mut memory)?;
        if let Some(start) = selected_start {
            let mut current = Some(start);
            while let Some(ordinal) = current {
                preload.push(ordinal);
                current = nodes[ordinal].parent;
            }
            preload.sort_unstable();
        } else {
            preload.extend(
                rows.iter()
                    .enumerate()
                    .filter(|(_, row)| row.fields.wdt_file_data_id != 0)
                    .map(|(ordinal, _)| ordinal),
            );
        }
        let mut processing = reserve(selected_count, &mut memory)?;
        processing.extend_from_slice(&preload);
        processing.sort_unstable_by_key(|&ordinal| (nodes[ordinal].depth, ordinal));
        let stats = Stats {
            rows: rows.len(),
            direct_rows: direct_count,
            active_maps: selected_count,
            max_depth: nodes.iter().map(|node| node.depth).max().unwrap_or(0),
            resident_bytes: memory.live,
            peak_bytes: memory.peak,
        };
        Ok(Self {
            rows,
            nodes,
            by_id,
            preload,
            processing,
            name_limit: limits.max_name_bytes,
            selection,
            domain_bytes,
            stats,
        })
    }
    fn entry(&self, ordinal: usize) -> MapEntry<'a> {
        MapEntry {
            row: &self.rows[ordinal],
            node: self.nodes[ordinal],
            ordinal,
            name_limit: self.name_limit,
        }
    }
    pub fn all(&self) -> impl ExactSizeIterator<Item = MapEntry<'a>> + '_ {
        (0..self.rows.len()).map(|ordinal| self.entry(ordinal))
    }
    pub fn preload(&self) -> impl ExactSizeIterator<Item = MapEntry<'a>> + '_ {
        self.preload.iter().map(|&ordinal| self.entry(ordinal))
    }
    pub const fn selection(&self) -> MapSelection {
        self.selection
    }
    pub const fn domain_bytes(&self) -> usize {
        self.domain_bytes
    }
    /// Active selected IDs in original physical order, never numeric-ID order.
    pub fn selected_ids(&self) -> impl ExactSizeIterator<Item = u32> + '_ {
        self.preload.iter().map(|&i| self.rows[i].id)
    }
    pub fn contains_selected(&self, id: u32) -> bool {
        self.get(id)
            .is_some_and(|map| self.preload.binary_search(&map.ordinal).is_ok())
    }
    pub fn processing(&self) -> impl ExactSizeIterator<Item = MapEntry<'a>> + '_ {
        self.processing.iter().map(|&ordinal| self.entry(ordinal))
    }
    pub fn get(&self, id: u32) -> Option<MapEntry<'a>> {
        let index = self
            .by_id
            .binary_search_by_key(&id, |&index| self.rows[index].id)
            .ok()?;
        Some(self.entry(self.by_id[index]))
    }
    /// Only the immediate graph parent is exposed. The pin repeatedly pushes
    /// that same WDT while walking ancestry; it does not acquire ancestor tiles.
    /// This check neither establishes a populated parent cache nor retries I/O.
    pub fn fallback_parent(&self, id: u32) -> Result<Option<MapEntry<'a>>, Error> {
        let entry = self.get(id).ok_or(Error::UnknownMap)?;
        let Some(parent) = entry.node.parent else {
            return Ok(None);
        };
        let parent = self.entry(parent);
        if !parent.active() {
            return Err(Error::ParentUnavailable);
        }
        Ok(Some(parent))
    }
    pub const fn stats(&self) -> Stats {
        self.stats
    }
}

struct Memory {
    limit: usize,
    live: usize,
    peak: usize,
}
impl Memory {
    fn new(limit: usize) -> Self {
        Self {
            limit,
            live: 0,
            peak: 0,
        }
    }
    fn check(&self, extra: usize) -> Result<usize, Error> {
        self.live
            .checked_add(extra)
            .filter(|&total| total <= self.limit)
            .ok_or(Error::Limit)
    }
}
fn reserve<T>(count: usize, memory: &mut Memory) -> Result<Vec<T>, Error> {
    memory.check(count.checked_mul(size_of::<T>()).ok_or(Error::Limit)?)?;
    let mut values = Vec::new();
    values
        .try_reserve_exact(count)
        .map_err(|_| Error::Allocation)?;
    memory.live = memory.check(capacity(&values)?)?;
    memory.peak = memory.peak.max(memory.live);
    Ok(values)
}
fn capacity<T>(values: &Vec<T>) -> Result<usize, Error> {
    values
        .capacity()
        .checked_mul(size_of::<T>())
        .ok_or(Error::Limit)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WdtStats {
    pub chunks: usize,
    pub global_records: usize,
    pub present_tiles: usize,
    pub resident_bytes: usize,
    pub peak_bytes: usize,
}
/// Full framing and global-placement validation precede every exposed view.
/// Missing MPHD/MAIN mean zero, missing MVER and arbitrary MVER are admitted.
/// Unknown chunks remain opaque. Duplicate metadata and absent MAID for an
/// actually present FDID-routed tile are conservative rejections.
pub struct Wdt<'a> {
    globals: Objects<'a>,
    main: Option<&'a [u8]>,
    maid: Option<&'a [u8]>,
    flags: u32,
    max_coordinate_digits: usize,
    limits: Limits,
    stats: WdtStats,
}
impl<'a> Wdt<'a> {
    pub fn parse(bytes: &'a [u8], limits: Limits) -> Result<Self, Error> {
        limits.validate()?;
        let globals = Objects::parse_wdt(
            bytes,
            objects::Limits {
                max_input_bytes: limits.max_wdt_bytes,
                max_chunks: limits.max_wdt_chunks,
                max_name_bytes: limits.max_name_bytes,
                max_memory_bytes: limits.max_memory_bytes,
                ..objects::Limits::default()
            },
        )
        .map_err(Error::Objects)?;
        let mut main = None;
        let mut maid = None;
        let mut flags = 0;
        let mut at = 0;
        // Objects has checked every header/extent and unique metadata tag.
        while at < bytes.len() {
            let header = &bytes[at..at + 8];
            let length = word(header, 4) as usize;
            let body = &bytes[at + 8..at + 8 + length];
            match &header[..4] {
                b"DHPM" => flags = word(body, 0),
                b"NIAM" => main = Some(body),
                b"DIAM" => maid = Some(body),
                _ => {}
            }
            at += 8 + length;
        }
        let mut present = 0;
        let mut max_coordinate_digits = 0;
        if let Some(main) = main {
            for y in 0..64 {
                for x in 0..64 {
                    if word(main, (y * 64 + x) * 8) & 1 != 0 {
                        present += 1;
                        max_coordinate_digits = max_coordinate_digits.max(digits(x) + digits(y));
                    }
                }
            }
        }
        if flags & 0x200 != 0 && present != 0 && maid.is_none() {
            return Err(Error::MissingFileIds);
        }
        let source = globals.stats();
        let stats = WdtStats {
            chunks: source.chunks,
            global_records: source.records,
            present_tiles: present,
            resident_bytes: source.resident_bytes,
            peak_bytes: source.peak_bytes,
        };
        Ok(Self {
            globals,
            main,
            maid,
            flags,
            max_coordinate_digits,
            limits,
            stats,
        })
    }
    pub fn globals(&self) -> &Objects<'a> {
        &self.globals
    }
    pub const fn flags(&self) -> u32 {
        self.flags
    }
    pub const fn stats(&self) -> WdtStats {
        self.stats
    }
    /// The caller binds this WDT to the map's FDID; the view carries no proof of
    /// that external association. Unused directory bytes are not inspected for
    /// FDID routing or a WDT with no present tiles. Logical names are ASCII/NUL
    /// checked but not case-normalized, basename-normalized or filesystem paths.
    /// Both this WDT's and the originating Maps' name ceilings remain in force.
    pub fn tiles<'w, 'map>(&'w self, map: MapEntry<'map>) -> Result<Tiles<'w, 'a, 'map>, Error> {
        if !map.active() {
            return Err(Error::InactiveMap);
        }
        let directory = map.directory().as_bytes();
        if self.flags & 0x200 == 0 && self.stats.present_tiles != 0 {
            if !directory.is_ascii() || directory.contains(&0) {
                return Err(Error::Name);
            }
            let length = directory
                .len()
                .checked_mul(2)
                .and_then(|length| length.checked_add(23 + self.max_coordinate_digits))
                .ok_or(Error::Limit)?;
            if length > self.limits.max_name_bytes.min(map.name_limit) {
                return Err(Error::Limit);
            }
        }
        Ok(Tiles {
            wdt: self,
            directory,
            cursor: 0,
        })
    }
}
#[derive(Clone, Copy)]
pub enum TileTarget<'a> {
    FileDataId(u32),
    Name(&'a [u8]),
}
#[derive(Clone, Copy)]
pub struct TileRequest<'a> {
    pub x: u8,
    pub y: u8,
    pub target: TileTarget<'a>,
}
/// A lending cursor with bounded caller-owned name storage. All fallible
/// validation happened at construction; after the final request it stays empty.
pub struct Tiles<'w, 'data, 'map> {
    wdt: &'w Wdt<'data>,
    directory: &'map [u8],
    cursor: usize,
}
impl Tiles<'_, '_, '_> {
    pub fn next<'b>(
        &mut self,
        name_buffer: &'b mut [u8; MAX_NAME_BYTES],
    ) -> Option<TileRequest<'b>> {
        let main = self.wdt.main?;
        while self.cursor < 4096 {
            let x = self.cursor / 64;
            let y = self.cursor % 64;
            self.cursor += 1;
            let cell = y * 64 + x;
            if word(main, cell * 8) & 1 == 0 {
                continue;
            }
            let target = if self.wdt.flags & 0x200 != 0 {
                // Wdt::parse requires MAID when even one routed tile is present.
                TileTarget::FileDataId(word(self.wdt.maid?, cell * 32 + 4))
            } else {
                let mut at = 0;
                append(name_buffer, &mut at, b"World\\Maps\\");
                append(name_buffer, &mut at, self.directory);
                append(name_buffer, &mut at, b"\\");
                append(name_buffer, &mut at, self.directory);
                append(name_buffer, &mut at, b"_");
                decimal(name_buffer, &mut at, x);
                append(name_buffer, &mut at, b"_");
                decimal(name_buffer, &mut at, y);
                append(name_buffer, &mut at, b"_obj0.adt");
                TileTarget::Name(&name_buffer[..at])
            };
            return Some(TileRequest {
                x: x as u8,
                y: y as u8,
                target,
            });
        }
        None
    }
}
fn digits(coordinate: usize) -> usize {
    if coordinate < 10 { 1 } else { 2 }
}
fn decimal(buffer: &mut [u8], at: &mut usize, coordinate: usize) {
    if coordinate >= 10 {
        append(buffer, at, &[b'0' + (coordinate / 10) as u8]);
    }
    append(buffer, at, &[b'0' + (coordinate % 10) as u8]);
}
fn append(buffer: &mut [u8], at: &mut usize, bytes: &[u8]) {
    buffer[*at..*at + bytes.len()].copy_from_slice(bytes);
    *at += bytes.len();
}
fn word(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}
