// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original bounded ENCODING CKey-route projection.
// Format facts: CascStructs.h:116-155 and CascOpenStorage.cpp:383-500 at
// TrinityCore commit 02245dcd245e7433e524577656177723d3e4992e.
// No upstream parser implementation imported.
//! Authenticated CKey-to-EKey routes from the reviewed ENCODING v1 subset.
//!
//! This pure projection checks the MD5 of the complete input against a CKey
//! supplied independently by the caller. It also checks every CKey page's MD5,
//! first key, entry bounds and zero padding before allocating the row vector.
//! Page checksums and padding checks are deliberately stricter than the pinned
//! C++ loader: that loader comments out page MD5 verification and stops at a
//! zero EKey count without checking the remaining page bytes.
//!
//! Each entry has a little-endian u16 EKey count, a big-endian u32 content size,
//! a 16-byte CKey and all of its 16-byte EKeys, as represented by that loader.
//! All rows, duplicate keys and EKey alternatives retain their input order.
//! Lookup compares full CKeys and never collapses nine-byte EKey collisions.
//! A content size of `0xffffffff` retains the pinned unknown-size semantics;
//! zero is a known zero. A future reader must resolve unknown sizes explicitly.
//!
//! ESpec and the tail after CKey pages are retained as opaque borrowed bytes.
//! Their contents participate in the complete-file MD5, but this module does
//! not validate their grammar or interpret them as compression instructions.
//! The pin declares EKey-page metadata and a 25-byte ESpec entry structure but
//! has no EKey-page reader establishing its full layout or trailer boundaries.
//! Consequently the tail is not assumed to be padding, nor checked against an
//! invented EKey-page extent. Declared EKey page counts/sizes still consume the
//! corresponding header budgets. No claim of complete manifest validation is
//! made by a successful CKey-route projection.
//!
//! The reviewed header profile is `EN`, version 1, both key widths 16 and
//! reserved byte 17 zero. A nonempty declared page set must have a nonzero page
//! size. Empty CKey page sets and zero-filled individual pages are supported;
//! every page descriptor's FirstKey must match bytes 6..22 of its page, as in
//! the reference. A completely filled page needs no zero-count terminator.
//! No filesystem, network, decompression or target-admission work occurs here.

use crate::blte::md5;

