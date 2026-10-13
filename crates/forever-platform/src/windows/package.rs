// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-13: separate bounded, scalable private NTFS package namespace.
//! A package keeps its root/ancestors, metadata and a package lock, with temporary
//! leases bounded by path depth. It never retains every output file. Relative NT
//! opens, exact stored spelling, ordinary objects and protected owner-only DACLs
//! are checked on each lease. Observations are not atomic snapshots; flush and
//! rename do not claim power-loss durability. Public root I/O is not reentrant
//! from a file callback; its same Reader/Writer lease remains usable. This
//! profile does not change v1.

use super::{Identity, ReadFile};
use crate::path::{AbsolutePath, Limits as PathLimits};
use std::{
    cell::Cell,
    fs::File,
    io::{self, Read, Seek, SeekFrom, Write},
    mem::size_of,
    os::windows::fs::FileExt,
    path::Path,
    rc::Rc,
};
pub(crate) mod append_support;
#[allow(unsafe_code)]
mod ffi;
mod membership;
#[cfg(test)]
mod tests;

pub const WINDOWS_PACKAGE_PROFILE: &str = "forever-windows-package-namespace-v1";
const MAX_ENTRIES: usize = 786_464;
const MAX_DIRECTORIES: usize = 262_144;
const MAX_DEPTH: usize = 16;
const MAX_PATH: usize = 512;
const MAX_FILE: u64 = 128 * 1024 * 1024;
// Common package payloads have a separate 1 TiB cap; protocol files need room.
const MAX_TOTAL: u64 = (1_u64 << 40) + 512 * 1024 * 1024;
const MAX_METADATA: usize = 256 * 1024 * 1024;
// Independent logical work ceiling, sized for guarded large package plans.
// Maxima are separate admission ceilings, not a full-world throughput promise.
const MAX_WORK: usize = 4_000_000_000;
const MAX_HANDLES: usize = 96;
const IO_CHUNK: usize = 1024 * 1024;
const LOCK_NAME: &str = ".forever-package-lock";

/// Static redacted diagnostics, without paths, SIDs or native messages.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Path,
    Limit,
    Allocation,
    Token,
    Impersonation,
    Privacy,
    Exists,
    Missing,
    Busy,
    Open,
    Identity,
    Reparse,
    HardLinks,
    Changed,
    Read,
    Write,
    Flush,
    Rename,
    Remove,
    Unsupported,
    Alias,
    Membership,
}
impl Error {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Path => "windows_package_path",
            Self::Limit => "windows_package_limit",
            Self::Allocation => "windows_package_allocation",
            Self::Token => "windows_package_token",
            Self::Impersonation => "windows_package_impersonation",
            Self::Privacy => "windows_package_privacy",
            Self::Exists => "windows_package_exists",
            Self::Missing => "windows_package_missing",
            Self::Busy => "windows_package_busy",
            Self::Open => "windows_package_open",
            Self::Identity => "windows_package_identity",
            Self::Reparse => "windows_package_reparse",
            Self::HardLinks => "windows_package_hard_links",
            Self::Changed => "windows_package_changed",
            Self::Read => "windows_package_read",
            Self::Write => "windows_package_write",
            Self::Flush => "windows_package_flush",
            Self::Rename => "windows_package_rename",
            Self::Remove => "windows_package_remove",
            Self::Unsupported => "windows_package_unsupported",
            Self::Alias => "windows_package_case_alias",
            Self::Membership => "windows_package_membership",
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}
impl std::error::Error for Error {}

/// Downward-only logical budgets, not RSS, disk reservation or all CPU work.
/// Entries include files/directories below root. Work counts guarded object/path
/// visits, enumeration calls and I/O calls. Handles include retained ancestors,
/// lock and transient path/file leases, independently of the entry count.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub max_entries: usize,
    pub max_directories: usize,
    pub max_depth: usize,
    pub max_path_bytes: usize,
    pub max_file_bytes: u64,
    pub max_total_bytes: u64,
    pub max_metadata_bytes: usize,
    pub max_work: usize,
    pub max_handles: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_entries: MAX_ENTRIES,
            max_directories: MAX_DIRECTORIES,
            max_depth: MAX_DEPTH,
            max_path_bytes: MAX_PATH,
            max_file_bytes: MAX_FILE,
            max_total_bytes: MAX_TOTAL,
            max_metadata_bytes: MAX_METADATA,
            max_work: MAX_WORK,
            max_handles: MAX_HANDLES,
        }
    }
}
impl Limits {
    pub fn validate(self) -> Result<Self, Error> {
        if self.max_entries == 0
            || self.max_entries > MAX_ENTRIES
            || self.max_directories > MAX_DIRECTORIES
            || self.max_depth == 0
            || self.max_depth > MAX_DEPTH
            || self.max_path_bytes == 0
            || self.max_path_bytes > MAX_PATH
            || self.max_file_bytes > MAX_FILE
            || self.max_total_bytes > MAX_TOTAL
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    File,
    Directory,
}
/// A completely observed entry. The caller may adopt identities only after its
/// complete protocol/inventory preflight; names alone are not adopted proofs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub path: String,
    pub identity: Identity,
    pub kind: Kind,
}
/// Sum of retained path capacities. Add Vec::capacity()*size_of::<Entry>() when
/// charging a whole observation; a slice cannot report its owning Vec capacity.
pub fn metadata_bytes(entries: &[Entry]) -> Result<usize, Error> {
    entries.iter().try_fold(0_usize, |n, e| {
        n.checked_add(e.path.capacity()).ok_or(Error::Limit)
    })
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stats {
    pub entries: usize,
    pub directories: usize,
    pub bytes: u64,
    pub retained_handles: usize,
    pub work: usize,
}
struct Held {
    file: File,
    identity: Identity,
    name: String,
}
struct Chain {
    held: Vec<Held>,
}
/// A callback holds its path/file leases until it returns. Rejecting nested
/// public root I/O prevents those leases from multiplying behind per-operation
/// handle admission. This also resets on callback error or unwind.
struct CallbackGate<'a> {
    active: &'a Cell<bool>,
}
impl Drop for CallbackGate<'_> {
    fn drop(&mut self) {
        self.active.set(false);
    }
}
impl Chain {
    fn file<'a>(&'a self, root: &'a Root) -> &'a File {
        self.held.last().map_or(&root.root.file, |v| &v.file)
    }
}

