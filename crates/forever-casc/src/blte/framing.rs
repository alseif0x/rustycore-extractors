// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original key-free complete BLTE framing inspection.
//! Inspect an exact, complete BLTE span without decoding or consulting keys.
//!
//! Success checks every framed encoded MD5, including a zero expected hash,
//! but does not authenticate an encoded key, content key or plaintext. `Z`
//! interiors are not parsed by [`inspect`]. `E` checks only its outer wrapper;
//! ciphertext and its encrypted inner mode remain unvalidated. The stricter
//! encrypted-unavailable inspector additionally checks observable N/Z frames,
//! but still cannot validate encrypted plaintext. No key identifiers, nonces,
//! digest bytes or borrowed payload are returned. Headerless input has no frame
//! hash and is explicitly distinguished from checksummed framing.

use super::{Error, Limits, be32, md5};

/// The hash capability actually present in the supplied object.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Form {
    /// Every encoded frame matched the MD5 declared in its descriptor.
    FramedMd5,
    /// The sole outer frame extends to EOF and has no frame checksum.
    HeaderlessUnhashed,
}

/// Redacted framing evidence, returned only after the entire span succeeds.
///
/// Sizes are declarations, not measured decoded lengths, except that `N`
/// payload length is checked. No field establishes complete plaintext,
/// zlib-stream, ciphertext-interior, encoded-key or content-key validity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Evidence {
    pub form: Form,
    pub encoded_bytes: u64,
    pub expected_content_size: u64,
    /// Sum of table declarations; `None` for the headerless form.
    pub declared_content_size: Option<u64>,
    pub frames: u32,
    pub uncompressed_frames: u32,
    pub zlib_frames: u32,
    pub encrypted_frames: u32,
    pub verified_frame_md5_count: u32,
}

/// Inspect exactly one supplied BLTE object, without allocation or I/O.
///
/// The caller supplies an independently known expected content size and the
/// existing BLTE ceilings. An archive owner must separately admit, retain and
/// read the exact selected span, removing its known 30-byte local prelude.
/// This function never searches, reads a prefix as a fallback, decrypts,
/// decompresses or returns partial evidence on an error.
pub fn inspect(
    encoded: &[u8],
    expected_content_size: u64,
    limits: Limits,
) -> Result<Evidence, Error> {
    let encoded_bytes = u64::try_from(encoded.len()).map_err(|_| Error::Limit)?;
    if encoded_bytes > limits.max_encoded_bytes || expected_content_size > limits.max_decoded_bytes
    {
        return Err(Error::Limit);
    }
    let prefix = encoded.get(..8).ok_or(Error::Truncated)?;
    if &prefix[..4] != b"BLTE" {
        return Err(Error::Header);
    }
    let header_size = usize::try_from(be32(&prefix[4..8])).map_err(|_| Error::Limit)?;
    let mut evidence = Evidence {
        form: Form::HeaderlessUnhashed,
        encoded_bytes,
        expected_content_size,
        declared_content_size: None,
        frames: 0,
        uncompressed_frames: 0,
        zlib_frames: 0,
        encrypted_frames: 0,
        verified_frame_md5_count: 0,
    };
    if header_size == 0 {
        if limits.max_frames == 0 {
            return Err(Error::Limit);
        }
        inspect_frame(&encoded[8..], expected_content_size, &mut evidence)?;
        return Ok(evidence);
    }
    if header_size < 12 {
        return Err(Error::Header);
    }
    let extended = encoded.get(..12).ok_or(Error::Truncated)?;
    if extended[8] != 0x0f {
        return Err(Error::Header);
    }
    let frame_count = u32::from_be_bytes([0, extended[9], extended[10], extended[11]]);
    if frame_count == 0 {
        return Err(Error::Header);
    }
    if frame_count > limits.max_frames {
        return Err(Error::Limit);
    }
    let table_size = usize::try_from(frame_count)
        .map_err(|_| Error::Limit)?
        .checked_mul(24)
        .and_then(|size| size.checked_add(12))
        .ok_or(Error::Limit)?;
    if header_size != table_size {
        return Err(Error::Header);
    }
    let descriptors = encoded.get(12..header_size).ok_or(Error::Truncated)?;
    let mut offset = header_size;
    let mut declared_size = 0_u64;
    for descriptor in descriptors.as_chunks::<24>().0 {
        let frame_size = usize::try_from(be32(&descriptor[..4])).map_err(|_| Error::Limit)?;
        let decoded_size = u64::from(be32(&descriptor[4..8]));
        declared_size = declared_size
            .checked_add(decoded_size)
            .ok_or(Error::Limit)?;
        if declared_size > limits.max_decoded_bytes {
            return Err(Error::Limit);
        }
        let end = offset.checked_add(frame_size).ok_or(Error::Limit)?;
        let frame = encoded.get(offset..end).ok_or(Error::Truncated)?;
        if md5::digest(frame).as_slice() != &descriptor[8..24] {
            return Err(Error::Checksum);
        }
        inspect_frame(frame, decoded_size, &mut evidence)?;
        evidence.verified_frame_md5_count += 1;
        offset = end;
    }
    if offset != encoded.len() || declared_size != expected_content_size {
        return Err(Error::Size);
    }
    evidence.form = Form::FramedMd5;
    evidence.declared_content_size = Some(declared_size);
    Ok(evidence)
}

