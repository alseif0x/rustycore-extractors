// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: synthetic-only caller for the public liquid projection API.
// No client discovery, CASC access, network or upstream implementation is used.
//! CLI: liquid_projection <adt-synthetic.bin> <lookup-synthetic.bin> <0|1> <output.bin>
//! Lookup: RCELIQ01, u32 LE type/material counts, then six-byte type records
//! (u32 ID, u8 sound bank, u8 material ID) and nine-byte material records
//! (u32 ID, u32 flags, i8 LVF). At most 256 each, last duplicate ID wins.
//! Output: RCELQP01, 16384 show bytes, 256 u16 LE entries, 256 flag bytes,
//! 16641 u32 LE height bits. Output is created exclusively after successful parsing.

use forever_terrain::{
    Adt, Limits,
    liquid::{LiquidLookup, LiquidMaterial, LiquidType, project_liquids},
};
use std::{
    collections::BTreeMap,
    ffi::OsStr,
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::Path,
};

const ADT_LIMIT: usize = 16 * 1024 * 1024;
const LOOKUP_LIMIT: usize = 16 + 256 * 6 + 256 * 9;
const OUTPUT_BYTES: usize = 8 + 16384 + 256 * 2 + 256 + 16641 * 4;

struct Lookup {
    types: BTreeMap<u32, LiquidType>,
    materials: BTreeMap<u32, LiquidMaterial>,
}

impl LiquidLookup for Lookup {
    fn liquid_type(&self, id: u32) -> Option<LiquidType> {
        self.types.get(&id).copied()
    }

    fn material(&self, id: u32) -> Option<LiquidMaterial> {
        self.materials.get(&id).copied()
    }
}

fn read_bounded(path: &Path, limit: usize) -> Result<Vec<u8>, &'static str> {
    let mut input = File::open(path).map_err(|_| "qa_input_open")?;
    let metadata = input.metadata().map_err(|_| "qa_input_metadata")?;
    if !metadata.is_file() {
        return Err("qa_input_not_file");
    }
    let size = usize::try_from(metadata.len()).map_err(|_| "qa_input_limit")?;
    if size > limit {
        return Err("qa_input_limit");
    }
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(size)
        .map_err(|_| "qa_input_limit")?;
    bytes.resize(size, 0);
    input.read_exact(&mut bytes).map_err(|_| "qa_input_read")?;
    if input.read(&mut [0]).map_err(|_| "qa_input_read")? != 0 {
        return Err("qa_input_read");
    }
    Ok(bytes)
}

fn word(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(
        bytes[offset..offset + 4]
            .try_into()
            .expect("checked lookup extent"),
    )
}

fn parse_lookup(bytes: &[u8]) -> Result<Lookup, &'static str> {
    if bytes.len() < 16 || &bytes[..8] != b"RCELIQ01" {
        return Err("qa_lookup_header");
    }
    let types = word(bytes, 8) as usize;
    let materials = word(bytes, 12) as usize;
    if types > 256 || materials > 256 {
        return Err("qa_lookup_limit");
    }
    if bytes.len() != 16 + types * 6 + materials * 9 {
        return Err("qa_lookup_extent");
    }
    let mut lookup = Lookup {
        types: BTreeMap::new(),
        materials: BTreeMap::new(),
    };
    for record in bytes[16..16 + types * 6].as_chunks::<6>().0 {
        lookup.types.insert(
            word(record, 0),
            LiquidType {
                sound_bank: record[4],
                material_id: record[5],
            },
        );
    }
    for record in bytes[16 + types * 6..].as_chunks::<9>().0 {
        lookup.materials.insert(
            word(record, 0),
            LiquidMaterial {
                flags: word(record, 4),
                vertex_format: record[8] as i8,
            },
        );
    }
    Ok(lookup)
}

fn run(
    adt_path: &Path,
    lookup_path: &Path,
    ignore: bool,
    output: &Path,
) -> Result<(), &'static str> {
    let adt_bytes = read_bounded(adt_path, ADT_LIMIT)?;
    let lookup = parse_lookup(&read_bounded(lookup_path, LOOKUP_LIMIT)?)?;
    let adt = Adt::parse(&adt_bytes, Limits::default()).map_err(|error| error.code())?;
    let result = project_liquids(&adt, &lookup, ignore).map_err(|error| error.code())?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(OUTPUT_BYTES)
        .map_err(|_| "qa_output_limit")?;
    bytes.extend_from_slice(b"RCELQP01");
    bytes.extend_from_slice(result.show());
    for value in result.entries() {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes.extend_from_slice(result.flags());
    for bits in result.height_bits() {
        bytes.extend_from_slice(&bits.to_le_bytes());
    }
    if bytes.len() != OUTPUT_BYTES {
        return Err("qa_output_shape");
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
        .map_err(|_| "qa_output_create")?;
    file.write_all(&bytes).map_err(|_| "qa_output_write")?;
    file.sync_all().map_err(|_| "qa_output_write")?;
    Ok(())
}

fn main() {
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    let ignore = match args.get(2).map(|value| value.as_os_str()) {
        Some(value) if value == OsStr::new("0") => false,
        Some(value) if value == OsStr::new("1") => true,
        _ => {
            println!("{{\"status\":\"error\",\"code\":\"qa_usage\"}}");
            std::process::exit(2);
        }
    };
    if args.len() != 4 {
        println!("{{\"status\":\"error\",\"code\":\"qa_usage\"}}");
        std::process::exit(2);
    }
    match run(
        Path::new(&args[0]),
        Path::new(&args[1]),
        ignore,
        Path::new(&args[3]),
    ) {
        Ok(()) => println!(
            "{{\"status\":\"ok\",\"show\":16384,\"entries\":256,\"flags\":256,\"heights\":16641,\"bytes\":83724}}"
        ),
        Err(code) => {
            println!("{{\"status\":\"error\",\"code\":\"{code}\"}}");
            std::process::exit(4);
        }
    }
}
