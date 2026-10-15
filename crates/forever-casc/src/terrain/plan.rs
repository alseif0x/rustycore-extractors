// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original two-phase, bounded terrain dependency planner.
// Source facts: TrinityCore 02245dcd245e7433e524577656177723d3e4992e,
// System.cpp:275-315,1100-1141 and wdt.h:25-86. No upstream code imported.
//! Phase one borrows a validated materialized Map domain, preserving its
//! post-copy/post-zero-filter physical ordinals. A separately admitted available
//! source caller may supply only admitted sections; this planner does not infer
//! completeness of omitted sections or authenticate that scoped domain.
//! Phase two accepts one checked WDT at a time and retains no WDT bytes.
//! Full-map selections expose all 4096 coordinates, including absent tiles;
//! explicit tile selections expose exactly those coordinates. Never truncate.
//! Selected duplicate map IDs reject instead of reproducing the native
//! extractor's repeated output-name overwrites. Missing selected maps reject;
//! selected absent MAIN coordinates remain explicit, never successful reads.
//!
//! A successful MapExpansion is only one map's dependency plan. Completion of
//! the whole selection requires Expansion::finish; any expansion failure
//! poisons that state. No result authenticates the caller's FDID-to-WDT binding,
//! marks a tile acquired, or authorizes a tile list for a tile subset.
//!
//! Names are logical uppercase/backslash CASC references, never local paths.
//! No name lookup, name hash, MAX_PATH truncation, remote/prefix fallback or
//! FDID invention occurs. Duplicate/zero/max ADT FDIDs remain explicit.
//! The 1024-byte logical-name policy does not imply CascLib MAX_PATH admission.

use forever_db2::terrain_sources::{DirectRecord, MapFields};
use forever_terrain::{Wdt, wdt};
use std::{fmt::Write, mem::size_of, ops::Range};

const SOURCE_ROWS: usize = 200_000;
const EXPLICIT_TILES: usize = 1_000_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    AllMaps,
    MapIds,
    Tiles,
}

/// Explicit coordinates use WDT x=column, y=row. Map ID is the full u32.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct TileRequest {
    pub map_id: u32,
    pub x: u8,
    pub y: u8,
}

/// Borrowed caller input. Duplicate requests and empty explicit lists reject.
/// AllMaps legitimately accepts an empty fully validated Map source.
#[derive(Clone, Copy)]
pub enum Selection<'a> {
    AllMaps,
    MapIds(&'a [u32]),
    Tiles(&'a [TileRequest]),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub max_source_rows: usize,
    pub max_maps: usize,
    pub max_explicit_tiles: usize,
    /// Counts selected coordinates INCLUDING absent MAIN cells. Full maps
    /// each charge 4096; this is not a resident all-tiles allocation.
    pub max_total_coordinates: u64,
    /// Logical vector capacities, including conservative phase-one scratch.
    /// Borrowed Map rows/strings, allocator overhead and retained caller
    /// MapExpansions are not included; this is not a process RSS bound.
    pub max_planner_bytes: usize,
    /// One returned MapExpansion's vector and logical-name capacities.
    pub max_expansion_bytes: usize,
    pub max_logical_name_bytes: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_source_rows: SOURCE_ROWS,
            max_maps: SOURCE_ROWS,
            max_explicit_tiles: EXPLICIT_TILES,
            max_total_coordinates: SOURCE_ROWS as u64 * wdt::TILE_COUNT as u64,
            max_planner_bytes: 64 * 1024 * 1024,
            max_expansion_bytes: 16 * 1024 * 1024,
            max_logical_name_bytes: 1024,
        }
    }
}
impl Limits {
    pub(super) fn validate(self) -> Result<(), Error> {
        let max = Self::default();
        if self.max_source_rows > max.max_source_rows
            || self.max_maps > max.max_maps
            || self.max_explicit_tiles > max.max_explicit_tiles
            || self.max_total_coordinates > max.max_total_coordinates
            || self.max_planner_bytes > max.max_planner_bytes
            || self.max_expansion_bytes > max.max_expansion_bytes
            || self.max_logical_name_bytes > max.max_logical_name_bytes
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
    EmptySelection,
    DuplicateRequest,
    InvalidCoordinates,
    InvalidMaterializedRows,
    MissingMap,
    AmbiguousMap,
    WrongWdt,
    UnexpectedWdt,
    MissingFileIds,
    InvalidLogicalName,
    FailedExpansion,
    IncompleteExpansion,
}
impl Error {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidLimits => "terrain_plan_invalid_limits",
            Self::Limit => "terrain_plan_policy_limit",
            Self::EmptySelection => "terrain_plan_empty_selection",
            Self::DuplicateRequest => "terrain_plan_duplicate_request",
            Self::InvalidCoordinates => "terrain_plan_invalid_coordinates",
            Self::InvalidMaterializedRows => "terrain_plan_invalid_materialized_rows",
            Self::MissingMap => "terrain_plan_missing_map",
            Self::AmbiguousMap => "terrain_plan_ambiguous_map",
            Self::WrongWdt => "terrain_plan_wrong_wdt",
            Self::UnexpectedWdt => "terrain_plan_unexpected_wdt",
            Self::MissingFileIds => "terrain_plan_missing_file_ids",
            Self::InvalidLogicalName => "terrain_plan_invalid_logical_name",
            Self::FailedExpansion => "terrain_plan_failed_expansion",
            Self::IncompleteExpansion => "terrain_plan_incomplete_expansion",
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        out.write_str(self.code())
    }
}
impl std::error::Error for Error {}

