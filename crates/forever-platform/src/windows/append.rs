// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-13: original bounded exclusive NTFS scratch archive.
//! A fresh private owner directory, retained namespace lock, and one unshared
//! read/write file. The API only appends exact extents and permits patching the
//! pending extent. This is not a kernel append-only handle, content snapshot,
//! scratch recovery protocol, disk reservation or power-loss durability claim.
//! The caller owns record framing and complete same-handle hash verification.
//! Drive paths must retain complete native volume-root ancestry; drive aliases
//! mapped to a subdirectory are rejected because they hide higher ancestors.
use super::{Identity, LinkPolicy, ReadDirectory, ReadFile, package};
use crate::path::{AbsolutePath, Limits as PathLimits};
use std::{
    cell::Cell,
    fs::File,
    mem::size_of,
    os::windows::fs::FileExt,
    path::{Path, PathBuf},
    rc::Rc,
};
#[allow(unsafe_code)]
mod ffi;
#[cfg(test)]
mod tests;

pub const PROFILE: &str = "windows-exclusive-append-v1";
const MAX_BYTES: u64 = 128 * 1024 * 1024 * 1024;
const MAX_IO: usize = 1024 * 1024;
const MAX_METADATA: usize = 4 * 1024 * 1024;
const MAX_WORK: u64 = 4_000_000_000;
const MAX_HANDLES: usize = 2048;
const TRANSIENT_BYTES: usize = 65536 + 8192;
const MAX_INPUTS: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Allocation,
    Limit,
    Path,
    Alias,
    Exists,
    Unsupported,
    Changed,
    Io,
    Poisoned,
}
impl Error {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Allocation => "windows_append_allocation",
            Self::Limit => "windows_append_limit",
            Self::Path => "windows_append_path",
            Self::Alias => "windows_append_alias",
            Self::Exists => "windows_append_exists",
            Self::Unsupported => "windows_append_unsupported",
            Self::Changed => "windows_append_changed",
            Self::Io => "windows_append_io",
            Self::Poisoned => "windows_append_poisoned",
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}
impl std::error::Error for Error {}
fn package_error(error: package::Error) -> Error {
    match error {
        package::Error::Allocation => Error::Allocation,
        package::Error::Limit => Error::Limit,
        package::Error::Path => Error::Path,
        package::Error::Alias => Error::Alias,
        package::Error::Exists => Error::Exists,
        package::Error::Unsupported => Error::Unsupported,
        package::Error::Open
        | package::Error::Read
        | package::Error::Write
        | package::Error::Flush => Error::Io,
        _ => Error::Changed,
    }
}
fn input_error(error: crate::Error) -> Error {
    match error {
        crate::Error::Allocation => Error::Allocation,
        crate::Error::Limit => Error::Limit,
        crate::Error::Path => Error::Path,
        crate::Error::Open | crate::Error::Missing | crate::Error::Read => Error::Io,
        _ => Error::Changed,
    }
}

/// Compile-time ABI support only; callers must also run `admit`.
pub const fn supported() -> bool {
    cfg!(all(
        windows,
        target_arch = "x86_64",
        target_pointer_width = "64"
    ))
}
/// Native AMD64 admission, before any pathname or filesystem operation. The
/// borrowed process pseudo-handle is never closed or exposed to the caller.
pub fn admit() -> Result<(), Error> {
    if !supported() {
        return Err(Error::Unsupported);
    }
    ffi::architecture()
}
fn architecture(success: bool, process_machine: u16, native_machine: u16) -> Result<(), Error> {
    if success && process_machine == 0 && native_machine == 0x8664 {
        Ok(())
    } else {
        Err(Error::Unsupported)
    }
}
pub(crate) fn mutable_identity(file: &File) -> Result<Identity, Error> {
    ffi::identity(file)
}
fn mutable_snapshot(before: Identity, after: Identity) -> Result<Identity, Error> {
    if before.file_index == 0
        || before.links != 1
        || after.links != 1
        || before.volume != after.volume
        || before.file_index != after.file_index
        || before.size != after.size
        || before.creation_time != after.creation_time
        || (before.attributes ^ after.attributes) & !0xa0 != 0
    {
        return Err(Error::Changed);
    }
    Ok(before)
}

