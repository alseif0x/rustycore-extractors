// SPDX-License-Identifier: GPL-3.0-or-later
// Authored Linux70425 bootstrap tests; no client inputs.
use super::*;

fn fixture(version: &'static str) -> Fixture {
    Builder {
        version,
        ..Builder::new(CONTENT)
    }
    .write()
}

fn supplied_keys() -> crate::keys::Keyring {
    crate::keys::Keyring::parse(
        b"forever-tact-keys\t1\n0000000000000001\t00000000000000000000000000000000\n",
    )
    .unwrap()
}

#[test]
fn scoped70425_opens_exact_metadata_and_reads_complete_authored_content() {
    let fixture = fixture("1.60.1.70425");
    assert_eq!(crate::admit_db2_70425(&fixture.path), Ok(70425));
    let mut store = LocalStore::open_db2_70425(&fixture.path, options()).unwrap();
    assert_eq!(store.build().product, "wow_classic_beta");
    assert_eq!(store.build().version, "1.60.1.70425");
    assert_eq!(store.build().build, 70425);
    assert_eq!(store.build().tags, "enUS");
    assert_eq!(
        store.build().config_build_uid.as_deref(),
        Some("wow_classic_beta")
    );
    assert_eq!(
        store.build().config_build_name.as_deref(),
        Some("synthetic-1.60.1.70425")
    );
    assert_eq!(
        store
            .read_file_data_id_with_limit(FILE_ID, CONTENT.len() as u64)
            .unwrap()
            .bytes,
        CONTENT
    );
    assert_eq!(store.verify_unchanged(), Ok(()));
}

#[test]
fn scoped70425_does_not_open_generic_or70374_keyed_or_unkeyed_selectors() {
    let fixture = fixture("1.60.1.70425");
    fs::write(&fixture.config_path, b"poison config not to read").unwrap();
    assert_eq!(
        crate::admit(&fixture.path),
        Err(crate::Error::UnsupportedClient)
    );
    assert_eq!(
        crate::admit_db2_70374(&fixture.path),
        Err(crate::Error::UnsupportedClient)
    );
    assert_eq!(fixture.open_error(options()), Error::UnsupportedClient);
    assert_eq!(
        LocalStore::open_with_keys(&fixture.path, options(), supplied_keys())
            .err()
            .unwrap(),
        Error::UnsupportedClient
    );
    assert_eq!(
        LocalStore::open_db2_70374(&fixture.path, options())
            .err()
            .unwrap(),
        Error::UnsupportedClient
    );
    assert_eq!(
        LocalStore::open_db2_70374_with_keys(&fixture.path, options(), supplied_keys())
            .err()
            .unwrap(),
        Error::UnsupportedClient
    );
}

#[test]
fn scoped70425_rejects_all_old_nearby_and_unknown_versions_before_config_io() {
    for version in [
        "1.60.1.70170",
        "1.60.1.70205",
        "1.60.1.70338",
        "1.60.1.70374",
        "1.60.1.70424",
        "1.60.1.70426",
        "1.60.1.99999",
        "1.60.2.70425",
        "1.60.1.070425",
    ] {
        let fixture = fixture(version);
        fs::write(&fixture.config_path, b"poison config not to read").unwrap();
        assert_eq!(
            LocalStore::open_db2_70425(&fixture.path, options())
                .err()
                .unwrap(),
            Error::UnsupportedClient,
            "{version}"
        );
        assert_eq!(
            crate::admit_db2_70425(&fixture.path),
            Err(crate::Error::UnsupportedClient),
            "{version}"
        );
    }
}

#[test]
fn scoped70425_keeps_old_success_routes_available() {
    for (version, build) in [
        ("1.60.1.70170", 70170),
        ("1.60.1.70205", 70205),
        ("1.60.1.70338", 70338),
    ] {
        let fixture = fixture(version);
        assert_eq!(crate::admit(&fixture.path), Ok(build));
        let mut store = LocalStore::open(&fixture.path, options()).unwrap();
        assert_eq!(store.build().build, build);
        assert_eq!(store.read_file_data_id(FILE_ID).unwrap().bytes, CONTENT);
        let mut keyed =
            LocalStore::open_with_keys(&fixture.path, options(), supplied_keys()).unwrap();
        assert_eq!(keyed.build().build, build);
        assert_eq!(keyed.read_file_data_id(FILE_ID).unwrap().bytes, CONTENT);
    }
    let fixture = fixture("1.60.1.70374");
    assert_eq!(crate::admit_db2_70374(&fixture.path), Ok(70374));
    let mut store = LocalStore::open_db2_70374(&fixture.path, options()).unwrap();
    assert_eq!(store.read_file_data_id(FILE_ID).unwrap().bytes, CONTENT);
    let mut keyed =
        LocalStore::open_db2_70374_with_keys(&fixture.path, options(), supplied_keys()).unwrap();
    assert_eq!(keyed.build().build, 70374);
    assert_eq!(keyed.read_file_data_id(FILE_ID).unwrap().bytes, CONTENT);
}

#[test]
fn scoped70425_rejects_malformed_typed_metadata_before_config_io() {
    let fixture = fixture("1.60.1.70425");
    let original = fs::read_to_string(fixture.path.join(".build.info")).unwrap();
    let build_key = hex(md5::digest(&fs::read(&fixture.config_path).unwrap()));
    fs::write(&fixture.config_path, b"poison config not to read").unwrap();
    for altered in [
        original.replace("Build Key!HEX:16", "Build Key!STRING:0"),
        original.replace("Version!STRING:0", "Version!STRING:0|Version!STRING:0"),
        original.replace("|1|", "|2|"),
        original.replace(&build_key, "invalid"),
        original.replace("|enUS\n", "|xx\n"),
        original.replace("|enUS\n", "|en\0US\n"),
        String::new(),
    ] {
        fs::write(fixture.path.join(".build.info"), altered).unwrap();
        assert_eq!(
            LocalStore::open_db2_70425(&fixture.path, options())
                .err()
                .unwrap(),
            Error::BuildMetadata
        );
    }
}

