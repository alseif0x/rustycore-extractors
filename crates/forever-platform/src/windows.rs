// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original conservative Windows NTFS readonly handle leases.

use crate::{
    Error,
    path::{AbsolutePath, Limits},
};
use std::{
    cell::Cell,
    fs::{File, OpenOptions},
    os::windows::fs::{FileExt, OpenOptionsExt},
    path::{Path, PathBuf},
};

#[allow(unsafe_code)]
mod ffi;

pub mod output;
pub mod package;
pub mod producer;

/// Conservative readonly NTFS lease profile; not a product support claim.
pub const WINDOWS_READ_PROFILE: &str = "forever-windows-read-ntfs-v1";

const FILE_SHARE_READ: u32 = 1;
const FILE_READ_ATTRIBUTES: u32 = 0x80;
const FILE_LIST_DIRECTORY: u32 = 1;
const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
const FILE_ATTRIBUTE_DIRECTORY: u32 = 0x10;
const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;

/// Whether extra names for the final file are admitted. Both modes fix link
/// count in the captured identity and reject any observed change.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinkPolicy {
    Reject,
    Allow,
}

/// Identity and change metadata from the same open file handle. No access time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Identity {
    pub volume: u32,
    pub file_index: u64,
    pub links: u32,
    pub attributes: u32,
    pub size: u64,
    pub creation_time: u64,
    pub write_time: u64,
    pub change_time: i64,
}
impl Identity {
    /// Compares object identity and attributes only; does not compare content
    /// size, timestamps, or link count. Callers must choose this policy explicitly.
    pub fn same_object(self, other: Self) -> bool {
        self.volume == other.volume
            && self.file_index == other.file_index
            && self.attributes == other.attributes
    }
}

struct Held {
    file: File,
    identity: Identity,
}

/// A readonly lease with retained ancestors. Not Clone, Debug, or Sync. Opening
/// and verification temporarily use at most ancestors + two handles; a settled
/// lease holds ancestors + one. Ancestor timestamps are intentionally ignored:
/// unrelated sibling creation must not invalidate the lease.
pub struct ReadFile {
    path: AbsolutePath,
    ancestors: Vec<Held>,
    leaf: Held,
    limits: Limits,
    links: LinkPolicy,
    directory: bool,
    invalid: Cell<bool>,
}
impl ReadFile {
    pub fn open(path: &Path, limits: Limits, links: LinkPolicy) -> Result<Self, Error> {
        Self::open_kind(path, limits, links, false)
    }
    fn open_kind(
        path: &Path,
        limits: Limits,
        links: LinkPolicy,
        directory: bool,
    ) -> Result<Self, Error> {
        let limits = limits.validate()?;
        let path = AbsolutePath::parse(path.to_str().ok_or(Error::Path)?, limits)?;
        ffi::fixed_drive(&path.as_str()[..3])?;
        let mut ancestors = Vec::new();
        ancestors
            .try_reserve_exact(path.ancestor_count())
            .map_err(|_| Error::Allocation)?;
        let mut prefixes = path.prefixes();
        for _ in 0..path.ancestor_count() {
            let file = open(prefixes.next().ok_or(Error::Path)?, true)?;
            let identity = ffi::object_identity(&file)?;
            admit(identity, true, links)?;
            if let Some(root) = ancestors.first() {
                let root: &Held = root;
                if root.identity.volume != identity.volume {
                    return Err(Error::Identity);
                }
            } else {
                ffi::ntfs_volume(&file)?;
            }
            ancestors.push(Held { file, identity });
        }
        let file = open(prefixes.next().ok_or(Error::Path)?, directory)?;
        let identity = ffi::identity(&file)?;
        admit(identity, directory, links)?;
        if identity.volume != ancestors[0].identity.volume {
            return Err(Error::Identity);
        }
        drop(prefixes);
        let result = Self {
            path,
            ancestors,
            leaf: Held { file, identity },
            limits,
            links,
            directory,
            invalid: Cell::new(false),
        };
        result.verify_unchanged()?;
        Ok(result)
    }
    pub fn identity(&self) -> Identity {
        self.leaf.identity
    }
    /// Actual retained handle count, including the final file. No hidden cache.
    pub fn retained_handles(&self) -> usize {
        self.ancestors.len() + 1
    }