/// Independent downward-only logical caps, not heap/RSS or reserved disk space.
/// Work conservatively admits object visits in input/root guards and I/O calls;
/// it does not count every instruction, kernel operation or allocator action.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub max_bytes: u64,
    pub max_io_bytes: usize,
    pub max_metadata_bytes: usize,
    pub max_work: u64,
    pub max_handles: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_bytes: MAX_BYTES,
            max_io_bytes: MAX_IO,
            max_metadata_bytes: MAX_METADATA,
            max_work: MAX_WORK,
            max_handles: MAX_HANDLES,
        }
    }
}
impl Limits {
    pub fn validate(self) -> Result<Self, Error> {
        if self.max_bytes > MAX_BYTES
            || self.max_io_bytes == 0
            || self.max_io_bytes > MAX_IO
            || self.max_metadata_bytes == 0
            || self.max_metadata_bytes > MAX_METADATA
            || self.max_work == 0
            || self.max_work > MAX_WORK
            || self.max_handles == 0
            || self.max_handles > MAX_HANDLES
        {
            return Err(Error::Limit);
        }
        Ok(self)
    }
}

struct Inputs {
    source: ReadDirectory,
    keys: Vec<ReadFile>,
}
/// Owned readonly input leases. Clones share the capture through Rc; every
/// archive conservatively charges the whole capture, independent of sharing.
#[derive(Clone)]
pub struct ProtectedInputs {
    inner: Rc<Inputs>,
}
impl ProtectedInputs {
    pub fn capture(source_root: &Path, key_files: &[&Path]) -> Result<Self, Error> {
        admit()?;
        if key_files.len() >= MAX_INPUTS {
            return Err(Error::Limit);
        }
        // Parse every path and admit all handles before acquiring input leases.
        let mut paths = std::iter::once(source_root).chain(key_files.iter().copied());
        let handles = paths.try_fold(0_usize, |n, path| {
            let parsed =
                AbsolutePath::parse(path.to_str().ok_or(Error::Path)?, PathLimits::default())
                    .map_err(input_error)?;
            n.checked_add(parsed.ancestor_count() + 2)
                .ok_or(Error::Limit)
        })?;
        if handles > MAX_HANDLES {
            return Err(Error::Limit);
        }
        // New append inputs require one link throughout each captured chain;
        // the public v1 readonly directory/file policies remain unchanged.
        let source = ReadDirectory {
            inner: ReadFile::open_kind(
                source_root,
                PathLimits::default(),
                LinkPolicy::Reject,
                true,
            )
            .map_err(input_error)?,
        };
        let mut keys = Vec::new();
        keys.try_reserve_exact(key_files.len())
            .map_err(|_| Error::Allocation)?;
        for path in key_files {
            keys.push(
                ReadFile::open(path, PathLimits::default(), LinkPolicy::Reject)
                    .map_err(input_error)?,
            );
        }
        let result = Self {
            inner: Rc::new(Inputs { source, keys }),
        };
        let mut refs = [&result.inner.source.inner; MAX_INPUTS];
        for (slot, key) in refs.iter_mut().zip(&result.inner.keys) {
            *slot = key;
        }
        package::append_support::input_roots(
            &[&result.inner.source],
            &refs[..result.inner.keys.len()],
        )
        .map_err(package_error)?;
        if result
            .retained_metadata_bytes()?
            .checked_add(TRANSIENT_BYTES)
            .is_none_or(|n| n > MAX_METADATA)
        {
            return Err(Error::Limit);
        }
        result.verify_unchanged()?;
        Ok(result)
    }
    pub fn verify_unchanged(&self) -> Result<(), Error> {
        admit()?;
        self.inner
            .source
            .verify_same_object()
            .map_err(|_| Error::Changed)?;
        for key in &self.inner.keys {
            key.verify_unchanged().map_err(|_| Error::Changed)?;
        }
        Ok(())
    }
    pub fn retained_handles(&self) -> usize {
        self.inner.source.retained_handles()
            + self
                .inner
                .keys
                .iter()
                .map(ReadFile::retained_handles)
                .sum::<usize>()
    }
    pub fn retained_metadata_bytes(&self) -> Result<usize, Error> {
        let initial = size_of::<Inputs>() + 2 * size_of::<usize>() + size_of::<Self>();
        let base = self
            .inner
            .keys
            .capacity()
            .checked_mul(size_of::<ReadFile>())
            .and_then(|n| n.checked_add(initial))
            .ok_or(Error::Limit)?;
        let base = base
            .checked_add(read_metadata(&self.inner.source.inner)?)
            .ok_or(Error::Limit)?;
        self.inner.keys.iter().try_fold(base, |n, key| {
            n.checked_add(read_metadata(key)?).ok_or(Error::Limit)
        })
    }
    fn protect(&self, path: &Path, max_handles: usize) -> Result<(), Error> {
        let mut refs = [&self.inner.source.inner; MAX_INPUTS];
        for (slot, key) in refs.iter_mut().zip(&self.inner.keys) {
            *slot = key;
        }
        package::append_support::protect_output(
            path,
            &[&self.inner.source],
            &refs[..self.inner.keys.len()],
            max_handles,
        )
        .map_err(package_error)
    }
    fn create_root(&self, path: &Path, limits: package::Limits) -> Result<package::Root, Error> {
        let mut refs = [&self.inner.source.inner; MAX_INPUTS];
        for (slot, key) in refs.iter_mut().zip(&self.inner.keys) {
            *slot = key;
        }
        package::append_support::create_root(
            path,
            limits,
            &[&self.inner.source],
            &refs[..self.inner.keys.len()],
            None,
        )
        .map_err(package_error)
    }
}
fn read_metadata(file: &ReadFile) -> Result<usize, Error> {
    file.ancestors
        .capacity()
        .checked_mul(size_of::<super::Held>())
        .and_then(|n| n.checked_add(file.path.retained_metadata_bytes()?))
        .ok_or(Error::Limit)
}

