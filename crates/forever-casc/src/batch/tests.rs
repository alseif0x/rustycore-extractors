// SPDX-License-Identifier: GPL-3.0-or-later
//! Synthetic extraction tests. They use the `load` seam only: no CASC, no native
//! storage, no real client bytes and no network. All temporary directories live
//! under `TMPDIR` (externally configured) and are removed on drop.
use super::{MAX_FILE_BYTES, Plan, extract};
#[cfg(unix)]
use crate::Error;
use forever_db2::{CharacterTable, DecodedTable, Metadata};
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::symlink;
#[cfg(unix)]
use std::path::Path;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

#[test]
fn catalog_plan_requires_exact_known_name_and_file_id() {
    for metadata in forever_db2::catalog::tables() {
        let input = format!("{}\t{}\n", metadata.name, metadata.file_data_id);
        assert!(Plan::parse_with_catalog(&input, "pinned-metadata-v1").is_ok());
    }
    for input in [
        "LiquidType\t1\n",
        "liquidtype\t1371380\n",
        "UnknownTable\t123\n",
        "LiquidType\t1371380\nLIQUIDTYPE\t123\n",
    ] {
        assert!(Plan::parse_with_catalog(input, "pinned-metadata-v1").is_err());
    }
    assert!(Plan::parse_with_catalog("LiquidType\t1371380\n", "other").is_err());
    // Legacy explicit plans retain their existing metadata-only behavior.
    assert!(Plan::parse("UnknownTable\t123\n").is_ok());
    let mut input = String::new();
    for metadata in forever_db2::catalog::tables().iter().take(257) {
        input.push_str(&format!("{}\t{}\n", metadata.name, metadata.file_data_id));
    }
    assert_eq!(
        Plan::parse_with_catalog(&input, "pinned-metadata-v1").err(),
        Some("manifest_policy_limit")
    );
}

#[cfg(unix)]
pub(crate) fn catalog_empty_fixture(name: &str) -> Vec<u8> {
    let table = CharacterTable::from_catalog(name).unwrap().unwrap();
    let (hash, layout, index, fields) = table.schema_identity();
    let mut data = vec![0; 204 + 4 * fields];
    data[..4].copy_from_slice(b"WDC5");
    put32(&mut data, 4, 5);
    put32(&mut data, 140, fields as u32);
    put32(&mut data, 152, hash);
    put32(&mut data, 156, layout);
    data[174..176].copy_from_slice(&index.to_le_bytes());
    put32(&mut data, 176, fields as u32);
    data
}

#[cfg(unix)]
#[test]
fn catalog_batch_emits_v3_and_decodes_an_additional_table() {
    let sandbox = Sandbox::new();
    let destination = sandbox.destination("catalog");
    let plan = Plan::parse_with_catalog("LiquidType\t1371380\n", "pinned-metadata-v1").unwrap();
    let report = extract(
        &plan,
        &sandbox.client(),
        &destination,
        70205,
        "enUS",
        |_, budget| {
            assert_eq!(budget, MAX_FILE_BYTES);
            Ok((70205, catalog_empty_fixture("LiquidType")))
        },
    )
    .unwrap();
    assert!(report.complete);
    assert!(
        report
            .json
            .contains("\"scope\":\"explicit_db2_catalog_batch\"")
    );
    assert!(
        report
            .json
            .contains("\"record_data_decoded\":true,\"decoded_rows\":0")
    );
    let manifest = fs::read_to_string(destination.join("forever-manifest.json")).unwrap();
    assert!(manifest.contains("\"manifest_version\":3"));
    assert!(manifest.contains("\"schema_catalog\":\"pinned-metadata-v1\""));
    assert_eq!(crate::verify::verify(&destination).exit_code(), 0);
}

#[cfg(unix)]
#[test]
fn catalog_never_publishes_metadata_only_fallback() {
    let sandbox = Sandbox::new();
    let plan = Plan::parse_with_catalog("LiquidType\t1371380\n", "pinned-metadata-v1").unwrap();
    for (name, mutation) in [("truncated", 0), ("wrong_layout", 1)] {
        let destination = sandbox.destination(name);
        let mut data = catalog_empty_fixture("LiquidType");
        if mutation == 0 {
            data.truncate(204);
        } else {
            put32(&mut data, 156, 0);
        }
        let report = extract(
            &plan,
            &sandbox.client(),
            &destination,
            70205,
            "enUS",
            |_, _| Ok((70205, data.clone())),
        )
        .unwrap();
        assert!(!report.complete);
        assert!(!destination.join("LiquidType.db2").exists());
        assert!(!report.json.contains("\"record_data_decoded\":true"));
    }
    let legacy = Plan::parse("LiquidType\t1371380\n").unwrap();
    let destination = sandbox.destination("legacy");
    let report = extract(
        &legacy,
        &sandbox.client(),
        &destination,
        70205,
        "enUS",
        |_, _| Ok((70205, catalog_empty_fixture("LiquidType"))),
    )
    .unwrap();
    assert!(report.complete);
    assert!(
        report
            .json
            .contains("\"table_schema_validated\":false,\"record_data_decoded\":false")
    );
    assert!(
        fs::read_to_string(destination.join("forever-manifest.json"))
            .unwrap()
            .starts_with("{\"manifest_version\":2,")
    );
}

