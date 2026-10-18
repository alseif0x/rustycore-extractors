// SPDX-License-Identifier: GPL-3.0-or-later
//! Synthetic v17 plans, keys and CASC records only; no real client or keys.
#![cfg(target_os = "linux")]
use forever_db2::{CharacterTable, DecodedTable, catalog};
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};

#[path = "../../forever-casc/src/local_index/jenkins.rs"]
mod jenkins;
#[path = "../../forever-casc/src/blte/md5.rs"]
mod md5;
#[path = "../../forever-casc/src/keys/salsa20.rs"]
mod salsa20;

static NEXT: AtomicU64 = AtomicU64::new(0);
const FILE_DATA_ID: u32 = 6_220_124;
const TACT_NAME: u64 = 0x0102_0304_0506_0708;
const TEST_KEY: [u8; 16] = [0x42; 16];
const WRONG_KEY: [u8; 16] = [0x99; 16];
const ROOT_EKEY: [u8; 16] = [0x33; 16];
const ENCODING_EKEY: [u8; 16] = [0x44; 16];
const DB2_EKEY: [u8; 16] = [0x66; 16];
type Route = ([u8; 16], u32, Vec<[u8; 16]>);

struct Lab(PathBuf);

impl Lab {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "rce-product17-cli-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn write(&self, name: &str, contents: &str) -> PathBuf {
        let path = self.0.join(name);
        fs::write(&path, contents).unwrap();
        path
    }

    fn plan(&self) -> PathBuf {
        self.write("v17.tsv", V17)
    }

    fn key_file(&self, key: [u8; 16]) -> PathBuf {
        self.key_file_named(TACT_NAME, key)
    }

    fn key_file_named(&self, name: u64, key: [u8; 16]) -> PathBuf {
        let path = self.0.join("supplied-keys.tsv");
        fs::write(&path, key_list(name, key)).unwrap();
        path
    }

    fn client(&self) -> PathBuf {
        self.build_client(false)
    }

    fn client_missing_db2_archive(&self) -> PathBuf {
        self.build_client(true)
    }

    fn build_client(&self, missing_db2_archive: bool) -> PathBuf {
        let client = self.0.join("synthetic-client");
        let data = client.join("Data/data");
        fs::create_dir_all(&data).unwrap();

        let db2 = empty_account_store_category();
        let content_key = md5::digest(&db2);
        let mut root = b"TSFM".to_vec();
        for value in [20_u32, 2, 1, 0] {
            root.extend_from_slice(&value.to_le_bytes());
        }
        for value in [1_u32, 2, 0x1000_0000, 0] {
            root.extend_from_slice(&value.to_le_bytes());
        }
        root.push(0);
        root.extend_from_slice(&FILE_DATA_ID.to_le_bytes());
        root.extend_from_slice(&content_key);

        let root_key = md5::digest(&root);
        let routes = [
            (root_key, root.len() as u32, vec![ROOT_EKEY]),
            (content_key, db2.len() as u32, vec![DB2_EKEY]),
        ];
        let encoding = encoding_file(&routes);
        let encoding_key = md5::digest(&encoding);
        let config = format!(
            "root = {}\nencoding = {} {}\nencoding-size = {} {}\nbuild-uid = wow_classic_beta\nbuild-name = synthetic-1.60.1.70450\n",
            hex16(root_key),
            hex16(encoding_key),
            hex16(ENCODING_EKEY),
            encoding.len(),
            encoding.len() + 39
        )
        .into_bytes();
        let config_key = md5::digest(&config);
        let config_hex = hex16(config_key);
        let config_path = client
            .join("Data/config")
            .join(&config_hex[..2])
            .join(&config_hex[2..4]);
        fs::create_dir_all(&config_path).unwrap();
        fs::write(config_path.join(&config_hex), config).unwrap();
        fs::write(
            client.join(".build.info"),
            format!(
                "Product!STRING:0|Active!DEC:1|Version!STRING:0|Build Key!HEX:16|CDN Key!HEX:16|Tags!STRING:0\nwow_classic_beta|1|1.60.1.70450|{}|{}|enUS\n",
                hex16(config_key),
                hex16([0x55; 16])
            ),
        )
        .unwrap();

        let records = [
            (ROOT_EKEY, unencrypted_record(&root)),
            (ENCODING_EKEY, unencrypted_record(&encoding)),
            (DB2_EKEY, encrypted_db2_record(&db2)),
        ];
        let mut archive = Vec::new();
        let mut index_rows = Vec::new();
        for (key, record) in records {
            let offset = archive.len() as u64;
            archive.extend_from_slice(&[0xa5; 30]);
            archive.extend_from_slice(&record);
            let index_offset = if missing_db2_archive && key == DB2_EKEY {
                offset | (1_u64 << 30)
            } else {
                offset
            };
            index_rows.push(index_entry(
                key,
                index_offset,
                (archive.len() as u64 - offset) as u32,
            ));
        }
        fs::write(data.join("data.000"), archive).unwrap();
        for bucket in 0..16_u8 {
            let entries = if bucket == 0 {
                index_rows.clone()
            } else {
                vec![index_entry([0xe0 + bucket; 16], 0, 39)]
            };
            fs::write(
                data.join(format!("{bucket:02x}00000001.idx")),
                index_file(bucket, &entries),
            )
            .unwrap();
        }
        client
    }
}

