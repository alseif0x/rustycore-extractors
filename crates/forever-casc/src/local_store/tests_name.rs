// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: opt-in name reads over complete synthetic disk stores.

use super::*;

const NAME: &[u8] = b"World/Maps/Synthetic/Synthetic_0_0_obj0.adt";
const OTHER_NAME: &[u8] = b"World/Maps/Other/Other_0_0_obj0.adt";
const OTHER_CONTENT: &[u8] = b"synthetic later content must not replace the first CKey";
const OTHER_EKEY: [u8; 16] = [0x65; 16];
type NamedRow = (u32, u32, u32, [u8; 16], Option<u64>);

fn name_hash(name: &[u8]) -> u64 {
    root::name::hash_name(name, 1024).unwrap()
}

fn named_root(rows: &[NamedRow]) -> Vec<u8> {
    let mut bytes = b"TSFM".to_vec();
    let named = rows.iter().filter(|row| row.4.is_some()).count() as u32;
    for value in [20, 2, rows.len() as u32, named] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    for &(id, flags, locale, key, hash) in rows {
        let flags = (flags & !0x1000_0000) | if hash.is_none() { 0x1000_0000 } else { 0 };
        for value in [1, locale, flags, 0] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes.push(0);
        bytes.extend_from_slice(&id.to_le_bytes());
        bytes.extend_from_slice(&key);
        if let Some(hash) = hash {
            bytes.extend_from_slice(&hash.to_le_bytes());
        }
    }
    bytes
}

fn builder_named(rows: &[NamedRow]) -> Builder {
    let mut builder = Builder::new(CONTENT);
    builder.root_override = Some(named_root(rows));
    builder
}

fn basic() -> Fixture {
    builder_named(&[(FILE_ID, 0, 2, md5::digest(CONTENT), Some(name_hash(NAME)))]).write()
}

fn add_other(builder: &mut Builder) {
    builder.routes.push((
        md5::digest(OTHER_CONTENT),
        OTHER_CONTENT.len() as u32,
        vec![OTHER_EKEY],
    ));
    builder
        .records
        .push((OTHER_EKEY, b'N', OTHER_CONTENT.to_vec()));
}

#[test]
fn names_are_lazy_explicit_and_separately_identified() {
    let fixture = basic();
    let mut store = fixture.open();
    let before = store.stats();
    assert!(!store.names_enabled());
    assert_eq!(store.names_capability(), None);
    assert_eq!(store.names_stats(), None);
    assert_eq!(store.names_limits(), None);
    assert_eq!(store.read_name(NAME).unwrap_err(), Error::NamesDisabled);
    assert_eq!(store.stats(), before);
    assert_eq!(store.read_file_data_id(FILE_ID).unwrap().bytes, CONTENT);

    let stats = store.enable_names(root::name::Limits::default()).unwrap();
    assert!(store.names_enabled());
    assert_eq!(store.names_capability(), Some(NAME_CAPABILITY));
    assert_eq!(NAME_CAPABILITY, "linux-ascii-stable-root-names-v1");
    assert_eq!(store.names_stats(), Some(stats));
    assert_eq!(stats.names, 1);
    assert_eq!(stats.nodes, 1);
    assert_eq!(
        store.stats().resident_bytes,
        before.resident_bytes + stats.resident_bytes
    );
    assert!(stats.peak_bytes >= stats.resident_bytes);
    let file = store.read_name(NAME).unwrap();
    assert_eq!(file.file_data_id, FILE_ID);
    assert_eq!(file.content_key, md5::digest(CONTENT));
    assert_eq!(file.bytes, CONTENT);
}

#[test]
fn late_hash_binds_to_first_centrally_present_ckey_and_zero_preserves_it() {
    let first = md5::digest(CONTENT);
    let later = md5::digest(OTHER_CONTENT);
    let mut builder = builder_named(&[
        (FILE_ID, 0, 2, first, None),
        (FILE_ID, 0, 2, later, Some(name_hash(NAME))),
        (FILE_ID, 0, 2, later, Some(0)),
    ]);
    add_other(&mut builder);
    let fixture = builder.write();
    let mut store = fixture.open();
    store.enable_names(root::name::Limits::default()).unwrap();
    let named = store.read_name(NAME).unwrap();
    assert_eq!(named.bytes, CONTENT);
    assert_eq!(named.content_key, first);
    assert_eq!(store.read_file_data_id(FILE_ID).unwrap().content_key, first);
}

