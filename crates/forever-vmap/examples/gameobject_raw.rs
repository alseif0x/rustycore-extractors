// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: bounded synthetic gameobject-list QA, no acquisition.
// Ordinary bounded file I/O; no source-identity or no-symlink guarantee.
// Filesystem basename admission is stricter than the pure borrowed codecs.
use forever_vmap::gameobject_models::{self, Dtree, Limits, RawModel, ResolvedRow, TempList};
use std::collections::{BTreeMap, btree_map::Entry};
use std::io::{Read, Write};

const MAX_FILE: usize = 64 * 1024 * 1024;
const MAX_RAW_TOTAL: usize = 256 * 1024 * 1024;
const MAX_ROWS: usize = 100_000;

fn read(path: &std::path::Path, maximum: usize) -> Result<Vec<u8>, &'static str> {
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

fn basename(name: &[u8]) -> Result<&str, &'static str> {
    if name.is_empty()
        || name.len() > 255
        || matches!(name, b"." | b".." | b"temp_gameobject_models")
        || name
            .iter()
            .any(|&c| !(32..=126).contains(&c) || matches!(c, b'/' | b'\\' | b':'))
    {
        return Err("qa_name");
    }
    std::str::from_utf8(name).map_err(|_| "qa_name")
}

fn run() -> Result<(), &'static str> {
    let args: Vec<_> = std::env::args_os().take(5).collect();
    let limits = Limits::default();
    if args.len() == 3 && (args[1] == "verify-temp" || args[1] == "verify-dtree") {
        let bytes = read(std::path::Path::new(&args[2]), MAX_FILE)?;
        let rows = if args[1] == "verify-temp" {
            TempList::parse(&bytes, limits)
                .map_err(|e| e.code())?
                .row_count()
        } else {
            Dtree::parse(&bytes, limits)
                .map_err(|e| e.code())?
                .row_count()
        };
        println!(
            "{{\"status\":\"ok\",\"rows\":{rows},\"bytes\":{}}}",
            bytes.len()
        );
        return Ok(());
    }
    if args.len() != 4 || args[1] != "build" {
        return Err("qa_usage");
    }
    let input = read(std::path::Path::new(&args[2]), MAX_FILE)?;
    let list = TempList::parse(&input, limits).map_err(|e| e.code())?;
    if list.row_count() > MAX_ROWS {
        return Err("qa_extent");
    }
    let raw_dir = std::path::Path::new(&args[3]);
    let mut models: BTreeMap<&[u8], Vec<u8>> = BTreeMap::new();
    let mut total = 0usize;
    for row in list.rows() {
        let name = row.name();
        let filename = basename(name)?;
        if let Entry::Vacant(entry) = models.entry(name) {
            let remaining = MAX_RAW_TOTAL.checked_sub(total).ok_or("qa_extent")?;
            let raw = read(&raw_dir.join(filename), MAX_FILE.min(remaining))?;
            total = total.checked_add(raw.len()).ok_or("qa_extent")?;
            entry.insert(raw);
        }
    }
    let mut rows = Vec::new();
    rows.try_reserve_exact(list.row_count())
        .map_err(|_| "qa_allocation")?;
    for row in list.rows() {
        let raw = models.get(row.name()).ok_or("qa_input")?;
        let flags = raw.get(20..24).ok_or("qa_raw_header")?;
        let model = if flags == 1u32.to_le_bytes() {
            RawModel::M2(raw)
        } else if flags == 0u32.to_le_bytes() {
            RawModel::Wmo(raw)
        } else {
            return Err("qa_raw_flags");
        };
        rows.push(ResolvedRow {
            display_id: row.display_id(),
            name: row.name(),
            model: Some(model),
        });
    }
    let output = gameobject_models::build(&rows, limits).map_err(|e| e.code())?;
    if output.temp_bytes() != input {
        return Err("qa_temp_roundtrip");
    }
    std::io::stdout()
        .lock()
        .write_all(output.dtree_bytes())
        .map_err(|_| "qa_output")
}

fn main() {
    if let Err(code) = run() {
        eprintln!("{code}");
        std::process::exit(4);
    }
}
