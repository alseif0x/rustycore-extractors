// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-12: original bounded exclusive file-description lease setup.
//! Explicit GNU/Linux x86_64 primitive, not a filesystem snapshot. The caller
//! retains the same File and must independently check names, metadata and bytes.
//! Duplicates of that file description share its lease; callers needing private
//! content must create and retain a fresh, unshared description. No sysctl,
//! process signal disposition or caller signal mask is changed.

use std::fs::File;

#[cfg(all(target_arch = "x86_64", target_env = "gnu"))]
#[allow(unsafe_code)]
mod ffi;

pub const PROFILE: &str = "linux-gnu-x64-exclusive-file-lease-v1";
/// Requested helper stack. This does not measure allocator overhead or RSS.
pub const HELPER_STACK_BYTES: usize = 128 * 1024;
/// Conservative caller admission reservation for transient helper work. Not RSS.
pub const HELPER_RESERVATION_BYTES: usize = 256 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Unsupported,
    Unavailable,
    Allocation,
    Changed,
}
impl Error {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Unsupported => "linux_lease_unsupported",
            Self::Unavailable => "linux_lease_unavailable",
            Self::Allocation => "linux_lease_allocation",
            Self::Changed => "linux_lease_changed",
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
    cfg!(all(target_arch = "x86_64", target_env = "gnu"))
}

/// Establish a kernel write lease on a fresh description. A targeted helper
/// blocks SIGIO before assigning ownership and exits without unblocking pending
/// thread signals. Ownership is cleared before returning. Failed admission never
/// falls back to an advisory lock. Keep the File alive for the entire lease.
pub fn establish(file: &File) -> Result<(), Error> {
    #[cfg(all(target_arch = "x86_64", target_env = "gnu"))]
    {
        use std::os::fd::AsRawFd;
        std::thread::scope(|scope| {
            let handle = std::thread::Builder::new()
                .name("rce-lease-setup".into())
                .stack_size(HELPER_STACK_BYTES)
                .spawn_scoped(scope, || ffi::establish(file.as_raw_fd()))
                .map_err(|_| Error::Allocation)?;
            handle.join().map_err(|_| Error::Unavailable)?
        })?;
        verify(file)
    }
    #[cfg(not(all(target_arch = "x86_64", target_env = "gnu")))]
    {
        let _ = file;
        Err(Error::Unsupported)
    }
}

/// Require the exact exclusive state and no signal recipient. A pending lease
/// break is already a failure, even while its foreign open remains blocked.
pub fn verify(file: &File) -> Result<(), Error> {
    #[cfg(all(target_arch = "x86_64", target_env = "gnu"))]
    {
        use std::os::fd::AsRawFd;
        if ffi::exclusive_without_owner(file.as_raw_fd()) {
            Ok(())
        } else {
            Err(Error::Changed)
        }
    }
    #[cfg(not(all(target_arch = "x86_64", target_env = "gnu")))]
    {
        let _ = file;
        Err(Error::Unsupported)
    }
}

#[cfg(all(test, target_arch = "x86_64", target_env = "gnu"))]
mod tests;
