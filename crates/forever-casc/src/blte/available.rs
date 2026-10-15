// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-15. Format fact: wowdev/TACT.Net commit
// 57618c47dc0cfef1bfc0e7585bf335ced7218389,
// TACT.Net/BlockTable/BlockTableStreamWriter.cs:124-133: framed EKey hashes the BLTE header;
// headerless EKey hashes the entire BLTE object. No implementation copied.
//! Explicit disabled-key availability, separate from complete plaintext decoding.
//! Full selected EKey authenticates the descriptor table (or headerless object).
//! Every frame hash/extent and exact EOF are checked before allocation. N/Z
//! frames decode exactly; valid encrypted wrappers become inaccessible holes.
//! An incomplete source never claims full CKey validation or exposes its backing.

use super::{Error, Limits, be32, decode_frame, framing, md5};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RangeState {
    Available,
    EncryptedUnavailable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CoverageRange {
    pub offset: u64,
    pub bytes: u64,
    pub state: RangeState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RangeError {
    Empty,
    Bounds,
    Unavailable,
}

/// No Debug, Deref, AsRef or conversion into a complete decoded Vec.
pub struct AvailableBytes {
    backing: Vec<u8>,
    coverage: Vec<CoverageRange>,
    available: u64,
    full_ckey_verified: bool,
}
impl AvailableBytes {
    pub fn declared_bytes(&self) -> u64 {
        self.backing.len() as u64
    }
    pub const fn available_bytes(&self) -> u64 {
        self.available
    }
    pub fn coverage(&self) -> &[CoverageRange] {
        &self.coverage
    }
    pub const fn full_ckey_verified(&self) -> bool {
        self.full_ckey_verified
    }
    /// Check the entire span before exposing any byte. Adjacent available frames
    /// may be crossed; an encrypted gap, including an empty request, never leaks.
    pub fn range(&self, offset: u64, bytes: usize) -> Result<&[u8], RangeError> {
        if bytes == 0 {
            return Err(RangeError::Empty);
        }
        let end = offset.checked_add(bytes as u64).ok_or(RangeError::Bounds)?;
        if end > self.declared_bytes() {
            return Err(RangeError::Bounds);
        }
        if self.coverage.iter().any(|r| {
            r.state == RangeState::EncryptedUnavailable
                && r.offset < end
                && offset < r.offset + r.bytes
        }) {
            return Err(RangeError::Unavailable);
        }
        Ok(&self.backing[offset as usize..end as usize])
    }
    #[cfg(target_os = "linux")]
    pub(crate) fn resident_bytes(&self) -> Result<usize, Error> {
        self.coverage
            .capacity()
            .checked_mul(size_of::<CoverageRange>())
            .and_then(|n| n.checked_add(self.backing.capacity()))
            .ok_or(Error::Limit)
    }
    #[cfg(target_os = "linux")]
    pub(crate) fn from_verified(backing: Vec<u8>) -> Result<Self, Error> {
        let bytes = backing.len() as u64;
        let mut coverage = Vec::new();
        coverage.try_reserve_exact(1).map_err(|_| Error::Limit)?;
        coverage.push(CoverageRange {
            offset: 0,
            bytes,
            state: RangeState::Available,
        });
        Ok(Self {
            backing,
            coverage,
            available: bytes,
            full_ckey_verified: true,
        })
    }
}

/// Expected full EKey comes from independently authenticated ENCODING metadata,
/// not the local nine-byte index prefix or the ignored archive prelude.
pub fn decode(
    encoded: &[u8],
    expected_content_size: u64,
    expected_ekey: [u8; 16],
    expected_ckey: [u8; 16],
    limits: Limits,
) -> Result<AvailableBytes, Error> {
    if encoded.len() as u64 > limits.max_encoded_bytes
        || expected_content_size > limits.max_decoded_bytes
    {
        return Err(Error::Limit);
    }
    let prefix = encoded.get(..8).ok_or(Error::Truncated)?;
    if &prefix[..4] != b"BLTE" {
        return Err(Error::Header);
    }
    let header_size = usize::try_from(be32(&prefix[4..])).map_err(|_| Error::Limit)?;
    let hashed = if header_size == 0 {
        encoded
    } else {
        encoded.get(..header_size).ok_or(Error::Truncated)?
    };
    if md5::digest(hashed) != expected_ekey {
        return Err(Error::Checksum);
    }
    let evidence = framing::inspect(encoded, expected_content_size, limits)?;
    let frames = usize::try_from(evidence.frames).map_err(|_| Error::Limit)?;
    let output_size = usize::try_from(expected_content_size).map_err(|_| Error::Limit)?;
    let mut coverage = Vec::new();
    coverage
        .try_reserve_exact(frames)
        .map_err(|_| Error::Limit)?;
    if coverage.capacity() > limits.max_frames as usize {
        return Err(Error::Limit);
    }
    let mut backing = Vec::new();
    backing
        .try_reserve_exact(output_size)
        .map_err(|_| Error::Limit)?;
    if backing.capacity() as u64 > limits.max_decoded_bytes {
        return Err(Error::Limit);
    }
    // These bytes remain private; range() will never expose a hole's zeros.
    backing.resize(output_size, 0);
    let mut result = AvailableBytes {
        backing,
        coverage,
        available: 0,
        full_ckey_verified: false,
    };
    if header_size == 0 {
        decode_one(&encoded[8..], 0, output_size, &mut result)?;
    } else {
        let mut encoded_offset = header_size;
        let mut output_offset = 0;
        for descriptor in encoded[12..header_size].as_chunks::<24>().0 {
            let frame_size = be32(&descriptor[..4]) as usize;
            let decoded_size = be32(&descriptor[4..8]) as usize;
            decode_one(
                &encoded[encoded_offset..encoded_offset + frame_size],
                output_offset,
                decoded_size,
                &mut result,
            )?;
            encoded_offset += frame_size;
            output_offset += decoded_size;
        }
    }
    if evidence.encrypted_frames == 0 {
        if md5::digest(&result.backing) != expected_ckey {
            return Err(Error::Checksum);
        }
        result.full_ckey_verified = true;
    }
    Ok(result)
}

fn decode_one(
    frame: &[u8],
    offset: usize,
    bytes: usize,
    output: &mut AvailableBytes,
) -> Result<(), Error> {
    let state = if frame[0] == b'E' {
        RangeState::EncryptedUnavailable
    } else {
        decode_frame(frame, &mut output.backing[offset..offset + bytes])?;
        output.available += bytes as u64;
        RangeState::Available
    };
    output.coverage.push(CoverageRange {
        offset: offset as u64,
        bytes: bytes as u64,
        state,
    });
    Ok(())
}

#[cfg(test)]
mod tests;
