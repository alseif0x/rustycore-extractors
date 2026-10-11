// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: complete, bounded object placement source projection.
//! Facts: pinned adtfile.cpp:99-193 and wdtfile.cpp:56-137, wdtfile.h:29-67.
//! These readers frame a complete object ADT or the object chunks of a WDT.
//! They preserve physical order, ordinal name lookup, ADT name-table clearing
//! and persistent WDT names. MMID/MWID are opaque and never reinterpret ordinals.
//! WDT MPHD/MAIN/MAID extents are checked when present, without tile planning or
//! requiring metadata unrelated to global placements. Unknown chunks are opaque.
//! Stronger admission than the pin: exact record extents, terminated nonempty
//! ASCII names, valid ordinals and unique WDT metadata. No I/O, extraction,
//! name rewriting, filtering or partial-prefix result is performed.
//! The original bounded string sentinel also admits an unterminated last name
//! and empty entries; this profile rejects both, including unused names.
use crate::placement::{self, Mddf, Modf};
#[cfg(test)]
mod tests;

pub const MAX_INPUT_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_CHUNKS: usize = 4096;
pub const MAX_RECORDS: usize = 1_000_000;
pub const MAX_NAMES: usize = 1_000_000;
pub const MAX_NAME_BYTES: usize = 1024;
pub const MAX_MEMORY_BYTES: usize = 256 * 1024 * 1024;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub max_input_bytes: usize,
    pub max_chunks: usize,
    pub max_records: usize,
    pub max_names: usize,
    pub max_name_bytes: usize,
    pub max_memory_bytes: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_input_bytes: MAX_INPUT_BYTES,
            max_chunks: MAX_CHUNKS,
            max_records: MAX_RECORDS,
            max_names: MAX_NAMES,
            max_name_bytes: MAX_NAME_BYTES,
            max_memory_bytes: MAX_MEMORY_BYTES,
        }
    }
}
impl Limits {
    fn validate(self) -> Result<(), Error> {
        if self.max_input_bytes > MAX_INPUT_BYTES
            || self.max_chunks > MAX_CHUNKS
            || self.max_records > MAX_RECORDS
            || self.max_names > MAX_NAMES
            || self.max_name_bytes > MAX_NAME_BYTES
            || self.max_memory_bytes > MAX_MEMORY_BYTES
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
    Framing,
    Name,
    NameIndex,
    DuplicateMetadata,
    Allocation,
    Source(placement::Error),
}
impl Error {
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidLimits => "objects_invalid_limits",
            Self::Limit => "objects_policy_limit",
            Self::Framing => "objects_framing",
            Self::Name => "objects_name",
            Self::NameIndex => "objects_name_index",
            Self::DuplicateMetadata => "objects_duplicate_metadata",
            Self::Allocation => "objects_allocation",
            Self::Source(e) => e.code(),
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}
impl std::error::Error for Error {}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    M2,
    Wmo,
}
#[derive(Clone, Copy)]
pub enum Target<'a> {
    FileDataId(u32),
    Name(&'a [u8]),
}
#[derive(Clone, Copy)]
pub enum Source<'a> {
    M2(Mddf<'a>),
    Wmo(Modf<'a>),
}
#[derive(Clone, Copy)]
pub struct ModelPlacement<'a> {
    source: Source<'a>,
    target: Target<'a>,
    global: bool,
}
impl<'a> ModelPlacement<'a> {
    pub const fn kind(&self) -> Kind {
        match self.source {
            Source::M2(_) => Kind::M2,
            Source::Wmo(_) => Kind::Wmo,
        }
    }
    pub const fn source(&self) -> Source<'a> {
        self.source
    }
    pub const fn target(&self) -> Target<'a> {
        self.target
    }
    pub const fn global_wmo(&self) -> bool {
        self.global
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stats {
    pub chunks: usize,
    pub records: usize,
    pub names: usize,
    pub resident_bytes: usize,
    pub peak_bytes: usize,
}
/// Source records and names remain borrowed. Only request/name-index vector
/// capacities are charged; source bytes have their separate input ceiling.
pub struct Objects<'a> {
    requests: Vec<ModelPlacement<'a>>,
    stats: Stats,
}
impl<'a> Objects<'a> {
    pub fn parse_adt(bytes: &'a [u8], limits: Limits) -> Result<Self, Error> {
        parse(bytes, limits, false)
    }
    pub fn parse_wdt(bytes: &'a [u8], limits: Limits) -> Result<Self, Error> {
        parse(bytes, limits, true)
    }
    pub fn requests(&self) -> &[ModelPlacement<'a>] {
        &self.requests
    }
    pub const fn stats(&self) -> Stats {
        self.stats
    }
}
fn parse(bytes: &[u8], limits: Limits, wdt: bool) -> Result<Objects<'_>, Error> {
    limits.validate()?;
    if bytes.len() > limits.max_input_bytes {
        return Err(Error::Limit);
    }
    if bytes.is_empty() {
        return Err(Error::Framing);
    }
    let mut requests = Vec::new();
    let mut models = Vec::new();
    let mut wmos = Vec::new();
    let mut stats = Stats {
        chunks: 0,
        records: 0,
        names: 0,
        resident_bytes: 0,
        peak_bytes: 0,
    };
    let mut metadata = 0u8;
    let mut at = 0usize;
    while at < bytes.len() {
        if stats.chunks >= limits.max_chunks {
            return Err(Error::Limit);
        }
        stats.chunks += 1;
        let header = bytes
            .get(at..at.checked_add(8).ok_or(Error::Framing)?)
            .ok_or(Error::Framing)?;
        let tag = [header[3], header[2], header[1], header[0]];
        let length =
            u32::from_le_bytes(header[4..8].try_into().map_err(|_| Error::Framing)?) as usize;
        let start = at + 8;
        let end = start.checked_add(length).ok_or(Error::Framing)?;
        let body = bytes.get(start..end).ok_or(Error::Framing)?;
        at = end;
        match &tag {
            b"MPHD" | b"MAIN" | b"MAID" if wdt => {
                let (bit, size) = match &tag {
                    b"MPHD" => (1, 32),
                    b"MAIN" => (2, 32768),
                    _ => (4, 131072),
                };
                if metadata & bit != 0 {
                    return Err(Error::DuplicateMetadata);
                }
                if body.len() != size {
                    return Err(Error::Framing);
                }
                metadata |= bit;
            }
            b"MMDX" if !wdt => read_names(body, &mut models, limits, &mut stats)?,
            b"MWMO" => read_names(body, &mut wmos, limits, &mut stats)?,
            b"MDDF" if !wdt => {
                if !body.len().is_multiple_of(36) {
                    return Err(Error::Framing);
                }
                let count = body.len() / 36;
                reserve_records(&mut requests, count, limits, &mut stats)?;
                for record in body.as_chunks::<36>().0 {
                    let source =
                        Mddf::parse(record, placement::Limits::default()).map_err(Error::Source)?;
                    let target = resolve(source.source_id(), source.uses_file_data_id(), &models)?;
                    requests.push(ModelPlacement {
                        source: Source::M2(source),
                        target,
                        global: false,
                    });
                }
                if !body.is_empty() {
                    models.clear();
                }
            }
            b"MODF" => {
                if !body.len().is_multiple_of(64) {
                    return Err(Error::Framing);
                }
                let count = body.len() / 64;
                reserve_records(&mut requests, count, limits, &mut stats)?;
                for record in body.as_chunks::<64>().0 {
                    let source =
                        Modf::parse(record, placement::Limits::default()).map_err(Error::Source)?;
                    let target = resolve(source.source_id(), source.uses_file_data_id(), &wmos)?;
                    requests.push(ModelPlacement {
                        source: Source::Wmo(source),
                        target,
                        global: wdt,
                    });
                }
                if !wdt && !body.is_empty() {
                    wmos.clear();
                }
            }
            _ => {}
        }
    }
    stats.resident_bytes = capacity(&requests)?;
    Ok(Objects { requests, stats })
}
fn resolve<'a>(id: u32, fdid: bool, names: &[&'a [u8]]) -> Result<Target<'a>, Error> {
    if fdid {
        Ok(Target::FileDataId(id))
    } else {
        names
            .get(id as usize)
            .copied()
            .map(Target::Name)
            .ok_or(Error::NameIndex)
    }
}
fn read_names<'a>(
    mut bytes: &'a [u8],
    names: &mut Vec<&'a [u8]>,
    limits: Limits,
    stats: &mut Stats,
) -> Result<(), Error> {
    while !bytes.is_empty() {
        if stats.names >= limits.max_names {
            return Err(Error::Limit);
        }
        let end = bytes.iter().position(|v| *v == 0).ok_or(Error::Name)?;
        let name = &bytes[..end];
        if name.len() > limits.max_name_bytes {
            return Err(Error::Limit);
        }
        if name.is_empty() || !name.is_ascii() {
            return Err(Error::Name);
        }
        reserve(names, 1, limits.max_names, limits, stats)?;
        names.push(name);
        stats.names += 1;
        bytes = &bytes[end + 1..];
    }
    Ok(())
}
fn reserve_records(
    requests: &mut Vec<ModelPlacement<'_>>,
    count: usize,
    limits: Limits,
    stats: &mut Stats,
) -> Result<(), Error> {
    let total = stats.records.checked_add(count).ok_or(Error::Limit)?;
    if total > limits.max_records {
        return Err(Error::Limit);
    }
    reserve(requests, count, limits.max_records, limits, stats)?;
    stats.records = total;
    Ok(())
}
fn capacity<T>(v: &Vec<T>) -> Result<usize, Error> {
    v.capacity().checked_mul(size_of::<T>()).ok_or(Error::Limit)
}
fn reserve<T>(
    v: &mut Vec<T>,
    additional: usize,
    count_cap: usize,
    limits: Limits,
    stats: &mut Stats,
) -> Result<(), Error> {
    let required = v.len().checked_add(additional).ok_or(Error::Limit)?;
    if required <= v.capacity() {
        return Ok(());
    }
    if required > count_cap {
        return Err(Error::Limit);
    }
    let next = required
        .max(v.capacity().saturating_mul(2))
        .max(8)
        .min(count_cap);
    let new_bytes = next.checked_mul(size_of::<T>()).ok_or(Error::Limit)?;
    let peak = stats
        .resident_bytes
        .checked_add(new_bytes)
        .ok_or(Error::Limit)?;
    if peak > limits.max_memory_bytes {
        return Err(Error::Limit);
    }
    let old = capacity(v)?;
    v.try_reserve_exact(next - v.len())
        .map_err(|_| Error::Allocation)?;
    let actual = capacity(v)?;
    let peak = stats
        .resident_bytes
        .checked_add(actual)
        .ok_or(Error::Limit)?;
    if peak > limits.max_memory_bytes {
        return Err(Error::Limit);
    }
    stats.peak_bytes = stats.peak_bytes.max(peak);
    stats.resident_bytes = stats.resident_bytes - old + actual;
    Ok(())
}
