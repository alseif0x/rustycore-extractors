// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original Rust implementation from pinned format and
// operation facts; no upstream implementation is copied or imported.
//! Pure complete GameObject model lists and their all-vertex bounds.
//!
//! Pin 02245dcd245e7433e524577656177723d3e4992e, TileAssembler.cpp405-466:
//! temporary rows follow `VMAP04E\0`; final rows follow `VMAP_4.E` and append
//! six f32 bounds. Physical row order and duplicate display IDs/names survive.
//! Bounds merge every vertex of every RAW group, including unreferenced
//! vertices. Declared group bounds, triangles and liquids do not contribute.
//! New vertices win equal extrema, including signed-zero ties.
//!
//! Inputs are explicitly resolved M2/WMO bytes. No lookup, path normalization,
//! acquisition, DB2, CASC or I/O occurs. Names are opaque ASCII 1..499 bytes
//! without NUL; callers that use them as paths must apply their own policy.
//! Each row supplies its own model independently. A caller that represents a
//! native file inventory must additionally bind every repeated name to the same
//! immutable class/content; this codec does not validate that inventory binding.
//! Readers require full framing/EOF and finite ordered bounds. Construction
//! reuses the existing strict RAW readers and rejects the entire operation on
//! absent, malformed or vertex-empty models, unlike native row omission or
//! partial-success behavior. There is no originating-build/runtime claim.

#[cfg(test)]
mod tests;

pub const TEMP_MAGIC: &[u8; 8] = b"VMAP04E\0";
pub const DTREE_MAGIC: &[u8; 8] = b"VMAP_4.E";
pub const MAX_LIST_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_RAW_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_RAW_VISIT_BYTES: u64 = 256 * 1024 * 1024;
pub const MAX_ROWS: usize = 1_000_000;
pub const MAX_NAME_BYTES: usize = 499;
pub const MAX_VERTEX_VISITS: u64 = 128_000_000;
pub const MAX_MEMORY_BYTES: usize = 256 * 1024 * 1024;