#[test]
fn name_bound_fdid_zero_and_ckey_aliases_do_not_roundtrip_through_fdid() {
    let key = md5::digest(CONTENT);
    let fixture = builder_named(&[
        (0, 0, 2, key, Some(name_hash(NAME))),
        (FILE_ID, 0, 2, key, Some(name_hash(OTHER_NAME))),
    ])
    .write();
    let mut store = fixture.open();
    let stats = store.enable_names(root::name::Limits::default()).unwrap();
    assert_eq!(stats.nodes, 2);
    assert_eq!(stats.names, 2);
    let zero = store.read_name(NAME).unwrap();
    let alias = store.read_name(OTHER_NAME).unwrap();
    assert_eq!(zero.file_data_id, 0);
    assert_eq!(alias.file_data_id, FILE_ID);
    assert_eq!(zero.content_key, alias.content_key);
    assert_eq!(zero.bytes, CONTENT);
    assert_eq!(store.read_file_data_id(0).unwrap_err(), Error::MissingFile);
    assert_eq!(store.read_file_data_id(FILE_ID).unwrap().bytes, CONTENT);
}

#[test]
fn incomplete_first_ckey_does_not_fall_through_to_later_named_row() {
    let mut builder = builder_named(&[
        (FILE_ID, 0, 2, md5::digest(CONTENT), None),
        (
            FILE_ID,
            0,
            2,
            md5::digest(OTHER_CONTENT),
            Some(name_hash(NAME)),
        ),
    ]);
    add_other(&mut builder);
    let mut fixture = builder.write();
    let archive_len = fs::metadata(fixture.path.join("Data/data/data.000"))
        .unwrap()
        .len() as u32;
    fixture.rows[0][14..].copy_from_slice(&(archive_len + 1).to_le_bytes());
    fixture.rewrite_rows();
    let mut store = fixture.open();
    store.enable_names(root::name::Limits::default()).unwrap();
    let expected = Error::NoLocalEncoding(CandidateFailures {
        incomplete: 1,
        ..CandidateFailures::default()
    });
    assert_eq!(store.read_name(NAME).unwrap_err(), expected);
    assert_eq!(store.read_file_data_id(FILE_ID).unwrap_err(), expected);
}

#[test]
fn same_ckey_encoding_alternatives_and_md5_verification_are_shared() {
    let mut builder =
        builder_named(&[(FILE_ID, 0, 2, md5::digest(CONTENT), Some(name_hash(NAME)))]);
    builder.routes[0].2.insert(0, [0x66; 16]);
    builder
        .records
        .insert(0, (EKEY, b'N', vec![b'x'; CONTENT.len()]));
    let fixture = builder.write();
    let mut store = fixture.open();
    store.enable_names(root::name::Limits::default()).unwrap();
    let named = store.read_name(NAME).unwrap();
    let numeric = store.read_file_data_id(FILE_ID).unwrap();
    assert_eq!(named.bytes, numeric.bytes);
    assert_eq!(named.bytes, CONTENT);
    assert_eq!(named.candidate_attempts, 3);
    assert_eq!(named.candidate_attempts, numeric.candidate_attempts);
}

#[test]
fn absent_central_ckey_is_skipped_before_name_conflict_and_selection() {
    let fixture = builder_named(&[
        (FILE_ID, 0, 2, [0x79; 16], Some(name_hash(OTHER_NAME))),
        (FILE_ID, 0, 2, md5::digest(CONTENT), Some(name_hash(NAME))),
    ])
    .write();
    let mut store = fixture.open();
    let stats = store.enable_names(root::name::Limits::default()).unwrap();
    assert_eq!(stats.central_entries, 1);
    assert_eq!(store.read_name(NAME).unwrap().bytes, CONTENT);
    assert_eq!(store.read_name(OTHER_NAME).unwrap_err(), Error::MissingFile);
}

