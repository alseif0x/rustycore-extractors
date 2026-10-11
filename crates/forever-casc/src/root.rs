// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original bounded WoW ROOT parser and FDID selection.
// Format/selection facts: CascRootFile_WoW.cpp:39-99,171-367,432-546 and
// common/FileTree.cpp:393-421 at TrinityCore commit
// 02245dcd245e7433e524577656177723d3e4992e. No upstream implementation imported.
//! Authenticate a complete WoW ROOT and resolve FDIDs under pinned locale rules.
//!
//! Layout is explicit: invalid headers never fall back to another format.
//! Modern headers require a declared size of at least 20 bytes, bounded by the
//! input and caller budget. Extension bytes are retained without interpretation.
//! Complete empty headers/groups and groups ending exactly at EOF are accepted;
//! this does not reproduce the native autodetector's `>= end` boundary quirks or
//! its clamp of undersized modern headers to four bytes. No tail is discarded.
//!
//! Every group's arrays and FDID deltas are checked, including groups that
//! selection later excludes. FDID addition and increment are checked instead
//! of wrapping; `0xffffffff` is rejected as the native invalid-ID sentinel.
//! Header totals are retained and named <= total is enforced, as in the pin;
//! they are not used as extents or assumed to equal the parsed row counts.
//! Two passes validate/count before reserving bounded block and row vectors.
//!
//! Selection uses enUS=0x2 or esES=0x80, accepting neutral locale zero or an
//! intersecting mask. Blocks with 0x100 or 0x80 content flags are excluded.
//! Bit 31 zero is processed before bit 31 one, preserving physical block/row
//! order within each pass. No architecture flag preference, locale-specific
//! preference over neutral, or esES-to-enUS fallback is added.
//!
//! `resolve` returns the first candidate whose full CKey is in the caller's
//! central CKey map (ENCODING plus any admitted build-config entries). Presence
//! must not mean availability of complete local archive bytes: a selected
//! incomplete file must fail reading, not fall through to a later ROOT row.
//! All rows and duplicate FDID candidates remain available for inspection.
//! Name hashes are retained but never require a listfile. No I/O occurs here.

use crate::{Locale, blte::md5};
use std::ops::Range;

pub mod name;

const NO_NAME_HASH: u32 = 0x10000000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layout {
    Headerless18125,
    Header30080,
    Header50893V1,
    Header50893V2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    pub max_bytes: u64,
    pub max_header_bytes: usize,
    pub max_blocks: usize,
    pub max_entries: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header<'a> {
    pub layout: Layout,
    pub total_files: Option<u32>,
    pub files_with_name_hash: Option<u32>,
    /// Authenticated modern-header bytes beyond the reviewed 20-byte prefix.
    pub extension: &'a [u8],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentFlags {
    Legacy(u32),
    Modern {
        flags1: u32,
        flags2: u32,
        flags3: u8,
    },
}

