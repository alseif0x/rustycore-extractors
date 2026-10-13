// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11 from pinned format/operation facts; no upstream import.
//! Pure fresh-state assembly of ONE map from complete placements.
//!
//! Reference 02245dcd245e7433e524577656177723d3e4992e, TileAssembler.cpp
//! 169-330 and MapTree.h 64-65. Every input is validated before first-ID-wins
//! selection; missing bounds or invalid duplicate records fail the whole call.
//! Selected IDs are sorted, the map BIH uses leaf size 3, and tile indices are
//! ordinals in that sorted sequence, not positions in the BIH permutation.
//! Own records precede parent records, with ascending IDs within each class.
//! Parent-only tiles have an index and no payload. No ancestor is resolved here.
//!
//! Tile coordinates retain the source f32 multiply and truncation toward zero.
//! The reviewed injective filename domain is X=0..32767, Y=0..255 AFTER
//! truncation (negative fractions that truncate to zero remain admitted).
//! There is no 0..63 clipping. Other coordinates fail, avoiding native undefined
//! float casts and pack/unpack aliases. Global placements have no special case.
//!
//! Readers require complete framing, counts, bounds, order and EOF. Association
//! verification needs an explicitly supplied complete ordered placement proof;
//! it neither authenticates that proof nor establishes full tile inventory or
//! geometric BIH containment. Inherited verification compares the actual
//! ancestor payload in its existing order, ignoring only the parent flag bit.
//! No acquisition, I/O, runtime queries, parent lookup or omitted-error success.
use crate::{bih, placement};
mod build;
mod owned;
mod read;
#[cfg(test)]
mod tests;
pub use build::build;
pub use owned::OwnedTree;
pub use read::{IndexView, TileView, Tree, verify_associations, verify_inherited_associations};

pub const MAX_INPUT_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_TILE_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_SPAWNS: usize = 1_000_000;
pub const MAX_ASSOCIATIONS: usize = 16_000_000;
pub const MAX_TILES: usize = 1_000_000;
pub const MAX_OUTPUT_BYTES: usize = 512 * 1024 * 1024;
pub const MAX_MEMORY_BYTES: usize = 1024 * 1024 * 1024;
const MAGIC: &[u8; 8] = b"VMAP_4.E";

