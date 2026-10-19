// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: bounded read-only local metadata/archive IO.

pub(super) mod framing;
mod route;
pub(super) use route::observe_span;

use super::{Budget, CandidateFailures, Error, Limits, add, array_bytes, owned, reserve};
use crate::{archive_record, blte, local_index};
use std::{
    fs,
    path::{Path, PathBuf},
};
#[cfg(not(windows))]
use std::{
    fs::{File, Metadata},
    io::{Read, Seek, SeekFrom},
    time::SystemTime,
};

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub(super) use windows::{Guard, whole};

#[cfg(windows)]
pub(super) const fn windows_lease_reservation() -> usize {
    // Maximum 98,280 UTF-8 bytes, 65,522 temporary UTF-16 bytes, 63
    // prefix/ancestor slots and enumeration state, with conservative slack.
    256 * 1024
}

pub(super) fn directory_guard(path: &Path) -> Result<Guard, Error> {
    #[cfg(windows)]
    {
        Guard::capture_directory(path)
    }
    #[cfg(not(windows))]
    {
        Guard::capture(path)
    }
}

#[cfg(not(windows))]
#[derive(PartialEq, Eq)]
struct Identity {
    len: u64,
    modified: Option<SystemTime>,
    #[cfg(unix)]
    dev: u64,
    #[cfg(unix)]
    ino: u64,
}
#[cfg(not(windows))]
impl Identity {
    fn from(meta: &Metadata) -> Self {
        #[cfg(unix)]
        use std::os::unix::fs::MetadataExt;
        Self {
            len: meta.len(),
            modified: meta.modified().ok(),
            #[cfg(unix)]
            dev: meta.dev(),
            #[cfg(unix)]
            ino: meta.ino(),
        }
    }
}

#[cfg(not(windows))]
pub(super) struct Guard {
    path: PathBuf,
    identity: Identity,
    expected: Option<Vec<u8>>,
}
#[cfg(not(windows))]
impl Guard {
    pub(super) fn capture(path: &Path) -> Result<Self, Error> {
        let identity = Identity::from(&fs::metadata(path).map_err(|_| Error::Input)?);
        let mut owned = PathBuf::new();
        owned
            .try_reserve_exact(path.as_os_str().len())
            .map_err(|_| Error::Limit)?;
        owned.push(path);
        Ok(Self {
            path: owned,
            identity,
            expected: None,
        })
    }
    pub(super) fn with_bytes(mut self, bytes: Vec<u8>) -> Self {
        self.expected = Some(bytes);
        self
    }
    pub(super) fn check(&self) -> Result<(), Error> {
        let current = fs::metadata(&self.path).map_err(|_| Error::InputChanged)?;
        if Identity::from(&current) != self.identity {
            return Err(Error::InputChanged);
        }
        if let Some(expected) = &self.expected {
            let mut file = File::open(&self.path).map_err(|_| Error::InputChanged)?;
            let mut chunk = [0_u8; 4096];
            for part in expected.chunks(4096) {
                file.read_exact(&mut chunk[..part.len()])
                    .map_err(|_| Error::InputChanged)?;
                if &chunk[..part.len()] != part {
                    return Err(Error::InputChanged);
                }
            }
            if file
                .read(&mut chunk[..1])
                .map_err(|_| Error::InputChanged)?
                != 0
                || Identity::from(&file.metadata().map_err(|_| Error::InputChanged)?)
                    != self.identity
            {
                return Err(Error::InputChanged);
            }
        }
        Ok(())
    }
}

pub(super) fn push_guard(
    guards: &mut Vec<Guard>,
    guard: Guard,
    budget: &mut Budget,
) -> Result<(), Error> {
    budget.retain(add(std::mem::size_of::<Guard>(), guard.path.capacity())?)?;
    reserve(guards, 1)?;
    guards.push(guard);
    Ok(())
}

pub(super) fn check_guards(guards: &[Guard]) -> Result<(), Error> {
    for guard in guards {
        guard.check()?;
    }
    Ok(())
}

pub(super) fn config_path(client: &Path, key: [u8; 16]) -> PathBuf {
    let hex = key
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    client
        .join("Data")
        .join("config")
        .join(&hex[..2])
        .join(&hex[2..4])
        .join(hex)
}

