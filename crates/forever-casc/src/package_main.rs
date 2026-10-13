// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: offline artifact-integrity verifier, path-free diagnostics.
use forever_casc::package::Error;
use std::{ffi::OsString, path::PathBuf, process::ExitCode};
const HELP: &str = "Usage: forever-package verify --output <absolute-directory> [--expect-context <canonical-context.json>]\nOffline package integrity verification. Producer declarations and hashes do not establish format or runtime compatibility. The disk profile currently requires Linux.";
fn arguments(args: &[OsString]) -> Option<(PathBuf, Option<PathBuf>)> {
    match args {
        [verb, flag, path] if verb == "verify" && flag == "--output" && !path.is_empty() => {
            Some((path.into(), None))
        }
        [verb, flag, path, expect, context]
            if verb == "verify"
                && flag == "--output"
                && !path.is_empty()
                && expect == "--expect-context"
                && !context.is_empty() =>
        {
            Some((path.into(), Some(context.into())))
        }
        _ => None,
    }
}
fn fail(code: &str, exit: u8) -> ExitCode {
    println!(
        "{{\"report_version\":1,\"scope\":\"artifact_integrity\",\"status\":\"failed\",\"code\":\"{code}\"}}"
    );
    ExitCode::from(exit)
}
#[cfg(target_os = "linux")]
fn run(
    output: PathBuf,
    expected: Option<PathBuf>,
) -> Result<forever_casc::package::Summary, Error> {
    use forever_casc::package::{Context, Limits, verify};
    use std::{fs::File, io::Read};
    let context = expected
        .map(|p| {
            let mut s = String::new();
            File::open(p)
                .map_err(|_| Error::Io)?
                .take(1025)
                .read_to_string(&mut s)
                .map_err(|_| Error::Schema)?;
            Context::parse(&s)
        })
        .transpose()?;
    verify(&output, context, Limits::default())
}
#[cfg(not(target_os = "linux"))]
fn run(_: PathBuf, _: Option<PathBuf>) -> Result<forever_casc::package::Summary, Error> {
    Err(Error::Unsupported)
}
fn main() -> ExitCode {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args == [OsString::from("--help")] {
        println!("{HELP}");
        return ExitCode::SUCCESS;
    }
    let Some((output, expected)) = arguments(&args) else {
        return fail("invalid_invocation", 2);
    };
    match run(output, expected) {
        Ok(summary) => {
            println!("{}", summary.json());
            ExitCode::SUCCESS
        }
        Err(e) => fail(
            e.code(),
            match e {
                Error::Incomplete => 8,
                Error::Io => 4,
                Error::Unsupported => 2,
                _ => 9,
            },
        ),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn args(s: &[&str]) -> Vec<OsString> {
        s.iter().map(OsString::from).collect()
    }
    #[test]
    fn accepts_explicit_verify_and_context() {
        assert_eq!(
            arguments(&args(&[
                "verify",
                "--output",
                "/out",
                "--expect-context",
                "/c"
            ])),
            Some((PathBuf::from("/out"), Some(PathBuf::from("/c"))))
        );
    }
    #[test]
    fn accepts_verify_without_external_context() {
        assert_eq!(
            arguments(&args(&["verify", "--output", "/out"])),
            Some((PathBuf::from("/out"), None))
        );
    }
    #[test]
    fn rejects_implicit_or_repeated_options() {
        for a in [
            &["/out"][..],
            &["--output", "/out"],
            &["verify", "--output", ""],
            &["verify", "--output", "/out", "--output", "/other"],
            &["extract", "--output", "/out"],
        ] {
            assert!(arguments(&args(a)).is_none());
        }
    }
}