/// Reducible ceilings, including zero. Input/proof and EACH tile file have
/// 64 MiB codec ceilings; output sums tree/payload/index bytes, excluding proof.
/// Heap accounting includes actual Vec capacities, old/new overlap, sorting
/// storage, encoded input, retained proof, descriptors and BIH validation.
/// Caller input/name storage, bounded stack and allocator overhead are excluded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub max_input_bytes: usize,
    pub max_tile_bytes: usize,
    pub max_spawns: usize,
    pub max_associations: usize,
    pub max_tiles: usize,
    pub max_output_bytes: usize,
    pub max_memory_bytes: usize,
    pub max_name_bytes: usize,
    pub bih: bih::Limits,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_input_bytes: MAX_INPUT_BYTES,
            max_tile_bytes: MAX_TILE_BYTES,
            max_spawns: MAX_SPAWNS,
            max_associations: MAX_ASSOCIATIONS,
            max_tiles: MAX_TILES,
            max_output_bytes: MAX_OUTPUT_BYTES,
            max_memory_bytes: MAX_MEMORY_BYTES,
            max_name_bytes: placement::MAX_NAME_BYTES,
            bih: bih::Limits::default(),
        }
    }
}
impl Limits {
    fn validate(self) -> Result<(), Error> {
        let b = self.bih;
        if self.max_input_bytes > MAX_INPUT_BYTES
            || self.max_tile_bytes > MAX_TILE_BYTES
            || self.max_spawns > MAX_SPAWNS
            || self.max_associations > MAX_ASSOCIATIONS
            || self.max_tiles > MAX_TILES
            || self.max_output_bytes > MAX_OUTPUT_BYTES
            || self.max_memory_bytes > MAX_MEMORY_BYTES
            || self.max_name_bytes > placement::MAX_NAME_BYTES
            || b.max_primitives > bih::MAX_PRIMITIVES
            || b.max_tree_words > bih::MAX_TREE_WORDS
            || b.max_bytes > bih::MAX_BYTES
            || b.max_memory_bytes > bih::MAX_MEMORY_BYTES
            || b.max_work > bih::MAX_WORK
            || b.max_depth > bih::MAX_DEPTH
        {
            return Err(Error::InvalidLimits);
        }
        Ok(())
    }
    fn codec(self, bytes: usize) -> placement::Limits {
        placement::Limits {
            max_bytes: bytes.min(placement::MAX_BYTES),
            max_records: self.max_spawns,
            max_name_bytes: self.max_name_bytes,
        }
    }
    fn tree(self, resident: usize) -> Result<bih::Limits, Error> {
        Ok(bih::Limits {
            max_memory_bytes: self.bih.max_memory_bytes.min(
                self.max_memory_bytes
                    .checked_sub(resident)
                    .ok_or(Error::Limit)?,
            ),
            ..self.bih
        })
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidLimits,
    Limit,
    Allocation,
    MissingBounds,
    Coordinates,
    Framing,
    Order,
    Index,
    Association,
    Placement(placement::Error),
    Bih(bih::Error),
}
impl Error {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidLimits => "assembly_invalid_limits",
            Self::Limit => "assembly_policy_limit",
            Self::Allocation => "assembly_allocation",
            Self::MissingBounds => "assembly_missing_bounds",
            Self::Coordinates => "assembly_tile_coordinates",
            Self::Framing => "assembly_framing",
            Self::Order => "assembly_order",
            Self::Index => "assembly_index",
            Self::Association => "assembly_association",
            Self::Placement(_) => "assembly_placement",
            Self::Bih(_) => "assembly_bih",
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        out.write_str(self.code())
    }
}
impl std::error::Error for Error {}
impl From<placement::Error> for Error {
    fn from(e: placement::Error) -> Self {
        Self::Placement(e)
    }
}
impl From<bih::Error> for Error {
    fn from(e: bih::Error) -> Self {
        Self::Bih(e)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    pub input_records: usize,
    pub unique_spawns: usize,
    pub duplicates: usize,
    pub tiles: usize,
    pub associations: usize,
    pub payload_tiles: usize,
    pub parent_only_tiles: usize,
    pub output_bytes: usize,
    pub proof_bytes: usize,
    pub resident_bytes: usize,
    pub peak_bytes: usize,
    pub bih_nodes: usize,
    pub bih_depth: usize,
    pub bih_work: u64,
}
/// Owns all artifacts and the complete ordered dir_bin proof, with no data Debug.
pub struct AssembledMap {
    map_id: u32,
    tree: Vec<u8>,
    ordered: Vec<u8>,
    tiles: Vec<Tile>,
    stats: Stats,
}
impl AssembledMap {
    pub const fn map_id(&self) -> u32 {
        self.map_id
    }
    pub fn tree_bytes(&self) -> &[u8] {
        &self.tree
    }
    pub fn ordered_dir_bin(&self) -> &[u8] {
        &self.ordered
    }
    pub fn tiles(&self) -> &[Tile] {
        &self.tiles
    }
    pub const fn stats(&self) -> Stats {
        self.stats
    }
}
pub struct Tile {
    x: u16,
    y: u16,
    payload: Option<Vec<u8>>,
    index: Vec<u8>,
}
impl Tile {
    pub const fn x(&self) -> u32 {
        self.x as u32
    }
    pub const fn y(&self) -> u32 {
        self.y as u32
    }
    pub fn payload(&self) -> Option<&[u8]> {
        self.payload.as_deref()
    }
    pub fn index_bytes(&self) -> &[u8] {
        &self.index
    }
}

#[derive(Clone, Copy)]
struct Rect {
    low_x: u16,
    low_y: u16,
    high_x: u16,
    high_y: u16,
}
fn rectangle(p: &placement::Placement<'_>) -> Result<Rect, Error> {
    let b = p.bounds_bits().ok_or(Error::MissingBounds)?;
    bih::Bounds::from_bits(b)?;
    #[allow(clippy::excessive_precision)]
    const INVERSE: f32 = 1.0_f32 / 533.33333_f32;
    let coordinate = |bits: u32, max: f32| -> Result<u16, Error> {
        let value = (f32::from_bits(bits) * INVERSE).trunc();
        if !value.is_finite() || value < 0.0 || value > max {
            return Err(Error::Coordinates);
        }
        Ok(value as u16)
    };
    Ok(Rect {
        low_x: coordinate(b[0], 32767.)?,
        low_y: coordinate(b[1], 255.)?,
        high_x: coordinate(b[3], 32767.)?,
        high_y: coordinate(b[4], 255.)?,
    })
}
impl Rect {
    fn count(self) -> Result<usize, Error> {
        mul(
            usize::from(self.high_x) - usize::from(self.low_x) + 1,
            usize::from(self.high_y) - usize::from(self.low_y) + 1,
        )
    }
}
fn add(a: usize, b: usize) -> Result<usize, Error> {
    a.checked_add(b).ok_or(Error::Limit)
}
fn mul(a: usize, b: usize) -> Result<usize, Error> {
    a.checked_mul(b).ok_or(Error::Limit)
}
fn encoded_len(p: &placement::Placement<'_>) -> Result<usize, Error> {
    add(62, p.name().len())
}
fn word(bytes: &[u8], at: usize) -> Result<u32, Error> {
    Ok(u32::from_le_bytes(
        bytes
            .get(at..add(at, 4)?)
            .ok_or(Error::Framing)?
            .try_into()
            .map_err(|_| Error::Framing)?,
    ))
}
fn push(v: &mut Vec<u8>, value: u32) {
    v.extend_from_slice(&value.to_le_bytes());
}
struct Memory {
    live: usize,
    peak: usize,
    cap: usize,
}
impl Memory {
    fn new(cap: usize) -> Self {
        Self {
            live: 0,
            peak: 0,
            cap,
        }
    }
    fn observe(&mut self, extra: usize) -> Result<(), Error> {
        let total = add(self.live, extra)?;
        if total > self.cap {
            return Err(Error::Limit);
        }
        self.peak = self.peak.max(total);
        Ok(())
    }
    fn reserve<T>(&mut self, v: &mut Vec<T>, capacity: usize) -> Result<(), Error> {
        if capacity <= v.capacity() {
            return Ok(());
        }
        self.observe(mul(capacity, size_of::<T>())?)?;
        let old = mul(v.capacity(), size_of::<T>())?;
        v.try_reserve_exact(capacity - v.len())
            .map_err(|_| Error::Allocation)?;
        let actual = mul(v.capacity(), size_of::<T>())?;
        self.observe(actual)?;
        self.live = self.live - old + actual;
        Ok(())
    }
    fn adopt<T>(&mut self, v: &Vec<T>) -> Result<(), Error> {
        let actual = mul(v.capacity(), size_of::<T>())?;
        self.observe(actual)?;
        self.live = add(self.live, actual)?;
        Ok(())
    }
    fn release<T>(&mut self, v: Vec<T>) {
        self.live -= v.capacity() * size_of::<T>();
    }
}