impl Drop for Lab {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

const V17: &str =
    include_str!("../../../tests/contracts/fixtures/forever-product-v17/provided.tsv");
const V14: &str =
    include_str!("../../../tests/contracts/fixtures/forever-product-v14/all-available-maps.tsv");
const V15: &str =
    include_str!("../../../tests/contracts/fixtures/forever-product-v15/all-available-maps.tsv");
const V16: &str =
    include_str!("../../../tests/contracts/fixtures/forever-product-v16/disabled.tsv");

fn hex16(key: [u8; 16]) -> String {
    key.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn put32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn empty_account_store_category() -> Vec<u8> {
    let metadata =
        catalog::resolve_for(catalog::WOWDBDEFS_70450_CATALOG_ID, "AccountStoreCategory")
            .unwrap()
            .unwrap()
            .metadata();
    let schema = CharacterTable::from_catalog_id(
        catalog::WOWDBDEFS_70450_CATALOG_ID,
        "AccountStoreCategory",
    )
    .unwrap()
    .unwrap();
    let mut bytes = vec![0; 204 + 4 * metadata.file_field_count as usize];
    bytes[..4].copy_from_slice(b"WDC5");
    put32(&mut bytes, 4, 5);
    put32(&mut bytes, 140, metadata.file_field_count as u32);
    put32(&mut bytes, 152, metadata.table_hash.unwrap());
    put32(&mut bytes, 156, metadata.layout_hash);
    put32(&mut bytes, 176, metadata.file_field_count as u32);
    assert!(
        DecodedTable::read(&bytes, schema)
            .unwrap()
            .rows()
            .is_empty()
    );
    bytes
}

fn encrypted_db2_record(bytes: &[u8]) -> Vec<u8> {
    let nonce = [1_u8, 2, 3, 4];
    let mut full_nonce = [0_u8; 8];
    full_nonce[..4].copy_from_slice(&nonce);
    let mut ciphertext = Vec::with_capacity(bytes.len() + 1);
    ciphertext.push(b'N');
    ciphertext.extend_from_slice(bytes);
    salsa20::xor(&TEST_KEY, full_nonce, &mut ciphertext);

    let mut record = b"BLTE\0\0\0\0".to_vec();
    record.extend_from_slice(&[b'E', 8]);
    record.extend_from_slice(&TACT_NAME.to_le_bytes());
    record.push(nonce.len() as u8);
    record.extend_from_slice(&nonce);
    record.push(b'S');
    record.extend_from_slice(&ciphertext);
    record
}

fn unencrypted_record(bytes: &[u8]) -> Vec<u8> {
    let mut record = b"BLTE\0\0\0\0N".to_vec();
    record.extend_from_slice(bytes);
    record
}

fn encoding_file(routes: &[Route]) -> Vec<u8> {
    let mut page = Vec::new();
    for (key, size, ekeys) in routes {
        page.extend_from_slice(&(ekeys.len() as u16).to_le_bytes());
        page.extend_from_slice(&size.to_be_bytes());
        page.extend_from_slice(key);
        for ekey in ekeys {
            page.extend_from_slice(ekey);
        }
    }
    assert!(page.len() < 1024);
    page.resize(1024, 0);
    let mut bytes = b"EN\x01\x10\x10".to_vec();
    bytes.extend_from_slice(&1_u16.to_be_bytes());
    bytes.extend_from_slice(&1_u16.to_be_bytes());
    bytes.extend_from_slice(&1_u32.to_be_bytes());
    bytes.extend_from_slice(&0_u32.to_be_bytes());
    bytes.push(0);
    bytes.extend_from_slice(&0_u32.to_be_bytes());
    bytes.extend_from_slice(&routes[0].0);
    bytes.extend_from_slice(&md5::digest(&page));
    bytes.extend_from_slice(&page);
    bytes
}

fn index_entry(key: [u8; 16], offset: u64, size: u32) -> [u8; 18] {
    let mut row = [0; 18];
    row[..9].copy_from_slice(&key[..9]);
    row[9..14].copy_from_slice(&offset.to_be_bytes()[3..]);
    row[14..].copy_from_slice(&size.to_le_bytes());
    row
}

fn index_file(bucket: u8, rows: &[[u8; 18]]) -> Vec<u8> {
    let mut bytes = vec![0; 32];
    bytes[..4].copy_from_slice(&16_u32.to_le_bytes());
    bytes[8..10].copy_from_slice(&7_u16.to_le_bytes());
    bytes[10] = bucket;
    bytes[12..16].copy_from_slice(&[4, 5, 9, 30]);
    let header_hash = jenkins::hashlittle(&bytes[8..24], 0);
    bytes[4..8].copy_from_slice(&header_hash.to_le_bytes());
    bytes.extend_from_slice(&(rows.len() as u32 * 18).to_le_bytes());
    let mut checksum = (0, 0);
    for row in rows {
        checksum = jenkins::hashlittle2(row, checksum.0, checksum.1);
    }
    bytes.extend_from_slice(&checksum.0.to_le_bytes());
    for row in rows {
        bytes.extend_from_slice(row);
    }
    bytes
}

fn key_list(name: u64, key: [u8; 16]) -> Vec<u8> {
    format!("forever-tact-keys\t1\n{name:016x}\t{}\n", hex16(key)).into_bytes()
}

fn extract(
    plan: &Path,
    client: &Path,
    output: &Path,
    keys: Option<&Path>,
    scratch: Option<&Path>,
    diagnostic: Option<&Path>,
) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_forever-extractor"));
    command
        .args(["extract", "--plan"])
        .arg(plan)
        .args(["--client"])
        .arg(client)
        .args(["--output"])
        .arg(output)
        .env_remove("FOREVER_PRODUCT_DIAGNOSTIC");
    if let Some(keys) = keys {
        command.args(["--keys"]).arg(keys);
    }
    if let Some(scratch) = scratch {
        command.args(["--scratch"]).arg(scratch);
    }
    if let Some(diagnostic) = diagnostic {
        command.env("FOREVER_PRODUCT_DIAGNOSTIC", diagnostic);
    }
    command.output().unwrap()
}

fn top_level_keys(json: &str) -> BTreeSet<String> {
    let bytes = json.trim_end().as_bytes();
    assert_eq!(bytes.first(), Some(&b'{'));
    let mut fields = Vec::new();
    let mut start = 1;
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for (index, &byte) in bytes.iter().enumerate().skip(1) {
        if in_string {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                in_string = false;
            }
            continue;
        }
        match byte {
            b'"' => in_string = true,
            b'{' | b'[' => depth += 1,
            b'}' | b']' if depth != 0 => depth -= 1,
            b',' if depth == 0 => {
                fields.push(&json[start..index]);
                start = index + 1;
            }
            b'}' => {
                fields.push(&json[start..index]);
                break;
            }
            _ => {}
        }
    }
    fields
        .into_iter()
        .map(|field| {
            field
                .split_once(':')
                .unwrap()
                .0
                .trim()
                .trim_matches('"')
                .to_owned()
        })
        .collect()
}

