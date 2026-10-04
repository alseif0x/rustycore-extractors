// SPDX-License-Identifier: GPL-3.0-or-later
use std::{
    fs::{self, DirBuilder, OpenOptions},
    io::Write,
    os::unix::fs::{DirBuilderExt, OpenOptionsExt},
    path::{Path, PathBuf},
};
pub(super) fn create(client: &Path, destination: &Path) -> Result<PathBuf, &'static str> {
    let client = fs::canonicalize(client).map_err(|_| "input_io")?;
    let parent = destination
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .ok_or("unsafe_output_path")?;
    let parent = fs::canonicalize(parent).map_err(|_| "unsafe_output_path")?;
    let leaf = destination.file_name().ok_or("unsafe_output_path")?;
    let destination = parent.join(leaf);
    // Canonicalize aliases (including a client's symlinked Data) before writes.
    let data = fs::canonicalize(client.join("Data")).ok();
    if parent.starts_with(&client)
        || client.starts_with(&destination)
        || data.is_some_and(|data| parent.starts_with(&data) || data.starts_with(&destination))
        || parent
            .ancestors()
            .any(|ancestor| ancestor.join(".git").exists())
    {
        return Err("unsafe_output_path");
    }
    // No merge/resume/overwrite. Existing directories and symlinks both reject.
    DirBuilder::new()
        .mode(0o700)
        .create(&destination)
        .map_err(|_| "output_exists_or_unavailable")?;
    Ok(destination)
}
pub(super) fn publish(
    directory: &Path,
    name: &str,
    suffix: &str,
    bytes: &[u8],
) -> Result<(), &'static str> {
    let path = directory.join(format!("{name}.{suffix}"));
    let pending = directory.join(format!("{name}.{suffix}.part"));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&pending)
        .map_err(|_| "output_write_failed")?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|_| "output_write_failed")?;
    // Link is exclusive unlike rename; failed output stays visibly .part.
    fs::hard_link(&pending, &path).map_err(|_| "output_write_failed")?;
    fs::remove_file(&pending).map_err(|_| "output_write_failed")?;
    Ok(())
}
