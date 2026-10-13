// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: authored, nonsecret Windows product-input fixtures.
use super::{bytes, keys};
use std::{
    fs::{self, File, OpenOptions},
    io::{Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

const FIXTURE: &[u8] =
    b"forever-tact-keys\t1\n0000000000000000\t00000000000000000000000000000000\n";
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "rce-product-windows-synthetic-input-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        assert!(!path.ancestors().any(|p| p.join(".git").exists()));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn file(&self) -> PathBuf {
        self.0.join("explicit.keys")
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
fn key_code(path: &Path) -> &'static str {
    keys(path).err().expect("synthetic rejection").code
}

#[test]
fn complete_explicit_keys_retain_verified_source_lease() {
    let temp = Temp::new();
    fs::write(temp.file(), FIXTURE).unwrap();
    let supplied = keys(&temp.file()).unwrap();
    assert_eq!(supplied.keyring.len(), 1);
    supplied._source_lease.verify_unchanged().unwrap();
    assert_eq!(supplied._source_lease.identity().size, FIXTURE.len() as u64);
    assert_eq!(fs::read(temp.file()).unwrap(), FIXTURE);
}

#[test]
fn held_key_lease_denies_write_delete_and_ancestor_rename() {
    let temp = Temp::new();
    fs::write(temp.file(), FIXTURE).unwrap();
    let supplied = keys(&temp.file()).unwrap();
    assert!(OpenOptions::new().write(true).open(temp.file()).is_err());
    assert!(fs::remove_file(temp.file()).is_err());
    assert!(fs::rename(&temp.0, temp.0.with_extension("renamed")).is_err());
    supplied._source_lease.verify_unchanged().unwrap();
    drop(supplied);
    fs::write(temp.file(), FIXTURE).unwrap();
}

#[test]
fn consuming_keys_releases_source_lease_and_returns_opaque_keyring() {
    let temp = Temp::new();
    fs::write(temp.file(), FIXTURE).unwrap();
    let keyring = keys(&temp.file()).unwrap().into_keyring();
    assert_eq!(keyring.len(), 1);
    fs::write(temp.file(), FIXTURE).unwrap();
}

#[test]
fn product_key_parts_keep_the_original_lease_after_keyring_transfer() {
    let temp = Temp::new();
    fs::write(temp.file(), FIXTURE).unwrap();
    let (keyring, source_path, lease) = keys(&temp.file()).unwrap().into_windows_parts();
    assert_eq!(source_path, temp.file());
    assert_eq!(keyring.len(), 1);
    drop(keyring);
    assert!(OpenOptions::new().write(true).open(temp.file()).is_err());
    assert!(fs::remove_file(temp.file()).is_err());
    lease.verify_unchanged().unwrap();
    drop(lease);
    fs::write(temp.file(), FIXTURE).unwrap();
}

#[test]
fn existing_writable_handle_rejects_both_input_adapters() {
    let temp = Temp::new();
    fs::write(temp.file(), FIXTURE).unwrap();
    let writer = OpenOptions::new().write(true).open(temp.file()).unwrap();
    assert_eq!(key_code(&temp.file()), "product_keys_input_io");
    assert_eq!(bytes(&temp.file(), 1024).unwrap_err(), "product_input_io");
    drop(writer);
    assert!(keys(&temp.file()).is_ok());
}

#[test]
fn complete_plan_bytes_and_exact_zero_limits() {
    let temp = Temp::new();
    let plan = b"{\"contract\":\"authored-synthetic-plan\"}\n";
    fs::write(temp.file(), plan).unwrap();
    assert_eq!(bytes(&temp.file(), plan.len() as u64).unwrap(), plan);
    assert_eq!(
        bytes(&temp.file(), plan.len() as u64 - 1).unwrap_err(),
        "product_input_limit"
    );
    fs::write(temp.file(), []).unwrap();
    assert_eq!(bytes(&temp.file(), 0).unwrap(), []);
    assert_eq!(key_code(&temp.file()), "tact_keys_format");
}

#[test]
fn chunked_inputs_preserve_limit_above_single_platform_read() {
    let temp = Temp::new();
    let boundary = forever_platform::path::MAX_READ_BYTES as u64;
    let mut file = File::create(temp.file()).unwrap();
    file.set_len(boundary + 2).unwrap();
    for (offset, marker) in [
        (0, 7),
        (boundary - 1, 11),
        (boundary, 13),
        (boundary + 1, 17),
    ] {
        file.seek(SeekFrom::Start(offset)).unwrap();
        file.write_all(&[marker]).unwrap();
    }
    drop(file);
    assert_eq!(
        bytes(&temp.file(), boundary + 1).unwrap_err(),
        "product_input_limit"
    );
    let data = bytes(&temp.file(), boundary + 2).unwrap();
    assert_eq!(data.len() as u64, boundary + 2);
    assert_eq!(data[0], 7);
    assert_eq!(&data[boundary as usize - 1..], &[11, 13, 17]);
    assert!(data[1..boundary as usize - 1].iter().all(|&byte| byte == 0));
}

#[test]
fn malformed_and_oversized_keys_have_static_redacted_errors() {
    let temp = Temp::new();
    let path = temp.0.join("private-authored-filename");
    fs::write(&path, b"authored nonsecret malformed marker").unwrap();
    let error = keys(&path).err().unwrap();
    assert_eq!(error.to_string(), "tact_keys_format");
    assert!(!format!("{error:?}").contains("private-authored"));
    let file = File::create(&path).unwrap();
    file.set_len(forever_casc::keys::MAX_BYTES as u64 + 1)
        .unwrap();
    drop(file);
    assert_eq!(key_code(&path), "product_keys_input_limit");
}

#[test]
fn directory_missing_and_hardlinked_inputs_reject() {
    let temp = Temp::new();
    assert_eq!(key_code(&temp.0), "product_keys_input_type");
    assert_eq!(key_code(&temp.file()), "product_keys_input_io");
    assert_eq!(bytes(&temp.0, 1024).unwrap_err(), "product_input_type");
    fs::write(temp.file(), FIXTURE).unwrap();
    fs::hard_link(temp.file(), temp.0.join("second-name")).unwrap();
    assert_eq!(key_code(&temp.file()), "product_keys_input_io");
    assert_eq!(bytes(&temp.file(), 1024).unwrap_err(), "product_input_io");
}

#[test]
fn normal_absolute_drive_path_is_required_without_prefix_stripping() {
    for path in [
        "relative.keys",
        "C:relative.keys",
        "\\\\?\\C:\\explicit.keys",
        "\\\\server\\share\\explicit.keys",
        "C:\\directory\\..\\explicit.keys",
        "C:\\explicit.keys:stream",
    ] {
        assert_eq!(key_code(Path::new(path)), "product_keys_input_io");
        assert_eq!(
            bytes(Path::new(path), 1024).unwrap_err(),
            "product_input_io"
        );
    }
}

#[test]
#[ignore = "requires actual Windows NTFS symlink privilege; Wine is not reparse acceptance"]
fn actual_windows_file_and_ancestor_reparse_points_reject() {
    let temp = Temp::new();
    fs::write(temp.file(), FIXTURE).unwrap();
    let file_link = temp.0.join("file-link");
    std::os::windows::fs::symlink_file(temp.file(), &file_link).unwrap();
    assert_eq!(key_code(&file_link), "product_keys_input_io");
    let dir_link = temp.0.with_extension("directory-link");
    std::os::windows::fs::symlink_dir(&temp.0, &dir_link).unwrap();
    assert_eq!(
        key_code(&dir_link.join("explicit.keys")),
        "product_keys_input_io"
    );
    fs::remove_dir(&dir_link).unwrap();
}