#[cfg(not(windows))]
pub(super) fn whole(path: &Path, max_bytes: u64, budget: &Budget) -> Result<Vec<u8>, Error> {
    let mut file = File::open(path).map_err(|_| Error::Input)?;
    let before = file.metadata().map_err(|_| Error::Input)?;
    if !before.is_file() {
        return Err(Error::Input);
    }
    let size = usize::try_from(before.len()).map_err(|_| Error::Limit)?;
    if before.len() > max_bytes {
        return Err(Error::Limit);
    }
    budget.phase(size)?;
    let mut bytes = Vec::new();
    reserve(&mut bytes, size)?;
    bytes.resize(size, 0);
    file.read_exact(&mut bytes)
        .map_err(|_| Error::InputChanged)?;
    let mut tail = [0];
    if file.read(&mut tail).map_err(|_| Error::Input)? != 0
        || Identity::from(&before)
            != Identity::from(&file.metadata().map_err(|_| Error::InputChanged)?)
    {
        return Err(Error::InputChanged);
    }
    Ok(bytes)
}

pub(super) fn load_indexes(
    dir: &Path,
    layouts: [local_index::Layout; 16],
    limits: Limits,
    budget: &mut Budget,
    guards: &mut Vec<Guard>,
) -> Result<Vec<local_index::Entry>, Error> {
    let mut names = Vec::new();
    let mut paths = Vec::new();
    let mut scanned = 0_usize;
    let mut retained = 0_usize;
    // One directory lease is held during enumeration, then dropped before any
    // index-file lease is acquired. The enumerator adds at most one handle.
    #[cfg(windows)]
    let directory = windows::directory(dir)?;
    for entry in fs::read_dir(dir).map_err(|_| Error::Input)? {
        scanned = add(scanned, 1)?;
        if scanned > limits.max_directory_entries {
            return Err(Error::Limit);
        }
        let entry = entry.map_err(|_| Error::Input)?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        if !name
            .get(name.len().saturating_sub(4)..)
            .is_some_and(|ext| ext.eq_ignore_ascii_case(".idx"))
        {
            continue;
        }
        if names.len() >= limits.max_index_files {
            return Err(Error::Limit);
        }
        let parsed = local_index::parse_filename(name).map_err(Error::Index)?;
        let elements = add(
            std::mem::size_of::<local_index::IndexFilename>(),
            std::mem::size_of::<PathBuf>(),
        )?;
        budget.retain(elements)?;
        let path = inventory_path(dir, name, budget)?;
        retained = add(retained, add(elements, path.capacity())?)?;
        reserve(&mut names, 1)?;
        reserve(&mut paths, 1)?;
        names.push(parsed);
        paths.push(path);
    }
    #[cfg(windows)]
    {
        directory.verify_unchanged().map_err(windows::map_error)?;
        drop(directory);
    }
    budget.phase(array_bytes::<local_index::IndexFilename>(names.len())?)?;
    let latest =
        local_index::select_latest(&names, limits.max_index_files).map_err(Error::Index)?;
    budget.phase(0)?;
    if latest.iter().any(Option::is_none) {
        return Err(Error::IndexSet);
    }
    let mut entries = Vec::new();
    for bucket in 0..16 {
        let selected = latest[bucket].ok_or(Error::IndexSet)?;
        let position = names
            .iter()
            .position(|name| *name == selected)
            .ok_or(Error::IndexSet)?;
        let path = &paths[position];
        let guard = Guard::capture(path)?;
        let bytes = whole(path, limits.index.max_bytes, budget)?;
        guard.check()?;
        let remaining = limits
            .index
            .max_entries
            .checked_sub(entries.len())
            .ok_or(Error::Limit)?;
        let bound = remaining.min(bytes.len() / 18);
        budget.phase(add(bytes.len(), array_bytes::<local_index::Entry>(bound)?)?)?;
        let parsed = local_index::parse(
            &bytes,
            bucket as u8,
            layouts[bucket],
            local_index::Limits {
                max_entries: remaining,
                ..limits.index
            },
        )
        .map_err(Error::Index)?;
        budget.retain(array_bytes::<local_index::Entry>(parsed.entries.len())?)?;
        reserve(&mut entries, parsed.entries.len())?;
        entries.extend(parsed.entries);
        drop(bytes);
        budget.phase(0)?;
        push_guard(guards, guard, budget)?;
    }
    drop(names);
    drop(paths);
    budget.release(retained);
    Ok(entries)
}

// Preserve original filename case while avoiding PathBuf's geometric growth.
// Charge the requested storage before allocation and reconcile any allocator
// excess with its actual capacity before admitting the path to the inventory.
fn inventory_path(dir: &Path, name: &str, budget: &mut Budget) -> Result<PathBuf, Error> {
    let requested = add(dir.as_os_str().len(), add(name.len(), 1)?)?;
    budget.retain(requested)?;
    let mut path = PathBuf::new();
    path.try_reserve_exact(requested)
        .map_err(|_| Error::Limit)?;
    path.push(dir);
    path.push(name);
    if path.capacity() > requested {
        budget.retain(path.capacity() - requested)?;
    } else {
        budget.release(requested - path.capacity());
    }
    Ok(path)
}

