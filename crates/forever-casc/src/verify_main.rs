// SPDX-License-Identifier: GPL-3.0-or-later
//! Offline, read-only CLI verifying a Forever v2 output directory.
//!
//! `forever-db2-verify` never opens the client, CASC or a remote listfile: it
//! reads only the published `forever-manifest.json` and `.db2` files, then
//! recomputes their hashes and reopens their metadata/decode. Exit codes follow
//! `contracts/forever-contract-v2.json`:
//! - 0 verified
//! - 2 `invalid_invocation` / `MANIFEST_SCHEMA_INVALID`
//! - 4 `input_io`
//! - 8 `INCOMPLETE_RUN`
//! - 9 `UNSAFE_PATH` / `INVENTORY_MISMATCH` / `TABLE_IDENTITY_MISMATCH` /
//!   `ROW_DECODE_MISMATCH`
//! - 10 internal
use forever_casc::verify::{Code, VerifyReport, verify};
use std::ffi::OsString;
use std::path::PathBuf;
use std::process::ExitCode;

const HELP: &str = "Usage: forever-db2-verify --output <existing-directory>\n\
Offline read-only verification of a Forever v2 DB2 batch output directory.\n\
It never opens the client or CASC, and never writes to the output directory.";

fn main() -> ExitCode {
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    if args == [OsString::from("--help")] {
        println!("{HELP}");
        return ExitCode::SUCCESS;
    }
    let output = match args.as_slice() {
        [flag, value] if flag.to_str() == Some("--output") && !value.is_empty() => {
            PathBuf::from(value)
        }
        _ => return fail(Code::InvalidInvocation),
    };
    let report = verify(&output);
    println!("{}", report.json());
    ExitCode::from(report.exit_code())
}

fn fail(code: Code) -> ExitCode {
    println!("{}", VerifyReport::failure(code).json());
    ExitCode::from(code.exit())
}
