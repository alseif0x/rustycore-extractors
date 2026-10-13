// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original guarded Windows local-store acquisition.

use super::{Budget, Error, reserve};
use forever_platform::{
    Error as PlatformError,
    path::{Limits, MAX_READ_BYTES},
    windows::{Identity, LinkPolicy, ReadDirectory, ReadFile},
};
use std::path::{Path, PathBuf};

pub(in crate::local_store) fn map_error(error: PlatformError) -> Error {
    match error {
        PlatformError::Limit | PlatformError::Allocation => Error::Limit,
        PlatformError::Changed => Error::InputChanged,
        _ => Error::Input,
    }
}

pub(in crate::local_store) fn directory(path: &Path) -> Result<ReadDirectory, Error> {
    ReadDirectory::open(path, Limits::default()).map_err(map_error)
}

pub(in crate::local_store) fn archive(path: &Path) -> Result<ReadFile, PlatformError> {
    // Local client hardlinks are admitted, with their count and exact file
    // identity fixed throughout each operation. Reparse points never are.
    ReadFile::open(path, Limits::default(), LinkPolicy::Allow)
}

#[derive(Clone, Copy)]
enum Kind {
    File,
    Directory,
}

// Captured guards retain metadata and the exact admitted spelling, not handles.
// Each check opens one fresh full ancestor lease. Cumulative store handles thus
// do not multiply by the number of indexes/configuration guards.
pub(in crate::local_store) struct Guard {
    pub(in crate::local_store) path: PathBuf,
    identity: Identity,
    kind: Kind,
    expected: Option<Vec<u8>>,
}
impl Guard {
    pub(in crate::local_store) fn capture(path: &Path) -> Result<Self, Error> {
        let file = archive(path).map_err(map_error)?;
        let identity = file.identity();
        // Drop before storing the guard: captures retain no OS handle.
        drop(file);
        Self::snapshot(path, identity, Kind::File)
    }
    pub(in crate::local_store) fn capture_directory(path: &Path) -> Result<Self, Error> {
        let directory = directory(path)?;
        let identity = directory.identity();
        drop(directory);
        Self::snapshot(path, identity, Kind::Directory)
    }
    fn snapshot(path: &Path, identity: Identity, kind: Kind) -> Result<Self, Error> {
        let mut owned = PathBuf::new();
        owned
            .try_reserve_exact(path.as_os_str().len())
            .map_err(|_| Error::Limit)?;
        owned.push(path);
        Ok(Self {
            path: owned,
            identity,
            kind,
            expected: None,
        })
    }
    pub(in crate::local_store) fn with_bytes(mut self, bytes: Vec<u8>) -> Self {
        self.expected = Some(bytes);
        self
    }
    pub(in crate::local_store) fn check(&self) -> Result<(), Error> {
        // Any previously captured input that cannot be reacquired is a change,
        // including disappearance, sharing conflicts and filesystem rejection.
        match self.kind {
            Kind::Directory => {
                let directory = directory(&self.path).map_err(|_| Error::InputChanged)?;
                if directory.identity() != self.identity {
                    return Err(Error::InputChanged);
                }
                directory
                    .verify_unchanged()
                    .map_err(|_| Error::InputChanged)
            }
            Kind::File => {
                let file = archive(&self.path).map_err(|_| Error::InputChanged)?;
                if file.identity() != self.identity {
                    return Err(Error::InputChanged);
                }
                if let Some(expected) = &self.expected {
                    if expected.len() as u64 != self.identity.size {
                        return Err(Error::InputChanged);
                    }
                    let mut chunk = [0; 4096];
                    for (n, part) in expected.chunks(chunk.len()).enumerate() {
                        file.read_exact_at((n * chunk.len()) as u64, &mut chunk[..part.len()])
                            .map_err(|_| Error::InputChanged)?;
                        if &chunk[..part.len()] != part {
                            return Err(Error::InputChanged);
                        }
                    }
                }
                file.verify_unchanged().map_err(|_| Error::InputChanged)
            }
        }
    }
}

pub(in crate::local_store) fn whole(
    path: &Path,
    max_bytes: u64,
    budget: &Budget,
) -> Result<Vec<u8>, Error> {
    let file = archive(path).map_err(map_error)?;
    let len = file.identity().size;
    if len > max_bytes {
        return Err(Error::Limit);
    }
    let size = usize::try_from(len).map_err(|_| Error::Limit)?;
    budget.phase(size)?;
    let mut bytes = Vec::new();
    reserve(&mut bytes, size)?;
    bytes.resize(size, 0);
    read_range(&file, 0, &mut bytes)?;
    Ok(bytes)
}

pub(in crate::local_store) fn read_range(
    file: &ReadFile,
    offset: u64,
    bytes: &mut [u8],
) -> Result<(), Error> {
    // The primitive's 64-MiB operation ceiling is independent of the store's
    // admitted 256-MiB index/span ceiling. Allocate/account once, read in chunks
    // under the same retained lease, and verify the entire acquisition again.
    file.verify_unchanged().map_err(map_error)?;
    for (n, chunk) in bytes.chunks_mut(MAX_READ_BYTES).enumerate() {
        let at = offset
            .checked_add((n * MAX_READ_BYTES) as u64)
            .ok_or(Error::Limit)?;
        file.read_exact_at(at, chunk).map_err(map_error)?;
    }
    file.verify_unchanged().map_err(map_error)
}

#[cfg(test)]
mod tests;