/// Validate encrypted-unavailable framing against the complete expected EKey.
///
/// Unlike [`inspect`], this entry point verifies the complete encoded EKey and
/// validates every independently decodable N/Z frame. It does not decrypt E
/// frames or authenticate plaintext across encrypted frames.
pub fn inspect_encrypted_unavailable(
    encoded: &[u8],
    expected_content_size: u64,
    expected_ekey: [u8; 16],
    limits: Limits,
) -> Result<Evidence, Error> {
    let encoded_bytes = u64::try_from(encoded.len()).map_err(|_| Error::Limit)?;
    if encoded_bytes > limits.max_encoded_bytes || expected_content_size > limits.max_decoded_bytes
    {
        return Err(Error::Limit);
    }
    let mut scratch = Vec::new();
    inspect_encrypted_unavailable_with_dynamic_scratch(
        encoded,
        expected_content_size,
        expected_ekey,
        limits,
        &mut scratch,
    )
}

pub fn inspect_encrypted_unavailable_with_scratch(
    encoded: &[u8],
    expected_content_size: u64,
    expected_ekey: [u8; 16],
    limits: Limits,
    scratch: &mut [u8],
) -> Result<Evidence, Error> {
    inspect_encrypted_unavailable_inner(
        encoded,
        expected_content_size,
        expected_ekey,
        limits,
        &mut Scratch::Fixed(scratch),
    )
}

pub(crate) fn inspect_encrypted_unavailable_with_dynamic_scratch(
    encoded: &[u8],
    expected_content_size: u64,
    expected_ekey: [u8; 16],
    limits: Limits,
    scratch: &mut Vec<u8>,
) -> Result<Evidence, Error> {
    inspect_encrypted_unavailable_inner(
        encoded,
        expected_content_size,
        expected_ekey,
        limits,
        &mut Scratch::Dynamic(scratch),
    )
}

enum Scratch<'a> {
    Fixed(&'a mut [u8]),
    Dynamic(&'a mut Vec<u8>),
}
impl Scratch<'_> {
    fn decode(&mut self, frame: &[u8], output_size: usize) -> Result<(), Error> {
        match self {
            Self::Fixed(scratch) => {
                if output_size > scratch.len() {
                    return Err(Error::Limit);
                }
                super::decode_frame(frame, &mut scratch[..output_size])
            }
            Self::Dynamic(scratch) => {
                scratch
                    .try_reserve_exact(output_size.saturating_sub(scratch.len()))
                    .map_err(|_| Error::Limit)?;
                scratch.resize(output_size, 0);
                super::decode_frame(frame, &mut scratch[..output_size])
            }
        }
    }
}

