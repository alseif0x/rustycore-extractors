// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: bounded read-only operator plan/identity inputs.
#[cfg(not(windows))]
use std::{
    fs::{File, OpenOptions},
    io::Read,
    path::{Path, PathBuf},
};

/// Explicit operator input, never included in Debug, serialization or output.
/// Key material is retained only in the owned keyring. Secure erasure of the
/// input buffer or keyring is not guaranteed. Linux retains the canonical source
/// solely to protect that file from the output writer. Windows retains a readonly
/// source lease until this value is consumed or dropped; this is not an alias-safe
/// Windows output protocol, and the Windows product driver remains disabled.
pub struct SuppliedKeys {
    pub(crate) keyring: forever_casc::keys::Keyring,
    #[cfg(not(windows))]
    pub(crate) source: PathBuf,
    #[cfg(windows)]
    _source_lease: forever_platform::windows::ReadFile,
}

impl SuppliedKeys {
    pub fn into_keyring(self) -> forever_casc::keys::Keyring {
        self.keyring
    }
}

/// Read one explicitly named regular key file with a bounded extra-byte probe.
/// No environment variable, client path, parent directory or network is searched.
/// Errors contain static codes only; no path, record, key name or key material.
#[cfg(not(windows))]
pub fn keys(path: &Path) -> Result<SuppliedKeys, crate::Error> {
    let mut file = open(path).map_err(|error| match error {
        "product_input_type" => "product_keys_input_type",
        _ => "product_keys_input_io",
    })?;
    let before = file.metadata().map_err(|_| "product_keys_input_io")?;
    let length = before.len();
    if length > forever_casc::keys::MAX_BYTES as u64 {
        return Err("product_keys_input_limit".into());
    }
    let length = usize::try_from(length).map_err(|_| "product_keys_input_limit")?;
    let capacity = length.checked_add(1).ok_or("product_keys_input_limit")?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(capacity)
        .map_err(|_| "product_keys_allocation")?;
    if bytes.capacity() > capacity {
        return Err("product_keys_input_limit".into());
    }
    bytes.resize(capacity, 0);
    file.read_exact(&mut bytes[..length]).map_err(|error| {
        if error.kind() == std::io::ErrorKind::UnexpectedEof {
            "product_keys_input_changed"
        } else {
            "product_keys_input_io"
        }
    })?;
    let extra = loop {
        match file.read(&mut bytes[length..]) {
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            result => break result.map_err(|_| "product_keys_input_io")?,
        }
    };
    let after = file.metadata().map_err(|_| "product_keys_input_changed")?;
    let named = std::fs::symlink_metadata(path).map_err(|_| "product_keys_input_changed")?;
    if extra != 0 || !same_input(&before, &after) || !same_input(&before, &named) {
        return Err("product_keys_input_changed".into());
    }
    let source = std::fs::canonicalize(path).map_err(|_| "product_keys_input_changed")?;
    let canonical = std::fs::symlink_metadata(&source).map_err(|_| "product_keys_input_changed")?;
    if !same_input(&before, &canonical) {
        return Err("product_keys_input_changed".into());
    }
    let keyring = forever_casc::keys::Keyring::parse(&bytes[..length])
        .map_err(|error| crate::Error::from(error.code()))?;
    Ok(SuppliedKeys { keyring, source })
}

#[cfg(not(windows))]
fn same_input(a: &std::fs::Metadata, b: &std::fs::Metadata) -> bool {
    if !a.is_file() || !b.is_file() || a.len() != b.len() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        a.dev() == b.dev()
            && a.ino() == b.ino()
            && a.mtime() == b.mtime()
            && a.mtime_nsec() == b.mtime_nsec()
            && a.ctime() == b.ctime()
            && a.ctime_nsec() == b.ctime_nsec()
    }
    #[cfg(not(unix))]
    {
        a.modified().ok() == b.modified().ok()
    }
}

/// Reject special files without blocking on FIFOs. Linux descriptor flags also
/// reject final symlinks; parent path trust is supplied by the operator.
#[cfg(not(windows))]
pub fn open(path: &Path) -> Result<File, &'static str> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(0x800 | 0x20000); // O_NONBLOCK | O_NOFOLLOW
    }
    let file = options.open(path).map_err(|_| "product_input_io")?;
    if !file.metadata().map_err(|_| "product_input_io")?.is_file() {
        return Err("product_input_type");
    }
    Ok(file)
}

#[cfg(not(windows))]
pub fn bytes(path: &Path, limit: u64) -> Result<Vec<u8>, &'static str> {
    let mut file = open(path)?;
    let length = file.metadata().map_err(|_| "product_input_io")?.len();
    if length > limit {
        return Err("product_input_limit");
    }
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(length as usize)
        .map_err(|_| "product_allocation")?;
    (&mut file)
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "product_input_io")?;
    if bytes.len() as u64 > limit || bytes.len() as u64 != length {
        return Err("product_input_changed");
    }
    Ok(bytes)
}

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::{bytes, keys};

#[cfg(all(test, not(windows)))]
mod tests;
#[cfg(all(test, windows))]
mod windows_tests;