/// The root is intentionally neither Sync nor Clone. Short-lived leases retain
/// every intermediate directory while a read/write/rename/removal is running.
pub struct Root {
    ancestors: Vec<Held>,
    root: Held,
    drive: String,
    sid: ffi::Sid,
    limits: Limits,
    invalid: Rc<Cell<bool>>,
    work: Cell<usize>,
    entries: Cell<usize>,
    directories: Cell<usize>,
    bytes: Cell<u64>,
    counted: Cell<bool>,
    lock_state: Rc<Cell<Option<bool>>>,
    lock_identity: Cell<Option<Identity>>,
    callback_active: Cell<bool>,
}
impl Root {
    pub fn create(path: &Path, limits: Limits) -> Result<Self, Error> {
        Self::open_kind(path, limits, true)
    }
    pub fn open(path: &Path, limits: Limits) -> Result<Self, Error> {
        Self::open_kind(path, limits, false)
    }
    fn open_kind(path: &Path, limits: Limits, create: bool) -> Result<Self, Error> {
        Self::open_kind_checked(path, limits, create, |_| Ok(()))
    }
    // Separate append consumers tie protected identities to the actual anchors
    // immediately before relative creation. Existing package entrypoints retain
    // their original admission and behavior through the no-op closure above.
    fn open_kind_checked(
        path: &Path,
        limits: Limits,
        create: bool,
        precreate: impl FnOnce(&[Held]) -> Result<(), Error>,
    ) -> Result<Self, Error> {
        let limits = limits.validate()?;
        let parsed = AbsolutePath::parse(path.to_str().ok_or(Error::Path)?, PathLimits::default())
            .map_err(|_| Error::Path)?;
        let admission = parsed
            .ancestor_count()
            .checked_mul(size_of::<Held>())
            .and_then(|v| v.checked_add(parsed.as_str().len()))
            .and_then(|v| v.checked_add(size_of::<Root>() + size_of::<Lock>() + 65536 + 4096))
            .ok_or(Error::Limit)?;
        if admission > limits.max_metadata_bytes {
            return Err(Error::Limit);
        }
        let sid = ffi::process_sid()?;
        let (ancestors, drive, name) = anchors(&parsed, limits.max_handles)?;
        precreate(&ancestors)?;
        let parent = &ancestors.last().ok_or(Error::Path)?.file;
        let file = if create {
            ffi::create(parent, &name, true, &sid)?
        } else {
            ffi::open(parent, &name, ffi::Mode::Directory)?
        };
        let identity = capture(&file, true)?;
        admit(identity, true, ancestors[0].identity.volume)?;
        ffi::ordinary_directory(&file)?;
        ffi::private(&file, &sid)?;
        ffi::exact_name(&file, &name)?;
        let result = Self {
            ancestors,
            root: Held {
                file,
                identity,
                name,
            },
            drive,
            sid,
            limits,
            invalid: Rc::new(Cell::new(false)),
            work: Cell::new(0),
            entries: Cell::new(0),
            directories: Cell::new(0),
            bytes: Cell::new(0),
            counted: Cell::new(create),
            lock_state: Rc::new(Cell::new(None)),
            lock_identity: Cell::new(None),
            callback_active: Cell::new(false),
        };
        if result
            .retained_metadata_bytes()?
            .checked_add(65536)
            .is_none_or(|v| v > limits.max_metadata_bytes)
        {
            return Err(Error::Limit);
        }
        result.guard()?;
        Ok(result)
    }
    pub fn limits(&self) -> Limits {
        self.limits
    }
    /// Retained root/anchor Vec and string capacities plus fixed owner/lock/Rc
    /// state. Transient scan/name/path buffers are separately admitted.
    pub fn retained_metadata_bytes(&self) -> Result<usize, Error> {
        let base = self
            .ancestors
            .capacity()
            .checked_mul(size_of::<Held>())
            .and_then(|v| {
                v.checked_add(
                    size_of::<Root>()
                        + size_of::<Lock>()
                        + 4 * size_of::<usize>()
                        + size_of::<Cell<Option<bool>>>()
                        + size_of::<Cell<bool>>(),
                )
            })
            .ok_or(Error::Limit)?;
        self.ancestors
            .iter()
            .try_fold(base, |n, v| {
                n.checked_add(v.name.capacity()).ok_or(Error::Limit)
            })?
            .checked_add(self.root.name.capacity())
            .and_then(|v| v.checked_add(self.drive.capacity()))
            .ok_or(Error::Limit)
    }
    pub fn identity(&self) -> Identity {
        self.root.identity
    }
    pub fn stats(&self) -> Stats {
        Stats {
            entries: self.entries.get(),
            directories: self.directories.get(),
            bytes: self.bytes.get(),
            retained_handles: self.ancestors.len()
                + 1
                + usize::from(self.lock_state.get().is_some()),
            work: self.work.get(),
        }
    }
    fn enter_callback(&self) -> Result<CallbackGate<'_>, Error> {
        if self.callback_active.replace(true) {
            return Err(Error::Busy);
        }
        Ok(CallbackGate {
            active: &self.callback_active,
        })
    }
    fn spend(&self, amount: usize) -> Result<(), Error> {
        if self.invalid.get() {
            return Err(Error::Changed);
        }
        let n = self.work.get().checked_add(amount).ok_or(Error::Limit)?;
        if n > self.limits.max_work {
            return Err(Error::Limit);
        }
        self.work.set(n);
        Ok(())
    }
    fn handles(&self, extra: usize) -> Result<(), Error> {
        if self
            .stats()
            .retained_handles
            .checked_add(extra)
            .ok_or(Error::Limit)?
            > self.limits.max_handles
        {
            Err(Error::Limit)
        } else {
            Ok(())
        }
    }
    fn poison<T>(&self, result: Result<T, Error>) -> Result<T, Error> {
        if result.as_ref().err().is_some_and(|e| {
            !matches!(
                e,
                Error::Limit
                    | Error::Allocation
                    | Error::Exists
                    | Error::Busy
                    | Error::Missing
                    | Error::Path
            )
        }) {
            self.invalid.set(true);
        }
        result
    }
    fn writable(&self) -> Result<(), Error> {
        if self.lock_state.get() != Some(true) {
            return Err(Error::Busy);
        }
        if !self.counted.get() {
            return Err(Error::Membership);
        }
        self.guard()
    }
    /// Observed identity/security checks, not an atomic snapshot. Integrity
    /// failures permanently poison the root even if the object is later repaired.
    pub fn guard(&self) -> Result<(), Error> {
        // This check precedes work admission or native I/O. All public root I/O
        // enters through guard; Reader/Writer only operate their existing lease.
        if self.callback_active.get() {
            return Err(Error::Busy);
        }
        self.spend(1)?;
        let result = (|| {
            if ffi::process_sid()? != self.sid {
                return Err(Error::Token);
            }
            super::ffi::fixed_drive(&self.drive).map_err(|_| Error::Unsupported)?;
            self.handles(1)?;
            let named = super::open(&self.drive, true).map_err(|_| Error::Changed)?;
            check_external(&named, self.ancestors[0].identity)?;
            drop(named);
            for (i, held) in self.ancestors.iter().enumerate() {
                self.spend(1)?;
                check_external(&held.file, held.identity)?;
                if i > 0 {
                    let named = ffi::open(
                        &self.ancestors[i - 1].file,
                        &held.name,
                        ffi::Mode::Directory,
                    )?;
                    check_external(&named, held.identity)?;
                    ffi::exact_name(&named, &held.name)?;
                }
            }
            self.check_private(&self.root.file, self.root.identity, true)?;
            let named = ffi::open(
                &self.ancestors.last().ok_or(Error::Path)?.file,
                &self.root.name,
                ffi::Mode::Directory,
            )?;
            self.check_private(&named, self.root.identity, true)?;
            ffi::exact_name(&named, &self.root.name)?;
            drop(named);
            if self.lock_state.get().is_some() {
                let file = ffi::open(&self.root.file, LOCK_NAME, ffi::Mode::Lock)?;
                self.check_private(
                    &file,
                    self.lock_identity.get().ok_or(Error::Identity)?,
                    false,
                )?;
                ffi::exact_name(&file, LOCK_NAME)?;
            }
            Ok(())
        })();
        // Missing during a guard is a changed binding, not a normal absent file.
        self.poison(result.map_err(|e| {
            if e == Error::Missing {
                Error::Changed
            } else {
                e
            }
        }))
    }
    fn check_private(&self, file: &File, expected: Identity, directory: bool) -> Result<(), Error> {
        self.spend(1)?;
        let current = capture(file, directory)?;
        admit(current, directory, self.root.identity.volume)?;
        if if directory {
            !expected.same_object(current)
        } else {
            expected != current
        } {
            return Err(Error::Changed);
        }
        ffi::private(file, &self.sid)?;
        if directory {
            ffi::ordinary_directory(file)?;
        }
        Ok(())
    }
    fn capture_private(&self, file: &File, name: &str, directory: bool) -> Result<Identity, Error> {
        self.spend(1)?;
        let identity = capture(file, directory)?;
        admit(identity, directory, self.root.identity.volume)?;
        ffi::private(file, &self.sid)?;
        ffi::exact_name(file, name)?;
        if directory {
            ffi::ordinary_directory(file)?;
        } else if identity.size > self.limits.max_file_bytes {
            return Err(Error::Limit);
        }
        Ok(identity)
    }
    fn chain(&self, path: &str) -> Result<Chain, Error> {
        let mut held = Vec::new();
        let count = if path.is_empty() {
            0
        } else {
            path.split('/').count()
        };
        self.handles(count + 2)?;
        held.try_reserve_exact(count)
            .map_err(|_| Error::Allocation)?;
        if !path.is_empty() {
            validate_path(path, self.limits)?;
            for name in path.split('/') {
                let parent = held.last().map_or(&self.root.file, |e: &Held| &e.file);
                let file = ffi::open(parent, name, ffi::Mode::Directory)?;
                let identity = self.capture_private(&file, name, true)?;
                held.push(Held {
                    file,
                    identity,
                    name: owned(name)?,
                });
            }
        }
        Ok(Chain { held })
    }
    fn check_chain(&self, chain: &Chain) -> Result<(), Error> {
        self.guard()?;
        for (index, held) in chain.held.iter().enumerate() {
            self.check_private(&held.file, held.identity, true)?;
            let parent = if index == 0 {
                &self.root.file
            } else {
                &chain.held[index - 1].file
            };
            let named = ffi::open(parent, &held.name, ffi::Mode::Directory)?;
            self.check_private(&named, held.identity, true)?;
            ffi::exact_name(&named, &held.name)?;
        }
        Ok(())
    }
    pub fn create_dirs(&mut self, path: &str) -> Result<(), Error> {
        validate_path(path, self.limits)?;
        self.writable()?;
        let result = (|| {
            let mut prefix = String::new();
            prefix
                .try_reserve_exact(path.len())
                .map_err(|_| Error::Allocation)?;
            for name in path.split('/') {
                let parent = self.chain(&prefix)?;
                match ffi::open(parent.file(self), name, ffi::Mode::Directory) {
                    Ok(file) => {
                        self.capture_private(&file, name, true)?;
                    }
                    Err(Error::Missing) => {
                        if self.entries.get() >= self.limits.max_entries
                            || self.directories.get() >= self.limits.max_directories
                        {
                            return Err(Error::Limit);
                        }
                        let file = ffi::create(parent.file(self), name, true, &self.sid)?;
                        self.capture_private(&file, name, true)?;
                        self.entries.set(self.entries.get() + 1);
                        self.directories.set(self.directories.get() + 1);
                    }
                    Err(e) => return Err(e),
                }
                self.check_chain(&parent)?;
                if !prefix.is_empty() {
                    prefix.push('/');
                }
                prefix.push_str(name);
            }
            self.guard()
        })();
        self.poison(result)
    }
    /// A new file is created exclusively and never overwritten. Partial callback
    /// failures retain the file and poison the root; recovery belongs to package.
    pub fn with_new_file<T, E>(
        &mut self,
        path: &str,
        callback: impl FnOnce(&mut Writer<'_>) -> Result<T, E>,
    ) -> Result<(Identity, T), E>
    where
        E: From<Error>,
    {
        validate_path(path, self.limits).map_err(E::from)?;
        self.writable().map_err(E::from)?;
        if self.entries.get() >= self.limits.max_entries {
            return Err(E::from(Error::Limit));
        }
        let (parent, name) = split(path);
        let chain = self.poison(self.chain(parent)).map_err(E::from)?;
        let file = self
            .poison(ffi::create(chain.file(self), name, false, &self.sid))
            .map_err(E::from)?;
        self.entries.set(self.entries.get() + 1);
        let original = self
            .poison(self.capture_private(&file, name, false))
            .map_err(E::from)?;
        let mut writer = Writer {
            root: self,
            file: &file,
            position: 0,
            failed: false,
        };
        let callback_result = {
            let _gate = self.enter_callback().map_err(E::from)?;
            callback(&mut writer)
        };
        let value = match callback_result {
            Ok(v) => v,
            Err(e) => {
                self.invalid.set(true);
                return Err(e);
            }
        };
        if writer.failed {
            self.invalid.set(true);
            return Err(E::from(Error::Write));
        }
        let size = writer.position;
        let result = (|| {
            ffi::flush(&file)?;
            let identity = self.capture_private(&file, name, false)?;
            if original.volume != identity.volume
                || original.file_index != identity.file_index
                || original.creation_time != identity.creation_time
                || identity.size != size
                || ((original.attributes ^ identity.attributes) & !0xa0) != 0
            {
                return Err(Error::Changed);
            }
            self.check_chain(&chain)?;
            Ok(identity)
        })();
        let identity = self.poison(result).map_err(E::from)?;
        // Drop the DELETE-access writer before a deny-delete read reopen.
        drop(file);
        let result = (|| {
            let named = ffi::open(chain.file(self), name, ffi::Mode::Read)?;
            self.check_private(&named, identity, false)?;
            ffi::exact_name(&named, name)?;
            self.check_chain(&chain)
        })();
        self.poison(result).map_err(E::from)?;
        Ok((identity, value))
    }
    /// Hashing and inspection can seek/re-read one lease. A successful callback
    /// must have read the complete captured extent at least once, contiguously
    /// from offset zero. No raw handle or File escapes this API.
    pub fn with_read_file<T, E>(
        &self,
        path: &str,
        expected: Option<Identity>,
        callback: impl FnOnce(&mut Reader<'_>) -> Result<T, E>,
    ) -> Result<(Identity, T), E>
    where
        E: From<Error>,
    {
        validate_path(path, self.limits).map_err(E::from)?;
        self.guard().map_err(E::from)?;
        let result = (|| {
            let (parent, name) = split(path);
            let chain = self.chain(parent)?;
            let file = ffi::open(chain.file(self), name, ffi::Mode::Read)?;
            let identity = self.capture_private(&file, name, false)?;
            if expected.is_some_and(|v| v != identity) {
                return Err(Error::Changed);
            }
            Ok((chain, file, identity))
        })();
        let result = result.map_err(|e| {
            if e == Error::Missing && expected.is_some() {
                Error::Changed
            } else {
                e
            }
        });
        let (chain, file, identity) = self.poison(result).map_err(E::from)?;
        let mut reader = Reader {
            root: self,
            file: &file,
            identity,
            position: 0,
            covered: 0,
            failed: false,
        };
        let callback_result = {
            let _gate = self.enter_callback().map_err(E::from)?;
            callback(&mut reader)
        };
        let value = match callback_result {
            Ok(v) => v,
            Err(e) => {
                self.invalid.set(true);
                return Err(e);
            }
        };
        let complete = reader.covered == identity.size && !reader.failed;
        let result = (|| {
            if !complete {
                return Err(Error::Read);
            }
            let mut extra = [0];
            self.spend(1)?;
            if file
                .seek_read(&mut extra, identity.size)
                .map_err(|_| Error::Read)?
                != 0
            {
                return Err(Error::Changed);
            }
            self.check_private(&file, identity, false)?;
            self.check_chain(&chain)?;
            let (_, name) = split(path);
            let named = ffi::open(chain.file(self), name, ffi::Mode::Read)?;
            self.check_private(&named, identity, false)?;
            ffi::exact_name(&named, name)?;
            Ok(())
        })();
        self.poison(result).map_err(E::from)?;
        Ok((identity, value))
    }
    /// Cheap guarded identity lookup; only a missing path component returns
    /// None. Failed guards, aliases and nonordinary objects remain errors.
    pub fn file_identity(&self, path: &str) -> Result<Option<Identity>, Error> {
        validate_path(path, self.limits)?;
        self.guard()?;
        let result = (|| {
            let (parent, name) = split(path);
            let chain = match self.chain(parent) {
                Ok(v) => v,
                Err(Error::Missing) => return Ok(None),
                Err(e) => return Err(e),
            };
            let mode = if path == LOCK_NAME {
                ffi::Mode::Lock
            } else {
                ffi::Mode::Read
            };
            let file = match ffi::open(chain.file(self), name, mode) {
                Ok(v) => v,
                Err(Error::Missing) => {
                    self.check_chain(&chain)?;
                    return Ok(None);
                }
                Err(e) => return Err(e),
            };
            let identity = self.capture_private(&file, name, false)?;
            self.check_chain(&chain)?;
            Ok(Some(identity))
        })();
        self.poison(result)
    }
    pub fn rename_no_replace(
        &mut self,
        from: &str,
        to: &str,
        expected: Identity,
    ) -> Result<Identity, Error> {
        validate_path(from, self.limits)?;
        validate_path(to, self.limits)?;
        self.writable()?;
        let result = (|| {
            let (source, name) = split(from);
            let (target, new_name) = split(to);
            let source = self.chain(source)?;
            // Both chains plus source/target probes are live together.
            self.handles(
                source.held.len() + target.split('/').filter(|v| !v.is_empty()).count() + 3,
            )?;
            let target = self.chain(target)?;
            let file = ffi::open(source.file(self), name, ffi::Mode::Modify)?;
            let current = self.capture_private(&file, name, false)?;
            if current != expected {
                return Err(Error::Changed);
            }
            match ffi::open(target.file(self), new_name, ffi::Mode::Read) {
                Ok(_) => return Err(Error::Exists),
                Err(Error::Missing) => {}
                Err(e) => return Err(e),
            }
            self.check_chain(&source)?;
            self.check_chain(&target)?;
            ffi::rename(&file, target.file(self), new_name)?;
            ffi::flush(&file)?;
            // The renamed binding is checked by a fresh relative reopen below;
            // do not rely on a retained FILE_OBJECT's cached filename here.
            let identity = capture(&file, false)?;
            self.check_private(&file, identity, false)?;
            if !expected.same_object(identity)
                || expected.size != identity.size
                || expected.creation_time != identity.creation_time
                || expected.write_time != identity.write_time
            {
                return Err(Error::Changed);
            }
            self.check_chain(&source)?;
            self.check_chain(&target)?;
            // Only the source handle has DELETE access. Release it before a new
            // deny-delete read lease; keep both directory chains throughout.
            drop(file);
            let named = ffi::open(target.file(self), new_name, ffi::Mode::Read)?;
            self.check_private(&named, identity, false)?;
            ffi::exact_name(&named, new_name)?;
            match ffi::open(source.file(self), name, ffi::Mode::Read) {
                Err(Error::Missing) => {}
                Ok(_) => return Err(Error::Changed),
                Err(e) => return Err(e),
            }
            Ok(identity)
        })();
        self.poison(result.map_err(|e| {
            if e == Error::Missing {
                Error::Changed
            } else {
                e
            }
        }))
    }
    pub fn remove_same(&mut self, path: &str, expected: Identity) -> Result<(), Error> {
        validate_path(path, self.limits)?;
        self.writable()?;
        if path == LOCK_NAME {
            return Err(Error::Busy);
        }
        let result = (|| {
            let (parent, name) = split(path);
            let chain = self.chain(parent)?;
            let file = ffi::open(chain.file(self), name, ffi::Mode::Modify)?;
            let identity = self.capture_private(&file, name, false)?;
            if identity != expected {
                return Err(Error::Changed);
            }
            self.check_chain(&chain)?;
            ffi::remove(&file)?;
            // FileDispositionInformation deletion completes at final close.
            drop(file);
            match ffi::open(chain.file(self), name, ffi::Mode::Read) {
                Err(Error::Missing) => {}
                Ok(_) => return Err(Error::Changed),
                Err(e) => return Err(e),
            }
            self.entries
                .set(self.entries.get().checked_sub(1).ok_or(Error::Changed)?);
            self.bytes.set(
                self.bytes
                    .get()
                    .checked_sub(expected.size)
                    .ok_or(Error::Changed)?,
            );
            self.check_chain(&chain)
        })();
        self.poison(result.map_err(|e| {
            if e == Error::Missing {
                Error::Changed
            } else {
                e
            }
        }))
    }
    /// Complete bounded recursive observation with exact spelling and no aliases.
    /// Opening an existing root requires this preflight after acquiring its lock
    /// before any mutation. The returned inventory is owned by the caller.
    pub fn observe(&self) -> Result<Vec<Entry>, Error> {
        self.observe_charged(0)
    }
    /// Reserves caller-owned inventory/replay capacity before any observation
    /// allocation. The retained root and bounded transient leases are additional.
    pub fn observe_charged(&self, other_retained_bytes: usize) -> Result<Vec<Entry>, Error> {
        self.guard()?;
        let result = membership::observe(self, other_retained_bytes);
        if result
            .as_ref()
            .err()
            .is_some_and(|e| !matches!(e, Error::Limit | Error::Allocation))
        {
            self.invalid.set(true);
        }
        let entries = result?;
        self.guard()?;
        Ok(entries)
    }
    // Vec is required to account for its actual retained allocation capacity.
    #[allow(clippy::ptr_arg)]
    pub fn verify_membership(&self, expected: &Vec<Entry>) -> Result<(), Error> {
        self.verify_membership_charged(expected, 0)
    }
    /// Also reserves caller-owned inventory/replay allocations during the
    /// second observation and index comparison.
    #[allow(clippy::ptr_arg)]
    pub fn verify_membership_charged(
        &self,
        expected: &Vec<Entry>,
        other_retained_bytes: usize,
    ) -> Result<(), Error> {
        let reserved = expected
            .capacity()
            .checked_mul(size_of::<Entry>())
            .and_then(|v| v.checked_add(metadata_bytes(expected).ok()?))
            .and_then(|v| v.checked_add(other_retained_bytes))
            .ok_or(Error::Limit)?;
        self.guard()?;
        let result = membership::observe(self, reserved);
        if result
            .as_ref()
            .err()
            .is_some_and(|e| !matches!(e, Error::Limit | Error::Allocation))
        {
            self.invalid.set(true);
        }
        let observed = result?;
        self.guard()?;
        let result = membership::compare(self, &observed, expected, reserved);
        self.poison(result)
    }
    /// The lock is created/adopted before namespace observation, including an
    /// empty root left by interruption during initial package setup. A live
    /// shared lock permits reads; a live exclusive lock admits mutations.
    pub fn lock(&mut self, exclusive: bool) -> Result<Lock, Error> {
        self.guard()?;
        if self.lock_state.get().is_some() {
            return Err(Error::Busy);
        }
        self.handles(2)?;
        let file = match ffi::open(&self.root.file, LOCK_NAME, ffi::Mode::Lock) {
            Ok(file) => file,
            Err(Error::Missing) => {
                if !exclusive {
                    return Err(Error::Missing);
                }
                if self.counted.get() && self.entries.get() >= self.limits.max_entries {
                    return Err(Error::Limit);
                }
                let file =
                    self.poison(ffi::create(&self.root.file, LOCK_NAME, false, &self.sid))?;
                self.capture_private(&file, LOCK_NAME, false)?;
                ffi::flush(&file)?;
                drop(file);
                if self.counted.get() {
                    self.entries.set(self.entries.get() + 1);
                }
                ffi::open(&self.root.file, LOCK_NAME, ffi::Mode::Lock)?
            }
            Err(e) => return self.poison(Err(e)),
        };
        let identity = self.poison(self.capture_private(&file, LOCK_NAME, false))?;
        if identity.size != 0 {
            return self.poison(Err(Error::Changed));
        }
        let result = if exclusive {
            file.try_lock()
        } else {
            file.try_lock_shared()
        };
        result.map_err(|_| Error::Busy)?;
        let sid = ffi::process_sid()?;
        if sid != self.sid {
            return self.poison(Err(Error::Token));
        }
        self.lock_identity.set(Some(identity));
        self.lock_state.set(Some(exclusive));
        let lock = Lock {
            file,
            identity,
            sid,
            state: Rc::clone(&self.lock_state),
            invalid: Rc::clone(&self.invalid),
        };
        self.guard()?;
        Ok(lock)
    }
}

/// Retained package byte-range lock. Closing it releases exclusion; its live
/// state is tracked by Root so a dropped lock cannot authorize later writes.
pub struct Lock {
    file: File,
    identity: Identity,
    sid: ffi::Sid,
    state: Rc<Cell<Option<bool>>>,
    invalid: Rc<Cell<bool>>,
}
impl Lock {
    pub fn guard(&self) -> Result<(), Error> {
        if self.invalid.get() {
            return Err(Error::Changed);
        }
        let result = (|| {
            if ffi::process_sid()? != self.sid || capture(&self.file, false)? != self.identity {
                return Err(Error::Changed);
            }
            ffi::private(&self.file, &self.sid)
        })();
        if result.is_err() {
            self.invalid.set(true);
        }
        result
    }
}
impl Drop for Lock {
    fn drop(&mut self) {
        self.state.set(None);
    }
}

pub struct Reader<'a> {
    root: &'a Root,
    file: &'a File,
    identity: Identity,
    position: u64,
    covered: u64,
    failed: bool,
}
impl Reader<'_> {
    pub fn len(&self) -> u64 {
        self.identity.size
    }
    pub fn is_empty(&self) -> bool {
        self.identity.size == 0
    }
}
impl Read for Reader<'_> {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        if self.failed || self.root.invalid.get() {
            return Err(io_error(Error::Changed));
        }
        if output.is_empty() || self.position == self.identity.size {
            return Ok(0);
        }
        let n =
            (self.identity.size - self.position).min(output.len().min(IO_CHUNK) as u64) as usize;
        let result = (|| {
            self.root.spend(1)?;
            let count = self
                .file
                .seek_read(&mut output[..n], self.position)
                .map_err(|_| Error::Read)?;
            if count == 0 {
                return Err(Error::Changed);
            }
            if self.position <= self.covered {
                self.covered = self.covered.max(self.position + count as u64);
            }
            self.position += count as u64;
            Ok(count)
        })();
        if result.is_err() {
            self.failed = true;
            self.root.invalid.set(true);
        }
        result.map_err(io_error)
    }
}
impl Seek for Reader<'_> {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        let position = match position {
            SeekFrom::Start(v) => i128::from(v),
            SeekFrom::Current(v) => i128::from(self.position) + i128::from(v),
            SeekFrom::End(v) => i128::from(self.identity.size) + i128::from(v),
        };
        if position < 0 || position > i128::from(self.identity.size) {
            return Err(io_error(Error::Read));
        }
        self.position = position as u64;
        Ok(self.position)
    }
}
pub struct Writer<'a> {
    root: &'a Root,
    file: &'a File,
    position: u64,
    failed: bool,
}
impl Write for Writer<'_> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.failed || self.root.invalid.get() {
            return Err(io_error(Error::Changed));
        }
        if bytes.is_empty() {
            return Ok(0);
        }
        let count = bytes.len().min(IO_CHUNK);
        let result = (|| {
            self.root.spend(1)?;
            let size = self
                .position
                .checked_add(count as u64)
                .ok_or(Error::Limit)?;
            let total = self
                .root
                .bytes
                .get()
                .checked_add(count as u64)
                .ok_or(Error::Limit)?;
            if size > self.root.limits.max_file_bytes || total > self.root.limits.max_total_bytes {
                return Err(Error::Limit);
            }
            // Charge the admitted request even on a partial OS write failure.
            self.root.bytes.set(total);
            let mut file = self.file;
            file.write_all(&bytes[..count]).map_err(|_| Error::Write)?;
            self.position = size;
            Ok(count)
        })();
        if result.is_err() {
            self.failed = true;
            self.root.invalid.set(true);
        }
        result.map_err(io_error)
    }
    fn flush(&mut self) -> io::Result<()> {
        let result = self.root.spend(1).and_then(|()| ffi::flush(self.file));
        if result.is_err() {
            self.failed = true;
            self.root.invalid.set(true);
        }
        result.map_err(io_error)
    }
}
fn io_error(e: Error) -> io::Error {
    io::Error::other(e)
}
fn owned(text: &str) -> Result<String, Error> {
    let mut s = String::new();
    s.try_reserve_exact(text.len())
        .map_err(|_| Error::Allocation)?;
    s.push_str(text);
    Ok(s)
}
fn split(path: &str) -> (&str, &str) {
    path.rsplit_once('/').unwrap_or(("", path))
}
fn validate_path(path: &str, limits: Limits) -> Result<(), Error> {
    if path.is_empty()
        || path.len() > limits.max_path_bytes
        || path.split('/').count() > limits.max_depth
    {
        return Err(Error::Path);
    }
    for name in path.split('/') {
        validate_name(name)?;
    }
    Ok(())
}
fn validate_name(name: &str) -> Result<(), Error> {
    if name.is_empty()
        || name.len() > 255
        || name == "."
        || name == ".."
        || name.eq_ignore_ascii_case(".git")
        || name.ends_with(['.', ' '])
        || name
            .bytes()
            .any(|b| !(0x20..=0x7e).contains(&b) || b"<>:\"/\\|?*".contains(&b))
    {
        return Err(Error::Path);
    }
    let stem = name.split('.').next().unwrap_or("").trim_end_matches(' ');
    if ["CON", "PRN", "AUX", "NUL", "CONIN$", "CONOUT$"]
        .iter()
        .any(|n| stem.eq_ignore_ascii_case(n))
        || (stem.len() == 4
            && (stem[..3].eq_ignore_ascii_case("COM") || stem[..3].eq_ignore_ascii_case("LPT"))
            && matches!(stem.as_bytes()[3], b'1'..=b'9'))
    {
        return Err(Error::Path);
    }
    Ok(())
}
fn capture(file: &File, directory: bool) -> Result<Identity, Error> {
    if directory {
        super::ffi::object_identity(file)
    } else {
        super::ffi::identity(file)
    }
    .map_err(|_| Error::Identity)
}
fn admit(i: Identity, directory: bool, volume: u32) -> Result<(), Error> {
    if i.attributes & 0x400 != 0 {
        return Err(Error::Reparse);
    }
    if i.links != 1 {
        return Err(Error::HardLinks);
    }
    if i.file_index == 0
        || i.volume != volume
        || (i.attributes & 0x10 != 0) != directory
        || i.attributes & (1 | 4 | 0x40 | 0x200 | 0x800 | 0x1000 | 0x4000) != 0
    {
        return Err(Error::Identity);
    }
    Ok(())
}
fn check_external(file: &File, expected: Identity) -> Result<(), Error> {
    let current = capture(file, true)?;
    if current.attributes & 0x400 != 0 {
        return Err(Error::Reparse);
    }
    if current.file_index == 0
        || current.links == 0
        || current.attributes & 0x10 == 0
        || !expected.same_object(current)
    {
        return Err(Error::Changed);
    }
    ffi::ordinary_directory(file)
}
fn anchors(
    parsed: &AbsolutePath,
    max_handles: usize,
) -> Result<(Vec<Held>, String, String), Error> {
    if parsed.ancestor_count() + 3 > max_handles {
        return Err(Error::Limit);
    }
    let drive = owned(&parsed.as_str()[..3])?;
    super::ffi::fixed_drive(&drive).map_err(|_| Error::Unsupported)?;
    let mut held = Vec::new();
    held.try_reserve_exact(parsed.ancestor_count())
        .map_err(|_| Error::Allocation)?;
    let root = super::open(&drive, true).map_err(|_| Error::Open)?;
    super::ffi::ntfs_volume(&root).map_err(|_| Error::Unsupported)?;
    let identity = capture(&root, true)?;
    check_external(&root, identity)?;
    held.push(Held {
        file: root,
        identity,
        name: String::new(),
    });
    let mut parts = parsed.as_str()[3..].split('\\').peekable();
    let name = loop {
        let part = parts.next().ok_or(Error::Path)?;
        if parts.peek().is_none() {
            break owned(part)?;
        }
        let file = ffi::open(
            &held.last().ok_or(Error::Path)?.file,
            part,
            ffi::Mode::Directory,
        )?;
        let identity = capture(&file, true)?;
        if identity.volume != held[0].identity.volume {
            return Err(Error::Identity);
        }
        check_external(&file, identity)?;
        ffi::exact_name(&file, part)?;
        held.push(Held {
            file,
            identity,
            name: owned(part)?,
        });
    };
    if name.eq_ignore_ascii_case(".git") {
        return Err(Error::Path);
    }
    Ok((held, drive, name))
}

