// SPDX-License-Identifier: GPL-3.0-or-later
//! Synthetic inspect-db2 CLI refusals; no client, keys or network are read.
#![cfg(target_os = "linux")]

use forever_db2::catalog;
use std::{
    collections::BTreeSet,
    fs,
    path::PathBuf,
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};

#[path = "../../forever-casc/src/local_index/jenkins.rs"]
mod jenkins;
#[path = "../../forever-casc/src/blte/md5.rs"]
mod md5;
mod synthetic_fixture {
    include!("../../forever-db2/src/records/sparse/synthetic_fixture.rs");
    define_synthetic_sparse_fixture!(forever_db2);
}

const V16: &str =
    include_str!("../../../tests/contracts/fixtures/forever-product-v16/disabled.tsv");
const V10: &str =
    include_str!("../../../tests/contracts/fixtures/forever-product-v10/disabled.tsv");
const V20: &str = include_str!("../../../tests/contracts/fixtures/forever-product-v20/map2.tsv");
static NEXT: AtomicU64 = AtomicU64::new(0);
const ROOT_EKEY: [u8; 16] = [0x33; 16];
const ENCODING_EKEY: [u8; 16] = [0x44; 16];
const DB2_EKEY: [u8; 16] = [0x55; 16];

struct Lab(PathBuf);
impl Lab {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "rce-inspect-db2-cli-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
    fn plan(&self, name: &str, contents: &str) -> PathBuf {
        let path = self.0.join(name);
        fs::write(&path, contents).unwrap();
        path
    }

