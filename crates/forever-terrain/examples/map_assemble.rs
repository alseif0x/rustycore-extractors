// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original synthetic-only caller for typed MAPS assembly.
// No upstream implementation, client discovery, acquisition or network access.
//! CLI: map_assemble <input-synthetic.bin> <output.bin>
//! Envelope: RCEMAP01, build u32 LE, cap u32 LE, options u8, 256 AREA u16 LE,
//! height-fixture length u32 LE and bytes, optionally a liquid-fixture length
//! u32 LE and bytes, optionally 2048 hole bytes; exact EOF, <=256 KiB.
//! Options: bit0 includes a liquid fixture; bit1 includes holes (even all-zero).
//! Height fixture: RCEHGT01, flags u8 (bit0 pack, bit1 flight), cap u32 LE,
//! V9[16641] then V8[16384] u32 LE height bits, optional 36 flight-bound bytes.
//! Liquid fixture: exact 83729-byte RCELS001 format from liquid_section.rs.
//! Output is raw MAPS, exclusively created after constructors and verification
//! succeed. Section comparison does not establish native full-ADT acceptance.

use forever_terrain::{
    height::{self, FlightBounds, HeightSection},
    liquid_section::{self, LiquidSection},
    maps::{self, AreaSection, MapSections},
};
use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::Path,
};

const INPUT_LIMIT: usize = 256 * 1024;
const CELL_COUNT: usize = 256;
const HEIGHT_SAMPLES: usize = height::V9_COUNT + height::V8_COUNT;
const HEIGHT_INPUT_BASE: usize = 13 + HEIGHT_SAMPLES * 4;
const LIQUID_SHOW: usize = 16384;
const LIQUID_HEIGHTS: usize = 16641;
const LIQUID_ENTRY_START: usize = 13 + LIQUID_SHOW;
const LIQUID_FLAG_START: usize = LIQUID_ENTRY_START + CELL_COUNT * 2;
const LIQUID_HEIGHT_START: usize = LIQUID_FLAG_START + CELL_COUNT;
const LIQUID_INPUT_BYTES: usize = LIQUID_HEIGHT_START + LIQUID_HEIGHTS * 4;

struct Cursor<'a> {
    bytes: &'a [u8],
    at: usize,
}
impl<'a> Cursor<'a> {
    fn take(&mut self, count: usize) -> Result<&'a [u8], &'static str> {
        let end = self.at.checked_add(count).ok_or("qa_input_extent")?;
        let out = self.bytes.get(self.at..end).ok_or("qa_input_extent")?;
        self.at = end;
        Ok(out)
    }
    fn word(&mut self) -> Result<u32, &'static str> {
        Ok(word(self.take(4)?))
    }
    fn framed(&mut self) -> Result<&'a [u8], &'static str> {
        let size = usize::try_from(self.word()?).map_err(|_| "qa_input_extent")?;
        self.take(size)
    }
    fn finish(self) -> Result<(), &'static str> {
        if self.at == self.bytes.len() {
            Ok(())
        } else {
            Err("qa_input_extent")
        }
    }
}

fn read_input(path: &Path) -> Result<Vec<u8>, &'static str> {
    let mut file = File::open(path).map_err(|_| "qa_input_open")?;
    let metadata = file.metadata().map_err(|_| "qa_input_metadata")?;
    if !metadata.is_file() {
        return Err("qa_input_not_file");
    }
    let size = usize::try_from(metadata.len()).map_err(|_| "qa_input_limit")?;
    if size > INPUT_LIMIT {
        return Err("qa_input_limit");
    }
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(size)
        .map_err(|_| "qa_allocation_limit")?;
    bytes.resize(size, 0);
    file.read_exact(&mut bytes).map_err(|_| "qa_input_read")?;
    if file.read(&mut [0]).map_err(|_| "qa_input_read")? != 0 {
        return Err("qa_input_read");
    }
    Ok(bytes)
}

fn word(bytes: &[u8]) -> u32 {
    u32::from_le_bytes(bytes.try_into().expect("checked four-byte extent"))
}

fn sample_words(bytes: &[u8]) -> Result<Vec<u32>, &'static str> {
    let (chunks, tail) = bytes.as_chunks::<4>();
    if !tail.is_empty() {
        return Err("qa_input_extent");
    }
    let mut out = Vec::new();
    out.try_reserve_exact(chunks.len())
        .map_err(|_| "qa_allocation_limit")?;
    for chunk in chunks {
        out.push(u32::from_le_bytes(*chunk));
    }
    Ok(out)
}

