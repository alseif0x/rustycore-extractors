// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: supplied synthetic keys and complete toy stores only.
use super::*;
use crate::{keys, keys::Keyring};

const NAME: u64 = 0x0102030405060708;
const KEY: [u8; 16] = [0x5a; 16];
fn keyring(name: u64, key: [u8; 16]) -> Keyring {
    Keyring::parse(format!("forever-tact-keys\t1\n{name:016x}\t{}\n", hex(key)).as_bytes()).unwrap()
}
fn encrypted(inner: &[u8]) -> Vec<u8> {
    let nonce = [1, 2, 3, 4, 5, 6, 7, 8];
    let mut out = vec![8];
    out.extend_from_slice(&NAME.to_le_bytes());
    out.push(8);
    out.extend_from_slice(&nonce);
    out.push(b'S');
    let mut cipher = inner.to_vec();
    keys::xor(&KEY, nonce, &mut cipher);
    out.extend_from_slice(&cipher);
    out
}
fn builder() -> Builder {
    let mut b = Builder::new(CONTENT);
    let mut inner = vec![b'N'];
    inner.extend_from_slice(CONTENT);
    b.records[0] = (EKEY, b'E', encrypted(&inner));
    b
}
fn rejection(store: &mut LocalStore) -> CandidateFailures {
    match store.read_file_data_id(FILE_ID) {
        Err(Error::NoLocalEncoding(failures)) => failures,
        _ => panic!("expected complete synthetic candidate rejection"),
    }
}
#[test]
fn default_rejects_and_explicit_keys_return_identical_verified_content() {
    let fixture = builder().write();
    let mut default = fixture.open();
    assert_eq!(rejection(&mut default).encrypted, 1);
    let ring = keyring(NAME, KEY);
    let resident = ring.resident_bytes();
    let mut supplied = LocalStore::open_with_keys(&fixture.path, options(), ring).unwrap();
    let result = supplied.read_file_data_id(FILE_ID).unwrap();
    assert_eq!(result.bytes, CONTENT);
    assert_eq!(result.content_key, md5::digest(CONTENT));
    assert_eq!(result.candidate_attempts, 1);
    assert_eq!(default.content_identity(), supplied.content_identity());
    assert_eq!(
        supplied.stats().resident_bytes,
        default.stats().resident_bytes + resident
    );
    supplied.verify_unchanged().unwrap();
}
#[test]
fn missing_and_wrong_supplied_keys_produce_no_unverified_bytes() {
    let fixture = builder().write();
    let mut missing =
        LocalStore::open_with_keys(&fixture.path, options(), keyring(NAME + 1, KEY)).unwrap();
    assert_eq!(rejection(&mut missing).encrypted, 1);
    let mut wrong =
        LocalStore::open_with_keys(&fixture.path, options(), keyring(NAME, [0x5b; 16])).unwrap();
    let rejected = rejection(&mut wrong);
    assert_eq!(rejected.corrupt + rejected.unsupported, 1);
}
#[test]
fn different_cipher_and_malformed_header_remain_distinct_rejections() {
    let mut b = builder();
    b.records[0].2[18] = b'A';
    let fixture = b.write();
    let mut supplied =
        LocalStore::open_with_keys(&fixture.path, options(), keyring(NAME, KEY)).unwrap();
    assert_eq!(rejection(&mut supplied).unsupported, 1);
    let mut b = builder();
    b.records[0].2[0] = 7;
    let fixture = b.write();
    let mut supplied =
        LocalStore::open_with_keys(&fixture.path, options(), keyring(NAME, KEY)).unwrap();
    assert_eq!(rejection(&mut supplied).corrupt, 1);
}
#[test]
fn resident_keys_are_charged_before_store_io_and_default_limits_are_not_raised() {
    let fixture = builder().write();
    let baseline = fixture.open();
    let mut opts = options();
    // Admission retains temporary config references that are released later;
    // find the actual resident high-water bound, rather than final stats.
    let (mut low, mut high) = (0, baseline.stats().resident_bytes + 1024 * 1024);
    while low < high {
        let mid = low + (high - low) / 2;
        opts.limits.max_resident_bytes = mid;
        if LocalStore::open(&fixture.path, opts).is_ok() {
            high = mid;
        } else {
            low = mid + 1;
        }
    }
    opts.limits.max_resident_bytes = low;
    assert!(LocalStore::open(&fixture.path, opts).is_ok());
    assert!(matches!(
        LocalStore::open_with_keys(&fixture.path, opts, keyring(NAME, KEY)),
        Err(Error::Limit)
    ));
    opts.limits.max_resident_bytes = 0;
    assert!(matches!(
        LocalStore::open_with_keys(
            Path::new("/nonexistent-synthetic-store"),
            opts,
            keyring(NAME, KEY)
        ),
        Err(Error::Limit)
    ));
}
#[cfg(target_os = "linux")]
#[test]
fn strict_keyless_encrypted_framing_validates_wrappers_and_rejects_mixed_failures() {
    fn fixture_with_encoded_candidates(encoded_candidates: Vec<Vec<u8>>) -> Fixture {
        let mut builder = Builder::new(CONTENT);
        let mut ekeys = Vec::new();
        for (index, encoded) in encoded_candidates.into_iter().enumerate() {
            let ekey = md5::digest(&encoded);
            if index == 0 {
                builder.records[0] = (ekey, b'N', Vec::new());
            } else {
                builder.records.push((ekey, b'N', Vec::new()));
            }
            builder.framed_records.insert(ekey, encoded);
            ekeys.push(ekey);
        }
        builder.routes[0].2 = ekeys;
        builder.write()
    }

    let mut valid = b"BLTE\0\0\0\0".to_vec();
    valid.push(b'E');
    valid.extend_from_slice(&encrypted(&[b'N', 1]));
    let fixture = fixture_with_encoded_candidates(vec![valid]);
    let mut store = fixture.open();
    let result = store.read_file_data_id_with_external_budget_policy(
        FILE_ID,
        4096,
        0,
        store.limits.max_memory_bytes,
        true,
    );
    assert!(matches!(result, Err(Error::NoLocalEncoding(failures)) if failures.encrypted == 1));

    let mut malformed = b"BLTE\0\0\0\0".to_vec();
    malformed.extend_from_slice(&[b'E', 8, 0, 0, 0]);
    let fixture = fixture_with_encoded_candidates(vec![malformed]);
    let mut store = fixture.open();
    let result = store.read_file_data_id_with_external_budget_policy(
        FILE_ID,
        4096,
        0,
        store.limits.max_memory_bytes,
        true,
    );
    assert!(matches!(result, Err(Error::NoLocalEncoding(failures)) if failures.corrupt == 1));

    let mut unsupported_cipher = b"BLTE\0\0\0\0".to_vec();
    unsupported_cipher.extend_from_slice(&[b'E', 0, 4, 0, 0, 0, 0, b'A', 0x55]);
    let fixture = fixture_with_encoded_candidates(vec![unsupported_cipher]);
    let mut store = fixture.open();
    let result = store.read_file_data_id_with_external_budget_policy(
        FILE_ID,
        4096,
        0,
        store.limits.max_memory_bytes,
        true,
    );
    assert!(matches!(result, Err(Error::NoLocalEncoding(failures)) if failures.unsupported == 1));

    let mut later_corrupt = b"BLTE".to_vec();
    later_corrupt.extend_from_slice(&60_u32.to_be_bytes());
    later_corrupt.extend_from_slice(&[15, 0, 0, 2]);
    let encrypted_frame = [b'E', 0, 4, 0, 0, 0, 0, b'S', 0x55];
    let normal_frame = b"Nabc";
    for (frame, decoded) in [(&encrypted_frame[..], 1_u32), (&normal_frame[..], 3)] {
        later_corrupt.extend_from_slice(&(frame.len() as u32).to_be_bytes());
        later_corrupt.extend_from_slice(&decoded.to_be_bytes());
        let mut digest = md5::digest(frame);
        if frame[0] == b'N' {
            digest[0] ^= 1;
        }
        later_corrupt.extend_from_slice(&digest);
    }
    later_corrupt.extend_from_slice(&encrypted_frame);
    later_corrupt.extend_from_slice(normal_frame);
    assert_eq!(
        later_corrupt.len(),
        60 + encrypted_frame.len() + normal_frame.len()
    );
    let fixture = fixture_with_encoded_candidates(vec![later_corrupt]);
    let mut store = fixture.open();
    let result = store.read_file_data_id_with_external_budget_policy(
        FILE_ID,
        4096,
        0,
        store.limits.max_memory_bytes,
        true,
    );
    assert!(
        matches!(result, Err(Error::NoLocalEncoding(failures)) if failures.corrupt == 1 && failures.encrypted == 0)
    );

    let mut mixed = b"BLTE\0\0\0\0".to_vec();
    mixed.push(b'E');
    mixed.extend_from_slice(&encrypted(&[b'N', 1]));
    let unsupported = {
        let mut frame = vec![b'E', 0, 4, 0, 0, 0, 0, b'A', 0x55];
        let mut encoded = b"BLTE\0\0\0\0".to_vec();
        encoded.append(&mut frame);
        encoded
    };
    let fixture = fixture_with_encoded_candidates(vec![mixed, unsupported]);
    let mut store = fixture.open();
    let result = store.read_file_data_id_with_external_budget_policy(
        FILE_ID,
        4096,
        0,
        store.limits.max_memory_bytes,
        true,
    );
    assert!(
        matches!(result, Err(Error::NoLocalEncoding(failures)) if failures.encrypted == 1 && failures.unsupported == 1)
    );

    let mut corrupt_wrapper = b"BLTE\0\0\0\0".to_vec();
    corrupt_wrapper.extend_from_slice(&[b'E', 8, 1]);
    let mut valid_encrypted = b"BLTE\0\0\0\0".to_vec();
    valid_encrypted.push(b'E');
    valid_encrypted.extend_from_slice(&encrypted(&[b'N', 1]));
    let fixture = fixture_with_encoded_candidates(vec![corrupt_wrapper, valid_encrypted]);
    let mut store = fixture.open();
    let result = store.read_file_data_id_with_external_budget_policy(
        FILE_ID,
        4096,
        0,
        store.limits.max_memory_bytes,
        true,
    );
    assert!(
        matches!(result, Err(Error::NoLocalEncoding(failures)) if failures.encrypted == 1 && failures.corrupt == 1)
    );
}

