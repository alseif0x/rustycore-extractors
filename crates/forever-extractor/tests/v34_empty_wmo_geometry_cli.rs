// SPDX-License-Identifier: GPL-3.0-or-later
//! Synthetic v34 RequestHeavyV1 geometry with vacated and intrinsically empty WMO omissions.
#![cfg(target_os = "linux")]

#[path = "v34_empty_wmo_geometry_cli/coverage.rs"]
mod coverage;
#[path = "v34_empty_wmo_geometry_cli/fixture.rs"]
mod fixture;
#[path = "v33_vacated_wmo_geometry_cli/fixture.rs"]
#[allow(dead_code)]
#[allow(clippy::duplicate_mod)]
mod v33_fixture;

use coverage::Coverage;
use forever_casc::{
    collision::{plan::MapSelection, run},
    integrity,
    package::{Artifact, Component, Context, Limits, PartialEncryptedSection, Run},
};
use forever_extractor::plan::{EMPTY_WMO_VERIFY_PROFILE, EMPTY_WMO_VERIFY_SCOPE};
use forever_vmap::{assembly, placement, vmo};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};

const PLAN: &str = include_str!("../../../tests/contracts/fixtures/forever-product-v34/all.tsv");
const V33_PLAN: &str =
    include_str!("../../../tests/contracts/fixtures/forever-product-v33/all.tsv");
const MAP_PATH: &str = "dbc/esES/Map.db2";
static NEXT: AtomicU64 = AtomicU64::new(0);

struct Lab(PathBuf);

impl Lab {
    fn new() -> Self {
        let root = std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join(format!(
                ".local/share/rce-v34-empty-wmo-geometry-cli-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
        fs::create_dir_all(root.parent().unwrap()).unwrap();
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        fs::write(root.join("plan.tsv"), PLAN).unwrap();
        Self(root)
    }

    fn plan(&self) -> PathBuf {
        self.0.join("plan.tsv")
    }

    fn v33_plan(&self) -> PathBuf {
        let plan = self.0.join("plan-v33.tsv");
        fs::write(&plan, V33_PLAN).unwrap();
        plan
    }

    fn extract_v33(&self, stem: &str) -> PathBuf {
        let client = self.private_dir(&format!("{stem}-client"));
        v33_fixture::write_client(&client);
        let package = self.0.join(format!("{stem}-package"));
        let scratch = self.private_dir(&format!("{stem}-scratch"));
        assert_success(
            self.produce_with_plan("extract", &self.v33_plan(), &client, &package, &scratch),
            "v33 extract under v34 CLI harness",
        );
        package
    }

    fn private_dir(&self, name: &str) -> PathBuf {
        let path = self.0.join(name);
        fs::create_dir(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        path
    }

    fn client(&self, name: &str) -> (PathBuf, fixture::WrittenClient) {
        self.client_with(name, fixture::ClientMutation::default())
    }

    fn client_with(
        &self,
        name: &str,
        mutation: fixture::ClientMutation,
    ) -> (PathBuf, fixture::WrittenClient) {
        let path = self.private_dir(name);
        let written = fixture::write_client_with(&path, mutation);
        (path, written)
    }

    fn client_with_logical_name(&self, name: &str) -> (PathBuf, fixture::WrittenClient) {
        let path = self.private_dir(name);
        let written = fixture::write_client_with_logical_name(
            &path,
            fixture::ClientMutation::default(),
            true,
        );
        (path, written)
    }

    fn command(&self, verb: &str) -> Command {
        self.command_with_plan(verb, &self.plan())
    }

    fn command_with_plan(&self, verb: &str, plan: &Path) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_forever-extractor"));
        command
            .current_dir(&self.0)
            .env_remove("FOREVER_PRODUCT_DIAGNOSTIC")
            .args([verb, "--plan"])
            .arg(plan);
        command
    }

    fn produce(&self, verb: &str, client: &Path, output: &Path, scratch: &Path) -> Output {
        self.produce_with_plan(verb, &self.plan(), client, output, scratch)
    }

    fn produce_with_plan(
        &self,
        verb: &str,
        plan: &Path,
        client: &Path,
        output: &Path,
        scratch: &Path,
    ) -> Output {
        self.command_with_plan(verb, plan)
            .args(["--client"])
            .arg(client)
            .args(["--output"])
            .arg(output)
            .args(["--scratch"])
            .arg(scratch)
            .output()
            .unwrap()
    }

    fn extract_with_plan(&self, stem: &str, plan: &Path, mode: fixture::AdtMode) -> PathBuf {
        let (client, _) = self.client_with(
            &format!("{stem}-client"),
            fixture::ClientMutation {
                populated_adt_mode: mode,
                ..Default::default()
            },
        );
        let output = self.0.join(format!("{stem}-package"));
        let scratch = self.private_dir(&format!("{stem}-scratch"));
        assert_success(
            self.produce_with_plan("extract", plan, &client, &output, &scratch),
            "extract with selected product plan",
        );
        output
    }

    fn verify_with_plan_failure(&self, package: &Path, plan: &Path, codes: &[&str]) {
        for verb in ["verify", "verify-links"] {
            assert_failure_one_of(
                self.command_with_plan(verb, plan)
                    .args(["--output"])
                    .arg(package)
                    .output()
                    .unwrap(),
                codes,
            );
        }
    }

    fn extract(&self, stem: &str) -> PathBuf {
        let (client, _) = self.client(&format!("{stem}-client"));
        let output = self.0.join(format!("{stem}-package"));
        let scratch = self.private_dir(&format!("{stem}-scratch"));
        assert_success(
            self.produce("extract", &client, &output, &scratch),
            "v34 extract",
        );
        output
    }

    fn resume(&self, stem: &str, output: &Path) {
        let (client, _) = self.client(&format!("{stem}-client"));
        let scratch = self.private_dir(&format!("{stem}-scratch"));
        assert_success(
            self.produce("resume", &client, output, &scratch),
            "resume after v34 DB2 and VMap seals",
        );
    }
}

impl Drop for Lab {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn assert_success(output: Output, label: &str) -> String {
    assert!(
        output.status.success(),
        "{label} exited {:?}: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_cli_json(&output);
    String::from_utf8(output.stdout).unwrap()
}

fn assert_failure(output: Output, code: &str) {
    assert!(!output.status.success());
    assert_cli_json(&output);
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains(&format!("\"code\":\"{code}\"")), "{stderr}");
}

fn assert_failure_one_of(output: Output, codes: &[&str]) {
    assert!(!output.status.success());
    assert_cli_json(&output);
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        codes
            .iter()
            .any(|code| stderr.contains(&format!("\"code\":\"{code}\""))),
        "expected one of {codes:?}: {stderr}"
    );
}

#[derive(Debug)]
enum JsonValue {
    Object(BTreeMap<String, JsonValue>),
    Array(Vec<JsonValue>),
    Scalar,
}

struct JsonParser<'a> {
    text: &'a str,
    at: usize,
}

impl JsonParser<'_> {
    fn whitespace(&mut self) {
        while self
            .byte()
            .is_some_and(|byte| matches!(byte, b' ' | b'\n' | b'\r' | b'\t'))
        {
            self.at += 1;
        }
    }

    fn byte(&self) -> Option<u8> {
        self.text.as_bytes().get(self.at).copied()
    }

