// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original bounded Windows navigation inventory leases.
use super::*;
use forever_platform::{
    Error as PE,
    path::Limits as PL,
    windows::{Identity, LinkPolicy, ReadDirectory, ReadFile, operator_path},
};
use std::{fs, path::PathBuf};
pub(super) const SCRATCH: usize = 256 * 1024;
fn platform(e: PE) -> Error {
    match e {
        PE::Limit => Error::Limit,
        PE::Allocation => Error::Allocation,
        PE::Changed | PE::Read | PE::Identity => Error::InputChanged,
        PE::Path => Error::Path,
        PE::FileType | PE::Reparse | PE::HardLinks => Error::Kind,
        _ => Error::Io,
    }
}
fn known(e: PE) -> Error {
    match e {
        PE::Limit | PE::Allocation => platform(e),
        _ => Error::InputChanged,
    }
}
fn component(name: &[u8]) -> Result<&str, Error> {
    if name.is_empty()
        || name.len() > 255
        || name == b"."
        || name == b".."
        || !name.iter().all(|&c| (0x20..=0x7e).contains(&c))
        || name.iter().any(|c| b"<>:\"/\\|?*".contains(c))
        || matches!(name.last(), Some(b'.' | b' '))
    {
        return Err(Error::Filename);
    }
    let text = std::str::from_utf8(name).map_err(|_| Error::Filename)?;
    // The public platform parser rejects DOS device components and ambiguous names.
    Ok(text)
}
pub(super) struct Root {
    path: Vec<u8>,
    identity: Identity,
}
impl Root {
    pub fn new(path: &Path, b: &mut Budget) -> Result<Self, Error> {
        let text = path.to_str().ok_or(Error::Path)?;
        if text.is_empty() || text.len() > b.limits.root_bytes {
            return Err(Error::Path);
        }
        operator_path(path, PL::default()).map_err(platform)?;
        let mut saved = Vec::new();
        b.reserve(&mut saved, text.len(), text.len())?;
        saved.extend_from_slice(text.as_bytes());
        b.platform(path, 2)?;
        let lease = ReadDirectory::open(path, PL::default()).map_err(platform)?;
        let identity = lease.identity();
        lease.verify_unchanged().map_err(platform)?;
        drop(lease);
        Ok(Self {
            path: saved,
            identity,
        })
    }
    pub fn path(&self) -> &Path {
        Path::new(std::str::from_utf8(&self.path).expect("validated root"))
    }
    fn verify(&self, b: &mut Budget) -> Result<(), Error> {
        identity(self.path(), true, Some(self.identity), b).map(|_| ())
    }
}
fn identity(
    path: &Path,
    directory: bool,
    expected: Option<Identity>,
    b: &mut Budget,
) -> Result<Identity, Error> {
    b.platform(path, 2)?;
    let map = |e| {
        if expected.is_some() {
            known(e)
        } else {
            platform(e)
        }
    };
    let got = if directory {
        let lease = ReadDirectory::open(path, PL::default()).map_err(map)?;
        let id = lease.identity();
        let result = lease.verify_unchanged().map_err(known);
        drop(lease);
        result?;
        id
    } else {
        let lease = ReadFile::open(path, PL::default(), LinkPolicy::Allow).map_err(map)?;
        let id = lease.identity();
        let result = lease.verify_unchanged().map_err(known);
        drop(lease);
        result?;
        id
    };
    if expected.is_some_and(|e| e != got) {
        return Err(Error::InputChanged);
    }
    Ok(got)
}
struct Entry {
    root: usize,
    path: Vec<u8>,
    identity: Identity,
    directory: bool,
}
pub(super) struct Observed {
    pub name: Vec<u8>,
    pub identity: Identity,
    pub directory: bool,
}
pub(super) fn release_scan(entries: Vec<Observed>, b: &mut Budget) {
    for e in &entries {
        b.release(&e.name);
    }
    b.release(&entries);
}
pub(super) struct Guard {
    roots: [Root; 2],
    entries: Vec<Entry>,
    failed: bool,
}
impl Guard {
    pub fn new(maps: Root, vmaps: Root) -> Self {
        Self {
            roots: [maps, vmaps],
            entries: Vec::new(),
            failed: false,
        }
    }
    pub fn path(&self, root: usize) -> &Path {
        self.roots[root].path()
    }
    fn physical(&self, root: usize, relative: &[u8]) -> Result<PathBuf, Error> {
        if relative.len() > 511 {
            return Err(Error::Limit);
        }
        let mut path = self.path(root).to_path_buf();
        if !relative.is_empty() {
            for part in relative.split(|&c| c == b'/') {
                path.push(component(part)?);
            }
        }
        operator_path(&path, PL::default()).map_err(platform)
    }
    fn remember(
        &mut self,
        root: usize,
        relative: &[u8],
        id: Identity,
        directory: bool,
        b: &mut Budget,
    ) -> Result<(), Error> {
        if self.entries.len() >= b.limits.entries {
            return Err(Error::Limit);
        }
        let mut path = Vec::new();
        b.reserve(&mut path, relative.len(), relative.len())?;
        path.extend_from_slice(relative);
        b.grow(&mut self.entries, b.limits.entries)?;
        self.entries.push(Entry {
            root,
            path,
            identity: id,
            directory,
        });
        b.stats.entries += 1;
        Ok(())
    }
    pub fn canonical_order(&mut self) {
        self.entries
            .sort_unstable_by(|a, b| a.root.cmp(&b.root).then(a.path.cmp(&b.path)));
    }
    pub fn scan(
        &mut self,
        root: usize,
        relative: &[u8],
        b: &mut Budget,
    ) -> Result<Vec<Observed>, Error> {
        self.verify_roots(b)?;
        let path = self.physical(root, relative)?;
        let expected = if relative.is_empty() {
            self.roots[root].identity
        } else {
            self.entries
                .iter()
                .find(|e| e.root == root && e.path == relative && e.directory)
                .ok_or(Error::InputChanged)?
                .identity
        };
        // Reserve open + final verification before enumeration so a collection
        // limit cannot consume the final lease recheck allowance.
        b.platform(&path, 2)?;
        let lease = ReadDirectory::open(&path, PL::default()).map_err(known)?;
        if lease.identity() != expected {
            return Err(Error::InputChanged);
        }
        let mut entries = Vec::new();
        let collected = (|| {
            b.metadata()?;
            let iterator = fs::read_dir(&path).map_err(|_| Error::Io)?;
            // Iterator is dropped at closure exit BEFORE named lease verification.
            for item in iterator {
                b.metadata()?;
                let item = item.map_err(|_| Error::Io)?;
                if self
                    .entries
                    .len()
                    .checked_add(entries.len())
                    .ok_or(Error::Limit)?
                    >= b.limits.entries
                {
                    return Err(Error::Limit);
                }
                let raw = item.file_name();
                let text = raw.to_str().ok_or(Error::Filename)?;
                let name = text.as_bytes();
                component(name)?;
                b.metadata()?;
                let kind = item.file_type().map_err(|_| Error::Io)?;
                if kind.is_symlink() || (!kind.is_file() && !kind.is_dir()) {
                    return Err(Error::Kind);
                }
                let mut saved = Vec::new();
                b.reserve(&mut saved, name.len(), name.len())?;
                saved.extend_from_slice(name);
                b.grow(&mut entries, b.limits.entries)?;
                entries.push(Observed {
                    name: saved,
                    identity: expected,
                    directory: kind.is_dir(),
                });
            }
            Ok(())
        })();
        let final_check = lease.verify_unchanged().map_err(known);
        drop(lease);
        let result = (|| {
            final_check?;
            collected?;
            for entry in &mut entries {
                let mut name = Vec::new();
                let len = relative.len() + usize::from(!relative.is_empty()) + entry.name.len();
                b.reserve(&mut name, len, len)?;
                name.extend_from_slice(relative);
                if !relative.is_empty() {
                    name.push(b'/');
                }
                name.extend_from_slice(&entry.name);
                let result = (|| {
                    let path = self.physical(root, &name)?;
                    entry.identity = identity(&path, entry.directory, None, b)?;
                    self.remember(root, &name, entry.identity, entry.directory, b)
                })();
                b.release(&name);
                result?;
            }
            identity(&path, true, Some(expected), b)?;
            self.verify_roots(b)
        })();
        if let Err(e) = result {
            release_scan(entries, b);
            if e.changed() {
                self.failed = true;
            }
            return Err(e);
        }
        Ok(entries)
    }
    pub fn verify(&mut self, b: &mut Budget) -> Result<(), Error> {
        let result = (|| {
            self.verify_roots(b)?;
            for e in &self.entries {
                identity(
                    &self.physical(e.root, &e.path)?,
                    e.directory,
                    Some(e.identity),
                    b,
                )?;
            }
            self.verify_roots(b)
        })();
        if result.is_err_and(Error::changed) {
            self.failed = true;
        }
        result
    }
    pub fn verify_roots(&self, b: &mut Budget) -> Result<(), Error> {
        if self.failed {
            return Err(Error::InputChanged);
        }
        for root in &self.roots {
            root.verify(b)?;
        }
        Ok(())
    }
    fn entry(&self, root: usize, relative: &[u8], b: &mut Budget) -> Result<(), Error> {
        let position = self
            .entries
            .binary_search_by(|e| e.root.cmp(&root).then(e.path.as_slice().cmp(relative)));
        let path = self.physical(root, relative)?;
        if let Ok(i) = position {
            identity(
                &path,
                self.entries[i].directory,
                Some(self.entries[i].identity),
                b,
            )?;
            return Ok(());
        }
        b.platform(&path, 2)?;
        match ReadFile::open(&path, PL::default(), LinkPolicy::Allow) {
            Err(PE::Missing) => Ok(()),
            Err(PE::Limit | PE::Allocation) => Err(Error::Limit),
            _ => Err(Error::InputChanged),
        }
    }
    pub fn artifact(&self, root: usize, relative: &[u8], b: &mut Budget) -> Result<(), Error> {
        self.verify_roots(b)?;
        if let Some(i) = relative.iter().position(|&c| c == b'/') {
            self.entry(root, &relative[..i], b)?;
        }
        self.entry(root, relative, b)?;
        self.verify_roots(b)
    }
    pub fn read_list(
        &self,
        root: usize,
        relative: &[u8],
        expected: Identity,
        b: &mut Budget,
    ) -> Result<Vec<u8>, Error> {
        self.verify_roots(b)?;
        let path = self.physical(root, relative)?;
        b.platform(&path, 4)?; // open, complete read, EOF, final identity
        let lease = ReadFile::open(&path, PL::default(), LinkPolicy::Allow).map_err(known)?;
        let mut bytes = Vec::new();
        let result = (|| {
            if lease.identity() != expected {
                return Err(Error::InputChanged);
            }
            if expected.size > 4108 {
                return Err(Error::Inventory(super::super::Error::TileList));
            }
            let size = expected.size as usize;
            b.reserve(&mut bytes, size, size)?;
            bytes.resize(size, 0);
            if size != 0 {
                lease.read_exact_at(0, &mut bytes).map_err(known)?;
            }
            lease.verify_eof().map_err(known)?;
            Ok(())
        })();
        let after = lease.verify_unchanged().map_err(known);
        drop(lease);
        let result = after.and(result);
        // Preserve already observed mutation/read failure over a later budget refusal.
        let root_result = self.verify_roots(b);
        let result = match result {
            Err(e) if e.changed() => Err(e),
            other => root_result.and(other),
        };
        if let Err(e) = result {
            b.release(&bytes);
            return Err(e);
        }
        b.stats.tilelist_bytes += bytes.len();
        Ok(bytes)
    }
}
