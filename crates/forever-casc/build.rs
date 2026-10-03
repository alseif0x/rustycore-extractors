// SPDX-License-Identifier: GPL-3.0-or-later
use std::{env, path::PathBuf, process::Command};

fn run(command: &mut Command) {
    let status = command.status().expect("Cannot start native build command");
    assert!(status.success(), "Pinned native build failed");
}

fn main() {
    println!("cargo:rerun-if-env-changed=RCE_FOREVER_CASC_REFERENCE");
    if env::var_os("CARGO_FEATURE_NATIVE").is_none() {
        return;
    }
    assert_eq!(
        env::var("CARGO_CFG_TARGET_OS").unwrap(),
        "linux",
        "Prototype supports Linux only"
    );
    assert_eq!(
        env::var("HOST").unwrap(),
        env::var("TARGET").unwrap(),
        "Cross-compilation is not implemented"
    );
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("../..");
    let reference = env::var_os("RCE_FOREVER_CASC_REFERENCE")
        .expect("native requires RCE_FOREVER_CASC_REFERENCE at pinned commit 02245dcd");
    let output = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("native");
    println!(
        "cargo:rerun-if-changed={}",
        root.join("native/forever-casc").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        root.join("src/compat/CascSocketsDisabled.cpp").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        root.join("src/compat/CascOverwatchDisabled.cpp").display()
    );
    // Source changes must invalidate Cargo's build-script cache too. CMake
    // verifies each actual input against immutable Git blobs before compiling.
    println!(
        "cargo:rerun-if-changed={}",
        PathBuf::from(&reference).join("dep/CascLib").display()
    );
    run(Command::new("cmake")
        .arg("-S")
        .arg(root.join("native/forever-casc"))
        .arg("-B")
        .arg(&output)
        .arg("-DCMAKE_BUILD_TYPE=Release")
        .arg(format!(
            "-DRCE_REFERENCE={}",
            PathBuf::from(reference).display()
        )));
    run(Command::new("cmake")
        .arg("--build")
        .arg(&output)
        .arg("--target")
        .arg("forever_casc_bridge")
        .arg("--parallel")
        .arg("1"));
    println!("cargo:rustc-link-search=native={}", output.display());
    for library in ["forever_casc_bridge", "forever_casc_core", "forever_zlib"] {
        println!("cargo:rustc-link-lib=static={library}");
    }
    for library in ["crypto", "stdc++", "pthread"] {
        println!("cargo:rustc-link-lib={library}");
    }
}
