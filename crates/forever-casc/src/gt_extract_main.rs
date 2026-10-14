// SPDX-License-Identifier: GPL-3.0-or-later
//! Offline GT acquisition. Raw bytes are preserved, never rewritten as floats.
use forever_casc::{
    acquisition::{Profile, Session},
    gametables::{
        batch, catalog_batch,
        contract::CatalogContract,
        options::{self, Parsed, Selection},
    },
};
use std::process::ExitCode;

struct Failure {
    contract: CatalogContract,
    code: &'static str,
    exit: u8,
    catalog: bool,
    acquisition: Option<Profile>,
}
fn run() -> Result<ExitCode, Failure> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let options = match options::parse(&args).map_err(|e| Failure {
        contract: CatalogContract::Legacy,
        code: e.code,
        exit: e.exit,
        catalog: false,
        acquisition: None,
    })? {
        Parsed::Help => {
            println!(
                "Usage: forever-gt-extract --client <root> --locale <enUS|esES> --output <new-root> [--group <all|initial|spell-values|catalog>] [--profile gt-catalog-70338-v1] [--backend <native|rust-local>] [--index-layout <contiguous|paged>] [--root-layout <headerless-18125|header-30080|header-50893-v1|header-50893-v2>] [--max-file-bytes <1..4194304>]\nNative is default. Rust local requires both layouts; only Rust accepts a reduced file limit. Default all selects six files; catalog selects twenty. Build 70338 requires the explicit catalog profile and Rust local. Offline only; no typed-loader or full-extractor claim."
            );
            return Ok(ExitCode::SUCCESS);
        }
        Parsed::Run(options) => options,
    };
    let catalog = options.selection == Selection::Catalog;
    let acquisition = options.acquisition.profile();
    let failure = |code, exit| Failure {
        contract: options.contract,
        code,
        exit,
        catalog,
        acquisition,
    };
    // Availability precedes any client/output access. A single session is
    // established before output creation and reused throughout the batch.
    options
        .acquisition
        .ensure_available()
        .map_err(|e| failure(e.code, e.exit))?;
    let admitted =
        forever_casc::admit(&options.client).map_err(|e| failure(e.code(), e.exit_code()))?;
    options
        .contract
        .admit_build(admitted)
        .map_err(|code| failure(code, 3))?;
    let mut session = Session::open(&options.client, options.locale, options.acquisition)
        .map_err(|e| failure(e.code(), e.exit_code()))?;
    let build = session.build();
    if build != admitted {
        return Err(failure("client_build_changed", 4));
    }
    let fail_batch = |code| failure(code, if code == "unsupported_client" { 3 } else { 4 });
    let complete = match options.selection {
        Selection::Typed(plan) => {
            let report = batch::extract_with_profile(
                plan,
                &options.client,
                &options.output,
                build,
                options.locale,
                acquisition,
                |table, budget| session.read(table.file_id(), budget),
            )
            .map_err(fail_batch)?;
            println!("{}", report.json());
            report.complete
        }
        Selection::Catalog => {
            let report = if options.contract == CatalogContract::Build70338V1 {
                catalog_batch::extract_with_contract(
                    options.contract,
                    &options.client,
                    &options.output,
                    build,
                    options.locale,
                    acquisition.expect("parsed Rust-local profile"),
                    |table, budget| session.read(table.file_id(), budget),
                )
            } else {
                catalog_batch::extract_with_profile(
                    &options.client,
                    &options.output,
                    build,
                    options.locale,
                    acquisition,
                    |table, budget| session.read(table.file_id(), budget),
                )
            }
            .map_err(fail_batch)?;
            println!("{}", report.json());
            report.complete
        }
    };
    Ok(ExitCode::from(if complete { 0 } else { 8 }))
}

fn main() -> ExitCode {
    match run() {
        Ok(exit) => exit,
        Err(Failure {
            contract,
            code,
            exit,
            catalog,
            acquisition,
        }) => {
            let version = if contract == CatalogContract::Build70338V1 {
                4
            } else if acquisition.is_some() {
                3
            } else if catalog {
                2
            } else {
                1
            };
            let scope = if catalog {
                "pinned_gt_catalog"
            } else {
                "explicit_gt_batch"
            };
            let provenance = acquisition
                .map_or_else(String::new, |p| format!(",\"acquisition\":{}", p.render()));
            let component = if contract == CatalogContract::Build70338V1 {
                format!(",\"component_profile\":\"{}\"", CatalogContract::PROFILE)
            } else {
                String::new()
            };
            println!(
                "{{\"report_version\":{version}{component}{provenance},\"scope\":\"{scope}\",\"status\":\"error\",\"code\":\"{code}\",\"offline\":true,\"paths_redacted\":true,\"full_client_extraction\":false}}"
            );
            ExitCode::from(exit)
        }
    }
}
