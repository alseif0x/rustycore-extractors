// SPDX-License-Identifier: GPL-3.0-or-later
//! Synthetic build-70450 CASC inputs only; this test never opens a real client.
#![cfg(target_os = "linux")]

use forever_db2::catalog;
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

const CATALOG: &str = catalog::WOWDBDEFS_70450_CATALOG_ID;
const V20: &str = include_str!("../../../tests/contracts/fixtures/forever-product-v20/map2.tsv");
const V15: &str =
    include_str!("../../../tests/contracts/fixtures/forever-product-v15/all-available-maps.tsv");
const V17: &str =
    include_str!("../../../tests/contracts/fixtures/forever-product-v17/provided.tsv");
const V16: &str =
    include_str!("../../../tests/contracts/fixtures/forever-product-v16/disabled.tsv");
const V18: &str =
    include_str!("../../../tests/contracts/fixtures/forever-product-v18/all-available-maps.tsv");
const V19: &str =
    include_str!("../../../tests/contracts/fixtures/forever-product-v19/disabled.tsv");
const V14: &str =
    include_str!("../../../tests/contracts/fixtures/forever-product-v14/all-available-maps.tsv");
static NEXT: AtomicU64 = AtomicU64::new(0);

const ROOT_EKEY: [u8; 16] = [0x33; 16];
const ENCODING_EKEY: [u8; 16] = [0x44; 16];
const READABLE_EKEY: [u8; 16] = [0x55; 16];
const ENCRYPTED_EKEY: [u8; 16] = [0x66; 16];
const UNSUPPORTED_EKEY: [u8; 16] = [0x77; 16];

