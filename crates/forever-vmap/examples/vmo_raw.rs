// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: bounded synthetic raw/VMO adapter, no acquisition.
// Ordinary file I/O; no no-symlink or file-identity guarantee.
use forever_vmap::{RawM2, vmo, wmo};
use std::io::{Read, Write};

fn read(path: &std::ffi::OsStr, limit: usize) -> Result<Vec<u8>, &'static str> {
    let file = std::fs::File::open(path).map_err(|_| "qa_input")?;
    if !file.metadata().map_err(|_| "qa_input")?.is_file() {
        return Err("qa_input");
    }
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "qa_input")?;
    if bytes.len() > limit {
        return Err("qa_extent");
    }
    Ok(bytes)
}

fn run() -> Result<(), &'static str> {
    let args: Vec<_> = std::env::args_os().take(5).collect();
    if args.len() != 3 {
        return Err("qa_usage");
    }
    let limits = vmo::Limits::default();
    if args[1] == "--verify" {
        let bytes = read(&args[2], limits.max_bytes)?;
        let view = vmo::View::parse(&bytes, limits).map_err(|error| error.code())?;
        let stats = view.stats();
        println!(
            "{{\"status\":\"ok\",\"groups\":{},\"vertices\":{},\"triangles\":{},\"liquids\":{},\"bytes\":{}}}",
            stats.groups,
            stats.vertices,
            stats.triangles,
            stats.liquid_groups,
            bytes.len()
        );
        return Ok(());
    }
    let bytes = read(&args[2], limits.max_raw_bytes)?;
    let model = if args[1] == "m2" {
        let raw =
            RawM2::parse(&bytes, forever_vmap::Limits::default()).map_err(|error| error.code())?;
        vmo::convert_m2(&raw, limits)
    } else if args[1] == "wmo" {
        let raw = wmo::raw::RawWmo::parse(&bytes, wmo::raw::ConversionLimits::default())
            .map_err(|error| error.code())?;
        vmo::convert_wmo(&raw, limits)
    } else {
        return Err("qa_usage");
    }
    .map_err(|error| error.code())?;
    vmo::View::parse(model.bytes(), limits).map_err(|error| error.code())?;
    std::io::stdout()
        .lock()
        .write_all(model.bytes())
        .map_err(|_| "qa_output")
}

fn main() {
    if let Err(code) = run() {
        eprintln!("{code}");
        std::process::exit(4);
    }
}
