// SPDX-License-Identifier: GPL-3.0-or-later
//! Private local diagnostic: terrain_sources <Map|VMap|LiquidType|LiquidMaterial>
//! <input.db2> <output.json>. No CASC, discovery or network.
//!
//! The output contains complete client-derived projections and must stay in
//! the private external run directory, never Git, reports or model context.
//! Stdout contains only status, counts and static codes. All decoding/replay
//! completes before create_new; malformed inputs never create an output file.
//! An output I/O failure may leave a partial diagnostic file; it is never
//! reported as successful and is not replaced or silently removed.
use forever_db2::{
    RecordError,
    terrain_sources::{
        self, DirectRecord, LiquidMaterialFields, LiquidTypeFields, MapFields, RecordCopy,
        VMapFields,
    },
};
use std::{
    collections::BTreeMap,
    fs::{File, Metadata, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    process::ExitCode,
};

const MAX_INPUT_BYTES: u64 = 16 * 1024 * 1024;

enum Table {
    Map,
    VMap,
    LiquidType,
    LiquidMaterial,
}
struct Options {
    table: Table,
    input: PathBuf,
    output: PathBuf,
}
enum Dump {
    VMap {
        direct: Vec<DirectRecord<VMapFields>>,
        copies: Vec<RecordCopy>,
        materialized: Vec<DirectRecord<VMapFields>>,
    },
    Map {
        direct: Vec<DirectRecord<MapFields>>,
        copies: Vec<RecordCopy>,
        materialized: Vec<DirectRecord<MapFields>>,
    },
    LiquidType {
        direct: Vec<DirectRecord<LiquidTypeFields>>,
        copies: Vec<RecordCopy>,
        materialized: BTreeMap<u32, LiquidTypeFields>,
    },
    LiquidMaterial {
        direct: Vec<DirectRecord<LiquidMaterialFields>>,
        copies: Vec<RecordCopy>,
        materialized: BTreeMap<u32, LiquidMaterialFields>,
    },
}
fn options() -> Result<Options, &'static str> {
    let mut args = std::env::args_os().skip(1);
    let table = match args.next().as_deref().and_then(std::ffi::OsStr::to_str) {
        Some("Map") => Table::Map,
        Some("VMap") => Table::VMap,
        Some("LiquidType") => Table::LiquidType,
        Some("LiquidMaterial") => Table::LiquidMaterial,
        _ => return Err("invalid_invocation"),
    };
    let input = args.next().ok_or("invalid_invocation")?.into();
    let output = args.next().ok_or("invalid_invocation")?.into();
    if args.next().is_some() {
        return Err("invalid_invocation");
    }
    Ok(Options {
        table,
        input,
        output,
    })
}
fn main() -> ExitCode {
    let result = options().and_then(|options| run(&options));
    let (message, exit) = match result {
        Ok((direct, copies, materialized)) => (
            format!(
                "{{\"status\":\"success\",\"direct_records\":{direct},\"copy_records\":{copies},\"materialized_records\":{materialized}}}"
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
    ExitCode::from(exit)
}
fn read_input(path: &Path) -> Result<Vec<u8>, &'static str> {
    let path_before = std::fs::symlink_metadata(path).map_err(|_| "input_io")?;
    if !path_before.is_file() {
        return Err("input_not_regular");
    }
    let mut file = File::open(path).map_err(|_| "input_io")?;
    let before = file.metadata().map_err(|_| "input_io")?;
    if !before.is_file() || !same_file(&path_before, &before) {
        return Err("input_changed");
    }
    if before.len() > MAX_INPUT_BYTES {
        return Err(RecordError::Limit.code());
    }
    let mut data = Vec::new();
    data.try_reserve_exact(before.len() as usize)
        .map_err(|_| RecordError::Limit.code())?;
    (&mut file)
        .take(MAX_INPUT_BYTES + 1)
        .read_to_end(&mut data)
        .map_err(|_| "input_io")?;
    let after = file.metadata().map_err(|_| "input_io")?;
    let path_after = std::fs::symlink_metadata(path).map_err(|_| "input_changed")?;
    if !same_file(&before, &after)
        || !same_file(&after, &path_after)
        || data.len() as u64 != before.len()
    {
        return Err("input_changed");
    }
    if data.len() as u64 > MAX_INPUT_BYTES {
        return Err(RecordError::Limit.code());
    }
    Ok(data)
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
fn cloned<T: Clone>(input: &[T]) -> Result<Vec<T>, &'static str> {
    let mut output = Vec::new();
    output
        .try_reserve_exact(input.len())
        .map_err(|_| RecordError::Limit.code())?;
    output.extend_from_slice(input);
    Ok(output)
}
fn decode(data: &[u8], table: &Table) -> Result<Dump, &'static str> {
    Ok(match table {
        Table::VMap => {
            let source = terrain_sources::read_vmap_maps(data).map_err(RecordError::code)?;
            let direct = cloned(source.direct_records())?;
            let copies = cloned(source.copies())?;
            let materialized = source.materialize().map_err(RecordError::code)?;
            Dump::VMap {
                direct,
                copies,
                materialized,
            }
        }
        Table::Map => {
            let source = terrain_sources::read_map(data).map_err(RecordError::code)?;
            let direct = cloned(source.direct_records())?;
            let copies = cloned(source.copies())?;
            let materialized = source.materialize().map_err(RecordError::code)?;
            Dump::Map {
                direct,
                copies,
                materialized,
            }
        }
        Table::LiquidType => {
            let source = terrain_sources::read_liquid_types(data).map_err(RecordError::code)?;
            let direct = cloned(source.direct_records())?;
            let copies = cloned(source.copies())?;
            let materialized = source.materialize().map_err(RecordError::code)?;
            Dump::LiquidType {
                direct,
                copies,
                materialized,
            }
        }
        Table::LiquidMaterial => {
            let source = terrain_sources::read_liquid_materials(data).map_err(RecordError::code)?;
            let direct = cloned(source.direct_records())?;
            let copies = cloned(source.copies())?;
            let materialized = source.materialize().map_err(RecordError::code)?;
            Dump::LiquidMaterial {
                direct,
                copies,
                materialized,
            }
        }
    })
}
fn run(options: &Options) -> Result<(usize, usize, usize), &'static str> {
    let data = read_input(&options.input)?;
    let dump = decode(&data, &options.table)?;
    let mut open = OpenOptions::new();
    open.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        open.mode(0o600);
    }
    let file = open.open(&options.output).map_err(|_| "output_io")?;
    let mut out = io::BufWriter::new(file);
    let counts = match &dump {
        Dump::VMap {
            direct,
            copies,
            materialized,
        } => {
            document(
                &mut out,
                "VMap",
                direct,
                copies,
                materialized.iter().map(|row| (row.id, &row.fields)),
            )
            .map_err(|_| "output_io")?;
            (direct.len(), copies.len(), materialized.len())
        }
        Dump::Map {
            direct,
            copies,
            materialized,
        } => {
            document(
                &mut out,
                "Map",
                direct,
                copies,
                materialized.iter().map(|row| (row.id, &row.fields)),
            )
            .map_err(|_| "output_io")?;
            (direct.len(), copies.len(), materialized.len())
        }
        Dump::LiquidType {
            direct,
            copies,
            materialized,
        } => {
            document(
                &mut out,
                "LiquidType",
                direct,
                copies,
                materialized.iter().map(|(&id, fields)| (id, fields)),
            )
            .map_err(|_| "output_io")?;
            (direct.len(), copies.len(), materialized.len())
        }
        Dump::LiquidMaterial {
            direct,
            copies,
            materialized,
        } => {
            document(
                &mut out,
                "LiquidMaterial",
                direct,
                copies,
                materialized.iter().map(|(&id, fields)| (id, fields)),
            )
            .map_err(|_| "output_io")?;
            (direct.len(), copies.len(), materialized.len())
        }
    };
    out.flush().map_err(|_| "output_io")?;
    Ok(counts)
}
trait Fields {
    fn write_json(&self, out: &mut impl Write) -> io::Result<()>;
}
impl Fields for MapFields {
    fn write_json(&self, out: &mut impl Write) -> io::Result<()> {
        write!(out, ",{},", self.wdt_file_data_id)?;
        string(out, &self.name)?;
        out.write_all(b",")?;
        string(out, &self.directory)
    }
}
impl Fields for VMapFields {
    fn write_json(&self, out: &mut impl Write) -> io::Result<()> {
        write!(out, ",{},{},", self.wdt_file_data_id, self.parent_map_id)?;
        string(out, &self.name)?;
        out.write_all(b",")?;
        string(out, &self.directory)
    }
}
impl Fields for LiquidTypeFields {
    fn write_json(&self, out: &mut impl Write) -> io::Result<()> {
        write!(out, ",{},{}", self.sound_bank, self.material_id)
    }
}
impl Fields for LiquidMaterialFields {
    fn write_json(&self, out: &mut impl Write) -> io::Result<()> {
        write!(out, ",{},{}", self.flags_bits, self.lvf)
    }
}
fn document<'a, T: Fields + 'a>(
    out: &mut impl Write,
    table: &str,
    direct: &[DirectRecord<T>],
    copies: &[RecordCopy],
    materialized: impl Iterator<Item = (u32, &'a T)>,
) -> io::Result<()> {
    out.write_all(b"{\"table\":")?;
    string(out, table)?;
    out.write_all(b",\"direct\":[")?;
    rows(out, direct.iter().map(|row| (row.id, &row.fields)))?;
    out.write_all(b"],\"copies\":[")?;
    for (index, copy) in copies.iter().enumerate() {
        if index != 0 {
            out.write_all(b",")?;
        }
        write!(out, "[{},{}]", copy.new_id, copy.source_id)?;
    }
    out.write_all(b"],\"materialized\":[")?;
    rows(out, materialized)?;
    out.write_all(b"]}\n")
}
fn rows<'a, T: Fields + 'a>(
    out: &mut impl Write,
    rows: impl Iterator<Item = (u32, &'a T)>,
) -> io::Result<()> {
    for (index, (id, fields)) in rows.enumerate() {
        if index != 0 {
            out.write_all(b",")?;
        }
        write!(out, "[{id}")?;
        fields.write_json(out)?;
        out.write_all(b"]")?;
    }
    Ok(())
}
fn string(out: &mut impl Write, value: &str) -> io::Result<()> {
    out.write_all(b"\"")?;
    for c in value.chars() {
        match c {
            '"' => out.write_all(b"\\\"")?,
            '\\' => out.write_all(b"\\\\")?,
            '\n' => out.write_all(b"\\n")?,
            '\r' => out.write_all(b"\\r")?,
            '\t' => out.write_all(b"\\t")?,
            c if c <= '\u{1f}' => write!(out, "\\u{:04x}", u32::from(c))?,
            c => {
                let mut buffer = [0; 4];
                out.write_all(c.encode_utf8(&mut buffer).as_bytes())?;
            }
        }
    }
    out.write_all(b"\"")
}
