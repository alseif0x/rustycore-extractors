// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: synthetic-only caller for the public MLIQ section API.
// No upstream implementation, client discovery, CASC or network access.
//! CLI: liquid_section <input-synthetic.bin> <output.bin>
//! Input: RCELS001, pack u8 (0|1), cap u32 LE, show[16384] u8,
//! entries[256] u16 LE, flags[256] u8, heights[16641] u32 LE bits.
//! Exactly 83729 bytes. Output is raw MLIQ, or an empty file for omission,
//! created exclusively after successful validation and complete serialization.

use forever_terrain::liquid_section::{LiquidSection, Options};
use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::Path,
};

const SHOW_COUNT: usize = 16384;
const CELL_COUNT: usize = 256;
const HEIGHT_COUNT: usize = 16641;
const INPUT_BYTES: usize = 13 + SHOW_COUNT + CELL_COUNT * 3 + HEIGHT_COUNT * 4;
const ENTRY_START: usize = 13 + SHOW_COUNT;
const FLAG_START: usize = ENTRY_START + CELL_COUNT * 2;
const HEIGHT_START: usize = FLAG_START + CELL_COUNT;

fn read_input(path: &Path) -> Result<Vec<u8>, &'static str> {
    let mut file = File::open(path).map_err(|_| "qa_input_open")?;
    let metadata = file.metadata().map_err(|_| "qa_input_metadata")?;
    if !metadata.is_file() {
        return Err("qa_input_not_file");
    }
    if metadata.len() != INPUT_BYTES as u64 {
        return Err("qa_input_extent");
    }
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(INPUT_BYTES)
        .map_err(|_| "qa_allocation_limit")?;
    bytes.resize(INPUT_BYTES, 0);
    file.read_exact(&mut bytes).map_err(|_| "qa_input_read")?;
    if file.read(&mut [0]).map_err(|_| "qa_input_read")? != 0 {
        return Err("qa_input_read");
    }
    Ok(bytes)
}

fn parse(bytes: &[u8]) -> Result<Option<LiquidSection>, &'static str> {
    if bytes.len() != INPUT_BYTES {
        return Err("qa_input_extent");
    }
    if &bytes[..8] != b"RCELS001" {
        return Err("qa_input_header");
    }
    let pack_integers = match bytes[8] {
        0 => false,
        1 => true,
        _ => return Err("qa_input_option"),
    };
    let cap = u32::from_le_bytes(bytes[9..13].try_into().expect("fixed input header"));
    let entries: [u16; CELL_COUNT] = std::array::from_fn(|index| {
        let at = ENTRY_START + index * 2;
        u16::from_le_bytes([bytes[at], bytes[at + 1]])
    });
    let heights: [u32; HEIGHT_COUNT] = std::array::from_fn(|index| {
        let at = HEIGHT_START + index * 4;
        u32::from_le_bytes(bytes[at..at + 4].try_into().expect("fixed height extent"))
    });
    LiquidSection::from_arrays(
        &bytes[13..ENTRY_START],
        &entries,
        &bytes[FLAG_START..HEIGHT_START],
        &heights,
        Options { pack_integers },
        usize::try_from(cap).map_err(|_| "liquid_section_policy_limit")?,
    )
    .map_err(|error| error.code())
}

fn run(input: &Path, output: &Path) -> Result<(bool, usize), &'static str> {
    let input = read_input(input)?;
    let section = parse(&input)?;
    let bytes = section.as_ref().map_or(&[][..], LiquidSection::bytes);
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
        .map_err(|_| "qa_output_create")?;
    file.write_all(bytes).map_err(|_| "qa_output_write")?;
    file.sync_all().map_err(|_| "qa_output_write")?;
    Ok((section.is_none(), bytes.len()))
}

fn main() {
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    if args.len() != 2 {
        println!("{{\"status\":\"error\",\"code\":\"qa_usage\"}}");
        std::process::exit(2);
    }
    match run(Path::new(&args[0]), Path::new(&args[1])) {
        Ok((omitted, bytes)) => {
            let status = if omitted { "omit" } else { "ok" };
            println!("{{\"status\":\"{status}\",\"bytes\":{bytes}}}");
        }
        Err(code) => {
            println!("{{\"status\":\"error\",\"code\":\"{code}\"}}");
            std::process::exit(4);
        }
    }
}