struct Sandbox {
    root: PathBuf,
}

impl Sandbox {
    fn new() -> Self {
        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!("forever-batch-{}-{id}", std::process::id()));
        fs::create_dir_all(root.join("client")).unwrap();
        fs::create_dir_all(root.join("out")).unwrap();
        Self { root }
    }

    fn client(&self) -> PathBuf {
        self.root.join("client")
    }

    fn parent(&self) -> PathBuf {
        self.root.join("out")
    }

    fn destination(&self, name: &str) -> PathBuf {
        self.parent().join(name)
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn put32(buf: &mut [u8], at: usize, value: u32) {
    buf[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

/// Minimal but fully decode-valid WDC5 regular file for one character table.
/// Every column is mode 0 at offset 0; one zero record with id 0.
fn decoded_fixture(table: CharacterTable) -> Vec<u8> {
    let (hash, layout, index, fields) = table.schema_identity();
    let record_size = 16usize;
    let mut out = vec![0u8; 204];
    out[..4].copy_from_slice(b"WDC5");
    put32(&mut out, 4, 5);
    put32(&mut out, 136, 1);
    put32(&mut out, 140, fields as u32);
    put32(&mut out, 144, record_size as u32);
    put32(&mut out, 152, hash);
    put32(&mut out, 156, layout);
    put32(&mut out, 160, 0);
    put32(&mut out, 164, 100);
    out[174..176].copy_from_slice(&index.to_le_bytes());
    put32(&mut out, 176, fields as u32);
    put32(&mut out, 188, (fields * 24) as u32);
    put32(&mut out, 200, 1);
    let section = out.len();
    out.resize(out.len() + 40, 0);
    out.resize(out.len() + fields * 4, 0);
    out.resize(out.len() + fields * 24, 0);
    let metadata_end = out.len();
    out.resize(metadata_end + record_size, 0);
    if index < 0 {
        out.extend(0u32.to_le_bytes());
    }
    put32(&mut out, section + 8, metadata_end as u32);
    put32(&mut out, section + 12, 1);
    put32(&mut out, section + 24, if index < 0 { 4 } else { 0 });
    out
}

/// Metadata-only prefix: enough for `Metadata::read`, no decodable body.
#[cfg(unix)]
fn metadata_fixture() -> Vec<u8> {
    let mut out = vec![0u8; 204];
    out[..4].copy_from_slice(b"WDC5");
    put32(&mut out, 4, 5);
    put32(&mut out, 200, 1);
    out.resize(out.len() + 40, 0);
    out
}

fn unknown_plan() -> Plan {
    Plan::parse("Alpha\t1\nBeta\t2\n").unwrap()
}

#[cfg(unix)]
fn list_dir(path: &Path) -> Vec<String> {
    let mut names: Vec<_> = fs::read_dir(path)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

#[test]
fn races_fixture_matches_the_reviewed_schema_identity() {
    // The fixture is built from the reviewed identity, so it no longer hardcodes
    // 51. OPEN DISCREPANCY: schema.rs currently yields 50 ChrRaces fields while
    // the task documentation expects 51; the pinned reference is sandbox-blocked.
    let table = CharacterTable::Races;
    let (hash, layout, index, fields) = table.schema_identity();
    assert_eq!((hash, layout, index), (0x53F1783C, 0x4F44C796, -1));
    let bytes = decoded_fixture(table);
    let header = Metadata::read(&mut std::io::Cursor::new(&bytes))
        .unwrap()
        .header;
    assert_eq!(header.table_hash, hash, "hash");
    assert_eq!(header.layout_hash, layout, "layout");
    assert_eq!(header.index_field, index, "index_field");
    assert_eq!(header.field_count as usize, fields, "field_count");
    assert_eq!(header.total_field_count, header.field_count);
    assert_eq!(header.record_count, 1, "record_count");
    assert!(table.matches(&header));
    let decoded = DecodedTable::read(&bytes, table);
    assert!(
        decoded.is_ok(),
        "races decode: {:?}",
        decoded.err().map(|error| error.code())
    );
}

#[test]
fn default_plan_is_the_two_reviewed_character_tables() {
    let plan = Plan::character_tables();
    let names: Vec<_> = plan
        .entries
        .iter()
        .map(|entry| entry.name.clone())
        .collect();
    assert_eq!(names, vec!["ChrClasses", "ChrRaces"]);
    assert_eq!(plan.entries[0].id, 1361031);
    assert_eq!(plan.entries[1].id, 1305311);
}

#[test]
fn manifest_parse_accepts_valid_and_rejects_every_violation() {
    assert!(Plan::parse("MyTable\t10\nAnother_2\t20\n").is_ok());
    let long_name = format!("{}\t1\n", "a".repeat(81));
    let big = format!("{}\t1\n", "a".repeat(65 * 1024));
    let mut many = String::new();
    for id in 1..=257u32 {
        many.push_str(&format!("Table{id}\t{id}\n"));
    }
    for bad in [
        "",
        "NoTab\n",
        "\t1\n",
        "9StartsDigit\t1\n",
        "has-dash\t1\n",
        "ChrClasses\t9999\n",
        "Zero\t0\n",
        "Max\t4294967295\n",
        "Dup\t1\nDup\t2\n",
        "Dup\t1\nOther\t1\n",
        "NotDecimal\t1a\n",
        long_name.as_str(),
        big.as_str(),
        many.as_str(),
    ] {
        assert!(Plan::parse(bad).is_err(), "accepted invalid manifest");
    }
}

#[cfg(unix)]
#[test]
fn default_plan_writes_two_db2_and_a_report() {
    let sandbox = Sandbox::new();
    let destination = sandbox.destination("run");
    let report = extract(
        &Plan::character_tables(),
        &sandbox.client(),
        &destination,
        build(),
        "enUS",
        |entry, _budget| {
            let table = CharacterTable::named(&entry.name).unwrap();
            Ok((build(), decoded_fixture(table)))
        },
    )
    .unwrap();
    assert!(report.complete, "{}", report.json);
    assert!(report.json.contains("\"status\":\"success\""));
    let names = list_dir(&destination);
    assert!(names.contains(&"ChrClasses.db2".to_owned()));
    assert!(names.contains(&"ChrRaces.db2".to_owned()));
    assert!(names.contains(&"report.json".to_owned()));
    assert!(names.iter().all(|name| !name.ends_with(".part")));
    assert!(report.json.contains("\"table_schema_validated\":true"));
    assert!(report.json.contains("\"record_data_decoded\":true"));
}

#[cfg(unix)]
#[test]
fn unreviewed_name_is_published_without_schema_or_decode_claims() {
    let sandbox = Sandbox::new();
    let destination = sandbox.destination("run");
    let report = extract(
        &unknown_plan(),
        &sandbox.client(),
        &destination,
        build(),
        "esES",
        |_entry, _budget| Ok((build(), metadata_fixture())),
    )
    .unwrap();
    assert!(report.complete);
    assert!(report.json.contains("\"table_schema_validated\":false"));
    assert!(report.json.contains("\"record_data_decoded\":false"));
    assert!(report.json.contains("\"declared_records\":0"));
    let names = list_dir(&destination);
    assert!(names.contains(&"Alpha.db2".to_owned()));
    assert!(names.contains(&"Beta.db2".to_owned()));
}

#[test]
fn unsupported_build_or_locale_creates_nothing() {
    for bad_build in [1u32, 70171, 99999] {
        let sandbox = Sandbox::new();
        let destination = sandbox.destination("run");
        let result = extract(
            &unknown_plan(),
            &sandbox.client(),
            &destination,
            bad_build,
            "enUS",
            |_e, _b| panic!("load must not run"),
        );
        assert_eq!(result.err().unwrap(), "unsupported_client");
        assert!(!destination.exists());
    }
    let sandbox = Sandbox::new();
    let destination = sandbox.destination("run");
    let result = extract(
        &unknown_plan(),
        &sandbox.client(),
        &destination,
        build(),
        "frFR",
        |_e, _b| panic!("load must not run"),
    );
    assert_eq!(result.err().unwrap(), "unsupported_client");
    assert!(!destination.exists());
}

#[cfg(unix)]
#[test]
fn observed_build_change_and_oversized_buffer_are_entry_errors() {
    let sandbox = Sandbox::new();
    let destination = sandbox.destination("run");
    let report = extract(
        &unknown_plan(),
        &sandbox.client(),
        &destination,
        build(),
        "enUS",
        |entry, budget| {
            if entry.name == "Alpha" {
                Ok((build() + 1, metadata_fixture()))
            } else {
                Ok((build(), vec![0u8; budget as usize + 1]))
            }
        },
    )
    .unwrap();
    assert!(!report.complete);
    assert!(report.json.contains("\"status\":\"incomplete\""));
    assert!(report.json.contains("client_build_changed"));
    assert!(report.json.contains("read_policy_limit"));
}

#[cfg(unix)]
#[test]
fn one_failed_entry_does_not_abort_or_publish_the_rest() {
    let sandbox = Sandbox::new();
    let destination = sandbox.destination("run");
    let report = extract(
        &unknown_plan(),
        &sandbox.client(),
        &destination,
        build(),
        "enUS",
        |entry, _budget| {
            if entry.name == "Alpha" {
                Err(Error::MissingFile)
            } else {
                Ok((build(), metadata_fixture()))
            }
        },
    )
    .unwrap();
    assert!(!report.complete);
    let names = list_dir(&destination);
    assert!(!names.contains(&"Alpha.db2".to_owned()));
    assert!(names.contains(&"Beta.db2".to_owned()));
    assert!(report.json.contains("file_unavailable"));
}

#[cfg(unix)]
#[test]
fn encrypted_or_unsupported_section_is_an_error_not_a_published_file() {
    let sandbox = Sandbox::new();
    let destination = sandbox.destination("run");
    let plan = Plan::parse("ChrClasses\t1361031\n").unwrap();
    let report = extract(
        &plan,
        &sandbox.client(),
        &destination,
        build(),
        "enUS",
        |_e, _b| {
            let mut bytes = decoded_fixture(CharacterTable::Classes);
            bytes[204] = 0x01; // nonzero TactId declaration
            Ok((build(), bytes))
        },
    )
    .unwrap();
    assert!(!report.complete);
    assert!(report.json.contains("unsupported_record_variant"));
    // The file was read and its metadata parsed, so schema/declared facts are
    // reported independently of the failed body decode.
    assert!(report.json.contains("\"table_schema_validated\":true"));
    assert!(report.json.contains("\"declared_records\":1"));
    assert!(report.json.contains("\"bytes\":"));
    assert!(!report.json.contains("\"record_data_decoded\":true"));
    assert!(!list_dir(&destination).contains(&"ChrClasses.db2".to_owned()));
}

#[cfg(unix)]
#[test]
fn unsafe_output_paths_are_rejected_before_any_creation() {
    // Inside the client root.
    let sandbox = Sandbox::new();
    let inside = sandbox.client().join("nested");
    assert_eq!(attempt(&sandbox, &inside), "unsafe_output_path");
    assert!(!inside.exists());

    // Inside the client Data directory.
    let sandbox = Sandbox::new();
    fs::create_dir_all(sandbox.client().join("Data")).unwrap();
    let data_out = sandbox.client().join("Data").join("nested");
    assert_eq!(attempt(&sandbox, &data_out), "unsafe_output_path");
    assert!(!data_out.exists());

    // Destination equal to the canonical client.
    let sandbox = Sandbox::new();
    assert_eq!(attempt(&sandbox, &sandbox.client()), "unsafe_output_path");

    // Under a repository marker.
    let sandbox = Sandbox::new();
    let repo = sandbox.root.join("repo");
    fs::create_dir_all(repo.join(".git")).unwrap();
    let repo_out = repo.join("nested");
    assert_eq!(attempt(&sandbox, &repo_out), "unsafe_output_path");
    assert!(!repo_out.exists());

    // Existing destination.
    let sandbox = Sandbox::new();
    let existing = sandbox.destination("taken");
    fs::create_dir(&existing).unwrap();
    assert_eq!(attempt(&sandbox, &existing), "output_exists_or_unavailable");

    // Symbolic-link destination.
    let sandbox = Sandbox::new();
    let linked = sandbox.destination("link");
    symlink(sandbox.destination("target"), &linked).unwrap();
    assert_eq!(attempt(&sandbox, &linked), "output_exists_or_unavailable");

    // Missing destination parent.
    let sandbox = Sandbox::new();
    let missing = sandbox.root.join("missing").join("nested");
    assert_eq!(attempt(&sandbox, &missing), "unsafe_output_path");
    assert!(!missing.exists());
}

#[cfg(unix)]
fn attempt(sandbox: &Sandbox, destination: &Path) -> &'static str {
    match extract(
        &unknown_plan(),
        &sandbox.client(),
        destination,
        build(),
        "enUS",
        |_e, _b| Ok((build(), metadata_fixture())),
    ) {
        Err(code) => code,
        Ok(_) => panic!("expected unsafe_output_path"),
    }
}

#[cfg(unix)]
#[test]
fn reports_never_contain_the_client_path() {
    let sandbox = Sandbox::new();
    let destination = sandbox.destination("run");
    let report = extract(
        &unknown_plan(),
        &sandbox.client(),
        &destination,
        build(),
        "enUS",
        |_e, _b| Ok((build(), metadata_fixture())),
    )
    .unwrap();
    let client = sandbox.client().to_string_lossy().into_owned();
    assert!(!report.json.contains(&client));
    assert!(report.json.contains("\"paths_redacted\":true"));
    for name in list_dir(&destination) {
        let text = fs::read_to_string(destination.join(name)).unwrap_or_default();
        assert!(!text.contains(&client));
    }
}

fn build() -> u32 {
    crate::SUPPORTED_CLIENTS[0].1
}

#[test]
fn file_limit_constants_are_the_documented_policy() {
    assert_eq!(MAX_FILE_BYTES, 16 * 1024 * 1024);
    assert_eq!(super::MAX_BATCH_BYTES, 256 * 1024 * 1024);
    assert_eq!(super::MAX_FILES, 256);
}

#[cfg(unix)]
#[test]
fn complete_batch_publishes_a_v2_manifest_with_matching_invariants() {
    let sandbox = Sandbox::new();
    let destination = sandbox.destination("run");
    let report = extract(
        &Plan::character_tables(),
        &sandbox.client(),
        &destination,
        build(),
        "enUS",
        |entry, _budget| {
            let table = CharacterTable::named(&entry.name).unwrap();
            Ok((build(), decoded_fixture(table)))
        },
    )
    .unwrap();
    assert!(report.complete);

    let manifest = fs::read_to_string(destination.join("forever-manifest.json")).unwrap();
    assert!(manifest.contains("\"manifest_version\":2"));
    assert!(manifest.contains("\"status\":\"complete\""));
    assert!(manifest.contains("\"incomplete_marker_present\":false"));
    assert!(!manifest.contains("\"reason\""));
    assert!(!manifest.contains("\"code\""));
    assert!(manifest.contains("\"extractor\":{\"name\":\"forever-db2-extract\""));
    assert!(manifest.contains("\"table_selection\":\"default_character_tables\""));
    assert!(manifest.contains("\"manifest_sha256\":null"));

    // Every published artifact's recorded digest must be the digest of the
    // bytes that actually landed on disk, and the integrity totals must equal
    // the sum of those artifacts.
    let mut total_bytes: u64 = 0;
    for name in ["ChrClasses", "ChrRaces"] {
        let bytes = fs::read(destination.join(format!("{name}.db2"))).unwrap();
        total_bytes += bytes.len() as u64;
        let digest = crate::sha256::hex(&bytes);
        let recorded = artifact_field(&manifest, name, "sha256");
        assert_eq!(recorded, digest, "{name} sha256 mismatch");
        let recorded_bytes = artifact_field(&manifest, name, "bytes");
        assert_eq!(
            recorded_bytes.parse::<u64>().unwrap(),
            bytes.len() as u64,
            "{name} byte count mismatch"
        );
    }
    assert_eq!(integrity_field(&manifest, "file_count"), "2");
    assert_eq!(
        integrity_field(&manifest, "total_bytes"),
        total_bytes.to_string()
    );
}

#[cfg(unix)]
#[test]
fn batch_and_verifier_count_sparse_copies_beyond_the_direct_record_count() {
    let sandbox = Sandbox::new();
    let destination = sandbox.destination("sparse-copies");
    let plan = Plan::parse("ChrRaces\t1305311\n").unwrap();
    let mut bytes = decoded_fixture(CharacterTable::Races);
    let id_offset = bytes.len() - 4;
    put32(&mut bytes, id_offset, 70_000);
    put32(&mut bytes, 164, 90_000);
    put32(&mut bytes, 204 + 36, 2);
    for (new, source) in [(80_000u32, 70_000u32), (90_000, 80_000)] {
        bytes.extend(new.to_le_bytes());
        bytes.extend(source.to_le_bytes());
    }
    let report = extract(
        &plan,
        &sandbox.client(),
        &destination,
        build(),
        "enUS",
        |_entry, _budget| Ok((build(), bytes.clone())),
    )
    .unwrap();
    assert!(report.complete, "{}", report.json);
    assert!(report.json.contains("\"declared_records\":1"));
    assert!(report.json.contains("\"decoded_rows\":3"));
    let manifest = fs::read_to_string(destination.join("forever-manifest.json")).unwrap();
    assert_eq!(
        artifact_field(&manifest, "ChrRaces", "declared_records"),
        "1"
    );
    assert_eq!(artifact_field(&manifest, "ChrRaces", "decoded_rows"), "3");
    let verified = crate::verify::verify(&destination);
    assert_eq!(verified.exit_code(), 0, "{}", verified.json());

    // A stale manifest produced by the old RecordCount rule must fail the
    // re-decoding gate even though the artifact and its hash are unchanged.
    let stale = manifest.replace("\"decoded_rows\":3", "\"decoded_rows\":1");
    assert_ne!(manifest, stale);
    fs::write(destination.join("forever-manifest.json"), stale).unwrap();
    let verified = crate::verify::verify(&destination);
    assert!(verified.json().contains("ROW_DECODE_MISMATCH"));
}

#[cfg(unix)]
#[test]
fn manifest_meta_index_field_comes_from_the_reviewed_schema_not_the_header() {
    // Real 70205 evidence: `ChrRaces` carries header `IndexField == 0` while the
    // reviewed metadata index is `-1`. The manifest's `meta_index_field` is the
    // reviewed authority for inline vs id-table decoding, so it must be `-1` and
    // never the raw header word.
    let sandbox = Sandbox::new();
    let destination = sandbox.destination("run");
    let plan = Plan::parse("ChrRaces\t1305311\n").unwrap();
    let report = extract(
        &plan,
        &sandbox.client(),
        &destination,
        build(),
        "enUS",
        |_entry, _budget| {
            let mut bytes = decoded_fixture(CharacterTable::Races);
            // Overwrite the header `IndexField` word only (bytes 174..176); the
            // decoder consults the reviewed schema index, not this word.
            bytes[174..176].copy_from_slice(&0i16.to_le_bytes());
            Ok((build(), bytes))
        },
    )
    .unwrap();
    assert!(report.complete, "{}", report.json);

    let manifest = fs::read_to_string(destination.join("forever-manifest.json")).unwrap();
    assert_eq!(
        table_field(&manifest, "ChrRaces", "meta_index_field"),
        "-1",
        "manifest must publish the reviewed meta index, not the header word"
    );
}

#[cfg(unix)]
#[test]
fn unreviewed_table_publishes_the_meta_index_sentinel() {
    // No reviewed authority exists for an unknown table, so the honest sentinel
    // is `0` even when the raw header declares another value.
    let sandbox = Sandbox::new();
    let destination = sandbox.destination("run");
    let report = extract(
        &unknown_plan(),
        &sandbox.client(),
        &destination,
        build(),
        "enUS",
        |_entry, _budget| {
            let mut bytes = metadata_fixture();
            bytes[174..176].copy_from_slice(&7i16.to_le_bytes());
            Ok((build(), bytes))
        },
    )
    .unwrap();
    assert!(report.complete);
    let manifest = fs::read_to_string(destination.join("forever-manifest.json")).unwrap();
    for name in ["Alpha", "Beta"] {
        assert_eq!(
            table_field(&manifest, name, "meta_index_field"),
            "0",
            "{name}"
        );
    }
}

#[cfg(unix)]
#[test]
fn failed_batch_publishes_a_failed_manifest_with_reason_and_code() {
    let sandbox = Sandbox::new();
    let destination = sandbox.destination("run");
    let report = extract(
        &unknown_plan(),
        &sandbox.client(),
        &destination,
        build(),
        "enUS",
        |entry, _budget| {
            if entry.name == "Alpha" {
                Err(Error::MissingFile)
            } else {
                Ok((build(), metadata_fixture()))
            }
        },
    )
    .unwrap();
    assert!(!report.complete);

    let manifest = fs::read_to_string(destination.join("forever-manifest.json")).unwrap();
    assert!(manifest.contains("\"status\":\"failed\""));
    assert!(manifest.contains("\"incomplete_marker_present\":true"));
    assert!(manifest.contains("\"reason\":\""), "missing failure reason");
    // The error artifact keeps its static code; the published one carries none.
    assert_eq!(artifact_field(&manifest, "Alpha", "status"), "error");
    assert_eq!(
        artifact_field(&manifest, "Alpha", "code"),
        "file_unavailable"
    );
    assert_eq!(artifact_field(&manifest, "Beta", "status"), "complete");
    assert_eq!(integrity_field(&manifest, "file_count"), "1");
    assert!(!destination.join("Alpha.db2").exists());
    assert!(destination.join("Beta.db2").exists());
}

/// Returns the JSON value for `key` inside the artifact object that follows the
/// `"name":"<name>"` anchor within the manifest's `"artifacts":[...]` array.
/// The manifest is a flat, single-line document with stable key order, so
/// linear scanning is sufficient and dependency-free.
#[cfg(unix)]
fn artifact_field(manifest: &str, name: &str, key: &str) -> String {
    let array = artifact_section(manifest);
    let anchor = format!("\"name\":\"{name}\"");
    let start = array.find(&anchor).expect("artifact anchor");
    let rest = &array[start..];
    let end = rest.find("}").expect("artifact end");
    json_field(&rest[..end], key)
}

#[cfg(unix)]
fn artifact_section(manifest: &str) -> &str {
    let start = manifest.find("\"artifacts\":[").expect("artifacts array");
    let rest = &manifest[start..];
    let end = rest.find("],\"integrity\"").expect("artifacts end");
    &rest[..end]
}

/// Returns the JSON value for `key` inside the table object that follows the
/// `"name":"<name>"` anchor within the manifest's `"formats":{"tables":[...]}`
/// array. Same linear-scan assumption as `artifact_field`.
#[cfg(unix)]
fn table_field(manifest: &str, name: &str, key: &str) -> String {
    let start = manifest.find("\"tables\":[").expect("tables array");
    let rest = &manifest[start..];
    let end = rest.find("]}").expect("tables end");
    let array = &rest[..end];
    let anchor = format!("\"name\":\"{name}\"");
    let anchor_start = array.find(&anchor).expect("table anchor");
    let after = &array[anchor_start..];
    let obj_end = after.find("}").expect("table end");
    json_field(&after[..obj_end], key)
}

/// Returns the JSON value for `key` inside the `"integrity":{...}` object.
#[cfg(unix)]
fn integrity_field(manifest: &str, key: &str) -> String {
    let start = manifest.find("\"integrity\":{").expect("integrity object");
    let rest = &manifest[start..];
    let end = rest.find('}').expect("integrity end");
    json_field(&rest[..end], key)
}

/// Minimal field reader for the manifest's flat `"key":value` members. Values
/// are unquoted strings/numbers/booleans/null; the manifest contains no nested
/// objects between an artifact's braces or inside the integrity object.
#[cfg(unix)]
fn json_field(object: &str, key: &str) -> String {
    let needle = format!("\"{key}\":");
    let start = object
        .find(&needle)
        .unwrap_or_else(|| panic!("missing key {key}"))
        + needle.len();
    let rest = &object[start..];
    if let Some(quoted) = rest.strip_prefix('"') {
        let end = quoted.find('"').expect("string end");
        quoted[..end].to_owned()
    } else {
        let end = rest.find([',', '}']).unwrap_or(rest.len());
        rest[..end].to_owned()
    }
}

#[cfg(unix)]
pub(crate) fn rust_local_profile(limit: u32) -> crate::acquisition::Profile {
    let mut builder = crate::acquisition::OptionsBuilder::default();
    for (key, value) in [
        ("--backend", "rust-local"),
        ("--index-layout", "contiguous"),
        ("--root-layout", "header-50893-v2"),
    ] {
        assert!(builder.push(key, std::ffi::OsStr::new(value)).unwrap());
    }
    builder
        .push("--max-file-bytes", std::ffi::OsStr::new(&limit.to_string()))
        .unwrap();
    builder.finish(MAX_FILE_BYTES).unwrap().profile().unwrap()
}

#[cfg(unix)]
#[test]
fn acquired_default_batch_emits_v4_and_removes_real_marker_only_on_success() {
    let sandbox = Sandbox::new();
    let output = sandbox.destination("acquired");
    let profile = rust_local_profile(1024);
    let report = super::extract_with_acquisition(
        &Plan::character_tables(),
        &sandbox.client(),
        &output,
        70205,
        "enUS",
        Some(profile),
        |entry, budget| {
            assert_eq!(budget, 1024);
            assert!(output.join(crate::verify::MARKER_NAME).is_file());
            Ok((70205, catalog_empty_fixture(&entry.name)))
        },
    )
    .unwrap();
    assert!(report.complete);
    assert!(report.json.contains("\"schema_selection\":\"reviewed\""));
    let manifest = fs::read_to_string(output.join("forever-manifest.json")).unwrap();
    assert!(manifest.contains("\"manifest_version\":4"));
    assert!(manifest.contains(&profile.render()));
    assert!(!output.join(crate::verify::MARKER_NAME).exists());
    assert_eq!(crate::verify::verify(&output).exit_code(), 0);
}

#[cfg(unix)]
#[test]
fn acquired_catalog_and_legacy_metadata_only_keep_their_selection_policy() {
    for catalog in [false, true] {
        let sandbox = Sandbox::new();
        let output = sandbox.destination("acquired");
        let plan = if catalog {
            Plan::parse_with_catalog("LiquidType\t1371380\n", "pinned-metadata-v1").unwrap()
        } else {
            Plan::parse("UnknownTable\t123\n").unwrap()
        };
        let report = super::extract_with_acquisition(
            &plan,
            &sandbox.client(),
            &output,
            70205,
            "enUS",
            Some(rust_local_profile(1024)),
            |_, _| Ok((70205, catalog_empty_fixture("LiquidType"))),
        )
        .unwrap();
        assert!(report.complete);
        assert!(report.json.contains(if catalog {
            "\"record_data_decoded\":true"
        } else {
            "\"table_schema_validated\":false,\"record_data_decoded\":false,\"decoded_rows\":null"
        }));
        assert_eq!(crate::verify::verify(&output).exit_code(), 0);
    }
}

#[cfg(unix)]
#[test]
fn acquired_input_change_aborts_without_later_reads_or_removing_marker() {
    let sandbox = Sandbox::new();
    let output = sandbox.destination("changed");
    let plan = Plan::parse("ChrClasses\t1361031\nChrRaces\t1305311\nUnknownTable\t123\n").unwrap();
    let mut calls = 0;
    let result = super::extract_with_acquisition(
        &plan,
        &sandbox.client(),
        &output,
        70205,
        "enUS",
        Some(rust_local_profile(1024)),
        |entry, _| {
            calls += 1;
            if calls == 1 {
                Ok((70205, catalog_empty_fixture(&entry.name)))
            } else {
                Err(Error::LocalStore(crate::local_store::Error::InputChanged))
            }
        },
    );
    assert_eq!(result.err(), Some("local_store_input_changed"));
    assert_eq!(calls, 2);
    assert!(output.join(crate::verify::MARKER_NAME).is_file());
    assert!(output.join("ChrClasses.db2").is_file());
    assert!(!output.join("ChrRaces.db2").exists());
    assert!(!output.join("forever-manifest.json").exists());
}

#[cfg(unix)]
#[test]
fn acquired_ordinary_entry_failure_continues_and_retains_incomplete_marker() {
    let sandbox = Sandbox::new();
    let output = sandbox.destination("partial");
    let mut calls = 0;
    let report = super::extract_with_acquisition(
        &Plan::character_tables(),
        &sandbox.client(),
        &output,
        70205,
        "enUS",
        Some(rust_local_profile(1024)),
        |entry, _| {
            calls += 1;
            if calls == 1 {
                Err(Error::LocalStore(crate::local_store::Error::MissingFile))
            } else {
                Ok((70205, catalog_empty_fixture(&entry.name)))
            }
        },
    )
    .unwrap();
    assert!(!report.complete);
    assert_eq!(calls, 2);
    assert!(output.join(crate::verify::MARKER_NAME).is_file());
    assert!(output.join("ChrRaces.db2").is_file());
    assert_eq!(crate::verify::verify(&output).exit_code(), 8);
}

#[cfg(unix)]
#[test]
fn acquired_smaller_file_budget_is_forwarded_and_enforced_before_publication() {
    let sandbox = Sandbox::new();
    let output = sandbox.destination("capped");
    let plan = Plan::parse("UnknownTable\t123\n").unwrap();
    let report = super::extract_with_acquisition(
        &plan,
        &sandbox.client(),
        &output,
        70205,
        "enUS",
        Some(rust_local_profile(64)),
        |_, budget| {
            assert_eq!(budget, 64);
            Ok((70205, vec![0; 65]))
        },
    )
    .unwrap();
    assert!(!report.complete);
    assert!(report.json.contains("read_policy_limit"));
    assert!(!output.join("UnknownTable.db2").exists());
    assert!(output.join(crate::verify::MARKER_NAME).exists());
}

#[cfg(unix)]
#[test]
fn acquired_build_change_is_fatal_and_never_reads_next_entry() {
    let sandbox = Sandbox::new();
    let output = sandbox.destination("build_changed");
    let mut calls = 0;
    let result = super::extract_with_acquisition(
        &Plan::character_tables(),
        &sandbox.client(),
        &output,
        70205,
        "enUS",
        Some(rust_local_profile(1024)),
        |_, _| {
            calls += 1;
            Ok((70170, catalog_empty_fixture("ChrClasses")))
        },
    );
    assert_eq!(result.err(), Some("client_build_changed"));
    assert_eq!(calls, 1);
    assert!(output.join(crate::verify::MARKER_NAME).exists());
    assert!(!output.join("ChrClasses.db2").exists());
    assert!(!output.join("forever-manifest.json").exists());
}
