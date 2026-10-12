// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-12: original borrowed-directory free-space observation.
//! Free space observed on an already open directory. This is not an atomic
//! reservation, a capacity guarantee, a filesystem snapshot or an RSS measure.
//! Callers retain responsibility for directory identity, namespace policy,
//! concurrent consumers, quotas and every subsequent write failure.

use std::fs::File;

#[cfg(all(
    target_arch = "x86_64",
    target_env = "gnu",
    target_pointer_width = "64"
))]
#[allow(unsafe_code)]
mod ffi;

pub const PROFILE: &str = "linux-gnu-x64-directory-available-space-v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Available {
    /// Nonzero filesystem fragment size used for block accounting. This does not
    /// bound all metadata, journal, copy-on-write or directory overhead.
    pub allocation_unit: u64,
    /// Unprivileged available blocks multiplied by the filesystem fragment size.
    pub bytes: u64,
    /// Unprivileged available inodes, absent when the filesystem does not report
    /// a finite total/count. An absent count must not be interpreted as zero.
    pub inodes: Option<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Unsupported,
    FileType,
    Metadata,
    Unavailable,
    Invalid,
    Overflow,
}
impl Error {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Unsupported => "linux_space_unsupported",
            Self::FileType => "linux_space_file_type",
            Self::Metadata => "linux_space_metadata",
            Self::Unavailable => "linux_space_unavailable",
            Self::Invalid => "linux_space_invalid",
            Self::Overflow => "linux_space_overflow",
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
    cfg!(all(
        target_arch = "x86_64",
        target_env = "gnu",
        target_pointer_width = "64"
    ))
}

/// Observe the filesystem containing this borrowed directory descriptor. No
/// pathname is opened and no descriptor flags, locks or ownership are changed.
/// Unsupported targets reject before metadata I/O. On the admitted ABI a
/// non-directory rejects before the filesystem-space syscall.
pub fn available(directory: &File) -> Result<Available, Error> {
    #[cfg(all(
        target_arch = "x86_64",
        target_env = "gnu",
        target_pointer_width = "64"
    ))]
    {
        use std::os::fd::AsRawFd;
        if !directory.metadata().map_err(|_| Error::Metadata)?.is_dir() {
            return Err(Error::FileType);
        }
        let counts = ffi::query(directory.as_raw_fd())?;
        from_counts(counts.0, counts.1, counts.2, counts.3)
    }
    #[cfg(not(all(
        target_arch = "x86_64",
        target_env = "gnu",
        target_pointer_width = "64"
    )))]
    {
        let _ = directory;
        Err(Error::Unsupported)
    }
}

#[cfg(all(
    target_arch = "x86_64",
    target_env = "gnu",
    target_pointer_width = "64"
))]
fn from_counts(
    fragment_size: u64,
    blocks: u64,
    files: u64,
    free_files: u64,
) -> Result<Available, Error> {
    if fragment_size == 0 {
        return Err(Error::Invalid);
    }
    let bytes = blocks.checked_mul(fragment_size).ok_or(Error::Overflow)?;
    let inodes = if files == 0 || files == u64::MAX || free_files == u64::MAX {
        None
    } else if free_files > files {
        return Err(Error::Invalid);
    } else {
        Some(free_files)
    };
    Ok(Available {
        allocation_unit: fragment_size,
        bytes,
        inodes,
    })
}

#[cfg(all(
    test,
    target_arch = "x86_64",
    target_env = "gnu",
    target_pointer_width = "64"
))]
mod tests;
