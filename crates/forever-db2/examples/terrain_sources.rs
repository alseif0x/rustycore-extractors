// SPDX-License-Identifier: GPL-3.0-or-later
//! Private local diagnostic: terrain_sources <Map|VMap|LiquidType|LiquidMaterial|LiquidObject>
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
        self, DirectRecord, LiquidMaterialFields, LiquidObjectFields, LiquidTypeFields, MapFields,
        RecordCopy, VMapFields,
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
    LiquidObject,
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
    LiquidObject {
        direct: Vec<DirectRecord<LiquidObjectFields>>,
        copies: Vec<RecordCopy>,
        materialized: BTreeMap<u32, LiquidObjectFields>,
    },
}
fn options() -> Result<Options, &'static str> {
    parse_options(std::env::args_os().skip(1))
}
fn parse_options(
    mut args: impl Iterator<Item = std::ffi::OsString>,
) -> Result<Options, &'static str> {
    let table = match args.next().as_deref().and_then(std::ffi::OsStr::to_str) {
        Some("Map") => Table::Map,
        Some("VMap") => Table::VMap,
        Some("LiquidType") => Table::LiquidType,
        Some("LiquidMaterial") => Table::LiquidMaterial,
        Some("LiquidObject") => Table::LiquidObject,
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
        Table::LiquidObject => {
            let source = terrain_sources::read_liquid_objects(data).map_err(RecordError::code)?;
            let direct = cloned(source.direct_records())?;
            let copies = cloned(source.copies())?;
            let materialized = source.materialize().map_err(RecordError::code)?;
            Dump::LiquidObject {
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
        Dump::LiquidObject {
            direct,
            copies,
            materialized,
        } => {
            document(
                &mut out,
                "LiquidObject",
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
impl Fields for LiquidObjectFields {
    fn write_json(&self, out: &mut impl Write) -> io::Result<()> {
        write!(out, ",{}", self.liquid_type_id)
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        ffi::OsString,
        fs,
        sync::atomic::{AtomicU64, Ordering},
    };

    // Literal authored regular WDC5, independent of schema-generation helpers.
    // Physical fields: float32, float32, signed16, unsigned8, unsigned8.
    fn liquid_object(rows: &[(u32, i16)], copies: &[(u32, u32)]) -> Vec<u8> {
        let mut bytes = vec![0; 384];
        bytes[..4].copy_from_slice(b"WDC5");
        for (at, value) in [
            (4, 5),
            (136, rows.len() as u32),
            (140, 5),
            (144, 12),
            (156, 0xCB0D_39E8),
            (164, u32::MAX),
            (176, 5),
            (188, 120),
            (200, 1),
            (212, 384),
            (216, rows.len() as u32),
            (228, rows.len() as u32 * 4),
            (240, copies.len() as u32),
        ] {
            put(&mut bytes, at, value);
        }
        bytes[172..174].copy_from_slice(&4u16.to_le_bytes());
        bytes[174..176].copy_from_slice(&u16::MAX.to_le_bytes());
        for (field, (offset, bits)) in [(0u16, 32u16), (4, 32), (8, 16), (10, 8), (11, 8)]
            .into_iter()
            .enumerate()
        {
            let primitive = 244 + field * 4;
            bytes[primitive..primitive + 2].copy_from_slice(&(32 - bits).to_le_bytes());
            bytes[primitive + 2..primitive + 4].copy_from_slice(&offset.to_le_bytes());
            let column = 264 + field * 24;
            bytes[column..column + 2].copy_from_slice(&(offset * 8).to_le_bytes());
            bytes[column + 2..column + 4].copy_from_slice(&bits.to_le_bytes());
        }
        for (_, liquid_type) in rows {
            let mut record = [0; 12];
            // Ignored float fields are validated, without a finiteness gate.
            put(&mut record, 0, 0x7fc0_1234);
            put(&mut record, 4, 0x7f80_0000);
            record[8..10].copy_from_slice(&liquid_type.to_le_bytes());
            record[10..].copy_from_slice(&[7, 9]);
            bytes.extend_from_slice(&record);
        }
        for (id, _) in rows {
            bytes.extend_from_slice(&id.to_le_bytes());
        }
        for (new, old) in copies {
            bytes.extend_from_slice(&new.to_le_bytes());
            bytes.extend_from_slice(&old.to_le_bytes());
        }
        bytes
    }
    fn put(bytes: &mut [u8], at: usize, value: u32) {
        bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
    }
    static NEXT: AtomicU64 = AtomicU64::new(0);
    struct Private(PathBuf);
    impl Private {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "forever-liquid-object-diagnostic-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            let mut builder = fs::DirBuilder::new();
            #[cfg(unix)]
            {
                use std::os::unix::fs::DirBuilderExt;
                builder.mode(0o700);
            }
            builder.create(&path).unwrap();
            Self(path)
        }
        fn options(&self) -> Options {
            Options {
                table: Table::LiquidObject,
                input: self.0.join("authored-input.db2"),
                output: self.0.join("private-projection.json"),
            }
        }
    }
    impl Drop for Private {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }
    fn arguments(values: &[&str]) -> impl Iterator<Item = OsString> {
        values.iter().map(|value| OsString::from(*value))
    }

    #[test]
    fn liquid_object_mode_requires_exact_arguments_and_preserves_older_modes() {
        let options =
            parse_options(arguments(&["LiquidObject", "input.db2", "private.json"])).unwrap();
        assert!(matches!(options.table, Table::LiquidObject));
        assert_eq!(options.input, Path::new("input.db2"));
        assert_eq!(options.output, Path::new("private.json"));
        for mode in ["Map", "VMap", "LiquidType", "LiquidMaterial"] {
            assert!(parse_options(arguments(&[mode, "in", "out"])).is_ok());
        }
        for values in [
            vec![],
            vec!["LiquidObject"],
            vec!["LiquidObject", "in"],
            vec!["LiquidObject", "in", "out", "extra"],
            vec!["liquidobject", "in", "out"],
        ] {
            assert_eq!(
                parse_options(arguments(&values)).err(),
                Some("invalid_invocation")
            );
        }
    }
    #[test]
    fn liquid_object_document_preserves_direct_order_bits_and_serial_copy_semantics() {
        let private = Private::new();
        let options = private.options();
        let rows = [(9, -1), (3, i16::MIN), (9, 7)];
        let copies = [(4, 3), (4, 9), (8, 4), (0, 77), (77, 77), (9, 0)];
        fs::write(&options.input, liquid_object(&rows, &copies)).unwrap();
        assert_eq!(run(&options), Ok((3, 6, 6)));
        assert_eq!(
            fs::read_to_string(&options.output).unwrap(),
            "{\"table\":\"LiquidObject\",\"direct\":[[9,65535],[3,32768],[9,7]],\"copies\":[[4,3],[4,9],[8,4],[0,77],[77,77],[9,0]],\"materialized\":[[0,0],[3,32768],[4,7],[8,7],[9,0],[77,0]]}\n"
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&options.output).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
    }
    #[test]
    fn liquid_object_empty_complete_source_has_complete_empty_document() {
        let private = Private::new();
        let options = private.options();
        fs::write(&options.input, liquid_object(&[], &[])).unwrap();
        assert_eq!(run(&options), Ok((0, 0, 0)));
        assert_eq!(
            fs::read_to_string(&options.output).unwrap(),
            "{\"table\":\"LiquidObject\",\"direct\":[],\"copies\":[],\"materialized\":[]}\n"
        );
    }
    #[test]
    fn every_truncated_prefix_rejects_without_creating_projection() {
        let private = Private::new();
        let options = private.options();
        let bytes = liquid_object(&[(4, -1), (8, 17)], &[(7, 4)]);
        for len in 0..bytes.len() {
            fs::write(&options.input, &bytes[..len]).unwrap();
            assert!(run(&options).is_err(), "accepted authored prefix {len}");
            assert!(!options.output.exists());
        }
    }
    #[test]
    fn wrong_layout_encrypted_section_bad_unused_column_and_tail_reject() {
        let valid = liquid_object(&[(4, -1)], &[]);
        let mut wrong_layout = valid.clone();
        put(&mut wrong_layout, 156, 0xCB0D_39E9);
        let mut encrypted = valid.clone();
        put(&mut encrypted, 204, 1);
        let mut unused_column = valid.clone();
        put(&mut unused_column, 264 + 4 * 24 + 8, 99);
        let mut tail = valid;
        tail.push(0);
        for bytes in [wrong_layout, encrypted, unused_column, tail] {
            let private = Private::new();
            let options = private.options();
            fs::write(&options.input, bytes).unwrap();
            assert!(run(&options).is_err());
            assert!(!options.output.exists());
        }
    }
    #[test]
    fn missing_and_non_regular_inputs_never_create_output() {
        let private = Private::new();
        let mut options = private.options();
        assert_eq!(run(&options), Err("input_io"));
        assert!(!options.output.exists());
        options.input = private.0.clone();
        assert_eq!(run(&options), Err("input_not_regular"));
        assert!(!options.output.exists());
    }
    #[test]
    fn maximum_input_limit_is_unchanged_and_checked_before_output() {
        assert_eq!(MAX_INPUT_BYTES, 16 * 1024 * 1024);
        let private = Private::new();
        let options = private.options();
        File::create(&options.input)
            .unwrap()
            .set_len(MAX_INPUT_BYTES + 1)
            .unwrap();
        assert_eq!(run(&options), Err(RecordError::Limit.code()));
        assert!(!options.output.exists());
    }
    #[test]
    fn successful_decode_never_overwrites_existing_output() {
        let private = Private::new();
        let options = private.options();
        fs::write(&options.input, liquid_object(&[(4, 17)], &[])).unwrap();
        fs::write(&options.output, b"authored existing output").unwrap();
        assert_eq!(run(&options), Err("output_io"));
        assert_eq!(
            fs::read(&options.output).unwrap(),
            b"authored existing output"
        );
    }
}
