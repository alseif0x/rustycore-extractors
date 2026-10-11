// SPDX-License-Identifier: GPL-3.0-or-later
//! Read-only GT v1 output verifier; no native feature or CASC access.
use forever_casc::gametables::{
    options,
    verify::{self, Code, Report},
};
use std::{ffi::OsString, process::ExitCode};

fn main() -> ExitCode {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args == [OsString::from("--help")] {
        println!(
            "Usage: forever-gt-verify --output <existing-root>\nOffline read-only verification of the closed GT6/GT3 output contract."
        );
        return ExitCode::SUCCESS;
    }
    let report = match options::verify_output(&args) {
        Ok(path) => verify::verify(&path),
        Err(_) => Report::failure(Code::Invocation),
    };
    println!("{}", report.json());
    ExitCode::from(report.code.exit())
}
