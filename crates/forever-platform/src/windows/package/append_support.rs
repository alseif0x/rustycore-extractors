// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-13: private bridge for the separate exclusive append profile.
// Existing package admission, limits and public file APIs are unchanged.
use super::*;
use crate::windows::{ReadDirectory, ReadFile};

pub(crate) const ARCHIVE_NAME: &str = "records.archive";

pub(crate) fn create_root(
    path: &Path,
    limits: Limits,
    directories: &[&ReadDirectory],
    files: &[&ReadFile],
    required_parent: Option<&ReadDirectory>,
) -> Result<Root, Error> {
    Root::open_kind_checked(path, limits, true, |ancestors| {
        ffi::volume_root(&ancestors.first().ok_or(Error::Path)?.file)?;
        input_roots(directories, files)?;
        if let Some(parent) = required_parent {
            private_directory(parent)?;
            if !ancestors
                .last()
                .ok_or(Error::Path)?
                .identity
                .same_object(parent.identity())
            {
                return Err(Error::Changed);
            }
        }
        // The new root cannot contain a preexisting input. Being inside an
        // input is excluded using these exact retained creation ancestors.
        for source in directories {
            source.verify_same_object().map_err(|_| Error::Changed)?;
            if ancestors
                .iter()
                .any(|held| held.identity.same_object(source.identity()))
            {
                return Err(Error::Path);
            }
        }
        for source in files {
            source.verify_unchanged().map_err(|_| Error::Changed)?;
        }
        for held in ancestors {
            check_external(&held.file, held.identity)?;
            refuse_git(&held.file)?;
        }
        Ok(())
    })
}

pub(crate) fn create(root: &mut Root) -> Result<(File, Identity), Error> {
    root.writable()?;
    // Root's configured ceiling excludes the one externally retained archive.
    root.handles(2)?;
    let file = ffi::create_archive(&root.root.file, ARCHIVE_NAME, &root.sid)?;
    let identity = capture(&file, false)?;
    check(root, &file, identity, 0)?;
    Ok((file, identity))
}

pub(crate) fn check(
    root: &Root,
    file: &File,
    original: Identity,
    length: u64,
) -> Result<(), Error> {
    root.guard()?;
    root.spend(1)?;
    let current = super::super::append::mutable_identity(file).map_err(|_| Error::Changed)?;
    admit(current, false, root.root.identity.volume)?;
    // Archive/normal can change on the first write. Other attributes, object,
    // creation time, link count and exact EOF remain fixed. Mutable write/change
    // timestamps are not stable until writer close on Windows.
    if current.volume != original.volume
        || current.file_index != original.file_index
        || current.creation_time != original.creation_time
        || (current.attributes ^ original.attributes) & !0xa0 != 0
        || current.size != length
        || file.metadata().map_err(|_| Error::Changed)?.len() != length
    {
        return Err(Error::Changed);
    }
    ffi::private(file, &root.sid)?;
    ffi::exact_name(file, ARCHIVE_NAME)?;
    membership::archive_membership(root, current)
}

pub(crate) fn flush(file: &File) -> Result<(), Error> {
    ffi::flush(file)
}

pub(crate) fn private_directory(directory: &ReadDirectory) -> Result<(), Error> {
    directory.verify_same_object().map_err(|_| Error::Changed)?;
    input_roots(&[directory], &[])?;
    let file = &directory.inner.leaf.file;
    let identity = capture(file, true)?;
    admit(identity, true, directory.identity().volume)?;
    super::super::ffi::ntfs_volume(file).map_err(|_| Error::Unsupported)?;
    ffi::ordinary_directory(file)?;
    ffi::private(file, &ffi::process_sid()?)?;
    directory.verify_same_object().map_err(|_| Error::Changed)
}

pub(crate) fn name(name: &str) -> Result<(), Error> {
    validate_name(name)
}

pub(crate) fn input_roots(
    directories: &[&ReadDirectory],
    files: &[&ReadFile],
) -> Result<(), Error> {
    for source in directories {
        ffi::volume_root(&source.inner.ancestors.first().ok_or(Error::Path)?.file)?;
    }
    for source in files {
        ffi::volume_root(&source.ancestors.first().ok_or(Error::Path)?.file)?;
    }
    Ok(())
}

/// Readonly exclusion using already retained input guards. Its independent
/// handle allowance includes all borrowed captures and transient candidate opens.
pub(crate) fn protect_output(
    output: &Path,
    directories: &[&ReadDirectory],
    files: &[&ReadFile],
    max_handles: usize,
) -> Result<(), Error> {
    if directories
        .len()
        .checked_add(files.len())
        .is_none_or(|n| n > 16)
    {
        return Err(Error::Limit);
    }
    let parsed = AbsolutePath::parse(output.to_str().ok_or(Error::Path)?, PathLimits::default())
        .map_err(|_| Error::Path)?;
    let supplied = directories
        .iter()
        .map(|v| v.retained_handles())
        .chain(files.iter().map(|v| v.retained_handles()))
        .try_fold(0_usize, |n, v| n.checked_add(v).ok_or(Error::Limit))?;
    // Two temporary handles: candidate final and one guard/.git probe.
    let available = max_handles.checked_sub(supplied).ok_or(Error::Limit)?;
    let (ancestors, _, name) = anchors(&parsed, available)?;
    ffi::volume_root(&ancestors.first().ok_or(Error::Path)?.file)?;
    input_roots(directories, files)?;
    let existing = match ffi::open(
        &ancestors.last().ok_or(Error::Path)?.file,
        &name,
        ffi::Mode::Directory,
    ) {
        Ok(file) => {
            let identity = capture(&file, true)?;
            check_external(&file, identity)?;
            ffi::exact_name(&file, &name)?;
            Some((file, identity))
        }
        Err(Error::Missing) => None,
        Err(e) => return Err(e),
    };
    for held in &ancestors {
        refuse_git(&held.file)?;
    }
    if let Some((file, _)) = &existing {
        refuse_git(file)?;
    }
    for source in directories {
        source.verify_same_object().map_err(|_| Error::Changed)?;
        let identity = source.identity();
        if ancestors.iter().any(|v| v.identity.same_object(identity))
            || existing
                .as_ref()
                .is_some_and(|(_, v)| v.same_object(identity))
            || existing.as_ref().is_some_and(|(_, root)| {
                source
                    .inner
                    .ancestors
                    .iter()
                    .any(|v| v.identity.same_object(*root))
            })
        {
            return Err(Error::Path);
        }
        source.verify_same_object().map_err(|_| Error::Changed)?;
    }
    for source in files {
        source.verify_unchanged().map_err(|_| Error::Changed)?;
        if existing.as_ref().is_some_and(|(_, root)| {
            source
                .ancestors
                .iter()
                .any(|v| v.identity.same_object(*root))
                || source.identity().same_object(*root)
        }) {
            return Err(Error::Path);
        }
        source.verify_unchanged().map_err(|_| Error::Changed)?;
    }
    for held in &ancestors {
        check_external(&held.file, held.identity)?;
    }
    Ok(())
}

#[cfg(test)]
pub(crate) fn test_dacl(file: &File, mask: u32) -> Result<(), u32> {
    ffi::test_dacl(file, &ffi::process_sid().map_err(|_| u32::MAX)?, mask)
}