fn height_fixture(bytes: &[u8]) -> Result<HeightSection, &'static str> {
    if bytes.len() < 13 {
        return Err("qa_height_extent");
    }
    if &bytes[..8] != b"RCEHGT01" {
        return Err("qa_height_header");
    }
    let flags = bytes[8];
    if flags & !3 != 0 {
        return Err("qa_height_option");
    }
    let has_flight = flags & 2 != 0;
    let expected = HEIGHT_INPUT_BASE + usize::from(has_flight) * height::FLIGHT_BOUNDS_BYTES;
    if bytes.len() != expected {
        return Err("qa_height_extent");
    }
    let cap = usize::try_from(word(&bytes[9..13])).map_err(|_| "height_policy_limit")?;
    let samples = sample_words(&bytes[13..HEIGHT_INPUT_BASE])?;
    let flight = if has_flight {
        Some(FlightBounds::parse(&bytes[HEIGHT_INPUT_BASE..]).map_err(|error| error.code())?)
    } else {
        None
    };
    HeightSection::from_bits(
        &samples[..height::V9_COUNT],
        &samples[height::V9_COUNT..],
        height::Options {
            pack_integers: flags & 1 != 0,
        },
        flight.as_ref(),
        cap,
    )
    .map_err(|error| error.code())
}

fn liquid_fixture(bytes: &[u8]) -> Result<Option<LiquidSection>, &'static str> {
    if bytes.len() != LIQUID_INPUT_BYTES {
        return Err("qa_liquid_extent");
    }
    if &bytes[..8] != b"RCELS001" {
        return Err("qa_liquid_header");
    }
    if bytes[8] > 1 {
        return Err("qa_liquid_option");
    }
    let cap = usize::try_from(word(&bytes[9..13])).map_err(|_| "liquid_section_policy_limit")?;
    let entries: [u16; CELL_COUNT] = std::array::from_fn(|index| {
        let at = LIQUID_ENTRY_START + index * 2;
        u16::from_le_bytes([bytes[at], bytes[at + 1]])
    });
    let heights = sample_words(&bytes[LIQUID_HEIGHT_START..])?;
    LiquidSection::from_arrays(
        &bytes[13..LIQUID_ENTRY_START],
        &entries,
        &bytes[LIQUID_FLAG_START..LIQUID_HEIGHT_START],
        &heights,
        liquid_section::Options {
            pack_integers: bytes[8] != 0,
        },
        cap,
    )
    .map_err(|error| error.code())
}

fn assemble_fixture(bytes: &[u8]) -> Result<Vec<u8>, &'static str> {
    let mut cursor = Cursor { bytes, at: 0 };
    if cursor.take(8)? != b"RCEMAP01" {
        return Err("qa_input_header");
    }
    let build = cursor.word()?;
    let cap = usize::try_from(cursor.word()?).map_err(|_| "maps_policy_limit")?;
    let options = cursor.take(1)?[0];
    if options & !3 != 0 {
        return Err("qa_input_option");
    }
    let area_bytes = cursor.take(CELL_COUNT * 2)?;
    let height_bytes = cursor.framed()?;
    let liquid_bytes = if options & 1 != 0 {
        Some(cursor.framed()?)
    } else {
        None
    };
    let holes: Option<&[u8; maps::HOLE_BYTES]> = if options & 2 != 0 {
        Some(
            cursor
                .take(maps::HOLE_BYTES)?
                .try_into()
                .expect("checked hole extent"),
        )
    } else {
        None
    };
    // Validate the whole envelope before running any section constructor.
    cursor.finish()?;
    let area_ids: [u16; CELL_COUNT] = std::array::from_fn(|index| {
        u16::from_le_bytes([area_bytes[index * 2], area_bytes[index * 2 + 1]])
    });
    let area = AreaSection::from_ids(&area_ids);
    let height = height_fixture(height_bytes)?;
    let liquid = liquid_bytes.map(liquid_fixture).transpose()?.flatten();
    let out = maps::assemble(
        build,
        MapSections {
            area: &area,
            height: &height,
            liquid: liquid.as_ref(),
            holes,
        },
        cap,
    )
    .map_err(|error| error.code())?;
    maps::verify(&out, build, cap).map_err(|error| error.code())?;
    Ok(out)
}

fn run(input: &Path, output: &Path) -> Result<usize, &'static str> {
    let input = read_input(input)?;
    let bytes = assemble_fixture(&input)?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
        .map_err(|_| "qa_output_create")?;
    file.write_all(&bytes).map_err(|_| "qa_output_write")?;
    file.sync_all().map_err(|_| "qa_output_write")?;
    Ok(bytes.len())
}

fn main() {
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    if args.len() != 2 {
        println!("{{\"status\":\"error\",\"code\":\"qa_usage\"}}");
        std::process::exit(2);
    }
    match run(Path::new(&args[0]), Path::new(&args[1])) {
        Ok(bytes) => println!("{{\"status\":\"ok\",\"bytes\":{bytes}}}"),
        Err(code) => {
            println!("{{\"status\":\"error\",\"code\":\"{code}\"}}");
            std::process::exit(4);
        }
    }
}
