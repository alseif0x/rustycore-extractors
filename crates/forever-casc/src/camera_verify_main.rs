// SPDX-License-Identifier: GPL-3.0-or-later
//! Offline verification with rediscovery; no client or native feature needed.
use forever_casc::cameras::{
    options,
    verify::{self, Code, Report},
};
use std::{ffi::OsString, process::ExitCode};
fn main() -> ExitCode {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args == [OsString::from("--help")] {
        println!(
            "Usage: forever-camera-verify --output <existing-root>\nRead-only verification of source DB2 discovery and complete raw camera files. No M2/runtime claim."
        );
        return ExitCode::SUCCESS;
    }
    let report = match options::verify_output(&args) {
        Ok(root) => verify::verify(&root),
        Err(_) => Report::failure(Code::Invocation),
    };
    println!("{}", report.json());
    ExitCode::from(report.code.exit())
}
