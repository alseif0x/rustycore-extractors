// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-12: original Rust kernel lease and targeted-signal tests.
use super::*;
use std::{
    fs::{self, OpenOptions},
    os::{
        fd::AsRawFd,
        unix::fs::{DirBuilderExt, FileExt, OpenOptionsExt},
    },
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};
static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Fixture {
    dir: PathBuf,
    path: PathBuf,
    file: File,
}
impl Fixture {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!(
            "rce-linux-lease-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::DirBuilder::new().mode(0o700).create(&dir).unwrap();
        let path = dir.join("private");
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&path)
            .unwrap();
        Self { dir, path, file }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.dir).unwrap();
    }
}
#[test]
fn fresh_description_lease_own_io_and_caller_mask() {
    let before = ffi::caller_mask().unwrap();
    let f = Fixture::new();
    establish(&f.file).unwrap();
    verify(&f.file).unwrap();
    f.file.write_all_at(b"authored", 0).unwrap();
    let mut b = [0; 8];
    f.file.read_exact_at(&mut b, 0).unwrap();
    assert_eq!(&b, b"authored");
    verify(&f.file).unwrap();
    assert_eq!(ffi::caller_mask().unwrap(), before);
}
#[test]
fn existing_independent_read_and_write_descriptions_refuse() {
    for write in [false, true] {
        let f = Fixture::new();
        let extra = OpenOptions::new()
            .read(!write)
            .write(write)
            .open(&f.path)
            .unwrap();
        assert_eq!(establish(&f.file), Err(Error::Unavailable));
        drop(extra);
        establish(&f.file).unwrap();
    }
}
#[test]
fn foreign_nonblocking_open_exposes_pending_break_without_signal_recipient() {
    let before = ffi::caller_mask().unwrap();
    let f = Fixture::new();
    establish(&f.file).unwrap();
    assert!(
        OpenOptions::new()
            .read(true)
            .custom_flags(0x800)
            .open(&f.path)
            .is_err()
    );
    assert_eq!(verify(&f.file), Err(Error::Changed));
    assert_eq!(ffi::caller_mask().unwrap(), before);
}
#[test]
fn targeted_sigio_setup_races_reject_without_changing_callers_mask() {
    let before = ffi::caller_mask().unwrap();
    for write in [false, true] {
        for _ in 0..8 {
            let f = Fixture::new();
            let (ready, wait) = std::sync::mpsc::channel();
            let (done, release) = std::sync::mpsc::channel();
            std::thread::scope(|scope| {
                let file = &f.file;
                let handle = std::thread::Builder::new()
                    .stack_size(HELPER_STACK_BYTES)
                    .spawn_scoped(scope, move || {
                        ffi::establish_with_hook(file.as_raw_fd(), move || {
                            ready.send(()).unwrap();
                            release
                                .recv_timeout(std::time::Duration::from_secs(5))
                                .unwrap();
                        })
                    })
                    .unwrap();
                wait.recv_timeout(std::time::Duration::from_secs(5))
                    .unwrap();
                assert!(
                    OpenOptions::new()
                        .read(!write)
                        .write(write)
                        .custom_flags(0x800)
                        .open(&f.path)
                        .is_err()
                );
                done.send(()).unwrap();
                assert_eq!(handle.join().unwrap(), Err(Error::Unavailable));
            });
            assert_eq!(ffi::caller_mask().unwrap(), before);
        }
    }
}
