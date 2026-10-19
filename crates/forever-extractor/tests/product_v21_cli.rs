// SPDX-License-Identifier: GPL-3.0-or-later
//! Synthetic plans, CASC records and CLI diagnostics only; no real client or keys.
#![cfg(target_os = "linux")]

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};

#[path = "../../forever-casc/src/local_index/jenkins.rs"]
mod jenkins;
#[path = "../../forever-casc/src/blte/md5.rs"]
mod md5;

const V21: &str =
    include_str!("../../../tests/contracts/fixtures/forever-product-v21/navigation-map2.tsv");
const V18: &str =
    include_str!("../../../tests/contracts/fixtures/forever-product-v18/all-available-maps.tsv");
const V19: &str =
    include_str!("../../../tests/contracts/fixtures/forever-product-v19/disabled.tsv");
const V20: &str = include_str!("../../../tests/contracts/fixtures/forever-product-v20/map2.tsv");
static NEXT: AtomicU64 = AtomicU64::new(0);
const TACT_NAME: u64 = 0x0102_0304_0506_0708;
const TEST_KEY: [u8; 16] = [0x42; 16];
const ROOT_EKEY: [u8; 16] = [0x33; 16];
const ENCODING_EKEY: [u8; 16] = [0x44; 16];
const CONTENT_EKEY: [u8; 16] = [0x66; 16];
type Route = ([u8; 16], u32, Vec<[u8; 16]>);

