use super::*;
use crate::local_store::availability::{Error as AvailableError, RangeError};

fn encrypted_record(tag: u8) -> Vec<u8> {
    let mut p = vec![8];
    p.extend([0x11; 8]);
    p.push(4);
    p.extend([0; 4]);
    p.extend([b'S', tag]);
    p
}
fn exact_ekey(payload: &[u8]) -> [u8; 16] {
    let mut encoded = b"BLTE\0\0\0\0E".to_vec();
    encoded.extend(payload);
    md5::digest(&encoded)
}
fn partial_builder() -> Builder {
    let mut b = Builder::new(CONTENT);
    b.version = "1.60.1.70374";
    let payload = encrypted_record(0x22);
    let ekey = exact_ekey(&payload);
    b.records[0] = (ekey, b'E', payload);
    b.routes[0].2 = vec![ekey];
    b
}
fn available_store(f: &Fixture) -> LocalStore {
    LocalStore::open_db2_70374(&f.path, options()).unwrap()
}

#[test]
fn strict_api_stays_failed_and_partial_api_exposes_no_hole_bytes() {
    let f = partial_builder().write();
    let mut s = available_store(&f);
    assert!(matches!(
        s.read_file_data_id(FILE_ID),
        Err(Error::NoLocalEncoding(_))
    ));
    let a = s
        .read_file_data_id_available_with_limit(FILE_ID, 1024)
        .unwrap();
    assert_eq!(a.file_data_id(), FILE_ID);
    assert_eq!(a.declared_bytes(), CONTENT.len() as u64);
    assert_eq!(a.available_bytes(), 0);
    assert_eq!(a.range(0, 1), Err(RangeError::Unavailable));
    assert!(!a.full_ckey_verified());
    assert!(a.encoded_key_authenticated());
    assert_eq!(a.work().strict_candidate_attempts, 1);
    assert_eq!(a.work().partial_candidate_attempts, 1);
    assert!(a.work().partial_encoded_bytes_read > 30);
    s.verify_unchanged().unwrap();
}
#[test]
fn complete_source_uses_original_ckey_verified_path() {
    let mut b = Builder::new(CONTENT);
    b.version = "1.60.1.70374";
    let f = b.write();
    let mut s = available_store(&f);
    let a = s
        .read_file_data_id_available_with_limit(FILE_ID, 1024)
        .unwrap();
    assert!(a.full_ckey_verified());
    assert!(!a.encoded_key_authenticated()); // Not invented from the old prefix route.
    assert_eq!(a.range(0, CONTENT.len()), Ok(CONTENT));
    assert_eq!(a.work().partial_candidate_attempts, 0);
}
#[test]
fn full_ekey_suffix_mismatch_is_fatal_even_when_index_prefix_matches() {
    let mut b = partial_builder();
    b.records[0].0[15] ^= 1;
    b.routes[0].2[0][15] ^= 1;
    let f = b.write();
    assert!(matches!(
        available_store(&f).read_file_data_id_available_with_limit(FILE_ID, 1024),
        Err(AvailableError::Blte(crate::blte::Error::Checksum))
    ));
}
#[test]
fn two_authenticated_encodings_are_not_merged_or_ranked() {
    let mut b = partial_builder();
    let p = encrypted_record(0x33);
    let ekey = exact_ekey(&p);
    b.routes[0].2.push(ekey);
    b.records.push((ekey, b'E', p));
    let f = b.write();
    assert!(matches!(
        available_store(&f).read_file_data_id_available_with_limit(FILE_ID, 1024),
        Err(AvailableError::AmbiguousEncoding)
    ));
}
#[test]
fn malformed_encrypted_wrapper_is_not_an_omission() {
    let mut b = partial_builder();
    b.records[0].2 = vec![7];
    b.records[0].0 = exact_ekey(&b.records[0].2);
    b.routes[0].2[0] = b.records[0].0;
    let f = b.write();
    assert!(matches!(
        available_store(&f).read_file_data_id_available_with_limit(FILE_ID, 1024),
        Err(AvailableError::Blte(crate::blte::Error::EncryptionHeader))
    ));
}
#[test]
fn downward_file_and_cumulative_candidate_limits_remain_enforced() {
    let f = partial_builder().write();
    let mut s = available_store(&f);
    assert!(matches!(
        s.read_file_data_id_available_with_limit(FILE_ID, 1),
        Err(AvailableError::Store(Error::Limit))
    ));
    s.limits.max_candidate_attempts = 1; // Strict+partial must not reset the ledger.
    assert!(matches!(
        s.read_file_data_id_available_with_limit(FILE_ID, 1024),
        Err(AvailableError::Store(Error::Limit))
    ));
}
#[test]
fn changed_sources_and_keys_reject_without_available_prefix() {
    let f = partial_builder().write();
    let mut s = available_store(&f);
    fs::write(&f.config_path, b"changed fake config").unwrap();
    assert!(matches!(
        s.read_file_data_id_available_with_limit(FILE_ID, 1024),
        Err(AvailableError::Store(Error::InputChanged))
    ));
    let f = partial_builder().write();
    let mut s = available_store(&f);
    s.keys = Some(
        crate::keys::Keyring::parse(
            b"forever-tact-keys\t1\n0102030405060708\t11111111111111111111111111111111\n",
        )
        .unwrap(),
    );
    assert!(matches!(
        s.read_file_data_id_available_with_limit(FILE_ID, 1024),
        Err(AvailableError::KeysPresent)
    ));
}
#[test]
fn other_builds_cannot_implicitly_use_new_profile() {
    let f = Builder::new(CONTENT).write();
    assert!(matches!(
        f.open()
            .read_file_data_id_available_with_limit(FILE_ID, 1024),
        Err(AvailableError::Build)
    ));
}

#[test]
fn external_live_budget_constrains_strict_and_partial_before_allocation() {
    for partial in [false, true] {
        let mut b = if partial {
            partial_builder()
        } else {
            Builder::new(CONTENT)
        };
        b.version = "1.60.1.70374";
        let f = b.write();
        let mut s = available_store(&f);
        let resident = s.stats().resident_bytes;
        for (external, cap) in [(1, resident), (usize::MAX, resident), (0, resident)] {
            assert!(matches!(
                s.read_file_data_id_available_with_external_budget(FILE_ID, 1024, external, cap),
                Err(AvailableError::Store(Error::Limit))
            ));
            s.verify_unchanged().unwrap();
        }
        assert!(
            s.read_file_data_id_available_with_external_budget(
                FILE_ID,
                1024,
                1024,
                Limits::default().max_memory_bytes
            )
            .is_ok()
        );
    }
}
