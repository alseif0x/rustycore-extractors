// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original bounded direct-placement comparison adapter.
//! Linux QA only: placement_raw <RCEPLC01-fixture> <new-output>.
//! No acquisition/network or client-data logging; the library remains pure.

#[cfg(target_os = "linux")]
mod qa {
    use std::fs::{self, Metadata, OpenOptions};
    use std::io::{Read, Write};
    use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
    use std::path::Path;

    use forever_vmap::placement::{self, Context, DirBin, Limits, Mddf, Modf};

    const MAX_INPUT_BYTES: usize = 32 + 64 + 500;

    // Linux open(2) constants, used only by this Linux QA adapter. No FFI.
    const O_NONBLOCK: i32 = 0x800;
    const O_NOFOLLOW: i32 = 0x20000;

    fn same(first: &Metadata, second: &Metadata) -> bool {
        first.dev() == second.dev()
            && first.ino() == second.ino()
            && first.mode() == second.mode()
            && first.len() == second.len()
            && first.mtime() == second.mtime()
            && first.mtime_nsec() == second.mtime_nsec()
            && first.ctime() == second.ctime()
            && first.ctime_nsec() == second.ctime_nsec()
    }

    fn read_complete(path: &Path) -> Result<Vec<u8>, &'static str> {
        let named = fs::symlink_metadata(path).map_err(|_| "qa_input_metadata")?;
        if !named.is_file() || named.file_type().is_symlink() {
            return Err("qa_input_not_file");
        }
        if named.len() > MAX_INPUT_BYTES as u64 {
            return Err("qa_input_limit");
        }
        let mut file = OpenOptions::new()
            .read(true)
            .custom_flags(O_NOFOLLOW | O_NONBLOCK)
            .open(path)
            .map_err(|_| "qa_input_open")?;
        let before = file.metadata().map_err(|_| "qa_input_metadata")?;
        if !same(&named, &before) {
            return Err("qa_input_changed");
        }
        let length = usize::try_from(before.len()).map_err(|_| "qa_input_limit")?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(length)
            .map_err(|_| "qa_allocation")?;
        bytes.resize(length, 0);
        file.read_exact(&mut bytes).map_err(|_| "qa_input_read")?;
        let mut tail = [0u8; 1];
        if file.read(&mut tail).map_err(|_| "qa_input_read")? != 0 {
            return Err("qa_input_changed");
        }
        let after = file.metadata().map_err(|_| "qa_input_metadata")?;
        let named_after = fs::symlink_metadata(path).map_err(|_| "qa_input_changed")?;
        if !same(&before, &after) || !same(&before, &named_after) {
            return Err("qa_input_changed");
        }
        Ok(bytes)
    }

    fn write_new(path: &Path, bytes: &[u8]) -> Result<(), &'static str> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(path)
            .map_err(|_| "qa_output_create")?;
        // All validation completed before creation. An I/O failure may leave
        // this new partial file for inspection; no automatic cleanup can race
        // with another process replacing its path. Existing outputs are never
        // opened by create_new, and failures never print a success status.
        file.write_all(bytes)
            .and_then(|()| file.sync_all())
            .map_err(|_| "qa_output_write")
    }

    fn word(bytes: &[u8], at: usize) -> Result<u32, &'static str> {
        let value = bytes.get(at..at + 4).ok_or("qa_fixture_header")?;
        Ok(u32::from_le_bytes(
            value.try_into().map_err(|_| "qa_fixture_header")?,
        ))
    }

    pub fn run() -> Result<(), &'static str> {
        let args: Vec<_> = std::env::args_os().take(4).collect();
        if args.len() != 3 {
            return Err("qa_usage");
        }
        let bytes = read_complete(Path::new(&args[1]))?;
        if bytes.len() < 32
            || &bytes[..8] != b"RCEPLC01"
            || bytes[9] & !3 != 0
            || bytes[10..12] != [0, 0]
        {
            return Err("qa_fixture_header");
        }
        let kind = bytes[8];
        let record_size: usize = match kind {
            1 => 36,
            2 => 64,
            _ => return Err("qa_fixture_kind"),
        };
        if kind == 1 && bytes[9] & 1 != 0 {
            return Err("qa_fixture_options");
        }
        let name_len = usize::try_from(word(&bytes, 24)?).map_err(|_| "qa_fixture_limit")?;
        let cap = usize::try_from(word(&bytes, 28)?).map_err(|_| "qa_fixture_limit")?;
        if cap > placement::MAX_BYTES {
            return Err("qa_output_limit");
        }
        let name_at = 32 + record_size;
        if name_at.checked_add(name_len).ok_or("qa_fixture_limit")? != bytes.len() {
            return Err("qa_fixture_extent");
        }
        let context = Context {
            map_id: word(&bytes, 12)?,
            original_map_id: word(&bytes, 16)?,
            global_wmo: bytes[9] & 1 != 0,
        };
        let assigned_id = word(&bytes, 20)?;
        let limits = Limits {
            max_bytes: cap,
            ..Limits::default()
        };
        let name = &bytes[name_at..];
        let record = &bytes[32..name_at];
        let value = if kind == 1 {
            let source = Mddf::parse(record, Limits::default()).map_err(|e| e.code())?;
            placement::project_m2(&source, assigned_id, name, context, limits)
                .map_err(|e| e.code())?
        } else {
            let source = Modf::parse(record, Limits::default()).map_err(|e| e.code())?;
            placement::project_wmo(&source, assigned_id, name, context, limits)
                .map_err(|e| e.code())?
        };
        let encoded = placement::encode(&[value], limits).map_err(|e| e.code())?;
        let parsed = DirBin::parse(&encoded, limits).map_err(|e| e.code())?;
        if parsed.record_count() != 1 {
            return Err("qa_reader_count");
        }
        let recovered = parsed.records().next().ok_or("qa_reader_count")?;
        let roundtrip = placement::encode(&[recovered], limits).map_err(|e| e.code())?;
        if encoded != roundtrip {
            return Err("qa_reader_roundtrip");
        }
        write_new(Path::new(&args[2]), &encoded)?;
        // Cache option is a native-only observation; this caller does not
        // implement or claim the original cache replay/ID allocator.
        println!(
            "{{\"status\":\"ok\",\"bytes\":{},\"records\":1,\"roundtrip\":true}}",
            encoded.len()
        );
        Ok(())
    }
}

#[cfg(target_os = "linux")]
fn main() {
    if let Err(code) = qa::run() {
        println!("{{\"status\":\"error\",\"code\":\"{code}\"}}");
        std::process::exit(4);
    }
}

#[cfg(not(target_os = "linux"))]
fn main() {
    println!("{{\"status\":\"error\",\"code\":\"qa_platform_unsupported\"}}");
    std::process::exit(4);
}
