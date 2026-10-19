// SPDX-License-Identifier: GPL-3.0-or-later
//! Probe CLI refusal checks use authored plans and empty temporary directories.
#![cfg(target_os = "linux")]

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

const V20: &str = include_str!("../../../tests/contracts/fixtures/forever-product-v20/map2.tsv");
const V15: &str =
    include_str!("../../../tests/contracts/fixtures/forever-product-v15/all-available-maps.tsv");
const V16: &str =
    include_str!("../../../tests/contracts/fixtures/forever-product-v16/disabled.tsv");
const V17: &str =
    include_str!("../../../tests/contracts/fixtures/forever-product-v17/provided.tsv");
const V18: &str =
    include_str!("../../../tests/contracts/fixtures/forever-product-v18/all-available-maps.tsv");
const V19: &str =
    include_str!("../../../tests/contracts/fixtures/forever-product-v19/disabled.tsv");
const V13: &str =
    include_str!("../../../tests/contracts/fixtures/forever-product-v13/available.tsv");
const V14: &str =
    include_str!("../../../tests/contracts/fixtures/forever-product-v14/all-available-maps.tsv");
static NEXT: AtomicU64 = AtomicU64::new(0);

struct Lab(PathBuf);
impl Lab {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "rce-probe-encoding-cli-{}-{}",
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

