// SPDX-License-Identifier: GPL-3.0-or-later
// This file is part of the TrinityCore Project. See AUTHORS for Copyright information.
// This program is free software; you can redistribute it and/or modify it under
// the terms of the GNU General Public License as published by the Free Software
// Foundation; either version 2 of the License, or (at your option) any later version.
// This program is distributed in the hope that it will be useful, but WITHOUT
// ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
// FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.
// You should have received a copy of the GNU General Public License along with
// this program. If not, see <http://www.gnu.org/licenses/>.
// Modified/ported 2026-10-11: bounded MapBuilder inventory/ancestor planning.
//! Planning consumes complete supplied directory descriptors. The optional Linux
//! `fs` adapter discovers and binds rooted immutable local packages.
//! Original PathGenerator::LoadMap supplies direct last-ID-wins numeric rows;
//! parent selection, filters, tile union/order and root parameters follow pinned
//! MapBuilder.cpp. Copies are not replayed. Invalid topology/incomplete ancestor
//! inventories reject instead of native default-inserted rows/invalid navmeshes.
use crate::{
    mmap,
    tilebuilder::{self, Coordinate},
};
use forever_db2::terrain_sources::{DirectRecord, NavigationMapFields, SourceRecords};
use std::mem::size_of;

pub mod fs;
#[cfg(windows)]
pub mod windows;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Limit,
    Allocation,
    UnknownMap,
    Parent,
    Cycle,
    Depth,
    Coordinates,
    Build,
    TileList,
    Selection,
    MissingInventory,
    EmptyAncestor,
    Params,
}
impl Error {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Limit => "navigation_inventory_policy_limit",
            Self::Allocation => "navigation_inventory_allocation",
            Self::UnknownMap => "navigation_inventory_unknown_map",
            Self::Parent => "navigation_inventory_invalid_parent",
            Self::Cycle => "navigation_inventory_parent_cycle",
            Self::Depth => "navigation_inventory_parent_depth",
            Self::Coordinates => "navigation_inventory_tile_coordinates",
            Self::Build => "navigation_inventory_build",
            Self::TileList => "navigation_inventory_tile_list",
            Self::Selection => "navigation_inventory_selection",
            Self::MissingInventory => "navigation_inventory_missing_ancestor_inventory",
            Self::EmptyAncestor => "navigation_inventory_empty_ancestor_inventory",
            Self::Params => "navigation_inventory_navmesh_params",
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}
impl std::error::Error for Error {}
#[derive(Clone, Copy, Debug, Default)]
pub struct Options {
    pub map: Option<u32>,
    pub skip_continents: bool,
    pub skip_junk: bool,
    pub skip_battlegrounds: bool,
}
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub rows: usize,
    pub maps: usize,
    pub descriptors: usize,
    pub depth: usize,
    pub heap_bytes: usize,
    pub work: u64,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            rows: 200_000,
            maps: 4096,
            descriptors: 1_000_000,
            depth: 1024,
            heap_bytes: 64 * 1024 * 1024,
            work: 128_000_000,
        }
    }
}
impl Limits {
    fn validate(self) -> Result<(), Error> {
        let m = Self::default();
        if self.rows > m.rows
            || self.maps > m.maps
            || self.descriptors > m.descriptors
            || self.depth > m.depth
            || self.heap_bytes > m.heap_bytes
            || self.work > m.work
        {
            return Err(Error::Limit);
        }
        Ok(())
    }
}
/// The caller binds complete immutable tile-list bytes and collision filenames
/// to map IDs. Presence of an empty collision directory creates an empty entry.
/// Coordinates use filename order (row,column), not ADT (x,y).
#[derive(Clone, Copy)]
pub enum Input<'a> {
    Terrain { map_id: u32, bytes: &'a [u8] },
    CollisionMap { map_id: u32 },
    CollisionTile { map_id: u32, tile: Coordinate },
}
impl Input<'_> {
    fn map_id(self) -> u32 {
        match self {
            Self::Terrain { map_id, .. }
            | Self::CollisionMap { map_id }
            | Self::CollisionTile { map_id, .. } => map_id,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Statistics {
    pub direct_rows: usize,
    pub unique_source_maps: usize,
    pub discovered_maps: usize,
    pub inventory_tiles: usize,
    pub output_maps: usize,
    pub output_tiles: usize,
    pub work: u64,
    pub heap_reservation: usize,
}
struct Budget {
    limits: Limits,
    stats: Statistics,
}
impl Budget {
    fn charge(&mut self, n: u64) -> Result<(), Error> {
        self.stats.work = self.stats.work.checked_add(n).ok_or(Error::Limit)?;
        if self.stats.work > self.limits.work {
            return Err(Error::Limit);
        }
        Ok(())
    }
    fn reserve<T>(&mut self, n: usize) -> Result<Vec<T>, Error> {
        let bytes = n.checked_mul(size_of::<T>()).ok_or(Error::Limit)?;
        if self
            .stats
            .heap_reservation
            .checked_add(bytes)
            .ok_or(Error::Limit)?
            > self.limits.heap_bytes
        {
            return Err(Error::Limit);
        }
        let mut out = Vec::new();
        out.try_reserve_exact(n).map_err(|_| Error::Allocation)?;
        self.stats.heap_reservation = self
            .stats
            .heap_reservation
            .checked_add(out.capacity() * size_of::<T>())
            .ok_or(Error::Limit)?;
        if self.stats.heap_reservation > self.limits.heap_bytes {
            return Err(Error::Limit);
        }
        Ok(out)
    }
    fn sort(&mut self, n: usize) -> Result<(), Error> {
        self.charge(n as u64 * (usize::BITS - n.max(1).leading_zeros()) as u64)
    }
}
#[derive(Clone, Copy)]
struct Node {
    id: u32,
    fields: NavigationMapFields,
    ordinal: usize,
    parent: Option<usize>,
    root: usize,
    depth: usize,
}
pub struct Map {
    map_id: u32,
    root_map_id: u32,
    output: bool,
    tiles: [u64; 64],
    terrain_tiles: [u64; 64],
    collision_tiles: [u64; 64],
    count: usize,
    params: Option<mmap::Params>,
}
impl Map {
    pub const fn map_id(&self) -> u32 {
        self.map_id
    }
    pub const fn root_map_id(&self) -> u32 {
        self.root_map_id
    }
    /// Empty maps have no output even in all-map mode. Ancestor-only entries
    /// remain discoverable but are not extra requested output maps.
    pub const fn is_output(&self) -> bool {
        self.output
    }
    pub const fn tile_count(&self) -> usize {
        self.count
    }
    pub const fn params(&self) -> Option<mmap::Params> {
        self.params
    }
    pub fn contains(&self, tile: Coordinate) -> bool {
        tile.row < 64
            && tile.column < 64
            && self.tiles[tile.row as usize] & (1u64 << tile.column) != 0
    }
    pub fn terrain_contains(&self, tile: Coordinate) -> bool {
        tile.row < 64
            && tile.column < 64
            && self.terrain_tiles[tile.row as usize] & (1u64 << tile.column) != 0
    }
    pub fn collision_contains(&self, tile: Coordinate) -> bool {
        tile.row < 64
            && tile.column < 64
            && self.collision_tiles[tile.row as usize] & (1u64 << tile.column) != 0
    }
    /// Original FlatSet packTileID order: row major, then column.
    pub fn tiles(&self) -> impl Iterator<Item = Coordinate> + '_ {
        (0u8..64).flat_map(move |row| {
            (0u8..64).filter_map(move |column| {
                self.contains(Coordinate { row, column })
                    .then_some(Coordinate { row, column })
            })
        })
    }
    fn insert(&mut self, tile: Coordinate, terrain: bool) -> Result<(), Error> {
        if tile.row >= 64 || tile.column >= 64 {
            return Err(Error::Coordinates);
        }
        let component = if terrain {
            &mut self.terrain_tiles
        } else {
            &mut self.collision_tiles
        };
        component[tile.row as usize] |= 1u64 << tile.column;
        let word = &mut self.tiles[tile.row as usize];
        let bit = 1u64 << tile.column;
        if *word & bit == 0 {
            *word |= bit;
            self.count += 1;
        }
        Ok(())
    }
}
pub struct Plan {
    build: u32,
    nodes: Vec<Node>,
    maps: Vec<Map>,
    statistics: Statistics,
}
impl Plan {
    /// This entry point borrows a fully validated closed source snapshot. Copies
    /// are intentionally ignored, as in original navigation startup.
    pub fn from_source(
        source: &SourceRecords<NavigationMapFields>,
        inputs: &[Input<'_>],
        build: u32,
        options: Options,
        limits: Limits,
    ) -> Result<Self, Error> {
        Self::from_rows(source.direct_records(), inputs, build, options, limits)
    }
    /// Independently supplied direct rows must come from the complete source.
    /// This API cannot authenticate provenance or descriptor/file associations.
    /// Last physical duplicate ID wins, including map zero. No WDT-zero filter.
    pub fn from_rows(
        rows: &[DirectRecord<NavigationMapFields>],
        inputs: &[Input<'_>],
        build: u32,
        options: Options,
        limits: Limits,
    ) -> Result<Self, Error> {
        limits.validate()?;
        if !matches!(build, 70170 | 70205) {
            return Err(Error::Build);
        }
        if rows.len() > limits.rows || inputs.len() > limits.descriptors {
            return Err(Error::Limit);
        }
        if options.map.is_some_and(|id| id > i32::MAX as u32) {
            return Err(Error::Selection);
        }
        let mut b = Budget {
            limits,
            stats: Statistics {
                direct_rows: rows.len(),
                ..Default::default()
            },
        };
        let mut nodes = b.reserve(rows.len())?;
        b.charge(rows.len() as u64)?;
        for (ordinal, r) in rows.iter().enumerate() {
            nodes.push(Node {
                id: r.id,
                fields: r.fields,
                ordinal,
                parent: None,
                root: usize::MAX,
                depth: 0,
            });
        }
        b.sort(nodes.len())?;
        nodes.sort_unstable_by(|a, c| a.id.cmp(&c.id).then(c.ordinal.cmp(&a.ordinal)));
        nodes.dedup_by_key(|n| n.id);
        b.stats.unique_source_maps = nodes.len();
        for i in 0..nodes.len() {
            b.charge(1)?;
            let parent = nodes[i].fields.effective_parent();
            nodes[i].parent = match parent {
                -1 => None,
                0.. => Some(
                    nodes
                        .binary_search_by_key(&(parent as u32), |n| n.id)
                        .map_err(|_| Error::Parent)?,
                ),
                _ => return Err(Error::Parent),
            };
        }
        // Resolve every source chain before filters. Temporary stack residency
        // is charged and cycles never rely on unchecked native operator[].
        let mut stack = b.reserve(limits.depth.min(nodes.len()))?;
        for i in 0..nodes.len() {
            if nodes[i].root != usize::MAX {
                continue;
            }
            stack.clear();
            let mut at = i;
            while nodes[at].root == usize::MAX {
                b.charge(1 + stack.len() as u64)?;
                if stack.contains(&at) {
                    return Err(Error::Cycle);
                }
                if stack.len() >= limits.depth {
                    return Err(Error::Depth);
                }
                stack.push(at);
                if let Some(parent) = nodes[at].parent {
                    at = parent;
                } else {
                    nodes[at].root = at;
                    nodes[at].depth = 0;
                    break;
                }
            }
            while let Some(n) = stack.pop() {
                if let Some(parent) = nodes[n].parent {
                    let depth = nodes[parent].depth + 1;
                    if depth >= limits.depth {
                        return Err(Error::Depth);
                    }
                    nodes[n].root = nodes[parent].root;
                    nodes[n].depth = depth;
                }
            }
        }
        drop(stack);
        let selected = options
            .map
            .map(|id| {
                nodes
                    .binary_search_by_key(&id, |n| n.id)
                    .map_err(|_| Error::UnknownMap)
            })
            .transpose()?;
        let mut ids = b.reserve(inputs.len())?;
        for &input in inputs {
            b.charge(1)?;
            let index = nodes
                .binary_search_by_key(&input.map_id(), |n| n.id)
                .map_err(|_| Error::UnknownMap)?;
            if admitted(index, &nodes, selected, options, &mut b)? {
                ids.push(index);
            }
        }
        b.sort(ids.len())?;
        ids.sort_unstable();
        ids.dedup();
        if ids.len() > limits.maps {
            return Err(Error::Limit);
        }
        let mut maps = b.reserve(ids.len())?;
        for &i in &ids {
            maps.push(Map {
                map_id: nodes[i].id,
                root_map_id: nodes[nodes[i].root].id,
                output: false,
                tiles: [0; 64],
                terrain_tiles: [0; 64],
                collision_tiles: [0; 64],
                count: 0,
                params: None,
            });
        }
        drop(ids);
        for &input in inputs {
            b.charge(1)?;
            let Ok(i) = maps.binary_search_by_key(&input.map_id(), |m| m.map_id) else {
                continue;
            };
            match input {
                Input::CollisionMap { .. } => (),
                Input::CollisionTile { tile, .. } => maps[i].insert(tile, false)?,
                Input::Terrain { bytes, .. } => {
                    b.charge(8192)?;
                    validate_tilelist(bytes, build)?;
                    for row in 0..64 {
                        for column in 0..64 {
                            if bytes[12 + 4095 - (row as usize * 64 + column as usize)] == b'1' {
                                maps[i].insert(Coordinate { row, column }, true)?;
                            }
                        }
                    }
                }
            }
        }
        if let Some(id) = options.map
            && maps.binary_search_by_key(&id, |m| m.map_id).is_err()
        {
            return Err(Error::MissingInventory);
        }
        for i in 0..maps.len() {
            b.stats.inventory_tiles += maps[i].count;
            maps[i].output =
                maps[i].count != 0 && options.map.is_none_or(|id| id == maps[i].map_id);
            if !maps[i].output {
                continue;
            }
            let root = maps
                .binary_search_by_key(&maps[i].root_map_id, |m| m.map_id)
                .map_err(|_| Error::MissingInventory)?;
            if maps[root].count == 0 {
                return Err(Error::EmptyAncestor);
            }
            b.charge(4096 + maps[root].count as u64)?;
            let mut tiles = [Coordinate { row: 0, column: 0 }; 4096];
            for (j, t) in maps[root].tiles().enumerate() {
                tiles[j] = t;
            }
            maps[i].params = Some(
                tilebuilder::global_params(&tiles[..maps[root].count])
                    .map_err(|_| Error::Params)?,
            );
            b.stats.output_maps += 1;
            b.stats.output_tiles += maps[i].count;
        }
        b.stats.discovered_maps = maps.len();
        Ok(Self {
            build,
            nodes,
            maps,
            statistics: b.stats,
        })
    }
    /// Complete chosen effective graph, normalized for static-store startup.
    /// Raw parent/cosmetic fields remain in the original source snapshot.
    pub fn parent_rows(&self) -> impl Iterator<Item = forever_vmap::store::MapParent> + '_ {
        self.nodes.iter().map(|n| forever_vmap::store::MapParent {
            map_id: n.id,
            parent: n.parent.map(|p| self.nodes[p].id),
            cosmetic_parent: None,
        })
    }
    pub const fn build(&self) -> u32 {
        self.build
    }
    #[cfg(any(target_os = "linux", windows))]
    pub(crate) fn admits_inventory(&self, id: u32, options: Options) -> Result<bool, Error> {
        let index = self
            .nodes
            .binary_search_by_key(&id, |n| n.id)
            .map_err(|_| Error::UnknownMap)?;
        let selected = options
            .map
            .map(|id| {
                self.nodes
                    .binary_search_by_key(&id, |n| n.id)
                    .map_err(|_| Error::UnknownMap)
            })
            .transpose()?;
        admitted(
            index,
            &self.nodes,
            selected,
            options,
            &mut Budget {
                limits: Limits::default(),
                stats: Statistics::default(),
            },
        )
    }
    pub fn maps(&self) -> &[Map] {
        &self.maps
    }
    pub fn map(&self, id: u32) -> Option<&Map> {
        self.maps
            .binary_search_by_key(&id, |m| m.map_id)
            .ok()
            .map(|i| &self.maps[i])
    }
    pub const fn statistics(&self) -> Statistics {
        self.statistics
    }
    /// Self followed by chosen parents. The graph is already complete/cycle-free.
    pub fn ancestors(&self, id: u32) -> Result<impl Iterator<Item = u32> + '_, Error> {
        let mut index = Some(
            self.nodes
                .binary_search_by_key(&id, |n| n.id)
                .map_err(|_| Error::UnknownMap)?,
        );
        Ok(std::iter::from_fn(move || {
            let n = index?;
            index = self.nodes[n].parent;
            Some(self.nodes[n].id)
        }))
    }
}
fn admitted(
    index: usize,
    nodes: &[Node],
    selected: Option<usize>,
    o: Options,
    b: &mut Budget,
) -> Result<bool, Error> {
    if let Some(mut at) = selected {
        loop {
            b.charge(1)?;
            if index == at {
                return Ok(true);
            }
            let Some(parent) = nodes[at].parent else {
                return Ok(false);
            };
            at = parent;
        }
    }
    let n = &nodes[index];
    Ok(!(o.skip_continents
        && matches!(
            n.id,
            0 | 1 | 530 | 571 | 870 | 1116 | 1220 | 1642 | 1643 | 2222
        )
        || o.skip_junk && (n.fields.flags & 2 != 0 || n.fields.map_type == 3)
        || o.skip_battlegrounds && n.fields.instance_type == 3))
}
fn validate_tilelist(bytes: &[u8], build: u32) -> Result<(), Error> {
    if bytes.len() != 4108
        || &bytes[..4] != b"MAPS"
        || bytes[4..8] != 10u32.to_le_bytes()
        || bytes[8..12] != build.to_le_bytes()
        || bytes[12..].iter().any(|b| !matches!(b, b'0' | b'1'))
    {
        return Err(Error::TileList);
    }
    Ok(())
}
#[cfg(test)]
mod tests;
