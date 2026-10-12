// SPDX-License-Identifier: GPL-3.0-or-later
//! Authored bounded supplied-file QA for the explicit regular-section API.
//! regular_sections <synthetic.db2> <new-output.json> <SpellDuration> <max-sections>
//! This tool performs no discovery, acquisition or product extraction. It
//! compares all three signed32 scalar bit patterns. Complete
//! decoding/projection validation precedes
//! exclusive private output creation. Ordinary file I/O is not publication or
//! crash durability; a failed write may leave an incomplete private file.
use forever_db2::{CharacterTable, DecodedTable, RecordError, Value};
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
        Ok(rows) => (format!("{{\"status\":\"success\",\"rows\":{rows}}}"), 0),
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

fn invocation() -> Result<usize, &'static str> {
    let mut args = std::env::args_os().skip(1);
    let input = args.next().ok_or("invalid_invocation")?;
    let output = args.next().ok_or("invalid_invocation")?;
    let table = args.next().ok_or("invalid_invocation")?;
    let ceiling = args.next().ok_or("invalid_invocation")?;
    if args.next().is_some() {
        return Err("invalid_invocation");
    }
    let table = match table.to_str() {
        Some("SpellDuration") => CharacterTable::SpellDuration,
        _ => return Err("invalid_invocation"),
    };
    let ceiling = ceiling
        .to_str()
        .and_then(|v| v.parse::<u32>().ok())
        .ok_or("invalid_invocation")?;
    run(Path::new(&input), Path::new(&output), table, ceiling)
}

fn run(
    input: &Path,
    output: &Path,
    table: CharacterTable,
    ceiling: u32,
) -> Result<usize, &'static str> {
    let data = read_input(input)?;
    let decoded =
        DecodedTable::read_regular_sections(&data, table, ceiling).map_err(RecordError::code)?;
    // Validate the QA projection itself before creating any private output.
    for fields in decoded.rows().values() {
        if fields.len() != 3
            || fields
                .iter()
                .any(|field| field.len() != 1 || !matches!(field.first(), Some(Value::Signed(_))))
        {
            return Err("qa_value_shape");
        }
    }
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
        out.write_all(b"{\"rows\":[")?;
        for (i, (id, fields)) in decoded.rows().iter().enumerate() {
            if i != 0 {
                out.write_all(b",")?;
            }
            write!(out, "[{id},[")?;
            for (j, value) in fields.iter().flatten().enumerate() {
                if j != 0 {
                    out.write_all(b",")?;
                }
                let bits = match value {
                    Value::Signed(v) => *v as u32,
                    _ => unreachable!("projection checked before output"),
                };
                write!(out, "{bits}")?;
            }
            out.write_all(b"]]")?;
        }
        out.write_all(b"]}\n")?;
        out.flush()
    };
    write().map_err(|_| "output_io")?;
    Ok(decoded.rows().len())
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
