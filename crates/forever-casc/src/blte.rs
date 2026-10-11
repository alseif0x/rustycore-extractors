// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original bounded Rust BLTE parser and checksum validation.
// Wire-format facts checked against CascLib at TrinityCore commit
// 02245dcd245e7433e524577656177723d3e4992e, CascReadFile.cpp, CascDecrypt.cpp
// and CascStructs.h. Modified 2026-10-11: explicit supplied-key decoding.
// No upstream implementation is copied or translated here.
//! Complete, bounded decoding of uncompressed and zlib BLTE frames.
//!
//! Input starts at the `BLTE` signature and covers exactly one encoded object.
//! A future archive reader must remove its known 30-byte segment prelude; this
//! parser does not search for signatures or accept plain-file fallbacks. The
//! caller supplies the expected content size and independent resource limits.
//! Framed objects require every encoded-frame MD5 to match, including an all-zero
//! hash. An optional content MD5 checks the complete decoded object and supplies
//! integrity for the headerless-frame form, which has no frame hash.
//!
//! `decode` supports `N` and `Z`. The explicit `decode_with_keys` entry point
//! additionally admits one Salsa20 encrypted `E` wrapper around `N` or `Z`,
//! with an independently supplied content MD5 and an explicit keyring. It has
//! no built-in keys or key discovery. Recursive `F` and unknown modes fail.
//! The zlib stream must terminate, consume its entire input,
//! produce exactly its declared output size and pass its Adler32 check. This
//! module neither opens CASC stores nor returns partial data after any error.

pub(crate) mod md5;
pub mod framing;

use miniz_oxide::inflate::{
    TINFLStatus,
    core::{DecompressorOxide, decompress, inflate_flags},
};

/// Bounds supplied by the owner of the encoded object, before decoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    pub max_encoded_bytes: u64,
    pub max_decoded_bytes: u64,
    pub max_frames: u32,
}

/// Path-free errors suitable for structured local reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Header,
    Truncated,
    Size,
    Checksum,
    Limit,
    Compression,
    EncryptedUnsupported,
    EncryptionHeader,
    EncryptionUnsupported,
    MissingKey,
    UnverifiedEncryption,
    RecursiveUnsupported,
    UnknownMode,
}

impl Error {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Header => "blte_invalid_header",
            Self::Truncated => "blte_truncated",
            Self::Size => "blte_size_mismatch",
            Self::Checksum => "blte_checksum_mismatch",
            Self::Limit => "blte_policy_limit",
            Self::Compression => "blte_invalid_zlib",
            Self::EncryptedUnsupported => "blte_encrypted_unsupported",
            Self::EncryptionHeader => "blte_invalid_encryption_header",
            Self::EncryptionUnsupported => "blte_cipher_unsupported",
            Self::MissingKey => "blte_encryption_key_unavailable",
            Self::UnverifiedEncryption => "blte_encryption_requires_content_key",
            Self::RecursiveUnsupported => "blte_recursive_unsupported",
            Self::UnknownMode => "blte_unknown_mode",
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.code())
    }
}

impl std::error::Error for Error {}

/// Decode a complete encoded object, returning bytes only after every check.
///
/// `expected_content_size` comes from the caller's independently parsed content
/// metadata. It is required even when the BLTE object contains frame lengths.
/// `expected_content_md5`, when known, is its content key, not its encoded key.
pub fn decode(
    encoded: &[u8],
    expected_content_size: u64,
    expected_content_md5: Option<[u8; 16]>,
    limits: Limits,
) -> Result<Vec<u8>, Error> {
    decode_impl::<false>(
        encoded,
        expected_content_size,
        expected_content_md5,
        limits,
        None,
    )
}

/// Decode using only explicitly supplied Salsa20 keys.
///
/// Every encrypted frame requires `expected_content_md5`: a successful decode
/// authenticates the complete plaintext against that independent content key.
/// Encoded frame hashes are checked before decryption. Encryption adds at most
/// one fallibly allocated scratch vector, no larger than its encoded frame,
/// at a time; the caller must budget this in addition to input and output.
/// Decrypted wrappers are validated during preflight before output allocation,
/// then decrypted again during decoding. No plaintext is returned on failure.
pub fn decode_with_keys(
    encoded: &[u8],
    expected_content_size: u64,
    expected_content_md5: Option<[u8; 16]>,
    limits: Limits,
    keys: &crate::keys::Keyring,
) -> Result<Vec<u8>, Error> {
    decode_impl::<true>(
        encoded,
        expected_content_size,
        expected_content_md5,
        limits,
        Some(keys),
    )
}

