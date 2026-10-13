// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-13: original Windows exclusive private append repository.
//! Fresh private intermediates, never published artifacts. Native ownership
//! protects one append handle and the captured input/directory chains. Record
//! framing, SHA256, exact reads and all logical resource accounting live here.
//! No process resume, cleanup, overwrite or atomic snapshot is provided.

use super::{payload::Error, resources};
use crate::sha256::Sha256;
use forever_platform::windows::append as native;
use std::path::Path;

const CONTAINER_BYTES: usize = 32;
const RECORD_BYTES: usize = 64;
const IO_BYTES: usize = 1024 * 1024;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BackendProfile {
    #[default]
    ExclusiveAppendV1,
}
impl BackendProfile {
    pub const fn id(self) -> &'static str {
        "windows-exclusive-append-v1"
    }
    pub const fn supported(self) -> bool {
        native::supported()
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Profile {
    #[default]
    FixedNtfsV1,
}
impl Profile {
    pub const fn id(self) -> &'static str {
        "windows-fixed-ntfs-spool-volumes-v1"
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub max_records: usize,
    pub max_disk_bytes: usize,
    pub max_active_bytes: usize,
    pub max_descriptor_bytes: usize,
    pub max_reloads: usize,
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
    fn validate_resources(self, profile: resources::Profile) -> Result<(), Error> {
        let cap = profile.spool_limits();
        if !profile.supported()
            || self.max_records > cap.max_records
            || self.max_disk_bytes > cap.max_disk_bytes
            || self.max_active_bytes > cap.max_active_bytes
            || self.max_descriptor_bytes > cap.max_descriptor_bytes
            || self.max_reloads > cap.max_reloads
            || self.max_reload_bytes > cap.max_reload_bytes
        {
            return Err(Error::InvalidLimits);
        }
        Ok(())
    }
}

/// Explicit source/key ownership declaration; no source or key content is read.
/// The source must be the LocalStore root, and every supplied key must appear.
pub struct ProtectedInputs {
    inner: native::ProtectedInputs,
}
impl ProtectedInputs {
    pub fn capture(source_root: &Path, key_files: &[&Path]) -> Result<Self, Error> {
        Ok(Self {
            inner: native::ProtectedInputs::capture(source_root, key_files).map_err(mapped)?,
        })
    }
}

#[derive(Clone, Copy)]
pub(crate) struct Record(usize);
#[derive(Clone, Copy)]
struct Entry {
    start: u64,
    bytes: usize,
    digest: [u8; 32],
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stats {
    pub records: usize,
    pub disk_bytes: usize,
    pub reloads: usize,
    pub reload_bytes: usize,
    pub descriptor_bytes: usize,
    pub peak_reload_bytes: usize,
}
pub struct Spool {
    archive: native::OwnedArchive,
    entries: Vec<Entry>,
    limits: Limits,
    profile: Profile,
    resources: resources::Profile,
    disk_bytes: usize,
    reloads: usize,
    reload_bytes: usize,
    peak_reload_bytes: usize,
    failed: Option<Error>,
}

fn mapped(error: native::Error) -> Error {
    match error {
        native::Error::Allocation => Error::Allocation,
        native::Error::Limit => Error::Limit,
        native::Error::Path => Error::Path,
        native::Error::Alias => Error::Alias,
        native::Error::Exists => Error::Exists,
        native::Error::Unsupported => Error::BackendUnavailable,
        native::Error::Changed => Error::Changed,
        native::Error::Io => Error::Io,
        native::Error::Poisoned => Error::State,
    }
}
fn add(a: usize, b: usize) -> Result<usize, Error> {
    a.checked_add(b).ok_or(Error::Limit)
}
fn allocation(count: usize) -> Result<usize, Error> {
    count.checked_mul(size_of::<Entry>()).ok_or(Error::Limit)
}
fn container() -> [u8; CONTAINER_BYTES] {
    let mut header = [0; CONTAINER_BYTES];
    header[..8].copy_from_slice(b"RCEARC1\0");
    header[8..12].copy_from_slice(&1u32.to_le_bytes());
    header[12..16].copy_from_slice(&(CONTAINER_BYTES as u32).to_le_bytes());
    header
}
fn header(ordinal: usize, bytes: usize, digest: [u8; 32]) -> [u8; RECORD_BYTES] {
    let mut header = [0; RECORD_BYTES];
    header[..8].copy_from_slice(b"RCERCD1\0");
    header[8..16].copy_from_slice(&(ordinal as u64).to_le_bytes());
    header[16..24].copy_from_slice(&(bytes as u64).to_le_bytes());
    header[24..56].copy_from_slice(&digest);
    header
}
impl Spool {
    pub fn create(path: &Path, protected: ProtectedInputs, limits: Limits) -> Result<Self, Error> {
        Self::create_with_profile(path, protected, limits, Profile::FixedNtfsV1)
    }
    pub fn create_with_profile(
        path: &Path,
        protected: ProtectedInputs,
        limits: Limits,
        profile: Profile,
    ) -> Result<Self, Error> {
        Self::create_with_policy(
            path,
            protected,
            limits,
            profile,
            resources::Profile::BoundedV1,
        )
    }
    pub fn create_with_policy(
        path: &Path,
        protected: ProtectedInputs,
        limits: Limits,
        profile: Profile,
        resources: resources::Profile,
    ) -> Result<Self, Error> {
        Self::create_with_backend(
            path,
            protected,
            limits,
            profile,
            resources,
            BackendProfile::ExclusiveAppendV1,
        )
    }
    pub fn create_with_backend(
        path: &Path,
        protected: ProtectedInputs,
        limits: Limits,
        profile: Profile,
        resources: resources::Profile,
        backend: BackendProfile,
    ) -> Result<Self, Error> {
        native::admit().map_err(mapped)?;
        if !backend.supported() {
            return Err(Error::BackendUnavailable);
        }
        limits.validate_resources(resources)?;
        if limits.max_disk_bytes < CONTAINER_BYTES {
            return Err(Error::Limit);
        }
        let native_limits = native::Limits {
            max_bytes: limits.max_disk_bytes as u64,
            max_metadata_bytes: native::Limits::default()
                .max_metadata_bytes
                .min(limits.max_descriptor_bytes),
            ..native::Limits::default()
        };
        let mut archive =
            native::OwnedArchive::create(path, protected.inner, native_limits).map_err(mapped)?;
        if archive.resident_bytes().map_err(mapped)? > limits.max_descriptor_bytes {
            return Err(Error::Limit);
        }
        let mut txn = archive
            .begin_append(CONTAINER_BYTES as u64)
            .map_err(mapped)?;
        txn.write_all(&container()).map_err(mapped)?;
        txn.flush().map_err(mapped)?;
        let mut stored = [0; CONTAINER_BYTES];
        txn.read_exact_at(0, &mut stored).map_err(mapped)?;
        if stored != container() {
            return Err(Error::Changed);
        }
        txn.commit().map_err(mapped)?;
        let mut out = Self {
            archive,
            entries: Vec::new(),
            limits,
            profile,
            resources,
            disk_bytes: CONTAINER_BYTES,
            reloads: 0,
            reload_bytes: 0,
            peak_reload_bytes: 0,
            failed: None,
        };
        out.verify_unchanged()?;
        Ok(out)
    }
    pub const fn backend(&self) -> BackendProfile {
        BackendProfile::ExclusiveAppendV1
    }
    pub const fn resources(&self) -> resources::Profile {
        self.resources
    }
    pub const fn profile(&self) -> Profile {
        self.profile
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
        add(
            self.archive.resident_bytes().map_err(mapped)?,
            allocation(self.entries.capacity())?,
        )
    }
    pub(crate) fn prepare_records(&mut self, count: usize, budget: usize) -> Result<(), Error> {
        if count > self.limits.max_records {
            return Err(Error::Limit);
        }
        if count <= self.entries.capacity() {
            return Ok(());
        }
        let before = self.resident_bytes()?;
        let available = budget.min(
            self.limits
                .max_descriptor_bytes
                .checked_sub(before)
                .ok_or(Error::Limit)?,
        );
        let grown = count.max(
            self.entries
                .capacity()
                .checked_mul(2)
                .unwrap_or(self.limits.max_records)
                .min(self.limits.max_records),
        );
        let requested = if allocation(grown)? <= available {
            grown
        } else {
            count
        };
        if allocation(requested)? > available {
            return Err(Error::Limit);
        }
        self.entries
            .try_reserve_exact(requested - self.entries.len())
            .map_err(|_| Error::Allocation)?;
        // The old allocation and the full new allocation coexist during growth.
        // Actual capacity is checked independently of the requested capacity.
        if allocation(self.entries.capacity())? > available {
            return Err(Error::Limit);
        }
        Ok(())
    }
    pub(crate) fn failure(&self) -> Option<Error> {
        self.failed.or_else(|| self.archive.failure().map(mapped))
    }
    fn check(&self) -> Result<(), Error> {
        self.archive.guard().map_err(mapped)?;
        let mut stored = [0; CONTAINER_BYTES];
        self.archive.read_exact_at(0, &mut stored).map_err(mapped)?;
        if stored != container() {
            return Err(Error::Changed);
        }
        self.failed.map_or(Ok(()), Err)
    }
    fn remember(&mut self, error: Error) -> Error {
        if error == Error::Changed {
            self.failed = Some(error);
        } else {
            self.failed.get_or_insert(error);
        }
        self.archive.poison();
        self.failed.unwrap_or(error)
    }
    pub fn verify_unchanged(&mut self) -> Result<(), Error> {
        let result = self.check();
        result.map_err(|e| self.remember(e))
    }
    fn finish<T>(&mut self, result: Result<T, Error>) -> Result<T, Error> {
        let guard = self.check();
        match (result, guard) {
            (Ok(value), Ok(())) => Ok(value),
            (Ok(value), Err(error)) => {
                drop(value);
                let again = self.check();
                let error = if again == Err(Error::Changed) {
                    Error::Changed
                } else {
                    error
                };
                Err(self.remember(error))
            }
            (Err(error), guard) => {
                let error = if guard == Err(Error::Changed) {
                    Error::Changed
                } else {
                    error
                };
                Err(self.remember(error))
            }
        }
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
            let extent = add(RECORD_BYTES, size)?;
            let disk = add(self.disk_bytes, extent)?;
            if disk > self.limits.max_disk_bytes {
                return Err(Error::Limit);
            }
            let budget = self
                .limits
                .max_descriptor_bytes
                .checked_sub(self.resident_bytes()?)
                .ok_or(Error::Limit)?;
            self.prepare_records(self.entries.len() + 1, budget)?;
            self.disk_bytes = disk;
            let ordinal = self.entries.len();
            let mut txn = self.archive.begin_append(extent as u64).map_err(mapped)?;
            let start = txn.start();
            txn.write_all(&header(ordinal, size, [0; 32]))
                .map_err(mapped)?;
            let digest = {
                let mut stream = |bytes: &[u8]| {
                    for part in bytes.chunks(IO_BYTES) {
                        txn.write_all(part).map_err(mapped)?;
                    }
                    Ok(())
                };
                let mut output = RecordWriter {
                    write: &mut stream,
                    hash: Sha256::new(),
                    written: 0,
                    expected: size,
                };
                writer(&mut output)?;
                if output.written != size {
                    return Err(Error::Format);
                }
                output.hash.finish()
            };
            txn.patch_at(0, &header(ordinal, size, digest))
                .map_err(mapped)?;
            txn.flush().map_err(mapped)?;
            let mut hash = Sha256::new();
            let mut scratch = [0; 8192];
            let mut consumed = 0;
            while consumed < size {
                let n = (size - consumed).min(scratch.len());
                txn.read_exact_at((RECORD_BYTES + consumed) as u64, &mut scratch[..n])
                    .map_err(mapped)?;
                hash.update(&scratch[..n]);
                consumed += n;
            }
            let mut stored = [0; RECORD_BYTES];
            txn.read_exact_at(0, &mut stored).map_err(mapped)?;
            if hash.finish() != digest || stored != header(ordinal, size, digest) {
                return Err(Error::Changed);
            }
            txn.commit().map_err(mapped)?;
            self.entries.push(Entry {
                start,
                bytes: size,
                digest,
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
        let result = (|| {
            self.verify_unchanged()?;
            let entry = *self.entries.get(record.0).ok_or(Error::State)?;
            if entry.bytes > budget.min(self.limits.max_active_bytes)
                || self.reloads >= self.limits.max_reloads
            {
                return Err(Error::Limit);
            }
            let total = add(self.reload_bytes, add(RECORD_BYTES, entry.bytes)?)?;
            if total > self.limits.max_reload_bytes {
                return Err(Error::Limit);
            }
            self.reloads += 1;
            self.reload_bytes = total;
            let mut bytes = Vec::new();
            bytes
                .try_reserve_exact(entry.bytes)
                .map_err(|_| Error::Allocation)?;
            if bytes.capacity() > budget.min(self.limits.max_active_bytes) {
                return Err(Error::Limit);
            }
            self.peak_reload_bytes = self.peak_reload_bytes.max(bytes.capacity());
            bytes.resize(entry.bytes, 0);
            let mut stored = [0; RECORD_BYTES];
            self.archive
                .read_exact_at(entry.start, &mut stored)
                .map_err(mapped)?;
            if stored != header(record.0, entry.bytes, entry.digest) {
                return Err(Error::Changed);
            }
            let body = entry
                .start
                .checked_add(RECORD_BYTES as u64)
                .ok_or(Error::Limit)?;
            let end = body.checked_add(entry.bytes as u64).ok_or(Error::Limit)?;
            if entry.start < CONTAINER_BYTES as u64 || end > self.archive.len() {
                return Err(Error::Changed);
            }
            for (i, part) in bytes.chunks_mut(IO_BYTES).enumerate() {
                self.archive
                    .read_exact_at(body + (i * IO_BYTES) as u64, part)
                    .map_err(mapped)?;
            }
            let mut hash = Sha256::new();
            hash.update(&bytes);
            if hash.finish() != entry.digest {
                return Err(Error::Changed);
            }
            let mut poison = CallbackPoison {
                archive: &self.archive,
                armed: true,
            };
            let result = call(&bytes);
            poison.armed = false;
            result
        })();
        self.finish(result)
    }
}
struct CallbackPoison<'a> {
    archive: &'a native::OwnedArchive,
    armed: bool,
}
impl Drop for CallbackPoison<'_> {
    fn drop(&mut self) {
        if self.armed {
            self.archive.poison();
        }
    }
}
pub(crate) struct RecordWriter<'a> {
    write: &'a mut dyn FnMut(&[u8]) -> Result<(), Error>,
    hash: Sha256,
    written: usize,
    expected: usize,
}
impl RecordWriter<'_> {
    pub(crate) fn bytes(&mut self, bytes: &[u8]) -> Result<(), Error> {
        self.written = add(self.written, bytes.len())?;
        if self.written > self.expected {
            return Err(Error::Format);
        }
        (self.write)(bytes)?;
        self.hash.update(bytes);
        Ok(())
    }
}
#[cfg(test)]
#[path = "windows/tests.rs"]
mod tests;