/// Check a held scratch directory's exact protected current-owner-only DACL.
pub fn private_directory(directory: &ReadDirectory) -> Result<(), Error> {
    admit()?;
    package::append_support::private_directory(directory).map_err(package_error)
}
/// Readonly volume/file-identity exclusion, including drive-letter aliases and
/// output containing an input. Existing root and all ancestors exclude `.git`.
pub fn protect_output(
    candidate: &Path,
    protected_dirs: &[&ReadDirectory],
    protected_files: &[&ReadFile],
) -> Result<(), Error> {
    admit()?;
    package::append_support::protect_output(candidate, protected_dirs, protected_files, MAX_HANDLES)
        .map_err(package_error)
}

/// Fresh private scratch namespace keeper. Children belong to independent
/// append owners; this namespace does not adopt or recover previous scratch.
pub struct Namespace {
    root: package::Root,
    lock: package::Lock,
    path: PathBuf,
}
impl Namespace {
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn guard(&self) -> Result<(), Error> {
        admit()?;
        self.lock.guard().map_err(package_error)?;
        self.root.guard().map_err(package_error)
    }
    pub fn resident_bytes(&self) -> Result<usize, Error> {
        self.root
            .retained_metadata_bytes()
            .map_err(package_error)?
            .checked_add(size_of::<Self>() + self.path.capacity())
            .ok_or(Error::Limit)
    }
    pub fn retained_handles(&self) -> usize {
        self.root.stats().retained_handles
    }
}
pub fn create_namespace(parent: &ReadDirectory, name: &str) -> Result<Namespace, Error> {
    admit()?;
    private_directory(parent)?;
    package::append_support::name(name).map_err(package_error)?;
    let mut path = PathBuf::new();
    path.try_reserve_exact(parent.inner.path.as_str().len() + 1 + name.len())
        .map_err(|_| Error::Allocation)?;
    path.push(parent.inner.path.as_str());
    path.push(name);
    // Identity exclusion and .git checks precede mutation. Parent stays retained.
    protect_output(&path, &[], &[])?;
    let mut root = package::append_support::create_root(
        &path,
        package::Limits {
            max_metadata_bytes: MAX_METADATA,
            ..package::Limits::default()
        },
        &[],
        &[],
        Some(parent),
    )
    .map_err(package_error)?;
    parent.verify_same_object().map_err(|_| Error::Changed)?;
    let lock = root.lock(true).map_err(package_error)?;
    let result = Namespace { root, lock, path };
    result.guard()?;
    Ok(result)
}