const HEADER_BYTES: usize = 22;
const DESCRIPTOR_BYTES: usize = 32;
const ENTRY_HEADER_BYTES: usize = 22;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    pub max_bytes: u64,
    /// Sum of declared CKey and EKey page counts, including the opaque EKey set.
    pub max_pages: usize,
    /// Applies separately to both declared page sizes after conversion from KiB.
    pub max_page_bytes: usize,
    pub max_espec_bytes: usize,
    pub max_tail_bytes: usize,
    pub max_entries: usize,
    /// Total number of EKeys across all rows, counting duplicates.
    pub max_encoded_keys: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header {
    pub ckey_page_bytes: usize,
    pub ekey_page_bytes: usize,
    pub ckey_page_count: u32,
    pub ekey_page_count: u32,
    pub espec_bytes: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entry<'a> {
    pub content_key: [u8; 16],
    pub content_size: Option<u32>,
    /// Borrowed complete keys, preserving alternatives and duplicate keys.
    pub encoded_keys: &'a [[u8; 16]],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncodingRoutes<'a> {
    pub header: Header,
    pub entries: Vec<Entry<'a>>,
    /// Authenticated bytes; ESpec grammar is outside this projection.
    pub espec: &'a [u8],
    /// Authenticated bytes after the reviewed CKey area; format is opaque.
    pub tail: &'a [u8],
}

impl<'a> EncodingRoutes<'a> {
    /// Returns every exact full-CKey match, including conflicting-size rows.
    pub fn lookup(&self, content_key: [u8; 16]) -> impl Iterator<Item = &Entry<'a>> {
        self.entries
            .iter()
            .filter(move |entry| entry.content_key == content_key)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Limit,
    Checksum,
    Truncated,
    Header,
    Unsupported,
    PageChecksum,
    FirstKey,
    Padding,
}

impl Error {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Limit => "encoding_policy_limit",
            Self::Checksum => "encoding_content_checksum_mismatch",
            Self::Truncated => "encoding_truncated",
            Self::Header => "encoding_invalid_header",
            Self::Unsupported => "encoding_unsupported_profile",
            Self::PageChecksum => "encoding_page_checksum_mismatch",
            Self::FirstKey => "encoding_first_key_mismatch",
            Self::Padding => "encoding_nonzero_page_padding",
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for Error {}

pub fn parse_ckey_routes(
    bytes: &[u8],
    expected_ckey: [u8; 16],
    limits: Limits,
) -> Result<EncodingRoutes<'_>, Error> {
    if u64::try_from(bytes.len()).map_err(|_| Error::Limit)? > limits.max_bytes {
        return Err(Error::Limit);
    }
    if md5::digest(bytes) != expected_ckey {
        return Err(Error::Checksum);
    }
    let header = read_header(bytes, limits)?;
    let ckey_count = usize::try_from(header.ckey_page_count).map_err(|_| Error::Limit)?;
    let espec_size = usize::try_from(header.espec_bytes).map_err(|_| Error::Limit)?;
    let espec_end = HEADER_BYTES.checked_add(espec_size).ok_or(Error::Limit)?;
    let descriptor_size = ckey_count
        .checked_mul(DESCRIPTOR_BYTES)
        .ok_or(Error::Limit)?;
    let descriptor_end = espec_end.checked_add(descriptor_size).ok_or(Error::Limit)?;
    let pages_size = ckey_count
        .checked_mul(header.ckey_page_bytes)
        .ok_or(Error::Limit)?;
    let pages_end = descriptor_end.checked_add(pages_size).ok_or(Error::Limit)?;
    let espec = bytes.get(HEADER_BYTES..espec_end).ok_or(Error::Truncated)?;
    let descriptors = bytes
        .get(espec_end..descriptor_end)
        .ok_or(Error::Truncated)?;
    let pages = bytes
        .get(descriptor_end..pages_end)
        .ok_or(Error::Truncated)?;
    let tail = &bytes[pages_end..];
    if tail.len() > limits.max_tail_bytes {
        return Err(Error::Limit);
    }

    // Integrity and count pass: no row or key collections allocated yet.
    let mut entry_count = 0_usize;
    let mut key_count = 0_usize;
    if ckey_count != 0 {
        for (descriptor, page) in descriptors
            .as_chunks::<DESCRIPTOR_BYTES>()
            .0
            .iter()
            .zip(pages.chunks_exact(header.ckey_page_bytes))
        {
            if md5::digest(page).as_slice() != &descriptor[16..] {
                return Err(Error::PageChecksum);
            }
            if page[6..22] != descriptor[..16] {
                return Err(Error::FirstKey);
            }
            walk_page(page, |entry| {
                entry_count = entry_count.checked_add(1).ok_or(Error::Limit)?;
                key_count = key_count
                    .checked_add(entry.encoded_keys.len())
                    .ok_or(Error::Limit)?;
                if entry_count > limits.max_entries || key_count > limits.max_encoded_keys {
                    return Err(Error::Limit);
                }
                Ok(())
            })?;
        }
    }

    let mut entries = Vec::new();
    entries
        .try_reserve_exact(entry_count)
        .map_err(|_| Error::Limit)?;
    if ckey_count != 0 {
        for page in pages.chunks_exact(header.ckey_page_bytes) {
            walk_page(page, |entry| {
                entries.push(entry);
                Ok(())
            })?;
        }
    }
    Ok(EncodingRoutes {
        header,
        entries,
        espec,
        tail,
    })
}

fn read_header(bytes: &[u8], limits: Limits) -> Result<Header, Error> {
    let bytes = bytes.get(..HEADER_BYTES).ok_or(Error::Truncated)?;
    if &bytes[..2] != b"EN" {
        return Err(Error::Header);
    }
    if bytes[2..5] != [1, 16, 16] || bytes[17] != 0 {
        return Err(Error::Unsupported);
    }
    let ckey_page_bytes = usize::from(u16::from_be_bytes([bytes[5], bytes[6]]))
        .checked_mul(1024)
        .ok_or(Error::Limit)?;
    let ekey_page_bytes = usize::from(u16::from_be_bytes([bytes[7], bytes[8]]))
        .checked_mul(1024)
        .ok_or(Error::Limit)?;
    let ckey_page_count = be32(&bytes[9..13]);
    let ekey_page_count = be32(&bytes[13..17]);
    let espec_bytes = be32(&bytes[18..22]);
    let pages = usize::try_from(ckey_page_count)
        .map_err(|_| Error::Limit)?
        .checked_add(usize::try_from(ekey_page_count).map_err(|_| Error::Limit)?)
        .ok_or(Error::Limit)?;
    if pages > limits.max_pages
        || ckey_page_bytes > limits.max_page_bytes
        || ekey_page_bytes > limits.max_page_bytes
        || usize::try_from(espec_bytes).map_err(|_| Error::Limit)? > limits.max_espec_bytes
    {
        return Err(Error::Limit);
    }
    if (ckey_page_count != 0 && ckey_page_bytes == 0)
        || (ekey_page_count != 0 && ekey_page_bytes == 0)
    {
        return Err(Error::Header);
    }
    Ok(Header {
        ckey_page_bytes,
        ekey_page_bytes,
        ckey_page_count,
        ekey_page_count,
        espec_bytes,
    })
}

fn walk_page<'a>(
    page: &'a [u8],
    mut consume: impl FnMut(Entry<'a>) -> Result<(), Error>,
) -> Result<(), Error> {
    let mut offset = 0;
    while offset < page.len() {
        let remaining = &page[offset..];
        let count_bytes = remaining.get(..2).ok_or(Error::Truncated)?;
        let key_count = usize::from(u16::from_le_bytes([count_bytes[0], count_bytes[1]]));
        if key_count == 0 {
            return if remaining.iter().all(|&byte| byte == 0) {
                Ok(())
            } else {
                Err(Error::Padding)
            };
        }
        let size = key_count
            .checked_mul(16)
            .and_then(|size| ENTRY_HEADER_BYTES.checked_add(size))
            .ok_or(Error::Limit)?;
        let record = remaining.get(..size).ok_or(Error::Truncated)?;
        let mut content_key = [0; 16];
        content_key.copy_from_slice(&record[6..22]);
        let content_size = be32(&record[2..6]);
        consume(Entry {
            content_key,
            content_size: (content_size != u32::MAX).then_some(content_size),
            encoded_keys: record[ENTRY_HEADER_BYTES..].as_chunks::<16>().0,
        })?;
        offset = offset.checked_add(size).ok_or(Error::Limit)?;
    }
    Ok(())
}

fn be32(bytes: &[u8]) -> u32 {
    u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

#[cfg(test)]
mod tests;