fn decode_impl<const WITH_KEYS: bool>(
    encoded: &[u8],
    expected_content_size: u64,
    expected_content_md5: Option<[u8; 16]>,
    limits: Limits,
    keys: Option<&crate::keys::Keyring>,
) -> Result<Vec<u8>, Error> {
    let encoded_size = u64::try_from(encoded.len()).map_err(|_| Error::Limit)?;
    if encoded_size > limits.max_encoded_bytes || expected_content_size > limits.max_decoded_bytes {
        return Err(Error::Limit);
    }
    let output_size = usize::try_from(expected_content_size).map_err(|_| Error::Limit)?;
    let prefix = encoded.get(..8).ok_or(Error::Truncated)?;
    if &prefix[..4] != b"BLTE" {
        return Err(Error::Header);
    }
    let header_size = usize::try_from(be32(&prefix[4..8])).map_err(|_| Error::Limit)?;

    // Validate the entire object before allocating its output. No descriptor
    // vector is needed: a second pass decodes into the single bounded output.
    let table = if header_size == 0 {
        if limits.max_frames == 0 {
            return Err(Error::Limit);
        }
        preflight::<WITH_KEYS>(
            &encoded[8..],
            expected_content_size,
            None,
            0,
            keys,
            expected_content_md5.is_some(),
        )?;
        None
    } else {
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
        let mut content_size = 0_u64;
        for (index, descriptor) in descriptors.as_chunks::<24>().0.iter().enumerate() {
            let frame_size = usize::try_from(be32(&descriptor[..4])).map_err(|_| Error::Limit)?;
            let decoded_size = u64::from(be32(&descriptor[4..8]));
            content_size = content_size.checked_add(decoded_size).ok_or(Error::Limit)?;
            if content_size > limits.max_decoded_bytes {
                return Err(Error::Limit);
            }
            let end = offset.checked_add(frame_size).ok_or(Error::Limit)?;
            let frame = encoded.get(offset..end).ok_or(Error::Truncated)?;
            let mut digest = [0; 16];
            digest.copy_from_slice(&descriptor[8..24]);
            preflight::<WITH_KEYS>(
                frame,
                decoded_size,
                Some(digest),
                index as u32,
                keys,
                expected_content_md5.is_some(),
            )?;
            offset = end;
        }
        if offset != encoded.len() || content_size != expected_content_size {
            return Err(Error::Size);
        }
        Some(descriptors)
    };

    let mut output = Vec::new();
    output
        .try_reserve_exact(output_size)
        .map_err(|_| Error::Limit)?;
    // Initialized storage is required for safe slice writes. A short decode is
    // an error, so unwritten bytes are never returned as synthetic padding.
    output.resize(output_size, 0);
    if let Some(descriptors) = table {
        let mut offset = header_size;
        let mut output_offset = 0;
        for (index, descriptor) in descriptors.as_chunks::<24>().0.iter().enumerate() {
            // The first pass has checked these conversions, additions and ranges.
            let frame_size = be32(&descriptor[..4]) as usize;
            let end = offset + frame_size;
            let output_end = output_offset + be32(&descriptor[4..8]) as usize;
            decode_selected::<WITH_KEYS>(
                &encoded[offset..end],
                &mut output[output_offset..output_end],
                index as u32,
                keys,
            )?;
            offset = end;
            output_offset = output_end;
        }
    } else {
        decode_selected::<WITH_KEYS>(&encoded[8..], &mut output, 0, keys)?;
    }
    if let Some(expected) = expected_content_md5
        && md5::digest(&output) != expected
    {
        return Err(Error::Checksum);
    }
    Ok(output)
}

fn preflight<const WITH_KEYS: bool>(
    frame: &[u8],
    expected_size: u64,
    digest: Option<[u8; 16]>,
    index: u32,
    keys: Option<&crate::keys::Keyring>,
    has_content_key: bool,
) -> Result<(), Error> {
    // This also authenticates the ciphertext before any key lookup/allocation.
    match validate_frame(frame, expected_size, digest) {
        Err(Error::EncryptedUnsupported) if WITH_KEYS => {
            if !has_content_key {
                return Err(Error::UnverifiedEncryption);
            }
            let inner = decrypt_frame(frame, index, keys.ok_or(Error::MissingKey)?)?;
            validate_frame(&inner, expected_size, None)
        }
        result => result,
    }
}