#[cfg(target_os = "linux")]
#[test]
fn strict_encrypted_inspection_rejects_symlinked_archive_without_following() {
    use std::os::unix::fs::symlink;

    let fixture = builder().write();
    let archive = fixture.path.join("Data/data/data.000");
    let target = fixture.path.join("Data/data/target.000");
    fs::rename(&archive, &target).unwrap();
    symlink("target.000", archive).unwrap();
    let mut store = fixture.open();
    assert!(matches!(
        store.read_file_data_id_with_external_budget_policy(
            FILE_ID,
            4096,
            0,
            store.limits.max_memory_bytes,
            true,
        ),
        Err(Error::Input)
    ));
}

#[test]
fn extra_encoded_scratch_is_reserved_for_opt_in_without_changing_default_profile() {
    let fixture = builder().write();
    let mut supplied =
        LocalStore::open_with_keys(&fixture.path, options(), keyring(NAME, KEY)).unwrap();
    let span = fixture.rows[0][14..18]
        .try_into()
        .map(u32::from_le_bytes)
        .unwrap() as usize;
    // Set limits only after metadata admission, so this isolates file decoding.
    supplied.limits.max_scratch_bytes = span + CONTENT.len();
    assert!(matches!(
        supplied.read_file_data_id(FILE_ID),
        Err(Error::Limit)
    ));
    supplied.limits.max_scratch_bytes = 2 * span + CONTENT.len();
    assert_eq!(supplied.read_file_data_id(FILE_ID).unwrap().bytes, CONTENT);
    let mut default = fixture.open();
    default.limits.max_scratch_bytes = span + CONTENT.len();
    assert_eq!(rejection(&mut default).encrypted, 1);
}
#[test]
fn encrypted_metadata_can_only_open_through_explicit_keys_and_preserves_identity() {
    let mut fixture = Builder::new(CONTENT).write();
    let identity = fixture.open().content_identity();
    let archive_path = fixture.path.join("Data/data/data.000");
    let plain = fs::read(&archive_path).unwrap();
    let mut archive = Vec::new();
    for row in &mut fixture.rows {
        // Fixture has archive index zero, big-endian five-byte offset and LE size.
        let mut offset = [0u8; 8];
        offset[3..].copy_from_slice(&row[9..14]);
        let old_offset = u64::from_be_bytes(offset) as usize;
        let size = u32::from_le_bytes(row[14..18].try_into().unwrap()) as usize;
        let old = &plain[old_offset..old_offset + size];
        let new_offset = archive.len() as u64;
        archive.extend_from_slice(&old[..38]);
        if row[..9] == ROOT_EKEY[..9] || row[..9] == ENCODING_EKEY[..9] {
            archive.push(b'E');
            archive.extend_from_slice(&encrypted(&old[38..]));
        } else {
            archive.extend_from_slice(&old[38..]);
        }
        let mut key = [0; 16];
        key[..9].copy_from_slice(&row[..9]);
        *row = index_entry(key, new_offset, (archive.len() as u64 - new_offset) as u32);
    }
    fs::write(&archive_path, archive).unwrap();
    fixture.rewrite_rows();
    assert!(matches!(
        LocalStore::open(&fixture.path, options()),
        Err(Error::NoLocalEncoding(CandidateFailures {
            encrypted: 1,
            ..
        }))
    ));
    let mut supplied =
        LocalStore::open_with_keys(&fixture.path, options(), keyring(NAME, KEY)).unwrap();
    assert_eq!(supplied.content_identity(), identity);
    assert_eq!(supplied.read_file_data_id(FILE_ID).unwrap().bytes, CONTENT);
}
#[test]
fn incomplete_encrypted_input_stays_incomplete_and_never_uses_available_prefix() {
    let fixture = builder().write();
    let mut store =
        LocalStore::open_with_keys(&fixture.path, options(), keyring(NAME, KEY)).unwrap();
    let archive = fixture.path.join("Data/data/data.000");
    fs::OpenOptions::new()
        .write(true)
        .open(archive)
        .unwrap()
        .set_len(40)
        .unwrap();
    assert_eq!(rejection(&mut store).incomplete, 1);
}
#[test]
fn same_ckey_alternative_can_succeed_after_missing_key_but_cannot_change_content() {
    let mut b = builder();
    let alternative = [0x23; 16];
    b.routes[0].2.push(alternative);
    b.records.push((alternative, b'N', CONTENT.to_vec()));
    let fixture = b.write();
    let mut store =
        LocalStore::open_with_keys(&fixture.path, options(), keyring(NAME + 1, KEY)).unwrap();
    let file = store.read_file_data_id(FILE_ID).unwrap();
    assert_eq!(file.bytes, CONTENT);
    assert_eq!(file.candidate_attempts, 2);
}