fn nested_object_keys(json: &str, field: &str) -> BTreeSet<String> {
    let marker = format!("\"{field}\":");
    let value_start = json.find(&marker).unwrap() + marker.len();
    let bytes = json.as_bytes();
    let start = value_start
        + bytes[value_start..]
            .iter()
            .position(|byte| *byte == b'{')
            .unwrap();
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for (index, &byte) in bytes.iter().enumerate().skip(start) {
        if in_string {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                in_string = false;
            }
            continue;
        }
        match byte {
            b'"' => in_string = true,
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return top_level_keys(&json[start..=index]);
                }
            }
            _ => {}
        }
    }
    unreachable!("unterminated JSON object")
}

fn assert_diagnostic_record_key_sets(record: &str, typed_cause: &[&str], candidate_failures: bool) {
    assert_eq!(
        top_level_keys(record),
        BTreeSet::from([
            "phase".to_owned(),
            "request_kind".to_owned(),
            "file_data_id".to_owned(),
            "ordinal".to_owned(),
            "typed_cause".to_owned(),
            "code".to_owned(),
            "guards_observed".to_owned(),
            "tolerated_absences".to_owned(),
        ])
    );
    assert_eq!(
        nested_object_keys(record, "typed_cause"),
        typed_cause
            .iter()
            .map(|field| (*field).to_owned())
            .collect()
    );
    if candidate_failures {
        assert_eq!(
            nested_object_keys(record, "candidate_failures"),
            BTreeSet::from([
                "missing".to_owned(),
                "incomplete".to_owned(),
                "encrypted".to_owned(),
                "corrupt".to_owned(),
                "unsupported".to_owned(),
            ])
        );
    }
}

