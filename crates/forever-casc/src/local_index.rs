// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original bounded parser for local CASC index v7.
// Format facts: CascStructs.h and CascIndexFiles.cpp at TrinityCore commit
// 02245dcd245e7433e524577656177723d3e4992e. No CascLib parser code imported.
//! Checked local index bytes and deterministic generation selection, without I/O.
//!
//! The caller must specify the layout; a failed contiguous parse never falls
//! back to pages. Both pinned contiguous checksum conventions are recognized.
//! A contiguous block must contain complete 18-byte entries; any file tail must
//! be zero padding. Header alignment bytes 24..32 are ignored, as in CascLib.
//!
//! The explicit paged layout requires a zero gap from byte 32 to 4096 followed
//! by one or more complete 512-byte pages. Each 24-byte slot contains a 4-byte
//! hash, an 18-byte entry and two alignment bytes. The hash covers the entry
//! and the first alignment byte; the second alignment byte is ignored. A zero
//! hash terminates a page only when its entire remaining region is zero.
//! Nonzero invalid slots or tails reject the entire index, without a prefix.
//!
//! This intentionally does not reproduce two pinned loader behaviors: its
//! page-end pointer does not advance past the first page, and a bad page hash
//! ends parsing silently. Explicit layout selection also removes the loader's
//! 0x7800-byte autodetection heuristic. These are strict page-format checks,
//! not a claim of general parity with that loader's paged fallback.

pub(crate) mod jenkins;

const HEADER_END: usize = 32;
const PAGE_START: usize = 0x1000;
const PAGE_SIZE: usize = 0x200;
const ENTRY_SIZE: usize = 18;
const SLOT_SIZE: usize = 24;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    pub max_bytes: u64,
    pub max_entries: usize,
    pub max_pages: usize,
}

