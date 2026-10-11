// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: synthetic RCECCH01 cache-replay adapter. No acquisition.
// Complete bounded input is validated before any binary stdout output. Like
// the native QA counterpart, this ordinary synthetic-file caller does not
// claim file-identity/no-symlink guards. Errors expose only static codes.
use forever_vmap::placement::{self, cache};
use std::io::{Read, Write};

fn word(bytes: &[u8], at: usize) -> Result<u32, &'static str> {
    Ok(u32::from_le_bytes(
        bytes
            .get(at..at + 4)
            .ok_or("qa_extent")?
            .try_into()
            .map_err(|_| "qa_extent")?,
    ))
}
fn run() -> Result<(), &'static str> {
    let args: Vec<_> = std::env::args_os().take(3).collect();
    if args.len() != 2 {
        return Err("qa_usage");
    }
    let file = std::fs::File::open(&args[1]).map_err(|_| "qa_input")?;
    if !file.metadata().map_err(|_| "qa_input")?.is_file() {
        return Err("qa_input");
    }
    let mut bytes = Vec::new();
    file.take(24 + 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "qa_input")?;
    if bytes.len() < 24 || bytes.len() > 24 + 1024 * 1024 || &bytes[..8] != b"RCECCH01" {
        return Err("qa_header");
    }
    let count = word(&bytes, 16)? as usize;
    if count > 10000 || word(&bytes, 20)? as usize != bytes.len() - 24 {
        return Err("qa_extent");
    }
    let limits = cache::Limits {
        records: placement::Limits {
            max_bytes: 1024 * 1024,
            max_records: 10000,
            max_name_bytes: 500,
        },
        max_memory_bytes: 2 * 1024 * 1024,
    };
    let cached = cache::Cached::from_bytes(&bytes[24..], limits).map_err(|e| e.code())?;
    if cached.record_count() != count {
        return Err("qa_count");
    }
    let output = cached
        .replay(word(&bytes, 8)?, word(&bytes, 12)?, limits)
        .map_err(|e| e.code())?;
    std::io::stdout()
        .lock()
        .write_all(&output)
        .map_err(|_| "qa_output")
}
fn main() {
    if let Err(code) = run() {
        eprintln!("{code}");
        std::process::exit(4);
    }
}
