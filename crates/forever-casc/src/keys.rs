// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: bounded explicit operator key material; no built-in keys.
//! This module performs no I/O, discovery, download, logging or serialization.
//! Key material is deliberately not Debug/Clone and errors disclose no records.
mod salsa20;
#[cfg(test)]
mod tests;

pub const MAX_BYTES: usize = 256 * 1024;
pub const MAX_ENTRIES: usize = 4096;
const HEADER: &[u8] = b"forever-tact-keys\t1\n";
const RECORD_BYTES: usize = 50;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Limit,
    Format,
    Order,
    Allocation,
}
impl Error {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Limit => "tact_keys_limit",
            Self::Format => "tact_keys_format",
            Self::Order => "tact_keys_order",
            Self::Allocation => "tact_keys_allocation",
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, output: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        output.write_str(self.code())
    }
}
impl std::error::Error for Error {}

struct Entry {
    name: u64,
    key: [u8; 16],
}
/// Exactly supplied 128-bit keys. Ownership does not guarantee secure erasure;
/// callers must keep both input bytes and this value out of reports/model input.
pub struct Keyring {
    entries: Vec<Entry>,
}
impl Keyring {
    /// Canonical ASCII header followed by 1..4096 sorted unique records:
    /// lowercase 16-hex name, tab, lowercase 32-hex key, newline.
    /// Syntax/order validation precedes the single fallible heap reservation.
    pub fn parse(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() > MAX_BYTES {
            return Err(Error::Limit);
        }
        let body = bytes.strip_prefix(HEADER).ok_or(Error::Format)?;
        if body.is_empty() || !body.len().is_multiple_of(RECORD_BYTES) {
            return Err(Error::Format);
        }
        let count = body.len() / RECORD_BYTES;
        if count > MAX_ENTRIES {
            return Err(Error::Limit);
        }
        let mut previous = None;
        for record in body.as_chunks::<RECORD_BYTES>().0 {
            if record[16] != b'\t'
                || record[49] != b'\n'
                || !record[..16]
                    .iter()
                    .chain(&record[17..49])
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte))
            {
                return Err(Error::Format);
            }
            let name = name(&record[..16]);
            if previous.is_some_and(|previous| previous >= name) {
                return Err(Error::Order);
            }
            previous = Some(name);
        }
        let mut entries = Vec::new();
        entries
            .try_reserve_exact(count)
            .map_err(|_| Error::Allocation)?;
        if entries.capacity() > MAX_ENTRIES {
            return Err(Error::Limit);
        }
        for record in body.as_chunks::<RECORD_BYTES>().0 {
            let mut key = [0; 16];
            for (output, pair) in key.iter_mut().zip(record[17..49].as_chunks::<2>().0) {
                *output = (nibble(pair[0]) << 4) | nibble(pair[1]);
            }
            entries.push(Entry {
                name: name(&record[..16]),
                key,
            });
        }
        Ok(Self { entries })
    }
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    /// Actual owned heap capacity; the caller-owned struct/input are separate.
    pub fn resident_bytes(&self) -> usize {
        self.entries.capacity() * std::mem::size_of::<Entry>()
    }
    pub(crate) fn key(&self, name: u64) -> Option<&[u8; 16]> {
        self.entries
            .binary_search_by_key(&name, |entry| entry.name)
            .ok()
            .map(|index| &self.entries[index].key)
    }
}
fn nibble(byte: u8) -> u8 {
    if byte.is_ascii_digit() {
        byte - b'0'
    } else {
        byte - b'a' + 10
    }
}
fn name(hex: &[u8]) -> u64 {
    hex.iter()
        .fold(0, |value, byte| (value << 4) | u64::from(nibble(*byte)))
}

/// Salsa20/20, 128-bit key, 64-bit nonce and initial 64-bit block counter zero.
/// No allocation; only fixed-size word/block scratch. Integrity is the caller's
/// responsibility: the stream cipher itself does not authenticate plaintext.
pub(crate) fn xor(key: &[u8; 16], nonce: [u8; 8], bytes: &mut [u8]) {
    salsa20::xor(key, nonce, bytes);
}
