// SPDX-License-Identifier: GPL-3.0-or-later
//! Synthetic verification tests. They build a temporary manifest plus `.db2`
//! files from the in-tree WDC5 fixture generator: no CASC, no native storage,
//! no real client bytes and no network. All temporary directories live under
//! `TMPDIR` (externally configured) and are removed on drop.
use super::{Code, MARKER_NAME, verify};
use forever_db2::CharacterTable;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::symlink;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

#[test]
fn json_depth_policy_bounds_recursion_and_ignores_escaped_string_content() {
    let at_limit = format!("{}null{}", "[".repeat(64), "]".repeat(64));
    assert!(super::json::parse(&at_limit).is_ok());
    let over_limit = format!("[{at_limit}]");
    assert!(super::json::parse(&over_limit).is_err());
    let mixed = format!("{}null{}", "{\"x\":[".repeat(32), "]}".repeat(32));
    assert!(super::json::parse(&mixed).is_ok());
    assert!(super::json::parse(&format!("[{mixed}]")).is_err());
    let quoted = format!(r#"{{"text":"{}\"\\[]{{}}"}}"#, "[".repeat(256));
    assert!(super::json::parse(&quoted).is_ok());
    for invalid in ["]", "[", "\"unterminated", "{]"] {
        assert!(super::json::parse(invalid).is_err());
    }
}

#[cfg(unix)]
#[test]
fn deeply_nested_manifest_and_companion_report_fail_below_byte_limit() {
    let nested = format!("{}null{}", "[".repeat(65), "]".repeat(65));
    assert!((nested.len() as u64) < super::MAX_MANIFEST_BYTES);
    for name in [super::MANIFEST_NAME, super::REPORT_NAME] {
        let sandbox = Sandbox::new();
        let output = acquired_run(&sandbox, false, false);
        fs::write(output.join(name), &nested).unwrap();
        assert_eq!(verify(&output).code, Code::ManifestSchemaInvalid, "{name}");
    }
}

#[cfg(unix)]
fn catalog_run(sandbox: &Sandbox, name: &str) -> PathBuf {
    let table = CharacterTable::from_catalog(name).unwrap().unwrap();
    let plan = crate::batch::Plan::parse_with_catalog(
        &format!("{name}\t{}\n", table.file_id()),
        "pinned-metadata-v1",
    )
    .unwrap();
    let client = sandbox.root.join("client");
    fs::create_dir_all(&client).unwrap();
    let output = sandbox.root.join("catalog");
    let report = crate::batch::extract(&plan, &client, &output, 70205, "enUS", |_, _| {
        Ok((70205, crate::batch::tests::catalog_empty_fixture(name)))
    })
    .unwrap();
    assert!(report.complete);
    output
}

fn change_manifest(output: &std::path::Path, from: &str, to: &str) {
    let path = output.join(super::MANIFEST_NAME);
    let original = fs::read_to_string(&path).unwrap();
    assert!(original.contains(from), "mutation absent: {from}");
    fs::write(path, original.replace(from, to)).unwrap();
}

#[cfg(unix)]
#[test]
fn catalog_v3_verifies_additional_and_reviewed_tables() {
    for name in [
        "LiquidType",
        "UiWidgetVisTypeDataReq",
        "ChrClasses",
        "ChrRaces",
    ] {
        let sandbox = Sandbox::new();
        let output = catalog_run(&sandbox, name);
        assert_eq!(verify(&output).exit_code(), 0, "{name}");
    }
}

#[cfg(unix)]
#[test]
fn catalog_v3_rejects_changed_provenance_selector_or_version() {
    for (from, to) in [
        ("\"manifest_version\":3", "\"manifest_version\":2"),
        ("\"manifest_version\":3", "\"manifest_version\":4"),
        (
            "\"scope\":\"explicit_db2_catalog_batch\"",
            "\"scope\":\"explicit_db2_batch\"",
        ),
        ("\"id\":\"pinned-metadata-v1\"", "\"id\":\"other\""),
        (
            "\"source_commit\":\"02245dcd245e7433e524577656177723d3e4992e\"",
            "\"source_commit\":\"12245dcd245e7433e524577656177723d3e4992e\"",
        ),
        (
            "\"source_blob\":\"d06b97bad72ca2aecad3987ce9c348ac6055e0db\"",
            "\"source_blob\":\"e06b97bad72ca2aecad3987ce9c348ac6055e0db\"",
        ),
        (
            "897bde5aa04bab3b40ce4fe2e34d2127385b11d908d5266dce0fae7b52e23def",
            "197bde5aa04bab3b40ce4fe2e34d2127385b11d908d5266dce0fae7b52e23def",
        ),
        (
            "0f1ed0c419e851078d99f46ad3b37cadddb27b0556a2d1b52d3fd9ab62b76ec9",
            "1f1ed0c419e851078d99f46ad3b37cadddb27b0556a2d1b52d3fd9ab62b76ec9",
        ),
        (",\"schema_catalog\":\"pinned-metadata-v1\"", ""),
        (
            "\"table_selection\":\"explicit_manifest\"",
            "\"table_selection\":\"default_character_tables\"",
        ),
    ] {
        let sandbox = Sandbox::new();
        let output = catalog_run(&sandbox, "LiquidType");
        change_manifest(&output, from, to);
        assert_eq!(verify(&output).code, Code::ManifestSchemaInvalid, "{from}");
    }
}

#[cfg(unix)]
#[test]
fn catalog_v3_rejects_unknown_identity_counts_paths_and_downgraded_claims() {
    for (from, to) in [
        ("LiquidType", "liquidtype"),
        ("LiquidType", "UnknownTable"),
        ("1371380", "1371381"),
        ("\"entry_count\":1", "\"entry_count\":2"),
        (
            "\"relative_path\":\"LiquidType.db2\"",
            "\"relative_path\":\"Other.db2\"",
        ),
        (
            "\"table_schema_validated\":true",
            "\"table_schema_validated\":false",
        ),
        (
            "\"record_data_decoded\":true",
            "\"record_data_decoded\":false",
        ),
        ("\"decoded_rows\":0", "\"decoded_rows\":null"),
        ("\"total_bytes\":288", "\"total_bytes\":268435457"),
    ] {
        let sandbox = Sandbox::new();
        let output = catalog_run(&sandbox, "LiquidType");
        change_manifest(&output, from, to);
        assert_eq!(verify(&output).code, Code::ManifestSchemaInvalid, "{from}");
    }
}

#[cfg(unix)]
#[test]
fn catalog_v3_rechecks_meta_index_declared_count_and_decoded_rows() {
    for (from, to, code) in [
        (
            "\"meta_index_field\":-1",
            "\"meta_index_field\":0",
            Code::TableIdentityMismatch,
        ),
        (
            "\"declared_records\":0",
            "\"declared_records\":1",
            Code::TableIdentityMismatch,
        ),
        (
            "\"decoded_rows\":0",
            "\"decoded_rows\":1",
            Code::RowDecodeMismatch,
        ),
    ] {
        let sandbox = Sandbox::new();
        let output = catalog_run(&sandbox, "LiquidType");
        change_manifest(&output, from, to);
        assert_eq!(verify(&output).code, code);
    }
}

#[cfg(unix)]
#[test]
fn catalog_v3_keeps_known_table_hash_gate_after_artifact_rehash() {
    let sandbox = Sandbox::new();
    let output = catalog_run(&sandbox, "ChrClasses");
    let path = output.join("ChrClasses.db2");
    let old = fs::read(&path).unwrap();
    let mut changed = old.clone();
    put32(&mut changed, 152, 0);
    fs::write(path, &changed).unwrap();
    let old_hash = crate::sha256::hex(&old);
    let new_hash = crate::sha256::hex(&changed);
    let old_inventory = crate::sha256::hex(format!("{old_hash}  ChrClasses.db2").as_bytes());
    let new_inventory = crate::sha256::hex(format!("{new_hash}  ChrClasses.db2").as_bytes());
    change_manifest(&output, &old_hash, &new_hash);
    change_manifest(&output, &old_inventory, &new_inventory);
    change_manifest(
        &output,
        &format!(
            "\"table_hash\":{}",
            CharacterTable::Classes.schema_identity().0
        ),
        "\"table_hash\":0",
    );
    assert_eq!(verify(&output).code, Code::TableIdentityMismatch);
}

#[test]
fn v2_cannot_acquire_catalog_claim_by_changing_only_version() {
    let sandbox = Sandbox::new();
    let (spec, files) = valid_run();
    let output = materialize(&sandbox, &spec, &files);
    assert_eq!(verify(&output).exit_code(), 0);
    change_manifest(&output, "\"manifest_version\":2", "\"manifest_version\":3");
    assert_eq!(verify(&output).code, Code::ManifestSchemaInvalid);
}

#[cfg(unix)]
#[test]
fn catalog_v3_rejects_duplicate_entries_even_with_matching_counts() {
    let sandbox = Sandbox::new();
    let output = catalog_run(&sandbox, "LiquidType");
    let path = output.join(super::MANIFEST_NAME);
    let mut text = fs::read_to_string(&path).unwrap();
    for key in ["tables", "artifacts"] {
        let prefix = format!("\"{key}\":[");
        let start = text.find(&prefix).unwrap() + prefix.len();
        let end = start + text[start..].find(']').unwrap();
        let duplicate = format!(",{}", &text[start..end]);
        text.insert_str(end, &duplicate);
    }
    text = text.replace("\"entry_count\":1", "\"entry_count\":2");
    fs::write(path, text).unwrap();
    assert_eq!(verify(&output).code, Code::ManifestSchemaInvalid);
}

#[cfg(unix)]
#[test]
fn catalog_v3_redecodes_rehashed_metadata_only_or_extra_tail_files() {
    for truncated in [true, false] {
        let sandbox = Sandbox::new();
        let output = catalog_run(&sandbox, "LiquidType");
        let path = output.join("LiquidType.db2");
        let old = fs::read(&path).unwrap();
        let mut changed = old.clone();
        if truncated {
            changed.truncate(204);
        } else {
            changed.push(0);
        }
        fs::write(path, &changed).unwrap();
        let old_hash = crate::sha256::hex(&old);
        let new_hash = crate::sha256::hex(&changed);
        let old_inventory = crate::sha256::hex(format!("{old_hash}  LiquidType.db2").as_bytes());
        let new_inventory = crate::sha256::hex(format!("{new_hash}  LiquidType.db2").as_bytes());
        change_manifest(&output, &old_hash, &new_hash);
        change_manifest(&output, &old_inventory, &new_inventory);
        change_manifest(
            &output,
            "\"bytes\":288",
            &format!("\"bytes\":{}", changed.len()),
        );
        change_manifest(
            &output,
            "\"total_bytes\":288",
            &format!("\"total_bytes\":{}", changed.len()),
        );
        assert_eq!(verify(&output).code, Code::RowDecodeMismatch);
    }
}

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

#[cfg(unix)]
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

#[cfg(unix)]
fn acquired_run(sandbox: &Sandbox, catalog: bool, metadata_only: bool) -> PathBuf {
    let name = if catalog || metadata_only {
        "LiquidType"
    } else {
        "ChrClasses"
    };
    let table = CharacterTable::from_catalog(name).unwrap().unwrap();
    let text = if metadata_only {
        "UnknownTable\t123\n".to_owned()
    } else {
        format!("{name}\t{}\n", table.file_id())
    };
    let plan = if catalog {
        crate::batch::Plan::parse_with_catalog(&text, "pinned-metadata-v1").unwrap()
    } else {
        crate::batch::Plan::parse(&text).unwrap()
    };
    let client = sandbox.root.join("client");
    fs::create_dir_all(&client).unwrap();
    let output = sandbox.root.join("acquired");
    let report = crate::batch::extract_with_acquisition(
        &plan,
        &client,
        &output,
        70205,
        "enUS",
        Some(crate::batch::tests::rust_local_profile(1024)),
        |_, _| Ok((70205, crate::batch::tests::catalog_empty_fixture(name))),
    )
    .unwrap();
    assert!(report.complete);
    output
}

#[cfg(unix)]
#[test]
fn acquired_v4_verifies_both_selectors_and_preserves_metadata_only_claims() {
    for (catalog, metadata_only) in [(false, false), (false, true), (true, false)] {
        let sandbox = Sandbox::new();
        let output = acquired_run(&sandbox, catalog, metadata_only);
        assert_eq!(verify(&output).exit_code(), 0);
        let manifest = fs::read_to_string(output.join(super::MANIFEST_NAME)).unwrap();
        assert!(manifest.contains("\"manifest_version\":4"));
        if metadata_only {
            assert!(manifest.contains("\"table_schema_validated\":false,\"record_data_decoded\":false,\"decoded_rows\":null"));
        }
    }
}

#[cfg(unix)]
#[test]
fn acquired_v4_rejects_profile_selector_budget_and_version_tampering() {
    for (from, to) in [
        ("\"manifest_version\":4", "\"manifest_version\":3"),
        ("\"manifest_version\":4", "\"manifest_version\":2"),
        ("\"manifest_version\":4", "\"manifest_version\":5"),
        (
            "\"schema_selection\":\"reviewed\"",
            "\"schema_selection\":\"catalog\"",
        ),
        ("\"backend\":\"rust-local\"", "\"backend\":\"native\""),
        ("local-v7-ckey-root-nz-v1", "local-v7-ckey-root-nz-v2"),
        ("\"offline\":true", "\"offline\":false"),
        ("\"complete_ckey_md5\":true", "\"complete_ckey_md5\":false"),
        (
            "\"ekey_validation\":\"prefix9\"",
            "\"ekey_validation\":\"full\"",
        ),
        (
            "\"root_layout\":\"header-50893-v2\"",
            "\"root_layout\":\"auto\"",
        ),
        ("\"file_bytes\":1024", "\"file_bytes\":2048"),
        ("\"max_file_bytes\":1024", "\"max_file_bytes\":2048"),
        ("\"entry_count\":1", "\"entry_count\":2"),
        (
            "\"max_batch_bytes\":268435456",
            "\"max_batch_bytes\":268435457",
        ),
    ] {
        let sandbox = Sandbox::new();
        let output = acquired_run(&sandbox, false, false);
        change_manifest(&output, from, to);
        assert_eq!(verify(&output).code, Code::ManifestSchemaInvalid, "{from}");
    }
}

#[cfg(unix)]
#[test]
fn acquired_v4_requires_its_profile_and_companion_report_to_match() {
    for mutation in 0..5 {
        let sandbox = Sandbox::new();
        let output = acquired_run(&sandbox, true, false);
        let report = output.join(super::REPORT_NAME);
        if mutation == 0 {
            fs::remove_file(&report).unwrap();
        } else if mutation == 1 {
            let bytes = fs::read_to_string(&report).unwrap();
            fs::write(
                &report,
                bytes.replace("\"file_bytes\":1024", "\"file_bytes\":512"),
            )
            .unwrap();
        } else if mutation == 2 {
            let bytes = fs::read_to_string(&report).unwrap();
            fs::write(
                &report,
                bytes.replace(
                    "\"schema_selection\":\"catalog\"",
                    "\"schema_selection\":\"reviewed\"",
                ),
            )
            .unwrap();
        } else if mutation == 3 {
            let path = output.join(super::MANIFEST_NAME);
            let text = fs::read_to_string(&path).unwrap();
            let profile = crate::batch::tests::rust_local_profile(1024).render();
            fs::write(
                path,
                text.replace(&format!(",\"acquisition\":{profile}"), ""),
            )
            .unwrap();
        } else {
            let path = output.join(super::MANIFEST_NAME);
            let text = fs::read_to_string(&path).unwrap();
            let profile = crate::batch::tests::rust_local_profile(1024).render();
            fs::write(
                path,
                text.replace(
                    &format!(",\"acquisition\":{profile}"),
                    &format!(",\"acquisition\":{profile},\"acquisition\":{profile}"),
                ),
            )
            .unwrap();
        }
        assert_eq!(
            verify(&output).code,
            Code::ManifestSchemaInvalid,
            "case {mutation}"
        );
    }
}

#[cfg(unix)]
#[test]
fn acquired_v4_cannot_exceed_lower_profile_cap_even_with_consistent_hashes() {
    let sandbox = Sandbox::new();
    let output = acquired_run(&sandbox, false, false);
    // Original empty ChrClasses file is 376 bytes. The 300-byte profile and
    // options agree with each other but cannot describe this artifact.
    change_manifest(&output, "\"file_bytes\":1024", "\"file_bytes\":300");
    change_manifest(&output, "\"max_file_bytes\":1024", "\"max_file_bytes\":300");
    assert_eq!(verify(&output).code, Code::ManifestSchemaInvalid);
}

#[cfg(unix)]
#[test]
fn acquired_v4_catalog_never_downgrades_to_metadata_only() {
    let sandbox = Sandbox::new();
    let output = acquired_run(&sandbox, true, false);
    change_manifest(
        &output,
        "\"record_data_decoded\":true,\"decoded_rows\":0",
        "\"record_data_decoded\":false,\"decoded_rows\":null",
    );
    assert_eq!(verify(&output).code, Code::ManifestSchemaInvalid);
}

#[cfg(unix)]
#[test]
fn acquired_v4_marker_and_failed_manifest_remain_incomplete() {
    let sandbox = Sandbox::new();
    let output = acquired_run(&sandbox, false, false);
    fs::write(output.join(MARKER_NAME), b"{}").unwrap();
    assert_eq!(verify(&output).code, Code::IncompleteRun);
}
