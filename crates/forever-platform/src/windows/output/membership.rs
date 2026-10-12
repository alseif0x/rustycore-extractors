// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-12: original bounded NTFS directory membership observation.
//! Explicit membership observation for a live private-output root. Every owned
//! directory is enumerated through its existing synchronous LIST_DIRECTORY
//! keeper. Exact canonical child spelling, NTFS FileId and file/directory kind
//! must match the current owner inventory. No names or partial inventory escape.
//!
//! Pre/post v1 guards and transient full directory metadata comparisons detect
//! observed changes. Windows documents that concurrent changes may or may not
//! appear in enumeration: this is not an atomic namespace/content snapshot,
//! same-SID isolation, authentication, reservation, publication or durability.
//! No handles are opened, adopted, deleted or exposed by enumeration. The root
//! is not Sync, so safe callers cannot concurrently change its directory cursor.
//!
//! Limits count enumerated directories, admitted decoded records (including dot
//! entries), and NtQueryDirectoryFile calls. At the record limit one fixed-buffer
//! query may establish EOF; a returned unadmitted record fails without decoding.
//! The query-call ceiling is never exceeded. Existing v1 guard I/O is separate;
//! these are logical work limits, not RSS, all I/O or all CPU measurements.

use super::{Entry, Kind, MAX_NAME, MAX_OBJECTS, Name, PrivateRoot};
use crate::windows::Identity;

#[allow(unsafe_code)]
mod ffi;
#[cfg(test)]
mod tests;

pub const WINDOWS_MEMBERSHIP_PROFILE: &str = "forever-windows-private-membership-v1";
const MAX_DIRECTORIES: usize = 128;
const MAX_RECORDS: usize = 512;
const MAX_QUERY_CALLS: usize = 640;
const BUFFER_BYTES: usize = 640;
const NAME_OFFSET: usize = 80;
const MAX_NATIVE_NAME_UNITS: usize = 255;

/// Downward-only per-call admission. Zero and values above the profile ceilings
/// reject before filesystem I/O and do not poison the borrowed root.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub max_directories: usize,
    pub max_records: usize,
    pub max_query_calls: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_directories: MAX_DIRECTORIES,
            max_records: MAX_RECORDS,
            max_query_calls: MAX_QUERY_CALLS,
        }
    }
}
impl Limits {
    fn validate(self) -> Result<Self, Error> {
        if self.max_directories == 0
            || self.max_directories > MAX_DIRECTORIES
            || self.max_records == 0
            || self.max_records > MAX_RECORDS
            || self.max_query_calls == 0
            || self.max_query_calls > MAX_QUERY_CALLS
        {
            return Err(Error::Limit);
        }
        Ok(self)
    }
}

/// Complete successful observation only. Records include valid . and .. entries
/// when the filesystem supplies them; children exclude those dot entries.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Summary {
    pub directories: usize,
    pub records: usize,
    pub children: usize,
    pub query_calls: usize,
}

/// Static, redacted errors. Query/scan/budget failures poison the root even if
/// an external writer subsequently repairs the observed namespace.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Limit,
    Budget,
    Changed,
    Query,
    Framing,
    Name,
    Reparse,
    Extra,
    Missing,
    Duplicate,
    Identity,
    FileType,
}
impl Error {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Limit => "windows_membership_limit",
            Self::Budget => "windows_membership_budget",
            Self::Changed => "windows_membership_changed",
            Self::Query => "windows_membership_query",
            Self::Framing => "windows_membership_framing",
            Self::Name => "windows_membership_name",
            Self::Reparse => "windows_membership_reparse",
            Self::Extra => "windows_membership_extra",
            Self::Missing => "windows_membership_missing",
            Self::Duplicate => "windows_membership_duplicate",
            Self::Identity => "windows_membership_identity",
            Self::FileType => "windows_membership_file_type",
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}
impl std::error::Error for Error {}

/// Enumerates all owned directories with a fresh RestartScan per directory.
/// Valid-limit errors permanently poison root; invalid Limits perform no I/O.
/// There is no recovery, adoption, public listing or partial-success return.
pub fn verify_membership(root: &PrivateRoot<'_>, limits: Limits) -> Result<Summary, Error> {
    let limits = limits.validate()?;
    if root.invalid.get() {
        return Err(Error::Changed);
    }
    root.verify_unchanged().map_err(|_| Error::Changed)?;
    let scanned = scan(root, limits);
    // Run the final v1 guard even when scanning failed. Its existing ownership
    // policy is preserved; the explicit membership failure poisons below.
    let guarded = root.verify_unchanged().map_err(|_| Error::Changed);
    let result = scanned.and_then(|summary| guarded.map(|()| summary));
    if result.is_err() {
        root.invalid.set(true);
    }
    result
}

