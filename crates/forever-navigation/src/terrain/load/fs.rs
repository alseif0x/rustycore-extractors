// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: Linux anchored immutable-package file source.
// Adapted 2026-10-11 from this repository's store/fs.rs for flat MAPS files.
//! Reads complete files through a retained root directory descriptor and an
//! explicit filename. Names are preserved exactly; no CASC,
//! network, fallback, directory enumeration or per-query I/O is performed.
//!
//! Each successful read hashes all bytes and compares any previous digest.
//! `verify_unchanged` checks recorded identities/absence and directory metadata
//! ONLY; it does not reread all file contents or provide an atomic filesystem
//! snapshot. Callers requiring full post-run hash evidence must collect it
//! separately. Observed changes poison this source permanently.
//!
//! Metadata capacities are charged separately from terrain geometry/ReadBuffer:
//! root-path bytes and actual snapshot Vec capacity, including old/new growth
//! overlap. Fixed stack paths/hash/chunk buffers and allocator/OS metadata are
//! excluded. This is a logical capacity bound, not RSS. The intended package is
//! immutable; even unrelated directory-entry changes invalidate its identity.
#![cfg(target_os = "linux")]
use super::{Artifact, Presence, ReadBuffer, Source, SourceError};
use std::fmt::Write as _;
use std::fs::{self, File, Metadata, OpenOptions};
use std::io::{self, Read};
use std::os::fd::{AsRawFd, RawFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::Path;

pub(crate) mod hash;
#[cfg(test)]
mod tests;

pub const MAX_METADATA_BYTES: usize = 128 * 1024 * 1024;
pub const MAX_ENTRIES: usize = 65_536;
pub const MAX_ROOT_BYTES: usize = 4096;
pub const MAX_VERIFY_STEPS: usize = 262_144;
const O_NONBLOCK: i32 = 0x800;
const O_DIRECTORY: i32 = 0x10000;
const O_NOFOLLOW: i32 = 0x20000;
const PATH_BYTES: usize = 520;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FsLimits {
    pub max_metadata_bytes: usize,
    pub max_entries: usize,
    pub max_root_bytes: usize,
    /// Metadata calls per read/verification operation; checked before calls.
    pub max_verify_steps: usize,
}
impl Default for FsLimits {
    fn default() -> Self {
        Self {
            max_metadata_bytes: MAX_METADATA_BYTES,
            max_entries: MAX_ENTRIES,
            max_root_bytes: MAX_ROOT_BYTES,
            max_verify_steps: MAX_VERIFY_STEPS,
        }
    }
}
impl FsLimits {
    fn validate(self) -> Result<(), SourceError> {
        if self.max_metadata_bytes > MAX_METADATA_BYTES
            || self.max_entries > MAX_ENTRIES
            || self.max_root_bytes > MAX_ROOT_BYTES
            || self.max_verify_steps > MAX_VERIFY_STEPS
        {
            return Err(SourceError::Limit);
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FsStats {
    pub entries: usize,
    pub resident_bytes: usize,
    pub peak_bytes: usize,
    pub read_calls: u64,
    /// Bytes actually read, including a read that subsequently fails validation.
    pub read_bytes: u64,
    pub verify_calls: u64,
    pub verify_steps: u64,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct Identity {
    dev: u64,
    ino: u64,
    mode: u32,
    uid: u32,
    gid: u32,
    nlink: u64,
    size: u64,
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
            uid: m.uid(),
            gid: m.gid(),
            nlink: m.nlink(),
            size: m.len(),
            mtime: m.mtime(),
            mtime_ns: m.mtime_nsec(),
            ctime: m.ctime(),
            ctime_ns: m.ctime_nsec(),
        }
    }
}
#[derive(Clone, Copy)]
struct Text<const N: usize> {
    bytes: [u8; N],
    len: usize,
}
impl<const N: usize> Text<N> {
    fn new() -> Self {
        Self {
            bytes: [0; N],
            len: 0,
        }
    }
    fn bytes(&self) -> &[u8] {
        &self.bytes[..self.len]
    }
    fn text(&self) -> &str {
        std::str::from_utf8(self.bytes()).expect("ASCII artifact path")
    }
    fn path(&self) -> &Path {
        Path::new(self.text())
    }
}
impl<const N: usize> std::fmt::Write for Text<N> {
    fn write_str(&mut self, text: &str) -> std::fmt::Result {
        let end = self.len.checked_add(text.len()).ok_or(std::fmt::Error)?;
        let to = self.bytes.get_mut(self.len..end).ok_or(std::fmt::Error)?;
        to.copy_from_slice(text.as_bytes());
        self.len = end;
        Ok(())
    }
}
type Key = Text<PATH_BYTES>;
fn anchor(fd: RawFd, name: &str) -> Result<Text<560>, SourceError> {
    let mut out = Text::new();
    write!(&mut out, "/proc/self/fd/{fd}/{name}").map_err(|_| SourceError::Limit)?;
    Ok(out)
}
fn request(artifact: Artifact) -> Result<Key, SourceError> {
    if artifact.tile.row >= 64 || artifact.tile.column >= 64 {
        return Err(SourceError::Io);
    }
    let mut out = Key::new();
    write!(
        &mut out,
        "{:04}_{:02}_{:02}.map",
        artifact.map_id, artifact.tile.row, artifact.tile.column
    )
    .map_err(|_| SourceError::Limit)?;
    Ok(out)
}
#[derive(Clone, Copy)]
enum State {
    Missing,
    File(Identity, [u8; 32]),
}
#[derive(Clone, Copy)]
struct Entry {
    path: Key,
    state: State,
}
struct Work {
    used: usize,
    cap: usize,
}
impl Work {
    fn step(&mut self) -> Result<(), SourceError> {
        if self.used >= self.cap {
            return Err(SourceError::Limit);
        }
        self.used += 1;
        Ok(())
    }
}

pub struct RootedSource {
    root: File,
    root_path: Vec<u8>,
    identity: Identity,
    entries: Vec<Entry>,
    limits: FsLimits,
    stats: FsStats,
    failed: bool,
}
impl RootedSource {
    /// Root components must not contain `.`/`..` or symlink aliases. Ordinary
    /// relative paths are retained relative to the unchanged process cwd;
    /// changing cwd so that the path no longer identifies the root is a change.
    pub fn new(root: &Path, limits: FsLimits) -> Result<Self, SourceError> {
        limits.validate()?;
        let raw_path = root.as_os_str().as_bytes();
        if raw_path.is_empty() || raw_path.len() > limits.max_root_bytes {
            return Err(SourceError::Limit);
        }
        if raw_path
            .split(|b| *b == b'/')
            .any(|part| part == b"." || part == b"..")
        {
            return Err(SourceError::Io);
        }
        // Trailing slashes must not make lstat/O_NOFOLLOW follow a final link.
        let mut length = raw_path.len();
        while length > 1 && raw_path[length - 1] == b'/' {
            length -= 1;
        }
        let path = &raw_path[..length];
        if path.is_empty()
            || path.len() > limits.max_root_bytes
            || path.len() > limits.max_metadata_bytes
        {
            return Err(SourceError::Limit);
        }
        let mut root_path = Vec::new();
        root_path
            .try_reserve_exact(path.len())
            .map_err(|_| SourceError::Limit)?;
        if root_path.capacity() > limits.max_metadata_bytes {
            return Err(SourceError::Limit);
        }
        root_path.extend_from_slice(path);
        let root = Path::new(std::ffi::OsStr::from_bytes(&root_path));
        // Reject aliases through symlink ancestors as well as the final root.
        // This constructor walk is bounded by max_root_bytes, uses borrowed
        // paths only, and makes no directory inventory allocations.
        for ancestor in root.ancestors().filter(|p| !p.as_os_str().is_empty()) {
            let meta = fs::symlink_metadata(ancestor).map_err(|_| SourceError::Io)?;
            if !meta.is_dir() || meta.file_type().is_symlink() {
                return Err(SourceError::Io);
            }
        }
        let named = fs::symlink_metadata(root).map_err(|_| SourceError::Io)?;
        if !named.is_dir() || named.file_type().is_symlink() {
            return Err(SourceError::Io);
        }
        let file = open_dir(root)?;
        let held = file.metadata().map_err(|_| SourceError::Io)?;
        let after = fs::symlink_metadata(root).map_err(|_| SourceError::InputChanged)?;
        let identity = Identity::of(&named);
        if identity != Identity::of(&held) || identity != Identity::of(&after) {
            return Err(SourceError::InputChanged);
        }
        let resident = root_path.capacity();
        Ok(Self {
            root: file,
            root_path,
            identity,
            entries: Vec::new(),
            limits,
            stats: FsStats {
                resident_bytes: resident,
                peak_bytes: resident,
                ..FsStats::default()
            },
            failed: false,
        })
    }
    pub const fn stats(&self) -> FsStats {
        self.stats
    }
    pub const fn failed(&self) -> bool {
        self.failed
    }
    fn position(&self, key: &Key) -> Result<usize, usize> {
        self.entries
            .binary_search_by(|e| e.path.bytes().cmp(key.bytes()))
    }
    fn remember(&mut self, path: Key, state: State) -> Result<(), SourceError> {
        let at = match self.position(&path) {
            Ok(i) => {
                if !same_state(self.entries[i].state, state) {
                    return Err(SourceError::InputChanged);
                }
                return Ok(());
            }
            Err(i) => i,
        };
        if self.entries.len() >= self.limits.max_entries {
            return Err(SourceError::Limit);
        }
        if self.entries.len() == self.entries.capacity() {
            let target = self
                .entries
                .capacity()
                .checked_mul(2)
                .ok_or(SourceError::Limit)?
                .max(8)
                .min(self.limits.max_entries);
            let old = self.stats.resident_bytes;
            let requested = target
                .checked_mul(size_of::<Entry>())
                .and_then(|n| n.checked_add(old))
                .ok_or(SourceError::Limit)?;
            if requested > self.limits.max_metadata_bytes {
                return Err(SourceError::Limit);
            }
            let mut next = Vec::new();
            next.try_reserve_exact(target)
                .map_err(|_| SourceError::Limit)?;
            let added = next
                .capacity()
                .checked_mul(size_of::<Entry>())
                .ok_or(SourceError::Limit)?;
            let peak = old.checked_add(added).ok_or(SourceError::Limit)?;
            if peak > self.limits.max_metadata_bytes {
                return Err(SourceError::Limit);
            }
            next.extend_from_slice(&self.entries);
            self.entries = next;
            self.stats.resident_bytes = self
                .root_path
                .capacity()
                .checked_add(added)
                .ok_or(SourceError::Limit)?;
            self.stats.peak_bytes = self.stats.peak_bytes.max(peak);
        }
        self.entries.insert(at, Entry { path, state });
        self.stats.entries = self.entries.len();
        Ok(())
    }
    fn guard_root(&self, work: &mut Work) -> Result<(), SourceError> {
        work.step()?;
        let held = self
            .root
            .metadata()
            .map_err(|_| SourceError::InputChanged)?;
        if self.identity != Identity::of(&held) {
            return Err(SourceError::InputChanged);
        }
        work.step()?;
        let path = Path::new(std::ffi::OsStr::from_bytes(&self.root_path));
        let named = fs::symlink_metadata(path).map_err(|_| SourceError::InputChanged)?;
        if self.identity != Identity::of(&named) {
            return Err(SourceError::InputChanged);
        }
        Ok(())
    }
    fn previous_metadata(&self, key: &Key, metadata: Option<&Metadata>) -> Result<(), SourceError> {
        if let Ok(i) = self.position(key)
            && !matches_metadata(self.entries[i].state, metadata)
        {
            return Err(SourceError::InputChanged);
        }
        Ok(())
    }
    fn read_inner(
        &mut self,
        artifact: Artifact,
        out: &mut ReadBuffer<'_>,
        work: &mut Work,
    ) -> Result<Presence, SourceError> {
        if !out.is_empty() {
            return Err(SourceError::Io);
        }
        let request = request(artifact)?;
        self.guard_root(work)?;
        let path = anchor(self.root.as_raw_fd(), request.text())?;
        work.step()?;
        let named = metadata(path.path())?;
        self.previous_metadata(&request, named.as_ref())?;
        let Some(named) = named else {
            self.guard_root(work)?;
            self.remember(request, State::Missing)?;
            return Ok(Presence::Missing);
        };
        if !named.is_file() || named.file_type().is_symlink() {
            return Err(SourceError::Io);
        }
        let opened = OpenOptions::new()
            .read(true)
            .custom_flags(O_NOFOLLOW | O_NONBLOCK)
            .open(path.path());
        let mut file = match opened {
            Ok(file) => file,
            Err(_) => {
                work.step()?;
                let after =
                    fs::symlink_metadata(path.path()).map_err(|_| SourceError::InputChanged)?;
                if Identity::of(&after) != Identity::of(&named) {
                    return Err(SourceError::InputChanged);
                }
                self.guard_root(work)?;
                return Err(SourceError::Io);
            }
        };
        work.step()?;
        let held = file.metadata().map_err(|_| SourceError::InputChanged)?;
        let id = Identity::of(&named);
        if id != Identity::of(&held) {
            return Err(SourceError::InputChanged);
        }
        let count = usize::try_from(held.len()).map_err(|_| SourceError::Limit)?;
        if count > out.limit() {
            return Err(SourceError::Limit);
        }
        let mut remaining = count;
        let mut buffer = [0u8; 16 * 1024];
        let mut hash = hash::Sha256::new();
        let result = (|| {
            while remaining != 0 {
                let wanted = remaining.min(buffer.len());
                let n = file
                    .read(&mut buffer[..wanted])
                    .map_err(|_| SourceError::Io)?;
                if n == 0 {
                    return Err(SourceError::Io);
                }
                self.stats.read_bytes = self
                    .stats
                    .read_bytes
                    .checked_add(n as u64)
                    .ok_or(SourceError::Limit)?;
                out.append(&buffer[..n])?;
                hash.update(&buffer[..n]);
                remaining -= n;
            }
            let tail = file.read(&mut buffer[..1]).map_err(|_| SourceError::Io)?;
            self.observe_tail(tail)
        })();
        // A change already observed by the read wins over later guard limits.
        if matches!(result, Err(SourceError::InputChanged)) {
            return result.map(|()| Presence::Present);
        }
        // Even EOF/I/O/budget errors still require the remaining identity guards.
        work.step()?;
        let after = file.metadata().map_err(|_| SourceError::InputChanged)?;
        if id != Identity::of(&after) {
            return Err(SourceError::InputChanged);
        }
        work.step()?;
        let named_after =
            fs::symlink_metadata(path.path()).map_err(|_| SourceError::InputChanged)?;
        if id != Identity::of(&named_after) {
            return Err(SourceError::InputChanged);
        }
        self.guard_root(work)?;
        result?;
        self.remember(request, State::File(id, hash.finish()))?;
        Ok(Presence::Present)
    }
    fn observe_tail(&mut self, count: usize) -> Result<(), SourceError> {
        if count == 0 {
            return Ok(());
        }
        // The byte proving an extent change is an actual read, too. Preserve
        // that evidence even if a counter or a later metadata budget is full.
        self.failed = true;
        if let Some(total) = self.stats.read_bytes.checked_add(count as u64) {
            self.stats.read_bytes = total;
        }
        Err(SourceError::InputChanged)
    }
    fn verify_inner(&self, work: &mut Work) -> Result<(), SourceError> {
        self.guard_root(work)?;
        for entry in &self.entries {
            work.step()?;
            let path = anchor(self.root.as_raw_fd(), entry.path.text())?;
            let named = metadata(path.path()).map_err(|_| SourceError::InputChanged)?;
            if !matches_metadata(entry.state, named.as_ref()) {
                return Err(SourceError::InputChanged);
            }
        }
        self.guard_root(work)
    }
    fn finish_operation<T>(
        &mut self,
        result: Result<T, SourceError>,
        work: Work,
    ) -> Result<T, SourceError> {
        if matches!(result, Err(SourceError::InputChanged)) {
            self.failed = true;
        }
        self.stats.verify_steps = self
            .stats
            .verify_steps
            .checked_add(work.used as u64)
            .ok_or(if self.failed {
                SourceError::InputChanged
            } else {
                SourceError::Limit
            })?;
        if self.failed {
            Err(SourceError::InputChanged)
        } else {
            result
        }
    }
}
impl Source for RootedSource {
    fn read(
        &mut self,
        artifact: Artifact,
        out: &mut ReadBuffer<'_>,
    ) -> Result<Presence, SourceError> {
        if self.failed {
            return Err(SourceError::InputChanged);
        }
        self.stats.read_calls = self
            .stats
            .read_calls
            .checked_add(1)
            .ok_or(SourceError::Limit)?;
        let mut work = Work {
            used: 0,
            cap: self.limits.max_verify_steps,
        };
        let result = self.read_inner(artifact, out, &mut work);
        self.finish_operation(result, work)
    }
    fn verify_unchanged(&mut self) -> Result<(), SourceError> {
        if self.failed {
            return Err(SourceError::InputChanged);
        }
        self.stats.verify_calls = self
            .stats
            .verify_calls
            .checked_add(1)
            .ok_or(SourceError::Limit)?;
        let mut work = Work {
            used: 0,
            cap: self.limits.max_verify_steps,
        };
        let result = self.verify_inner(&mut work);
        self.finish_operation(result, work)
    }
}
fn open_dir(path: &Path) -> Result<File, SourceError> {
    OpenOptions::new()
        .read(true)
        .custom_flags(O_DIRECTORY | O_NOFOLLOW | O_NONBLOCK)
        .open(path)
        .map_err(|_| SourceError::Io)
}
fn metadata(path: &Path) -> Result<Option<Metadata>, SourceError> {
    match fs::symlink_metadata(path) {
        Ok(value) => Ok(Some(value)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(_) => Err(SourceError::Io),
    }
}
fn matches_metadata(state: State, meta: Option<&Metadata>) -> bool {
    match (state, meta) {
        (State::Missing, None) => true,
        (State::File(id, _), Some(meta)) => id == Identity::of(meta),
        _ => false,
    }
}
fn same_state(a: State, b: State) -> bool {
    match (a, b) {
        (State::Missing, State::Missing) => true,
        (State::File(a, ah), State::File(b, bh)) => a == b && ah == bh,
        _ => false,
    }
}