/// One fresh archive with fixed O(input-depth + owner-depth) handle/metadata
/// residency. No record descriptors or per-record filesystem leases are cached.
pub struct OwnedArchive {
    root: package::Root,
    lock: package::Lock,
    file: File,
    identity: Identity,
    protected: ProtectedInputs,
    limits: Limits,
    committed: u64,
    physical: Cell<u64>,
    in_progress: Cell<bool>,
    failure: Cell<Option<Error>>,
    work: Cell<u64>,
}
impl OwnedArchive {
    pub fn create(
        owner_directory: &Path,
        protected: ProtectedInputs,
        limits: Limits,
    ) -> Result<Self, Error> {
        admit()?;
        let limits = limits.validate()?;
        let parsed = AbsolutePath::parse(
            owner_directory.to_str().ok_or(Error::Path)?,
            PathLimits::default(),
        )
        .map_err(input_error)?;
        let initial_work = (protected.retained_handles() as u64)
            .checked_mul(4)
            .and_then(|n| n.checked_add((parsed.ancestor_count() as u64) * 12 + 64))
            .ok_or(Error::Limit)?;
        if initial_work > limits.max_work {
            return Err(Error::Limit);
        }
        drop(parsed);
        protected.verify_unchanged()?;
        let base = protected
            .retained_metadata_bytes()?
            .checked_add(size_of::<Self>() + TRANSIENT_BYTES)
            .ok_or(Error::Limit)?;
        let metadata = limits
            .max_metadata_bytes
            .checked_sub(base)
            .ok_or(Error::Limit)?;
        let handles = limits
            .max_handles
            .checked_sub(protected.retained_handles() + 1)
            .ok_or(Error::Limit)?;
        protected.protect(owner_directory, limits.max_handles)?;
        let mut root = protected.create_root(
            owner_directory,
            package::Limits {
                max_entries: 2,
                max_directories: 0,
                max_handles: handles.min(95),
                max_metadata_bytes: metadata.min(package::Limits::default().max_metadata_bytes),
                max_work: usize::try_from(limits.max_work).map_err(|_| Error::Limit)?,
                ..package::Limits::default()
            },
        )?;
        let lock = root.lock(true).map_err(package_error)?;
        let (file, identity) = package::append_support::create(&mut root).map_err(package_error)?;
        let result = Self {
            root,
            lock,
            file,
            identity,
            protected,
            limits,
            committed: 0,
            physical: Cell::new(0),
            in_progress: Cell::new(false),
            failure: Cell::new(None),
            work: Cell::new(initial_work),
        };
        if result
            .resident_bytes()?
            .checked_add(TRANSIENT_BYTES)
            .is_none_or(|n| n > limits.max_metadata_bytes)
            || result.retained_handles() + 2 > limits.max_handles
        {
            return Err(Error::Limit);
        }
        result.guard()?;
        Ok(result)
    }
    pub fn len(&self) -> u64 {
        self.committed
    }
    pub fn is_empty(&self) -> bool {
        self.committed == 0
    }
    pub fn resident_bytes(&self) -> Result<usize, Error> {
        self.root
            .retained_metadata_bytes()
            .map_err(package_error)?
            .checked_add(self.protected.retained_metadata_bytes()?)
            .and_then(|n| n.checked_add(size_of::<Self>()))
            .ok_or(Error::Limit)
    }
    pub fn retained_handles(&self) -> usize {
        self.root.stats().retained_handles + 1 + self.protected.retained_handles()
    }
    pub fn failure(&self) -> Option<Error> {
        self.failure.get()
    }
    pub fn poison(&self) {
        self.fail(Error::Poisoned);
    }
    fn fail(&self, error: Error) -> Error {
        if error == Error::Changed || self.failure.get().is_none() {
            self.failure.set(Some(error));
        }
        error
    }
    fn spend(&self, amount: u64) -> Result<(), Error> {
        let n = self.work.get().checked_add(amount).ok_or(Error::Limit)?;
        if n > self.limits.max_work {
            return Err(Error::Limit);
        }
        self.work.set(n);
        Ok(())
    }
    fn integrity(&self) -> Result<(), Error> {
        let result = (|| {
            admit()?;
            // Two object visits per protected handle (held and named), plus
            // root held/named/private/lock visits and fixed native enumeration.
            let visits = (self.protected.retained_handles() as u64) * 2
                + (self.root.stats().retained_handles as u64) * 3
                + 16;
            self.spend(visits)?;
            self.protected.verify_unchanged()?;
            self.lock.guard().map_err(package_error)?;
            package::append_support::check(
                &self.root,
                &self.file,
                self.identity,
                self.physical.get(),
            )
            .map_err(|e| match e {
                package::Error::Limit => Error::Limit,
                package::Error::Allocation => Error::Allocation,
                _ => Error::Changed,
            })
        })();
        result.map_err(|e| self.fail(e))
    }
    /// Integrity checks still run after an abandoned transaction. This permits
    /// a caller to distinguish observed Changed from logical Poisoned and keep
    /// its original callback error. No successful guard clears sticky failure.
    pub fn guard(&self) -> Result<(), Error> {
        self.guard_pending()?;
        if self.in_progress.get() {
            return Err(self.fail(Error::Poisoned));
        }
        Ok(())
    }
    fn guard_pending(&self) -> Result<(), Error> {
        let observed = self.integrity();
        if self.failure.get() == Some(Error::Changed) {
            return Err(Error::Changed);
        }
        observed?;
        self.failure.get().map_or(Ok(()), Err)
    }
    fn range(&self, offset: u64, length: usize, end: u64) -> Result<(), Error> {
        if length > self.limits.max_io_bytes
            || offset.checked_add(length as u64).is_none_or(|n| n > end)
        {
            Err(Error::Limit)
        } else {
            Ok(())
        }
    }
    fn read_at(&self, offset: u64, output: &mut [u8]) -> Result<(), Error> {
        self.integrity()?;
        let result = (|| {
            let mut done = 0;
            while done < output.len() {
                self.spend(1)?;
                let n = self
                    .file
                    .seek_read(&mut output[done..], offset + done as u64)
                    .map_err(|_| Error::Io)?;
                if n == 0 {
                    return Err(Error::Changed);
                }
                done += n;
            }
            Ok(())
        })();
        let post = self.integrity();
        match (result, post) {
            (_, Err(e)) => Err(e),
            (Err(e), _) => Err(self.fail(e)),
            _ => Ok(()),
        }
    }
    pub fn read_exact_at(&self, offset: u64, output: &mut [u8]) -> Result<(), Error> {
        self.guard()?;
        self.range(offset, output.len(), self.committed)?;
        self.read_at(offset, output)
    }
    pub fn begin_append(&mut self, exact_bytes: u64) -> Result<AppendTxn<'_>, Error> {
        self.guard()?;
        if exact_bytes == 0
            || self
                .committed
                .checked_add(exact_bytes)
                .is_none_or(|n| n > self.limits.max_bytes || n > i64::MAX as u64)
        {
            return Err(Error::Limit);
        }
        let start = self.committed;
        // Explicit state survives a forgotten transaction as well as unwind.
        // No later append may overwrite an uncommitted physical suffix.
        self.in_progress.set(true);
        Ok(AppendTxn {
            archive: self,
            start,
            length: exact_bytes,
            written: 0,
            flushed: false,
            committed: false,
        })
    }
}

