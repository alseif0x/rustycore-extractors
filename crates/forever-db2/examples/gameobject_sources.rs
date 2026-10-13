// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: bounded local direct-record QA projection.
//! gameobject_sources <input.db2> <new-output.json>
//!
//! Private QA output contains all direct display-ID/FileDataID pairs, validated
//! copy operations and nonzero direct candidates. It must stay outside Git and
//! model context when the input is real. Only static codes/counts reach stdout.
//! Candidate naming is diagnostic; no model acquisition or temporary GameObject
//! list is produced. Copies never create candidates. Complete decoding precedes
//! output creation. An output I/O error may leave an incomplete file, never a
//! successful report. Ordinary bounded file I/O is not a publication protocol.
use forever_db2::{RecordError, terrain_sources};
use std::{
    fs::{File, Metadata, OpenOptions},
    io::{self, Read, Write},
    path::Path,
    process::ExitCode,
};

const MAX_INPUT_BYTES: u64 = 16 * 1024 * 1024;

fn main() -> ExitCode {
    let result = invocation();
    let (message, code) = match result {
        Ok((direct, copies, candidates)) => (
            format!(
                "{{\"status\":\"success\",\"direct_records\":{direct},\"copy_records\":{copies},\"candidate_records\":{candidates}}}"
            ),
            0,
        ),
        Err(code) => (
            format!("{{\"status\":\"error\",\"code\":\"{code}\"}}"),
            if code == "invalid_invocation" { 2 } else { 4 },
        ),
    };
    if writeln!(io::stdout().lock(), "{message}").is_err() {
        return ExitCode::from(4);
    }
    ExitCode::from(code)
}

fn invocation() -> Result<(usize, usize, usize), &'static str> {
    let mut args = std::env::args_os().skip(1);
    let input = args.next().ok_or("invalid_invocation")?;
    let output = args.next().ok_or("invalid_invocation")?;
    if args.next().is_some() {
        return Err("invalid_invocation");
    }
    run(Path::new(&input), Path::new(&output))
}

fn run(input: &Path, output: &Path) -> Result<(usize, usize, usize), &'static str> {
    let data = read_input(input)?;
    let source = terrain_sources::read_gameobject_display_info(&data).map_err(RecordError::code)?;
    let rows = source.direct_records();
    let copies = source.copies();
    let candidates = rows
        .iter()
        .filter(|row| row.fields.file_data_id != 0)
        .count();
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let file = options.open(output).map_err(|_| "output_io")?;
    let mut out = io::BufWriter::new(file);
    let mut write = || -> io::Result<()> {
        out.write_all(b"{\"direct\":[")?;
        for (i, row) in rows.iter().enumerate() {
            if i != 0 {
                out.write_all(b",")?;
            }
            write!(out, "[{},{}]", row.id, row.fields.file_data_id)?;
        }
        out.write_all(b"],\"copies\":[")?;
        for (i, copy) in copies.iter().enumerate() {
            if i != 0 {
                out.write_all(b",")?;
            }
            write!(out, "[{},{}]", copy.new_id, copy.source_id)?;
        }
        out.write_all(b"],\"candidates\":[")?;
        for (i, row) in rows
            .iter()
            .filter(|row| row.fields.file_data_id != 0)
            .enumerate()
        {
            if i != 0 {
                out.write_all(b",")?;
            }
            let fdid = row.fields.file_data_id;
            write!(out, "[{},{fdid},\"FILE{fdid:08X}.xxx\"]", row.id)?;
        }
        out.write_all(b"]}\n")?;
        out.flush()
    };
    write().map_err(|_| "output_io")?;
    Ok((rows.len(), copies.len(), candidates))
}

fn read_input(path: &Path) -> Result<Vec<u8>, &'static str> {
    let named_before = std::fs::symlink_metadata(path).map_err(|_| "input_io")?;
    if !named_before.is_file() {
        return Err("input_not_regular");
    }
    let mut file = File::open(path).map_err(|_| "input_io")?;
    let before = file.metadata().map_err(|_| "input_io")?;
    if !same_file(&named_before, &before) {
        return Err("input_changed");
    }
    if before.len() > MAX_INPUT_BYTES {
        return Err(RecordError::Limit.code());
    }
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(before.len() as usize)
        .map_err(|_| RecordError::Limit.code())?;
    (&mut file)
        .take(MAX_INPUT_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "input_io")?;
    let after = file.metadata().map_err(|_| "input_io")?;
    let named_after = std::fs::symlink_metadata(path).map_err(|_| "input_changed")?;
    if !same_file(&before, &after)
        || !same_file(&after, &named_after)
        || bytes.len() as u64 != before.len()
    {
        return Err("input_changed");
    }
    if bytes.len() as u64 > MAX_INPUT_BYTES {
        return Err(RecordError::Limit.code());
    }
    Ok(bytes)
}

fn same_file(a: &Metadata, b: &Metadata) -> bool {
    let same =
        a.is_file() && b.is_file() && a.len() == b.len() && a.modified().ok() == b.modified().ok();
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        same && a.dev() == b.dev()
            && a.ino() == b.ino()
            && a.ctime() == b.ctime()
            && a.ctime_nsec() == b.ctime_nsec()
    }
    #[cfg(not(unix))]
    {
        same
    }
}