    fn synthetic_client(&self) -> PathBuf {
        const FILE_DATA_ID: u32 = 5_152_908;
        let db2 =
            synthetic_fixture::empty_parented_sparse_70450("CollectableSourceEncounterSparse");
        assert_eq!(
            catalog::resolve_for(
                catalog::WOWDBDEFS_70450_CATALOG_ID,
                "CollectableSourceEncounterSparse"
            )
            .unwrap()
            .unwrap()
            .metadata()
            .file_data_id,
            FILE_DATA_ID
        );
        let db2_ckey = md5::digest(&db2);

        let mut root = b"TSFM".to_vec();
        for value in [20_u32, 2, 3, 0] {
            root.extend_from_slice(&value.to_le_bytes());
        }
        for value in [1_u32, 0, 0x1000_0000, 0] {
            root.extend_from_slice(&value.to_le_bytes());
        }
        root.push(0);
        root.extend_from_slice(&FILE_DATA_ID.to_le_bytes());
        root.extend_from_slice(&db2_ckey);
        let root_ckey = md5::digest(&root);
        let routes = [
            (root_ckey, root.len() as u32, ROOT_EKEY),
            (db2_ckey, db2.len() as u32, DB2_EKEY),
        ];
        let encoding = encoding_file(&routes);
        let encoding_ckey = md5::digest(&encoding);
        let config = format!(
            "root = {}\nencoding = {} {}\nencoding-size = {} {}\nbuild-uid = wow_classic_beta\nbuild-name = synthetic-1.60.1.70450\n",
            hex16(root_ckey),
            hex16(encoding_ckey),
            hex16(ENCODING_EKEY),
            encoding.len(),
            encoding.len() + 39
        )
        .into_bytes();
        let config_ckey = md5::digest(&config);
        let config_name = hex16(config_ckey);
        let client = self.0.join("synthetic-client");
        let data = client.join("Data/data");
        let config_dir = client
            .join("Data/config")
            .join(&config_name[..2])
            .join(&config_name[2..4]);
        fs::create_dir_all(&data).unwrap();
        fs::create_dir_all(&config_dir).unwrap();
        fs::write(config_dir.join(&config_name), &config).unwrap();
        fs::write(
            client.join(".build.info"),
            format!(
                "Product!STRING:0|Active!DEC:1|Version!STRING:0|Build Key!HEX:16|CDN Key!HEX:16|Tags!STRING:0\nwow_classic_beta|1|1.60.1.70450|{}|{}|enUS\n",
                hex16(config_ckey),
                hex16([0x21; 16])
            ),
        )
        .unwrap();

        let records = [
            (ROOT_EKEY, n_record(&root)),
            (ENCODING_EKEY, n_record(&encoding)),
            (DB2_EKEY, n_record(&db2)),
        ];
        let mut archive = Vec::new();
        let mut index_rows = Vec::new();
        for (ekey, record) in records {
            let offset = archive.len() as u64;
            archive.extend_from_slice(&[0xa5; 30]);
            archive.extend_from_slice(&record);
            index_rows.push(index_entry(
                ekey,
                offset,
                u32::try_from(30 + record.len()).unwrap(),
            ));
        }
        fs::write(data.join("data.000"), archive).unwrap();
        for bucket in 0..16_u8 {
            let rows = if bucket == 0 {
                index_rows.clone()
            } else {
                vec![index_entry([0xe0 + bucket; 16], 0, 39)]
            };
            fs::write(
                data.join(format!("{bucket:02x}00000001.idx")),
                index_file(bucket, &rows),
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

fn inspect(plan: &PathBuf, client: &PathBuf, output: &PathBuf, extra: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_forever-extractor"))
        .args(["inspect-db2", "--plan"])
        .arg(plan)
        .args(["--client"])
        .arg(client)
        .args(["--output"])
        .arg(output)
        .args(extra)
        .output()
        .unwrap()
}

fn hex16(bytes: [u8; 16]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn encoding_file(rows: &[([u8; 16], u32, [u8; 16])]) -> Vec<u8> {
    let mut page = Vec::new();
    for (ckey, content_size, ekey) in rows {
        page.extend_from_slice(&1_u16.to_le_bytes());
        page.extend_from_slice(&content_size.to_be_bytes());
        page.extend_from_slice(ckey);
        page.extend_from_slice(ekey);
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
    bytes.extend_from_slice(&rows[0].0);
    bytes.extend_from_slice(&md5::digest(&page));
    bytes.extend_from_slice(&page);
    bytes
}

fn n_record(content: &[u8]) -> Vec<u8> {
    let mut record = b"BLTE\0\0\0\0N".to_vec();
    record.extend_from_slice(content);
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
    bytes.extend_from_slice(&u32::try_from(rows.len() * 18).unwrap().to_le_bytes());
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

fn error_code(output: &Output, code: &str, exit: i32) {
    assert_eq!(output.status.code(), Some(exit));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(code),
        "expected {code}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn inspect_db2_help_documents_keyless_report_only_scope() {
    let result = Command::new(env!("CARGO_BIN_EXE_forever-extractor"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(result.status.success());
    let help = String::from_utf8(result.stdout).unwrap();
    assert!(help.contains("forever-extractor inspect-db2 --plan PLAN"));
    assert!(help.contains("extracts nothing; --keys is forbidden"));
}

#[test]
fn inspect_db2_synthetic_store_writes_only_the_exact_redacted_report() {
    let lab = Lab::new();
    let plan = lab.plan("v20.tsv", V20);
    let client = lab.synthetic_client();
    let report_dir = lab.0.join("reports");
    fs::create_dir(&report_dir).unwrap();
    let report_path = report_dir.join("inspection.json");

    let result = inspect(&plan, &client, &report_path, &["--file-data-id", "5152908"]);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(result.stderr.is_empty());
    assert_eq!(
        String::from_utf8(result.stdout).unwrap(),
        "{\"contract\":\"forever-db2-inspection-contract-v1\",\"status\":\"inspection_written\",\"build\":70450,\"declarations\":1,\"keyless\":true,\"extracts_nothing\":true}\n"
    );
    assert_eq!(
        fs::read_to_string(&report_path).unwrap(),
        concat!(
            "{\"schema\":\"forever-db2-inspection-contract-v1\",\"build\":70450,\"locale\":\"enUS\",\"keyless\":true,\"extracts_nothing\":true,\"files\":[",
            "{\"file_data_id\":5152908,\"table_name\":\"CollectableSourceEncounterSparse\",\"total_decoded_size_bytes\":212,\"header_magic\":\"WDC5\",\"header_version\":5,\"header_flags\":7,\"section_count\":0,\"record_count\":0,\"field_count\":2,\"total_field_count\":2,\"min_id\":0,\"max_id\":0,\"packed_data_offset\":0,\"parent_lookup_count\":0,\"record_size\":8,\"string_block_size\":0,\"column_meta_size\":0,\"common_data_size\":0,\"pallet_data_size\":0,\"compression_kinds\":[],\"sections\":[],",
            "\"classification\":{\"outcome\":\"accepted_raw_only\",\"reason_code\":\"unsupported_record_variant\",\"reason\":\"product source validation succeeded without materializing decoded rows\"}}]}\n"
        )
    );
    let written = fs::read_dir(&report_dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect::<BTreeSet<_>>();
    assert_eq!(written, ["inspection.json".into()].into_iter().collect());
    assert!(!client.join("dbc").exists());
}

#[test]
fn inspect_db2_refuses_keys_and_non_70450_plans_before_client_access() {
    let lab = Lab::new();
    let v16 = lab.plan("v16.tsv", V16);
    let v10 = lab.plan("v10.tsv", V10);
    let missing_client = lab.0.join("client-not-opened");
    let output = lab.0.join("report-not-created.json");
    let key_path = lab.0.join("keys-not-opened");

    let keys = inspect(
        &v16,
        &missing_client,
        &output,
        &[
            "--file-data-id",
            "5152908",
            "--keys",
            key_path.to_str().unwrap(),
        ],
    );
    error_code(&keys, "product_keys_forbidden", 2);
    assert!(!key_path.exists());
    assert!(!output.exists());

    let foreign = inspect(
        &v10,
        &missing_client,
        &output,
        &["--file-data-id", "5152908"],
    );
    error_code(&foreign, "product_db2_inspection_profile", 2);
    assert!(!output.exists());
}

#[test]
fn inspect_db2_refuses_client_and_git_checkout_output_paths() {
    let lab = Lab::new();
    let plan = lab.plan("v16.tsv", V16);
    let client = lab.0.join("client");
    fs::create_dir_all(client.join("reports")).unwrap();
    let in_client = client.join("reports/inspection.json");
    let client_result = inspect(&plan, &client, &in_client, &["--file-data-id", "5152908"]);
    error_code(&client_result, "product_diagnostic_path", 9);
    assert!(!in_client.exists());

    let checkout = lab.0.join("checkout");
    fs::create_dir_all(checkout.join(".git")).unwrap();
    fs::create_dir_all(checkout.join("reports")).unwrap();
    let in_checkout = checkout.join("reports/inspection.json");
    let checkout_result = inspect(&plan, &client, &in_checkout, &["--file-data-id", "5152908"]);
    error_code(&checkout_result, "product_diagnostic_path", 9);
    assert!(!in_checkout.exists());
}