    /// Checks held handles and each exact named prefix. Any observed mismatch or
    /// check failure permanently invalidates this lease. This is not an atomic
    /// content snapshot, authentication, or a USN-journal transaction.
    pub fn verify_unchanged(&self) -> Result<(), Error> {
        self.verify(false)
    }
    fn verify(&self, object_only: bool) -> Result<(), Error> {
        if self.invalid.get() {
            return Err(Error::Changed);
        }
        let result = self.check(object_only);
        if result.is_err() {
            self.invalid.set(true);
        }
        result.map_err(|_| Error::Changed)
    }
    fn check(&self, object_only: bool) -> Result<(), Error> {
        for (prefix, held) in self.path.prefixes().zip(&self.ancestors) {
            let current = ffi::object_identity(&held.file)?;
            admit(current, true, self.links)?;
            if !held.identity.same_object(current) {
                return Err(Error::Changed);
            }
            let named = open(prefix, true)?;
            let identity = ffi::object_identity(&named)?;
            admit(identity, true, self.links)?;
            if !held.identity.same_object(identity) {
                return Err(Error::Changed);
            }
        }
        self.check_leaf(&self.leaf.file, object_only)?;
        let named = open(self.path.as_str(), self.directory)?;
        self.check_leaf(&named, object_only)
    }
    fn check_leaf(&self, file: &File, object_only: bool) -> Result<(), Error> {
        let current = if object_only {
            ffi::object_identity(file)?
        } else {
            ffi::identity(file)?
        };
        admit(current, self.directory, self.links)?;
        let matches = |identity| {
            if object_only {
                self.leaf.identity.same_object(identity)
            } else {
                self.leaf.identity == identity
            }
        };
        if !matches(current) {
            return Err(Error::Changed);
        }
        Ok(())
    }
    pub fn read_all(&self, max_bytes: usize) -> Result<Vec<u8>, Error> {
        if max_bytes > self.limits.max_read_bytes {
            return Err(Error::Limit);
        }
        let len = usize::try_from(self.leaf.identity.size).map_err(|_| Error::Limit)?;
        if len > max_bytes {
            return Err(Error::Limit);
        }
        let mut result = Vec::new();
        result
            .try_reserve_exact(len)
            .map_err(|_| Error::Allocation)?;
        result.resize(len, 0);
        self.read_exact_at(0, &mut result)?;
        let mut extra = [0];
        let extra_read = self
            .leaf
            .file
            .seek_read(&mut extra, self.leaf.identity.size);
        self.check_eof_result(extra_read)?;
        self.verify_unchanged()?;
        Ok(result)
    }
    /// Probes the exact captured EOF while retaining all read-only leases.
    /// Checks both before and after the one-byte read; an observed extent/read
    /// failure permanently invalidates the lease. No payload bytes are returned.
    pub fn verify_eof(&self) -> Result<(), Error> {
        self.verify_unchanged()?;
        let mut extra = [0];
        let result = self
            .leaf
            .file
            .seek_read(&mut extra, self.leaf.identity.size);
        let result = self.check_eof_result(result);
        self.verify_unchanged()?;
        result
    }
    fn check_eof_result(&self, result: std::io::Result<usize>) -> Result<(), Error> {
        match result {
            Ok(0) => Ok(()),
            Ok(_) => {
                self.invalid.set(true);
                Err(Error::Changed)
            }
            Err(_) => {
                self.invalid.set(true);
                Err(Error::Read)
            }
        }
    }
    /// Reads a bounded complete range. On failure the caller's buffer may contain
    /// a prefix and must be discarded; success requires pre/post verification.
    pub fn read_exact_at(&self, offset: u64, output: &mut [u8]) -> Result<(), Error> {
        if output.len() > self.limits.max_read_bytes {
            return Err(Error::Limit);
        }
        let end = offset
            .checked_add(output.len() as u64)
            .ok_or(Error::Limit)?;
        if end > self.leaf.identity.size {
            return Err(Error::Read);
        }
        self.verify_unchanged()?;
        let mut done = 0;
        while done < output.len() {
            let n = match self
                .leaf
                .file
                .seek_read(&mut output[done..], offset + done as u64)
            {
                Ok(0) => {
                    self.invalid.set(true);
                    return Err(Error::Changed);
                }
                Ok(n) => n,
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => {
                    self.invalid.set(true);
                    return Err(Error::Read);
                }
            };
            done += n;
        }
        self.verify_unchanged()
    }
}

/// A guarded final directory, with the same ancestor, reparse, NTFS and sharing
/// policy as ReadFile. Snapshot verification includes final-directory metadata;
/// same-object verification is an explicit alternative that permits sibling
/// entry changes. Any observed check failure permanently invalidates the lease.
/// No listing or descendant opening is performed by this primitive.
pub struct ReadDirectory {
    inner: ReadFile,
}
impl ReadDirectory {
    pub fn open(path: &Path, limits: Limits) -> Result<Self, Error> {
        Ok(Self {
            inner: ReadFile::open_kind(path, limits, LinkPolicy::Allow, true)?,
        })
    }
    pub fn identity(&self) -> Identity {
        self.inner.identity()
    }
    pub fn retained_handles(&self) -> usize {
        self.inner.retained_handles()
    }
    pub fn verify_unchanged(&self) -> Result<(), Error> {
        self.inner.verify_unchanged()
    }
    pub fn verify_same_object(&self) -> Result<(), Error> {
        self.inner.verify(true)
    }
}

/// Admits an operator-supplied normal absolute drive path and owns its exact
/// spelling. Does not access the filesystem, resolve relative paths, canonicalize,
/// normalize aliases, follow reparses, or strip Windows verbatim/device prefixes.
/// Volume-root-only paths remain outside this versioned profile.
pub fn operator_path(path: &Path, limits: Limits) -> Result<PathBuf, Error> {
    let admitted = AbsolutePath::parse(path.to_str().ok_or(Error::Path)?, limits)?;
    let mut result = PathBuf::new();
    result
        .try_reserve_exact(path.as_os_str().len())
        .map_err(|_| Error::Allocation)?;
    result.push(admitted.as_str());
    Ok(result)
}

fn open(path: &str, directory: bool) -> Result<File, Error> {
    let mut options = OpenOptions::new();
    options
        .share_mode(FILE_SHARE_READ)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT);
    if directory {
        options.access_mode(FILE_LIST_DIRECTORY | FILE_READ_ATTRIBUTES);
    } else {
        options.read(true);
    }
    options.open(path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            Error::Missing
        } else {
            Error::Open
        }
    })
}
fn admit(identity: Identity, directory: bool, links: LinkPolicy) -> Result<(), Error> {
    if identity.attributes & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return Err(Error::Reparse);
    }
    if (identity.attributes & FILE_ATTRIBUTE_DIRECTORY != 0) != directory {
        return Err(Error::FileType);
    }
    if identity.file_index == 0 || identity.links == 0 {
        return Err(Error::Identity);
    }
    if !directory && links == LinkPolicy::Reject && identity.links != 1 {
        return Err(Error::HardLinks);
    }
    Ok(())
}

#[cfg(test)]
mod tests;
