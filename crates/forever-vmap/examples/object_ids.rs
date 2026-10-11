// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: bounded synthetic RCEID001 serial allocator QA.
// Input/output framing matches tests/forever/native_object_ids.cpp. Output
// is buffered until every request succeeds. No real assets or paths emitted.
use forever_vmap::placement::ids::{IdAllocator, Limits};
use std::io::{Read, Write};

fn run() -> Result<(), &'static str> {
    let args: Vec<_> = std::env::args_os().take(3).collect();
    if args.len() != 2 {
        return Err("qa_usage");
    }
    let file = std::fs::File::open(&args[1]).map_err(|_| "qa_input")?;
    if !file.metadata().map_err(|_| "qa_input")?.is_file() {
        return Err("qa_input");
    }
    let mut bytes = Vec::new();
    file.take(800013)
        .read_to_end(&mut bytes)
        .map_err(|_| "qa_input")?;
    if bytes.len() < 12 || bytes.len() > 800012 || &bytes[..8] != b"RCEID001" {
        return Err("qa_header");
    }
    let count = u32::from_le_bytes(bytes[8..12].try_into().map_err(|_| "qa_header")?);
    if count > 100000 || bytes.len() != 12 + count as usize * 8 {
        return Err("qa_extent");
    }
    if bytes[12..]
        .as_chunks::<8>()
        .0
        .iter()
        .any(|r| r[6] > 1 || r[7] != 0)
    {
        return Err("qa_record");
    }
    let mut allocator = IdAllocator::new(Limits::default()).map_err(|e| e.code())?;
    let mut result = Vec::with_capacity(12 + count as usize * 4);
    result.extend_from_slice(&bytes[..12]);
    for r in bytes[12..].as_chunks::<8>().0 {
        let client = u32::from_le_bytes(r[..4].try_into().map_err(|_| "qa_record")?);
        let doodad = u16::from_le_bytes(r[4..6].try_into().map_err(|_| "qa_record")?);
        let id = allocator
            .assign(client, doodad, r[6] != 0)
            .map_err(|e| e.code())?;
        result.extend_from_slice(&id.to_le_bytes());
    }
    std::io::stdout()
        .lock()
        .write_all(&result)
        .map_err(|_| "qa_output")
}

fn main() {
    if let Err(code) = run() {
        eprintln!("{code}");
        std::process::exit(4);
    }
}
