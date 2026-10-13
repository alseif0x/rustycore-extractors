// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: bounded directory inventory identities and anchored reads.
// Identity/opening policy follows this repository's authored rooted MAPS source.
use super::*;
use std::{
    fs::{File, Metadata, OpenOptions},
    io::Read,
    os::{
        fd::{AsRawFd, RawFd},
        unix::{
            ffi::OsStrExt,
            fs::{MetadataExt, OpenOptionsExt},
        },
    },
};
const O_NONBLOCK: i32 = 0x800;
const O_DIRECTORY: i32 = 0x10000;
const O_NOFOLLOW: i32 = 0x20000;
pub(super) const SCRATCH: usize = 64 * 1024;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Identity {
    dev: u64,
    ino: u64,
    mode: u32,
    nlink: u64,
    uid: u32,
    gid: u32,
    len: u64,
    mtime: i64,
    mtime_ns: i64,
    ctime: i64,
    ctime_ns: i64,
}
impl Identity {
    fn of(m: &Metadata) -> Self {
        Self {
            dev: m.dev(),
            ino: m.ino(),
            mode: m.mode(),
            nlink: m.nlink(),
            uid: m.uid(),
            gid: m.gid(),
            len: m.len(),
            mtime: m.mtime(),
            mtime_ns: m.mtime_nsec(),
            ctime: m.ctime(),
            ctime_ns: m.ctime_nsec(),
        }
    }
}
struct Entry {
    root: usize,
    path: Vec<u8>,
    identity: Identity,
}
pub(super) struct Root {
    file: File,
    path: Vec<u8>,
    identity: Identity,
}
impl Root {
    pub fn new(path: &Path, b: &mut Budget) -> Result<Self, Error> {
        let raw = path.as_os_str().as_bytes();
        if raw.is_empty()
            || raw.len() > b.limits.root_bytes
            || raw.split(|&c| c == b'/').any(|p| p == b"." || p == b"..")
        {
            return Err(Error::Path);
        }
        let mut length = raw.len();
        while length > 1 && raw[length - 1] == b'/' {
            length -= 1;
        }
        let mut saved = Vec::new();
        b.reserve(&mut saved, length, length)?;
        saved.extend_from_slice(&raw[..length]);
        let path = Path::new(std::ffi::OsStr::from_bytes(&saved));
        for p in path.ancestors().filter(|p| !p.as_os_str().is_empty()) {
            b.metadata()?;
            let m = fs::symlink_metadata(p).map_err(|_| Error::Io)?;
            if !m.is_dir() || m.file_type().is_symlink() {
                return Err(Error::Path);
            }
        }
        b.metadata()?;
        let before = fs::symlink_metadata(path).map_err(|_| Error::Io)?;
        let file = open_dir(path)?;
        b.metadata()?;
        let held = file.metadata().map_err(|_| Error::InputChanged)?;
        b.metadata()?;
        let after = fs::symlink_metadata(path).map_err(|_| Error::InputChanged)?;
        let identity = Identity::of(&before);
        if identity != Identity::of(&held) || identity != Identity::of(&after) {
            return Err(Error::InputChanged);
        }
        Ok(Self {
            file,
            path: saved,
            identity,
        })
    }
    pub fn path(&self) -> &Path {
        Path::new(std::ffi::OsStr::from_bytes(&self.path))
    }
    pub fn anchored(&self) -> std::path::PathBuf {
        anchor(self.file.as_raw_fd())
    }
    fn verify(&self, b: &mut Budget) -> Result<(), Error> {
        b.metadata()?;
        let held = self.file.metadata().map_err(|_| Error::InputChanged)?;
        b.metadata()?;
        let named = fs::symlink_metadata(self.path()).map_err(|_| Error::InputChanged)?;
        if self.identity != Identity::of(&held) || self.identity != Identity::of(&named) {
            return Err(Error::InputChanged);
        }
        Ok(())
    }
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
    pub fn anchored(&self, root: usize) -> std::path::PathBuf {
        self.roots[root].anchored()
    }
    pub fn remember(
        &mut self,
        root: usize,
        relative: &[u8],
        metadata: &Metadata,
        b: &mut Budget,
    ) -> Result<(), Error> {
        if self.entries.len() >= b.limits.entries || relative.is_empty() || relative.len() > 511 {
            return Err(Error::Limit);
        }
        let mut path = Vec::new();
        b.reserve(&mut path, relative.len(), relative.len())?;
        path.extend_from_slice(relative);
        b.grow(&mut self.entries, b.limits.entries)?;
        self.entries.push(Entry {
            root,
            path,
            identity: Identity::of(metadata),
        });
        b.stats.entries += 1;
        Ok(())
    }
    pub fn canonical_order(&mut self) {
        self.entries
            .sort_unstable_by(|a, b| a.root.cmp(&b.root).then(a.path.cmp(&b.path)));
    }
    pub fn verify(&mut self, b: &mut Budget) -> Result<(), Error> {
        if self.failed {
            return Err(Error::InputChanged);
        }
        let result = (|| {
            for r in &self.roots {
                r.verify(b)?;
            }
            for e in &self.entries {
                b.metadata()?;
                let path = self
                    .anchored(e.root)
                    .join(std::ffi::OsStr::from_bytes(&e.path));
                let named = fs::symlink_metadata(path).map_err(|_| Error::InputChanged)?;
                if Identity::of(&named) != e.identity {
                    return Err(Error::InputChanged);
                }
            }
            for r in &self.roots {
                r.verify(b)?;
            }
            Ok(())
        })();
        if matches!(result, Err(Error::InputChanged)) {
            self.failed = true;
        }
        result
    }
    /// Binds a requested artifact (and its directory) to the original complete
    /// enumeration. Unused entries are checked at batch boundaries, rather than
    /// rescanning the entire package for each tile. This is metadata admission,
    /// not an atomic snapshot or a replacement for the rooted provider's reads.
    pub fn artifact(&self, root: usize, relative: &[u8], b: &mut Budget) -> Result<(), Error> {
        if self.failed {
            return Err(Error::InputChanged);
        }
        self.roots[root].verify(b)?;
        if let Some(i) = relative.iter().position(|&c| c == b'/') {
            self.entry(root, &relative[..i], b)?;
        }
        self.entry(root, relative, b)?;
        self.roots[root].verify(b)
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
        let path = self
            .anchored(root)
            .join(std::ffi::OsStr::from_bytes(relative));
        b.metadata()?;
        let named = match fs::symlink_metadata(path) {
            Ok(m) => Some(m),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(_) => return Err(Error::InputChanged),
        };
        match (position, named) {
            (Ok(i), Some(m)) if self.entries[i].identity == Identity::of(&m) => Ok(()),
            (Err(_), None) => Ok(()),
            _ => Err(Error::InputChanged),
        }
    }
    pub fn read_list(
        &self,
        root: usize,
        relative: &[u8],
        named: &Metadata,
        b: &mut Budget,
    ) -> Result<Vec<u8>, Error> {
        if !named.is_file() || named.file_type().is_symlink() {
            return Err(Error::Kind);
        }
        if named.len() > 4108 {
            return Err(Error::Inventory(super::super::Error::TileList));
        }
        let path = self
            .anchored(root)
            .join(std::ffi::OsStr::from_bytes(relative));
        let mut file = OpenOptions::new()
            .read(true)
            .custom_flags(O_NONBLOCK | O_NOFOLLOW)
            .open(&path)
            .map_err(|_| Error::Io)?;
        b.metadata()?;
        let before = file.metadata().map_err(|_| Error::InputChanged)?;
        let identity = Identity::of(named);
        if identity != Identity::of(&before) {
            return Err(Error::InputChanged);
        }
        let size = named.len() as usize;
        let mut bytes = Vec::new();
        b.reserve(&mut bytes, size, size)?;
        bytes.resize(size, 0);
        file.read_exact(&mut bytes)
            .map_err(|_| Error::InputChanged)?;
        let mut eof = [0];
        if file.read(&mut eof).map_err(|_| Error::Io)? != 0 {
            return Err(Error::InputChanged);
        }
        b.metadata()?;
        let held = file.metadata().map_err(|_| Error::InputChanged)?;
        b.metadata()?;
        let after = fs::symlink_metadata(path).map_err(|_| Error::InputChanged)?;
        if identity != Identity::of(&held) || identity != Identity::of(&after) {
            return Err(Error::InputChanged);
        }
        b.stats.tilelist_bytes += size;
        Ok(bytes)
    }
}
pub(super) fn open_dir(path: &Path) -> Result<File, Error> {
    OpenOptions::new()
        .read(true)
        .custom_flags(O_DIRECTORY | O_NOFOLLOW | O_NONBLOCK)
        .open(path)
        .map_err(|_| Error::Kind)
}
pub(super) fn directory(fd: &File) -> std::path::PathBuf {
    anchor(fd.as_raw_fd())
}
fn anchor(fd: RawFd) -> std::path::PathBuf {
    std::path::PathBuf::from(format!("/proc/self/fd/{fd}"))
}
pub(super) fn same(a: &Metadata, b: &Metadata) -> bool {
    Identity::of(a) == Identity::of(b)
}