#[test]
fn v17_plan_only_inspection_reports_keyed_identity_without_loading_keys() {
    let lab = Lab::new();
    let plan = lab.plan();
    let result = Command::new(env!("CARGO_BIN_EXE_forever-extractor"))
        .args(["inspect", "--plan"])
        .arg(plan)
        .output()
        .unwrap();
    assert!(result.status.success());
    let report = String::from_utf8(result.stdout).unwrap();
    assert!(report.contains("\"contract\":\"forever-product-v17\""));
    assert!(report.contains("\"build\":70450"));
    assert!(report.contains("\"decryption\":\"provided-salsa20-v1\""));
    assert!(report.contains("\"full_extractor_complete\":false"));
}

#[test]
fn v17_client_commands_require_keys_before_client_access() {
    for command_name in ["inspect", "extract", "resume"] {
        let lab = Lab::new();
        let plan = lab.plan();
        let output_path = lab.0.join("missing-key-output");
        let mut command = Command::new(env!("CARGO_BIN_EXE_forever-extractor"));
        command.args([command_name, "--plan"]).arg(&plan);
        if command_name == "inspect" {
            command.args(["--client", "/nonexistent-v17-client"]);
        } else {
            command
                .args(["--client", "/nonexistent-v17-client", "--output"])
                .arg(&output_path);
        }
        let result = command.output().unwrap();
        assert_eq!(result.status.code(), Some(2), "{command_name}");
        let error = String::from_utf8(result.stderr).unwrap();
        assert!(
            error.contains("product_keys_required"),
            "{command_name}: {error}"
        );
        assert!(
            error.contains("forever-product-v17"),
            "{command_name}: {error}"
        );
        assert!(!output_path.exists());
    }
}

