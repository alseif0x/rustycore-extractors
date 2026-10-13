// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original Rust implementation from pinned BIH operation
// and wire-format facts; no upstream C++ or G3D implementation is imported.
//! Fresh-instance BIH construction and strict complete little-endian storage.
//! Reference: 02245dcd245e7433e524577656177723d3e4992e, common/Collision/
//! BoundingIntervalHierarchy.{h,cpp}, G3D AABox/Vector3/g3dmath helpers.
//! The reference attributes its BIH algorithm to Sunflow, copyright 2003-2007
//! Christopher Kulla, MIT/X11; TrinityCore's implementation is GPL-2.0-or-later.
//!
//! Source float operation order, partition swaps, Z/Y/X axis tie precedence,
//! 1.3f BVH2 criterion and 1e-5f fuzzy equality are retained. Inputs must be finite
//! ordered boxes. Finite inputs may overflow intermediate sums/extents; valid
//! resulting trees remain admitted. A separate work limit bounds retries.
//! Only leaf sizes 1 and 3 are reviewed. Every build starts with empty bounds;
//! native reuse of one BIH instance can accumulate previous bounds and is outside
//! this API. The Linux IEEE f32 profile emits 0x7fc00000 for the empty bounds.
//!
//! The reader is stricter than native fread: full EOF, bounds, reachable graph,
//! leaf ranges and an exact object permutation are validated before exposure.
//! Dead +/-infinity edges are not traversed or mistaken for allocated children.
//! The separate query module traverses validated views. Runtime loading and
//! geometry acquisition are outside this storage/constructor API.
mod build;
mod owned;
pub(crate) use owned::Layout;
pub use owned::Owned;
#[cfg(test)]
mod tests;
mod validate;