struct Lab(PathBuf);
impl Lab {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "rce-product21-cli-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
    fn plan(&self, name: &str, text: &str) -> PathBuf {
        let path = self.0.join(name);
        fs::write(&path, text).unwrap();
        path
    }
    fn scratch(&self, name: &str) -> PathBuf {
        let path = self.0.join(name);
        fs::create_dir(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        path
    }
    fn keys(&self, name: u64) -> PathBuf {
        let path = self.0.join("synthetic-keys.tsv");
        let key_hex: String = TEST_KEY.iter().map(|byte| format!("{byte:02x}")).collect();
        fs::write(
            &path,
            format!("forever-tact-keys\t1\n{name:016x}\t{key_hex}\n"),
        )
        .unwrap();
        path
    }
    fn client(&self, file_data_id: u32) -> PathBuf {
        let client = self.0.join("synthetic-client");
        let data = client.join("Data/data");
        fs::create_dir_all(&data).unwrap();

        let source = b"synthetic encrypted source";
        let content_key = md5::digest(source);
        let root = root_file(file_data_id, content_key);
        let root_key = md5::digest(&root);
        let routes = [
            (root_key, root.len() as u32, vec![ROOT_EKEY]),
            (content_key, source.len() as u32, vec![CONTENT_EKEY]),
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
            (CONTENT_EKEY, encrypted_record()),
        ];
        let mut archive = Vec::new();
        let mut rows = Vec::new();
        for (key, record) in records {
            let offset = archive.len() as u64;
            archive.extend_from_slice(&[0xa5; 30]);
            archive.extend_from_slice(&record);
            rows.push(index_entry(
                key,
                offset,
                (archive.len() as u64 - offset) as u32,
            ));
        }
        fs::write(data.join("data.000"), archive).unwrap();
        for bucket in 0..16_u8 {
            let entries = if bucket == 0 {
                rows.clone()
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
    fn extract(
        &self,
        plan: &Path,
        client: &Path,
        output: &Path,
        scratch: Option<&Path>,
        keys: Option<&Path>,
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
        if let Some(scratch) = scratch {
            command.args(["--scratch"]).arg(scratch);
        }
        if let Some(keys) = keys {
            command.args(["--keys"]).arg(keys);
        }
        if let Some(diagnostic) = diagnostic {
            command.env("FOREVER_PRODUCT_DIAGNOSTIC", diagnostic);
        }
        command.output().unwrap()
    }
    fn command(&self, verb: &str, plan: &PathBuf, extra: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_forever-extractor"))
            .arg(verb)
            .arg("--plan")
            .arg(plan)
            .args(extra)
            .output()
            .unwrap()
    }
}
impl Drop for Lab {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn stderr(output: Output) -> String {
    String::from_utf8(output.stderr).unwrap()
}

fn typed_source_failure(record: &str, phase: &str, file_data_id: u32, blte_code: &str) {
    assert_eq!(record.lines().count(), 1, "{record}");
    assert!(
        record.contains(&format!("\"phase\":\"{phase}\"")),
        "{record}"
    );
    assert!(
        record.contains("\"request_kind\":\"db2_table\""),
        "{record}"
    );
    assert!(
        record.contains(&format!("\"file_data_id\":{file_data_id}")),
        "{record}"
    );
    assert!(
        record.contains("\"variant\":\"NoLocalEncoding\""),
        "{record}"
    );
    assert!(record.contains("\"encrypted\":1"), "{record}");
    assert!(
        record.contains(&format!("\"blte_code\":\"{blte_code}\"")),
        "{record}"
    );
    assert!(
        record.contains("\"code\":\"local_store_no_verified_encoding\""),
        "{record}"
    );
    for forbidden in [
        "synthetic-client".to_owned(),
        "synthetic-keys".to_owned(),
        "content_key".to_owned(),
        "encoded_key".to_owned(),
        "ciphertext".to_owned(),
        "bytes".to_owned(),
        hex16(TEST_KEY),
        format!("{TACT_NAME:016x}"),
        format!("{:016x}", TACT_NAME + 1),
    ] {
        assert!(
            !record.contains(&forbidden),
            "unexpected {forbidden}: {record}"
        );
    }
}

fn inspect(plan: &Path, diagnostic: Option<&Path>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_forever-extractor"));
    command
        .args(["inspect", "--plan"])
        .arg(plan)
        .env_remove("FOREVER_PRODUCT_DIAGNOSTIC");
    if let Some(diagnostic) = diagnostic {
        command.env("FOREVER_PRODUCT_DIAGNOSTIC", diagnostic);
    }
    command.output().unwrap()
}

fn hex16(key: [u8; 16]) -> String {
    key.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn root_file(file_data_id: u32, content_key: [u8; 16]) -> Vec<u8> {
    let mut bytes = b"TSFM".to_vec();
    for value in [20_u32, 2, 1, 0, 1, u32::MAX, 0x1000_0000, 0] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes.push(0);
    bytes.extend_from_slice(&file_data_id.to_le_bytes());
    bytes.extend_from_slice(&content_key);
    bytes
}

fn unencrypted_record(bytes: &[u8]) -> Vec<u8> {
    let mut record = b"BLTE\0\0\0\0N".to_vec();
    record.extend_from_slice(bytes);
    record
}

fn encrypted_record() -> Vec<u8> {
    let nonce = [1_u8, 2, 3, 4];
    let mut record = b"BLTE\0\0\0\0".to_vec();
    record.extend_from_slice(&[b'E', 8]);
    record.extend_from_slice(&TACT_NAME.to_le_bytes());
    record.push(nonce.len() as u8);
    record.extend_from_slice(&nonce);
    record.push(b'S');
    record.extend_from_slice(b"synthetic ciphertext");
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

#[test]
fn v18_keyed_compatible_gate_is_not_reclassified_as_a_typed_source_failure() {
    let lab = Lab::new();
    let plan = lab.plan("v18.tsv", V18);
    let client = lab.client(1_266_277);
    let keys = lab.keys(TACT_NAME + 1);
    let scratch = lab.scratch("scratch");

    let without_diagnostic = lab.0.join("v18-no-diagnostic-output");
    let diagnostic = lab.0.join("v18-diagnostic.jsonl");
    let without = lab.extract(
        &plan,
        &client,
        &without_diagnostic,
        Some(&scratch),
        Some(&keys),
        None,
    );
    assert_eq!(without.status.code(), Some(9));
    let error = stderr(without);
    assert!(
        error.contains("available_source_keys_not_admitted"),
        "{error}"
    );
    assert!(!diagnostic.exists());

    let with_diagnostic = lab.0.join("v18-with-diagnostic-output");
    let with = lab.extract(
        &plan,
        &client,
        &with_diagnostic,
        Some(&scratch),
        Some(&keys),
        Some(&diagnostic),
    );
    assert_eq!(with.status.code(), Some(9));
    assert!(stderr(with).contains("available_source_keys_not_admitted"));
    assert!(!diagnostic.exists());
}

#[test]
fn v19_db2_source_failure_is_typed_and_opt_in() {
    let lab = Lab::new();
    let plan = lab.plan("v19.tsv", V19);
    let client = lab.client(1_294_214);

    let without_diagnostic = lab.0.join("v19-no-diagnostic-output");
    let diagnostic = lab.0.join("v19-diagnostic.jsonl");
    let without = lab.extract(&plan, &client, &without_diagnostic, None, None, None);
    assert_eq!(without.status.code(), Some(9));
    let without_stdout = without.stdout.clone();
    let without_stderr = without.stderr.clone();
    let error = String::from_utf8(without.stderr).unwrap();
    assert!(
        error.contains("local_store_no_verified_encoding"),
        "{error}"
    );
    assert!(!diagnostic.exists());

    let with_diagnostic = lab.0.join("v19-with-diagnostic-output");
    let with = lab.extract(
        &plan,
        &client,
        &with_diagnostic,
        None,
        None,
        Some(&diagnostic),
    );
    assert_eq!(with.status.code(), Some(9));
    assert_eq!(with.stdout, without_stdout);
    assert_eq!(with.stderr, without_stderr);
    assert!(stderr(with).contains("local_store_no_verified_encoding"));
    let record = fs::read_to_string(&diagnostic).unwrap();
    typed_source_failure(&record, "db2", 1_294_214, "blte_encrypted_unsupported");
}

#[test]
fn v21_domain_map_failure_is_typed_and_opt_in_before_package_creation() {
    let lab = Lab::new();
    let plan = lab.plan("v21.tsv", V21);
    let client = lab.client(1_349_477);
    let diagnostic = lab.0.join("v21-diagnostic.jsonl");

    let scratch = lab.scratch("scratch-without-diagnostic");
    let without_output = lab.0.join("v21-no-diagnostic-output");
    let without = lab.extract(&plan, &client, &without_output, Some(&scratch), None, None);
    assert_eq!(without.status.code(), Some(9));
    let without_stdout = without.stdout.clone();
    let without_stderr = without.stderr.clone();
    assert!(
        String::from_utf8(without.stderr)
            .unwrap()
            .contains("local_store_no_verified_encoding")
    );
    assert!(!without_output.exists());
    assert!(!diagnostic.exists());

    let scratch = lab.scratch("scratch-with-diagnostic");
    let with_output = lab.0.join("v21-with-diagnostic-output");
    let with = lab.extract(
        &plan,
        &client,
        &with_output,
        Some(&scratch),
        None,
        Some(&diagnostic),
    );
    assert_eq!(with.status.code(), Some(9));
    assert_eq!(with.stdout, without_stdout);
    assert_eq!(with.stderr, without_stderr);
    assert!(stderr(with).contains("local_store_no_verified_encoding"));
    assert!(!with_output.exists());
    let record = fs::read_to_string(&diagnostic).unwrap();
    typed_source_failure(&record, "db2", 1_349_477, "blte_encrypted_unsupported");
}

#[test]
fn diagnostics_do_not_change_successful_inspection_for_build_70450_profiles() {
    for (name, text) in [("v18", V18), ("v19", V19), ("v20", V20), ("v21", V21)] {
        let lab = Lab::new();
        let plan = lab.plan("plan.tsv", text);
        let baseline = inspect(&plan, None);
        assert!(baseline.status.success(), "{name}: {}", stderr(baseline));
        let expected_stdout = baseline.stdout;
        let expected_stderr = baseline.stderr;
        let diagnostic = lab.0.join("unused-diagnostic.jsonl");
        let with_diagnostic = inspect(&plan, Some(&diagnostic));
        assert_eq!(with_diagnostic.status.code(), Some(0), "{name}");
        assert_eq!(with_diagnostic.stdout, expected_stdout, "{name}");
        assert_eq!(with_diagnostic.stderr, expected_stderr, "{name}");
        assert!(!diagnostic.exists(), "{name}");
    }
}

#[test]
fn diagnostics_reject_client_and_package_output_paths_for_covered_profiles() {
    for (name, text, file_data_id, needs_scratch, needs_keys) in [
        ("v18", V18, 1_266_277, true, true),
        ("v19", V19, 1_294_214, false, false),
        ("v21", V21, 1_349_477, true, false),
    ] {
        let lab = Lab::new();
        let plan = lab.plan("plan.tsv", text);
        let client = lab.client(file_data_id);
        let scratch = needs_scratch.then(|| lab.scratch("scratch"));
        let keys = needs_keys.then(|| lab.keys(TACT_NAME + 1));

        let output = lab.0.join("client-path-output");
        let inside_client = client.join("diagnostic.jsonl");
        let result = lab.extract(
            &plan,
            &client,
            &output,
            scratch.as_deref(),
            keys.as_deref(),
            Some(&inside_client),
        );
        assert!(
            stderr(result).contains("product_diagnostic_path"),
            "{name} diagnostic inside client"
        );
        assert!(!inside_client.exists());

        let output = lab.0.join("package-output");
        fs::create_dir(&output).unwrap();
        let inside_output = output.join("diagnostic.jsonl");
        let result = lab.extract(
            &plan,
            &client,
            &output,
            scratch.as_deref(),
            keys.as_deref(),
            Some(&inside_output),
        );
        assert!(
            stderr(result).contains("product_diagnostic_path"),
            "{name} diagnostic inside package output"
        );
        assert!(!inside_output.exists());
    }
}

#[test]
fn v21_plan_only_inspection_reports_the_navigation_profile_and_map_domain() {
    let lab = Lab::new();
    let plan = lab.plan("v21.tsv", V21);
    let result = lab.command("inspect", &plan, &[]);
    assert!(result.status.success());
    let stdout = String::from_utf8(result.stdout).unwrap();
    for fact in [
        "\"contract\":\"forever-product-v21\"",
        "\"build\":70450",
        "\"components\":57",
        "\"schema_catalog\":\"wowdbdefs-70450-v1\"",
        "\"decryption\":\"disabled\"",
        "\"collision_storage\":\"linux-leased-append-disk-world-v1\"",
        "\"map_domain\":\"map-and-ancestors-v1\"",
        "\"requested_map\":2",
        "\"gameobject_scope\":\"global-startup-v1\"",
        "\"placement_ids\":\"selected-closure-serial-ids-v1\"",
        "\"full_extractor_complete\":false",
    ] {
        assert!(stdout.contains(fact), "missing {fact}: {stdout}");
    }
}

#[test]
fn v21_cli_refuses_keys_before_opening_client_or_output_paths() {
    let lab = Lab::new();
    let plan = lab.plan("v21.tsv", V21);
    let client = lab.0.join("synthetic-client-not-opened");
    let output = lab.0.join("output-not-created");
    let key_path = lab.0.join("keys-not-opened");
    let scratch = lab.0.join("scratch-not-opened");
    for verb in ["inspect", "extract", "resume"] {
        let mut extra = vec![
            "--client",
            client.to_str().unwrap(),
            "--keys",
            key_path.to_str().unwrap(),
        ];
        if verb != "inspect" {
            extra.extend(["--output", output.to_str().unwrap()]);
            extra.extend(["--scratch", scratch.to_str().unwrap()]);
        }
        let result = lab.command(verb, &plan, &extra);
        assert_eq!(result.status.code(), Some(2), "{verb}");
        assert!(stderr(result).contains("product_keys_forbidden"), "{verb}");
        assert!(!key_path.exists());
        assert!(!output.exists());
    }
}
