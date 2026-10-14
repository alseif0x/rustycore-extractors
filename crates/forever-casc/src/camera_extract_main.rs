// SPDX-License-Identifier: GPL-3.0-or-later
//! Offline complete camera files; no M2 or runtime claim.
use forever_casc::{
    acquisition::{Profile, Session},
    cameras::{
        CameraContract, batch,
        options::{self, Parsed},
    },
};
use std::process::ExitCode;
struct Failure {
    contract: CameraContract,
    code: &'static str,
    exit: u8,
    acquisition: Option<Profile>,
}
fn run() -> Result<ExitCode, Failure> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let options = match options::parse(&args).map_err(|e| Failure {
        contract: CameraContract::Legacy,
        code: e.code,
        exit: e.exit,
        acquisition: None,
    })? {
        Parsed::Help => {
            println!(
                "Usage: forever-camera-extract --client <root> --locale <enUS|esES> --output <new-root> [--profile camera-raw-70338-v1] [--backend <native|rust-local>] [--index-layout <contiguous|paged>] [--root-layout <headerless-18125|header-30080|header-50893-v1|header-50893-v2>] [--max-file-bytes <1..16777216>]\nNative is default. Rust local requires both layouts; only Rust accepts a reduced file limit. Build 70338 requires its explicit profile and Rust local. Offline complete DB2 discovery and raw camera files. No M2 parsing or runtime acceptance."
            );
            return Ok(ExitCode::SUCCESS);
        }
        Parsed::Run(options) => options,
    };
    let acquisition = options.acquisition.profile();
    let failure = |code, exit| Failure {
        contract: options.contract,
        code,
        exit,
        acquisition,
    };
    options
        .acquisition
        .ensure_available()
        .map_err(|e| failure(e.code, e.exit))?;
    // Discovery DB2 and all cameras use the same bounded local store. Locale
    // selects storage; native CascOpenFile locale behavior is unchanged.
    let admitted =
        forever_casc::admit(&options.client).map_err(|e| failure(e.code(), e.exit_code()))?;
    options
        .contract
        .admit_build(admitted)
        .map_err(|code| failure(code, 3))?;
    let mut session = Session::open(&options.client, options.locale, options.acquisition)
        .map_err(|e| failure(e.code(), e.exit_code()))?;
    if session.build() != admitted {
        return Err(failure("client_build_changed", 4));
    }
    let report = if options.contract == CameraContract::Build70338V1 {
        batch::extract_with_contract(
            options.contract,
            &options.client,
            &options.output,
            session.build(),
            options.locale,
            acquisition.expect("parsed Rust-local profile"),
            |id, budget| session.read(id, budget),
        )
    } else {
        batch::extract_with_profile(
            &options.client,
            &options.output,
            session.build(),
            options.locale,
            acquisition,
            |id, budget| session.read(id, budget),
        )
    }
    .map_err(|code| failure(code, if code == "unsupported_client" { 3 } else { 4 }))?;
    println!("{}", report.json());
    Ok(ExitCode::from(if report.complete { 0 } else { 8 }))
}
fn main() -> ExitCode {
    match run() {
        Ok(exit) => exit,
        Err(Failure {
            contract,
            code,
            exit,
            acquisition,
        }) => {
            let version = if contract == CameraContract::Build70338V1 {
                3
            } else if acquisition.is_some() {
                2
            } else {
                1
            };
            let provenance = acquisition
                .map_or_else(String::new, |p| format!(",\"acquisition\":{}", p.render()));
            let component = if contract == CameraContract::Build70338V1 {
                format!(",\"component_profile\":\"{}\"", CameraContract::PROFILE)
            } else {
                String::new()
            };
            println!(
                "{{\"report_version\":{version}{component}{provenance},\"scope\":\"complete_camera_files\",\"status\":\"error\",\"code\":\"{code}\",\"offline\":true,\"paths_redacted\":true,\"full_client_extraction\":false,\"m2_parsed\":false,\"waypoints_generated\":false,\"runtime_accepted\":false}}"
            );
            ExitCode::from(exit)
        }
    }
}
