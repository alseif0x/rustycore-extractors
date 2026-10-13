// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original Windows immutable VMap package reader, retaining
// this repository's Linux request/snapshot/accounting operation contract.
//! Complete reads with full file/directory snapshots and permanent change poison.
//! Each operation opens fresh platform leases; no handles survive an operation.
//! Final root/map directories use FULL identities. Ancestor object-only checks
//! are supplied by forever-platform. This is metadata verification, not an
//! atomic snapshot or authentication. Every successful read hashes all bytes.
//!
//! Exact artifact spelling is checked by bounded directory enumeration. Win32
//! aliases, reserved names, non-normal/relative/UNC/verbatim root paths, reparses
//! and components over 255 UTF16 units reject rather than normalize. Model names
//! outside the platform's stricter path profile reject explicitly.
//!
//! Source metadata Vec/root capacities and growth overlap are charged separately
//! from Store geometry/ReadBuffer. Each operation reserves 256KiB for ONE platform
//! lease, composed paths and bounded std enumeration state, plus an explicitly
//! charged <=2MiB chunk Vec. At most65 handles (lease+one enumerator or check),
//! never simultaneous independent leases. No inventory name Vec is retained.
//! Fixed stack hash state and allocator/OS overhead are excluded; logical policy
//! is not RSS. Enumeration/name acquisition failures consume admitted work.
#![cfg(windows)]
use super::{Artifact, Presence, ReadBuffer, Source, SourceError};
use forever_platform::{
    Error as PlatformError,
    path::{AbsolutePath, Limits as PlatformLimits},
    windows::{Identity, LinkPolicy, ReadDirectory, ReadFile, operator_path},
};
use std::{
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
};
#[path = "fs/hash.rs"]
mod hash;
#[cfg(test)]
mod tests;
const LEASE_BYTES: usize = 256 * 1024;
const CHUNK_BYTES: usize = 2 * 1024 * 1024;
pub const MAX_METADATA_BYTES: usize = 128 * 1024 * 1024;
pub const MAX_ENTRIES: usize = 65_536;
pub const MAX_ROOT_BYTES: usize = 4096;
pub const MAX_VERIFY_STEPS: usize = 262_144;
const PATH_BYTES: usize = 520;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FsLimits {
    pub max_metadata_bytes: usize,
    /// Caps both retained snapshots and entries visited in each directory scan.
    pub max_entries: usize,
    pub max_root_bytes: usize,
    /// Conservative native-check units per operation, admitted before each
    /// platform call. Enumeration entries additionally consume two units.
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
    /// Bytes returned by complete chunk calls, including later failed operations.
    /// A failed platform chunk can read an unreported prefix; this is not a
    /// count of every kernel byte or of EOF probe bytes.
    pub read_bytes: u64,
    pub verify_calls: u64,
    pub verify_steps: u64,
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
struct Request {
    full: Key,
    directory: Option<Key>,
    leaf: Key,
}
fn request(artifact: Artifact<'_>) -> Result<Request, SourceError> {
    let mut leaf = Key::new();
    let directory = match artifact {
        Artifact::Model { name } => {
            if name.is_empty()
                || name.len() > 500
                || !name.is_ascii()
                || name == b"."
                || name == b".."
                || name.iter().any(|&b| matches!(b, 0 | b'/' | b'\\'))
            {
                return Err(SourceError::Io);
            }
            leaf.write_str(std::str::from_utf8(name).map_err(|_| SourceError::Io)?)
                .map_err(|_| SourceError::Limit)?;
            leaf.write_str(".vmo").map_err(|_| SourceError::Limit)?;
            None
        }
        Artifact::Tree { map_id } => {
            write!(&mut leaf, "{map_id:04}.vmtree").map_err(|_| SourceError::Limit)?;
            Some(map_id)
        }
        Artifact::Tile { map_id, x, y } | Artifact::Index { map_id, x, y } => {
            if x > 32767 {
                return Err(SourceError::Io);
            }
            write!(&mut leaf, "{map_id:04}_{x:02}_{y:02}.vmtile")
                .map_err(|_| SourceError::Limit)?;
            if matches!(artifact, Artifact::Index { .. }) {
                leaf.write_str("idx").map_err(|_| SourceError::Limit)?;
            }
            Some(map_id)
        }
    };
    let mut full = Key::new();
    let directory = if let Some(id) = directory {
        let mut name = Key::new();
        write!(&mut name, "{id:04}").map_err(|_| SourceError::Limit)?;
        write!(&mut full, "{}\\{}", name.text(), leaf.text()).map_err(|_| SourceError::Limit)?;
        Some(name)
    } else {
        full = leaf;
        None
    };
    Ok(Request {
        full,
        directory,
        leaf,
    })
}
#[derive(Clone, Copy)]
enum State {
    Missing,
    Directory(Identity),
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
    root_path: PathBuf,
    identity: Identity,
    entries: Vec<Entry>,
    limits: FsLimits,
    stats: FsStats,
    failed: bool,
    transient_bytes: usize,
}
impl RootedSource {
    /// Root must already be a normal absolute drive path. No canonicalization,
    /// prefix removal, cwd interpretation or path spelling normalization occurs.
    pub fn new(root: &Path, limits: FsLimits) -> Result<Self, SourceError> {
        limits.validate()?;
        if root.as_os_str().is_empty() || root.as_os_str().len() > limits.max_root_bytes {
            return Err(SourceError::Limit);
        }
        if root
            .as_os_str()
            .len()
            .checked_add(LEASE_BYTES)
            .ok_or(SourceError::Limit)?
            > limits.max_metadata_bytes
        {
            return Err(SourceError::Limit);
        }
        let root_path = operator_path(root, PlatformLimits::default()).map_err(platform_error)?;
        let resident = root_path.capacity();
        let peak = resident
            .checked_add(LEASE_BYTES)
            .ok_or(SourceError::Limit)?;
        if peak > limits.max_metadata_bytes {
            return Err(SourceError::Limit);
        }
        let mut work = Work {
            used: 0,
            cap: limits.max_verify_steps,
        };
        platform_work(&root_path, &mut work)?;
        let lease =
            ReadDirectory::open(&root_path, PlatformLimits::default()).map_err(platform_error)?;
        let identity = lease.identity();
        platform_work(&root_path, &mut work)?;
        lease.verify_unchanged().map_err(platform_error)?;
        drop(lease);
        Ok(Self {
            root_path,
            identity,
            entries: Vec::new(),
            limits,
            stats: FsStats {
                resident_bytes: resident,
                peak_bytes: peak,
                ..FsStats::default()
            },
            failed: false,
            transient_bytes: 0,
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
            .binary_search_by(|entry| entry.path.bytes().cmp(key.bytes()))
    }
    fn observe_extra(&mut self, extra: usize) -> Result<(), SourceError> {
        let peak = self
            .stats
            .resident_bytes
            .checked_add(self.transient_bytes)
            .and_then(|n| n.checked_add(extra))
            .ok_or(SourceError::Limit)?;
        if peak > self.limits.max_metadata_bytes {
            return Err(SourceError::Limit);
        }
        self.stats.peak_bytes = self.stats.peak_bytes.max(peak);
        Ok(())
    }
    fn remember(&mut self, path: Key, state: State) -> Result<(), SourceError> {
        let at = match self.position(&path) {
            Ok(i) => {
                return if same_state(self.entries[i].state, state) {
                    Ok(())
                } else {
                    Err(SourceError::InputChanged)
                };
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
            self.observe_extra(
                target
                    .checked_mul(size_of::<Entry>())
                    .ok_or(SourceError::Limit)?,
            )?;
            let mut next = Vec::new();
            next.try_reserve_exact(target)
                .map_err(|_| SourceError::Limit)?;
            let added = next
                .capacity()
                .checked_mul(size_of::<Entry>())
                .ok_or(SourceError::Limit)?;
            self.observe_extra(added)?;
            next.extend_from_slice(&self.entries);
            self.entries = next;
            self.stats.resident_bytes = self
                .root_path
                .capacity()
                .checked_add(added)
                .ok_or(SourceError::Limit)?;
        }
        self.entries.insert(at, Entry { path, state });
        self.stats.entries = self.entries.len();
        Ok(())
    }
    fn previous_identity(
        &self,
        key: &Key,
        current: Option<Identity>,
        directory: bool,
    ) -> Result<(), SourceError> {
        if let Ok(i) = self.position(key) {
            let matches = match (self.entries[i].state, current) {
                (State::Missing, None) => true,
                (State::Directory(old), Some(id)) if directory => old == id,
                (State::File(old, _), Some(id)) if !directory => old == id,
                _ => false,
            };
            if !matches {
                return Err(SourceError::InputChanged);
            }
        }
        Ok(())
    }
    fn named_path(&self, key: &Key) -> Result<PathBuf, SourceError> {
        // Request keys contain only the exact admitted ASCII separators/names.
        let len = self
            .root_path
            .as_os_str()
            .len()
            .checked_add(1 + key.len)
            .ok_or(SourceError::Limit)?;
        let mut path = PathBuf::new();
        path.try_reserve_exact(len)
            .map_err(|_| SourceError::Limit)?;
        path.push(&self.root_path);
        path.push(key.text());
        // Validate all Windows component aliases before any filesystem access.
        AbsolutePath::parse(
            path.to_str().ok_or(SourceError::Io)?,
            PlatformLimits::default(),
        )
        .map_err(platform_error)?;
        Ok(path)
    }
    fn directory_identity(
        &self,
        path: &Path,
        expected: Option<State>,
        work: &mut Work,
    ) -> Result<Option<Identity>, SourceError> {
        platform_work(path, work)?;
        let lease = match ReadDirectory::open(path, PlatformLimits::default()) {
            Ok(lease) => lease,
            Err(PlatformError::Missing) => return Ok(None),
            Err(error) => return Err(platform_error(error)),
        };
        let id = lease.identity();
        if let Some(previous) = expected
            && !matches!(previous, State::Directory(old) if old == id)
        {
            return Err(SourceError::InputChanged);
        }
        platform_work(path, work)?;
        lease.verify_unchanged().map_err(platform_error)?;
        Ok(Some(id))
    }
    fn guard_root(&self, work: &mut Work) -> Result<(), SourceError> {
        match self.directory_identity(&self.root_path, Some(State::Directory(self.identity)), work)
        {
            Ok(Some(id)) if id == self.identity => Ok(()),
            Err(SourceError::Limit) => Err(SourceError::Limit),
            _ => Err(SourceError::InputChanged),
        }
    }
    fn guard_after(
        &self,
        directory: Option<(Key, Identity)>,
        work: &mut Work,
    ) -> Result<(), SourceError> {
        if let Some((key, expected)) = directory {
            let path = self.named_path(&key)?;
            match self.directory_identity(&path, Some(State::Directory(expected)), work) {
                Ok(Some(id)) if id == expected => {}
                Err(SourceError::Limit) => return Err(SourceError::Limit),
                _ => return Err(SourceError::InputChanged),
            }
        }
        self.guard_root(work)
    }
    fn exact_name(
        &self,
        parent: &Path,
        expected: Identity,
        name: &Key,
        work: &mut Work,
    ) -> Result<bool, SourceError> {
        platform_work(parent, work)?;
        let lease = ReadDirectory::open(parent, PlatformLimits::default()).map_err(|e| {
            if matches!(e, PlatformError::Limit | PlatformError::Allocation) {
                SourceError::Limit
            } else {
                SourceError::InputChanged
            }
        })?;
        if lease.identity() != expected {
            return Err(SourceError::InputChanged);
        }
        // At most one lease plus the enumeration handle, <=65 handles. A DirEntry
        // has one bounded component name; no vector of names is collected.
        work.step()?;
        let result = (|| {
            let mut entries = fs::read_dir(parent).map_err(|_| SourceError::Io)?;
            let mut found = false;
            let mut seen = 0;
            loop {
                work.step()?;
                let Some(entry) = entries.next() else { break };
                if seen >= self.limits.max_entries {
                    return Err(SourceError::Limit);
                }
                seen += 1;
                work.step()?;
                let entry = entry.map_err(|_| SourceError::Io)?;
                let filename = entry.file_name();
                if filename.as_encoded_bytes() == name.bytes() {
                    if found {
                        return Err(SourceError::Io);
                    }
                    found = true;
                }
            }
            Ok(found)
        })();
        // Iterator and last DirEntry are dropped BEFORE a fresh named verification.
        platform_work(parent, work)?;
        lease.verify_unchanged().map_err(platform_error)?;
        result
    }
    fn read_inner(
        &mut self,
        artifact: Artifact<'_>,
        out: &mut ReadBuffer<'_>,
        work: &mut Work,
    ) -> Result<Presence, SourceError> {
        if !out.is_empty() {
            return Err(SourceError::Io);
        }
        let request = request(artifact)?;
        let path = self.named_path(&request.full)?;
        self.guard_root(work)?;
        let mut directory = None;
        let exact = if let Some(key) = request.directory {
            let spelling = self.exact_name(&self.root_path, self.identity, &key, work)?;
            let parent = self.named_path(&key)?;
            let previous = self.position(&key).ok().map(|i| self.entries[i].state);
            let id = self.directory_identity(&parent, previous, work)?;
            self.previous_identity(&key, id, true)?;
            let Some(id) = id else {
                if spelling {
                    return Err(SourceError::InputChanged);
                }
                self.guard_root(work)?;
                self.remember(key, State::Missing)?;
                self.remember(request.full, State::Missing)?;
                return Ok(Presence::Missing);
            };
            if !spelling {
                return Err(SourceError::Io);
            }
            self.remember(key, State::Directory(id))?;
            directory = Some((key, id));
            self.exact_name(&parent, id, &request.leaf, work)?
        } else {
            self.exact_name(&self.root_path, self.identity, &request.leaf, work)?
        };
        platform_work(&path, work)?;
        let opened = ReadFile::open(&path, PlatformLimits::default(), LinkPolicy::Allow);
        let file = match opened {
            Ok(file) => file,
            Err(PlatformError::Missing) => {
                if exact {
                    return Err(SourceError::InputChanged);
                }
                self.previous_identity(&request.full, None, false)?;
                self.guard_after(directory, work)?;
                self.remember(request.full, State::Missing)?;
                return Ok(Presence::Missing);
            }
            Err(error) => return self.failed_open(&request.full, error, directory, work),
        };
        let id = file.identity();
        self.previous_identity(&request.full, Some(id), false)?;
        if !exact {
            drop(file);
            self.guard_after(directory, work)?;
            return Err(SourceError::Io);
        }
        let count = usize::try_from(id.size).map_err(|_| SourceError::Limit)?;
        // Preserve the Store's128MiB ceiling, independent of64MiB primitive calls.
        let result = (|| {
            if count > out.limit() || count > super::MAX_FILE_BYTES {
                return Err(SourceError::Limit);
            }
            let size = count.min(CHUNK_BYTES);
            self.observe_extra(size)?;
            let mut buffer = Vec::new();
            buffer
                .try_reserve_exact(size)
                .map_err(|_| SourceError::Limit)?;
            self.observe_extra(buffer.capacity())?;
            self.transient_bytes = self
                .transient_bytes
                .checked_add(buffer.capacity())
                .ok_or(SourceError::Limit)?;
            buffer.resize(size, 0);
            let mut hash = hash::Sha256::new();
            let mut offset = 0;
            while offset < count {
                let wanted = (count - offset).min(buffer.len());
                platform_work(&path, work)?;
                file.read_exact_at(offset as u64, &mut buffer[..wanted])
                    .map_err(platform_error)?;
                self.stats.read_bytes = self
                    .stats
                    .read_bytes
                    .checked_add(wanted as u64)
                    .ok_or(SourceError::Limit)?;
                out.append(&buffer[..wanted])?;
                hash.update(&buffer[..wanted]);
                offset += wanted;
            }
            platform_work(&path, work)?;
            file.verify_eof().map_err(platform_error)?;
            Ok(hash.finish())
        })();
        // Source changes already observed take priority over subsequent budgets.
        if matches!(result, Err(SourceError::InputChanged)) {
            return Err(SourceError::InputChanged);
        }
        platform_work(&path, work)?;
        file.verify_unchanged().map_err(platform_error)?;
        drop(file);
        self.guard_after(directory, work)?;
        self.remember(request.full, State::File(id, result?))?;
        Ok(Presence::Present)
    }
    fn failed_open(
        &self,
        key: &Key,
        error: PlatformError,
        directory: Option<(Key, Identity)>,
        work: &mut Work,
    ) -> Result<Presence, SourceError> {
        let reason = platform_error(error);
        // A primitive already observed a change/read failure, or a previously
        // captured artifact can no longer be verified. Preserve that terminal
        // evidence BEFORE admitting any later guard work. Budget/allocation
        // refusals alone are not change evidence and must not poison the source.
        if reason == SourceError::InputChanged
            || (self.position(key).is_ok() && reason != SourceError::Limit)
        {
            return Err(SourceError::InputChanged);
        }
        self.guard_after(directory, work)?;
        Err(reason)
    }
    fn verify_inner(&self, work: &mut Work) -> Result<(), SourceError> {
        self.guard_root(work)?;
        for entry in &self.entries {
            let path = self.named_path(&entry.path)?;
            platform_work(&path, work)?;
            match entry.state {
                State::Directory(expected) => {
                    let lease = ReadDirectory::open(&path, PlatformLimits::default())
                        .map_err(verify_error)?;
                    if lease.identity() != expected {
                        return Err(SourceError::InputChanged);
                    }
                    platform_work(&path, work)?;
                    lease.verify_unchanged().map_err(verify_error)?;
                }
                State::File(expected, _) => {
                    let lease = ReadFile::open(&path, PlatformLimits::default(), LinkPolicy::Allow)
                        .map_err(verify_error)?;
                    if lease.identity() != expected {
                        return Err(SourceError::InputChanged);
                    }
                    platform_work(&path, work)?;
                    lease.verify_unchanged().map_err(verify_error)?;
                }
                State::Missing => {
                    // A missing map directory and a missing file are both genuine
                    // NotFound only; alternate types/aliases/errors never Missing.
                    match ReadFile::open(&path, PlatformLimits::default(), LinkPolicy::Allow) {
                        Err(PlatformError::Missing) => {}
                        Err(PlatformError::Limit | PlatformError::Allocation) => {
                            return Err(SourceError::Limit);
                        }
                        _ => return Err(SourceError::InputChanged),
                    }
                }
            }
        }
        // Recheck every final map-directory snapshot after descendant checks.
        for entry in &self.entries {
            if let State::Directory(expected) = entry.state {
                match self.directory_identity(
                    &self.named_path(&entry.path)?,
                    Some(State::Directory(expected)),
                    work,
                ) {
                    Ok(Some(id)) if id == expected => {}
                    Err(SourceError::Limit) => return Err(SourceError::Limit),
                    _ => return Err(SourceError::InputChanged),
                }
            }
        }
        self.guard_root(work)
    }
    fn finish_operation<T>(
        &mut self,
        result: Result<T, SourceError>,
        work: Work,
    ) -> Result<T, SourceError> {
        self.transient_bytes = 0;
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
        artifact: Artifact<'_>,
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
        self.transient_bytes = LEASE_BYTES;
        let mut work = Work {
            used: 0,
            cap: self.limits.max_verify_steps,
        };
        let result = self
            .observe_extra(0)
            .and_then(|()| self.read_inner(artifact, out, &mut work));
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
        self.transient_bytes = LEASE_BYTES;
        let mut work = Work {
            used: 0,
            cap: self.limits.max_verify_steps,
        };
        let result = self
            .observe_extra(0)
            .and_then(|()| self.verify_inner(&mut work));
        self.finish_operation(result, work)
    }
}
fn platform_error(error: PlatformError) -> SourceError {
    match error {
        PlatformError::Limit | PlatformError::Allocation => SourceError::Limit,
        PlatformError::Changed | PlatformError::Read => SourceError::InputChanged,
        _ => SourceError::Io,
    }
}
fn verify_error(error: PlatformError) -> SourceError {
    match error {
        PlatformError::Limit | PlatformError::Allocation => SourceError::Limit,
        _ => SourceError::InputChanged,
    }
}
fn platform_work(path: &Path, work: &mut Work) -> Result<(), SourceError> {
    // A primitive can acquire/validate each ancestor and final handle several
    // times.32units*(components+2) conservatively covers opens, file-type and
    // identity/volume calls, including both guards inside a read/EOF operation.
    let components = path
        .as_os_str()
        .as_encoded_bytes()
        .iter()
        .filter(|&&b| b == b'\\')
        .count();
    let units = components
        .checked_add(2)
        .and_then(|n| n.checked_mul(32))
        .ok_or(SourceError::Limit)?;
    if units > work.cap.saturating_sub(work.used) {
        return Err(SourceError::Limit);
    }
    work.used += units;
    Ok(())
}
fn same_state(a: State, b: State) -> bool {
    match (a, b) {
        (State::Missing, State::Missing) => true,
        (State::Directory(a), State::Directory(b)) => a == b,
        (State::File(a, ah), State::File(b, bh)) => a == b && ah == bh,
        _ => false,
    }
}