#[test]
fn name_conflict_is_atomic_and_does_not_disable_fdid() {
    for rows in [
        vec![
            (FILE_ID, 0, 2, md5::digest(CONTENT), Some(name_hash(NAME))),
            (
                FILE_ID,
                0,
                2,
                md5::digest(CONTENT),
                Some(name_hash(OTHER_NAME)),
            ),
        ],
        vec![
            (0, 0, 2, md5::digest(CONTENT), None),
            (FILE_ID, 0, 2, md5::digest(CONTENT), Some(name_hash(NAME))),
            (0, 0, 2, md5::digest(CONTENT), Some(name_hash(NAME))),
        ],
    ] {
        let fixture = builder_named(&rows).write();
        let mut store = fixture.open();
        let before = store.stats();
        assert_eq!(
            store
                .enable_names(root::name::Limits::default())
                .unwrap_err(),
            Error::Name(root::name::Error::NameConflict)
        );
        assert!(!store.names_enabled());
        assert_eq!(store.names_stats(), None);
        assert_eq!(store.names_limits(), None);
        assert_eq!(store.stats(), before);
        assert_eq!(store.read_name(NAME).unwrap_err(), Error::NamesDisabled);
        assert_eq!(store.read_file_data_id(FILE_ID).unwrap().bytes, CONTENT);
    }
}

#[test]
fn failed_enable_can_retry_but_successful_enable_cannot_change_options() {
    let fixture = basic();
    let mut store = fixture.open();
    let before = store.stats();
    assert_eq!(
        store
            .enable_names(root::name::Limits {
                max_names: 0,
                ..root::name::Limits::default()
            })
            .unwrap_err(),
        Error::Name(root::name::Error::Limit)
    );
    assert_eq!(store.stats(), before);
    assert!(!store.names_enabled());
    assert_eq!(store.read_file_data_id(FILE_ID).unwrap().bytes, CONTENT);
    let requested = root::name::Limits::default();
    let first = store.enable_names(requested).unwrap();
    let charged = store.stats();
    assert_eq!(store.enable_names(requested).unwrap(), first);
    assert_eq!(store.stats(), charged);
    let reduced = root::name::Limits {
        max_name_bytes: 100,
        ..requested
    };
    assert_eq!(
        store.enable_names(reduced).unwrap_err(),
        Error::NamesAlreadyEnabled
    );
    assert_eq!(store.stats(), charged);
    assert_eq!(store.names_stats(), Some(first));
    assert_eq!(store.read_name(NAME).unwrap().bytes, CONTENT);
}

#[test]
fn construction_respects_each_preallocation_memory_budget() {
    let fixture = basic();
    let mut measured = fixture.open();
    let base = measured.stats().resident_bytes;
    let peak = measured
        .enable_names(root::name::Limits::default())
        .unwrap()
        .peak_bytes;
    assert!(peak > 0);
    for policy in 0..4 {
        for admitted in [false, true] {
            let mut store = fixture.open();
            let allowance = peak - usize::from(!admitted);
            let mut requested = root::name::Limits::default();
            match policy {
                0 => store.limits.max_resident_bytes = base + allowance,
                1 => store.limits.max_scratch_bytes = allowance,
                2 => store.limits.max_memory_bytes = base + allowance,
                _ => requested.max_memory_bytes = allowance,
            }
            let before = store.stats();
            if admitted {
                let stats = store.enable_names(requested).unwrap();
                assert_eq!(store.names_limits().unwrap().max_memory_bytes, allowance);
                assert_eq!(store.stats().resident_bytes, base + stats.resident_bytes);
            } else {
                assert_eq!(
                    store.enable_names(requested).unwrap_err(),
                    Error::Name(root::name::Error::Limit)
                );
                assert_eq!(store.stats(), before);
                assert!(!store.names_enabled());
            }
        }
    }
}

#[test]
fn invalid_enable_options_win_before_guard_io() {
    let fixture = basic();
    let mut store = fixture.open();
    fs::remove_file(fixture.path.join(".build.info")).unwrap();
    let invalid = root::name::Limits {
        max_name_bytes: 1025,
        ..root::name::Limits::default()
    };
    assert_eq!(
        store.enable_names(invalid).unwrap_err(),
        Error::Name(root::name::Error::InvalidLimits)
    );
    assert_eq!(
        store
            .enable_names(root::name::Limits::default())
            .unwrap_err(),
        Error::InputChanged
    );
    assert!(!store.names_enabled());
}

