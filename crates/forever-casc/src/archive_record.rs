// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original checked adapter from local index spans to BLTE.
// Format facts: CascStructs.h:217-244, CascReadFile.cpp:174-220/324-330 and
// CascOpenFile.cpp:117-139 at TrinityCore 02245dcd245e7433e524577656177723d3e4992e.
//! Plan an exact local archive range, then decode its complete supplied bytes.
//!
//! This module performs no I/O. The caller supplies the archive's actual file
//! size and must read exactly the returned range. The index encoded size covers
//! the entire local record, including its 30-byte prelude. Raw BLTE objects use
//! [`crate::blte::decode`] directly; this adapter always removes exactly 30 bytes
//! and never scans for a signature or guesses between the two forms.
//!
//! Prelude EKey, size and checksums are deliberately ignored, as in the pinned
//! reader: those fields may be zero or garbage in modern stores. The expected
//! EKey is checked against the index's nine-byte prefix only. This does **not**
//! authenticate the remaining EKey bytes. A complete expected content MD5
//! (CKey) is mandatory and BLTE verifies it after decoding the complete object.
//!
//! A zero-length decoded payload is supported when a complete encoded BLTE
//! record is present. An absent/zero-length encoded span is not a BLTE record;
//! a future store reader must handle any metadata-only empty-file convention
//! separately. No incomplete record yields partial bytes or manufactured zeros.

use crate::{blte, local_index::Entry};

const PRELUDE_SIZE: usize = 30;
const MIN_RECORD_SIZE: u64 = 39; // Prelude + BLTE header + one mode byte.

/// A validated range. Construction is private so decoding cannot receive a
/// plan that skipped the caller's archive extent and encoded-byte budget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpanPlan {
    archive_index: u64,
    offset: u64,
    encoded_size: u64,
    end: u64,
    key_prefix: [u8; 9],
}

impl SpanPlan {
    pub const fn archive_index(self) -> u64 {
        self.archive_index
    }
    pub const fn offset(self) -> u64 {
        self.offset
    }
    /// Includes the 30-byte local archive prelude.
    pub const fn encoded_size(self) -> u64 {
        self.encoded_size
    }
    /// Exclusive end of the range within the actual archive file.
    pub const fn end(self) -> u64 {
        self.end
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Extent,
    Bounds,
    Size,
    Limit,
    KeyPrefix,
    Blte(blte::Error),
}

impl Error {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Extent => "archive_span_invalid_extent",
            Self::Bounds => "archive_span_out_of_bounds",
            Self::Size => "archive_span_size_mismatch",
            Self::Limit => "archive_span_policy_limit",
            Self::KeyPrefix => "archive_ekey_prefix_mismatch",
            Self::Blte(error) => error.code(),
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Blte(error) => Some(error),
            _ => None,
        }
    }
}

/// Check bounds before the caller allocates or reads an archive record.
///
/// `archive_file_size` is the actual local file length, not the index header's
/// segment-size metadata. Archive selection and file handles remain the caller's
/// responsibility. No physical archive-count cap or offset geometry is guessed.
pub fn plan(entry: &Entry, archive_file_size: u64, max_span_bytes: u64) -> Result<SpanPlan, Error> {
    let encoded_size = u64::from(entry.encoded_size);
    if encoded_size > max_span_bytes {
        return Err(Error::Limit);
    }
    if encoded_size < MIN_RECORD_SIZE {
        return Err(Error::Extent);
    }
    let end = entry
        .archive_offset
        .checked_add(encoded_size)
        .ok_or(Error::Bounds)?;
    if end > archive_file_size {
        return Err(Error::Bounds);
    }
    Ok(SpanPlan {
        archive_index: entry.archive_index,
        offset: entry.archive_offset,
        encoded_size,
        end,
        key_prefix: entry.key_prefix,
    })
}

/// Decode exactly the planned local record with mandatory full CKey validation.
///
/// `expected_ekey` selects candidates by its first nine bytes. All matching
/// index entries must remain available to a future store reader; this function
/// validates one supplied candidate and never substitutes another range.
/// `limits.max_encoded_bytes` covers BLTE bytes after the prelude; the complete
/// archive span was already bounded by [`plan`]'s separate `max_span_bytes`.
pub fn decode_span(
    record: &[u8],
    plan: &SpanPlan,
    expected_ekey: [u8; 16],
    expected_ckey: [u8; 16],
    expected_decoded_size: u64,
    limits: blte::Limits,
) -> Result<Vec<u8>, Error> {
    decode_span_inner(
        record,
        plan,
        expected_ekey,
        expected_ckey,
        expected_decoded_size,
        limits,
        None,
    )
}

/// Explicitly supplied-key variant. The original [`decode_span`] continues to
/// reject encrypted frames. Full decoded CKey validation remains mandatory;
/// this performs no discovery, key I/O, fallback or network request.
pub fn decode_span_with_keys(
    record: &[u8],
    plan: &SpanPlan,
    expected_ekey: [u8; 16],
    expected_ckey: [u8; 16],
    expected_decoded_size: u64,
    limits: blte::Limits,
    keys: &crate::keys::Keyring,
) -> Result<Vec<u8>, Error> {
    decode_span_inner(
        record,
        plan,
        expected_ekey,
        expected_ckey,
        expected_decoded_size,
        limits,
        Some(keys),
    )
}

#[allow(clippy::too_many_arguments)]
fn decode_span_inner(
    record: &[u8],
    plan: &SpanPlan,
    expected_ekey: [u8; 16],
    expected_ckey: [u8; 16],
    expected_decoded_size: u64,
    limits: blte::Limits,
    keys: Option<&crate::keys::Keyring>,
) -> Result<Vec<u8>, Error> {
    if u64::try_from(record.len()).map_err(|_| Error::Limit)? != plan.encoded_size {
        return Err(Error::Size);
    }
    if expected_ekey[..9] != plan.key_prefix {
        return Err(Error::KeyPrefix);
    }
    // A private validated plan plus exact record length guarantees this slice.
    // The BLTE parser checks the signature at its start, exactly offset 30 here.
    let encoded = &record[PRELUDE_SIZE..];
    match keys {
        Some(keys) => blte::decode_with_keys(
            encoded,
            expected_decoded_size,
            Some(expected_ckey),
            limits,
            keys,
        ),
        None => blte::decode(encoded, expected_decoded_size, Some(expected_ckey), limits),
    }
    .map_err(Error::Blte)
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod encrypted_tests;