/// Only reductions are allowed. List byte ceilings apply to EACH format; the
/// raw ceiling applies per resolved row, in addition to existing M2/WMO caps.
/// Heap covers both returned Vec capacities and temporary row bounds, including
/// allocation overlap. RAW readers/row views borrow geometry without allocating
/// descriptors. Caller inputs/names, fixed stack and allocator overhead are
/// excluded. Raw-byte and projection vertex visits count repeated rows/models
/// again. The RAW readers' validation passes are covered by the raw-byte cap;
/// they are not added to the projection vertex counter.
/// All row/name/dependency lengths and total raw visits are checked before any
/// RAW parsing or allocation. No RSS claim.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub max_list_bytes: usize,
    pub max_raw_bytes: usize,
    pub max_raw_visit_bytes: u64,
    pub max_rows: usize,
    pub max_name_bytes: usize,
    pub max_vertex_visits: u64,
    pub max_memory_bytes: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_list_bytes: MAX_LIST_BYTES,
            max_raw_bytes: MAX_RAW_BYTES,
            max_raw_visit_bytes: MAX_RAW_VISIT_BYTES,
            max_rows: MAX_ROWS,
            max_name_bytes: MAX_NAME_BYTES,
            max_vertex_visits: MAX_VERTEX_VISITS,
            max_memory_bytes: MAX_MEMORY_BYTES,
        }
    }
}
impl Limits {
    fn validate(self) -> Result<(), Error> {
        if self.max_list_bytes > MAX_LIST_BYTES
            || self.max_raw_bytes > MAX_RAW_BYTES
            || self.max_raw_visit_bytes > MAX_RAW_VISIT_BYTES
            || self.max_rows > MAX_ROWS
            || self.max_name_bytes > MAX_NAME_BYTES
            || self.max_vertex_visits > MAX_VERTEX_VISITS
            || self.max_memory_bytes > MAX_MEMORY_BYTES
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
    Framing,
    Name,
    NonFinite,
    InvertedBounds,
    MissingModel,
    EmptyVertices,
    RawM2(crate::Error),
    RawWmo(crate::wmo::Error),
}
impl Error {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidLimits => "gameobject_models_invalid_limits",
            Self::Limit => "gameobject_models_policy_limit",
            Self::Allocation => "gameobject_models_allocation",
            Self::Framing => "gameobject_models_framing",
            Self::Name => "gameobject_models_name",
            Self::NonFinite => "gameobject_models_nonfinite",
            Self::InvertedBounds => "gameobject_models_inverted_bounds",
            Self::MissingModel => "gameobject_models_missing_model",
            Self::EmptyVertices => "gameobject_models_empty_vertices",
            Self::RawM2(_) => "gameobject_models_raw_m2",
            Self::RawWmo(_) => "gameobject_models_raw_wmo",
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}
impl std::error::Error for Error {}

/// Class routing is supplied explicitly; model bytes never implement Debug.
#[derive(Clone, Copy)]
pub enum RawModel<'a> {
    M2(&'a [u8]),
    Wmo(&'a [u8]),
}
/// A missing resolved dependency is explicit and fails the complete build.
#[derive(Clone, Copy)]
pub struct ResolvedRow<'a> {
    pub display_id: u32,
    pub name: &'a [u8],
    pub model: Option<RawModel<'a>>,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    pub rows: usize,
    pub m2_rows: usize,
    pub wmo_rows: usize,
    pub vertex_visits: u64,
    pub raw_visit_bytes: u64,
    pub temp_bytes: usize,
    pub dtree_bytes: usize,
    pub resident_bytes: usize,
    pub peak_bytes: usize,
}
pub struct Lists {
    temp: Vec<u8>,
    dtree: Vec<u8>,
    stats: Stats,
}
impl Lists {
    pub fn temp_bytes(&self) -> &[u8] {
        &self.temp
    }
    pub fn dtree_bytes(&self) -> &[u8] {
        &self.dtree
    }
    pub fn into_bytes(self) -> (Vec<u8>, Vec<u8>) {
        (self.temp, self.dtree)
    }
    pub const fn stats(&self) -> Stats {
        self.stats
    }
}

pub fn build(rows: &[ResolvedRow<'_>], limits: Limits) -> Result<Lists, Error> {
    limits.validate()?;
    if rows.len() > limits.max_rows {
        return Err(Error::Limit);
    }
    let mut temp_size = 8usize;
    let mut dtree_size = 8usize;
    let mut raw_visit_bytes = 0u64;
    for row in rows {
        name(row.name, limits)?;
        let (RawModel::M2(raw) | RawModel::Wmo(raw)) = row.model.ok_or(Error::MissingModel)?;
        if raw.len() > limits.max_raw_bytes {
            return Err(Error::Limit);
        }
        raw_visit_bytes = raw_visit_bytes
            .checked_add(raw.len() as u64)
            .ok_or(Error::Limit)?;
        if raw_visit_bytes > limits.max_raw_visit_bytes {
            return Err(Error::Limit);
        }
        let length = add(8, row.name.len())?;
        temp_size = add(temp_size, length)?;
        dtree_size = add(dtree_size, add(length, 24)?)?;
        if temp_size > limits.max_list_bytes || dtree_size > limits.max_list_bytes {
            return Err(Error::Limit);
        }
    }
    if temp_size > limits.max_list_bytes || dtree_size > limits.max_list_bytes {
        return Err(Error::Limit);
    }
    let mut memory = Memory {
        live: 0,
        peak: 0,
        cap: limits.max_memory_bytes,
    };
    let mut all_bounds = Vec::new();
    memory.reserve(&mut all_bounds, rows.len())?;
    let mut stats = Stats {
        rows: rows.len(),
        temp_bytes: temp_size,
        dtree_bytes: dtree_size,
        raw_visit_bytes,
        ..Stats::default()
    };
    for row in rows {
        let mut bound = None;
        match row.model.ok_or(Error::MissingModel)? {
            RawModel::M2(bytes) => {
                if bytes.len() > limits.max_raw_bytes {
                    return Err(Error::Limit);
                }
                let raw = crate::RawM2::parse(
                    bytes,
                    crate::Limits {
                        max_input_bytes: limits.max_raw_bytes,
                        ..crate::Limits::default()
                    },
                )
                .map_err(Error::RawM2)?;
                charge_visits(&mut stats, raw.vertex_count(), limits)?;
                for index in 0..raw.vertex_count() {
                    merge(&mut bound, raw.vertex_bits(index).ok_or(Error::Framing)?)?;
                }
                stats.m2_rows += 1;
            }
            RawModel::Wmo(bytes) => {
                if bytes.len() > limits.max_raw_bytes {
                    return Err(Error::Limit);
                }
                let raw = crate::wmo::raw::RawWmo::parse(
                    bytes,
                    crate::wmo::raw::ConversionLimits {
                        max_raw_bytes: limits.max_raw_bytes,
                        ..crate::wmo::raw::ConversionLimits::default()
                    },
                )
                .map_err(Error::RawWmo)?;
                for group in raw.groups() {
                    charge_visits(&mut stats, group.vertex_count(), limits)?;
                    for index in 0..group.vertex_count() {
                        merge(&mut bound, group.vertex_bits(index).ok_or(Error::Framing)?)?;
                    }
                }
                stats.wmo_rows += 1;
            }
        }
        all_bounds.push(bound.ok_or(Error::EmptyVertices)?);
    }
    let mut temp = Vec::new();
    memory.reserve(&mut temp, temp_size)?;
    let mut dtree = Vec::new();
    memory.reserve(&mut dtree, dtree_size)?;
    if temp.capacity() > limits.max_list_bytes || dtree.capacity() > limits.max_list_bytes {
        return Err(Error::Limit);
    }
    temp.extend_from_slice(TEMP_MAGIC);
    dtree.extend_from_slice(DTREE_MAGIC);
    for (row, bounds) in rows.iter().zip(&all_bounds) {
        for buffer in [&mut temp, &mut dtree] {
            push(buffer, row.display_id);
            push(buffer, row.name.len() as u32);
            buffer.extend_from_slice(row.name);
        }
        for &value in bounds {
            push(&mut dtree, value);
        }
    }
    if temp.len() != temp_size || dtree.len() != dtree_size {
        return Err(Error::Framing);
    }
    // Borrowed reader revalidation allocates nothing while both outputs and the
    // temporary bound array are still resident.
    TempList::parse(&temp, limits)?;
    Dtree::parse(&dtree, limits)?;
    stats.resident_bytes = add(temp.capacity(), dtree.capacity())?;
    stats.peak_bytes = memory.peak;
    Ok(Lists { temp, dtree, stats })
}
fn charge_visits(stats: &mut Stats, count: usize, limits: Limits) -> Result<(), Error> {
    stats.vertex_visits = stats
        .vertex_visits
        .checked_add(count as u64)
        .ok_or(Error::Limit)?;
    if stats.vertex_visits > limits.max_vertex_visits {
        return Err(Error::Limit);
    }
    Ok(())
}
fn merge(target: &mut Option<[u32; 6]>, vertex: [u32; 3]) -> Result<(), Error> {
    for v in vertex {
        if !f32::from_bits(v).is_finite() {
            return Err(Error::NonFinite);
        }
    }
    if let Some(bound) = target {
        for axis in 0..3 {
            if f32::from_bits(vertex[axis]) <= f32::from_bits(bound[axis]) {
                bound[axis] = vertex[axis];
            }
            if f32::from_bits(vertex[axis]) >= f32::from_bits(bound[axis + 3]) {
                bound[axis + 3] = vertex[axis];
            }
        }
    } else {
        *target = Some([
            vertex[0], vertex[1], vertex[2], vertex[0], vertex[1], vertex[2],
        ]);
    }
    Ok(())
}

/// Fully validated, allocation-free temporary list; rows borrow name bytes.
pub struct TempList<'a> {
    bytes: &'a [u8],
    count: usize,
    limits: Limits,
}
impl<'a> TempList<'a> {
    pub fn parse(bytes: &'a [u8], limits: Limits) -> Result<Self, Error> {
        let count = validate_list(bytes, false, limits)?;
        Ok(Self {
            bytes,
            count,
            limits,
        })
    }
    pub const fn bytes(&self) -> &'a [u8] {
        self.bytes
    }
    pub const fn row_count(&self) -> usize {
        self.count
    }
    pub fn rows(&self) -> TempRows<'a> {
        TempRows {
            inner: Rows {
                rest: &self.bytes[8..],
                remaining: self.count,
                dtree: false,
                limits: self.limits,
            },
        }
    }
}
/// Fully validated final list; names borrow bytes and bounds retain float bits.
pub struct Dtree<'a> {
    bytes: &'a [u8],
    count: usize,
    limits: Limits,
}
impl<'a> Dtree<'a> {
    pub fn parse(bytes: &'a [u8], limits: Limits) -> Result<Self, Error> {
        let count = validate_list(bytes, true, limits)?;
        Ok(Self {
            bytes,
            count,
            limits,
        })
    }
    pub const fn bytes(&self) -> &'a [u8] {
        self.bytes
    }
    pub const fn row_count(&self) -> usize {
        self.count
    }
    pub fn rows(&self) -> DtreeRows<'a> {
        DtreeRows {
            inner: Rows {
                rest: &self.bytes[8..],
                remaining: self.count,
                dtree: true,
                limits: self.limits,
            },
        }
    }
}
#[derive(Clone, Copy)]
pub struct TempRow<'a> {
    pub display_id: u32,
    pub name: &'a [u8],
}
impl<'a> TempRow<'a> {
    pub const fn display_id(&self) -> u32 {
        self.display_id
    }
    pub const fn name(&self) -> &'a [u8] {
        self.name
    }
}
#[derive(Clone, Copy)]
pub struct DtreeRow<'a> {
    pub display_id: u32,
    pub name: &'a [u8],
    pub bounds_bits: [u32; 6],
}
impl<'a> DtreeRow<'a> {
    pub const fn display_id(&self) -> u32 {
        self.display_id
    }
    pub const fn name(&self) -> &'a [u8] {
        self.name
    }
    pub const fn bounds_bits(&self) -> [u32; 6] {
        self.bounds_bits
    }
}
pub struct TempRows<'a> {
    inner: Rows<'a>,
}
pub struct DtreeRows<'a> {
    inner: Rows<'a>,
}
struct Rows<'a> {
    rest: &'a [u8],
    remaining: usize,
    dtree: bool,
    limits: Limits,
}
impl<'a> Rows<'a> {
    fn next(&mut self) -> Option<(TempRow<'a>, Option<[u32; 6]>)> {
        if self.remaining == 0 {
            return None;
        }
        let (row, bounds, used) =
            row(self.rest, self.dtree, self.limits).expect("validated gameobject list");
        self.rest = &self.rest[used..];
        self.remaining -= 1;
        Some((row, bounds))
    }
}
impl<'a> Iterator for TempRows<'a> {
    type Item = TempRow<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(row, _)| row)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.inner.remaining, Some(self.inner.remaining))
    }
}
impl ExactSizeIterator for TempRows<'_> {}
impl std::iter::FusedIterator for TempRows<'_> {}
impl<'a> Iterator for DtreeRows<'a> {
    type Item = DtreeRow<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(row, bounds)| DtreeRow {
            display_id: row.display_id,
            name: row.name,
            bounds_bits: bounds.expect("validated final bounds"),
        })
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.inner.remaining, Some(self.inner.remaining))
    }
}
impl ExactSizeIterator for DtreeRows<'_> {}
impl std::iter::FusedIterator for DtreeRows<'_> {}

