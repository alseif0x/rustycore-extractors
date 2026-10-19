// SPDX-License-Identifier: GPL-3.0-or-later
//! Synthetic caller-output guards; no client files are opened.
use super::*;
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Lab(PathBuf);
impl Lab {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "rce-encoding-probe-output-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}
impl Drop for Lab {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn output_guard_rejects_client_and_git_checkout_destinations_without_creation() {
    let lab = Lab::new();
    let client = lab.0.join("synthetic-client");
    fs::create_dir(&client).unwrap();
    let in_client = client.join("probe.json");
    assert_eq!(
        output_path(&client, &in_client).unwrap_err().code,
        "product_diagnostic_path"
    );
    assert!(!in_client.exists());

    let checkout = lab.0.join("synthetic-checkout");
    fs::create_dir_all(checkout.join("reports")).unwrap();
    fs::write(checkout.join(".git"), b"synthetic marker").unwrap();
    let in_checkout = checkout.join("reports/probe.json");
    assert_eq!(
        output_path(&client, &in_checkout).unwrap_err().code,
        "product_diagnostic_path"
    );
    assert!(!in_checkout.exists());
}

#[test]
fn output_guard_requires_a_new_file_in_an_existing_external_directory() {
    let lab = Lab::new();
    let client = lab.0.join("synthetic-client");
    let external = lab.0.join("external");
    fs::create_dir(&client).unwrap();
    fs::create_dir(&external).unwrap();
    let path = external.join("trace.json");
    assert_eq!(output_path(&client, &path).unwrap(), path);
    fs::write(&path, b"synthetic existing trace").unwrap();
    assert_eq!(
        output_path(&client, &path).unwrap_err().code,
        "product_diagnostic_path"
    );
}
