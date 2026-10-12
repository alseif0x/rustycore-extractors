// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-12: original bounded NTFS private-output foundation.
//! Exclusive relative creation under a retained readonly parent. The protected
//! DACL admits the process owner SID only; it does not authenticate that owner
//! or exclude administrators. Directory keepers deny delete sharing, but permit
//! write sharing: observed identity/attribute/security changes poison the root.
//! Checks are not atomic namespace snapshots. Files deny competing write/delete
//! opens, but permit compatible reads. This is not a package publication,
//! recovery, disk reservation, crash-durability or Windows product protocol.

use super::{Identity, ReadDirectory};
use std::{
    cell::Cell,
    fs::File,
    io::Write,
    sync::atomic::{AtomicU64, Ordering},
};

#[allow(unsafe_code)]
mod ffi;
#[cfg(test)]
mod tests;

pub const WINDOWS_OUTPUT_PROFILE: &str = "forever-windows-private-output-v1";
const MAX_OBJECTS: usize = 128;
const MAX_DEPTH: usize = 16;
const MAX_NAME: usize = 64;
const MAX_FILE: u64 = 64 * 1024 * 1024;
const MAX_TOTAL: u64 = 256 * 1024 * 1024;
const MAX_WRITE: usize = 1024 * 1024;
const MAX_OPERATIONS: usize = 16_384;
static NEXT_OWNER: AtomicU64 = AtomicU64::new(1);

