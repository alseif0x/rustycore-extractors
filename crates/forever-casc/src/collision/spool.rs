// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original Linux private ordinal payload repository.
//! Private intermediate files, never product artifacts. No implicit cleanup,
//! overwrite, process resume or publication. The caller explicitly owns the
//! source/key paths and must supply ALL protected inputs. No key bytes are read.
//! Read integrity is held/named identity + SHA256 + exact EOF; this is not an
//! atomic filesystem snapshot, publisher authentication or crash durability.

use super::payload::Error;
use crate::sha256::Sha256;
use std::{
    fs::{self, DirBuilder, File, Metadata, OpenOptions, Permissions},
    io::{Read, Write},
    os::{
        fd::AsRawFd,
        unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt},
    },
    path::{Component, Path, PathBuf},
};
const NOFOLLOW: i32 = 0x20000;
const DIRECTORY: i32 = 0x10000;
const NONBLOCK: i32 = 0x800;
const MAX_PATH: usize = 4096;
const MAX_DEPTH: usize = 64;
const MAX_INPUTS: usize = 16;
const MAX_FDINFO: usize = 256;
fn parse_mount(bytes: &[u8]) -> Result<u64, Error> {
    if bytes.is_empty()
        || bytes.len() > MAX_FDINFO
        || bytes.last() != Some(&b'\n')
        || !bytes.is_ascii()
    {
        return Err(Error::Mount);
    }
    let mut found = None;
    for line in bytes.split(|b| *b == b'\n').filter(|line| !line.is_empty()) {
        let colon = line.iter().position(|b| *b == b':').ok_or(Error::Mount)?;
        if &line[..colon] != b"mnt_id" {
            continue;
        }
        if found.is_some() {
            return Err(Error::Mount);
        }
        let value = &line[colon + 1..];
        let first = value
            .iter()
            .position(|b| !matches!(b, b' ' | b'\t'))
            .ok_or(Error::Mount)?;
        let digits = &value[first..];
        if digits.is_empty() || !digits.iter().all(u8::is_ascii_digit) {
            return Err(Error::Mount);
        }
        let id = digits.iter().try_fold(0u64, |n, b| {
            n.checked_mul(10)
                .and_then(|n| n.checked_add(u64::from(b - b'0')))
                .ok_or(Error::Mount)
        })?;
        if id == 0 {
            return Err(Error::Mount);
        }
        found = Some(id);
    }
    found.ok_or(Error::Mount)
}
fn mount_id(file: &File) -> Result<u64, Error> {
    let mut fdinfo =
        File::open(format!("/proc/self/fdinfo/{}", file.as_raw_fd())).map_err(|_| Error::Mount)?;
    let mut bytes = [0u8; MAX_FDINFO + 1];
    let mut used = 0;
    loop {
        let n = fdinfo.read(&mut bytes[used..]).map_err(|_| Error::Mount)?;
        if n == 0 {
            break;
        }
        used += n;
        if used > MAX_FDINFO {
            return Err(Error::Mount);
        }
    }
    parse_mount(&bytes[..used])
}
fn same_mount(source: u64, destination: u64) -> Result<(), Error> {
    if source == destination {
        Ok(())
    } else {
        Err(Error::Mount)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub max_records: usize,
    pub max_disk_bytes: usize,
    /// Current conversion payload/scratch OR one reload; never a cumulative cap.
    pub max_active_bytes: usize,
    /// Repository paths/entries plus compact collector vectors/names.
    pub max_descriptor_bytes: usize,
    pub max_reloads: usize,
    /// Cumulative rereads, including failed attempts; never refunded.
    pub max_reload_bytes: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_records: 4096,
            max_disk_bytes: 512 * 1024 * 1024,
            max_active_bytes: 256 * 1024 * 1024,
            max_descriptor_bytes: 16 * 1024 * 1024,
            max_reloads: 16384,
            max_reload_bytes: 1024 * 1024 * 1024,
        }
    }
}
impl Limits {
    fn validate(self) -> Result<(), Error> {
        let d = Self::default();
        if self.max_records > d.max_records
            || self.max_disk_bytes > d.max_disk_bytes
            || self.max_active_bytes > d.max_active_bytes
            || self.max_descriptor_bytes > d.max_descriptor_bytes
            || self.max_reloads > d.max_reloads
            || self.max_reload_bytes > d.max_reload_bytes
        {
            return Err(Error::InvalidLimits);
        }
        Ok(())
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
struct Identity {
    dev: u64,
    ino: u64,
    mode: u32,
    uid: u32,
    gid: u32,
    links: u64,
    len: u64,
    mt: i64,
    mn: i64,
    ct: i64,
    cn: i64,
}
impl Identity {
    fn of(m: &Metadata) -> Self {
        Self {
            dev: m.dev(),
            ino: m.ino(),
            mode: m.mode(),
            uid: m.uid(),
            gid: m.gid(),
            links: m.nlink(),
            len: m.len(),
            mt: m.mtime(),
            mn: m.mtime_nsec(),
            ct: m.ctime(),
            cn: m.ctime_nsec(),
        }
    }
    fn same_object(self, b: Self) -> bool {
        self.dev == b.dev && self.ino == b.ino
    }
    fn same_directory(self, b: Self) -> bool {
        self.same_object(b) && self.mode == b.mode && self.uid == b.uid && self.gid == b.gid
    }
}
struct Node {
    held: File,
    mount: u64,
    named: PathBuf,
    identity: Identity,
    directory: bool,
}
impl Node {
    fn check(&self, current_named: &File) -> Result<(), Error> {
        let held = Identity::of(&self.held.metadata().map_err(|_| Error::Changed)?);
        let named_meta = fs::symlink_metadata(&self.named).map_err(|_| Error::Changed)?;
        let named = Identity::of(&named_meta);
        let opened = Identity::of(&current_named.metadata().map_err(|_| Error::Changed)?);
        if named_meta.file_type().is_symlink()
            || mount_id(&self.held).map_err(|_| Error::Changed)? != self.mount
            || mount_id(current_named).map_err(|_| Error::Changed)? != self.mount
            || if self.directory {
                !self.identity.same_directory(held)
                    || !self.identity.same_directory(named)
                    || !self.identity.same_directory(opened)
            } else {
                self.identity != held || self.identity != named || self.identity != opened
            }
        {
            return Err(Error::Changed);
        }
        Ok(())
    }
}
struct Chain {
    nodes: Vec<Node>,
}
impl Chain {
    fn open(path: &Path, leaf_directory: bool) -> Result<Self, Error> {
        if !path.is_absolute()
            || path.as_os_str().len() > MAX_PATH
            || path
                .as_os_str()
                .as_encoded_bytes()
                .split(|b| *b == b'/')
                .any(|s| s == b"." || s == b"..")
        {
            return Err(Error::Path);
        }
        let count = path.components().count();
        if count > MAX_DEPTH || count == 0 {
            return Err(Error::Path);
        }
        let mut nodes = Vec::new();
        nodes
            .try_reserve_exact(count)
            .map_err(|_| Error::Allocation)?;
        let mut named = PathBuf::new();
        named
            .try_reserve_exact(path.as_os_str().len())
            .map_err(|_| Error::Allocation)?;
        for (i, part) in path.components().enumerate() {
            let directory = i + 1 != count || leaf_directory;
            let opened = match part {
                Component::RootDir => PathBuf::from("/"),
                Component::Normal(leaf) => {
                    let parent: &Node = nodes.last().ok_or(Error::Path)?;
                    anchor(&parent.held).join(leaf)
                }
                _ => return Err(Error::Path),
            };
            named.push(part.as_os_str());
            let file = OpenOptions::new()
                .read(true)
                .custom_flags(NOFOLLOW | NONBLOCK | if directory { DIRECTORY } else { 0 })
                .open(opened)
                .map_err(|_| Error::Path)?;
            let meta = file.metadata().map_err(|_| Error::Io)?;
            if if directory {
                !meta.is_dir()
            } else {
                !meta.is_file()
            } {
                return Err(Error::Path);
            }
            let mut path_copy = PathBuf::new();
            path_copy
                .try_reserve_exact(named.as_os_str().len())
                .map_err(|_| Error::Allocation)?;
            path_copy.push(&named);
            let mount = mount_id(&file)?;
            nodes.push(Node {
                held: file,
                mount,
                named: path_copy,
                identity: Identity::of(&meta),
                directory,
            });
        }
        let chain = Self { nodes };
        chain.check()?;
        Ok(chain)
    }
    fn check(&self) -> Result<(), Error> {
        let mut parent: Option<File> = None;
        for node in &self.nodes {
            let path = if let Some(file) = &parent {
                anchor(file).join(node.named.file_name().ok_or(Error::Changed)?)
            } else {
                PathBuf::from("/")
            };
            let named = OpenOptions::new()
                .read(true)
                .custom_flags(NOFOLLOW | NONBLOCK | if node.directory { DIRECTORY } else { 0 })
                .open(path)
                .map_err(|_| Error::Changed)?;
            node.check(&named)?;
            parent = Some(named);
        }
        Ok(())
    }
    fn leaf(&self) -> &Node {
        self.nodes.last().expect("nonempty checked chain")
    }
    fn bytes(&self) -> Result<usize, Error> {
        self.nodes.iter().try_fold(
            self.nodes
                .capacity()
                .checked_mul(size_of::<Node>())
                .ok_or(Error::Limit)?,
            |n, node| n.checked_add(node.named.capacity()).ok_or(Error::Limit),
        )
    }
}
fn anchor(f: &File) -> PathBuf {
    PathBuf::from(format!("/proc/self/fd/{}", f.as_raw_fd()))
}

/// Explicit ownership assertion: source_root belongs to the same LocalStore
/// supplied to SpoolingExtractor; key_files must include every supplied key file.
/// The API checks held/named identities and aliases without reading their bytes.
/// This declaration cannot infer the LocalStore's source from its private state.
pub struct ProtectedInputs {
    chains: Vec<Chain>,
}
impl ProtectedInputs {
    pub fn capture(source_root: &Path, key_files: &[&Path]) -> Result<Self, Error> {
        if key_files.len() >= MAX_INPUTS {
            return Err(Error::Limit);
        }
        let mut chains = Vec::new();
        chains
            .try_reserve_exact(key_files.len() + 1)
            .map_err(|_| Error::Allocation)?;
        chains.push(Chain::open(source_root, true)?);
        for path in key_files {
            chains.push(Chain::open(path, false)?);
        }
        Ok(Self { chains })
    }
    fn check(&self) -> Result<(), Error> {
        for c in &self.chains {
            c.check()?;
        }
        Ok(())
    }
    fn bytes(&self) -> Result<usize, Error> {
        self.chains.iter().try_fold(
            self.chains
                .capacity()
                .checked_mul(size_of::<Chain>())
                .ok_or(Error::Limit)?,
            |n, c| n.checked_add(c.bytes()?).ok_or(Error::Limit),
        )
    }
}
struct Entry {
    identity: Identity,
    mount: u64,
    digest: [u8; 32],
    bytes: usize,
}
#[derive(Clone, Copy)]
pub(crate) struct Record(usize);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stats {
    pub records: usize,
    pub disk_bytes: usize,
    pub reloads: usize,
    pub reload_bytes: usize,
    pub descriptor_bytes: usize,
    pub peak_reload_bytes: usize,
}
/// Exclusive newly-created namespace beneath an existing private (0700) parent.
/// Files use ordinal names and 0400 after write. No Drop cleanup or resume.
pub struct Spool {
    root: Chain,
    protected: ProtectedInputs,
    entries: Vec<Entry>,
    limits: Limits,
    disk_bytes: usize,
    reloads: usize,
    reload_bytes: usize,
    peak_reload_bytes: usize,
    failed: Option<Error>,
}
impl Spool {
    pub fn create(path: &Path, protected: ProtectedInputs, limits: Limits) -> Result<Self, Error> {
        limits.validate()?;
        let leaf = path.file_name().ok_or(Error::Path)?;
        if !path.is_absolute()
            || path.components().count() > MAX_DEPTH
            || path
                .as_os_str()
                .as_encoded_bytes()
                .split(|b| *b == b'/')
                .any(|p| p == b"." || p == b"..")
            || leaf.is_empty()
            || path.as_os_str().len() > MAX_PATH
        {
            return Err(Error::Path);
        }
        let parent = Chain::open(path.parent().ok_or(Error::Path)?, true)?;
        if parent.leaf().identity.mode & 0o7777 != 0o700 {
            return Err(Error::Privacy);
        }
        protected.check()?;
        same_mount(
            protected.chains.first().ok_or(Error::State)?.leaf().mount,
            parent.leaf().mount,
        )?;
        // Reject both lexical containment and directory-object aliases (including
        // a bind-mounted protected source in the destination's ancestor chain).
        for input in &protected.chains {
            if path.starts_with(&input.leaf().named)
                || parent
                    .nodes
                    .iter()
                    .any(|n| n.identity.same_object(input.leaf().identity))
            {
                return Err(Error::Alias);
            }
        }
        let projected = protected
            .bytes()?
            .checked_add(parent.bytes()?)
            .ok_or(Error::Limit)?;
        if projected > limits.max_descriptor_bytes {
            return Err(Error::Limit);
        }
        DirBuilder::new()
            .mode(0o700)
            .create(anchor(&parent.leaf().held).join(leaf))
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::AlreadyExists {
                    Error::Exists
                } else {
                    Error::Io
                }
            })?;
        // Namespace is retained on every later failure; never unlink by pathname.
        let root = Chain::open(path, true)?;
        same_mount(parent.leaf().mount, root.leaf().mount)?;
        parent.check()?;
        protected.check()?;
        if root.leaf().identity.mode & 0o7777 != 0o700
            || root.leaf().identity.uid != parent.leaf().identity.uid
        {
            return Err(Error::Privacy);
        }
        let mut out = Self {
            root,
            protected,
            entries: Vec::new(),
            limits,
            disk_bytes: 0,
            reloads: 0,
            reload_bytes: 0,
            peak_reload_bytes: 0,
            failed: None,
        };
        if out.resident_bytes()? > limits.max_descriptor_bytes {
            return Err(Error::Limit);
        }
        out.verify_unchanged()?;
        Ok(out)
    }
    pub const fn limits(&self) -> Limits {
        self.limits
    }
    pub fn stats(&self) -> Result<Stats, Error> {
        Ok(Stats {
            records: self.entries.len(),
            disk_bytes: self.disk_bytes,
            reloads: self.reloads,
            reload_bytes: self.reload_bytes,
            descriptor_bytes: self.resident_bytes()?,
            peak_reload_bytes: self.peak_reload_bytes,
        })
    }
    pub(crate) fn resident_bytes(&self) -> Result<usize, Error> {
        self.root
            .bytes()?
            .checked_add(self.protected.bytes()?)
            .and_then(|n| n.checked_add(self.entries.capacity().checked_mul(size_of::<Entry>())?))
            .ok_or(Error::Limit)
    }
    pub(crate) fn prepare_records(&mut self, count: usize, budget: usize) -> Result<(), Error> {
        if count > self.limits.max_records {
            return Err(Error::Limit);
        }
        if count > self.entries.capacity() {
            let before = self.resident_bytes()?;
            let full = count.checked_mul(size_of::<Entry>()).ok_or(Error::Limit)?;
            if full > budget
                || before.checked_add(full).ok_or(Error::Limit)? > self.limits.max_descriptor_bytes
            {
                return Err(Error::Limit);
            }
            self.entries
                .try_reserve_exact(count - self.entries.len())
                .map_err(|_| Error::Allocation)?;
            let actual = self
                .entries
                .capacity()
                .checked_mul(size_of::<Entry>())
                .ok_or(Error::Limit)?;
            if actual > budget
                || before.checked_add(actual).ok_or(Error::Limit)?
                    > self.limits.max_descriptor_bytes
            {
                return Err(Error::Limit);
            }
        }
        Ok(())
    }
    pub(crate) const fn failure(&self) -> Option<Error> {
        self.failed
    }
    fn path(&self, ordinal: usize) -> PathBuf {
        anchor(&self.root.leaf().held).join(format!("{ordinal:08x}.payload"))
    }
    fn file(&self, ordinal: usize) -> Result<File, Error> {
        OpenOptions::new()
            .read(true)
            .custom_flags(NOFOLLOW | NONBLOCK)
            .open(self.path(ordinal))
            .map_err(|_| Error::Changed)
    }
    fn check_entry(&self, ordinal: usize, file: &File) -> Result<(), Error> {
        let entry = self.entries.get(ordinal).ok_or(Error::State)?;
        let expected = entry.identity;
        let current_named = self.file(ordinal)?;
        if mount_id(file).map_err(|_| Error::Changed)? != entry.mount
            || mount_id(&current_named).map_err(|_| Error::Changed)? != entry.mount
            || Identity::of(&current_named.metadata().map_err(|_| Error::Changed)?) != expected
        {
            return Err(Error::Changed);
        }
        let held = file.metadata().map_err(|_| Error::Changed)?;
        let named = fs::symlink_metadata(self.path(ordinal)).map_err(|_| Error::Changed)?;
        if !held.is_file()
            || !named.is_file()
            || named.file_type().is_symlink()
            || expected.links != 1
            || Identity::of(&held) != expected
            || Identity::of(&named) != expected
        {
            return Err(Error::Changed);
        }
        Ok(())
    }
    /// Metadata-only boundary verification. Each actual read additionally hashes
    /// the complete record and checks EOF while retaining its file descriptor.
    fn check(&self) -> Result<(), Error> {
        let root = self.root.check();
        let protected = self.protected.check();
        let mut entries = Ok(());
        for i in 0..self.entries.len() {
            let checked = self.file(i).and_then(|f| self.check_entry(i, &f));
            if let Err(e) = checked {
                entries = Err(e);
            }
        }
        // All independent guard groups ran before any return or sticky error.
        root.and(protected)
            .and(entries)
            .and(self.failed.map_or(Ok(()), Err))
    }
    pub fn verify_unchanged(&mut self) -> Result<(), Error> {
        let result = self.check();
        if let Err(e) = result {
            if e == Error::Changed {
                self.failed = Some(e)
            } else {
                self.failed.get_or_insert(e);
            }
        }
        result
    }
    fn finish<T>(&mut self, result: Result<T, Error>) -> Result<T, Error> {
        let guard = self.check();
        let result = guard.and(result);
        if let Err(e) = result {
            if e == Error::Changed {
                self.failed = Some(e)
            } else {
                self.failed.get_or_insert(e);
            }
        }
        result
    }
    fn check_created(
        &self,
        ordinal: usize,
        file: &File,
        initial: Identity,
        mode: u32,
    ) -> Result<Identity, Error> {
        let held = file.metadata().map_err(|_| Error::Changed)?;
        let named = fs::symlink_metadata(self.path(ordinal)).map_err(|_| Error::Changed)?;
        let named_file = self.file(ordinal)?;
        let expected_mount = self.root.leaf().mount;
        if mount_id(file).map_err(|_| Error::Changed)? != expected_mount
            || mount_id(&named_file).map_err(|_| Error::Changed)? != expected_mount
            || Identity::of(&named_file.metadata().map_err(|_| Error::Changed)?)
                != Identity::of(&held)
        {
            return Err(Error::Changed);
        }
        let current = Identity::of(&held);
        if !held.is_file()
            || !named.is_file()
            || named.file_type().is_symlink()
            || !initial.same_object(current)
            || initial.uid != current.uid
            || initial.gid != current.gid
            || current.links != 1
            || current.mode & 0o7777 != mode
            || current != Identity::of(&named)
        {
            return Err(Error::Changed);
        }
        Ok(current)
    }
    pub(crate) fn write(
        &mut self,
        size: usize,
        writer: impl FnOnce(&mut RecordWriter<'_>) -> Result<(), Error>,
    ) -> Result<Record, Error> {
        let result = (|| {
            self.verify_unchanged()?;
            if self.entries.len() >= self.limits.max_records || size > self.limits.max_active_bytes
            {
                return Err(Error::Limit);
            }
            let disk = self.disk_bytes.checked_add(size).ok_or(Error::Limit)?;
            if disk > self.limits.max_disk_bytes {
                return Err(Error::Limit);
            }
            let descriptors = self
                .limits
                .max_descriptor_bytes
                .checked_sub(self.resident_bytes()?)
                .ok_or(Error::Limit)?;
            self.prepare_records(self.entries.len() + 1, descriptors)?;
            self.disk_bytes = disk;
            let ordinal = self.entries.len();
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .custom_flags(NOFOLLOW | NONBLOCK)
                .open(self.path(ordinal))
                .map_err(|_| Error::Io)?;
            let initial = Identity::of(&file.metadata().map_err(|_| Error::Io)?);
            let mut mode = 0o600;
            let result = (|| {
                self.check_created(ordinal, &file, initial, mode)?;
                let mut out = RecordWriter {
                    file: &mut file,
                    hash: Sha256::new(),
                    written: 0,
                    expected: size,
                };
                writer(&mut out)?;
                if out.written != size {
                    return Err(Error::Format);
                }
                let digest = out.hash.finish();
                file.set_permissions(Permissions::from_mode(0o400))
                    .map_err(|_| Error::Io)?;
                mode = 0o400;
                file.sync_all().map_err(|_| Error::Io)?;
                let identity = self.check_created(ordinal, &file, initial, mode)?;
                if identity.len != size as u64 {
                    return Err(Error::Changed);
                }
                // Verify the actual completed on-disk file with fixed scratch;
                // never duplicate a full RAW/root buffer to seal a record.
                let mut read = self.file(ordinal)?;
                let read_result =
                    crate::integrity::reader(&mut read, size as u64).map_err(|_| Error::Changed);
                let read_held = read
                    .metadata()
                    .map_err(|_| Error::Changed)
                    .map(|m| Identity::of(&m));
                let read_named = self.check_created(ordinal, &file, initial, mode);
                if read_held? != identity || read_named? != identity {
                    return Err(Error::Changed);
                }
                if read_result? != digest {
                    return Err(Error::Changed);
                }
                Ok((digest, identity))
            })();
            // Includes writer/length/fsync/parser failures. Own append/chmod
            // mutations are allowed; pathname/object/mode/owner changes are not.
            let final_guard = self.check_created(ordinal, &file, initial, mode);
            let (digest, identity) = final_guard.and(result)?;
            self.entries.push(Entry {
                identity,
                mount: self.root.leaf().mount,
                digest,
                bytes: size,
            });
            Ok(Record(ordinal))
        })();
        self.finish(result)
    }
    pub(crate) fn with_record<T>(
        &mut self,
        record: Record,
        budget: usize,
        call: impl FnOnce(&[u8]) -> Result<T, Error>,
    ) -> Result<T, Error> {
        self.with_record_alloc(
            record,
            budget,
            |size| {
                let mut bytes = Vec::new();
                bytes
                    .try_reserve_exact(size)
                    .map_err(|_| Error::Allocation)?;
                Ok(bytes)
            },
            call,
        )
    }
    fn with_record_alloc<T>(
        &mut self,
        record: Record,
        budget: usize,
        allocate: impl FnOnce(usize) -> Result<Vec<u8>, Error>,
        call: impl FnOnce(&[u8]) -> Result<T, Error>,
    ) -> Result<T, Error> {
        let result = (|| {
            self.verify_unchanged()?;
            let entry = self.entries.get(record.0).ok_or(Error::State)?;
            let size = entry.bytes;
            let expected = entry.digest;
            if size > budget.min(self.limits.max_active_bytes)
                || self.reloads >= self.limits.max_reloads
            {
                return Err(Error::Limit);
            }
            let total = self.reload_bytes.checked_add(size).ok_or(Error::Limit)?;
            if total > self.limits.max_reload_bytes {
                return Err(Error::Limit);
            }
            self.reloads += 1;
            self.reload_bytes = total;
            let mut file = self.file(record.0)?;
            let result = (|| {
                self.check_entry(record.0, &file)?;
                let mut bytes = allocate(size)?;
                if bytes.capacity() > budget.min(self.limits.max_active_bytes) {
                    return Err(Error::Limit);
                }
                self.peak_reload_bytes = self.peak_reload_bytes.max(bytes.capacity());
                bytes.resize(size, 0);
                file.read_exact(&mut bytes).map_err(|_| Error::Changed)?;
                let mut tail = [0];
                if file.read(&mut tail).map_err(|_| Error::Io)? != 0 {
                    return Err(Error::Changed);
                }
                let mut hash = Sha256::new();
                hash.update(&bytes);
                if hash.finish() != expected {
                    return Err(Error::Changed);
                }
                call(&bytes)
            })();
            // All outcomes after open, including allocation/capacity refusal,
            // retain and recheck the held descriptor and its current name.
            let guard = self.check_entry(record.0, &file);
            guard.and(result)
        })();
        self.finish(result)
    }
}
pub(crate) struct RecordWriter<'a> {
    file: &'a mut File,
    hash: Sha256,
    written: usize,
    expected: usize,
}
impl RecordWriter<'_> {
    pub(crate) fn bytes(&mut self, bytes: &[u8]) -> Result<(), Error> {
        self.written = self.written.checked_add(bytes.len()).ok_or(Error::Limit)?;
        if self.written > self.expected {
            return Err(Error::Format);
        }
        self.file.write_all(bytes).map_err(|_| Error::Io)?;
        self.hash.update(bytes);
        Ok(())
    }
}
#[cfg(test)]
mod tests;
