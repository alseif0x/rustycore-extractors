// SPDX-License-Identifier: GPL-3.0-or-later
use std::path::{Path, PathBuf};
#[cfg(unix)]
use std::{
    fs::{self, DirBuilder, OpenOptions},
    io::Write,
    os::unix::fs::{DirBuilderExt, OpenOptionsExt},
};
#[cfg(unix)]
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
#[cfg(unix)]
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

// No Windows private-DACL/exclusive durable-publication backend is admitted.
// Refuse before any filesystem observation or mutation.
#[cfg(not(unix))]
pub(super) fn create(_client: &Path, _destination: &Path) -> Result<PathBuf, &'static str> {
    Err("output_platform")
}
#[cfg(not(unix))]
pub(super) fn publish(
    _directory: &Path,
    _name: &str,
    _suffix: &str,
    _bytes: &[u8],
) -> Result<(), &'static str> {
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
                "forever-batch-output-platform-{}-{}-{}",
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
        assert_eq!(create(&client, &absent), Err("output_platform"));
        assert_eq!(create(&client, &existing), Err("output_platform"));
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
            publish(&existing, "existing", "db2", b"replacement"),
            Err("output_platform")
        );
        assert_eq!(
            publish(&existing, "new", "db2", b"new"),
            Err("output_platform")
        );
        assert_eq!(
            publish(&absent, "new", "db2", b"new"),
            Err("output_platform")
        );
        assert!(!absent.exists());
        assert_eq!(
            fs::read(existing.join("existing.db2")).unwrap(),
            b"preserved"
        );
        assert_eq!(fs::read_dir(&existing).unwrap().count(), 1);
    }
}
