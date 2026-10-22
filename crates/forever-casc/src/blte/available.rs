// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-15. Format fact: wowdev/TACT.Net commit
// 57618c47dc0cfef1bfc0e7585bf335ced7218389,
// TACT.Net/BlockTable/BlockTableStreamWriter.cs:124-133: framed EKey hashes the BLTE header;
// headerless EKey hashes the entire BLTE object. No implementation copied.
//! Explicit disabled-key availability, separate from complete plaintext decoding.
//! Full selected EKey authenticates the descriptor table (or headerless object).
//! Every frame hash/extent and exact EOF are checked before allocation. N/Z
//! frames decode exactly; valid encrypted wrappers become inaccessible holes.
//! The explicit zerofill conversion is reserved for the v25 DB2 Skip policy.

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

/// Redacted facts for one checksum-verified encrypted outer BLTE frame.
/// Lookup is rendered as canonical lowercase 16-digit hexadecimal; nonce and
/// ciphertext bytes are never retained.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PartialSection {
    pub lookup_hex: String,
    pub frame_ordinal: u32,
    pub decoded_size: u64,
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
    partial_sections: Vec<PartialSection>,
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
    pub fn partial_sections(&self) -> &[PartialSection] {
        &self.partial_sections
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
    /// Consume the guarded source and explicitly expose its zero-filled address
    /// space with the matching unavailable-section declarations. Callers must
    /// apply the product-specific declared-skip semantics before publishing it.
    pub fn into_zerofilled(self) -> (Vec<u8>, Vec<PartialSection>) {
        (self.backing, self.partial_sections)
    }
    #[cfg(target_os = "linux")]
    pub(crate) fn resident_bytes(&self) -> Result<usize, Error> {
        let ranges = self
            .coverage
            .capacity()
            .checked_mul(size_of::<CoverageRange>())
            .ok_or(Error::Limit)?;
        let partials = self
            .partial_sections
            .capacity()
            .checked_mul(size_of::<PartialSection>())
            .ok_or(Error::Limit)?;
        let text = self.partial_sections.iter().try_fold(0usize, |n, part| {
            n.checked_add(part.lookup_hex.capacity())
                .ok_or(Error::Limit)
        })?;
        ranges
            .checked_add(partials)
            .and_then(|n| n.checked_add(text))
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
            partial_sections: Vec::new(),
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
    let mut partial_sections = Vec::new();
    partial_sections
        .try_reserve_exact(evidence.encrypted_frames as usize)
        .map_err(|_| Error::Limit)?;
    if partial_sections.capacity() > limits.max_frames as usize {
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
        partial_sections,
        available: 0,
        full_ckey_verified: false,
    };
    if header_size == 0 {
        decode_one(&encoded[8..], 0, output_size, 0, &mut result)?;
    } else {
        let mut encoded_offset = header_size;
        let mut output_offset = 0;
        for (ordinal, descriptor) in encoded[12..header_size]
            .as_chunks::<24>()
            .0
            .iter()
            .enumerate()
        {
            let frame_size = be32(&descriptor[..4]) as usize;
            let decoded_size = be32(&descriptor[4..8]) as usize;
            decode_one(
                &encoded[encoded_offset..encoded_offset + frame_size],
                output_offset,
                decoded_size,
                u32::try_from(ordinal).map_err(|_| Error::Limit)?,
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
    frame_ordinal: u32,
    output: &mut AvailableBytes,
) -> Result<(), Error> {
    let state = if frame[0] == b'E' {
        output
            .partial_sections
            .push(partial_section(frame, frame_ordinal, bytes as u64)?);
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

fn partial_section(
    frame: &[u8],
    frame_ordinal: u32,
    decoded_size: u64,
) -> Result<PartialSection, Error> {
    let content = frame.get(1..).ok_or(Error::EncryptionHeader)?;
    let (&name_size, rest) = content.split_first().ok_or(Error::EncryptionHeader)?;
    if name_size != 0 && name_size != 8 {
        return Err(Error::EncryptionHeader);
    }
    let (name, rest) = rest
        .split_at_checked(usize::from(name_size))
        .ok_or(Error::EncryptionHeader)?;
    let lookup = if name_size == 0 {
        0
    } else {
        u64::from_le_bytes(name.try_into().map_err(|_| Error::EncryptionHeader)?)
    };
    let (&nonce_size, rest) = rest.split_first().ok_or(Error::EncryptionHeader)?;
    if nonce_size != 4 && nonce_size != 8 {
        return Err(Error::EncryptionHeader);
    }
    let (_, rest) = rest
        .split_at_checked(usize::from(nonce_size))
        .ok_or(Error::EncryptionHeader)?;
    let (&cipher, ciphertext) = rest.split_first().ok_or(Error::EncryptionHeader)?;
    if cipher != b'S' {
        return Err(Error::EncryptionUnsupported);
    }
    if ciphertext.is_empty() {
        return Err(Error::EncryptionHeader);
    }
    Ok(PartialSection {
        lookup_hex: format!("{lookup:016x}"),
        frame_ordinal,
        decoded_size,
    })
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod partial_section_tests {
    use super::*;

    const LIMITS: Limits = Limits {
        max_encoded_bytes: 4096,
        max_decoded_bytes: 4096,
        max_frames: 8,
    };

    fn encrypted(lookup: u64, cipher: u8) -> Vec<u8> {
        let mut frame = vec![b'E', 8];
        frame.extend_from_slice(&lookup.to_le_bytes());
        frame.push(4);
        frame.extend_from_slice(&[0x44; 4]);
        frame.push(cipher);
        frame.extend_from_slice(&[0xa5; 16]);
        frame
    }

    fn framed(frames: &[(&[u8], u32)]) -> (Vec<u8>, [u8; 16], u64) {
        let header_size = 12 + frames.len() * 24;
        let mut encoded = b"BLTE".to_vec();
        encoded.extend_from_slice(&(header_size as u32).to_be_bytes());
        encoded.extend_from_slice(&[15, 0, 0, frames.len() as u8]);
        for (frame, decoded_size) in frames {
            encoded.extend_from_slice(&(frame.len() as u32).to_be_bytes());
            encoded.extend_from_slice(&decoded_size.to_be_bytes());
            encoded.extend_from_slice(&md5::digest(frame));
        }
        let ekey = md5::digest(&encoded);
        for (frame, _) in frames {
            encoded.extend_from_slice(frame);
        }
        (
            encoded,
            ekey,
            frames.iter().map(|(_, size)| u64::from(*size)).sum(),
        )
    }

    #[test]
    fn three_frames_keep_redacted_skip_facts_and_zero_fill_only_on_consumption() {
        let encrypted = encrypted(0x1122_3344_5566_7788, b'S');
        let (encoded, ekey, decoded_size) =
            framed(&[(b"Nabc", 3), (b"Ndefg", 4), (&encrypted, 138)]);
        let available = decode(&encoded, decoded_size, ekey, [0; 16], LIMITS).unwrap();
        assert_eq!(available.partial_sections().len(), 1);
        assert_eq!(
            available.partial_sections()[0],
            PartialSection {
                lookup_hex: "1122334455667788".into(),
                frame_ordinal: 2,
                decoded_size: 138,
            }
        );
        assert_eq!(available.range(0, 7), Ok(&b"abcdefg"[..]));
        assert_eq!(available.range(7, 1), Err(RangeError::Unavailable));
        assert!(!available.full_ckey_verified());
        let (bytes, declarations) = available.into_zerofilled();
        assert_eq!(bytes.len(), 145);
        assert_eq!(&bytes[..7], b"abcdefg");
        assert!(bytes[7..].iter().all(|byte| *byte == 0));
        assert_eq!(declarations.len(), 1);
        assert_eq!(declarations[0].decoded_size, 138);
    }

    #[test]
    fn unsupported_encrypted_cipher_is_fatal() {
        let encrypted = encrypted(7, b'X');
        let (encoded, ekey, decoded_size) = framed(&[(&encrypted, 138)]);
        assert!(matches!(
            decode(&encoded, decoded_size, ekey, [0; 16], LIMITS),
            Err(Error::EncryptionUnsupported)
        ));
    }
}