/// Borrows its owner exclusively. Drop (including unwind) without commit leaves
/// partial bytes in place and permanently poisons the owner; there is no rollback.
pub struct AppendTxn<'a> {
    archive: &'a mut OwnedArchive,
    start: u64,
    length: u64,
    written: u64,
    flushed: bool,
    committed: bool,
}
impl AppendTxn<'_> {
    pub fn start(&self) -> u64 {
        self.start
    }
    pub fn len(&self) -> u64 {
        self.length
    }
    pub fn is_empty(&self) -> bool {
        self.length == 0
    }
    fn check(&self) -> Result<(), Error> {
        self.archive.guard_pending()
    }
    fn error<T>(&self, result: Result<T, Error>) -> Result<T, Error> {
        result.map_err(|e| self.archive.fail(e))
    }
    pub fn write_all(&mut self, bytes: &[u8]) -> Result<(), Error> {
        let result = (|| {
            self.check()?;
            self.archive.range(self.written, bytes.len(), self.length)?;
            let mut done = 0;
            while done < bytes.len() {
                self.archive.spend(1)?;
                let n = self
                    .archive
                    .file
                    .seek_write(&bytes[done..], self.start + self.written)
                    .map_err(|_| Error::Io)?;
                if n == 0 {
                    return Err(Error::Io);
                }
                self.written += n as u64;
                done += n;
                self.archive.physical.set(self.start + self.written);
                self.flushed = false;
            }
            self.check()
        })();
        self.error(result)
    }
    /// Relative offset within the fully written portion of this new extent.
    pub fn patch_at(&mut self, relative: u64, bytes: &[u8]) -> Result<(), Error> {
        let result = (|| {
            self.check()?;
            self.archive.range(relative, bytes.len(), self.written)?;
            let mut done = 0;
            while done < bytes.len() {
                self.archive.spend(1)?;
                let n = self
                    .archive
                    .file
                    .seek_write(&bytes[done..], self.start + relative + done as u64)
                    .map_err(|_| Error::Io)?;
                if n == 0 {
                    return Err(Error::Io);
                }
                done += n;
                self.flushed = false;
            }
            self.check()
        })();
        self.error(result)
    }
    pub fn flush(&mut self) -> Result<(), Error> {
        let result = (|| {
            self.check()?;
            self.archive.spend(1)?;
            package::append_support::flush(&self.archive.file).map_err(|_| Error::Io)?;
            self.check()?;
            self.flushed = true;
            Ok(())
        })();
        self.error(result)
    }
    pub fn read_exact_at(&self, relative: u64, output: &mut [u8]) -> Result<(), Error> {
        let result = (|| {
            self.check()?;
            self.archive.range(relative, output.len(), self.written)?;
            self.archive.read_at(self.start + relative, output)
        })();
        self.error(result)
    }
    pub fn commit(mut self) -> Result<(), Error> {
        let result = (|| {
            self.check()?;
            if self.written != self.length {
                return Err(Error::Limit);
            }
            if !self.flushed {
                self.flush()?;
            }
            self.check()?;
            self.archive.committed = self.start + self.length;
            self.archive.in_progress.set(false);
            self.committed = true;
            Ok(())
        })();
        self.error(result)
    }
}
impl Drop for AppendTxn<'_> {
    fn drop(&mut self) {
        if !self.committed {
            self.archive.poison();
        }
    }
}