#[derive(Clone, Copy)]
struct IndexedRow {
    id: u32,
    ordinal: usize,
}
struct SelectedMap {
    ordinal: usize,
    tiles: Option<Range<usize>>,
}

/// Borrowed materialized source plus bounded selection indexes; no raw WDTs.
pub struct WdtPlan<'a> {
    rows: &'a [DirectRecord<MapFields>],
    maps: Vec<SelectedMap>,
    tiles: Vec<TileRequest>,
    scope: Scope,
    limits: Limits,
    coordinates: u64,
}
impl<'a> WdtPlan<'a> {
    pub fn from_materialized(
        rows: &'a [DirectRecord<MapFields>],
        selection: Selection<'_>,
        limits: Limits,
    ) -> Result<Self, Error> {
        limits.validate()?;
        if rows.len() > limits.max_source_rows {
            return Err(Error::Limit);
        }
        // read_map().materialize() has already removed WDT==0. Do not silently
        // repair a caller passing direct/unmaterialized rows to this interface.
        if rows.iter().any(|row| row.fields.wdt_file_data_id == 0) {
            return Err(Error::InvalidMaterializedRows);
        }
        let mut budget = Budget::new(limits.max_planner_bytes);
        let mut index = budget.vector::<IndexedRow>(rows.len())?;
        for (ordinal, row) in rows.iter().enumerate() {
            index.push(IndexedRow {
                id: row.id,
                ordinal,
            });
        }
        index.sort_unstable_by_key(|row| (row.id, row.ordinal));

        let (scope, maps, tiles) = match selection {
            Selection::AllMaps => {
                if rows.len() > limits.max_maps {
                    return Err(Error::Limit);
                }
                if index.windows(2).any(|pair| pair[0].id == pair[1].id) {
                    return Err(Error::AmbiguousMap);
                }
                let mut maps = budget.vector(rows.len())?;
                for ordinal in 0..rows.len() {
                    maps.push(SelectedMap {
                        ordinal,
                        tiles: None,
                    });
                }
                (Scope::AllMaps, maps, Vec::new())
            }
            Selection::MapIds(ids) => {
                if ids.is_empty() {
                    return Err(Error::EmptySelection);
                }
                if ids.len() > limits.max_maps {
                    return Err(Error::Limit);
                }
                let mut sorted = budget.vector(ids.len())?;
                sorted.extend_from_slice(ids);
                sorted.sort_unstable();
                if sorted.windows(2).any(|pair| pair[0] == pair[1]) {
                    return Err(Error::DuplicateRequest);
                }
                let mut maps = budget.vector(ids.len())?;
                for id in sorted {
                    maps.push(SelectedMap {
                        ordinal: unique_row(&index, id)?,
                        tiles: None,
                    });
                }
                maps.sort_unstable_by_key(|map| map.ordinal);
                (Scope::MapIds, maps, Vec::new())
            }
            Selection::Tiles(requests) => {
                if requests.is_empty() {
                    return Err(Error::EmptySelection);
                }
                if requests.len() > limits.max_explicit_tiles {
                    return Err(Error::Limit);
                }
                for request in requests {
                    coordinates(request.x, request.y)?;
                }
                let mut tiles = budget.vector(requests.len())?;
                tiles.extend_from_slice(requests);
                tiles.sort_unstable_by_key(|tile| (tile.map_id, tile.y, tile.x));
                if tiles.windows(2).any(|pair| pair[0] == pair[1]) {
                    return Err(Error::DuplicateRequest);
                }
                let count = 1 + tiles
                    .windows(2)
                    .filter(|pair| pair[0].map_id != pair[1].map_id)
                    .count();
                if count > limits.max_maps {
                    return Err(Error::Limit);
                }
                let mut maps = budget.vector(count)?;
                let mut start = 0;
                while start < tiles.len() {
                    let id = tiles[start].map_id;
                    let end = start + tiles[start..].partition_point(|tile| tile.map_id == id);
                    maps.push(SelectedMap {
                        ordinal: unique_row(&index, id)?,
                        tiles: Some(start..end),
                    });
                    start = end;
                }
                maps.sort_unstable_by_key(|map| map.ordinal);
                (Scope::Tiles, maps, tiles)
            }
        };
        let coordinates = if scope == Scope::Tiles {
            u64::try_from(tiles.len()).map_err(|_| Error::Limit)?
        } else {
            u64::try_from(maps.len())
                .map_err(|_| Error::Limit)?
                .checked_mul(wdt::TILE_COUNT as u64)
                .ok_or(Error::Limit)?
        };
        if coordinates > limits.max_total_coordinates {
            return Err(Error::Limit);
        }
        Ok(Self {
            rows,
            maps,
            tiles,
            scope,
            limits,
            coordinates,
        })
    }