fn validate_list(bytes: &[u8], dtree: bool, limits: Limits) -> Result<usize, Error> {
    limits.validate()?;
    if bytes.len() > limits.max_list_bytes {
        return Err(Error::Limit);
    }
    if bytes.get(..8)
        != Some(if dtree {
            DTREE_MAGIC.as_slice()
        } else {
            TEMP_MAGIC.as_slice()
        })
    {
        return Err(Error::Framing);
    }
    let mut rest = &bytes[8..];
    let mut count = 0;
    while !rest.is_empty() {
        if count == limits.max_rows {
            return Err(Error::Limit);
        }
        let (_, _, used) = row(rest, dtree, limits)?;
        rest = &rest[used..];
        count += 1;
    }
    Ok(count)
}
fn row(
    bytes: &[u8],
    dtree: bool,
    limits: Limits,
) -> Result<(TempRow<'_>, Option<[u32; 6]>, usize), Error> {
    let display_id = word(bytes, 0)?;
    let length = word(bytes, 4)? as usize;
    if length > limits.max_name_bytes {
        return Err(Error::Limit);
    }
    let end = add(8, length)?;
    let text = bytes.get(8..end).ok_or(Error::Framing)?;
    name(text, limits)?;
    let (bounds, used) = if dtree {
        let mut bounds = [0; 6];
        for (i, b) in bounds.iter_mut().enumerate() {
            *b = word(bytes, add(end, i * 4)?)?;
        }
        valid_bounds(bounds)?;
        (Some(bounds), add(end, 24)?)
    } else {
        (None, end)
    };
    Ok((
        TempRow {
            display_id,
            name: text,
        },
        bounds,
        used,
    ))
}
fn name(bytes: &[u8], limits: Limits) -> Result<(), Error> {
    if bytes.len() > limits.max_name_bytes {
        return Err(Error::Limit);
    }
    if bytes.is_empty() || bytes.iter().any(|b| *b == 0 || !b.is_ascii()) {
        return Err(Error::Name);
    }
    Ok(())
}
fn valid_bounds(bounds: [u32; 6]) -> Result<(), Error> {
    if bounds.iter().any(|b| !f32::from_bits(*b).is_finite()) {
        return Err(Error::NonFinite);
    }
    for axis in 0..3 {
        if f32::from_bits(bounds[axis]) > f32::from_bits(bounds[axis + 3]) {
            return Err(Error::InvertedBounds);
        }
    }
    Ok(())
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
fn push(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}
fn add(a: usize, b: usize) -> Result<usize, Error> {
    a.checked_add(b).ok_or(Error::Limit)
}
struct Memory {
    live: usize,
    peak: usize,
    cap: usize,
}
impl Memory {
    fn reserve<T>(&mut self, buffer: &mut Vec<T>, count: usize) -> Result<(), Error> {
        let planned = count.checked_mul(size_of::<T>()).ok_or(Error::Limit)?;
        if add(self.live, planned)? > self.cap {
            return Err(Error::Limit);
        }
        buffer
            .try_reserve_exact(count)
            .map_err(|_| Error::Allocation)?;
        let actual = buffer
            .capacity()
            .checked_mul(size_of::<T>())
            .ok_or(Error::Limit)?;
        self.live = add(self.live, actual)?;
        if self.live > self.cap {
            return Err(Error::Limit);
        }
        self.peak = self.peak.max(self.live);
        Ok(())
    }
}