fn decode_selected<const WITH_KEYS: bool>(
    frame: &[u8],
    output: &mut [u8],
    index: u32,
    keys: Option<&crate::keys::Keyring>,
) -> Result<(), Error> {
    if WITH_KEYS && frame[0] == b'E' {
        let inner = decrypt_frame(frame, index, keys.ok_or(Error::MissingKey)?)?;
        // Preflight already checked this same immutable plaintext's mode/size.
        return decode_frame(&inner, output);
    }
    decode_frame(frame, output)
}

fn decrypt_frame(frame: &[u8], index: u32, keys: &crate::keys::Keyring) -> Result<Vec<u8>, Error> {
    let input = frame.get(1..).ok_or(Error::EncryptionHeader)?;
    let (&name_size, rest) = input.split_first().ok_or(Error::EncryptionHeader)?;
    if name_size != 0 && name_size != 8 {
        return Err(Error::EncryptionHeader);
    }
    let (name, rest) = rest
        .split_at_checked(usize::from(name_size))
        .ok_or(Error::EncryptionHeader)?;
    let name = if name_size == 0 {
        0
    } else {
        u64::from_le_bytes(name.try_into().map_err(|_| Error::EncryptionHeader)?)
    };
    let (&nonce_size, rest) = rest.split_first().ok_or(Error::EncryptionHeader)?;
    if nonce_size != 4 && nonce_size != 8 {
        return Err(Error::EncryptionHeader);
    }
    let (nonce_bytes, rest) = rest
        .split_at_checked(usize::from(nonce_size))
        .ok_or(Error::EncryptionHeader)?;
    let (&cipher, ciphertext) = rest.split_first().ok_or(Error::EncryptionHeader)?;
    if cipher != b'S' {
        return Err(Error::EncryptionUnsupported);
    }
    if ciphertext.is_empty() {
        return Err(Error::EncryptionHeader);
    }
    let key = keys.key(name).ok_or(Error::MissingKey)?;
    let mut nonce = [0_u8; 8];
    nonce[..usize::from(nonce_size)].copy_from_slice(nonce_bytes);
    for (byte, index_byte) in nonce[..4].iter_mut().zip(index.to_le_bytes()) {
        *byte ^= index_byte;
    }
    // No header bytes or additional wrapper copies are retained in scratch.
    let mut plaintext = Vec::new();
    plaintext
        .try_reserve_exact(ciphertext.len())
        .map_err(|_| Error::Limit)?;
    if plaintext.capacity() > frame.len() {
        return Err(Error::Limit);
    }
    plaintext.extend_from_slice(ciphertext);
    crate::keys::xor(key, nonce, &mut plaintext);
    Ok(plaintext)
}

fn validate_frame(frame: &[u8], expected_size: u64, digest: Option<[u8; 16]>) -> Result<(), Error> {
    let (&mode, content) = frame.split_first().ok_or(Error::Truncated)?;
    if let Some(expected) = digest
        && md5::digest(frame) != expected
    {
        return Err(Error::Checksum);
    }
    match mode {
        b'N' if u64::try_from(content.len()).map_err(|_| Error::Limit)? == expected_size => Ok(()),
        b'N' => Err(Error::Size),
        b'Z' => Ok(()),
        b'E' => Err(Error::EncryptedUnsupported),
        b'F' => Err(Error::RecursiveUnsupported),
        _ => Err(Error::UnknownMode),
    }
}

fn decode_frame(frame: &[u8], output: &mut [u8]) -> Result<(), Error> {
    // The first pass validated mode, frame extent and the N output length.
    if frame[0] == b'N' {
        output.copy_from_slice(&frame[1..]);
        return Ok(());
    }
    let input = &frame[1..];
    let flags = inflate_flags::TINFL_FLAG_PARSE_ZLIB_HEADER
        | inflate_flags::TINFL_FLAG_USING_NON_WRAPPING_OUTPUT_BUF;
    let (status, consumed, written) =
        decompress(&mut DecompressorOxide::new(), input, output, 0, flags);
    match status {
        TINFLStatus::Done if consumed == input.len() && written == output.len() => Ok(()),
        TINFLStatus::Done | TINFLStatus::HasMoreOutput => Err(Error::Size),
        TINFLStatus::Adler32Mismatch => Err(Error::Checksum),
        TINFLStatus::FailedCannotMakeProgress | TINFLStatus::NeedsMoreInput => {
            Err(Error::Truncated)
        }
        TINFLStatus::Failed | TINFLStatus::BadParam => Err(Error::Compression),
    }
}

fn be32(bytes: &[u8]) -> u32 {
    u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod encrypted_tests;
