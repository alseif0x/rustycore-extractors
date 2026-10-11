// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: shared bounded product fingerprinting.
//! SHA-256 fingerprints are integrity identifiers, not source authentication.
use std::io::Read;

pub fn digest(bytes: &[u8]) -> [u8; 32] {
    let mut hash = crate::sha256::Sha256::new();
    hash.update(bytes);
    hash.finish()
}

/// Constant scratch memory; never reads more than `limit + 1` bytes.
pub fn reader(mut input: impl Read, limit: u64) -> std::io::Result<[u8; 32]> {
    let mut hash = crate::sha256::Sha256::new();
    let mut bytes = [0; 65536];
    let mut remaining = limit;
    loop {
        let count = input.read(&mut bytes[..remaining.min(65535) as usize + 1])?;
        if count == 0 {
            return Ok(hash.finish());
        }
        remaining = remaining.checked_sub(count as u64).ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, "fingerprint_limit")
        })?;
        hash.update(&bytes[..count]);
    }
}

pub fn hex(bytes: &[u8; 32]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn vectors_and_reader_bounds() {
        assert_eq!(
            hex(&digest(b"abc")),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        for size in [0, 1, 65535, 65536, 131073] {
            let bytes = vec![7; size];
            assert_eq!(
                reader(bytes.as_slice(), size as u64).unwrap(),
                digest(&bytes)
            );
            if size != 0 {
                assert!(reader(bytes.as_slice(), size as u64 - 1).is_err());
            }
        }
    }
}
