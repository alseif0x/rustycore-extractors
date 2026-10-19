// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: bounded source selection for original terrain loadMap.
//! Only genuine absence permits ancestor lookup. Present complete MAPS are
//! verified before projection; corrupt/read/change errors reject the selection.
use crate::{Mesh, inventory, tilebuilder::Coordinate};
use forever_terrain::maps;
#[cfg(target_os = "linux")]
pub mod fs;
#[cfg(test)]
mod tests;
#[cfg(windows)]
pub mod windows;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Artifact {
    pub map_id: u32,
    pub tile: Coordinate,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Presence {
    Missing,
    Present,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceError {
    Io,
    InputChanged,
    Limit,
}
pub trait Source {
    /// Exactly one immutable complete file; absence must be genuine and carry
    /// no bytes. Provider scratch/identity inventory has its own bounded policy.
    fn read(
        &mut self,
        artifact: Artifact,
        out: &mut ReadBuffer<'_>,
    ) -> Result<Presence, SourceError>;
    fn verify_unchanged(&mut self) -> Result<(), SourceError>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Selection,
    Limit,
    Allocation,
    Source(SourceError),
    Inventory(inventory::Error),
    Map(maps::Error),
    Geometry(crate::Error),
}
impl Error {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Selection => "navigation_terrain_selection",
            Self::Limit => "navigation_terrain_load_limit",
            Self::Allocation => "navigation_terrain_load_allocation",
            Self::Source(SourceError::InputChanged) => "navigation_terrain_input_changed",
            Self::Source(SourceError::Limit) => "navigation_terrain_source_limit",
            Self::Source(SourceError::Io) => "navigation_terrain_source_io",
            Self::Inventory(e) => e.code(),
            Self::Map(e) => e.code(),
            Self::Geometry(e) => e.code(),
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}
impl std::error::Error for Error {}
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub file_bytes: usize,
    pub read_bytes: usize,
    pub reads: usize,
    pub source_heap_bytes: usize,
    pub mesh: crate::Limits,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            file_bytes: maps::MAX_MAP_BYTES,
            read_bytes: 5 * maps::MAX_MAP_BYTES,
            reads: 5120,
            source_heap_bytes: 2 * 1024 * 1024,
            mesh: crate::Limits::default(),
        }
    }
}
impl Limits {
    pub(crate) fn validate(self) -> Result<(), Error> {
        let d = Self::default();
        if self.file_bytes > d.file_bytes
            || self.read_bytes > d.read_bytes
            || self.reads > d.reads
            || self.source_heap_bytes > d.source_heap_bytes
        {
            return Err(Error::Limit);
        }
        self.mesh.validate().map_err(Error::Geometry)
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Statistics {
    pub reads: usize,
    pub read_bytes: usize,
    pub peak_source_capacity: usize,
    pub found_files: usize,
}
struct Memory {
    live: usize,
    peak: usize,
    limit: usize,
}
/// Caller-owned transfer, bounded before allocation and during old/new overlap.
pub struct ReadBuffer<'a> {
    bytes: Vec<u8>,
    limit: usize,
    memory: &'a mut Memory,
}
impl ReadBuffer<'_> {
    pub fn append(&mut self, b: &[u8]) -> Result<(), SourceError> {
        let n = self
            .bytes
            .len()
            .checked_add(b.len())
            .ok_or(SourceError::Limit)?;
        if n > self.limit {
            return Err(SourceError::Limit);
        }
        if n > self.bytes.capacity() {
            let target = n
                .max(self.bytes.capacity().saturating_mul(2))
                .min(self.limit);
            let peak = self
                .memory
                .live
                .checked_add(target)
                .ok_or(SourceError::Limit)?;
            if peak > self.memory.limit {
                return Err(SourceError::Limit);
            }
            let mut next = Vec::new();
            next.try_reserve_exact(target)
                .map_err(|_| SourceError::Limit)?;
            let peak = self
                .memory
                .live
                .checked_add(next.capacity())
                .ok_or(SourceError::Limit)?;
            if peak > self.memory.limit {
                return Err(SourceError::Limit);
            }
            self.memory.peak = self.memory.peak.max(peak);
            next.extend_from_slice(&self.bytes);
            self.memory.live = self.memory.live - self.bytes.capacity() + next.capacity();
            self.bytes = next;
        }
        self.bytes.extend_from_slice(b);
        Ok(())
    }
    pub fn len(&self) -> usize {
        self.bytes.len()
    }
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }
    pub const fn limit(&self) -> usize {
        self.limit
    }
}
struct File {
    bytes: Vec<u8>,
    map: u32,
    tile: Coordinate,
    portion: super::Portion,
}
pub struct Loaded {
    mesh: Mesh,
    origins: [Option<u32>; 5],
    statistics: Statistics,
}
impl Loaded {
    pub fn mesh(&self) -> &Mesh {
        &self.mesh
    }
    pub fn into_mesh(self) -> Mesh {
        self.mesh
    }
    /// Center, column+1, column-1, row+1, row-1. None includes unrequested
    /// neighbors after an empty/missing center, and out-of-domain boundaries.
    pub const fn origins(&self) -> &[Option<u32>; 5] {
        &self.origins
    }
    pub const fn statistics(&self) -> Statistics {
        self.statistics
    }
}
#[allow(clippy::too_many_arguments)]
fn find<S: Source>(
    source: &mut S,
    plan: &inventory::Plan,
    map: u32,
    tile: Coordinate,
    portion: super::Portion,
    limits: Limits,
    memory: &mut Memory,
    stats: &mut Statistics,
) -> Result<Option<File>, Error> {
    for id in plan.ancestors(map).map_err(Error::Inventory)? {
        if stats.reads >= limits.reads {
            return Err(Error::Limit);
        }
        stats.reads += 1;
        let remaining = limits
            .read_bytes
            .checked_sub(stats.read_bytes)
            .ok_or(Error::Limit)?;
        let mut out = ReadBuffer {
            bytes: Vec::new(),
            limit: limits.file_bytes.min(remaining),
            memory,
        };
        let presence = source.read(Artifact { map_id: id, tile }, &mut out);
        stats.read_bytes = stats
            .read_bytes
            .checked_add(out.len())
            .ok_or(Error::Limit)?;
        let bytes = out.bytes;
        let presence = presence.map_err(Error::Source)?;
        match presence {
            Presence::Missing => {
                if !bytes.is_empty() {
                    return Err(Error::Source(SourceError::Io));
                }
                memory.live -= bytes.capacity();
            }
            Presence::Present => {
                verify_map(&bytes, plan.build(), limits.file_bytes).map_err(Error::Map)?;
                stats.found_files += 1;
                return Ok(Some(File {
                    bytes,
                    map: id,
                    tile,
                    portion,
                }));
            }
        }
    }
    Ok(None)
}
fn region<'a>(file: &'a File, build: u32) -> super::Region<'a> {
    super::Region {
        map: verify_map(&file.bytes, build, maps::MAX_MAP_BYTES).expect("complete checked terrain"),
        row: file.tile.row,
        column: file.tile.column,
        portion: file.portion,
    }
}
fn verify_map(
    bytes: &[u8],
    build: u32,
    max_bytes: usize,
) -> Result<maps::MapView<'_>, maps::Error> {
    #[cfg(target_os = "linux")]
    if build == 70450 {
        maps::verify_70450(bytes, build, max_bytes)
    } else {
        maps::verify(bytes, build, max_bytes)
    }
    #[cfg(not(target_os = "linux"))]
    {
        maps::verify(bytes, build, max_bytes)
    }
}
/// Load only a selected output tile. Center first; missing or triangle-empty
/// center suppresses all neighbor reads. Each requested coordinate traverses
/// the complete validated effective-parent chain independently. Present empty
/// files block fallback. Out-of-domain neighbors are absent without I/O.
/// The center is projected once to decide neighbor I/O, then recomputed with
/// selected neighbors; its first mesh is dropped before further source reads.
/// This bounds computation to at most six region projections. Source capacity
/// and geometry capacity are separate ledgers; provider storage is excluded.
pub fn neighborhood<S: Source>(
    source: &mut S,
    plan: &inventory::Plan,
    map: u32,
    tile: Coordinate,
    skip_liquids: bool,
    limits: Limits,
) -> Result<Loaded, Error> {
    limits.validate()?;
    if tile.row >= 64
        || tile.column >= 64
        || !plan
            .map(map)
            .is_some_and(|m| m.is_output() && m.contains(tile))
    {
        return Err(Error::Selection);
    }
    source.verify_unchanged().map_err(Error::Source)?;
    let mut memory = Memory {
        live: 0,
        peak: 0,
        limit: limits.source_heap_bytes,
    };
    let mut stats = Statistics::default();
    let mut files: [Option<File>; 5] = std::array::from_fn(|_| None);
    files[0] = find(
        source,
        plan,
        map,
        tile,
        super::Portion::Entire,
        limits,
        &mut memory,
        &mut stats,
    )?;
    let mesh = match &files[0] {
        Some(f) => super::project(&[region(f, plan.build())], skip_liquids, limits.mesh)
            .map_err(Error::Geometry)?,
        None => Mesh::default(),
    };
    source.verify_unchanged().map_err(Error::Source)?;
    let mesh = if mesh.has_triangles() {
        drop(mesh);
        let candidates = [
            (
                Some(tile.row),
                tile.column.checked_add(1),
                super::Portion::Left,
            ),
            (
                Some(tile.row),
                tile.column.checked_sub(1),
                super::Portion::Right,
            ),
            (
                tile.row.checked_add(1),
                Some(tile.column),
                super::Portion::Top,
            ),
            (
                tile.row.checked_sub(1),
                Some(tile.column),
                super::Portion::Bottom,
            ),
        ];
        for (i, (row, col, portion)) in candidates.into_iter().enumerate() {
            if let (Some(row), Some(column)) = (row, col)
                && row < 64
                && column < 64
            {
                files[i + 1] = find(
                    source,
                    plan,
                    map,
                    Coordinate { row, column },
                    portion,
                    limits,
                    &mut memory,
                    &mut stats,
                )?;
            }
        }
        let first = region(files[0].as_ref().expect("nonempty center"), plan.build());
        let mut regions = [first; 5];
        let mut n = 0;
        for file in files.iter().flatten() {
            regions[n] = region(file, plan.build());
            n += 1;
        }
        let result =
            super::project(&regions[..n], skip_liquids, limits.mesh).map_err(Error::Geometry)?;
        source.verify_unchanged().map_err(Error::Source)?;
        result
    } else {
        mesh
    };
    stats.peak_source_capacity = memory.peak;
    Ok(Loaded {
        mesh,
        origins: std::array::from_fn(|i| files[i].as_ref().map(|f| f.map)),
        statistics: stats,
    })
}
