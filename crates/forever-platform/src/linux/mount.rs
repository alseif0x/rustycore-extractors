// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-16: original descriptor-based conventional mount ID query.
//! GNU/Linux x86_64 LP64 statx fast path, queried anew at every call. Returns
//! the conventional mount ID, never STATX_MNT_ID_UNIQUE. It does not validate
//! object identity, names, bytes or leases; those remain the caller's guards.
//!
//! An unavailable syscall/target, EPERM (including optional-statx seccomp
//! denial), EOPNOTSUPP or a missing returned mount-mask bit permits the caller's
//! existing bounded fallback. A positive ID must still come from that strict
//! reader. Other query errors, zero IDs and invalid responses remain failures.

use std::fs::File;

#[cfg(all(
    target_arch = "x86_64",
    target_env = "gnu",
    target_pointer_width = "64"
))]
#[allow(unsafe_code)]
mod ffi;

#[cfg(any(
    test,
    all(
        target_arch = "x86_64",
        target_env = "gnu",
        target_pointer_width = "64"
    )
))]
const STATX_MNT_ID: u32 = 0x1000;
#[cfg(any(
    test,
    all(
        target_arch = "x86_64",
        target_env = "gnu",
        target_pointer_width = "64"
    )
))]
const STATX_MNT_ID_UNIQUE: u32 = 0x4000;
#[cfg(any(
    test,
    all(
        target_arch = "x86_64",
        target_env = "gnu",
        target_pointer_width = "64"
    )
))]
const ENOSYS: i32 = 38;
#[cfg(any(
    test,
    all(
        target_arch = "x86_64",
        target_env = "gnu",
        target_pointer_width = "64"
    )
))]
const EPERM: i32 = 1;
#[cfg(any(
    test,
    all(
        target_arch = "x86_64",
        target_env = "gnu",
        target_pointer_width = "64"
    )
))]
const EOPNOTSUPP: i32 = 95;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Unsupported,
    Unavailable,
    MissingMask,
    Query,
    Invalid,
}
impl Error {
    /// Compatibility fallback does not hide failed guards or invalid results.
    pub const fn allows_fallback(self) -> bool {
        matches!(
            self,
            Self::Unsupported | Self::Unavailable | Self::MissingMask
        )
    }
    pub const fn code(self) -> &'static str {
        match self {
            Self::Unsupported => "linux_mount_unsupported",
            Self::Unavailable => "linux_mount_unavailable",
            Self::MissingMask => "linux_mount_missing_mask",
            Self::Query => "linux_mount_query",
            Self::Invalid => "linux_mount_invalid",
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}
impl std::error::Error for Error {}

#[cfg(any(
    test,
    all(
        target_arch = "x86_64",
        target_env = "gnu",
        target_pointer_width = "64"
    )
))]
fn response(status: i32, errno: i32, mask: u32, mount: u64) -> Result<u64, Error> {
    match status {
        -1 if errno == ENOSYS => return Err(Error::Unsupported),
        -1 if matches!(errno, EPERM | EOPNOTSUPP) => return Err(Error::Unavailable),
        -1 => return Err(Error::Query),
        0 => {}
        _ => return Err(Error::Invalid),
    }
    // The same field can represent two different ID domains. An unexpected
    // UNIQUE response cannot be used for conventional same-mount comparisons.
    if mask & STATX_MNT_ID_UNIQUE != 0 {
        return Err(Error::Invalid);
    }
    if mask & STATX_MNT_ID == 0 {
        return Err(Error::MissingMask);
    }
    if mount == 0 {
        return Err(Error::Invalid);
    }
    Ok(mount)
}

/// Query the live held description with empty pathname and AT_EMPTY_PATH.
/// No pathname lookup, content read, ID cache, retry or fallback occurs here.
pub fn id(file: &File) -> Result<u64, Error> {
    #[cfg(all(
        target_arch = "x86_64",
        target_env = "gnu",
        target_pointer_width = "64"
    ))]
    {
        use std::os::fd::AsRawFd;
        ffi::id(file.as_raw_fd())
    }
    #[cfg(not(all(
        target_arch = "x86_64",
        target_env = "gnu",
        target_pointer_width = "64"
    )))]
    {
        let _ = file;
        Err(Error::Unsupported)
    }
}

#[cfg(test)]
mod tests;
