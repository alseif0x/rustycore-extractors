// SPDX-License-Identifier: GPL-3.0-or-later
//! LOCAL DIAGNOSTIC ONLY. This example dumps the fully decoded rows of one
//! `ChrClasses`/`ChrRaces`/`ChrClassTitle`/`ChrClassUIDisplay`/
//! `ChrClassUIChrModelInfo`/`ChrClassVillain` DB2 file to stdout so an
//! independent offline oracle can be compared byte-for-byte.
//!
//! Its output contains client-derived values. It must never be committed,
//! copied into Git, attached to a report, uploaded or pasted into a model
//! context. Run it only on a workstation that already holds the local client
//! file, and keep the output inside the external run directory.
//!
//! Contract: `db2_dump --file <path.db2> --table <name>`, where `<name>` is any
//! reviewed `CharacterTable` name. Metadata-only names require an explicit
//! `--schema-catalog pinned-metadata-v1` or `--schema-catalog
//! wowdbdefs-70205-v1`. The latter selects the build 70205 definitions. No
//! network and no CASC: only the named local file is read. The canonical
//! format is one header line `forever-db2-dump v1 <tablename>` followed by one
//! line per row in ascending id order; fields are tab-separated and array
//! elements are comma-joined.
//! Values are tagged `u:`, `i:`, `f:0x########` or `t:`.
use forever_db2::{CharacterTable, DecodedTable, RecordError, Value, catalog};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// Mirrors the decoder's own 16 MiB policy so an oversized file is rejected
/// before it is read, not after it is fully resident.
const MAX_BYTES: u64 = 16 * 1024 * 1024;

struct Config {
    file: PathBuf,
    table: CharacterTable,
}

fn main() -> ExitCode {
    let config = match parse_args() {
        Ok(config) => config,
        Err(code) => {
            eprintln!("{code}");
            return ExitCode::from(2);
        }
    };
    match dump(&config) {
        Ok(()) => ExitCode::SUCCESS,
        Err(code) => {
            eprintln!("{code}");
            ExitCode::from(4)
        }
    }
}

fn parse_args() -> Result<Config, &'static str> {
    parse_options(std::env::args_os().skip(1))
}

fn parse_options(
    args: impl IntoIterator<Item = std::ffi::OsString>,
) -> Result<Config, &'static str> {
    let mut file: Option<PathBuf> = None;
    let mut name: Option<String> = None;
    let mut catalog_selected = None;
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        if arg == "--file" && file.is_none() {
            let value = args.next().ok_or("invalid_invocation")?;
            file = Some(PathBuf::from(value));
        } else if arg == "--table" && name.is_none() {
            let value = args.next().ok_or("invalid_invocation")?;
            name = Some(value.to_str().ok_or("invalid_invocation")?.to_owned());
        } else if arg == "--schema-catalog" && catalog_selected.is_none() {
            let value = args.next().ok_or("invalid_invocation")?;
            catalog_selected = Some(match value.to_str() {
                Some(catalog::CATALOG_ID) => catalog::CATALOG_ID,
                Some(catalog::WOWDBDEFS_70205_CATALOG_ID) => catalog::WOWDBDEFS_70205_CATALOG_ID,
                _ => return Err("invalid_invocation"),
            });
        } else {
            return Err("invalid_invocation");
        }
    }
    let name = name.ok_or("invalid_invocation")?;
    let table = if let Some(catalog) = catalog_selected {
        CharacterTable::from_catalog_id(catalog, &name).map_err(|error| error.code())?
    } else {
        CharacterTable::named(&name)
    };
    Ok(Config {
        file: file.ok_or("invalid_invocation")?,
        table: table.ok_or(RecordError::Schema.code())?,
    })
}

fn dump(config: &Config) -> Result<(), &'static str> {
    let data = read_file(&config.file)?;
    let table = DecodedTable::read(&data, config.table).map_err(RecordError::code)?;
    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());
    writeln!(out, "forever-db2-dump v1 {}", config.table.name()).map_err(output_error)?;
    for (id, fields) in table.rows() {
        write!(out, "{id}").map_err(output_error)?;
        for values in fields {
            out.write_all(b"\t").map_err(output_error)?;
            write_values(&mut out, values).map_err(output_error)?;
        }
        out.write_all(b"\n").map_err(output_error)?;
    }
    out.flush().map_err(output_error)
}

