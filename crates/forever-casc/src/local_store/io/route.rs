// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original metadata-only bounded archive observations.

use super::super::{Error, Limits, routes::SpanState};
use crate::{archive_record, local_index::Entry};
use std::path::Path;

pub(in crate::local_store) fn observe_span(
    directory: &Path,
    entry: &Entry,
    limits: Limits,
) -> Result<(SpanState, Option<u64>), Error> {
    if u64::from(entry.encoded_size) > limits.max_archive_span_bytes {
        return Err(Error::Limit);
    }
    if entry.archive_index >= 0x1000 {
        return Ok((SpanState::UnsupportedArchive, None));
    }
    let path = super::archive_path(directory, entry.archive_index, limits.max_path_bytes)?;
    observe(&path, entry, limits)
}

fn classify(entry: &Entry, size: u64, limits: Limits) -> Result<(SpanState, Option<u64>), Error> {
    let span = match archive_record::plan(entry, size, limits.max_archive_span_bytes) {
        Ok(_) => SpanState::FitsObservedLength,
        Err(archive_record::Error::Limit) => return Err(Error::Limit),
        Err(archive_record::Error::Bounds) => SpanState::IncompleteSpan,
        Err(archive_record::Error::Extent) => SpanState::InvalidExtent,
        Err(_) => return Err(Error::Integrity),
    };
    Ok((span, Some(size)))
}

#[cfg(target_os = "linux")]
fn observe(path: &Path, entry: &Entry, limits: Limits) -> Result<(SpanState, Option<u64>), Error> {
    use std::{fs, os::unix::fs::OpenOptionsExt};
    let named = match fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok((SpanState::ArchiveAbsent, None));
        }
        Err(_) => return Err(Error::Input),
    };
    if !named.is_file() || named.file_type().is_symlink() {
        return Ok((SpanState::ArchiveRejected, None));
    }
    // Linux O_NONBLOCK | O_NOFOLLOW. No unsafe/raw descriptor is exposed;
    // NONBLOCK prevents a swapped FIFO from hanging before the type check.
    let file = fs::OpenOptions::new()
        .read(true)
        .custom_flags(0x800 | 0x20000)
        .open(path)
        .map_err(|_| Error::InputChanged)?;
    let before = file.metadata().map_err(|_| Error::InputChanged)?;
    let identity = super::Identity::from(&before);
    if !before.is_file() || identity != super::Identity::from(&named) {
        return Err(Error::InputChanged);
    }
    let result = classify(entry, before.len(), limits);
    let after = file.metadata().map_err(|_| Error::InputChanged)?;
    let named_after = fs::symlink_metadata(path).map_err(|_| Error::InputChanged)?;
    if !after.is_file()
        || !named_after.is_file()
        || identity != super::Identity::from(&after)
        || identity != super::Identity::from(&named_after)
    {
        return Err(Error::InputChanged);
    }
    result
}

#[cfg(windows)]
fn observe(path: &Path, entry: &Entry, limits: Limits) -> Result<(SpanState, Option<u64>), Error> {
    let file = match super::windows::archive(path) {
        Ok(file) => file,
        Err(forever_platform::Error::Missing) => return Ok((SpanState::ArchiveAbsent, None)),
        Err(forever_platform::Error::Limit | forever_platform::Error::Allocation) => {
            return Err(Error::Limit);
        }
        Err(forever_platform::Error::Changed) => return Err(Error::InputChanged),
        Err(
            forever_platform::Error::FileType
            | forever_platform::Error::Reparse
            | forever_platform::Error::HardLinks
            | forever_platform::Error::Path,
        ) => {
            return Ok((SpanState::ArchiveRejected, None));
        }
        Err(error) => return Err(super::windows::map_error(error)),
    };
    let result = classify(entry, file.identity().size, limits);
    // Retain ancestors and leaf through the final check even if planning fails.
    file.verify_unchanged().map_err(super::windows::map_error)?;
    result
}

#[cfg(not(any(windows, target_os = "linux")))]
fn observe(_: &Path, _: &Entry, _: Limits) -> Result<(SpanState, Option<u64>), Error> {
    // No weaker fallback for an unreviewed host.
    Err(Error::Input)
}