#[test]
fn scoped70425_rejects_ambiguous_wrong_product_and_inactive_metadata() {
    let fixture = fixture("1.60.1.70425");
    let original = fs::read_to_string(fixture.path.join(".build.info")).unwrap();
    let row = original.lines().nth(1).unwrap();
    fs::write(&fixture.config_path, b"poison config not to read").unwrap();
    fs::write(
        fixture.path.join(".build.info"),
        format!("{original}{row}\n"),
    )
    .unwrap();
    assert_eq!(
        LocalStore::open_db2_70425(&fixture.path, options())
            .err()
            .unwrap(),
        Error::AmbiguousClient
    );
    assert_eq!(
        crate::admit_db2_70425(&fixture.path),
        Err(crate::Error::AmbiguousClient)
    );
    for altered in [
        original.replace("wow_classic_beta", "wow_classic_era"),
        original.replace("|1|", "|0|"),
    ] {
        fs::write(fixture.path.join(".build.info"), altered).unwrap();
        assert_eq!(
            LocalStore::open_db2_70425(&fixture.path, options())
                .err()
                .unwrap(),
            Error::UnsupportedClient
        );
        assert_eq!(
            crate::admit_db2_70425(&fixture.path),
            Err(crate::Error::UnsupportedClient)
        );
    }
}

#[test]
fn scoped70425_detects_same_length_metadata_rewrite_with_restored_mtime() {
    let fixture = fixture("1.60.1.70425");
    let mut store = LocalStore::open_db2_70425(&fixture.path, options()).unwrap();
    let path = fixture.path.join(".build.info");
    let modified = fs::metadata(&path).unwrap().modified().unwrap();
    let bytes = fs::read_to_string(&path)
        .unwrap()
        .replace("|enUS\n", "|esES\n");
    fs::write(&path, bytes).unwrap();
    fs::File::options()
        .write(true)
        .open(&path)
        .unwrap()
        .set_times(fs::FileTimes::new().set_modified(modified))
        .unwrap();
    // A valid FDID checks the original source before its per-read byte limit.
    assert_eq!(store.verify_unchanged(), Err(Error::InputChanged));
    assert_eq!(
        store.read_file_data_id_with_limit(FILE_ID, 0).unwrap_err(),
        Error::InputChanged
    );
}

#[test]
fn scoped70425_keeps_config_and_index_source_identity_guards() {
    for variant in 0..2 {
        let fixture = fixture("1.60.1.70425");
        let mut store = LocalStore::open_db2_70425(&fixture.path, options()).unwrap();
        let changed = if variant == 0 {
            fixture.config_path.clone()
        } else {
            fixture.index_path(0)
        };
        fs::write(changed, b"changed authored metadata").unwrap();
        assert_eq!(store.verify_unchanged(), Err(Error::InputChanged));
        assert_eq!(
            store.read_file_data_id(FILE_ID).unwrap_err(),
            Error::InputChanged
        );
    }
}

#[test]
fn scoped70425_read_limits_tighten_and_cannot_enlarge_store_policy() {
    let fixture = fixture("1.60.1.70425");
    let mut store = LocalStore::open_db2_70425(&fixture.path, options()).unwrap();
    assert_eq!(
        store.read_file_data_id_with_limit(FILE_ID, 1).unwrap_err(),
        Error::Limit
    );
    assert_eq!(
        store
            .read_file_data_id_with_limit(FILE_ID, CONTENT.len() as u64)
            .unwrap()
            .bytes,
        CONTENT
    );
    let mut opts = options();
    opts.limits.max_decoded_file_bytes = 1;
    let mut store = LocalStore::open_db2_70425(&fixture.path, opts).unwrap();
    assert_eq!(
        store
            .read_file_data_id_with_limit(FILE_ID, u64::MAX)
            .unwrap_err(),
        Error::Limit
    );
}

#[test]
fn scoped70425_downward_open_limits_reject_without_rebaseline() {
    let fixture = fixture("1.60.1.70425");
    for variant in 0..9 {
        let mut opts = options();
        match variant {
            0 => opts.limits.max_resident_bytes = 0,
            1 => opts.limits.max_scratch_bytes = 1,
            2 => opts.limits.max_memory_bytes = 1,
            3 => opts.limits.max_build_info_bytes = 1,
            4 => opts.limits.max_build_info_rows = 0,
            5 => opts.limits.max_index_files = 15,
            6 => opts.limits.index.max_entries = 17,
            7 => opts.limits.max_path_bytes = 1,
            _ => opts.limits.max_archive_span_bytes = 1,
        }
        let error = LocalStore::open_db2_70425(&fixture.path, opts)
            .err()
            .unwrap();
        assert!(
            matches!(
                error,
                Error::Limit | Error::Index(local_index::Error::Limit)
            ),
            "{variant}: {error:?}"
        );
    }
}

#[test]
fn scoped70425_bootstrap_does_not_admit_existing70374_available_profile() {
    let fixture = fixture("1.60.1.70425");
    let mut store = LocalStore::open_db2_70425(&fixture.path, options()).unwrap();
    assert_eq!(
        store
            .read_file_data_id_available_with_limit(FILE_ID, 1000)
            .err()
            .unwrap(),
        availability::Error::Build
    );
}