    pub const fn scope(&self) -> Scope {
        self.scope
    }
    pub const fn selected_coordinates(&self) -> u64 {
        self.coordinates
    }
    pub fn maps(&self) -> impl ExactSizeIterator<Item = WdtRequest> + '_ {
        self.maps
            .iter()
            .enumerate()
            .map(|(index, map)| self.request(index, map))
    }
    pub fn expansion(&self) -> Expansion<'_, 'a> {
        Expansion {
            plan: self,
            next: 0,
            present: 0,
            failed: false,
        }
    }
    /// Separate whole-map discovery state. Only this state permits an explicit
    /// unavailable-WDT transition; it never classifies unknown MAIN as absent.
    pub fn compatible_expansion(&self) -> CompatibleExpansion<'_, 'a> {
        CompatibleExpansion {
            inner: self.expansion(),
            unknown_coordinates: 0,
            unavailable_wdts: 0,
        }
    }
    fn request(&self, index: usize, selected: &SelectedMap) -> WdtRequest {
        let row = &self.rows[selected.ordinal];
        WdtRequest {
            plan_index: index,
            materialized_ordinal: selected.ordinal,
            map_id: row.id,
            file_data_id: row.fields.wdt_file_data_id as u32,
            whole_map: selected.tiles.is_none(),
        }
    }
}

/// One expected input identity, including repeated WDT FDIDs across maps.
#[derive(Clone, Copy)]
pub struct WdtRequest {
    pub plan_index: usize,
    pub materialized_ordinal: usize,
    pub map_id: u32,
    pub file_data_id: u32,
    pub whole_map: bool,
}

/// Complete dependency projection for one checked WDT. No Debug emits names.
pub struct MapExpansion {
    request: WdtRequest,
    tiles: Vec<Tile>,
    present: usize,
}
impl MapExpansion {
    pub const fn request(&self) -> WdtRequest {
        self.request
    }
    pub fn tiles(&self) -> &[Tile] {
        &self.tiles
    }
    pub const fn present_tiles(&self) -> usize {
        self.present
    }
    pub fn absent_tiles(&self) -> usize {
        self.tiles.len() - self.present
    }
    /// Necessary scope for a future tile list, not proof of acquired outputs.
    /// Always false for Selection::Tiles, even if all coordinates were listed.
    pub const fn covers_whole_map(&self) -> bool {
        self.request.whole_map
    }
}

