// SPDX-License-Identifier: GPL-3.0-or-later
//! `forever-casc-probe`: bounded native CASC read diagnostic.
//!
//! Argument parsing is probe-specific and intentionally separate from the
//! `forever-db2-extract` parser in `forever_casc::options`: the two CLIs have
//! different contracts (`--file-id/--max-bytes/--compare/--json` here,
//! `--output/--manifest` there). Keeping the probe parser local avoids loading
//! the extract-only `options` module, which depends on the library crate.
use forever_casc::{CascFile, Error, Locale, PRODUCT};
use forever_db2::Metadata;
use std::{
    ffi::OsString,
    fs::File,
    io::{Cursor, Read},
    path::PathBuf,
    process::ExitCode,
};

/// Probe read budget ceiling: prototype policy, not a CASC format limit.
const MAX_BYTES: u32 = 128 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
struct Options {
    client: PathBuf,
    locale: Locale,
    id: u32,
    max: u32,
    compare: Option<PathBuf>,
}

/// Parses the probe CLI. `Ok(None)` is the standalone `--help`; every malformed
/// invocation (including an unsupported locale, which the probe reports as
/// `invalid_invocation`) is `Err(())`. No argument value is retained in errors.
fn parse(args: Vec<OsString>) -> Result<Option<Options>, ()> {
    if args.len() == 1 && args[0] == "--help" {
        return Ok(None);
    }
    let (mut client, mut locale, mut id, mut max, mut compare) = (None, None, None, None, None);
    let mut json = false;
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        if arg == "--json" && !json {
            json = true;
            continue;
        }
        let value = args.next().ok_or(())?;
        if value.is_empty() || value.to_string_lossy().starts_with("--") {
            return Err(());
        }
        match arg.to_str().ok_or(())? {
            "--client" if client.is_none() => client = Some(PathBuf::from(value)),
            "--locale" if locale.is_none() => {
                locale = Some(Locale::parse(value.to_str().ok_or(())?).map_err(|_| ())?)
            }
            "--file-id" if id.is_none() => {
                id = Some(value.to_str().ok_or(())?.parse::<u32>().map_err(|_| ())?)
            }
            "--max-bytes" if max.is_none() => {
                max = Some(value.to_str().ok_or(())?.parse::<u32>().map_err(|_| ())?)
            }
            "--compare" if compare.is_none() => compare = Some(PathBuf::from(value)),
            _ => return Err(()),
        }
    }
    let id = id.ok_or(())?;
    let max = max.unwrap_or(MAX_BYTES);
    if id == 0 || id == u32::MAX || max == 0 || max > MAX_BYTES {
        return Err(());
    }
    Ok(Some(Options {
        client: client.ok_or(())?,
        locale: locale.ok_or(())?,
        id,
        max,
        compare,
    }))
}

fn fail(code: &str, exit: u8) -> ExitCode {
    println!(
        "{{\"report_version\":1,\"status\":\"error\",\"code\":\"{code}\",\"paths_redacted\":true}}"
    );
    ExitCode::from(exit)
}
fn read_compare(path: &std::path::Path, max: u32) -> Result<Vec<u8>, Error> {
    let file = File::open(path).map_err(|_| Error::Input)?;
    let mut bytes = Vec::new();
    file.take(u64::from(max) + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| Error::Input)?;
    if bytes.len() > max as usize {
        return Err(Error::Limit);
    }
    Ok(bytes)
}
fn main() -> ExitCode {
    let options = match parse(std::env::args_os().skip(1).collect()) {
        Ok(None) => {
            println!(
                "Usage: forever-casc-probe --client <root> --locale <enUS|esES> \
                --file-id <id> [--max-bytes <budget>] [--compare <local-file>] [--json]\n\
                Offline read-only prototype; native CascLib backend, not full extraction."
            );
            return ExitCode::SUCCESS;
        }
        Ok(Some(options)) => options,
        Err(()) => return fail("invalid_invocation", 2),
    };
    let file = match CascFile::open(&options.client, options.locale, options.id) {
        Ok(file) => file,
        Err(error) => return fail(error.code(), error.exit_code()),
    };
    let build = file.build();
    let bytes = match file.read_all(options.max) {
        Ok(bytes) => bytes,
        Err(error) => return fail(error.code(), error.exit_code()),
    };
    let compared = match options.compare {
        Some(path) => match read_compare(&path, options.max) {
            Ok(expected) if expected == bytes => "true",
            Ok(_) => return fail("reference_bytes_differ", 9),
            Err(error) => return fail(error.code(), error.exit_code()),
        },
        None => "null",
    };
    let metadata = match Metadata::read(&mut Cursor::new(&bytes)) {
        Ok(metadata) => metadata,
        Err(error) => return fail(error.code(), error.exit_code()),
    };
    println!(
        "{{\"report_version\":1,\"status\":\"success\",\"scope\":\"casc_read_probe\",\
        \"implementation\":\"rust_with_native_casclib\",\"offline\":true,\"read_only\":true,\
        \"product\":\"{PRODUCT}\",\"build\":{build},\"client_build_admitted\":true,\
        \"locale\":\"{}\",\"file_id\":{},\"bytes\":{},\"reference_bytes_equal\":{},\
        \"magic\":\"WDC5\",\"version\":5,\"records\":{},\"fields\":{},\
        \"table_hash\":{},\"layout_hash\":{},\"sections\":{},\
        \"table_schema_validated\":false,\"record_data_decoded\":false,\"paths_redacted\":true}}",
        options.locale.name(),
        options.id,
        bytes.len(),
        compared,
        metadata.header.record_count,
        metadata.header.field_count,
        metadata.header.table_hash,
        metadata.header.layout_hash,
        metadata.header.section_count
    );
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(s: &str) -> Vec<OsString> {
        s.split_whitespace().map(OsString::from).collect()
    }

    #[test]
    fn help_is_standalone_and_exclusive() {
        assert_eq!(parse(args("--help")), Ok(None));
        for bad in ["", "--help --json", "--help --client x"] {
            assert!(parse(args(bad)).is_err(), "{bad}");
        }
    }

    #[test]
    fn explicit_input_locale_id_and_budget_required_or_bounded() {
        let good = "--client local --locale esES --file-id 1361031";
        assert!(parse(args(good)).is_ok());
        for bad in [
            "",
            "--client local",
            "--client x --locale zzZZ --file-id 1",
            "--client x --locale enUS --file-id 0",
            "--client x --locale enUS --file-id 4294967295",
            "--client x --locale enUS --file-id 1 --max-bytes 0",
            "--client x --locale enUS --file-id 1 --max-bytes 134217729",
            "--client x --locale enUS --file-id 1 --json --json",
            "--client x --locale enUS --file-id 1 --client y",
        ] {
            assert!(parse(args(bad)).is_err(), "{bad}");
        }
    }

    #[test]
    fn json_flag_is_optional_and_duplicate_free() {
        let parsed = parse(args("--client c --locale enUS --file-id 5 --json")).unwrap();
        let options = parsed.expect("run");
        assert_eq!(options.id, 5);
        assert_eq!(options.max, MAX_BYTES);
        assert_eq!(options.compare, None);
    }
}
