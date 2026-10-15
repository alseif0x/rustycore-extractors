// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-15: explicit Linux70374 disabled-key availability only.
//! Complete CKey reads retain their original API/policy. This separate source
//! exposes only authenticated available spans and explicit encrypted holes.
//! No conversion to VerifiedFile or complete Vec exists. Initial partial routing
//! accepts exactly one independently authenticated encoding, never a merged or
//! maximal-coverage choice. Candidate work includes the preceding strict pass.
#![cfg(target_os = "linux")]

use super::{Budget, CandidateFailures, LocalStore, add, array_bytes, io, owned, reserve};
use crate::{archive_record, blte};
use std::{
    cell::Cell,
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom},
    os::unix::fs::{MetadataExt, OpenOptionsExt},
};

pub use blte::available::{CoverageRange, RangeError, RangeState};
pub const PROFILE: &str = "linux70374-disabled-key-available-source-v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Store(super::Error),
    Blte(blte::Error),
    KeysPresent,
    Build,
    AmbiguousEncoding,
    UnsupportedArchive,
}
impl Error {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Store(e) => e.code(),
            Self::Blte(e) => e.code(),
            Self::KeysPresent => "available_source_keys_not_admitted",
            Self::Build => "available_source_build_not_admitted",
            Self::AmbiguousEncoding => "available_source_ambiguous_encoding",
            Self::UnsupportedArchive => "available_source_archive_not_admitted",
        }
    }
}
impl From<super::Error> for Error {
    fn from(e: super::Error) -> Self {
        Self::Store(e)
    }
}
impl From<blte::Error> for Error {
    fn from(e: blte::Error) -> Self {
        Self::Blte(e)
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}
impl std::error::Error for Error {}

/// Logical candidate work, not physical I/O, exact CPU, heap or RSS accounting.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Work {
    pub strict_candidate_attempts: usize,
    pub partial_candidate_attempts: usize,
    /// Complete spans actually read by the second pass, including duplicate
    /// physical routes for one encoding. Strict pass I/O is not measured here.
    pub partial_encoded_bytes_read: u64,
}
pub struct AvailableSource {
    file_data_id: u32,
    data: blte::available::AvailableBytes,
    encoded_key_authenticated: bool,
    work: Work,
}
impl AvailableSource {
    pub const fn file_data_id(&self) -> u32 {
        self.file_data_id
    }
    pub fn declared_bytes(&self) -> u64 {
        self.data.declared_bytes()
    }
    pub fn available_bytes(&self) -> u64 {
        self.data.available_bytes()
    }
    pub fn coverage(&self) -> &[CoverageRange] {
        self.data.coverage()
    }
    pub fn range(&self, offset: u64, bytes: usize) -> Result<&[u8], RangeError> {
        self.data.range(offset, bytes)
    }
    pub fn full_ckey_verified(&self) -> bool {
        self.data.full_ckey_verified()
    }
    /// A strict complete return authenticates plaintext by CKey. Only the new
    /// partial pass additionally authenticates the full independently selected
    /// EKey/header and every encoded frame; these claims remain distinct.
    pub const fn encoded_key_authenticated(&self) -> bool {
        self.encoded_key_authenticated
    }
    pub const fn work(&self) -> Work {
        self.work
    }
    pub fn resident_bytes(&self) -> Result<usize, Error> {
        Ok(self.data.resident_bytes()?)
    }
}
impl forever_db2::availability::Source for AvailableSource {
    fn declared_bytes(&self) -> u64 {
        AvailableSource::declared_bytes(self)
    }
    fn range(
        &self,
        offset: u64,
        bytes: usize,
    ) -> Result<&[u8], forever_db2::availability::Unavailable> {
        AvailableSource::range(self, offset, bytes)
            .map_err(|_| forever_db2::availability::Unavailable)
    }
}