    fn take(&mut self, expected: u8) -> Result<(), &'static str> {
        if self.byte() == Some(expected) {
            self.at += 1;
            Ok(())
        } else {
            Err("invalid JSON token")
        }
    }

    fn string(&mut self) -> Result<String, &'static str> {
        self.take(b'"')?;
        let mut value = String::new();
        loop {
            let byte = self.byte().ok_or("unterminated JSON string")?;
            match byte {
                b'"' => {
                    self.at += 1;
                    return Ok(value);
                }
                b'\\' => {
                    self.at += 1;
                    match self.byte().ok_or("unterminated JSON escape")? {
                        b'"' => value.push('"'),
                        b'\\' => value.push('\\'),
                        b'/' => value.push('/'),
                        b'b' => value.push('\u{0008}'),
                        b'f' => value.push('\u{000c}'),
                        b'n' => value.push('\n'),
                        b'r' => value.push('\r'),
                        b't' => value.push('\t'),
                        b'u' => {
                            self.at += 1;
                            let first = self.hex_quad()?;
                            let scalar = if (0xd800..=0xdbff).contains(&first) {
                                if self.text.as_bytes().get(self.at..self.at + 2) != Some(b"\\u") {
                                    return Err("unpaired JSON surrogate");
                                }
                                self.at += 2;
                                let second = self.hex_quad()?;
                                if !(0xdc00..=0xdfff).contains(&second) {
                                    return Err("unpaired JSON surrogate");
                                }
                                0x10000
                                    + ((u32::from(first) - 0xd800) << 10)
                                    + (u32::from(second) - 0xdc00)
                            } else if (0xdc00..=0xdfff).contains(&first) {
                                return Err("unpaired JSON surrogate");
                            } else {
                                u32::from(first)
                            };
                            value.push(char::from_u32(scalar).ok_or("invalid JSON scalar")?);
                            continue;
                        }
                        _ => return Err("invalid JSON escape"),
                    }
                    self.at += 1;
                }
                0..=0x1f => return Err("control byte in JSON string"),
                _ => {
                    let character = self.text[self.at..]
                        .chars()
                        .next()
                        .ok_or("invalid UTF-8 JSON string")?;
                    self.at += character.len_utf8();
                    value.push(character);
                }
            }
        }
    }

    fn hex_quad(&mut self) -> Result<u16, &'static str> {
        let digits = self
            .text
            .as_bytes()
            .get(self.at..self.at + 4)
            .ok_or("short JSON unicode escape")?;
        let mut value = 0_u16;
        for digit in digits {
            value = value.checked_mul(16).ok_or("invalid JSON unicode escape")?
                + match digit {
                    b'0'..=b'9' => u16::from(digit - b'0'),
                    b'a'..=b'f' => u16::from(digit - b'a') + 10,
                    b'A'..=b'F' => u16::from(digit - b'A') + 10,
                    _ => return Err("invalid JSON unicode escape"),
                };
        }
        self.at += 4;
        Ok(value)
    }

    fn value(&mut self) -> Result<JsonValue, &'static str> {
        self.whitespace();
        match self.byte().ok_or("missing JSON value")? {
            b'{' => self.object(),
            b'[' => self.array(),
            b'"' => {
                self.string()?;
                Ok(JsonValue::Scalar)
            }
            b't' => self.literal(b"true"),
            b'f' => self.literal(b"false"),
            b'n' => self.literal(b"null"),
            b'-' | b'0'..=b'9' => {
                self.number()?;
                Ok(JsonValue::Scalar)
            }
            _ => Err("invalid JSON value"),
        }
    }

    fn object(&mut self) -> Result<JsonValue, &'static str> {
        self.take(b'{')?;
        self.whitespace();
        let mut fields = BTreeMap::new();
        if self.byte() == Some(b'}') {
            self.at += 1;
            return Ok(JsonValue::Object(fields));
        }
        loop {
            self.whitespace();
            let key = self.string()?;
            if fields.contains_key(&key) {
                return Err("duplicate JSON object key");
            }
            self.whitespace();
            self.take(b':')?;
            let value = self.value()?;
            fields.insert(key, value);
            self.whitespace();
            match self.byte() {
                Some(b',') => self.at += 1,
                Some(b'}') => {
                    self.at += 1;
                    return Ok(JsonValue::Object(fields));
                }
                _ => return Err("unterminated JSON object"),
            }
        }
    }

    fn array(&mut self) -> Result<JsonValue, &'static str> {
        self.take(b'[')?;
        self.whitespace();
        let mut values = Vec::new();
        if self.byte() == Some(b']') {
            self.at += 1;
            return Ok(JsonValue::Array(values));
        }
        loop {
            values.push(self.value()?);
            self.whitespace();
            match self.byte() {
                Some(b',') => self.at += 1,
                Some(b']') => {
                    self.at += 1;
                    return Ok(JsonValue::Array(values));
                }
                _ => return Err("unterminated JSON array"),
            }
        }
    }

    fn literal(&mut self, literal: &[u8]) -> Result<JsonValue, &'static str> {
        if self.text.as_bytes().get(self.at..self.at + literal.len()) == Some(literal) {
            self.at += literal.len();
            Ok(JsonValue::Scalar)
        } else {
            Err("invalid JSON literal")
        }
    }

    fn number(&mut self) -> Result<(), &'static str> {
        let bytes = self.text.as_bytes();
        if self.byte() == Some(b'-') {
            self.at += 1;
        }
        match self.byte() {
            Some(b'0') => {
                self.at += 1;
                if self.byte().is_some_and(|byte| byte.is_ascii_digit()) {
                    return Err("leading zero in JSON number");
                }
            }
            Some(b'1'..=b'9') => {
                self.at += 1;
                while self.byte().is_some_and(|byte| byte.is_ascii_digit()) {
                    self.at += 1;
                }
            }
            _ => return Err("invalid JSON number"),
        }
        if self.byte() == Some(b'.') {
            self.at += 1;
            let start = self.at;
            while self.byte().is_some_and(|byte| byte.is_ascii_digit()) {
                self.at += 1;
            }
            if self.at == start {
                return Err("invalid JSON fraction");
            }
        }
        if self.byte().is_some_and(|byte| matches!(byte, b'e' | b'E')) {
            self.at += 1;
            if self.byte().is_some_and(|byte| matches!(byte, b'+' | b'-')) {
                self.at += 1;
            }
            let start = self.at;
            while self.byte().is_some_and(|byte| byte.is_ascii_digit()) {
                self.at += 1;
            }
            if self.at == start {
                return Err("invalid JSON exponent");
            }
        }
        let _ = bytes;
        Ok(())
    }
}

fn assert_json_without_duplicate_keys(text: &str) -> JsonValue {
    let mut parser = JsonParser { text, at: 0 };
    let value = parser
        .value()
        .unwrap_or_else(|error| panic!("{error}: {text}"));
    parser.whitespace();
    assert_eq!(parser.at, text.len(), "trailing JSON data: {text}");
    value
}

#[test]
fn json_parser_rejects_duplicate_keys_after_unescaping_and_inside_nested_objects() {
    for invalid in [
        r#"{"field":1,"field":2}"#,
        r#"{"field":1,"fi\u0065ld":2}"#,
        r#"{"outer":[{"field":1,"field":2}]}"#,
    ] {
        let mut parser = JsonParser {
            text: invalid,
            at: 0,
        };
        assert!(
            matches!(parser.value(), Err("duplicate JSON object key")),
            "{invalid}"
        );
    }
}

fn count_key(value: &JsonValue, wanted: &str) -> usize {
    match value {
        JsonValue::Object(fields) => {
            usize::from(fields.contains_key(wanted))
                + fields
                    .values()
                    .map(|value| count_key(value, wanted))
                    .sum::<usize>()
        }
        JsonValue::Array(values) => values.iter().map(|value| count_key(value, wanted)).sum(),
        JsonValue::Scalar => 0,
    }
}

fn assert_key_once(text: &str, key: &str) {
    let value = assert_json_without_duplicate_keys(text);
    assert_eq!(count_key(&value, key), 1, "{key} count in {text}");
}

fn assert_cli_json(output: &Output) {
    let bytes = if output.status.success() {
        &output.stdout
    } else {
        &output.stderr
    };
    let text = String::from_utf8_lossy(bytes);
    let value = assert_json_without_duplicate_keys(&text);
    assert_eq!(count_key(&value, "contract"), 1, "{text}");
    if !output.status.success() {
        assert_eq!(count_key(&value, "code"), 1, "{text}");
    }
}

fn client_context(package: &Path) -> Context {
    Context::parse(&fs::read_to_string(package.join(".state/context.json")).unwrap()).unwrap()
}

fn package_artifacts(package: &Path, context: Context) -> Vec<Artifact> {
    fs::read_dir(package.join(".state/files"))
        .unwrap()
        .map(|entry| {
            let text = fs::read_to_string(entry.unwrap().path()).unwrap();
            Artifact::parse(&text, context, Limits::default()).unwrap()
        })
        .collect()
}

fn artifact_inventory(package: &Path) -> Vec<String> {
    let context = client_context(package);
    let mut artifacts: Vec<_> = package_artifacts(package, context)
        .into_iter()
        .map(|artifact| artifact.json())
        .collect();
    artifacts.sort_unstable();
    artifacts
}

fn manifest(package: &Path) -> String {
    fs::read_to_string(package.join("forever-package.json")).unwrap()
}

fn declaration() -> PartialEncryptedSection {
    fixture::partial_map().declaration
}

fn assert_canonical_partial_manifest(text: &str) {
    let declaration = declaration();
    let table = format!(
        "\"partial_encrypted_tables\":[{{\"path\":\"{MAP_PATH}\",\"sections\":[{{\"lookup\":\"{}\",\"frame_ordinal\":1,\"decoded_size\":{}}}]}}]",
        declaration.lookup, declaration.decoded_size
    );
    assert!(text.contains(&table), "{text}");
    assert!(!text.contains("\"partial_encrypted_sections\":"));
    assert_eq!(
        text.matches("\"path\":\"dbc/esES/Map.db2\",\"sections\":")
            .count(),
        1
    );
    assert_eq!(
        text.matches(&format!("\"lookup\":\"{}\"", declaration.lookup))
            .count(),
        1
    );
}

fn verify_package_with_plan(lab: &Lab, package: &Path, plan: &Path) {
    for verb in ["verify", "verify-links"] {
        let report = assert_success(
            lab.command_with_plan(verb, plan)
                .args(["--output"])
                .arg(package)
                .output()
                .unwrap(),
            verb,
        );
        for (field, value) in [
            ("verification_profile", EMPTY_WMO_VERIFY_PROFILE),
            ("scope", EMPTY_WMO_VERIFY_SCOPE),
            ("coverage_authentication", "independently_derived"),
            ("root_absence_authentication", "producer_reported"),
            ("model_absence_authentication", "producer_reported"),
            ("wmo_absence_authentication", "producer_reported"),
            ("source_authentication_flags", "producer_reported"),
            ("partial_db2_declarations", "per-table-including-Map"),
        ] {
            let identity = format!("\"{field}\":\"{value}\"");
            assert!(
                report.contains(&identity),
                "{verb} omitted {identity}: {report}"
            );
        }
        assert_key_once(&report, "coverage_authentication");
        for expected in [
            "\"components\":17",
            "\"resource_profile\":\"linux-collision-request-heavy-resources-v1\"",
            "\"complete_global_map_coverage_claim\":false",
            "\"complete_vmap_component\":false",
            "\"full_extractor_complete\":false",
        ] {
            assert!(report.contains(expected), "{verb}: {report}");
        }
        assert!(!report.contains("requested_map"), "{verb}: {report}");
        assert!(!report.contains("\"full_world_coverage\":true"));
        if verb == "verify" {
            assert!(report.contains("\"client_reads\":false"), "{report}");
        }
    }
}

fn verify_package(lab: &Lab, package: &Path) {
    verify_package_with_plan(lab, package, &lab.plan());
}

fn verify_with_plan_package(lab: &Lab, package: &Path, plan: &Path, other_plan: &Path) {
    verify_package_with_plan(lab, package, plan);
    lab.verify_with_plan_failure(
        package,
        other_plan,
        &["product_plan_mismatch", "package_inventory"],
    );
}

#[test]
fn v33_package_and_v34_package_reject_the_other_product_plan() {
    let lab = Lab::new();
    let v33_package = lab.extract_v33("cross-v33");
    let v34_package =
        lab.extract_with_plan("cross-v34", &lab.plan(), fixture::AdtMode::VacatedWmoOnly);
    lab.verify_with_plan_failure(
        &v33_package,
        &lab.plan(),
        &["product_plan_mismatch", "package_inventory"],
    );
    lab.verify_with_plan_failure(
        &v34_package,
        &lab.v33_plan(),
        &["product_plan_mismatch", "package_inventory"],
    );
}

fn compare_with_baseline(lab: &Lab, baseline: &Path, resumed: &Path) {
    assert_eq!(
        fs::read(resumed.join(MAP_PATH)).unwrap(),
        fs::read(baseline.join(MAP_PATH)).unwrap()
    );
    assert_eq!(client_context(resumed), client_context(baseline));
    assert_eq!(
        fs::read(resumed.join(coverage::PATH)).unwrap(),
        fs::read(baseline.join(coverage::PATH)).unwrap()
    );
    assert_eq!(manifest(resumed), manifest(baseline));
    assert_eq!(artifact_inventory(resumed), artifact_inventory(baseline));
    assert_canonical_partial_manifest(&manifest(resumed));
    assert!(!resumed.join(".forever-package-incomplete").exists());
    assert!(resumed.join(".state/sealed/db2").is_file());
    assert!(resumed.join(".state/sealed/vmaps").is_file());
    verify_package(lab, resumed);
}

