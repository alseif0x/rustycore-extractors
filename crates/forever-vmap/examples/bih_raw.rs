// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: synthetic BIH build/reader adapter, no acquisition.
// Ordinary bounded synthetic-file I/O; no no-symlink or file-identity claim.
// Complete input, build and strict reader checks precede binary stdout.
use forever_vmap::bih::{self, Bounds, View};
use std::io::{Read, Write};

const MAX_PRIMITIVES: usize = 100_000;
const MAX_FIXTURE: usize = 16 + MAX_PRIMITIVES * 24;

fn read(path: &std::ffi::OsStr, limit: usize) -> Result<Vec<u8>, &'static str> {
    let file = std::fs::File::open(path).map_err(|_| "qa_input")?;
    if !file.metadata().map_err(|_| "qa_input")?.is_file() {
        return Err("qa_input");
    }
    let mut data = Vec::new();
    file.take(limit as u64 + 1)
        .read_to_end(&mut data)
        .map_err(|_| "qa_input")?;
    if data.len() > limit {
        return Err("qa_extent");
    }
    Ok(data)
}
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
    let args: Vec<_> = std::env::args_os().take(5).collect();
    if args.len() == 4 && args[1] == "--verify" {
        let expected: usize = args[3]
            .to_str()
            .ok_or("qa_count")?
            .parse()
            .map_err(|_| "qa_count")?;
        if expected > MAX_PRIMITIVES {
            return Err("qa_count");
        }
        let bytes = read(&args[2], 128 * 1024 * 1024)?;
        View::parse(&bytes, expected, bih::Limits::default()).map_err(|e| e.code())?;
        println!(
            "{{\"status\":\"ok\",\"primitives\":{expected},\"bytes\":{}}}",
            bytes.len()
        );
        return Ok(());
    }
    if args.len() != 2 {
        return Err("qa_usage");
    }
    let data = read(&args[1], MAX_FIXTURE)?;
    if data.len() < 16 || &data[..8] != b"RCEBIH01" {
        return Err("qa_header");
    }
    let leaf = word(&data, 8)?;
    let count = word(&data, 12)? as usize;
    if count > MAX_PRIMITIVES || data.len() != 16 + count * 24 {
        return Err("qa_extent");
    }
    let mut bounds = Vec::new();
    bounds
        .try_reserve_exact(count)
        .map_err(|_| "qa_allocation")?;
    for index in 0..count {
        let mut bits = [0; 6];
        for (field, bit) in bits.iter_mut().enumerate() {
            *bit = word(&data, 16 + index * 24 + field * 4)?;
        }
        bounds.push(Bounds::from_bits(bits).map_err(|e| e.code())?);
    }
    let output = bih::build(&bounds, leaf, bih::Limits::default()).map_err(|e| e.code())?;
    View::parse(output.bytes(), count, bih::Limits::default()).map_err(|e| e.code())?;
    std::io::stdout()
        .lock()
        .write_all(output.bytes())
        .map_err(|_| "qa_output")
}
fn main() {
    if let Err(code) = run() {
        eprintln!("{code}");
        std::process::exit(4);
    }
}