// Shared by acquisition and route diagnostics. CASC archive numbers are
// DECIMAL, minimum width three (10 -> data.010; 4095 -> data.4095).
// The bounded basename is built on the stack, then one fallibly reserved path
// is constructed; no independent formatting interpretation is permitted.
pub(super) fn archive_path(
    dir: &Path,
    index: u64,
    max_path_bytes: usize,
) -> Result<PathBuf, Error> {
    if index >= 0x1000 {
        return Err(Error::Input);
    }
    let mut basename = *b"data.0000";
    let digits = if index < 1000 { 3 } else { 4 };
    let mut number = index;
    for byte in basename[5..5 + digits].iter_mut().rev() {
        *byte = b'0' + (number % 10) as u8;
        number /= 10;
    }
    let name = std::str::from_utf8(&basename[..5 + digits]).map_err(|_| Error::Input)?;
    let separator = usize::from(
        !dir.as_os_str().is_empty()
            && !dir
                .as_os_str()
                .as_encoded_bytes()
                .last()
                .is_some_and(|&byte| std::path::is_separator(char::from(byte))),
    );
    let requested = add(dir.as_os_str().len(), add(name.len(), separator)?)?;
    if requested > max_path_bytes {
        return Err(Error::Limit);
    }
    let mut path = PathBuf::new();
    path.try_reserve_exact(requested)
        .map_err(|_| Error::Limit)?;
    path.push(dir);
    path.push(name);
    if path.capacity() > max_path_bytes {
        return Err(Error::Limit);
    }
    Ok(path)
}

#[derive(Clone, Copy)]
enum Rejection {
    Missing,
    Incomplete,
    Encrypted,
    Corrupt,
    Unsupported,
}
pub(super) enum ProbeRead {
    Decoded,
    Rejected {
        class: &'static str,
        blte_code: Option<&'static str>,
    },
}
struct BlteCodeCapture<'a> {
    enabled: bool,
    code: &'a mut Option<&'static str>,
}
impl BlteCodeCapture<'_> {
    fn record(&mut self, code: &'static str) {
        if self.enabled && self.code.is_none() {
            *self.code = Some(code);
        }
    }
}

fn record_failure(failures: &mut CandidateFailures, rejection: Rejection) -> Result<(), Error> {
    let counter = match rejection {
        Rejection::Missing => &mut failures.missing,
        Rejection::Incomplete => &mut failures.incomplete,
        Rejection::Encrypted => &mut failures.encrypted,
        Rejection::Corrupt => &mut failures.corrupt,
        Rejection::Unsupported => &mut failures.unsupported,
    };
    *counter = counter.checked_add(1).ok_or(Error::Limit)?;
    Ok(())
}

pub(super) struct Reader<'a> {
    pub(super) dir: &'a Path,
    pub(super) indexes: &'a [local_index::Entry],
    pub(super) order: &'a [u32],
    pub(super) limits: Limits,
    pub(super) budget: &'a Budget,
    pub(super) keys: Option<&'a crate::keys::Keyring>,
}

