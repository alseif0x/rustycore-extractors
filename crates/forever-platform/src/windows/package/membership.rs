// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-13: scalable bounded complete observed package membership.
use super::*;
#[allow(unsafe_code)]
mod ffi;
const BUFFER_BYTES: usize = 1024;
const NAME_OFFSET: usize = 80;

/// The append owner holds its data handle with SHARE0, so it cannot be reopened
/// by the normal package observer. Check this fixed two-file namespace against
/// the retained data/lock identities with bounded single-entry enumeration.
pub(super) fn archive_membership(root: &Root, archive: Identity) -> Result<(), Error> {
    let mut buffer = ffi::Buffer::new();
    let mut restart = true;
    let mut seen = [false; 4];
    for _ in 0..5 {
        root.spend(1)?;
        let Some(length) = ffi::query(&root.root.file, &mut buffer, restart)? else {
            return if seen[2] && seen[3] {
                Ok(())
            } else {
                Err(Error::Membership)
            };
        };
        restart = false;
        let record = decode(&buffer.bytes()[..length])?;
        let (index, expected, directory) = match record.name.as_str() {
            "." => (0, root.root.identity, true),
            ".." => (
                1,
                root.ancestors.last().ok_or(Error::Identity)?.identity,
                true,
            ),
            LOCK_NAME => (2, root.lock_identity.get().ok_or(Error::Identity)?, false),
            append_support::ARCHIVE_NAME => (3, archive, false),
            _ => return Err(Error::Membership),
        };
        if seen[index] || record.file_id != expected.file_index || record.directory != directory {
            return Err(Error::Membership);
        }
        seen[index] = true;
    }
    Err(Error::Membership)
}

