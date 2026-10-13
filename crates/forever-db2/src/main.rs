// SPDX-License-Identifier: GPL-3.0-or-later
mod cli;
mod report;

use forever_db2::{Error, Metadata};
use std::fs::File;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let json = args.iter().any(|arg| arg == "--json");
    let invocation = match cli::parse(args) {
        Ok(None) => {
            println!(
                "Usage: forever-db2-inspect --file <local.db2> [--json]\n\
                Read-only WDC5 header inspection; no CASC or record validation."
            );
            return ExitCode::SUCCESS;
        }
        Ok(Some(invocation)) => invocation,
        Err(()) => {
            report::failure("invalid_invocation", json);
            return ExitCode::from(2);
        }
    };
    let result = File::open(&invocation.file)
        .map_err(|_| Error::Io)
        .and_then(|mut file| Metadata::read(&mut file));
    match result {
        Ok(metadata) => {
            report::success(&metadata, invocation.json);
            ExitCode::SUCCESS
        }
        Err(error) => {
            report::failure(error.code(), invocation.json);
            ExitCode::from(error.exit_code())
        }
    }
}
