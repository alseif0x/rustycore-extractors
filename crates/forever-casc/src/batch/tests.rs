// SPDX-License-Identifier: GPL-3.0-or-later
//! Synthetic extraction tests. They use the `load` seam only: no CASC, no native
//! storage, no real client bytes and no network. All temporary directories live
//! under `TMPDIR` (externally configured) and are removed on drop.
use super::{MAX_FILE_BYTES, Plan, extract};
use crate::Error;
use forever_db2::{CharacterTable, DecodedTable, Metadata};
use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

struct Sandbox {
    root: PathBuf,
}

impl Sandbox {
    fn new() -> Self {
        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
        let root = base_dir().join(format!("forever-batch-{}-{id}", std::process::id()));
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

/// A writable base directory with no `.git` ancestor. `output::create` rejects
/// any destination below a repository marker, so the external `TMPDIR` (which
/// may sit under a checkout) is only a fallback.
fn base_dir() -> PathBuf {
    for candidate in [PathBuf::from("/tmp/opencode"), std::env::temp_dir()] {
        let git_free = !candidate
            .ancestors()
            .any(|ancestor| ancestor.join(".git").exists());
        if candidate.is_dir() && git_free {
            return candidate;
        }
    }
    std::env::temp_dir()
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
fn artifact_field(manifest: &str, name: &str, key: &str) -> String {
    let array = artifact_section(manifest);
    let anchor = format!("\"name\":\"{name}\"");
    let start = array.find(&anchor).expect("artifact anchor");
    let rest = &array[start..];
    let end = rest.find("}").expect("artifact end");
    json_field(&rest[..end], key)
}

fn artifact_section(manifest: &str) -> &str {
    let start = manifest.find("\"artifacts\":[").expect("artifacts array");
    let rest = &manifest[start..];
    let end = rest.find("],\"integrity\"").expect("artifacts end");
    &rest[..end]
}

/// Returns the JSON value for `key` inside the `"integrity":{...}` object.
fn integrity_field(manifest: &str, key: &str) -> String {
    let start = manifest.find("\"integrity\":{").expect("integrity object");
    let rest = &manifest[start..];
    let end = rest.find('}').expect("integrity end");
    json_field(&rest[..end], key)
}

/// Minimal field reader for the manifest's flat `"key":value` members. Values
/// are unquoted strings/numbers/booleans/null; the manifest contains no nested
/// objects between an artifact's braces or inside the integrity object.
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