fn present_artifacts(
    package: &Path,
    id: u32,
    expected_records: usize,
) -> (BTreeSet<String>, BTreeSet<String>) {
    let missing_name = fixture::model_name(fixture::MISSING_M2);
    let proof_path = format!("vmaps/coverage/{id:04}.placements");
    let tree_path = format!("vmaps/{id:04}/{id:04}.vmtree");
    let proof = fs::read(package.join(&proof_path)).unwrap();
    let tree = fs::read(package.join(&tree_path)).unwrap();
    let placements = placement::DirBin::parse(&proof, Default::default()).unwrap();
    assert_eq!(placements.record_count(), expected_records);
    let records: Vec<_> = placements.records().collect();
    let mut model_names = BTreeSet::new();
    for record in &records {
        assert_ne!(record.id(), 0);
        let name = std::str::from_utf8(record.name()).unwrap().to_owned();
        assert_ne!(name, missing_name);
        model_names.insert(name);
    }
    let assembled = assembly::build(id, &records, Default::default()).unwrap();
    assert_eq!(proof, assembled.ordered_dir_bin());
    assert_eq!(tree, assembled.tree_bytes());
    let mut paths = BTreeSet::from([proof_path, tree_path]);
    if expected_records == 0 {
        assert!(assembled.tiles().is_empty());
    } else {
        assert!(!assembled.tiles().is_empty());
        assert!(
            assembled
                .tiles()
                .iter()
                .all(|tile| tile.payload().is_some())
        );
    }
    for tile in assembled.tiles() {
        paths.insert(format!(
            "vmaps/{id:04}/{id:04}_{:02}_{:02}.vmtileidx",
            tile.x(),
            tile.y()
        ));
        if tile.payload().is_some() {
            paths.insert(format!(
                "vmaps/{id:04}/{id:04}_{:02}_{:02}.vmtile",
                tile.x(),
                tile.y()
            ));
        }
    }
    (paths, model_names)
}

fn assert_coverage_and_inventory(package: &Path) {
    fixture::assert_sealed_map(&fs::read(package.join(MAP_PATH)).unwrap());
    let map = fixture::partial_map();
    assert_canonical_partial_manifest(&manifest(package));
    let missing_name = fixture::model_name(fixture::MISSING_M2);
    let missing_wmo_name = fixture::model_name(fixture::MISSING_WMO);
    let empty_wmo_name = fixture::model_name(fixture::EMPTY_WMO);
    assert!(!package.join(format!("vmaps/{missing_name}.vmo")).exists());
    assert!(
        !package
            .join(format!("vmaps/{missing_wmo_name}.vmo"))
            .exists()
    );
    assert!(!package.join(format!("vmaps/{empty_wmo_name}.vmo")).exists());
    for excluded in [
        "maps",
        "mmaps",
        "vmaps/GameObjectModels.dtree",
        "vmaps/selected-geometry-coverage-v1.bin",
        "vmaps/available-geometry-coverage-v2.bin",
        "vmaps/available-root-absent-geometry-coverage-v3.bin",
        "vmaps/available-model-root-omissions-geometry-coverage-v4.bin",
        "vmaps/available-request-capacity-geometry-coverage-v5.bin",
        "vmaps/available-vacated-wmo-geometry-coverage-v6.bin",
        "dbc/esES/LiquidObject.db2",
        "dbc/esES/GameObjectDisplayInfo.db2",
    ] {
        assert!(!package.join(excluded).exists(), "unexpected {excluded}");
    }
    for id in [
        fixture::WDT_ZERO_CHILD,
        fixture::ROOT_ABSENT_PARENT,
        fixture::ROOT_ABSENT_OTHER,
        fixture::INACTIVE_MAP,
        fixture::COPIED_INACTIVE_MAP,
        fixture::HIDDEN_MAP,
    ] {
        assert!(!package.join(format!("vmaps/{id:04}")).exists());
        assert!(
            !package
                .join(format!("vmaps/coverage/{id:04}.placements"))
                .exists()
        );
    }

    let context = client_context(package);
    assert_eq!(context.build, 70450);
    assert_eq!(context.locale.name(), "esES");
    assert_eq!(context.components, 17);
    let artifacts = package_artifacts(package, context);
    assert!(artifacts.len() <= 65_536);
    assert!(
        artifacts
            .iter()
            .any(|artifact| { artifact.component == Component::Db2 && artifact.path == MAP_PATH })
    );
    assert!(artifacts.iter().all(|artifact| {
        (artifact.component == Component::Db2 && artifact.path == MAP_PATH)
            || (artifact.component == Component::Collision && artifact.path.starts_with("vmaps/"))
    }));

    let coverage_bytes = fs::read(package.join(coverage::PATH)).unwrap();
    let coverage = Coverage::parse(&coverage_bytes).unwrap();
    assert_eq!(coverage.encode(), coverage_bytes);
    assert_eq!(coverage.version, 7);
    assert_eq!(coverage.resource_profile, coverage::RESOURCE_PROFILE);
    assert_eq!(coverage.selection, MapSelection::All);
    assert_eq!(
        coverage.missing_wdt_policy,
        run::MissingWdtPolicy::DeclareRootAbsent
    );
    assert_eq!(
        coverage.missing_m2_policy,
        run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2
    );
    assert_eq!(coverage.missing_m2, [fixture::MISSING_M2]);
    assert_eq!(
        coverage.missing_wmo_policy,
        run::MissingWmoPolicy::DeclareVacatedAfterRootAbsentM2OrEmpty
    );
    assert_eq!(coverage.missing_wmo, [fixture::MISSING_WMO]);
    assert_eq!(coverage.empty_wmo, [fixture::EMPTY_WMO]);
    assert!(
        coverage
            .missing_wmo
            .iter()
            .all(|id| !coverage.empty_wmo.contains(id))
    );
    assert_eq!(coverage.coverage_profile, coverage::COVERAGE_PROFILE);
    assert_eq!(coverage.source_profile, coverage::SOURCE_PROFILE);
    assert_eq!(coverage.domain_profile, coverage::DOMAIN_PROFILE);
    assert_eq!(coverage.context, context);
    assert!(!coverage.complete_map_source);
    assert!(!coverage.complete_global_map_coverage);
    assert!(!coverage.gameobject_request_made);
    assert_eq!(coverage.gameobject_startup_reads, 0);
    assert!(!coverage.liquid_object_request_made);
    assert_eq!(
        coverage.map.file_data_id,
        forever_casc::collision::world::MAP_FILE_DATA_ID
    );
    assert_eq!(coverage.map.declared_bytes, map.bytes.len() as u64);
    assert_eq!(coverage.map.available_bytes, map.hidden_start as u64);
    assert!(!coverage.map.full_ckey_verified);
    assert!(coverage.map.encoded_key_authenticated);
    assert_eq!(coverage.map.declared_direct_rows, 8);
    assert_eq!(coverage.map.declared_copy_records, 1);
    assert_eq!(coverage.map.direct_rows, 7);
    assert_eq!(coverage.map.copy_records, 1);
    assert_eq!(coverage.map.materialized_rows, fixture::MATERIALIZED_ROWS);
    assert_eq!(coverage.map.sections.len(), 2);
    assert_eq!(coverage.map.sections[0].ordinal, 0);
    assert_eq!(coverage.map.sections[0].records, 7);
    assert_eq!(coverage.map.sections[0].copies, 1);
    assert!(coverage.map.sections[0].admitted);
    assert_eq!(coverage.map.sections[1].ordinal, 1);
    assert_eq!(coverage.map.sections[1].records, 1);
    assert_eq!(coverage.map.sections[1].copies, 0);
    assert!(!coverage.map.sections[1].admitted);

    assert_eq!(coverage.liquids.len(), 2);
    for liquid in &coverage.liquids {
        assert_eq!(liquid.direct_rows, 1);
        assert_eq!(liquid.copy_records, 0);
        assert_eq!(liquid.materialized_rows, 1);
    }
    assert_eq!(coverage.maps.len(), fixture::ACTIVE_MAPS);
    assert_eq!(
        coverage.maps.iter().map(|map| map.id).collect::<Vec<_>>(),
        [
            fixture::EMPTY_MAP,
            fixture::POPULATED_MAP,
            fixture::ROOT_ABSENT_PARENT,
            fixture::PRESENT_CHILD,
            fixture::ROOT_ABSENT_OTHER,
        ]
    );
    let present = coverage.maps.iter().filter(|map| !map.root_absent).count();
    let absent = coverage.maps.iter().filter(|map| map.root_absent).count();
    assert_eq!(present, fixture::PRESENT_WDT_FILES);
    assert_eq!(absent, fixture::ROOT_ABSENT_MAPS);
    assert_eq!(coverage.maps.len(), present + absent);

    let expected = [
        (fixture::EMPTY_MAP, None, fixture::EMPTY_WDT, 0, true, false),
        (
            fixture::POPULATED_MAP,
            Some(fixture::EMPTY_MAP),
            fixture::POPULATED_WDT,
            1,
            true,
            false,
        ),
        (
            fixture::ROOT_ABSENT_PARENT,
            Some(fixture::POPULATED_MAP),
            fixture::ROOT_ABSENT_PARENT_WDT,
            2,
            true,
            true,
        ),
        (
            fixture::PRESENT_CHILD,
            Some(fixture::ROOT_ABSENT_PARENT),
            fixture::CHILD_WDT,
            3,
            false,
            false,
        ),
        (
            fixture::ROOT_ABSENT_OTHER,
            None,
            fixture::ROOT_ABSENT_OTHER_WDT,
            0,
            false,
            true,
        ),
    ];
    for (fact, (id, parent, wdt, depth, is_parent, root_absent)) in
        coverage.maps.iter().zip(expected)
    {
        assert_eq!(fact.id, id);
        assert_eq!(fact.parent, parent);
        assert_eq!(fact.wdt_file_data_id, wdt);
        assert_eq!(fact.depth, depth);
        assert_eq!(fact.is_parent, is_parent);
        assert_eq!(fact.root_absent, root_absent);
    }

    let (empty_paths, empty_models) = present_artifacts(package, fixture::EMPTY_MAP, 0);
    assert!(empty_models.is_empty());
    assert!(!package.join("vmaps/0001/0001_00_00.vmtile").exists());
    assert!(!package.join("vmaps/0001/0001_00_00.vmtileidx").exists());
    let (populated_paths, populated_models) = present_artifacts(package, fixture::POPULATED_MAP, 2);
    let (child_paths, child_models) = present_artifacts(package, fixture::PRESENT_CHILD, 2);
    let expected_models = BTreeSet::from([
        fixture::model_name(fixture::WMO_ROOT),
        fixture::model_name(fixture::MODEL),
    ]);
    let missing_wmo_model = fixture::model_name(fixture::MISSING_WMO);
    let empty_wmo_model = fixture::model_name(fixture::EMPTY_WMO);
    let omitted_wmo_models = [&missing_wmo_model, &empty_wmo_model];
    let placement_records: Vec<_> = coverage
        .artifacts
        .iter()
        .filter(|artifact| {
            artifact.path.starts_with("vmaps/coverage/") && artifact.path.ends_with(".placements")
        })
        .flat_map(|artifact| {
            let bytes = fs::read(package.join(&artifact.path)).unwrap();
            placement::DirBin::parse(&bytes, Default::default())
                .unwrap()
                .records()
                .map(|record| (record.id(), record.name().to_vec()))
                .collect::<Vec<_>>()
        })
        .collect();
    for omitted in omitted_wmo_models {
        assert!(!populated_models.contains(omitted));
        assert!(!child_models.contains(omitted));
        assert!(
            !coverage
                .artifacts
                .iter()
                .any(|artifact| artifact.path == format!("vmaps/{omitted}.vmo"))
        );
        assert!(
            !placement_records
                .iter()
                .any(|(_, name)| name == omitted.as_bytes())
        );
    }
    assert_eq!(populated_models, expected_models);
    assert_eq!(child_models, expected_models);
    let model_names: BTreeSet<_> = populated_models.union(&child_models).collect();
    assert_eq!(model_names, expected_models.iter().collect::<BTreeSet<_>>());
    let mut expected_paths = BTreeSet::new();
    expected_paths.extend(empty_paths);
    expected_paths.extend(populated_paths);
    expected_paths.extend(child_paths);
    for name in model_names {
        expected_paths.insert(format!("vmaps/{name}.vmo"));
    }
    let actual_paths: BTreeSet<_> = coverage
        .artifacts
        .iter()
        .map(|artifact| artifact.path.clone())
        .collect();
    assert_eq!(actual_paths, expected_paths);
    assert_eq!(
        actual_paths
            .iter()
            .filter(|path| path.ends_with(".vmo"))
            .count(),
        2
    );
    assert_eq!(artifacts.len(), actual_paths.len() + 2);
    for fact in &coverage.artifacts {
        let bytes = fs::read(package.join(&fact.path)).unwrap();
        assert_eq!(fact.bytes, bytes.len() as u64);
        assert_eq!(fact.sha, integrity::digest(&bytes));
        if fact.path.ends_with(".vmo") {
            vmo::View::parse(&bytes, Default::default()).unwrap();
        }
    }
}