/// Read-only identity exclusion before output creation/adoption. This detects
/// ordinary drive-letter aliases by volume/file identity, including output
/// containing a protected input. The caller retains its supplied file leases.
pub fn protect_output(
    output: &Path,
    protected_directories: &[&Path],
    protected_files: &[&ReadFile],
) -> Result<(), Error> {
    if protected_directories.len() > 16 || protected_files.len() > 16 {
        return Err(Error::Limit);
    }
    let parsed = AbsolutePath::parse(output.to_str().ok_or(Error::Path)?, PathLimits::default())
        .map_err(|_| Error::Path)?;
    let supplied_handles = protected_files.iter().try_fold(0_usize, |n, v| {
        n.checked_add(v.retained_handles()).ok_or(Error::Limit)
    })?;
    let available = MAX_HANDLES
        .checked_sub(supplied_handles)
        .ok_or(Error::Limit)?;
    let (ancestors, _, name) = anchors(&parsed, available)?;
    let existing = match ffi::open(
        &ancestors.last().ok_or(Error::Path)?.file,
        &name,
        ffi::Mode::Directory,
    ) {
        Ok(file) => {
            let identity = capture(&file, true)?;
            check_external(&file, identity)?;
            ffi::exact_name(&file, &name)?;
            Some((file, identity))
        }
        Err(Error::Missing) => None,
        Err(e) => return Err(e),
    };
    for held in &ancestors {
        refuse_git(&held.file)?;
    }
    if let Some((file, _)) = &existing {
        refuse_git(file)?;
    }
    for path in protected_directories {
        let source_path =
            AbsolutePath::parse(path.to_str().ok_or(Error::Path)?, PathLimits::default())
                .map_err(|_| Error::Path)?;
        if ancestors
            .len()
            .checked_add(usize::from(existing.is_some()))
            .and_then(|v| v.checked_add(supplied_handles))
            .and_then(|v| v.checked_add(source_path.ancestor_count() + 2))
            .is_none_or(|v| v > MAX_HANDLES)
        {
            return Err(Error::Limit);
        }
        let source =
            super::ReadDirectory::open(path, PathLimits::default()).map_err(|_| Error::Open)?;
        source.verify_same_object().map_err(|_| Error::Changed)?;
        let identity = source.identity();
        if ancestors.iter().any(|v| v.identity.same_object(identity))
            || existing
                .as_ref()
                .is_some_and(|(_, v)| v.same_object(identity))
        {
            return Err(Error::Path);
        }
        if let Some((_, root)) = &existing
            && source
                .inner
                .ancestors
                .iter()
                .any(|v| v.identity.same_object(*root))
        {
            return Err(Error::Path);
        }
        source.verify_same_object().map_err(|_| Error::Changed)?;
    }
    for source in protected_files {
        source.verify_unchanged().map_err(|_| Error::Changed)?;
        if let Some((_, root)) = &existing
            && (source
                .ancestors
                .iter()
                .any(|v| v.identity.same_object(*root))
                || source.identity().same_object(*root))
        {
            return Err(Error::Path);
        }
        source.verify_unchanged().map_err(|_| Error::Changed)?;
    }
    for v in &ancestors {
        check_external(&v.file, v.identity)?;
    }
    Ok(())
}

fn refuse_git(directory: &File) -> Result<(), Error> {
    // A .git file is a worktree marker too. Any present or inaccessible marker
    // rejects output; only native name/path absence admits the ancestor.
    match ffi::open(directory, ".git", ffi::Mode::Directory) {
        Err(Error::Missing) => Ok(()),
        _ => Err(Error::Path),
    }
}