pub struct Tile {
    x: u8,
    y: u8,
    flags: u32,
    data1: u32,
    source: TileSource,
}
impl Tile {
    pub const fn x(&self) -> u8 {
        self.x
    }
    pub const fn y(&self) -> u8 {
        self.y
    }
    pub const fn flags(&self) -> u32 {
        self.flags
    }
    pub const fn data1(&self) -> u32 {
        self.data1
    }
    pub const fn source(&self) -> &TileSource {
        &self.source
    }
    pub const fn present(&self) -> bool {
        !matches!(self.source, TileSource::Absent)
    }
}

/// Absent is a selected coordinate whose MAIN bit 0 is clear. A name reference
/// is explicit dependency metadata only; there is no automatic acquisition.
pub enum TileSource {
    Absent,
    FileDataId(u32),
    LogicalName(String),
}

pub struct Expansion<'p, 'a> {
    plan: &'p WdtPlan<'a>,
    next: usize,
    present: u64,
    failed: bool,
}
impl Expansion<'_, '_> {
    /// The FDID tag prevents accidental association with the wrong request;
    /// the caller remains responsible for authenticated FDID-to-bytes binding.
    pub fn expand_next(&mut self, file_data_id: u32, wdt: &Wdt<'_>) -> Result<MapExpansion, Error> {
        if self.failed {
            return Err(Error::FailedExpansion);
        }
        match self.expand(file_data_id, wdt) {
            Ok(expanded) => {
                self.present += expanded.present as u64;
                self.next += 1;
                Ok(expanded)
            }
            Err(error) => {
                self.failed = true;
                Err(error)
            }
        }
    }

    fn expand(&self, file_data_id: u32, wdt: &Wdt<'_>) -> Result<MapExpansion, Error> {
        let selected = self.plan.maps.get(self.next).ok_or(Error::UnexpectedWdt)?;
        let request = self.plan.request(self.next, selected);
        if request.file_data_id != file_data_id {
            return Err(Error::WrongWdt);
        }
        let file_ids = wdt
            .mphd()
            .is_some_and(|mphd| mphd.flags & wdt::FILE_ID_REFERENCES != 0);
        let directory = &self.plan.rows[selected.ordinal].fields.directory;
        let count = selected.tiles.as_ref().map_or(wdt::TILE_COUNT, Range::len);
        let coordinate = |index: usize| -> (u8, u8) {
            selected.tiles.as_ref().map_or(
                ((index % wdt::MAP_SIZE) as u8, (index / wdt::MAP_SIZE) as u8),
                |range| {
                    let tile = self.plan.tiles[range.start + index];
                    (tile.x, tile.y)
                },
            )
        };
        // Full preflight before allocation of tile records or logical names.
        let mut estimate = count.checked_mul(size_of::<Tile>()).ok_or(Error::Limit)?;
        let mut present = 0;
        for index in 0..count {
            let (x, y) = coordinate(index);
            let tile = wdt
                .tile(usize::from(x), usize::from(y))
                .ok_or(Error::InvalidCoordinates)?;
            if tile.present() {
                present += 1;
                if file_ids {
                    if tile.file_ids.is_none() {
                        return Err(Error::MissingFileIds);
                    }
                } else {
                    let length =
                        name_length(directory, x, y, self.plan.limits.max_logical_name_bytes)?;
                    estimate = estimate.checked_add(length).ok_or(Error::Limit)?;
                }
            }
        }
        if estimate > self.plan.limits.max_expansion_bytes {
            return Err(Error::Limit);
        }
        let mut budget = Budget::new(self.plan.limits.max_expansion_bytes);
        let mut tiles = budget.vector(count)?;
        for index in 0..count {
            let (x, y) = coordinate(index);
            let tile = wdt
                .tile(usize::from(x), usize::from(y))
                .ok_or(Error::InvalidCoordinates)?;
            let source = if !tile.present() {
                TileSource::Absent
            } else if file_ids {
                TileSource::FileDataId(tile.file_ids.ok_or(Error::MissingFileIds)?.root_adt)
            } else {
                TileSource::LogicalName(logical_name(
                    directory,
                    x,
                    y,
                    self.plan.limits.max_logical_name_bytes,
                    &mut budget,
                )?)
            };
            tiles.push(Tile {
                x,
                y,
                flags: tile.flags,
                data1: tile.data1,
                source,
            });
        }
        Ok(MapExpansion {
            request,
            tiles,
            present,
        })
    }