struct Lab(PathBuf);
impl Lab {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "rce-census-db2-cli-{}-{}",
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
    fn mixed_client(&self) -> (PathBuf, [u32; 4]) {
        let catalog = catalog::tables_for(CATALOG).unwrap();
        assert_eq!(catalog.len(), 1161);
        let ids = [
            catalog[0].file_data_id,
            catalog[1].file_data_id,
            catalog[2].file_data_id,
            catalog[3].file_data_id,
        ];
        assert_eq!(ids.into_iter().collect::<BTreeSet<_>>().len(), 4);

        let readable = b"synthetic readable DB2 source".to_vec();
        let encrypted_size = 23_u32;
        let unsupported = b"synthetic unsupported DB2 source".to_vec();
        let readable_ckey = md5::digest(&readable);
        let encrypted_ckey = [0xa1; 16];
        let unsupported_ckey = md5::digest(&unsupported);

        let mut root = b"TSFM".to_vec();
        for value in [20_u32, 2, 3, 0] {
            root.extend_from_slice(&value.to_le_bytes());
        }
        for (file_data_id, ckey) in [
            (ids[0], readable_ckey),
            (ids[1], encrypted_ckey),
            (ids[2], unsupported_ckey),
        ] {
            for value in [1_u32, 0, 0x1000_0000, 0] {
                root.extend_from_slice(&value.to_le_bytes());
            }
            root.push(0);
            root.extend_from_slice(&file_data_id.to_le_bytes());
            root.extend_from_slice(&ckey);
        }
        let root_ckey = md5::digest(&root);
        let routes = [
            (root_ckey, root.len() as u32, ROOT_EKEY),
            (readable_ckey, readable.len() as u32, READABLE_EKEY),
            (encrypted_ckey, encrypted_size, ENCRYPTED_EKEY),
            (unsupported_ckey, unsupported.len() as u32, UNSUPPORTED_EKEY),
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
            (READABLE_EKEY, n_record(&readable)),
            (ENCRYPTED_EKEY, encrypted_record()),
            (UNSUPPORTED_EKEY, unsupported_record(&unsupported)),
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
        (client, ids)
    }
}
impl Drop for Lab {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn command(plan: &Path, client: &Path, output: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_forever-extractor"));
    command
        .args(["census-db2", "--plan"])
        .arg(plan)
        .args(["--client"])
        .arg(client)
        .args(["--output"])
        .arg(output);
    command
}

#[test]
fn census_is_listed_in_help_with_its_keyless_catalogue_scope() {
    let result = Command::new(env!("CARGO_BIN_EXE_forever-extractor"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(result.status.success());
    let help = String::from_utf8(result.stdout).unwrap();
    assert!(
        help.contains(
            "forever-extractor census-db2 --plan PLAN --client CLIENT --output NEW_REPORT"
        )
    );
    assert!(help.contains("wowdbdefs-70450-v1, revisions 16+; --keys is forbidden"));
}

fn stderr(output: Output) -> String {
    String::from_utf8(output.stderr).unwrap()
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

fn encrypted_record() -> Vec<u8> {
    let mut record = b"BLTE\0\0\0\0E\x08".to_vec();
    record.extend([0x91; 8]); // Synthetic key name; never emitted by census.
    record.push(4);
    record.extend([1, 2, 3, 4]);
    record.extend([b'S', 0x72]);
    record
}

fn unsupported_record(content: &[u8]) -> Vec<u8> {
    let mut record = b"BLTE\0\0\0\0F".to_vec();
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

fn json_keys(input: &str) -> BTreeSet<String> {
    let bytes = input.as_bytes();
    let mut keys = BTreeSet::new();
    let mut cursor = 0;
    while cursor < bytes.len() {
        if bytes[cursor] != b'"' {
            cursor += 1;
            continue;
        }
        let start = cursor + 1;
        cursor = start;
        let mut escaped = false;
        while cursor < bytes.len() {
            if escaped {
                escaped = false;
            } else if bytes[cursor] == b'\\' {
                escaped = true;
            } else if bytes[cursor] == b'"' {
                break;
            }
            cursor += 1;
        }
        let end = cursor;
        cursor = cursor.saturating_add(1);
        let mut next = cursor;
        while next < bytes.len() && bytes[next].is_ascii_whitespace() {
            next += 1;
        }
        if bytes.get(next) == Some(&b':') {
            keys.insert(input[start..end].to_owned());
        }
    }
    keys
}

#[test]
fn mixed_keyless_census_retains_all_catalogue_declarations_and_aggregates_states() {
    let catalog = catalog::tables_for(CATALOG).unwrap();
    assert_eq!(catalog.len(), 1161);
    let lab = Lab::new();
    let (client, ids) = lab.mixed_client();
    let plan = lab.plan("v20.tsv", V20);
    let first = lab.0.join("census-a.json");
    let second = lab.0.join("census-b.json");
    for output in [&first, &second] {
        let result = command(&plan, &client, output).output().unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(result.stderr.is_empty());
        assert_eq!(
            String::from_utf8(result.stdout).unwrap(),
            "{\"contract\":\"forever-db2-census-contract-v1\",\"status\":\"census_written\",\"catalogue\":\"wowdbdefs-70450-v1\",\"declarations\":1161,\"keyless\":true,\"complete_extraction_claim\":false}\n"
        );
    }
    let report = fs::read(&first).unwrap();
    assert_eq!(
        report,
        fs::read(&second).unwrap(),
        "census is deterministic"
    );
    let report = String::from_utf8(report).unwrap();
    assert!(report.contains("\"declaration_count\":1161"));
    assert_eq!(report.matches("\"declaration_ordinal\":").count(), 1161);
    assert!(report.contains(
        "\"fully_readable\":1,\"encrypted_no_available_key\":1,\"file_unavailable\":1158,\"unsupported_or_unknown\":1"
    ));
    assert!(report.contains(&format!(
        "\"table_name\":\"{}\",\"file_data_id\":{},\"state\":\"fully_readable\"",
        catalog[0].name, ids[0]
    )));
    assert!(report.contains(&format!(
        "\"table_name\":\"{}\",\"file_data_id\":{},\"state\":\"encrypted_no_available_key\"",
        catalog[1].name, ids[1]
    )));
    assert!(report.contains(&format!(
        "\"table_name\":\"{}\",\"file_data_id\":{},\"state\":\"unsupported_or_unknown\"",
        catalog[2].name, ids[2]
    )));
    assert!(report.contains(&format!(
        "\"table_name\":\"{}\",\"file_data_id\":{},\"state\":\"file_unavailable\"",
        catalog[3].name, ids[3]
    )));
    assert!(report.contains("encrypted_candidate_no_key_available"));
    assert!(report.contains("blte_encrypted_unsupported"));
    assert!(report.contains("candidate_unsupported_or_unverified"));
    assert!(report.contains("blte_recursive_unsupported"));
    assert!(report.contains("complete_keyless_decode_and_ckey_check"));
    assert!(report.contains("no_central_present_eligible_root_row"));
    assert!(report.contains("\"complete_extraction_claim\":false"));
    assert!(report.contains("\"partial_result_expected_when_keys_unavailable\":true"));

    let key_set = json_keys(&report);
    let expected = [
        "schema",
        "catalogue",
        "build",
        "locale",
        "locale_mask",
        "build_info_tags",
        "declaration_count",
        "keyless_availability_census",
        "partial_result_expected_when_keys_unavailable",
        "complete_extraction_claim",
        "limits",
        "max_per_candidate_span_bytes",
        "max_per_declaration_span_bytes",
        "max_total_candidate_span_bytes",
        "aggregate",
        "fully_readable",
        "encrypted_no_available_key",
        "file_unavailable",
        "unsupported_or_unknown",
        "declarations",
        "declaration_ordinal",
        "table_name",
        "file_data_id",
        "state",
        "state_reason",
        "trace",
        "files",
        "root_rows_considered",
        "selected_root_ordinal",
        "encoding_row_count",
        "root_rows",
        "ordinal",
        "block_index",
        "locale_flags",
        "effective_content_flags",
        "audio_locale_bit",
        "eligible_for_locale",
        "ckey_central_present",
        "selected",
        "route_rows",
        "kind",
        "content_size",
        "encoding_count",
        "encodings",
        "span_count",
        "index_prefix_match",
        "read_result",
        "rejection_class",
        "blte_code",
        "candidates",
        "span_ordinal",
    ];
    assert_eq!(key_set, expected.into_iter().map(str::to_owned).collect());
    for forbidden in [
        "\"ckey\":",
        "\"ekey\":",
        "\"key_bytes\":",
        "\"bytes\":",
        "\"payload\":",
        "\"sha256\":",
        "data.000",
        "9191919191919191",
        "5555555555555555",
        "6666666666666666",
        "7777777777777777",
    ] {
        assert!(!report.contains(forbidden), "census leaked {forbidden}");
    }
}

#[test]
fn census_accepts_each_current_70450_catalogue_revision_without_keys() {
    const PLANS: [(&str, &str); 5] = [
        ("v16", V16),
        ("v17", V17),
        ("v18", V18),
        ("v19", V19),
        ("v20", V20),
    ];
    let lab = Lab::new();
    let (client, _) = lab.mixed_client();
    for (name, plan_text) in PLANS {
        let plan = lab.plan(&format!("{name}.tsv"), plan_text);
        let output = lab.0.join(format!("{name}.json"));
        let result = command(&plan, &client, &output).output().unwrap();
        assert!(
            result.status.success(),
            "{name}: {}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(
            fs::read_to_string(&output)
                .unwrap()
                .contains("\"keyless_availability_census\":true")
        );

        let key_path = lab.0.join(format!("{name}-keys-not-opened.tsv"));
        let keyed_output = lab.0.join(format!("{name}-keyed.json"));
        let result = command(&plan, &client, &keyed_output)
            .args(["--keys"])
            .arg(&key_path)
            .output()
            .unwrap();
        assert_eq!(result.status.code(), Some(2), "{name} --keys");
        assert!(stderr(result).contains("product_keys_forbidden"), "{name}");
        assert!(!key_path.exists());
        assert!(!keyed_output.exists());
    }
}

#[test]
fn census_refuses_wrong_build_catalogue_keys_and_protected_output_paths() {
    let lab = Lab::new();
    let v14 = lab.plan("v14.tsv", V14);
    let client = lab.0.join("client-not-opened");
    let output = lab.0.join("wrong-profile.json");
    let result = command(&v14, &client, &output).output().unwrap();
    assert_eq!(result.status.code(), Some(2));
    assert!(stderr(result).contains("product_probe_profile"));
    assert!(!output.exists());

    let v15 = lab.plan("v15.tsv", V15);
    let result = command(&v15, &client, &output).output().unwrap();
    assert_eq!(result.status.code(), Some(2));
    assert!(stderr(result).contains("product_probe_profile"));
    assert!(!output.exists());

    let foreign_catalog = lab.plan(
        "v16-foreign-catalogue.tsv",
        &include_str!("../../../tests/contracts/fixtures/forever-product-v16/disabled.tsv")
            .replace("wowdbdefs-70450-v1", "wowdbdefs-70374-v1"),
    );
    let result = command(&foreign_catalog, &client, &output)
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(2));
    assert!(stderr(result).contains("product_plan_schema_catalog"));
    assert!(!output.exists());

    let v17 = lab.plan("v17.tsv", V17);
    let key_path = lab.0.join("keys-not-opened.tsv");
    let result = command(&v17, &client, &output)
        .args(["--keys"])
        .arg(&key_path)
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(2));
    assert!(stderr(result).contains("product_keys_forbidden"));
    assert!(!key_path.exists());
    assert!(!output.exists());

    let v20 = lab.plan("v20.tsv", V20);
    let synthetic_client = lab.0.join("synthetic-client");
    fs::create_dir_all(&synthetic_client).unwrap();
    let client_output = synthetic_client.join("census.json");
    let result = command(&v20, &synthetic_client, &client_output)
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(9));
    assert!(stderr(result).contains("product_diagnostic_path"));
    assert!(!client_output.exists());

    let checkout = lab.0.join("checkout");
    fs::create_dir_all(checkout.join("reports")).unwrap();
    fs::write(checkout.join(".git"), b"synthetic marker").unwrap();
    let checkout_output = checkout.join("reports/census.json");
    let result = command(&v20, &synthetic_client, &checkout_output)
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(9));
    assert!(stderr(result).contains("product_diagnostic_path"));
    assert!(!checkout_output.exists());
}
