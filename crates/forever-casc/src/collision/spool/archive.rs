// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-12: original explicit private lease-backed append archive.
use super::{Error, Identity, NOFOLLOW, NONBLOCK, RecordWriter, anchor, mount_id};
use crate::sha256::Sha256;
use forever_platform::linux::lease;
use std::{
    fs::{self, File, OpenOptions},
    io::{Seek, SeekFrom, Write},
    os::unix::fs::{FileExt, OpenOptionsExt},
    path::PathBuf,
};
const O_PATH: i32 = 0x200000;
const NAME: &str = "records.archive";
pub(super) const CONTAINER_BYTES: usize = 32;
pub(super) const RECORD_BYTES: usize = 64;
fn container() -> [u8; CONTAINER_BYTES] {
    let mut h = [0; CONTAINER_BYTES];
    h[..8].copy_from_slice(b"RCEARC1\0");
    h[8..12].copy_from_slice(&1u32.to_le_bytes());
    h[12..16].copy_from_slice(&(CONTAINER_BYTES as u32).to_le_bytes());
    h
}
fn header(ordinal: usize, bytes: usize, digest: [u8; 32]) -> [u8; RECORD_BYTES] {
    let mut h = [0; RECORD_BYTES];
    h[..8].copy_from_slice(b"RCERCD1\0");
    h[8..16].copy_from_slice(&(ordinal as u64).to_le_bytes());
    h[16..24].copy_from_slice(&(bytes as u64).to_le_bytes());
    h[24..56].copy_from_slice(&digest);
    h
}
fn mapped(e: lease::Error) -> Error {
    match e {
        lease::Error::Allocation => Error::Allocation,
        lease::Error::Changed => Error::Changed,
        lease::Error::Unsupported | lease::Error::Unavailable => Error::BackendUnavailable,
    }
}
struct Entry {
    start: u64,
    bytes: usize,
    digest: [u8; 32],
}
pub(super) struct Archive {
    file: File,
    identity: Identity,
    mount: u64,
    entries: Vec<Entry>,
    in_progress: bool,
    #[cfg(test)]
    checks: std::cell::Cell<usize>,
}
impl Archive {
    fn path(root: &super::Chain) -> PathBuf {
        anchor(&root.leaf().held).join(NAME)
    }
    pub(super) fn create(root: &super::Chain) -> Result<Self, Error> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .mode(0o600)
            .custom_flags(NOFOLLOW | NONBLOCK)
            .open(Self::path(root))
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::AlreadyExists {
                    Error::Exists
                } else {
                    Error::Io
                }
            })?;
        let meta = file.metadata().map_err(|_| Error::Io)?;
        let initial = Identity::of(&meta);
        let mount = mount_id(&file)?;
        if !meta.is_file()
            || initial.len != 0
            || initial.links != 1
            || initial.mode & 0o7777 != 0o600
            || initial.uid != root.leaf().identity.uid
            || mount != root.leaf().mount
        {
            return Err(Error::Changed);
        }
        // This fresh description is never cloned, exposed or reopened for I/O.
        lease::establish(&file).map_err(mapped)?;
        let mut out = Self {
            file,
            identity: initial,
            mount,
            entries: Vec::new(),
            in_progress: true,
            #[cfg(test)]
            checks: std::cell::Cell::new(0),
        };
        let result = (|| {
            out.file.write_all(&container()).map_err(|_| Error::Io)?;
            out.file.sync_all().map_err(|_| Error::Io)
        })();
        let guard = out.settle(root, 0, CONTAINER_BYTES as u64);
        guard.and(result)?;
        out.check(root)?;
        Ok(out)
    }
    pub(super) fn len(&self) -> usize {
        self.entries.len()
    }
    pub(super) fn capacity(&self) -> usize {
        self.entries.capacity()
    }
    pub(super) fn capacity_bytes(&self) -> Result<usize, Error> {
        self.entries
            .capacity()
            .checked_mul(size_of::<Entry>())
            .ok_or(Error::Limit)
    }
    pub(super) fn projected_capacity_bytes(count: usize) -> Result<usize, Error> {
        count.checked_mul(size_of::<Entry>()).ok_or(Error::Limit)
    }
    pub(super) fn resident_bytes(&self) -> Result<usize, Error> {
        // Deliberately retain conservative admission for the helper after its
        // transient stack is gone. This is not an allocator/RSS measurement.
        self.capacity_bytes()?
            .checked_add(lease::HELPER_RESERVATION_BYTES)
            .ok_or(Error::Limit)
    }
    pub(super) fn reserve(&mut self, count: usize) -> Result<(), Error> {
        if count > self.entries.capacity() {
            self.entries
                .try_reserve_exact(count - self.entries.len())
                .map_err(|_| Error::Allocation)?;
        }
        Ok(())
    }
    fn named(&self, root: &super::Chain) -> Result<(File, Identity), Error> {
        // O_PATH obtains current name/mount identity without breaking our own
        // lease. NOFOLLOW deliberately returns a symlink handle for rejection.
        let named = OpenOptions::new()
            .read(true)
            .custom_flags(O_PATH | NOFOLLOW)
            .open(Self::path(root))
            .map_err(|_| Error::Changed)?;
        let meta = named.metadata().map_err(|_| Error::Changed)?;
        if !meta.is_file() || mount_id(&named).map_err(|_| Error::Changed)? != self.mount {
            return Err(Error::Changed);
        }
        let identity = Identity::of(&meta);
        let path_meta = fs::symlink_metadata(Self::path(root)).map_err(|_| Error::Changed)?;
        if !path_meta.is_file()
            || path_meta.file_type().is_symlink()
            || Identity::of(&path_meta) != identity
        {
            return Err(Error::Changed);
        }
        Ok((named, identity))
    }
    fn current(&self, root: &super::Chain) -> Result<Identity, Error> {
        lease::verify(&self.file).map_err(|_| Error::Changed)?;
        let (_, named) = self.named(root)?;
        let meta = self.file.metadata().map_err(|_| Error::Changed)?;
        let held = Identity::of(&meta);
        if !meta.is_file()
            || held != named
            || mount_id(&self.file).map_err(|_| Error::Changed)? != self.mount
        {
            return Err(Error::Changed);
        }
        lease::verify(&self.file).map_err(|_| Error::Changed)?;
        Ok(held)
    }
    pub(super) fn check(&self, root: &super::Chain) -> Result<(), Error> {
        #[cfg(test)]
        self.checks
            .set(self.checks.get().checked_add(1).ok_or(Error::Limit)?);
        let current = self.current(root)?;
        if self.in_progress || current != self.identity || current.links != 1 {
            return Err(Error::Changed);
        }
        let mut h = [0; CONTAINER_BYTES];
        self.file
            .read_exact_at(&mut h, 0)
            .map_err(|_| Error::Changed)?;
        if h != container() {
            return Err(Error::Changed);
        }
        lease::verify(&self.file).map_err(|_| Error::Changed)
    }
    fn settle(&mut self, root: &super::Chain, start: u64, end: u64) -> Result<(), Error> {
        let current = self.current(root)?;
        let old = self.identity;
        if !old.same_object(current)
            || old.mode != current.mode
            || old.uid != current.uid
            || old.gid != current.gid
            || current.links != 1
            || current.len < start
            || current.len > end
        {
            return Err(Error::Changed);
        }
        // Only our private description can write during an intact write lease.
        // Recheck the lease before capturing its own append timestamps/length.
        lease::verify(&self.file).map_err(|_| Error::Changed)?;
        self.identity = current;
        self.in_progress = false;
        Ok(())
    }
    fn hash_body(&self, offset: u64, size: usize) -> Result<[u8; 32], Error> {
        let mut scratch = [0u8; 8192];
        let mut hash = Sha256::new();
        let mut consumed = 0usize;
        while consumed < size {
            let n = (size - consumed).min(scratch.len());
            let at = offset.checked_add(consumed as u64).ok_or(Error::Limit)?;
            self.file
                .read_exact_at(&mut scratch[..n], at)
                .map_err(|_| Error::Changed)?;
            hash.update(&scratch[..n]);
            consumed += n;
        }
        Ok(hash.finish())
    }
    pub(super) fn append(
        &mut self,
        root: &super::Chain,
        size: usize,
        writer: impl FnOnce(&mut RecordWriter<'_>) -> Result<(), Error>,
    ) -> Result<usize, Error> {
        self.check(root)?;
        if self.entries.len() >= self.entries.capacity() {
            return Err(Error::Limit);
        }
        let ordinal = self.entries.len();
        let start = self.identity.len;
        let body = start.checked_add(RECORD_BYTES as u64).ok_or(Error::Limit)?;
        let end = body.checked_add(size as u64).ok_or(Error::Limit)?;
        self.in_progress = true;
        let result = (|| {
            self.file
                .seek(SeekFrom::Start(start))
                .map_err(|_| Error::Io)?;
            self.file
                .write_all(&header(ordinal, size, [0; 32]))
                .map_err(|_| Error::Io)?;
            let mut output = RecordWriter {
                file: &mut self.file,
                hash: Sha256::new(),
                written: 0,
                expected: size,
            };
            writer(&mut output)?;
            if output.written != size {
                return Err(Error::Format);
            }
            let digest = output.hash.finish();
            // Seal only the just-appended fixed header, never a prior record.
            self.file
                .write_all_at(&header(ordinal, size, digest), start)
                .map_err(|_| Error::Io)?;
            self.file.sync_all().map_err(|_| Error::Io)?;
            if self.file.metadata().map_err(|_| Error::Changed)?.len() != end
                || self.hash_body(body, size)? != digest
            {
                return Err(Error::Changed);
            }
            let mut stored = [0; RECORD_BYTES];
            self.file
                .read_exact_at(&mut stored, start)
                .map_err(|_| Error::Changed)?;
            if stored != header(ordinal, size, digest) {
                return Err(Error::Changed);
            }
            Ok(digest)
        })();
        // Returned failures retain their typed cause and partial bytes. Panic
        // leaves in_progress set, so catching unwind cannot reuse this owner.
        let guard = self.settle(root, start, end);
        let digest = guard.and(result)?;
        if self.identity.len != end {
            return Err(Error::Changed);
        }
        self.entries.push(Entry {
            start,
            bytes: size,
            digest,
        });
        Ok(ordinal)
    }
    pub(super) fn record_bytes(&self, ordinal: usize) -> Result<usize, Error> {
        self.entries
            .get(ordinal)
            .map(|e| e.bytes)
            .ok_or(Error::State)
    }
    pub(super) fn read(&self, ordinal: usize, bytes: &mut [u8]) -> Result<(), Error> {
        let entry = self.entries.get(ordinal).ok_or(Error::State)?;
        if bytes.len() != entry.bytes {
            return Err(Error::State);
        }
        let body = entry
            .start
            .checked_add(RECORD_BYTES as u64)
            .ok_or(Error::Limit)?;
        let end = body.checked_add(entry.bytes as u64).ok_or(Error::Limit)?;
        if entry.start < CONTAINER_BYTES as u64 || end > self.identity.len {
            return Err(Error::Changed);
        }
        let mut stored = [0; RECORD_BYTES];
        self.file
            .read_exact_at(&mut stored, entry.start)
            .map_err(|_| Error::Changed)?;
        if stored != header(ordinal, entry.bytes, entry.digest) {
            return Err(Error::Changed);
        }
        self.file
            .read_exact_at(bytes, body)
            .map_err(|_| Error::Changed)?;
        let mut hash = Sha256::new();
        hash.update(bytes);
        if hash.finish() != entry.digest {
            return Err(Error::Changed);
        }
        Ok(())
    }
    #[cfg(test)]
    pub(super) fn guard_count(&self) -> usize {
        self.checks.get()
    }
}

#[cfg(test)]
mod tests;
