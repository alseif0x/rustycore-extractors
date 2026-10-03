// SPDX-License-Identifier: GPL-3.0-or-later
mod options;
use forever_casc::{BUILD, CascFile, Error, PRODUCT};
use forever_db2::Metadata;
use std::{
    fs::File,
    io::{Cursor, Read},
    process::ExitCode,
};

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
    let options = match options::parse(std::env::args_os().skip(1).collect()) {
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
    let bytes = match CascFile::open(&options.client, options.locale, options.id)
        .and_then(|file| file.read_all(options.max))
    {
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
        \"product\":\"{PRODUCT}\",\"build\":{BUILD},\"client_build_admitted\":true,\
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