#[test]
fn enable_reenable_and_reads_preserve_input_changed_variant() {
    for mode in 0..3 {
        let fixture = basic();
        let mut store = fixture.open();
        let limits = root::name::Limits::default();
        if mode != 0 {
            store.enable_names(limits).unwrap();
        }
        fs::write(&fixture.config_path, b"synthetic changed config").unwrap();
        let error = if mode == 2 {
            store.read_name(NAME).unwrap_err()
        } else {
            store.enable_names(limits).unwrap_err()
        };
        assert_eq!(error, Error::InputChanged);
        assert!(crate::Error::LocalStore(error).is_input_changed());
        if mode == 0 {
            assert!(!store.names_enabled());
        }
    }
}

#[test]
fn name_reads_use_remaining_cap_without_widening_store_policy() {
    let fixture = basic();
    let mut store = fixture.open();
    store.enable_names(root::name::Limits::default()).unwrap();
    let size = CONTENT.len() as u64;
    assert_eq!(
        store.read_name_with_limit(NAME, size - 1).unwrap_err(),
        Error::Limit
    );
    assert_eq!(
        store.read_name_with_limit(NAME, size).unwrap().bytes,
        CONTENT
    );
    store.limits.max_decoded_file_bytes = size - 1;
    assert_eq!(
        store.read_name_with_limit(NAME, u64::MAX).unwrap_err(),
        Error::Limit
    );
    assert_eq!(store.read_file_data_id(FILE_ID).unwrap_err(), Error::Limit);
}

#[test]
fn names_normalize_only_ascii_case_and_slashes_and_never_fallback() {
    let fixture = basic();
    let mut store = fixture.open();
    store.enable_names(root::name::Limits::default()).unwrap();
    assert_eq!(
        store
            .read_name(b"WORLD\\MAPS\\SYNTHETIC\\SYNTHETIC_0_0_OBJ0.ADT")
            .unwrap()
            .bytes,
        CONTENT
    );
    for unknown in [
        b"42".as_slice(),
        b"FILE0000002A.xxx",
        b"World/Maps/../Maps/Synthetic/Synthetic_0_0_obj0.adt",
    ] {
        assert_eq!(store.read_name(unknown).unwrap_err(), Error::MissingFile);
    }
    for invalid in [b"".as_slice(), b"x\0y", &[0xff]] {
        assert_eq!(
            store.read_name(invalid).unwrap_err(),
            Error::Name(root::name::Error::InvalidName)
        );
    }
    assert_eq!(
        store.read_name(b"root").unwrap_err(),
        Error::Name(root::name::Error::ReservedName)
    );
    assert_eq!(store.read_file_data_id(FILE_ID).unwrap().bytes, CONTENT);
}

#[test]
fn names_preserve_locale_and_audio_selection_without_cross_locale_fallback() {
    let first = md5::digest(CONTENT);
    let later = md5::digest(OTHER_CONTENT);
    let mut builder = builder_named(&[
        (FILE_ID, 0x8000_0000, 2, later, Some(name_hash(NAME))),
        (FILE_ID, 0, 2, first, Some(name_hash(NAME))),
        (FILE_ID + 1, 0, 0x80, later, Some(name_hash(OTHER_NAME))),
    ]);
    add_other(&mut builder);
    let fixture = builder.write();
    let mut store = fixture.open();
    store.enable_names(root::name::Limits::default()).unwrap();
    assert_eq!(store.read_name(NAME).unwrap().bytes, CONTENT);
    assert_eq!(store.read_name(OTHER_NAME).unwrap_err(), Error::MissingFile);
    let mut spanish = LocalStore::open(
        &fixture.path,
        OpenOptions {
            locale: Locale::EsEs,
            ..options()
        },
    )
    .unwrap();
    spanish.enable_names(root::name::Limits::default()).unwrap();
    assert_eq!(spanish.read_name(NAME).unwrap_err(), Error::MissingFile);
    assert_eq!(spanish.read_name(OTHER_NAME).unwrap().bytes, OTHER_CONTENT);
}