#[test]
fn disabled_v14_v15_v16_revisions_still_reject_keys() {
    for (revision, contents, scratch) in [(14, V14, true), (15, V15, true), (16, V16, false)] {
        let lab = Lab::new();
        let plan = lab.write(&format!("v{revision}.tsv"), contents);
        let output_path = lab.0.join("disabled-keys-output");
        let result = extract(
            &plan,
            Path::new("/nonexistent-disabled-client"),
            &output_path,
            Some(Path::new("/nonexistent-disabled-keys")),
            scratch.then(|| Path::new("/nonexistent-disabled-scratch")),
            None,
        );
        assert_eq!(result.status.code(), Some(2), "v{revision}");
        let error = String::from_utf8(result.stderr).unwrap();
        assert!(
            error.contains("product_keys_forbidden"),
            "v{revision}: {error}"
        );
        assert!(
            error.contains(&format!("forever-product-v{revision}")),
            "{error}"
        );
        assert!(!output_path.exists());
    }
}

#[test]
fn wrong_key_for_resolved_tact_name_fails_typed_and_discloses_no_key_material() {
    let lab = Lab::new();
    let plan = lab.plan();
    let client = lab.client();
    let key_file = lab.key_file(WRONG_KEY);
    let diagnostic = lab.0.join("wrong-key-diagnostic.jsonl");
    let output_path = lab.0.join("wrong-key-output");
    let result = extract(
        &plan,
        &client,
        &output_path,
        Some(&key_file),
        None,
        Some(&diagnostic),
    );
    assert_eq!(result.status.code(), Some(9));
    let stdout = String::from_utf8(result.stdout).unwrap();
    let stderr = String::from_utf8(result.stderr).unwrap();
    assert!(
        stderr.contains("local_store_no_verified_encoding"),
        "{stderr}"
    );
    assert!(stderr.contains("\"component\":\"db2\""), "{stderr}");
    assert!(
        !output_path
            .join("dbc/enUS/AccountStoreCategory.db2")
            .exists()
    );

    let key_hex = hex16(WRONG_KEY);
    let name_hex = format!("{TACT_NAME:016x}");
    let key_path = key_file.to_string_lossy();
    assert!(!stdout.contains(&key_hex));
    assert!(!stderr.contains(&key_hex));
    assert!(!stderr.contains(&name_hex));
    assert!(!stderr.contains(key_path.as_ref()));

    let report = fs::read_to_string(&diagnostic).unwrap();
    assert_eq!(report.lines().count(), 1);
    assert_eq!(
        top_level_keys(&report),
        BTreeSet::from([
            "phase".to_owned(),
            "request_kind".to_owned(),
            "file_data_id".to_owned(),
            "ordinal".to_owned(),
            "typed_cause".to_owned(),
            "code".to_owned(),
            "guards_observed".to_owned(),
            "tolerated_absences".to_owned(),
        ])
    );
    assert!(report.contains("\"variant\":\"NoLocalEncoding\""));
    assert!(report.contains("\"code\":\"local_store_no_verified_encoding\""));
    assert!(!report.contains(&key_hex));
    assert!(!report.contains(&name_hex));
    assert!(!report.contains(key_path.as_ref()));
}

