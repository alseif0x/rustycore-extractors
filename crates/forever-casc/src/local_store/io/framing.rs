// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original guarded exact-span, key-free framing adapter.
use super::super::{
    Budget, Error, Limits, add,
    framing::{self, Inspection},
    routes::SpanState,
};
use crate::{archive_record, blte, local_index};
use std::path::Path;

pub(in crate::local_store) struct Request<'a> {
    pub directory: &'a Path,
    pub entry: &'a local_index::Entry,
    pub ekey: [u8; 16],
    pub expected: u64,
    pub store_limits: Limits,
    pub limits: framing::Limits,
    pub budget: &'a Budget,
    pub scratch: usize,
}
type Observation = (SpanState, Option<u64>, Inspection);

pub(in crate::local_store) fn inspect_span(request: Request<'_>) -> Result<Observation, Error> {
    if u64::from(request.entry.encoded_size)
        > request
            .limits
            .max_record_bytes
            .min(request.store_limits.max_archive_span_bytes)
    {
        return Err(Error::Limit);
    }
    if request.entry.archive_index >= 0x1000 {
        return Ok((SpanState::UnsupportedArchive, None, Inspection::NotRead));
    }
    let path = super::archive_path(
        request.directory,
        request.entry.archive_index,
        request.store_limits.max_path_bytes,
    )?;
    #[cfg(target_os = "linux")]
    {
        inspect_linux(&path, request, || {})
    }
    #[cfg(windows)]
    {
        inspect_windows(&path, request)
    }
    #[cfg(not(any(target_os = "linux", windows)))]
    {
        let _ = (path, request);
        Err(Error::Input)
    }
}

fn classify(
    request: &Request<'_>,
    size: u64,
) -> Result<Result<archive_record::SpanPlan, SpanState>, Error> {
    match archive_record::plan(
        request.entry,
        size,
        request
            .limits
            .max_record_bytes
            .min(request.store_limits.max_archive_span_bytes),
    ) {
        Ok(plan) => Ok(Ok(plan)),
        Err(archive_record::Error::Limit) => Err(Error::Limit),
        Err(archive_record::Error::Bounds) => Ok(Err(SpanState::IncompleteSpan)),
        Err(archive_record::Error::Extent) => Ok(Err(SpanState::InvalidExtent)),
        Err(_) => Err(Error::Integrity),
    }
}

fn read_and_inspect(
    request: &Request<'_>,
    size: u64,
    read: impl FnOnce(u64, &mut [u8]) -> Result<(), Error>,
) -> Result<Observation, Error> {
    let plan = match classify(request, size)? {
        Ok(plan) => plan,
        Err(span) => return Ok((span, Some(size), Inspection::NotRead)),
    };
    let len = usize::try_from(plan.encoded_size()).map_err(|_| Error::Limit)?;
    request.budget.phase(add(request.scratch, len)?)?;
    let mut record = Vec::new();
    super::reserve(&mut record, len)?;
    request
        .budget
        .phase(add(request.scratch, record.capacity())?)?;
    record.resize(len, 0);
    read(plan.offset(), &mut record)?;
    // The prelude is deliberately ignored. This checks only the index prefix;
    // full EKey authentication and decoded CKey validity are not established.
    let result = if request.ekey[..9] != request.entry.key_prefix {
        Err(archive_record::Error::KeyPrefix.code())
    } else {
        blte::framing::inspect(
            &record[30..],
            request.expected,
            blte::Limits {
                max_encoded_bytes: request.limits.max_record_bytes.saturating_sub(30),
                max_decoded_bytes: request
                    .limits
                    .max_declared_content_bytes
                    .min(request.store_limits.max_decoded_file_bytes),
                max_frames: request
                    .limits
                    .max_frames
                    .min(request.store_limits.max_blte_frames),
            },
        )
        .map_err(|error| error.code())
    };
    Ok((
        SpanState::FitsObservedLength,
        Some(size),
        match result {
            Ok(evidence) => Inspection::Complete(evidence),
            Err(code) => Inspection::Rejected(code),
        },
    ))
}

#[cfg(target_os = "linux")]
fn inspect_linux(
    path: &Path,
    request: Request<'_>,
    after: impl FnOnce(),
) -> Result<Observation, Error> {
    use std::{
        fs,
        io::{Read, Seek, SeekFrom},
        os::unix::fs::OpenOptionsExt,
    };
    let named = match fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok((SpanState::ArchiveAbsent, None, Inspection::NotRead));
        }
        Err(_) => return Err(Error::Input),
    };
    if !named.is_file() || named.file_type().is_symlink() {
        return Ok((SpanState::ArchiveRejected, None, Inspection::NotRead));
    }
    let mut file = fs::OpenOptions::new()
        .read(true)
        .custom_flags(0x800 | 0x20000)
        .open(path)
        .map_err(|_| Error::InputChanged)?;
    let before = file.metadata().map_err(|_| Error::InputChanged)?;
    let identity = super::Identity::from(&before);
    if !before.is_file() || identity != super::Identity::from(&named) {
        return Err(Error::InputChanged);
    }
    let result = read_and_inspect(&request, before.len(), |offset, bytes| {
        file.seek(SeekFrom::Start(offset))
            .map_err(|_| Error::InputChanged)?;
        file.read_exact(bytes).map_err(|_| Error::InputChanged)
    });
    after();
    let held = file.metadata().map_err(|_| Error::InputChanged)?;
    let named = fs::symlink_metadata(path).map_err(|_| Error::InputChanged)?;
    if !held.is_file()
        || !named.is_file()
        || identity != super::Identity::from(&held)
        || identity != super::Identity::from(&named)
    {
        return Err(Error::InputChanged);
    }
    result
}

#[cfg(windows)]
fn inspect_windows(path: &Path, request: Request<'_>) -> Result<Observation, Error> {
    let file = match super::windows::archive(path) {
        Ok(file) => file,
        Err(forever_platform::Error::Missing) => {
            return Ok((SpanState::ArchiveAbsent, None, Inspection::NotRead));
        }
        Err(forever_platform::Error::Limit | forever_platform::Error::Allocation) => {
            return Err(Error::Limit);
        }
        Err(forever_platform::Error::Changed) => return Err(Error::InputChanged),
        Err(
            forever_platform::Error::FileType
            | forever_platform::Error::Reparse
            | forever_platform::Error::HardLinks
            | forever_platform::Error::Path,
        ) => return Ok((SpanState::ArchiveRejected, None, Inspection::NotRead)),
        Err(error) => return Err(super::windows::map_error(error)),
    };
    let result = read_and_inspect(&request, file.identity().size, |offset, bytes| {
        super::windows::read_range(&file, offset, bytes)
    });
    // Required on allocation, read, planning and parsing errors as well.
    file.verify_unchanged().map_err(super::windows::map_error)?;
    result
}

#[cfg(all(test, target_os = "linux"))]
pub(in crate::local_store) fn inspect_with_hook(
    request: Request<'_>,
    after: impl FnOnce(),
) -> Result<Observation, Error> {
    let path = super::archive_path(
        request.directory,
        request.entry.archive_index,
        request.store_limits.max_path_bytes,
    )?;
    inspect_linux(&path, request, after)
}
