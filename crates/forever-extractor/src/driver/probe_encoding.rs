// SPDX-License-Identifier: GPL-3.0-or-later
//! Explicit Linux-only redacted ROOT/ENCODING route probe. It opens only the
//! admitted keyless LocalStore, creates no package and writes one trace file.

use crate::{Error, plan::Plan};
use forever_casc::{
    integrity,
    local_store::{LocalStore, OpenOptions},
};
use std::{
    fs,
    io::Write,
    os::unix::fs::OpenOptionsExt,
    path::{Component as PathComponent, Path, PathBuf},
};

pub(crate) const MAX_REQUESTED_IDS: usize = forever_casc::local_store::probe::MAX_FILE_DATA_IDS;

#[derive(Clone, Copy)]
pub(crate) struct Summary {
    pub file_data_ids: usize,
    pub trace_sha256: [u8; 32],
}

/// Reject path aliases and Git checkouts before opening the admitted client.
/// This mirrors the product diagnostic's external-file and client-root guards.
pub(super) fn output_path(client: &Path, output: &Path) -> Result<PathBuf, Error> {
    let raw = output.as_os_str().as_encoded_bytes();
    if !output.is_absolute()
        || raw.len() > 4096
        || output.components().count() > 32
        || raw
            .split(|byte| *byte == b'/')
            .any(|part| matches!(part, b"." | b".." | b".git"))
    {
        return Err("product_diagnostic_path".into());
    }
    let leaf = output
        .file_name()
        .ok_or_else(|| Error::from("product_diagnostic_path"))?;
    let parent = output
        .parent()
        .ok_or_else(|| Error::from("product_diagnostic_path"))?;
    let client = checked_directory(client, false)?;
    let parent = checked_directory(parent, true)?;
    let output = parent.join(leaf);
    if output.starts_with(&client) {
        return Err("product_diagnostic_path".into());
    }
    match fs::symlink_metadata(&output) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(output),
        _ => Err("product_diagnostic_path".into()),
    }
}

pub(super) fn open_keyless_store_70450(plan: &Plan, client: &Path) -> Result<LocalStore, Error> {
    if forever_casc::admit_db2_70450(client).map_err(|error| Error::from(error.code()))? != 70450 {
        return Err("product_build_mismatch".into());
    }
    LocalStore::open_db2_70450(
        client,
        OpenOptions {
            locale: plan.locale,
            index_layouts: [plan.index_layout; 16],
            root_layout: plan.root_layout,
            limits: Default::default(),
        },
    )
    .map_err(|error| Error::from(error.code()))
}

fn checked_directory(path: &Path, reject_git: bool) -> Result<PathBuf, Error> {
    if !path.is_absolute()
        || path.as_os_str().as_encoded_bytes().len() > 4096
        || path.components().count() > 32
        || path
            .as_os_str()
            .as_encoded_bytes()
            .split(|byte| *byte == b'/')
            .any(|part| matches!(part, b"." | b".." | b".git"))
    {
        return Err("product_diagnostic_path".into());
    }
    let mut current = PathBuf::from("/");
    for component in path.components() {
        match component {
            PathComponent::RootDir => {
                if reject_git {
                    match fs::symlink_metadata(current.join(".git")) {
                        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                        _ => return Err("product_diagnostic_path".into()),
                    }
                }
                continue;
            }
            PathComponent::Normal(part) => current.push(part),
            _ => return Err("product_diagnostic_path".into()),
        }
        let metadata =
            fs::symlink_metadata(&current).map_err(|_| Error::from("product_diagnostic_path"))?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err("product_diagnostic_path".into());
        }
        if reject_git {
            match fs::symlink_metadata(current.join(".git")) {
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                _ => return Err("product_diagnostic_path".into()),
            }
        }
    }
    Ok(current)
}

pub(crate) fn run(
    plan: &Plan,
    client: &Path,
    output: &Path,
    file_data_ids: &[u32],
) -> Result<Summary, Error> {
    super::admit(plan)?;
    if plan.build != 70450
        || !(plan.is_v15()
            || plan.is_v16()
            || plan.is_v17()
            || plan.is_v18()
            || plan.is_v19()
            || plan.is_v20())
    {
        return Err("product_probe_profile".into());
    }
    if file_data_ids.is_empty()
        || file_data_ids.len() > MAX_REQUESTED_IDS
        || file_data_ids.windows(2).any(|pair| pair[0] >= pair[1])
    {
        return Err("product_probe_file_data_id".into());
    }
    let output = output_path(client, output)?;
    let store = open_keyless_store_70450(plan, client)?;
    let report = store
        .probe_file_data_ids(file_data_ids, Default::default())
        .map_err(|error| match error {
            forever_casc::local_store::probe::Error::KeysPresent => {
                Error::from("product_keys_forbidden")
            }
            _ => Error::from(error.code()),
        })?;
    let trace = report.render().map_err(|error| Error::from(error.code()))?;
    store
        .verify_unchanged()
        .map_err(|error| Error::from(error.code()))?;
    let output = output_path(client, &output)?;

    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&output)
        .map_err(|_| Error::from("product_probe_output"))?;
    if file.write_all(trace.as_bytes()).is_err() {
        drop(file);
        let _ = fs::remove_file(&output);
        return Err("product_probe_output".into());
    }
    file.sync_all()
        .map_err(|_| Error::from("product_probe_output"))?;
    Ok(Summary {
        file_data_ids: file_data_ids.len(),
        trace_sha256: integrity::digest(trace.as_bytes()),
    })
}

#[cfg(test)]
#[path = "probe_encoding/tests.rs"]
mod tests;
