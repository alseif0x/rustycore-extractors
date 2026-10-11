// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: original local synthetic comparison adapter.
//! Linux QA only: m2_raw <input.m2> <max_output_bytes> <output.raw>.
//! No acquisition/network or client-data logging; the library remains pure.

#[cfg(target_os = "linux")]
mod qa {
    use std::fs::{self, Metadata, OpenOptions};
    use std::io::{Read, Write};
    use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
    use std::path::Path;

    use forever_vmap::{Limits, M2Collision, MAX_INPUT_BYTES, MAX_RAW_BYTES, RawM2};

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

    pub fn run() -> Result<(), &'static str> {
        let args: Vec<_> = std::env::args_os().collect();
        if args.len() != 4 {
            return Err("qa_usage");
        }
        let cap_text = args[2].to_str().ok_or("qa_output_limit")?;
        if cap_text.is_empty() || !cap_text.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err("qa_output_limit");
        }
        let cap: usize = cap_text.parse().map_err(|_| "qa_output_limit")?;
        if cap > MAX_RAW_BYTES {
            return Err("qa_output_limit");
        }
        let bytes = read_complete(Path::new(&args[1]))?;
        let Some(model) = M2Collision::parse(&bytes, Limits::default()).map_err(|e| e.code())?
        else {
            println!("{{\"status\":\"no_collision\"}}");
            return Ok(());
        };
        let output = model.write_raw(cap).map_err(|e| e.code())?;
        let verified = RawM2::parse(&output, Limits::default()).map_err(|e| e.code())?;
        write_new(Path::new(&args[3]), verified.bytes())?;
        println!(
            "{{\"status\":\"ok\",\"bytes\":{},\"vertices\":{},\"indices\":{},\"triangles\":{}}}",
            verified.bytes().len(),
            verified.vertex_count(),
            verified.index_count(),
            verified.triangle_count(),
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