/// Static, redacted diagnostics: no paths, SID bytes or native error strings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Path,
    Limit,
    Allocation,
    Token,
    Impersonation,
    Privacy,
    Exists,
    Open,
    Identity,
    Reparse,
    HardLinks,
    Changed,
    Write,
    Flush,
    Rename,
    Unsupported,
}
impl Error {
    pub fn code(self) -> &'static str {
        match self {
            Self::Path => "windows_output_path",
            Self::Limit => "windows_output_limit",
            Self::Allocation => "windows_output_allocation",
            Self::Token => "windows_output_token",
            Self::Impersonation => "windows_output_impersonation",
            Self::Privacy => "windows_output_privacy",
            Self::Exists => "windows_output_exists",
            Self::Open => "windows_output_open",
            Self::Identity => "windows_output_identity",
            Self::Reparse => "windows_output_reparse",
            Self::HardLinks => "windows_output_hard_links",
            Self::Changed => "windows_output_changed",
            Self::Write => "windows_output_write",
            Self::Flush => "windows_output_flush",
            Self::Rename => "windows_output_rename",
            Self::Unsupported => "windows_output_unsupported",
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}
impl std::error::Error for Error {}

/// Downward-only logical admission limits, not RSS or physical allocation caps.
/// Objects include the root (depth zero). Operations count mutating attempts
/// admitted after guards, including root creation; public verification is excluded.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub max_objects: usize,
    pub max_depth: usize,
    pub max_name_bytes: usize,
    pub max_file_bytes: u64,
    pub max_total_bytes: u64,
    pub max_write_bytes: usize,
    pub max_operations: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_objects: MAX_OBJECTS,
            max_depth: MAX_DEPTH,
            max_name_bytes: MAX_NAME,
            max_file_bytes: MAX_FILE,
            max_total_bytes: MAX_TOTAL,
            max_write_bytes: MAX_WRITE,
            max_operations: MAX_OPERATIONS,
        }
    }
}
impl Limits {
    fn validate(self) -> Result<Self, Error> {
        if self.max_objects == 0
            || self.max_objects > MAX_OBJECTS
            || self.max_depth == 0
            || self.max_depth > MAX_DEPTH
            || self.max_name_bytes == 0
            || self.max_name_bytes > MAX_NAME
            || self.max_file_bytes > MAX_FILE
            || self.max_total_bytes > MAX_TOTAL
            || self.max_write_bytes == 0
            || self.max_write_bytes > MAX_WRITE
            || self.max_operations == 0
            || self.max_operations > MAX_OPERATIONS
        {
            return Err(Error::Limit);
        }
        Ok(self)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Directory {
    owner: u64,
    index: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OutputFile {
    owner: u64,
    index: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stats {
    pub objects: usize,
    pub retained_handles: usize,
    pub charged_bytes: u64,
    pub operations: usize,
}
#[derive(Clone, Copy, PartialEq, Eq)]
struct Name {
    bytes: [u8; MAX_NAME],
    len: usize,
}
impl Name {
    fn parse(name: &str, max: usize) -> Result<Self, Error> {
        let b = name.as_bytes();
        if b.is_empty()
            || b.len() > max
            || b.len() > MAX_NAME
            || b.iter()
                .any(|b| !matches!(b,b'a'..=b'z'|b'0'..=b'9'|b'.'|b'_'|b'-'))
            || name == "."
            || name == ".."
            || name == ".git"
            || name.ends_with('.')
        {
            return Err(Error::Path);
        }
        let stem = name.split('.').next().ok_or(Error::Path)?;
        if matches!(stem, "con" | "prn" | "aux" | "nul")
            || ((stem.starts_with("com") || stem.starts_with("lpt"))
                && stem.len() == 4
                && matches!(stem.as_bytes()[3], b'1'..=b'9'))
        {
            return Err(Error::Path);
        }
        let mut result = Self {
            bytes: [0; MAX_NAME],
            len: b.len(),
        };
        result.bytes[..b.len()].copy_from_slice(b);
        Ok(result)
    }
    fn text(&self) -> &str {
        std::str::from_utf8(&self.bytes[..self.len]).expect("validated ASCII")
    }
}
enum Kind {
    Directory { keeper: File, target: File },
    File(File),
}
struct Entry {
    kind: Kind,
    identity: Identity,
    name: Name,
    parent: Option<usize>,
    depth: usize,
    path_units: usize,
}
impl Entry {
    fn file(&self) -> &File {
        match &self.kind {
            Kind::Directory { keeper, .. } => keeper,
            Kind::File(file) => file,
        }
    }
    fn directory(&self) -> bool {
        matches!(self.kind, Kind::Directory { .. })
    }
}

/// Owns every created handle; failures retain filesystem objects. No adoption,
/// listing, deletion, directory rename or recovery is provided. Opaque tokens
/// remain usable after file rename and cannot cross roots. A root is not Sync.
pub struct PrivateRoot<'parent> {
    parent: &'parent ReadDirectory,
    sid: ffi::Sid,
    entries: Vec<Entry>,
    limits: Limits,
    owner: u64,
    invalid: Cell<bool>,
    charged_bytes: u64,
    operations: usize,
}
impl<'parent> PrivateRoot<'parent> {
    pub fn create(
        parent: &'parent ReadDirectory,
        name: &str,
        limits: Limits,
    ) -> Result<Self, Error> {
        let limits = limits.validate()?;
        let name = Name::parse(name, limits.max_name_bytes)?;
        let path_units = path_units(parent.inner.path.as_str().encode_utf16().count(), name.len)?;
        let owner = NEXT_OWNER
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| v.checked_add(1))
            .map_err(|_| Error::Limit)?;
        let mut entries = Vec::new();
        entries
            .try_reserve_exact(limits.max_objects)
            .map_err(|_| Error::Allocation)?;
        let sid = ffi::process_sid()?;
        parent.verify_same_object().map_err(|_| Error::Changed)?;
        ffi::ordinary_directory(&parent.inner.leaf.file)?;
        let keeper = ffi::create(&parent.inner.leaf.file, name.text(), true, &sid)?;
        let identity = capture(&keeper, true)?;
        admit(identity, true, parent.identity().volume)?;
        ffi::private(&keeper, &sid)?;
        ffi::ordinary_directory(&keeper)?;
        let target = ffi::probe(&parent.inner.leaf.file, name.text(), true)?;
        if !identity.same_object(capture(&target, true)?) {
            return Err(Error::Changed);
        }
        entries.push(Entry {
            kind: Kind::Directory { keeper, target },
            identity,
            name,
            parent: None,
            depth: 0,
            path_units,
        });
        let result = Self {
            parent,
            sid,
            entries,
            limits,
            owner,
            invalid: Cell::new(false),
            charged_bytes: 0,
            operations: 1,
        };
        result.verify_unchanged()?;
        Ok(result)
    }
    pub fn root(&self) -> Directory {
        Directory {
            owner: self.owner,
            index: 0,
        }
    }
    pub fn stats(&self) -> Stats {
        Stats {
            objects: self.entries.len(),
            retained_handles: self
                .entries
                .iter()
                .map(|e| if e.directory() { 2 } else { 1 })
                .sum(),
            charged_bytes: self.charged_bytes,
            operations: self.operations,
        }
    }
    fn directory_index(&self, token: Directory) -> Result<usize, Error> {
        if token.owner != self.owner || !self.entries.get(token.index).is_some_and(Entry::directory)
        {
            return Err(Error::Token);
        }
        Ok(token.index)
    }
    fn file_index(&self, token: OutputFile) -> Result<usize, Error> {
        if token.owner != self.owner
            || !self
                .entries
                .get(token.index)
                .is_some_and(|e| !e.directory())
        {
            return Err(Error::Token);
        }
        Ok(token.index)
    }
    fn target(&self, index: Option<usize>) -> &File {
        match index {
            None => &self.parent.inner.leaf.file,
            Some(i) => match &self.entries[i].kind {
                Kind::Directory { target, .. } => target,
                Kind::File(_) => unreachable!("validated directory"),
            },
        }
    }
    fn available_operation(&self) -> Result<(), Error> {
        if self.invalid.get() {
            Err(Error::Changed)
        } else if self.operations >= self.limits.max_operations {
            Err(Error::Limit)
        } else {
            Ok(())
        }
    }
    fn fail<T>(&self, result: Result<T, Error>) -> Result<T, Error> {
        if result.is_err() {
            self.invalid.set(true)
        }
        result
    }
    /// Metadata/security observations, never an atomic namespace snapshot.
    /// Any observed failure permanently poisons the root, including after repair.
    pub fn verify_unchanged(&self) -> Result<(), Error> {
        if self.invalid.get() {
            return Err(Error::Changed);
        }
        self.fail(self.check())
    }
    fn check(&self) -> Result<(), Error> {
        if ffi::process_sid()? != self.sid {
            return Err(Error::Token);
        }
        self.parent
            .verify_same_object()
            .map_err(|_| Error::Changed)?;
        ffi::ordinary_directory(&self.parent.inner.leaf.file)?;
        for e in &self.entries {
            self.check_entry(e, e.file())?;
            if let Kind::Directory { target, .. } = &e.kind {
                self.check_entry(e, target)?;
            }
            let named = ffi::probe(self.target(e.parent), e.name.text(), e.directory())?;
            self.check_entry(e, &named)?;
        }
        self.parent.verify_same_object().map_err(|_| Error::Changed)
    }
    fn check_entry(&self, e: &Entry, file: &File) -> Result<(), Error> {
        let current = capture(file, e.directory())?;
        admit(current, e.directory(), self.parent.identity().volume)?;
        if if e.directory() {
            !e.identity.same_object(current)
        } else {
            e.identity != current
        } {
            return Err(Error::Changed);
        }
        ffi::private(file, &self.sid)?;
        if e.directory() {
            ffi::ordinary_directory(file)?;
        }
        Ok(())
    }
    fn new_entry(
        &mut self,
        parent: Directory,
        name: &str,
        directory: bool,
    ) -> Result<usize, Error> {
        let p = self.directory_index(parent)?;
        let name = Name::parse(name, self.limits.max_name_bytes)?;
        self.available_operation()?;
        let depth = self.entries[p].depth.checked_add(1).ok_or(Error::Limit)?;
        let path_units = path_units(self.entries[p].path_units, name.len)?;
        if self.entries.len() >= self.limits.max_objects || depth > self.limits.max_depth {
            return Err(Error::Limit);
        }
        if self
            .entries
            .iter()
            .any(|e| e.parent == Some(p) && e.name == name)
        {
            return Err(Error::Exists);
        }
        self.verify_unchanged()?;
        self.operations += 1;
        let result = (|| {
            let file = ffi::create(self.target(Some(p)), name.text(), directory, &self.sid)?;
            let identity = capture(&file, directory)?;
            admit(identity, directory, self.parent.identity().volume)?;
            ffi::private(&file, &self.sid)?;
            let kind = if directory {
                ffi::ordinary_directory(&file)?;
                let target = ffi::probe(self.target(Some(p)), name.text(), true)?;
                if !identity.same_object(capture(&target, true)?) {
                    return Err(Error::Changed);
                }
                Kind::Directory {
                    keeper: file,
                    target,
                }
            } else {
                Kind::File(file)
            };
            Ok(Entry {
                kind,
                identity,
                name,
                parent: Some(p),
                depth,
                path_units,
            })
        })();
        let entry = self.fail(result)?;
        let index = self.entries.len();
        self.entries.push(entry);
        self.verify_unchanged()?;
        Ok(index)
    }
    pub fn create_directory(&mut self, parent: Directory, name: &str) -> Result<Directory, Error> {
        let index = self.new_entry(parent, name, true)?;
        Ok(Directory {
            owner: self.owner,
            index,
        })
    }
    pub fn create_file(&mut self, parent: Directory, name: &str) -> Result<OutputFile, Error> {
        let index = self.new_entry(parent, name, false)?;
        Ok(OutputFile {
            owner: self.owner,
            index,
        })
    }
    /// Charges the entire request before writing. Partial failures never refund
    /// admission bytes and poison the root. Each append flushes the file handle.
    pub fn append(&mut self, file: OutputFile, bytes: &[u8]) -> Result<(), Error> {
        let i = self.file_index(file)?;
        self.available_operation()?;
        let size = self.entries[i]
            .identity
            .size
            .checked_add(bytes.len() as u64)
            .ok_or(Error::Limit)?;
        let total = self
            .charged_bytes
            .checked_add(bytes.len() as u64)
            .ok_or(Error::Limit)?;
        if bytes.len() > self.limits.max_write_bytes
            || size > self.limits.max_file_bytes
            || total > self.limits.max_total_bytes
        {
            return Err(Error::Limit);
        }
        self.verify_unchanged()?;
        self.operations += 1;
        self.charged_bytes = total;
        let result = (|| {
            let mut writer = self.entries[i].file();
            writer.write_all(bytes).map_err(|_| Error::Write)?;
            ffi::flush(writer)?;
            Ok(())
        })();
        self.fail(result)?;
        self.settle(i, size)?;
        self.verify_unchanged()
    }
    fn settle(&mut self, i: usize, size: u64) -> Result<(), Error> {
        let result = (|| {
            let current = capture(self.entries[i].file(), false)?;
            admit(current, false, self.parent.identity().volume)?;
            let old = self.entries[i].identity;
            if old.volume != current.volume
                || old.file_index != current.file_index
                || old.creation_time != current.creation_time
                || current.size != size
                || ((old.attributes ^ current.attributes) & !0xa0) != 0
            {
                return Err(Error::Changed);
            }
            ffi::private(self.entries[i].file(), &self.sid)?;
            Ok(current)
        })();
        let current = self.fail(result)?;
        self.entries[i].identity = current;
        Ok(())
    }
    pub fn flush(&mut self, file: OutputFile) -> Result<(), Error> {
        let i = self.file_index(file)?;
        self.available_operation()?;
        self.verify_unchanged()?;
        self.operations += 1;
        self.fail(ffi::flush(self.entries[i].file()))?;
        let size = self.entries[i].identity.size;
        self.settle(i, size)?;
        self.verify_unchanged()
    }
    /// No overwrite, same-root file rename. The same opaque file token survives.
    /// The destination directory's retained target handle is used by the kernel.
    pub fn rename(
        &mut self,
        file: OutputFile,
        destination: Directory,
        name: &str,
    ) -> Result<(), Error> {
        let i = self.file_index(file)?;
        let p = self.directory_index(destination)?;
        let name = Name::parse(name, self.limits.max_name_bytes)?;
        self.available_operation()?;
        let depth = self.entries[p].depth.checked_add(1).ok_or(Error::Limit)?;
        let path_units = path_units(self.entries[p].path_units, name.len)?;
        if depth > self.limits.max_depth {
            return Err(Error::Limit);
        }
        if self
            .entries
            .iter()
            .any(|e| e.parent == Some(p) && e.name == name)
        {
            return Err(Error::Exists);
        }
        self.verify_unchanged()?;
        self.operations += 1;
        self.fail(ffi::rename(
            self.entries[i].file(),
            self.target(Some(p)),
            name.text(),
        ))?;
        self.entries[i].name = name;
        self.entries[i].parent = Some(p);
        self.entries[i].depth = depth;
        self.entries[i].path_units = path_units;
        let size = self.entries[i].identity.size;
        self.settle(i, size)?;
        self.verify_unchanged()
    }
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
        || i.attributes & (1 | 4 | 0x40 | 0x800 | 0x1000 | 0x4000) != 0
    {
        return Err(Error::Identity);
    }
    Ok(())
}

fn path_units(parent: usize, name: usize) -> Result<usize, Error> {
    let units = parent
        .checked_add(1)
        .and_then(|v| v.checked_add(name))
        .ok_or(Error::Limit)?;
    if units > 32760 {
        Err(Error::Limit)
    } else {
        Ok(units)
    }
}

#[cfg(test)]
mod privileged_tests {
    #[test]
    #[ignore = "requires native Windows symbolic-link privilege; run explicitly on NTFS"]
    fn privileged_reparse_root_is_never_adopted() {
        super::tests::privileged_reparse_root_is_never_adopted();
    }
}
