// SPDX-License-Identifier: GPL-3.0-or-later
#![cfg(target_os = "linux")]

#[path = "v36_material_filtered_wmo_geometry_cli/coverage.rs"]
mod coverage;
#[path = "v36_material_filtered_wmo_geometry_cli/fixture.rs"]
#[allow(dead_code)]
mod fixture;

use coverage::Coverage;
use forever_casc::{
    integrity,
    package::{Artifact, Component, Context, Limits, PartialEncryptedSection, Run},
};
use forever_extractor::plan::{
    MATERIAL_FILTERED_WMO_VERIFY_PROFILE, MATERIAL_FILTERED_WMO_VERIFY_SCOPE,
};
use forever_vmap::{assembly, placement, vmo};
use std::{
    collections::BTreeMap,
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};

const PLAN: &str = include_str!("../../../tests/contracts/fixtures/forever-product-v36/all.tsv");
const V35_PLAN: &str =
    include_str!("../../../tests/contracts/fixtures/forever-product-v35/all.tsv");
const MAP_PATH: &str = "dbc/esES/Map.db2";
static NEXT: AtomicU64 = AtomicU64::new(0);

struct Lab(PathBuf);

impl Lab {
    fn new() -> Self {
        let root = std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join(format!(
                ".local/share/rce-v36-material-filtered-wmo-cli-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
        fs::create_dir_all(root.parent().unwrap()).unwrap();
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        fs::write(root.join("plan.tsv"), PLAN).unwrap();
        Self(root)
    }

    fn private_dir(&self, name: &str) -> PathBuf {
        let path = self.0.join(name);
        fs::create_dir(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        path
    }

    fn plan(&self) -> PathBuf {
        self.0.join("plan.tsv")
    }

    fn v35_plan(&self) -> PathBuf {
        let path = self.0.join("plan-v35.tsv");
        fs::write(&path, V35_PLAN).unwrap();
        path
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

    fn produce(
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

    fn extract(&self, name: &str, mode: fixture::AdtMode) -> PathBuf {
        let client = self.private_dir(&format!("{name}-client"));
        fixture::write_client_with(
            &client,
            fixture::ClientMutation {
                populated_adt_mode: mode,
                ..Default::default()
            },
        );
        let package = self.0.join(format!("{name}-package"));
        let scratch = self.private_dir(&format!("{name}-scratch"));
        assert_success(
            self.produce("extract", &self.plan(), &client, &package, &scratch),
            "v36 extract",
        );
        package
    }

    fn verify(&self, package: &Path) {
        for verb in ["verify", "verify-links"] {
            let report = assert_success(
                self.command(verb)
                    .args(["--output"])
                    .arg(package)
                    .output()
                    .unwrap(),
                verb,
            );
            for identity in [
                format!("\"verification_profile\":\"{MATERIAL_FILTERED_WMO_VERIFY_PROFILE}\""),
                format!("\"scope\":\"{MATERIAL_FILTERED_WMO_VERIFY_SCOPE}\""),
                "\"coverage_authentication\":\"independently_derived\"".into(),
                "\"wmo_absence_authentication\":\"producer_reported\"".into(),
                "\"material_filtered_wmo_authentication\":\"producer_reported\"".into(),
                "\"components\":17".into(),
                "\"full_extractor_complete\":false".into(),
            ] {
                assert!(report.contains(&identity), "{verb}: {identity}: {report}");
            }
        }
    }

    fn extract_result(&self, name: &str, mode: fixture::AdtMode) -> (Output, PathBuf) {
        let client = self.private_dir(&format!("{name}-client"));
        fixture::write_client_with(
            &client,
            fixture::ClientMutation {
                populated_adt_mode: mode,
                ..Default::default()
            },
        );
        let package = self.0.join(format!("{name}-package"));
        let scratch = self.private_dir(&format!("{name}-scratch"));
        (
            self.produce("extract", &self.plan(), &client, &package, &scratch),
            package,
        )
    }

    fn verify_failure(&self, package: &Path, expected_code: &str) {
        for verb in ["verify", "verify-links"] {
            let output = self
                .command(verb)
                .args(["--output"])
                .arg(package)
                .output()
                .unwrap();
            assert!(
                !output.status.success(),
                "{verb} unexpectedly accepted package"
            );
            assert_cli_json(&output, "forever-product-v36");
            let error = String::from_utf8(output.stderr).unwrap();
            assert!(
                error.contains(&format!("\"code\":\"{expected_code}\"")),
                "{verb} expected {expected_code}: {error}"
            );
        }
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
    assert!(output.stderr.is_empty(), "{label} wrote to stderr");
    let text = String::from_utf8(output.stdout).unwrap();
    match assert_json_without_duplicate_keys(&text) {
        JsonValue::Object(fields) => assert_eq!(
            fields.get("contract"),
            Some(&JsonValue::String("forever-product-v36".into()))
        ),
        other => panic!("{label} root is not an object: {other:?}"),
    }
    text
}

fn assert_fatal_without_package(output: Output, package: &Path, label: &str, expected_code: &str) {
    assert_cli_json(&output, "forever-product-v36");
    let error = String::from_utf8(output.stderr.clone()).unwrap();
    assert!(
        error.contains(&format!("\"code\":\"{expected_code}\"")),
        "{label} expected {expected_code}: {error}"
    );
    assert!(
        !output.status.success(),
        "{label} unexpectedly succeeded: {}; coverage={}",
        String::from_utf8_lossy(&output.stdout),
        package.join(coverage::PATH).exists()
    );
    for path in [
        coverage::PATH,
        "forever-package.json",
        ".state/sealed/vmaps",
    ] {
        assert!(!package.join(path).exists(), "{label}: {path}");
    }
}

fn assert_cli_json(output: &Output, expected_contract: &str) {
    let (bytes, unused) = if output.status.success() {
        (&output.stdout, &output.stderr)
    } else {
        (&output.stderr, &output.stdout)
    };
    assert!(unused.is_empty(), "unexpected output on other channel");
    let text = std::str::from_utf8(bytes).unwrap();
    let root = assert_json_without_duplicate_keys(text);
    let JsonValue::Object(fields) = root else {
        panic!("CLI JSON root is not an object: {text}");
    };
    assert_eq!(
        fields.get("contract"),
        Some(&JsonValue::String(expected_contract.into())),
        "{text}"
    );
    if !output.status.success() {
        assert!(
            matches!(fields.get("code"), Some(JsonValue::String(_))),
            "{text}"
        );
    } else {
        assert!(!fields.contains_key("code"), "{text}");
    }
}

#[derive(Debug, PartialEq, Eq)]
enum JsonValue {
    Object(BTreeMap<String, JsonValue>),
    Array(Vec<JsonValue>),
    String(String),
    Scalar,
}

struct JsonParser<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl JsonParser<'_> {
    fn whitespace(&mut self) {
        while self
            .bytes
            .get(self.at)
            .is_some_and(|byte| matches!(byte, b' ' | b'\n' | b'\r' | b'\t'))
        {
            self.at += 1;
        }
    }

    fn byte(&self) -> Option<u8> {
        self.bytes.get(self.at).copied()
    }

    fn take(&mut self, expected: u8) -> Result<(), &'static str> {
        if self.byte() == Some(expected) {
            self.at += 1;
            Ok(())
        } else {
            Err("invalid JSON token")
        }
    }

    fn hex_quad(&mut self) -> Result<u16, &'static str> {
        let digits = self
            .bytes
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

    fn string(&mut self) -> Result<String, &'static str> {
        self.take(b'"')?;
        let mut value = String::new();
        loop {
            match self.byte().ok_or("unterminated JSON string")? {
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
                                if self.bytes.get(self.at..self.at + 2) != Some(b"\\u") {
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
                    let tail = std::str::from_utf8(&self.bytes[self.at..])
                        .map_err(|_| "invalid UTF-8 JSON string")?;
                    let character = tail.chars().next().ok_or("invalid UTF-8 JSON string")?;
                    self.at += character.len_utf8();
                    value.push(character);
                }
            }
        }
    }

    fn value(&mut self) -> Result<JsonValue, &'static str> {
        self.whitespace();
        match self.byte().ok_or("missing JSON value")? {
            b'{' => self.object(),
            b'[' => self.array(),
            b'"' => Ok(JsonValue::String(self.string()?)),
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

    fn literal(&mut self, expected: &[u8]) -> Result<JsonValue, &'static str> {
        if self.bytes.get(self.at..self.at + expected.len()) == Some(expected) {
            self.at += expected.len();
            Ok(JsonValue::Scalar)
        } else {
            Err("invalid JSON literal")
        }
    }

    fn number(&mut self) -> Result<(), &'static str> {
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
        Ok(())
    }
}

fn assert_json_without_duplicate_keys(text: &str) -> JsonValue {
    let mut parser = JsonParser {
        bytes: text.as_bytes(),
        at: 0,
    };
    let value = parser
        .value()
        .unwrap_or_else(|error| panic!("{error}: {text}"));
    parser.whitespace();
    assert_eq!(parser.at, parser.bytes.len(), "trailing JSON data: {text}");
    value
}

fn context(package: &Path) -> Context {
    Context::parse(&fs::read_to_string(package.join(".state/context.json")).unwrap()).unwrap()
}

fn package_artifacts(package: &Path, package_context: Context) -> Vec<Artifact> {
    fs::read_dir(package.join(".state/files"))
        .unwrap()
        .map(|entry| {
            Artifact::parse(
                &fs::read_to_string(entry.unwrap().path()).unwrap(),
                package_context,
                Limits::default(),
            )
            .unwrap()
        })
        .collect()
}

fn artifact_inventory(package: &Path) -> Vec<String> {
    let mut artifacts: Vec<_> = package_artifacts(package, context(package))
        .into_iter()
        .map(|artifact| artifact.json())
        .collect();
    artifacts.sort_unstable();
    artifacts
}

fn package_files(package: &Path) -> BTreeMap<String, Vec<u8>> {
    package_artifacts(package, context(package))
        .into_iter()
        .map(|artifact| {
            let bytes = fs::read(package.join(&artifact.path)).unwrap();
            (artifact.path, bytes)
        })
        .collect()
}

fn manifest(package: &Path) -> String {
    fs::read_to_string(package.join("forever-package.json")).unwrap()
}

fn coverage_at(package: &Path) -> Coverage {
    let bytes = fs::read(package.join(coverage::PATH)).unwrap();
    let parsed = Coverage::parse(&bytes).unwrap();
    assert_eq!(parsed.encode(), bytes);
    parsed
}

fn sync_coverage_artifacts(files: &BTreeMap<String, Vec<u8>>, parsed: &mut Coverage) {
    parsed.artifacts = files
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
    mutate: impl FnOnce(&mut BTreeMap<String, Vec<u8>>, &mut Coverage),
) {
    rebuilt_package_with_bytes(base, output, mutate, |_| {});
}

fn rebuilt_package_with_bytes(
    base: &Path,
    output: &Path,
    mutate: impl FnOnce(&mut BTreeMap<String, Vec<u8>>, &mut Coverage),
    mutate_encoded: impl FnOnce(&mut Vec<u8>),
) {
    let package_context = context(base);
    let mut files = package_files(base);
    let mut parsed = Coverage::parse(&files[coverage::PATH]).unwrap();
    mutate(&mut files, &mut parsed);
    sync_coverage_artifacts(&files, &mut parsed);
    let mut encoded = parsed.encode();
    mutate_encoded(&mut encoded);
    files.insert(coverage::PATH.into(), encoded);

    let declaration: PartialEncryptedSection = fixture::partial_map().declaration;
    let mut run = Run::create(output, package_context, &[base], Limits::default()).unwrap();
    run.set_partial_encrypted_table(MAP_PATH, &[declaration])
        .unwrap();
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
}

fn model_name(id: u32) -> String {
    fixture::model_name(id)
}

fn assert_omission_has_no_artifacts(package: &Path, id: u32, parsed: &Coverage) {
    let name = model_name(id);
    assert!(!package.join(format!("vmaps/{name}.vmo")).exists());
    assert!(
        !parsed
            .artifacts
            .iter()
            .any(|fact| fact.path == format!("vmaps/{name}.vmo"))
    );
    for fact in parsed.artifacts.iter().filter(|fact| {
        fact.path.starts_with("vmaps/coverage/") && fact.path.ends_with(".placements")
    }) {
        let bytes = fs::read(package.join(&fact.path)).unwrap();
        let placements = placement::DirBin::parse(&bytes, Default::default()).unwrap();
        assert!(
            placements
                .records()
                .all(|record| record.name() != name.as_bytes())
        );
    }
}

fn assert_placement_failure(lab: &Lab, package: &Path) {
    lab.verify_failure(package, "product_collision_association");
}

fn make_parent_only(files: &mut BTreeMap<String, Vec<u8>>, coverage: &mut Coverage) {
    fixture::set_parent(
        files.get_mut(MAP_PATH).unwrap(),
        4,
        fixture::POPULATED_MAP as i16,
    );
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
    let parent = files["vmaps/0002/0002_00_00.vmtile"].as_slice();
    assembly::verify_inherited_associations(
        parent,
        child.tiles()[0].index_bytes(),
        child.ordered_dir_bin(),
        Default::default(),
    )
    .unwrap();
    files.insert(
        "vmaps/coverage/0005.placements".into(),
        child.ordered_dir_bin().to_vec(),
    );
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
    coverage.maps[3].parent = Some(fixture::POPULATED_MAP);
    coverage.maps[3].depth = 2;
    coverage.maps[2].is_parent = false;
    sync_coverage_artifacts(files, coverage);
}

fn make_root_absent_parent_only(files: &mut BTreeMap<String, Vec<u8>>, coverage: &mut Coverage) {
    fixture::set_parent(
        files.get_mut(MAP_PATH).unwrap(),
        4,
        fixture::ROOT_ABSENT_PARENT as i16,
    );
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
    files.insert(
        "vmaps/coverage/0005.placements".into(),
        child.ordered_dir_bin().to_vec(),
    );
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
    coverage.maps[3].parent = Some(fixture::ROOT_ABSENT_PARENT);
    coverage.maps[3].depth = 3;
    sync_coverage_artifacts(files, coverage);
}

fn replace_placement_model(files: &mut BTreeMap<String, Vec<u8>>, id: u32) {
    let proof_path = format!("vmaps/coverage/{:04}.placements", fixture::POPULATED_MAP);
    let proof = files[&proof_path].clone();
    let target = model_name(id);
    let old_name = model_name(fixture::MODEL);
    let at = proof
        .windows(old_name.len())
        .position(|window| window == old_name.as_bytes())
        .unwrap();
    let mut mutated = proof;
    mutated[at..at + old_name.len()].copy_from_slice(target.as_bytes());
    let parsed = placement::DirBin::parse(&mutated, Default::default()).unwrap();
    let records: Vec<_> = parsed.records().collect();
    let assembled = assembly::build(fixture::POPULATED_MAP, &records, Default::default()).unwrap();
    for path in files
        .keys()
        .filter(|path| {
            path.starts_with("vmaps/0002/0002_")
                && (path.ends_with(".vmtile") || path.ends_with(".vmtileidx"))
        })
        .cloned()
        .collect::<Vec<_>>()
    {
        files.remove(&path);
    }
    files.insert(proof_path, assembled.ordered_dir_bin().to_vec());
    files.insert(
        "vmaps/0002/0002.vmtree".into(),
        assembled.tree_bytes().to_vec(),
    );
    for tile in assembled.tiles() {
        let stem = format!("vmaps/0002/0002_{:02}_{:02}", tile.x(), tile.y());
        files.insert(format!("{stem}.vmtileidx"), tile.index_bytes().to_vec());
        if let Some(payload) = tile.payload() {
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
}

fn copy_tree(source: &Path, destination: &Path) {
    fs::create_dir(destination).unwrap();
    fs::set_permissions(destination, fs::Permissions::from_mode(0o700)).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let kind = entry.file_type().unwrap();
        let target = destination.join(entry.file_name());
        if kind.is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            assert!(kind.is_file());
            fs::copy(entry.path(), &target).unwrap();
            fs::set_permissions(target, fs::Permissions::from_mode(0o600)).unwrap();
        }
    }
}

#[test]
fn json_parser_rejects_duplicate_keys_after_unescaping_and_inside_nested_objects() {
    for invalid in [
        r#"{"field":1,"field":2}"#,
        r#"{"field":1,"fi\u0065ld":2}"#,
        r#"{"outer":[{"field":1,"field":2}]}"#,
    ] {
        let mut parser = JsonParser {
            bytes: invalid.as_bytes(),
            at: 0,
        };
        assert!(
            matches!(parser.value(), Err("duplicate JSON object key")),
            "{invalid}"
        );
    }
}

#[test]
fn v36_inspect_extract_resume_and_both_verifiers_are_v9_bound() {
    let lab = Lab::new();
    let inspect = assert_success(lab.command("inspect").output().unwrap(), "v36 inspect");
    for expected in [
        "\"status\":\"planned\"",
        "\"components\":17",
        "linux70450-material-filtered-wmo-coverage-v9",
        "linux-collision-request-heavy-resources-v1",
    ] {
        assert!(inspect.contains(expected), "{inspect}");
    }

    let package = lab.extract("lifecycle", fixture::AdtMode::MaterialFilteredWmoOnly);
    let parsed = coverage_at(&package);
    fixture::assert_sealed_map(&fs::read(package.join(MAP_PATH)).unwrap());
    fixture::assert_partial_manifest(&manifest(&package));
    assert_eq!(parsed.version, 9);
    assert_eq!(parsed.missing_m2, [fixture::MISSING_M2]);
    assert!(parsed.missing_wmo.is_empty());
    assert!(parsed.empty_wmo.is_empty());
    assert!(parsed.unselected_wmo.is_empty());
    assert_eq!(
        parsed.material_filtered_wmo,
        [fixture::MATERIAL_FILTERED_WMO]
    );
    assert_eq!(
        (
            parsed.missing_m2.as_slice(),
            parsed.missing_wmo.as_slice(),
            parsed.empty_wmo.as_slice(),
            parsed.unselected_wmo.as_slice(),
            parsed.material_filtered_wmo.as_slice(),
        ),
        (&[970793][..], &[][..], &[][..], &[][..], &[462695][..],)
    );
    assert_eq!(parsed.maps.len(), fixture::ACTIVE_MAPS);
    assert_eq!(
        parsed.maps.iter().map(|map| map.id).collect::<Vec<_>>(),
        [
            fixture::EMPTY_MAP,
            fixture::POPULATED_MAP,
            fixture::ROOT_ABSENT_PARENT,
            fixture::PRESENT_CHILD,
            fixture::ROOT_ABSENT_OTHER,
        ]
    );
    assert_eq!(
        parsed.maps.iter().filter(|map| !map.root_absent).count(),
        fixture::PRESENT_WDT_FILES
    );
    assert_eq!(
        parsed.maps.iter().filter(|map| map.root_absent).count(),
        fixture::ROOT_ABSENT_MAPS
    );
    assert_eq!(
        parsed
            .maps
            .iter()
            .find(|map| map.id == fixture::PRESENT_CHILD)
            .unwrap()
            .parent,
        Some(fixture::ROOT_ABSENT_PARENT)
    );
    assert!(parsed.maps.iter().all(|map| !matches!(
        map.id,
        fixture::WDT_ZERO_CHILD
            | fixture::INACTIVE_MAP
            | fixture::COPIED_INACTIVE_MAP
            | fixture::HIDDEN_MAP
    )));
    for id in [fixture::ROOT_ABSENT_PARENT, fixture::ROOT_ABSENT_OTHER] {
        assert!(
            !parsed
                .artifacts
                .iter()
                .any(|artifact| artifact.path.starts_with(&format!("vmaps/{id:04}/")))
        );
        assert!(!package.join(format!("vmaps/{id:04}")).exists());
    }
    for id in [fixture::MISSING_M2, fixture::MATERIAL_FILTERED_WMO] {
        assert_omission_has_no_artifacts(&package, id, &parsed);
    }
    lab.verify(&package);

    let resumed = lab.0.join("resumed-package");
    copy_tree(&package, &resumed);
    fs::remove_file(resumed.join("forever-package.json")).unwrap();
    fs::write(resumed.join(".forever-package-incomplete"), b"incomplete\n").unwrap();
    let client = lab.private_dir("resume-client");
    fixture::write_client_with(
        &client,
        fixture::ClientMutation {
            populated_adt_mode: fixture::AdtMode::MaterialFilteredWmoOnly,
            ..Default::default()
        },
    );
    let scratch = lab.private_dir("resume-scratch");
    assert_success(
        lab.produce("resume", &lab.plan(), &client, &resumed, &scratch),
        "v36 resume",
    );
    assert_eq!(manifest(&resumed), manifest(&package));
    assert_eq!(artifact_inventory(&resumed), artifact_inventory(&package));
    assert_eq!(
        fs::read(resumed.join(coverage::PATH)).unwrap(),
        fs::read(package.join(coverage::PATH)).unwrap()
    );
    lab.verify(&resumed);

    for verb in ["verify", "verify-links"] {
        let output = lab
            .command_with_plan(verb, &lab.v35_plan())
            .args(["--output"])
            .arg(&package)
            .output()
            .unwrap();
        assert!(
            !output.status.success(),
            "{verb} accepted v36 under v35 plan"
        );
        assert_cli_json(&output, "forever-product-v35");
        let text = String::from_utf8(output.stderr).unwrap();
        assert!(
            text.contains("\"code\":\"package_inventory\"")
                || text.contains("\"code\":\"product_plan_mismatch\""),
            "{verb}: {text}"
        );
    }
}

#[test]
fn v36_five_way_mixed_omissions_are_disjoint_and_preserve_present_outputs() {
    let lab = Lab::new();
    let mixed = lab.extract("mixed", fixture::AdtMode::MaterialFilteredWmoMixed);
    let control = lab.extract("control", fixture::AdtMode::NoOmissions);
    let parsed = coverage_at(&mixed);
    assert_eq!(parsed.missing_m2, [fixture::MISSING_M2]);
    assert_eq!(parsed.missing_wmo, [fixture::MISSING_WMO]);
    assert_eq!(parsed.empty_wmo, [fixture::EMPTY_WMO]);
    assert_eq!(parsed.unselected_wmo, [fixture::UNSELECTED_WMO]);
    assert_eq!(
        parsed.material_filtered_wmo,
        [fixture::MATERIAL_FILTERED_WMO_MIXED]
    );
    assert_eq!(
        (
            parsed.missing_m2.as_slice(),
            parsed.missing_wmo.as_slice(),
            parsed.empty_wmo.as_slice(),
            parsed.unselected_wmo.as_slice(),
            parsed.material_filtered_wmo.as_slice(),
        ),
        (
            &[970793][..],
            &[462694][..],
            &[462697][..],
            &[462695][..],
            &[462701][..],
        )
    );
    let omitted_ids = [
        fixture::MISSING_M2,
        fixture::MISSING_WMO,
        fixture::EMPTY_WMO,
        fixture::UNSELECTED_WMO,
        fixture::MATERIAL_FILTERED_WMO_MIXED,
    ];
    let distinct_ids = omitted_ids
        .into_iter()
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(distinct_ids.len(), omitted_ids.len());
    for id in omitted_ids {
        assert_omission_has_no_artifacts(&mixed, id, &parsed);
    }

    let clean = coverage_at(&control);
    assert!(clean.missing_m2.is_empty());
    assert!(clean.missing_wmo.is_empty());
    assert!(clean.empty_wmo.is_empty());
    assert!(clean.unselected_wmo.is_empty());
    assert!(clean.material_filtered_wmo.is_empty());
    let mut later_map_ids = Vec::new();
    for map in [fixture::POPULATED_MAP, fixture::PRESENT_CHILD] {
        let proof_path = format!("vmaps/coverage/{map:04}.placements");
        let actual = fs::read(mixed.join(&proof_path)).unwrap();
        let expected = fs::read(control.join(&proof_path)).unwrap();
        let ids = |bytes: &[u8]| {
            placement::DirBin::parse(bytes, Default::default())
                .unwrap()
                .records()
                .map(|record| record.id())
                .collect::<Vec<_>>()
        };
        assert_eq!(ids(&actual), ids(&expected), "map {map}");
        assert_eq!(actual, expected, "map {map}");
        later_map_ids.push(ids(&actual));
    }
    for id in [fixture::WMO_ROOT, fixture::MODEL] {
        let path = format!("vmaps/{}.vmo", model_name(id));
        let mixed_bytes = fs::read(mixed.join(&path)).unwrap();
        let control_bytes = fs::read(control.join(&path)).unwrap();
        assert_eq!(mixed_bytes, control_bytes, "{path}");
        assert!(vmo::View::parse(&mixed_bytes, Default::default()).is_ok());
    }
    let vmo_paths = |package: &Path| {
        package_artifacts(package, context(package))
            .into_iter()
            .filter(|artifact| {
                artifact.path.starts_with("vmaps/") && artifact.path.ends_with(".vmo")
            })
            .map(|artifact| artifact.path)
            .collect::<std::collections::BTreeSet<_>>()
    };
    let expected_vmos = std::collections::BTreeSet::from([
        format!("vmaps/{}.vmo", model_name(fixture::WMO_ROOT)),
        format!("vmaps/{}.vmo", model_name(fixture::MODEL)),
    ]);
    assert_eq!(vmo_paths(&mixed), expected_vmos);
    assert_eq!(vmo_paths(&control), expected_vmos);
    assert!(
        later_map_ids[1]
            .iter()
            .any(|id| !later_map_ids[0].contains(id))
    );
    for map in [fixture::ROOT_ABSENT_PARENT, fixture::ROOT_ABSENT_OTHER] {
        assert!(!mixed.join(format!("vmaps/{map:04}")).exists());
        assert!(!control.join(format!("vmaps/{map:04}")).exists());
    }
    lab.verify(&mixed);
    lab.verify(&control);
}

#[test]
fn v36_production_verifiers_reject_resealed_omission_artifact_and_placement_attacks() {
    let lab = Lab::new();
    let baseline = lab.extract(
        "attack-baseline",
        fixture::AdtMode::MaterialFilteredWmoMixed,
    );
    let omitted = [
        fixture::MISSING_M2,
        fixture::MISSING_WMO,
        fixture::EMPTY_WMO,
        fixture::UNSELECTED_WMO,
        fixture::MATERIAL_FILTERED_WMO_MIXED,
    ];

    for id in omitted {
        let output = lab.0.join(format!("vmo-for-{id}"));
        let valid_vmo =
            fs::read(baseline.join(format!("vmaps/{}.vmo", model_name(fixture::MODEL)))).unwrap();
        rebuilt_package(&baseline, &output, |files, _| {
            files.insert(format!("vmaps/{}.vmo", model_name(id)), valid_vmo);
        });
        lab.verify_failure(&output, "product_collision_association");
    }

    for id in omitted {
        let output = lab.0.join(format!("placement-for-{id}"));
        rebuilt_package(&baseline, &output, |files, _| {
            replace_placement_model(files, id)
        });
        assert_placement_failure(&lab, &output);
    }

    let raw = fs::read(baseline.join(coverage::PATH)).unwrap();
    for (label, mutation) in [
        ("zero-missing-m2", 0_u8),
        ("duplicate-missing-m2", 1),
        ("descending-missing-m2", 2),
        ("zero-missing-wmo", 3),
        ("duplicate-missing-wmo", 4),
        ("descending-missing-wmo", 5),
        ("zero-empty-wmo", 6),
        ("duplicate-empty-wmo", 7),
        ("descending-empty-wmo", 8),
        ("zero-unselected-wmo", 9),
        ("duplicate-unselected-wmo", 10),
        ("descending-unselected-wmo", 11),
        ("zero-material-filtered-wmo", 12),
        ("duplicate-material-filtered-wmo", 13),
        ("descending-material-filtered-wmo", 14),
        ("m2-missing-wmo-overlap", 15),
        ("m2-empty-wmo-overlap", 16),
        ("m2-unselected-wmo-overlap", 17),
        ("m2-material-filtered-overlap", 18),
        ("missing-empty-wmo-overlap", 19),
        ("missing-unselected-wmo-overlap", 20),
        ("missing-material-filtered-overlap", 21),
        ("empty-unselected-wmo-overlap", 22),
        ("empty-material-filtered-overlap", 23),
        ("unselected-material-filtered-overlap", 24),
    ] {
        let output = lab.0.join(format!("coverage-{label}"));
        rebuilt_package(&baseline, &output, |_, parsed| match mutation {
            0 => parsed.missing_m2 = vec![0],
            1 => parsed.missing_m2 = vec![fixture::MODEL, fixture::MODEL],
            2 => parsed.missing_m2 = vec![fixture::MISSING_M2 + 1, fixture::MISSING_M2],
            3 => parsed.missing_wmo = vec![0],
            4 => parsed.missing_wmo = vec![fixture::MISSING_WMO, fixture::MISSING_WMO],
            5 => parsed.missing_wmo = vec![fixture::MISSING_WMO + 1, fixture::MISSING_WMO],
            6 => parsed.empty_wmo = vec![0],
            7 => parsed.empty_wmo = vec![fixture::EMPTY_WMO, fixture::EMPTY_WMO],
            8 => parsed.empty_wmo = vec![fixture::EMPTY_WMO + 1, fixture::EMPTY_WMO],
            9 => parsed.unselected_wmo = vec![0],
            10 => parsed.unselected_wmo = vec![fixture::UNSELECTED_WMO, fixture::UNSELECTED_WMO],
            11 => {
                parsed.unselected_wmo = vec![fixture::UNSELECTED_WMO + 1, fixture::UNSELECTED_WMO]
            }
            12 => parsed.material_filtered_wmo = vec![0],
            13 => {
                parsed.material_filtered_wmo = vec![
                    fixture::MATERIAL_FILTERED_WMO,
                    fixture::MATERIAL_FILTERED_WMO,
                ]
            }
            14 => {
                parsed.material_filtered_wmo = vec![
                    fixture::MATERIAL_FILTERED_WMO + 1,
                    fixture::MATERIAL_FILTERED_WMO,
                ]
            }
            15 => {
                parsed.missing_m2 = vec![3_000_000];
                parsed.missing_wmo = vec![3_000_000];
            }
            16 => {
                parsed.missing_m2 = vec![3_000_000];
                parsed.empty_wmo = vec![3_000_000];
            }
            17 => {
                parsed.missing_m2 = vec![3_000_000];
                parsed.unselected_wmo = vec![3_000_000];
            }
            18 => {
                parsed.missing_m2 = vec![3_000_000];
                parsed.material_filtered_wmo = vec![3_000_000];
            }
            19 => {
                parsed.missing_wmo = vec![3_000_000];
                parsed.empty_wmo = vec![3_000_000];
            }
            20 => {
                parsed.missing_wmo = vec![3_000_000];
                parsed.unselected_wmo = vec![3_000_000];
            }
            21 => {
                parsed.missing_wmo = vec![3_000_000];
                parsed.material_filtered_wmo = vec![3_000_000];
            }
            22 => {
                parsed.empty_wmo = vec![3_000_000];
                parsed.unselected_wmo = vec![3_000_000];
            }
            23 => {
                parsed.empty_wmo = vec![3_000_000];
                parsed.material_filtered_wmo = vec![3_000_000];
            }
            24 => {
                parsed.unselected_wmo = vec![3_000_000];
                parsed.material_filtered_wmo = vec![3_000_000];
            }
            _ => unreachable!(),
        });
        lab.verify_failure(&output, "product_selected_geometry_coverage_schema");
    }

    for (label, mutation) in [
        ("wrong-map-authentication", 0_u8),
        ("wrong-m2-policy", 1),
        ("wrong-wmo-policy", 2),
        ("wrong-coverage-profile", 3),
        ("wrong-source-profile", 4),
        ("wrong-domain-profile", 5),
        ("wrong-resource-profile", 6),
        ("wrong-wdt-policy", 7),
    ] {
        let output = lab.0.join(format!("coverage-{label}"));
        rebuilt_package(&baseline, &output, |_, parsed| match mutation {
            0 => parsed.map.full_ckey_verified = true,
            1 => {
                parsed.missing_m2_policy = forever_casc::collision::run::MissingModelPolicy::Reject
            }
            2 => parsed.missing_wmo_policy = forever_casc::collision::run::MissingWmoPolicy::Reject,
            3 => parsed.coverage_profile = "wrong".into(),
            4 => parsed.source_profile = "wrong".into(),
            5 => parsed.domain_profile = "wrong".into(),
            6 => parsed.resource_profile = "wrong".into(),
            7 => parsed.missing_wdt_policy = forever_casc::collision::run::MissingWdtPolicy::Reject,
            _ => unreachable!(),
        });
        lab.verify_failure(&output, "product_selected_geometry_coverage_schema");
    }

    for end in [0, 7, 8, 11, 12, raw.len() / 2, raw.len() - 1] {
        let output = lab.0.join(format!("coverage-prefix-{end}"));
        rebuilt_package_with_bytes(&baseline, &output, |_, _| {}, |bytes| bytes.truncate(end));
        lab.verify_failure(&output, "product_selected_geometry_coverage_schema");
    }
    for count in [1, 8] {
        let output = lab.0.join(format!("coverage-suffix-{count}"));
        rebuilt_package_with_bytes(
            &baseline,
            &output,
            |_, _| {},
            |bytes| bytes.extend(vec![0; count]),
        );
        lab.verify_failure(&output, "product_selected_geometry_coverage_schema");
    }

    let parent_replay = lab.0.join("valid-parent-replay");
    rebuilt_package(&baseline, &parent_replay, |files, parsed| {
        make_parent_only(files, parsed);
    });
    lab.verify(&parent_replay);

    let forbidden_parent = lab.0.join("root-absent-parent-replay");
    rebuilt_package(&baseline, &forbidden_parent, |files, parsed| {
        make_root_absent_parent_only(files, parsed);
    });
    lab.verify_failure(&forbidden_parent, "product_collision_association");

    for version in [8_u32, 7] {
        let crossed = lab.0.join(format!("v{version}-layout-under-v36"));
        rebuilt_package_with_bytes(
            &baseline,
            &crossed,
            |_, _| {},
            |bytes| {
                bytes[8..12].copy_from_slice(&version.to_le_bytes());
            },
        );
        lab.verify_failure(&crossed, "product_selected_geometry_coverage_schema");
    }
}

#[test]
fn v36_five_omission_lists_enforce_4096_and_4097_in_both_production_verifiers() {
    let lab = Lab::new();
    let baseline = lab.extract("list-limits", fixture::AdtMode::MaterialFilteredWmoOnly);
    for field in [
        "missing_m2",
        "missing_wmo",
        "empty_wmo",
        "unselected_wmo",
        "material_filtered_wmo",
    ] {
        for count in [4096_usize, 4097] {
            let output = lab.0.join(format!("{field}-{count}"));
            rebuilt_package(&baseline, &output, |_, parsed| {
                let ids: Vec<_> = (3_000_000..3_000_000 + count as u32).collect();
                match field {
                    "missing_m2" => parsed.missing_m2 = ids,
                    "missing_wmo" => parsed.missing_wmo = ids,
                    "empty_wmo" => parsed.empty_wmo = ids,
                    "unselected_wmo" => parsed.unselected_wmo = ids,
                    "material_filtered_wmo" => parsed.material_filtered_wmo = ids,
                    _ => unreachable!(),
                }
            });
            if count == 4096 {
                lab.verify(&output);
            } else {
                lab.verify_failure(&output, "product_selected_geometry_limit");
            }
        }
    }
}

#[test]
fn v36_material_filtered_wmo_source_geometry_is_present_but_fully_filtered() {
    let lab = Lab::new();
    let (root_bytes, group_bytes) = fixture::material_filtered_wmo_source();
    fixture::assert_material_filtered_wmo_source(&root_bytes, &group_bytes);
    let package = lab.extract("source-geometry", fixture::AdtMode::MaterialFilteredWmoOnly);
    let coverage = coverage_at(&package);
    assert_eq!(
        coverage.material_filtered_wmo,
        [fixture::MATERIAL_FILTERED_WMO]
    );
    assert!(coverage.missing_wmo.is_empty());
    assert!(coverage.empty_wmo.is_empty());
    assert!(coverage.unselected_wmo.is_empty());
    let name = model_name(fixture::MATERIAL_FILTERED_WMO);
    assert!(!package.join(format!("vmaps/{name}.vmo")).exists());
    assert_omission_has_no_artifacts(&package, fixture::MATERIAL_FILTERED_WMO, &coverage);
    lab.verify(&package);
}

#[test]
fn v36_useful_geometry_is_emitted_and_material_filtered_omissions_require_the_full_predicate() {
    let lab = Lab::new();
    let (control_root, control_group) = fixture::material_filtered_wmo_selected_geometry_source();
    fixture::assert_material_filtered_wmo_material_controls(&control_root, &control_group, 0x08);
    let package = lab.extract(
        "selected-geometry",
        fixture::AdtMode::MaterialFilteredWmoSelectedGeometry,
    );
    let coverage = coverage_at(&package);
    assert!(coverage.material_filtered_wmo.is_empty());
    assert!(coverage.empty_wmo.is_empty());
    let model = model_name(fixture::MATERIAL_FILTERED_WMO);
    let model_bytes = fs::read(package.join(format!("vmaps/{model}.vmo"))).unwrap();
    let model_view = vmo::View::parse(&model_bytes, Default::default()).unwrap();
    assert_eq!(model_view.stats().triangles, 1);
    let placements_path = format!("vmaps/coverage/{:04}.placements", fixture::POPULATED_MAP);
    let placement_bytes = fs::read(package.join(placements_path)).unwrap();
    let placements = placement::DirBin::parse(&placement_bytes, Default::default()).unwrap();
    assert!(
        placements
            .records()
            .any(|record| record.name() == model.as_bytes())
    );
    lab.verify(&package);

    let (kept_root, kept_group) = fixture::material_filtered_wmo_kept_surface_source();
    fixture::assert_material_filtered_wmo_material_controls(&kept_root, &kept_group, 0x20);
    let kept_surface = lab.extract(
        "kept-surface",
        fixture::AdtMode::MaterialFilteredWmoKeptSurface,
    );
    let kept_coverage = coverage_at(&kept_surface);
    assert!(kept_coverage.material_filtered_wmo.is_empty());
    assert!(kept_coverage.empty_wmo.is_empty());
    let kept_model = model_name(fixture::MATERIAL_FILTERED_WMO);
    let kept_vmo = fs::read(kept_surface.join(format!("vmaps/{kept_model}.vmo"))).unwrap();
    assert_eq!(
        vmo::View::parse(&kept_vmo, Default::default())
            .unwrap()
            .stats()
            .triangles,
        1
    );
    let kept_placements = fs::read(kept_surface.join(format!(
        "vmaps/coverage/{:04}.placements",
        fixture::POPULATED_MAP
    )))
    .unwrap();
    assert!(
        placement::DirBin::parse(&kept_placements, Default::default())
            .unwrap()
            .records()
            .any(|record| record.name() == kept_model.as_bytes())
    );
    lab.verify(&kept_surface);

    let useful_bindings = lab.extract(
        "useful-bindings",
        fixture::AdtMode::MaterialFilteredWmoUsefulBindings,
    );
    let useful_coverage = coverage_at(&useful_bindings);
    assert!(useful_coverage.material_filtered_wmo.is_empty());
    assert!(useful_coverage.empty_wmo.is_empty());
    let useful_model = model_name(fixture::MODEL);
    assert!(
        useful_bindings
            .join(format!("vmaps/{useful_model}.vmo"))
            .is_file()
    );
    assert!(
        !useful_bindings
            .join(format!(
                "vmaps/{}.vmo",
                model_name(fixture::MATERIAL_FILTERED_WMO)
            ))
            .exists()
    );
    let useful_placements = fs::read(useful_bindings.join(format!(
        "vmaps/coverage/{:04}.placements",
        fixture::POPULATED_MAP
    )))
    .unwrap();
    assert_eq!(
        placement::DirBin::parse(&useful_placements, Default::default())
            .unwrap()
            .records()
            .filter(|record| record.name() == useful_model.as_bytes())
            .count(),
        2
    );
    lab.verify(&useful_bindings);

    let (reference_root, selected_group, skipped_group) =
        fixture::material_filtered_wmo_useful_reference_source();
    fixture::assert_material_filtered_wmo_useful_reference_source(
        &reference_root,
        &selected_group,
        &skipped_group,
    );
    let useful_reference = lab.extract(
        "useful-reference",
        fixture::AdtMode::MaterialFilteredWmoUsefulReference,
    );
    let useful_reference_coverage = coverage_at(&useful_reference);
    assert!(useful_reference_coverage.material_filtered_wmo.is_empty());
    assert!(useful_reference_coverage.empty_wmo.is_empty());
    let reference_model = model_name(fixture::USEFUL_REFERENCE_MODEL);
    let reference_model_bytes =
        fs::read(useful_reference.join(format!("vmaps/{reference_model}.vmo"))).unwrap();
    assert!(vmo::View::parse(&reference_model_bytes, Default::default()).is_ok());
    assert_ne!(
        reference_model_bytes,
        fs::read(useful_reference.join(format!("vmaps/{}.vmo", useful_model))).unwrap()
    );
    let own_wmo = model_name(fixture::MATERIAL_FILTERED_WMO);
    assert!(
        !useful_reference
            .join(format!("vmaps/{own_wmo}.vmo"))
            .exists()
    );
    let useful_placements = fs::read(useful_reference.join(format!(
        "vmaps/coverage/{:04}.placements",
        fixture::POPULATED_MAP
    )))
    .unwrap();
    assert_eq!(
        placement::DirBin::parse(&useful_placements, Default::default())
            .unwrap()
            .records()
            .filter(|record| record.name() == reference_model.as_bytes())
            .count(),
        1
    );
    lab.verify(&useful_reference);

    let (local_missing_result, local_missing_package) = lab.extract_result(
        "local-missing-m2",
        fixture::AdtMode::MaterialFilteredWmoLocalMissingM2,
    );
    assert_success(local_missing_result, "v36 local missing doodad M2");
    let local_missing_coverage = coverage_at(&local_missing_package);
    assert_eq!(local_missing_coverage.missing_m2, [fixture::MISSING_M2]);
    assert_eq!(
        local_missing_coverage.missing_wmo,
        [fixture::MATERIAL_FILTERED_WMO]
    );
    assert!(local_missing_coverage.material_filtered_wmo.is_empty());
    assert!(local_missing_coverage.empty_wmo.is_empty());
    lab.verify(&local_missing_package);

    for (label, mode) in [
        (
            "missing-root-sets",
            fixture::AdtMode::MaterialFilteredWmoMissingSets,
        ),
        (
            "logical-origin-with-material-filtered-source",
            fixture::AdtMode::MaterialFilteredWmoLogicalOrigin,
        ),
        (
            "liquid-with-filtered-geometry",
            fixture::AdtMode::MaterialFilteredWmoLiquid,
        ),
        (
            "filtered-and-skipped-groups",
            fixture::AdtMode::MaterialFilteredWmoSkippedGroup,
        ),
    ] {
        let client = lab.private_dir(&format!("{label}-client"));
        fixture::write_client_with(
            &client,
            fixture::ClientMutation {
                populated_adt_mode: mode,
                ..Default::default()
            },
        );
        let output = lab.0.join(format!("{label}-package"));
        let scratch = lab.private_dir(&format!("{label}-scratch"));
        let result = lab.produce("extract", &lab.plan(), &client, &output, &scratch);
        assert_fatal_without_package(result, &output, label, "collision_no_valid_collision");
    }

    let collision_control = lab.extract(
        "ordinary-collision",
        fixture::AdtMode::MaterialFilteredWmoSelectedGeometry,
    );
    assert!(
        coverage_at(&collision_control)
            .material_filtered_wmo
            .is_empty()
    );
    let collision_wmo = model_name(fixture::MATERIAL_FILTERED_WMO);
    assert!(
        vmo::View::parse(
            &fs::read(collision_control.join(format!("vmaps/{collision_wmo}.vmo"))).unwrap(),
            Default::default()
        )
        .is_ok()
    );
    lab.verify(&collision_control);
}

#[test]
fn v36_material_filtered_wmo_acquisition_and_parse_failures_cannot_become_omissions() {
    let lab = Lab::new();
    for (label, mutation) in [
        (
            "root-absent",
            fixture::ClientMutation {
                omit_root: Some(fixture::MATERIAL_FILTERED_WMO),
                populated_adt_mode: fixture::AdtMode::MaterialFilteredWmoOnly,
                ..Default::default()
            },
        ),
        (
            "group-absent",
            fixture::ClientMutation {
                omit_root: Some(fixture::MATERIAL_FILTERED_WMO_GROUP),
                populated_adt_mode: fixture::AdtMode::MaterialFilteredWmoOnly,
                ..Default::default()
            },
        ),
        (
            "root-central-absent",
            fixture::ClientMutation {
                omit_central: Some(fixture::MATERIAL_FILTERED_WMO),
                populated_adt_mode: fixture::AdtMode::MaterialFilteredWmoOnly,
                ..Default::default()
            },
        ),
        (
            "root-encoding-absent",
            fixture::ClientMutation {
                omit_encoding: Some(fixture::MATERIAL_FILTERED_WMO),
                populated_adt_mode: fixture::AdtMode::MaterialFilteredWmoOnly,
                ..Default::default()
            },
        ),
        (
            "root-ineligible",
            fixture::ClientMutation {
                ineligible_root: Some(fixture::MATERIAL_FILTERED_WMO),
                populated_adt_mode: fixture::AdtMode::MaterialFilteredWmoOnly,
                ..Default::default()
            },
        ),
        (
            "group-central-absent",
            fixture::ClientMutation {
                omit_central: Some(fixture::MATERIAL_FILTERED_WMO_GROUP),
                populated_adt_mode: fixture::AdtMode::MaterialFilteredWmoOnly,
                ..Default::default()
            },
        ),
        (
            "group-encoding-absent",
            fixture::ClientMutation {
                omit_encoding: Some(fixture::MATERIAL_FILTERED_WMO_GROUP),
                populated_adt_mode: fixture::AdtMode::MaterialFilteredWmoOnly,
                ..Default::default()
            },
        ),
        (
            "group-ineligible",
            fixture::ClientMutation {
                ineligible_root: Some(fixture::MATERIAL_FILTERED_WMO_GROUP),
                populated_adt_mode: fixture::AdtMode::MaterialFilteredWmoOnly,
                ..Default::default()
            },
        ),
        (
            "malformed-root",
            fixture::ClientMutation {
                malformed_model: Some(fixture::MATERIAL_FILTERED_WMO),
                populated_adt_mode: fixture::AdtMode::MaterialFilteredWmoOnly,
                ..Default::default()
            },
        ),
        (
            "malformed-group",
            fixture::ClientMutation {
                malformed_model: Some(fixture::MATERIAL_FILTERED_WMO_GROUP),
                populated_adt_mode: fixture::AdtMode::MaterialFilteredWmoOnly,
                ..Default::default()
            },
        ),
    ] {
        let client = lab.private_dir(&format!("{label}-client"));
        fixture::write_client_with(&client, mutation);
        let output = lab.0.join(format!("{label}-package"));
        let scratch = lab.private_dir(&format!("{label}-scratch"));
        let result = lab.produce("extract", &lab.plan(), &client, &output, &scratch);
        let code = match label {
            "root-absent"
            | "group-absent"
            | "root-encoding-absent"
            | "root-ineligible"
            | "group-encoding-absent"
            | "group-ineligible" => "file_unavailable",
            "root-central-absent" | "group-central-absent" => "local_store_no_verified_encoding",
            "malformed-root" | "malformed-group" => "wmo_framing",
            _ => unreachable!(),
        };
        assert_fatal_without_package(result, &output, label, code);
    }

    for (label, id) in [
        ("root-corrupt", fixture::MATERIAL_FILTERED_WMO),
        ("group-corrupt", fixture::MATERIAL_FILTERED_WMO_GROUP),
    ] {
        let client = lab.private_dir(&format!("{label}-client"));
        let written = fixture::write_client_with(
            &client,
            fixture::ClientMutation {
                populated_adt_mode: fixture::AdtMode::MaterialFilteredWmoOnly,
                ..Default::default()
            },
        );
        fixture::corrupt_input(&client, &written, id);
        let output = lab.0.join(format!("{label}-package"));
        let scratch = lab.private_dir(&format!("{label}-scratch"));
        let result = lab.produce("extract", &lab.plan(), &client, &output, &scratch);
        assert_fatal_without_package(result, &output, label, "local_store_no_verified_encoding");
    }
}

#[test]
fn v35_resume_and_verifiers_use_only_the_synthetic_plan_fixture() {
    let raw = PLAN.as_bytes();
    assert!(raw.ends_with(b"\n"));
    assert!(raw.is_ascii());
    assert!(
        !raw.windows(b"requested_map".len())
            .any(|part| part == b"requested_map")
    );
}