fn inspect_encrypted_unavailable_inner(
    encoded: &[u8],
    expected_content_size: u64,
    expected_ekey: [u8; 16],
    limits: Limits,
    scratch: &mut Scratch<'_>,
) -> Result<Evidence, Error> {
    let encoded_bytes = u64::try_from(encoded.len()).map_err(|_| Error::Limit)?;
    if encoded_bytes > limits.max_encoded_bytes || expected_content_size > limits.max_decoded_bytes
    {
        return Err(Error::Limit);
    }
    let prefix = encoded.get(..8).ok_or(Error::Truncated)?;
    if &prefix[..4] != b"BLTE" {
        return Err(Error::Header);
    }
    let header_size = usize::try_from(be32(&prefix[4..8])).map_err(|_| Error::Limit)?;
    let authenticated = if header_size == 0 {
        encoded
    } else {
        encoded.get(..header_size).ok_or(Error::Truncated)?
    };
    if md5::digest(authenticated) != expected_ekey {
        return Err(Error::Checksum);
    }
    let evidence = inspect(encoded, expected_content_size, limits)?;
    if header_size == 0 {
        validate_decodable_frame(&encoded[8..], expected_content_size, scratch)?;
    } else {
        let mut offset = header_size;
        for descriptor in encoded[12..header_size].as_chunks::<24>().0 {
            let frame_size = usize::try_from(be32(&descriptor[..4])).map_err(|_| Error::Limit)?;
            let decoded_size = u64::from(be32(&descriptor[4..8]));
            let end = offset.checked_add(frame_size).ok_or(Error::Limit)?;
            let frame = encoded.get(offset..end).ok_or(Error::Truncated)?;
            validate_decodable_frame(frame, decoded_size, scratch)?;
            offset = end;
        }
    }
    Ok(evidence)
}

fn validate_decodable_frame(
    frame: &[u8],
    declared_size: u64,
    scratch: &mut Scratch<'_>,
) -> Result<(), Error> {
    match frame.first().copied().ok_or(Error::Truncated)? {
        b'E' => encrypted_outer(frame.get(1..).ok_or(Error::EncryptionHeader)?),
        b'N' => {
            if u64::try_from(frame.len() - 1).map_err(|_| Error::Limit)? != declared_size {
                return Err(Error::Size);
            }
            Ok(())
        }
        b'Z' => {
            let output_size = usize::try_from(declared_size).map_err(|_| Error::Limit)?;
            scratch.decode(frame, output_size)
        }
        b'F' => Err(Error::RecursiveUnsupported),
        _ => Err(Error::UnknownMode),
    }
}

fn inspect_frame(frame: &[u8], declared_size: u64, evidence: &mut Evidence) -> Result<(), Error> {
    let (&mode, content) = frame.split_first().ok_or(Error::Truncated)?;
    match mode {
        b'N' => {
            if u64::try_from(content.len()).map_err(|_| Error::Limit)? != declared_size {
                return Err(Error::Size);
            }
            evidence.uncompressed_frames += 1;
        }
        b'Z' => evidence.zlib_frames += 1,
        b'E' => {
            encrypted_outer(content)?;
            evidence.encrypted_frames += 1;
        }
        b'F' => return Err(Error::RecursiveUnsupported),
        _ => return Err(Error::UnknownMode),
    }
    evidence.frames += 1;
    Ok(())
}

fn encrypted_outer(content: &[u8]) -> Result<(), Error> {
    let (&name_size, rest) = content.split_first().ok_or(Error::EncryptionHeader)?;
    if name_size != 0 && name_size != 8 {
        return Err(Error::EncryptionHeader);
    }
    // Skip opaque identifiers and nonce bytes without interpreting or copying.
    let (_, rest) = rest
        .split_at_checked(usize::from(name_size))
        .ok_or(Error::EncryptionHeader)?;
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
    Ok(())
}

#[cfg(test)]
mod tests;