#[derive(Clone, Copy)]
struct Expected {
    name: Name,
    parent: Option<usize>,
    file_id: u64,
    directory: bool,
}
impl Expected {
    const EMPTY: Self = Self {
        name: Name {
            bytes: [0; MAX_NAME],
            len: 0,
        },
        parent: None,
        file_id: 0,
        directory: false,
    };
    fn from_entry(entry: &Entry) -> Self {
        Self {
            name: entry.name,
            parent: entry.parent,
            file_id: entry.identity.file_index,
            directory: entry.directory(),
        }
    }
}
#[derive(Clone, Copy)]
enum RecordName {
    Dot,
    DotDot,
    Child(Name),
}
#[derive(Clone, Copy)]
struct Record {
    name: RecordName,
    file_id: u64,
    directory: bool,
}

fn scan(root: &PrivateRoot<'_>, limits: Limits) -> Result<Summary, Error> {
    if root.entries.len() > MAX_OBJECTS {
        return Err(Error::Budget);
    }
    let directory_count = root
        .entries
        .iter()
        .filter(|entry| entry.directory())
        .count();
    if directory_count > limits.max_directories {
        return Err(Error::Budget);
    }
    let mut inventory = [Expected::EMPTY; MAX_OBJECTS];
    let mut before = [None; MAX_OBJECTS];
    for (index, entry) in root.entries.iter().enumerate() {
        inventory[index] = Expected::from_entry(entry);
        if entry.directory() {
            before[index] = Some(full_directory(entry)?);
        }
    }
    let inventory = &inventory[..root.entries.len()];
    let mut seen = [false; MAX_OBJECTS];
    let mut summary = Summary::default();
    let mut buffer = ffi::Buffer::new();
    for (directory, entry) in root.entries.iter().enumerate() {
        let Kind::Directory { keeper, .. } = &entry.kind else {
            continue;
        };
        summary.directories += 1;
        let mut restart = true;
        let mut dots = [false; 2];
        loop {
            if summary.query_calls >= limits.max_query_calls {
                return Err(Error::Budget);
            }
            summary.query_calls += 1;
            let length = ffi::query(keeper, &mut buffer, restart)?;
            restart = false;
            let Some(length) = length else {
                break;
            };
            if summary.records >= limits.max_records {
                return Err(Error::Budget);
            }
            summary.records += 1;
            let record = decode(&buffer.bytes()[..length])?;
            if accept(
                inventory,
                directory,
                root.parent.identity().file_index,
                record,
                &mut seen,
                &mut dots,
            )? {
                summary.children += 1;
            }
        }
        complete(inventory, directory, &seen)?;
    }
    // Full transient directory metadata spans the whole scan, not merely the
    // moment a particular directory was enumerated. Access time is excluded.
    directories_unchanged(root, &before)?;
    Ok(summary)
}
fn directories_unchanged(
    root: &PrivateRoot<'_>,
    before: &[Option<Identity>; MAX_OBJECTS],
) -> Result<(), Error> {
    for (index, entry) in root.entries.iter().enumerate() {
        if let Some(captured) = before[index]
            && full_directory(entry)? != captured
        {
            return Err(Error::Changed);
        }
    }
    Ok(())
}
fn full_directory(entry: &Entry) -> Result<Identity, Error> {
    let current = crate::windows::ffi::identity(entry.file()).map_err(|_| Error::Changed)?;
    if !entry.identity.same_object(current) || current.links != 1 {
        return Err(Error::Changed);
    }
    Ok(current)
}
fn accept(
    expected: &[Expected],
    directory: usize,
    external_parent: u64,
    record: Record,
    seen: &mut [bool; MAX_OBJECTS],
    dots: &mut [bool; 2],
) -> Result<bool, Error> {
    let owned = expected.get(directory).ok_or(Error::Identity)?;
    match record.name {
        RecordName::Dot | RecordName::DotDot => {
            let dot = if matches!(record.name, RecordName::Dot) {
                0
            } else {
                1
            };
            if dots[dot] {
                return Err(Error::Duplicate);
            }
            let file_id = if dot == 0 {
                owned.file_id
            } else {
                match owned.parent {
                    Some(parent) => expected.get(parent).ok_or(Error::Identity)?.file_id,
                    None => external_parent,
                }
            };
            if !record.directory {
                return Err(Error::FileType);
            }
            if record.file_id != file_id {
                return Err(Error::Identity);
            }
            dots[dot] = true;
            Ok(false)
        }
        RecordName::Child(name) => {
            let (index, child) = expected
                .iter()
                .enumerate()
                .find(|(_, child)| child.parent == Some(directory) && child.name == name)
                .ok_or(Error::Extra)?;
            if seen[index] {
                return Err(Error::Duplicate);
            }
            if child.file_id != record.file_id {
                return Err(Error::Identity);
            }
            if child.directory != record.directory {
                return Err(Error::FileType);
            }
            seen[index] = true;
            Ok(true)
        }
    }
}
fn complete(
    expected: &[Expected],
    directory: usize,
    seen: &[bool; MAX_OBJECTS],
) -> Result<(), Error> {
    if expected
        .iter()
        .enumerate()
        .any(|(index, child)| child.parent == Some(directory) && !seen[index])
    {
        return Err(Error::Missing);
    }
    Ok(())
}

