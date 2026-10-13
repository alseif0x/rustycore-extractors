// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-13: guarded same-volume NTFS caller-available space query.
//! A handle-based observation, not a reservation or a subsequent-write promise.
//! NTFS does not expose a finite Unix-style free-inode count through this API.
use super::ReadDirectory;
#[allow(unsafe_code)]
mod ffi;
#[cfg(test)]
mod tests;

pub const PROFILE: &str = "windows-native-amd64-ntfs-available-space-v1";
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Available {
    pub volume: u32,
    pub allocation_unit: u64,
    pub bytes: u64,
    pub inodes: Option<u64>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Unsupported,
    Changed,
    Unavailable,
    Invalid,
    Overflow,
}
impl Error {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Unsupported => "windows_space_unsupported",
            Self::Changed => "windows_space_changed",
            Self::Unavailable => "windows_space_unavailable",
            Self::Invalid => "windows_space_invalid",
            Self::Overflow => "windows_space_overflow",
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}
impl std::error::Error for Error {}
pub const fn supported() -> bool {
    super::append::supported()
}
/// Borrow the already retained directory. No pathname or administrative volume
/// handle is opened. Directory timestamps may change as our children are made;
/// object identity and the complete ancestor chain are checked before/after.
pub fn available(directory: &ReadDirectory) -> Result<Available, Error> {
    super::append::admit().map_err(|_| Error::Unsupported)?;
    directory.verify_same_object().map_err(|_| Error::Changed)?;
    let result = ffi::query(&directory.inner.leaf.file)
        .and_then(|c| from_counts(directory.identity().volume, c));
    directory.verify_same_object().map_err(|_| Error::Changed)?;
    result
}
fn from_counts(volume: u32, counts: ffi::Counts) -> Result<Available, Error> {
    if counts.total < 0
        || counts.caller < 0
        || counts.actual < 0
        || counts.caller > counts.total
        || counts.caller > counts.actual
        || counts.sectors == 0
        || counts.bytes_per_sector == 0
    {
        return Err(Error::Invalid);
    }
    // Quotas can make total caller units smaller than actual volume availability.
    let allocation_unit = u64::from(counts.sectors)
        .checked_mul(u64::from(counts.bytes_per_sector))
        .ok_or(Error::Overflow)?;
    let bytes = (counts.caller as u64)
        .checked_mul(allocation_unit)
        .ok_or(Error::Overflow)?;
    Ok(Available {
        volume,
        allocation_unit,
        bytes,
        inodes: None,
    })
}
