// SPDX-License-Identifier: GPL-3.0-or-later
//! Offline CLI for the bounded explicit DB2 extraction batch.
//!
//! Exit codes and codes (native reports v1; explicit Rust-local reports v2):
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
    acquisition::{Profile, Session},
    batch::{Plan, extract_with_acquisition},
    options::{self, CliError, Parsed},
};
use std::{io::Read, process::ExitCode};

const HELP: &str = "Usage: forever-db2-extract --client <root> --locale <enUS|esES> \
--output <new-directory> [--manifest <name-tab-id.tsv>] \
[--schema-catalog pinned-metadata-v1|wowdbdefs-70205-v1|wowdbdefs-70338-v1] [--backend native|rust-local] \
[--index-layout contiguous|paged] \
[--root-layout headerless-18125|header-30080|header-50893-v1|header-50893-v2] \
[--max-file-bytes <1..16777216>]\n\
Catalog selection requires --manifest. Default selection: ChrClasses and ChrRaces.\n\
Native is the default. Rust-local requires both layouts and writes DB2 v4 provenance;\n\
--max-file-bytes only reduces its 16 MiB ceiling. Native rejects layout/limit options.\n\
Offline local CASC; no full-client extraction claim.";

fn fail(error: CliError, acquisition: Option<Profile>) -> ExitCode {
    let provenance = acquisition.map_or_else(String::new, |profile| {
        format!(",\"acquisition\":{}", profile.render())
    });
    let report_version = if acquisition.is_some() { 2 } else { 1 };
    println!(
        "{{\"report_version\":{report_version},\"status\":\"error\",\"code\":\"{}\",\"paths_redacted\":true{provenance}}}",
        error.code
    );
    ExitCode::from(error.exit)
}

fn run(options: options::Options) -> Result<ExitCode, CliError> {
    // Capability rejection precedes all client, manifest and output IO.
    options.acquisition.ensure_available()?;
    // Preserve native admission precedence. Rust-local validates the explicit
    // plan before its more expensive bootstrap; both open before output create.
    let mut session = if options.acquisition.is_rust_local() {
        None
    } else {
        Some(
            Session::open(&options.client, options.locale, options.acquisition)
                .map_err(|error| CliError::new(error.code(), error.exit_code()))?,
        )
    };
    let plan = if let Some(path) = &options.manifest {
        let file = std::fs::File::open(path).map_err(|_| CliError::new("manifest_input_io", 4))?;
        let mut input = String::new();
        file.take(64 * 1024 + 1)
            .read_to_string(&mut input)
            .map_err(|_| CliError::new("invalid_manifest", 2))?;
        if let Some(catalog) = options.schema_catalog {
            Plan::parse_with_catalog(&input, catalog)
        } else {
            Plan::parse(&input)
        }
        .map_err(|code| CliError::new(code, 2))?
    } else {
        Plan::character_tables()
    };
    if session.is_none() {
        session = Some(
            Session::open(&options.client, options.locale, options.acquisition)
                .map_err(|error| CliError::new(error.code(), error.exit_code()))?,
        );
    }
    let mut session = session.expect("one selected acquisition session");
    let report = extract_with_acquisition(
        &plan,
        &options.client,
        &options.output,
        session.build(),
        options.locale.name(),
        session.profile(),
        |entry, budget| session.read(entry.id, budget),
    )
    .map_err(|code| CliError::new(code, if code == "unsupported_client" { 3 } else { 4 }))?;
    println!("{}", report.json);
    Ok(ExitCode::from(if report.complete { 0 } else { 8 }))
}

fn main() -> ExitCode {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let options = match options::parse(&args) {
        Ok(Parsed::Help) => {
            println!("{HELP}");
            return ExitCode::SUCCESS;
        }
        Ok(Parsed::Run(options)) => options,
        Err(error) => return fail(error, None),
    };
    let profile = options.acquisition.profile();
    match run(options) {
        Ok(exit) => exit,
        Err(error) => fail(error, profile),
    }
}
