// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: synthetic bounds/map assembly caller, no acquisition.
// Ordinary bounded file I/O; no source-identity or no-symlink guarantee.
// Map stdout is a QA transport bundle, never a production VMap file format.
use forever_vmap::{RawM2, assembly, placement, wmo};
use std::io::{Read, Write};

fn read(path: &std::ffi::OsStr, maximum: usize) -> Result<Vec<u8>, &'static str> {
    let file = std::fs::File::open(path).map_err(|_| "qa_input")?;
    if !file.metadata().map_err(|_| "qa_input")?.is_file() {
        return Err("qa_input");
    }
    let mut bytes = Vec::new();
    file.take(maximum as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "qa_input")?;
    if bytes.len() > maximum {
        return Err("qa_extent");
    }
    Ok(bytes)
}

fn u32_out(out: &mut impl Write, value: usize) -> Result<(), &'static str> {
    let word = u32::try_from(value).map_err(|_| "qa_extent")?;
    out.write_all(&word.to_le_bytes()).map_err(|_| "qa_output")
}

fn block(out: &mut impl Write, bytes: &[u8]) -> Result<(), &'static str> {
    u32_out(out, bytes.len())?;
    out.write_all(bytes).map_err(|_| "qa_output")
}

fn run() -> Result<(), &'static str> {
    let args: Vec<_> = std::env::args_os().take(6).collect();
    if args.len() != 4 {
        return Err("qa_usage");
    }
    if args[1] == "verify-tree" || args[1] == "verify-tile" || args[1] == "verify-index" {
        let expected: usize = args[2]
            .to_str()
            .ok_or("qa_count")?
            .parse()
            .map_err(|_| "qa_count")?;
        let bytes = read(&args[3], 512 * 1024 * 1024)?;
        let limits = assembly::Limits::default();
        if args[1] == "verify-tree" {
            assembly::Tree::parse(&bytes, expected, limits).map_err(|e| e.code())?;
        } else if args[1] == "verify-tile" {
            assembly::TileView::parse(&bytes, limits).map_err(|e| e.code())?;
        } else {
            assembly::IndexView::parse(&bytes, expected, limits).map_err(|e| e.code())?;
        }
        println!("{{\"status\":\"ok\",\"bytes\":{}}}", bytes.len());
        return Ok(());
    }
    let input = read(&args[3], placement::MAX_BYTES)?;
    let parsed =
        placement::DirBin::parse(&input, placement::Limits::default()).map_err(|e| e.code())?;
    if args[1] == "bounds" {
        if parsed.record_count() != 1 {
            return Err("qa_count");
        }
        let spawn = parsed.records().next().ok_or("qa_count")?;
        let model_bounds = if spawn.bounds_bits().is_some() {
            None
        } else {
            let raw_bytes = read(&args[2], wmo::raw::MAX_RAW_BYTES)?;
            let flags = raw_bytes.get(20..24).ok_or("qa_raw_header")?;
            if flags == 1u32.to_le_bytes() {
                let raw = RawM2::parse(&raw_bytes, forever_vmap::Limits::default())
                    .map_err(|e| e.code())?;
                Some(raw.bounds_bits())
            } else if flags == 0u32.to_le_bytes() {
                let raw =
                    wmo::raw::RawWmo::parse(&raw_bytes, wmo::raw::ConversionLimits::default())
                        .map_err(|e| e.code())?;
                Some(raw.groups().next().ok_or("qa_raw_groups")?.bounds_bits())
            } else {
                return Err("qa_raw_flags");
            }
        };
        let completed =
            placement::bounds::complete(spawn, model_bounds, placement::Limits::default())
                .map_err(|e| e.code())?;
        let bytes =
            placement::encode(&[completed], placement::Limits::default()).map_err(|e| e.code())?;
        return std::io::stdout()
            .lock()
            .write_all(&bytes)
            .map_err(|_| "qa_output");
    }
    if args[1] != "map" {
        return Err("qa_usage");
    }
    let map_id: u32 = args[2]
        .to_str()
        .ok_or("qa_map_id")?
        .parse()
        .map_err(|_| "qa_map_id")?;
    let mut records = Vec::new();
    records
        .try_reserve_exact(parsed.record_count())
        .map_err(|_| "qa_allocation")?;
    records.extend(parsed.records());
    let result =
        assembly::build(map_id, &records, assembly::Limits::default()).map_err(|e| e.code())?;
    let mut out = std::io::stdout().lock();
    out.write_all(b"RCEMAP01").map_err(|_| "qa_output")?;
    block(&mut out, result.tree_bytes())?;
    block(&mut out, result.ordered_dir_bin())?;
    u32_out(&mut out, result.tiles().len())?;
    for tile in result.tiles() {
        u32_out(&mut out, tile.x() as usize)?;
        u32_out(&mut out, tile.y() as usize)?;
        block(&mut out, tile.payload().unwrap_or(&[]))?;
        block(&mut out, tile.index_bytes())?;
    }
    Ok(())
}

fn main() {
    if let Err(code) = run() {
        eprintln!("{code}");
        std::process::exit(4);
    }
}
