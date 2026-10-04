// SPDX-License-Identifier: GPL-3.0-or-later
//! Synthetic verification tests. They build a temporary manifest plus `.db2`
//! files from the in-tree WDC5 fixture generator: no CASC, no native storage,
//! no real client bytes and no network. All temporary directories live under
//! `TMPDIR` (externally configured) and are removed on drop.
use super::{Code, MARKER_NAME, verify};
use forever_db2::CharacterTable;
use std::fs;
use std::os::unix::fs::symlink;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

struct Sandbox {
    root: PathBuf,
}

impl Sandbox {
    fn new() -> Self {
        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
        let root = base_dir().join(format!("forever-verify-{}-{id}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        Self { root }
    }

    /// A fresh output directory inside the sandbox.
    fn out(&self) -> PathBuf {
        let path = self.root.join("out");
        fs::create_dir_all(&path).unwrap();
        path
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// A writable base directory with no `.git` ancestor, mirroring the batch tests
/// so no synthetic byte lands inside a checkout.
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

/// Minimal but fully decode-valid WDC5 regular file for one character table,
/// copied from the batch fixture generator (one zero record with id 0).
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

struct TableSpec {
    name: String,
    file_id: u64,
    table_hash: u64,
    layout_hash: u64,
    meta_index: i64,
    field_count: u64,
}

struct ArtifactSpec {
    name: String,
    file_id: u64,
    relative_path: String,
    status: &'static str,
    code: Option<&'static str>,
    bytes: u64,
    sha256: String,
    declared_records: u64,
    schema_validated: bool,
    record_data_decoded: bool,
    decoded_rows: Option<u64>,
}

struct ManifestSpec {
    status: &'static str,
    reason: Option<&'static str>,
    paths_redacted: bool,
    tables: Vec<TableSpec>,
    artifacts: Vec<ArtifactSpec>,
    integrity_override: Option<(u64, u64, String)>,
}

fn table_spec(table: CharacterTable) -> TableSpec {
    let (hash, layout, index, fields) = table.schema_identity();
    TableSpec {
        name: table.name().to_owned(),
        file_id: u64::from(table.file_id()),
        table_hash: u64::from(hash),
        layout_hash: u64::from(layout),
        meta_index: i64::from(index),
        field_count: fields as u64,
    }
}

fn artifact_spec(table: CharacterTable, bytes: &[u8], decoded_rows: Option<u64>) -> ArtifactSpec {
    let name = table.name();
    ArtifactSpec {
        name: name.to_owned(),
        file_id: u64::from(table.file_id()),
        relative_path: format!("{name}.db2"),
        status: "complete",
        code: None,
        bytes: bytes.len() as u64,
        sha256: crate::sha256::hex(bytes),
        declared_records: 1,
        schema_validated: true,
        record_data_decoded: decoded_rows.is_some(),
        decoded_rows,
    }
}

/// A complete, self-consistent two-table run plus the bytes to publish.
fn valid_run() -> (ManifestSpec, Vec<(String, Vec<u8>)>) {
    let classes = decoded_fixture(CharacterTable::Classes);
    let races = decoded_fixture(CharacterTable::Races);
    let files = vec![
        ("ChrClasses.db2".to_owned(), classes.clone()),
        ("ChrRaces.db2".to_owned(), races.clone()),
    ];
    let spec = ManifestSpec {
        status: "complete",
        reason: None,
        paths_redacted: true,
        tables: vec![
            table_spec(CharacterTable::Classes),
            table_spec(CharacterTable::Races),
        ],
        artifacts: vec![
            artifact_spec(CharacterTable::Classes, &classes, Some(1)),
            artifact_spec(CharacterTable::Races, &races, Some(1)),
        ],
        integrity_override: None,
    };
    (spec, files)
}

fn materialize(sandbox: &Sandbox, spec: &ManifestSpec, files: &[(String, Vec<u8>)]) -> PathBuf {
    let directory = sandbox.out();
    for (name, bytes) in files {
        fs::write(directory.join(name), bytes).unwrap();
    }
    fs::write(directory.join(super::MANIFEST_NAME), render(spec)).unwrap();
    directory
}

/// Emit a manifest that is valid against the v2 schema unless `spec` breaks one
/// field. Integrity is derived from the complete artifact rows so a single
/// mutation isolates exactly one invariant.
fn render(spec: &ManifestSpec) -> String {
    let complete: Vec<&ArtifactSpec> = spec
        .artifacts
        .iter()
        .filter(|artifact| artifact.status == "complete")
        .collect();
    let (file_count, total_bytes, inventory_sha) = match &spec.integrity_override {
        Some((count, bytes, sha)) => (*count, *bytes, sha.clone()),
        None => {
            let mut lines: Vec<String> = complete
                .iter()
                .map(|artifact| format!("{}  {}.db2", artifact.sha256, artifact.name))
                .collect();
            lines.sort_by(|left, right| left.as_bytes().cmp(right.as_bytes()));
            let inventory = lines.join("\n");
            (
                complete.len() as u64,
                complete.iter().map(|artifact| artifact.bytes).sum(),
                crate::sha256::hex(inventory.as_bytes()),
            )
        }
    };

    let mut out = String::new();
    out.push_str("{\"manifest_version\":2");
    out.push_str(&format!(",\"status\":\"{}\"", spec.status));
    if let Some(reason) = spec.reason {
        out.push_str(&format!(",\"reason\":\"{reason}\""));
    }
    out.push_str(",\"compatibility\":{\"support\":\"admitted\",\"target_build\":70205}");
    out.push_str(
        ",\"client\":{\"product\":\"wow_classic_beta\",\"version\":\"1.60.1.70205\",\
        \"build\":70205,\"locale\":\"enUS\"}",
    );
    out.push_str(
        ",\"extractor\":{\"name\":\"forever-db2-extract\",\"version\":\"0.1.0\",\
        \"implementation\":\"rust\",\
        \"compatibility_oracle_commit\":\"02245dcd245e7433e524577656177723d3e4992e\"}",
    );
    out.push_str(",\"scope\":\"explicit_db2_batch\"");
    out.push_str(",\"formats\":{\"db2\":{\"magic\":\"WDC5\",\"version\":5},\"tables\":[");
    for (index, table) in spec.tables.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(&format!(
            "{{\"name\":\"{}\",\"file_id\":{},\"table_hash\":{},\"layout_hash\":{},\
            \"meta_index_field\":{},\"field_count\":{}}}",
            table.name,
            table.file_id,
            table.table_hash,
            table.layout_hash,
            table.meta_index,
            table.field_count
        ));
    }
    out.push_str("]}");
    out.push_str(&format!(
        ",\"options\":{{\"locale\":\"enUS\",\"table_selection\":\"default_character_tables\",\
        \"manifest_sha256\":null,\"entry_count\":{},\"max_file_bytes\":16777216,\
        \"max_batch_bytes\":268435456}}",
        spec.artifacts.len().max(1)
    ));
    out.push_str(
        ",\"network\":{\"mode\":\"local\",\"allow_network\":false,\"remote_casc\":false,\
        \"lock_sha256\":null,\"downloaded_bytes\":0}",
    );
    out.push_str(&format!(
        ",\"outputs\":{{\"root\":\".\",\"incomplete_marker_present\":{},\"artifacts\":[",
        spec.status != "complete"
    ));
    for (index, artifact) in spec.artifacts.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(&format!(
            "{{\"name\":\"{}\",\"file_id\":{},\"relative_path\":\"{}\",\"status\":\"{}\"",
            artifact.name, artifact.file_id, artifact.relative_path, artifact.status
        ));
        if let Some(code) = artifact.code {
            out.push_str(&format!(",\"code\":\"{code}\""));
        }
        out.push_str(&format!(
            ",\"bytes\":{},\"sha256\":\"{}\",\"declared_records\":{},\
            \"table_schema_validated\":{},\"record_data_decoded\":{},\"decoded_rows\":{}}}",
            artifact.bytes,
            artifact.sha256,
            artifact.declared_records,
            artifact.schema_validated,
            artifact.record_data_decoded,
            artifact
                .decoded_rows
                .map_or_else(|| "null".to_owned(), |rows| rows.to_string())
        ));
    }
    out.push_str(&format!(
        "],\"integrity\":{{\"algorithm\":\"sha256\",\"file_count\":{file_count},\
        \"total_bytes\":{total_bytes},\"inventory_sha256\":\"{inventory_sha}\"}}}}"
    ));
    out.push_str(&format!(
        ",\"timestamps\":{{\"started_at\":\"2026-10-04T10:00:00Z\",\"completed_at\":{}}}",
        if spec.status == "complete" {
            "\"2026-10-04T10:00:02Z\""
        } else {
            "null"
        }
    ));
    out.push_str(&format!(
        ",\"privacy\":{{\"paths_redacted\":{},\"client_data_embedded\":false}}}}",
        spec.paths_redacted
    ));
    out
}

#[test]
fn valid_run_verifies_with_exit_zero() {
    let sandbox = Sandbox::new();
    let (spec, files) = valid_run();
    let directory = materialize(&sandbox, &spec, &files);
    let report = verify(&directory);
    assert_eq!(report.code, Code::Verified);
    assert_eq!(report.exit_code(), 0);
    assert_eq!(report.build, Some(70205));
    assert_eq!(report.artifacts_verified, 2);
    let json = report.json();
    assert!(json.contains("\"status\":\"verified\""));
    assert!(json.contains("\"code\":\"verified\""));
    assert!(json.contains("\"paths_redacted\":true"));
    assert!(json.contains("\"offline\":true"));
}

#[test]
fn altered_sha256_is_an_inventory_mismatch() {
    let sandbox = Sandbox::new();
    let (mut spec, files) = valid_run();
    // A different but well-formed digest, kept self-consistent in `integrity`.
    spec.artifacts[0].sha256 = "0".repeat(64);
    let directory = materialize(&sandbox, &spec, &files);
    let report = verify(&directory);
    assert_eq!(report.code, Code::InventoryMismatch);
    assert_eq!(report.exit_code(), 9);
}

#[test]
fn a_part_file_marks_the_run_incomplete() {
    let sandbox = Sandbox::new();
    let (spec, files) = valid_run();
    let directory = materialize(&sandbox, &spec, &files);
    fs::write(directory.join("ChrClasses.db2.part"), b"x").unwrap();
    let report = verify(&directory);
    assert_eq!(report.code, Code::IncompleteRun);
    assert_eq!(report.exit_code(), 8);
}

#[test]
fn an_unlisted_db2_is_an_inventory_mismatch() {
    let sandbox = Sandbox::new();
    let (spec, files) = valid_run();
    let directory = materialize(&sandbox, &spec, &files);
    fs::write(directory.join("Extra.db2"), b"x").unwrap();
    let report = verify(&directory);
    assert_eq!(report.code, Code::InventoryMismatch);
    assert_eq!(report.exit_code(), 9);
}

#[test]
fn a_wrong_table_hash_is_a_table_identity_mismatch() {
    let sandbox = Sandbox::new();
    let (mut spec, files) = valid_run();
    spec.tables[0].table_hash ^= 1;
    let directory = materialize(&sandbox, &spec, &files);
    let report = verify(&directory);
    assert_eq!(report.code, Code::TableIdentityMismatch);
    assert_eq!(report.exit_code(), 9);
}

#[test]
fn false_paths_redacted_is_a_schema_violation() {
    let sandbox = Sandbox::new();
    let (mut spec, files) = valid_run();
    spec.paths_redacted = false;
    let directory = materialize(&sandbox, &spec, &files);
    let report = verify(&directory);
    assert_eq!(report.code, Code::ManifestSchemaInvalid);
    assert_eq!(report.exit_code(), 2);
}

#[test]
fn a_wrong_decoded_row_count_is_a_row_decode_mismatch() {
    let sandbox = Sandbox::new();
    let (mut spec, files) = valid_run();
    spec.artifacts[0].decoded_rows = Some(999);
    let directory = materialize(&sandbox, &spec, &files);
    let report = verify(&directory);
    assert_eq!(report.code, Code::RowDecodeMismatch);
    assert_eq!(report.exit_code(), 9);
}

#[test]
fn a_missing_output_directory_is_input_io() {
    let sandbox = Sandbox::new();
    let missing = sandbox.root.join("absent");
    let report = verify(&missing);
    assert_eq!(report.code, Code::InputIo);
    assert_eq!(report.exit_code(), 4);
}

#[test]
fn a_missing_manifest_is_input_io() {
    let sandbox = Sandbox::new();
    let empty = sandbox.out();
    let report = verify(&empty);
    assert_eq!(report.code, Code::InputIo);
    assert_eq!(report.exit_code(), 4);
}

#[test]
fn a_symlinked_output_is_an_unsafe_path() {
    let sandbox = Sandbox::new();
    let (spec, files) = valid_run();
    let real = materialize(&sandbox, &spec, &files);
    let link = sandbox.root.join("link");
    symlink(&real, &link).unwrap();
    let report = verify(&link);
    assert_eq!(report.code, Code::UnsafePath);
    assert_eq!(report.exit_code(), 9);
}

#[test]
fn a_non_complete_status_is_incomplete() {
    let sandbox = Sandbox::new();
    let (mut spec, files) = valid_run();
    spec.status = "incomplete";
    spec.reason = Some("one table failed body decode");
    let directory = materialize(&sandbox, &spec, &files);
    // Materialize does not write the marker; the manifest field alone triggers.
    let report = verify(&directory);
    assert_eq!(report.code, Code::IncompleteRun);
    assert_eq!(report.exit_code(), 8);
}

#[test]
fn an_marker_file_makes_a_self_declared_complete_run_incomplete() {
    let sandbox = Sandbox::new();
    let (spec, files) = valid_run();
    let directory = materialize(&sandbox, &spec, &files);
    fs::write(directory.join(MARKER_NAME), b"{}").unwrap();
    let report = verify(&directory);
    assert_eq!(report.code, Code::IncompleteRun);
    assert_eq!(report.exit_code(), 8);
}