#[test]
fn absent_tact_name_key_is_reported_as_blte_key_unavailable() {
    let lab = Lab::new();
    let plan = lab.plan();
    let client = lab.client();
    let key_file = lab.key_file_named(TACT_NAME + 1, TEST_KEY);
    let diagnostic = lab.0.join("missing-key-diagnostic.jsonl");
    let output_path = lab.0.join("missing-key-output");
    let result = extract(
        &plan,
        &client,
        &output_path,
        Some(&key_file),
        None,
        Some(&diagnostic),
    );
    assert_eq!(result.status.code(), Some(9));
    let stderr = String::from_utf8(result.stderr).unwrap();
    assert!(
        stderr.contains("local_store_no_verified_encoding"),
        "{stderr}"
    );
    let record = fs::read_to_string(&diagnostic).unwrap();
    assert_eq!(record.lines().count(), 1);
    assert_diagnostic_record_key_sets(
        &record,
        &["variant", "candidate_failures", "blte_code"],
        true,
    );
    assert!(record.contains("\"blte_code\":\"blte_encryption_key_unavailable\""));
    assert!(record.contains("\"encrypted\":1"));
    assert!(!record.contains(&hex16(TEST_KEY)));
    assert!(!record.contains(&format!("{TACT_NAME:016x}")));
    assert!(!record.contains(&format!("{:016x}", TACT_NAME + 1)));
    assert!(!record.contains(key_file.to_string_lossy().as_ref()));
}

#[test]
fn keyless_encrypted_frame_is_reported_as_blte_decryption_unsupported() {
    let lab = Lab::new();
    let plan_text = V16.replace("locale\tesES\n", "locale\tenUS\n");
    let plan = lab.write("v16.tsv", &plan_text);
    let client = lab.client();
    let diagnostic = lab.0.join("unsupported-diagnostic.jsonl");
    let output_path = lab.0.join("unsupported-output");
    let result = extract(&plan, &client, &output_path, None, None, Some(&diagnostic));
    assert_eq!(result.status.code(), Some(9));
    let stderr = String::from_utf8(result.stderr).unwrap();
    assert!(
        stderr.contains("local_store_no_verified_encoding"),
        "{stderr}"
    );
    let record = fs::read_to_string(&diagnostic).unwrap();
    assert_eq!(record.lines().count(), 1);
    assert_diagnostic_record_key_sets(
        &record,
        &["variant", "candidate_failures", "blte_code"],
        true,
    );
    assert!(record.contains("\"blte_code\":\"blte_encrypted_unsupported\""));
    assert!(record.contains("\"encrypted\":1"));
    assert!(!record.contains(&hex16(TEST_KEY)));
    assert!(!record.contains(&format!("{TACT_NAME:016x}")));
}

#[test]
fn missing_selected_file_emits_no_blte_code_or_extra_record_fields() {
    let lab = Lab::new();
    let plan_text = V17
        .replace("AccountStoreCategory", "Map")
        .replace("6220124", "1349477");
    let plan = lab.write("missing-map.tsv", &plan_text);
    let client = lab.client();
    let key_file = lab.key_file_named(TACT_NAME + 1, TEST_KEY);
    let diagnostic = lab.0.join("missing-file-diagnostic.jsonl");
    let output_path = lab.0.join("missing-file-output");
    let result = extract(
        &plan,
        &client,
        &output_path,
        Some(&key_file),
        None,
        Some(&diagnostic),
    );
    assert!(!result.status.success());
    assert!(
        String::from_utf8(result.stderr)
            .unwrap()
            .contains("file_unavailable")
    );
    let record = fs::read_to_string(&diagnostic).unwrap();
    assert_eq!(record.lines().count(), 1);
    assert_diagnostic_record_key_sets(&record, &["variant"], false);
    assert!(record.contains("\"typed_cause\":{\"variant\":\"MissingFile\"}"));
    assert!(!record.contains("blte_code"));
    assert!(!record.contains(&hex16(TEST_KEY)));
    assert!(!record.contains(&format!("{:016x}", TACT_NAME + 1)));
    assert!(!record.contains(key_file.to_string_lossy().as_ref()));
}