impl LocalStore {
    /// New opt-in Linux70374 profile. Keys must be disabled, and the limit can
    /// only tighten existing ceilings. Failed strict reads are not accepted as
    /// a plaintext prefix. All guards close even when the new pass fails.
    pub fn read_file_data_id_available_with_limit(
        &mut self,
        file_data_id: u32,
        max_decoded_bytes: u64,
    ) -> Result<AvailableSource, Error> {
        if self.build.build != 70374 {
            return Err(Error::Build);
        }
        if self.keys.is_some() {
            return Err(Error::KeysPresent);
        }
        io::check_guards(&self.guards)?;
        let result = self.available_inner(file_data_id, max_decoded_bytes);
        io::check_guards(&self.guards)?;
        result
    }

    fn available_inner(&mut self, id: u32, max_decoded: u64) -> Result<AvailableSource, Error> {
        let failure = match self.read_file_data_id_with_limit(id, max_decoded) {
            Ok(full) => {
                let budget = Budget {
                    limits: self.limits,
                    resident: self.stats.resident_bytes,
                    scratch: Cell::new(0),
                };
                budget.phase(add(full.bytes.capacity(), size_of::<CoverageRange>())?)?;
                let attempts = full.candidate_attempts;
                return Ok(AvailableSource {
                    file_data_id: id,
                    data: blte::available::AvailableBytes::from_verified(full.bytes)?,
                    encoded_key_authenticated: false,
                    work: Work {
                        strict_candidate_attempts: attempts,
                        ..Work::default()
                    },
                });
            }
            Err(super::Error::NoLocalEncoding(f)) if only_encrypted(f) => f,
            Err(e) => return Err(e.into()),
        };
        let mut work = Work {
            strict_candidate_attempts: failure.encrypted as usize,
            ..Work::default()
        };
        let selected = self
            .root
            .resolve(id, self.locale, |k| self.central.contains(k))
            .ok_or(super::Error::MissingFile)?;
        let ckey = selected.content_key;
        let size = u64::from(
            self.central
                .content_size(ckey)
                .ok_or(super::Error::UnknownSize)?,
        );
        let max_decoded = max_decoded.min(self.limits.max_decoded_file_bytes);
        if size > max_decoded {
            return Err(super::Error::Limit.into());
        }
        let budget = Budget {
            limits: self.limits,
            resident: self.stats.resident_bytes,
            scratch: Cell::new(0),
        };
        let mut chosen: Option<([u8; 16], blte::available::AvailableBytes)> = None;
        for ekey in self.central.encoded_keys(ckey) {
            let indexes = owned::equal_range(&self.index_order, |index| {
                self.indexes[index].key_prefix.as_slice().cmp(&ekey[..9])
            });
            if indexes.is_empty() {
                return Err(super::Error::MissingFile.into());
            }
            for &index in &self.index_order[indexes] {
                work.partial_candidate_attempts = add(work.partial_candidate_attempts, 1)?;
                if add(
                    work.strict_candidate_attempts,
                    work.partial_candidate_attempts,
                )? > self.limits.max_candidate_attempts
                {
                    return Err(super::Error::Limit.into());
                }
                let retained = chosen
                    .as_ref()
                    .map(|(_, d)| d.resident_bytes())
                    .transpose()?
                    .unwrap_or(0);
                let decoded = self.available_candidate(
                    &self.indexes[index as usize],
                    ekey,
                    ckey,
                    size,
                    max_decoded,
                    &budget,
                    retained,
                    &mut work,
                )?;
                if let Some((previous, _)) = &chosen {
                    if *previous != ekey {
                        return Err(Error::AmbiguousEncoding);
                    }
                    // Same full authenticated header/EKey and checked frame
                    // hashes describe one encoding, not a second coverage vote.
                } else {
                    chosen = Some((ekey, decoded));
                }
            }
        }
        let (_, data) = chosen.ok_or(super::Error::MissingFile)?;
        Ok(AvailableSource {
            file_data_id: id,
            data,
            encoded_key_authenticated: true,
            work,
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn available_candidate(
        &self,
        entry: &crate::local_index::Entry,
        ekey: [u8; 16],
        ckey: [u8; 16],
        size: u64,
        max_decoded: u64,
        budget: &Budget,
        retained: usize,
        work: &mut Work,
    ) -> Result<blte::available::AvailableBytes, Error> {
        if entry.archive_index >= 0x1000 {
            return Err(Error::UnsupportedArchive);
        }
        let path = io::archive_path(
            &self.data_dir,
            entry.archive_index,
            self.limits.max_path_bytes,
        )?;
        let mut file: File = OpenOptions::new()
            .read(true)
            .custom_flags(0x20000 | 0x800)
            .open(&path)
            .map_err(|_| super::Error::Input)?;
        let before = file.metadata().map_err(|_| super::Error::Input)?;
        if !before.is_file() {
            return Err(super::Error::Input.into());
        }
        let plan = archive_record::plan(entry, before.len(), self.limits.max_archive_span_bytes)
            .map_err(|e| match e {
                archive_record::Error::Limit => super::Error::Limit,
                _ => super::Error::Integrity,
            })?;
        let span = usize::try_from(plan.encoded_size()).map_err(|_| super::Error::Limit)?;
        let output = usize::try_from(size).map_err(|_| super::Error::Limit)?;
        let ranges = array_bytes::<CoverageRange>(self.limits.max_blte_frames as usize)?;
        budget.phase(add(retained, add(span, add(output, ranges)?)?)?)?;
        let mut encoded = Vec::new();
        reserve(&mut encoded, span)?;
        encoded.resize(span, 0);
        file.seek(SeekFrom::Start(plan.offset()))
            .map_err(|_| super::Error::Input)?;
        let read_result = file.read_exact(&mut encoded);
        let after = file.metadata().map_err(|_| super::Error::InputChanged)?;
        let named = path
            .symlink_metadata()
            .map_err(|_| super::Error::InputChanged)?;
        if identity(&before) != identity(&after) || identity(&before) != identity(&named) {
            return Err(super::Error::InputChanged.into());
        }
        read_result.map_err(|_| super::Error::Input)?;
        work.partial_encoded_bytes_read = work
            .partial_encoded_bytes_read
            .checked_add(span as u64)
            .ok_or(super::Error::Limit)?;
        // Exact local record/prelude plan, never signature search or guessed metadata.
        if encoded.len() < 30 || ekey[..9] != entry.key_prefix {
            return Err(super::Error::Integrity.into());
        }
        let result = blte::available::decode(
            &encoded[30..],
            size,
            ekey,
            ckey,
            blte::Limits {
                max_encoded_bytes: self.limits.max_archive_span_bytes,
                max_decoded_bytes: max_decoded,
                max_frames: self.limits.max_blte_frames,
            },
        );
        let after = file.metadata().map_err(|_| super::Error::InputChanged)?;
        let named = path
            .symlink_metadata()
            .map_err(|_| super::Error::InputChanged)?;
        if identity(&before) != identity(&after) || identity(&before) != identity(&named) {
            return Err(super::Error::InputChanged.into());
        }
        let decoded = result?;
        budget.phase(add(
            retained,
            add(encoded.capacity(), decoded.resident_bytes()?)?,
        )?)?;
        Ok(decoded)
    }
}
fn only_encrypted(f: CandidateFailures) -> bool {
    f.encrypted > 0 && f.missing == 0 && f.incomplete == 0 && f.corrupt == 0 && f.unsupported == 0
}
fn identity(m: &std::fs::Metadata) -> (u64, u64, u32, u64, i64, i64, i64, i64) {
    (
        m.dev(),
        m.ino(),
        m.mode(),
        m.len(),
        m.mtime(),
        m.mtime_nsec(),
        m.ctime(),
        m.ctime_nsec(),
    )
}
