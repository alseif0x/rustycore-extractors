// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: authored native-ciphertext/public Rust BLTE acceptance.
// This is a synthetic test adapter, not a product CLI or key-discovery tool.
use forever_casc::{blte, keys::Keyring};
use std::{fs, path::PathBuf};

fn leaf(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'.')
        && value != "."
        && value != ".."
        && !value.starts_with('.')
}
fn hex(text: &str) -> [u8; 16] {
    assert_eq!(text.len(), 32);
    let mut bytes = [0; 16];
    for (out, pair) in bytes.iter_mut().zip(text.as_bytes().as_chunks::<2>().0) {
        *out = u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap();
    }
    bytes
}
fn main() {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    assert_eq!(args.len(), 2);
    let plan_path = PathBuf::from(&args[0]);
    let output = PathBuf::from(&args[1]);
    let source = plan_path.parent().unwrap();
    let plan = fs::read_to_string(&plan_path).unwrap();
    assert!(plan.len() <= 128 * 1024);
    assert!(plan.is_ascii());
    fs::create_dir(&output).unwrap();
    let mut count = 0;
    let mut total = 0u64;
    for line in plan.lines() {
        let parts: Vec<_> = line.split('\t').collect();
        assert_eq!(parts.len(), 4);
        assert!(leaf(parts[0]) && leaf(parts[1]));
        let encoded = fs::read(source.join(parts[0])).unwrap();
        let key_bytes = fs::read(source.join(parts[1])).unwrap();
        assert!(encoded.len() <= 1024 * 1024 && key_bytes.len() <= 256 * 1024);
        let keys = Keyring::parse(&key_bytes).unwrap();
        let size: u64 = parts[3].parse().unwrap();
        assert!(size <= 1024 * 1024);
        let ckey = hex(parts[2]);
        let limits = blte::Limits {
            max_encoded_bytes: 1024 * 1024,
            max_decoded_bytes: 1024 * 1024,
            max_frames: 512,
        };
        let bytes = blte::decode_with_keys(&encoded, size, Some(ckey), limits, &keys).unwrap();
        assert_eq!(bytes.len() as u64, size);
        // Default capability still rejects the exact same encrypted object.
        assert_eq!(
            blte::decode(&encoded, size, Some(ckey), limits),
            Err(blte::Error::EncryptedUnsupported)
        );
        fs::write(output.join(format!("{}.out", parts[0])), &bytes).unwrap();
        total += size;
        count += 1;
        assert!(count <= 512 && total <= 64 * 1024 * 1024);
    }
    assert!(count > 0);
    println!(
        "{{\"cases\":{count},\"decoded_bytes\":{total},\"public_blte_decode_with_keys\":true,\"default_rejection_per_case\":true}}"
    );
}