#[test]
fn missing_archive_candidate_keeps_missing_counter_and_omits_blte_code() {
    let lab = Lab::new();
    let plan = lab.plan();
    let client = lab.client_missing_db2_archive();
    let key_file = lab.key_file(TEST_KEY);
    let diagnostic = lab.0.join("missing-archive-diagnostic.jsonl");
    let output_path = lab.0.join("missing-archive-output");
    let result = extract(
        &plan,
        &client,
        &output_path,
        Some(&key_file),
        None,
        Some(&diagnostic),
    );
    assert_eq!(result.status.code(), Some(9));
    let stderr = String::from_utf8(result.stderr).unwrap();
    assert!(
        stderr.contains("local_store_no_verified_encoding"),
        "{stderr}"
    );
    let record = fs::read_to_string(&diagnostic).unwrap();
    assert_diagnostic_record_key_sets(&record, &["variant", "candidate_failures"], true);
    assert!(record.contains("\"missing\":1"));
    assert!(record.contains("\"encrypted\":0"));
    assert!(!record.contains("blte_code"));
    assert!(!record.contains(&hex16(TEST_KEY)));
    assert!(!record.contains(&format!("{TACT_NAME:016x}")));
    assert!(!record.contains(key_file.to_string_lossy().as_ref()));
}

#[test]
fn matching_supplied_key_fully_decodes_the_selected_db2_source() {
    let lab = Lab::new();
    let plan = lab.plan();
    let client = lab.client();
    let key_file = lab.key_file(TEST_KEY);

    let inspect = Command::new(env!("CARGO_BIN_EXE_forever-extractor"))
        .args(["inspect", "--plan"])
        .arg(&plan)
        .args(["--client"])
        .arg(&client)
        .args(["--keys"])
        .arg(&key_file)
        .output()
        .unwrap();
    assert!(
        inspect.status.success(),
        "{}",
        String::from_utf8_lossy(&inspect.stderr)
    );
    let inspected = String::from_utf8(inspect.stdout).unwrap();
    assert!(inspected.contains("\"status\":\"admitted\""));
    assert!(!inspected.contains(&hex16(TEST_KEY)));
    assert!(!inspected.contains(&format!("{TACT_NAME:016x}")));
    assert!(!inspected.contains(key_file.to_string_lossy().as_ref()));

    let output_path = lab.0.join("decoded-output");
    let diagnostic = lab.0.join("decoded-diagnostic.jsonl");
    let result = extract(
        &plan,
        &client,
        &output_path,
        Some(&key_file),
        None,
        Some(&diagnostic),
    );
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let stdout = String::from_utf8(result.stdout).unwrap();
    assert!(stdout.contains("\"status\":\"selected_producers_complete\""));
    assert!(stdout.contains("\"full_extractor_complete\":false"));
    assert!(!stdout.contains(&hex16(TEST_KEY)));
    assert!(!stdout.contains(&format!("{TACT_NAME:016x}")));
    let manifest =
        String::from_utf8(fs::read(output_path.join(forever_casc::package::MANIFEST)).unwrap())
            .unwrap();
    assert!(!manifest.contains(&hex16(TEST_KEY)));
    assert!(!manifest.contains(&format!("{TACT_NAME:016x}")));
    assert!(!manifest.contains(key_file.to_string_lossy().as_ref()));
    assert_eq!(
        fs::read(output_path.join("dbc/enUS/AccountStoreCategory.db2")).unwrap(),
        empty_account_store_category()
    );
    assert!(!diagnostic.exists());

    let verify = Command::new(env!("CARGO_BIN_EXE_forever-extractor"))
        .args(["verify", "--plan"])
        .arg(plan)
        .args(["--output"])
        .arg(&output_path)
        .output()
        .unwrap();
    assert!(
        verify.status.success(),
        "{}",
        String::from_utf8_lossy(&verify.stderr)
    );
}
