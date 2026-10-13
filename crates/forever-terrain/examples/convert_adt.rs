// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original fixture caller of the public conversion API.
//! CLI: convert_adt <adt.bin> <lookup.bin> <context.bin> <output.map>
//! Lookup ABI is RCELIQ01, LE u32 type/material counts, six-byte type records
//! {u32 id,u8 sound,u8 material}, nine-byte material records {u32 id,u32 flags,
//! i8 LVF}. At most 256 each, last duplicate wins, exact EOF; max3856 bytes.
//! Context is exactly31 bytes: RCECONV1, u32 map_id, u8 tile_y, u8 tile_x,
//! u32 build, u8 pack(0|1), u32 max_input_bytes, u32 max_chunks,
//! u32 max_output_bytes. All integers use little endian, no padding.
//! ADT is bounded to16MiB. Output is exclusive and synced only after the
//! public conversion API has returned a complete verified MAPS object.
//! Status exposes counts/booleans/static codes, never paths or payloads.
//! No discovery, network, CASC, DB2 loading or engine integration occurs.

use forever_terrain::{
    conversion::{self, Context, Limits},
    liquid::{LiquidLookup, LiquidMaterial, LiquidType},
};
use std::{
    collections::BTreeMap,
    fs::{File, Metadata, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    process::ExitCode,
};

const ADT_BYTES: usize = 16 * 1024 * 1024;
const LOOKUP_BYTES: usize = 3856;
const CONTEXT_BYTES: usize = 31;

struct Paths {
    adt: PathBuf,
    lookup: PathBuf,
    context: PathBuf,
    output: PathBuf,
}
fn paths() -> Result<Paths, &'static str> {
    let mut args = std::env::args_os().skip(1);
    let result = Paths {
        adt: args.next().ok_or("qa_usage")?.into(),
        lookup: args.next().ok_or("qa_usage")?.into(),
        context: args.next().ok_or("qa_usage")?.into(),
        output: args.next().ok_or("qa_usage")?.into(),
    };
    if args.next().is_some() {
        return Err("qa_usage");
    }
    Ok(result)
}
fn same_file(left: &Metadata, right: &Metadata) -> bool {
    let same = left.is_file()
        && right.is_file()
        && left.len() == right.len()
        && left.modified().ok() == right.modified().ok();
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        same && left.dev() == right.dev()
            && left.ino() == right.ino()
            && left.ctime() == right.ctime()
            && left.ctime_nsec() == right.ctime_nsec()
    }
    #[cfg(not(unix))]
    {
        same
    }
}
fn read_bounded(path: &Path, limit: usize) -> Result<Vec<u8>, &'static str> {
    let named = std::fs::symlink_metadata(path).map_err(|_| "qa_input_metadata")?;
    if !named.is_file() {
        return Err("qa_input_not_regular");
    }
    let mut input = File::open(path).map_err(|_| "qa_input_open")?;
    let before = input.metadata().map_err(|_| "qa_input_metadata")?;
    if !same_file(&named, &before) {
        return Err("qa_input_changed");
    }
    let size = usize::try_from(before.len()).map_err(|_| "qa_input_limit")?;
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
        return Err("qa_input_changed");
    }
    let after = input.metadata().map_err(|_| "qa_input_metadata")?;
    let named = std::fs::symlink_metadata(path).map_err(|_| "qa_input_changed")?;
    if !same_file(&before, &after) || !same_file(&after, &named) {
        return Err("qa_input_changed");
    }
    Ok(bytes)
}
fn word(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(
        bytes[at..at + 4]
            .try_into()
            .expect("validated fixture extent"),
    )
}
fn parse_context(bytes: &[u8]) -> Result<(Context, Limits), &'static str> {
    if bytes.len() != CONTEXT_BYTES || &bytes[..8] != b"RCECONV1" {
        return Err("qa_context_header");
    }
    let pack = match bytes[18] {
        0 => false,
        1 => true,
        _ => return Err("qa_context_pack"),
    };
    Ok((
        Context {
            map_id: word(bytes, 8),
            tile_y: bytes[12],
            tile_x: bytes[13],
            build: word(bytes, 14),
            pack,
        },
        Limits {
            max_input_bytes: word(bytes, 19) as usize,
            max_chunks: word(bytes, 23) as usize,
            max_output_bytes: word(bytes, 27) as usize,
        },
    ))
}
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
fn run(paths: Paths) -> Result<String, &'static str> {
    let (context, limits) = parse_context(&read_bounded(&paths.context, CONTEXT_BYTES)?)?;
    let lookup = parse_lookup(&read_bounded(&paths.lookup, LOOKUP_BYTES)?)?;
    let input = read_bounded(&paths.adt, ADT_BYTES)?;
    let result =
        conversion::convert_adt(&input, &lookup, context, limits).map_err(|error| error.code())?;
    let mut open = OpenOptions::new();
    open.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        open.mode(0o600);
    }
    let mut output = open.open(&paths.output).map_err(|_| "qa_output_create")?;
    output
        .write_all(result.bytes())
        .map_err(|_| "qa_output_write")?;
    output.sync_all().map_err(|_| "qa_output_sync")?;
    // An output I/O failure may leave an incomplete file. It is never
    // reported successful, removed or replaced by this diagnostic caller.
    let stats = result.stats();
    Ok(format!(
        "{{\"status\":\"ok\",\"bytes\":{},\"visited_cells\":{},\"unique_cells\":{},\"missing_mcvt\":{},\"has_flight_bounds\":{},\"has_liquid\":{},\"has_holes\":{},\"ignore_deep_water\":{}}}",
        result.bytes().len(),
        stats.visited_cells,
        stats.unique_cells,
        stats.missing_mcvt,
        stats.has_flight_bounds,
        stats.has_liquid,
        stats.has_holes,
        stats.ignore_deep_water,
    ))
}
fn main() -> ExitCode {
    let (message, exit) = match paths().and_then(run) {
        Ok(status) => (status, 0),
        Err(code) => (
            format!("{{\"status\":\"error\",\"code\":\"{code}\"}}"),
            if code == "qa_usage" { 2 } else { 4 },
        ),
    };
    if writeln!(io::stdout().lock(), "{message}").is_err() {
        return ExitCode::from(4);
    }
    ExitCode::from(exit)
}