    fn client(&self, file_data_id: u32, content: &[u8]) -> PathBuf {
        const ROOT_EKEY: [u8; 16] = [0x33; 16];
        const ENCODING_EKEY: [u8; 16] = [0x44; 16];
        const CONTENT_EKEY: [u8; 16] = [0x55; 16];
        let client = self.0.join("synthetic-client");
        let data = client.join("Data/data");
        fs::create_dir_all(&data).unwrap();
        let content_key = md5::digest(content);
        let mut root = b"TSFM".to_vec();
        for value in [20_u32, 2, 1, 0] {
            root.extend_from_slice(&value.to_le_bytes());
        }
        for value in [1_u32, 0, 0x1000_0000, 0] {
            root.extend_from_slice(&value.to_le_bytes());
        }
        root.push(0);
        root.extend_from_slice(&file_data_id.to_le_bytes());
        root.extend_from_slice(&content_key);
        let root_key = md5::digest(&root);
        let routes = [
            (root_key, root.len() as u32, ROOT_EKEY),
            (content_key, content.len() as u32, CONTENT_EKEY),
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
        fs::write(config_path.join(&config_hex), &config).unwrap();
        fs::write(
            client.join(".build.info"),
            format!(
                "Product!STRING:0|Active!DEC:1|Version!STRING:0|Build Key!HEX:16|CDN Key!HEX:16|Tags!STRING:0\nwow_classic_beta|1|1.60.1.70450|{}|{}|enUS\n",
                hex16(config_key),
                hex16([0x66; 16])
            ),
        )
        .unwrap();

        let records = [
            (ROOT_EKEY, root),
            (ENCODING_EKEY, encoding),
            (CONTENT_EKEY, content.to_vec()),
        ];
        let mut archive = Vec::new();
        let mut index_rows = Vec::new();
        for (key, payload) in records {
            let offset = archive.len() as u64;
            archive.extend_from_slice(&[0xa5; 30]);
            archive.extend_from_slice(b"BLTE\0\0\0\0N");
            archive.extend_from_slice(&payload);
            index_rows.push(index_entry(
                key,
                offset,
                u32::try_from(39 + payload.len()).unwrap(),
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

fn command(plan: &Path, client: &Path, output: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_forever-extractor"));
    command
        .args(["probe-encoding", "--plan"])
        .arg(plan)
        .args(["--client"])
        .arg(client)
        .args(["--output"])
        .arg(output);
    command
}
fn error(output: Output) -> String {
    String::from_utf8(output.stderr).unwrap()
}

fn hex16(bytes: [u8; 16]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn encoding_file(rows: &[([u8; 16], u32, [u8; 16])]) -> Vec<u8> {
    let mut page = Vec::new();
    for (content_key, content_size, ekey) in rows {
        page.extend_from_slice(&1_u16.to_le_bytes());
        page.extend_from_slice(&content_size.to_be_bytes());
        page.extend_from_slice(content_key);
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

#[test]
fn probe_success_writes_only_the_redacted_trace_and_stdout_envelope() {
    const FILE_DATA_ID: u32 = 42;
    let lab = Lab::new();
    let plan = lab.plan("v20.tsv", V20);
    let payload = b"synthetic CLI probe payload";
    let content_key = md5::digest(payload);
    let client = lab.client(FILE_DATA_ID, payload);
    let trace_path = lab.0.join("trace.json");
    let result = command(&plan, &client, &trace_path)
        .args(["--file-data-id", "42"])
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(result.stderr.is_empty());
    let trace = fs::read(&trace_path).unwrap();
    let trace_sha256 = forever_casc::integrity::hex(&forever_casc::integrity::digest(&trace));
    let stdout = String::from_utf8(result.stdout).unwrap();
    assert_eq!(
        stdout,
        format!(
            "{{\"contract\":\"forever-product-v20\",\"status\":\"probe_complete\",\"file_data_ids\":1,\"trace_sha256\":\"{trace_sha256}\",\"offline\":true,\"full_extractor_complete\":false}}\n"
        )
    );
    let trace = String::from_utf8(trace).unwrap();
    assert!(trace.contains("\"root_rows_considered\":1"));
    assert!(trace.contains("\"encoding_row_count\":1"));
    assert!(trace.contains("\"read_result\":\"decoded\""));
    assert!(!trace.contains(&hex16(content_key)));
    assert!(!trace.contains("synthetic CLI probe payload"));
}

#[test]
fn probe_help_documents_the_command_and_revision_20() {
    let output = Command::new(env!("CARGO_BIN_EXE_forever-extractor"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains(
        "forever-extractor probe-encoding --plan PLAN --client CLIENT --output NEW_TRACE --file-data-id FDID"
    ));
    assert!(stdout.contains("build 70450 revisions 15-20"));
    assert!(stdout.contains("Linux v20 selects build-70450"));
}

#[test]
fn probe_is_keyless_and_reachable_for_each_revision_15_through_20() {
    const PLANS: [(&str, &str); 6] = [
        ("v15", V15),
        ("v16", V16),
        ("v17", V17),
        ("v18", V18),
        ("v19", V19),
        ("v20", V20),
    ];
    let lab = Lab::new();
    let client = lab.client(42, b"synthetic all-revision probe content");
    for (name, plan_text) in PLANS {
        let plan = lab.plan(&format!("{name}.tsv"), plan_text);
        let trace = lab.0.join(format!("{name}-trace.json"));
        let result = command(&plan, &client, &trace)
            .args(["--file-data-id", "42"])
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{name} without keys: {}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(result.stderr.is_empty(), "{name}");
        let trace_text = fs::read_to_string(&trace).unwrap();
        assert!(trace_text.contains("\"read_result\":\"decoded\""), "{name}");
        assert!(trace_text.contains("\"selected\":true"), "{name}");
        let expected_mask = if plan_text.contains("locale\tesES") {
            0x80
        } else {
            0x2
        };
        assert!(
            trace_text.contains(&format!("\"locale_mask\":{expected_mask}")),
            "{name}"
        );

        let keyed_trace = lab.0.join(format!("{name}-keyed-trace.json"));
        let key_path = lab.0.join(format!("{name}-keys-not-opened.tsv"));
        let result = command(&plan, &client, &keyed_trace)
            .args(["--file-data-id", "42", "--keys"])
            .arg(&key_path)
            .output()
            .unwrap();
        assert_eq!(result.status.code(), Some(2), "{name}");
        assert!(error(result).contains("product_keys_forbidden"), "{name}");
        assert!(!key_path.exists(), "{name}");
        assert!(!keyed_trace.exists(), "{name}");
    }
}

#[test]
fn keyless_probe_exemption_keeps_key_requirements_for_v17_v18_client_commands() {
    for (name, plan_text, needs_scratch) in [("v17", V17, false), ("v18", V18, true)] {
        let lab = Lab::new();
        let plan = lab.plan(&format!("{name}.tsv"), plan_text);
        let client = lab.0.join("client-not-opened");
        let output = lab.0.join("output-not-created");

        let result = Command::new(env!("CARGO_BIN_EXE_forever-extractor"))
            .args(["inspect", "--plan"])
            .arg(&plan)
            .args(["--client"])
            .arg(&client)
            .output()
            .unwrap();
        assert_eq!(result.status.code(), Some(2), "{name} inspect");
        assert!(
            error(result).contains("product_keys_required"),
            "{name} inspect"
        );

        for verb in ["extract", "resume"] {
            let mut command = Command::new(env!("CARGO_BIN_EXE_forever-extractor"));
            command
                .args([verb, "--plan"])
                .arg(&plan)
                .args(["--client"])
                .arg(&client)
                .args(["--output"])
                .arg(&output);
            if needs_scratch {
                command
                    .args(["--scratch"])
                    .arg(lab.0.join("scratch-not-opened"));
            }
            let result = command.output().unwrap();
            assert_eq!(result.status.code(), Some(2), "{name} {verb}");
            assert!(
                error(result).contains("product_keys_required"),
                "{name} {verb}"
            );
        }
    }
}

#[test]
fn probe_requires_at_least_one_canonical_repeatable_file_data_id() {
    let lab = Lab::new();
    let plan = lab.plan("v20.tsv", V20);
    let client = lab.0.join("client-not-opened");
    let trace = lab.0.join("trace.json");
    for ids in [
        vec![],
        vec!["not-a-u32"],
        vec!["1", "1"],
        vec!["4294967296"],
    ] {
        let mut cmd = command(&plan, &client, &trace);
        for id in ids {
            cmd.args(["--file-data-id", id]);
        }
        let result = cmd.output().unwrap();
        assert_eq!(result.status.code(), Some(2));
        assert!(error(result).contains("product_probe_file_data_id"));
        assert!(!trace.exists());
    }
    let mut too_many = command(&plan, &client, &trace);
    for id in 1..=65 {
        too_many.args(["--file-data-id", &id.to_string()]);
    }
    let result = too_many.output().unwrap();
    assert_eq!(result.status.code(), Some(2));
    assert!(error(result).contains("product_probe_file_data_id"));
    assert!(!trace.exists());
}

#[test]
fn probe_refuses_non_70450_plans_and_any_key_argument_without_opening_paths() {
    let lab = Lab::new();
    let v13 = lab.plan("v13.tsv", V13);
    let client = lab.0.join("client-not-opened");
    let trace = lab.0.join("trace.json");
    let result = command(&v13, &client, &trace)
        .args(["--file-data-id", "1349477"])
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(2));
    assert!(error(result).contains("product_probe_profile"));
    assert!(!trace.exists());

    let v14 = lab.plan("v14.tsv", V14);
    let result = command(&v14, &client, &trace)
        .args(["--file-data-id", "1349477"])
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(2));
    assert!(error(result).contains("product_probe_profile"));
    assert!(!trace.exists());

    let wrong_build = lab.plan(
        "v20-wrong-build.tsv",
        &V20.replace("build\t70450", "build\t70425"),
    );
    let result = command(&wrong_build, &client, &trace)
        .args(["--file-data-id", "1349477"])
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(2));
    assert!(!error(result).is_empty());
    assert!(!trace.exists());

    let v20 = lab.plan("v20.tsv", V20);
    let key_path = lab.0.join("key-list-not-opened");
    let result = command(&v20, &client, &trace)
        .args(["--file-data-id", "1349477", "--keys"])
        .arg(&key_path)
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(2));
    assert!(error(result).contains("product_keys_forbidden"));
    assert!(!key_path.exists());
    assert!(!trace.exists());
}

#[test]
fn probe_refuses_trace_paths_inside_client_or_any_git_checkout() {
    let lab = Lab::new();
    let plan = lab.plan("v20.tsv", V20);
    let client = lab.0.join("synthetic-client");
    fs::create_dir(&client).unwrap();
    let client_trace = client.join("trace.json");
    let result = command(&plan, &client, &client_trace)
        .args(["--file-data-id", "1349477"])
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(9));
    assert!(error(result).contains("product_diagnostic_path"));
    assert!(!client_trace.exists());

    let checkout = lab.0.join("checkout");
    fs::create_dir_all(checkout.join("reports")).unwrap();
    fs::write(checkout.join(".git"), b"synthetic git directory marker").unwrap();
    let checkout_trace = checkout.join("reports/trace.json");
    let result = command(&plan, &client, &checkout_trace)
        .args(["--file-data-id", "1349477"])
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(9));
    assert!(error(result).contains("product_diagnostic_path"));
    assert!(!checkout_trace.exists());
}