pub(super) fn observe(root: &Root, reserved: usize) -> Result<Vec<Entry>, Error> {
    let before = super::super::ffi::identity(&root.root.file).map_err(|_| Error::Identity)?;
    let mut entries = Vec::new();
    if reserved
        .checked_add(transient(root))
        .is_none_or(|v| v > root.limits.max_metadata_bytes)
    {
        return Err(Error::Limit);
    }
    let mut metadata = reserved;
    let mut directories = 0_usize;
    let mut total = 0_u64;
    scan(
        root,
        "",
        &mut entries,
        &mut metadata,
        &mut directories,
        &mut total,
    )?;
    let mut cursor = 0;
    while cursor < entries.len() {
        if entries[cursor].kind == Kind::Directory {
            // Keep only one path copy beyond the charged result, at most512B.
            let path = owned(&entries[cursor].path)?;
            scan(
                root,
                &path,
                &mut entries,
                &mut metadata,
                &mut directories,
                &mut total,
            )?;
        }
        cursor += 1;
    }
    entries.sort_unstable_by(|a, b| a.path.cmp(&b.path));
    // No O(N) retained handles. Reopen every observed binding after the scan;
    // directory full metadata spans the whole observed enumeration interval.
    for entry in &entries {
        let (parent, name) = split(&entry.path);
        let chain = root.chain(parent)?;
        let directory = entry.kind == Kind::Directory;
        let mode = if directory {
            super::ffi::Mode::Directory
        } else if entry.path == LOCK_NAME {
            super::ffi::Mode::Lock
        } else {
            super::ffi::Mode::Read
        };
        let file = super::ffi::open(chain.file(root), name, mode)?;
        let current = root.capture_private(&file, name, directory)?;
        if if directory {
            super::super::ffi::identity(&file).map_err(|_| Error::Identity)? != entry.identity
        } else {
            current != entry.identity
        } {
            return Err(Error::Changed);
        }
        root.check_chain(&chain)?;
    }
    if super::super::ffi::identity(&root.root.file).map_err(|_| Error::Identity)? != before {
        return Err(Error::Changed);
    }
    root.entries.set(entries.len());
    root.directories.set(directories);
    root.bytes.set(total);
    root.counted.set(true);
    Ok(entries)
}
fn scan(
    root: &Root,
    path: &str,
    entries: &mut Vec<Entry>,
    metadata: &mut usize,
    directories: &mut usize,
    total: &mut u64,
) -> Result<(), Error> {
    let chain = root.chain(path)?;
    let directory = chain.file(root);
    let identity = super::super::ffi::identity(directory).map_err(|_| Error::Identity)?;
    let parent_identity = if chain.held.len() > 1 {
        chain.held[chain.held.len() - 2].identity
    } else if chain.held.len() == 1 {
        root.root.identity
    } else {
        root.ancestors.last().ok_or(Error::Identity)?.identity
    };
    let mut buffer = ffi::Buffer::new();
    let mut restart = true;
    let mut dots = [false; 2];
    let first = entries.len();
    loop {
        root.spend(1)?;
        let Some(length) = ffi::query(directory, &mut buffer, restart)? else {
            break;
        };
        restart = false;
        let record = decode(&buffer.bytes()[..length])?;
        if record.name == "." || record.name == ".." {
            let index = usize::from(record.name == "..");
            if dots[index]
                || !record.directory
                || record.file_id
                    != if index == 0 {
                        identity.file_index
                    } else {
                        parent_identity.file_index
                    }
            {
                return Err(Error::Membership);
            }
            dots[index] = true;
            continue;
        }
        validate_name(&record.name)?;
        let length = path.len() + usize::from(!path.is_empty()) + record.name.len();
        if length > root.limits.max_path_bytes
            || (if path.is_empty() {
                0
            } else {
                path.split('/').count()
            }) + 1
                > root.limits.max_depth
        {
            return Err(Error::Limit);
        }
        if entries.len() >= root.limits.max_entries {
            return Err(Error::Limit);
        }
        if record.directory && *directories >= root.limits.max_directories {
            return Err(Error::Limit);
        }
        // Every directory's entries are accumulated consecutively. Sorting the
        // segment after enumeration detects case aliases in O(n log n).
        let mode = if record.directory {
            super::ffi::Mode::Directory
        } else if path.is_empty() && record.name == LOCK_NAME {
            super::ffi::Mode::Lock
        } else {
            super::ffi::Mode::Read
        };
        let file = super::ffi::open(directory, &record.name, mode)?;
        let mut current = root.capture_private(&file, &record.name, record.directory)?;
        if current.file_index != record.file_id {
            return Err(Error::Identity);
        }
        if record.directory {
            current = super::super::ffi::identity(&file).map_err(|_| Error::Identity)?;
            *directories += 1;
        } else {
            *total = total.checked_add(current.size).ok_or(Error::Limit)?;
            if *total > root.limits.max_total_bytes {
                return Err(Error::Limit);
            }
        }
        // Admit retained capacities before allocation. One transient copied
        // path, the scan buffer and path leases are charged separately below.
        if entries.len() == entries.capacity() {
            let target = entries
                .capacity()
                .max(16)
                .checked_mul(2)
                .ok_or(Error::Limit)?
                .min(root.limits.max_entries);
            let extra = (target - entries.capacity())
                .checked_mul(size_of::<Entry>())
                .ok_or(Error::Limit)?;
            if metadata
                .checked_add(extra)
                .and_then(|v| v.checked_add(length + transient(root)))
                .is_none_or(|v| v > root.limits.max_metadata_bytes)
            {
                return Err(Error::Limit);
            }
            let old = entries.capacity();
            entries
                .try_reserve_exact(target - entries.len())
                .map_err(|_| Error::Allocation)?;
            *metadata = metadata
                .checked_add((entries.capacity() - old) * size_of::<Entry>())
                .ok_or(Error::Limit)?;
        }
        if metadata
            .checked_add(length + transient(root))
            .is_none_or(|v| v > root.limits.max_metadata_bytes)
        {
            return Err(Error::Limit);
        }
        let mut full = String::new();
        full.try_reserve_exact(length)
            .map_err(|_| Error::Allocation)?;
        if !path.is_empty() {
            full.push_str(path);
            full.push('/');
        }
        full.push_str(&record.name);
        *metadata = metadata.checked_add(full.capacity()).ok_or(Error::Limit)?;
        if metadata
            .checked_add(transient(root))
            .is_none_or(|v| v > root.limits.max_metadata_bytes)
        {
            return Err(Error::Limit);
        }
        entries.push(Entry {
            path: full,
            identity: current,
            kind: if record.directory {
                Kind::Directory
            } else {
                Kind::File
            },
        });
    }
    entries[first..].sort_unstable_by(|a, b| fold_cmp(&a.path, &b.path));
    if entries[first..]
        .windows(2)
        .any(|w| w[0].path.eq_ignore_ascii_case(&w[1].path))
    {
        return Err(Error::Alias);
    }
    if super::super::ffi::identity(directory).map_err(|_| Error::Identity)? != identity {
        return Err(Error::Changed);
    }
    root.check_chain(&chain)
}
pub(super) fn transient(root: &Root) -> usize {
    // Retained anchor/path Vecs and names, SID/lock/root state, one copied path,
    // one native exact-name stack buffer and the enumeration buffer. Counters
    // are logical capacity admission, not an allocator or RSS measurement.
    root.retained_metadata_bytes().unwrap_or(usize::MAX / 2)
        + root.limits.max_depth * (size_of::<Held>() + 255)
        + MAX_PATH
        + BUFFER_BYTES
        + 65536
}
fn fold_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    a.bytes()
        .map(|v| v.to_ascii_lowercase())
        .cmp(b.bytes().map(|v| v.to_ascii_lowercase()))
}
#[allow(clippy::ptr_arg)]
pub(super) fn compare(
    root: &Root,
    observed: &Vec<Entry>,
    expected: &[Entry],
    reserved: usize,
) -> Result<(), Error> {
    if observed.len() != expected.len() {
        return Err(Error::Membership);
    }
    // Callers provide canonical sorted observations, or an inventory in any
    // order. A bounded index vector avoids copying all retained path strings.
    let retained = observed
        .capacity()
        .checked_mul(size_of::<Entry>())
        .and_then(|v| v.checked_add(metadata_bytes(observed).ok()?))
        .and_then(|v| v.checked_add(transient(root)))
        .and_then(|v| v.checked_add(reserved))
        .ok_or(Error::Limit)?;
    if expected
        .len()
        .checked_mul(size_of::<usize>())
        .and_then(|v| v.checked_add(retained))
        .is_none_or(|v| v > root.limits.max_metadata_bytes)
    {
        return Err(Error::Limit);
    }
    let mut order = Vec::new();
    order
        .try_reserve_exact(expected.len())
        .map_err(|_| Error::Allocation)?;
    if order
        .capacity()
        .checked_mul(size_of::<usize>())
        .and_then(|v| v.checked_add(retained))
        .is_none_or(|v| v > root.limits.max_metadata_bytes)
    {
        return Err(Error::Limit);
    }
    order.extend(0..expected.len());
    order.sort_unstable_by(|a, b| expected[*a].path.cmp(&expected[*b].path));
    for (current, index) in observed.iter().zip(order) {
        let e = &expected[index];
        if current.path != e.path
            || current.kind != e.kind
            || if e.kind == Kind::Directory {
                !current.identity.same_object(e.identity)
            } else {
                current.identity != e.identity
            }
        {
            return Err(Error::Membership);
        }
    }
    Ok(())
}
struct Record {
    name: String,
    file_id: u64,
    directory: bool,
}
fn decode(bytes: &[u8]) -> Result<Record, Error> {
    if bytes.len() < NAME_OFFSET + 2 || bytes.len() > BUFFER_BYTES {
        return Err(Error::Membership);
    }
    let u32_at = |offset| -> Result<u32, Error> {
        Ok(u32::from_le_bytes(
            bytes
                .get(offset..offset + 4)
                .ok_or(Error::Membership)?
                .try_into()
                .map_err(|_| Error::Membership)?,
        ))
    };
    if u32_at(0)? != 0 {
        return Err(Error::Membership);
    }
    let attributes = u32_at(56)?;
    if attributes & 0x400 != 0 {
        return Err(Error::Reparse);
    }
    let length = u32_at(60)? as usize;
    let end = NAME_OFFSET.checked_add(length).ok_or(Error::Membership)?;
    if length == 0
        || !length.is_multiple_of(2)
        || length > 510
        || (bytes.len() != end && bytes.len() != ((end + 7) & !7))
    {
        return Err(Error::Membership);
    }
    let raw = bytes.get(NAME_OFFSET..end).ok_or(Error::Membership)?;
    let mut name = String::new();
    name.try_reserve_exact(length / 2)
        .map_err(|_| Error::Allocation)?;
    for pair in raw.as_chunks::<2>().0 {
        let value = u16::from_le_bytes(*pair);
        if !(0x20..=0x7e).contains(&value) {
            return Err(Error::Path);
        }
        name.push(value as u8 as char);
    }
    let file_id = u64::from_le_bytes(bytes[72..80].try_into().map_err(|_| Error::Membership)?);
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
    if status == 0x8000_0006 && io_status == status && information == 0 {
        Ok(None)
    } else if status == 0 && io_status == 0 && information > 0 && information <= BUFFER_BYTES {
        Ok(Some(information))
    } else {
        Err(Error::Membership)
    }
}