pub const MAX_PRIMITIVES: usize = 1_000_000;
pub const MAX_TREE_WORDS: usize = 16_000_000;
pub const MAX_BYTES: usize = 128 * 1024 * 1024;
pub const MAX_MEMORY_BYTES: usize = 256 * 1024 * 1024;
pub const MAX_WORK: u64 = 128_000_000;
/// Native stops on entry at depth >= 64. An inserted unary node can increase
/// depth inside the parent call, allowing its immediately terminal child at 65.
pub const MAX_DEPTH: usize = 65;
const EMPTY_BITS: u32 = 0x7fc0_0000;
const LEAF: u32 = 0xc000_0000;
const OFFSET_MASK: u32 = 0x1fff_ffff;
const BVH2: u32 = 0x2000_0000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub max_primitives: usize,
    pub max_tree_words: usize,
    pub max_bytes: usize,
    pub max_memory_bytes: usize,
    /// Constructor subdivision calls, partition retries and primitive visits.
    /// Initial bounds merging and reader validation are separately bounded by
    /// primitive/tree limits; this counter does not charge those linear passes.
    pub max_work: u64,
    pub max_depth: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_primitives: MAX_PRIMITIVES,
            max_tree_words: MAX_TREE_WORDS,
            max_bytes: MAX_BYTES,
            max_memory_bytes: MAX_MEMORY_BYTES,
            max_work: MAX_WORK,
            max_depth: MAX_DEPTH,
        }
    }
}
impl Limits {
    fn validate(self) -> Result<(), Error> {
        if self.max_primitives > MAX_PRIMITIVES
            || self.max_tree_words > MAX_TREE_WORDS
            || self.max_bytes > MAX_BYTES
            || self.max_memory_bytes > MAX_MEMORY_BYTES
            || self.max_work > MAX_WORK
            || self.max_depth > MAX_DEPTH
        {
            return Err(Error::InvalidLimits);
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidLimits,
    LeafSize,
    Limit,
    WorkLimit,
    Allocation,
    NonFinite,
    Bounds,
    Partition,
    Framing,
    PrimitiveCount,
    Graph,
    Clip,
    Range,
    Permutation,
    Empty,
}
impl Error {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidLimits => "bih_invalid_limits",
            Self::LeafSize => "bih_leaf_size",
            Self::Limit => "bih_policy_limit",
            Self::WorkLimit => "bih_work_limit",
            Self::Allocation => "bih_allocation",
            Self::NonFinite => "bih_nonfinite_bounds",
            Self::Bounds => "bih_inverted_bounds",
            Self::Partition => "bih_invalid_partition",
            Self::Framing => "bih_framing",
            Self::PrimitiveCount => "bih_primitive_count",
            Self::Graph => "bih_invalid_graph",
            Self::Clip => "bih_invalid_clip",
            Self::Range => "bih_invalid_leaf_range",
            Self::Permutation => "bih_object_permutation",
            Self::Empty => "bih_empty_encoding",
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}
impl std::error::Error for Error {}

/// Validated primitive bounds, low XYZ then high XYZ, retaining exact f32 bits.
/// No Debug implementation exposes geometry values.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Bounds {
    bits: [u32; 6],
}
impl Bounds {
    pub fn from_bits(bits: [u32; 6]) -> Result<Self, Error> {
        if bits.iter().any(|v| !f32::from_bits(*v).is_finite()) {
            return Err(Error::NonFinite);
        }
        for axis in 0..3 {
            if f32::from_bits(bits[axis]) > f32::from_bits(bits[axis + 3]) {
                return Err(Error::Bounds);
            }
        }
        Ok(Self { bits })
    }
    pub const fn bits(self) -> [u32; 6] {
        self.bits
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stats {
    pub primitive_count: usize,
    pub tree_words: usize,
    pub node_count: usize,
    pub max_depth: usize,
    pub build_work: u64,
    pub resident_bytes: usize,
    pub peak_bytes: usize,
}
/// Owns just the complete validated serialized tree. Construction accounts real
/// vector capacities and old/new growth overlap, including encoding overlap and
/// validation scratch. Borrowed input boxes, stack and allocator overhead are
/// excluded. Maximum recursion/validation stack depth is fixed at 65.
pub struct Bih {
    bytes: Vec<u8>,
    stats: Stats,
}
impl Bih {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
    pub const fn stats(&self) -> Stats {
        self.stats
    }
}
pub fn build(primitives: &[Bounds], leaf_size: u32, limits: Limits) -> Result<Bih, Error> {
    build::build(primitives, leaf_size, limits)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ViewStats {
    pub primitive_count: usize,
    pub tree_words: usize,
    pub node_count: usize,
    pub max_depth: usize,
    pub validation_peak_bytes: usize,
}
/// A complete borrowed view. Validation scratch is bounded and released before
/// this value is returned; source bytes are not copied or included in that cap.
#[derive(Clone, Copy)]
pub struct View<'a> {
    bytes: &'a [u8],
    bounds: [u32; 6],
    tree: &'a [u8],
    objects: &'a [u8],
    stats: ViewStats,
}
impl<'a> View<'a> {
    pub fn parse(
        bytes: &'a [u8],
        expected_primitives: usize,
        limits: Limits,
    ) -> Result<Self, Error> {
        validate::parse(bytes, expected_primitives, limits)
    }
    /// Validate the complete tree using its declared primitive count. This
    /// establishes structure/permutation, not binding to external geometry.
    pub fn parse_intrinsic(bytes: &'a [u8], limits: Limits) -> Result<Self, Error> {
        Self::parse(bytes, intrinsic_count(bytes, limits)?, limits)
    }
    pub const fn bytes(&self) -> &'a [u8] {
        self.bytes
    }
    pub const fn bounds_bits(&self) -> [u32; 6] {
        self.bounds
    }
    pub const fn stats(&self) -> ViewStats {
        self.stats
    }
    pub fn tree_words(&self) -> impl ExactSizeIterator<Item = u32> + '_ {
        self.tree
            .as_chunks::<4>()
            .0
            .iter()
            .map(|word| u32::from_le_bytes(*word))
    }
    pub fn objects(&self) -> impl ExactSizeIterator<Item = u32> + '_ {
        self.objects
            .as_chunks::<4>()
            .0
            .iter()
            .map(|word| u32::from_le_bytes(*word))
    }
    // Checked constant-time access for query traversal; no allocation, iterator
    // rescanning, changed validation or format/constructor behavior.
    pub(crate) fn query_word(&self, index: usize) -> Option<u32> {
        word(self.tree, index.checked_mul(4)?).ok()
    }
    pub(crate) fn query_object(&self, index: usize) -> Option<u32> {
        word(self.objects, index.checked_mul(4)?).ok()
    }
}
fn intrinsic_count(bytes: &[u8], limits: Limits) -> Result<usize, Error> {
    limits.validate()?;
    if bytes.len() > limits.max_bytes {
        return Err(Error::Limit);
    }
    let words = word(bytes, 24)? as usize;
    if words > limits.max_tree_words {
        return Err(Error::Limit);
    }
    let at = words
        .checked_mul(4)
        .and_then(|v| v.checked_add(28))
        .ok_or(Error::Framing)?;
    let count = word(bytes, at)? as usize;
    if count > limits.max_primitives {
        return Err(Error::Limit);
    }
    Ok(count)
}
fn word(bytes: &[u8], at: usize) -> Result<u32, Error> {
    Ok(u32::from_le_bytes(
        bytes
            .get(at..at.checked_add(4).ok_or(Error::Framing)?)
            .ok_or(Error::Framing)?
            .try_into()
            .map_err(|_| Error::Framing)?,
    ))
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
    fn check(&self, extra: usize) -> Result<usize, Error> {
        self.live
            .checked_add(extra)
            .filter(|total| *total <= self.cap)
            .ok_or(Error::Limit)
    }
    fn reserve<T>(&mut self, values: &mut Vec<T>, capacity: usize) -> Result<(), Error> {
        if capacity <= values.capacity() {
            return Ok(());
        }
        self.check(capacity.checked_mul(size_of::<T>()).ok_or(Error::Limit)?)?;
        let old = values
            .capacity()
            .checked_mul(size_of::<T>())
            .ok_or(Error::Limit)?;
        values
            .try_reserve_exact(capacity - values.len())
            .map_err(|_| Error::Allocation)?;
        let actual = values
            .capacity()
            .checked_mul(size_of::<T>())
            .ok_or(Error::Limit)?;
        self.peak = self.peak.max(self.check(actual)?);
        self.live = self.live - old + actual;
        Ok(())
    }
}
