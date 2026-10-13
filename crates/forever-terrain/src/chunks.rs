// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original checked framing, not an upstream code port.
//! On-disk tag reversal and payload length facts: pinned
//! src/tools/map_extractor/loadlib.cpp:109-155; loadlib/loadlib.h:31-48.
//! Unlike the C++ scanner, this parses consecutive complete frames and never
//! resynchronizes or recursively scans arbitrary payloads for signatures.

use crate::Error;
use std::ops::Range;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub max_file_bytes: usize,
    pub max_chunks: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_file_bytes: 16 * 1024 * 1024,
            max_chunks: 4096,
        }
    }
}

/// A borrowed frame. No Debug implementation dumps its opaque payload.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Chunk<'a> {
    fourcc: [u8; 4],
    offset: usize,
    payload: &'a [u8],
}

impl<'a> Chunk<'a> {
    /// Logical tag, e.g. MVER. Arbitrary unknown tag bytes are preserved.
    pub const fn fourcc(self) -> [u8; 4] {
        self.fourcc
    }
    /// On-disk tag, e.g. REVM. Input bytes are never changed.
    pub const fn raw_fourcc(self) -> [u8; 4] {
        [
            self.fourcc[3],
            self.fourcc[2],
            self.fourcc[1],
            self.fourcc[0],
        ]
    }
    pub const fn offset(self) -> usize {
        self.offset
    }
    pub const fn payload(self) -> &'a [u8] {
        self.payload
    }
    pub fn range(self) -> Range<usize> {
        // Both additions were checked against the input by parse().
        self.offset..self.offset + 8 + self.payload.len()
    }
}

/// Constructed only after every frame and the final boundary have validated.
pub struct Chunks<'a> {
    bytes: &'a [u8],
    chunks: Vec<Chunk<'a>>,
}

impl<'a> Chunks<'a> {
    pub fn parse(bytes: &'a [u8], limits: Limits) -> Result<Self, Error> {
        if bytes.len() > limits.max_file_bytes {
            return Err(Error::Limit);
        }
        // The first pass validates every length/count without allocating.
        let count = walk(bytes, limits, |_| {})?;
        let mut chunks = Vec::new();
        chunks.try_reserve_exact(count).map_err(|_| Error::Limit)?;
        walk(bytes, limits, |chunk| chunks.push(chunk))?;
        Ok(Self { bytes, chunks })
    }
    pub const fn bytes(&self) -> &'a [u8] {
        self.bytes
    }
    pub fn all(&self) -> &[Chunk<'a>] {
        &self.chunks
    }
    /// Unknown duplicate tags remain available through all(); typed users
    /// explicitly request uniqueness only for the fields they interpret.
    pub fn unique(&self, fourcc: [u8; 4]) -> Result<Option<Chunk<'a>>, Error> {
        let mut matches = self
            .chunks
            .iter()
            .copied()
            .filter(|chunk| chunk.fourcc == fourcc);
        let first = matches.next();
        if matches.next().is_some() {
            return Err(Error::DuplicateChunk);
        }
        Ok(first)
    }
}

fn walk<'a>(
    bytes: &'a [u8],
    limits: Limits,
    mut emit: impl FnMut(Chunk<'a>),
) -> Result<usize, Error> {
    let mut offset = 0usize;
    let mut count = 0usize;
    while offset < bytes.len() {
        if count >= limits.max_chunks {
            return Err(Error::Limit);
        }
        let body = offset.checked_add(8).ok_or(Error::Limit)?;
        let header = bytes.get(offset..body).ok_or(Error::Truncated)?;
        let size = usize::try_from(u32::from_le_bytes([
            header[4], header[5], header[6], header[7],
        ]))
        .map_err(|_| Error::Limit)?;
        let end = body.checked_add(size).ok_or(Error::Limit)?;
        let payload = bytes.get(body..end).ok_or(Error::Truncated)?;
        emit(Chunk {
            fourcc: [header[3], header[2], header[1], header[0]],
            offset,
            payload,
        });
        offset = end;
        count = count.checked_add(1).ok_or(Error::Limit)?;
    }
    Ok(count)
}