impl Reader<'_> {
    /// Probe one exact local encoding/index candidate without returning decoded
    /// bytes. The keyless caller receives only a rejection class and static BLTE
    /// code; keys, encoded bytes and plaintext stay inside this module.
    pub(super) fn probe_candidate(
        &self,
        entry: &local_index::Entry,
        ekey: [u8; 16],
        ckey: [u8; 16],
        expected_size: Option<u32>,
        max_decoded: u64,
    ) -> Result<ProbeRead, Error> {
        let Some(size) = expected_size else {
            return Ok(ProbeRead::Rejected {
                class: "unknown_decoded_size",
                blte_code: None,
            });
        };
        let mut blte_code = None;
        let mut capture = BlteCodeCapture {
            enabled: true,
            code: &mut blte_code,
        };
        match self.candidate(
            entry,
            ekey,
            ckey,
            u64::from(size),
            max_decoded,
            &mut capture,
        )? {
            Ok(bytes) => {
                drop(bytes);
                Ok(ProbeRead::Decoded)
            }
            Err(rejection) => Ok(ProbeRead::Rejected {
                class: match rejection {
                    Rejection::Missing => "missing",
                    Rejection::Incomplete => "incomplete",
                    Rejection::Encrypted => "encrypted",
                    Rejection::Corrupt => "corrupt",
                    Rejection::Unsupported => "unsupported",
                },
                blte_code,
            }),
        }
    }

    pub(super) fn read_reference(
        &self,
        ckey: [u8; 16],
        size: Option<u32>,
        ekeys: impl Iterator<Item = [u8; 16]>,
        max_decoded: u64,
        capture_blte_code: bool,
        blte_code: &mut Option<&'static str>,
    ) -> Result<(Vec<u8>, usize), Error> {
        let Self {
            indexes,
            order,
            limits,
            ..
        } = self;
        let size = u64::from(size.ok_or(Error::UnknownSize)?);
        if size > max_decoded {
            return Err(Error::Limit);
        }
        let mut failures = CandidateFailures::default();
        let mut attempts = 0_usize;
        let mut blte_capture = BlteCodeCapture {
            enabled: capture_blte_code,
            code: blte_code,
        };
        for ekey in ekeys {
            let range = owned::equal_range(order, |index| {
                indexes[index].key_prefix.as_slice().cmp(&ekey[..9])
            });
            if range.is_empty() {
                attempts = add(attempts, 1)?;
                if attempts > limits.max_candidate_attempts {
                    return Err(Error::Limit);
                }
                record_failure(&mut failures, Rejection::Missing)?;
                continue;
            }
            for &index in &order[range] {
                attempts = add(attempts, 1)?;
                if attempts > limits.max_candidate_attempts {
                    return Err(Error::Limit);
                }
                match self.candidate(
                    &indexes[index as usize],
                    ekey,
                    ckey,
                    size,
                    max_decoded,
                    &mut blte_capture,
                )? {
                    Ok(bytes) => return Ok((bytes, attempts)),
                    Err(rejection) => record_failure(&mut failures, rejection)?,
                }
            }
        }
        Err(Error::NoLocalEncoding(failures))
    }

    fn candidate(
        &self,
        entry: &local_index::Entry,
        ekey: [u8; 16],
        ckey: [u8; 16],
        size: u64,
        max_decoded: u64,
        blte_capture: &mut BlteCodeCapture<'_>,
    ) -> Result<Result<Vec<u8>, Rejection>, Error> {
        let Self {
            dir,
            limits,
            budget,
            ..
        } = self;
        // CascStructs.h declares CASC_MAX_DATA_FILES = 0x1000 for this local profile.
        if entry.archive_index >= 0x1000 {
            return Ok(Err(Rejection::Unsupported));
        }
        let path = archive_path(dir, entry.archive_index, limits.max_path_bytes)?;
        #[cfg(not(windows))]
        let mut file = match File::open(&path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Err(Rejection::Missing));
            }
            Err(_) => return Err(Error::Input),
        };
        #[cfg(not(windows))]
        let before = file.metadata().map_err(|_| Error::Input)?;
        #[cfg(not(windows))]
        if !before.is_file() {
            return Err(Error::Input);
        }
        #[cfg(not(windows))]
        let archive_len = before.len();
        #[cfg(windows)]
        let file = match windows::archive(&path) {
            Ok(file) => file,
            Err(forever_platform::Error::Missing) => return Ok(Err(Rejection::Missing)),
            Err(error) => return Err(windows::map_error(error)),
        };
        #[cfg(windows)]
        let archive_len = file.identity().size;
        let plan = match archive_record::plan(entry, archive_len, limits.max_archive_span_bytes) {
            Ok(plan) => plan,
            Err(archive_record::Error::Limit) => return Err(Error::Limit),
            Err(archive_record::Error::Bounds) => return Ok(Err(Rejection::Incomplete)),
            Err(_) => return Ok(Err(Rejection::Corrupt)),
        };
        let span = usize::try_from(plan.encoded_size()).map_err(|_| Error::Limit)?;
        // Opt-in decoding can hold one decrypted frame while the complete
        // encoded span and decoded output remain live. Reserve a whole extra
        // span conservatively, even when this particular object is N/Z.
        let encoded_scratch = if self.keys.is_some() {
            add(span, span)?
        } else {
            span
        };
        budget.phase(add(
            encoded_scratch,
            usize::try_from(size).map_err(|_| Error::Limit)?,
        )?)?;
        let mut encoded = Vec::new();
        reserve(&mut encoded, span)?;
        encoded.resize(span, 0);
        #[cfg(not(windows))]
        file.seek(SeekFrom::Start(plan.offset()))
            .map_err(|_| Error::Input)?;
        #[cfg(not(windows))]
        let read_result = file.read_exact(&mut encoded);
        #[cfg(not(windows))]
        let after = Identity::from(&file.metadata().map_err(|_| Error::InputChanged)?);
        #[cfg(not(windows))]
        if let Err(rejection) = classify_read(read_result, &Identity::from(&before), after)? {
            return Ok(Err(rejection));
        }
        #[cfg(windows)]
        windows::read_range(&file, plan.offset(), &mut encoded)?;
        let decode_limits = blte::Limits {
            max_encoded_bytes: limits.max_archive_span_bytes,
            max_decoded_bytes: max_decoded,
            max_frames: limits.max_blte_frames,
        };
        let result = match self.keys {
            Some(keys) => archive_record::decode_span_with_keys(
                &encoded,
                &plan,
                ekey,
                ckey,
                size,
                decode_limits,
                keys,
            ),
            None => archive_record::decode_span(&encoded, &plan, ekey, ckey, size, decode_limits),
        };
        #[cfg(not(windows))]
        if Identity::from(&before)
            != Identity::from(&file.metadata().map_err(|_| Error::InputChanged)?)
        {
            return Err(Error::InputChanged);
        }
        #[cfg(windows)]
        file.verify_unchanged().map_err(windows::map_error)?;
        Ok(match result {
            Ok(bytes) => Ok(bytes),
            Err(archive_record::Error::Blte(blte::Error::Limit)) => return Err(Error::Limit),
            Err(archive_record::Error::Blte(error)) => {
                blte_capture.record(error.code());
                match error {
                    blte::Error::EncryptedUnsupported | blte::Error::MissingKey => {
                        Err(Rejection::Encrypted)
                    }
                    blte::Error::RecursiveUnsupported
                    | blte::Error::UnknownMode
                    | blte::Error::EncryptionUnsupported => Err(Rejection::Unsupported),
                    _ => Err(Rejection::Corrupt),
                }
            }
            Err(_) => Err(Rejection::Corrupt),
        })
    }
}

