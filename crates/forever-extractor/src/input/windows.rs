// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: explicit Windows inputs through bounded NTFS read leases.
use super::SuppliedKeys;
use forever_platform::{
    Error,
    path::{Limits, MAX_READ_BYTES},
    windows::{LinkPolicy, ReadFile},
};
use std::path::Path;

/// The input must already be a normal absolute drive path. No canonicalization,
/// verbatim-prefix removal, relative resolution, or reparse traversal is used.
/// Only a read lease is returned internally; ancestors stay held during all I/O.
fn open(path: &Path) -> Result<ReadFile, Error> {
    ReadFile::open(path, Limits::default(), LinkPolicy::Reject)
}

fn key_error(error: Error) -> crate::Error {
    match error {
        Error::FileType => "product_keys_input_type",
        Error::Limit => "product_keys_input_limit",
        Error::Changed => "product_keys_input_changed",
        Error::Allocation => "product_keys_allocation",
        _ => "product_keys_input_io",
    }
    .into()
}

/// Read one explicitly selected complete key file. The opaque return value holds
/// its NTFS lease, including ancestors. Consuming it releases that lease; a future
/// Windows output driver still needs file-identity alias protection of its own.
pub fn keys(path: &Path) -> Result<SuppliedKeys, crate::Error> {
    let lease = open(path).map_err(key_error)?;
    let bytes = lease
        .read_all(forever_casc::keys::MAX_BYTES)
        .map_err(key_error)?;
    let keyring = forever_casc::keys::Keyring::parse(&bytes)
        .map_err(|error| crate::Error::from(error.code()))?;
    lease.verify_unchanged().map_err(key_error)?;
    Ok(SuppliedKeys {
        keyring,
        _source_lease: lease,
    })
}

/// Read a complete explicit plan/identity input under the Windows lease profile.
/// This bounded adapter does not supply Windows loaded-executable identity: the
/// Linux driver uses its unchanged descriptor-based /proc/self/exe path instead.
pub fn bytes(path: &Path, limit: u64) -> Result<Vec<u8>, &'static str> {
    let lease = open(path).map_err(input_error)?;
    let length = lease.identity().size;
    if length > limit {
        return Err("product_input_limit");
    }
    let length = usize::try_from(length).map_err(|_| "product_input_limit")?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(length)
        .map_err(|_| "product_allocation")?;
    bytes.resize(length, 0);
    for (index, chunk) in bytes.chunks_mut(MAX_READ_BYTES).enumerate() {
        let offset = index
            .checked_mul(MAX_READ_BYTES)
            .ok_or("product_input_limit")?;
        lease
            .read_exact_at(offset as u64, chunk)
            .map_err(input_error)?;
    }
    lease.verify_unchanged().map_err(input_error)?;
    Ok(bytes)
}

fn input_error(error: Error) -> &'static str {
    match error {
        Error::FileType => "product_input_type",
        Error::Limit => "product_input_limit",
        Error::Changed => "product_input_changed",
        Error::Allocation => "product_allocation",
        _ => "product_input_io",
    }
}
