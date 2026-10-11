// SPDX-License-Identifier: GPL-3.0-or-later
//! Private new-directory publication. No merge, cleanup, resume or overwrite.
#[cfg(unix)]
use super::manifest::MARKER;
#[cfg(unix)]
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
#[cfg(unix)]
use std::{
    fs::{self, DirBuilder, File, OpenOptions},
    io::Write,
    path::Component,
};

#[cfg(unix)]
pub(super) fn create(
    client: &Path,
    destination: &Path,
    locale: crate::Locale,
) -> Result<PathBuf, &'static str> {
    if destination
        .components()
        .any(|part| matches!(part, Component::ParentDir | Component::CurDir))
    {
        return Err("unsafe_output_path");
    }
    let client = fs::canonicalize(client).map_err(|_| "input_io")?;
    if !client.is_dir() {
        return Err("input_io");
    }
    let parent = destination
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .ok_or("unsafe_output_path")?;
    let parent = fs::canonicalize(parent).map_err(|_| "unsafe_output_path")?;
    let leaf = destination.file_name().ok_or("unsafe_output_path")?;
    let destination = parent.join(leaf);
    let data = fs::canonicalize(client.join("Data")).ok();
    if parent.starts_with(&client)
        || client.starts_with(&destination)
        || data.is_some_and(|data| parent.starts_with(&data) || data.starts_with(&destination))
        || parent.ancestors().any(|path| path.join(".git").exists())
    {
        return Err("unsafe_output_path");
    }
    make_directory(&destination).map_err(|_| "output_exists_or_unavailable")?;
    sync_directory(&parent)?;
    publish(
        &destination,
        MARKER,
        b"{\"status\":\"incomplete\",\"scope\":\"complete_camera_files\"}\n",
    )?;
    make_directory(&destination.join("cameras")).map_err(|_| "output_write_failed")?;
    make_directory(&destination.join("dbc")).map_err(|_| "output_write_failed")?;
    make_directory(&destination.join("dbc").join(locale.name()))
        .map_err(|_| "output_write_failed")?;
    sync_directory(&destination.join("dbc"))?;
    sync_directory(&destination)?;
    Ok(destination)
}

#[cfg(unix)]
fn make_directory(path: &Path) -> std::io::Result<()> {
    let mut builder = DirBuilder::new();
    #[cfg(unix)]
    builder.mode(0o700);
    builder.create(path)
}

#[cfg(unix)]
pub(super) fn publish(directory: &Path, name: &str, bytes: &[u8]) -> Result<(), &'static str> {
    let pending = directory.join(format!("{name}.part"));
    let final_path = directory.join(name);
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = options.open(&pending).map_err(|_| "output_write_failed")?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|_| "output_write_failed")?;
    // Exclusive publication: unlike rename, a pre-existing name is never replaced.
    fs::hard_link(&pending, &final_path).map_err(|_| "output_write_failed")?;
    fs::remove_file(&pending).map_err(|_| "output_write_failed")?;
    sync_directory(directory)
}

#[cfg(unix)]
pub(super) fn complete(directory: &Path) -> Result<(), &'static str> {
    fs::remove_file(directory.join(MARKER)).map_err(|_| "output_write_failed")?;
    sync_directory(directory)
}

#[cfg(unix)]
fn sync_directory(path: &Path) -> Result<(), &'static str> {
    #[cfg(unix)]
    File::open(path)
        .and_then(|file| file.sync_all())
        .map_err(|_| "output_write_failed")?;
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

// No Windows private-DACL/exclusive durable-publication backend is admitted.
// Refuse before any filesystem observation or mutation.
#[cfg(not(unix))]
pub(super) fn create(
    _client: &Path,
    _destination: &Path,
    _locale: crate::Locale,
) -> Result<PathBuf, &'static str> {
    Err("output_platform")
}
#[cfg(not(unix))]
pub(super) fn publish(_directory: &Path, _name: &str, _bytes: &[u8]) -> Result<(), &'static str> {
    Err("output_platform")
}
#[cfg(not(unix))]
pub(super) fn complete(_directory: &Path) -> Result<(), &'static str> {
    Err("output_platform")
}

#[cfg(all(test, windows))]
mod windows_tests {
    use super::*;
    use std::{
        fs,
        sync::atomic::{AtomicU64, Ordering},
    };
    static NEXT: AtomicU64 = AtomicU64::new(0);
    struct Temp(PathBuf);
    impl Temp {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "forever-cameras-output-platform-{}-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    #[test]
    fn windows_create_refuses_before_client_or_output_io() {
        let tmp = Temp::new();
        let client = tmp.0.join("nonexistent-client");
        let absent = tmp.0.join("absent");
        let existing = tmp.0.join("existing");
        fs::create_dir(&existing).unwrap();
        fs::write(existing.join("sentinel"), b"preserved").unwrap();
        assert_eq!(
            create(&client, &absent, crate::Locale::EnUs),
            Err("output_platform")
        );
        assert_eq!(
            create(&client, &existing, crate::Locale::EnUs),
            Err("output_platform")
        );
        assert!(!absent.exists());
        assert!(!client.exists());
        assert_eq!(fs::read(existing.join("sentinel")).unwrap(), b"preserved");
        assert_eq!(fs::read_dir(&existing).unwrap().count(), 1);
    }
    #[test]
    fn windows_publish_refuses_without_pending_files_or_replacement() {
        let tmp = Temp::new();
        let existing = tmp.0.join("existing");
        let absent = tmp.0.join("absent");
        fs::create_dir(&existing).unwrap();
        fs::write(existing.join("existing.db2"), b"preserved").unwrap();
        assert_eq!(
            publish(&existing, "existing.db2", b"replacement"),
            Err("output_platform")
        );
        assert_eq!(
            publish(&existing, "new.db2", b"new"),
            Err("output_platform")
        );
        assert_eq!(publish(&absent, "new.db2", b"new"), Err("output_platform"));
        assert!(!absent.exists());
        assert_eq!(
            fs::read(existing.join("existing.db2")).unwrap(),
            b"preserved"
        );
        assert_eq!(fs::read_dir(&existing).unwrap().count(), 1);
    }
    #[test]
    fn windows_complete_refuses_without_removing_marker() {
        let tmp = Temp::new();
        let marker = super::super::manifest::MARKER;
        fs::write(tmp.0.join(marker), b"incomplete-preserved").unwrap();
        assert_eq!(complete(&tmp.0), Err("output_platform"));
        assert_eq!(complete(&tmp.0.join("absent")), Err("output_platform"));
        assert_eq!(
            fs::read(tmp.0.join(marker)).unwrap(),
            b"incomplete-preserved"
        );
        assert_eq!(fs::read_dir(&tmp.0).unwrap().count(), 1);
    }
}