// Observe input change even on EOF or another IO error, before deciding whether
// another encoding candidate may be tried. Pure classification permits race-free
// coverage of these otherwise timing-sensitive paths.
#[cfg(not(windows))]
fn classify_read(
    result: std::io::Result<()>,
    before: &Identity,
    after: Identity,
) -> Result<Result<(), Rejection>, Error> {
    if before != &after {
        return Err(Error::InputChanged);
    }
    match result {
        Ok(()) => Ok(Ok(())),
        Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => {
            Ok(Err(Rejection::Incomplete))
        }
        Err(_) => Err(Error::Input),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(not(windows))]
    fn identity(len: u64) -> Identity {
        Identity {
            len,
            modified: None,
            #[cfg(unix)]
            dev: 1,
            #[cfg(unix)]
            ino: 2,
        }
    }
    #[test]
    fn inventory_path_charges_actual_capacity_and_preserves_case() {
        let dir = PathBuf::from("a".repeat(3500));
        let mut budget = Budget::new(Limits::default());
        let path = inventory_path(&dir, "0A00000001.IDX", &mut budget).unwrap();
        assert_eq!(path.file_name().unwrap(), "0A00000001.IDX");
        assert_eq!(budget.resident, path.capacity());
        let mut budget = Budget::new(Limits {
            max_resident_bytes: 1,
            ..Limits::default()
        });
        assert!(matches!(
            inventory_path(&dir, "0A00000001.IDX", &mut budget),
            Err(Error::Limit)
        ));
        assert_eq!(budget.resident, 0);
    }

    #[test]
    #[cfg(not(windows))]
    fn changed_handle_wins_over_eof_and_other_io_errors() {
        for kind in [
            std::io::ErrorKind::UnexpectedEof,
            std::io::ErrorKind::PermissionDenied,
        ] {
            assert!(matches!(
                classify_read(Err(kind.into()), &identity(10), identity(9)),
                Err(Error::InputChanged)
            ));
        }
        assert!(matches!(
            classify_read(Ok(()), &identity(10), identity(9)),
            Err(Error::InputChanged)
        ));
    }
    #[test]
    #[cfg(not(windows))]
    fn stable_eof_remains_incomplete_and_stable_io_error_is_fatal() {
        assert!(matches!(
            classify_read(
                Err(std::io::ErrorKind::UnexpectedEof.into()),
                &identity(10),
                identity(10)
            ),
            Ok(Err(Rejection::Incomplete))
        ));
        assert!(matches!(
            classify_read(
                Err(std::io::ErrorKind::PermissionDenied.into()),
                &identity(10),
                identity(10)
            ),
            Err(Error::Input)
        ));
        assert!(matches!(
            classify_read(Ok(()), &identity(10), identity(10)),
            Ok(Ok(()))
        ));
    }
}