fn assert_v7_codec_cases(package: &Path) {
    let bytes = fs::read(package.join(coverage::PATH)).unwrap();
    let baseline = Coverage::parse(&bytes).unwrap();
    assert_eq!(baseline.encode(), bytes);

    for end in 0..bytes.len() {
        assert!(
            Coverage::parse(&bytes[..end]).is_err(),
            "accepted prefix {end}"
        );
    }
    for suffix in [&[0][..], &[0; 8][..]] {
        assert!(Coverage::parse(&[bytes.as_slice(), suffix].concat()).is_err());
    }

    for (field, invalid) in [
        ("missing_m2", vec![0]),
        ("missing_m2", vec![fixture::MISSING_M2, fixture::MISSING_M2]),
        ("missing_m2", vec![fixture::MISSING_M2, fixture::MODEL]),
        ("missing_wmo", vec![0]),
        (
            "missing_wmo",
            vec![fixture::MISSING_WMO, fixture::MISSING_WMO],
        ),
        ("missing_wmo", vec![fixture::MISSING_WMO, fixture::MODEL]),
        ("empty_wmo", vec![0]),
        ("empty_wmo", vec![fixture::EMPTY_WMO, fixture::EMPTY_WMO]),
        ("empty_wmo", vec![fixture::EMPTY_WMO, fixture::MODEL]),
    ] {
        let mut candidate = baseline.clone();
        match field {
            "missing_m2" => candidate.missing_m2 = invalid,
            "missing_wmo" => candidate.missing_wmo = invalid,
            "empty_wmo" => candidate.empty_wmo = invalid,
            _ => unreachable!(),
        }
        assert!(Coverage::parse(&candidate.encode()).is_err(), "{field}");
    }

    for (field, overlap) in [
        ("missing_wmo", fixture::MISSING_M2),
        ("empty_wmo", fixture::MISSING_M2),
        ("empty_wmo", fixture::MISSING_WMO),
    ] {
        let mut candidate = baseline.clone();
        match field {
            "missing_wmo" => candidate.missing_wmo = vec![overlap],
            "empty_wmo" => candidate.empty_wmo = vec![overlap],
            _ => unreachable!(),
        }
        assert!(
            Coverage::parse(&candidate.encode()).is_err(),
            "{field} overlap {overlap}"
        );
    }

    for field in ["missing_m2", "missing_wmo", "empty_wmo"] {
        let mut candidate = baseline.clone();
        let ids: Vec<_> = (1..=4097).collect();
        match field {
            "missing_m2" => candidate.missing_m2 = ids,
            "missing_wmo" => candidate.missing_wmo = ids,
            "empty_wmo" => candidate.empty_wmo = ids,
            _ => unreachable!(),
        }
        assert!(
            Coverage::parse(&candidate.encode()).is_err(),
            "{field} limit"
        );
    }
}

#[test]
fn v34_inspect_extract_verify_and_verify_links_preserve_present_wmo_and_vacate_absent_one() {
    let lab = Lab::new();
    let inspection = assert_success(lab.command("inspect").output().unwrap(), "v34 inspect");
    for expected in [
        "\"contract\":\"forever-product-v34\"",
        "\"status\":\"planned\"",
        "\"build\":70450",
        "\"locale\":\"esES\"",
        "\"components\":17",
        "\"db2_files\":1",
        "\"collision_storage\":\"linux-leased-append-disk-world-v1\"",
        "\"collision_domain\":\"all-available-maps\"",
        "\"resource_profile\":\"linux-collision-request-heavy-resources-v1\"",
        "\"collision_source_profile\":\"linux70450-available-empty-wmo-v1\"",
        "\"coverage_profile\":\"linux70450-available-empty-wmo-coverage-v7\"",
        "\"coverage_authentication\":\"independently_derived\"",
        "\"root_absence_authentication\":\"producer_reported\"",
        "\"model_absence_authentication\":\"producer_reported\"",
        "\"wmo_absence_authentication\":\"producer_reported\"",
        "\"source_authentication_flags\":\"producer_reported\"",
        "\"partial_db2_declarations\":\"per-table-including-Map\"",
        "\"liquid_object_scope\":\"out_of_scope\"",
        "\"complete_global_map_coverage_claim\":false",
        "\"complete_vmap_component\":false",
        "\"full_extractor_complete\":false",
    ] {
        assert!(inspection.contains(expected), "{inspection}");
    }
    assert_key_once(&inspection, "coverage_authentication");
    assert_key_once(&inspection, "wmo_absence_authentication");
    assert!(!inspection.contains("requested_map"), "{inspection}");
    assert!(!inspection.contains("\"full_world_coverage\":true"));

    let package = lab.extract("lifecycle");
    assert_coverage_and_inventory(&package);
    assert_v7_codec_cases(&package);
    verify_package(&lab, &package);
    verify_with_plan_package(&lab, &package, &lab.plan(), &lab.v33_plan());

    let empty_only = lab.extract_with_plan(
        "empty-wmo-only",
        &lab.plan(),
        fixture::AdtMode::EmptyWmoOnly,
    );
    let empty_coverage =
        Coverage::parse(&fs::read(empty_only.join(coverage::PATH)).unwrap()).unwrap();
    assert_eq!(empty_coverage.missing_wmo, []);
    assert_eq!(empty_coverage.empty_wmo, [462695]);
    assert_eq!(empty_coverage.missing_m2, []);
    let empty_names: Vec<_> = empty_coverage
        .artifacts
        .iter()
        .filter(|artifact| {
            artifact.path.starts_with("vmaps/coverage/") && artifact.path.ends_with(".placements")
        })
        .flat_map(|artifact| {
            let bytes = fs::read(empty_only.join(&artifact.path)).unwrap();
            placement::DirBin::parse(&bytes, Default::default())
                .unwrap()
                .records()
                .map(|record| record.name().to_vec())
                .collect::<Vec<_>>()
        })
        .collect();
    let empty_name = fixture::model_name(462695);
    assert!(!empty_names.iter().any(|name| name == empty_name.as_bytes()));
    assert!(!empty_only.join(format!("vmaps/{empty_name}.vmo")).exists());
    verify_package(&lab, &empty_only);

    let mixed = Coverage::parse(&fs::read(package.join(coverage::PATH)).unwrap()).unwrap();
    assert_eq!(mixed.missing_wmo, [fixture::MISSING_WMO]);
    assert_eq!(mixed.empty_wmo, [fixture::EMPTY_WMO]);
    assert_eq!(mixed.missing_m2, [fixture::MISSING_M2]);

    let (logical_client, _) = lab.client_with_logical_name("logical-empty-origin-client");
    let logical_output = lab.0.join("logical-empty-origin-package");
    let logical_scratch = lab.private_dir("logical-empty-origin-scratch");
    let logical_result = lab.produce(
        "extract",
        &logical_client,
        &logical_output,
        &logical_scratch,
    );
    assert_failure(logical_result, "collision_no_valid_collision");
    assert!(!logical_output.join(coverage::PATH).exists());
    assert!(!logical_output.join("forever-package.json").exists());
}

fn copy_tree(source: &Path, destination: &Path) {
    fs::create_dir(destination).unwrap();
    fs::set_permissions(destination, fs::Permissions::from_mode(0o700)).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let file_type = entry.file_type().unwrap();
        let target = destination.join(entry.file_name());
        if file_type.is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            assert!(file_type.is_file());
            fs::copy(entry.path(), &target).unwrap();
            fs::set_permissions(target, fs::Permissions::from_mode(0o600)).unwrap();
        }
    }
}

