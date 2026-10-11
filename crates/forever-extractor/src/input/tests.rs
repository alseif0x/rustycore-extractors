// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: explicit nonsecret key-file admission tests only.
use super::*;
use std::{
    fs,
    sync::atomic::{AtomicU64, Ordering},
};
const FIXTURE: &[u8] =
    b"forever-tact-keys\t1\n0000000000000000\t00000000000000000000000000000000\n";
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "rce-product-synthetic-keys-{}-{}",
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
fn code(path: &Path) -> &'static str {
    keys(path).err().expect("synthetic rejection").code
}
#[test]
fn regular_explicit_key_input_has_owned_material_and_canonical_protected_source() {
    let temp = Temp::new();
    fs::write(temp.file(), FIXTURE).unwrap();
    let supplied = keys(&temp.file()).unwrap();
    assert_eq!(supplied.source, fs::canonicalize(temp.file()).unwrap());
    assert_eq!(supplied.into_keyring().len(), 1);
    assert_eq!(fs::read(temp.file()).unwrap(), FIXTURE);
}
#[test]
fn malformed_key_bytes_and_filename_are_not_disclosed_in_errors() {
    let temp = Temp::new();
    let path = temp.0.join("private_synthetic_filename");
    fs::write(&path, b"authored nonsecret malformed marker").unwrap();
    let error = keys(&path).err().unwrap();
    assert_eq!(error.code, "tact_keys_format");
    assert_eq!(error.to_string(), "tact_keys_format");
    assert!(!format!("{error:?}").contains("private_synthetic"));
}
#[test]
fn empty_truncated_and_oversized_key_files_fail_without_file_mutation() {
    let temp = Temp::new();
    for data in [b"".as_slice(), &FIXTURE[..FIXTURE.len() - 1]] {
        fs::write(temp.file(), data).unwrap();
        assert_eq!(code(&temp.file()), "tact_keys_format");
    }
    let file = File::create(temp.file()).unwrap();
    file.set_len(forever_casc::keys::MAX_BYTES as u64 + 1)
        .unwrap();
    assert_eq!(code(&temp.file()), "product_keys_input_limit");
    assert_eq!(
        file.metadata().unwrap().len(),
        forever_casc::keys::MAX_BYTES as u64 + 1
    );
}
#[test]
fn directories_missing_files_and_final_symlinks_reject() {
    let temp = Temp::new();
    assert_eq!(code(&temp.0), "product_keys_input_type");
    assert_eq!(code(&temp.file()), "product_keys_input_io");
    #[cfg(target_os = "linux")]
    {
        fs::write(temp.file(), FIXTURE).unwrap();
        let link = temp.0.join("alias");
        std::os::unix::fs::symlink(temp.file(), &link).unwrap();
        assert_eq!(code(&link), "product_keys_input_io");
    }
}
#[test]
fn key_input_identity_rejects_a_different_file_even_with_identical_bytes() {
    let temp = Temp::new();
    let a = temp.file();
    let b = temp.0.join("second.keys");
    fs::write(&a, FIXTURE).unwrap();
    fs::write(&b, FIXTURE).unwrap();
    let before = fs::metadata(&a).unwrap();
    assert!(same_input(&before, &fs::metadata(&a).unwrap()));
    #[cfg(unix)]
    assert!(!same_input(&before, &fs::metadata(&b).unwrap()));
    assert!(!same_input(&before, &fs::metadata(&temp.0).unwrap()));
}