impl ContentFlags {
    pub const fn effective(self) -> u32 {
        match self {
            Self::Legacy(flags) => flags,
            Self::Modern {
                flags1,
                flags2,
                flags3,
            } => flags1 | flags2 | ((flags3 as u32) << 17),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    pub locale_flags: u32,
    pub content_flags: ContentFlags,
    /// Indices in `Root::entries`, including empty blocks and duplicate FDIDs.
    pub entries: Range<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entry {
    pub file_data_id: u32,
    pub content_key: [u8; 16],
    /// None means the array is absent; Some(0) preserves a stored zero hash.
    pub file_name_hash: Option<u64>,
    pub block_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Root<'a> {
    pub header: Header<'a>,
    blocks: Vec<Block>,
    entries: Vec<Entry>,
}

impl Root<'_> {
    pub(crate) fn into_parts(self) -> (Vec<Block>, Vec<Entry>) {
        (self.blocks, self.entries)
    }

    pub fn blocks(&self) -> &[Block] {
        &self.blocks
    }

    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    pub fn candidates(&self, file_data_id: u32, locale: Locale) -> impl Iterator<Item = &Entry> {
        let mask = match locale {
            Locale::EnUs => 0x2,
            Locale::EsEs => 0x80,
        };
        [0_u32, 1].into_iter().flat_map(move |audio| {
            self.blocks
                .iter()
                .filter(move |block| {
                    let flags = block.content_flags.effective();
                    flags & (0x100 | 0x80) == 0
                        && flags >> 31 == audio
                        && (block.locale_flags == 0 || block.locale_flags & mask != 0)
                })
                .flat_map(move |block| self.entries[block.entries.clone()].iter())
                .filter(move |entry| entry.file_data_id == file_data_id)
        })
    }

    pub fn resolve(
        &self,
        file_data_id: u32,
        locale: Locale,
        mut contains_central_ckey: impl FnMut([u8; 16]) -> bool,
    ) -> Option<&Entry> {
        self.candidates(file_data_id, locale)
            .find(|entry| contains_central_ckey(entry.content_key))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Limit,
    Checksum,
    Truncated,
    Header,
    Unsupported,
    FileDataId,
}

impl Error {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Limit => "root_policy_limit",
            Self::Checksum => "root_content_checksum_mismatch",
            Self::Truncated => "root_truncated",
            Self::Header => "root_invalid_header",
            Self::Unsupported => "root_unsupported_layout",
            Self::FileDataId => "root_invalid_file_data_id",
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for Error {}

pub fn parse_root(
    bytes: &[u8],
    expected_ckey: [u8; 16],
    layout: Layout,
    limits: Limits,
) -> Result<Root<'_>, Error> {
    if u64::try_from(bytes.len()).map_err(|_| Error::Limit)? > limits.max_bytes {
        return Err(Error::Limit);
    }
    if md5::digest(bytes) != expected_ckey {
        return Err(Error::Checksum);
    }
    let (header, body) = header(bytes, layout, limits)?;
    let (block_count, entry_count) = walk(body, layout, limits, |_| {}, |_| {})?;
    let mut blocks = Vec::new();
    let mut entries = Vec::new();
    blocks
        .try_reserve_exact(block_count)
        .map_err(|_| Error::Limit)?;
    entries
        .try_reserve_exact(entry_count)
        .map_err(|_| Error::Limit)?;
    walk(
        body,
        layout,
        limits,
        |block| blocks.push(block),
        |entry| entries.push(entry),
    )?;
    Ok(Root {
        header,
        blocks,
        entries,
    })
}

fn header(bytes: &[u8], layout: Layout, limits: Limits) -> Result<(Header<'_>, &[u8]), Error> {
    if layout == Layout::Headerless18125 {
        return Ok((
            Header {
                layout,
                total_files: None,
                files_with_name_hash: None,
                extension: &bytes[..0],
            },
            bytes,
        ));
    }
    let known_size = if layout == Layout::Header30080 {
        12
    } else {
        20
    };
    if known_size > limits.max_header_bytes {
        return Err(Error::Limit);
    }
    let prefix = bytes.get(..known_size).ok_or(Error::Truncated)?;
    if &prefix[..4] != b"TSFM" {
        return Err(Error::Header);
    }
    let (size, total, named) = if layout == Layout::Header30080 {
        (12, le32(&prefix[4..8]), le32(&prefix[8..12]))
    } else {
        let version = if layout == Layout::Header50893V1 {
            1
        } else {
            2
        };
        if le32(&prefix[8..12]) != version {
            return Err(Error::Unsupported);
        }
        let size = usize::try_from(le32(&prefix[4..8])).map_err(|_| Error::Limit)?;
        if size < 20 {
            return Err(Error::Header);
        }
        if size > limits.max_header_bytes {
            return Err(Error::Limit);
        }
        (size, le32(&prefix[12..16]), le32(&prefix[16..20]))
    };
    if named > total {
        return Err(Error::Header);
    }
    let extension = bytes.get(known_size..size).ok_or(Error::Truncated)?;
    let body = bytes.get(size..).ok_or(Error::Truncated)?;
    Ok((
        Header {
            layout,
            total_files: Some(total),
            files_with_name_hash: Some(named),
            extension,
        },
        body,
    ))
}

fn walk(
    bytes: &[u8],
    layout: Layout,
    limits: Limits,
    mut on_block: impl FnMut(Block),
    mut on_entry: impl FnMut(Entry),
) -> Result<(usize, usize), Error> {
    let mut offset = 0_usize;
    let mut blocks = 0_usize;
    let mut entries = 0_usize;
    while offset < bytes.len() {
        if blocks >= limits.max_blocks {
            return Err(Error::Limit);
        }
        let modern = layout == Layout::Header50893V2;
        let group_header = take(bytes, &mut offset, if modern { 17 } else { 12 })?;
        let count = usize::try_from(le32(&group_header[..4])).map_err(|_| Error::Limit)?;
        let end_entries = entries.checked_add(count).ok_or(Error::Limit)?;
        if end_entries > limits.max_entries {
            return Err(Error::Limit);
        }
        let (content_flags, locale_flags) = if modern {
            (
                ContentFlags::Modern {
                    flags1: le32(&group_header[8..12]),
                    flags2: le32(&group_header[12..16]),
                    flags3: group_header[16],
                },
                le32(&group_header[4..8]),
            )
        } else {
            (
                ContentFlags::Legacy(le32(&group_header[4..8])),
                le32(&group_header[8..12]),
            )
        };
        let deltas = take_array(bytes, &mut offset, count, 4)?;
        let legacy_entries = layout == Layout::Headerless18125;
        let key_width = if legacy_entries { 24 } else { 16 };
        let keys = take_array(bytes, &mut offset, count, key_width)?;
        let hashes = if !legacy_entries && content_flags.effective() & NO_NAME_HASH == 0 {
            Some(take_array(bytes, &mut offset, count, 8)?)
        } else {
            None
        };
        on_block(Block {
            locale_flags,
            content_flags,
            entries: entries..end_entries,
        });
        let mut next_id = 0_u32;
        for (index, delta) in deltas.as_chunks::<4>().0.iter().enumerate() {
            let file_data_id = next_id
                .checked_add(u32::from_le_bytes(*delta))
                .ok_or(Error::FileDataId)?;
            next_id = file_data_id.checked_add(1).ok_or(Error::FileDataId)?;
            let key_start = index * key_width;
            let mut content_key = [0; 16];
            content_key.copy_from_slice(&keys[key_start..key_start + 16]);
            let file_name_hash = if legacy_entries {
                Some(le64(&keys[key_start + 16..key_start + 24]))
            } else {
                hashes.map(|hashes| le64(&hashes[index * 8..index * 8 + 8]))
            };
            on_entry(Entry {
                file_data_id,
                content_key,
                file_name_hash,
                block_index: blocks,
            });
        }
        entries = end_entries;
        blocks = blocks.checked_add(1).ok_or(Error::Limit)?;
    }
    Ok((blocks, entries))
}

fn take_array<'a>(
    bytes: &'a [u8],
    offset: &mut usize,
    count: usize,
    width: usize,
) -> Result<&'a [u8], Error> {
    take(bytes, offset, count.checked_mul(width).ok_or(Error::Limit)?)
}

fn take<'a>(bytes: &'a [u8], offset: &mut usize, size: usize) -> Result<&'a [u8], Error> {
    let end = offset.checked_add(size).ok_or(Error::Limit)?;
    let region = bytes.get(*offset..end).ok_or(Error::Truncated)?;
    *offset = end;
    Ok(region)
}

fn le32(bytes: &[u8]) -> u32 {
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

fn le64(bytes: &[u8]) -> u64 {
    u64::from_le_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
    ])
}

#[cfg(test)]
mod tests;