    pub fn finish(self) -> Result<Completion, Error> {
        if self.failed {
            return Err(Error::FailedExpansion);
        }
        if self.next != self.plan.maps.len() {
            return Err(Error::IncompleteExpansion);
        }
        Ok(Completion {
            scope: self.plan.scope,
            maps: self.next,
            selected_coordinates: self.plan.coordinates,
            present_tiles: self.present,
            absent_tiles: self.plan.coordinates - self.present,
        })
    }
}

/// Discovery completion only; no file acquisition or MAPS verification claim.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Completion {
    pub scope: Scope,
    pub maps: usize,
    pub selected_coordinates: u64,
    pub present_tiles: u64,
    pub absent_tiles: u64,
}

/// Discovery facts for the separate compatible whole-map traversal. Unknown
/// coordinates belong to unavailable WDTs, and are neither present nor absent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompatibleCompletion {
    pub scope: Scope,
    pub maps: usize,
    pub selected_coordinates: u64,
    pub known_present_tiles: u64,
    pub known_absent_tiles: u64,
    pub unknown_coordinates: u64,
    pub unavailable_wdts: usize,
}

/// Serial whole-map discovery with a separately named unavailable transition.
/// Tile selections reject and poison this state; strict Expansion is unchanged.
pub struct CompatibleExpansion<'p, 'a> {
    inner: Expansion<'p, 'a>,
    unknown_coordinates: u64,
    unavailable_wdts: usize,
}
impl CompatibleExpansion<'_, '_> {
    fn whole_map_state(&mut self) -> Result<(), Error> {
        if self.inner.failed {
            return Err(Error::FailedExpansion);
        }
        if self.inner.plan.scope == Scope::Tiles {
            self.inner.failed = true;
            return Err(Error::UnexpectedWdt);
        }
        Ok(())
    }
    pub fn expand_next(&mut self, file_data_id: u32, wdt: &Wdt<'_>) -> Result<MapExpansion, Error> {
        self.whole_map_state()?;
        self.inner.expand_next(file_data_id, wdt)
    }
    /// The full request tag must be the exact next selected ordinal, even when
    /// different maps use the same FDID. No WDT bytes or MAIN facts are invented.
    pub fn unavailable_next(&mut self, request: WdtRequest) -> Result<(), Error> {
        self.whole_map_state()?;
        let result = (|| {
            let selected = self
                .inner
                .plan
                .maps
                .get(self.inner.next)
                .ok_or(Error::UnexpectedWdt)?;
            let expected = self.inner.plan.request(self.inner.next, selected);
            if request.plan_index != expected.plan_index
                || request.materialized_ordinal != expected.materialized_ordinal
                || request.map_id != expected.map_id
                || request.file_data_id != expected.file_data_id
                || request.whole_map != expected.whole_map
                || !request.whole_map
            {
                return Err(Error::WrongWdt);
            }
            let coordinates = self
                .unknown_coordinates
                .checked_add(wdt::TILE_COUNT as u64)
                .ok_or(Error::Limit)?;
            let unavailable = self.unavailable_wdts.checked_add(1).ok_or(Error::Limit)?;
            self.unknown_coordinates = coordinates;
            self.unavailable_wdts = unavailable;
            self.inner.next += 1;
            Ok(())
        })();
        if result.is_err() {
            self.inner.failed = true;
        }
        result
    }
    pub fn finish_compatible(mut self) -> Result<CompatibleCompletion, Error> {
        self.whole_map_state()?;
        let complete = self.inner.finish()?;
        Ok(CompatibleCompletion {
            scope: complete.scope,
            maps: complete.maps,
            selected_coordinates: complete.selected_coordinates,
            known_present_tiles: complete.present_tiles,
            known_absent_tiles: complete
                .absent_tiles
                .checked_sub(self.unknown_coordinates)
                .ok_or(Error::Limit)?,
            unknown_coordinates: self.unknown_coordinates,
            unavailable_wdts: self.unavailable_wdts,
        })
    }
}

