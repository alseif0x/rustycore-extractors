// SPDX-License-Identifier: GPL-3.0-or-later
//! Offline CLI for the bounded explicit DB2 extraction batch.
//!
//! Exit codes and codes (all reports carry `report_version: 1`):
//! - 0 complete
//! - 2 `invalid_invocation` / `invalid_manifest` / `manifest_policy_limit` /
//!   `unsupported_locale`
//! - 3 `unsupported_client`
//! - 4 `input_io` / `manifest_input_io` / `native_read_failed` /
//!   `file_unavailable` / `encrypted_key_unavailable` / `local_file_incomplete`
//! - 6 `read_policy_limit`
//! - 8 `incomplete`: the run finished but at least one entry failed
//!
//! Argument parsing lives in `forever_casc::options` and is unit-tested without
//! a client. An unreadable manifest is `manifest_input_io` (exit 4); malformed
//! or non-UTF-8 manifest content is `invalid_manifest` (exit 2); an oversized
//! or over-long manifest is `manifest_policy_limit` (exit 2). Reports are
//! path-free: no client root, manifest path or library error is echoed.
use forever_casc::{
    CascFile, admit,
    batch::{Plan, extract},
    options::{self, CliError, Parsed},
};
use std::{io::Read, process::ExitCode};

const HELP: &str = "Usage: forever-db2-extract --client <root> --locale <enUS|esES> \
--output <new-directory> [--manifest <name-tab-id.tsv>]\nDefault selection: \
ChrClasses and ChrRaces. Offline native CASC; no full-client extraction claim.";

fn fail(error: CliError) -> ExitCode {
    println!(
        "{{\"report_version\":1,\"status\":\"error\",\"code\":\"{}\",\"paths_redacted\":true}}",
        error.code
    );
    ExitCode::from(error.exit)
}

fn run() -> Result<ExitCode, CliError> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let options = match options::parse(&args)? {
        Parsed::Help => {
            println!("{HELP}");
            return Ok(ExitCode::SUCCESS);
        }
        Parsed::Run(options) => options,
    };
    let build =
        admit(&options.client).map_err(|error| CliError::new(error.code(), error.exit_code()))?;
    let plan = if let Some(path) = &options.manifest {
        let file = std::fs::File::open(path).map_err(|_| CliError::new("manifest_input_io", 4))?;
        let mut input = String::new();
        file.take(64 * 1024 + 1)
            .read_to_string(&mut input)
            .map_err(|_| CliError::new("invalid_manifest", 2))?;
        Plan::parse(&input).map_err(|code| CliError::new(code, 2))?
    } else {
        Plan::character_tables()
    };
    let report = extract(
        &plan,
        &options.client,
        &options.output,
        build,
        options.locale.name(),
        |entry, budget| {
            let file = CascFile::open(&options.client, options.locale, entry.id)?;
            let build = file.build();
            file.read_all(budget).map(|bytes| (build, bytes))
        },
    )
    .map_err(|code| CliError::new(code, 4))?;
    println!("{}", report.json);
    Ok(ExitCode::from(if report.complete { 0 } else { 8 }))
}

fn main() -> ExitCode {
    match run() {
        Ok(exit) => exit,
        Err(error) => fail(error),
    }
}