/// Layout selection is an explicit responsibility of the future store reader.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layout {
    Contiguous,
    Paged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Checksums {
    /// Chained `hashlittle2`, with both previous outputs reused as seeds.
    Blizzard,
    /// Chained `hashlittle`, with the previous output reused as its seed.
    BlizzGet,
    /// Each slot has its own `hashlittle(entry + first padding byte) | 0x80000000`.
    Paged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header {
    pub bucket: u8,
    /// Explicit 1..=40 geometry. Zero does not silently substitute a default.
    pub file_offset_bits: u8,
    /// Retained metadata; physical archive bounds belong to the archive reader.
    pub segment_size: u64,
}

/// An index prefix is not a complete encoded key. Duplicate prefixes and even
/// duplicate entries are retained for subsequent full-key/content validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entry {
    pub key_prefix: [u8; 9],
    pub storage_offset: u64,
    pub archive_index: u64,
    pub archive_offset: u64,
    pub encoded_size: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Index {
    pub header: Header,
    pub checksums: Checksums,
    pub entries: Vec<Entry>,
}

impl Index {
    pub fn matching<'a>(&'a self, prefix: &'a [u8; 9]) -> impl Iterator<Item = &'a Entry> {
        self.entries
            .iter()
            .filter(move |entry| &entry.key_prefix == prefix)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Truncated,
    Header,
    Unsupported,
    Checksum,
    Trailing,
    Limit,
    Filename,
    Ambiguous,
}

impl Error {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Truncated => "casc_index_truncated",
            Self::Header => "casc_index_invalid_header",
            Self::Unsupported => "casc_index_unsupported_layout",
            Self::Checksum => "casc_index_checksum_mismatch",
            Self::Trailing => "casc_index_nonzero_padding",
            Self::Limit => "casc_index_policy_limit",
            Self::Filename => "casc_index_invalid_filename",
            Self::Ambiguous => "casc_index_ambiguous_generation",
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for Error {}

pub fn parse(
    bytes: &[u8],
    expected_bucket: u8,
    layout: Layout,
    limits: Limits,
) -> Result<Index, Error> {
    if u64::try_from(bytes.len()).map_err(|_| Error::Limit)? > limits.max_bytes {
        return Err(Error::Limit);
    }
    let header = header(bytes, expected_bucket)?;
    match layout {
        Layout::Contiguous => contiguous(bytes, header, limits),
        Layout::Paged => paged(bytes, header, limits),
    }
}

fn header(bytes: &[u8], expected_bucket: u8) -> Result<Header, Error> {
    if expected_bucket >= 16 {
        return Err(Error::Header);
    }
    let region = bytes.get(..HEADER_END).ok_or(Error::Truncated)?;
    if le32(&region[..4]) != 16 {
        return Err(Error::Header);
    }
    let fields = &region[8..24];
    if jenkins::hashlittle(fields, 0) != le32(&region[4..8]) {
        return Err(Error::Checksum);
    }
    if u16::from_le_bytes([fields[0], fields[1]]) != 7 {
        return Err(Error::Unsupported);
    }
    if fields[2] != expected_bucket || fields[3] != 0 {
        return Err(Error::Header);
    }
    if fields[4..7] != [4, 5, 9] || !(1..=40).contains(&fields[7]) {
        return Err(Error::Unsupported);
    }
    let mut segment = [0; 8];
    segment.copy_from_slice(&fields[8..16]);
    Ok(Header {
        bucket: fields[2],
        file_offset_bits: fields[7],
        segment_size: u64::from_le_bytes(segment),
    })
}

fn contiguous(bytes: &[u8], header: Header, limits: Limits) -> Result<Index, Error> {
    let guard = bytes
        .get(HEADER_END..HEADER_END + 8)
        .ok_or(Error::Truncated)?;
    let size = usize::try_from(le32(&guard[..4])).map_err(|_| Error::Limit)?;
    if size == 0 || !size.is_multiple_of(ENTRY_SIZE) {
        return Err(Error::Header);
    }
    let count = size / ENTRY_SIZE;
    if count > limits.max_entries {
        return Err(Error::Limit);
    }
    let end = (HEADER_END + 8).checked_add(size).ok_or(Error::Limit)?;
    let entries = bytes.get(HEADER_END + 8..end).ok_or(Error::Truncated)?;
    zero_padding(&bytes[end..])?;
    let mut pair = (0, 0);
    let mut single = 0;
    for entry in entries.as_chunks::<ENTRY_SIZE>().0 {
        pair = jenkins::hashlittle2(entry, pair.0, pair.1);
        single = jenkins::hashlittle(entry, single);
    }
    let expected = le32(&guard[4..8]);
    let checksums = if expected == pair.0 {
        Checksums::Blizzard
    } else if expected == single {
        Checksums::BlizzGet
    } else {
        return Err(Error::Checksum);
    };
    let mut decoded = Vec::new();
    decoded.try_reserve_exact(count).map_err(|_| Error::Limit)?;
    decoded.extend(
        entries
            .as_chunks::<ENTRY_SIZE>()
            .0
            .iter()
            .map(|bytes| entry(bytes, header)),
    );
    Ok(Index {
        header,
        checksums,
        entries: decoded,
    })
}

fn paged(bytes: &[u8], header: Header, limits: Limits) -> Result<Index, Error> {
    let gap = bytes.get(HEADER_END..PAGE_START).ok_or(Error::Truncated)?;
    zero_padding(gap)?;
    let pages = &bytes[PAGE_START..];
    if pages.is_empty() || !pages.len().is_multiple_of(PAGE_SIZE) {
        return Err(Error::Truncated);
    }
    if pages.len() / PAGE_SIZE > limits.max_pages {
        return Err(Error::Limit);
    }
    // Complete integrity and count pass before allocating any entry storage.
    let mut count = 0_usize;
    for page in pages.as_chunks::<PAGE_SIZE>().0 {
        let mut offset = 0;
        while offset + SLOT_SIZE <= PAGE_SIZE {
            let expected = le32(&page[offset..offset + 4]);
            if expected == 0 {
                break;
            }
            let covered = &page[offset + 4..offset + 4 + ENTRY_SIZE + 1];
            if (jenkins::hashlittle(covered, 0) | 0x80000000) != expected {
                return Err(Error::Checksum);
            }
            count = count.checked_add(1).ok_or(Error::Limit)?;
            if count > limits.max_entries {
                return Err(Error::Limit);
            }
            offset += SLOT_SIZE;
        }
        zero_padding(&page[offset..])?;
    }
    let mut entries = Vec::new();
    entries.try_reserve_exact(count).map_err(|_| Error::Limit)?;
    for page in pages.as_chunks::<PAGE_SIZE>().0 {
        for slot in page.as_chunks::<SLOT_SIZE>().0 {
            if le32(&slot[..4]) == 0 {
                break;
            }
            entries.push(entry(&slot[4..4 + ENTRY_SIZE], header));
        }
    }
    Ok(Index {
        header,
        checksums: Checksums::Paged,
        entries,
    })
}

fn entry(bytes: &[u8], header: Header) -> Entry {
    let mut key_prefix = [0; 9];
    key_prefix.copy_from_slice(&bytes[..9]);
    let storage_offset = u64::from_be_bytes([
        0, 0, 0, bytes[9], bytes[10], bytes[11], bytes[12], bytes[13],
    ]);
    let mask = (1_u64 << header.file_offset_bits) - 1;
    Entry {
        key_prefix,
        storage_offset,
        archive_index: storage_offset >> header.file_offset_bits,
        archive_offset: storage_offset & mask,
        // The pinned struct comment says BE; CopyEKeyEntry actually uses LE.
        encoded_size: le32(&bytes[14..18]),
    }
}

fn le32(bytes: &[u8]) -> u32 {
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

fn zero_padding(bytes: &[u8]) -> Result<(), Error> {
    if bytes.iter().any(|byte| *byte != 0) {
        Err(Error::Trailing)
    } else {
        Ok(())
    }
}

/// Parsed basename only. This object never resolves or opens a filesystem path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct IndexFilename {
    pub bucket: u8,
    pub generation: u32,
}

pub fn parse_filename(name: &str) -> Result<IndexFilename, Error> {
    let bytes = name.as_bytes();
    if bytes.len() != 14
        || !bytes[10..].eq_ignore_ascii_case(b".idx")
        || !bytes[..10].iter().all(u8::is_ascii_hexdigit)
    {
        return Err(Error::Filename);
    }
    let bucket = u8::from_str_radix(&name[..2], 16).map_err(|_| Error::Filename)?;
    if bucket >= 16 {
        return Err(Error::Filename);
    }
    let generation = u32::from_str_radix(&name[2..10], 16).map_err(|_| Error::Filename)?;
    Ok(IndexFilename { bucket, generation })
}

/// Select the numerically newest generation independently for every bucket.
/// Duplicate (bucket, generation) candidates reject, including aliases that
/// differ only in filename case. Missing buckets remain `None`.
pub fn select_latest(
    candidates: &[IndexFilename],
    max_candidates: usize,
) -> Result<[Option<IndexFilename>; 16], Error> {
    if candidates.len() > max_candidates {
        return Err(Error::Limit);
    }
    if candidates.iter().any(|candidate| candidate.bucket >= 16) {
        return Err(Error::Filename);
    }
    let mut ordered = Vec::new();
    ordered
        .try_reserve_exact(candidates.len())
        .map_err(|_| Error::Limit)?;
    ordered.extend_from_slice(candidates);
    ordered.sort_unstable();
    if ordered.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(Error::Ambiguous);
    }
    let mut latest = [None; 16];
    for candidate in ordered {
        latest[usize::from(candidate.bucket)] = Some(candidate);
    }
    Ok(latest)
}

#[cfg(test)]
mod tests;
