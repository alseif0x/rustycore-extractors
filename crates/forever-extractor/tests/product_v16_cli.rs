// SPDX-License-Identifier: GPL-3.0-or-later
//! Synthetic plan and CASC source input only; no real client or key input is read.
#![cfg(target_os = "linux")]
use forever_db2::{CharacterTable, DecodedTable, catalog};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};

#[path = "../../forever-casc/src/local_index/jenkins.rs"]
mod jenkins;
#[path = "../../forever-casc/src/blte/md5.rs"]
mod md5;

static NEXT: AtomicU64 = AtomicU64::new(0);
const FILE_DATA_ID: u32 = 6_220_124;
const ROOT_EKEY: [u8; 16] = [0x33; 16];
const ENCODING_EKEY: [u8; 16] = [0x44; 16];
const DB2_EKEY: [u8; 16] = [0x66; 16];
type Route = ([u8; 16], u32, Vec<[u8; 16]>);

struct Lab(PathBuf);

impl Lab {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "rce-product16-cli-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn plan(&self) -> PathBuf {
        let path = self.0.join("plan.tsv");
        fs::write(&path, PLAN).unwrap();
        path
    }

    fn en_us_plan(&self) -> PathBuf {
        let path = self.0.join("plan.tsv");
        fs::write(&path, PLAN.replace("locale\tesES", "locale\tenUS")).unwrap();
        path
    }

    fn client(&self, encrypted: bool) -> PathBuf {
        let client = self.0.join("client");
        let data = client.join("Data/data");
        fs::create_dir_all(&data).unwrap();

        let db2 = account_store_category();
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
            hex(root_key),
            hex(encoding_key),
            hex(ENCODING_EKEY),
            encoding.len(),
            encoding.len() + 39
        )
        .into_bytes();
        let config_key = md5::digest(&config);
        let config_hex = hex(config_key);
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
                hex(config_key),
                hex([0x55; 16])
            ),
        )
        .unwrap();

        let records = [
            (ROOT_EKEY, headerless_record(&root, false)),
            (ENCODING_EKEY, headerless_record(&encoding, false)),
            (DB2_EKEY, headerless_record(&db2, encrypted)),
        ];
        let mut archive = Vec::new();
        let mut index_rows = Vec::new();
        for (key, record) in records {
            let offset = archive.len() as u64;
            archive.extend_from_slice(&[0xa5; 30]);
            archive.extend_from_slice(&record);
            index_rows.push(index_entry(
                key,
                offset,
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

fn hex(key: [u8; 16]) -> String {
    key.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn put32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn account_store_category() -> Vec<u8> {
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

fn headerless_record(bytes: &[u8], encrypted: bool) -> Vec<u8> {
    let mut record = b"BLTE\0\0\0\0".to_vec();
    if encrypted {
        record.extend_from_slice(&[b'E', 8]);
        record.extend_from_slice(&[0x11; 8]);
        record.push(4);
        record.extend_from_slice(&[0; 4]);
        record.extend_from_slice(&[b'S', 0x22]);
    } else {
        record.push(b'N');
        record.extend_from_slice(bytes);
    }
    record
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

const PLAN: &str =
    include_str!("../../../tests/contracts/fixtures/forever-product-v16/disabled.tsv");

fn cli(args: &[&str], plan: &PathBuf) -> Output {
    Command::new(env!("CARGO_BIN_EXE_forever-extractor"))
        .args(args)
        .arg(plan)
        .output()
        .unwrap()
}

fn extract(plan: &Path, client: &Path, output: &Path, diagnostic: Option<&Path>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_forever-extractor"));
    command
        .args(["extract", "--plan"])
        .arg(plan)
        .args(["--client"])
        .arg(client)
        .args(["--output"])
        .arg(output);
    if let Some(diagnostic) = diagnostic {
        command.env("FOREVER_PRODUCT_DIAGNOSTIC", diagnostic);
    } else {
        command.env_remove("FOREVER_PRODUCT_DIAGNOSTIC");
    }
    command.output().unwrap()
}

#[test]
fn v16_help_and_plan_only_inspection_report_the_exact_db2_identity() {
    let help = Command::new(env!("CARGO_BIN_EXE_forever-extractor"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(help.status.success());
    assert!(
        String::from_utf8(help.stdout)
            .unwrap()
            .contains("Linux v16 separately selects disabled-key 70450 DB2-only")
    );

    let lab = Lab::new();
    let plan = lab.plan();
    let result = cli(&["inspect", "--plan"], &plan);
    assert!(result.status.success());
    let text = String::from_utf8(result.stdout).unwrap();
    assert!(text.contains("\"contract\":\"forever-product-v16\""));
    assert!(text.contains("\"build\":70450"));
    assert!(text.contains("\"components\":1"));
    assert!(text.contains("\"schema_catalog\":\"wowdbdefs-70450-v1\""));
    assert!(text.contains("\"decryption\":\"disabled\""));
    assert!(text.contains("\"full_extractor_complete\":false"));
}

#[test]
fn supplied_keys_for_v16_are_refused_before_client_or_key_input() {
    let lab = Lab::new();
    let plan = lab.plan();
    let output = lab.0.join("output");
    let result = Command::new(env!("CARGO_BIN_EXE_forever-extractor"))
        .args(["extract", "--plan"])
        .arg(&plan)
        .args(["--client", "/nonexistent-v16-client", "--output"])
        .arg(&output)
        .args(["--keys", "/nonexistent-v16-keys"])
        .output()
        .unwrap();
    let text = String::from_utf8(result.stderr).unwrap();
    assert_eq!(result.status.code(), Some(2));
    assert!(text.contains("product_keys_forbidden"), "{text}");
    assert!(text.contains("forever-product-v16"), "{text}");
    assert!(!output.exists());
}

#[test]
fn resident_v16_plan_refuses_scratch_before_client_or_output_io() {
    let lab = Lab::new();
    let plan = lab.plan();
    let output = lab.0.join("output");
    let result = Command::new(env!("CARGO_BIN_EXE_forever-extractor"))
        .args(["extract", "--plan"])
        .arg(&plan)
        .args(["--client", "/nonexistent-v16-client", "--output"])
        .arg(&output)
        .args(["--scratch", "/nonexistent-v16-scratch"])
        .output()
        .unwrap();
    let text = String::from_utf8(result.stderr).unwrap();
    assert_eq!(result.status.code(), Some(2));
    assert!(text.contains("product_scratch_forbidden"), "{text}");
    assert!(text.contains("forever-product-v16"), "{text}");
    assert!(!output.exists());
}

#[test]
fn verification_commands_do_not_accept_a_keys_option() {
    for command in ["verify", "verify-links"] {
        let lab = Lab::new();
        let plan = lab.plan();
        let output = lab.0.join("nonexistent-output");
        let result = Command::new(env!("CARGO_BIN_EXE_forever-extractor"))
            .args([command, "--plan"])
            .arg(&plan)
            .args(["--output"])
            .arg(&output)
            .args(["--keys", "/nonexistent-v16-keys"])
            .output()
            .unwrap();
        let text = String::from_utf8(result.stderr).unwrap();
        assert_eq!(result.status.code(), Some(2), "{command}: {text}");
        assert!(text.contains("product_usage"), "{command}: {text}");
        assert!(!output.exists());
    }
}

#[test]
fn db2_source_failure_diagnostic_is_typed_single_record_and_opt_in_only() {
    let diagnostic_lab = Lab::new();
    let plan = diagnostic_lab.en_us_plan();
    let client = diagnostic_lab.client(true);
    let diagnostic = diagnostic_lab.0.join("db2-diagnostic.jsonl");
    let with_output = diagnostic_lab.0.join("with-diagnostic");
    let with = extract(&plan, &client, &with_output, Some(&diagnostic));
    assert_eq!(with.status.code(), Some(9));
    assert_eq!(
        with.stderr,
        b"{\"contract\":\"forever-product-v16\",\"status\":\"failed\",\"code\":\"local_store_no_verified_encoding\",\"component\":\"db2\",\"full_extractor_complete\":false}\n"
    );
    assert!(
        !with_output
            .join("dbc/enUS/AccountStoreCategory.db2")
            .exists()
    );
    let record = fs::read_to_string(&diagnostic).unwrap();

    let no_diagnostic_lab = Lab::new();
    let no_diagnostic_plan = no_diagnostic_lab.en_us_plan();
    let no_diagnostic_client = no_diagnostic_lab.client(true);
    let no_diagnostic_report = no_diagnostic_lab.0.join("unset-diagnostic.jsonl");
    let without_output = no_diagnostic_lab.0.join("without-diagnostic");
    let without = extract(
        &no_diagnostic_plan,
        &no_diagnostic_client,
        &without_output,
        None,
    );
    assert_eq!(without.status.code(), Some(9));
    assert_eq!(without.stderr, with.stderr);
    assert!(!no_diagnostic_report.exists());
    assert_eq!(record.lines().count(), 1);
    assert_eq!(
        record,
        "{\"phase\":\"db2\",\"request_kind\":\"db2_table\",\"file_data_id\":6220124,\"ordinal\":0,\"typed_cause\":{\"variant\":\"NoLocalEncoding\",\"candidate_failures\":{\"missing\":0,\"incomplete\":0,\"encrypted\":1,\"corrupt\":0,\"unsupported\":0},\"blte_code\":\"blte_encrypted_unsupported\"},\"code\":\"local_store_no_verified_encoding\",\"guards_observed\":false,\"tolerated_absences\":{\"db2_table\":0,\"wdt\":0,\"tile_adt\":0,\"root_m2\":0,\"wmo_root\":0,\"wmo_group\":0,\"child\":0,\"gameobject_model\":0,\"other\":0}}\n"
    );
}

#[test]
fn successful_db2_component_with_diagnostic_enabled_writes_no_record() {
    let lab = Lab::new();
    let plan = lab.en_us_plan();
    let client = lab.client(false);
    let output = lab.0.join("successful-output");
    let diagnostic = lab.0.join("successful-diagnostic.jsonl");
    let result = extract(&plan, &client, &output, Some(&diagnostic));
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(
        String::from_utf8(result.stdout)
            .unwrap()
            .contains("\"status\":\"selected_producers_complete\"")
    );
    assert_eq!(
        fs::read(output.join("dbc/enUS/AccountStoreCategory.db2")).unwrap(),
        account_store_category()
    );
    assert!(!diagnostic.exists());
}