// FILE_ID_FULL_DIR_INFORMATION is parsed from checked little-endian bytes,
// without casting untrusted record storage to a native struct. FileIndex at4
// is not NTFS identity: only the 64-bit FileId at72 is used.
fn decode(bytes: &[u8]) -> Result<Record, Error> {
    if bytes.len() < NAME_OFFSET + 2 || bytes.len() > BUFFER_BYTES {
        return Err(Error::Framing);
    }
    let u32_at = |offset: usize| -> Result<u32, Error> {
        Ok(u32::from_le_bytes(
            bytes
                .get(offset..offset + 4)
                .ok_or(Error::Framing)?
                .try_into()
                .map_err(|_| Error::Framing)?,
        ))
    };
    if u32_at(0)? != 0 {
        return Err(Error::Framing);
    }
    let attributes = u32_at(56)?;
    if attributes & 0x400 != 0 {
        return Err(Error::Reparse);
    }
    let length = usize::try_from(u32_at(60)?).map_err(|_| Error::Framing)?;
    if length == 0 || length % 2 != 0 || length > MAX_NATIVE_NAME_UNITS * 2 {
        return Err(Error::Framing);
    }
    let end = NAME_OFFSET.checked_add(length).ok_or(Error::Framing)?;
    let padded = end.checked_add(7).ok_or(Error::Framing)? & !7;
    if bytes.len() != end && bytes.len() != padded {
        return Err(Error::Framing);
    }
    let raw = bytes.get(NAME_OFFSET..end).ok_or(Error::Framing)?;
    let units = || {
        raw.as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
    };
    if char::decode_utf16(units()).any(|value| value.is_err()) {
        return Err(Error::Framing);
    }
    let count = length / 2;
    if count > MAX_NAME {
        return Err(Error::Name);
    }
    let mut name = [0_u8; MAX_NAME];
    for (index, unit) in units().enumerate() {
        name[index] = u8::try_from(unit).map_err(|_| Error::Name)?;
    }
    let text = std::str::from_utf8(&name[..count]).map_err(|_| Error::Name)?;
    let name = match text {
        "." => RecordName::Dot,
        ".." => RecordName::DotDot,
        _ => RecordName::Child(Name::parse(text, MAX_NAME).map_err(|_| Error::Name)?),
    };
    let file_id = u64::from_le_bytes(
        bytes
            .get(72..80)
            .ok_or(Error::Framing)?
            .try_into()
            .map_err(|_| Error::Framing)?,
    );
    if file_id == 0 {
        return Err(Error::Identity);
    }
    Ok(Record {
        name,
        file_id,
        directory: attributes & 0x10 != 0,
    })
}

fn completion(status: u32, io_status: u32, information: usize) -> Result<Option<usize>, Error> {
    if status == 0x8000_0006 {
        if io_status == status && information == 0 {
            return Ok(None);
        }
    } else if status == 0 && io_status == 0 && information > 0 && information <= BUFFER_BYTES {
        return Ok(Some(information));
    }
    Err(Error::Query)
}
