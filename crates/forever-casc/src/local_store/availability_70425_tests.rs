// SPDX-License-Identifier: GPL-3.0-or-later
//! Authored 2026-10-16: synthetic70425 stores; no client inputs.
use super::*;

fn builder70425(partial: bool) -> Builder {
    let mut builder = if partial {
        partial_builder()
    } else {
        Builder::new(CONTENT)
    };
    builder.version = "1.60.1.70425";
    builder
}
fn store70425(fixture: &Fixture) -> LocalStore {
    LocalStore::open_db2_70425(&fixture.path, options()).unwrap()
}
fn keys70425() -> crate::keys::Keyring {
    // Authored dummy key: no actual client/provider material.
    crate::keys::Keyring::parse(
        b"forever-tact-keys\t1\n0000000000000001\t00000000000000000000000000000000\n",
    )
    .unwrap()
}

#[test]
fn available70425_complete_source_retains_ckey_and_profile_separation() {
    assert_eq!(
        availability::PROFILE,
        "linux70374-disabled-key-available-source-v1"
    );
    assert_eq!(
        availability::PROFILE_70425,
        "linux70425-disabled-key-available-source-v1"
    );
    let fixture = builder70425(false).write();
    let mut store = store70425(&fixture);
    let source = store
        .read_file_data_id_available_70425_with_limit(FILE_ID, 1024)
        .unwrap();
    assert_eq!(source.file_data_id(), FILE_ID);
    assert_eq!(source.range(0, CONTENT.len()), Ok(CONTENT));
    assert_eq!(source.declared_bytes(), CONTENT.len() as u64);
    assert!(source.full_ckey_verified());
    assert!(!source.encoded_key_authenticated());
    assert_eq!(source.work().partial_candidate_attempts, 0);
    store.verify_unchanged().unwrap();
}
#[test]
fn available70425_authenticated_hole_never_exposes_plaintext() {
    let fixture = builder70425(true).write();
    let mut store = store70425(&fixture);
    assert!(matches!(
        store.read_file_data_id(FILE_ID),
        Err(Error::NoLocalEncoding(_))
    ));
    let source = store
        .read_file_data_id_available_70425_with_limit(FILE_ID, 1024)
        .unwrap();
    assert_eq!(source.declared_bytes(), CONTENT.len() as u64);
    assert_eq!(source.available_bytes(), 0);
    assert_eq!(source.range(0, 1), Err(RangeError::Unavailable));
    assert!(!source.full_ckey_verified());
    assert!(source.encoded_key_authenticated());
    assert_eq!(source.work().strict_candidate_attempts, 1);
    assert_eq!(source.work().partial_candidate_attempts, 1);
    store.verify_unchanged().unwrap();
}
#[test]
fn available70425_and70374_routes_reject_cross_build_before_keys_and_guards() {
    let fixture = builder70425(false).write();
    let mut store = store70425(&fixture);
    store.keys = Some(keys70425());
    fs::write(&fixture.config_path, b"changed authored config").unwrap();
    assert!(matches!(
        store.read_file_data_id_available_with_limit(FILE_ID, 1024),
        Err(AvailableError::Build)
    ));
    assert!(matches!(
        store.read_file_data_id_available_with_external_budget(FILE_ID, 1024, usize::MAX, 0),
        Err(AvailableError::Build)
    ));
    let mut builder = Builder::new(CONTENT);
    builder.version = "1.60.1.70374";
    let fixture = builder.write();
    let mut store = available_store(&fixture);
    store.keys = Some(keys70425());
    fs::write(&fixture.config_path, b"changed authored config").unwrap();
    assert!(matches!(
        store.read_file_data_id_available_70425_with_limit(FILE_ID, 1024),
        Err(AvailableError::Build)
    ));
    assert!(matches!(
        store.read_file_data_id_available_70425_with_external_budget(FILE_ID, 1024, usize::MAX, 0),
        Err(AvailableError::Build)
    ));
}
#[test]
fn available70425_old_generic_build_stays_closed() {
    let fixture = Builder::new(CONTENT).write();
    assert!(matches!(
        fixture
            .open()
            .read_file_data_id_available_70425_with_limit(FILE_ID, 1024),
        Err(AvailableError::Build)
    ));
}
#[test]
fn available70425_keys_reject_before_changed_source_or_limit() {
    let fixture = builder70425(true).write();
    let mut store = store70425(&fixture);
    store.keys = Some(keys70425());
    fs::write(&fixture.config_path, b"changed authored config").unwrap();
    assert!(matches!(
        store.read_file_data_id_available_70425_with_external_budget(FILE_ID, 1, usize::MAX, 0),
        Err(AvailableError::KeysPresent)
    ));
}
#[test]
fn available70425_source_identity_change_is_fatal_for_full_and_partial() {
    for partial in [false, true] {
        let fixture = builder70425(partial).write();
        let mut store = store70425(&fixture);
        fs::write(&fixture.config_path, b"changed authored config").unwrap();
        assert!(matches!(
            store.read_file_data_id_available_70425_with_limit(FILE_ID, 1024),
            Err(AvailableError::Store(Error::InputChanged))
        ));
    }
}
#[test]
fn available70425_external_budget_bounds_both_paths_without_increasing_store_cap() {
    for partial in [false, true] {
        let fixture = builder70425(partial).write();
        let mut store = store70425(&fixture);
        let resident = store.stats().resident_bytes;
        for (external, cap) in [(1, resident), (usize::MAX, resident), (0, resident)] {
            assert!(matches!(
                store.read_file_data_id_available_70425_with_external_budget(
                    FILE_ID, 1024, external, cap
                ),
                Err(AvailableError::Store(Error::Limit))
            ));
            store.verify_unchanged().unwrap();
        }
        assert!(
            store
                .read_file_data_id_available_70425_with_external_budget(
                    FILE_ID,
                    1024,
                    1024,
                    Limits::default().max_memory_bytes
                )
                .is_ok()
        );
        assert!(
            store
                .read_file_data_id_available_70425_with_external_budget(
                    FILE_ID,
                    1024,
                    1024,
                    usize::MAX
                )
                .is_ok()
        );
    }
}
#[test]
fn available70425_decoded_and_strict_plus_partial_attempt_caps_remain_downward() {
    let fixture = builder70425(true).write();
    let mut store = store70425(&fixture);
    assert!(matches!(
        store.read_file_data_id_available_70425_with_limit(FILE_ID, 1),
        Err(AvailableError::Store(Error::Limit))
    ));
    store.limits.max_candidate_attempts = 1;
    assert!(matches!(
        store.read_file_data_id_available_70425_with_limit(FILE_ID, 1024),
        Err(AvailableError::Store(Error::Limit))
    ));
}
#[test]
fn available70425_full_ekey_checksum_stays_required() {
    let mut builder = builder70425(true);
    builder.records[0].0[15] ^= 1;
    builder.routes[0].2[0][15] ^= 1;
    let fixture = builder.write();
    assert!(matches!(
        store70425(&fixture).read_file_data_id_available_70425_with_limit(FILE_ID, 1024),
        Err(AvailableError::Blte(crate::blte::Error::Checksum))
    ));
}
#[test]
fn available70425_authenticated_encodings_remain_ambiguous_not_merged() {
    let mut builder = builder70425(true);
    let payload = encrypted_record(0x33);
    let ekey = exact_ekey(&payload);
    builder.routes[0].2.push(ekey);
    builder.records.push((ekey, b'E', payload));
    let fixture = builder.write();
    assert!(matches!(
        store70425(&fixture).read_file_data_id_available_70425_with_limit(FILE_ID, 1024),
        Err(AvailableError::AmbiguousEncoding)
    ));
}
#[test]
fn available70425_malformed_encryption_is_fatal_not_hole() {
    let mut builder = builder70425(true);
    builder.records[0].2 = vec![7];
    builder.records[0].0 = exact_ekey(&builder.records[0].2);
    builder.routes[0].2[0] = builder.records[0].0;
    let fixture = builder.write();
    assert!(matches!(
        store70425(&fixture).read_file_data_id_available_70425_with_limit(FILE_ID, 1024),
        Err(AvailableError::Blte(crate::blte::Error::EncryptionHeader))
    ));
}
#[test]
fn available70425_non_encryption_failure_and_missing_id_remain_errors() {
    let mut builder = builder70425(false);
    builder.records[0].2[0] ^= 1;
    let fixture = builder.write();
    let mut store = store70425(&fixture);
    assert!(matches!(
        store.read_file_data_id_available_70425_with_limit(FILE_ID, 1024),
        Err(AvailableError::Store(Error::NoLocalEncoding(_)))
    ));
    for id in [0, u32::MAX, FILE_ID + 1] {
        assert!(matches!(
            store.read_file_data_id_available_70425_with_limit(id, 1024),
            Err(AvailableError::Store(Error::MissingFile))
        ));
    }
}