/// Canonical relative output identity; never uses Name or Directory. Decimal
/// widths are minimums, retaining full-u32 map IDs and row-before-column order.
pub fn map_relative_path(map_id: u32, x: u8, y: u8) -> Result<String, Error> {
    coordinates(x, y)?;
    let mut out = String::new();
    out.try_reserve_exact(32).map_err(|_| Error::Limit)?;
    write!(out, "maps/{map_id:04}_{y:02}_{x:02}.map").map_err(|_| Error::Limit)?;
    Ok(out)
}

fn unique_row(index: &[IndexedRow], id: u32) -> Result<usize, Error> {
    let start = index.partition_point(|row| row.id < id);
    let Some(row) = index.get(start).filter(|row| row.id == id) else {
        return Err(Error::MissingMap);
    };
    if index.get(start + 1).is_some_and(|next| next.id == id) {
        return Err(Error::AmbiguousMap);
    }
    Ok(row.ordinal)
}
fn coordinates(x: u8, y: u8) -> Result<(), Error> {
    if usize::from(x) >= wdt::MAP_SIZE || usize::from(y) >= wdt::MAP_SIZE {
        Err(Error::InvalidCoordinates)
    } else {
        Ok(())
    }
}
fn name_length(directory: &str, x: u8, y: u8, cap: usize) -> Result<usize, Error> {
    if directory.contains('\0') {
        return Err(Error::InvalidLogicalName);
    }
    let length = directory
        .len()
        .checked_mul(2)
        .and_then(|length| {
            length.checked_add(18 + if x < 10 { 1 } else { 2 } + if y < 10 { 1 } else { 2 })
        })
        .ok_or(Error::Limit)?;
    if length > cap {
        return Err(Error::Limit);
    }
    Ok(length)
}
fn logical_name(
    directory: &str,
    x: u8,
    y: u8,
    cap: usize,
    budget: &mut Budget,
) -> Result<String, Error> {
    let length = name_length(directory, x, y, cap)?;
    budget.check(length)?;
    let mut out = String::new();
    out.try_reserve_exact(length).map_err(|_| Error::Limit)?;
    budget.charge(out.capacity())?;
    out.push_str("WORLD\\MAPS\\");
    let append_directory = |out: &mut String| {
        for value in directory.chars() {
            out.push(if value == '/' {
                '\\'
            } else {
                value.to_ascii_uppercase()
            });
        }
    };
    append_directory(&mut out);
    out.push('\\');
    append_directory(&mut out);
    write!(out, "_{x}_{y}.ADT").map_err(|_| Error::Limit)?;
    if out.len() != length {
        return Err(Error::InvalidLogicalName);
    }
    Ok(out)
}

struct Budget {
    used: usize,
    limit: usize,
}
impl Budget {
    fn new(limit: usize) -> Self {
        Self { used: 0, limit }
    }
    fn check(&self, size: usize) -> Result<(), Error> {
        if self.used.checked_add(size).ok_or(Error::Limit)? > self.limit {
            Err(Error::Limit)
        } else {
            Ok(())
        }
    }
    fn charge(&mut self, size: usize) -> Result<(), Error> {
        self.check(size)?;
        self.used += size;
        Ok(())
    }
    fn vector<T>(&mut self, count: usize) -> Result<Vec<T>, Error> {
        self.check(count.checked_mul(size_of::<T>()).ok_or(Error::Limit)?)?;
        let mut out = Vec::new();
        out.try_reserve_exact(count).map_err(|_| Error::Limit)?;
        self.charge(
            out.capacity()
                .checked_mul(size_of::<T>())
                .ok_or(Error::Limit)?,
        )?;
        Ok(out)
    }
}

#[cfg(test)]
mod tests;