fn read_file(path: &Path) -> Result<Vec<u8>, &'static str> {
    let mut file = std::fs::File::open(path).map_err(|_| "input_io")?;
    let len = file.metadata().map_err(|_| "input_io")?.len();
    if len > MAX_BYTES {
        return Err(RecordError::Limit.code());
    }
    let mut data = Vec::with_capacity(len as usize);
    file.read_to_end(&mut data).map_err(|_| "input_io")?;
    Ok(data)
}

fn write_values(out: &mut impl Write, values: &[Value]) -> io::Result<()> {
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            out.write_all(b",")?;
        }
        match value {
            Value::Unsigned(number) => write!(out, "u:{number}")?,
            Value::Unsigned64(number) => write!(out, "u:{number}")?,
            Value::Signed(number) => write!(out, "i:{number}")?,
            Value::Signed64(number) => write!(out, "i:{number}")?,
            Value::FloatBits(bits) => write!(out, "f:0x{bits:08x}")?,
            Value::Text(text) => {
                out.write_all(b"t:")?;
                write_text(out, text)?;
            }
        }
    }
    Ok(())
}

/// `\` -> `\\`, tab -> `\t`, CR -> `\r`, LF -> `\n`; every other UTF-8 byte is
/// emitted unchanged.
fn write_text(out: &mut impl Write, text: &str) -> io::Result<()> {
    for character in text.chars() {
        match character {
            '\\' => out.write_all(b"\\\\")?,
            '\t' => out.write_all(b"\\t")?,
            '\r' => out.write_all(b"\\r")?,
            '\n' => out.write_all(b"\\n")?,
            _ => {
                let mut buffer = [0u8; 4];
                out.write_all(character.encode_utf8(&mut buffer).as_bytes())?;
            }
        }
    }
    Ok(())
}

fn output_error(_: io::Error) -> &'static str {
    "output_io"
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options(args: &[&str]) -> Result<Config, &'static str> {
        parse_options(args.iter().map(|arg| std::ffi::OsString::from(*arg)))
    }

    #[test]
    fn catalog_diagnostic_is_explicit_and_order_independent() {
        for args in [
            vec![
                "--file",
                "synthetic.db2",
                "--table",
                "LiquidType",
                "--schema-catalog",
                "pinned-metadata-v1",
            ],
            vec![
                "--schema-catalog",
                "pinned-metadata-v1",
                "--table",
                "LiquidType",
                "--file",
                "synthetic.db2",
            ],
        ] {
            assert_eq!(options(&args).unwrap().table.name(), "LiquidType");
        }
        assert!(options(&["--file", "synthetic.db2", "--table", "LiquidType"]).is_err());
        assert!(
            options(&[
                "--file",
                "synthetic.db2",
                "--table",
                "LiquidType",
                "--schema-catalog",
                "untrusted"
            ])
            .is_err()
        );
        assert!(
            options(&[
                "--file",
                "synthetic.db2",
                "--table",
                "LiquidType",
                "--schema-catalog",
                "pinned-metadata-v1",
                "--schema-catalog",
                "pinned-metadata-v1"
            ])
            .is_err()
        );
        assert_eq!(
            options(&[
                "--file",
                "synthetic.db2",
                "--table",
                "ChrClasses",
                "--schema-catalog",
                "pinned-metadata-v1"
            ])
            .unwrap()
            .table,
            CharacterTable::Classes
        );
    }

    #[test]
    fn canonical_signed_64_text_is_lossless() {
        let mut out = Vec::new();
        write_values(
            &mut out,
            &[
                Value::Signed64(i64::MIN),
                Value::Signed64(i64::MAX),
                Value::Signed64(-1),
            ],
        )
        .unwrap();
        assert_eq!(out, b"i:-9223372036854775808,i:9223372036854775807,i:-1");
    }
}