fn remove_artifact(package: &Path, path: &str) {
    let context = client_context(package);
    let receipt = fs::read_dir(package.join(".state/files"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|candidate| {
            let text = fs::read_to_string(candidate).unwrap();
            Artifact::parse(&text, context, Limits::default())
                .unwrap()
                .path
                == path
        })
        .unwrap();
    fs::remove_file(receipt).unwrap();
    fs::remove_file(package.join(path)).unwrap();
}

fn interrupted_clone(base: &Path, output: &Path, collision_sealed: bool) {
    copy_tree(base, output);
    fs::remove_file(output.join("forever-package.json")).unwrap();
    fs::write(output.join(".forever-package-incomplete"), b"incomplete\n").unwrap();
    if !collision_sealed {
        fs::remove_file(output.join(".state/sealed/vmaps")).unwrap();
        remove_artifact(output, coverage::PATH);
    }
}

#[test]
fn v34_resume_matches_baseline_at_db2_collision_and_final_seal_boundaries() {
    let lab = Lab::new();
    let baseline = lab.extract("resume-baseline");
    assert_coverage_and_inventory(&baseline);

    let (early_client, early_written) = lab.client("resume-early-source");
    fixture::corrupt_input(&early_client, &early_written, fixture::POPULATED_WDT);
    let early = lab.0.join("resume-early-package");
    let early_scratch = lab.private_dir("resume-early-initial-scratch");
    let interrupted = lab.produce("extract", &early_client, &early, &early_scratch);
    assert_cli_json(&interrupted);
    assert!(!interrupted.status.success());
    assert!(early.join(".forever-package-incomplete").is_file());
    assert!(early.join(".state/sealed/db2").is_file());
    assert!(!early.join(".state/sealed/vmaps").exists());
    assert!(!early.join("forever-package.json").exists());
    assert!(!early.join(coverage::PATH).exists());
    fixture::write_client(&early_client);
    let early_resume_scratch = lab.private_dir("resume-early-resume-scratch");
    assert_success(
        lab.produce("resume", &early_client, &early, &early_resume_scratch),
        "resume after DB2 seal",
    );
    compare_with_baseline(&lab, &baseline, &early);

    let sealed = lab.0.join("resume-collision-sealed-package");
    interrupted_clone(&baseline, &sealed, true);
    assert!(sealed.join(".state/sealed/db2").is_file());
    assert!(sealed.join(".state/sealed/vmaps").is_file());
    assert!(sealed.join(coverage::PATH).is_file());
    assert!(!sealed.join("forever-package.json").exists());
    lab.resume("resume-collision-sealed", &sealed);
    compare_with_baseline(&lab, &baseline, &sealed);

    let incomplete = lab.0.join("resume-collision-incomplete-package");
    interrupted_clone(&baseline, &incomplete, false);
    assert!(incomplete.join(".state/sealed/db2").is_file());
    assert!(!incomplete.join(".state/sealed/vmaps").exists());
    assert!(!incomplete.join(coverage::PATH).exists());
    for id in [
        fixture::EMPTY_MAP,
        fixture::POPULATED_MAP,
        fixture::PRESENT_CHILD,
    ] {
        assert!(
            incomplete
                .join(format!("vmaps/{id:04}/{id:04}.vmtree"))
                .is_file()
        );
    }
    lab.resume("resume-collision-incomplete", &incomplete);
    compare_with_baseline(&lab, &baseline, &incomplete);
}

fn inject_global_declaration(package: &Path, sections: &[PartialEncryptedSection]) {
    let path = package.join("forever-package.json");
    let mut text = fs::read_to_string(&path).unwrap();
    let declarations = sections
        .iter()
        .map(PartialEncryptedSection::json)
        .collect::<Vec<_>>()
        .join(",");
    let insertion = format!("\"partial_encrypted_sections\":[{declarations}],");
    let at = text.find("\"partial_encrypted_tables\"").unwrap();
    text.insert_str(at, &insertion);
    fs::write(path, text).unwrap();
}

fn inject_foreign_table(package: &Path, sections: &[PartialEncryptedSection]) {
    let path = package.join("forever-package.json");
    let mut text = fs::read_to_string(&path).unwrap();
    let declarations = sections
        .iter()
        .map(PartialEncryptedSection::json)
        .collect::<Vec<_>>()
        .join(",");
    let insertion = format!("{{\"path\":\"dbc/esES/Foreign.db2\",\"sections\":[{declarations}]}},");
    let key = "\"partial_encrypted_tables\":[";
    let at = text.find(key).unwrap() + key.len();
    text.insert_str(at, &insertion);
    fs::write(path, text).unwrap();
}

fn assert_resume_rejects_foreign_declaration(lab: &Lab, baseline: &Path, stem: &str, global: bool) {
    let output = lab.0.join(stem);
    copy_tree(baseline, &output);
    if global {
        inject_global_declaration(&output, &[declaration()]);
    } else {
        inject_foreign_table(&output, &[declaration()]);
    }
    fs::write(output.join(".forever-package-incomplete"), b"incomplete\n").unwrap();
    let (client, _) = lab.client(&format!("{stem}-client"));
    let scratch = lab.private_dir(&format!("{stem}-scratch"));
    assert_failure(
        lab.produce("resume", &client, &output, &scratch),
        "product_partial_manifest_scope",
    );
    assert!(output.join(".forever-package-incomplete").is_file());
}

#[test]
fn v34_resume_rejects_recovered_global_and_foreign_table_declarations() {
    let lab = Lab::new();
    let baseline = lab.extract("resume-foreign-baseline");
    assert_resume_rejects_foreign_declaration(&lab, &baseline, "resume-global", true);
    assert_resume_rejects_foreign_declaration(&lab, &baseline, "resume-table", false);
}

fn sync_coverage_artifacts(files: &BTreeMap<String, Vec<u8>>, coverage: &mut Coverage) {
    coverage.artifacts = files
        .iter()
        .filter(|(path, _)| path.as_str() != MAP_PATH && path.as_str() != coverage::PATH)
        .map(|(path, bytes)| coverage::ArtifactFact {
            path: path.clone(),
            bytes: bytes.len() as u64,
            sha: integrity::digest(bytes),
        })
        .collect();
}

fn rebuilt_package(
    base: &Path,
    output: &Path,
    global_sections: Option<Vec<PartialEncryptedSection>>,
    map_sections: Option<Vec<PartialEncryptedSection>>,
    mutate: impl FnOnce(&mut BTreeMap<String, Vec<u8>>, &mut Coverage),
) {
    rebuilt_package_with_bytes(base, output, global_sections, map_sections, mutate, |_| {});
}

fn rebuilt_package_with_bytes(
    base: &Path,
    output: &Path,
    global_sections: Option<Vec<PartialEncryptedSection>>,
    map_sections: Option<Vec<PartialEncryptedSection>>,
    mutate: impl FnOnce(&mut BTreeMap<String, Vec<u8>>, &mut Coverage),
    mutate_encoded: impl FnOnce(&mut Vec<u8>),
) {
    let context = client_context(base);
    let mut files: BTreeMap<String, Vec<u8>> = package_artifacts(base, context)
        .into_iter()
        .map(|artifact| {
            let bytes = fs::read(base.join(&artifact.path)).unwrap();
            (artifact.path, bytes)
        })
        .collect();
    let mut coverage = Coverage::parse(&files[coverage::PATH]).unwrap();
    mutate(&mut files, &mut coverage);
    let mut encoded = coverage.encode();
    mutate_encoded(&mut encoded);
    files.insert(coverage::PATH.into(), encoded);
    let mut run = Run::create(output, context, &[base], Limits::default()).unwrap();
    if let Some(sections) = map_sections {
        run.set_partial_encrypted_table(MAP_PATH, &sections)
            .unwrap();
    }
    for (path, bytes) in files {
        let component = if path == MAP_PATH {
            Component::Db2
        } else {
            Component::Collision
        };
        run.put(component, &path, &bytes).unwrap();
    }
    run.finish_component(Component::Db2).unwrap();
    run.finish_component(Component::Collision).unwrap();
    run.finish().unwrap();
    if let Some(sections) = global_sections {
        inject_global_declaration(output, &sections);
    }
}

fn verify_failure(lab: &Lab, package: &Path, code: &str) {
    assert_failure(
        lab.command("verify")
            .args(["--output"])
            .arg(package)
            .output()
            .unwrap(),
        code,
    );
}

fn verify_links_failure(lab: &Lab, package: &Path, code: &str) {
    assert_failure(
        lab.command("verify-links")
            .args(["--output"])
            .arg(package)
            .output()
            .unwrap(),
        code,
    );
}

fn verify_failure_one_of(lab: &Lab, package: &Path, codes: &[&str]) {
    assert_failure_one_of(
        lab.command("verify")
            .args(["--output"])
            .arg(package)
            .output()
            .unwrap(),
        codes,
    );
}

#[test]
fn v34_resealed_manifest_codec_policy_and_context_negatives_reach_product_checks() {
    let lab = Lab::new();
    let baseline = lab.extract("negative-codec-baseline");
    let map = fixture::partial_map();
    let declaration = map.declaration.clone();

    let global = lab.0.join("negative-global-declaration");
    rebuilt_package(
        &baseline,
        &global,
        Some(vec![declaration.clone()]),
        Some(vec![declaration.clone()]),
        |_, _| {},
    );
    verify_failure(&lab, &global, "product_partial_manifest_scope");

    let undeclared = lab.0.join("negative-undeclared-map");
    rebuilt_package(&baseline, &undeclared, None, None, |_, _| {});
    verify_failure(&lab, &undeclared, "product_partial_undeclared");

    let mut wrong_size = declaration.clone();
    wrong_size.decoded_size -= 1;
    let inconsistent = lab.0.join("negative-wrong-decoded-size");
    rebuilt_package(
        &baseline,
        &inconsistent,
        None,
        Some(vec![wrong_size]),
        |_, _| {},
    );
    verify_failure(&lab, &inconsistent, "product_partial_section_count");

    let zero_fill = lab.0.join("negative-zero-fill");
    rebuilt_package(
        &baseline,
        &zero_fill,
        None,
        Some(vec![declaration.clone()]),
        |files, _| {
            let bytes = files.get_mut(MAP_PATH).unwrap();
            assert_eq!(bytes[map.hidden_start], 0);
            bytes[map.hidden_start] = 1;
        },
    );
    verify_failure(&lab, &zero_fill, "product_partial_zerofill");

    for (name, version) in [("v30", 3), ("v29", 2), ("v28", 1)] {
        let output = lab.0.join(format!("negative-cross-{name}"));
        rebuilt_package(
            &baseline,
            &output,
            None,
            Some(vec![declaration.clone()]),
            |_, coverage| {
                coverage.version = version;
                coverage.missing_m2_policy = run::MissingModelPolicy::Reject;
                match version {
                    3 => {
                        coverage.selection = MapSelection::All;
                        coverage.missing_wdt_policy = run::MissingWdtPolicy::DeclareRootAbsent;
                        coverage.coverage_profile = coverage::V30_COVERAGE_PROFILE.into();
                        coverage.source_profile = coverage::V30_SOURCE_PROFILE.into();
                        coverage.domain_profile = coverage::V30_DOMAIN_PROFILE.into();
                    }
                    2 => {
                        coverage.selection = MapSelection::All;
                        coverage.missing_wdt_policy = run::MissingWdtPolicy::Reject;
                        coverage.coverage_profile = coverage::V29_COVERAGE_PROFILE.into();
                        coverage.source_profile = coverage::V29_SOURCE_PROFILE.into();
                        coverage.domain_profile = coverage::V29_DOMAIN_PROFILE.into();
                    }
                    _ => {
                        coverage.selection = MapSelection::MapAndAncestors(fixture::POPULATED_MAP);
                        coverage.missing_wdt_policy = run::MissingWdtPolicy::Reject;
                        coverage.coverage_profile = coverage::SELECTED_COVERAGE_PROFILE.into();
                        coverage.source_profile = coverage::SELECTED_SOURCE_PROFILE.into();
                        coverage.domain_profile = coverage::SELECTED_DOMAIN_PROFILE.into();
                    }
                }
                coverage.missing_m2.clear();
            },
        );
        verify_failure(&lab, &output, "product_selected_geometry_coverage_schema");
        if version == 3 {
            verify_links_failure(&lab, &output, "product_selected_geometry_coverage_schema");
        }
    }

    for name in ["coverage-profile", "source-profile", "domain-profile"] {
        let output = lab.0.join(format!("negative-{name}"));
        rebuilt_package(
            &baseline,
            &output,
            None,
            Some(vec![declaration.clone()]),
            |_, coverage| match name {
                "coverage-profile" => {
                    coverage.coverage_profile = coverage::V30_COVERAGE_PROFILE.into();
                }
                "source-profile" => {
                    coverage.source_profile = coverage::V30_SOURCE_PROFILE.into();
                }
                "domain-profile" => {
                    coverage.domain_profile = coverage::V30_DOMAIN_PROFILE.into();
                }
                _ => unreachable!(),
            },
        );
        verify_failure(&lab, &output, "product_selected_geometry_coverage_schema");
    }

    for (name, wdt, model) in [
        (
            "wdt-policy",
            run::MissingWdtPolicy::Reject,
            run::MissingModelPolicy::DeclareRootAbsentWmoDoodadM2,
        ),
        (
            "model-policy",
            run::MissingWdtPolicy::DeclareRootAbsent,
            run::MissingModelPolicy::Reject,
        ),
    ] {
        let output = lab.0.join(format!("negative-{name}"));
        rebuilt_package(
            &baseline,
            &output,
            None,
            Some(vec![declaration.clone()]),
            |_, coverage| {
                coverage.missing_wdt_policy = wdt;
                coverage.missing_m2_policy = model;
            },
        );
        verify_failure(&lab, &output, "product_selected_geometry_coverage_schema");
    }

    let wrong_mask = lab.0.join("negative-context-mask");
    rebuilt_package(
        &baseline,
        &wrong_mask,
        None,
        Some(vec![declaration.clone()]),
        |_, coverage| coverage.context.components = 16,
    );
    verify_failure(
        &lab,
        &wrong_mask,
        "product_selected_geometry_coverage_schema",
    );

    let wrong_plan = lab.0.join("negative-context-plan");
    rebuilt_package(
        &baseline,
        &wrong_plan,
        None,
        Some(vec![declaration.clone()]),
        |_, coverage| coverage.context.plan_sha256 = [0x99; 32],
    );
    verify_failure(&lab, &wrong_plan, "product_plan_mismatch");

    for (name, missing_ids, empty_ids) in [
        ("missing-wmo-zero", vec![0], vec![fixture::EMPTY_WMO]),
        (
            "missing-wmo-duplicate",
            vec![fixture::MISSING_WMO, fixture::MISSING_WMO],
            vec![fixture::EMPTY_WMO],
        ),
        (
            "missing-wmo-unsorted",
            vec![fixture::MISSING_WMO, fixture::MODEL],
            vec![fixture::EMPTY_WMO],
        ),
        (
            "missing-wmo-overlap-missing-m2",
            vec![fixture::MISSING_M2],
            vec![fixture::EMPTY_WMO],
        ),
        (
            "missing-wmo-overlap-empty-wmo",
            vec![fixture::EMPTY_WMO],
            vec![fixture::EMPTY_WMO],
        ),
        ("empty-wmo-zero", vec![fixture::MISSING_WMO], vec![0]),
        (
            "empty-wmo-duplicate",
            vec![fixture::MISSING_WMO],
            vec![fixture::EMPTY_WMO, fixture::EMPTY_WMO],
        ),
        (
            "empty-wmo-unsorted",
            vec![fixture::MISSING_WMO],
            vec![fixture::EMPTY_WMO, fixture::MODEL],
        ),
        (
            "empty-wmo-overlap-missing-m2",
            vec![fixture::MISSING_WMO],
            vec![fixture::MISSING_M2],
        ),
        (
            "empty-wmo-overlap-missing-wmo",
            vec![fixture::MISSING_WMO],
            vec![fixture::MISSING_WMO],
        ),
    ] {
        let output = lab.0.join(format!("negative-{name}"));
        rebuilt_package(
            &baseline,
            &output,
            None,
            Some(vec![declaration.clone()]),
            |_, coverage| {
                coverage.missing_wmo = missing_ids;
                coverage.empty_wmo = empty_ids;
            },
        );
        verify_failure(&lab, &output, "product_selected_geometry_coverage_schema");
        verify_links_failure(&lab, &output, "product_selected_geometry_coverage_schema");
    }

    let wrong_wmo_policy = lab.0.join("negative-wmo-policy");
    rebuilt_package(
        &baseline,
        &wrong_wmo_policy,
        None,
        Some(vec![declaration.clone()]),
        |_, coverage| coverage.missing_wmo_policy = run::MissingWmoPolicy::Reject,
    );
    verify_failure(
        &lab,
        &wrong_wmo_policy,
        "product_selected_geometry_coverage_schema",
    );

    let global_claim = lab.0.join("negative-global-coverage-claim");
    rebuilt_package(
        &baseline,
        &global_claim,
        None,
        Some(vec![declaration.clone()]),
        |_, coverage| coverage.complete_global_map_coverage = true,
    );
    verify_failure(
        &lab,
        &global_claim,
        "product_selected_geometry_coverage_schema",
    );
}

#[test]
fn v34_resealed_omission_artifact_and_parent_replay_negatives_are_rejected() {
    let lab = Lab::new();
    let baseline = lab.extract("negative-omission-baseline");
    let declaration = declaration();

    let false_omission = lab.0.join("negative-false-wdt-omission");
    rebuilt_package(
        &baseline,
        &false_omission,
        None,
        Some(vec![declaration.clone()]),
        |_, coverage| coverage.maps[0].root_absent = true,
    );
    verify_failure(&lab, &false_omission, "product_collision_association");

    let overlapping = lab.0.join("negative-overlapping-wdt-omission");
    rebuilt_package(
        &baseline,
        &overlapping,
        None,
        Some(vec![declaration.clone()]),
        |_, coverage| {
            let duplicate = coverage.maps[2].clone();
            coverage.maps.insert(3, duplicate);
        },
    );
    verify_failure(
        &lab,
        &overlapping,
        "product_selected_geometry_coverage_schema",
    );

    let wrong_fdid = lab.0.join("negative-wdt-omission-wrong-fdid");
    rebuilt_package(
        &baseline,
        &wrong_fdid,
        None,
        Some(vec![declaration.clone()]),
        |_, coverage| coverage.maps[2].wdt_file_data_id += 1,
    );
    verify_failure(&lab, &wrong_fdid, "product_collision_association");

    let missing_omission = lab.0.join("negative-required-wdt-omission-missing");
    rebuilt_package(
        &baseline,
        &missing_omission,
        None,
        Some(vec![declaration.clone()]),
        |_, coverage| coverage.maps[2].root_absent = false,
    );
    verify_failure(&lab, &missing_omission, "product_collision_association");

    for (name, omitted_path, source_path) in [
        ("tree", "vmaps/0004/0004.vmtree", "vmaps/0001/0001.vmtree"),
        (
            "proof",
            "vmaps/coverage/0004.placements",
            "vmaps/coverage/0001.placements",
        ),
        (
            "tile",
            "vmaps/0004/0004_00_00.vmtile",
            "vmaps/0002/0002_00_00.vmtile",
        ),
    ] {
        let output = lab.0.join(format!("negative-omitted-artifact-{name}"));
        let source = fs::read(baseline.join(source_path)).unwrap();
        rebuilt_package(
            &baseline,
            &output,
            None,
            Some(vec![declaration.clone()]),
            move |files, coverage| {
                files.insert(omitted_path.into(), source);
                sync_coverage_artifacts(files, coverage);
            },
        );
        verify_failure(&lab, &output, "product_collision_association");
    }

    let missing_empty_proof = lab.0.join("negative-present-empty-without-proof");
    rebuilt_package(
        &baseline,
        &missing_empty_proof,
        None,
        Some(vec![declaration.clone()]),
        |files, coverage| {
            files.remove("vmaps/coverage/0001.placements");
            sync_coverage_artifacts(files, coverage);
        },
    );
    verify_failure(&lab, &missing_empty_proof, "product_collision_association");
    verify_links_failure(&lab, &missing_empty_proof, "product_collision_association");

    for (name, ids) in [
        ("duplicate", vec![fixture::MISSING_M2, fixture::MISSING_M2]),
        ("zero", vec![0]),
        ("unsorted", vec![fixture::MISSING_M2, fixture::MODEL]),
        ("present-fdid", vec![fixture::MODEL]),
    ] {
        let output = lab.0.join(format!("negative-missing-m2-{name}"));
        rebuilt_package(
            &baseline,
            &output,
            None,
            Some(vec![declaration.clone()]),
            |_, coverage| coverage.missing_m2 = ids,
        );
        verify_failure_one_of(
            &lab,
            &output,
            &[
                "product_selected_geometry_coverage_schema",
                "product_collision_association",
                "product_selected_geometry_inventory",
                "product_selected_geometry_model_binding",
            ],
        );
    }

    let omitted_vmo = lab.0.join("negative-vmo-for-omitted-doodad");
    let omitted_name = fixture::model_name(fixture::MISSING_M2);
    let present_vmo =
        fs::read(baseline.join(format!("vmaps/{}.vmo", fixture::model_name(fixture::MODEL))))
            .unwrap();
    rebuilt_package(
        &baseline,
        &omitted_vmo,
        None,
        Some(vec![declaration.clone()]),
        move |files, coverage| {
            files.insert(format!("vmaps/{omitted_name}.vmo"), present_vmo);
            sync_coverage_artifacts(files, coverage);
        },
    );
    verify_failure_one_of(
        &lab,
        &omitted_vmo,
        &[
            "product_selected_geometry_coverage_schema",
            "product_collision_association",
            "product_selected_geometry_inventory",
        ],
    );

    for id in [
        fixture::MISSING_M2,
        fixture::MISSING_WMO,
        fixture::EMPTY_WMO,
    ] {
        let output = lab.0.join(format!("negative-spawn-for-omitted-{id}"));
        rebuilt_package(
            &baseline,
            &output,
            None,
            Some(vec![declaration.clone()]),
            |files, coverage| {
                replace_placement_model(files, fixture::MODEL, id);
                sync_coverage_artifacts(files, coverage);
                assert_eq!(
                    coverage
                        .artifacts
                        .iter()
                        .find(|fact| fact.path == "vmaps/coverage/0002.placements")
                        .unwrap()
                        .sha,
                    integrity::digest(&files["vmaps/coverage/0002.placements"])
                );
            },
        );
        verify_failure(&lab, &output, "product_collision_association");
        verify_links_failure(&lab, &output, "product_collision_association");
        let control = lab.0.join(format!("binding-without-omission-{id}"));
        rebuilt_package(
            &baseline,
            &control,
            None,
            Some(vec![declaration.clone()]),
            |files, coverage| {
                replace_placement_model(files, fixture::MODEL, id);
                match id {
                    fixture::MISSING_M2 => coverage.missing_m2.clear(),
                    fixture::MISSING_WMO => coverage.missing_wmo.clear(),
                    fixture::EMPTY_WMO => coverage.empty_wmo.clear(),
                    _ => unreachable!(),
                }
                sync_coverage_artifacts(files, coverage);
            },
        );
        verify_failure(&lab, &control, "product_selected_geometry_model_binding");
        verify_links_failure(&lab, &control, "product_selected_geometry_model_binding");
    }

    let hidden_unknown = lab.0.join("negative-hidden-unknown-invented");
    rebuilt_package(
        &baseline,
        &hidden_unknown,
        None,
        Some(vec![declaration.clone()]),
        |_, coverage| {
            coverage.maps.push(coverage::DomainMapFact {
                id: fixture::HIDDEN_MAP,
                parent: None,
                wdt_file_data_id: 109,
                depth: 0,
                is_parent: false,
                root_absent: true,
            });
        },
    );
    verify_failure(&lab, &hidden_unknown, "product_collision_association");
}

fn replace_placement_model(files: &mut BTreeMap<String, Vec<u8>>, old: u32, new: u32) {
    let old_name = fixture::model_name(old);
    let new_name = fixture::model_name(new);
    let mut proof = files["vmaps/coverage/0002.placements"].clone();
    let at = proof
        .windows(old_name.len())
        .position(|window| window == old_name.as_bytes())
        .unwrap();
    proof[at..at + old_name.len()].copy_from_slice(new_name.as_bytes());
    let parsed = placement::DirBin::parse(&proof, Default::default()).unwrap();
    assert!(
        parsed
            .records()
            .any(|record| record.name() == new_name.as_bytes())
    );
    let records: Vec<_> = parsed.records().collect();
    let assembled = assembly::build(fixture::POPULATED_MAP, &records, Default::default()).unwrap();
    files.insert(
        "vmaps/coverage/0002.placements".into(),
        assembled.ordered_dir_bin().to_vec(),
    );
    files.insert(
        "vmaps/0002/0002.vmtree".into(),
        assembled.tree_bytes().to_vec(),
    );
    for tile in assembled.tiles() {
        let stem = format!("vmaps/0002/0002_{:02}_{:02}", tile.x(), tile.y());
        files.insert(format!("{stem}.vmtileidx"), tile.index_bytes().to_vec());
        let payload = tile.payload().unwrap();
        assembly::verify_associations(
            payload,
            tile.index_bytes(),
            assembled.ordered_dir_bin(),
            Default::default(),
        )
        .unwrap();
        files.insert(format!("{stem}.vmtile"), payload.to_vec());
    }
}

fn make_parent_only(files: &mut BTreeMap<String, Vec<u8>>, coverage: &mut Coverage) {
    let proof_path = "vmaps/coverage/0005.placements";
    let mut proof = files["vmaps/coverage/0002.placements"].clone();
    let lengths: Vec<usize> = placement::DirBin::parse(&proof, Default::default())
        .unwrap()
        .records()
        .map(|record| (if record.flags() & 1 != 0 { 62 } else { 38 }) + record.name().len())
        .collect();
    let mut at = 0;
    for length in lengths {
        proof[at] |= 2;
        at += length;
    }
    assert_eq!(at, proof.len());
    let placements = placement::DirBin::parse(&proof, Default::default()).unwrap();
    let records: Vec<_> = placements.records().collect();
    let child = assembly::build(fixture::PRESENT_CHILD, &records, Default::default()).unwrap();
    assert!(child.stats().parent_only_tiles > 0);
    assert!(child.tiles().iter().all(|tile| tile.payload().is_none()));
    let grandparent = files["vmaps/0002/0002_00_00.vmtile"].as_slice();
    assembly::verify_inherited_associations(
        grandparent,
        child.tiles()[0].index_bytes(),
        child.ordered_dir_bin(),
        Default::default(),
    )
    .unwrap();
    files.insert(proof_path.into(), child.ordered_dir_bin().to_vec());
    files.insert("vmaps/0005/0005.vmtree".into(), child.tree_bytes().to_vec());
    for tile in child.tiles() {
        files.insert(
            format!("vmaps/0005/0005_{:02}_{:02}.vmtileidx", tile.x(), tile.y()),
            tile.index_bytes().to_vec(),
        );
        files.remove(&format!(
            "vmaps/0005/0005_{:02}_{:02}.vmtile",
            tile.x(),
            tile.y()
        ));
    }
    sync_coverage_artifacts(files, coverage);
}

#[test]
fn v34_parent_replay_accepts_present_parent_and_rejects_root_absent_parent() {
    let lab = Lab::new();
    let baseline = lab.extract("parent-replay-baseline");
    let declaration = declaration();

    let valid = lab.0.join("valid-present-parent-inheritance");
    rebuilt_package(
        &baseline,
        &valid,
        None,
        Some(vec![declaration.clone()]),
        |files, coverage| {
            fixture::set_parent(files.get_mut(MAP_PATH).unwrap(), 4, 2);
            coverage.maps[3].parent = Some(2);
            coverage.maps[3].depth = 2;
            coverage.maps[2].is_parent = false;
            make_parent_only(files, coverage);
        },
    );
    verify_package(&lab, &valid);

    let forbidden = lab.0.join("negative-parent-only-through-root-absent");
    rebuilt_package(
        &baseline,
        &forbidden,
        None,
        Some(vec![declaration.clone()]),
        make_parent_only,
    );
    verify_failure_one_of(
        &lab,
        &forbidden,
        &[
            "product_collision_association",
            "product_selected_geometry_parent_payload",
        ],
    );
}

#[test]
fn v34_vacated_wmo_fatal_negatives_require_the_complete_predicate() {
    let lab = Lab::new();
    let stem = "missing-doodad-unused-by-modr";
    let (client, _) = lab.client_with(
        &format!("fatal-{stem}-client"),
        fixture::ClientMutation {
            populated_adt_mode: fixture::AdtMode::UnusedMissingDoodad,
            ..Default::default()
        },
    );
    let output = lab.0.join(format!("fatal-{stem}-package"));
    let scratch = lab.private_dir(&format!("fatal-{stem}-scratch"));
    assert_failure(
        lab.produce("extract", &client, &output, &scratch),
        "collision_no_valid_collision",
    );
    assert!(!output.join(coverage::PATH).exists(), "{stem}");
    assert!(!output.join("forever-package.json").exists(), "{stem}");
    assert!(!output.join(".state/sealed/vmaps").exists(), "{stem}");

    for (stem, mutation) in [
        (
            "direct-m2-absent",
            fixture::ClientMutation {
                populated_adt_mode: fixture::AdtMode::DirectM2Absent,
                ..Default::default()
            },
        ),
        (
            "wmo-root-absent",
            fixture::ClientMutation {
                populated_adt_mode: fixture::AdtMode::MissingWmo,
                ..Default::default()
            },
        ),
        (
            "wmo-group-absent",
            fixture::ClientMutation {
                populated_adt_mode: fixture::AdtMode::MissingWmoGroup,
                ..Default::default()
            },
        ),
        (
            "logical-m2-name",
            fixture::ClientMutation {
                populated_adt_mode: fixture::AdtMode::LogicalM2Name,
                ..Default::default()
            },
        ),
        (
            "adt-absent",
            fixture::ClientMutation {
                omit_root: Some(fixture::POPULATED_ADT),
                ..Default::default()
            },
        ),
        (
            "ineligible-m2-root",
            fixture::ClientMutation {
                ineligible_root: Some(fixture::MODEL),
                ..Default::default()
            },
        ),
        (
            "m2-central-absent",
            fixture::ClientMutation {
                omit_central: Some(fixture::MODEL),
                ..Default::default()
            },
        ),
        (
            "m2-encoding-absent",
            fixture::ClientMutation {
                omit_encoding: Some(fixture::MODEL),
                ..Default::default()
            },
        ),
        (
            "malformed-m2",
            fixture::ClientMutation {
                malformed_model: Some(fixture::MODEL),
                ..Default::default()
            },
        ),
        (
            "ineligible-wdt-root",
            fixture::ClientMutation {
                ineligible_root: Some(fixture::POPULATED_WDT),
                ..Default::default()
            },
        ),
        (
            "malformed-wdt",
            fixture::ClientMutation {
                malformed_wdt: Some(fixture::POPULATED_WDT),
                ..Default::default()
            },
        ),
    ] {
        let (client, _) = lab.client_with(&format!("fatal-{stem}-client"), mutation);
        let output = lab.0.join(format!("fatal-{stem}-package"));
        let scratch = lab.private_dir(&format!("fatal-{stem}-scratch"));
        let result = lab.produce("extract", &client, &output, &scratch);
        assert_cli_json(&result);
        assert!(!result.status.success(), "{stem} unexpectedly passed");
        assert!(!output.join(coverage::PATH).exists(), "{stem}");
        assert!(!output.join("forever-package.json").exists(), "{stem}");
        assert!(!output.join(".state/sealed/vmaps").exists(), "{stem}");
    }

    for (stem, id) in [
        ("corrupt-m2", fixture::MODEL),
        ("corrupt-wdt", fixture::POPULATED_WDT),
    ] {
        let (client, written) = lab.client(&format!("fatal-{stem}-client"));
        fixture::corrupt_input(&client, &written, id);
        let output = lab.0.join(format!("fatal-{stem}-package"));
        let scratch = lab.private_dir(&format!("fatal-{stem}-scratch"));
        let result = lab.produce("extract", &client, &output, &scratch);
        assert_cli_json(&result);
        assert!(!result.status.success());
        assert!(!output.join(coverage::PATH).exists());
        assert!(!output.join("forever-package.json").exists());
        assert!(!output.join(".state/sealed/vmaps").exists());
    }
}

#[test]
fn v34_intrinsic_empty_predicate_checks_geometry_dependencies_groups_and_sets() {
    let lab = Lab::new();
    for (stem, mode, succeeds) in [
        ("geometry", fixture::AdtMode::EmptyWmoGeometry, true),
        (
            "missing-group",
            fixture::AdtMode::EmptyWmoMissingGroup,
            false,
        ),
        ("missing-sets", fixture::AdtMode::EmptyWmoMissingSets, false),
        (
            "useful-binding",
            fixture::AdtMode::EmptyWmoUsefulBindings,
            false,
        ),
    ] {
        let (client, _) = lab.client_with(
            &format!("predicate-{stem}-client"),
            fixture::ClientMutation {
                populated_adt_mode: mode,
                ..Default::default()
            },
        );
        let output = lab.0.join(format!("predicate-{stem}-package"));
        let scratch = lab.private_dir(&format!("predicate-{stem}-scratch"));
        let result = lab.produce("extract", &client, &output, &scratch);
        if succeeds {
            assert_success(result, stem);
            let coverage =
                Coverage::parse(&fs::read(output.join(coverage::PATH)).unwrap()).unwrap();
            assert!(coverage.empty_wmo.is_empty());
            assert!(coverage.missing_wmo.is_empty());
            let name = fixture::model_name(fixture::EMPTY_WMO);
            let bytes = fs::read(output.join(format!("vmaps/{name}.vmo"))).unwrap();
            vmo::View::parse(&bytes, Default::default()).unwrap();
            let (_, names) = present_artifacts(&output, fixture::POPULATED_MAP, 3);
            assert!(names.contains(&name));
            verify_package(&lab, &output);
        } else {
            if mode == fixture::AdtMode::EmptyWmoMissingGroup {
                assert_failure(result, "file_unavailable");
            } else {
                assert_failure(result, "collision_no_valid_collision");
            }
            for path in [
                coverage::PATH,
                "forever-package.json",
                ".state/sealed/vmaps",
            ] {
                assert!(!output.join(path).exists(), "{stem}: {path}");
            }
        }
    }
}

#[test]
fn v34_omissions_do_not_consume_ids_or_change_present_placements() {
    let lab = Lab::new();
    let mixed = lab.extract("ids-mixed");
    let control = lab.extract_with_plan("ids-control", &lab.plan(), fixture::AdtMode::NoOmissions);
    let omitted = Coverage::parse(&fs::read(mixed.join(coverage::PATH)).unwrap()).unwrap();
    assert_eq!(omitted.missing_m2, [fixture::MISSING_M2]);
    assert_eq!(omitted.missing_wmo, [fixture::MISSING_WMO]);
    assert_eq!(omitted.empty_wmo, [fixture::EMPTY_WMO]);
    let clean = Coverage::parse(&fs::read(control.join(coverage::PATH)).unwrap()).unwrap();
    assert!(
        clean.missing_m2.is_empty() && clean.missing_wmo.is_empty() && clean.empty_wmo.is_empty()
    );
    // This black-box check observes serialized IDs/placements only. Internal bindings
    // and valid_names are covered by forever-casc collision/run/shared tests.
    let mut ids_by_map = Vec::new();
    for map in [fixture::POPULATED_MAP, fixture::PRESENT_CHILD] {
        let path = format!("vmaps/coverage/{map:04}.placements");
        let actual = fs::read(mixed.join(&path)).unwrap();
        let expected = fs::read(control.join(&path)).unwrap();
        let ids = |bytes: &[u8]| {
            placement::DirBin::parse(bytes, Default::default())
                .unwrap()
                .records()
                .map(|record| record.id())
                .collect::<Vec<_>>()
        };
        assert_eq!(ids(&actual), ids(&expected), "map {map}");
        assert_eq!(actual, expected, "map {map}");
        ids_by_map.push(ids(&actual));
    }
    assert_eq!(ids_by_map.len(), 2);
    assert!(ids_by_map.iter().all(|ids| ids.len() == 2));
    assert!(
        ids_by_map[1].iter().any(|id| !ids_by_map[0].contains(id)),
        "the child must exercise a new present placement after the omissions"
    );
    verify_package(&lab, &mixed);
    verify_package(&lab, &control);
}

#[test]
fn v34_production_verifiers_reject_raw_truncations_suffixes_and_omission_limits() {
    let lab = Lab::new();
    let baseline = lab.extract("raw-codec-baseline");
    let bytes = fs::read(baseline.join(coverage::PATH)).unwrap();
    for end in [0, 7, 8, 11, 12, bytes.len() / 2, bytes.len() - 1] {
        let output = lab.0.join(format!("raw-prefix-{end}"));
        rebuilt_package_with_bytes(
            &baseline,
            &output,
            None,
            Some(vec![declaration()]),
            |_, _| {},
            |encoded| encoded.truncate(end),
        );
        verify_failure(&lab, &output, "product_selected_geometry_coverage_schema");
        verify_links_failure(&lab, &output, "product_selected_geometry_coverage_schema");
    }
    for length in [1, 8] {
        let output = lab.0.join(format!("raw-suffix-{length}"));
        rebuilt_package_with_bytes(
            &baseline,
            &output,
            None,
            Some(vec![declaration()]),
            |_, _| {},
            |encoded| encoded.extend(vec![0; length]),
        );
        verify_failure(&lab, &output, "product_selected_geometry_coverage_schema");
        verify_links_failure(&lab, &output, "product_selected_geometry_coverage_schema");
    }
    for field in ["missing_m2", "missing_wmo", "empty_wmo"] {
        for count in [4096, 4097] {
            let output = lab.0.join(format!("raw-{field}-{count}"));
            rebuilt_package_with_bytes(
                &baseline,
                &output,
                None,
                Some(vec![declaration()]),
                |_, coverage| {
                    let ids = (2_000_000..2_000_000 + count).collect();
                    match field {
                        "missing_m2" => coverage.missing_m2 = ids,
                        "missing_wmo" => coverage.missing_wmo = ids,
                        "empty_wmo" => coverage.empty_wmo = ids,
                        _ => unreachable!(),
                    }
                },
                |_| {},
            );
            if count == 4096 {
                verify_package(&lab, &output);
            } else {
                verify_failure(&lab, &output, "product_selected_geometry_limit");
                verify_links_failure(&lab, &output, "product_selected_geometry_limit");
            }
        }
    }
}

fn produce_diagnostic(
    lab: &Lab,
    client: &Path,
    output: &Path,
    scratch: &Path,
    diagnostic: &Path,
) -> Output {
    lab.command("extract")
        .args(["--client"])
        .arg(client)
        .args(["--output"])
        .arg(output)
        .args(["--scratch"])
        .arg(scratch)
        .env("FOREVER_PRODUCT_DIAGNOSTIC", diagnostic)
        .output()
        .unwrap()
}

fn assert_redacted_diagnostic(record: &str, client: &Path, package: &Path) {
    assert_eq!(record.lines().count(), 1, "{record}");
    for expected in [
        "\"phase\":\"models\"",
        "\"request_kind\":\"child\"",
        &format!("\"file_data_id\":{}", fixture::MODEL),
        "\"ordinal\":",
        "\"typed_cause\":{",
        "\"variant\":\"NoLocalEncoding\"",
        "\"corrupt\":1",
        "\"blte_code\":\"blte_checksum_mismatch\"",
        "\"code\":\"local_store_no_verified_encoding\"",
        "\"guards_observed\":",
        "\"tolerated_absences\":",
    ] {
        assert!(record.contains(expected), "missing {expected}: {record}");
    }
    for forbidden in [
        "\"path\"",
        "\"payload\"",
        "\"bytes\"",
        "content_key",
        "encoded_key",
        "ciphertext",
        "Data/data",
        "vmaps/",
        "BLTE",
        "MD20",
    ] {
        assert!(
            !record.contains(forbidden),
            "forbidden {forbidden}: {record}"
        );
    }
    for path in [client, package] {
        let path = path.to_string_lossy();
        assert!(!record.contains(path.as_ref()), "path leaked: {record}");
    }
}

#[test]
fn v34_diagnostic_is_opt_in_redacted_and_preserves_the_terminal_failure() {
    let lab = Lab::new();
    let (client, written) = lab.client("diagnostic-opt-in-client");
    fixture::corrupt_input(&client, &written, fixture::MODEL);
    let diagnostic = lab.0.join("diagnostic-opt-in.jsonl");

    let without_output = lab.0.join("diagnostic-without-package");
    let without_scratch = lab.private_dir("diagnostic-without-scratch");
    let without = lab.produce("extract", &client, &without_output, &without_scratch);
    assert_cli_json(&without);
    let without_status = without.status.code();
    let without_stdout = without.stdout.clone();
    let without_stderr = without.stderr.clone();
    assert!(!diagnostic.exists());

    let with_output = lab.0.join("diagnostic-with-package");
    let with_scratch = lab.private_dir("diagnostic-with-scratch");
    let with = produce_diagnostic(&lab, &client, &with_output, &with_scratch, &diagnostic);
    assert_cli_json(&with);
    assert_eq!(with.status.code(), without_status);
    assert_eq!(with.stdout, without_stdout);
    assert_eq!(with.stderr, without_stderr);
    assert!(String::from_utf8_lossy(&with.stderr).contains("local_store_no_verified_encoding"));
    let record = fs::read_to_string(&diagnostic).unwrap();
    assert_redacted_diagnostic(&record, &client, &with_output);
}
