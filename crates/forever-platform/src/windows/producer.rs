// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original guarded loaded-executable seal acquisition.
// Modified 2026-10-13: explicit MSVC v2 literal and native-AMD64 acquisition gate.

use super::{LinkPolicy, ReadFile};
use crate::path::Limits;
use std::cell::Cell;

#[allow(unsafe_code)]
mod ffi;
#[cfg(test)]
mod tests;

/// Acquisition protocol only; cryptographic PE validation belongs to the caller.
/// The optional MSVC profile has a distinct literal and native-AMD64 gate.
#[cfg(not(all(feature = "producer-msvc-v2", target_env = "msvc")))]
pub const WINDOWS_PRODUCER_PROFILE: &str = "forever-windows-producer-seal-v1";
#[cfg(all(feature = "producer-msvc-v2", target_env = "msvc"))]
pub const WINDOWS_PRODUCER_PROFILE: &str = "forever-windows-producer-seal-msvc-v2";
pub const MAX_PRODUCER_BYTES: usize = 256 * 1024 * 1024;
pub const MAX_PRODUCER_CHUNKS: usize = 4096;
pub const PAYLOAD_BYTES: usize = 64;
#[cfg(not(all(feature = "producer-msvc-v2", target_env = "msvc")))]
pub const MARKER: [u8; 32] = *b"RCE-FOREVER-PE-SEAL-V1\0\0\0\0\0\0\0\0\0\0";
#[cfg(all(feature = "producer-msvc-v2", target_env = "msvc"))]
pub const MARKER: [u8; 32] = *b"RCE-FOREVER-PE-SEAL-V2\0\0\0\0\0\0\0\0\0\0";

/// Static diagnostics, without a filename, raw handle, or OS error string.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Unsealed,
    Module,
    Path,
    Limit,
    Input,
    Changed,
    Allocation,
    Architecture,
}
impl Error {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Unsealed => "producer_unsealed",
            Self::Module => "producer_module",
            Self::Path => "producer_path",
            Self::Limit => "producer_limit",
            Self::Input => "producer_input",
            Self::Changed => "producer_changed",
            Self::Allocation => "producer_allocation",
            Self::Architecture => "producer_architecture",
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}
impl std::error::Error for Error {}

fn platform_error(error: crate::Error) -> Error {
    match error {
        crate::Error::Path => Error::Path,
        crate::Error::Limit => Error::Limit,
        crate::Error::Allocation => Error::Allocation,
        crate::Error::Changed => Error::Changed,
        _ => Error::Input,
    }
}

/// Holds the actual main module's seal and a guarded pathname candidate. This
/// acquisition alone does not authenticate that candidate or its PE layout.
/// No Debug, Clone, raw handle, or pathname accessor is provided.
pub struct LoadedProducer {
    payload: [u8; PAYLOAD_BYTES],
    payload_rva: u32,
    source: ReadFile,
    limits: Limits,
    invalid: Cell<bool>,
}
impl LoadedProducer {
    /// Rejects an all-zero digest before querying the module pathname or opening
    /// any file. The explicit MSVC v2 profile first requires a successful native
    /// AMD64 machine query. Nonzero seals require the caller's strict PE/hash check.
    pub fn capture(limits: Limits) -> Result<Self, Error> {
        let limits = limits.validate().map_err(platform_error)?;
        let snapshot = ffi::snapshot()?;
        let path = ffi::module_path(limits)?;
        // Hardlink names are admitted explicitly: cryptographic association with
        // the loaded seal, not lexical spelling, establishes candidate content.
        let source = ReadFile::open(&path, limits, LinkPolicy::Allow).map_err(platform_error)?;
        let result = Self {
            payload: snapshot.payload,
            payload_rva: snapshot.payload_rva,
            source,
            limits,
            invalid: Cell::new(false),
        };
        result.verify_unchanged()?;
        Ok(result)
    }

    /// Captured live-loaded 64-byte payload, beginning with the 32-byte marker.
    pub fn payload(&self) -> [u8; PAYLOAD_BYTES] {
        self.payload
    }
    /// Actual payload START relative to the main module base, not digest RVA.
    pub fn payload_rva(&self) -> u32 {
        self.payload_rva
    }

    /// Rechecks the loaded payload, its actual module association and RVA, and
    /// the retained file/ancestor identities. Any observed failure poisons this
    /// object permanently; this is not a second cryptographic content check.
    pub fn verify_unchanged(&self) -> Result<(), Error> {
        self.verify_snapshot(ffi::snapshot)
    }

    fn verify_snapshot(
        &self,
        snapshot: impl FnOnce() -> Result<ffi::Snapshot, Error>,
    ) -> Result<(), Error> {
        if self.invalid.get() {
            return Err(Error::Changed);
        }
        let result = (|| {
            let current = snapshot()?;
            if current.payload != self.payload || current.payload_rva != self.payload_rva {
                return Err(Error::Changed);
            }
            self.source.verify_unchanged().map_err(platform_error)
        })();
        if result.is_err() {
            self.invalid.set(true);
        }
        result.map_err(|_| Error::Changed)
    }

    /// Reads the entire captured candidate under one retained lease. The caller
    /// owns the returned allocation and must verify PE structure, measured RVA,
    /// loaded/disk payload equality, and canonical digest before trusting it.
    /// Limits cap total bytes separately from the existing per-operation cap.
    pub fn read_all(&self, max_bytes: usize) -> Result<Vec<u8>, Error> {
        self.verify_unchanged()?;
        let len = admitted_length(self.source.identity().size, max_bytes, self.limits)?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(len)
            .map_err(|_| Error::Allocation)?;
        bytes.resize(len, 0);
        for (index, chunk) in bytes.chunks_mut(self.limits.max_read_bytes).enumerate() {
            let offset = (index * self.limits.max_read_bytes) as u64;
            if self.source.read_exact_at(offset, chunk).is_err() {
                self.invalid.set(true);
                return Err(Error::Changed);
            }
        }
        self.verify_unchanged()?;
        Ok(bytes)
    }
}

fn admitted_length(size: u64, max_bytes: usize, limits: Limits) -> Result<usize, Error> {
    let limits = limits.validate().map_err(platform_error)?;
    if max_bytes > MAX_PRODUCER_BYTES {
        return Err(Error::Limit);
    }
    let len = usize::try_from(size).map_err(|_| Error::Limit)?;
    if len > max_bytes || len.div_ceil(limits.max_read_bytes) > MAX_PRODUCER_CHUNKS {
        return Err(Error::Limit);
    }
    Ok(len)
}

// These checks use only captured values. The FFI gate supplies actual process
// machine values; tests can prove rejection without querying any pathname.
fn admit_payload(payload: &[u8; PAYLOAD_BYTES]) -> Result<(), Error> {
    if payload[..32] != MARKER || payload[32..].iter().all(|byte| *byte == 0) {
        Err(Error::Unsealed)
    } else {
        Ok(())
    }
}
#[cfg(any(test, all(feature = "producer-msvc-v2", target_env = "msvc")))]
fn admit_architecture(
    success: i32,
    process_machine: u16,
    native_machine: u16,
) -> Result<(), Error> {
    if success == 0 || process_machine != 0 || native_machine != 0x8664 {
        Err(Error::Architecture)
    } else {
        Ok(())
    }
}
